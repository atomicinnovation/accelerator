"""Unit tests for the warm-dispatch measurement harness.

The harness splits into a pure analysis core (`tasks/shared/measurement.py`)
and a subprocess driver (`tasks/measure.py`). Everything decidable is a pure
function over recorded observations, because the measurement session runs once
and a defect in it yields a plausible-looking ratio rather than a crash.
"""

import json
import os
import platform
import random
import re
import shutil
import signal
import subprocess
from dataclasses import replace
from itertools import pairwise
from pathlib import Path
from types import SimpleNamespace

import pytest

from tasks.measure import (
    BASELINE_COMMIT,
    CACHE_TEMP_PREFIX,
    FALLBACK_BACKEND,
    FAST_BACKEND,
    FLOOR_RETRY_CAP,
    GUARDED_FILES,
    MANIFEST_DIRNAME,
    MANIFEST_NAME,
    PLATFORM_TABLE,
    RATIO_TARGET,
    RATIO_THRESHOLD,
    RECOVERED_FILES,
    WALL_CLOCK_BUDGET_S,
    ArtefactKind,
    Manifest,
    MeasurementSession,
    PreconditionFailureError,
    RunResult,
    StaleManifestError,
    assert_backends,
    backend_delta_check,
    build_farm,
    calibration_note,
    cells_for,
    classify_cell,
    create_fixture,
    criterion_constants,
    digest_backend_population,
    farm_environment,
    gate_floors,
    jj_pin,
    last_floors,
    measure_digest_bracket,
    measure_floors,
    next_record_paths,
    parse_term_report,
    plugin_version,
    prime_cache,
    recover_baseline,
    recovery_argv,
    staged_shim_targets,
    unwind_signals,
    warm_cache_gaps,
)
from tasks.shared.measurement import (
    PLATFORM_KEY_ENV,
    ArtefactState,
    Branch,
    Calibration,
    CellKind,
    CellOutcome,
    CpuProbes,
    Decision,
    IllFormedCellError,
    Interval,
    PlatformEntry,
    Validity,
    Variant,
    accelerator_override_keys,
    budget_exhausted,
    calibration_holds,
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
    normalise_envelope,
    outlier_trip,
    paired_ratio_interval,
    percentile,
    pilot_sizing,
    platform_constants,
    power_state,
    ratio_of_medians,
    required_samples,
    residual_verdict,
    resolve_cpu_count,
    resolve_platform_key,
    retry_budget,
    spawned_executables,
    subtract_floor,
    summarise,
    thirds,
    tmp_containment,
    unchanged_artefacts,
    unpaired_interval,
    unpaired_ratio_interval,
    validate_sample,
)

REPO = Path(__file__).resolve().parents[3]
README = REPO / "tasks/README.md"
CONSTANTS_HEADING = "### Criterion constants"


def rng() -> random.Random:
    return random.Random(20260813)


def deny_envelope(reason: str) -> str:
    return json.dumps(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        }
    )


BLOCK_REASON = (
    "This is a pure jujutsu repository. Use jj instead of git status. "
    "Equivalent: jj status"
)
LEGACY_BLOCK = json.dumps({"decision": "block", "reason": BLOCK_REASON})


class TestSummaryStatistics:
    def test_percentile_interpolates_linearly(self):
        # Hand-computed: with n = 4 the 90th percentile sits at index
        # 0.9 * 3 = 2.7, i.e. 30 + 0.7 * (40 - 30).
        assert percentile([10, 20, 30, 40], 0.9) == pytest.approx(37.0)
        assert percentile([10, 20, 30, 40], 0.25) == pytest.approx(17.5)
        assert percentile([10, 20, 30, 40], 0.75) == pytest.approx(32.5)

    def test_percentile_at_the_bounds_returns_the_extremes(self):
        assert percentile([5, 6, 7], 0.0) == 5
        assert percentile([5, 6, 7], 1.0) == 7

    def test_median_of_an_even_sample_averages_the_two_middles(self):
        assert summarise([1, 2, 3, 4]).median == pytest.approx(2.5)

    def test_median_of_an_odd_sample_is_the_middle(self):
        assert summarise([1, 2, 3, 4, 5]).median == pytest.approx(3.0)

    def test_summary_carries_n_min_median_p90_and_iqr(self):
        summary = summarise([10, 20, 30, 40])
        assert summary.n == 4
        assert summary.minimum == 10
        assert summary.median == pytest.approx(25.0)
        assert summary.p90 == pytest.approx(37.0)
        assert summary.iqr == pytest.approx(15.0)

    def test_an_empty_sample_raises(self):
        with pytest.raises(ValueError, match="empty"):
            summarise([])


class TestPairedRatioInterval:
    def test_bounds_bracket_the_point_estimate(self):
        baseline = [30 + (i % 7) for i in range(200)]
        variant = [40 + (i % 5) for i in range(200)]
        interval = paired_ratio_interval(
            baseline, variant, resamples=200, confidence=0.95, rng=rng()
        )
        assert interval.lower <= interval.point <= interval.upper

    def test_the_same_seed_yields_identical_bounds(self):
        baseline = [30 + (i % 7) for i in range(120)]
        variant = [40 + (i % 5) for i in range(120)]
        first = paired_ratio_interval(
            baseline, variant, resamples=200, confidence=0.95, rng=rng()
        )
        second = paired_ratio_interval(
            baseline, variant, resamples=200, confidence=0.95, rng=rng()
        )
        assert (first.lower, first.point, first.upper) == (
            second.lower,
            second.point,
            second.upper,
        )

    def test_a_wider_confidence_level_never_narrows_the_interval(self):
        baseline = [30 + (i % 7) for i in range(200)]
        variant = [40 + (i % 5) for i in range(200)]
        narrow = paired_ratio_interval(
            baseline, variant, resamples=400, confidence=0.80, rng=rng()
        )
        wide = paired_ratio_interval(
            baseline, variant, resamples=400, confidence=0.99, rng=rng()
        )
        assert wide.upper - wide.lower >= narrow.upper - narrow.lower

    def test_zero_variance_input_collapses_to_zero_width(self):
        interval = paired_ratio_interval(
            [30.0] * 50, [42.0] * 50, resamples=100, confidence=0.95, rng=rng()
        )
        assert interval.lower == interval.upper == pytest.approx(1.4)

    def test_unequal_length_vectors_raise_rather_than_being_truncated(self):
        with pytest.raises(ValueError, match="length"):
            paired_ratio_interval(
                [30.0] * 10,
                [42.0] * 9,
                resamples=10,
                confidence=0.95,
                rng=rng(),
            )

    def test_the_resample_count_is_honoured(self):
        draws: list[int] = []

        class CountingRandom(random.Random):
            def randrange(self, *args, **kwargs):  # type: ignore[override]
                draws.append(1)
                return super().randrange(*args, **kwargs)

        paired_ratio_interval(
            [30.0] * 4,
            [42.0] * 4,
            resamples=25,
            confidence=0.95,
            rng=CountingRandom(1),
        )
        assert len(draws) == 25 * 4

    def test_upper_distance_measures_the_gated_tail(self):
        baseline = [30 + (i % 7) for i in range(200)]
        variant = [40 + (i % 5) for i in range(200)]
        interval = paired_ratio_interval(
            baseline, variant, resamples=200, confidence=0.95, rng=rng()
        )
        assert interval.upper_distance == pytest.approx(
            interval.upper - interval.point
        )


class TestUnpairedInterval:
    def test_bounds_bracket_the_point_estimate_of_the_statistic(self):
        samples = [40 + (i % 11) * 0.5 for i in range(300)]
        interval = unpaired_interval(
            samples,
            statistic=lambda values: summarise(values).median,
            resamples=200,
            confidence=0.95,
            rng=rng(),
        )
        assert interval.lower <= interval.point <= interval.upper

    def test_it_carries_the_statistic_over_the_whole_sample_as_the_point(self):
        samples = [40.0, 41.0, 42.0, 43.0]
        interval = unpaired_interval(
            samples,
            statistic=lambda values: summarise(values).p90,
            resamples=50,
            confidence=0.95,
            rng=rng(),
        )
        assert interval.point == pytest.approx(percentile(samples, 0.9))


class TestSizing:
    def test_it_round_trips_the_pilot_when_the_target_is_the_achieved(self):
        assert required_samples(200, 0.0086, 0.0086) == 200

    def test_a_tighter_target_scales_by_the_square_of_the_ratio(self):
        assert required_samples(300, 0.0086, 0.0043) == 1200

    def test_a_non_integral_result_rounds_up(self):
        # 200 * (0.0086 / 0.0060)^2 = 410.9 — truncation would under-sample.
        assert required_samples(200, 0.0086, 0.0060) == 411

    def test_a_looser_target_never_returns_fewer_than_the_pilot(self):
        assert required_samples(200, 0.0086, 0.02) == 200

    def test_a_non_positive_target_raises(self):
        with pytest.raises(ValueError, match="positive"):
            required_samples(200, 0.0086, 0.0)


class TestScheduleGeneration:
    def schedule(self, **kwargs):
        defaults = {
            "block_a_pairs": 300,
            "block_b_samples": 250,
            "pilot_pairs": 50,
            "pilot_samples": 50,
            "segment": 100,
            "rng": rng(),
        }
        return generate_schedule(**{**defaults, **kwargs})

    def analysed(self, schedule):
        return [sample for sample in schedule if not sample.pilot]

    def test_each_block_reaches_its_own_n(self):
        schedule = self.analysed(self.schedule())
        pairs = {s.pair for s in schedule if s.block == "A"}
        b_samples = [s for s in schedule if s.block == "B"]
        assert len(pairs) == 300
        assert len(b_samples) == 250

    def test_every_block_a_pair_holds_both_variants_exactly_once(self):
        by_pair: dict[int, list[Variant]] = {}
        for sample in self.schedule():
            if sample.block == "A":
                by_pair.setdefault(sample.pair, []).append(sample.variant)
        assert by_pair
        for variants in by_pair.values():
            assert sorted(variants) == sorted([Variant.BASELINE, Variant.FAST])

    def test_block_b_samples_never_enter_a_pair(self):
        for sample in self.schedule():
            if sample.block == "B":
                assert sample.pair is None
                assert sample.variant is Variant.FALLBACK

    def test_segments_alternate_between_the_blocks(self):
        blocks = [s.block for s in self.analysed(self.schedule())]
        runs = [blocks[0]]
        runs += [b for a, b in pairwise(blocks) if a != b]
        assert len(runs) > 2, "a batched all-A-then-all-B schedule is rejected"
        assert set(runs) == {"A", "B"}

    def test_the_pilot_segments_run_first_and_are_flagged(self):
        schedule = self.schedule()
        pilot_len = len([s for s in schedule if s.pilot])
        assert pilot_len == 50 * 2 + 50
        assert all(s.pilot for s in schedule[:pilot_len])
        assert not any(s.pilot for s in schedule[pilot_len:])

    def test_pilot_pairs_are_disjoint_from_the_analysed_pairs(self):
        schedule = self.schedule()
        pilot = {s.pair for s in schedule if s.pilot and s.block == "A"}
        analysed = {s.pair for s in self.analysed(schedule) if s.block == "A"}
        assert pilot and analysed
        assert not pilot & analysed

    def test_a_fixed_seed_reproduces_the_schedule(self):
        first = self.schedule(rng=random.Random(7))
        second = self.schedule(rng=random.Random(7))
        assert first == second

    def test_within_pair_order_is_randomised_rather_than_alternated(self):
        firsts = [
            s.variant
            for s in self.schedule(block_a_pairs=400)
            if s.block == "A" and s.first_of_pair
        ]
        alternating = [
            Variant.BASELINE if i % 2 == 0 else Variant.FAST
            for i in range(len(firsts))
        ]
        assert set(firsts) == {Variant.BASELINE, Variant.FAST}
        assert firsts != alternating


