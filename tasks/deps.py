from invoke import Context, task

from tasks.shared.paths import FRONTEND
from tasks.shared.targets import TARGETS

_CROSS_TARGETS = tuple(triple for triple, _ in TARGETS)


@task
def install_python(context: Context):
    """Install all Python dependencies."""
    context.run("uv sync --all-groups --frozen")


@task
def install_rust_targets(context: Context):
    """Install the four Rust cross-compile targets required for release builds."""
    context.run(f"rustup target add {' '.join(_CROSS_TARGETS)}")


@task
def install_node(context: Context):
    """Install Node.js dependencies for the visualiser frontend."""
    context.run(f"npm --prefix {FRONTEND} ci")
