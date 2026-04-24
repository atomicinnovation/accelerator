from invoke import Context, task

from .helpers import repo_root


def _ensure_frontend_dist(context: Context) -> None:
    """Build `frontend/dist/` if its index.html is missing."""
    frontend_root = repo_root() / "skills/visualisation/visualise/frontend"
    dist_index = frontend_root / "dist" / "index.html"
    if not dist_index.exists():
        context.run(f"npm --prefix {frontend_root} run build")


@task
def visualiser(context: Context):
    """Unit tests for the visualiser server.

    Runs cargo test twice to cover both feature-gated test modules:
      1. `--no-default-features --features dev-frontend` — covers
         `path_normalisation_tests` and `dev_frontend_tests`. Does not
         require the SPA to be built.
      2. default features (embed-dist) — covers `path_normalisation_tests`
         and `embed_tests`. Requires `frontend/dist/index.html` because
         rust-embed reads the folder at compile time, so we build first if
         missing.
    """
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(
        f"cargo test --manifest-path {manifest} --lib "
        f"--no-default-features --features dev-frontend"
    )
    _ensure_frontend_dist(context)
    context.run(f"cargo test --manifest-path {manifest} --lib")


@task
def frontend(context: Context):
    """Unit tests for the visualiser frontend (Vitest)."""
    frontend_root = repo_root() / "skills/visualisation/visualise/frontend"
    context.run(f"npm --prefix {frontend_root} run test")
