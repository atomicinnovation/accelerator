"""Warm-dispatch latency measurement (quiet host; outside `check`/`default`).

Times the dispatched `vcs guard` (`G`) against the shell guard it replaced
(`B`), recovered from history, and classifies the six cells C1-C6.

Prerequisites and the reason the namespace sits outside the aggregate `check`
are in `tasks/README.md`'s `### The measure namespace`.
"""

from __future__ import annotations

import json
import os
import platform
import random
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable, Mapping, Sequence
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
    budget_exhausted,
    ceiling_directories,
    classify,
    closure_verdict,
    dirname_spawn_count,
    drift_band_from_permutation,
    drift_significance,
    drift_statistic,
    drift_verdict,
    expected_decision,
    generate_schedule,
    log_appended_lines,
    median,
    median_of_ratios,
    outlier_trip,
    paired_ratio_interval,
    percentile,
    pilot_sizing,
    platform_constants,
    power_state,
    ratio_of_medians,
    residual_verdict,
    resolve_cpu_count,
    resolve_platform_key,
    retry_budget,
    subtract_floor,
    summarise,
    thirds,
    tmp_containment,
    unchanged_artefacts,
    unconfirmed_calibration_fields,
    unpaired_interval,
    unpaired_ratio_interval,
    validate_dispatch,
    validate_sample,
)
from tasks.shared.paths import REPO_ROOT

# --- Criterion constants --------------------------------------------------
# Held in lockstep with `tasks/README.md`, bidirectionally: every constant
# appears there and every number there resolves to a name here.

RESAMPLES = 10000
CONFIDENCE = 0.95
RATIO_THRESHOLD = 1.4
MEDIAN_TARGET_MS = 1.0
P90_TARGET_MS = 2.0
RATIO_TARGET = 0.0036
RATIO_ESCALATION_TARGET = 0.0018
# Recorded alongside the derived band so a record states both verdicts. Its
# false-positive rate was an unstated ~11%.
SUPERSEDED_DRIFT_BAND = 0.005
# A constant cannot serve: the null narrows with the sample size, so one number
# is too tight at large n and too loose at small n.
DRIFT_QUANTILE = 0.95
DRIFT_PERMUTATIONS = 2000
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
REHEARSAL_N = 8
# Not gate constants: these set how precisely a figure is known, not what it
# is judged against.
FLOOR_SAMPLES = 50
TERM_SAMPLES = 200

# Comparing against a published value detects a key substituted *before* the
# session; recording the digest detects one *during*.
RELEASE_KEY_SHA256 = (
    "0f3fe9a91ab6869ce36209691e06c722259e5754f2228b1539ef566b00f6fb2e"
)

# The commit id is recorded as well as the revset: a short hex prefix can be a
# jj *change* id, and a plain git clone has no `.jj` to resolve a revset.
BASELINE_REVSET = "cf42441e2aad-"
BASELINE_COMMIT = "2cfbf81e2e7b4934e868bd42c69374c335b05317"

STDIN_ENVELOPE = json.dumps(
    {
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "git status"},
    }
)

# `bash` and `shasum` are `None` because the calibrating session did not record
# them. Asserting a value nobody measured would report agreement with a figure
# that does not exist, so their absence demotes this key to uncalibrated context
# until a session records them.
_DARWIN_CALIBRATION = Calibration(
    session="0205",
    chip="Apple M4 Max",
    bash=None,
    shasum=None,
)

