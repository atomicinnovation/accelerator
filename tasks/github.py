from invoke import task, Context

from . import version


@task
def create_release(context: Context, target_version: str | None = None):
    """Create a release on GitHub."""
    resolved_version = target_version or version.read(context, print_to_stdout=False)
    context.run(
        f'gh release create "{resolved_version}" --generate-notes --title "{resolved_version}"',
        pty=True,
    )
