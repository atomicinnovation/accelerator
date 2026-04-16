from invoke import task, Context

from . import version


@task
def configure(
    context: Context,
    user_name: str = "Atomic Maintainers",
    user_email: str = "maintainers@go-atomic.io"
):
    """Configure git settings for the project."""
    context.run(f"git config --local user.name '{user_name}'")
    context.run(f"git config --local user.email '{user_email}'")


@task
def pull(context: Context):
    """Ensure current branch up to date with remote."""
    context.run("git pull")

@task
def push(context: Context):
    """Push the current branch to remote."""
    context.run(f"git push origin HEAD --tags")

@task
def tag_version(context: Context):
    """Tag the current git commit with the current project version."""
    current_version = version.read(context, print_to_stdout=False)
    context.run(f"git tag -a 'v{current_version}' -m 'Release version {current_version}'")

@task
def commit_version(context: Context):
    """Commit changes with a version bump message."""
    current_version = version.read(context, print_to_stdout=False)
    context.run("git add .claude-plugin/plugin.json")
    context.run(f"git commit -m 'Bump version to {current_version} [skip ci]'")
