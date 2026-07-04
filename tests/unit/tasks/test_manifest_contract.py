"""The 0164/0165 manifest contract and cross-language alias coherence.

The signed `manifest.json` is the distribution contract both the launcher (Rust
reader) and the 0165 signer (Python writer) depend on. These tests pin the
shared golden fixture + JSON schema and assert the platform-alias tables agree
across `tasks/shared/targets.py`, the launcher's `HOST_PLATFORM` cfg block, and
the schema — so cross-language drift is a failing test here, not a
first-production-release failure.
"""

import json
import re
from pathlib import Path

from tasks.shared.targets import ALIASES, UNAME_TO_ALIAS

_REPO_ROOT = Path(__file__).resolve().parents[3]
_FIXTURES = _REPO_ROOT / "cli/launcher/tests/fixtures"
_GOLDEN = _FIXTURES / "manifest.example.json"
_SCHEMA = _FIXTURES / "manifest.schema.json"
_MANIFEST_RS = (
    _REPO_ROOT / "cli/launcher/src/launch/outbound/resolve/manifest.rs"
)
_RESOLVE_RS = _REPO_ROOT / "cli/launcher/src/launch/outbound/resolve/mod.rs"

_SHA256 = re.compile(r"^(sha256:)?[0-9a-f]{64}$")


def _golden() -> dict:
    return json.loads(_GOLDEN.read_text())


def test_golden_fixture_aliases_are_single_sourced() -> None:
    for entry in _golden()["binaries"].values():
        for alias in entry["platforms"]:
            assert alias in ALIASES, f"unknown platform alias {alias!r}"


def test_golden_fixture_digests_match_the_contract_pattern() -> None:
    for entry in _golden()["binaries"].values():
        for platform in entry["platforms"].values():
            assert _SHA256.match(platform["sha256"]), platform["sha256"]
            assert platform["signature"], "empty signature"


def test_schema_platform_enum_matches_the_alias_set() -> None:
    schema = json.loads(_SCHEMA.read_text())
    enum = schema["$defs"]["binaryEntry"]["properties"]["platforms"][
        "propertyNames"
    ]["enum"]
    assert set(enum) == set(ALIASES)


def test_golden_schema_version_matches_the_launcher() -> None:
    match = re.search(
        r"SUPPORTED_SCHEMA_VERSION:\s*u64\s*=\s*(\d+)",
        _MANIFEST_RS.read_text(),
    )
    assert match, "SUPPORTED_SCHEMA_VERSION not found in manifest.rs"
    assert _golden()["schema_version"] == int(match.group(1))


def test_uname_table_covers_exactly_the_alias_set() -> None:
    assert set(UNAME_TO_ALIAS.values()) == set(ALIASES)
    # Both machine spellings for each arch, both OSes.
    for os_name in ("darwin", "linux"):
        for machine in ("arm64", "aarch64", "x86_64", "amd64"):
            assert (os_name, machine) in UNAME_TO_ALIAS


def test_launcher_host_platform_literals_match_the_alias_set() -> None:
    # The launcher's HOST_PLATFORM is a compile-time cfg with one arm per alias;
    # grep the four string literals and assert they equal the oracle, so a hand
    # edit that drops or renames an arm fails here.
    literals = set(
        re.findall(
            r'HOST_PLATFORM:\s*&str\s*=\s*"([^"]+)"', _RESOLVE_RS.read_text()
        )
    )
    assert literals == set(ALIASES), literals
