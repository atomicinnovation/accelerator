"""Pure analysis core for the warm-dispatch latency measurement.

Everything decidable about a session is a function over recorded observations,
so a defect surfaces as a failing test rather than as a plausible-looking
figure. The driver in `tasks/measure.py` holds what needs a process, a clock or
a filesystem, and the numeric constants it is judged against.
"""

from __future__ import annotations

import json
import math
import platform
import random
import re
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum, unique
from pathlib import Path

# --- Summary statistics ---------------------------------------------------


def percentile(values: Sequence[float], quantile: float) -> float:
    """Linearly interpolated percentile over `values`.

    Interpolated rather than nearest-rank; the two differ materially at the
    sample sizes this runs at.
    """
    if not values:
        raise ValueError("percentile of an empty sample")
    if not 0.0 <= quantile <= 1.0:
        raise ValueError(f"quantile out of range: {quantile}")
    ordered = sorted(values)
    position = quantile * (len(ordered) - 1)
    low = math.floor(position)
    high = math.ceil(position)
    if low == high:
        return float(ordered[low])
    return float(
        ordered[low] + (position - low) * (ordered[high] - ordered[low])
    )


def median(values: Sequence[float]) -> float:
    """Median with the even-n midpoint averaged."""
    if not values:
        raise ValueError("median of an empty sample")
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return float(ordered[middle])
    return (ordered[middle - 1] + ordered[middle]) / 2


@dataclass(frozen=True)
class Summary:
    n: int
    minimum: float
    median: float
    p90: float
    iqr: float


def summarise(samples: Sequence[float]) -> Summary:
    if not samples:
        raise ValueError("cannot summarise an empty sample")
    return Summary(
        n=len(samples),
        minimum=float(min(samples)),
        median=median(samples),
        p90=percentile(samples, 0.9),
        iqr=percentile(samples, 0.75) - percentile(samples, 0.25),
    )


# --- Bootstrap intervals --------------------------------------------------


@dataclass(frozen=True)
class Interval:
    point: float
    lower: float
    upper: float

    @property
    def upper_distance(self) -> float:
        """Distance from the point estimate to the upper bound.

        The gated tail for every `statistic ≤ ceiling` cell, and asymmetric
        against the lower one on a right-skewed distribution, so a half-width
        would correspond to neither.
        """
        return self.upper - self.point


def _bounds(
    replicates: Sequence[float], point: float, confidence: float
) -> Interval:
    tail = (1.0 - confidence) / 2.0
    return Interval(
        point=point,
        lower=percentile(replicates, tail),
        upper=percentile(replicates, 1.0 - tail),
    )


def _resample_indices(rng: random.Random, size: int) -> Iterator[list[int]]:
    while True:
        yield [rng.randrange(size) for _ in range(size)]


def paired_ratio_interval(
    baseline: Sequence[float],
    variant: Sequence[float],
    *,
    resamples: int,
    confidence: float,
    rng: random.Random,
) -> Interval:
    """Percentile bootstrap on `median(variant) / median(baseline)`.

    Resamples pair indices, so each replicate keeps every drawn pair intact.
    Unequal-length inputs raise rather than being truncated to the shorter,
    which would misalign the pairs into a confident wrong interval.
    """
    if len(baseline) != len(variant):
        raise ValueError(
            f"paired vectors differ in length: {len(baseline)} vs "
            f"{len(variant)}"
        )
    if not baseline:
        raise ValueError("paired bootstrap over an empty sample")
    draws = _resample_indices(rng, len(baseline))
    replicates = []
    for _ in range(resamples):
        indices = next(draws)
        replicates.append(
            median([variant[i] for i in indices])
            / median([baseline[i] for i in indices])
        )
    point = median(variant) / median(baseline)
    return _bounds(replicates, point, confidence)


def unpaired_interval(
    samples: Sequence[float],
    *,
    statistic: Callable[[Sequence[float]], float],
    resamples: int,
    confidence: float,
    rng: random.Random,
) -> Interval:
    """Percentile bootstrap on a single variant's `statistic`.

    Unpaired, because a paired bootstrap over `(B, G)` pairs is not the
    estimator for a single-variant quantity.
    """
    if not samples:
        raise ValueError("unpaired bootstrap over an empty sample")
    draws = _resample_indices(rng, len(samples))
    replicates = [
        statistic([samples[i] for i in next(draws)]) for _ in range(resamples)
    ]
    return _bounds(replicates, statistic(samples), confidence)


