"""Tests for the call-site migration gate in
``tasks/lint/call_site_migration.py``.

Synthetic ``tmp_path`` trees exercise Grep A (functional vs mention), Grep B,
and the ``--allow-legacy-layout`` confinement, plus a real-tree assertion that
the shipped tree carries no gated violation.
"""

from pathlib import Path

from tasks.lint import call_site_migration as gate

REPO_ROOT = Path(__file__).resolve().parents[3]


def _write(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_functional_invocation_is_flagged(tmp_path: Path) -> None:
    _write(tmp_path, "skills/x/run.sh", 'bash "$DIR/config-read-value.sh"\n')
    hits = gate.functional_hits(tmp_path)
    assert any("skills/x/run.sh" in h for h in hits)


def test_comment_mention_is_not_flagged(tmp_path: Path) -> None:
    _write(
        tmp_path, "skills/x/run.sh", "# once used config-read-value.sh here\n"
    )
    assert gate.functional_hits(tmp_path) == []
    assert gate.mention_count(tmp_path) == 1


def test_removal_set_member_is_excluded(tmp_path: Path) -> None:
    _write(
        tmp_path, "scripts/config-read-value.sh", "exec config-read-path.sh\n"
    )
    assert gate.functional_hits(tmp_path) == []


def test_pruned_and_changelog_are_ignored(tmp_path: Path) -> None:
    _write(tmp_path, "meta/notes.md", "bash config-read-value.sh\n")
    _write(tmp_path, "CHANGELOG.md", "bash config-read-value.sh\n")
    assert gate.functional_hits(tmp_path) == []


def test_grep_b_flags_a_skill_md_script_reference(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "skills/x/SKILL.md",
        "body\n!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh`\n",
    )
    assert gate.grep_b_hits(tmp_path)


def test_grep_b_permits_the_browser_executor(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "skills/x/SKILL.md",
        "!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-browser-executor.sh`\n",
    )
    assert gate.grep_b_hits(tmp_path) == []


def test_stray_legacy_flag_is_flagged(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "scripts/rogue.sh",
        "accelerator config get --allow-legacy-layout x\n",
    )
    assert "scripts/rogue.sh" in gate.stray_legacy_flag(tmp_path)


def test_legacy_flag_in_migrations_is_permitted(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "skills/config/migrate/migrations/0001-x.sh",
        "accelerator config path --allow-legacy-layout paths.tickets\n",
    )
    assert gate.stray_legacy_flag(tmp_path) == []


def test_the_real_tree_has_no_gated_violation() -> None:
    assert gate.violations(REPO_ROOT) == []
