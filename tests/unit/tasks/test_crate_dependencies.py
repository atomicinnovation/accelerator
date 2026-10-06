"""Tests for the ADR-0069 crate-dependency lint in
``tasks/lint/crate_dependencies.py``.

Synthetic ``cargo metadata`` packages exercise every rule, constraint and
declaration case. Real-tree assertions pin the shipped declarations and the
findings the shipped workspace still carries.
"""

import pytest
from invoke import Context, Exit

from tasks.lint import crate_dependencies
from tasks.shared.cargo_metadata import workspace_packages

Package = dict


def _dep(name: str, kind: str | None = None) -> dict:
    return {"name": name, "kind": kind}


def _crate(
    name: str,
    role: str | None = None,
    *,
    context: str | None = None,
    kind: str | None = None,
    downstreams: list[str] | None = None,
    dependencies: list[dict] | None = None,
) -> Package:
    declared = {
        key: value
        for key, value in {
            "role": role,
            "context": context,
            "kind": kind,
            "downstreams": downstreams,
        }.items()
        if value is not None
    }
    return {
        "name": name,
        "metadata": {"accelerator": declared} if declared else None,
        "dependencies": dependencies or [],
    }


def _workspace() -> list[Package]:
    return [
        _crate("kernel", "kernel"),
        _crate("config", "domain", context="config", kind="platform"),
        _crate("config-adapters", "adapter", context="config"),
        _crate("config-codec", "adapter", context="config"),
        _crate("vcs", "domain", context="vcs", kind="platform"),
        _crate("vcs-adapters", "adapter", context="vcs"),
        _crate("corpus", "domain", context="corpus", kind="platform"),
        _crate("corpus-adapters", "adapter", context="corpus"),
        _crate(
            "tracker",
            "domain",
            context="tracker",
            kind="shared",
            downstreams=["work", "jira"],
        ),
        _crate("tracker-support", "adapter", context="tracker"),
        _crate("work", "domain", context="work", kind="product"),
        _crate("work-adapters", "adapter", context="work"),
        _crate("research", "domain", context="research", kind="product"),
        _crate("research-adapters", "adapter", context="research"),
        _crate("jira-client", "adapter", context="jira", kind="product"),
        _crate("document", "technical-library"),
        _crate("store", "technical-library"),
        _crate("work-cli", "composition-root"),
        _crate("corpus-cli", "composition-root"),
        _crate("launcher", "launcher"),
        _crate("verify", "bootstrap-verifier"),
        _crate("vcs-test-support", "test-support"),
    ]


def _findings(*edges: tuple[str, str] | tuple[str, str, str]) -> list[str]:
    packages = {package["name"]: package for package in _workspace()}
    for source, target, *kind in edges:
        packages[source]["dependencies"].append(_dep(target, *kind))
    return [
        str(f) for f in crate_dependencies.findings(list(packages.values()))
    ]


def _declaration_findings(*packages: Package) -> list[str]:
    return [str(f) for f in crate_dependencies.findings(list(packages))]


@pytest.mark.parametrize(
    "target", ["document", "config-adapters", "work-cli", "launcher"]
)
def test_a_domain_crate_reaches_only_kernel_and_domains(target) -> None:
    assert _findings(("work", target)) == [f"work -> {target}: rule 1"]


@pytest.mark.parametrize("source", ["work", "corpus-adapters"])
def test_nothing_depends_on_a_product_context(source) -> None:
    assert _findings((source, "research")) == [f"{source} -> research: rule 2"]


def test_two_platform_contexts_in_a_cycle_are_reported() -> None:
    assert _findings(
        ("config-adapters", "vcs"), ("vcs-adapters", "config")
    ) == [
        "config-adapters -> vcs: rule 2 (cycle among config, vcs)",
        "vcs-adapters -> config: rule 2 (cycle among config, vcs)",
    ]


def test_three_platform_contexts_in_a_cycle_are_reported() -> None:
    cycle = "rule 2 (cycle among config, corpus, vcs)"
    assert _findings(
        ("config", "vcs"), ("vcs-adapters", "corpus"), ("corpus", "config")
    ) == [
        f"config -> vcs: {cycle}",
        f"corpus -> config: {cycle}",
        f"vcs-adapters -> corpus: {cycle}",
    ]


@pytest.mark.parametrize("kind", [None, "build"])
def test_an_adapter_never_reaches_another_contexts_adapter(kind) -> None:
    assert _findings(("work-adapters", "vcs-adapters", kind)) == [
        "work-adapters -> vcs-adapters: rule 4"
    ]


def test_an_adapter_never_reaches_a_composition_root() -> None:
    assert _findings(("work-adapters", "work-cli")) == [
        "work-adapters -> work-cli: rule 4"
    ]


@pytest.mark.parametrize(
    ("source", "target"),
    [
        ("work-cli", "corpus-cli"),
        ("work-cli", "launcher"),
        ("launcher", "work-cli"),
    ],
)
def test_no_composition_root_depends_on_another(source, target) -> None:
    assert _findings((source, target)) == [f"{source} -> {target}: rule 5"]


