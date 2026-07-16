"""Documentation-site tasks (Astro Starlight in docs-site/)."""

from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import DOCS_SITE, REPO_ROOT
from tasks.shared.skill_pages import (
    DOCS_GENERATED_RELATIVE,
    discover_skills,
    generate_pages,
    output_path,
)


@task
def build(context: Context) -> None:
    """Build the documentation site (strict: link validation fails it)."""
    context.run(f"npm --prefix {DOCS_SITE} run build")


@task
def serve(context: Context) -> None:
    """Serve the documentation site with live reload."""
    context.run(f"npm --prefix {DOCS_SITE} run dev", pty=True)


@task
def preview(context: Context) -> None:
    """Serve the built documentation site from docs-site/dist/."""
    context.run(f"npm --prefix {DOCS_SITE} run preview", pty=True)


@task
def audit(context: Context) -> None:
    """Fail on high/critical npm advisories in the docs-site tree."""
    context.run(f"npm --prefix {DOCS_SITE} audit --audit-level=high")


@task
def generate(context: Context, repo_root: str | None = None) -> None:
    """Generate per-skill reference pages from SKILL.md sources."""
    root = Path(repo_root) if repo_root else REPO_ROOT
    written = generate_pages(root)
    print(f"generated {len(written)} skill pages + index")


@task
def generate_check(context: Context, repo_root: str | None = None) -> None:
    """Verify generated pages match the plugin.json skill registry.

    Every skill discovered via the plugin.json globs must have a generated
    page, and no orphan page may exist for an unregistered skill.
    """
    root = Path(repo_root) if repo_root else REPO_ROOT
    generated_dir = root / DOCS_GENERATED_RELATIVE
    expected = {
        output_path(page, generated_dir): page.name
        for page in discover_skills(root)
    }
    index = generated_dir / "index.md"
    actual = (
        {p for p in generated_dir.rglob("*.md") if p != index}
        if generated_dir.is_dir()
        else set()
    )

    problems = [
        f"missing page for skill '{name}': {path.relative_to(root).as_posix()}"
        for path, name in sorted(expected.items())
        if path not in actual
    ]
    problems += [
        f"orphan page with no registered skill: "
        f"{path.relative_to(root).as_posix()}"
        for path in sorted(actual - expected.keys())
    ]
    if problems:
        listing = "\n".join(f"  - {p}" for p in problems)
        raise Exit(
            "generated docs pages are out of sync with plugin.json "
            f"skills:\n{listing}\nRe-run `mise run docs:generate`.",
            code=1,
        )
    print(f"docs coverage OK: {len(expected)} skills, {len(actual)} pages")
