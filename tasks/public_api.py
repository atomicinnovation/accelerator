from pathlib import Path

from invoke import Context, Exit, Result, task

from tasks.shared.paths import CLI_DIR
from tasks.shared.rust import PUP_NIGHTLY

# Crates whose surface this lane pins, named explicitly: a new library crate
# is exempt from the pin until it is added here.
_PINNED_CRATES = ("tracker",)


def _snapshot(crate: str) -> Path:
    return CLI_DIR / crate / "tests" / "fixtures" / "public-api.txt"


def _render(context: Context, crate: str) -> Result:
    # Reuses the cargo-pup lane's pinned nightly for its rustdoc-JSON build;
    # cargo-public-api itself has no rustc_private driver, so it builds on
    # stable and needs no toolchain of its own. function-parameter-names is
    # load-bearing, not cosmetic — without it a same-typed parameter swap
    # (e.g. create's title/body) renders identically and the pin would not
    # catch it.
    with context.cd(str(CLI_DIR)):
        return context.run(
            f"cargo +{PUP_NIGHTLY} public-api "
            "--omit blanket-impls,auto-trait-impls "
            f"--include function-parameter-names -p {crate}",
            warn=True,
            pty=False,
            hide="out",
        )


@task
def check(context: Context) -> None:
    """Pin each crate's public API against its committed snapshot.

    Provisioning is guaranteed by the mise `depends` edge on
    deps:install:public-api. Reads rustdoc JSON, so the pin is immune to
    source formatting and catches a derive semantically, as the impls it
    generates.
    """
    for crate in _PINNED_CRATES:
        snapshot = _snapshot(crate)
        if not snapshot.exists() or not snapshot.read_text().strip():
            raise Exit(
                f"{snapshot} is missing or empty — regenerate it with "
                "`mise run public-api:update` only after a deliberate, "
                "reviewed surface change",
                code=1,
            )
        result = _render(context, crate)
        if result.exited != 0:
            raise Exit(
                f"cargo public-api: failed to render {crate}'s surface",
                code=1,
            )
        if result.stdout != snapshot.read_text():
            raise Exit(
                f"{crate}'s public API has drifted from {snapshot} — "
                "review the diff and, if intentional, run "
                "`mise run public-api:update`",
                code=1,
            )


@task
def update(context: Context) -> None:
    """Regenerate every pinned crate's public-API snapshot."""
    for crate in _PINNED_CRATES:
        result = _render(context, crate)
        if result.exited != 0:
            raise Exit(
                f"cargo public-api: failed to render {crate}'s surface",
                code=1,
            )
        _snapshot(crate).write_text(result.stdout)