@pytest.mark.parametrize("target", ["tracker", "tracker-support", "work"])
def test_the_launcher_reaches_only_platform_contexts(target) -> None:
    assert _findings(("launcher", target)) == [f"launcher -> {target}: rule 6"]


def test_kernel_depends_on_no_workspace_crate() -> None:
    assert _findings(("kernel", "config")) == [
        "kernel -> config: kernel constraint"
    ]


@pytest.mark.parametrize("target", ["config", "config-adapters"])
def test_a_technical_library_reaches_only_kernel_and_libraries(target) -> None:
    assert _findings(("document", target)) == [
        f"document -> {target}: technical-library constraint"
    ]


def test_only_the_launcher_depends_on_the_bootstrap_verifier() -> None:
    assert _findings(("work-cli", "verify")) == [
        "work-cli -> verify: verifier constraint"
    ]


@pytest.mark.parametrize("kind", [None, "build"])
def test_nothing_ships_a_dependency_on_test_support(kind) -> None:
    assert _findings(("work-adapters", "vcs-test-support", kind)) == [
        "work-adapters -> vcs-test-support: test-support check"
    ]


@pytest.mark.parametrize(
    ("source", "target", "kind"),
    [
        ("config-adapters", "config-codec", None),
        ("config-adapters", "config", None),
        ("vcs-test-support", "work-cli", None),
        ("verify", "work-adapters", None),
        ("work-adapters", "vcs-adapters", "dev"),
        ("launcher", "verify", None),
        ("document", "kernel", None),
        ("document", "store", None),
        ("launcher", "config", None),
        ("launcher", "config-adapters", None),
        ("launcher", "document", None),
        ("work-adapters", "vcs", None),
    ],
)
def test_a_permitted_dependency_is_not_reported(source, target, kind) -> None:
    assert _findings((source, target, kind)) == []


def test_a_composition_root_wires_adapters_of_several_contexts() -> None:
    assert (
        _findings(
            ("work-cli", "config-adapters"),
            ("work-cli", "vcs-adapters"),
            ("work-cli", "work-adapters"),
        )
        == []
    )


def test_a_declared_downstream_builds_on_a_shared_context() -> None:
    assert (
        _findings(
            ("work", "tracker"),
            ("work-adapters", "tracker-support"),
            ("jira-client", "tracker-support"),
        )
        == []
    )


def test_an_undeclared_downstream_may_not_build_on_a_shared_context() -> None:
    assert _findings(
        ("research", "tracker"), ("research-adapters", "tracker-support")
    ) == [
        "research -> tracker: rule 2",
        "research-adapters -> tracker-support: rule 2",
        "research-adapters -> tracker-support: rule 4",
    ]


@pytest.mark.parametrize(
    ("packages", "expected"),
    [
        (
            [_crate("store")],
            "store: declaration check (no role declared)",
        ),
        (
            [_crate("store", "service")],
            "store: declaration check (unrecognised role 'service')",
        ),
        (
            [_crate("work", "domain", kind="product")],
            "work: declaration check (role 'domain' needs a context)",
        ),
        (
            [_crate("work", "domain", context="work")],
            "work: declaration check (context 'work' declares no kind)",
        ),
        (
            [_crate("work", "domain", context="work", kind="core")],
            "work: declaration check (unrecognised kind 'core')",
        ),
        (
            [
                _crate("tracker", "domain", context="tracker", kind="shared"),
            ],
            "tracker: declaration check (shared context 'tracker' declares "
            "no downstreams)",
        ),
        (
            [
                _crate(
                    "vcs",
                    "domain",
                    context="vcs",
                    kind="platform",
                    downstreams=["vcs"],
                )
            ],
            "vcs: declaration check (only a shared context's domain crate "
            "declares downstreams)",
        ),
        (
            [
                _crate(
                    "tracker",
                    "domain",
                    context="tracker",
                    kind="shared",
                    downstreams=["billing"],
                )
            ],
            "tracker: declaration check (downstream 'billing' is not a "
            "context)",
        ),
        (
            [_crate("document", "technical-library", context="corpus")],
            "document: declaration check (role 'technical-library' carries "
            "no context)",
        ),
        (
            [_crate("document", "technical-library", kind="platform")],
            "document: declaration check (role 'technical-library' carries "
            "no context)",
        ),
    ],
)
def test_a_malformed_declaration_is_reported(packages, expected) -> None:
    assert _declaration_findings(*packages) == [expected]


def test_conflicting_kinds_for_one_context_are_reported() -> None:
    assert _declaration_findings(
        _crate("jira-client", "adapter", context="jira", kind="product"),
        _crate("jira-sync", "adapter", context="jira", kind="platform"),
    ) == [
        "jira-client: declaration check (context 'jira' declares conflicting "
        "kinds platform, product)",
        "jira-sync: declaration check (context 'jira' declares conflicting "
        "kinds platform, product)",
    ]


