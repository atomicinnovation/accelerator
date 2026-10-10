import datetime as dt

import pytest

from tasks.shared.vendor.pin_guard.local_inputs import (
    LocalInputError,
    LocalInputPaths,
    Owner,
    Pin,
    PinName,
    read_local_inputs,
)
from tests.unit.tasks.shared.doubles import (
    BUMP_SUBJECTS,
    clear_repository,
    pins_text,
)

TODAY = dt.date(2026, 10, 10)
FRESH = dict.fromkeys(BUMP_SUBJECTS, TODAY)


def test_a_clear_repository_yields_every_pin_and_the_owner(tmp_path):
    paths = clear_repository(tmp_path, TODAY)
    inputs = read_local_inputs(paths, TODAY)
    assert inputs.pins == (
        Pin(PinName.PLAYWRIGHT_CORE, "1.55.1", TODAY),
        Pin(PinName.NODE, "22.22.2", TODAY),
        Pin(PinName.CHROMIUM, "1193", TODAY),
    )
    assert inputs.keyring_bumped == TODAY
    assert inputs.owner == Owner("Toby Clemson", "tobyclemson")
    assert inputs.node_keyring == paths.node_keyring


def test_the_repository_own_inputs_parse():
    inputs = read_local_inputs(LocalInputPaths.repository(), TODAY)
    assert inputs.owner.login == "tobyclemson"
    assert inputs.chromium.version == "1193"


@pytest.mark.parametrize("subject", BUMP_SUBJECTS)
def test_a_missing_bump_date_names_its_field(tmp_path, subject):
    dates = {s: d for s, d in FRESH.items() if s != subject}
    paths = clear_repository(tmp_path, TODAY, pins=pins_text(dates))
    with pytest.raises(LocalInputError, match=rf"{subject}\.bumped"):
        read_local_inputs(paths, TODAY)


def test_a_deleted_keyring_table_names_its_bump_date(tmp_path):
    text = pins_text(FRESH).split("[keyring]")[0]
    paths = clear_repository(tmp_path, TODAY, pins=text)
    with pytest.raises(LocalInputError, match=r"keyring\.bumped"):
        read_local_inputs(paths, TODAY)


def test_a_bump_date_after_today_names_its_field(tmp_path):
    tomorrow = TODAY + dt.timedelta(days=1)
    paths = clear_repository(tmp_path, TODAY, bumped={"node": tomorrow})
    with pytest.raises(LocalInputError, match=r"node\.bumped"):
        read_local_inputs(paths, TODAY)


@pytest.mark.parametrize(
    "value", ['"2026-10-01"', "2026-10-01T00:00:00Z", "2026-10-01T00:00:00"]
)
def test_a_bump_date_not_in_local_date_form_names_its_field(tmp_path, value):
    paths = clear_repository(tmp_path, TODAY, bumped={"chromium": value})
    with pytest.raises(LocalInputError, match=r"chromium\.bumped"):
        read_local_inputs(paths, TODAY)


def test_an_impossible_date_names_the_line(tmp_path):
    paths = clear_repository(tmp_path, TODAY, bumped={"keyring": "2026-13-01"})
    with pytest.raises(
        LocalInputError, match=r"pins\.toml line \d+: bumped = 2026-13-01"
    ):
        read_local_inputs(paths, TODAY)


def test_a_missing_node_table_names_the_node_version(tmp_path):
    text = pins_text(FRESH).replace(
        f'[node]\nversion = "22.22.2"\nbumped = {TODAY}\n', ""
    )
    paths = clear_repository(tmp_path, TODAY, pins=text)
    with pytest.raises(LocalInputError, match=r"node\.version"):
        read_local_inputs(paths, TODAY)


def test_a_missing_chromium_revision_names_it(tmp_path):
    text = pins_text(FRESH).replace('revision = "1193"\n', "")
    paths = clear_repository(tmp_path, TODAY, pins=text)
    with pytest.raises(LocalInputError, match=r"chromium\.revision"):
        read_local_inputs(paths, TODAY)


@pytest.mark.parametrize("version", ["22.22", "v22.22.2", "22.22.2-rc.1"])
def test_a_malformed_node_version_names_it(tmp_path, version):
    paths = clear_repository(
        tmp_path, TODAY, pins=pins_text(FRESH, node_version=version)
    )
    with pytest.raises(LocalInputError, match=r"node\.version"):
        read_local_inputs(paths, TODAY)


@pytest.mark.parametrize("revision", ["", "r1193", "1193.1"])
def test_a_malformed_chromium_revision_names_it(tmp_path, revision):
    paths = clear_repository(
        tmp_path, TODAY, pins=pins_text(FRESH, chromium_revision=revision)
    )
    with pytest.raises(LocalInputError, match=r"chromium\.revision"):
        read_local_inputs(paths, TODAY)


def test_a_ranged_playwright_version_names_the_package_json(tmp_path):
    paths = clear_repository(tmp_path, TODAY, playwright="^1.55.1")
    with pytest.raises(LocalInputError, match=r"package\.json"):
        read_local_inputs(paths, TODAY)


def test_a_missing_package_json_names_it(tmp_path):
    paths = clear_repository(tmp_path, TODAY)
    paths.package_json.unlink()
    with pytest.raises(LocalInputError, match=r"package\.json"):
        read_local_inputs(paths, TODAY)


@pytest.mark.parametrize(
    "owner_line",
    ["Owner: Jane Doe", "Owner: (@janedoe)", "Owner: Jane Doe (@jane doe)", ""],
)
def test_an_unparseable_owner_line_is_refused(tmp_path, owner_line):
    paths = clear_repository(tmp_path, TODAY, owner_line=owner_line)
    with pytest.raises(LocalInputError, match=r"RELEASING\.md Owner: line"):
        read_local_inputs(paths, TODAY)


def test_two_owner_lines_are_refused(tmp_path):
    two = "Owner: Jane Doe (@janedoe)\nOwner: John Roe (@johnroe)"
    paths = clear_repository(tmp_path, TODAY, owner_line=two)
    with pytest.raises(LocalInputError, match=r"RELEASING\.md Owner: line"):
        read_local_inputs(paths, TODAY)


def test_a_well_formed_owner_line_parses():
    assert Owner.from_releasing("x\nOwner: Jane Doe (@janedoe)\n") == Owner(
        "Jane Doe", "janedoe"
    )


def test_a_missing_releasing_md_names_the_owner_line(tmp_path):
    paths = clear_repository(tmp_path, TODAY)
    paths.releasing.unlink()
    with pytest.raises(LocalInputError, match=r"RELEASING\.md Owner: line"):
        read_local_inputs(paths, TODAY)


def test_a_missing_keyring_names_it(tmp_path):
    paths = clear_repository(tmp_path, TODAY)
    paths.node_keyring.unlink()
    with pytest.raises(LocalInputError, match=r"keys/nodejs-release\.asc"):
        read_local_inputs(paths, TODAY)


def test_an_empty_keyring_names_it(tmp_path):
    paths = clear_repository(tmp_path, TODAY)
    paths.node_keyring.write_text("")
    with pytest.raises(LocalInputError, match=r"keys/nodejs-release\.asc"):
        read_local_inputs(paths, TODAY)
