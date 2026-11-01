from pathlib import Path

import pytest
from invoke import Context
from invoke.exceptions import Exit

import tasks.eval as evals
from tasks.eval.ask_and_report import runs, startup
from tasks.eval.headless.transcript import Init, Result

REPO_ROOT = Path(__file__).resolve().parents[3]
RULE = REPO_ROOT / "skills/research/outputters/citing-once/SKILL.md"
INIT = Init("s", "claude-test", (("accelerator", str(REPO_ROOT)),))


@pytest.mark.parametrize(
    ("version", "refused"),
    [
        ("2.1.144 (Claude Code)", False),
        ("2.1.316 (Claude Code)", False),
        ("2.2.0", False),
        ("2.1.143 (Claude Code)", True),
        ("1.9.999", True),
        ("unknown", True),
    ],
)
def test_the_version_floor_is_2_1_144(version: str, refused: bool) -> None:
    assert (startup.version_refusal(version) is not None) is refused


def _outcome(
    *,
    exit_status: int = 0,
    error: str = "",
    denials: tuple = (),
    wrote: bool = True,
    stderr: str = "",
) -> startup.ProbeOutcome:
    result = Result(
        is_error=bool(error), permission_denials=denials, text=error
    )
    return startup.ProbeOutcome(
        command=["claude", "-p", "probe"],
        exit_status=exit_status,
        events=(INIT, result),
        stderr=stderr,
        wrote_probe=wrote,
        rule=RULE,
    )


def test_a_probe_that_reads_and_writes_passes() -> None:
    assert startup.probe_refusal(_outcome()) is None


@pytest.mark.parametrize(
    "error",
    [
        "Invalid API key · Please run /login",
        "Authentication failed",
        "API Error: 401",
        "Not logged in · Please log in",
    ],
)
def test_an_authentication_failure_names_both_ways_to_authenticate(
    error: str,
) -> None:
    refusal = startup.probe_refusal(_outcome(exit_status=1, error=error))

    assert refusal is not None
    assert error in refusal
    for named in (
        "--setting-sources project",
        "CLAUDE_CONFIG_DIR",
        "apiKeyHelper",
    ):
        assert named in refusal


def test_an_authentication_failure_on_stderr_alone_is_recognised() -> None:
    refusal = startup.probe_refusal(
        _outcome(exit_status=1, stderr="Invalid API key")
    )

    assert refusal is not None
    assert "--setting-sources project" in refusal


def test_a_denied_rule_read_names_the_scoped_grant() -> None:
    denial = {"tool_name": "Read", "tool_input": {"file_path": str(RULE)}}

    refusal = startup.probe_refusal(_outcome(denials=(denial,)))

    assert refusal is not None
    assert f"Read(/{RULE.parent}/**)" in refusal


@pytest.mark.parametrize(
    "outcome",
    [
        _outcome(
            denials=(
                {
                    "tool_name": "Write",
                    "tool_input": {"file_path": "probe.txt"},
                },
            ),
            wrote=False,
        ),
        _outcome(wrote=False),
    ],
    ids=["denied", "absent"],
)
def test_an_unwritten_probe_names_accept_edits(
    outcome: startup.ProbeOutcome,
) -> None:
    refusal = startup.probe_refusal(outcome)

    assert refusal is not None
    assert "--permission-mode acceptEdits" in refusal
    assert "2.1.281" in refusal


def test_any_other_denial_fails_the_probe_naming_the_tool() -> None:
    denial = {"tool_name": "Bash", "tool_input": {"command": "ls"}}

    refusal = startup.probe_refusal(_outcome(denials=(denial,)))

    assert refusal is not None
    assert "denied Bash" in refusal


def test_any_other_probe_failure_names_the_command_and_status() -> None:
    refusal = startup.probe_refusal(_outcome(exit_status=3, error="Overloaded"))

    assert refusal is not None
    assert "claude -p probe" in refusal
    assert "exited 3" in refusal
    assert "--setting-sources" not in refusal


class Gates:
    """Record each start-up gate the task reaches, refusing at one."""

    def __init__(self, monkeypatch: pytest.MonkeyPatch, refuse_at: str) -> None:
        self.reached: list[str] = []
        self.refuse_at = refuse_at
        for gate, module, name in (
            ("marker", evals, "require_dev_launcher_marker"),
            ("version", evals, "claude_version"),
            ("preflight", evals, "preflight_problems"),
            ("probe", evals, "authentication_refusal"),
            ("plan", evals.ask_and_report_runs, "plan_sessions"),
        ):
            monkeypatch.setattr(module, name, self._gate(gate))

    def _gate(self, gate: str):
        def reach(*args, **kwargs):
            self.reached.append(gate)
            if gate == self.refuse_at:
                raise Exit(f"refused at {gate}", code=1)
            return {"version": "2.1.316", "preflight": [], "plan": []}.get(gate)

        return reach


@pytest.mark.parametrize(
    ("refuse_at", "reached"),
    [
        ("marker", ["marker"]),
        ("version", ["marker", "version"]),
        ("preflight", ["marker", "version", "preflight"]),
        ("probe", ["marker", "version", "preflight", "probe"]),
    ],
)
def test_the_start_up_gates_run_cheapest_first(
    monkeypatch: pytest.MonkeyPatch, refuse_at: str, reached: list[str]
) -> None:
    gates = Gates(monkeypatch, refuse_at)

    with pytest.raises(Exit):
        evals.ask_and_report(Context(), out=str(Path("/nonexistent")))

    assert gates.reached == reached


def test_a_re_check_runs_only_the_pre_flight(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    gates = Gates(monkeypatch, refuse_at="plan")

    with pytest.raises(Exit):
        evals.ask_and_report(Context(), out=str(tmp_path), check_only=True)

    assert gates.reached == ["preflight", "plan"]


def test_a_re_check_keeps_the_run_s_recorded_settings(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    Gates(monkeypatch, refuse_at="")
    recorded = runs.RunSettings("2.1.300", "claude-test", "/config")
    runs.save_results([], tmp_path, recorded)

    evals.ask_and_report(Context(), out=str(tmp_path), check_only=True)

    assert runs.load_settings(tmp_path) == recorded


def test_rerunning_errored_sessions_needs_an_out_directory(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    gates = Gates(monkeypatch, refuse_at="")

    with pytest.raises(Exit, match="--out"):
        evals.ask_and_report(Context(), rerun_errored=True)

    assert gates.reached == []


def test_a_run_refused_at_the_marker_prints_no_reminder(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    Gates(monkeypatch, refuse_at="marker")

    with pytest.raises(Exit):
        evals.ask_and_report(Context())

    assert "rm .accelerator-dev-launcher" not in capsys.readouterr().out


def test_a_run_that_started_claude_ends_with_the_reminder(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    Gates(monkeypatch, refuse_at="probe")

    with pytest.raises(Exit):
        evals.ask_and_report(Context())

    assert "rm .accelerator-dev-launcher" in capsys.readouterr().out
