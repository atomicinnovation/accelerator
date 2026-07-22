"""Guard against regressing to the bash config cluster (ADR-0048 guardrail).

The 0167 removal set is deleted and `accelerator config` is the only config
reader in the product. Two anti-regression guards remain, both cheap:

* **Grep B** — no ``skills/**/SKILL.md`` names a ``scripts/config-`` script;
  a reintroduced bash config call site would prompt at load and bypass the
  launcher contract. The retained ``config-read-browser-executor.sh`` (0173)
  and ``config-common.sh`` (0174) are permitted.
* **``--allow-legacy-layout`` confinement** — the flag stays inside the
  migration engine (``skills/config/migrate/migrations/`` and the allowlisted
  ``doc-type-table.sh``); anywhere else it would silently suppress the uniform
  legacy-layout refusal.

The functional-invocation census (Grep A) that proved the migration landed is
retired with the removal set it enumerated.
"""

from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root


def grep_b_hits(root: Path) -> list[str]:
    """Return SKILL.md lines naming a removed ``scripts/config-`` script.

    The browser executor (0173) and config-common (0174) are permitted.
    """
    hits: list[str] = []
    allowed = ("config-read-browser-executor.sh", "config-common.sh")
    for path in sorted((root / "skills").rglob("SKILL.md")):
        rel = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            if "scripts/config-" in line and not any(
                a in line for a in allowed
            ):
                hits.append(f"{rel}:{number}:{line.strip()}")
    return hits


def stray_legacy_flag(root: Path) -> list[str]:
    """Return ``*.sh`` files naming ``--allow-legacy-layout`` out of bounds.

    Scans ``skills/`` and ``scripts/``; the migration engine (migrations/ and
    the allowlisted ``doc-type-table.sh``), tests, and this gate are permitted.
    """
    stray: list[str] = []
    for base in ("skills", "scripts"):
        for path in sorted((root / base).rglob("*.sh")):
            rel = path.relative_to(root).as_posix()
            if "allow-legacy-layout" not in path.read_text():
                continue
            if rel.startswith("skills/config/migrate/migrations/"):
                continue
            if rel == "scripts/doc-type-table.sh":
                continue
            name = Path(rel).name
            if (
                name.startswith("test-")
                or name == "check-call-site-migration.sh"
            ):
                continue
            stray.append(rel)
    return stray


def violations(root: Path) -> list[str]:
    """Every gated failure across Grep B and the legacy-flag confinement."""
    found: list[str] = []
    found.extend(
        f"Grep B: SKILL.md names a removal-set script: {hit}"
        for hit in grep_b_hits(root)
    )
    found.extend(
        f"--allow-legacy-layout outside the migration engine: {rel}"
        for rel in stray_legacy_flag(root)
    )
    return found


@task
def check(context: Context) -> None:
    """Fail on any SKILL.md config-script reference or stray legacy flag."""
    root = repo_root()
    offenders = violations(root)
    if offenders:
        raise Exit(
            "check-call-site-migration found violation(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
