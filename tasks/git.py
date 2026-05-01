from invoke import task, Context

from . import version


@task
def check_clean(context: Context):
    """Abort if the working tree has any uncommitted changes."""
    result = context.run("git status --porcelain", hide=True, warn=True)
    if result.stdout.strip():
        raise RuntimeError(
            f"working tree is not clean; commit or stash changes before releasing:\n"
            f"{result.stdout.strip()}"
        )


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
def tag_version(context: Context, target_version: str | None = None):
    """Tag the current git commit with the current project version."""
    resolved_version = target_version or version.read(context, print_to_stdout=False)
    context.run(f"git tag -a 'v{resolved_version}' -m 'Release version {resolved_version}'")

@task
def commit_version(context: Context, target_version: str | None = None):
    """Commit changes with a version bump message."""
    resolved_version = target_version or version.read(context, print_to_stdout=False)
    context.run("git add .")
    context.run(f"git commit -m 'Bump version to {resolved_version} [skip ci]'")
