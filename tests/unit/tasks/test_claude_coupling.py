"""Tests for the CLAUDE_* boundary guard.

Every probe stays in ``tmp_path``. A sentinel written into the live tree would
make the checks flake: ``test:unit:tasks`` runs concurrently with ``cli:check``
under ``mise run``, and the guard rides in the latter.
"""

from pathlib import Path

import pytest
from invoke import Exit

from tasks.lint.claude_coupling import (
    _FILES,
    _MIN_SCANNED,
    _OPTIONAL_HARNESS_VARIABLES,
    _in_scope,
    violations,
)

REPO_ROOT = Path(__file__).resolve().parents[3]

_READ = 'std::env::var_os("CLAUDE_PLUGIN_ROOT")\n'


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def _tree(root: Path) -> Path:
    """A minimal in-scope tree: every named file plus one cli/ source."""
    admitted = [
        rel for files in _OPTIONAL_HARNESS_VARIABLES.values() for rel in files
    ]
    for rel in [*_FILES, *admitted]:
        _write(root / rel, "clean\n")
    _write(root / "cli/launcher/src/main.rs", "fn main() {}\n")
    return root


def _found(root: Path) -> list[str]:
    return violations(root, min_scanned=1)


class TestFlagging:
    def test_flags_a_var_os_read_under_cli(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / "cli/launcher/src/main.rs", _READ)

        found = _found(tmp_path)
        assert found == ["cli/launcher/src/main.rs:1:" + _READ.strip()]

    def test_flags_a_comment_mention(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / "cli/x/src/lib.rs", "// needs CLAUDE_PLUGIN_ROOT\n")

        assert any("lib.rs" in hit for hit in _found(tmp_path))

    def test_flags_an_mjs_injection(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / "cli/f/e2e/start.mjs", "CLAUDE_PLUGIN_ROOT: p,\n")

        assert any("start.mjs" in hit for hit in _found(tmp_path))

    def test_flags_a_markdown_and_a_shell_file_under_cli(self, tmp_path: Path):
        # Scan-by-default: nothing narrows the guard to source suffixes.
        _tree(tmp_path)
        _write(tmp_path / "cli/README.md", "Set CLAUDE_PLUGIN_ROOT first.\n")
        _write(tmp_path / "cli/run.sh", 'echo "$CLAUDE_PLUGIN_ROOT"\n')

        hits = " ".join(_found(tmp_path))
        assert "cli/README.md" in hits
        assert "cli/run.sh" in hits

    def test_flags_a_read_reintroduced_into_the_bootstrap(self, tmp_path: Path):
        # The entry point the bug was in, and where the transitional export
        # lived — the reason the guard's scope is the rename set, not cli/.
        _tree(tmp_path)
        _write(tmp_path / "bin/accelerator", 'export CLAUDE_PLUGIN_ROOT="$r"\n')

        assert any("bin/accelerator" in hit for hit in _found(tmp_path))

    def test_flags_a_read_reintroduced_into_an_out_of_tree_writer(
        self, tmp_path: Path
    ):
        _tree(tmp_path)
        _write(tmp_path / "tasks/dev.py", 'env["CLAUDE_PLUGIN_ROOT"] = r\n')

        assert any("tasks/dev.py" in hit for hit in _found(tmp_path))

    def test_reports_path_line_and_text(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / "cli/x/src/lib.rs", f"fn a() {{}}\n{_READ}")

        assert _found(tmp_path) == [
            "cli/x/src/lib.rs:2:" + _READ.strip(),
        ]

    def test_flags_any_claude_prefixed_name_not_only_the_plugin_root(
        self, tmp_path: Path
    ):
        _tree(tmp_path)
        _write(tmp_path / "cli/x/src/lib.rs", 'var("CLAUDE_CONFIG_DIR")\n')

        assert any("CLAUDE_CONFIG_DIR" in hit for hit in _found(tmp_path))


