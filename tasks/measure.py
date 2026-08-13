"""Warm-dispatch latency measurement (quiet host; outside `check`/`default`).

Measures the Rust `vcs guard` dispatched through the real bootstrap (`G`)
against the shell guard it replaced (`B`), recovered from the revision
preceding its deletion, and classifies the six cells work item 0189's Latency
Criterion defines.

The analysis lives in `tasks/shared/measurement.py` as pure functions; this
module holds the per-platform constants, the artefact manifest and its
capture/restore/verify context manager, the subprocess ports and the task
surface. Prerequisites and the reason the namespace sits outside the aggregate
`check` are documented in `tasks/README.md`'s `### The measure namespace`.
"""

from __future__ import annotations

import json
import os
import random
import shutil
import signal
import subprocess
import tempfile
import time
from collections.abc import Mapping, Sequence
from contextlib import suppress
from dataclasses import asdict, dataclass, field
from enum import StrEnum, unique
from pathlib import Path
from types import FrameType, TracebackType
from typing import Protocol, Self

from invoke import Context, Exit, task

from tasks.shared.measurement import (
    ArtefactState,
    Branch,
    Calibration,
    CellKind,
    CellOutcome,
    CpuProbes,
    Decision,
    Interval,
    PlatformEntry,
    Validity,
    Variant,
    accelerator_override_keys,
    calibration_holds,
    ceiling_directories,
    classify,
    closure_verdict,
    drift_verdict,
    expected_decision,
    generate_schedule,
    log_appended_lines,
    median,
    outlier_trip,
    paired_ratio_interval,
    percentile,
    platform_constants,
    power_state,
    residual_verdict,
    resolve_cpu_count,
    resolve_platform_key,
    retry_budget,
    summarise,
    tmp_containment,
    unchanged_artefacts,
    unpaired_interval,
    validate_sample,
)
from tasks.shared.paths import REPO_ROOT

# --- Criterion constants --------------------------------------------------
# Held in lockstep with `tasks/README.md`'s `### Criterion constants` block by
# tests/unit/tasks/test_measure.py, bidirectionally: every constant appears in
# the doc block and every number in the doc block resolves to a name here.

RESAMPLES = 10000
CONFIDENCE = 0.95
RATIO_THRESHOLD = 1.3
MEDIAN_TARGET_MS = 1.0
P90_TARGET_MS = 2.0
RATIO_TARGET = 0.0036
RATIO_ESCALATION_TARGET = 0.0018
DRIFT_BAND = 0.005
BLOCK_A_PAIRS = 1700
BLOCK_B_SAMPLES = 900
BLOCK_A_MAX_PAIRS = 6900
BLOCK_B_MAX_SAMPLES = 3600
PILOT_PAIRS = 200
PILOT_SAMPLES = 200
SEGMENT_SAMPLES = 100
WALL_CLOCK_BUDGET_S = 2100.0
FLOOR_RETRY_CAP = 3
SEED = 20260813
SMOKE_N = 2

# The published digest of the release verification key. Comparing against a
# published value — not the act of recording a digest — is what detects a key
# substituted *before* the session; recording it detects one *during*.
RELEASE_KEY_SHA256 = (
    "0f3fe9a91ab6869ce36209691e06c722259e5754f2228b1539ef566b00f6fb2e"
)

# The revision preceding `hooks/vcs-guard.sh`'s deletion, and the commit it
# resolves to. The commit id is recorded because a short hex prefix can be a jj
# *change* id, and because a plain git clone has no `.jj` to resolve a revset
# against.
BASELINE_REVSET = "cf42441e2aad-"
BASELINE_COMMIT = "2cfbf81e2e7b4934e868bd42c69374c335b05317"

STDIN_ENVELOPE = json.dumps(
    {
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "git status"},
    }
)

_DARWIN_CALIBRATION = Calibration(
    session="0205",
    chip="Apple M4 Max",
    bash="/bin/bash 3.2.57(1)-release",
    shasum="Perl shasum 6.04",
)

PLATFORM_TABLE: dict[tuple[str, str], PlatformEntry] = {
    ("Darwin", "arm64"): PlatformEntry(
        key="darwin-arm64",
        path_tools=(
            "awk",
            "bash",
            "cat",
            "cp",
            "curl",
            "cut",
            "dirname",
            "grep",
            "head",
            "jj",
            "jq",
            "mkdir",
            "mv",
            "printf",
            "realpath",
            "rm",
            "sed",
            "shasum",
            "sort",
            "tr",
            "true",
            "uname",
            "wc",
        ),
        power_probes=(
            ["pmset", "-g", "ps"],
            ["pmset", "-g", "therm"],
            ["pmset", "-g", "live"],
        ),
        median_ceiling_fast_ms=50.0,
        p90_ceiling_fast_ms=60.0,
        median_ceiling_fallback_ms=70.0,
        p90_ceiling_fallback_ms=80.0,
        bash_floor_ms=7.8,
        true_floor_ms=1.95,
        reference_bash="/bin/bash 3.2.57(1)-release",
        calibration=_DARWIN_CALIBRATION,
    ),
}

# The fast digest backend is `sha256sum`; the fallback is Perl `shasum -a 256`.
# The two farms differ solely in whether the `sha256sum` link is present.
# Cache-root entries the per-sample and post-run witnesses cover: the launcher
# and its signature, the staged verify shim, and every dispatched sub-binary
# asset (named `<token>-<version>-<digest>`).
_CACHED_ENTRY_PREFIXES = (
    "accelerator-launcher-",
    "accelerator-verify-",
    "vcs-",
)

FAST_BACKEND = "sha256sum"
FALLBACK_BACKEND = "shasum"