PLATFORM_TABLE: dict[tuple[str, str], PlatformEntry] = {
    ("Darwin", "arm64"): PlatformEntry(
        key="darwin-arm64",
        # The union of both variants' spawns plus the two floor binaries,
        # derived rather than transcribed: a missing tool exits 0 under
        # `--fail-safe`, which records a spuriously low latency instead of
        # failing. A test re-derives this set from the scripts themselves.
        path_tools=(
            "awk",
            "bash",
            "cat",
            "chmod",
            "cp",
            "curl",
            "date",
            "dirname",
            "git",
            "grep",
            "head",
            "jj",
            "jq",
            "kill",
            "mkdir",
            "mv",
            "readlink",
            "realpath",
            "rm",
            "rmdir",
            "sed",
            "sha256sum",
            "shasum",
            "sleep",
            "timeout",
            "true",
            "uname",
            "wget",
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

# The two farms differ solely in whether the fast backend's link is present.
_CACHED_ENTRY_PREFIXES = (
    "accelerator-launcher-",
    "accelerator-verify-",
    "vcs-",
)

SHA256_HEX_LENGTH = 64
FAST_BACKEND = "sha256sum"
FALLBACK_BACKEND = "shasum"


def criterion_constants() -> dict[str, float]:
    """Every pre-registered number a run is judged by, by name."""
    constants: dict[str, float] = {
        "RESAMPLES": RESAMPLES,
        "CONFIDENCE": CONFIDENCE,
        "RATIO_THRESHOLD": RATIO_THRESHOLD,
        "MEDIAN_TARGET_MS": MEDIAN_TARGET_MS,
        "P90_TARGET_MS": P90_TARGET_MS,
        "RATIO_TARGET": RATIO_TARGET,
        "RATIO_ESCALATION_TARGET": RATIO_ESCALATION_TARGET,
        "SUPERSEDED_DRIFT_BAND": SUPERSEDED_DRIFT_BAND,
        "DRIFT_QUANTILE": DRIFT_QUANTILE,
        "DRIFT_PERMUTATIONS": DRIFT_PERMUTATIONS,
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
    """Every throwaway the harness creates.

    Teardown drives from this enumeration rather than a directory listing, so
    exhaustiveness does not depend on where the artefacts live.
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
# The tree files a mid-run edit would invalidate the measurement through. The
# baseline is recovered at a pinned revision, so its live counterparts are not
# inputs and are deliberately absent here.
GUARDED_FILES = (
    "bin/accelerator",
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
    guarded_file_digests: dict[str, str | None]
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

    Separate from the measurement runner because the farm holds only the two
    variants' tools: probes routed through it would return `unknown` always.
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


def _digest_or_absent(path: Path) -> str | None:
    """Digest the file, or return `None` when it is not there.

    A guarded file may legitimately be deleted between releases; the harness
    should report the change rather than die reading it.
    """
    try:
        return _digest(path)
    except OSError:
        return None


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

    v1 is out of scope; its per-controller hierarchy needs a different walk.
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

    The clock is read here rather than in the child: an interpreter startup
    inside the measured interval would dwarf the margin under test.
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

    Recovered bytes are compared against a recorded digest, so a stripped
    trailing newline reads as corruption.
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

    SIGHUP included: a dropped ssh across a long unattended run would otherwise
    terminate by default and leave every artefact behind.
    """
    return _UNWIND_SIGNALS


class MeasurementSession:
    """Captures baseline state on entry, restores and verifies on exit.

    The manifest is written before anything is created, so a SIGKILL leaves a
    findable record for `mise run measure:teardown` to replay.
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
        """Turn a signal into an exception so the `with` block unwinds."""
        del frame
        raise KeyboardInterrupt(f"signal {number}")

    def capture(self) -> Baseline:
        """Record the state every exit assertion is measured against.

        A substituted verification key and a dirty diff over the guarded paths
        refuse the run outright: both invalidate it from sample one, so
        reporting them at the end would waste the session.
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
                name: _digest_or_absent(self.plugin_root / name)
                for name in GUARDED_FILES
            },
            dev_launcher_marker=(
                self.plugin_root / ".accelerator-dev-launcher"
            ).exists(),
            temp_root=str(self.host.temp_root()),
        )

    def _cached_names(self) -> list[str]:
        """List the cache entries a warm dispatch must not move.

        The sub-binary is included: it is what a dispatch resolves, so omitting
        it would let a re-fetch inflate a sample undetected.
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
        """Record an artefact before creating it, and return its path."""
        resolved = Path(path).resolve()
        self.manifest.artefacts[str(kind)] = str(resolved)
        self.manifest.write(self.manifest_path)
        return resolved

    # -- exit --

    def restore(self) -> None:
        """Remove every recorded artefact, containment-checked, then stop.

        Never resolves symlinks, and idempotent so teardown can replay it.
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

        The two in-repository allowances are tested first and everything else
        under the plugin root refused outright, so a temp root that happens to
        be an ancestor of the checkout cannot admit a tracked path by
        transitivity.
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

        Any failure invalidates the session, so a cleanup failure blocks the
        verdict rather than being recorded beneath a passing one.
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
            if _digest_or_absent(self.plugin_root / name) != digest:
                problems.append(f"{name} changed during the run")
        return problems

    def _verify_unverified_log(self) -> list[str]:
        """Treat the unverified log as append-only.

        Growth *is* a trust-chain failure or an engaged override, so it
        invalidates the session rather than being attributed and tidied — the
        record carries a subprocess pid, which pid reuse makes unreliable.
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
    """Build the six cells, in the order the criterion lists them."""
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

    `jj git init` colocates by default, and a colocated fixture emits **warn**
    rather than the blocked decision, so the wrong path would be measured
    silently. Asserted on the resulting tree, not on the flag, because the
    flag's meaning is what a version bump could change.
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

    A missing tool exits 0 under `--fail-safe`, recording a spuriously low
    latency rather than failing, so the union is required. Links resolve to the
    concrete binary and never a mise shim, which would re-resolve its version
    from the config discovered at the sampling cwd.
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

    `LC_ALL=C` rather than `LANG` alone: the baseline is dominated by text-tool
    spawns whose speed varies materially between `C` and a UTF-8 locale.
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

    Two engines because a GitHub checkout has no `.jj`. The git form needs
    unshallowed history, so its lane must set `fetch-depth: 0`.
    """
    if engine == "jj":
        return ["jj", "file", "show", "-r", BASELINE_REVSET, source]
    return ["git", "show", f"{BASELINE_COMMIT}:{source}"]


def recover_baseline(
    scratch: Path, *, runner: DiagnosticRunner, engine: str = "jj"
) -> Path:
    """Recover the baseline and its one dependency into `scratch`.

    Both at the same revision, so the subject is self-contained rather than
    resolving a live dependency that may change under it, and the layout
    mirrors the original so the baseline's own relative lookup resolves within
    `scratch`. Nothing is written inside the repository, whose cache root is
    both an exec target and an integrity witness.

    Digests are compared against recorded provenance, without which the
    recovery is verifiable only inside a jj workspace.
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


def entry_platform() -> str:
    """Derive the platform alias the bootstrap uses for this host.

    Derived the same way, so the cache-entry names predicted here are the ones
    actually written.
    """
    from tasks.shared.targets import host_platform

    return host_platform()


def _entry_for(platform_key: tuple[str, str]) -> PlatformEntry | None:
    return platform_constants(platform_key, PLATFORM_TABLE)


@task(name="warm-dispatch")
def warm_dispatch(
    context: Context,
    platform_key: str | None = None,
    blocks: str = "AB",
    rehearse: bool = False,
) -> None:
    """Measure warm-dispatch latency against the shell baseline.

    Needs a quiet host, a pinned `PATH`, network egress and several minutes.
    `--rehearse` drives the whole path at a token sample count and stamps its
    record non-gating: a smoke run, never evidence.
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
    run_session(REPO_ROOT, entry=entry, blocks=blocks, rehearse=rehearse)


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

    Owns the `measure:*` namespace against rot, since it rests on several
    volatile external contracts. Emits no gating figure by construction: the
    ceilings are calibrated for a quiet host and no shared runner reliably
    clears the instrument-floor gate.
    """
    del context
    print(smoke_report(REPO_ROOT, engine=engine, live=live))


def digest_backend_population() -> dict[str, str | None]:
    """Which digest backend this host resolves, recorded rather than assumed.

    The fast backend is absent from a stock macOS image and universal on linux,
    so which cells a lane could enforce is a property of the runner.
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

    Fails on a rotted contract — a moved revision, a colocating fixture, an
    unbuildable farm, a baseline that no longer emits a decision. An absent
    published release for the tree's own version is reported as an unmet
    prerequisite rather than a failure, since the tree is routinely ahead of
    the last release cut.
    """
    lines = [f"digest backends: {digest_backend_population()}"]
    if live:
        witness = FilesystemWitness()
        priming = prime_cache(
            plugin_root,
            runner=runner,
            entries=lambda: witness.entries(plugin_root / "bin"),
            version=plugin_version(plugin_root),
            platform=entry_platform(),
        )
        if priming["primed"]:
            lines.append(
                f"cache primed before capture; gaps closed: "
                f"{not priming['gaps_after']}"
            )
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
    # A discarded warm-up, so a still-degraded sample means an unmet
    # prerequisite rather than a cold cache.
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


@dataclass
class Rig:
    """Everything one session samples through, built once and torn down once."""

    session: MeasurementSession
    temp_parent: Path
    guard: Path
    fixture: Path
    fast_farm: Path
    fallback_farm: Path
    floor_script: Path
    variants: dict[Variant, list[str]]
    environments: dict[Variant, dict[str, str]]
    expected_reason: str


def build_rig(
    session: MeasurementSession,
    plugin_root: Path,
    *,
    tools: Sequence[str],
    runner: MeasurementRunner,
) -> Rig:
    """Register, create and assert every throwaway the session samples under.

    Registration precedes creation, so a run killed between the two leaves a
    manifest naming a path that does not exist rather than an orphan.
    """
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
    assert_backends(fast_farm, fallback_farm)

    dispatch = [
        str(plugin_root / "bin/accelerator"),
        "vcs",
        "guard",
        "--format=hook",
        "--fail-safe",
    ]
    variants = {
        Variant.BASELINE: [str(guard)],
        Variant.FAST: dispatch,
        Variant.FALLBACK: dispatch,
    }
    environments = {
        Variant.BASELINE: farm_environment(fast_farm, temp_root=temp_parent),
        Variant.FAST: farm_environment(fast_farm, temp_root=temp_parent),
        Variant.FALLBACK: farm_environment(
            fallback_farm, temp_root=temp_parent
        ),
    }

    probe = runner(
        variants[Variant.BASELINE],
        cwd=fixture,
        env=environments[Variant.BASELINE],
    )
    decision, reason = expected_decision(probe.stdout)
    if decision is not Decision.BLOCK:
        raise PreconditionFailureError(
            f"the recovered baseline emits {decision}, not the blocked "
            f"decision — the session would measure the wrong path"
        )
    return Rig(
        session=session,
        temp_parent=temp_parent,
        guard=guard,
        fixture=fixture,
        fast_farm=fast_farm,
        fallback_farm=fallback_farm,
        floor_script=floor_script,
        variants=variants,
        environments=environments,
        expected_reason=reason,
    )


@dataclass(frozen=True)
class RecordPaths:
    attempt: int
    record: Path
    samples: Path


def next_record_paths(
    directory: Path, *, rehearse: bool = False
) -> RecordPaths:
    """Where this attempt's record and raw samples go.

    Numbered and never reused: an invalidated session's record is the evidence
    that an attempt was made, so a re-run must not clobber it.
    """
    if rehearse:
        stem = "warm-dispatch-rehearsal"
        return RecordPaths(
            attempt=0,
            record=directory / f"{stem}.json",
            samples=directory / f"{stem}-samples.json",
        )
    taken = []
    for path in directory.glob("warm-dispatch-*.json"):
        suffix = path.stem.removeprefix("warm-dispatch-")
        if suffix.isdigit():
            taken.append(int(suffix))
    attempt = max(taken, default=0) + 1
    return RecordPaths(
        attempt=attempt,
        record=directory / f"warm-dispatch-{attempt}.json",
        samples=directory / f"warm-dispatch-{attempt}-samples.json",
    )


def prime_cache(
    plugin_root: Path,
    *,
    runner: MeasurementRunner,
    entries: Callable[[], Sequence[str]],
    version: str,
    platform: str,
) -> dict[str, object]:
    """Populate the cache root before a baseline is captured, if it is cold.

    A cold cache is a prerequisite of the smoke check, not a cleanup failure:
    its own first dispatch fetches, and a baseline captured beforehand would
    report those entries as appearing during the run. Priming outside the
    witnessed window satisfies it without weakening the witness. The
    measurement proper instead refuses a cold cache, a freshly fetched entry
    not being the warm path it times.
    """
    before = warm_cache_gaps(
        version=version,
        cache_root_entries=entries(),
        platform=platform,
    )
    if not before:
        return {"primed": False, "gaps_before": [], "gaps_after": []}
    # The ambient environment: no farm exists yet, and a cold fetch needs a
    # downloader the variants themselves may not use.
    runner(
        [
            str(plugin_root / "bin/accelerator"),
            "vcs",
            "guard",
            "--format=hook",
            "--fail-safe",
        ],
        cwd=plugin_root,
        env=dict(os.environ),
    )
    after = warm_cache_gaps(
        version=version,
        cache_root_entries=entries(),
        platform=platform,
    )
    return {"primed": True, "gaps_before": before, "gaps_after": after}


def backend_delta_check(
    *, fast_ms: float, fallback_ms: float | None
) -> dict[str, object]:
    """Cross-check the direct digest measurement against the backend delta.

    The delta is a cross-check, never the measurement: it yields twice the
    difference between the backends, not the absolute cost under the gating
    configuration.
    """
    delta = None if fallback_ms is None else fallback_ms - fast_ms
    return {
        "absolute_under_the_gating_backend_ms": fast_ms,
        "fallback_backend_ms": fallback_ms,
        "delta_ms": delta,
        "implied_per_call_difference_ms": (
            None if delta is None else delta / 2
        ),
        "role": (
            "the delta is a cross-check on the direct measurement, not a "
            "substitute for it: it carries twice the per-call difference "
            "between the backends, not the absolute cost the budget needs"
        ),
    }


def warm_cache_gaps(
    *, version: str, cache_root_entries: Sequence[str], platform: str
) -> list[str]:
    """Name what the cache root lacks for a warm dispatch of `version`.

    The cache is keyed by version, so a tree bumped past its last fetch has a
    full cache and a cold path. Unchecked, a failure in that fetch surfaces as
    an exit 0 with empty stdout rather than a named prerequisite.
    """
    entries = set(cache_root_entries)
    gaps = []
    launcher = f"accelerator-launcher-{version}-{platform}"
    if launcher not in entries:
        gaps.append(f"no cached launcher for {version}: expected {launcher}")
    elif f"{launcher}.minisig" not in entries:
        gaps.append(f"the cached launcher for {version} has no signature")
    subbinaries = [
        entry
        for entry in entries
        if entry.startswith(f"vcs-{version}-")
        and not entry.endswith(".minisig")
    ]
    if not subbinaries:
        gaps.append(f"no cached vcs sub-binary for {version}")
    elif not any(f"{name}.minisig" in entries for name in subbinaries):
        gaps.append(f"the cached vcs sub-binary for {version} has no signature")
    return gaps


def check_preconditions(
    plugin_root: Path,
    session: MeasurementSession,
    rig: Rig,
    *,
    strict: bool = True,
) -> dict[str, object]:
    """Assert every pre-sampling condition and return what was observed.

    A failure produces no figures at all: a session that cannot establish its
    own conditions has measured nothing. A rehearsal passes `strict=False` and
    records the violations instead, its record being stamped non-gating.
    """
    violations: list[str] = []

    def refuse(reason: str) -> None:
        if strict:
            raise PreconditionFailureError(reason)
        violations.append(reason)

    observed_keys = sorted(
        key for key in session.host.env() if key.startswith("ACCELERATOR_")
    )
    offenders = accelerator_override_keys(session.host.env())
    if offenders:
        refuse(f"ACCELERATOR_* overrides are set: {offenders}")
    ambient_root = session.host.env().get("CLAUDE_PLUGIN_ROOT")
    if ambient_root and Path(ambient_root).resolve() != plugin_root.resolve():
        refuse(
            f"the driving session dispatches against {ambient_root}, not "
            f"{plugin_root} — every integrity witness would point at one root "
            f"while the interfering session wrote the other"
        )
    observed_jj = session.diagnostics(["jj", "--version"])
    pinned = jj_pin(plugin_root)
    if pinned not in observed_jj:
        refuse(
            f"jj {observed_jj!r} is not the mise.toml pin {pinned} — the "
            f"fixture's colocation flag is justified by that release's default"
        )
    gaps = warm_cache_gaps(
        version=plugin_version(plugin_root),
        cache_root_entries=session.witness.entries(plugin_root / "bin"),
        platform=entry_platform(),
    )
    if gaps:
        refuse(
            "the cache is cold for this tree's version, so the first dispatch "
            "would take the fetch branch and any failure in it would surface "
            "as an unexplained degraded sample: "
            + "; ".join(gaps)
            + ". Run `bin/accelerator vcs detect` once to warm it, or check "
            "that a signed release is published for this version."
        )
    others = concurrent_sessions(session.diagnostics)
    if others:
        refuse(
            f"other Claude Code sessions are active against this plugin root: "
            f"{others}"
        )
    return {
        "violations": violations,
        "concurrent_sessions": others,
        "accelerator_env_keys": observed_keys,
        "dev_launcher_marker": (
            plugin_root / ".accelerator-dev-launcher"
        ).exists(),
        "plugin_root": str(plugin_root),
        "plugin_version": plugin_version(plugin_root),
        "jj": observed_jj,
        "jj_pin": pinned,
        "driving_session_pid": os.getpid(),
        "fixture": str(rig.fixture),
        "fixture_depth": len(rig.fixture.resolve().parts),
        "stdin_envelope": STDIN_ENVELOPE,
        "expected_reason": rig.expected_reason,
        "baseline_commit": BASELINE_COMMIT,
        "warm_cache_gaps": gaps,
        "recovered_digests": {
            name: expected for name, (_, expected) in RECOVERED_FILES.items()
        },
    }


def gate_floors(
    entry: PlatformEntry,
    rig: Rig,
    runner: MeasurementRunner,
    *,
    when: str,
) -> dict[str, object]:
    """Measure both instrument floors and gate on them, retries recorded.

    Capped and recorded: retrying informally until the floors look good is
    optional stopping through the back door.
    """
    attempts: list[dict[str, float]] = []
    while retry_budget(len(attempts), cap=FLOOR_RETRY_CAP):
        bash_ms, true_ms = measure_floors(
            runner,
            floor_script=rig.floor_script,
            farm=rig.fast_farm,
            cwd=rig.fixture,
            temp_root=rig.temp_parent,
        )
        attempts.append({"bash_ms": bash_ms, "true_ms": true_ms})
        if bash_ms <= entry.bash_floor_ms and true_ms <= entry.true_floor_ms:
            return {"when": when, "attempts": attempts, "holds": True}
        print(
            f"{when} floors breached the gate "
            f"({bash_ms:.2f} > {entry.bash_floor_ms} or "
            f"{true_ms:.2f} > {entry.true_floor_ms}) — retrying"
        )
    return {"when": when, "attempts": attempts, "holds": False}


def run_session(
    plugin_root: Path,
    *,
    entry: PlatformEntry | None,
    blocks: str = "AB",
    runner: MeasurementRunner = subprocess_measurement_runner,
    record_path: Path | None = None,
    rehearse: bool = False,
) -> dict[str, object]:
    """Drive one recorded session end to end and return its record.

    Everything happens in the one session: figures assembled from several are
    not comparable.
    """
    started = time.perf_counter()
    tools = entry.path_tools if entry else ("bash", "jj", "true")
    record: dict[str, object] = {"blocks": blocks, "gating": not rehearse}
    with MeasurementSession(plugin_root) as session:
        rig = build_rig(session, plugin_root, tools=tools, runner=runner)
        record["preconditions"] = check_preconditions(
            plugin_root, session, rig, strict=not rehearse
        )
        record.update(record_provenance(session, rig, entry))

        if entry is None:
            raise PreconditionFailureError(
                "no calibrated platform entry — every cell is branch 7, so "
                "there is nothing to gate; re-run with --platform-key or add "
                "an entry"
            )
        floors_pre = gate_floors(entry, rig, runner, when="pre")
        record["floors_pre"] = floors_pre
        if not floors_pre["holds"] and not rehearse:
            raise PreconditionFailureError(
                "the instrument floors did not clear their gate in three "
                "recorded attempts — the host is not quiet (branch 5a)"
            )

        # Discarded from the figures but not from the assertions: the first
        # dispatch through the farm is the cheapest place to find the
        # environment wrong.
        warm_up = runner(
            rig.variants[Variant.FAST],
            cwd=rig.fixture,
            env=rig.environments[Variant.FAST],
        )
        verdict = validate_dispatch(
            warm_up.stdout,
            rig.expected_reason,
            label="warm-up dispatch",
            stderr=warm_up.stderr,
        )
        record["warm_up"] = {
            "valid": verdict.valid,
            "diagnostic": verdict.diagnostic,
            "elapsed_ms": warm_up.elapsed_ms,
        }
        if not verdict.valid:
            raise PreconditionFailureError(
                f"the warm-up dispatch did not reach the blocked decision, so "
                f"no sample would either: {verdict.diagnostic}"
            )

        sizes, pilot = run_pilot(rig, runner, blocks=blocks, rehearse=rehearse)
        record["pilot"] = pilot
        samples = sample_blocks(
            rig,
            runner,
            block_a_pairs=sizes["block_a_pairs"],
            block_b_samples=sizes["block_b_samples"],
            blocks=blocks,
            started=started,
        )
        record["floors_post"] = gate_floors(entry, rig, runner, when="post")
        record["dispersion"] = dispersion(samples)
        record["terms"] = close_the_budget(
            plugin_root, rig, runner, samples=samples, floors=floors_pre
        )
        outcomes, analysis = analyse(
            entry,
            samples,
            floors=floors_pre,
            elapsed=time.perf_counter() - started,
        )
        record["analysis"] = analysis
        record["cells"] = [asdict(outcome) for outcome in outcomes]
        record["closure_verdict"] = closure_verdict(outcomes)
    record["teardown"] = session.failures
    if session.failures:
        record["closure_verdict"] = False
        record["analysis"]["validity"] = str(  # type: ignore[index]
            Validity.INVALID_POST_RUN
        )
    paths = next_record_paths(
        (record_path or plugin_root / "meta/measurements"), rehearse=rehearse
    )
    paths.record.parent.mkdir(parents=True, exist_ok=True)
    record["attempt"] = paths.attempt
    record["samples_path"] = paths.samples.name
    # Without the raw samples an invalidated session is unrecoverable: nothing
    # can be re-derived and no estimator corrected.
    paths.samples.write_text(
        json.dumps(
            {str(variant): list(values) for variant, values in samples.items()}
        )
    )
    paths.record.write_text(json.dumps(record, indent=2, default=str))
    print(f"record written to {paths.record}")
    print(f"raw samples written to {paths.samples}")
    if session.failures:
        raise Exit(
            "teardown verify failed — the session is invalidated (branch 5b), "
            "and its figures are recorded explicitly non-gating:\n"
            + "\n".join(session.failures),
            code=1,
        )
    return record


def observed_dirname_spawns(rig: Rig) -> int:
    """Trace the baseline once under `bash -x` and count its `dirname` spawns.

    Observed rather than implied by fixture depth. A count independent of depth
    means another host's temp root cannot perturb the denominator.
    """
    environment = rig.environments[Variant.BASELINE]
    completed = subprocess.run(
        [str(rig.fast_farm / "bash"), "-x", str(rig.guard)],
        check=False,
        capture_output=True,
        text=True,
        input=STDIN_ENVELOPE,
        cwd=rig.fixture,
        env=environment,
    )
    return dirname_spawn_count(completed.stderr)


def run_pilot(
    rig: Rig,
    runner: MeasurementRunner,
    *,
    blocks: str,
    rehearse: bool = False,
) -> tuple[dict[str, int], dict[str, object]]:
    """Size the run from an in-session pilot whose samples are then discarded.

    A dispersion estimate only: pooling it into the analysed set would make the
    final interval's coverage depend on a data-dependent stopping rule. A
    size-up recomputes n from the same targets, never a relaxed one.
    """
    pilot_pairs = REHEARSAL_N if rehearse else PILOT_PAIRS
    pilot = sample_blocks(
        rig,
        runner,
        block_a_pairs=pilot_pairs,
        block_b_samples=pilot_pairs,
        blocks=blocks,
        started=time.perf_counter(),
        pilot=True,
    )
    rng = random.Random(SEED)  # noqa: S311 — statistical resampling, not a security context
    floor_a = REHEARSAL_N if rehearse else BLOCK_A_PAIRS
    floor_b = REHEARSAL_N if rehearse else BLOCK_B_SAMPLES
    sizes = {"block_a_pairs": floor_a, "block_b_samples": floor_b}
    report: dict[str, object] = {}
    baseline = pilot[Variant.BASELINE]
    fast = pilot[Variant.FAST]
    if baseline and len(baseline) == len(fast):
        interval = paired_ratio_interval(
            baseline, fast, resamples=2000, confidence=CONFIDENCE, rng=rng
        )
        needed, feasible = pilot_sizing(
            interval,
            target=RATIO_TARGET,
            pilot_n=len(baseline),
            cap=BLOCK_A_MAX_PAIRS,
        )
        sizes["block_a_pairs"] = floor_a if rehearse else max(floor_a, needed)
        report["block_a"] = {
            "achieved_upper_distance": interval.upper_distance,
            "required": needed,
            "feasible": feasible,
        }
    fallback = pilot[Variant.FALLBACK]
    if fallback:
        interval = unpaired_interval(
            fallback,
            statistic=lambda values: summarise(values).median,
            resamples=2000,
            confidence=CONFIDENCE,
            rng=rng,
        )
        needed, feasible = pilot_sizing(
            interval,
            target=MEDIAN_TARGET_MS,
            pilot_n=len(fallback),
            cap=BLOCK_B_MAX_SAMPLES,
        )
        sizes["block_b_samples"] = floor_b if rehearse else max(floor_b, needed)
        report["block_b"] = {
            "achieved_upper_distance": interval.upper_distance,
            "required": needed,
            "feasible": feasible,
        }
    report["sizes"] = dict(sizes)
    report["discarded"] = {
        str(variant): len(values) for variant, values in pilot.items()
    }
    return (sizes, report)


def dispersion(
    samples: Mapping[Variant, Sequence[float]],
) -> dict[str, object]:
    return {
        str(variant): asdict(summarise(values))
        for variant, values in samples.items()
        if values
    }


# The terms that compose a warm dispatch, in the order they run. A listed
# term's own sub-operations are recorded as context and never summed, or the
# budget would double-count them.
SUMMED_TERMS = (
    "bash startup",
    "two sha256_file calls",
    "shim minisign-verify of the launcher",
    "launcher startup net of the fork floor",
    "cache::find",
    "reverify",
    "vcs exec and guard work net of the fork floor",
)


def close_the_budget(
    plugin_root: Path,
    rig: Rig,
    runner: MeasurementRunner,
    *,
    samples: Mapping[Variant, Sequence[float]],
    floors: Mapping[str, object],
) -> dict[str, object]:
    """Re-measure every warm-path term in-session and report the residual.

    In-session rather than against published figures, which carry a
    cross-session difference indistinguishable from unattributed cost.
    Re-measurement is triggered by the residual's magnitude, never its sign: a
    sum of noisy medians lands negative roughly half the time.
    """
    version = plugin_version(plugin_root)
    launcher_terms = decompose_terms(plugin_root, version=version)
    true_floor, bash_floor = last_floors(floors)
    shell_terms, backend_check = measure_shell_terms(
        plugin_root,
        rig,
        runner,
        version=version,
        bash_floor=bash_floor,
        true_floor=true_floor,
    )
    terms = {**launcher_terms, **shell_terms}
    summed = [terms[name] for name in SUMMED_TERMS if name in terms]
    fast = list(samples[Variant.FAST])
    report: dict[str, object] = {
        "terms": {name: asdict(term) for name, term in terms.items()},
        "summed": [name for name in SUMMED_TERMS if name in terms],
        "sub_operations_not_summed": [
            name for name in terms if name not in SUMMED_TERMS
        ],
        "cache_root_entries": len(
            rig.session.witness.entries(plugin_root / "bin")
        ),
        "cache_root_bytes": sum(
            path.stat().st_size
            for path in (plugin_root / "bin").iterdir()
            if path.is_file()
        ),
    }
    if not fast or not summed:
        return report
    observed = summarise(fast).median
    verdict = residual_verdict(summed, observed, attempts_used=0)
    report["residual"] = asdict(verdict)
    report["digest_backend_cross_check"] = backend_check
    report["observed_median_ms"] = observed
    report["cross_checked_fraction"] = verdict.total / observed
    report["uncross_checked_fraction"] = 1.0 - (verdict.total / observed)
    return report


def measure_shell_terms(
    plugin_root: Path,
    rig: Rig,
    runner: MeasurementRunner,
    *,
    version: str,
    bash_floor: float,
    true_floor: float,
) -> tuple[dict[str, Interval], dict[str, object]]:
    """Measure the terms that live outside the launcher's library surface.

    Each process launch is a marginal over the fork floor, so the terms compose
    against a dispatch that pays that floor once per exec.
    """
    cache_root = plugin_root / "bin"
    targets = staged_shim_targets(plugin_root)
    fast_digest_ms = measure_digest_bracket(
        runner,
        farm=rig.fast_farm,
        cwd=rig.fixture,
        temp_root=rig.temp_parent,
        targets=targets,
    )
    terms: dict[str, Interval] = {
        "bash startup": _point(bash_floor),
        "two sha256_file calls": _point(fast_digest_ms),
    }
    shim = _newest(cache_root, "accelerator-verify-*-*")
    launcher = _newest(cache_root, f"accelerator-launcher-{version}-*")
    subbinary = _newest(cache_root, f"vcs-{version}-*")
    environment = farm_environment(rig.fast_farm, temp_root=rig.temp_parent)
    signature = _sidecar(launcher) if launcher else None
    if shim and launcher and signature and signature.exists():
        terms["shim minisign-verify of the launcher"] = _point(
            _marginal(
                runner,
                [
                    str(shim),
                    str(plugin_root / "keys/accelerator-release.pub"),
                    str(signature),
                    str(launcher),
                ],
                rig,
                environment,
                floor=true_floor,
            )
        )
    if launcher:
        terms["launcher startup net of the fork floor"] = _point(
            _marginal(
                runner,
                [str(launcher), "--version"],
                rig,
                environment,
                floor=true_floor,
            )
        )
    if subbinary:
        terms["vcs exec and guard work net of the fork floor"] = _point(
            _marginal(
                runner,
                [str(subbinary), "guard", "--format=hook"],
                rig,
                environment,
                floor=true_floor,
            )
        )
    fallback_digest_ms = measure_digest_bracket(
        runner,
        farm=rig.fallback_farm,
        cwd=rig.fixture,
        temp_root=rig.temp_parent,
        targets=staged_shim_targets(plugin_root),
        backend=FALLBACK_BACKEND,
    )
    return (
        terms,
        backend_delta_check(
            fast_ms=terms["two sha256_file calls"].point,
            fallback_ms=fallback_digest_ms,
        ),
    )


def _sidecar(binary: Path) -> Path:
    """Name the detached signature beside a cached entry.

    Appended, never substituted: an entry name carries a dotted version segment
    that a suffix replacement would eat.
    """
    return binary.with_name(binary.name + ".minisig")


def _point(value: float) -> Interval:
    """Wrap a term measured as one figure, with no interval of its own.

    Its zero upper distance contributes nothing to the propagated uncertainty,
    which is why the residual band also carries an absolute floor.
    """
    return Interval(point=value, lower=value, upper=value)


def _newest(root: Path, pattern: str) -> Path | None:
    matches = sorted(
        (
            path
            for path in root.glob(pattern)
            if not path.name.endswith(".minisig")
        ),
        key=lambda path: path.stat().st_mtime,
    )
    return matches[-1] if matches else None


def _marginal(
    runner: MeasurementRunner,
    argv: Sequence[str],
    rig: Rig,
    environment: Mapping[str, str],
    *,
    floor: float,
    samples: int = FLOOR_SAMPLES,
) -> float:
    observed = [
        runner(argv, cwd=rig.fixture, env=environment).elapsed_ms
        for _ in range(samples)
    ]
    return max(0.0, median(observed) - floor)


def staged_shim_targets(plugin_root: Path) -> list[Path]:
    """Locate the two files the bootstrap hashes on every dispatch."""
    cache_root = plugin_root / "bin"
    sources = sorted(cache_root.glob("accelerator-verify-*"))
    unstaged = [path for path in sources if len(path.name.split("-")) == 4]
    staged = [path for path in sources if len(path.name.split("-")) > 4]
    return (unstaged[:1] or sources[:1]) + (staged[:1] or sources[:1])


def analyse(
    entry: PlatformEntry,
    samples: Mapping[Variant, Sequence[float]],
    *,
    floors: Mapping[str, object],
    elapsed: float,
) -> tuple[list[CellOutcome], dict[str, object]]:
    """Compute every cell's interval, classify it, and record the diagnostics.

    The three floor treatments keep fixed roles: raw medians gate, the
    `true`-floor-subtracted point estimate is the robustness check, and the
    bash-floor-subtracted ratio is diagnostic only because it over-subtracts —
    bash startup is real cost the dispatched variant pays.
    """
    rng = random.Random(SEED)  # noqa: S311 — statistical resampling, not a security context
    baseline = list(samples[Variant.BASELINE])
    fast = list(samples[Variant.FAST])
    fallback = list(samples[Variant.FALLBACK])
    true_floor, bash_floor = last_floors(floors)

    intervals: dict[str, Interval | None] = {
        "C1": _absolute(fast, lambda v: summarise(v).median, RESAMPLES, rng),
        "C2": _absolute(fast, lambda v: summarise(v).p90, RESAMPLES, rng),
        "C3": _absolute(
            fallback, lambda v: summarise(v).median, RESAMPLES, rng
        ),
        "C4": _absolute(fallback, lambda v: summarise(v).p90, RESAMPLES, rng),
        "C5": _ratio(baseline, fast, RESAMPLES, rng, paired=True),
        "C6": _ratio(baseline, fallback, RESAMPLES, rng, paired=False),
    }
    ratios: dict[str, object] = {}
    robustness_ok: bool | None = None
    if baseline and len(baseline) == len(fast):
        raw = ratio_of_medians(baseline, fast)
        true_subtracted = ratio_of_medians(
            subtract_floor(baseline, true_floor),
            subtract_floor(fast, true_floor),
        )
        bash_subtracted = ratio_of_medians(
            subtract_floor(baseline, bash_floor),
            subtract_floor(fast, bash_floor),
        )
        robustness_ok = true_subtracted <= RATIO_THRESHOLD
        # The robustness condition gates on the point estimate. Its interval
        # is recorded too, so the gate can move to the upper bound without
        # another measurement.
        robustness_interval = paired_ratio_interval(
            subtract_floor(baseline, true_floor),
            subtract_floor(fast, true_floor),
            resamples=RESAMPLES,
            confidence=CONFIDENCE,
            rng=rng,
        )
        first_b, last_b = thirds(baseline)
        first_g, last_g = thirds(fast)
        ratios = {
            "raw_gates": raw,
            "true_floor_subtracted_robustness_check": true_subtracted,
            "true_floor_subtracted_interval": asdict(robustness_interval),
            "robustness_holds_on_the_upper_bound": (
                robustness_interval.upper <= RATIO_THRESHOLD
            ),
            "bash_floor_subtracted_diagnostic_only": bash_subtracted,
            "median_of_ratios": median_of_ratios(baseline, fast),
            "p90_ratio_context": summarise(fast).p90 / summarise(baseline).p90,
            "drift_first_third": (
                ratio_of_medians(first_b, first_g) if first_b else None
            ),
            "drift_last_third": (
                ratio_of_medians(last_b, last_g) if last_b else None
            ),
        }
        ratios["drift"] = assess_drift(baseline, fast, rng=rng)
        ratios["drift_holds"] = bool(ratios["drift"]["holds"])
    validity = (
        Validity.VALID
        if ratios.get("drift_holds", True)
        else Validity.INVALID_POST_RUN
    )
    outcomes = []
    for cell in cells_for(entry):
        interval = intervals[cell.name]
        outcome = classify_cell(
            cell,
            interval,
            robustness_ok=(
                None if cell.kind is CellKind.ABSOLUTE else robustness_ok
            ),
            escalations_used=0,
            validity=validity,
            sizing_feasible=True,
            applicable=interval is not None,
            budget_spent=elapsed >= WALL_CLOCK_BUDGET_S,
        )
        outcomes.append(outcome)
        _report_cell(cell, interval, outcome)
    return (
        outcomes,
        {
            "intervals": {
                name: (asdict(i) if i else None)
                for name, i in intervals.items()
            },
            "ratios": ratios,
            "validity": str(validity),
            "elapsed_s": elapsed,
        },
    )


def last_floors(floors: Mapping[str, object]) -> tuple[float, float]:
    """Read the final attempt's floors, or zero when none ran.

    The final attempt, not the first: only the one that cleared the gate is the
    instrument the samples were taken under.
    """
    attempts = floors.get("attempts")
    if not isinstance(attempts, list) or not attempts:
        return (0.0, 0.0)
    last = attempts[-1]
    if not isinstance(last, dict):
        return (0.0, 0.0)
    return (float(last.get("true_ms", 0.0)), float(last.get("bash_ms", 0.0)))


def _report_cell(
    cell: Cell, interval: Interval | None, outcome: CellOutcome
) -> None:
    if interval is None:
        print(f"{cell.name} ({cell.description}): not applicable (branch 7)")
        return
    print(
        f"{cell.name} ({cell.description}): {interval.point:.4f} "
        f"[{interval.lower:.4f}, {interval.upper:.4f}] against "
        f"{cell.threshold} -> branch {outcome.branch}"
    )


def assert_backends(fast_farm: Path, fallback_farm: Path) -> None:
    """Assert both farms in both directions before sampling either block.

    A fast farm missing its backend link would silently measure the fallback
    one under the cells gated on the fast backend.
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


def sample_blocks(
    rig: Rig,
    runner: MeasurementRunner,
    *,
    block_a_pairs: int,
    block_b_samples: int,
    blocks: str,
    started: float,
    pilot: bool = False,
) -> dict[Variant, list[float]]:
    """Take one block's samples, gated and braked on every one of them.

    `started` is the session's start, not this call's: the wall-clock budget
    covers the pilot and every subsequent run together.
    """
    schedule = generate_schedule(
        block_a_pairs=0 if pilot else block_a_pairs,
        block_b_samples=0 if pilot else block_b_samples,
        pilot_pairs=block_a_pairs if pilot else 0,
        pilot_samples=block_b_samples if pilot else 0,
        segment=SEGMENT_SAMPLES,
        rng=random.Random(SEED),  # noqa: S311 — statistical sampling, not a security context
    )
    schedule = [
        sample
        for sample in schedule
        if sample.pilot == pilot and sample.block in blocks
    ]
    observed: dict[Variant, list[float]] = {variant: [] for variant in Variant}
    pending: dict[Variant, str] = {}
    for index, sample in enumerate(schedule):
        result = runner(
            rig.variants[sample.variant],
            cwd=rig.fixture,
            env=rig.environments[sample.variant],
        )
        arm = observed[sample.variant]
        if arm and outlier_trip(
            result.elapsed_ms, arm_median=median(arm), arm_count=len(arm)
        ):
            raise Exit(
                f"outlier trip on {sample.variant} at sample {index}: "
                f"{result.elapsed_ms:.2f} ms against a running median of "
                f"{median(arm):.2f} ms over {len(arm)} samples — a re-fetch or "
                f"a noisy host, not a warm dispatch",
                code=1,
            )
        _gate_sample(rig, sample.variant, result.stdout, pending, index)
        arm.append(result.elapsed_ms)
        if (
            budget_exhausted(
                time.perf_counter() - started,
                WALL_CLOCK_BUDGET_S,
                index,
                len(schedule),
            )
            and index < len(schedule) - 1
        ):
            raise Exit(
                "the wall-clock budget is exhausted mid-run (branch 6b) — "
                "partial figures are recorded explicitly non-gating",
                code=1,
            )
    return observed


def _gate_sample(
    rig: Rig,
    variant: Variant,
    stdout: str,
    pending: dict[Variant, str],
    index: int,
) -> None:
    """Assert this sample exercised the path being timed, and abort if not.

    Runs outside the timed bracket. A fail-safe swallow exits 0 with empty
    stdout having skipped most of the work, recording a spuriously low latency.
    """
    pending[variant] = stdout
    if variant is Variant.FALLBACK:
        verdict = validate_sample(stdout, stdout, rig.expected_reason)
    elif Variant.BASELINE in pending and Variant.FAST in pending:
        verdict = validate_sample(
            pending.pop(Variant.BASELINE),
            pending.pop(Variant.FAST),
            rig.expected_reason,
        )
    else:
        return
    if not verdict.valid:
        raise Exit(
            f"per-sample validity gate failed at sample {index}: "
            f"{verdict.diagnostic}",
            code=1,
        )


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
    *,
    paired: bool,
) -> Interval | None:
    """Build a ratio cell's interval, paired or not as the block dictates.

    The fast block's arms are interleaved pairs; the fallback block is
    single-arm and has no baseline of its own, so pairing it is impossible
    rather than merely unnecessary.
    """
    if not baseline or not variant:
        return None
    if paired:
        if len(baseline) != len(variant):
            return None
        return paired_ratio_interval(
            baseline,
            variant,
            resamples=resamples,
            confidence=CONFIDENCE,
            rng=rng,
        )
    return unpaired_ratio_interval(
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

    On linux the load average is host-scoped regardless of cgroup membership,
    so dividing it by a container's quota yields a meaningless number.
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


def report_load(quietness_record: Mapping[str, object]) -> None:
    """Print the observed load beside the CPU count, and flag oversubscription.

    Deliberately not a gate: the direction of the load bias is unresolved, so
    refusing on it would encode an unsupported model. Printed before sampling
    because the floors can pass on a loaded host that then fails on drift.
    """
    load = quietness_record.get("loadavg")
    count = quietness_record.get("cpu_count")
    one_minute = load[0] if isinstance(load, (list, tuple)) and load else None
    print(
        f"load {load} over {count} cpus (rung: "
        f"{quietness_record.get('cpu_count_rung')})"
    )
    if (
        isinstance(one_minute, (int, float))
        and isinstance(count, int)
        and one_minute > count
    ):
        print(
            f"⚠ the host is oversubscribed ({one_minute:.1f} > {count}). The "
            f"instrument floors may still pass, but drift is the diagnostic "
            f"that fails on a host which is not in a steady state — consider "
            f"waiting for the machine to settle before spending the session."
        )


def floors_hold(
    entry: PlatformEntry,
    *,
    bash_floor_ms: float,
    true_floor_ms: float,
    attempts_used: int,
) -> tuple[bool, bool]:
    """Report whether the instrument floors clear their gate, and may retry.

    A breach is a precondition failure, not a note: a session that cannot reach
    the calibrated floors is not measuring the same instrument.
    """
    holds = (
        bash_floor_ms <= entry.bash_floor_ms
        and true_floor_ms <= entry.true_floor_ms
    )
    return (holds, retry_budget(attempts_used, cap=FLOOR_RETRY_CAP))


def assess_drift(
    baseline: Sequence[float], variant: Sequence[float], *, rng: random.Random
) -> dict[str, object]:
    """Judge the session's drift against a band derived from its own null.

    Records the superseded constant's verdict alongside, so a reader can see
    whether the change of basis changed the outcome.
    """
    observed = drift_statistic(baseline, variant)
    band = drift_band_from_permutation(
        baseline,
        variant,
        permutations=DRIFT_PERMUTATIONS,
        quantile=DRIFT_QUANTILE,
        rng=rng,
    )
    significance = drift_significance(
        baseline, variant, permutations=DRIFT_PERMUTATIONS, rng=rng
    )
    return {
        "observed": observed,
        "band": band,
        "quantile": DRIFT_QUANTILE,
        "permutations": DRIFT_PERMUTATIONS,
        "significance": significance,
        "holds": abs(observed) <= band,
        "holds_under_superseded_constant": drift_verdict(
            0.0, observed, band=SUPERSEDED_DRIFT_BAND
        ),
        "superseded_band": SUPERSEDED_DRIFT_BAND,
    }


def budget_closes(
    terms: Sequence[Interval], observed_median: float, attempts_used: int
) -> object:
    return residual_verdict(terms, observed_median, attempts_used)


def observed_chip(diagnostics: DiagnosticRunner) -> str:
    """Read the chip's brand string, not `platform.processor()`.

    On darwin that returns `"arm"`, which no calibration provenance can
    meaningfully agree or disagree with: floors are calibrated per chip
    generation, and only the brand string carries that distinction.
    """
    for argv in (
        ["sysctl", "-n", "machdep.cpu.brand_string"],
        ["lscpu", "-J"],
    ):
        try:
            observed = diagnostics(argv).strip()
        except FileNotFoundError, PermissionError:
            continue
        if observed:
            return observed.splitlines()[0]
    return platform.processor() or platform.machine()


def record_provenance(
    session: MeasurementSession, rig: Rig, entry: PlatformEntry | None
) -> dict[str, object]:
    """Gather everything the figures are read against, verdict or not."""
    quietness_record = quietness(session.host, entry)
    report_load(quietness_record)
    tools = {
        "fast": tool_provenance(rig.fast_farm, session.diagnostics),
        "fallback": tool_provenance(rig.fallback_farm, session.diagnostics),
    }
    provenance: dict[str, object] = {
        "quietness": quietness_record,
        "tools": tools,
        "interpreter": {
            "executable": sys.executable,
            "version": sys.version,
            "clock": str(time.get_clock_info("perf_counter")),
            "seed": SEED,
        },
        "host": {
            "system": platform.system(),
            "machine": platform.machine(),
            "release": platform.release(),
            "platform": platform.platform(),
        },
        "dirname_spawns": observed_dirname_spawns(rig),
    }
    if entry is not None:
        calibration = observed_calibration(
            entry, tools["fast"], chip=observed_chip(session.diagnostics)
        )
        provenance["calibration"] = calibration
        print(f"calibration: {calibration['note']}")
    return provenance


def observed_calibration(
    entry: PlatformEntry, tools: Mapping[str, object], *, chip: str
) -> dict[str, object]:
    """Compare this host against the entry's calibration provenance.

    A provenance disagreement demotes the verdict to uncalibrated context, so
    this is evaluated during the run rather than left as an unreached helper.
    """

    def version_of(tool: str) -> str:
        record = tools.get(tool)
        if isinstance(record, dict):
            return str(record.get("version", "unknown"))
        return "unknown"

    bash = version_of("bash")
    shasum = version_of(FALLBACK_BACKEND)
    unconfirmed = unconfirmed_calibration_fields(
        entry, observed_chip=chip, observed_bash=bash, observed_shasum=shasum
    )
    return {
        "observed": {"chip": chip, "bash": bash, "shasum": shasum},
        "recorded": asdict(entry.calibration) if entry.calibration else None,
        "unconfirmed": unconfirmed,
        "holds": not unconfirmed,
        "note": calibration_note(entry, chip=chip, bash=bash, shasum=shasum),
    }


def calibration_note(
    entry: PlatformEntry, *, chip: str, bash: str, shasum: str
) -> str:
    """State whether this host is judged by numbers calibrated for it."""
    unconfirmed = unconfirmed_calibration_fields(
        entry, observed_chip=chip, observed_bash=bash, observed_shasum=shasum
    )
    if not unconfirmed:
        return "calibrated"
    return "uncalibrated for this host — " + "; ".join(unconfirmed)


def percentile_of(values: Sequence[float], quantile: float) -> float:
    return percentile(values, quantile)


# --- Instrument floors, provenance and the composition budget -------------


def measure_floors(
    runner: MeasurementRunner,
    *,
    floor_script: Path,
    farm: Path,
    cwd: Path,
    temp_root: Path,
    samples: int = FLOOR_SAMPLES,
) -> tuple[float, float]:
    """Median cost of a trivial bash script and of `true`, in milliseconds.

    Resolved through the farm the samples are taken under, so floor and
    measurement share an instrument. Taken before and after sampling, the pair
    being the cheapest witness that the instrument did not move.
    """
    environment = farm_environment(farm, temp_root=temp_root)
    bash = farm / "bash"
    true = farm / "true"
    if not true.exists():
        raise PreconditionFailureError(
            f"no `true` in the farm at {farm} — the floor cannot be measured"
        )
    bash_samples = [
        runner([str(bash), str(floor_script)], cwd=cwd, env=environment)
        for _ in range(samples)
    ]
    true_samples = [
        runner([str(true)], cwd=cwd, env=environment) for _ in range(samples)
    ]
    return (
        median([sample.elapsed_ms for sample in bash_samples]),
        median([sample.elapsed_ms for sample in true_samples]),
    )


def decompose_terms(
    plugin_root: Path, *, version: str, subbinary: str = "vcs"
) -> dict[str, Interval]:
    """Re-measure the launcher-side warm-path terms in this same session.

    One term is measured by a replica of a private method; the caveat is on the
    test that reports it.
    """
    environment = dict(os.environ)
    environment["ACCELERATOR_MEASURE_CACHE_ROOT"] = str(plugin_root / "bin")
    environment["ACCELERATOR_MEASURE_VERSION"] = version
    environment["ACCELERATOR_MEASURE_SUBBINARY"] = subbinary
    completed = subprocess.run(
        [
            "cargo",
            "test",
            "--release",
            "--manifest-path",
            str(plugin_root / "cli/Cargo.toml"),
            "-p",
            "accelerator",
            "--test",
            "warm_terms",
            "--",
            "--ignored",
            "--nocapture",
        ],
        check=False,
        capture_output=True,
        text=True,
        env=environment,
    )
    if completed.returncode != 0:
        raise PreconditionFailureError(
            f"the term harness failed:\n{completed.stderr[-2000:]}"
        )
    return parse_term_report(completed.stdout)


def parse_term_report(stdout: str) -> dict[str, Interval]:
    """Parse the term harness's JSON lines into intervals by term name."""
    terms: dict[str, Interval] = {}
    for line in stdout.splitlines():
        stripped = line.strip()
        if not stripped.startswith("{") or '"term"' not in stripped:
            continue
        payload = json.loads(stripped)
        terms[payload["term"]] = Interval(
            point=payload["median_ms"],
            lower=payload["p2_5_ms"],
            upper=payload["p97_5_ms"],
        )
    return terms


def measure_digest_bracket(
    runner: MeasurementRunner,
    *,
    farm: Path,
    cwd: Path,
    temp_root: Path,
    targets: Sequence[Path],
    samples: int = FLOOR_SAMPLES,
    backend: str = FAST_BACKEND,
) -> float:
    """Cost of the bootstrap's two digest substitutions, directly.

    A `bash -c` bracket marginal over an empty body. The digests are asserted
    because an absent backend makes the substitution empty rather than failing
    the script, so an unmeasured bracket would return a plausible small number.
    """
    environment = farm_environment(farm, temp_root=temp_root)
    bash = str(farm / "bash")
    invocation = (
        f"{FALLBACK_BACKEND} -a 256"
        if backend == FALLBACK_BACKEND
        else FAST_BACKEND
    )
    body = "; ".join(
        f"{invocation} \"{target}\" | awk '{{print $1}}'" for target in targets
    )
    loaded = [
        runner([bash, "-c", body], cwd=cwd, env=environment)
        for _ in range(samples)
    ]
    digests = loaded[0].stdout.split()
    if len(digests) != len(targets) or not all(
        len(digest) == SHA256_HEX_LENGTH for digest in digests
    ):
        raise PreconditionFailureError(
            f"the {backend} bracket computed {digests} rather than "
            f"{len(targets)} digests — the backend is missing from the farm at "
            f"{farm}, and the timing would be of a failed lookup: "
            f"{loaded[0].stderr[:200]}"
        )
    empty = [
        runner([bash, "-c", ":"], cwd=cwd, env=environment)
        for _ in range(samples)
    ]
    return median([s.elapsed_ms for s in loaded]) - median(
        [s.elapsed_ms for s in empty]
    )


def ancestor_pids(diagnostics: DiagnosticRunner) -> set[int]:
    """Every pid from this process up to init.

    The driving session is an ancestor several frames up, so excluding
    `getpid`/`getppid` alone would count it as a competitor and refuse the run.
    """
    pids: set[int] = set()
    current = os.getpid()
    while current > 1 and current not in pids:
        pids.add(current)
        try:
            parent = diagnostics(["ps", "-o", "ppid=", "-p", str(current)])
        except FileNotFoundError, PermissionError:
            break
        if not parent.strip().isdigit():
            break
        current = int(parent.strip())
    return pids


def concurrent_sessions(diagnostics: DiagnosticRunner) -> list[str]:
    """List Claude Code sessions other than the one driving this run.

    Matched on the executable name exactly: a command-line match also catches
    the desktop app's helpers and any command mentioning the word. A concurrent
    session can flip a cached inode, failing an integrity witness for a benign
    reason indistinguishable from tampering.
    """
    try:
        listing = diagnostics(["pgrep", "-x", "claude"])
    except FileNotFoundError, PermissionError:
        return []
    own = ancestor_pids(diagnostics)
    return [
        line.strip()
        for line in listing.splitlines()
        if line.strip().isdigit() and int(line.strip()) not in own
    ]


def tool_provenance(
    farm: Path, diagnostics: DiagnosticRunner
) -> dict[str, dict[str, str]]:
    """Every farm link's target realpath and version, re-probed through it.

    Re-probed through the farm rather than trusted from the pre-build probe: a
    mise shim re-resolves its version from the config found at the cwd, and the
    sampling cwd is outside every mise config.
    """
    record = {}
    for link in sorted(farm.iterdir()):
        try:
            version = diagnostics([str(link), "--version"])
        except FileNotFoundError, PermissionError, subprocess.SubprocessError:
            version = "unknown"
        record[link.name] = {
            "target": str(link.resolve()),
            "version": version.splitlines()[0] if version else "unknown",
        }
    return record


def plugin_version(plugin_root: Path) -> str:
    payload = json.loads(
        (plugin_root / ".claude-plugin/plugin.json").read_text()
    )
    return str(payload["version"])


def jj_pin(plugin_root: Path) -> str:
    """Read the `jj` version `mise.toml` pins.

    Asserted rather than merely recorded: the fixture's colocation flag is
    justified by one release's default, so a differently versioned `jj` could
    change which mode the fixture is in.
    """
    import tomllib

    tools = tomllib.loads((plugin_root / "mise.toml").read_text())["tools"]
    return str(tools["jj"])
