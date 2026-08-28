from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks.lint import scripts as lint


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
    def test_raises_on_findings(self, ctx, mocker):
        mocker.patch.object(lint, "shell_sources", return_value=["a.sh"])
        mocker.patch.object(
            lint, "scan_bashisms", return_value=["a.sh:1: bash-4 construct: x"]
        )
        with pytest.raises(Exit):
            lint.bashisms(ctx)

    def test_passes_when_the_scanner_is_clean(self, ctx, mocker):
        mocker.patch.object(lint, "shell_sources", return_value=["a.sh"])
        mocker.patch.object(lint, "scan_bashisms", return_value=[])
        lint.bashisms(ctx)  # must not raise

    def test_raises_on_empty_source_set(self, ctx, mocker):
        # Fail-closed, as for shellcheck — no source to scan means discovery
        # broke, not that the tree is clean.
        mocker.patch.object(lint, "shell_sources", return_value=[])
        with pytest.raises(Exit):
            lint.bashisms(ctx)


def _scan(tmp_path: Path, content: str, name: str = "x.sh") -> list[str]:
    (tmp_path / name).write_text(content, encoding="utf-8")
    return lint.scan_bashisms([name], tmp_path)


class TestBashismsScanner:
    """The Python denylist scanner, driven over synthetic tmp files."""

    def test_flags_associative_array(self, tmp_path: Path):
        found = _scan(tmp_path, "#!/usr/bin/env bash\ndeclare -A MAP\n")
        assert found and "associative array" in found[0]

    def test_flags_nameref(self, tmp_path: Path):
        found = _scan(
            tmp_path, '#!/usr/bin/env bash\nf() { local -n ref="$1"; }\n'
        )
        assert found and "nameref" in found[0]

    def test_flags_escaped_brace_in_expansion_default(self, tmp_path: Path):
        found = _scan(tmp_path, '#!/usr/bin/env bash\nv="${1:-{\\}}"\n')
        assert found and "escaped brace" in found[0]

    def test_flags_mapfile(self, tmp_path: Path):
        found = _scan(tmp_path, "#!/usr/bin/env bash\nmapfile -t arr <f\n")
        assert found and "mapfile/readarray" in found[0]

    def test_flags_readarray(self, tmp_path: Path):
        found = _scan(tmp_path, "#!/usr/bin/env bash\nreadarray -t arr <f\n")
        assert found and "mapfile/readarray" in found[0]

    def test_flags_case_modification_expansion(self, tmp_path: Path):
        found = _scan(tmp_path, '#!/usr/bin/env bash\necho "${x^^}"\n')
        assert found and "case-modification" in found[0]

    def test_flags_append_both_redirect(self, tmp_path: Path):
        found = _scan(tmp_path, "#!/usr/bin/env bash\ncmd &>>log\n")
        assert found and "append-both" in found[0]

    def test_flags_pipe_both(self, tmp_path: Path):
        found = _scan(tmp_path, "#!/usr/bin/env bash\ncmd |& grep x\n")
        assert found and "pipe-both" in found[0]

    def test_flags_negative_array_subscript(self, tmp_path: Path):
        found = _scan(tmp_path, '#!/usr/bin/env bash\necho "${arr[-1]}"\n')
        assert found and "negative array subscript" in found[0]

    def test_flags_an_indented_mid_line_offender(self, tmp_path: Path):
        # Position independence: the offender is neither at column zero nor the
        # first token on the line.
        found = _scan(tmp_path, "#!/usr/bin/env bash\n    x=1; declare -A M\n")
        assert found and "associative array" in found[0]
        assert found[0].startswith("x.sh:2:")

    def test_unescaped_braces_in_default_are_not_flagged(self, tmp_path: Path):
        found = _scan(
            tmp_path, '#!/usr/bin/env bash\nv="${1:-{}}"\necho "$v"\n'
        )
        assert found == []

    def test_substitution_with_escaped_brace_not_flagged(self, tmp_path: Path):
        found = _scan(
            tmp_path, '#!/usr/bin/env bash\nv="${var//\\}/x}"\necho "$v"\n'
        )
        assert found == []

    def test_comment_naming_a_construct_is_not_flagged(self, tmp_path: Path):
        found = _scan(
            tmp_path,
            "#!/usr/bin/env bash\n# do not use declare -A here\necho ok\n",
        )
        assert found == []

    def test_inline_opt_out_marker(self, tmp_path: Path):
        found = _scan(
            tmp_path,
            "#!/usr/bin/env bash\ndeclare -A MAP # lint-bashisms: ignore\n",
        )
        assert found == []

    def test_parameter_expansion_strip_is_not_a_case_mod(self, tmp_path: Path):
        found = _scan(tmp_path, '#!/usr/bin/env bash\necho "${x#prefix}"\n')
        assert found == []

    def test_a_plain_flag_near_miss_is_not_flagged(self, tmp_path: Path):
        # `declare -x` is a bash-3.2 export, not `-A`/`-n`.
        found = _scan(tmp_path, "#!/usr/bin/env bash\ndeclare -x VAR\n")
        assert found == []

    def test_clean_file_passes(self, tmp_path: Path):
        found = _scan(
            tmp_path,
            '#!/usr/bin/env bash\nfor i in 1 2 3; do echo "$i"; done\n',
        )
        assert found == []
