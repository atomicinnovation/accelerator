"""Unit tests for the shared Docker Playwright version-pin source of truth."""

import json
from pathlib import Path

import pytest

from tasks.shared.playwright import (
    PLAYWRIGHT_IMAGE_FLAVOUR,
    playwright_image,
    resolved_playwright_version,
)


def _write_lock(frontend: Path, packages: dict | None) -> Path:
    frontend.mkdir(parents=True, exist_ok=True)
    payload: dict = {} if packages is None else {"packages": packages}
    (frontend / "package-lock.json").write_text(json.dumps(payload))
    return frontend


def _entry(version: str | None) -> dict:
    entry: dict = {} if version is None else {"version": version}
    return {"node_modules/@playwright/test": entry}


class TestResolvedPlaywrightVersion:
    def test_reads_exact_version_from_lockfile(self, tmp_path):
        frontend = _write_lock(tmp_path / "fe", _entry("1.59.1"))
        assert resolved_playwright_version(frontend) == "1.59.1"

    def test_missing_lockfile_raises_clear_error(self, tmp_path):
        frontend = tmp_path / "fe"
        frontend.mkdir()
        with pytest.raises(ValueError, match="not readable"):
            resolved_playwright_version(frontend)

    def test_invalid_json_raises_clear_error(self, tmp_path):
        frontend = tmp_path / "fe"
        frontend.mkdir()
        (frontend / "package-lock.json").write_text("{not json")
        with pytest.raises(ValueError, match="not valid JSON"):
            resolved_playwright_version(frontend)

    def test_missing_packages_key_raises_clear_error(self, tmp_path):
        frontend = _write_lock(tmp_path / "fe", None)
        with pytest.raises(ValueError, match="no 'packages' object"):
            resolved_playwright_version(frontend)

    def test_missing_playwright_entry_raises_clear_error(self, tmp_path):
        frontend = _write_lock(tmp_path / "fe", {"node_modules/other": {}})
        with pytest.raises(ValueError, match="no 'node_modules/@playwright"):
            resolved_playwright_version(frontend)

    def test_entry_without_version_raises_clear_error(self, tmp_path):
        frontend = _write_lock(tmp_path / "fe", _entry(None))
        with pytest.raises(ValueError, match="no 'version' string"):
            resolved_playwright_version(frontend)

    def test_malformed_version_is_rejected(self, tmp_path):
        frontend = _write_lock(tmp_path / "fe", _entry("1.x"))
        with pytest.raises(ValueError, match="not a valid image-tag"):
            resolved_playwright_version(frontend)


class TestPlaywrightImage:
    def test_builds_pinned_noble_tag(self, tmp_path):
        frontend = _write_lock(tmp_path / "fe", _entry("1.59.1"))
        expected = (
            f"mcr.microsoft.com/playwright:v1.59.1-{PLAYWRIGHT_IMAGE_FLAVOUR}"
        )
        assert playwright_image(frontend) == expected

    def test_prerelease_version_is_tag_encoded_not_silently_malformed(
        self, tmp_path
    ):
        frontend = _write_lock(tmp_path / "fe", _entry("1.60.0-beta.2"))
        assert (
            playwright_image(frontend)
            == "mcr.microsoft.com/playwright:v1.60.0-beta.2-noble"
        )

    def test_real_lockfile_resolves_pinned_version(self):
        # Guards the production path: the committed lockfile must resolve to a
        # real, single version (catches a lockfile-format drift).
        version = resolved_playwright_version()
        assert version.count(".") >= 2