def criterion_constants() -> dict[str, float]:
    """Every pre-registered number a run is judged by, by name.

    One function so the lockstep guard has a single source to compare the doc
    block against — including the per-platform ceilings and floor gates, which
    must move with the mechanism rather than staying host-specific behind it.
    """
    constants: dict[str, float] = {
        "RESAMPLES": RESAMPLES,
        "CONFIDENCE": CONFIDENCE,
        "RATIO_THRESHOLD": RATIO_THRESHOLD,
        "MEDIAN_TARGET_MS": MEDIAN_TARGET_MS,
        "P90_TARGET_MS": P90_TARGET_MS,
        "RATIO_TARGET": RATIO_TARGET,
        "RATIO_ESCALATION_TARGET": RATIO_ESCALATION_TARGET,
        "DRIFT_BAND": DRIFT_BAND,
        "BLOCK_A_PAIRS": BLOCK_A_PAIRS,
        "BLOCK_B_SAMPLES": BLOCK_B_SAMPLES,
        "BLOCK_A_MAX_PAIRS": BLOCK_A_MAX_PAIRS,
        "BLOCK_B_MAX_SAMPLES": BLOCK_B_MAX_SAMPLES,
        "PILOT_PAIRS": PILOT_PAIRS,
        "PILOT_SAMPLES": PILOT_SAMPLES,
        "SEGMENT_SAMPLES": SEGMENT_SAMPLES,
        "WALL_CLOCK_BUDGET_S": WALL_CLOCK_BUDGET_S,
        "FLOOR_RETRY_CAP": FLOOR_RETRY_CAP,
    }
    for _key, entry in sorted(PLATFORM_TABLE.items()):
        prefix = entry.key
        constants[f"{prefix}.median_ceiling_fast_ms"] = (
            entry.median_ceiling_fast_ms
        )
        constants[f"{prefix}.p90_ceiling_fast_ms"] = entry.p90_ceiling_fast_ms
        constants[f"{prefix}.median_ceiling_fallback_ms"] = (
            entry.median_ceiling_fallback_ms
        )
        constants[f"{prefix}.p90_ceiling_fallback_ms"] = (
            entry.p90_ceiling_fallback_ms
        )
        constants[f"{prefix}.bash_floor_ms"] = entry.bash_floor_ms
        constants[f"{prefix}.true_floor_ms"] = entry.true_floor_ms
    return constants


# --- Artefact manifest ----------------------------------------------------


@unique
class ArtefactKind(StrEnum):
    """Every throwaway the harness creates, as one enumerated list.

    Both teardown phases drive from this enumeration rather than from a
    directory listing, so exhaustiveness is a property of the enumeration and
    not of where the artefacts happen to live.
    """

    SCRATCH_TREE = "scratch-tree"
    FIXTURE_ROOT = "fixture-root"
    FAST_FARM = "fast-farm"
    FALLBACK_FARM = "fallback-farm"
    FLOOR_SCRIPT = "floor-script"
    SESSION_MARKER = "session-marker"


MANIFEST_DIRNAME = ".accelerator-measure"
MANIFEST_NAME = "manifest.json"
GUARDED_PATHS = ("keys/", "bin/", "hooks/", "scripts/", "cli/")
GUARDED_FILES = (
    "bin/accelerator",
    "scripts/vcs-common.sh",
    "hooks/hooks.json",
)
CACHE_TEMP_PREFIX = ".tmp-"


class StaleManifestError(RuntimeError):
    """A manifest from an unclean prior run is still on disk."""


class PreconditionFailureError(RuntimeError):
    """A pre-sampling assertion failed; no figures are produced."""


@dataclass
class Baseline:
    release_key_digest: str
    cached: dict[str, ArtefactState | None]
    cache_root_entries: list[str]
    unverified_log: str | None
    repo_diff: str
    guarded_file_digests: dict[str, str]
    dev_launcher_marker: bool
    temp_root: str


@dataclass
class Manifest:
    plugin_root: str
    cache_root: str
    artefacts: dict[str, str] = field(default_factory=dict)
    baseline: dict[str, object] | None = None

    def write(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(asdict(self), indent=2, sort_keys=True))

    @classmethod
    def load(cls, path: Path) -> Manifest:
        payload = json.loads(path.read_text())
        return cls(**payload)


# --- Ports ----------------------------------------------------------------


@dataclass(frozen=True)
class RunResult:
    stdout: str
    stderr: str
    exit_code: int
    elapsed_ms: float


class MeasurementRunner(Protocol):
    """Runs a timed sample under the pinned farm environment."""

    def __call__(
        self, argv: Sequence[str], *, cwd: Path, env: Mapping[str, str]
    ) -> RunResult: ...


class DiagnosticRunner(Protocol):
    """Runs an untimed probe under the ambient environment.

    Separate from the measurement runner for a concrete reason: the farm holds
    exactly the two variants' tools, and `pmset` is not among them — so routing
    power probes through the measurement runner would return `unknown` on every
    run by construction.
    """

    def __call__(self, argv: Sequence[str]) -> str: ...


class ArtefactWitness(Protocol):
    def state(self, path: Path) -> ArtefactState | None: ...
    def entries(self, path: Path) -> list[str]: ...
    def read_text(self, path: Path) -> str | None: ...
    def remove(self, path: Path) -> None: ...


class HostProbe(Protocol):
    def env(self) -> Mapping[str, str]: ...
    def loadavg(self) -> tuple[float, float, float]: ...
    def cpu_probes(self) -> CpuProbes: ...
    def temp_root(self) -> Path: ...


def _digest(path: Path) -> str:
    import hashlib

    return hashlib.sha256(path.read_bytes()).hexdigest()


class FilesystemWitness:
    def state(self, path: Path) -> ArtefactState | None:
        try:
            stat = path.stat()
        except OSError:
            return None
        return ArtefactState(
            inode=stat.st_ino, mtime=stat.st_mtime, digest=_digest(path)
        )

    def entries(self, path: Path) -> list[str]:
        try:
            return sorted(entry.name for entry in path.iterdir())
        except OSError:
            return []

    def read_text(self, path: Path) -> str | None:
        try:
            return path.read_text()
        except OSError:
            return None

    def remove(self, path: Path) -> None:
        if path.is_dir() and not path.is_symlink():
            shutil.rmtree(path, ignore_errors=True)
        else:
            with suppress(OSError):
                path.unlink()


class SystemHostProbe:
    def env(self) -> Mapping[str, str]:
        return dict(os.environ)

    def loadavg(self) -> tuple[float, float, float]:
        return os.getloadavg()

    def cpu_probes(self) -> CpuProbes:
        return CpuProbes(
            cgroup_cpu_max=_cgroup_cpu_max(),
            process_cpu_count=os.process_cpu_count() or 1,
        )

    def temp_root(self) -> Path:
        return Path(tempfile.gettempdir()).resolve()


def _cgroup_cpu_max() -> str | None:
    """Cgroup v2's `cpu.max` for this process's own leaf, or `None`.

    v1 is explicitly out of scope: its per-controller hierarchy needs a
    different traversal, and no lane this harness runs on uses it.
    """
    try:
        leaf = Path("/proc/self/cgroup").read_text().strip().split(":")[-1]
        return (
            (Path("/sys/fs/cgroup") / leaf.lstrip("/") / "cpu.max")
            .read_text()
            .strip()
        )
    except OSError:
        return None


