from pathlib import Path

from invoke import Context, task

_REPO_ROOT = Path(__file__).resolve().parent.parent


@task
def frontend(context: Context):
    """Build the visualiser frontend (Vite production build into dist/)."""
    frontend_root = _REPO_ROOT / "skills/visualisation/visualise/frontend"
    context.run(f"npm --prefix {frontend_root} run build")


@task
def server_dev(context: Context):
    """Build the visualiser server binary with the dev-frontend feature.

    Serves the frontend from the filesystem at runtime; used for local
    development and E2E tests. Not for release.
    """
    manifest = _REPO_ROOT / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(
        f"cargo build --manifest-path {manifest} "
        f"--no-default-features --features dev-frontend"
    )


@task
def server_release(context: Context):
    """Build the visualiser server binary for release.

    Uses the default embed-dist feature, which bakes the frontend assets
    into the binary at compile time for a self-contained release artifact.
    """
    manifest = _REPO_ROOT / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(f"cargo build --manifest-path {manifest} --release")
