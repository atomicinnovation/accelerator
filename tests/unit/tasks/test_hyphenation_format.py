"""Tests for the hyphenation format guard, ported from the retired test-format
shell guard.

The guard forbids ``work item`` (with a space) wherever it reads as part of a
compound identifier or path — the correct form there is ``work-item``. It
scans ``skills/``, ``scripts/``, ``templates/``, ``README.md`` and
``CHANGELOG.md`` for three targeted patterns and excludes its own source file.

Synthetic ``tmp_path`` trees exercise each pattern and its negative twin, plus
a real-tree assertion that the shipped repository passes the guard.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

_SCAN_DIRS = ("skills", "scripts", "templates")
_SCAN_FILES = ("README.md", "CHANGELOG.md")
_EXCLUDED = "scripts/test-format.sh"

_COMPOUND_HYPHEN = re.compile(r"work item-[a-z]")
_PLURAL_PATH = re.compile(r"work items/")
_CONFIG_KEY = re.compile(r"paths\.work items")
_PATTERNS = (_COMPOUND_HYPHEN, _PLURAL_PATH, _CONFIG_KEY)


def _scan_paths(root: Path) -> list[Path]:
    """Every file the guard reads, in a stable order."""
    paths: list[Path] = []
    for name in _SCAN_DIRS:
        directory = root / name
        if directory.is_dir():
            paths.extend(
                path for path in sorted(directory.rglob("*")) if path.is_file()
            )
    for name in _SCAN_FILES:
        path = root / name
        if path.is_file():
            paths.append(path)
    return paths


def violations(root: Path) -> list[str]:
    """Every ``work item`` identifier/path hit, one message per line."""
    found: list[str] = []
    for path in _scan_paths(root):
        rel = path.relative_to(root).as_posix()
        if rel == _EXCLUDED:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError, OSError:
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            if any(pattern.search(line) for pattern in _PATTERNS):
                found.append(
                    f"{rel}:{number}: 'work item' in identifier/path "
                    "context — use 'work-item'"
                )
    return found


def _write(root: Path, rel: str, content: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def test_compound_hyphen_is_flagged(tmp_path: Path) -> None:
    _write(tmp_path, "scripts/x.sh", "work item-template-field-hints.sh\n")
    assert len(violations(tmp_path)) == 1


def test_hyphenated_compound_is_clean(tmp_path: Path) -> None:
    _write(tmp_path, "scripts/x.sh", "work-item-template-field-hints.sh\n")
    assert violations(tmp_path) == []


def test_compound_hyphen_needs_a_lowercase_letter(tmp_path: Path) -> None:
    _write(tmp_path, "scripts/x.sh", "work item-1 and work item-X\n")
    assert violations(tmp_path) == []


def test_plural_path_component_is_flagged(tmp_path: Path) -> None:
    _write(tmp_path, "skills/a/SKILL.md", "meta/reviews/work items/\n")
    assert len(violations(tmp_path)) == 1


def test_hyphenated_plural_path_is_clean(tmp_path: Path) -> None:
    _write(tmp_path, "skills/a/SKILL.md", "meta/reviews/work-items/\n")
    assert violations(tmp_path) == []


def test_config_key_wrong_plural_is_flagged(tmp_path: Path) -> None:
    _write(tmp_path, "templates/t.md", "paths.work items\n")
    assert len(violations(tmp_path)) == 1


def test_config_key_needs_the_dot(tmp_path: Path) -> None:
    _write(tmp_path, "templates/t.md", "paths work items\n")
    assert violations(tmp_path) == []


def test_prose_plural_without_slash_is_clean(tmp_path: Path) -> None:
    _write(tmp_path, "README.md", "We closed several work items today.\n")
    assert violations(tmp_path) == []


def test_readme_and_changelog_are_scanned(tmp_path: Path) -> None:
    _write(tmp_path, "README.md", "work items/\n")
    _write(tmp_path, "CHANGELOG.md", "paths.work items\n")
    assert len(violations(tmp_path)) == 2


def test_the_guards_own_source_is_excluded(tmp_path: Path) -> None:
    _write(tmp_path, _EXCLUDED, "work items/ and paths.work items\n")
    assert violations(tmp_path) == []


def test_files_outside_the_scan_targets_are_ignored(tmp_path: Path) -> None:
    _write(tmp_path, "meta/work items/note.md", "work items/\n")
    _write(tmp_path, "tasks/thing.py", "paths.work items\n")
    assert violations(tmp_path) == []


def test_a_conforming_tree_yields_no_violations(tmp_path: Path) -> None:
    _write(tmp_path, "skills/a/SKILL.md", "Use work-items/ and work-item-x.\n")
    _write(tmp_path, "README.md", "paths.work-items are hyphenated.\n")
    assert violations(tmp_path) == []


def test_the_real_tree_passes() -> None:
    assert violations(REPO_ROOT) == []
