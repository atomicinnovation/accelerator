from invoke import Context, task

from .helpers import repo_root, run_shell_suites


@task
def visualiser(context: Context):
    """Integration tests for the visualiser (cargo --tests + shell suites).

    The `spa_serving.rs` integration test is gated on the `dev-frontend`
    feature, so the cargo invocation enables that feature to include it.
    """
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(
        f"cargo test --manifest-path {manifest} --tests "
        f"--no-default-features --features dev-frontend"
    )
    run_shell_suites(context, "skills/visualisation/visualise")


@task
def config(context: Context):
    """Integration tests for the plugin-wide config scripts."""
    run_shell_suites(context, "scripts")


@task
def decisions(context: Context):
    """Integration tests for the decisions skill scripts."""
    run_shell_suites(context, "skills/decisions")


@task
def binary_acquisition(context: Context):
    """Test launch-server.sh binary acquisition paths (sentinel rejection, SHA mismatch, 404)."""
    script = repo_root() / "skills/visualisation/visualise/scripts/test-launch-server.sh"
    context.run(f"bash {script}")


@task
def hooks(context: Context):
    """Integration tests for the hooks/ subtree."""
    run_shell_suites(context, "hooks")