def subprocess_measurement_runner(
    argv: Sequence[str], *, cwd: Path, env: Mapping[str, str]
) -> RunResult:
    """One `perf_counter`-bracketed dispatch.

    The clock is read in this process rather than the child's: a per-call
    interpreter startup inside the measured interval would dwarf the margin
    under test. Everything else — the envelope normalisation, the validity
    comparison, the inode witness — runs outside the bracket.
    """
    started = time.perf_counter()
    completed = subprocess.run(
        list(argv),
        check=False,
        capture_output=True,
        text=True,
        input=STDIN_ENVELOPE,
        cwd=cwd,
        env=dict(env),
    )
    elapsed_ms = (time.perf_counter() - started) * 1000.0
    return RunResult(
        stdout=completed.stdout,
        stderr=completed.stderr,
        exit_code=completed.returncode,
        elapsed_ms=elapsed_ms,
    )


def raw_diagnostic_runner(argv: Sequence[str]) -> str:
    """Like the ambient runner, but returns stdout **unstripped**.

    File recovery compares the recovered bytes against a recorded digest, so a
    stripped trailing newline is the difference between an intact contract and
    a reported rot.
    """
    resolved = shutil.which(argv[0])
    if resolved is None:
        raise FileNotFoundError(argv[0])
    completed = subprocess.run(
        [resolved, *argv[1:]], check=True, capture_output=True, text=True
    )
    return completed.stdout


def ambient_diagnostic_runner(argv: Sequence[str]) -> str:
    resolved = shutil.which(argv[0])
    if resolved is None:
        raise FileNotFoundError(argv[0])
    completed = subprocess.run(
        [resolved, *argv[1:]], check=False, capture_output=True, text=True
    )
    return completed.stdout.strip()


# --- The session ----------------------------------------------------------

_UNWIND_SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)


def unwind_signals() -> tuple[signal.Signals, ...]:
    """List the signals the session unwinds on rather than dying under.

    SIGHUP is not decoration: a closed terminal or a dropped ssh across a
    multi-minute unattended run would otherwise terminate by default and bypass
    the context manager entirely, leaving every artefact behind.
    """
    return _UNWIND_SIGNALS


