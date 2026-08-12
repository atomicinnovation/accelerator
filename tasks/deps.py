import re

from invoke import Context, Exit, task

from tasks.shared.paths import DOCS_SITE, FRONTEND
from tasks.shared.rust import PUBLIC_API_VERSION, PUP_VERSION, RUST_NIGHTLY
from tasks.shared.targets import TARGETS

_CROSS_TARGETS = tuple(triple for triple, _ in TARGETS)

_ANSI = re.compile(r"\x1b\[[0-9;]*m")


@task
def install_python(context: Context) -> None:
    """Install all Python dependencies."""
    context.run("uv sync --all-groups --frozen")


@task
def install_rust_targets(context: Context) -> None:
    """Install the Rust cross-compile targets needed for release builds."""
    context.run(f"rustup target add {' '.join(_CROSS_TARGETS)}")


@task
def install_rust_components(context: Context) -> None:
    """Install the rustfmt, clippy, and llvm-tools-preview components.

    Not trusted to mise's [tools] rust `components` field: that is silently
    skipped when the toolchain is already present (a cached/pre-installed
    stable on a CI runner), so the components are provisioned explicitly —
    mirroring deps:install:rust-targets. llvm-tools-preview is included so the
    coverage test tasks (Phase 5) do not each trigger `cargo llvm-cov`'s
    implicit `rustup component add` at runtime — two such installs racing on
    ~/.rustup fails the parallel test:unit roll-up.
    """
    context.run("rustup component add rustfmt clippy llvm-tools-preview")


@task
def install_nightly(context: Context) -> None:
    """Provision the pinned nightly toolchain the nightly-lane steps share.

    Not a mise [tool]: mise cannot pin two rust toolchains, so the nightly is
    rustup-managed here and invoked via `cargo +<nightly>`. Every task that
    needs it depends on this one, directly or through its tool's install task,
    so no two of them race on ~/.rustup. Idempotent.

    rustc-dev and rust-src carry the `rustc_private` internals a compiler
    plugin links against; the minimal profile's rustc component carries the
    `rustdoc` that emits the JSON a surface pin reads. llvm-tools-preview
    mirrors the stable component set (deps:install:rust-components).
    """
    # If the dated nightly has been GC'd from static.rust-lang.org this fails
    # HERE, before any `+nightly` invocation emits an opaque "override does not
    # resolve" error.
    nightly = context.run(
        f"rustup toolchain install {RUST_NIGHTLY} --profile minimal "
        "--component rustc-dev --component rust-src "
        "--component llvm-tools-preview",
        warn=True,
        pty=False,
    )
    if nightly.exited != 0:
        raise Exit(
            f"failed to install {RUST_NIGHTLY} (GC'd from"
            " static.rust-lang.org?) — bump RUST_NIGHTLY in"
            " tasks/shared/rust.py, together with PUP_VERSION, to a compatible"
            " pair",
            code=1,
        )

    # Pre-flight: confirm rustup's `+toolchain` override resolves on this
    # machine, so a broken override fails here rather than as an opaque
    # rustc_private load error inside a check that assumes the lane works.
    preflight = context.run(
        f"cargo +{RUST_NIGHTLY} --version", warn=True, pty=False
    )
    if preflight.exited != 0:
        raise Exit(
            f"`cargo +{RUST_NIGHTLY}` does not resolve — is ~/.cargo/bin "
            "(rustup's proxies) on PATH ahead of any cargo shim?",
            code=1,
        )


def _pup_already_installed(context: Context) -> bool:
    # Token equality on the version line, not a substring match (0.1.80 would
    # false-match 0.1.8): strip ANSI (cargo-pup colourises even when piped),
    # then split so PUP_VERSION must be a whole token.
    probe = context.run(
        f"cargo +{RUST_NIGHTLY} pup --version", warn=True, pty=False
    )
    return PUP_VERSION in _ANSI.sub("", probe.stdout).split()


@task
def install_pup(context: Context) -> None:
    """Provision the pinned cargo-pup.

    Built under RUST_NIGHTLY because its `rustc_private` driver only loads
    under the nightly it was compiled against, which is why the two pins move
    together. The toolchain itself arrives via the deps:install:nightly edge.

    Guarded by a presence probe so the multi-minute source build is skipped in
    steady state (`cargo install --locked` is not a pure no-op — it resolves and
    can rebuild or hit the network). --locked pins cargo-pup's transitive build
    dependencies.
    """
    if _pup_already_installed(context):
        return
    install = context.run(
        f"cargo +{RUST_NIGHTLY} install cargo_pup "
        f"--version {PUP_VERSION} --locked",
        warn=True,
        pty=False,
    )
    if install.exited != 0:
        raise Exit(f"failed to install cargo-pup {PUP_VERSION}", code=1)


def _public_api_already_installed(context: Context) -> bool:
    # Token equality on the version line, not a substring match (0.52.10 would
    # false-match 0.52.1): strip ANSI, then split so PUBLIC_API_VERSION must be
    # a whole token.
    probe = context.run("cargo public-api --version", warn=True, pty=False)
    return PUBLIC_API_VERSION in _ANSI.sub("", probe.stdout).split()


@task
def install_public_api(context: Context) -> None:
    """Provision the pinned cargo-public-api.

    Built on stable: the tool has no `rustc_private` driver, so it is coupled to
    RUST_NIGHTLY only by the rustdoc-JSON format it reads, and reaches the
    nightly by shelling out to its `rustdoc` at check time. Idempotent — no-ops
    when the pinned version is already present.
    """
    if _public_api_already_installed(context):
        return
    install = context.run(
        f"cargo install cargo-public-api --version {PUBLIC_API_VERSION} "
        "--locked",
        warn=True,
        pty=False,
    )
    if install.exited != 0:
        raise Exit(
            f"failed to install cargo-public-api {PUBLIC_API_VERSION}",
            code=1,
        )


@task
def install_node(context: Context) -> None:
    """Install Node.js dependencies for the visualiser frontend."""
    context.run(f"npm --prefix {FRONTEND} ci")


@task
def install_playwright(context: Context) -> None:
    """Install Playwright browser binaries (Chromium + OS-level deps)."""
    context.run(
        f"npx --prefix {FRONTEND} playwright install --with-deps chromium"
    )


@task
def install_docs(context: Context) -> None:
    """Install Node.js dependencies for the documentation site."""
    context.run(f"npm --prefix {DOCS_SITE} ci")


@task
def install_docs_playwright(context: Context) -> None:
    """Install the Chromium binary rehype-mermaid renders with."""
    context.run(
        f"npx --prefix {DOCS_SITE} playwright install --with-deps chromium"
    )
