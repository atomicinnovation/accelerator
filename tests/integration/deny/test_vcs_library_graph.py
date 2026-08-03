"""Regression: the library-backed VCS adapter's resolved graph stays pinned.

deny.toml bans by crate name; this asserts what names cannot express. gix and
jj-lib take *ranges* (~0.85.0 / =0.43.0) and gix republishes its feature set
with every patch, so the effective surface is upstream-controlled within the
permitted range — the same reasoning that made test_launcher_feature_graph.py
exist alongside deny.toml's bans.

Four invariants, each guarding a distinct failure:

* Versions and single-graph. gix at 0.85.x specifically, not merely at one
  version: gix is optional in jj-lib, so a bare single-version assertion would
  hold vacuously if that feature were ever off. Duplicate detection reads
  Cargo.lock directly because the repo's multiple-versions policy is warn-level
  and would not fail on its own.
* Features. The six the adapter's calls need must be on; the network client and
  credentials families must be off. Feature absence — not crate absence — is
  what keeps gix-transport and gix-protocol inert: both ARE in the graph via
  jj-lib's defaults, so banning them by name would fail deny:check outright.
* MSRV. Asserted directly rather than trusting resolver 3's
  incompatible-rust-versions = "fallback", which is a *preference*, not a hard
  constraint — without the gix/jj-lib trees the graph selects kstring 2.0.4,
  which needs Rust 1.96 and will not build on the pinned toolchain. The value
  comes from cargo metadata: Cargo.lock does not record rust-version at all, so
  a lock-parsing implementation would pass vacuously on exactly the mechanism
  this backstops.
* Build-script and proc-macro snapshot. These trees execute ~48 crates' build
  scripts and proc macros on every developer machine and on the release runner,
  which already documents build-script trust as a live concern. A lock
  regeneration could otherwise grow that set silently.
"""

import json
import re
import shutil
import subprocess
import tomllib
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_CLI = _REPO_ROOT / "cli"
_CLI_CARGO = _CLI / "Cargo.toml"
_CLI_LOCK = _CLI / "Cargo.lock"

_CARGO = shutil.which("cargo")

_PACKAGE = "vcs-adapters"

# The gix release line jj-lib 0.43 requires, and the exact jj-lib pin.
_GIX_VERSION = re.compile(r"^0\.85\.\d+$")
_JJ_LIB_VERSION = re.compile(r"^0\.43\.\d+$")

# What the adapter's calls need. attributes/blob-diff/index/sha1/zlib-rs and
# max-performance-safe arrive from jj-lib's own selection, not from gix's
# defaults, which the pin turns off.
_FEATURES_PRESENT = (
    "attributes",
    "blob-diff",
    "index",
    "max-performance-safe",
    "sha1",
    "zlib-rs",
)
# credentials is load-bearing here: it pulls gix-credentials, which spawns
# `git credential-*` helper programs, against a module whose whole point is
# reading git without a subprocess.
_FEATURES_ABSENT = (
    "blocking-network-client",
    "async-network-client",
    "credentials",
)
_FEATURE_PREFIXES_ABSENT = ("blocking-http-transport",)

# Subtree-scoped, which is narrower than deny.toml's whole-graph bans and cannot
# be expressed there: rustls is a first-party dependency the launcher consumes.
_TLS_CRATES = ("rustls", "openssl", "openssl-sys", "native-tls", "curl-sys")

# deny.toml's [graph].targets, which [bans] is evaluated against.
_TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
    "x86_64-unknown-linux-gnu",
)

_BUILD_SCRIPT_CRATES = frozenset(
    {
        "anyhow",
        "crc32fast",
        "crossbeam-deque",
        "crossbeam-epoch",
        "crossbeam-utils",
        "defmt",
        "defmt-macros",
        "generic-array",
        "getrandom",
        "heapless",
        "iana-time-zone-haiku",
        "libc",
        "logos-codegen",
        "num-traits",
        "parking_lot_core",
        "portable-atomic",
        "portable-atomic-util",
        "proc-macro2",
        "quote",
        "rayon-core",
        "ref-cast",
        "rustix",
        "rustversion",
        "serde",
        "serde_core",
        "thiserror",
        "valuable",
        "wasm-bindgen",
        "wasm-bindgen-shared",
        "zerocopy",
    }
)