class MeasurementSession:
    """Captures baseline state on entry, restores and verifies on exit.

    The manifest is written **first, before anything is created**, so a SIGKILL
    or a power loss leaves a findable record: `mise run measure:teardown`
    replays restore and verify from it, and the start-up refusal stops the next
    run adopting the residue as its baseline.
    """

    def __init__(
        self,
        plugin_root: Path,
        *,
        witness: ArtefactWitness | None = None,
        diagnostics: DiagnosticRunner | None = None,
        host: HostProbe | None = None,
    ) -> None:
        self.plugin_root = plugin_root
        self.witness = witness or FilesystemWitness()
        self.diagnostics = diagnostics or ambient_diagnostic_runner
        self.host = host or SystemHostProbe()
        self.cache_root = plugin_root / "bin"
        self.manifest_path = plugin_root / MANIFEST_DIRNAME / MANIFEST_NAME
        self.manifest = Manifest(
            plugin_root=str(plugin_root), cache_root=str(self.cache_root)
        )
        self.baseline: Baseline | None = None
        self.failures: list[str] = []
        self._previous_handlers: dict[int, object] = {}

    # -- entry --

    def __enter__(self) -> Self:
        if self.manifest_path.exists():
            raise StaleManifestError(
                f"a manifest from an unclean prior run exists at "
                f"{self.manifest_path} — run `mise run measure:teardown` "
                f"before sampling again"
            )
        self.baseline = self.capture()
        self.manifest.baseline = asdict(self.baseline)
        self.manifest.write(self.manifest_path)
        self._install_handlers()
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> bool:
        self._restore_handlers()
        self.restore()
        self.failures = self.verify()
        with suppress(OSError):
            self.manifest_path.unlink()
        with suppress(OSError):
            self.manifest_path.parent.rmdir()
        return False

    def _install_handlers(self) -> None:
        for number in _UNWIND_SIGNALS:
            self._previous_handlers[number] = signal.getsignal(number)
            signal.signal(number, self._unwind)

    def _restore_handlers(self) -> None:
        for number, previous in self._previous_handlers.items():
            with suppress(ValueError, TypeError):
                signal.signal(number, previous)  # type: ignore[arg-type]

    def _unwind(self, number: int, frame: FrameType | None) -> None:
        """Turn a signal into an exception so the `with` block unwinds.

        SIGHUP matters as much as SIGINT: a closed terminal across a
        multi-minute unattended run would otherwise terminate by default and
        bypass the context manager entirely.
        """
        del frame
        raise KeyboardInterrupt(f"signal {number}")

    def capture(self) -> Baseline:
        """Record the state every exit assertion is measured against.

        Two of these refuse the run outright rather than being reported at the
        end: a substituted verification key, and a non-empty diff over the
        paths the two variants are built from — ordinary mid-stack jj state
        there invalidates the measurement from sample one.
        """
        key = self.plugin_root / "keys/accelerator-release.pub"
        key_digest = _digest(key)
        if key_digest != RELEASE_KEY_SHA256:
            raise PreconditionFailureError(
                f"the working-copy release key does not match its published "
                f"digest: {key_digest} != {RELEASE_KEY_SHA256}"
            )
        diff = self.diagnostics(["jj", "diff", "--summary", *GUARDED_PATHS])
        if diff.strip():
            raise PreconditionFailureError(
                "uncommitted changes under "
                f"{' '.join(GUARDED_PATHS)} invalidate the measurement from "
                f"sample one:\n{diff}"
            )
        return Baseline(
            release_key_digest=key_digest,
            cached={
                name: self.witness.state(self.cache_root / name)
                for name in self._cached_names()
            },
            cache_root_entries=self.witness.entries(self.cache_root),
            unverified_log=self.witness.read_text(self._unverified_log()),
            repo_diff=diff,
            guarded_file_digests={
                name: _digest(self.plugin_root / name) for name in GUARDED_FILES
            },
            dev_launcher_marker=(
                self.plugin_root / ".accelerator-dev-launcher"
            ).exists(),
            temp_root=str(self.host.temp_root()),
        )

    def _cached_names(self) -> list[str]:
        """List the cache entries a warm dispatch must not move.

        The cached sub-binary asset as well as the launcher, its `.minisig` and
        the staged verify shim: restricted to the launcher, the witness would
        let a re-fetched or re-staged *sub-binary* inflate a sample undetected,
        and that is the entry `cache::find` resolves on every dispatch.
        """
        return [
            name
            for name in self.witness.entries(self.cache_root)
            if name.startswith(_CACHED_ENTRY_PREFIXES)
        ]

    def _unverified_log(self) -> Path:
        return self.cache_root / ".accelerator-unverified.log"

    # -- artefacts --

    def register_artefact(self, kind: ArtefactKind, path: Path) -> Path:
        """Record an artefact before creating it, and return its path.

        Every creating call site goes through this seam, and a unit test
        asserts the seam covers every kind — so the manifest's exhaustiveness
        does not depend on anyone remembering to add a row.
        """
        resolved = Path(path).resolve()
        self.manifest.artefacts[str(kind)] = str(resolved)
        self.manifest.write(self.manifest_path)
        return resolved

    # -- exit --

    def restore(self) -> None:
        """Remove every recorded artefact, containment-checked, then stop.

        Never resolves symlinks and never runs a recursive delete outside the
        three admitted roots. Idempotent, so `measure:teardown` can replay it.
        """
        for kind, recorded in sorted(self.manifest.artefacts.items()):
            path = Path(recorded)
            if not self._contained(path):
                self.failures.append(
                    f"{kind}: {path} is outside every admitted root — "
                    f"refusing to remove it"
                )
                continue
            self.witness.remove(path)

    def _contained(self, path: Path) -> bool:
        """Three admitted roots, and no others.

        The harness's temp parent; the gitignored `bin/.tmp-*` namespace, whose
        prefix `store::TEMP_PREFIX` reserves; and the gitignored manifest
        directory. The two in-repository allowances are tested first and
        everything else under the plugin root is then refused outright, so a
        temp root that happens to be an ancestor of the checkout cannot admit a
        tracked path by transitivity.
        """
        if path.name.startswith(CACHE_TEMP_PREFIX) and path.parent == (
            self.cache_root
        ):
            return True
        if path.parent == self.plugin_root / MANIFEST_DIRNAME:
            return True
        if path == self.plugin_root or tmp_containment(path, self.plugin_root):
            return False
        temp_root = Path(
            self.baseline.temp_root if self.baseline else self.host.temp_root()
        )
        return tmp_containment(path, temp_root)

    def verify(self) -> list[str]:
        """Aggregate every exit assertion; never exit on the first failure.

        Any failure here selects branch 5 of the taxonomy, so a cleanup failure
        blocks the outcome-keyed closure guard rather than being recorded as a
        documented fact beneath a passing verdict.
        """
        problems = list(self.failures)
        problems += self._verify_artefacts_absent()
        problems += self._verify_cache_root()
        problems += self._verify_integrity()
        problems += self._verify_environment()
        return problems

    def _verify_artefacts_absent(self) -> list[str]:
        return [
            f"{kind}: still present at {recorded}"
            for kind, recorded in sorted(self.manifest.artefacts.items())
            if Path(recorded).exists()
        ]

    def _verify_cache_root(self) -> list[str]:
        if self.baseline is None:
            return []
        now = self.witness.entries(self.cache_root)
        appeared = sorted(set(now) - set(self.baseline.cache_root_entries))
        problems = []
        for entry in appeared:
            if entry.startswith(CACHE_TEMP_PREFIX):
                problems.append(
                    f"leaked temp entry in the cache root: {entry} — remove "
                    f"with `rm -rf {self.cache_root / entry}`"
                )
            elif entry.startswith(".accelerator-lock-"):
                problems.append(
                    f"orphaned lock directory: {entry} — remove with "
                    f"`rmdir {self.cache_root / entry}`"
                )
            else:
                problems.append(f"new cache-root entry: {entry}")
        return problems

    def _verify_integrity(self) -> list[str]:
        if self.baseline is None:
            return []
        problems = []
        key_digest = _digest(self.plugin_root / "keys/accelerator-release.pub")
        if key_digest != RELEASE_KEY_SHA256:
            problems.append(
                f"the release key changed during the run: {key_digest}"
            )
        problems += unchanged_artefacts(
            self.baseline.cached,
            {
                name: self.witness.state(self.cache_root / name)
                for name in self.baseline.cached
            },
        )
        problems += self._verify_unverified_log()
        diff = self.diagnostics(["jj", "diff", "--summary", *GUARDED_PATHS])
        if diff != self.baseline.repo_diff:
            problems.append(
                f"the guarded paths' diff changed during the run:\n{diff}"
            )
        for name, digest in self.baseline.guarded_file_digests.items():
            if _digest(self.plugin_root / name) != digest:
                problems.append(f"{name} changed during the run")
        return problems

    def _verify_unverified_log(self) -> list[str]:
        """Treat the unverified log as append-only.

        Any appended line is written by `fail_integrity` or the dev-override
        exec, so growth *is* a trust-chain failure or an engaged override —
        it invalidates the session rather than being attributed and tidied.
        Attribution was never reliable: the record carries the bootstrap
        subprocess's pid, and this repo has already been bitten by pid reuse.
        """
        if self.baseline is None:
            return []
        after = self.witness.read_text(self._unverified_log())
        before = self.baseline.unverified_log
        if before is None and after is not None:
            return [
                "the unverified log was created during the run — a "
                "trust-chain failure or an engaged override"
            ]
        if before is not None and after is None:
            return ["the unverified log disappeared during the run"]
        if before is None or after is None:
            return []
        try:
            appended = log_appended_lines(before, after)
        except ValueError as error:
            return [str(error)]
        return [f"the unverified log grew: {line}" for line in appended[:1]]

    def _verify_environment(self) -> list[str]:
        if self.baseline is None:
            return []
        problems = []
        offenders = accelerator_override_keys(self.host.env())
        if offenders:
            problems.append(
                f"ACCELERATOR_* overrides present at exit: {offenders}"
            )
        marker = (self.plugin_root / ".accelerator-dev-launcher").exists()
        if marker != self.baseline.dev_launcher_marker:
            problems.append(
                "the operator's dev-launcher marker state changed during the "
                "run"
            )
        return problems


# --- Cells ----------------------------------------------------------------


@dataclass(frozen=True)
class Cell:
    name: str
    kind: CellKind
    gates: bool
    threshold: float
    target: float
    description: str


def cells_for(entry: PlatformEntry) -> tuple[Cell, ...]:
    """Build the six cells, in the order 0189's Latency Criterion lists."""
    return (
        Cell(
            "C1",
            CellKind.ABSOLUTE,
            gates=True,
            threshold=entry.median_ceiling_fast_ms,
            target=MEDIAN_TARGET_MS,
            description="median(G), fast backend",
        ),
        Cell(
            "C2",
            CellKind.ABSOLUTE,
            gates=True,
            threshold=entry.p90_ceiling_fast_ms,
            target=P90_TARGET_MS,
            description="p90(G), fast backend",
        ),
        Cell(
            "C3",
            CellKind.ABSOLUTE,
            gates=True,
            threshold=entry.median_ceiling_fallback_ms,
            target=MEDIAN_TARGET_MS,
            description="median(G), fallback backend",
        ),
        Cell(
            "C4",
            CellKind.ABSOLUTE,
            gates=True,
            threshold=entry.p90_ceiling_fallback_ms,
            target=P90_TARGET_MS,
            description="p90(G), fallback backend",
        ),
        Cell(
            "C5",
            CellKind.RATIO,
            gates=True,
            threshold=RATIO_THRESHOLD,
            target=RATIO_TARGET,
            description="median(G)/median(B), fast backend",
        ),
        Cell(
            "C6",
            CellKind.RATIO,
            gates=False,
            threshold=RATIO_THRESHOLD,
            target=RATIO_TARGET,
            description="median(G)/median(B), fallback backend (ungated)",
        ),
    )


