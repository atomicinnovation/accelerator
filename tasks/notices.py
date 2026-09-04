import json
from dataclasses import dataclass, field
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import ATTRIBUTION_ARTEFACT, CLI_DIR, FRONTEND

_MPL = "MPL-2.0"
_RULE = "-" * 78


@dataclass
class _Component:
    name: str
    version: str
    repository: str | None
    licences: dict[str, str] = field(default_factory=dict)


_HEADER = """\
Accelerator Third-Party Notices
===============================================================================

This file reproduces the verbatim licence text and copyright of every
third-party component in the two distributed Accelerator closures: the Rust
cli/ workspace linked into the signed launcher and sub-binaries, and the
Vite-bundled frontend embedded in the visualiser binary.

It does not cover the vendored runtime, which ships as its own archives: Node
and playwright-core are attributed in the accelerator-driver-* archives, and
Chromium in the accelerator-browser-* archives, each under a NOTICES/ directory
that travels with the archive it describes.

It is generated. Run `mise run notices:update` to regenerate it from both
dependency graphs and `mise run notices:check` to pin it against drift. Do not
edit it by hand.

Every component under MPL-2.0 carries a §3.2 corresponding-source statement
below its entry, naming where its source is obtained.

The Rust section follows, then the frontend section. Within each, one block per
component, sorted by name and version.
"""

_RUST_SECTION = """\
===============================================================================
Rust components (cli/ workspace)
===============================================================================
"""

_FRONTEND_SECTION = """\
===============================================================================
Frontend components (embedded visualiser bundle)
===============================================================================
"""


def _block(
    *,
    name: str,
    version: str,
    license_id: str,
    source: str,
    copyright_line: str | None,
    text: str,
    corresponding_source: list[str] | None,
) -> str:
    """One component's attribution block. The shared shape both renderers emit.

    `corresponding_source`, when present, is the MPL-2.0 §3.2 statement listing
    resolvable source locations; a copyright line is emitted only when known,
    because upstream metadata does not always carry one.
    """
    lines = [
        _RULE,
        f"{name} {version}",
        _RULE,
        f"License: {license_id}",
        f"Source: {source}",
    ]
    if copyright_line:
        lines.append(f"Copyright: {copyright_line}")
    lines.append("")
    lines.append(text.rstrip("\n"))
    if corresponding_source:
        lines.append("")
        lines.append("MPL-2.0 corresponding source (§3.2):")
        lines.extend(f"  {url}" for url in corresponding_source)
    lines.append("")
    return "\n".join(lines)


def _crates_io_download(name: str, version: str) -> str:
    return f"https://crates.io/api/v1/crates/{name}/{version}/download"


def _npm_tarball(name: str, version: str) -> str:
    unscoped = name.rsplit("/", 1)[-1]
    return f"https://registry.npmjs.org/{name}/-/{unscoped}-{version}.tgz"


def _render_rust(raw: str) -> str:
    """Render the Rust section from cargo-about's `--format json` output.

    cargo-about groups verbatim text by licence, listing the crates under each;
    this inverts that into one block per crate carrying all its resolved
    licences, so the block shape matches the frontend renderer's. Each block
    names the crate's repository as its source, and an MPL-2.0 crate adds a §3.2
    source resolving to that repository plus the immutable crates.io download
    endpoint.
    """
    data = json.loads(raw)
    components: dict[str, _Component] = {}
    for licence in data.get("licenses", []):
        licence_id = licence["id"]
        text = licence["text"]
        for usage in licence.get("used_by", []):
            crate = usage["crate"]
            key = f"{crate['name']}@{crate['version']}"
            component = components.setdefault(
                key,
                _Component(
                    name=crate["name"],
                    version=crate["version"],
                    repository=crate.get("repository"),
                ),
            )
            component.licences[licence_id] = text

    blocks: list[str] = []
    for key in sorted(components):
        component = components[key]
        licences = component.licences
        source = (
            component.repository or f"https://crates.io/crates/{component.name}"
        )
        corresponding_source = None
        if _MPL in licences:
            corresponding_source = [
                source,
                _crates_io_download(component.name, component.version),
            ]
        blocks.append(
            _block(
                name=component.name,
                version=component.version,
                license_id=", ".join(sorted(licences)),
                source=source,
                copyright_line=None,
                text="\n\n".join(licences[lid] for lid in sorted(licences)),
                corresponding_source=corresponding_source,
            )
        )
    return "\n".join(blocks)


