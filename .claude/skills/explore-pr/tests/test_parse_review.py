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


SEVERITY_FIRST_REVIEW = """## Inline Comments

### `src/core/store.rs:10-14` — Store drops writes
**Severity**: major | **Confidence**: high | **Lens**: Correctness

The store drops a write when the queue is full.

---

## Per-Lens Results

### Correctness

**Strengths**:
- `src/core/load.rs:1` is tidy.

**Comments**:
- `src/core/store.rs:10-14` (major, high) — Writes are dropped when full.\
 See inline comment.
- `core/load.rs:3` (minor, medium) — The loader retries forever; cap it.

**General Findings**:
- 🔵 (suggestion) Add a property test for the queue.

### Code Quality

**Comments**:
- `src/core/store.rs:12` (RIGHT) — minor, high — The drop path is untested.\
 Inline above.

**General Findings**:
- minor — The queue type has two names.
"""


def test_a_comment_written_severity_first_is_read(script):
    findings = script("parse_review").parse(
        SEVERITY_FIRST_REVIEW, CHANGED, prefix="R1"
    )

    loader = next(f for f in findings if f["file"] == "src/core/load.rs")
    assert (loader["severity"], loader["start"], loader["kind"]) == (
        "minor",
        3,
        "comment",
    )
    assert loader["summary"] == "The loader retries forever; cap it."


def test_a_comment_that_points_at_its_inline_merges_into_it(script):
    findings = script("parse_review").parse(
        SEVERITY_FIRST_REVIEW, CHANGED, prefix="R1"
    )

    store = [f for f in findings if f["file"] == "src/core/store.rs"]
    assert len(store) == 1
    assert store[0]["lenses"] == ["correctness", "code-quality"]
    assert store[0]["summary"] == "Writes are dropped when full."


def test_a_general_finding_led_by_a_severity_marker_is_read(script):
    findings = script("parse_review").parse(
        SEVERITY_FIRST_REVIEW, CHANGED, prefix="R1"
    )

    general = [
        (f["severity"], f["summary"])
        for f in findings
        if f["kind"] == "general"
    ]
    assert general == [
        ("minor", "The queue type has two names."),
        ("suggestion", "Add a property test for the queue."),
    ]


def test_a_pointer_to_the_inline_comments_in_the_plural_merges(script):
    review = SEVERITY_FIRST_REVIEW.replace(
        "See inline comment.", "See the inline comments."
    )

    findings = script("parse_review").parse(review, CHANGED, prefix="R1")

    store = [f for f in findings if f["file"] == "src/core/store.rs"]
    assert len(store) == 1
    assert store[0]["summary"] == "Writes are dropped when full."