class TestScopeBoundaries:
    def test_does_not_descend_into_gitignored_build_trees(self, tmp_path: Path):
        # cli/target/ is gitignored; node_modules/ is too. Both need the
        # .gitignore written into tmp_path — _ignore_spec reads only that file.
        _tree(tmp_path)
        _write(tmp_path / ".gitignore", "cli/target/\nnode_modules/\n")
        _write(tmp_path / "cli/target/debug/build.rs", _READ)
        _write(tmp_path / "cli/f/node_modules/p/index.js", _READ)

        assert _found(tmp_path) == []

    def test_does_not_descend_into_unconditionally_pruned_build_output(
        self, tmp_path: Path
    ):
        # dist/ and playwright-report/ are pruned by walk_files regardless of
        # any .gitignore, which is what keeps the minified SPA bundle out.
        _tree(tmp_path)
        _write(tmp_path / "cli/f/dist/assets/bundle.js", _READ)
        _write(tmp_path / "cli/f/playwright-report/trace.txt", _READ)

        assert _found(tmp_path) == []

    def test_does_not_flag_the_matcher_model_or_the_adapter_layer(
        self, tmp_path: Path
    ):
        # Both legitimately name the variable and are outside the rename set.
        _tree(tmp_path)
        _write(tmp_path / "tasks/lint/skill_permissions.py", _READ)
        _write(tmp_path / "hooks/config-detect.sh", _READ)

        assert _found(tmp_path) == []


class TestFailClosed:
    def test_a_scan_below_the_floor_raises(self, tmp_path: Path):
        # A silently-emptied scan otherwise reads as cleanliness in both the
        # lint task and the `violations(REPO_ROOT) == []` assertion.
        _tree(tmp_path)

        with pytest.raises(Exit, match="scope discovery is broken"):
            violations(tmp_path)

    def test_a_missing_named_file_raises(self, tmp_path: Path):
        _tree(tmp_path)
        (tmp_path / "tasks/test/helpers.py").unlink()

        with pytest.raises(Exit, match="silently dropped out"):
            _found(tmp_path)

    def test_undecodable_bytes_are_skipped_rather_than_raising(
        self, tmp_path: Path
    ):
        # An unlisted suffix, so _SKIP_SUFFIXES cannot be what saves it — the
        # strict decode is.
        _tree(tmp_path)
        (tmp_path / "cli/asset.dat").write_bytes(b"\xff\xfe\x00binary")

        assert _found(tmp_path) == []


class TestRealTree:
    def test_the_scanned_set_clears_its_floor(self):
        # Guards the guard: a silently-emptied scan otherwise reads as a clean
        # tree in the assertion below.
        assert len(_in_scope(REPO_ROOT)) >= _MIN_SCANNED

    def test_the_real_tree_is_clean(self):
        assert violations(REPO_ROOT) == []


_CAP = "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"
_CAP_READER = "cli/research-cli/src/subagent_cap.rs"
_CAP_TEST = "cli/research-cli/tests/topic_outstanding.rs"
_CAP_READ = f'const HARNESS_CAPS: [&str; 1] = ["{_CAP}"];\n'


class TestOptionalHarnessVariables:
    def test_permits_the_cap_in_its_reader_and_its_test(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / _CAP_READER, _CAP_READ)
        _write(tmp_path / _CAP_TEST, f'.env("{_CAP}", "8")\n')

        assert _found(tmp_path) == []

    def test_flags_the_cap_anywhere_else_under_cli(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / "cli/research-cli/src/main.rs", _CAP_READ)

        assert _found(tmp_path) == [
            "cli/research-cli/src/main.rs:1:" + _CAP_READ.strip()
        ]

    def test_flags_the_cap_in_bin_accelerator(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / "bin/accelerator", f'echo "${_CAP}"\n')

        assert any("bin/accelerator" in hit for hit in _found(tmp_path))

    def test_flags_another_claude_name_in_the_reader(self, tmp_path: Path):
        _tree(tmp_path)
        _write(tmp_path / _CAP_READER, _READ)

        assert _found(tmp_path) == [f"{_CAP_READER}:1:" + _READ.strip()]

    def test_flags_a_name_extending_the_cap_in_the_reader(self, tmp_path: Path):
        _tree(tmp_path)
        line = f'std::env::var_os("{_CAP}_OVERRIDE")\n'
        _write(tmp_path / _CAP_READER, line)

        assert _found(tmp_path) == [f"{_CAP_READER}:1:" + line.strip()]

    def test_flags_a_line_naming_the_cap_beside_a_banned_name(
        self, tmp_path: Path
    ):
        _tree(tmp_path)
        line = f'["{_CAP}", "CLAUDE_PLUGIN_ROOT"]\n'
        _write(tmp_path / _CAP_READER, line)

        assert _found(tmp_path) == [f"{_CAP_READER}:1:" + line.strip()]

    def test_the_admitted_set_is_exactly_the_subagent_cap(self):
        assert _OPTIONAL_HARNESS_VARIABLES == {_CAP: (_CAP_READER, _CAP_TEST)}

    def test_a_missing_reader_raises(self, tmp_path: Path):
        _tree(tmp_path)
        (tmp_path / _CAP_READER).unlink()

        with pytest.raises(Exit, match="silently dropped out"):
            _found(tmp_path)
