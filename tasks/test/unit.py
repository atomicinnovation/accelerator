from invoke import Context, task

from .helpers import repo_root


@task
def visualiser(context: Context):
    """Unit tests for the visualiser server.

    Runs cargo test twice to cover both feature-gated test modules:
      1. `--no-default-features --features dev-frontend` — covers
         `path_normalisation_tests` and `dev_frontend_tests`. Does not
         require the SPA to be built.
      2. default features (embed-dist) — covers `path_normalisation_tests`
         and `embed_tests`. Requires `frontend/dist/index.html` because
         rust-embed reads the folder at compile time; when invoked via
         `mise run test:unit:visualiser`, `build:frontend` runs first.
    """
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(
        f"cargo test --manifest-path {manifest} --lib "
        f"--no-default-features --features dev-frontend"
    )
    context.run(f"cargo test --manifest-path {manifest} --lib")


@task
def frontend(context: Context):
    """Unit tests for the visualiser frontend (Vitest)."""
    frontend_root = repo_root() / "skills/visualisation/visualise/frontend"
    context.run(f"npm --prefix {frontend_root} run test")