def unpaired_ratio_interval(
    baseline: Sequence[float],
    variant: Sequence[float],
    *,
    resamples: int,
    confidence: float,
    rng: random.Random,
) -> Interval:
    """Percentile bootstrap on a ratio of medians over independent arms.

    Each arm is resampled to its own length, so arms of different length are
    admissible — which they must be, since the block this serves is single-arm
    and takes no baseline samples of its own.
    """
    if not baseline or not variant:
        raise ValueError("unpaired ratio over an empty arm")
    baseline_draws = _resample_indices(rng, len(baseline))
    variant_draws = _resample_indices(rng, len(variant))
    replicates = [
        median([variant[i] for i in next(variant_draws)])
        / median([baseline[i] for i in next(baseline_draws)])
        for _ in range(resamples)
    ]
    return _bounds(replicates, median(variant) / median(baseline), confidence)


def required_samples(
    pilot_n: int, achieved_distance: float, target_distance: float
) -> int:
    """Size a sample: `n = n0 * (h0 / target)^2`, over **upper** distances.

    Rounded up: a truncated n passes an approximate round-trip while
    systematically under-sampling.
    """
    if target_distance <= 0:
        raise ValueError("target distance must be positive")
    scaled = pilot_n * (achieved_distance / target_distance) ** 2
    return max(pilot_n, math.ceil(scaled))


# --- Sampling schedule ----------------------------------------------------


@unique
class Variant(StrEnum):
    BASELINE = "B"
    FAST = "G-fast"
    FALLBACK = "G-fallback"


@dataclass(frozen=True)
class Sample:
    block: str
    variant: Variant
    pair: int | None
    pilot: bool
    first_of_pair: bool = False


def _block_a_pairs(
    pairs: Sequence[int], *, pilot: bool, rng: random.Random
) -> list[Sample]:
    samples: list[Sample] = []
    for pair in pairs:
        order = [Variant.BASELINE, Variant.FAST]
        rng.shuffle(order)
        samples += [
            Sample(
                block="A",
                variant=variant,
                pair=pair,
                pilot=pilot,
                first_of_pair=position == 0,
            )
            for position, variant in enumerate(order)
        ]
    return samples


def _block_b_samples(count: int, *, pilot: bool) -> list[Sample]:
    return [
        Sample(block="B", variant=Variant.FALLBACK, pair=None, pilot=pilot)
        for _ in range(count)
    ]


