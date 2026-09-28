"""Documentation-site tasks (Astro Starlight in docs-site/)."""

import datetime as dt
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.npm_audit import (
    IgnoreListError,
    blocking_advisories,
    lapsed_ignores,
    parse_ignores,
    parse_report,
)
from tasks.shared.paths import DOCS_AUDIT_IGNORES, DOCS_SITE, REPO_ROOT
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
def audit_check(
    context: Context, ignores: str | None = None, today: str | None = None
) -> None:
    """Fail on high/critical npm advisories in the docs-site tree."""
    ignores_path = Path(ignores) if ignores else DOCS_AUDIT_IGNORES
    on = (
        dt.date.fromisoformat(today)
        if today
        else dt.datetime.now(tz=dt.UTC).date()
    )
    try:
        advisory_ignores = parse_ignores(ignores_path.read_text())
    except IgnoreListError as error:
        raise Exit(f"{ignores_path}: {error}", code=1) from error
    report = context.run(
        f"npm --prefix {DOCS_SITE} audit --json", warn=True, hide="out"
    )
    blocking = blocking_advisories(
        parse_report(report.stdout), advisory_ignores, on
    )
    lapsed = lapsed_ignores(advisory_ignores, on)
    if lapsed:
        raise Exit(
            f"advisory ignores in {ignores_path} past their review-by date — "
            "re-check whether upstream now offers a fix, then remove or "
            "re-date each: "
            + ", ".join(f"{i.ghsa_id} ({i.review_by})" for i in lapsed),
            code=1,
        )
    if blocking:
        raise Exit(
            "npm audit reported high or critical advisories: "
            + "; ".join(
                f"{a.ghsa_id} {a.package}: {a.title} ({a.severity})"
                for a in blocking
            )
            + " — run `mise run docs:audit:fix` for the non-breaking subset, "
            f"then resolve the rest by hand or, with no fix upstream, add a "
            f"dated ignore to {ignores_path}",
            code=1,
        )


@task
def audit_fix(context: Context) -> None:
    """Apply npm's non-breaking advisory fixes to the docs-site lockfile."""
    context.run(f"npm --prefix {DOCS_SITE} audit fix")


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