def classify_cell(
    cell: Cell,
    interval: Interval | None,
    *,
    robustness_ok: bool | None,
    escalations_used: int,
    validity: Validity,
    sizing_feasible: bool,
    applicable: bool,
    budget_spent: bool,
) -> CellOutcome:
    """Classify one cell, or mark it not applicable when it has no interval."""
    if interval is None or not applicable:
        return CellOutcome(
            cell.name, gates=cell.gates, branch=Branch.NOT_APPLICABLE
        )
    branch = classify(
        cell_kind=cell.kind,
        lower=interval.lower,
        upper=interval.upper,
        threshold=cell.threshold,
        upper_distance=interval.upper_distance,
        target_distance=cell.target,
        robustness_ok=robustness_ok,
        escalations_used=escalations_used,
        validity=validity,
        sizing_feasible=sizing_feasible,
        applicable=True,
        budget_exhausted=budget_spent,
    )
    return CellOutcome(cell.name, gates=cell.gates, branch=branch)


# --- Fixture and farms ----------------------------------------------------


def create_fixture(root: Path, *, runner: DiagnosticRunner) -> Path:
    """Create a pure-jj fixture, colocation pinned off and asserted.

    `jj git init` colocates by default at 0.43, and a colocated fixture emits
    **warn** rather than the blocked decision — so the harness would silently
    measure the wrong path. The assertion is on the resulting tree, not on the
    flag, because the flag's meaning is what a version bump could change.
    """
    root.mkdir(parents=True, exist_ok=True)
    runner(
        [
            "jj",
            "--config",
            "git.colocate=false",
            "git",
            "init",
            "--quiet",
            str(root),
        ]
    )
    if not (root / ".jj").is_dir():
        raise PreconditionFailureError(f"{root} is not a jj repository")
    if (root / ".git").exists():
        raise PreconditionFailureError(
            f"{root} is colocated — it would emit warn, not the blocked "
            f"decision, and the session would measure the wrong path"
        )
    return root


def build_farm(
    root: Path, tools: Sequence[str], *, include_fast_backend: bool
) -> Path:
    """Build a symlink farm over the union of both variants' tools.

    Enumerating from the baseline alone would break the variant being gated:
    `G` spawns `uname`, `sed`, `awk` and possibly `curl`, none of which `B`
    uses, and under `--fail-safe` each absence exits **0** — the degraded shape
    that records a spuriously low latency. Links resolve to the concrete
    binary, never a wrapper or a mise shim, because a shim re-resolves its
    version from the config discovered at the cwd and the sampling cwd is the
    fixture.
    """
    root.mkdir(parents=True, exist_ok=True)
    missing: list[str] = []
    for tool in tools:
        if tool == FAST_BACKEND and not include_fast_backend:
            continue
        resolved = shutil.which(tool)
        if resolved is None:
            missing.append(tool)
            continue
        link = root / tool
        if not link.exists():
            link.symlink_to(Path(resolved).resolve())
    if missing:
        raise PreconditionFailureError(
            f"tools absent from PATH, so the farm cannot be built: {missing}"
        )
    return root


def farm_environment(farm: Path, *, temp_root: Path) -> dict[str, str]:
    """Build the exact environment a sample is taken under.

    `LC_ALL=C` rather than `LANG` alone: `B` is dominated by `grep`/`sed`/`awk`
    spawns whose speed varies materially between `C` and a UTF-8 locale, an
    uncontrolled multiplier on the denominator.
    """
    return {
        "PATH": str(farm),
        "LC_ALL": "C",
        "LANG": "C",
        "TZ": "UTC",
        "HOME": str(temp_root),
        "GIT_CEILING_DIRECTORIES": ceiling_directories(temp_root),
    }


RECOVERED_FILES = {
    "bin/vcs-guard": (
        "hooks/vcs-guard.sh",
        "3dc02b9ab1c5b6865c4f17116a558bceb383a517f2769de269366c45974ac2bb",
    ),
    "scripts/vcs-common.sh": (
        "scripts/vcs-common.sh",
        "e929f943726134b254a5539ccd673f3f2b154e9ed3093a60ba131e52f04a924a",
    ),
}


def recovery_argv(source: str, *, engine: str) -> list[str]:
    """Build the command that reads `source` at the pinned revision.

    Two engines because two contexts: a jj workspace resolves the revset, while
    a GitHub checkout has no `.jj` at all and needs the resolved commit id
    through git — and `actions/checkout` defaults to a shallow fetch, so the
    owning lane must set `fetch-depth: 0` for the git form to resolve.
    """
    if engine == "jj":
        return ["jj", "file", "show", "-r", BASELINE_REVSET, source]
    return ["git", "show", f"{BASELINE_COMMIT}:{source}"]


def recover_baseline(
    scratch: Path, *, runner: DiagnosticRunner, engine: str = "jj"
) -> Path:
    """Recover the deleted shell guard and its one dependency into `scratch`.

    Both at the same revision, so the subject is self-contained: pinning only
    the guard would leave it resolving a mutable `scripts/vcs-common.sh`, which
    a separate work item is scoped to change. The layout puts them where the
    guard's own `"$SCRIPT_DIR/../scripts/vcs-common.sh"` resolves within
    `scratch`, and nothing is written inside the repository — a staged copy in
    `bin/` would park an unreviewed executable where the launcher execs from and
    add an entry to the directory whose entry set is an integrity witness.

    Each recovered file's digest is compared against its recorded provenance:
    without that the recovery is verifiable only inside this jj workspace, and
    the recorded `B` could drift silently under a rewritten history.
    """
    guard = scratch / "bin/vcs-guard"
    for relative, (source, expected) in RECOVERED_FILES.items():
        destination = scratch / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(runner(recovery_argv(source, engine=engine)))
        digest = _digest(destination)
        if digest != expected:
            raise PreconditionFailureError(
                f"{source} at {BASELINE_COMMIT} hashes to {digest}, not its "
                f"recorded {expected} — the recovery contract has rotted"
            )
    guard.chmod(0o755)
    return guard