def generate_schedule(
    *,
    block_a_pairs: int,
    block_b_samples: int,
    pilot_pairs: int,
    pilot_samples: int,
    segment: int,
    rng: random.Random,
) -> list[Sample]:
    """Build the two blocks' schedule: pilots first, then alternating.

    `segment` counts samples, so a paired block's segment holds half as many
    pairs. Alternating keeps monotone drift from landing wholly on one block
    while keeping the heavier block's load out of the gated pairs.
    """
    schedule = _block_a_pairs(range(pilot_pairs), pilot=True, rng=rng)
    schedule += _block_b_samples(pilot_samples, pilot=True)

    remaining_a = list(range(pilot_pairs, pilot_pairs + block_a_pairs))
    remaining_b = block_b_samples
    pairs_per_segment = max(1, segment // 2)
    while remaining_a or remaining_b:
        if remaining_a:
            batch, remaining_a = (
                remaining_a[:pairs_per_segment],
                remaining_a[pairs_per_segment:],
            )
            schedule += _block_a_pairs(batch, pilot=False, rng=rng)
        if remaining_b:
            batch_size = min(segment, remaining_b)
            remaining_b -= batch_size
            schedule += _block_b_samples(batch_size, pilot=False)
    return schedule


# --- Hook envelope normalisation ------------------------------------------


@unique
class Decision(StrEnum):
    ALLOW = "allow"
    BLOCK = "block"
    WARN = "warn"
    DEGRADED = "degraded"
    UNRECOGNISED = "unrecognised"


_LEGACY_DECISIONS = {"allow": Decision.ALLOW, "block": Decision.BLOCK}


def normalise_envelope(stdout: str) -> tuple[Decision, str]:
    """Normalise a guard's stdout to `(decision, reason)`.

    A total union over both guards' wire shapes. Empty stdout normalises to
    `degraded` rather than `allow`, because a fail-safe swallow and a genuine
    allow are the same bytes and only one of them is a valid sample. Denial is
    asserted rather than inferred from the presence of a reason.
    """
    text = stdout.strip()
    if not text:
        return (Decision.DEGRADED, "")
    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        return (Decision.DEGRADED, stdout)
    if not isinstance(payload, dict):
        return (Decision.UNRECOGNISED, stdout)

    if "decision" in payload:
        decision = _LEGACY_DECISIONS.get(str(payload["decision"]))
        if decision is not None:
            return (decision, str(payload.get("reason", "")))
        return (Decision.UNRECOGNISED, stdout)

    raw_hook = payload.get("hookSpecificOutput")
    hook: dict[str, object] = raw_hook if isinstance(raw_hook, dict) else {}
    if hook.get("permissionDecision") == "deny":
        return (Decision.BLOCK, str(hook.get("permissionDecisionReason", "")))
    for holder in (payload, hook):
        if "systemMessage" in holder:
            return (Decision.WARN, str(holder["systemMessage"]))
    return (Decision.UNRECOGNISED, stdout)


def expected_decision(probe_stdout: str) -> tuple[Decision, str]:
    """Derive the expected `(decision, reason)` from a pre-sampling probe.

    Refuses an expectation the sampling gate could not falsify: a degraded or
    unrecognised probe means the fixture's behaviour is unknown.
    """
    decision, reason = normalise_envelope(probe_stdout)
    if decision in {Decision.DEGRADED, Decision.UNRECOGNISED}:
        raise ValueError(
            f"probe produced no usable decision ({decision}) — refusing to "
            f"derive an expectation from it"
        )
    return (decision, reason)


@dataclass(frozen=True)
class SampleVerdict:
    valid: bool
    diagnostic: str
    baseline: tuple[Decision, str] | None = None
    variant: tuple[Decision, str] | None = None


def validate_dispatch(
    raw: str,
    expected_reason: str,
    *,
    label: str,
    stderr: str = "",
) -> SampleVerdict:
    """Assert one dispatch reached the expected blocked decision.

    The stderr is carried into the diagnostic because it is the only clue a
    degraded dispatch has: stdout is empty by construction and the exit code is
    0 whether the guard blocked, allowed or swallowed a failure.
    """
    observed = normalise_envelope(raw)
    noise = stderr.strip()
    explanation = f"; stderr: {noise[:400]}" if noise else ""
    if observed[0] is not Decision.BLOCK:
        return SampleVerdict(
            valid=False,
            diagnostic=(
                f"{label} did not block: normalised to {observed[0]} "
                f"({observed[1][:120]!r}){explanation}"
            ),
            variant=observed,
        )
    if observed[1] != expected_reason:
        return SampleVerdict(
            valid=False,
            diagnostic=(
                f"{label} blocked with an unexpected reason: "
                f"{observed[1]!r} != {expected_reason!r}"
            ),
            variant=observed,
        )
    return SampleVerdict(valid=True, diagnostic="", variant=observed)


def validate_sample(
    raw_b: str,
    raw_g: str,
    expected_reason: str,
    *,
    baseline_stderr: str = "",
    variant_stderr: str = "",
) -> SampleVerdict:
    """Assert both variants produced the same blocked decision on one stdin.

    The envelope is the only witness that a sample exercised the path being
    timed; the exit code is 0 either way.
    """
    baseline = normalise_envelope(raw_b)
    variant = normalise_envelope(raw_g)
    for raw, stderr, label in (
        (raw_b, baseline_stderr, "baseline"),
        (raw_g, variant_stderr, "variant"),
    ):
        verdict = validate_dispatch(
            raw, expected_reason, label=label, stderr=stderr
        )
        if not verdict.valid:
            return SampleVerdict(
                valid=False,
                diagnostic=verdict.diagnostic,
                baseline=baseline,
                variant=variant,
            )
    return SampleVerdict(
        valid=True, diagnostic="", baseline=baseline, variant=variant
    )


# --- Runaway brakes -------------------------------------------------------

WARM_UP_SAMPLES = 20
OUTLIER_MULTIPLE = 5.0
ABSOLUTE_OUTLIER_MS = 500.0


def outlier_trip(sample: float, *, arm_median: float, arm_count: int) -> bool:
    """Report whether `sample` is out of range for its own arm.

    Per arm rather than pooled, the arms differing by tens of milliseconds. The
    absolute ceiling governs until a running median means anything; a network
    re-fetch is orders of magnitude above either bound.
    """
    if arm_count < WARM_UP_SAMPLES:
        return sample > ABSOLUTE_OUTLIER_MS
    return sample > OUTLIER_MULTIPLE * arm_median


def budget_exhausted(
    elapsed: float, budget: float, samples_done: int, samples_max: int
) -> bool:
    return elapsed >= budget or samples_done >= samples_max


def drift_verdict(
    first_third: float, last_third: float, *, band: float
) -> bool:
    """Report whether the session's drift is within `band`."""
    return abs(last_third - first_third) <= band


def retry_budget(attempts_used: int, *, cap: int) -> bool:
    """Report whether another recorded attempt is permitted.

    Capped so that retrying until the numbers look good is not available.
    """
    return attempts_used < cap


def log_appended_lines(before: str, after: str) -> list[str]:
    """Lines appended to an append-only log, or `[]` when it is unchanged.

    Raises when the recorded prefix changed: a rewritten prefix is a different
    failure from a growth and must not be reported as a diff.
    """
    if not after.startswith(before):
        raise ValueError(
            "the unverified log is append-only, but its recorded prefix "
            "changed — the session cannot be attributed"
        )
    return after[len(before) :].splitlines()


# --- Outcome taxonomy -----------------------------------------------------


@unique
class CellKind(StrEnum):
    ABSOLUTE = "absolute"
    RATIO = "ratio"


@unique
class Validity(StrEnum):
    VALID = "valid"
    INVALID_PRE_SAMPLING = "invalid-pre-sampling"
    INVALID_POST_RUN = "invalid-post-run"


@unique
class Branch(StrEnum):
    PASS = "1"  # noqa: S105 — a taxonomy label, not a credential
    FAIL = "2"
    INDETERMINATE = "3"
    TERMINAL = "4"
    INVALID_PRE = "5a"
    INVALID_POST = "5b"
    INFEASIBLE = "6a"
    BUDGET = "6b"
    NOT_APPLICABLE = "7"


class IllFormedCellError(ValueError):
    """A classifier input whose `cell_kind` and `robustness_ok` disagree."""


def classify(
    *,
    cell_kind: CellKind,
    lower: float,
    upper: float,
    threshold: float,
    upper_distance: float,
    target_distance: float,
    robustness_ok: bool | None,
    escalations_used: int,
    validity: Validity,
    sizing_feasible: bool,
    applicable: bool,
    budget_exhausted: bool,
) -> Branch:
    """Select the one branch a cell's recorded state falls in.

    An ordered cascade, first match wins. Two junctions depend on that order: a
    spent escalation is checked before the positional branches, so one cannot
    be spent twice; and an invalid session outranks infeasible sizing, whose
    sizing is moot.
    """
    if (robustness_ok is None) is not (cell_kind is CellKind.ABSOLUTE):
        raise IllFormedCellError(
            f"{cell_kind} carries robustness_ok={robustness_ok!r}: ratio "
            f"cells take a bool, absolute cells take None"
        )

    if not applicable:
        return Branch.NOT_APPLICABLE
    if validity is Validity.INVALID_PRE_SAMPLING:
        return Branch.INVALID_PRE
    if validity is Validity.INVALID_POST_RUN:
        return Branch.INVALID_POST
    if not sizing_feasible:
        return Branch.INFEASIBLE
    if budget_exhausted:
        return Branch.BUDGET

    fails = lower > threshold
    passes = (
        upper <= threshold
        and upper_distance <= target_distance
        and (robustness_ok is not False)
    )
    if escalations_used >= 1 and not passes and not fails:
        return Branch.TERMINAL
    if fails:
        return Branch.FAIL
    if not passes:
        return Branch.INDETERMINATE
    return Branch.PASS


@dataclass(frozen=True)
class CellOutcome:
    cell: str
    gates: bool
    branch: Branch
    accepted_by: str | None = None


def closure_verdict(cells: Sequence[CellOutcome]) -> bool:
    """Report whether every gating cell permits the item to close.

    A cell that cannot be measured on this host is not evidence against
    closing, so it needs a recorded acceptance rather than blocking outright.
    Non-gating cells are ignored in every branch.
    """
    gating = [cell for cell in cells if cell.gates]
    if not gating:
        return False
    return all(
        cell.branch is Branch.PASS
        or (cell.branch is Branch.NOT_APPLICABLE and cell.accepted_by)
        for cell in gating
    )


# --- Host-environment predicates ------------------------------------------

# `ACCELERATOR_RELEASE_BASE_URL` is the only seam for pointing the bootstrap at
# anything other than the public release host. Rejecting it outright would
# hard-couple the harness to anonymous github.com egress, which a mirrored or
# proxied hand-off host cannot satisfy — so it is permitted and recorded in the
# provenance set instead, with any figures taken against a mirror marked so.
PERMITTED_OVERRIDES = ("ACCELERATOR_RELEASE_BASE_URL",)


def accelerator_override_keys(
    env: Mapping[str, str], *, permitted: Sequence[str] = PERMITTED_OVERRIDES
) -> list[str]:
    """Report the `ACCELERATOR_*` keys set in `env`, minus the permitted ones.

    Matches key names: matching `env` output would also match values, over
    which it is not line-oriented.
    """
    return sorted(
        key
        for key in env
        if key.startswith("ACCELERATOR_") and key not in permitted
    )


def ceiling_directories(tmpdir: Path) -> str:
    """Canonicalise a temp root for `GIT_CEILING_DIRECTORIES`.

    git ignores non-canonical entries and resolves no symlinks itself, and the
    primary host's temp root sits under one, so an uncanonicalised entry is
    silently ignored exactly where it matters.
    """
    return str(Path(tmpdir).resolve())


def tmp_containment(path: Path, tmproot: Path) -> bool:
    """Report whether `path` lies strictly beneath `tmproot`.

    Canonicalises both sides: a recorded canonical path and a `gettempdir()`
    root are otherwise different strings for the same directory.
    """
    resolved = Path(path).resolve()
    root = Path(tmproot).resolve()
    return resolved != root and root in resolved.parents


@dataclass(frozen=True)
class ArtefactState:
    inode: int
    mtime: float
    digest: str


def unchanged_artefacts(
    before: Mapping[str, ArtefactState | None],
    after: Mapping[str, ArtefactState | None],
) -> list[str]:
    """Report every witnessed artefact whose identity moved.

    A missing file counts: a re-fetch unlinks before it stores, and every
    non-hit route renames a fresh inode over the entry.
    """
    problems = []
    for name in sorted(set(before) | set(after)):
        was, now = before.get(name), after.get(name)
        if was == now:
            continue
        if now is None:
            problems.append(f"{name}: missing after the run")
        elif was is None:
            problems.append(f"{name}: appeared during the run")
        else:
            problems.append(f"{name}: {was} -> {now}")
    return problems


@dataclass(frozen=True)
class CpuProbes:
    cgroup_cpu_max: str | None
    process_cpu_count: int


def resolve_cpu_count(probes: CpuProbes) -> tuple[int, str]:
    """Resolve the CPU count, reporting which rung fired.

    A cgroup quota first, where one is set; the literal `max` means none is.
    cgroup v1 is out of scope, and the chain stops at `process_cpu_count`
    because the pinned interpreter always has it.
    """
    quota = (probes.cgroup_cpu_max or "").split()
    if len(quota) == 2 and quota[0] != "max":
        try:
            allowance = int(quota[0]) / int(quota[1])
        except ValueError, ZeroDivisionError:
            pass
        else:
            return (max(1, math.ceil(allowance)), "cgroup-v2")
    return (probes.process_cpu_count, "process-cpu-count")


def power_state(
    diagnostic_runner: Callable[[Sequence[str]], str],
    probes: Sequence[Sequence[str]],
) -> dict[str, str]:
    """Record each power probe's output, or `unknown` where it is absent.

    Additive so one harness runs on both OSes. Driven by the diagnostic runner,
    never the measurement one, whose farm holds only the variants' own tools.
    """
    state = {}
    for probe in probes:
        try:
            state[probe[0]] = diagnostic_runner(probe)
        except FileNotFoundError, PermissionError:
            state[probe[0]] = "unknown"
    return state


def pilot_sizing(
    pilot_interval: Interval, *, target: float, pilot_n: int, cap: int
) -> tuple[int, bool]:
    """Size the run from a pilot's achieved **upper** distance.

    An n beyond `cap` is infeasible rather than a licence to relax the target.
    """
    needed = required_samples(pilot_n, pilot_interval.upper_distance, target)
    return (needed, needed <= cap)


@dataclass(frozen=True)
class ResidualVerdict:
    total: float
    signed_residual: float
    absolute_residual: float
    band: float
    closed: bool
    remeasure: bool


RESIDUAL_FLOOR_MS = 1.5
RESIDUAL_ATTEMPT_CAP = 2


def residual_verdict(
    term_intervals: Sequence[Interval],
    observed_median: float,
    attempts_used: int,
) -> ResidualVerdict:
    """Close the composition budget against `max(floor, propagated)`.

    The floor keeps the band able to detect a term moving by as much as the
    changes under consideration; the propagated term stops it being tighter
    than the measurement can resolve. Triggered by the residual's magnitude: a
    sum of noisy medians lands negative roughly half the time, so a
    sign-triggered re-measurement would be a selection filter.
    """
    total = sum(term.point for term in term_intervals)
    propagated = math.sqrt(
        sum(term.upper_distance**2 for term in term_intervals)
    )
    band = max(RESIDUAL_FLOOR_MS, propagated)
    signed = total - observed_median
    closed = abs(signed) <= band
    return ResidualVerdict(
        total=total,
        signed_residual=signed,
        absolute_residual=abs(signed),
        band=band,
        closed=closed,
        remeasure=not closed
        and retry_budget(attempts_used, cap=RESIDUAL_ATTEMPT_CAP),
    )


# --- Per-platform calibration ---------------------------------------------

PLATFORM_KEY_ENV = "MEASURE_PLATFORM_KEY"


@dataclass(frozen=True)
class Calibration:
    """Where a platform entry's numbers came from.

    `(system, machine)` under-determines them: two hosts sharing a key can
    differ materially in shell startup and digest implementation, so without
    this one would be judged by the other's numbers while reporting as
    calibrated.
    """

    session: str
    chip: str
    bash: str | None
    shasum: str | None


@dataclass(frozen=True)
class PlatformEntry:
    key: str
    path_tools: tuple[str, ...]
    power_probes: tuple[Sequence[str], ...]
    median_ceiling_fast_ms: float
    p90_ceiling_fast_ms: float
    median_ceiling_fallback_ms: float
    p90_ceiling_fallback_ms: float
    bash_floor_ms: float
    true_floor_ms: float
    reference_bash: str
    calibration: Calibration | None


def platform_constants(
    key: tuple[str, str], table: Mapping[tuple[str, str], PlatformEntry]
) -> PlatformEntry | None:
    """Look up the calibrated entry for `key`, or `None` if absent.

    A key with no entry yields no gating verdict, only recorded context.
    """
    return table.get(key)


def calibration_holds(
    entry: PlatformEntry,
    *,
    observed_chip: str,
    observed_bash: str,
    observed_shasum: str,
) -> bool:
    """Report whether the observed host matches the entry's provenance."""
    if entry.calibration is None:
        return False
    return (
        unconfirmed_calibration_fields(
            entry,
            observed_chip=observed_chip,
            observed_bash=observed_bash,
            observed_shasum=observed_shasum,
        )
        == []
    )


def unconfirmed_calibration_fields(
    entry: PlatformEntry,
    *,
    observed_chip: str,
    observed_bash: str,
    observed_shasum: str,
) -> list[str]:
    """Name every provenance field this host does not confirm.

    An unrecorded field is unconfirmable rather than matching: treating it as
    agreement would report a verdict calibrated against a figure nobody
    measured.
    """
    if entry.calibration is None:
        return ["no calibration provenance at all"]
    observed = {
        "chip": observed_chip,
        "bash": observed_bash,
        "shasum": observed_shasum,
    }
    unconfirmed = []
    for field, seen in observed.items():
        recorded = getattr(entry.calibration, field)
        if recorded is None:
            unconfirmed.append(
                f"{field}: the {entry.calibration.session} session recorded "
                f"none, so {seen!r} confirms nothing"
            )
        elif recorded != seen:
            unconfirmed.append(f"{field}: {seen!r} != recorded {recorded!r}")
    return unconfirmed


def resolve_platform_key(
    *, option: str | None, env: Mapping[str, str]
) -> tuple[tuple[str, str], str]:
    """Resolve the platform key, reporting where it came from.

    The override is deliberately un-prefixed, since every `ACCELERATOR_*` key
    is rejected by the preconditions and so could not coexist with them.
    """
    if option:
        return (_split_key(option), "--platform-key")
    override = env.get(PLATFORM_KEY_ENV)
    if override:
        return (_split_key(override), PLATFORM_KEY_ENV)
    return ((platform.system(), platform.machine()), "host")


def _split_key(value: str) -> tuple[str, str]:
    system, _, machine = value.partition("/")
    if not system or not machine:
        raise ValueError(
            f"platform key must read '<system>/<machine>', got {value!r}"
        )
    return (system, machine)


# --- Paired statistics and diagnostics ------------------------------------


def subtract_floor(samples: Sequence[float], floor: float) -> list[float]:
    """Shift every sample down by a shared instrument floor, clamped at zero.

    Subtracting a shared floor makes a ratio larger, so raw medians are the
    lenient statistic for a `ratio ≤ k` gate and the subtracted form is the
    check on it, never the other way round.
    """
    return [max(0.0, sample - floor) for sample in samples]


def _require_pairs(baseline: Sequence[float], variant: Sequence[float]) -> None:
    if len(baseline) != len(variant):
        raise ValueError(
            f"paired vectors differ in length: {len(baseline)} vs "
            f"{len(variant)}"
        )
    if not baseline:
        raise ValueError("paired statistic over an empty sample")


def ratio_of_medians(
    baseline: Sequence[float], variant: Sequence[float]
) -> float:
    _require_pairs(baseline, variant)
    return median(variant) / median(baseline)


def median_of_ratios(
    baseline: Sequence[float], variant: Sequence[float]
) -> float:
    """Take the median of the per-pair ratios, not of the medians.

    Pairing enters the gated statistic only through the resampling, so the two
    diverge under drift and their divergence is itself a diagnostic.
    """
    _require_pairs(baseline, variant)
    return median([g / b for b, g in zip(baseline, variant, strict=True) if b])


def thirds(
    samples: Sequence[float],
) -> tuple[list[float], list[float]]:
    """Split into equal first and last thirds, dropping the remainder.

    Equal-sized, or the larger window's dispersion could masquerade as drift.
    """
    size = len(samples) // 3
    if size == 0:
        return ([], [])
    return (list(samples[:size]), list(samples[len(samples) - size :]))


def drift_statistic(
    baseline: Sequence[float], variant: Sequence[float]
) -> float:
    """Measure the gated ratio's shift from the first third to the last.

    On the gated quantity rather than per variant: a shift in one arm alone can
    move the ratio while each arm's own band looks clean, and a common drift in
    both would discard a session that is fine.
    """
    _require_pairs(baseline, variant)
    first_b, last_b = thirds(baseline)
    first_g, last_g = thirds(variant)
    if not first_b:
        return 0.0
    return ratio_of_medians(last_b, last_g) - ratio_of_medians(first_b, first_g)


def _permuted_statistics(
    baseline: Sequence[float],
    variant: Sequence[float],
    *,
    permutations: int,
    rng: random.Random,
) -> list[float]:
    """Sample the drift statistic over random orderings of the same pairs.

    Permuting the order destroys temporal structure while preserving the
    pairing and both arms' dispersion, giving the statistic's distribution
    under no drift at this sample size and this instrument's spread.
    """
    _require_pairs(baseline, variant)
    pairs = list(zip(baseline, variant, strict=True))
    observed = []
    for _ in range(permutations):
        rng.shuffle(pairs)
        observed.append(
            abs(drift_statistic([b for b, _ in pairs], [g for _, g in pairs]))
        )
    return sorted(observed)


def drift_band_from_permutation(
    baseline: Sequence[float],
    variant: Sequence[float],
    *,
    permutations: int,
    quantile: float,
    rng: random.Random,
) -> float:
    """Derive a drift band with a stated false-positive rate.

    The `quantile` of the no-drift null, which a stationary session exceeds
    `1 - quantile` of the time by construction. Derived from the null rather
    than the observed drift, so it is not circular: scrambling the session's
    order leaves the band unchanged while changing the statistic completely.

    Necessarily depends on the sample size and the dispersion, so it is a
    procedure per session rather than a constant.
    """
    return percentile(
        _permuted_statistics(
            baseline, variant, permutations=permutations, rng=rng
        ),
        quantile,
    )


def drift_significance(
    baseline: Sequence[float],
    variant: Sequence[float],
    *,
    permutations: int,
    rng: random.Random,
) -> float:
    """Report the share of no-drift orderings drifting at least this much.

    Says how far outside the null a session sits, where the band says only
    whether it is outside.
    """
    observed = abs(drift_statistic(baseline, variant))
    null = _permuted_statistics(
        baseline, variant, permutations=permutations, rng=rng
    )
    return sum(1 for value in null if value >= observed) / len(null)


def dirname_spawn_count(trace: str) -> int:
    """Count `dirname` spawns in a `bash -x` trace.

    Observed rather than implied by fixture depth. A count independent of depth
    means no depth pinning is needed across hosts.
    """
    return sum(
        1
        for line in trace.splitlines()
        if line.lstrip("+ ").split(" ")[0].rsplit("/", 1)[-1] == "dirname"
    )


# --- Spawn enumeration ----------------------------------------------------

# Shell keywords and builtins: present in the text but never a `PATH`
# lookup, so a farm that lacks them is not a farm that changes behaviour.
_SHELL_WORDS = frozenset(
    (
        "break",
        "builtin",
        "case",
        "cd",
        "command",
        "continue",
        "declare",
        "do",
        "done",
        "echo",
        "elif",
        "else",
        "esac",
        "eval",
        "exec",
        "exit",
        "export",
        "false",
        "fi",
        "for",
        "function",
        "getopts",
        "hash",
        "if",
        "in",
        "jobs",
        "let",
        "local",
        "printf",
        "pwd",
        "read",
        "readonly",
        "return",
        "set",
        "shift",
        "shopt",
        "source",
        "test",
        "then",
        "times",
        "trap",
        "true",
        "type",
        "typeset",
        "ulimit",
        "umask",
        "unset",
        "until",
        "wait",
        "while",
    )
)

# Every executable either variant is known to reach. Keeping the vocabulary
# closed is what makes the enumeration decidable: the alternative is guessing
# which bare words in comments and error strings are commands.
_KNOWN_EXECUTABLES = frozenset(
    (
        "awk",
        "basename",
        "cat",
        "chmod",
        "cp",
        "curl",
        "cut",
        "date",
        "dirname",
        "find",
        "git",
        "grep",
        "head",
        "jj",
        "jq",
        "kill",
        "ln",
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
        "sort",
        "stat",
        "tail",
        "timeout",
        "tr",
        "uname",
        "wc",
        "wget",
        "xargs",
    )
)

_COMMAND_POSITION = re.compile(
    r"""(?:^|\||&&|\|\||;|\$\(|`|\bthen\b|\belse\b|\bdo\b|!\s)\s*
        ([a-z][a-z0-9_.-]*)""",
    re.MULTILINE | re.VERBOSE,
)
_EXPLICIT_LOOKUP = re.compile(r"command -v ([a-z][a-z0-9_.-]*)")


def spawned_executables(text: str) -> set[str]:
    """Enumerate the external executables a shell script reaches for.

    Derived rather than transcribed: a tool missing from the farm exits 0 under
    a fail-safe dispatch, recording a spuriously low latency instead of
    failing. Locally defined functions are excluded, since they shadow any
    same-named executable.
    """
    functions = set(re.findall(r"^\s*(\w+)\(\)", text, re.MULTILINE))
    candidates = {match.group(1) for match in _COMMAND_POSITION.finditer(text)}
    candidates |= set(_EXPLICIT_LOOKUP.findall(text))
    return {
        name
        for name in candidates
        if name in _KNOWN_EXECUTABLES
        and name not in _SHELL_WORDS
        and name not in functions
    }
