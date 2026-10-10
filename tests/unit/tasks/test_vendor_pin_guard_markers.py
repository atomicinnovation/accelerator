import pytest

from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind

MARKER = IssueMarker(MarkerKind.FINDING, ("pin-age", "chromium", "1193"))
RENDERED = "<!-- finding: pin-age chromium 1193 -->"


def test_a_marker_renders_as_a_hidden_comment():
    assert MARKER.render() == RENDERED


@pytest.mark.parametrize(
    "body",
    [
        f"Body text.\n\n{RENDERED}",
        f"Body text.\n\n{RENDERED}\n",
        f"Body text.\n\n{RENDERED}\n\n\n",
        f"Body text.\r\n\r\n{RENDERED}\r\n",
        f"Body text.\n\n{RENDERED}   \n",
    ],
)
def test_the_marker_on_the_last_line_is_read(body):
    assert IssueMarker.of_body(body) == MARKER


def test_a_marker_above_a_different_last_line_is_ignored():
    assert IssueMarker.of_body(f"{RENDERED}\nA note added later.") is None


def test_a_body_without_a_marker_has_none():
    assert IssueMarker.of_body("Just a body.") is None


def test_an_empty_body_has_no_marker():
    assert IssueMarker.of_body("") is None


@pytest.mark.parametrize(
    "line",
    [
        "<!-- unknown-kind: pin-age -->",
        "<!-- finding: -->",
        "<!-- finding: pin@age -->",
        "<!--finding: pin-age-->",
    ],
)
def test_a_malformed_last_line_has_no_marker(line):
    assert IssueMarker.of_body(line) is None


def test_every_kind_round_trips():
    for kind in MarkerKind:
        marker = IssueMarker(kind, ("a", "b.c_d-1"))
        assert IssueMarker.of_body(f"x\n{marker.render()}") == marker


def test_empty_parts_are_refused():
    with pytest.raises(ValueError, match="part"):
        IssueMarker(MarkerKind.FINDING, ())


@pytest.mark.parametrize("part", ["-->", "two words", "@user", "", "a\nb"])
def test_a_part_outside_the_safe_alphabet_is_refused(part):
    with pytest.raises(ValueError, match="part"):
        IssueMarker(MarkerKind.FINDING, ("pin-age", part))


def test_markers_order_by_kind_then_parts():
    later = IssueMarker(MarkerKind.FINDING, ("pin-age", "node", "22"))
    assert sorted([later, MARKER]) == [MARKER, later]
