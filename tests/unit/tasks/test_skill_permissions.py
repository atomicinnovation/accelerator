"""Tests for the SKILL.md permission/census guard in
``tasks/lint/skill_permissions.py``.

Synthetic ``tmp_path`` skill trees exercise each branch, plus a real-tree
assertion that the shipped ``skills/`` passes the guard.
"""

from pathlib import Path

from tasks.lint import skill_permissions

REPO_ROOT = Path(__file__).resolve().parents[3]

_RULE = "Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)"
_ACC = "${CLAUDE_PLUGIN_ROOT}/bin/accelerator config"


def _inject(rest: str) -> str:
    return f"!`{_ACC} {rest}`"


def _skill(root: Path, name: str, rules: str, body: str) -> None:
    path = root / "skills" / name / "SKILL.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"---\nname: {name}\nallowed-tools: {rules}\n---\n{body}\n")


def _injecting_body(name: str) -> str:
    return (
        f"!`${{CLAUDE_PLUGIN_ROOT}}/bin/accelerator config context "
        f"--skill {name} --fail-safe`\n\n"
        f"!`${{CLAUDE_PLUGIN_ROOT}}/bin/accelerator config instructions "
        f"{name} --fail-safe`"
    )


def test_missing_fail_safe_is_flagged(tmp_path: Path) -> None:
    _skill(
        tmp_path,
        "demo",
        _RULE,
        "!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config get x`",
    )
    assert any(
        "missing --fail-safe" in v
        for v in skill_permissions.violations(tmp_path)
    )


def test_uncovered_command_is_flagged(tmp_path: Path) -> None:
    _skill(
        tmp_path,
        "demo",
        "Bash(${CLAUDE_PLUGIN_ROOT}/scripts/other.sh)",
        "!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config get x --fail-safe`",
    )
    assert any(
        "not covered" in v for v in skill_permissions.violations(tmp_path)
    )


def test_metacharacter_is_flagged(tmp_path: Path) -> None:
    _skill(
        tmp_path,
        "demo",
        _RULE,
        _inject("get x --fail-safe && rm"),
    )
    assert any(
        "metacharacter" in v for v in skill_permissions.violations(tmp_path)
    )


def test_bare_launcher_rule_is_flagged(tmp_path: Path) -> None:
    _skill(tmp_path, "demo", "Bash(${CLAUDE_PLUGIN_ROOT}/*)", "body")
    assert any(
        "without a subcommand" in v
        for v in skill_permissions.violations(tmp_path)
    )


def test_skill_name_mismatch_is_flagged(tmp_path: Path) -> None:
    _skill(
        tmp_path,
        "demo",
        _RULE,
        "!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context "
        "--skill wrong-name --fail-safe`",
    )
    assert any(
        "does not name this skill's frontmatter name" in v
        for v in skill_permissions.violations(tmp_path)
    )


def test_instructions_not_last_is_flagged(tmp_path: Path) -> None:
    body = (
        _inject("instructions demo --fail-safe")
        + "\n\n"
        + _inject("get x --fail-safe")
    )
    _skill(tmp_path, "demo", _RULE, body)
    assert any(
        "is not the last" in v for v in skill_permissions.violations(tmp_path)
    )


def test_census_count_mismatch_is_flagged(tmp_path: Path) -> None:
    # Zero injecting skills against the expected non-zero floor.
    _skill(tmp_path, "demo", _RULE, "no injection here")
    flagged = skill_permissions.violations(tmp_path)
    assert any("context injection present in 0" in v for v in flagged)
    assert any("instructions injection present in 0" in v for v in flagged)


def test_a_clean_injecting_skill_only_trips_the_census(tmp_path: Path) -> None:
    # A well-formed injecting skill has no per-skill violation; only the global
    # count differs from the expected 42 in this one-skill tree.
    _skill(tmp_path, "demo", _RULE, _injecting_body("demo"))
    per_skill = [
        v
        for v in skill_permissions.violations(tmp_path)
        if "injection present" not in v
    ]
    assert per_skill == []


def test_the_real_skills_tree_passes() -> None:
    assert skill_permissions.violations(REPO_ROOT) == []
