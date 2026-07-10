"""Documentation-site tasks (Astro Starlight in docs-site/)."""

from invoke import Context, task

from tasks.shared.paths import DOCS_SITE


@task
def build(context: Context) -> None:
    """Build the documentation site (strict: link validation fails it)."""
    context.run(f"npm --prefix {DOCS_SITE} run build")


@task
def serve(context: Context) -> None:
    """Serve the documentation site with live reload."""
    context.run(f"npm --prefix {DOCS_SITE} run dev", pty=True)
