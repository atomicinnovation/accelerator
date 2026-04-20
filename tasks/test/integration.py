from invoke import Context, task

from .helpers import repo_root, run_shell_suites


@task
def visualiser(context: Context):
    """Integration tests for the visualiser (cargo --tests + shell suites)."""
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(f"cargo test --manifest-path {manifest} --tests")
    run_shell_suites(context, "skills/visualisation/visualise")


@task
def config(context: Context):
    """Integration tests for the plugin-wide config scripts."""
    run_shell_suites(context, "scripts")


@task
def decisions(context: Context):
    """Integration tests for the decisions skill scripts."""
    run_shell_suites(context, "skills/decisions")
