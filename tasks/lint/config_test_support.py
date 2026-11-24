"""Guard: each guarded ``test-support`` feature stays in its permitted crates.

``config/test-support`` builds key descriptors the catalogue never declared,
so a consent resolution under it can return a value no real key would.
Confined to ``config`` and ``config-adapters``' own tests, every consumer's
tests must exercise real, declared keys.

``work-adapters/test-support`` seeds baseline entries without the rule that
sets, keeps and clears their rewrite marks. Confined to ``work-adapters`` and
``accelerator-work``' tests, no production write can bypass that rule.

A crate could forward a feature through one of its own, so every workspace
package's ``[features]`` table is scanned too, including an optional
(``config?/``) or renamed dependency.

Not a cargo-deny rule: cargo-deny's feature bans apply to external crates,
and cannot tell a dev-dependency edge from a normal one.
"""

from invoke import Context, Exit, task

from tasks.shared.cargo_metadata import Package, workspace_packages

FEATURE = "test-support"
PERMITTED: dict[str, frozenset[str]] = {
    "config": frozenset({"config", "config-adapters"}),
    "work-adapters": frozenset({"work-adapters", "accelerator-work"}),
}


def _dependency_keys(package: Package, guarded: str) -> set[str]:
    return {
        dependency.get("rename") or dependency["name"]
        for dependency in package["dependencies"]
        if dependency["name"] == guarded
    }


def _enabling_violations(package: Package) -> list[str]:
    name = package["name"]
    found: list[str] = []
    for dependency in package["dependencies"]:
        guarded = dependency["name"]
        if guarded not in PERMITTED or FEATURE not in dependency["features"]:
            continue
        if name not in PERMITTED[guarded]:
            found.append(f"{name} enables {guarded}/{FEATURE}")
        elif dependency["kind"] != "dev":
            found.append(
                f"{name} enables {guarded}/{FEATURE} outside [dev-dependencies]"
            )
    return found


def _forwarding_violations(package: Package) -> list[str]:
    name = package["name"]
    found: list[str] = []
    for guarded in PERMITTED:
        forwards = {
            f"{key}{optional}/{FEATURE}"
            for key in _dependency_keys(package, guarded) | {guarded}
            for optional in ("", "?")
        }
        for feature, enables in sorted(package["features"].items()):
            if forwards & set(enables):
                found.append(
                    f"{name} forwards {guarded}/{FEATURE} through feature "
                    f"'{feature}'"
                )
    return found


def violations(packages: list[Package]) -> list[str]:
    """Each workspace package that enables or forwards a feature wrongly."""
    found: list[str] = []
    for package in packages:
        found.extend(_enabling_violations(package))
        found.extend(_forwarding_violations(package))
    return found


@task
def check(context: Context) -> None:
    """Fail if a guarded test-support feature leaks beyond its crates."""
    offenders = violations(workspace_packages())
    if offenders:
        permitted = "; ".join(
            f"{guarded}/{FEATURE} only in {', '.join(sorted(crates))}"
            for guarded, crates in PERMITTED.items()
        )
        raise Exit(
            "a guarded test-support feature is enabled where it may not be "
            f"({permitted}, and only in [dev-dependencies]):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