def test_every_adapter_of_a_domainless_context_declares_its_kind() -> None:
    assert _declaration_findings(
        _crate("jira-client", "adapter", context="jira", kind="product"),
        _crate("jira-sync", "adapter", context="jira"),
    ) == ["jira-sync: declaration check (context 'jira' declares no kind)"]


def test_a_context_with_a_domain_crate_declares_its_kind_there() -> None:
    assert _declaration_findings(
        _crate("config", "domain", context="config", kind="platform"),
        _crate("config-adapters", "adapter", context="config", kind="platform"),
    ) == [
        "config-adapters: declaration check (kind belongs on context "
        "'config''s domain crate)"
    ]


def test_check_names_every_finding(monkeypatch) -> None:
    packages = {package["name"]: package for package in _workspace()}
    packages["work"]["dependencies"].append(_dep("document"))
    packages["launcher"]["dependencies"].append(_dep("tracker"))
    monkeypatch.setattr(
        crate_dependencies,
        "workspace_packages",
        lambda: list(packages.values()),
    )

    with pytest.raises(Exit) as raised:
        crate_dependencies.check(Context())

    assert "work -> document: rule 1" in str(raised.value.message)
    assert "launcher -> tracker: rule 6" in str(raised.value.message)


_PLATFORM = "platform"
_PRODUCT = "product"

_EXPECTED_DECLARATIONS = {
    "kernel": ("kernel", None, None, None),
    "config": ("domain", "config", _PLATFORM, None),
    "config-adapters": ("adapter", "config", None, None),
    "consent-adapters": ("adapter", "config", None, None),
    "vcs": ("domain", "vcs", _PLATFORM, None),
    "vcs-adapters": ("adapter", "vcs", None, None),
    "corpus": ("domain", "corpus", _PLATFORM, None),
    "corpus-adapters": ("adapter", "corpus", None, None),
    "tracker": ("domain", "tracker", "shared", ["work", "jira", "linear"]),
    "tracker-support": ("adapter", "tracker", None, None),
    "work": ("domain", "work", _PRODUCT, None),
    "work-adapters": ("adapter", "work", None, None),
    "collaboration": ("domain", "collaboration", _PRODUCT, None),
    "github": ("adapter", "collaboration", None, None),
    "migrate": ("domain", "migrate", _PRODUCT, None),
    "migrate-adapters": ("adapter", "migrate", None, None),
    "design": ("domain", "design", _PRODUCT, None),
    "design-adapters": ("adapter", "design", None, None),
    "research": ("domain", "research", _PRODUCT, None),
    "research-adapters": ("adapter", "research", None, None),
    "jira-client": ("adapter", "jira", _PRODUCT, None),
    "linear-client": ("adapter", "linear", _PRODUCT, None),
    "document": ("technical-library", None, None, None),
    "store": ("technical-library", None, None, None),
    "remote-projection": ("technical-library", None, None, None),
    "process-probe": ("technical-library", None, None, None),
    "accelerator-corpus": ("composition-root", None, None, None),
    "accelerator-vcs": ("composition-root", None, None, None),
    "accelerator-work": ("composition-root", None, None, None),
    "accelerator-collaboration": ("composition-root", None, None, None),
    "accelerator-migrate": ("composition-root", None, None, None),
    "jira-cli": ("composition-root", None, None, None),
    "linear-cli": ("composition-root", None, None, None),
    "accelerator-design": ("composition-root", None, None, None),
    "accelerator-research": ("composition-root", None, None, None),
    "accelerator-visualiser": ("composition-root", None, None, None),
    "accelerator": ("launcher", None, None, None),
    "cli-test-support": ("test-support", None, None, None),
    "vcs-test-support": ("test-support", None, None, None),
    "tracker-test-support": ("test-support", None, None, None),
    "http-test-support": ("test-support", None, None, None),
    "graphql-test-support": ("test-support", None, None, None),
    "accelerator-verify": ("bootstrap-verifier", None, None, None),
}


def test_the_shipped_declarations_match_the_expected_table() -> None:
    declared = {}
    for package in workspace_packages():
        table = (package["metadata"] or {}).get("accelerator", {})
        declared[package["name"]] = (
            table.get("role"),
            table.get("context"),
            table.get("kind"),
            table.get("downstreams"),
        )
    assert declared == _EXPECTED_DECLARATIONS


def test_the_shipped_workspace_findings_are_the_known_violations() -> None:
    assert [
        str(f) for f in crate_dependencies.findings(workspace_packages())
    ] == [
        "accelerator -> tracker-support: rule 6",
        "consent-adapters -> vcs-adapters: rule 4",
        "migrate -> document: rule 1",
        "migrate-adapters -> config-adapters: rule 4",
        "migrate-adapters -> corpus-adapters: rule 4",
        "migrate-adapters -> vcs-adapters: rule 4",
        "migrate-adapters -> work-adapters: rule 2",
        "migrate-adapters -> work-adapters: rule 4",
        "research-adapters -> corpus-adapters: rule 4",
    ]
