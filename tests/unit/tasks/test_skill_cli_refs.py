"""Tests for the SKILL<->CLI reference guard in ``tasks/lint/skill_cli_refs.py``
(0286).

Two invariants: every ``accelerator vcs <sub>`` a skill names resolves to a real
subcommand (the subcommand set pinned against the clap ``Command`` enum by a
cross-language test here), and the diff-range and user-identity idiom anchors
still exist in the ``detect.rs`` reference consts.
"""

import re
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks.lint import skill_cli_refs
from tasks.shared.paths import REPO_ROOT
from tasks.shared.sources import repo_root

_CLI_RS = REPO_ROOT / "cli/vcs-cli/src/cli.rs"
_DETECT_RS = REPO_ROOT / "cli/vcs-cli/src/detect.rs"


@pytest.fixture
def ctx():
    return MagicMock(spec=Context)


def _skill(root: Path, name: str, body: str) -> None:
    path = root / "skills" / name / "SKILL.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"---\nname: {name}\n---\n{body}\n")


def _detect(root: Path, text: str) -> None:
    path = root / "cli/vcs-cli/src/detect.rs"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def _command_variants(text: str) -> set[str]:
    body = re.search(
        r"pub enum Command \{(.*?)^\}", text, re.DOTALL | re.MULTILINE
    )
    assert body, "the Command enum was not found"
    return set(re.findall(r"^    ([A-Z]\w*)", body.group(1), re.MULTILINE))


class TestSubcommandResolution:
    def test_an_unknown_subcommand_is_flagged(self, tmp_path: Path) -> None:
        _skill(tmp_path, "demo", "Run `accelerator vcs bogus` to inspect.")
        found = skill_cli_refs.skill_ref_violations(tmp_path)
        assert any("skills/demo/SKILL.md" in v and "bogus" in v for v in found)

    def test_a_real_subcommand_is_not_flagged(self, tmp_path: Path) -> None:
        _skill(tmp_path, "demo", "Run `accelerator vcs root` to find the root.")
        assert skill_cli_refs.skill_ref_violations(tmp_path) == []


class TestRenderLock:
    def test_a_removed_jj_anchor_is_flagged(self, tmp_path: Path) -> None:
        mutated = _DETECT_RS.read_text().replace("fork_point(trunk() | @)", "x")
        _detect(tmp_path, mutated)
        assert skill_cli_refs.render_lock_violations(tmp_path)

    def test_a_removed_git_anchor_is_flagged(self, tmp_path: Path) -> None:
        mutated = _DETECT_RS.read_text().replace("<trunk>...HEAD", "x")
        _detect(tmp_path, mutated)
        assert skill_cli_refs.render_lock_violations(tmp_path)

    def test_a_removed_jj_fallback_clause_is_flagged(
        self, tmp_path: Path
    ) -> None:
        # The fallback clause is `git config user.name` inside the jj block;
        # removing it must fail even though the git block still carries it.
        text = _DETECT_RS.read_text()
        jj = re.search(
            r"const JJ_COMMAND_REFERENCE: &str = concat!\((.*?)\n\);",
            text,
            re.DOTALL,
        )
        assert jj
        mutated = text.replace(
            jj.group(1),
            jj.group(1).replace("git config user.name", "the git identity"),
        )
        _detect(tmp_path, mutated)
        violations = skill_cli_refs.render_lock_violations(tmp_path)
        assert any("JJ_COMMAND_REFERENCE" in v for v in violations)

    def test_the_unmutated_detect_rs_passes(self, tmp_path: Path) -> None:
        _detect(tmp_path, _DETECT_RS.read_text())
        assert skill_cli_refs.render_lock_violations(tmp_path) == []


class TestCrossLanguagePin:
    def test_vcs_subcommands_match_the_clap_command_enum(self) -> None:
        variants = _command_variants(_CLI_RS.read_text())
        assert {v.lower() for v in variants} == set(
            skill_cli_refs.VCS_SUBCOMMANDS
        )

    def test_the_variant_extractor_sees_an_added_command(self) -> None:
        mutated = _CLI_RS.read_text().replace(
            "pub enum Command {", "pub enum Command {\n    Frobnicate,", 1
        )
        assert "Frobnicate" in _command_variants(mutated)


def test_check_raises_when_violations_exist(ctx, mocker) -> None:
    mocker.patch.object(
        skill_cli_refs, "violations", return_value=["some: problem"]
    )
    with pytest.raises(Exit):
        skill_cli_refs.check(ctx)


def test_check_passes_when_clean(ctx, mocker) -> None:
    mocker.patch.object(skill_cli_refs, "violations", return_value=[])
    skill_cli_refs.check(ctx)


def test_the_real_tree_passes() -> None:
    assert skill_cli_refs.violations(repo_root()) == []
