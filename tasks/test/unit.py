from invoke import Context, task

from .helpers import repo_root


@task
def visualiser(context: Context):
    """Unit tests for the visualiser server (cargo --lib)."""
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(f"cargo test --manifest-path {manifest} --lib")
