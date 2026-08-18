"""The tree-artifact registry agrees across the language boundary.

TREE_ARTIFACTS, the launcher's compiled-in digest map, the pins.toml anchor and
the manifest.example.json artifact keys must name the same set — otherwise
retiring an artifact leaves the launcher exporting a variable nothing publishes,
or the design binary requesting a name the manifest no longer carries, both of
which surface at runtime on a user's machine since trees are exempt from the
per-exec re-verification that would catch a mismatch.
"""

import json
import re
import tomllib
from pathlib import Path

from tasks.shared.paths import TREE_ARTIFACTS
from tasks.shared.targets import ALIASES

_REPO_ROOT = Path(__file__).resolve().parents[3]
_PINS = _REPO_ROOT / "pins.toml"
_GOLDEN = _REPO_ROOT / "cli/launcher/tests/fixtures/manifest.example.json"
_PINS_RS = (
    _REPO_ROOT / "cli/launcher/src/launch/outbound/resolve/tree/pins.rs"
)


def _assembled() -> dict:
    with _PINS.open("rb") as handle:
        return tomllib.load(handle)["assembled_sha256"]


def test_pins_toml_names_exactly_the_tree_artifacts() -> None:
    assert set(_assembled()) == set(TREE_ARTIFACTS)


def test_every_artifact_pins_every_platform() -> None:
    for artifact, platforms in _assembled().items():
        assert set(platforms) == set(ALIASES), (
            f"{artifact} does not pin exactly the published platforms"
        )
        for platform, digest in platforms.items():
            assert re.fullmatch(r"[0-9a-f]{64}", digest), (
                f"{artifact}/{platform} is not a lowercase-hex digest"
            )


def test_manifest_fixture_artifacts_match_the_registry() -> None:
    manifest = json.loads(_GOLDEN.read_text())
    assert set(manifest["artifacts"]) == set(TREE_ARTIFACTS)
    for artifact, entry in manifest["artifacts"].items():
        assert set(entry["platforms"]) == set(ALIASES), artifact


def test_manifest_fixture_digests_are_the_pinned_ones() -> None:
    manifest = json.loads(_GOLDEN.read_text())
    assembled = _assembled()
    for artifact, entry in manifest["artifacts"].items():
        for platform, platform_entry in entry["platforms"].items():
            assert platform_entry["sha256"] == assembled[artifact][platform], (
                f"{artifact}/{platform} disagrees with pins.toml"
            )


def test_launcher_pins_module_reads_the_generated_map() -> None:
    # The compiled-in map is generated from pins.toml by build.rs, so the Rust
    # source must include that generated file rather than hard-coding digests
    # that could drift from the anchor.
    source = _PINS_RS.read_text()
    assert 'include!(concat!(env!("OUT_DIR"), "/tree_pins.rs"))' in source, (
        "pins.rs no longer embeds the build-time generated digest map"
    )
