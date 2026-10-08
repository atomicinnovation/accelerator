"""Tests for the ``config/test-support`` guard in
``tasks/lint/config_test_support.py``.

The feature builds test-only key descriptors that bypass the catalogue, so
only ``config`` and ``config-adapters`` may enable it, and only as a
dev-dependency: a consumer's tests must exercise real keys.

Synthetic ``cargo metadata`` packages exercise every branch, and a real-tree
assertion proves the shipped workspace is clean.
"""

from tasks.lint import config_test_support
from tasks.shared.cargo_metadata import workspace_packages


def _package(
    name: str,
    dependencies: list[dict] | None = None,
    features: dict[str, list[str]] | None = None,
) -> dict:
    return {
        "name": name,
        "dependencies": dependencies or [],
        "features": features or {},
    }


def _config(kind: str | None, *, rename: str | None = None) -> dict:
    return {
        "name": "config",
        "kind": kind,
        "rename": rename,
        "features": ["test-support"],
    }


def test_a_permitted_crate_may_enable_it_as_a_dev_dependency() -> None:
    packages = [
        _package("config", [_config("dev")], {"test-support": []}),
        _package("config-adapters", [_config("dev")]),
    ]
    assert config_test_support.violations(packages) == []


def test_any_other_crate_enabling_it_is_flagged() -> None:
    packages = [_package("jira-client", [_config("dev")])]
    assert config_test_support.violations(packages) == [
        "jira-client enables config/test-support"
    ]


def test_a_permitted_crate_enabling_it_outside_dev_is_flagged() -> None:
    packages = [_package("config-adapters", [_config(None)])]
    assert config_test_support.violations(packages) == [
        "config-adapters enables config/test-support outside [dev-dependencies]"
    ]


def test_a_build_dependency_is_not_a_dev_dependency() -> None:
    packages = [_package("config-adapters", [_config("build")])]
    assert config_test_support.violations(packages) == [
        "config-adapters enables config/test-support outside [dev-dependencies]"
    ]


def test_config_without_the_feature_is_fine() -> None:
    dependency = {**_config(None), "features": []}
    packages = [_package("jira-client", [dependency])]
    assert config_test_support.violations(packages) == []


def test_forwarding_through_a_feature_is_flagged() -> None:
    packages = [
        _package(
            "linear-client",
            features={
                "testing": ["config/test-support"],
                "maybe": ["config?/test-support"],
            },
        )
    ]
    assert config_test_support.violations(packages) == [
        "linear-client forwards config/test-support through feature 'maybe'",
        "linear-client forwards config/test-support through feature 'testing'",
    ]


def test_forwarding_through_a_renamed_dependency_is_flagged() -> None:
    packages = [
        _package(
            "work-cli",
            dependencies=[{**_config(None, rename="cfg"), "features": []}],
            features={"testing": ["cfg/test-support"]},
        )
    ]
    assert config_test_support.violations(packages) == [
        "work-cli forwards config/test-support through feature 'testing'"
    ]


def test_the_shipped_workspace_is_clean() -> None:
    packages = workspace_packages()
    names = {package["name"] for package in packages}
    assert {"config", "config-adapters", "jira-cli"} <= names
    assert config_test_support.violations(packages) == []