class TestEnvelopeNormalisation:
    def test_the_legacy_shape_is_taken_verbatim(self):
        assert normalise_envelope(LEGACY_BLOCK) == (
            Decision.BLOCK,
            BLOCK_REASON,
        )

    def test_a_legacy_allow_is_taken_verbatim(self):
        envelope = json.dumps({"decision": "allow", "reason": ""})
        assert normalise_envelope(envelope) == (Decision.ALLOW, "")

    def test_a_rust_deny_normalises_to_block_and_never_to_allow(self):
        decision, reason = normalise_envelope(deny_envelope(BLOCK_REASON))
        assert decision is Decision.BLOCK
        assert reason == BLOCK_REASON

    def test_a_top_level_system_message_normalises_to_warn(self):
        envelope = json.dumps({"systemMessage": "colocated"})
        assert normalise_envelope(envelope) == (Decision.WARN, "colocated")

    def test_a_nested_system_message_normalises_to_warn(self):
        envelope = json.dumps(
            {"hookSpecificOutput": {"systemMessage": "colocated"}}
        )
        assert normalise_envelope(envelope) == (Decision.WARN, "colocated")

    def test_empty_stdout_is_degraded_rather_than_allow(self):
        assert normalise_envelope("")[0] is Decision.DEGRADED
        assert normalise_envelope("   \n")[0] is Decision.DEGRADED

    def test_unparseable_stdout_is_degraded(self):
        assert normalise_envelope("not json at all")[0] is Decision.DEGRADED

    def test_a_permission_decision_of_allow_is_unrecognised_not_block(self):
        envelope = json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "permissionDecisionReason": "fine",
                }
            }
        )
        decision, reason = normalise_envelope(envelope)
        assert decision is Decision.UNRECOGNISED
        assert envelope in reason

    def test_a_session_start_envelope_is_unrecognised(self):
        envelope = json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": "ctx",
                }
            }
        )
        assert normalise_envelope(envelope)[0] is Decision.UNRECOGNISED

    def test_a_json_scalar_is_unrecognised(self):
        assert normalise_envelope("42")[0] is Decision.UNRECOGNISED

    @pytest.mark.parametrize(
        "row",
        json.loads(
            (
                Path(__file__).resolve().parents[3]
                / "cli/vcs-test-support/fixtures/vcs-guard/decision-table.json"
            ).read_text()
        )[:40],
    )
    def test_every_golden_row_round_trips_through_its_wire_shape(self, row):
        decision, reason = row["decision"], row["reason_pattern"]
        if decision == "block":
            wire = deny_envelope(reason)
            expected = (Decision.BLOCK, reason)
        elif decision == "warn":
            wire = json.dumps({"systemMessage": reason})
            expected = (Decision.WARN, reason)
        else:
            wire = ""
            expected = (Decision.DEGRADED, "")
        assert normalise_envelope(wire) == expected


class TestValidateSample:
    def test_a_matching_block_pair_is_accepted(self):
        verdict = validate_sample(
            LEGACY_BLOCK, deny_envelope(BLOCK_REASON), BLOCK_REASON
        )
        assert verdict.valid
        assert verdict.diagnostic == ""

    @pytest.mark.parametrize(
        "raw_g",
        [
            "",
            json.dumps({"systemMessage": "colocated"}),
            "{not json",
            json.dumps({"hookSpecificOutput": {"hookEventName": "X"}}),
        ],
        ids=["empty", "warn", "malformed", "unrecognised"],
    )
    def test_every_degraded_variant_shape_is_rejected(self, raw_g):
        verdict = validate_sample(LEGACY_BLOCK, raw_g, BLOCK_REASON)
        assert not verdict.valid
        assert verdict.diagnostic

    def test_the_wrong_reason_text_is_rejected(self):
        verdict = validate_sample(
            LEGACY_BLOCK, deny_envelope("some other reason"), BLOCK_REASON
        )
        assert not verdict.valid
        assert "reason" in verdict.diagnostic

    def test_a_degraded_baseline_is_rejected_too(self):
        verdict = validate_sample("", deny_envelope(BLOCK_REASON), BLOCK_REASON)
        assert not verdict.valid


class TestBrakes:
    def test_the_absolute_ceiling_governs_before_the_warm_up_window(self):
        assert not outlier_trip(499.0, arm_median=42.0, arm_count=19)
        assert outlier_trip(501.0, arm_median=42.0, arm_count=19)

    def test_the_warm_up_window_ends_at_twenty_samples(self):
        # At 19 samples the running median is not yet trusted, so a 6x
        # sample passes; at 20 the 5x rule takes over and trips it.
        assert not outlier_trip(252.0, arm_median=42.0, arm_count=19)
        assert outlier_trip(252.0, arm_median=42.0, arm_count=20)
        assert outlier_trip(252.0, arm_median=42.0, arm_count=21)

    def test_five_times_the_running_median_is_the_boundary(self):
        assert not outlier_trip(210.0, arm_median=42.0, arm_count=50)
        assert outlier_trip(210.1, arm_median=42.0, arm_count=50)

    def test_the_wall_clock_budget_is_exhausted_at_its_limit(self):
        assert not budget_exhausted(2099.0, 2100.0, 100, 1700)
        assert budget_exhausted(2100.0, 2100.0, 100, 1700)

    def test_the_sample_cap_exhausts_the_budget_too(self):
        assert not budget_exhausted(10.0, 2100.0, 1699, 1700)
        assert budget_exhausted(10.0, 2100.0, 1700, 1700)

    def test_drift_is_judged_on_the_gated_ratio_against_its_band(self):
        assert drift_verdict(1.2800, 1.2849, band=0.005)
        assert not drift_verdict(1.2800, 1.2851, band=0.005)
        assert not drift_verdict(1.2851, 1.2800, band=0.005)

    def test_the_retry_budget_permits_three_attempts_and_no_fourth(self):
        assert retry_budget(0, cap=3)
        assert retry_budget(2, cap=3)
        assert not retry_budget(3, cap=3)


class TestLogAppendedLines:
    def test_byte_identical_contents_are_accepted(self):
        assert log_appended_lines("a\nb\n", "a\nb\n") == []

    def test_an_appended_line_invalidates_rather_than_truncating(self):
        appended = log_appended_lines("a\n", "a\nb\n")
        assert appended == ["b"]

    def test_a_rewritten_prefix_is_reported_rather_than_diffed(self):
        with pytest.raises(ValueError, match="append-only"):
            log_appended_lines("a\n", "z\n")


def cell_states():
    """Every well-formed classifier input, by relative position.

    `cell_kind` and `robustness_ok` are coupled — `bool` for ratio cells,
    `None` for absolute ones — so they are enumerated as pairs rather than
    crossed, which would make branch 1 and branch 3 both unmatchable for
    `(ratio, None)` and leave the cascade with no verdict.
    """
    kinds = [
        (CellKind.ABSOLUTE, None),
        (CellKind.RATIO, True),
        (CellKind.RATIO, False),
    ]
    threshold = 50.0
    positions = [
        (threshold + 1, threshold + 2),
        (threshold, threshold + 1),
        (threshold - 1, threshold + 1),
        (threshold - 1, threshold),
        (threshold - 2, threshold - 1),
    ]
    for kind, robustness in kinds:
        for lower, upper in positions:
            for escalations in (0, 1):
                for validity in Validity:
                    for sizing in (True, False):
                        for applicable in (True, False):
                            for budget in (True, False):
                                for distance in (0.5, 2.0):
                                    yield {
                                        "cell_kind": kind,
                                        "lower": lower,
                                        "upper": upper,
                                        "threshold": threshold,
                                        "upper_distance": distance,
                                        "target_distance": 1.0,
                                        "robustness_ok": robustness,
                                        "escalations_used": escalations,
                                        "validity": validity,
                                        "sizing_feasible": sizing,
                                        "applicable": applicable,
                                        "budget_exhausted": budget,
                                    }


class TestClassifier:
    @pytest.mark.parametrize("state", cell_states())
    def test_every_well_formed_state_selects_exactly_one_branch(self, state):
        assert isinstance(classify(**state), Branch)

    def test_the_domain_is_covered_by_the_enumeration(self):
        branches = {classify(**state) for state in cell_states()}
        assert branches == set(Branch)

    @pytest.mark.parametrize(
        ("kind", "robustness"),
        [
            (CellKind.ABSOLUTE, True),
            (CellKind.ABSOLUTE, False),
            (CellKind.RATIO, None),
        ],
    )
    def test_an_ill_formed_kind_and_robustness_pair_raises(
        self, kind, robustness
    ):
        with pytest.raises(IllFormedCellError):
            classify(
                cell_kind=kind,
                lower=1.0,
                upper=2.0,
                threshold=3.0,
                upper_distance=0.5,
                target_distance=1.0,
                robustness_ok=robustness,
                escalations_used=0,
                validity=Validity.VALID,
                sizing_feasible=True,
                applicable=True,
                budget_exhausted=False,
            )

    def state(self, **overrides):
        base = {
            "cell_kind": CellKind.ABSOLUTE,
            "lower": 40.0,
            "upper": 45.0,
            "threshold": 50.0,
            "upper_distance": 0.5,
            "target_distance": 1.0,
            "robustness_ok": None,
            "escalations_used": 0,
            "validity": Validity.VALID,
            "sizing_feasible": True,
            "applicable": True,
            "budget_exhausted": False,
        }
        return {**base, **overrides}

    def test_an_interval_below_the_ceiling_passes(self):
        assert classify(**self.state()) is Branch.PASS

    def test_the_upper_bound_exactly_at_the_ceiling_passes(self):
        assert classify(**self.state(upper=50.0)) is Branch.PASS

    def test_the_lower_bound_exactly_at_the_ceiling_is_indeterminate(self):
        assert (
            classify(**self.state(lower=50.0, upper=55.0))
            is Branch.INDETERMINATE
        )

    def test_a_lower_bound_above_the_ceiling_fails(self):
        assert classify(**self.state(lower=51.0, upper=55.0)) is Branch.FAIL

    def test_a_straddling_interval_is_indeterminate(self):
        assert (
            classify(**self.state(lower=45.0, upper=55.0))
            is Branch.INDETERMINATE
        )

    def test_an_imprecise_interval_is_indeterminate_before_escalation(self):
        assert (
            classify(**self.state(upper_distance=2.0)) is Branch.INDETERMINATE
        )

    def test_a_ratio_cell_failing_robustness_is_indeterminate(self):
        assert (
            classify(
                **self.state(cell_kind=CellKind.RATIO, robustness_ok=False)
            )
            is Branch.INDETERMINATE
        )

    def test_a_ratio_cell_clearing_robustness_passes(self):
        assert (
            classify(**self.state(cell_kind=CellKind.RATIO, robustness_ok=True))
            is Branch.PASS
        )

    @pytest.mark.parametrize(
        "overrides",
        [
            {"lower": 45.0, "upper": 55.0},
            {"cell_kind": CellKind.RATIO, "robustness_ok": False},
            {"upper_distance": 2.0},
        ],
        ids=["straddle", "robustness", "imprecise"],
    )
    def test_a_spent_escalation_is_terminal_for_each_cause(self, overrides):
        state = self.state(escalations_used=1, **overrides)
        assert classify(**state) is Branch.TERMINAL

    def test_a_spent_escalation_does_not_block_a_pass_or_a_fail(self):
        assert classify(**self.state(escalations_used=1)) is Branch.PASS
        assert (
            classify(**self.state(escalations_used=1, lower=51.0, upper=55.0))
            is Branch.FAIL
        )

    @pytest.mark.parametrize(
        ("validity", "expected"),
        [
            (Validity.INVALID_PRE_SAMPLING, Branch.INVALID_PRE),
            (Validity.INVALID_POST_RUN, Branch.INVALID_POST),
        ],
    )
    def test_an_invalid_session_selects_five_whatever_the_bounds(
        self, validity, expected
    ):
        assert classify(**self.state(validity=validity)) is expected
        assert (
            classify(**self.state(validity=validity, lower=51.0, upper=55.0))
            is expected
        )

    def test_infeasible_sizing_selects_six_a(self):
        assert (
            classify(**self.state(sizing_feasible=False)) is Branch.INFEASIBLE
        )

    def test_an_exhausted_budget_selects_six_b(self):
        assert classify(**self.state(budget_exhausted=True)) is Branch.BUDGET

    def test_an_inapplicable_cell_selects_seven_ahead_of_everything(self):
        state = self.state(
            applicable=False,
            validity=Validity.INVALID_PRE_SAMPLING,
            sizing_feasible=False,
            budget_exhausted=True,
        )
        assert classify(**state) is Branch.NOT_APPLICABLE

    def test_invalidity_precedes_infeasible_sizing(self):
        state = self.state(
            validity=Validity.INVALID_PRE_SAMPLING, sizing_feasible=False
        )
        assert classify(**state) is Branch.INVALID_PRE

    def test_a_budget_abort_precedes_a_terminal_indeterminate(self):
        state = self.state(
            budget_exhausted=True, escalations_used=1, lower=45.0, upper=55.0
        )
        assert classify(**state) is Branch.BUDGET


