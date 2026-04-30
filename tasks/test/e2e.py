from invoke import Context, task

from .helpers import repo_root


@task
def visualiser(context: Context):
    """E2E tests for the visualiser (Playwright).

    Requires build.frontend and build.server to have run first.
    When invoked via `mise run test:e2e:visualiser`, those build tasks
    are declared as dependencies and run automatically.
    """
    frontend_root = repo_root() / "skills/visualisation/visualise/frontend"
    server_bin = (
        repo_root()
        / "skills/visualisation/visualise/server/target/debug/accelerator-visualiser"
    )
    context.run(
        f"npm --prefix {frontend_root} run test:e2e",
        env={"ACCELERATOR_VISUALISER_BIN": str(server_bin)},
    )
