"""Regression: every reqwest client in cli/ trusts only the bundled roots.

From 0.13 reqwest links rustls-platform-verifier whatever its features, so the
host certificate store's crates are in the launcher's graph and the feature
graph test can no longer keep them out. What keeps a client off the host store
is building it with `tls_certs_only` over webpki-root-certs' roots; this scan
fails any production client builder that does not.
"""

import re
from pathlib import Path

_CLI = Path(__file__).resolve().parents[3] / "cli"
_BUILDER = re.compile(r"\bClient::builder\(\)")
_TRUST = re.compile(r"\A\s*\.tls_certs_only\(")


def _production_sources() -> list[Path]:
    return [
        path
        for path in _CLI.glob("*/src/**/*.rs")
        if "target" not in path.parts
    ]


def _builders_without_bundled_roots(source: str) -> list[int]:
    lines = source.splitlines()
    return [
        number
        for number, line in enumerate(lines, 1)
        if _BUILDER.search(line)
        and not (number < len(lines) and _TRUST.match(lines[number]))
    ]


def test_every_production_client_trusts_only_the_bundled_roots() -> None:
    unconfined = [
        f"{path.relative_to(_CLI)}:{line}"
        for path in _production_sources()
        for line in _builders_without_bundled_roots(path.read_text())
    ]
    assert not unconfined, (
        "reqwest clients built without `.tls_certs_only(bundled roots)` as "
        f"the first builder call trust the host store: {unconfined}"
    )


def test_the_scan_finds_the_clients_it_guards() -> None:
    builders = [
        path
        for path in _production_sources()
        if _BUILDER.search(path.read_text())
    ]
    assert len(builders) >= 6, f"has client construction moved? {builders}"


def test_the_scan_catches_a_client_without_the_bundled_roots() -> None:
    source = "let client = Client::builder()\n    .timeout(limit)\n"

    assert _builders_without_bundled_roots(source) == [1]
