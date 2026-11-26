"""Tests for the guarded ``test-support`` features in
``tasks/lint/config_test_support.py``.

``config/test-support`` builds test-only key descriptors that bypass the
catalogue, so only ``config`` and ``config-adapters`` may enable it.
``work-adapters/test-support`` seeds baseline entries around the rule that
keeps their rewrite marks, so only ``work-adapters`` and ``accelerator-work``
may enable it. Either only as a dev-dependency.

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


def _config(
    kind: str | None, *, rename: str | None = None, name: str = "config"
) -> dict:
    return {
        "name": name,
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


def _work_adapters(kind: str | None) -> dict:
    return _config(kind, name="work-adapters")


def test_work_adapters_and_the_work_cli_may_enable_its_feature() -> None:
    packages = [
        _package(
            "work-adapters", [_work_adapters("dev")], {"test-support": []}
        ),
        _package("accelerator-work", [_work_adapters("dev")]),
    ]
    assert config_test_support.violations(packages) == []


def test_another_crate_enabling_the_work_adapters_feature_is_flagged() -> None:
    packages = [_package("accelerator-migrate", [_work_adapters("dev")])]
    assert config_test_support.violations(packages) == [
        "accelerator-migrate enables work-adapters/test-support"
    ]


def test_the_work_cli_enabling_it_outside_dev_is_flagged() -> None:
    packages = [_package("accelerator-work", [_work_adapters(None)])]
    assert config_test_support.violations(packages) == [
        "accelerator-work enables work-adapters/test-support outside "
        "[dev-dependencies]"
    ]


def test_forwarding_the_work_adapters_feature_is_flagged() -> None:
    packages = [
        _package(
            "accelerator-work",
            features={"testing": ["work-adapters/test-support"]},
        )
    ]
    assert config_test_support.violations(packages) == [
        "accelerator-work forwards work-adapters/test-support through "
        "feature 'testing'"
    ]


def test_the_shipped_workspace_is_clean() -> None:
    packages = workspace_packages()
    names = {package["name"] for package in packages}
    assert {
        "config",
        "config-adapters",
        "jira-cli",
        "work-adapters",
        "accelerator-work",
    } <= names
    assert config_test_support.violations(packages) == []