def _render_frontend(raw: str) -> str:
    """Render the frontend section from license-checker's JSON output.

    Keyed by `name@version`; each value carries the verbatim `licenseText`, the
    `copyright` line where the tool could derive one, and the `repository`. An
    MPL-2.0 package mirrors the Rust §3.2 emission against the npm registry
    tarball, so the header's blanket §3.2 claim stays true if a copyleft
    frontend dependency ever enters the closure.
    """
    data = json.loads(raw)
    blocks: list[str] = []
    for key in sorted(data):
        entry = data[key]
        name = entry["name"]
        version = entry["version"]
        licence_id = entry.get("licenses") or "(no licence declared)"
        if isinstance(licence_id, list):
            licence_id = ", ".join(licence_id)
        source = (
            entry.get("repository") or f"https://www.npmjs.com/package/{name}"
        )
        text = entry.get("licenseText") or "(no licence text provided)"
        corresponding_source = None
        if _MPL in licence_id:
            corresponding_source = [source, _npm_tarball(name, version)]
        blocks.append(
            _block(
                name=name,
                version=version,
                license_id=licence_id,
                source=source,
                copyright_line=entry.get("copyright") or None,
                text=text,
                corresponding_source=corresponding_source,
            )
        )
    return "\n".join(blocks)


def _fold(rust: str, frontend: str) -> str:
    """Concatenate the header and both sections, normalised to stable bytes.

    Verbatim upstream text can carry CRLF or a stray trailing newline, so line
    endings collapse to LF and the whole file ends in exactly one newline. This
    is what lets the byte-compare agree across the macOS generate host and the
    Linux check host.
    """
    combined = (
        f"{_HEADER}\n{_RUST_SECTION}\n{rust}\n{_FRONTEND_SECTION}\n{frontend}"
    )
    combined = combined.replace("\r\n", "\n").replace("\r", "\n")
    return combined.rstrip("\n") + "\n"


def _read(path: Path) -> str:
    with path.open(encoding="utf-8", newline="") as handle:
        return handle.read()


def _write(path: Path, content: str) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        handle.write(content)


def _run_cargo_about(context: Context) -> str:
    """Render the Rust closure as cargo-about JSON, offline and deterministic.

    `--frozen` (locked + offline) reads the licence files the cli/ registry
    already carries, so the render never touches the network and its bytes stay
    stable across cache warmth. `--fail` turns an unresolved licence into an
    error rather than a silently dropped component.
    """
    with context.cd(str(CLI_DIR)):
        result = context.run(
            "cargo about generate --workspace --frozen --fail --format json",
            warn=True,
            pty=False,
            hide=True,
        )
    if result.exited != 0:
        raise Exit(
            "cargo-about failed to render the Rust closure — see output above",
            code=1,
        )
    return result.stdout


def _run_license_checker(context: Context) -> str:
    """Render the frontend production closure as license-checker JSON.

    npm's run banner can still reach stdout on some npm versions even under
    `--silent`, so the payload is sliced from its first brace before return.
    """
    result = context.run(
        f"npm --prefix {FRONTEND} run --silent licenses:generate",
        warn=True,
        pty=False,
        hide=True,
    )
    if result.exited != 0:
        raise Exit(
            "license-checker failed to render the frontend closure — see "
            "output above",
            code=1,
        )
    stdout = result.stdout
    brace = stdout.find("{")
    if brace == -1:
        raise Exit(
            "license-checker produced no JSON object — see output above",
            code=1,
        )
    return stdout[brace:]


def _render(context: Context) -> str:
    rust = _render_rust(_run_cargo_about(context))
    frontend = _render_frontend(_run_license_checker(context))
    return _fold(rust, frontend)


@task
def check(context: Context) -> None:
    """Pin the notices file against a fresh dual-generator render."""
    if not ATTRIBUTION_ARTEFACT.exists():
        raise Exit(
            f"{ATTRIBUTION_ARTEFACT} is missing — run "
            "`mise run notices:update`",
            code=1,
        )
    if _render(context) != _read(ATTRIBUTION_ARTEFACT):
        raise Exit(
            f"{ATTRIBUTION_ARTEFACT} has drifted from the dependency graphs — "
            "run `mise run notices:update`",
            code=1,
        )


@task
def update(context: Context) -> None:
    """Regenerate the third-party notices file from both dependency graphs."""
    ATTRIBUTION_ARTEFACT.parent.mkdir(parents=True, exist_ok=True)
    _write(ATTRIBUTION_ARTEFACT, _render(context))