class TestClosureVerdict:
    def cells(self, **overrides):
        base = {
            "C1": Branch.PASS,
            "C2": Branch.PASS,
            "C3": Branch.PASS,
            "C4": Branch.PASS,
            "C5": Branch.PASS,
            "C6": Branch.FAIL,
        }
        base.update(overrides)
        return [
            CellOutcome(cell=cell, gates=cell != "C6", branch=branch)
            for cell, branch in base.items()
        ]

    def test_every_gating_cell_passing_closes(self):
        assert closure_verdict(self.cells())

    def test_c6_is_ignored_in_every_branch(self):
        for branch in Branch:
            assert closure_verdict(self.cells(C6=branch))

    @pytest.mark.parametrize(
        "branch",
        [
            Branch.FAIL,
            Branch.INDETERMINATE,
            Branch.TERMINAL,
            Branch.INVALID_PRE,
            Branch.INVALID_POST,
            Branch.INFEASIBLE,
            Branch.BUDGET,
        ],
    )
    def test_one_failing_gating_cell_blocks_closure(self, branch):
        assert not closure_verdict(self.cells(C3=branch))

    def test_a_branch_seven_gating_cell_needs_a_recorded_acceptance(self):
        cells = [
            c
            if c.cell != "C3"
            else CellOutcome("C3", gates=True, branch=Branch.NOT_APPLICABLE)
            for c in self.cells()
        ]
        assert not closure_verdict(cells)

    def test_a_branch_seven_gating_cell_with_acceptance_closes(self):
        cells = [
            c
            if c.cell != "C3"
            else CellOutcome(
                "C3",
                gates=True,
                branch=Branch.NOT_APPLICABLE,
                accepted_by="Toby Clemson",
            )
            for c in self.cells()
        ]
        assert closure_verdict(cells)

    def test_every_gating_cell_in_branch_seven_with_acceptance_closes(self):
        cells = [
            CellOutcome(
                c.cell,
                gates=c.gates,
                branch=Branch.NOT_APPLICABLE,
                accepted_by="Toby Clemson" if c.gates else None,
            )
            for c in self.cells()
        ]
        assert closure_verdict(cells)

    def test_an_empty_cell_set_does_not_close(self):
        assert not closure_verdict([])


class TestExpectedDecision:
    def test_a_legacy_probe_yields_its_own_decision(self):
        assert expected_decision(LEGACY_BLOCK) == (Decision.BLOCK, BLOCK_REASON)

    def test_a_rust_deny_probe_yields_block(self):
        assert expected_decision(deny_envelope("r")) == (Decision.BLOCK, "r")

    def test_a_warn_probe_yields_warn(self):
        envelope = json.dumps({"systemMessage": "colocated"})
        assert expected_decision(envelope) == (Decision.WARN, "colocated")

    def test_empty_stdout_refuses_rather_than_yielding_an_expectation(self):
        with pytest.raises(ValueError, match="no usable decision"):
            expected_decision("")

    def test_an_unrecognised_probe_refuses_too(self):
        with pytest.raises(ValueError, match="no usable decision"):
            expected_decision(json.dumps({"hookSpecificOutput": {}}))


class TestAcceleratorOverrideKeys:
    def test_it_reports_the_offending_key_names(self):
        env = {"PATH": "/usr/bin", "ACCELERATOR_CACHE_DIR": "/tmp/x"}
        assert accelerator_override_keys(env) == ["ACCELERATOR_CACHE_DIR"]

    def test_a_clean_environment_reports_nothing(self):
        assert accelerator_override_keys({"PATH": "/usr/bin"}) == []

    def test_the_release_base_url_is_permitted_but_still_reported(self):
        env = {"ACCELERATOR_RELEASE_BASE_URL": "https://mirror.example"}
        assert accelerator_override_keys(env) == []
        assert accelerator_override_keys(env, permitted=()) == [
            "ACCELERATOR_RELEASE_BASE_URL"
        ]

    def test_it_matches_names_not_values(self):
        env = {"EDITOR": "ACCELERATOR_BIN"}
        assert accelerator_override_keys(env) == []


class TestTmpContainment:
    def test_a_symlinked_temp_root_is_accepted_on_both_sides(self, tmp_path):
        real = tmp_path / "real"
        real.mkdir()
        link = tmp_path / "link"
        link.symlink_to(real)
        assert tmp_containment(link / "artefact", real)

    def test_a_path_outside_the_root_is_rejected(self, tmp_path):
        outside = tmp_path / "outside"
        root = tmp_path / "root"
        root.mkdir()
        assert not tmp_containment(outside, root)

    def test_the_root_itself_is_not_contained(self, tmp_path):
        assert not tmp_containment(tmp_path, tmp_path)


class TestCeilingDirectories:
    def test_it_canonicalises_a_symlinked_root(self, tmp_path):
        real = tmp_path / "real"
        real.mkdir()
        link = tmp_path / "link"
        link.symlink_to(real)
        assert ceiling_directories(link) == str(real.resolve())


class TestUnchangedArtefacts:
    def witness(self, **kwargs):
        base = {"inode": 1, "mtime": 100.0, "digest": "abc"}
        return {"launcher": ArtefactState(**{**base, **kwargs})}

    def test_identical_state_is_accepted(self):
        assert unchanged_artefacts(self.witness(), self.witness()) == []

    def test_a_changed_inode_is_reported(self):
        assert unchanged_artefacts(self.witness(), self.witness(inode=2))

    def test_a_changed_mtime_is_reported(self):
        assert unchanged_artefacts(self.witness(), self.witness(mtime=101.0))

    def test_a_changed_digest_is_reported(self):
        assert unchanged_artefacts(self.witness(), self.witness(digest="def"))

    def test_a_missing_file_is_reported_rather_than_ignored(self):
        problems = unchanged_artefacts(self.witness(), {"launcher": None})
        assert problems and "missing" in problems[0]

    def test_an_appearing_file_is_reported_too(self):
        problems = unchanged_artefacts({"launcher": None}, self.witness())
        assert problems


class TestResolveCpuCount:
    def test_a_resolving_cgroup_quota_wins(self):
        count, rung = resolve_cpu_count(
            CpuProbes(cgroup_cpu_max="200000 100000", process_cpu_count=10)
        )
        assert (count, rung) == (2, "cgroup-v2")

    def test_the_literal_max_means_the_rung_did_not_fire(self):
        count, rung = resolve_cpu_count(
            CpuProbes(cgroup_cpu_max="max 100000", process_cpu_count=10)
        )
        assert (count, rung) == (10, "process-cpu-count")

    def test_an_absent_cgroup_falls_through(self):
        count, rung = resolve_cpu_count(
            CpuProbes(cgroup_cpu_max=None, process_cpu_count=8)
        )
        assert (count, rung) == (8, "process-cpu-count")

    def test_an_unparseable_quota_falls_through(self):
        count, rung = resolve_cpu_count(
            CpuProbes(cgroup_cpu_max="garbage", process_cpu_count=8)
        )
        assert (count, rung) == (8, "process-cpu-count")

    def test_a_fractional_quota_rounds_up_to_a_whole_cpu(self):
        count, rung = resolve_cpu_count(
            CpuProbes(cgroup_cpu_max="150000 100000", process_cpu_count=10)
        )
        assert (count, rung) == (2, "cgroup-v2")


class TestPowerState:
    def test_a_resolving_probe_is_recorded_verbatim(self):
        probes = [["pmset", "-g", "ps"]]
        state = power_state(lambda argv: f"output of {argv[0]}", probes)
        assert state == {"pmset": "output of pmset"}

    def test_an_absent_probe_yields_unknown_rather_than_propagating(self):
        def runner(argv):
            raise FileNotFoundError(argv[0])

        assert power_state(runner, [["pmset", "-g", "ps"]]) == {
            "pmset": "unknown"
        }


class TestPilotSizing:
    def test_h_zero_is_the_upper_distance_not_the_half_width(self):
        # 0.0151 below the point, 0.0086 above it: the half-width 0.0119
        # corresponds to neither tail.
        interval = Interval(point=1.2813, lower=1.2662, upper=1.2899)
        n, feasible = pilot_sizing(
            interval, target=0.0043, pilot_n=300, cap=7000
        )
        assert feasible
        assert n == required_samples(300, 0.0086, 0.0043)

    def test_a_worse_dispersion_pilot_sizes_up(self):
        tight = Interval(point=1.28, lower=1.27, upper=1.285)
        loose = Interval(point=1.28, lower=1.26, upper=1.30)
        assert (
            pilot_sizing(loose, target=0.003, pilot_n=200, cap=99999)[0]
            > pilot_sizing(tight, target=0.003, pilot_n=200, cap=99999)[0]
        )

    def test_a_target_beyond_the_cap_is_infeasible(self):
        interval = Interval(point=1.2813, lower=1.2662, upper=1.2899)
        n, feasible = pilot_sizing(
            interval, target=0.0001, pilot_n=300, cap=6900
        )
        assert not feasible
        assert n > 6900


class TestPlatformConstants:
    def table(self):
        return {
            ("Darwin", "arm64"): PlatformEntry(
                key="darwin-arm64",
                path_tools=("bash",),
                power_probes=(["pmset", "-g", "ps"],),
                median_ceiling_fast_ms=50.0,
                p90_ceiling_fast_ms=60.0,
                median_ceiling_fallback_ms=70.0,
                p90_ceiling_fallback_ms=80.0,
                bash_floor_ms=7.8,
                true_floor_ms=1.95,
                reference_bash="GNU bash, version 3.2.57",
                calibration=Calibration(
                    session="calibrating-session",
                    chip="Apple M4 Max",
                    bash="GNU bash, version 3.2.57",
                    shasum="Perl shasum 6.04",
                ),
            )
        }

    def test_a_calibrated_key_returns_its_entry(self):
        entry = platform_constants(("Darwin", "arm64"), self.table())
        assert entry is not None
        assert entry.median_ceiling_fast_ms == 50.0

    def test_an_uncalibrated_key_returns_nothing(self):
        assert platform_constants(("Linux", "riscv64"), self.table()) is None

    def test_a_non_host_key_exercises_the_other_platform_path(self):
        table = self.table()
        table[("Linux", "x86_64")] = PlatformEntry(
            key="linux-x64",
            path_tools=("bash",),
            power_probes=(),
            median_ceiling_fast_ms=45.0,
            p90_ceiling_fast_ms=55.0,
            median_ceiling_fallback_ms=65.0,
            p90_ceiling_fallback_ms=75.0,
            bash_floor_ms=5.0,
            true_floor_ms=1.0,
            reference_bash="GNU bash, version 5.2.21",
            calibration=None,
        )
        entry = platform_constants(("Linux", "x86_64"), table)
        assert entry is not None
        assert entry.key == "linux-x64"
        assert entry.calibration is None

    def test_the_host_table_carries_every_shipped_platform_key(self):
        assert set(PLATFORM_TABLE) >= {("Darwin", "arm64")}

    def test_a_calibrated_entry_demotes_on_a_provenance_mismatch(self):
        entry = platform_constants(("Darwin", "arm64"), self.table())
        assert entry is not None
        assert calibration_holds(
            entry,
            observed_chip="Apple M4 Max",
            observed_bash="GNU bash, version 3.2.57",
            observed_shasum="Perl shasum 6.04",
        )
        assert not calibration_holds(
            entry,
            observed_chip="Apple M1",
            observed_bash="GNU bash, version 3.2.57",
            observed_shasum="Perl shasum 6.04",
        )

    def test_an_uncalibrated_entry_never_holds(self):
        table = self.table()
        entry = table[("Darwin", "arm64")]
        bare = replace(entry, calibration=None)
        assert not calibration_holds(
            bare, observed_chip="x", observed_bash="y", observed_shasum="z"
        )