# --- Tasks ----------------------------------------------------------------


def _entry_for(platform_key: tuple[str, str]) -> PlatformEntry | None:
    return platform_constants(platform_key, PLATFORM_TABLE)


@task(name="warm-dispatch")
def warm_dispatch(
    context: Context, platform_key: str | None = None, blocks: str = "AB"
) -> None:
    """Measure warm-dispatch latency against the shell baseline.

    Needs a quiet host, a pinned `PATH`, network egress and several minutes.
    """
    del context
    key, source = resolve_platform_key(
        option=platform_key, env=dict(os.environ)
    )
    entry = _entry_for(key)
    print(f"platform key {key} from {source}")
    if entry is None:
        print(
            "no calibrated entry for this platform key — figures will be "
            "recorded as uncalibrated context, never a verdict (branch 7)"
        )
    run_session(REPO_ROOT, entry=entry, blocks=blocks)


@task
def teardown(context: Context) -> None:
    """Replay restore and verify from a stale manifest."""
    del context
    manifest_path = REPO_ROOT / MANIFEST_DIRNAME / MANIFEST_NAME
    if not manifest_path.exists():
        print(f"no manifest at {manifest_path} — nothing to tear down")
        return
    session = MeasurementSession(REPO_ROOT)
    session.manifest = Manifest.load(manifest_path)
    if session.manifest.baseline is not None:
        session.baseline = Baseline(**session.manifest.baseline)  # type: ignore[arg-type]
    session.restore()
    problems = session.verify()
    with suppress(OSError):
        manifest_path.unlink()
    with suppress(OSError):
        manifest_path.parent.rmdir()
    if problems:
        raise Exit("teardown verify failed:\n" + "\n".join(problems), code=1)
    print("teardown complete: every recorded artefact is absent")


@task(name="smoke-check")
def smoke_check(
    context: Context, engine: str = "jj", live: bool = True
) -> None:
    """Live-dispatch smoke check: n = 2, floors only, no gating figure.

    Owns the `measure:*` namespace against rot — the module depends on volatile
    external contracts (the digest-backend selection, the cache-root
    derivation, the hook envelope shape, `jj`'s colocation default and a revset
    anchoring two deleted files), and a module no automated path ever executes
    rots invisibly. Emits no gating figure, by construction: the ceilings are
    calibrated for a quiet darwin-arm64 host and the instrument-floor gate is
    one no shared runner reliably clears.
    """
    del context
    print(smoke_report(REPO_ROOT, engine=engine, live=live))


def digest_backend_population() -> dict[str, str | None]:
    """Which digest backend this host resolves, recorded rather than assumed.

    The fast backend is absent from a stock macOS image and universal on linux,
    so which cells a lane could ever enforce is a property of the runner. Costs
    nothing to record and turns an assumption into a known number.
    """
    return {
        FAST_BACKEND: shutil.which(FAST_BACKEND),
        FALLBACK_BACKEND: shutil.which(FALLBACK_BACKEND),
    }


def smoke_report(
    plugin_root: Path,
    *,
    engine: str = "jj",
    live: bool = True,
    runner: MeasurementRunner = subprocess_measurement_runner,
) -> str:
    """Exercise every volatile contract the harness rests on, and report.

    Fails on a rotted contract — a moved revision, a colocating `jj git init`,
    an unbuildable farm, a baseline that no longer emits a decision. Reports an
    absent published release for the tree's own version as an **unmet
    prerequisite** rather than a failure: the tree is routinely ahead of the
    last release cut, and a lane that reddens on that would be red by default.
    """
    lines = [f"digest backends: {digest_backend_population()}"]
    with MeasurementSession(plugin_root) as session:
        if session.baseline is None:
            raise PreconditionFailureError("the session captured no baseline")
        temp_parent = Path(
            tempfile.mkdtemp(
                prefix="accelerator-measure-", dir=session.baseline.temp_root
            )
        )
        scratch = session.register_artefact(
            ArtefactKind.SCRATCH_TREE, temp_parent / "baseline"
        )
        fixture = session.register_artefact(
            ArtefactKind.FIXTURE_ROOT, temp_parent / "fixture"
        )
        fast_farm = session.register_artefact(
            ArtefactKind.FAST_FARM, temp_parent / "farm-fast"
        )
        fallback_farm = session.register_artefact(
            ArtefactKind.FALLBACK_FARM, temp_parent / "farm-fallback"
        )
        floor_script = session.register_artefact(
            ArtefactKind.FLOOR_SCRIPT, temp_parent / "floor.sh"
        )
        marker = session.register_artefact(
            ArtefactKind.SESSION_MARKER, temp_parent / "session.marker"
        )

        guard = recover_baseline(
            scratch, runner=raw_diagnostic_runner, engine=engine
        )
        lines.append(
            f"recovery contract intact at {BASELINE_COMMIT} via {engine}"
        )
        create_fixture(fixture, runner=session.diagnostics)
        lines.append("fixture is pure-jj, not colocated")

        resolvable = tuple(
            tool for tool in _smoke_tools() if shutil.which(tool) is not None
        )
        build_farm(fast_farm, resolvable, include_fast_backend=True)
        build_farm(fallback_farm, resolvable, include_fast_backend=False)
        lines.append(f"both farms built over {len(resolvable)} tools")
        floor_script.write_text("#!/usr/bin/env bash\nexit 0\n")
        floor_script.chmod(0o755)
        marker.write_text(f"pid={os.getpid()}\n")

        environment = farm_environment(fast_farm, temp_root=temp_parent)
        probe = runner([str(guard)], cwd=fixture, env=environment)
        decision, reason = expected_decision(probe.stdout)
        if decision is not Decision.BLOCK:
            raise PreconditionFailureError(
                f"the recovered baseline emits {decision}, not the blocked "
                f"decision — the fixture or the guard has changed"
            )
        lines.append(f"baseline blocks: {reason[:60]!r}")

        if live:
            lines.append(
                _live_dispatch(
                    plugin_root, runner, fixture, environment, reason
                )
            )
    if session.failures:
        raise Exit(
            "teardown verify failed:\n" + "\n".join(session.failures), code=1
        )
    lines.append("every registered artefact is absent; no gating figure taken")
    return "\n".join(lines)


def _smoke_tools() -> tuple[str, ...]:
    entry = _entry_for(("Darwin", "arm64"))
    return entry.path_tools if entry else ("bash", "jj", "true")


