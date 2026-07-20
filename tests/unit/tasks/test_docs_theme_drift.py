"""Drift guards for docs-site theme values duplicated from the visualiser.

The docs site is a separate npm build from the visualiser frontend, so it
duplicates the brand palette (``--atomic-*``). These tests pin every
duplicated value to its canonical source so the copies cannot diverge
silently, and check theme.css only references fonts that exist in the
visualiser's canonical fonts directory (docs-site/src/fonts is a
symlink to it).
"""

import json
import re
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[3]
_THEME_CSS = _REPO_ROOT / "docs-site/src/styles/theme.css"
_FRONTEND = _REPO_ROOT / "skills/visualisation/visualise/frontend"
_FIXTURE = _FRONTEND / "src/styles/fixtures/prototype-tokens.json"
_FRONTEND_FONTS = _FRONTEND / "public/fonts"

_SHIKI_THEME = _REPO_ROOT / "docs-site/shiki-atomic.mjs"

_DECLARATION = re.compile(r"(--[\w-]+)\s*:\s*([^;{}]+);")
_SHIKI_COLOUR = re.compile(r"(\w+)\s*:\s*'(#[0-9a-fA-F]{6})'")

_SHIKI_TOKEN_SOURCES = {
    "bg": "--code-bg",
    "fg": "--code-fg",
    "comment": "--tk-com",
    "string": "--tk-str",
    "number": "--tk-num",
    "keyword": "--tk-kw",
    "literal": "--tk-lit",
    "type": "--tk-typ",
    "function": "--tk-fn",
    "attribute": "--tk-attr",
    "variable": "--tk-var",
    "punctuation": "--tk-pun",
    "tag": "--tk-tag",
    "diffInserted": "--tk-dadd",
    "diffDeleted": "--tk-ddel",
}


def _normalise(value: str) -> str:
    return re.sub(r"\s+", "", value).lower()


def _theme_declarations() -> dict[str, str]:
    assert _THEME_CSS.is_file(), (
        f"docs theme sheet missing: {_THEME_CSS} — the docs site brand "
        "theme must exist and declare its --atomic-* tokens"
    )
    return dict(_DECLARATION.findall(_THEME_CSS.read_text()))


def test_atomic_tokens_match_canonical_fixture():
    fixture = json.loads(_FIXTURE.read_text())
    declared = {
        name: value
        for name, value in _theme_declarations().items()
        if name.startswith("--atomic-")
    }
    assert declared, (
        f"no --atomic-* tokens declared in {_THEME_CSS} — the docs theme "
        "must consume the brand palette via --atomic-* declarations"
    )
    for name, value in declared.items():
        assert name in fixture, (
            f"docs theme token {name} does not exist in the canonical "
            f"brand palette fixture {_FIXTURE} — remove it from "
            f"{_THEME_CSS} or use a canonical token name"
        )
        assert _normalise(value) == _normalise(fixture[name]), (
            f"docs theme token {name} diverged from the canonical brand "
            f"palette: {_THEME_CSS} has {value.strip()!r} but "
            f"{_FIXTURE} has {fixture[name]!r} — update theme.css to "
            "match the canonical value"
        )


def test_shiki_theme_colours_match_canonical_fixture():
    assert _SHIKI_THEME.is_file(), (
        f"docs Shiki theme missing: {_SHIKI_THEME} — the docs site code "
        "blocks must use the atomic-code Shiki theme"
    )
    fixture = json.loads(_FIXTURE.read_text())
    declared = dict(_SHIKI_COLOUR.findall(_SHIKI_THEME.read_text()))
    assert declared, (
        f"no colour entries parsed from {_SHIKI_THEME} — the theme must "
        "export a named map of key: '#hex' pairs for drift guarding"
    )
    for key in declared:
        assert key in _SHIKI_TOKEN_SOURCES, (
            f"Shiki theme colour {key!r} in {_SHIKI_THEME} has no "
            "canonical --code-*/--tk-* source registered in "
            "_SHIKI_TOKEN_SOURCES — map it to its fixture token or "
            "remove it"
        )
    for key, token in _SHIKI_TOKEN_SOURCES.items():
        assert key in declared, (
            f"Shiki theme colour {key!r} missing from {_SHIKI_THEME} — "
            f"it must carry the canonical {token} value from {_FIXTURE}"
        )
        assert _normalise(declared[key]) == _normalise(fixture[token]), (
            f"Shiki theme colour {key!r} diverged from the canonical "
            f"palette: {_SHIKI_THEME} has {declared[key]!r} but {token} "
            f"in {_FIXTURE} is {fixture[token]!r} — update the theme to "
            "match the canonical value"
        )


def test_theme_css_fonts_exist_in_canonical_frontend_directory():
    # docs-site/src/fonts is a symlink to the frontend fonts directory
    # (Vite resolves the relative url()s through it and rewrites them
    # from the configured base), so identity is guaranteed by
    # construction; here we pin every URL theme.css references to an
    # existing canonical source file. Pin the symlink target too, so a
    # relocated fonts directory fails here rather than as a stale docs
    # cache or a broken Pages deploy.
    docs_fonts = _REPO_ROOT / "docs-site/src/fonts"
    assert docs_fonts.is_symlink(), (
        f"{docs_fonts} must be a symlink into the visualiser frontend "
        "fonts directory"
    )
    assert docs_fonts.resolve() == _FRONTEND_FONTS.resolve(), (
        f"{docs_fonts} resolves to {docs_fonts.resolve()}, not the "
        f"canonical fonts directory {_FRONTEND_FONTS} — update the "
        "symlink and the docs:build source glob in mise.toml together"
    )
    css = _THEME_CSS.read_text()
    referenced = re.findall(r"url\('\.\./fonts/([^']+)'\)", css)
    assert referenced, f"no font URLs found in {_THEME_CSS}"
    for name in referenced:
        assert (_FRONTEND_FONTS / name).is_file(), (
            f"theme.css references {name} but it does not exist in the "
            f"canonical fonts directory {_FRONTEND_FONTS} — the docs "
            "build serves fonts from there via the docs-site/src/fonts "
            "symlink"
        )
