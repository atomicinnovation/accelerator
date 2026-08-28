"""Guard that the canonical hierarchy tree fence is byte-for-byte identical.

Ported from ``scripts/test-hierarchy-format.sh`` (the guard) and the
hierarchy-format cases of ``scripts/test-evals-structure-self.sh`` (its
meta-tests over fixture pairs).

Both ``list-work-items/SKILL.md`` and ``refine-work-item/SKILL.md`` bracket
the canonical hierarchy example with::

    <!-- canonical-tree-fence -->
    ...tree...
    <!-- /canonical-tree-fence -->

The fences must match exactly; a missing marker or an empty fence is also a
violation. Synthetic ``tmp_path`` trees exercise each branch, the carried
fixture pairs drive the folded self-tests, and a live-tree assertion checks
the shipped SKILL.md pair.
"""

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "hierarchy-format"

FENCE_OPEN = "<!-- canonical-tree-fence -->"
FENCE_CLOSE = "<!-- /canonical-tree-fence -->"

CANONICAL_FILES = (
    "skills/work/list-work-items/SKILL.md",
    "skills/work/refine-work-item/SKILL.md",
)


def extract_fence(text: str) -> str:
    """Return the lines between the fence markers, mirroring the awk pass."""
    captured: list[str] = []
    inside = False
    for line in text.splitlines():
        if FENCE_OPEN in line:
            inside = True
            continue
        if FENCE_CLOSE in line:
            inside = False
        if inside:
            captured.append(line)
    return "\n".join(captured)


def _capture(text: str) -> str:
    """Fence content with trailing newlines dropped, as ``$(...)`` would."""
    return extract_fence(text).rstrip("\n")


def _extraction_violations(rel: str, text: str) -> list[str]:
    """First extraction failure for one file, or none (check_extraction)."""
    if FENCE_OPEN not in text:
        return [f"{rel}: marker missing — opening fence not found"]
    if FENCE_CLOSE not in text:
        return [f"{rel}: marker missing — closing fence not found"]
    if not _capture(text):
        return [f"{rel}: empty extraction — fence block is empty"]
    return []


def fence_violations(file_a: Path, file_b: Path) -> list[str]:
    """Every violation for a fence pair; empty list means the fences match."""
    text_a = file_a.read_text()
    text_b = file_b.read_text()

    found: list[str] = []
    found.extend(_extraction_violations(str(file_a), text_a))
    found.extend(_extraction_violations(str(file_b), text_b))
    if found:
        return found

    if _capture(text_a) != _capture(text_b):
        return [f"canonical tree fences differ between {file_a} and {file_b}"]
    return []


def violations(root: Path) -> list[str]:
    """Violations across the live canonical SKILL.md fence pair."""
    file_a, file_b = (root / rel for rel in CANONICAL_FILES)
    return fence_violations(file_a, file_b)


def _write(path: Path, body: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)
    return path


def _fenced(inner: str) -> str:
    return f"# Heading\n\n{FENCE_OPEN}\n{inner}\n{FENCE_CLOSE}\n"


_TREE = (
    "NNNN — parent title (type: <type>, status: <status>)\n"
    "  └── NNNN — child title (type: <type>, status: <status>)"
)


def test_matching_fences_produce_no_violations(tmp_path: Path) -> None:
    file_a = _write(tmp_path / "a.md", _fenced(_TREE))
    file_b = _write(tmp_path / "b.md", _fenced(_TREE))
    assert fence_violations(file_a, file_b) == []


def test_differing_fences_are_flagged(tmp_path: Path) -> None:
    file_a = _write(tmp_path / "a.md", _fenced(_TREE))
    file_b = _write(
        tmp_path / "b.md", _fenced(_TREE.replace("child", "CHANGED"))
    )
    assert any("differ" in v for v in fence_violations(file_a, file_b))


def test_surrounding_content_is_ignored(tmp_path: Path) -> None:
    file_a = _write(
        tmp_path / "a.md", "Intro A\n\n" + _fenced(_TREE) + "\nEnd A"
    )
    file_b = _write(
        tmp_path / "b.md", "Intro B\n\n" + _fenced(_TREE) + "\nEnd B"
    )
    assert fence_violations(file_a, file_b) == []


def test_missing_opening_marker_is_flagged(tmp_path: Path) -> None:
    file_a = _write(tmp_path / "a.md", _fenced(_TREE))
    file_b = _write(tmp_path / "b.md", f"# No fence\n\n{_TREE}\n")
    assert any(
        "opening fence not found" in v for v in fence_violations(file_a, file_b)
    )


def test_missing_closing_marker_is_flagged(tmp_path: Path) -> None:
    file_a = _write(tmp_path / "a.md", _fenced(_TREE))
    file_b = _write(
        tmp_path / "b.md", f"# Open only\n\n{FENCE_OPEN}\n{_TREE}\n"
    )
    assert any(
        "closing fence not found" in v for v in fence_violations(file_a, file_b)
    )


def test_empty_fence_is_flagged(tmp_path: Path) -> None:
    file_a = _write(tmp_path / "a.md", _fenced(_TREE))
    file_b = _write(
        tmp_path / "b.md", f"# Empty\n\n{FENCE_OPEN}\n{FENCE_CLOSE}\n"
    )
    assert any(
        "fence block is empty" in v for v in fence_violations(file_a, file_b)
    )


def _pair(name: str) -> tuple[Path, Path]:
    return FIXTURES / name / "file-a.md", FIXTURES / name / "file-b.md"


def test_matched_fences_fixture_passes() -> None:
    assert fence_violations(*_pair("matched-fences")) == []


def test_mismatched_fences_fixture_fails() -> None:
    assert fence_violations(*_pair("mismatched-fences")) != []


def test_missing_marker_fixture_fails() -> None:
    assert fence_violations(*_pair("missing-marker")) != []


def test_the_live_canonical_fences_match() -> None:
    assert violations(REPO_ROOT) == []