def _live_dispatch(
    plugin_root: Path,
    runner: MeasurementRunner,
    fixture: Path,
    environment: Mapping[str, str],
    expected_reason: str,
) -> str:
    argv = [
        str(plugin_root / "bin/accelerator"),
        "vcs",
        "guard",
        "--format=hook",
        "--fail-safe",
    ]
    # One discarded warm-up, so the launcher takes the cache-hit branch and a
    # still-degraded sample means an unmet prerequisite rather than a cold
    # cache. Without it the first dispatch's fetch is indistinguishable from an
    # absent release.
    runner(argv, cwd=fixture, env=environment)
    results = [
        runner(argv, cwd=fixture, env=environment) for _ in range(SMOKE_N)
    ]
    verdict = validate_sample(
        results[0].stdout, results[-1].stdout, expected_reason
    )
    if verdict.valid:
        elapsed = ", ".join(f"{r.elapsed_ms:.1f} ms" for r in results)
        return f"live dispatch blocked on {SMOKE_N} samples ({elapsed})"
    return (
        "unmet prerequisite: the bootstrap produced no decision after a "
        f"discarded warm-up ({verdict.diagnostic}). Either no signed release "
        "is published for the tree's own version, or the release base URL is "
        "unreachable — recorded rather than failed, because the tree is "
        "routinely ahead of the last cut and --fail-safe exits 0 either way."
    )


def run_session(
    plugin_root: Path,
    *,
    entry: PlatformEntry | None,
    blocks: str = "AB",
    smoke: bool = False,
    runner: MeasurementRunner = subprocess_measurement_runner,
) -> list[CellOutcome]:
    """Drive one recorded session end to end.

    Returns the classified cells. Every artefact it creates is registered
    before creation, and the context manager restores and verifies on every
    exit path including the three unwinding signals.
    """
    tools = entry.path_tools if entry else ("bash", "jj", "true")
    with MeasurementSession(plugin_root) as session:
        if session.baseline is None:
            raise PreconditionFailureError("the session captured no baseline")
        temp_parent = Path(
            tempfile.mkdtemp(
                prefix="accelerator-measure-", dir=session.baseline.temp_root
            )
        )
        scratch = session.register_artefact(
            ArtefactKind.SCRATCH_TREE, temp_parent / "baseline"
        )
        fixture = session.register_artefact(
            ArtefactKind.FIXTURE_ROOT, temp_parent / "fixture"
        )
        fast_farm = session.register_artefact(
            ArtefactKind.FAST_FARM, temp_parent / "farm-fast"
        )
        fallback_farm = session.register_artefact(
            ArtefactKind.FALLBACK_FARM, temp_parent / "farm-fallback"
        )
        floor_script = session.register_artefact(
            ArtefactKind.FLOOR_SCRIPT, temp_parent / "floor.sh"
        )
        marker = session.register_artefact(
            ArtefactKind.SESSION_MARKER, temp_parent / "session.marker"
        )

        guard = recover_baseline(scratch, runner=raw_diagnostic_runner)
        create_fixture(fixture, runner=session.diagnostics)
        build_farm(fast_farm, tools, include_fast_backend=True)
        build_farm(fallback_farm, tools, include_fast_backend=False)
        floor_script.write_text("#!/usr/bin/env bash\nexit 0\n")
        floor_script.chmod(0o755)
        marker.write_text(f"pid={os.getpid()}\n")

        launcher = plugin_root / "bin/accelerator"
        variants = {
            Variant.BASELINE: [str(guard)],
            Variant.FAST: [
                str(launcher),
                "vcs",
                "guard",
                "--format=hook",
                "--fail-safe",
            ],
        }
        variants[Variant.FALLBACK] = variants[Variant.FAST]
        environments = {
            Variant.BASELINE: farm_environment(
                fast_farm, temp_root=temp_parent
            ),
            Variant.FAST: farm_environment(fast_farm, temp_root=temp_parent),
            Variant.FALLBACK: farm_environment(
                fallback_farm, temp_root=temp_parent
            ),
        }
        assert_backends(fast_farm, fallback_farm)

        probe = runner(
            variants[Variant.BASELINE],
            cwd=fixture,
            env=environments[Variant.BASELINE],
        )
        expected, expected_reason = expected_decision(probe.stdout)
        print(f"baseline probe: {expected} / {expected_reason!r}")
        if expected is not Decision.BLOCK:
            raise PreconditionFailureError(
                f"the fixture does not emit the blocked decision (got "
                f"{expected}) — the session would measure the wrong path"
            )

        samples = _sample(
            runner,
            variants,
            environments,
            fixture=fixture,
            expected_reason=expected_reason,
            block_a_pairs=2 if smoke else BLOCK_A_PAIRS,
            block_b_samples=2 if smoke else BLOCK_B_SAMPLES,
            pilot_pairs=0 if smoke else PILOT_PAIRS,
            pilot_samples=0 if smoke else PILOT_SAMPLES,
            blocks=blocks,
        )
        outcomes = _analyse(entry, samples, smoke=smoke)
    if session.failures:
        raise Exit(
            "teardown verify failed — the session is invalidated (branch 5):\n"
            + "\n".join(session.failures),
            code=1,
        )
    return outcomes


def assert_backends(fast_farm: Path, fallback_farm: Path) -> None:
    """Assert both farms in both directions before sampling either block.

    A fast farm missing its `sha256sum` link would silently measure the
    fallback backend under the cells carrying C1, C2 and C5.
    """
    if not (fast_farm / FAST_BACKEND).exists():
        raise PreconditionFailureError(
            f"the fast farm resolves no {FAST_BACKEND} — C1, C2 and C5 would "
            f"be measured against the fallback backend"
        )
    if (fallback_farm / FAST_BACKEND).exists():
        raise PreconditionFailureError(
            f"the fallback farm resolves {FAST_BACKEND} — it is not the "
            f"fallback backend"
        )
    if not (fallback_farm / FALLBACK_BACKEND).exists():
        raise PreconditionFailureError(
            f"the fallback farm resolves no {FALLBACK_BACKEND}, so C3, C4 and "
            f"C6 are not applicable on this host (branch 7)"
        )


