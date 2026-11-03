REVIEW = """---
type: "pr-review"
---

## Code Review: #9 - Example

### Additional Findings

- noise

## Inline Comments

### `src/core/store.rs:10-14` — Store drops writes (correctness)
**Severity**: major | **Confidence**: high | **Lens**: correctness

The store drops a write when the queue is full.

---

## Per-Lens Results

### Correctness

**Comments**:
- `core/store.rs:10-14` (RIGHT), major/high: writes are dropped when full.\
 *(Merged inline.)*
- `core/load.rs:3` (RIGHT), minor/medium: the loader retries forever.

**General Findings**:
- suggestion: add a property test for the queue.

### Code Quality

**Comments**:
- `core/store.rs:12` (RIGHT), minor/high: the drop path is untested.\
 *(Merged into the correctness inline.)*

**General Findings**: none.
"""

CHANGED = ["src/core/store.rs", "src/core/load.rs"]


def test_lens_comments_merge_into_the_inline_comment_they_belong_to(script):
    findings = script("parse_review").parse(REVIEW, CHANGED, prefix="R1")

    store = [f for f in findings if f["file"] == "src/core/store.rs"]
    assert len(store) == 1
    assert store[0]["lenses"] == ["correctness", "code-quality"]
    assert store[0]["summary"] == "writes are dropped when full."
    assert "queue is full" in store[0]["body"]


def test_findings_are_ordered_by_severity_and_numbered(script):
    findings = script("parse_review").parse(REVIEW, CHANGED, prefix="R1")

    assert [(f["id"], f["severity"]) for f in findings] == [
        ("R1-01", "major"),
        ("R1-02", "minor"),
        ("R1-03", "suggestion"),
    ]


def test_short_paths_expand_to_the_changed_file_they_name(script):
    findings = script("parse_review").parse(REVIEW, CHANGED, prefix="R1")

    loader = next(f for f in findings if f["severity"] == "minor")
    assert (loader["file"], loader["start"]) == ("src/core/load.rs", 3)


def test_a_general_finding_has_no_location(script):
    findings = script("parse_review").parse(REVIEW, CHANGED, prefix="R1")

    general = findings[-1]
    assert general["kind"] == "general"
    assert general["file"] is None