class TestPlatformKeyResolution:
    def test_the_host_key_is_the_default(self):
        key, source = resolve_platform_key(option=None, env={})
        assert source == "host"
        assert key == (platform.system(), platform.machine())

    def test_the_task_option_wins(self):
        key, source = resolve_platform_key(option="Linux/x86_64", env={})
        assert key == ("Linux", "x86_64")
        assert source == "--platform-key"

    def test_the_env_fallback_is_honoured_and_named(self):
        key, source = resolve_platform_key(
            option=None, env={PLATFORM_KEY_ENV: "Linux/aarch64"}
        )
        assert key == ("Linux", "aarch64")
        assert source == PLATFORM_KEY_ENV

    def test_the_override_env_var_carries_no_accelerator_prefix(self):
        assert not PLATFORM_KEY_ENV.startswith("ACCELERATOR_")


class TestResidualVerdict:
    def terms(self, total: float, spread: float = 0.2):
        # Four terms summing to `total`, each with the same upper distance.
        each = total / 4
        return [
            Interval(point=each, lower=each - spread, upper=each + spread)
            for _ in range(4)
        ]

    def test_a_residual_inside_the_band_closes(self):
        verdict = residual_verdict(self.terms(43.0), 42.28, attempts_used=0)
        assert verdict.closed
        assert not verdict.remeasure

    def test_the_band_never_falls_below_the_floor(self):
        verdict = residual_verdict(self.terms(43.0), 42.28, attempts_used=0)
        assert verdict.band == pytest.approx(1.5)

    def test_propagated_uncertainty_dominates_when_it_exceeds_the_floor(self):
        verdict = residual_verdict(
            self.terms(43.0, spread=2.0), 42.28, attempts_used=0
        )
        assert verdict.band == pytest.approx(4.0)

    def test_equal_magnitude_residuals_of_both_signs_are_treated_alike(self):
        over = residual_verdict(self.terms(45.0), 42.28, attempts_used=0)
        under = residual_verdict(self.terms(39.56), 42.28, attempts_used=0)
        assert over.closed == under.closed
        assert over.absolute_residual == pytest.approx(under.absolute_residual)
        assert over.signed_residual == pytest.approx(-under.signed_residual)

    def test_a_residual_outside_the_band_triggers_a_re_measurement(self):
        verdict = residual_verdict(self.terms(48.0), 42.28, attempts_used=0)
        assert not verdict.closed
        assert verdict.remeasure

    def test_the_second_attempt_exhausts_the_cap(self):
        verdict = residual_verdict(self.terms(48.0), 42.28, attempts_used=2)
        assert not verdict.closed
        assert not verdict.remeasure


class TestCriterionConstantsLockstep:
    """The doc block and `criterion_constants()` must agree, both ways.

    Deliberately does not read `meta/`: no test under `tests/` does, and corpus
    documents have their own gate. This table is the authoritative numeric
    source, so the two cannot drift without the doc block changing.
    """

    def block(self) -> dict[str, float]:
        text = README.read_text()
        start = text.index(CONSTANTS_HEADING)
        end = text.index("\n### ", start + len(CONSTANTS_HEADING))
        pattern = re.compile(r"^- `([^`]+)` = (-?[\d.]+)$", re.MULTILINE)
        return {
            name: float(value)
            for name, value in pattern.findall(text[start:end])
        }

    def test_the_section_exists(self):
        assert CONSTANTS_HEADING in README.read_text()

    def test_every_constant_appears_in_the_doc_block(self):
        missing = set(criterion_constants()) - set(self.block())
        assert not missing

    def test_every_documented_number_resolves_to_a_named_constant(self):
        stale = set(self.block()) - set(criterion_constants())
        assert not stale

    def test_every_documented_value_matches_its_constant(self):
        documented = self.block()
        for name, value in criterion_constants().items():
            assert documented[name] == pytest.approx(float(value)), name

    def test_the_gate_numbers_are_per_platform_rather_than_host_constants(self):
        keys = set(self.block())
        assert any(key.startswith("darwin-arm64.") for key in keys)
        assert "darwin-arm64.bash_floor_ms" in keys


class TestMeasureNamespaceDocs:
    def test_the_namespace_has_its_own_conventions_subsection(self):
        assert "### The measure namespace" in README.read_text()

    def test_it_states_every_prerequisite_a_run_needs(self):
        text = README.read_text()
        start = text.index("### The measure namespace")
        section = text[start : text.index("\n### Criterion constants")]
        for prerequisite in (
            "quiet",
            "network egress",
            "published signed release",
            "measure:teardown",
            ".accelerator-measure/manifest.json",
        ):
            assert prerequisite in section


class TestArtefactManifestCoverage:
    """Exhaustiveness is a property of the enumeration, not of a directory."""

    def source(self) -> str:
        return (REPO / "tasks/measure.py").read_text()

    def test_every_kind_is_created_through_the_register_seam(self):
        registered = set(
            re.findall(
                r"register_artefact\(\s*ArtefactKind\.([A-Z_]+)", self.source()
            )
        )
        assert registered == {kind.name for kind in ArtefactKind}

    def test_the_manifest_is_not_a_row_in_its_own_table(self):
        # It is the interlock token, not a managed artefact: the containment
        # guard admits the temp parent, bin/.tmp-* and the manifest directory,
        # and a manifest row would fail the absence assertion on every run.
        assert MANIFEST_NAME not in {kind.value for kind in ArtefactKind}

    def test_the_manifest_lives_outside_the_launchers_cache_root(self):
        assert MANIFEST_DIRNAME != "bin"
        assert not MANIFEST_DIRNAME.startswith(CACHE_TEMP_PREFIX)

    def test_the_manifest_directory_is_gitignored(self):
        ignored = (REPO / ".gitignore").read_text()
        assert f"/{MANIFEST_DIRNAME}/" in ignored


class FakeWitness:
    def __init__(self, entries=(), log=None):
        self._entries = list(entries)
        self._log = log
        self.removed: list[Path] = []

    def state(self, path):
        return ArtefactState(inode=1, mtime=1.0, digest="d") if path else None

    def entries(self, path):
        del path
        return sorted(self._entries)

    def read_text(self, path):
        del path
        return self._log

    def remove(self, path):
        self.removed.append(path)
        if path.is_dir() and not path.is_symlink():
            shutil.rmtree(path, ignore_errors=True)
        elif path.exists():
            path.unlink()


class FakeHost:
    def __init__(self, temp_root: Path, env=None):
        self._temp_root = temp_root
        self._env = env or {}

    def env(self):
        return self._env

    def loadavg(self):
        return (0.1, 0.2, 0.3)

    def cpu_probes(self):
        return CpuProbes(cgroup_cpu_max=None, process_cpu_count=8)

    def temp_root(self):
        return self._temp_root


def fake_session(plugin_root: Path, temp_root: Path, **kwargs):
    return MeasurementSession(
        plugin_root,
        witness=kwargs.pop("witness", FakeWitness()),
        diagnostics=kwargs.pop("diagnostics", lambda argv: ""),
        host=FakeHost(temp_root),
    )


@pytest.fixture
def plugin_root(tmp_path, monkeypatch):
    """A plugin root carrying just enough for the session to capture state."""
    root = tmp_path / "plugin"
    (root / "keys").mkdir(parents=True)
    (root / "bin").mkdir()
    (root / "scripts").mkdir()
    (root / "hooks").mkdir()
    shutil.copy(
        REPO / "keys/accelerator-release.pub",
        root / "keys/accelerator-release.pub",
    )
    for name in (
        "bin/accelerator",
        "scripts/vcs-common.sh",
        "hooks/hooks.json",
    ):
        (root / name).write_text("stub\n")
    return root


class TestMeasurementSession:
    def test_it_captures_and_then_verifies_a_clean_exit(
        self, plugin_root, tmp_path
    ):
        with fake_session(plugin_root, tmp_path) as session:
            assert session.baseline is not None
            assert session.manifest_path.exists()
        assert session.failures == []
        assert not session.manifest_path.exists()

    def test_it_refuses_to_start_while_a_stale_manifest_exists(
        self, plugin_root, tmp_path
    ):
        stale = plugin_root / MANIFEST_DIRNAME / MANIFEST_NAME
        stale.parent.mkdir(parents=True)
        stale.write_text("{}")
        with (
            pytest.raises(StaleManifestError, match="measure:teardown"),
            fake_session(plugin_root, tmp_path),
        ):
            pass

    def test_a_substituted_release_key_refuses_the_run(
        self, plugin_root, tmp_path
    ):
        (plugin_root / "keys/accelerator-release.pub").write_text("swapped\n")
        with (
            pytest.raises(PreconditionFailureError, match="published"),
            fake_session(plugin_root, tmp_path),
        ):
            pass

    def test_a_dirty_guarded_path_refuses_the_run(self, plugin_root, tmp_path):
        with (
            pytest.raises(PreconditionFailureError, match="sample one"),
            fake_session(
                plugin_root,
                tmp_path,
                diagnostics=lambda argv: "M bin/accelerator\n",
            ),
        ):
            pass

    def test_every_registered_artefact_is_removed_on_exit(
        self, plugin_root, tmp_path
    ):
        created = []
        with fake_session(plugin_root, tmp_path) as session:
            for kind in ArtefactKind:
                path = session.register_artefact(
                    kind, tmp_path / f"artefact-{kind.name}"
                )
                path.mkdir()
                created.append(path)
        assert session.failures == []
        assert all(not path.exists() for path in created)

    def test_an_artefact_outside_every_admitted_root_is_refused(
        self, plugin_root, tmp_path, monkeypatch
    ):
        outside = plugin_root / "keys"
        with fake_session(plugin_root, tmp_path) as session:
            session.manifest.artefacts["scratch-tree"] = str(outside)
        assert any("outside every admitted root" in f for f in session.failures)
        assert outside.exists()

    def test_a_surviving_artefact_is_reported_rather_than_ignored(
        self, plugin_root, tmp_path
    ):
        class RefusingWitness(FakeWitness):
            def remove(self, path):
                self.removed.append(path)

        with fake_session(
            plugin_root, tmp_path, witness=RefusingWitness()
        ) as session:
            path = session.register_artefact(
                ArtefactKind.FIXTURE_ROOT, tmp_path / "fixture"
            )
            path.mkdir()
        assert any("still present" in f for f in session.failures)

    def test_a_leaked_cache_temp_entry_is_reported_with_its_remedy(
        self, plugin_root, tmp_path
    ):
        witness = FakeWitness(entries=[])
        with fake_session(plugin_root, tmp_path, witness=witness) as session:
            witness._entries = [".tmp-accelerator-vcs-123-1"]
        assert any("leaked temp entry" in f for f in session.failures)
        assert any("rm -rf" in f for f in session.failures)

    def test_an_orphaned_lock_directory_is_reported_with_its_remedy(
        self, plugin_root, tmp_path
    ):
        witness = FakeWitness(entries=[])
        with fake_session(plugin_root, tmp_path, witness=witness) as session:
            witness._entries = [".accelerator-lock-darwin-arm64"]
        assert any("orphaned lock" in f for f in session.failures)
        assert any("rmdir" in f for f in session.failures)

    def test_a_grown_unverified_log_invalidates_the_session(
        self, plugin_root, tmp_path
    ):
        witness = FakeWitness(log="2026-08-13 pid=1 first\n")
        with fake_session(plugin_root, tmp_path, witness=witness) as session:
            witness._log = "2026-08-13 pid=1 first\n2026-08-13 pid=2 second\n"
        assert any("the unverified log grew" in f for f in session.failures)

    def test_a_created_unverified_log_invalidates_the_session(
        self, plugin_root, tmp_path
    ):
        witness = FakeWitness(log=None)
        with fake_session(plugin_root, tmp_path, witness=witness) as session:
            witness._log = "2026-08-13 pid=2 integrity failure\n"
        assert any("was created during the run" in f for f in session.failures)

    def test_a_mid_run_edit_to_a_guarded_path_is_reported(
        self, plugin_root, tmp_path
    ):
        with fake_session(plugin_root, tmp_path) as session:
            (plugin_root / "bin/accelerator").write_text("edited\n")
        assert any("bin/accelerator changed" in f for f in session.failures)

    def test_the_manifest_is_removed_even_when_verification_fails(
        self, plugin_root, tmp_path
    ):
        witness = FakeWitness(entries=[])
        with fake_session(plugin_root, tmp_path, witness=witness) as session:
            witness._entries = [".tmp-leaked"]
        assert session.failures
        assert not session.manifest_path.exists(), (
            "an unclearable failure must not wedge the harness shut"
        )

    def test_an_accelerator_override_at_exit_is_reported(
        self, plugin_root, tmp_path
    ):
        session = MeasurementSession(
            plugin_root,
            witness=FakeWitness(),
            diagnostics=lambda argv: "",
            host=FakeHost(tmp_path, env={"ACCELERATOR_VCS_BIN": "/tmp/x"}),
        )
        with session:
            pass
        assert any("overrides present at exit" in f for f in session.failures)


