import re

from tasks.shared.paths import RELEASING_MD
from tasks.shared.vendor.pin_guard.ages import (
    KEYRING_MAXIMUM_AGE_DAYS,
    PIN_MAXIMUM_AGE_DAYS,
)
from tasks.shared.vendor.pin_guard.chromium_advisories import (
    CHROMIUM_COMPONENT_PREFIXES,
    CHROMIUM_COMPONENTS,
    CHROMIUM_VENDOR,
)
from tasks.shared.vendor.pin_guard.issues import MAXIMUM_NEW_ISSUES
from tasks.shared.vendor.pin_guard.keyring import KEY_EXPIRY_WARNING_DAYS
from tasks.shared.vendor.pin_guard.local_inputs import Owner

GUARD_HEADING = "## Vendored-runtime pin guard"


def _guard_section() -> str:
    text = RELEASING_MD.read_text()
    start = text.index(GUARD_HEADING)
    following = re.search(
        r"^## ", text[start + len(GUARD_HEADING) :], re.MULTILINE
    )
    end = start + len(GUARD_HEADING) + following.start() if following else None
    return text[start:end]


def _guard_prose() -> str:
    return " ".join(_guard_section().split())


def test_the_owner_line_names_the_guard_owner():
    assert Owner.from_releasing(RELEASING_MD.read_text()).login == (
        "tobyclemson"
    )


def test_the_guard_section_states_the_key_expiry_warning_window():
    assert f"{KEY_EXPIRY_WARNING_DAYS}-day key-expiry warning window" in (
        _guard_prose()
    )


def test_the_guard_section_states_the_issue_cap():
    assert f"more than {MAXIMUM_NEW_ISSUES} new findings" in _guard_prose()


def test_the_limits_table_gives_every_pin_and_the_keyring_its_age():
    section = _guard_section()
    for subject, days in [
        ("`playwright-core`", PIN_MAXIMUM_AGE_DAYS),
        ("Node", PIN_MAXIMUM_AGE_DAYS),
        ("Chromium", PIN_MAXIMUM_AGE_DAYS),
        ("Node keyring", KEYRING_MAXIMUM_AGE_DAYS),
    ]:
        assert re.search(
            rf"^\| {re.escape(subject)} \|.*\| {days} days \|",
            section,
            re.MULTILINE,
        ), subject


def _feeds_subsection() -> str:
    section = _guard_section()
    start = section.index("### Feeds")
    following = re.search(r"^### ", section[start + 1 :], re.MULTILINE)
    end = start + 1 + following.start() if following else None
    return section[start:end]


def test_the_feeds_subsection_names_every_chromium_component_rule():
    feeds = _feeds_subsection()
    for name in [
        CHROMIUM_VENDOR,
        *CHROMIUM_COMPONENT_PREFIXES,
        *sorted(CHROMIUM_COMPONENTS),
    ]:
        assert f"`{name}`" in feeds, name
