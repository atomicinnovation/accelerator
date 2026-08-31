import json
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

import tasks.notices as tn
from tasks.notices import (
    _RULE,
    _fold,
    _render_frontend,
    _render_rust,
    _run_cargo_about,
    _run_license_checker,
    check,
)
from tasks.shared.paths import ATTRIBUTION_ARTEFACT


def _about_json(*crates):
    """A cargo-about JSON payload from (name, version, id, text, repo)."""
    licences: dict[str, dict] = {}
    for name, version, licence_id, text, repository in crates:
        entry = licences.setdefault(
            licence_id, {"id": licence_id, "text": text, "used_by": []}
        )
        entry["used_by"].append(
            {
                "crate": {
                    "name": name,
                    "version": version,
                    "repository": repository,
                }
            }
        )
    return json.dumps({"licenses": list(licences.values())})


def _frontend_json(*packages):
    payload = {}
    for pkg in packages:
        key = f"{pkg['name']}@{pkg['version']}"
        payload[key] = pkg
    return json.dumps(payload)


class TestRenderFrontend:
    def test_sorts_by_name_at_version_and_preserves_multiline_text(self):
        raw = _frontend_json(
            {
                "name": "zeta",
                "version": "2.0.0",
                "licenses": "MIT",
                "repository": "https://example.test/zeta",
                "copyright": "Copyright (c) 2020 Zeta",
                "licenseText": 'Line one\nLine "two" with quotes\nLine three',
            },
            {
                "name": "alpha",
                "version": "1.0.0",
                "licenses": "ISC",
                "repository": "https://example.test/alpha",
                "copyright": "Copyright (c) 2019 Alpha",
                "licenseText": "Alpha licence body",
            },
        )
        out = _render_frontend(raw)
        assert out.index("alpha 1.0.0") < out.index("zeta 2.0.0")
        assert 'Line "two" with quotes' in out
        assert "License: ISC" in out
        assert "Copyright: Copyright (c) 2019 Alpha" in out

    def test_missing_copyright_omits_the_line_not_the_block(self):
        raw = _frontend_json(
            {
                "name": "no-copyright",
                "version": "1.0.0",
                "licenses": "MIT",
                "repository": "https://example.test/x",
                "licenseText": "body",
            }
        )
        out = _render_frontend(raw)
        assert "no-copyright 1.0.0" in out
        assert "Copyright:" not in out

    def test_missing_licence_text_renders_a_placeholder(self):
        raw = _frontend_json(
            {
                "name": "no-text",
                "version": "1.0.0",
                "licenses": "MIT",
                "repository": "https://example.test/x",
            }
        )
        out = _render_frontend(raw)
        assert "no-text 1.0.0" in out
        assert "no licence text provided" in out

    def test_empty_payload_renders_nothing(self):
        assert _render_frontend("{}") == ""

    def test_same_name_two_versions_are_both_present_and_ordered(self):
        raw = _frontend_json(
            {
                "name": "dup",
                "version": "3.0.0",
                "licenses": "MIT",
                "repository": "r",
                "licenseText": "v3",
            },
            {
                "name": "dup",
                "version": "2.0.0",
                "licenses": "MIT",
                "repository": "r",
                "licenseText": "v2",
            },
        )
        out = _render_frontend(raw)
        assert out.index("dup 2.0.0") < out.index("dup 3.0.0")

    def test_licences_list_is_joined(self):
        raw = _frontend_json(
            {
                "name": "multi",
                "version": "1.0.0",
                "licenses": ["MIT", "ISC"],
                "repository": "r",
                "licenseText": "body",
            }
        )
        out = _render_frontend(raw)
        assert "License: MIT, ISC" in out

    def test_mpl_frontend_package_emits_the_corresponding_source(self):
        raw = _frontend_json(
            {
                "name": "copyleft",
                "version": "1.2.3",
                "licenses": "MPL-2.0",
                "repository": "https://example.test/copyleft",
                "licenseText": "MPL body",
            }
        )
        out = _render_frontend(raw)
        assert "MPL-2.0 corresponding source (§3.2):" in out
        assert "https://example.test/copyleft" in out
        assert "https://registry.npmjs.org/copyleft/-/copyleft-1.2.3.tgz" in out


