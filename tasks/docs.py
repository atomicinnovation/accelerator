"""Documentation-site tasks (Astro Starlight in docs-site/)."""

from pathlib import Path

from invoke import Context, task

from tasks.shared.paths import DOCS_SITE, REPO_ROOT
from tasks.shared.skill_pages import generate_pages


@task
def build(context: Context) -> None:
    """Build the documentation site (strict: link validation fails it)."""
    context.run(f"npm --prefix {DOCS_SITE} run build")


@task
def serve(context: Context) -> None:
    """Serve the documentation site with live reload."""
    context.run(f"npm --prefix {DOCS_SITE} run dev", pty=True)


@task
def generate(context: Context, repo_root: str | None = None) -> None:
    """Generate per-skill reference pages from SKILL.md sources."""
    root = Path(repo_root) if repo_root else REPO_ROOT
    written = generate_pages(root)
    print(f"generated {len(written)} skill pages + index")
