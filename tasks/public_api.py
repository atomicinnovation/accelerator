from pathlib import Path

from invoke import Context, Exit, Result, task

from tasks.shared.paths import CLI_DIR
from tasks.shared.rust import RUST_NIGHTLY

# The crates whose surface is a deliverable rather than whatever happened to be
# `pub`: every domain crate, plus the shared libraries every layer builds
# against. For a domain crate this pairs with cargo-pup — pup constrains what it
# may import inward, this pin constrains what it exposes outward.
#
# Every entry is a directory name that is also the package name, which the
# snapshot path and the `-p` argument both rely on.
_PINNED_CRATES = (
    "collaboration",
    "config",
    "corpus",
    "design",
    "document",
    "kernel",
    "migrate",
    "store",
    "tracker",
    "vcs",
    "work",
)

_ADAPTER = (
    "an adapter: its surface is a construction API for one composition root, "
    "not a contract siblings build against, and it changes with the technology "
    "it wraps rather than with the domain"
)
_COMPOSITION_ROOT = (
    "a composition root: it owns a binary, and its library surface exists only "
    "to be wired up by its own main"
)
_TEST_SUPPORT = (
    "test support: consumed only by other crates' test targets, where a"
    " widened surface is caught by the tests that use it failing to compile"
)

_EXEMPT_MEMBERS = {
    "config-adapters": _ADAPTER,
    "design-adapters": _ADAPTER,
    "corpus-adapters": _ADAPTER,
    "migrate-adapters": _ADAPTER,
    "vcs-adapters": _ADAPTER,
    "work-adapters": _ADAPTER,
    "github": (
        f"{_ADAPTER}. Named for the forge rather than as"
        " collaboration-adapters, but that is what it is"
    ),
    "collaboration-cli": _COMPOSITION_ROOT,
    "corpus-cli": _COMPOSITION_ROOT,
    "design-cli": _COMPOSITION_ROOT,
    "migrate-cli": _COMPOSITION_ROOT,
    "vcs-cli": _COMPOSITION_ROOT,
    "work-cli": _COMPOSITION_ROOT,
    "launcher": (
        f"{_COMPOSITION_ROOT}. Its dispatch surface is pinned instead by the"
        " thirteen-point registration guard and the bootstrap tests"
    ),
    "verify": "a bin-only crate: it exposes no library surface at all",
    "visualiser/server": (
        f"{_COMPOSITION_ROOT}. Its contract with the outside world is the HTTP"
        " API, held by the server's own integration and E2E suites"
    ),
    "process-probe": (
        "a two-function primitive over one syscall per platform: its contract"
        " is the epoch value itself, held by the fixture-replayed arithmetic"
        " tests and by the identity semantics each caller builds on top"
    ),
    "http-test-support": _TEST_SUPPORT,
    "tracker-test-support": _TEST_SUPPORT,
    "vcs-test-support": _TEST_SUPPORT,
}


def _snapshot(crate: str) -> Path:
    return CLI_DIR / crate / "tests" / "fixtures" / "public-api.txt"


def _render(context: Context, crate: str) -> Result:
    # Runs on RUST_NIGHTLY for one reason: rustdoc emits its JSON only there.
    # function-parameter-names is load-bearing, not cosmetic — without it a
    # same-typed parameter swap (e.g. create's title/body) renders identically
    # and the pin would not catch it.
    with context.cd(str(CLI_DIR)):
        return context.run(
            f"cargo +{RUST_NIGHTLY} public-api "
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
    deps:install:public-api, which pulls the nightly in behind it. Reads rustdoc
    JSON, so the pin is immune to source formatting and catches a derive
    semantically, as the impls it generates.
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
        snapshot = _snapshot(crate)
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        snapshot.write_text(result.stdout)
