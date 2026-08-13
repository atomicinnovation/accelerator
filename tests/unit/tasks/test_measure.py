"""Unit tests for the warm-dispatch measurement harness.

The harness splits into a pure analysis core (`tasks/shared/measurement.py`)
and a subprocess driver (`tasks/measure.py`). Everything decidable is a pure
function over recorded observations, because the measurement session runs once
and a defect in it yields a plausible-looking ratio rather than a crash.
"""

import json
import random
import subprocess
from pathlib import Path

import pytest

from tasks.shared.measurement import (
    Branch,
    CellKind,
    CellOutcome,
    Decision,
    IllFormedCell,
    Validity,
    Variant,
    budget_exhausted,
    classify,
    closure_verdict,
    drift_verdict,
    generate_schedule,
    log_appended_lines,
    normalise_envelope,
    outlier_trip,
    paired_ratio_interval,
    percentile,
    required_samples,
    retry_budget,
    summarise,
    unpaired_interval,
    validate_sample,
)


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
            assert sorted(variants) == sorted(
                [Variant.BASELINE, Variant.FAST]
            )

    def test_block_b_samples_never_enter_a_pair(self):
        for sample in self.schedule():
            if sample.block == "B":
                assert sample.pair is None
                assert sample.variant is Variant.FALLBACK

    def test_segments_alternate_between_the_blocks(self):
        blocks = [s.block for s in self.analysed(self.schedule())]
        runs = [blocks[0]]
        runs += [b for a, b in zip(blocks, blocks[1:], strict=True) if a != b]
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
                / "hooks/test-fixtures/vcs-guard/decision-table.json"
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
        verdict = validate_sample(
            "", deny_envelope(BLOCK_REASON), BLOCK_REASON
        )
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
        [(CellKind.ABSOLUTE, True), (CellKind.ABSOLUTE, False),
         (CellKind.RATIO, None)],
    )
    def test_an_ill_formed_kind_and_robustness_pair_raises(
        self, kind, robustness
    ):
        with pytest.raises(IllFormedCell):
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
            classify(**self.state(upper_distance=2.0))
            is Branch.INDETERMINATE
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
            classify(
                **self.state(cell_kind=CellKind.RATIO, robustness_ok=True)
            )
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
            c if c.cell != "C3" else CellOutcome("C3", True, Branch.NOT_APPLICABLE)
            for c in self.cells()
        ]
        assert not closure_verdict(cells)

    def test_a_branch_seven_gating_cell_with_acceptance_closes(self):
        cells = [
            c
            if c.cell != "C3"
            else CellOutcome(
                "C3", True, Branch.NOT_APPLICABLE, accepted_by="Toby Clemson"
            )
            for c in self.cells()
        ]
        assert closure_verdict(cells)

    def test_every_gating_cell_in_branch_seven_with_acceptance_closes(self):
        cells = [
            CellOutcome(
                c.cell,
                c.gates,
                Branch.NOT_APPLICABLE,
                accepted_by="Toby Clemson" if c.gates else None,
            )
            for c in self.cells()
        ]
        assert closure_verdict(cells)

    def test_an_empty_cell_set_does_not_close(self):
        assert not closure_verdict([])