class TestRenderRust:
    def test_inverts_licences_into_sorted_per_crate_blocks(self):
        raw = _about_json(
            ("zebra", "1.0.0", "MIT", "MIT body", "https://ex.test/zebra"),
            ("apple", "2.0.0", "ISC", "ISC body", "https://ex.test/apple"),
        )
        out = _render_rust(raw)
        assert out.index("apple 2.0.0") < out.index("zebra 1.0.0")
        assert "License: MIT" in out
        assert "Source: https://ex.test/apple" in out

    def test_block_shape_matches_the_frontend_renderer(self):
        rust = _render_rust(
            _about_json(
                ("crate-x", "1.0.0", "MIT", "body", "https://ex.test/x")
            )
        )
        frontend = _render_frontend(
            _frontend_json(
                {
                    "name": "pkg-x",
                    "version": "1.0.0",
                    "licenses": "MIT",
                    "repository": "https://ex.test/x",
                    "licenseText": "body",
                }
            )
        )
        for section in (rust, frontend):
            assert _RULE in section
            assert "License: MIT" in section
            assert "Source: https://ex.test/x" in section

    def test_mpl_crate_carries_the_section_three_two_source(self):
        out = _render_rust(
            _about_json(
                ("uluru", "3.1.0", "MPL-2.0", "MPL body", "https://ex.test/u")
            )
        )
        assert "MPL-2.0 corresponding source (§3.2):" in out
        assert "https://ex.test/u" in out
        assert "https://crates.io/api/v1/crates/uluru/3.1.0/download" in out

    def test_non_mpl_crate_has_no_corresponding_source(self):
        out = _render_rust(
            _about_json(
                ("plain", "1.0.0", "MIT", "MIT body", "https://ex.test/p")
            )
        )
        assert "corresponding source" not in out

    def test_missing_repository_falls_back_to_crates_io(self):
        out = _render_rust(_about_json(("norepo", "1.0.0", "MIT", "b", None)))
        assert "Source: https://crates.io/crates/norepo" in out


class TestFold:
    def test_header_prefixes_and_both_sections_present(self):
        out = _fold("RUST-BODY", "FRONTEND-BODY")
        assert out.startswith("Accelerator Third-Party Notices")
        assert out.index("Rust components") < out.index("RUST-BODY")
        assert out.index("Frontend components") < out.index("FRONTEND-BODY")
        assert out.index("RUST-BODY") < out.index("FRONTEND-BODY")

    def test_normalises_crlf_and_trailing_newlines_to_lf(self):
        out = _fold("has\r\ncrlf\r\n\r\n", "trailing\n\n\n")
        assert "\r" not in out
        assert out.endswith("\n")
        assert not out.endswith("\n\n")


class TestGeneratorFailures:
    def _ctx(self, exited):
        ctx = MagicMock(spec=Context)
        ctx.run.return_value = MagicMock(exited=exited, stdout="{}")
        return ctx

    def test_cargo_about_failure_names_the_generator(self):
        with pytest.raises(Exit, match="cargo-about"):
            _run_cargo_about(self._ctx(1))

    def test_license_checker_failure_names_the_generator(self):
        with pytest.raises(Exit, match="license-checker"):
            _run_license_checker(self._ctx(1))

    def test_license_checker_without_json_object_raises(self):
        ctx = MagicMock(spec=Context)
        ctx.run.return_value = MagicMock(exited=0, stdout="no json here")
        with pytest.raises(Exit, match="no JSON object"):
            _run_license_checker(ctx)


class TestCheck:
    def _patch_artefact(self, monkeypatch, tmp_path, content=None):
        path = tmp_path / "notices.txt"
        if content is not None:
            path.write_text(content, encoding="utf-8")
        monkeypatch.setattr(tn, "ATTRIBUTION_ARTEFACT", path)
        return path

    def test_passes_when_the_render_matches(self, monkeypatch, tmp_path):
        self._patch_artefact(monkeypatch, tmp_path, content="RENDER")
        monkeypatch.setattr(tn, "_render", lambda _ctx: "RENDER")
        check(MagicMock(spec=Context))  # must not raise

    def test_drift_raises_naming_update(self, monkeypatch, tmp_path):
        self._patch_artefact(monkeypatch, tmp_path, content="OLD")
        monkeypatch.setattr(tn, "_render", lambda _ctx: "NEW")
        with pytest.raises(Exit, match="notices:update"):
            check(MagicMock(spec=Context))

    def test_missing_file_raises_naming_update(self, monkeypatch, tmp_path):
        self._patch_artefact(monkeypatch, tmp_path, content=None)
        monkeypatch.setattr(tn, "_render", lambda _ctx: "ANYTHING")
        with pytest.raises(Exit, match="notices:update"):
            check(MagicMock(spec=Context))


class TestCommittedArtefact:
    def _blocks(self):
        return ATTRIBUTION_ARTEFACT.read_text(encoding="utf-8")

    def test_every_mpl_block_carries_a_corresponding_source(self):
        text = self._blocks()
        mpl_licence_lines = sum(
            1
            for line in text.splitlines()
            if line.startswith("License: ") and "MPL-2.0" in line
        )
        section_three_two = text.count("MPL-2.0 corresponding source (§3.2):")
        assert mpl_licence_lines >= 1
        assert section_three_two == mpl_licence_lines

    def test_uluru_resolves_to_an_obtainable_crates_io_source(self):
        text = self._blocks()
        assert "uluru" in text
        assert (
            "https://crates.io/api/v1/crates/uluru/" in text
            and "/download" in text
        )
