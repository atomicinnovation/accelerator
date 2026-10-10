import pytest

from tasks.shared.vendor.pin_guard.chromium_versions import (
    ChromiumVersion,
    ChromiumVersionError,
)


def test_a_four_part_version_parses_and_renders():
    version = ChromiumVersion.parse("140.0.7339.186")
    assert version.parts == (140, 0, 7339, 186)
    assert version.major == 140
    assert str(version) == "140.0.7339.186"


def test_versions_compare_numerically_part_by_part():
    assert ChromiumVersion.parse("120.0.6099.9") < ChromiumVersion.parse(
        "120.0.6099.10"
    )


@pytest.mark.parametrize(
    "text",
    ["26.2", "18.7.3", "140.0.7339", "1.2.3.4.5", "140.0.x.1", " 1.2.3.4"],
)
def test_anything_but_four_numeric_parts_is_refused(text):
    with pytest.raises(ChromiumVersionError):
        ChromiumVersion.parse(text)
