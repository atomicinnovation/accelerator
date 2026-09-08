"""Tests for the bare-invocation guard in ``tasks/lint/bare_invocation.py``.

Synthetic ``tmp_path`` skill trees exercise each forbidden form and both
carve-outs, plus a real-tree assertion that the converted ``skills/`` passes.
"""

from pathlib import Path

import pytest

from tasks.lint import bare_invocation
from tasks.shared.sources import repo_root


def _skill(root: Path, name: str, body: str) -> None:
    path = root / "skills" / name / "SKILL.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"---\nname: {name}\n---\n{body}\n")


def test_the_prefixed_pathed_form_is_flagged(tmp_path: Path) -> None:
    _skill(
        tmp_path,
        "demo",
        "!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config x --fail-safe`",
    )
    found = bare_invocation.violations(tmp_path)
    assert any("skills/demo/SKILL.md" in v for v in found)


def test_an_absolute_path_render_is_flagged(tmp_path: Path) -> None:
    _skill(
        tmp_path,
        "demo",
        "!`/opt/plugin/bin/accelerator config x --fail-safe`",
    )
    found = bare_invocation.violations(tmp_path)
    assert any("skills/demo/SKILL.md" in v for v in found)


@pytest.mark.parametrize("wrapper", ["bash", "sh", "env"])
def test_a_shell_wrapper_is_flagged(tmp_path: Path, wrapper: str) -> None:
    _skill(tmp_path, "demo", f"!`{wrapper} accelerator config x --fail-safe`")
    found = bare_invocation.violations(tmp_path)
    assert any("skills/demo/SKILL.md" in v for v in found)


def test_a_shell_wrapper_in_a_fenced_block_is_flagged(tmp_path: Path) -> None:
    _skill(tmp_path, "demo", "```\nbash accelerator config x\n```")
    found = bare_invocation.violations(tmp_path)
    assert any("skills/demo/SKILL.md" in v for v in found)


def test_a_clean_bare_skill_yields_none(tmp_path: Path) -> None:
    _skill(tmp_path, "demo", "!`accelerator config x --fail-safe`")
    assert bare_invocation.violations(tmp_path) == []


def test_a_plugin_skills_reference_is_not_flagged(tmp_path: Path) -> None:
    # The lens / output-format references carry `/skills/`, not
    # `/bin/accelerator`, so they must survive the scan.
    _skill(
        tmp_path,
        "demo",
        "Read the output format at: "
        "${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/x/SKILL.md",
    )
    assert bare_invocation.violations(tmp_path) == []


def test_prose_naming_the_launcher_is_not_flagged(tmp_path: Path) -> None:
    # `sh` occurs inside `publish`; the leading-command anchor spares it.
    _skill(
        tmp_path,
        "demo",
        "You can publish accelerator output directly to the user.",
    )
    assert bare_invocation.violations(tmp_path) == []


def test_the_real_skills_tree_passes() -> None:
    assert bare_invocation.violations(repo_root()) == []