_PROC_MACRO_CRATES = frozenset(
    {
        "async-trait",
        "defmt-macros",
        "futures-macro",
        "jiff-static",
        "jj-lib-proc-macros",
        "logos-derive",
        "maybe-async",
        "pest_derive",
        "prost-derive",
        "ref-cast-impl",
        "rustversion",
        "serde_derive",
        "thiserror-impl",
        "tracing-attributes",
        "wasm-bindgen-macro",
        "windows-implement",
        "windows-interface",
        "zerocopy-derive",
    }
)


def _require_cargo() -> None:
    if _CARGO is None:
        pytest.skip("cargo not on PATH")


def _feature_tree(target: str | None = None) -> str:
    _require_cargo()
    command = ["cargo", "tree", "-e", "features", "-p", _PACKAGE]
    if target is not None:
        command += ["--target", target]
    result = subprocess.run(
        command, cwd=_CLI, capture_output=True, text=True, check=False
    )
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout


def _metadata() -> dict:
    _require_cargo()
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=_CLI,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def _lock_packages() -> dict[str, set[str]]:
    packages: dict[str, set[str]] = {}
    pattern = re.compile(
        r'\[\[package\]\]\nname = "([^"]+)"\nversion = "([^"]+)"'
    )
    for name, version in pattern.findall(_CLI_LOCK.read_text()):
        packages.setdefault(name, set()).add(version)
    return packages


def _node_present(tree: str, crate: str) -> bool:
    # A node renders as "<crate> v<version>"; the lookbehind stops `openssl`
    # matching `openssl-probe` and `gix` matching `gix-odb`.
    return re.search(rf"(?<![\w-]){re.escape(crate)} v\d", tree) is not None


def _gix_features(tree: str) -> set[str]:
    return set(re.findall(r'(?<![\w-])gix feature "([^"]+)"', tree))


def _subtree_packages() -> dict[str, dict]:
    """Every package reachable from vcs-adapters by normal and build edges.

    Dev edges are excluded deliberately: they do not reach a shipped binary, and
    including them would make the snapshot churn on test-only dependencies.
    """
    metadata = _metadata()
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    packages = {package["id"]: package for package in metadata["packages"]}

    roots = [
        identifier
        for identifier, package in packages.items()
        if package["name"] == _PACKAGE
    ]
    assert roots, f"{_PACKAGE} is not in the workspace metadata"

    seen: set[str] = set()
    stack = list(roots)
    while stack:
        identifier = stack.pop()
        if identifier in seen:
            continue
        seen.add(identifier)
        for dependency in nodes[identifier]["deps"]:
            kinds = {kind["kind"] for kind in dependency["dep_kinds"]}
            if kinds & {None, "build"}:
                stack.append(dependency["pkg"])
    return {identifier: packages[identifier] for identifier in seen}


def _version_key(version: str) -> tuple[int, ...]:
    return tuple(
        int(part) for part in version.split("-", maxsplit=1)[0].split(".")[:3]
    )


def _workspace_msrv() -> str:
    data = tomllib.loads(_CLI_CARGO.read_text())
    return data["workspace"]["package"]["rust-version"]


# --- Versions and the single graph ---


def test_gix_resolves_to_the_pinned_release_line() -> None:
    versions = _lock_packages().get("gix", set())
    assert len(versions) == 1, f"expected exactly one gix, found {versions}"
    (version,) = versions
    assert _GIX_VERSION.match(version), f"gix resolved to {version}, not 0.85.x"


def test_jj_lib_resolves_to_the_pinned_version() -> None:
    versions = _lock_packages().get("jj-lib", set())
    assert len(versions) == 1, f"expected exactly one jj-lib, found {versions}"
    (version,) = versions
    assert _JJ_LIB_VERSION.match(version), f"jj-lib resolved to {version}"


def test_no_gix_package_is_present_at_more_than_one_version() -> None:
    duplicated = {
        name: versions
        for name, versions in _lock_packages().items()
        if (name == "gix" or name.startswith("gix-")) and len(versions) > 1
    }
    assert not duplicated, f"duplicate gix-family versions: {duplicated}"


