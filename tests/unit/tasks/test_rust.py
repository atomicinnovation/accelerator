from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks import public_api, pup
from tasks.shared import rust
from tasks.shared.cargo_metadata import Package, workspace_packages
from tasks.shared.coverage import parse_totals
from tasks.shared.paths import (
    CLI_DIR,
    CLI_WORKSPACE_CARGO_TOML,
    cli_member_manifests,
)
from tasks.shared.rust import RUST_NIGHTLY
from tasks.test import cli as test_cli


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


def _passed(stdout: str = EXPORT) -> MagicMock:
    return MagicMock(exited=0, stdout=stdout)


def _failed() -> MagicMock:
    return MagicMock(exited=1, stdout="")


class TestTestUnitCli:
    @pytest.fixture
    def lane(self, monkeypatch: pytest.MonkeyPatch) -> MagicMock:
        monkeypatch.delenv("ACCELERATOR_COVERAGE", raising=False)
        m = MagicMock(spec=Context)
        m.run.return_value = _passed()
        return m

    def _commands(self, lane: MagicMock) -> list[str]:
        return [call.args[0] for call in lane.run.call_args_list]

    def test_clears_profile_data_then_tests_then_reports(self, lane: MagicMock):
        test_cli.run(lane)

        clean, tests, report = self._commands(lane)
        assert clean.startswith("cargo llvm-cov clean")
        assert "--profraw-only" in clean
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
        assert report.startswith("cargo llvm-cov report")
        for flag in ("--json", "--summary-only"):
            assert flag in report
        assert "features" not in report, (
            "cargo llvm-cov 0.8.7 rejects feature flags on `report`"
        )
        for command in (clean, tests, report):
            assert f"--manifest-path {CLI_WORKSPACE_CARGO_TOML}" in command

    def test_only_the_report_output_is_captured(self, lane: MagicMock):
        test_cli.run(lane)

        captured = [
            call.args[0].startswith("cargo llvm-cov report")
            for call in lane.run.call_args_list
            if call.kwargs.get("hide") == "out"
        ]
        assert captured == [True]

    def test_prints_the_totals_line(
        self, lane: MagicMock, capsys: pytest.CaptureFixture[str]
    ):
        test_cli.run(lane)

        out = capsys.readouterr().out
        assert parse_totals(EXPORT).summary_line() in out
        assert '"files"' not in out
        assert "{" not in out

    def test_plain_nextest_when_coverage_off(
        self, lane: MagicMock, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", "off")
        test_cli.run(lane)

        [command] = self._commands(lane)
        assert command.startswith("cargo nextest run")
        assert "llvm-cov" not in command

    def test_carries_no_coverage_threshold(self, lane: MagicMock):
        test_cli.run(lane)

        assert not any("--fail-under" in c for c in self._commands(lane))

    def test_failing_plain_nextest_fails_the_task(
        self, lane: MagicMock, monkeypatch: pytest.MonkeyPatch
    ):
        monkeypatch.setenv("ACCELERATOR_COVERAGE", "off")
        lane.run.return_value = _failed()
        with pytest.raises(Exit):
            test_cli.run(lane)

    def test_failing_profile_clean_stops_before_the_tests(
        self, lane: MagicMock
    ):
        lane.run.side_effect = [_failed()]
        with pytest.raises(Exit):
            test_cli.run(lane)
        assert lane.run.call_count == 1

    def test_failing_tests_stop_before_the_report(self, lane: MagicMock):
        lane.run.side_effect = [_passed(), _failed()]
        with pytest.raises(Exit):
            test_cli.run(lane)
        assert lane.run.call_count == 2

    def test_failing_report_fails_the_task(self, lane: MagicMock):
        lane.run.side_effect = [_passed(), _passed(), _failed()]
        with pytest.raises(Exit):
            test_cli.run(lane)

    def test_unreadable_report_fails_the_task(self, lane: MagicMock):
        lane.run.side_effect = [_passed(), _passed(), _passed("not json")]
        with pytest.raises(Exit):
            test_cli.run(lane)

    def test_docstring_names_the_stale_binary_remedy(self):
        assert "cargo llvm-cov clean --workspace" in test_cli.run.body.__doc__


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