class TestTeardownReplay:
    def test_it_removes_the_artefacts_a_dead_run_left_behind(
        self, plugin_root, tmp_path
    ):
        leftover = tmp_path / "leftover"
        leftover.mkdir()
        manifest = Manifest(
            plugin_root=str(plugin_root),
            cache_root=str(plugin_root / "bin"),
            artefacts={"fixture-root": str(leftover)},
            baseline=None,
        )
        path = plugin_root / MANIFEST_DIRNAME / MANIFEST_NAME
        manifest.write(path)

        session = MeasurementSession(
            plugin_root, witness=FakeWitness(), diagnostics=lambda argv: ""
        )
        session.manifest = Manifest.load(path)
        session.restore()
        assert not leftover.exists()

    def test_a_manifest_round_trips_through_disk(self, plugin_root, tmp_path):
        manifest = Manifest(
            plugin_root=str(plugin_root),
            cache_root=str(plugin_root / "bin"),
            artefacts={"fast-farm": str(tmp_path / "farm")},
            baseline=None,
        )
        path = plugin_root / MANIFEST_DIRNAME / MANIFEST_NAME
        manifest.write(path)
        assert Manifest.load(path) == manifest


class TestFixtureConstruction:
    """Tested against a real `jj git init`, not a double.

    `jj` is pinned in `mise.toml` and the init is offline, so this stays
    hermetic — and it is the only way to test the property that matters. A
    colocated fixture emits **warn** rather than the blocked decision, so it is
    the one fixture defect that would invalidate a whole session without
    crashing anything; stubbing the init would leave it untested by
    construction.
    """

    def runner(self, argv):
        resolved = shutil.which(argv[0])
        if resolved is None:
            pytest.skip(f"{argv[0]} not on PATH")
        subprocess.run([resolved, *argv[1:]], check=True, capture_output=True)
        return ""

    def test_it_leaves_git_absent_and_jj_present(self, tmp_path):
        root = create_fixture(tmp_path / "fixture", runner=self.runner)
        assert (root / ".jj").is_dir()
        assert not (root / ".git").exists()

    def test_a_colocated_fixture_is_refused(self, tmp_path):
        def colocating(argv):
            resolved = shutil.which("jj")
            if resolved is None:
                pytest.skip("jj not on PATH")
            stripped = [a for a in argv[1:] if a != "git.colocate=false"]
            stripped = [a for a in stripped if a != "--config"]
            subprocess.run(
                [resolved, *stripped], check=True, capture_output=True
            )
            return ""

        with pytest.raises(PreconditionFailureError, match="colocated"):
            create_fixture(tmp_path / "colocated", runner=colocating)


class TestFarmConstruction:
    def test_the_fast_farm_carries_the_backend_and_the_fallback_does_not(
        self, tmp_path
    ):
        tools = ("bash", "true", FAST_BACKEND)
        if shutil.which(FAST_BACKEND) is None:
            pytest.skip(f"{FAST_BACKEND} not on PATH")
        fast = build_farm(tmp_path / "fast", tools, include_fast_backend=True)
        fallback = build_farm(
            tmp_path / "fallback", tools, include_fast_backend=False
        )
        assert (fast / FAST_BACKEND).exists()
        assert not (fallback / FAST_BACKEND).exists()

    def test_every_link_resolves_to_a_concrete_binary(self, tmp_path):
        farm = build_farm(
            tmp_path / "farm", ("bash",), include_fast_backend=False
        )
        target = (farm / "bash").readlink()
        assert target.is_absolute()
        assert target == target.resolve()

    def test_an_absent_tool_refuses_rather_than_degrading_silently(
        self, tmp_path
    ):
        with pytest.raises(PreconditionFailureError, match="absent from PATH"):
            build_farm(
                tmp_path / "farm",
                ("definitely-not-a-real-binary",),
                include_fast_backend=False,
            )

    def test_the_environment_pins_the_locale_and_the_git_ceiling(
        self, tmp_path
    ):
        env = farm_environment(tmp_path / "farm", temp_root=tmp_path)
        assert env["LC_ALL"] == "C"
        assert env["PATH"] == str(tmp_path / "farm")
        assert env["GIT_CEILING_DIRECTORIES"] == str(tmp_path.resolve())
        assert not accelerator_override_keys(env)


class TestBackendAssertions:
    def farms(
        self,
        tmp_path,
        *,
        fast_has_backend,
        fallback_has_backend,
        fallback_has_shasum=True,
    ):
        fast = tmp_path / "fast"
        fallback = tmp_path / "fallback"
        fast.mkdir()
        fallback.mkdir()
        if fast_has_backend:
            (fast / FAST_BACKEND).write_text("")
        if fallback_has_backend:
            (fallback / FAST_BACKEND).write_text("")
        if fallback_has_shasum:
            (fallback / FALLBACK_BACKEND).write_text("")
        return fast, fallback

    def test_both_farms_correctly_configured_pass(self, tmp_path):
        fast, fallback = self.farms(
            tmp_path, fast_has_backend=True, fallback_has_backend=False
        )
        assert_backends(fast, fallback)

    def test_a_fast_farm_missing_its_backend_is_refused(self, tmp_path):
        fast, fallback = self.farms(
            tmp_path, fast_has_backend=False, fallback_has_backend=False
        )
        with pytest.raises(
            PreconditionFailureError, match="fast farm resolves no"
        ):
            assert_backends(fast, fallback)

    def test_a_fallback_farm_resolving_the_backend_is_refused(self, tmp_path):
        fast, fallback = self.farms(
            tmp_path, fast_has_backend=True, fallback_has_backend=True
        )
        with pytest.raises(PreconditionFailureError, match="not the"):
            assert_backends(fast, fallback)

    def test_a_host_without_shasum_makes_the_fallback_cells_inapplicable(
        self, tmp_path
    ):
        fast, fallback = self.farms(
            tmp_path,
            fast_has_backend=True,
            fallback_has_backend=False,
            fallback_has_shasum=False,
        )
        with pytest.raises(PreconditionFailureError, match="branch 7"):
            assert_backends(fast, fallback)


class TestCellsAndClassification:
    def entry(self):
        return PLATFORM_TABLE[("Darwin", "arm64")]

    def test_six_cells_are_defined_with_five_gating(self):
        cells = cells_for(self.entry())
        assert [cell.name for cell in cells] == [
            "C1",
            "C2",
            "C3",
            "C4",
            "C5",
            "C6",
        ]
        assert [cell.gates for cell in cells] == [True] * 5 + [False]

    def test_the_absolute_cells_carry_the_platform_ceilings(self):
        cells = {cell.name: cell for cell in cells_for(self.entry())}
        entry = self.entry()
        assert cells["C1"].threshold == entry.median_ceiling_fast_ms
        assert cells["C2"].threshold == entry.p90_ceiling_fast_ms
        assert cells["C3"].threshold == entry.median_ceiling_fallback_ms
        assert cells["C4"].threshold == entry.p90_ceiling_fallback_ms

    def test_the_ratio_cells_carry_the_ratio_threshold_and_target(self):
        cells = {cell.name: cell for cell in cells_for(self.entry())}
        assert cells["C5"].threshold == RATIO_THRESHOLD
        assert cells["C5"].target == RATIO_TARGET
        assert cells["C5"].kind is CellKind.RATIO

    def test_a_cell_with_no_interval_is_not_applicable(self):
        cell = cells_for(self.entry())[2]
        outcome = classify_cell(
            cell,
            None,
            robustness_ok=None,
            escalations_used=0,
            validity=Validity.VALID,
            sizing_feasible=True,
            applicable=False,
            budget_spent=False,
        )
        assert outcome.branch is Branch.NOT_APPLICABLE
        assert outcome.gates

    def test_an_absolute_cell_under_its_ceiling_passes(self):
        cell = cells_for(self.entry())[0]
        outcome = classify_cell(
            cell,
            Interval(point=42.28, lower=41.8, upper=42.8),
            robustness_ok=None,
            escalations_used=0,
            validity=Validity.VALID,
            sizing_feasible=True,
            applicable=True,
            budget_spent=False,
        )
        assert outcome.branch is Branch.PASS


class TestRehearsals:
    """The abort paths, driven through the injected ports.

    A gate never shown to fire is not evidence, and on a happy-path session
    none of these ever runs — so their first execution would otherwise be the
    incident they exist to stop.
    """

    def test_the_validity_gate_refuses_every_non_block_shape(self):
        for stdout in (
            "",
            json.dumps({"decision": "allow", "reason": ""}),
            json.dumps({"systemMessage": "colocated"}),
            json.dumps({"hookSpecificOutput": {"hookEventName": "X"}}),
        ):
            verdict = validate_sample(LEGACY_BLOCK, stdout, BLOCK_REASON)
            assert not verdict.valid, stdout
            assert verdict.diagnostic

    def test_an_injected_outlier_trips_the_brake_with_its_diagnostic(self):
        arm = [42.0] * 30
        assert outlier_trip(420.0, arm_median=median(arm), arm_count=len(arm))

    def test_an_exhausted_budget_selects_branch_six_b(self):
        assert budget_exhausted(WALL_CLOCK_BUDGET_S, WALL_CLOCK_BUDGET_S, 1, 2)

    @pytest.mark.parametrize(
        "number",
        [signal.SIGINT, signal.SIGTERM, signal.SIGHUP],
        ids=["sigint", "sigterm", "sighup"],
    )
    def test_each_unwinding_signal_runs_restore_and_verify(
        self, plugin_root, tmp_path, number
    ):
        session = fake_session(plugin_root, tmp_path)
        with pytest.raises(KeyboardInterrupt), session as entered:
            artefacts = [
                entered.register_artefact(kind, tmp_path / kind.name)
                for kind in ArtefactKind
            ]
            for path in artefacts:
                path.mkdir()
            os.kill(os.getpid(), number)
        assert all(not path.exists() for path in artefacts), (
            "every artefact in the manifest table must be positively absent"
        )
        assert session.failures == []
        assert not session.manifest_path.exists()

    def test_all_three_signals_are_handled(self):
        assert set(unwind_signals()) == {
            signal.SIGINT,
            signal.SIGTERM,
            signal.SIGHUP,
        }


