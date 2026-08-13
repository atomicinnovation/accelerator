"""Drift guards for the published docs anchor two shipped files point at.

`skills/visualisation/visualise/SKILL.md` and `hooks/launcher-link-refresh.sh`
both reach a plugin user at runtime, who has no checkout, so both name an
absolute URL into the published site rather than a repository path. That URL is
a copy of the hosting decision `docs-site/astro.config.mjs` owns, so the
expected value is composed from that file rather than hardcoded here — a domain
or base move then breaks the pointers loudly instead of leaving a stale literal
passing.

`starlightLinksValidator` cannot cover this: it runs with
`errorOnRelativeLinks: false` and does not resolve absolute off-site links.
"""

import re
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[3]
_ASTRO_CONFIG = _REPO_ROOT / "docs-site/astro.config.mjs"
_DOCS_INTERNALS = _REPO_ROOT / "docs-site/src/content/docs/internals.md"

_HEADING = "## Terminal Invocation"
_ANCHOR = "#terminal-invocation"

_POINTERS = (
    _REPO_ROOT / "skills/visualisation/visualise/SKILL.md",
    _REPO_ROOT / "hooks/launcher-link-refresh.sh",
)

_DEFAULT = r"const {name} = process\.env\.\w+ \?\? '([^']+)'"


def _hosting_default(name):
    match = re.search(_DEFAULT.format(name=name), _ASTRO_CONFIG.read_text())
    assert match, (
        f"docs-site/astro.config.mjs no longer declares a `{name}` default — "
        "the shipped pointers copy it, so they can no longer be checked"
    )
    return match.group(1)


def _published_anchor():
    site = _hosting_default("site")
    base = _hosting_default("base")
    return f"{site}{base}/internals/{_ANCHOR}"


def test_internals_keeps_the_heading_the_pointers_target():
    # A whole line, not a substring: `## Terminal Invocations` contains the
    # heading but slugs to a different anchor.
    headings = _DOCS_INTERNALS.read_text().splitlines()
    assert _HEADING in [line.rstrip() for line in headings], (
        f"{_DOCS_INTERNALS} no longer carries `{_HEADING}`, so the "
        f"`{_ANCHOR}` fragment the shipped pointers use resolves to nothing"
    )


def test_every_shipped_pointer_names_the_published_anchor():
    expected = _published_anchor()
    for pointer in _POINTERS:
        assert expected in pointer.read_text(), (
            f"{pointer.relative_to(_REPO_ROOT)} does not point at {expected}. "
            "It reaches a user with no checkout, so it must name the "
            "published URL composed from docs-site/astro.config.mjs"
        )


def test_no_shipped_pointer_names_a_repository_path():
    for pointer in _POINTERS:
        assert "docs/internals.md" not in pointer.read_text(), (
            f"{pointer.relative_to(_REPO_ROOT)} names a repository path a "
            "plugin user cannot open"
        )
