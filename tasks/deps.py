from invoke import Context, task

_CROSS_TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
)


@task
def install_python(context: Context):
    """Install all Python dependencies."""
    context.run("uv sync --all-groups --frozen")


@task
def install_rust_targets(context: Context):
    """Install the four Rust cross-compile targets required for release builds."""
    context.run(f"rustup target add {' '.join(_CROSS_TARGETS)}")