class TestRecoveryContract:
    """The most fragile of the volatile contracts the self-test protects."""

    def recovered(self, name: str) -> str:
        source, _ = RECOVERED_FILES[name]
        return source

    def test_both_files_are_recovered_at_one_revision(self):
        assert set(RECOVERED_FILES) == {
            "bin/vcs-guard",
            "scripts/vcs-common.sh",
        }

    def test_the_layout_resolves_the_guards_own_relative_dependency(self):
        # The recovered layout must satisfy the baseline's own relative
        # lookup of its dependency.
        assert self.recovered("bin/vcs-guard") == "hooks/vcs-guard.sh"
        assert (
            self.recovered("scripts/vcs-common.sh") == "scripts/vcs-common.sh"
        )

    def test_the_jj_form_uses_the_revset(self):
        argv = recovery_argv("hooks/vcs-guard.sh", engine="jj")
        assert argv[:2] == ["jj", "file"]
        assert "cf42441e2aad-" in argv

    def test_the_git_form_uses_the_resolved_commit_id(self):
        argv = recovery_argv("hooks/vcs-guard.sh", engine="git")
        assert argv[0] == "git"
        assert argv[-1] == f"{BASELINE_COMMIT}:hooks/vcs-guard.sh"
        assert len(BASELINE_COMMIT) == 40, (
            "a short prefix can be a jj change id, so the full commit id is "
            "what a git clone resolves"
        )

    def test_a_rotted_recovery_is_refused_rather_than_measured(self, tmp_path):
        with pytest.raises(PreconditionFailureError, match="rotted"):
            recover_baseline(
                tmp_path / "scratch", runner=lambda argv: "not the guard\n"
            )

    def test_the_recovered_files_match_their_recorded_digests(self, tmp_path):
        def runner(argv):
            resolved = shutil.which("jj")
            if resolved is None:
                pytest.skip("jj not on PATH")
            completed = subprocess.run(
                [resolved, *argv[1:]],
                check=False,
                capture_output=True,
                text=True,
                cwd=REPO,
            )
            if completed.returncode != 0:
                pytest.skip(f"revision unresolvable here: {completed.stderr}")
            return completed.stdout

        guard = recover_baseline(tmp_path / "scratch", runner=runner)
        assert guard.exists()
        assert os.access(guard, os.X_OK)
        assert (guard.parent.parent / "scripts/vcs-common.sh").exists()


class TestDigestBackendPopulation:
    def test_both_backends_are_reported_by_name(self):
        population = digest_backend_population()
        assert set(population) == {FAST_BACKEND, FALLBACK_BACKEND}

    def test_an_absent_backend_reports_none_rather_than_raising(self):
        for resolved in digest_backend_population().values():
            assert resolved is None or Path(resolved).exists()


class TestFloorTreatment:
    def test_subtracting_a_floor_shifts_every_sample_down(self):
        assert subtract_floor([10.0, 12.0], 2.0) == [8.0, 10.0]

    def test_a_floor_never_drives_a_sample_negative(self):
        assert subtract_floor([1.0], 2.0) == [0.0]

    def test_raw_medians_are_the_lenient_statistic_for_a_ratio_gate(self):
        # (G - c) / (B - c) > G / B, so subtracting a shared floor makes the
        # ratio larger — raw medians gate, the subtracted one is the check.
        baseline, variant = [33.0] * 20, [42.28] * 20
        raw = ratio_of_medians(baseline, variant)
        subtracted = ratio_of_medians(
            subtract_floor(baseline, 1.75), subtract_floor(variant, 1.75)
        )
        assert subtracted > raw

    def test_the_ratio_of_medians_is_not_the_median_of_ratios(self):
        baseline = [30.0, 40.0, 50.0]
        variant = [45.0, 44.0, 55.0]
        assert ratio_of_medians(baseline, variant) != pytest.approx(
            median_of_ratios(baseline, variant)
        )

    def test_the_median_of_ratios_pairs_element_wise(self):
        assert median_of_ratios([10.0, 20.0], [20.0, 60.0]) == pytest.approx(
            2.5
        )

    def test_unequal_vectors_raise_in_the_paired_statistic(self):
        with pytest.raises(ValueError, match="length"):
            median_of_ratios([1.0], [1.0, 2.0])


class TestThirds:
    def test_it_splits_a_paired_sequence_into_first_and_last_thirds(self):
        first, last = thirds(list(range(9)))
        assert first == [0, 1, 2]
        assert last == [6, 7, 8]

    def test_a_remainder_never_leaks_into_either_third(self):
        first, last = thirds(list(range(11)))
        assert len(first) == len(last) == 3
        assert first == [0, 1, 2]
        assert last == [8, 9, 10]

    def test_too_short_a_sequence_yields_empty_thirds(self):
        assert thirds([1, 2]) == ([], [])


class TestDirnameSpawnCount:
    def test_it_counts_dirname_invocations_in_a_trace(self):
        trace = "+ dir=/tmp/x\n+ dirname /tmp/x\n+ dirname /tmp\n+ echo done\n"
        assert dirname_spawn_count(trace) == 2

    def test_a_trace_with_no_spawns_counts_zero(self):
        # find_repo_root tests -e "$dir/.jj" on $PWD before its first dirname
        # call, and the sampling cwd is the fixture root — so the expected
        # count is zero at any depth, which makes B depth-insensitive.
        trace = "+ dir=/tmp/fixture\n+ [ -e /tmp/fixture/.jj ]\n"
        assert dirname_spawn_count(trace) == 0

    def test_a_dirname_inside_a_path_is_not_counted(self):
        assert dirname_spawn_count("+ cat /usr/bin/dirname-ish\n") == 0


class TestTermReportParsing:
    REPORT = (
        "running 1 test\n"
        '{"term":"cache::find","n":200,"median_ms":0.0232,'
        '"p2_5_ms":0.0171,"p97_5_ms":0.0677}\n'
        '{"term":"reverify","n":200,"median_ms":6.3964,'
        '"p2_5_ms":6.1949,"p97_5_ms":6.6337}\n'
        '{"asset_bytes":2493376}\n'
        "test result: ok.\n"
    )

    def test_it_reads_every_term_as_an_interval(self):
        terms = parse_term_report(self.REPORT)
        assert set(terms) == {"cache::find", "reverify"}
        assert terms["reverify"].point == pytest.approx(6.3964)
        assert terms["reverify"].upper == pytest.approx(6.6337)

    def test_non_term_lines_are_ignored_rather_than_parsed(self):
        assert "asset_bytes" not in parse_term_report(self.REPORT)

    def test_an_empty_report_yields_no_terms(self):
        assert parse_term_report("running 1 test\n") == {}

    def test_the_upper_distance_feeds_the_residual_band(self):
        terms = parse_term_report(self.REPORT)
        verdict = residual_verdict(list(terms.values()), 6.0, attempts_used=0)
        assert verdict.band == pytest.approx(1.5)
        assert verdict.total == pytest.approx(6.4196)


class TestLastFloors:
    def test_the_final_attempt_is_the_instrument_the_samples_used(self):
        floors = {
            "attempts": [
                {"bash_ms": 9.0, "true_ms": 3.0},
                {"bash_ms": 6.1, "true_ms": 1.4},
            ]
        }
        assert last_floors(floors) == (1.4, 6.1)

    def test_no_attempts_yields_zero_rather_than_raising(self):
        assert last_floors({}) == (0.0, 0.0)
        assert last_floors({"attempts": []}) == (0.0, 0.0)


class TestFloorGate:
    def entry(self):
        return PLATFORM_TABLE[("Darwin", "arm64")]

    def rig(self, tmp_path):
        farm = tmp_path / "farm"
        farm.mkdir()
        (farm / "bash").write_text("")
        (farm / "true").write_text("")
        script = tmp_path / "floor.sh"
        script.write_text("exit 0\n")
        return SimpleNamespace(
            fast_farm=farm,
            fixture=tmp_path,
            temp_parent=tmp_path,
            floor_script=script,
        )

    def runner_at(self, elapsed):
        def runner(argv, *, cwd, env):
            del cwd, env
            quiet = elapsed if "true" in argv[0] else elapsed * 4
            return RunResult(
                stdout="", stderr="", exit_code=0, elapsed_ms=quiet
            )

        return runner

    def test_a_quiet_host_clears_the_gate_on_the_first_attempt(self, tmp_path):
        report = gate_floors(
            self.entry(),
            self.rig(tmp_path),
            self.runner_at(1.4),
            when="pre",
        )
        assert report["holds"]
        assert len(report["attempts"]) == 1

    def test_a_noisy_host_retries_to_the_cap_and_then_fails(self, tmp_path):
        report = gate_floors(
            self.entry(),
            self.rig(tmp_path),
            self.runner_at(5.0),
            when="pre",
        )
        assert not report["holds"]
        assert len(report["attempts"]) == FLOOR_RETRY_CAP, (
            "every attempt is recorded — an operator retrying informally "
            "until the floors look good is optional stopping"
        )

    def test_a_farm_without_true_refuses_rather_than_reporting_zero(
        self, tmp_path
    ):
        rig = self.rig(tmp_path)
        (rig.fast_farm / "true").unlink()
        with pytest.raises(PreconditionFailureError, match="no `true`"):
            measure_floors(
                self.runner_at(1.0),
                floor_script=rig.floor_script,
                farm=rig.fast_farm,
                cwd=rig.fixture,
                temp_root=rig.temp_parent,
                samples=1,
            )


class TestStagedShimTargets:
    def test_it_names_the_source_shim_and_its_staged_copy(self, tmp_path):
        cache = tmp_path / "bin"
        cache.mkdir()
        source = cache / "accelerator-verify-darwin-arm64"
        staged = cache / f"accelerator-verify-darwin-arm64-{'a' * 64}"
        source.write_text("")
        staged.write_text("")
        targets = staged_shim_targets(tmp_path)
        assert targets == [source, staged]

    def test_two_targets_are_always_returned(self, tmp_path):
        cache = tmp_path / "bin"
        cache.mkdir()
        (cache / "accelerator-verify-darwin-arm64").write_text("")
        assert len(staged_shim_targets(tmp_path)) == 2


class TestPluginProvenance:
    def test_the_plugin_version_is_read_from_the_manifest(self):
        assert plugin_version(REPO).count(".") >= 2

    def test_the_jj_pin_is_read_from_mise_toml(self):
        assert jj_pin(REPO)[0].isdigit()


