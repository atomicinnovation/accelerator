"""Guard: ``config/test-support`` stays in two crates' dev-dependencies.

Only ``config`` and ``config-adapters`` may enable it, and only there.

The feature builds key descriptors the catalogue never declared, so a consent
resolution under it can return a value no real key would. Confined to the two
crates' own tests, every consumer's tests must exercise real, declared keys.

A crate could forward the feature through one of its own, so every workspace
package's ``[features]`` table is scanned too, including an optional
(``config?/``) or renamed dependency.

Not a cargo-deny rule: cargo-deny's feature bans apply to external crates,
and cannot tell a dev-dependency edge from a normal one.
"""

from invoke import Context, Exit, task

from tasks.shared.cargo_metadata import Package, workspace_packages

FEATURE = "test-support"
PERMITTED = frozenset({"config", "config-adapters"})


def _config_keys(package: Package) -> set[str]:
    return {
        dependency.get("rename") or dependency["name"]
        for dependency in package["dependencies"]
        if dependency["name"] == "config"
    }


def violations(packages: list[Package]) -> list[str]:
    """Each workspace package that enables or forwards the feature wrongly."""
    found: list[str] = []
    for package in packages:
        name = package["name"]
        for dependency in package["dependencies"]:
            if dependency["name"] != "config":
                continue
            if FEATURE not in dependency["features"]:
                continue
            if name not in PERMITTED:
                found.append(f"{name} enables config/{FEATURE}")
            elif dependency["kind"] != "dev":
                found.append(
                    f"{name} enables config/{FEATURE} outside "
                    "[dev-dependencies]"
                )
        forwards = {
            f"{key}{optional}/{FEATURE}"
            for key in _config_keys(package) | {"config"}
            for optional in ("", "?")
        }
        for feature, enables in sorted(package["features"].items()):
            if forwards & set(enables):
                found.append(
                    f"{name} forwards config/{FEATURE} through feature "
                    f"'{feature}'"
                )
    return found


@task
def check(context: Context) -> None:
    """Fail if config/test-support leaks beyond config's own tests."""
    offenders = violations(workspace_packages())
    if offenders:
        raise Exit(
            f"config/{FEATURE} builds undeclared key descriptors, so only "
            "config and config-adapters may enable it, and only in "
            "[dev-dependencies]:\n  " + "\n  ".join(offenders),
            code=1,
        )
