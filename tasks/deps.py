from invoke import Context, task

from tasks.shared.paths import FRONTEND
from tasks.shared.targets import TARGETS

_CROSS_TARGETS = tuple(triple for triple, _ in TARGETS)


@task
def install_python(context: Context) -> None:
    """Install all Python dependencies."""
    context.run("uv sync --all-groups --frozen")


@task
def install_rust_targets(context: Context) -> None:
    """Install the Rust cross-compile targets needed for release builds."""
    context.run(f"rustup target add {' '.join(_CROSS_TARGETS)}")


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
