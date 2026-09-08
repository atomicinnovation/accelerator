import os

from invoke import Context, task

from . import version


@task
def check_clean(context: Context) -> None:
    """Abort if the working tree has any uncommitted changes."""
    result = context.run("git status --porcelain", hide=True, warn=True)
    if result.stdout.strip():
        raise RuntimeError(
            f"working tree is not clean; commit or stash changes before "
            f"releasing:\n"
            f"{result.stdout.strip()}"
        )


@task
def configure(
    context: Context,
    user_name: str = "Atomic Maintainers",
    user_email: str = "maintainers@go-atomic.io",
) -> None:
    """Configure git settings for the project."""
    context.run(f"git config --local user.name '{user_name}'")
    context.run(f"git config --local user.email '{user_email}'")


@task
def pull(context: Context) -> None:
    """Ensure current branch up to date with remote."""
    context.run("git pull")


@task
def push(context: Context, target_version: str | None = None) -> None:
    """Push the current branch and its version tag to remote, atomically.

    --atomic makes the branch ref and the tag ref a single all-or-nothing
    update: if the branch push is rejected (e.g. main advanced on the remote
    since this pipeline pulled), the tag is NOT pushed either. Without this a
    rejected branch push still leaves the tag behind, orphaning it on a commit
    that never reached main and wedging the release. We push the one explicit
    version tag rather than --tags so the result never depends on which tags
    happen to be present in the runner's checkout.

    Under CI the push authenticates with a GitHub App token supplied in
    RELEASER_TOKEN, minted immediately beforehand. The checkout credential
    cannot serve here: a GitHub App token lives 60 minutes and the four-target
    cross-compile in *:prepare routinely outlives it, so by finalise the
    persisted credential is dead. The token reaches git through the remote URL,
    left unexpanded in the command so the shell substitutes it at push time and
    the secret never enters this process's arguments. Local dev has no
    RELEASER_TOKEN and pushes through the developer's own `origin` credential.
    """
    resolved_version = target_version or version.read(
        context, print_to_stdout=False
    )
    tag_ref = f"refs/tags/v{resolved_version}"
    if os.environ.get("RELEASER_TOKEN"):
        origin = context.run(
            "git remote get-url origin", hide=True
        ).stdout.strip()
        remote = origin.replace(
            "https://", "https://x-access-token:${RELEASER_TOKEN}@", 1
        )
        context.run(f"git push --atomic {remote} HEAD '{tag_ref}'")
    else:
        context.run(f"git push --atomic origin HEAD '{tag_ref}'")


@task
def tag_version(context: Context, target_version: str | None = None) -> None:
    """Tag the current git commit with the current project version."""
    resolved_version = target_version or version.read(
        context, print_to_stdout=False
    )
    context.run(
        f"git tag -a 'v{resolved_version}' "
        f"-m 'Release version {resolved_version}'"
    )


@task
def commit_version(context: Context, target_version: str | None = None) -> None:
    """Commit changes with a version bump message."""
    resolved_version = target_version or version.read(
        context, print_to_stdout=False
    )
    context.run("git add .")
    context.run(f"git commit -m 'Bump version to {resolved_version} [skip ci]'")