@pytest.mark.parametrize("crate", ["prost", "pollster"])
def test_the_jj_helper_crates_resolve_to_one_version(crate: str) -> None:
    # vcs-adapters depends on both directly, to decode jj's checkout state and
    # to drive the OpStore trait's async reads. The decoded type comes FROM
    # jj-lib, so a second prost graph would mean the generated code and the
    # decoder in use were different crates — the failure the pin comment in
    # cli/Cargo.toml names. Both must stay single-version.
    versions = _lock_packages().get(crate, set())
    assert len(versions) == 1, f"{crate} resolved to {sorted(versions)}"


# --- Features ---


@pytest.mark.parametrize("feature", _FEATURES_PRESENT)
def test_required_gix_feature_is_enabled(feature: str) -> None:
    enabled = _gix_features(_feature_tree())
    assert feature in enabled, (
        f"gix feature {feature!r} is off; enabled: {sorted(enabled)}"
    )


@pytest.mark.parametrize("feature", _FEATURES_ABSENT)
def test_prohibited_gix_feature_is_disabled(feature: str) -> None:
    enabled = _gix_features(_feature_tree())
    assert feature not in enabled, f"gix feature {feature!r} is unexpectedly on"


def test_no_http_transport_feature_is_enabled() -> None:
    offenders = sorted(
        feature
        for feature in _gix_features(_feature_tree())
        if feature.startswith(_FEATURE_PREFIXES_ABSENT)
    )
    assert not offenders, f"http transport features enabled: {offenders}"


def test_the_feature_assertion_is_not_vacuous() -> None:
    # A tree that parsed to nothing would pass every absence assertion above.
    assert len(_gix_features(_feature_tree())) >= len(_FEATURES_PRESENT)


# --- No TLS stack in the subtree, on every target deny.toml evaluates ---


@pytest.mark.parametrize("target", _TARGETS)
@pytest.mark.parametrize("crate", _TLS_CRATES)
def test_no_tls_stack_in_the_vcs_subtree(crate: str, target: str) -> None:
    tree = _feature_tree(target)
    assert not _node_present(tree, crate), (
        f"{crate} unexpectedly present in the {_PACKAGE} subtree for {target}"
    )


# --- MSRV ---


def test_no_package_requires_a_newer_rust_than_the_pinned_toolchain() -> None:
    pinned = _workspace_msrv()
    ceiling = _version_key(pinned)
    offenders = sorted(
        (package["name"], package["version"], package["rust_version"])
        for package in _metadata()["packages"]
        if package.get("rust_version")
        and _version_key(package["rust_version"]) > ceiling
    )
    assert not offenders, (
        f"packages requiring Rust newer than the pinned {pinned}: {offenders}"
    )


def test_the_msrv_assertion_is_not_vacuous() -> None:
    # A graph where nothing declared rust_version would pass the check above
    # while proving nothing — the exact failure a lock-parsing implementation
    # would have, since Cargo.lock does not carry the field at all.
    declaring = [
        package
        for package in _metadata()["packages"]
        if package.get("rust_version")
    ]
    assert len(declaring) > 100, (
        f"only {len(declaring)} packages declare rust_version — the MSRV "
        "check has lost its subject"
    )


# --- Build-script and proc-macro snapshot ---


def test_build_script_crates_in_the_subtree_are_the_snapshot() -> None:
    found = {
        package["name"]
        for package in _subtree_packages().values()
        if any(
            target["kind"] == ["custom-build"] for target in package["targets"]
        )
    }
    assert found == set(_BUILD_SCRIPT_CRATES), (
        "the build-script set changed — review the additions, then update the "
        f"snapshot. added={sorted(found - _BUILD_SCRIPT_CRATES)} "
        f"removed={sorted(_BUILD_SCRIPT_CRATES - found)}"
    )


def test_proc_macro_crates_in_the_subtree_are_the_snapshot() -> None:
    found = {
        package["name"]
        for package in _subtree_packages().values()
        if any("proc-macro" in target["kind"] for target in package["targets"])
    }
    assert found == set(_PROC_MACRO_CRATES), (
        "the proc-macro set changed — review the additions, then update the "
        f"snapshot. added={sorted(found - _PROC_MACRO_CRATES)} "
        f"removed={sorted(_PROC_MACRO_CRATES - found)}"
    )