class TestSpawnedExecutables:
    """The farm's tool set is derived from the scripts, not from memory.

    A hand-written list is how the farm came to be missing `chmod`: the
    bootstrap's exec probe could not make its probe file executable, so it
    reported the cache root unwritable, and `--fail-safe` turned that into an
    exit 0 with empty stdout — the degraded shape that records a spuriously low
    latency. Enumerating mechanically removes the class of error rather than
    that instance of it.
    """

    def test_it_finds_a_command_at_the_start_of_a_pipeline(self):
        assert "chmod" in spawned_executables("chmod +x /tmp/x\n")

    def test_it_finds_a_command_inside_a_substitution(self):
        assert "uname" in spawned_executables("arch=$(uname -m)\n")

    def test_it_ignores_a_locally_defined_function(self):
        text = "sha256_file() {\n  true\n}\nsha256_file /tmp/x\n"
        assert "sha256_file" not in spawned_executables(text)

    def test_it_ignores_shell_builtins(self):
        assert spawned_executables("printf '%s' x\nexport A=b\n") == set()

    def test_it_ignores_a_bare_word_in_prose(self):
        assert spawned_executables("# we give the cache a chance\n") == set()

    def test_the_bootstrap_spawns_nothing_the_farm_lacks(self):
        text = (REPO / "bin/accelerator").read_text()
        missing = spawned_executables(text) - set(
            PLATFORM_TABLE[("Darwin", "arm64")].path_tools
        )
        assert not missing, (
            f"bin/accelerator spawns {sorted(missing)}, absent from the farm — "
            f"under --fail-safe each absence exits 0 with empty stdout, the "
            f"degraded shape that records a spuriously low latency"
        )

    def test_the_recovered_baseline_spawns_nothing_the_farm_lacks(self):
        tools = set(PLATFORM_TABLE[("Darwin", "arm64")].path_tools)
        for source, _ in RECOVERED_FILES.values():
            text = _recovered_text(source)
            assert not spawned_executables(text) - tools, source

    def test_the_bootstrap_needs_chmod_and_the_farm_has_it(self):
        # The specific spawn whose absence produced a degraded sample.
        text = (REPO / "bin/accelerator").read_text()
        assert "chmod" in spawned_executables(text)
        assert "chmod" in PLATFORM_TABLE[("Darwin", "arm64")].path_tools


def _recovered_text(source: str) -> str:
    resolved = shutil.which("jj")
    if resolved is None:
        pytest.skip("jj not on PATH")
    completed = subprocess.run(
        [resolved, "file", "show", "-r", "cf42441e2aad-", source],
        check=False,
        capture_output=True,
        text=True,
        cwd=REPO,
    )
    if completed.returncode != 0:
        pytest.skip("the baseline revision is unresolvable here")
    return completed.stdout


class TestDegradedSampleDiagnostic:
    def test_the_diagnostic_carries_the_dispatch_stderr(self):
        stderr = (
            "accelerator: no writable, exec-capable cache directory: "
            "/repo/bin is not writable"
        )
        verdict = validate_sample(
            LEGACY_BLOCK, "", BLOCK_REASON, variant_stderr=stderr
        )
        assert not verdict.valid
        assert "not writable" in verdict.diagnostic, (
            "a degraded sample's stderr is its only clue; discarding it turns "
            "a clear bootstrap message into an unexplained empty envelope"
        )

    def test_a_valid_sample_needs_no_stderr(self):
        verdict = validate_sample(
            LEGACY_BLOCK, deny_envelope(BLOCK_REASON), BLOCK_REASON
        )
        assert verdict.valid

    def test_the_baseline_stderr_is_carried_too(self):
        verdict = validate_sample(
            "",
            deny_envelope(BLOCK_REASON),
            BLOCK_REASON,
            baseline_stderr="vcs-guard: jq not found",
        )
        assert not verdict.valid
        assert "jq not found" in verdict.diagnostic

    def test_an_empty_stderr_leaves_the_diagnostic_unchanged(self):
        verdict = validate_sample(LEGACY_BLOCK, "", BLOCK_REASON)
        assert not verdict.valid
        assert verdict.diagnostic.endswith("'')")


class TestWarmCachePrecondition:
    def state(self, **overrides):
        base = {
            "version": "1.24.0-pre.41",
            "cache_root_entries": [
                "accelerator-launcher-1.24.0-pre.41-darwin-arm64",
                "accelerator-launcher-1.24.0-pre.41-darwin-arm64.minisig",
                "vcs-1.24.0-pre.41-" + "a" * 64,
                "vcs-1.24.0-pre.41-" + "a" * 64 + ".minisig",
            ],
            "platform": "darwin-arm64",
        }
        return {**base, **overrides}

    def test_a_warm_cache_for_this_version_satisfies_it(self):
        assert warm_cache_gaps(**self.state()) == []

    def test_a_cache_holding_only_an_older_version_is_named_as_the_gap(self):
        gaps = warm_cache_gaps(
            **self.state(
                cache_root_entries=[
                    "accelerator-launcher-1.24.0-pre.38-darwin-arm64",
                    "vcs-1.24.0-pre.38-" + "a" * 64,
                ]
            )
        )
        assert gaps
        assert any("1.24.0-pre.41" in gap for gap in gaps)

    def test_a_missing_signature_sidecar_is_a_gap(self):
        gaps = warm_cache_gaps(
            **self.state(
                cache_root_entries=[
                    "accelerator-launcher-1.24.0-pre.41-darwin-arm64",
                    "vcs-1.24.0-pre.41-" + "a" * 64,
                ]
            )
        )
        assert gaps

    def test_a_missing_sub_binary_is_a_gap(self):
        gaps = warm_cache_gaps(
            **self.state(
                cache_root_entries=[
                    "accelerator-launcher-1.24.0-pre.41-darwin-arm64",
                    "accelerator-launcher-1.24.0-pre.41-darwin-arm64.minisig",
                ]
            )
        )
        assert gaps
        assert any("vcs" in gap for gap in gaps)


class TestUnpairedRatioInterval:
    """C6's estimator: the two arms are independent, not paired.

    Block B is single-arm by design — it takes no `B` samples, because the
    fallback cells are absolute and C6 is not gated. Pairing its samples with
    Block A's baseline is therefore impossible, and the paired estimator
    silently produced no figure at all when the arms differed in length.
    """

    def test_the_point_estimate_is_the_ratio_of_the_two_medians(self):
        baseline = [31.0] * 400
        variant = [55.0] * 900
        interval = unpaired_ratio_interval(
            baseline, variant, resamples=100, confidence=0.95, rng=rng()
        )
        assert interval.point == pytest.approx(55.0 / 31.0)

    def test_arms_of_different_length_are_accepted(self):
        interval = unpaired_ratio_interval(
            [31 + (i % 5) for i in range(600)],
            [55 + (i % 7) for i in range(900)],
            resamples=200,
            confidence=0.95,
            rng=rng(),
        )
        assert interval.lower <= interval.point <= interval.upper

    def test_the_same_seed_reproduces_the_bounds(self):
        args = ([31.0, 32.0, 33.0] * 40, [55.0, 56.0] * 60)
        first = unpaired_ratio_interval(
            *args, resamples=150, confidence=0.95, rng=rng()
        )
        second = unpaired_ratio_interval(
            *args, resamples=150, confidence=0.95, rng=rng()
        )
        assert first == second

    def test_zero_variance_arms_collapse_to_zero_width(self):
        interval = unpaired_ratio_interval(
            [31.0] * 50, [55.0] * 90, resamples=80, confidence=0.95, rng=rng()
        )
        assert interval.lower == interval.upper

    def test_an_empty_arm_raises(self):
        with pytest.raises(ValueError, match="empty"):
            unpaired_ratio_interval(
                [], [55.0], resamples=10, confidence=0.95, rng=rng()
            )

    def test_it_reproduces_the_recorded_session_c6(self):
        # The invalidated session recorded median(B) = 31.085 and
        # median(G-fallback) = 55.474, which the paired estimator dropped.
        interval = unpaired_ratio_interval(
            [31.085] * 100,
            [55.474] * 100,
            resamples=50,
            confidence=0.95,
            rng=rng(),
        )
        assert interval.point == pytest.approx(1.7846, abs=1e-4)


class TestBackendDeltaCrossCheck:
    def test_the_delta_is_the_fallback_cost_less_the_fast_cost(self):
        check = backend_delta_check(fast_ms=5.79, fallback_ms=22.7)
        assert check["delta_ms"] == pytest.approx(16.91)

    def test_it_reports_the_implied_per_call_difference(self):
        # The delta covers two calls, so per call it is half of it.
        check = backend_delta_check(fast_ms=5.79, fallback_ms=22.7)
        assert check["implied_per_call_difference_ms"] == pytest.approx(8.455)

    def test_it_states_that_the_delta_is_not_the_absolute_figure(self):
        check = backend_delta_check(fast_ms=5.79, fallback_ms=22.7)
        assert "cross-check" in check["role"]
        assert check["absolute_under_the_gating_backend_ms"] == pytest.approx(
            5.79
        )

    def test_a_missing_fallback_measurement_yields_no_delta(self):
        check = backend_delta_check(fast_ms=5.79, fallback_ms=None)
        assert check["delta_ms"] is None


class TestRecordPaths:
    def test_the_first_attempt_is_numbered_one(self, tmp_path):
        paths = next_record_paths(tmp_path)
        assert paths.attempt == 1
        assert paths.record.name == "warm-dispatch-1.json"
        assert paths.samples.name == "warm-dispatch-1-samples.json"

    def test_an_existing_attempt_is_never_overwritten(self, tmp_path):
        (tmp_path / "warm-dispatch-1.json").write_text("{}")
        paths = next_record_paths(tmp_path)
        assert paths.attempt == 2
        assert not paths.record.exists(), (
            "an invalidated session's record is evidence of an attempt; a "
            "re-run must not clobber it"
        )

    def test_gaps_in_the_numbering_do_not_reuse_a_number(self, tmp_path):
        (tmp_path / "warm-dispatch-1.json").write_text("{}")
        (tmp_path / "warm-dispatch-3.json").write_text("{}")
        assert next_record_paths(tmp_path).attempt == 4

    def test_a_rehearsal_keeps_its_own_unnumbered_name(self, tmp_path):
        paths = next_record_paths(tmp_path, rehearse=True)
        assert paths.attempt == 0
        assert "rehearsal" in paths.record.name


class TestDigestBracket:
    """The bracket must fail loudly when its backend is absent.

    A backend missing from the farm makes `$(...)` empty rather than making the
    script fail, so the bracket times a failed lookup and returns a plausible
    small number — which is how the fallback-backend figure came back *smaller*
    than the fast one, implying a negative delta.
    """

    def farm(self, tmp_path):
        farm = tmp_path / "farm"
        farm.mkdir()
        (farm / "bash").write_text("")
        return farm

    def runner_returning(self, stdout, stderr=""):
        def runner(argv, *, cwd, env):
            del cwd, env
            empty = argv[-1] == ":"
            return RunResult(
                stdout="" if empty else stdout,
                stderr="" if empty else stderr,
                exit_code=0,
                elapsed_ms=4.0 if empty else 11.0,
            )

        return runner

    def targets(self, tmp_path):
        return [tmp_path / "one", tmp_path / "two"]

    def test_two_digests_give_the_marginal_cost(self, tmp_path):
        stdout = f"{'a' * 64}\n{'b' * 64}\n"
        marginal = measure_digest_bracket(
            self.runner_returning(stdout),
            farm=self.farm(tmp_path),
            cwd=tmp_path,
            temp_root=tmp_path,
            targets=self.targets(tmp_path),
            samples=2,
        )
        assert marginal == pytest.approx(7.0)

    def test_an_absent_backend_is_refused_rather_than_timed(self, tmp_path):
        with pytest.raises(PreconditionFailureError, match="missing from the"):
            measure_digest_bracket(
                self.runner_returning("", "bash: sha256sum: command not found"),
                farm=self.farm(tmp_path),
                cwd=tmp_path,
                temp_root=tmp_path,
                targets=self.targets(tmp_path),
                samples=2,
            )

    def test_the_absence_diagnostic_carries_the_stderr(self, tmp_path):
        with pytest.raises(PreconditionFailureError, match="command not found"):
            measure_digest_bracket(
                self.runner_returning("", "bash: shasum: command not found"),
                farm=self.farm(tmp_path),
                cwd=tmp_path,
                temp_root=tmp_path,
                targets=self.targets(tmp_path),
                samples=2,
                backend=FALLBACK_BACKEND,
            )

    def test_a_short_digest_is_refused(self, tmp_path):
        with pytest.raises(PreconditionFailureError):
            measure_digest_bracket(
                self.runner_returning("deadbeef\ncafebabe\n"),
                farm=self.farm(tmp_path),
                cwd=tmp_path,
                temp_root=tmp_path,
                targets=self.targets(tmp_path),
                samples=2,
            )

    def test_one_digest_for_two_targets_is_refused(self, tmp_path):
        with pytest.raises(PreconditionFailureError):
            measure_digest_bracket(
                self.runner_returning(f"{'a' * 64}\n"),
                farm=self.farm(tmp_path),
                cwd=tmp_path,
                temp_root=tmp_path,
                targets=self.targets(tmp_path),
                samples=2,
            )


