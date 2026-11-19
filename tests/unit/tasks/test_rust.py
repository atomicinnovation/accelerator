import contextlib
import os
import shlex
import signal
from collections.abc import Callable
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks import public_api, pup
from tasks.shared import rust
from tasks.shared.cargo_metadata import Package, workspace_packages
from tasks.shared.coverage import CoverageFloor, Metric, parse_totals
from tasks.shared.paths import (
    CLI_DIR,
    CLI_WORKSPACE_CARGO_TOML,
    cli_member_manifests,
)
from tasks.shared.rust import RUST_NIGHTLY
from tasks.shared.shared_fixtures.errors import SharedFixturesError
from tasks.shared.shared_fixtures.families import (
    FIXTURE_LOG_VARIABLE,
    WORKBENCHES_VARIABLE,
    Family,
)
from tasks.test import cli as test_cli
from tests.unit.tasks.shared_fixture_doubles import (
    FakeSharedFixtures,
    builder_artefact,
    captured_listing,
    failed,
    passed,
)


@pytest.fixture
def ctx() -> MagicMock:
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(exited=0, stdout="")
    return m


# ── pup_mode() ────────────────────────────────────────────────────────


class TestPupMode:
    def test_defaults_to_deny_when_env_absent(
        self, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.delenv("ACCELERATOR_PUP_MODE", raising=False)
        assert rust.pup_mode() == "deny"

    @pytest.mark.parametrize("value", ["warn", "Warn", " warn ", "WARN"])
    def test_normalises_warn(self, monkeypatch: pytest.MonkeyPatch, value: str):
        monkeypatch.setenv("ACCELERATOR_PUP_MODE", value)
        assert rust.pup_mode() == "warn"

    @pytest.mark.parametrize("value", ["off", "lenient", "0", "true"])
    def test_unrecognised_value_fails_closed_with_warning(
        self,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
        value: str,
    ):
        monkeypatch.setenv("ACCELERATOR_PUP_MODE", value)
        assert rust.pup_mode() == "deny"
        assert "WARNING" in capsys.readouterr().out


# ── coverage_enabled() ────────────────────────────────────────────────


class TestCoverageEnabled:
    def test_defaults_on_when_env_absent(self, monkeypatch: pytest.MonkeyPatch):
        monkeypatch.delenv("ACCELERATOR_COVERAGE", raising=False)
        assert rust.coverage_enabled() is True

    @pytest.mark.parametrize(
        "value", ["off", "false", "0", "no", "OFF", " no "]
    )
    def test_falsey_values_disable(
        self, monkeypatch: pytest.MonkeyPatch, value: str
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", value)
        assert rust.coverage_enabled() is False

    @pytest.mark.parametrize("value", ["on", "yes", "1", "anything"])
    def test_non_falsey_values_enable(
        self, monkeypatch: pytest.MonkeyPatch, value: str
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", value)
        assert rust.coverage_enabled() is True


# ── pup.check() leaf branches ─────────────────────────────────────────


class TestPupCheck:
    def test_runs_pup_on_the_pinned_nightly(self, ctx: MagicMock):
        pup.check(ctx)
        assert ctx.run.call_args.args[0] == f"cargo +{RUST_NIGHTLY} pup"

    def test_deny_mode_raises_on_findings(
        self, ctx: MagicMock, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.delenv("ACCELERATOR_PUP_MODE", raising=False)
        ctx.run.return_value = MagicMock(exited=1)
        with pytest.raises(Exit):
            pup.check(ctx)

    def test_warn_mode_logs_and_returns_cleanly(
        self,
        ctx: MagicMock,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ):
        monkeypatch.setenv("ACCELERATOR_PUP_MODE", "warn")
        ctx.run.return_value = MagicMock(exited=1)
        pup.check(ctx)
        assert "WARNING" in capsys.readouterr().out


# ── test:unit:cli leaf branches ───────────────────────────────────────


EXPORT = (
    Path(__file__).parent / "fixtures/coverage/llvm-cov-summary.json"
).read_text()
UNITS = (
    Path(__file__).parent / "fixtures/coverage/cargo-units.jsonl"
).read_text()
BUILD_ENVIRONMENT = (
    "LLVM_PROFILE_FILE='/workspace/cli/target/cli-%p-%16m.profraw'\n"
    "RUSTC_WRAPPER=/opt/bin/cargo-llvm-cov\n"
)
CAPTURED_BUILD_DIR = "/workspace/cli/target/llvm-cov-target/debug"
LIVE_ARTEFACT = "deps/libsmallvec-0e524c8a43f44c11.rlib"
UPLIFTED_BINARY = "accelerator-verify"
BINARY_TWIN = "deps/accelerator_verify-bccc7b6827f71b41"
SHARED_FIXTURES = "shared-fixtures"
SHARED_FIXTURES_TWIN = "deps/shared_fixtures-5a1e2b3c4d5e6f70"
LIVE_SESSION = "incremental/kernel-1newer"
SUPERSEDED_SESSION = "incremental/kernel-0older"
SUPERSEDED_ARTEFACT = "deps/libsmallvec-3d4b3aa50bf88d80.rlib"

REPORT = "cargo llvm-cov report"
SHOW_ENV = "cargo llvm-cov show-env"
UNIT_LISTING = "cargo test"
_OUTPUTS = {REPORT: EXPORT, SHOW_ENV: BUILD_ENVIRONMENT, UNIT_LISTING: UNITS}
CLASSIFY = "binary_id(vcs-adapters::classify)"
_HANDOVER_VARIABLES = (
    Family.MATRIX.handover_variable,
    WORKBENCHES_VARIABLE,
    FIXTURE_LOG_VARIABLE,
)


def _only_classify(binary: str, _name: str) -> bool:
    return binary == "vcs-adapters::classify"


def _responding(
    override: tuple[str, MagicMock] | None = None, *, occurrence: int = 1
):
    seen: dict[str, int] = {}

    def respond(command: str, **_kwargs) -> MagicMock:
        if override and command.startswith(override[0]):
            seen[override[0]] = seen.get(override[0], 0) + 1
            if seen[override[0]] == occurrence:
                return override[1]
        for prefix, stdout in _OUTPUTS.items():
            if command.startswith(prefix):
                return passed(stdout)
        return passed()

    return respond


def _failing_at(
    prefix: str, result: MagicMock | None = None, *, occurrence: int = 1
) -> FakeSharedFixtures:
    if "nextest" in prefix:
        return FakeSharedFixtures(_responding(), nextest=result or failed())
    return FakeSharedFixtures(
        _responding((prefix, result or failed()), occurrence=occurrence)
    )


@pytest.mark.usefixtures("shared_fixture_roots")
class TestTestUnitCli:
    @pytest.fixture
    def build_dir(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> Path:
        build_dir = tmp_path / "llvm-cov-target" / "debug"
        for artefact in (
            LIVE_ARTEFACT,
            SUPERSEDED_ARTEFACT,
            UPLIFTED_BINARY,
            BINARY_TWIN,
            SHARED_FIXTURES,
            SHARED_FIXTURES_TWIN,
        ):
            (build_dir / artefact).parent.mkdir(parents=True, exist_ok=True)
            (build_dir / artefact).write_bytes(b"x" * 1_500)
        for modified, session in enumerate((SUPERSEDED_SESSION, LIVE_SESSION)):
            (build_dir / session).mkdir(parents=True)
            os.utime(build_dir / session, (modified, modified))
        monkeypatch.setattr(test_cli, "_INSTRUMENTED_BUILD", build_dir)
        monkeypatch.setitem(
            _OUTPUTS,
            UNIT_LISTING,
            UNITS.replace(CAPTURED_BUILD_DIR, str(build_dir))
            + builder_artefact(str(build_dir / SHARED_FIXTURES))
            + "\n",
        )
        return build_dir

    @pytest.fixture
    def lane(
        self, monkeypatch: pytest.MonkeyPatch, build_dir: Path
    ) -> MagicMock:
        monkeypatch.delenv("ACCELERATOR_COVERAGE", raising=False)
        monkeypatch.setattr(
            test_cli,
            "COVERAGE_FLOOR",
            CoverageFloor(dict.fromkeys(Metric, 0)),
        )
        m = MagicMock(spec=Context)
        m.run.side_effect = FakeSharedFixtures(_responding())
        return m

    def _commands(self, lane: MagicMock) -> list[str]:
        return [call.args[0] for call in lane.run.call_args_list]

    def _call(self, lane: MagicMock, prefix: str):
        [call] = [
            call
            for call in lane.run.call_args_list
            if call.args[0].startswith(prefix)
        ]
        return call

    def _builder_call(self, lane: MagicMock, build_dir: Path):
        return self._call(lane, str(build_dir / SHARED_FIXTURES))

    def _swept_nothing(self, build_dir: Path) -> bool:
        return all(
            (build_dir / superseded).exists()
            for superseded in (SUPERSEDED_ARTEFACT, SUPERSEDED_SESSION)
        )

    def test_builds_the_fixtures_then_tests_reports_and_sweeps(
        self, lane: MagicMock, build_dir: Path, tmp_path: Path
    ):
        test_cli.run(lane)

        (
            clean,
            show_env,
            builder_listing,
            builder,
            tests,
            report,
            listing,
        ) = self._commands(lane)
        assert clean.startswith("cargo llvm-cov clean")
        assert "--profraw-only" in clean
        assert show_env == f"cargo llvm-cov show-env {test_cli._WORKSPACE}"
        assert builder_listing == (
            f"cargo test {test_cli._MANIFEST} --no-run --message-format=json"
        )
        [executable, flag, tree] = shlex.split(builder)
        assert executable == str(build_dir / SHARED_FIXTURES)
        assert flag == "--matrix"
        assert Path(tree).parent.parent == tmp_path / "shared-fixtures"
        assert Path(tree).name == "matrix"
        assert tests.startswith("cargo llvm-cov nextest")
        for flag in (
            "--no-report",
            "--exclude accelerator-visualiser",
            "--all-features",
        ):
            assert flag in tests
        assert "--no-clean" not in tests, (
            "cargo llvm-cov 0.8.7 rejects --no-clean beside --no-report, "
            "which already skips the full clean"
        )
        assert report.startswith(REPORT)
        for flag in ("--json", "--summary-only"):
            assert flag in report
        assert "features" not in report, (
            "cargo llvm-cov 0.8.7 rejects feature flags on `report`"
        )
        assert listing == builder_listing
        for command in (clean, show_env, builder_listing, tests, report):
            assert f"--manifest-path {CLI_WORKSPACE_CARGO_TOML}" in command

    def test_both_unit_listings_run_in_the_instrumented_environment(
        self, lane: MagicMock, build_dir: Path
    ):
        test_cli.run(lane)

        listings = [
            call
            for call in lane.run.call_args_list
            if call.args[0].startswith(UNIT_LISTING)
        ]
        assert len(listings) == 2
        for listing in listings:
            assert listing.kwargs["env"] == {
                "LLVM_PROFILE_FILE": (
                    "/workspace/cli/target/cli-%p-%16m.profraw"
                ),
                "RUSTC_WRAPPER": "/opt/bin/cargo-llvm-cov",
                "CARGO_TARGET_DIR": str(build_dir.parent),
            }

    def test_the_builder_runs_instrumented_and_logging(
        self, lane: MagicMock, build_dir: Path
    ):
        test_cli.run(lane)

        builder = self._builder_call(lane, build_dir)
        environment = builder.kwargs["env"]
        assert builder.kwargs["replace_env"] is True
        assert environment["RUSTC_WRAPPER"] == "/opt/bin/cargo-llvm-cov"
        assert environment["CARGO_TARGET_DIR"] == str(build_dir.parent)
        assert environment[FIXTURE_LOG_VARIABLE].endswith(".jsonl")
        assert environment["PATH"] == os.environ["PATH"]

    def test_the_builder_s_profile_data_lands_beside_the_tests(
        self, lane: MagicMock, build_dir: Path
    ):
        test_cli.run(lane)

        environment = self._builder_call(lane, build_dir).kwargs["env"]
        assert environment["LLVM_PROFILE_FILE"] == str(
            build_dir.parent / "cli-%p-%16m.profraw"
        )

    def test_nextest_is_handed_the_trees_the_workbenches_and_the_log(
        self, lane: MagicMock, build_dir: Path
    ):
        test_cli.run(lane)

        builder = self._builder_call(lane, build_dir)
        tests = self._call(lane, "cargo llvm-cov nextest")
        environment = tests.kwargs["env"]
        assert tests.kwargs["replace_env"] is True
        assert (
            environment[Family.MATRIX.handover_variable]
            == (shlex.split(builder.args[0])[2])
        )
        assert environment[WORKBENCHES_VARIABLE].endswith("/workbenches")
        assert (
            environment[FIXTURE_LOG_VARIABLE]
            == (builder.kwargs["env"][FIXTURE_LOG_VARIABLE])
        )

    def test_handed_over_variables_from_outside_never_reach_the_run(
        self,
        lane: MagicMock,
        build_dir: Path,
        monkeypatch: pytest.MonkeyPatch,
    ):
        for variable in _HANDOVER_VARIABLES:
            monkeypatch.setenv(variable, "/elsewhere")

        test_cli.run(lane)

        builder = self._builder_call(lane, build_dir).kwargs["env"]
        tests = self._call(lane, "cargo llvm-cov nextest").kwargs["env"]
        assert Family.MATRIX.handover_variable not in builder
        assert WORKBENCHES_VARIABLE not in builder
        assert "/elsewhere" not in builder.values()
        assert "/elsewhere" not in tests.values()

    def test_prints_the_fixture_summary(
        self, lane: MagicMock, capsys: pytest.CaptureFixture[str]
    ):
        test_cli.run(lane)

        assert (
            "  matrix: 1 shared build, 4 adoptions, 0 copies, 0 private builds"
        ) in capsys.readouterr().out

    def test_only_the_report_and_listing_output_is_captured(
        self, lane: MagicMock
    ):
        test_cli.run(lane)

        captured = [
            call.args[0].split(" --")[0]
            for call in lane.run.call_args_list
            if call.kwargs.get("hide") == "out"
        ]
        assert captured == [SHOW_ENV, UNIT_LISTING, REPORT, UNIT_LISTING]

    def test_prints_the_totals_line(
        self, lane: MagicMock, capsys: pytest.CaptureFixture[str]
    ):
        test_cli.run(lane)

        out = capsys.readouterr().out
        assert parse_totals(EXPORT).summary_line() in out
        assert '"files"' not in out
        assert "{" not in out

    def test_sweeps_superseded_entries_and_keeps_live_ones(
        self,
        lane: MagicMock,
        build_dir: Path,
        capsys: pytest.CaptureFixture[str],
    ):
        test_cli.run(lane)

        assert (build_dir / LIVE_ARTEFACT).exists()
        assert (build_dir / BINARY_TWIN).exists()
        assert (build_dir / SHARED_FIXTURES_TWIN).exists()
        assert (build_dir / LIVE_SESSION).exists()
        assert not (build_dir / SUPERSEDED_ARTEFACT).exists()
        assert not (build_dir / SUPERSEDED_SESSION).exists()
        assert "swept 2 superseded artefacts (1.50 kB)" in (
            capsys.readouterr().out
        )

    def test_the_sweep_runs_after_the_totals_line(
        self, lane: MagicMock, capsys: pytest.CaptureFixture[str]
    ):
        test_cli.run(lane)

        out = capsys.readouterr().out
        assert out.index("coverage:") < out.index("swept")

    def test_plain_nextest_when_coverage_off(
        self,
        lane: MagicMock,
        build_dir: Path,
        monkeypatch: pytest.MonkeyPatch,
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", "off")
        test_cli.run(lane)

        listing, builder, tests = lane.run.call_args_list
        assert listing.args[0].startswith(UNIT_LISTING)
        assert "env" not in listing.kwargs
        assert builder.args[0].startswith(str(build_dir / SHARED_FIXTURES))
        assert "RUSTC_WRAPPER" not in builder.kwargs["env"]
        assert tests.args[0].startswith("cargo nextest run")
        assert "llvm-cov" not in tests.args[0]
        assert Family.MATRIX.handover_variable in tests.kwargs["env"]
        assert self._swept_nothing(build_dir)

    def test_passes_no_threshold_flag_to_cargo_llvm_cov(self, lane: MagicMock):
        test_cli.run(lane)

        assert not any("--fail-under" in c for c in self._commands(lane))

    def test_totals_below_the_floor_fail_the_task(
        self,
        lane: MagicMock,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ):
        totals = parse_totals(EXPORT).percents
        monkeypatch.setattr(
            test_cli,
            "COVERAGE_FLOOR",
            CoverageFloor(
                {
                    **dict.fromkeys(Metric, 0),
                    Metric.REGIONS: totals[Metric.REGIONS] + 1,
                }
            ),
        )

        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane)

        message = str(exc_info.value)
        assert "regions" in message
        assert "lines" not in message
        assert "COVERAGE_FLOOR" in message
        assert "cargo llvm-cov clean --workspace" in message
        assert parse_totals(EXPORT).summary_line() in capsys.readouterr().out

    def test_totals_at_the_floor_pass(
        self, lane: MagicMock, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.setattr(
            test_cli,
            "COVERAGE_FLOOR",
            CoverageFloor(parse_totals(EXPORT).percents),
        )

        test_cli.run(lane)

    def test_failing_plain_nextest_fails_the_task(
        self, lane: MagicMock, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", "off")
        lane.run.side_effect = _failing_at("cargo nextest run")
        with pytest.raises(Exit, match="nextest: cli tests failed"):
            test_cli.run(lane)

    def test_failing_profile_clean_stops_before_the_tests(
        self, lane: MagicMock
    ):
        lane.run.side_effect = _failing_at("cargo llvm-cov clean")
        with pytest.raises(Exit):
            test_cli.run(lane)
        assert lane.run.call_count == 1

    def test_failing_tests_stop_before_the_report(self, lane: MagicMock):
        lane.run.side_effect = _failing_at("cargo llvm-cov nextest")
        with pytest.raises(Exit, match="nextest: cli tests failed"):
            test_cli.run(lane)
        assert lane.run.call_count == 5

    def test_a_failing_builder_stops_before_the_tests(
        self, lane: MagicMock, build_dir: Path
    ):
        lane.run.side_effect = FakeSharedFixtures(
            _responding(), builder=failed("matrix: git is too old\n")
        )

        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane)

        message = str(exc_info.value)
        assert "matrix: git is too old" in message
        assert message.endswith(test_cli._RUN_DIRECTLY)
        assert not any("nextest" in c for c in self._commands(lane))

    def test_a_listing_without_the_builder_stops_before_the_tests(
        self, lane: MagicMock
    ):
        lane.run.side_effect = _failing_at(UNIT_LISTING, passed(UNITS))

        with pytest.raises(Exit, match="shared-fixtures"):
            test_cli.run(lane)

        assert not any("nextest" in c for c in self._commands(lane))

    def test_an_interrupt_during_the_tests_ends_the_task_as_one(
        self, lane: MagicMock
    ):
        def interrupted_by_ctrl_c() -> MagicMock:
            with contextlib.suppress(KeyboardInterrupt):
                os.kill(os.getpid(), signal.SIGINT)
            return failed()

        lane.run.side_effect = FakeSharedFixtures(
            _responding(), nextest=interrupted_by_ctrl_c
        )

        with pytest.raises(KeyboardInterrupt):
            test_cli.run(lane)

    def test_a_refused_root_is_one_exit_line(
        self, lane: MagicMock, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
    ):
        (tmp_path / "elsewhere").mkdir()
        (tmp_path / "link").symlink_to(tmp_path / "elsewhere")
        monkeypatch.setattr(test_cli, "_shared_root", lambda: tmp_path / "link")

        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane)

        assert "is a symlink" in str(exc_info.value)
        assert isinstance(exc_info.value.__cause__, SharedFixturesError)

    def test_a_hold_under_ci_is_one_exit_line(
        self, lane: MagicMock, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
    ):
        monkeypatch.setenv("ACCELERATOR_SHARED_FIXTURES_HOLD", str(tmp_path))
        monkeypatch.setenv("CI", "true")

        with pytest.raises(Exit, match="never honoured in CI"):
            test_cli.run(lane)

    def test_a_recompiling_nextest_fails_the_task(self, lane: MagicMock):
        lane.run.side_effect = FakeSharedFixtures(
            _responding(),
            nextest=passed(stderr="   Compiling vcs-adapters v1.24.0\n"),
        )

        with pytest.raises(Exit, match="recompiled after the build step"):
            test_cli.run(lane)

    def test_a_recompiling_failing_nextest_names_both(self, lane: MagicMock):
        lane.run.side_effect = FakeSharedFixtures(
            _responding(),
            nextest=failed("   Compiling vcs-adapters v1.24.0\n"),
        )

        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane)

        assert "recompiled after the build step" in str(exc_info.value)
        assert "nextest: cli tests failed" in str(exc_info.value)

    def test_failing_report_stops_before_the_sweep(
        self, lane: MagicMock, build_dir: Path
    ):
        lane.run.side_effect = _failing_at(REPORT)
        with pytest.raises(Exit):
            test_cli.run(lane)
        assert lane.run.call_count == 6
        assert self._swept_nothing(build_dir)

    def test_unreadable_report_fails_the_task(self, lane: MagicMock):
        lane.run.side_effect = _failing_at(REPORT, passed("not json"))
        with pytest.raises(Exit):
            test_cli.run(lane)

    @pytest.mark.parametrize(
        ("failing", "occurrence"),
        [(SHOW_ENV, 1), (UNIT_LISTING, 1), (UNIT_LISTING, 2)],
    )
    def test_a_failing_unit_listing_step_fails_the_task(
        self, lane: MagicMock, build_dir: Path, failing: str, occurrence: int
    ):
        lane.run.side_effect = _failing_at(failing, occurrence=occurrence)
        with pytest.raises(Exit):
            test_cli.run(lane)
        assert self._swept_nothing(build_dir)

    def test_a_listing_that_rebuilt_units_sweeps_nothing(
        self, lane: MagicMock, build_dir: Path
    ):
        rebuilt = _OUTPUTS[UNIT_LISTING].replace(
            '"fresh":true', '"fresh":false', 2
        )
        lane.run.side_effect = _failing_at(
            UNIT_LISTING, passed(rebuilt), occurrence=2
        )
        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane)
        assert "rebuilt 2" in str(exc_info.value)
        assert "no longer matches the instrumented build" in str(exc_info.value)
        assert self._swept_nothing(build_dir)

    def test_a_listing_with_no_units_sweeps_nothing(
        self, lane: MagicMock, build_dir: Path
    ):
        lane.run.side_effect = _failing_at(
            UNIT_LISTING, passed(""), occurrence=2
        )
        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane)
        assert "no units" in str(exc_info.value)
        assert self._swept_nothing(build_dir)

    def test_a_binary_without_a_twin_sweeps_nothing(
        self, lane: MagicMock, build_dir: Path
    ):
        (build_dir / BINARY_TWIN).write_bytes(b"an older build")
        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane)
        assert UPLIFTED_BINARY in str(exc_info.value)
        assert self._swept_nothing(build_dir)

    def test_docstring_names_the_stale_binary_remedy(self):
        assert "cargo llvm-cov clean --workspace" in test_cli.run.body.__doc__

    def _filtered_to(
        self, lane: MagicMock, matching: Callable[[str, str], bool]
    ) -> None:
        lane.run.side_effect = FakeSharedFixtures(
            _responding(), listing=passed(captured_listing(matching))
        )

    def test_the_option_is_named_filter(self):
        [argument] = test_cli.run.get_arguments()

        assert argument.names == ("filter", "f")

    def test_an_unfiltered_run_lists_nothing(self, lane: MagicMock):
        test_cli.run(lane)

        assert not any("nextest list" in c for c in self._commands(lane))

    def test_a_filtered_run_lists_then_builds_what_its_tests_consume(
        self,
        lane: MagicMock,
        build_dir: Path,
        monkeypatch: pytest.MonkeyPatch,
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", "off")
        self._filtered_to(lane, _only_classify)

        test_cli.run(lane, filter_=CLASSIFY)

        listing, units, builder, tests = self._commands(lane)
        assert listing == (
            f"cargo nextest list {test_cli._MANIFEST} -E '{CLASSIFY}' "
            "--message-format json"
        )
        assert units.startswith(UNIT_LISTING)
        [executable, flag, _tree] = shlex.split(builder)
        assert executable == str(build_dir / SHARED_FIXTURES)
        assert flag == "--matrix"
        assert (
            tests == f"cargo nextest run {test_cli._MANIFEST} -E '{CLASSIFY}'"
        )

    def test_the_listing_leaves_nextest_s_own_errors_on_the_terminal(
        self, lane: MagicMock
    ):
        self._filtered_to(lane, _only_classify)

        test_cli.run(lane, filter_=CLASSIFY)

        assert self._call(lane, "cargo nextest list").kwargs["hide"] == "out"

    def test_a_filtered_instrumented_run_lists_in_the_instrumented_build(
        self, lane: MagicMock, build_dir: Path
    ):
        self._filtered_to(lane, _only_classify)

        test_cli.run(lane, filter_=CLASSIFY)

        clean, show_env, listing, units, builder, tests = self._commands(lane)
        assert clean.startswith("cargo llvm-cov clean")
        assert show_env.startswith(SHOW_ENV)
        assert listing.startswith("cargo nextest list")
        assert units.startswith(UNIT_LISTING)
        assert builder.startswith(str(build_dir / SHARED_FIXTURES))
        assert tests == (
            f"cargo llvm-cov nextest {test_cli._MANIFEST} --no-report "
            f"-E '{CLASSIFY}'"
        )

    def test_the_instrumented_listing_profiles_into_a_directory_it_removes(
        self, lane: MagicMock, build_dir: Path
    ):
        self._filtered_to(lane, _only_classify)

        test_cli.run(lane, filter_=CLASSIFY)

        environment = self._call(lane, "cargo nextest list").kwargs["env"]
        profile_file = Path(environment["LLVM_PROFILE_FILE"])
        assert profile_file.name == "list-%p.profraw"
        assert not profile_file.parent.exists()
        assert environment["RUSTC_WRAPPER"] == "/opt/bin/cargo-llvm-cov"
        assert environment["CARGO_TARGET_DIR"] == str(build_dir.parent)

    def test_a_filtered_instrumented_run_skips_the_report_sweep_and_floor(
        self,
        lane: MagicMock,
        build_dir: Path,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ):
        monkeypatch.setattr(
            test_cli,
            "COVERAGE_FLOOR",
            CoverageFloor(dict.fromkeys(Metric, 100)),
        )
        self._filtered_to(lane, _only_classify)

        test_cli.run(lane, filter_=CLASSIFY)

        commands = self._commands(lane)
        assert not any(c.startswith(REPORT) for c in commands)
        assert sum(c.startswith(UNIT_LISTING) for c in commands) == 1
        assert self._swept_nothing(build_dir)
        assert (
            "coverage: report, sweep and floor skipped for a filtered run"
            in capsys.readouterr().out.splitlines()
        )

    def test_a_filter_selecting_no_consumer_builds_nothing(
        self,
        lane: MagicMock,
        build_dir: Path,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", "off")
        self._filtered_to(
            lane, lambda binary, _name: binary == "vcs-adapters::detection"
        )

        test_cli.run(lane, filter_="binary_id(vcs-adapters::detection)")

        assert not any(
            c.startswith(str(build_dir / SHARED_FIXTURES))
            for c in self._commands(lane)
        )
        assert (
            "shared VCS fixtures: building nothing (no selected test "
            "consumes a shared family)"
        ) in capsys.readouterr().out.splitlines()
        handover = self._call(lane, "cargo nextest run").kwargs["env"]
        assert Family.MATRIX.handover_variable not in handover
        assert WORKBENCHES_VARIABLE in handover
        assert FIXTURE_LOG_VARIABLE in handover

    def test_a_filter_selecting_no_tests_stops_before_any_fixture(
        self, lane: MagicMock, tmp_path: Path
    ):
        self._filtered_to(lane, lambda _binary, _name: False)

        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane, filter_="test(clasify)")

        assert str(exc_info.value) == (
            "--filter 'test(clasify)' selected no tests; see mise run "
            "test:unit:cli --help for its syntax; test() matches test "
            "names; binary_id() matches test binaries"
        )
        self._assert_no_fixture_was_touched(lane, tmp_path)

    def test_a_failing_listing_stops_before_any_fixture(
        self, lane: MagicMock, tmp_path: Path
    ):
        lane.run.side_effect = FakeSharedFixtures(
            _responding(), listing=failed()
        )

        with pytest.raises(Exit) as exc_info:
            test_cli.run(lane, filter_="test(classify")

        assert str(exc_info.value) == (
            "nextest: listing the tests for --filter 'test(classify' failed "
            "(a malformed filterset or a build error); see nextest's output "
            "above"
        )
        self._assert_no_fixture_was_touched(lane, tmp_path)

    @pytest.mark.parametrize("filter_", ["", "  "])
    def test_an_empty_filter_is_refused_before_anything_runs(
        self, lane: MagicMock, filter_: str
    ):
        with pytest.raises(Exit, match=r"^--filter needs an expression$"):
            test_cli.run(lane, filter_=filter_)

        assert lane.run.call_count == 0

    def _assert_no_fixture_was_touched(
        self, lane: MagicMock, tmp_path: Path
    ) -> None:
        commands = self._commands(lane)
        assert not any(SHARED_FIXTURES in c for c in commands)
        assert not any("nextest run" in c for c in commands)
        assert not any("llvm-cov nextest" in c for c in commands)
        shared = tmp_path / "shared-fixtures"
        assert not shared.exists() or not any(shared.iterdir())


# ── The surface-pin coverage guard ────────────────────────────────────
#
# public-api:check names the crates it pins explicitly, so without this guard a
# new crate escapes the pin and nothing reports the omission.


def _workspace_members() -> set[str]:
    return {
        manifest.parent.relative_to(CLI_DIR).as_posix()
        for manifest in cli_member_manifests(CLI_DIR / "Cargo.toml")
    }


def _unclassified(members: set[str], pinned, exempt) -> set[str]:
    return members - (set(pinned) | set(exempt))


class TestPinnedCrateCoverage:
    def test_every_workspace_member_is_classified(self):
        missing = _unclassified(
            _workspace_members(),
            public_api._PINNED_CRATES,
            public_api._EXEMPT_MEMBERS,
        )
        assert not missing, (
            f"unclassified cli/ workspace members: {sorted(missing)} — add "
            "each to _PINNED_CRATES in tasks/public_api.py, or to "
            "_EXEMPT_MEMBERS with the reason it needs no surface pin"
        )

    def test_the_guard_reports_a_member_in_neither_collection(self):
        # A guard that only ever runs against a complete classification cannot
        # show that it would notice an incomplete one.
        assert _unclassified(
            _workspace_members() | {"widget"},
            public_api._PINNED_CRATES,
            public_api._EXEMPT_MEMBERS,
        ) == {"widget"}

    def test_no_classified_name_is_absent_from_the_workspace(self):
        # A renamed or removed crate leaves a stale entry, which would otherwise
        # sit in the classification unnoticed.
        members = _workspace_members()
        stale = (
            set(public_api._PINNED_CRATES) | set(public_api._EXEMPT_MEMBERS)
        ) - members
        assert not stale, (
            f"classified but not a workspace member: {sorted(stale)}"
        )

    def test_no_member_is_both_pinned_and_exempt(self):
        overlap = set(public_api._PINNED_CRATES) & set(
            public_api._EXEMPT_MEMBERS
        )
        assert not overlap, f"classified twice: {sorted(overlap)}"

    def test_every_exemption_states_a_reason(self):
        unexplained = [
            member
            for member, reason in public_api._EXEMPT_MEMBERS.items()
            if not reason.strip()
        ]
        assert not unexplained, (
            f"exempted without a reason: {sorted(unexplained)}"
        )

    def test_every_pinned_crate_has_a_committed_snapshot(self):
        missing = [
            crate
            for crate in public_api._PINNED_CRATES
            if not public_api._snapshot(crate).exists()
        ]
        assert not missing, (
            f"pinned with no committed snapshot: {sorted(missing)} — "
            "regenerate with `mise run public-api:update`"
        )


# ── The report-scope guard ────────────────────────────────────────────


def _dependents_of(crate: str, packages: list[Package]) -> set[str]:
    return {
        package["name"]
        for package in packages
        if any(dep["name"] == crate for dep in package["dependencies"])
    }


def _package(name: str, *dependencies: dict) -> Package:
    return {"name": name, "dependencies": list(dependencies)}


class TestVisualiserHasNoDependents:
    def test_no_workspace_member_depends_on_the_visualiser(self):
        dependents = _dependents_of(test_cli._VISUALISER, workspace_packages())
        assert not dependents, (
            f"{sorted(dependents)} depend on {test_cli._VISUALISER}; "
            "`cargo llvm-cov report` cannot exclude a package, so a dependent "
            "would pull the visualiser into test:unit:cli's coverage totals"
        )

    @pytest.mark.parametrize("kind", [None, "dev", "build"])
    def test_the_guard_finds_a_dependent(self, kind: str | None):
        packages = [
            _package("widget", {"name": test_cli._VISUALISER, "kind": kind})
        ]
        assert _dependents_of(test_cli._VISUALISER, packages) == {"widget"}

    def test_the_guard_ignores_other_dependencies(self):
        packages = [_package("widget", {"name": "kernel", "kind": None})]
        assert _dependents_of(test_cli._VISUALISER, packages) == set()


# ── public_api.check() / public_api.update() leaf branches ────────────


@pytest.fixture
def snapshot(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    monkeypatch.setattr(public_api, "_PINNED_CRATES", ("widget",))
    monkeypatch.setattr(public_api, "CLI_DIR", tmp_path)
    path = tmp_path / "widget" / "tests" / "fixtures" / "public-api.txt"
    path.parent.mkdir(parents=True)
    return path


class TestPublicApiCheck:
    def test_runs_on_the_pinned_nightly_with_parameter_names(
        self, ctx: MagicMock, snapshot: Path
    ):
        snapshot.write_text("pub struct widget::Thing\n")
        ctx.run.return_value = MagicMock(
            exited=0, stdout="pub struct widget::Thing\n"
        )
        public_api.check(ctx)
        assert ctx.run.call_args.args[0] == (
            f"cargo +{RUST_NIGHTLY} public-api "
            "--omit blanket-impls,auto-trait-impls "
            "--include function-parameter-names -p widget"
        )

    def test_raises_when_snapshot_is_missing(
        self, ctx: MagicMock, snapshot: Path
    ):
        with pytest.raises(Exit) as exc_info:
            public_api.check(ctx)
        assert str(snapshot) in str(exc_info.value)

    def test_raises_when_snapshot_is_empty(
        self, ctx: MagicMock, snapshot: Path
    ):
        snapshot.write_text("")
        with pytest.raises(Exit):
            public_api.check(ctx)

    def test_raises_when_the_render_fails(self, ctx: MagicMock, snapshot: Path):
        snapshot.write_text("pub struct widget::Thing\n")
        ctx.run.return_value = MagicMock(exited=1, stdout="")
        with pytest.raises(Exit):
            public_api.check(ctx)

    def test_passes_when_the_render_matches_the_snapshot(
        self, ctx: MagicMock, snapshot: Path
    ):
        snapshot.write_text("pub struct widget::Thing\n")
        ctx.run.return_value = MagicMock(
            exited=0, stdout="pub struct widget::Thing\n"
        )
        public_api.check(ctx)

    def test_raises_when_the_render_diverges_from_the_snapshot(
        self, ctx: MagicMock, snapshot: Path
    ):
        snapshot.write_text("pub struct widget::Thing\n")
        ctx.run.return_value = MagicMock(
            exited=0, stdout="pub struct widget::Other\n"
        )
        with pytest.raises(Exit) as exc_info:
            public_api.check(ctx)
        assert str(snapshot) in str(exc_info.value)


class TestPublicApiUpdate:
    def test_creates_the_fixtures_directory_when_absent(
        self, ctx: MagicMock, snapshot: Path
    ):
        # A crate reaching the pin for the first time has no tests/fixtures/,
        # so the first update must make one rather than fail on the write.
        snapshot.parent.rmdir()
        ctx.run.return_value = MagicMock(
            exited=0, stdout="pub struct widget::Thing\n"
        )
        public_api.update(ctx)
        assert snapshot.read_text() == "pub struct widget::Thing\n"

    def test_writes_the_render_to_the_snapshot(
        self, ctx: MagicMock, snapshot: Path
    ):
        ctx.run.return_value = MagicMock(
            exited=0, stdout="pub struct widget::Thing\n"
        )
        public_api.update(ctx)
        assert snapshot.read_text() == "pub struct widget::Thing\n"

    def test_raises_when_the_render_fails(self, ctx: MagicMock, snapshot: Path):
        ctx.run.return_value = MagicMock(exited=1, stdout="")
        with pytest.raises(Exit):
            public_api.update(ctx)
