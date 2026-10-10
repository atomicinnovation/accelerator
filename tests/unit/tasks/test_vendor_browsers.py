import json
from pathlib import Path

import pytest

from tasks.shared.vendor.browsers import (
    HEADLESS_SHELL,
    BrowserBuild,
    BrowsersManifest,
    BrowsersManifestError,
    UnpinnedRevisionError,
)

RECORDED = (
    Path(__file__).resolve().parent
    / "fixtures"
    / "pin-guard"
    / "browsers-1.55.1.json"
)


def test_the_recorded_manifest_parses_every_build():
    manifest = BrowsersManifest.read(RECORDED)
    names = [build.name for build in manifest.builds]
    assert HEADLESS_SHELL in names
    assert "ffmpeg" in names


def test_a_build_without_a_browser_version_keeps_none():
    manifest = BrowsersManifest.read(RECORDED)
    ffmpeg = next(b for b in manifest.builds if b.name == "ffmpeg")
    assert ffmpeg == BrowserBuild("ffmpeg", "1011", None)


def test_the_pinned_headless_shell_carries_its_browser_version():
    build = BrowsersManifest.read(RECORDED).pinned_headless_shell("1193")
    assert build == BrowserBuild(HEADLESS_SHELL, "1193", "140.0.7339.186")


def test_an_unpinned_revision_is_refused_by_name():
    manifest = BrowsersManifest.read(RECORDED)
    with pytest.raises(UnpinnedRevisionError, match="1194"):
        manifest.pinned_headless_shell("1194")


def test_an_unpinned_revision_is_not_mistaken_for_a_malformed_manifest():
    assert not issubclass(UnpinnedRevisionError, ValueError)


def test_a_chromium_build_sharing_the_revision_is_not_the_headless_shell():
    manifest = BrowsersManifest.parse(
        {"browsers": [{"name": "chromium", "revision": "1193"}]}
    )
    with pytest.raises(UnpinnedRevisionError):
        manifest.pinned_headless_shell("1193")


@pytest.mark.parametrize(
    "document",
    [
        {},
        {"browsers": {"name": HEADLESS_SHELL}},
        {"browsers": []},
        [],
    ],
    ids=["missing", "not-a-list", "empty", "not-an-object"],
)
def test_a_manifest_without_browsers_is_malformed(document):
    with pytest.raises(BrowsersManifestError, match="browsers"):
        BrowsersManifest.parse(document)


@pytest.mark.parametrize(
    "entry",
    [
        {"name": HEADLESS_SHELL},
        {"name": HEADLESS_SHELL, "revision": 1193},
        {"revision": "1193"},
        "chromium-headless-shell",
    ],
    ids=["no-revision", "numeric-revision", "no-name", "not-an-object"],
)
def test_an_entry_without_a_string_name_and_revision_is_malformed(entry):
    with pytest.raises(BrowsersManifestError):
        BrowsersManifest.parse({"browsers": [entry]})


def test_reading_an_unparseable_file_is_malformed(tmp_path):
    path = tmp_path / "browsers.json"
    path.write_text("{not json")
    with pytest.raises(BrowsersManifestError, match=str(path)):
        BrowsersManifest.read(path)


def test_reading_round_trips_a_written_manifest(tmp_path):
    path = tmp_path / "browsers.json"
    path.write_text(
        json.dumps({"browsers": [{"name": HEADLESS_SHELL, "revision": "1181"}]})
    )
    assert BrowsersManifest.read(path).pinned_headless_shell("1181") == (
        BrowserBuild(HEADLESS_SHELL, "1181", None)
    )