class TestUnrecordedCalibration:
    """A provenance field the reference session never recorded cannot match.

    The criterion demotes a verdict to uncalibrated context when the observed
    host disagrees with the entry's provenance. That rule is only sound if the
    provenance is real. The calibrating session recorded its chip and its
    floors but not which `bash` or `shasum` it resolved, so asserting values for
    those two would confirm agreement with a figure nobody measured.
    """

    def entry(self, **overrides):
        base = {
            "session": "calibrating-session",
            "chip": "Apple M4 Max",
            "bash": None,
            "shasum": None,
        }
        return replace(
            PLATFORM_TABLE[("Darwin", "arm64")],
            calibration=Calibration(**{**base, **overrides}),
        )

    def test_an_unrecorded_field_cannot_be_confirmed(self):
        assert not calibration_holds(
            self.entry(),
            observed_chip="Apple M4 Max",
            observed_bash="GNU bash, version 5.3.15(1)-release",
            observed_shasum="6.02",
        )

    def test_a_fully_recorded_matching_provenance_holds(self):
        entry = self.entry(bash="GNU bash, version 5.3.15", shasum="6.02")
        assert calibration_holds(
            entry,
            observed_chip="Apple M4 Max",
            observed_bash="GNU bash, version 5.3.15",
            observed_shasum="6.02",
        )

    def test_the_shipped_darwin_entry_records_no_bash_or_shasum(self):
        # The calibrating session did not record them, so the table must not
        # claim them.
        calibration = PLATFORM_TABLE[("Darwin", "arm64")].calibration
        assert calibration is not None
        assert calibration.bash is None
        assert calibration.shasum is None

    def test_the_shipped_entry_never_reports_itself_calibrated(self):
        entry = PLATFORM_TABLE[("Darwin", "arm64")]
        assert not calibration_holds(
            entry,
            observed_chip="Apple M4 Max",
            observed_bash="anything",
            observed_shasum="anything",
        )

    def test_the_note_names_what_could_not_be_confirmed(self):
        note = calibration_note(
            PLATFORM_TABLE[("Darwin", "arm64")],
            chip="Apple M4 Max",
            bash="GNU bash, version 5.3.15",
            shasum="6.02",
        )
        assert "uncalibrated" in note
        assert "bash" in note or "shasum" in note


def stationary_pairs(n: int, seed: int = 11):
    """A stationary paired series: no temporal structure, realistic spread."""
    source = random.Random(seed)
    return (
        [source.gauss(28.0, 1.0) for _ in range(n)],
        [source.gauss(37.5, 1.0) for _ in range(n)],
    )


def drifting_pairs(n: int, shift: float, seed: int = 11):
    """The same series with the variant ramping linearly across the session."""
    baseline, variant = stationary_pairs(n, seed)
    return (
        baseline,
        [value + shift * index / n for index, value in enumerate(variant)],
    )


class TestDriftStatistic:
    def test_it_is_the_signed_last_third_minus_first_third_ratio(self):
        baseline = [10.0] * 30
        variant = [10.0] * 10 + [11.0] * 10 + [12.0] * 10
        assert drift_statistic(baseline, variant) == pytest.approx(0.2)

    def test_a_stationary_series_has_a_small_statistic(self):
        assert abs(drift_statistic(*stationary_pairs(900))) < 0.02

    def test_unequal_arms_raise(self):
        with pytest.raises(ValueError, match="length"):
            drift_statistic([1.0, 2.0], [1.0])


class TestDriftBandFromPermutation:
    """The band is derived from the null, never from the observed drift.

    Permuting the pair *order* destroys temporal structure while preserving the
    pairing and both arms' dispersion, so the resulting spread of the statistic
    is what no-drift looks like at this sample size. A quantile of it is a band
    with a stated false-positive rate — which the superseded constant, taken as
    a fraction of a margin the measurement disproved, did not have.
    """

    def band(self, baseline, variant, **kwargs):
        defaults = {"permutations": 300, "quantile": 0.95, "rng": rng()}
        return drift_band_from_permutation(
            baseline, variant, **{**defaults, **kwargs}
        )

    def test_the_band_is_positive_and_finite(self):
        assert self.band(*stationary_pairs(600)) > 0

    def test_it_is_reproducible_under_a_fixed_seed(self):
        args = stationary_pairs(600)
        assert self.band(*args) == self.band(*args)

    def test_a_stationary_series_falls_inside_its_own_band(self):
        baseline, variant = stationary_pairs(900)
        assert abs(drift_statistic(baseline, variant)) <= self.band(
            baseline, variant
        )

    def test_a_strongly_drifting_series_exceeds_its_band(self):
        baseline, variant = drifting_pairs(900, shift=4.0)
        assert abs(drift_statistic(baseline, variant)) > self.band(
            baseline, variant
        )

    def test_the_band_does_not_depend_on_the_observed_ordering(self):
        # The non-circularity property: the band measures the null, so
        # scrambling the input's order must leave it essentially unchanged even
        # though it changes the observed statistic beyond recognition.
        baseline, variant = drifting_pairs(600, shift=4.0)
        scrambled = list(zip(baseline, variant, strict=True))
        random.Random(3).shuffle(scrambled)
        rescrambled = (
            [b for b, _ in scrambled],
            [g for _, g in scrambled],
        )
        assert self.band(baseline, variant) == pytest.approx(
            self.band(*rescrambled), rel=0.25
        )

    def test_a_higher_quantile_widens_the_band(self):
        args = stationary_pairs(600)
        assert self.band(*args, quantile=0.99) >= self.band(
            *args, quantile=0.90
        )

    def test_more_samples_tighten_the_band(self):
        assert self.band(*stationary_pairs(1500)) < self.band(
            *stationary_pairs(300)
        )


class TestDriftSignificance:
    def test_a_stationary_series_is_not_significant(self):
        baseline, variant = stationary_pairs(900)
        assert (
            drift_significance(baseline, variant, permutations=300, rng=rng())
            > 0.05
        )

    def test_a_strongly_drifting_series_is_significant(self):
        baseline, variant = drifting_pairs(900, shift=4.0)
        assert (
            drift_significance(baseline, variant, permutations=300, rng=rng())
            <= 0.01
        )

    def test_the_p_value_is_bounded(self):
        baseline, variant = stationary_pairs(300)
        p = drift_significance(baseline, variant, permutations=200, rng=rng())
        assert 0.0 <= p <= 1.0


class TestCachePriming:
    """A cold cache is a prerequisite, not a cleanup failure.

    The smoke check's own dispatch populates the cache root on a fresh runner,
    and the integrity witness then reports those entries as appearing during
    the run — which they did.
    """

    def gaps_for(self, version, entries):
        return warm_cache_gaps(
            version=version,
            cache_root_entries=entries,
            platform="darwin-arm64",
        )

    def warm(self, version="1.24.0-pre.41"):
        digest = "a" * 64
        return [
            f"accelerator-launcher-{version}-darwin-arm64",
            f"accelerator-launcher-{version}-darwin-arm64.minisig",
            f"vcs-{version}-{digest}",
            f"vcs-{version}-{digest}.minisig",
        ]

    def test_a_warm_cache_is_left_alone(self, tmp_path):
        dispatches = []

        def runner(argv, *, cwd, env):
            dispatches.append(argv)
            return RunResult("", "", 0, 1.0)

        report = prime_cache(
            tmp_path,
            runner=runner,
            entries=self.warm,
            version="1.24.0-pre.41",
            platform="darwin-arm64",
        )
        assert report["primed"] is False
        assert not dispatches, "a warm cache needs no fetch"

    def test_a_cold_cache_is_primed_by_one_dispatch(self, tmp_path):
        state = {"entries": []}
        dispatches = []

        def runner(argv, *, cwd, env):
            dispatches.append(argv)
            state["entries"] = self.warm()
            return RunResult("", "", 0, 1.0)

        report = prime_cache(
            tmp_path,
            runner=runner,
            entries=lambda: state["entries"],
            version="1.24.0-pre.41",
            platform="darwin-arm64",
        )
        assert report["primed"] is True
        assert len(dispatches) == 1
        assert report["gaps_before"]
        assert report["gaps_after"] == []

    def test_a_fetch_that_does_not_close_the_gaps_is_reported(self, tmp_path):
        def runner(argv, *, cwd, env):
            return RunResult("", "no release published", 0, 1.0)

        report = prime_cache(
            tmp_path,
            runner=runner,
            entries=list,
            version="1.24.0-pre.41",
            platform="darwin-arm64",
        )
        assert report["primed"] is True
        assert report["gaps_after"], (
            "an unclosed gap must be reported, not swallowed — it is the "
            "unmet-prerequisite signal the lane exists to surface"
        )

    def test_priming_uses_the_ambient_environment_not_a_farm(self, tmp_path):
        seen = {}

        def runner(argv, *, cwd, env):
            seen["path"] = env.get("PATH")
            return RunResult("", "", 0, 1.0)

        prime_cache(
            tmp_path,
            runner=runner,
            entries=list,
            version="1.24.0-pre.41",
            platform="darwin-arm64",
        )
        # The farms do not exist yet at priming time, and the fetch needs curl
        # or wget, which a farm built for the two variants need not carry.
        assert seen["path"] == os.environ.get("PATH")

    def test_only_the_smoke_check_primes_the_cache(self):
        """The measurement must refuse a cold cache, never populate it.

        A freshly fetched entry is not the warm path the measurement times, and
        the fetch would mutate the cache root its own integrity witness is
        compared against. Asserted at the source, so the two paths cannot
        converge by accident.
        """
        source = (REPO / "tasks/measure.py").read_text()
        definitions = re.split(r"^def ", source, flags=re.MULTILINE)[1:]
        callers = {
            name
            for name, body in ((d.split("(", 1)[0], d) for d in definitions)
            # The definition of `prime_cache` names itself in its own signature.
            if name != "prime_cache"
            and re.search(r"(?<![\w_])prime_cache\(", body)
        }
        assert callers == {"smoke_report"}, (
            f"prime_cache is called from {sorted(callers)}; only the smoke "
            f"check may prime, because the measurement must refuse a cold "
            f"cache rather than fetch into the root it witnesses"
        )


class TestGuardedFiles:
    def test_the_baseline_is_recovered_not_read_from_the_tree(self):
        """No guarded file may be one the measurement recovers from history.

        The baseline and its dependency are read at a pinned revision, so the
        live copies are not inputs to anything measured. Guarding them would
        couple the harness to files that may be deleted.
        """
        recovered = {source for source, _ in RECOVERED_FILES.values()}
        assert not recovered & set(GUARDED_FILES)

    def test_an_absent_guarded_file_does_not_crash_the_capture(
        self, plugin_root, tmp_path
    ):
        (plugin_root / "hooks/hooks.json").unlink()
        with fake_session(plugin_root, tmp_path) as session:
            assert session.baseline is not None
            digests = session.baseline.guarded_file_digests
            assert digests["hooks/hooks.json"] is None
        assert session.failures == []

    def test_a_guarded_file_appearing_mid_run_is_reported(
        self, plugin_root, tmp_path
    ):
        (plugin_root / "hooks/hooks.json").unlink()
        with fake_session(plugin_root, tmp_path) as session:
            (plugin_root / "hooks/hooks.json").write_text("stub\n")
        assert any("hooks.json changed" in f for f in session.failures)

    def test_a_guarded_file_vanishing_mid_run_is_reported(
        self, plugin_root, tmp_path
    ):
        with fake_session(plugin_root, tmp_path) as session:
            (plugin_root / "hooks/hooks.json").unlink()
        assert any("hooks.json changed" in f for f in session.failures)