def _sample(
    runner: MeasurementRunner,
    variants: Mapping[Variant, Sequence[str]],
    environments: Mapping[Variant, Mapping[str, str]],
    *,
    fixture: Path,
    expected_reason: str,
    block_a_pairs: int,
    block_b_samples: int,
    pilot_pairs: int,
    pilot_samples: int,
    blocks: str,
) -> dict[Variant, list[float]]:
    schedule = generate_schedule(
        block_a_pairs=block_a_pairs,
        block_b_samples=block_b_samples,
        pilot_pairs=pilot_pairs,
        pilot_samples=pilot_samples,
        segment=SEGMENT_SAMPLES,
        rng=random.Random(SEED),  # noqa: S311 — statistical sampling, not a security context
    )
    schedule = [
        sample
        for sample in schedule
        if not sample.pilot and sample.block in blocks
    ]
    observed: dict[Variant, list[float]] = {variant: [] for variant in Variant}
    pending: dict[Variant, str] = {}
    started = time.perf_counter()
    for index, sample in enumerate(schedule):
        result = runner(
            variants[sample.variant],
            cwd=fixture,
            env=environments[sample.variant],
        )
        arm = observed[sample.variant]
        if arm and outlier_trip(
            result.elapsed_ms,
            arm_median=median(arm),
            arm_count=len(arm),
        ):
            raise Exit(
                f"outlier trip on {sample.variant} at sample {index}: "
                f"{result.elapsed_ms:.2f} ms against a running median of "
                f"{median(arm):.2f} ms over {len(arm)} samples — a re-fetch or "
                f"a noisy host, not a warm dispatch",
                code=1,
            )
        pending[sample.variant] = result.stdout
        if sample.variant is Variant.FALLBACK:
            verdict = validate_sample(
                pending[sample.variant], result.stdout, expected_reason
            )
        elif Variant.BASELINE in pending and Variant.FAST in pending:
            verdict = validate_sample(
                pending.pop(Variant.BASELINE),
                pending.pop(Variant.FAST),
                expected_reason,
            )
        else:
            verdict = None
        if verdict is not None and not verdict.valid:
            raise Exit(
                f"per-sample validity gate failed at sample {index}: "
                f"{verdict.diagnostic}",
                code=1,
            )
        arm.append(result.elapsed_ms)
        if time.perf_counter() - started > WALL_CLOCK_BUDGET_S:
            raise Exit(
                "the wall-clock budget is exhausted mid-run (branch 6b) — "
                "partial figures are non-gating",
                code=1,
            )
    return observed


def _analyse(
    entry: PlatformEntry | None,
    samples: Mapping[Variant, Sequence[float]],
    *,
    smoke: bool,
) -> list[CellOutcome]:
    if entry is None:
        print("uncalibrated platform key — recording context, not a verdict")
        return []
    rng = random.Random(SEED)  # noqa: S311 — statistical resampling, not a security context
    resamples = 10 if smoke else RESAMPLES
    outcomes = []
    fast = list(samples[Variant.FAST])
    baseline = list(samples[Variant.BASELINE])
    fallback = list(samples[Variant.FALLBACK])
    intervals: dict[str, Interval | None] = {
        "C1": _absolute(fast, lambda v: summarise(v).median, resamples, rng),
        "C2": _absolute(fast, lambda v: summarise(v).p90, resamples, rng),
        "C3": _absolute(
            fallback, lambda v: summarise(v).median, resamples, rng
        ),
        "C4": _absolute(fallback, lambda v: summarise(v).p90, resamples, rng),
        "C5": _ratio(baseline, fast, resamples, rng),
        "C6": _ratio(baseline, fallback, resamples, rng),
    }
    for cell in cells_for(entry):
        interval = intervals[cell.name]
        outcome = classify_cell(
            cell,
            interval,
            robustness_ok=None if cell.kind is CellKind.ABSOLUTE else True,
            escalations_used=0,
            validity=Validity.VALID,
            sizing_feasible=True,
            applicable=interval is not None,
            budget_spent=False,
        )
        outcomes.append(outcome)
        if interval is None:
            print(f"{cell.name} ({cell.description}): not applicable")
        else:
            print(
                f"{cell.name} ({cell.description}): {interval.point:.4f} "
                f"[{interval.lower:.4f}, {interval.upper:.4f}] against "
                f"{cell.threshold} -> branch {outcome.branch}"
            )
    if smoke:
        print("smoke check: no gating figure recorded")
    else:
        print(f"closure verdict: {closure_verdict(outcomes)}")
    return outcomes


def _absolute(
    values: Sequence[float],
    statistic: object,
    resamples: int,
    rng: random.Random,
) -> Interval | None:
    if not values:
        return None
    return unpaired_interval(
        values,
        statistic=statistic,  # type: ignore[arg-type]
        resamples=resamples,
        confidence=CONFIDENCE,
        rng=rng,
    )


def _ratio(
    baseline: Sequence[float],
    variant: Sequence[float],
    resamples: int,
    rng: random.Random,
) -> Interval | None:
    if not baseline or len(baseline) != len(variant):
        return None
    return paired_ratio_interval(
        baseline,
        variant,
        resamples=resamples,
        confidence=CONFIDENCE,
        rng=rng,
    )


def quietness(
    host: HostProbe, entry: PlatformEntry | None
) -> dict[str, object]:
    """Record load and the CPU count as two values, never a derived ratio.

    On linux `/proc/loadavg` is host-scoped regardless of cgroup membership, so
    dividing it by a container's quota yields a meaningless number. Load is
    read through `os.getloadavg()`, which exists on both OSes.
    """
    count, rung = resolve_cpu_count(host.cpu_probes())
    return {
        "loadavg": host.loadavg(),
        "cpu_count": count,
        "cpu_count_rung": rung,
        "power": power_state(
            ambient_diagnostic_runner, entry.power_probes if entry else ()
        ),
    }


def floors_hold(
    entry: PlatformEntry,
    *,
    bash_floor_ms: float,
    true_floor_ms: float,
    attempts_used: int,
) -> tuple[bool, bool]:
    """Report whether the instrument floors clear their gate, and may retry.

    A breach is a precondition failure, not a note: the floors are calibrated
    ~10% above the ones the confirmed result's own session implies, so a
    session that cannot reach them is not measuring the same instrument.
    """
    holds = (
        bash_floor_ms <= entry.bash_floor_ms
        and true_floor_ms <= entry.true_floor_ms
    )
    return (holds, retry_budget(attempts_used, cap=FLOOR_RETRY_CAP))


def drift_holds(first_third: float, last_third: float) -> bool:
    return drift_verdict(first_third, last_third, band=DRIFT_BAND)


def budget_closes(
    terms: Sequence[Interval], observed_median: float, attempts_used: int
) -> object:
    return residual_verdict(terms, observed_median, attempts_used)


def calibration_note(
    entry: PlatformEntry, *, chip: str, bash: str, shasum: str
) -> str:
    if calibration_holds(
        entry, observed_chip=chip, observed_bash=bash, observed_shasum=shasum
    ):
        return "calibrated"
    return (
        "uncalibrated for this host: the observed chip, bash or shasum "
        "disagrees with the entry's provenance — figures are context, not a "
        "verdict"
    )


def percentile_of(values: Sequence[float], quantile: float) -> float:
    return percentile(values, quantile)
