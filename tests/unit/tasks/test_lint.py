import subprocess
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks.lint import scripts as lint

REPO_ROOT = Path(__file__).resolve().parents[3]
LINT_BASHISMS = REPO_ROOT / "scripts/lint-bashisms.sh"


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(exited=0, stdout="")
    return m


def _command(ctx) -> str:
    return ctx.run.call_args.args[0]


class TestShellcheckTask:
    def test_command(self, ctx, mocker):
        mocker.patch.object(
            lint, "shell_sources", return_value=["a.sh", "b.sh"]
        )
        lint.shellcheck(ctx)
        cmd = _command(ctx)
        # Flag ownership moved to .shellcheckrc: the invocation is now bare.
        assert cmd.startswith("shellcheck ")
        # Explicit absence checks — a startswith-only assertion would still pass
        # if a stray flag survived later in the command string.
        assert "-x" not in cmd
        assert "--severity" not in cmd
        assert "a.sh" in cmd and "b.sh" in cmd

    def test_raises_on_findings(self, ctx, mocker):
        mocker.patch.object(lint, "shell_sources", return_value=["a.sh"])
        ctx.run.return_value = MagicMock(exited=1)
        with pytest.raises(Exit):
            lint.shellcheck(ctx)

    def test_raises_on_empty_source_set(self, ctx, mocker):
        # Fail-closed: an empty match set means scope discovery broke, not that
        # there is nothing to lint — the task must raise, not pass green.
        mocker.patch.object(lint, "shell_sources", return_value=[])
        with pytest.raises(Exit):
            lint.shellcheck(ctx)
        ctx.run.assert_not_called()


class TestBashismsTask:
    def test_command(self, ctx, mocker):
        mocker.patch.object(lint, "shell_sources", return_value=["a.sh"])
        lint.bashisms(ctx)
        assert _command(ctx).startswith("bash scripts/lint-bashisms.sh ")

    def test_raises_on_findings(self, ctx, mocker):
        mocker.patch.object(lint, "shell_sources", return_value=["a.sh"])
        ctx.run.return_value = MagicMock(exited=1)
        with pytest.raises(Exit):
            lint.bashisms(ctx)

    def test_raises_on_empty_source_set(self, ctx, mocker):
        # Fail-closed, as for shellcheck.
        mocker.patch.object(lint, "shell_sources", return_value=[])
        with pytest.raises(Exit):
            lint.bashisms(ctx)
        ctx.run.assert_not_called()


def _run_lint(path: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["bash", str(LINT_BASHISMS), str(path)],
        capture_output=True,
        text=True,
        check=False,
    )


class TestBashismsScript:
    """Behavioural coverage of scripts/lint-bashisms.sh itself."""

    def test_flags_associative_array(self, tmp_path: Path):
        f = tmp_path / "x.sh"
        f.write_text("#!/usr/bin/env bash\ndeclare -A MAP\n")
        result = _run_lint(f)
        assert result.returncode != 0
        assert "associative array" in result.stdout

    def test_flags_nameref(self, tmp_path: Path):
        # local -n / declare -n namerefs are bash 4.3+ — invalid on the
        # bash-3.2 floor, where `local: -n: invalid option` is fatal.
        f = tmp_path / "x.sh"
        f.write_text('#!/usr/bin/env bash\nf() { local -n ref="$1"; }\n')
        result = _run_lint(f)
        assert result.returncode != 0
        assert "nameref" in result.stdout

    def test_flags_escaped_brace_in_expansion_default(self, tmp_path: Path):
        # ${x:-{\}} yields "{}" on bash 4+ but "{\}" on the bash-3.2 floor,
        # which kept the literal backslash — a silent data-corruption bug.
        f = tmp_path / "x.sh"
        f.write_text('#!/usr/bin/env bash\nv="${1:-{\\}}"\n')
        result = _run_lint(f)
        assert result.returncode != 0
        assert "escaped brace" in result.stdout

    def test_unescaped_braces_in_default_are_not_flagged(self, tmp_path: Path):
        # ${x:-{}} is a plain empty-object default — identical on 3.2 and 4+.
        f = tmp_path / "x.sh"
        f.write_text('#!/usr/bin/env bash\nv="${1:-{}}"\necho "$v"\n')
        result = _run_lint(f)
        assert result.returncode == 0, result.stdout

    def test_substitution_with_escaped_brace_not_flagged(self, tmp_path: Path):
        # ${var//\}/x} is a pattern substitution, not a default — no false hit.
        f = tmp_path / "x.sh"
        f.write_text('#!/usr/bin/env bash\nv="${var//\\}/x}"\necho "$v"\n')
        result = _run_lint(f)
        assert result.returncode == 0, result.stdout

    def test_comment_naming_a_construct_is_not_flagged(self, tmp_path: Path):
        # A comment that mentions a forbidden construct must not trip the lint.
        f = tmp_path / "x.sh"
        f.write_text(
            "#!/usr/bin/env bash\n# do not use declare -A here\necho ok\n"
        )
        result = _run_lint(f)
        assert result.returncode == 0, result.stdout

    def test_inline_opt_out_marker(self, tmp_path: Path):
        f = tmp_path / "x.sh"
        f.write_text(
            "#!/usr/bin/env bash\ndeclare -A MAP # lint-bashisms: ignore\n"
        )
        result = _run_lint(f)
        assert result.returncode == 0, result.stdout

    def test_parameter_expansion_strip_is_not_a_case_mod(self, tmp_path: Path):
        # ${x#prefix} is a bash-3.2 prefix strip, not a ${x^^} case
        # modification.
        f = tmp_path / "x.sh"
        f.write_text('#!/usr/bin/env bash\necho "${x#prefix}"\n')
        result = _run_lint(f)
        assert result.returncode == 0, result.stdout

    def test_clean_file_passes(self, tmp_path: Path):
        f = tmp_path / "x.sh"
        f.write_text(
            '#!/usr/bin/env bash\nfor i in 1 2 3; do echo "$i"; done\n'
        )
        result = _run_lint(f)
        assert result.returncode == 0, result.stdout
