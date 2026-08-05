"""Guard the `## Registering a dispatched sub-binary` checklist.

Its own file rather than the dispatch guard's, following the test_mise.py /
test_workflows.py precedent for guards over non-Python artefacts: the checklist
spans Cargo manifests, `.gitignore`, workflow YAML and user docs, of which only
points 1, 7, 10 and 12 concern dispatch.

The pairing of literal-string and source-existence assertions is what makes the
guard bidirectional. A test that read only the README would freeze doc text:
renaming a registration point in code would leave the checklist stale and the
test green, while an author *correcting* the README to the new name would make
it fail.
"""

import re

import pytest

from tasks.shared.sources import repo_root

REPO_ROOT = repo_root()
README = REPO_ROOT / "tasks/README.md"
HEADING = "## Registering a dispatched sub-binary"
ANCHOR = "tasks/README.md#registering-a-dispatched-sub-binary"

# Closed set. The predicate is not decidable in general, so section and test are
# drafted against each other: widen this only alongside the section.
_VERBS = ("Add", "Register", "Update", "Document", "Extend")
_TAGS = ("**[PR]**", "**[release]**", "**[author]**")

_NAMED = (
    "DISPATCHED_SUBBINARIES",
    "SKILL_EXEMPT_SUBBINARIES",
    "DEBUG_ARCHIVE_DIRS",
    "BUILTIN_SUBCOMMANDS",
    "_SUBBINARY_MANIFESTS",
    "_CLI_RELEASE_BINARIES",
    "EXPECTED_INJECTION_SKILLS",
    "package.description",
    "version.workspace",
    "cli/Cargo.toml",
    "bin/<token>-*",
    "manifest.example.json",
    "test_manifest_contract.py",
    "resolve/manifest.rs",
    "_assert_static_elf",
    "dist/release/accelerator-*",
    "cli/deny.toml",
    "cli/Cargo.lock",
    "accelerator-<token>",
    "ACCELERATOR_<TOKEN>_BIN",
    "same change",
)

# Every source the section attributes something to, and the thing it attributes.
_RESOLVES = (
    ("tasks/shared/paths.py", "DISPATCHED_SUBBINARIES"),
    ("tasks/shared/paths.py", "SKILL_EXEMPT_SUBBINARIES"),
    ("tasks/shared/paths.py", "DEBUG_ARCHIVE_DIRS"),
    ("tasks/shared/paths.py", "subbinary_asset_path"),
    ("tasks/manifest.py", "_SUBBINARY_MANIFESTS"),
    ("tasks/build.py", "_assert_static_elf"),
    ("tasks/build.py", "_assert_magic_bytes"),
    ("tasks/build.py", "_CLI_RELEASE_BINARIES"),
    ("tasks/release.py", "prerelease_prepare"),
    ("tasks/release.py", "release_prepare"),
    ("tasks/shared/dispatch_coherence.py", "BUILTIN_SUBCOMMANDS"),
    ("tasks/lint/skill_permissions.py", "EXPECTED_INJECTION_SKILLS"),
    # The source point 10 attributes authority to — the clap enum, not the
    # help-routing heuristic.
    ("cli/launcher/src/launch/inbound/cli.rs", "Command"),
    ("cli/launcher/src/launch/inbound/cli.rs", "External"),
    ("README.md", "## Documentation"),
    ("cli/visualiser/server/Cargo.toml", "[lints.clippy]"),
    ("cli/launcher/src/launch/core.rs", "ACCELERATOR_"),
    (".gitignore", "**/bin/*.debug.tar.gz"),
    ("cli/Cargo.toml", "members"),
    ("mise.toml", "build:cli:cross-compile"),
    (".github/workflows/main.yml", "dist/release/accelerator-*"),
    (
        "docs-site/src/content/docs/visualiser.md",
        "ACCELERATOR_VISUALISER_BIN",
    ),
    ("tasks/CLAUDE.md", ANCHOR),
)

_EXISTS = (
    "tests/unit/tasks/test_manifest_contract.py",
    "cli/launcher/src/launch/outbound/resolve/manifest.rs",
    "cli/deny.toml",
)


def _section() -> str:
    text = README.read_text()
    start = text.index(HEADING)
    end = text.index("\n## ", start + len(HEADING))
    return text[start:end]


def _items() -> list[str]:
    # Split on top-level `N. ` lines; sub-bullets are indented and so excluded.
    return re.split(r"^\d+\. ", _section(), flags=re.MULTILINE)[1:]


def test_the_section_exists() -> None:
    assert HEADING in README.read_text()


def test_the_checklist_has_thirteen_points() -> None:
    assert len(_items()) == 13


@pytest.mark.parametrize("index", range(13))
def test_each_item_opens_with_an_action_or_declines_one(index: int) -> None:
    first_line = _items()[index].splitlines()[0]
    # Every item is written `1. **Add** …`, so the emphasis strip is part of
    # the contract — a bare startswith would match none of the thirteen.
    stripped = first_line.lstrip("*")
    assert stripped.startswith(_VERBS) or "No action when" in _items()[index]


@pytest.mark.parametrize("index", range(13))
def test_each_item_carries_exactly_one_surfacing_tag(index: int) -> None:
    item = _items()[index]
    assert sum(item.count(tag) for tag in _TAGS) == 1


@pytest.mark.parametrize("named", _NAMED)
def test_the_section_names_each_registration_point(named: str) -> None:
    assert named in _section()


@pytest.mark.parametrize(("path", "named"), _RESOLVES)
def test_each_named_thing_still_resolves_in_its_source(
    path: str, named: str
) -> None:
    assert named in (REPO_ROOT / path).read_text(), (
        f"{named} is attributed to {path} but no longer appears there"
    )


@pytest.mark.parametrize("path", _EXISTS)
def test_each_referenced_path_still_exists(path: str) -> None:
    assert (REPO_ROOT / path).exists()
