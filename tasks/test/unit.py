from invoke import Context, Exit, task

from .helpers import repo_root


@task
def visualiser(context: Context) -> None:
    """Run the visualiser server unit tests.

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
def frontend(context: Context) -> None:
    """Run the visualiser frontend unit tests (Vitest)."""
    frontend_root = repo_root() / "skills/visualisation/visualise/frontend"
    context.run(f"npm --prefix {frontend_root} run test")


@task
def templates(context: Context) -> None:
    """Run template / SKILL / metadata-helper schema tests."""
    drivers = [
        "scripts/test-template-frontmatter.sh",
        "scripts/test-skill-frontmatter-population.sh",
        "scripts/test-metadata-helpers.sh",
    ]
    failures: list[str] = []
    for driver in drivers:
        result = context.run(f"bash {driver}", warn=True, pty=False)
        if result.exited != 0:
            failures.append(driver)
    if failures:
        raise Exit(
            f"Template schema tests failed: {', '.join(failures)}", code=1
        )
