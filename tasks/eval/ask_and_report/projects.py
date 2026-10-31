"""Lay out the reference corpus as the project a scenario's session meets."""

import shutil
from pathlib import Path

from tasks.eval.ask_and_report.layout import SET
from tasks.eval.ask_and_report.scenarios import Setup, TemplateTier
from tasks.shared.sources import repo_root

CONFIG = Path(".accelerator/config.md")
REPORT_TEMPLATE = "topic-research-report"
OVERRIDES: dict[TemplateTier, Path] = {
    "user": Path(".accelerator/templates") / f"{REPORT_TEMPLATE}.md",
    "config": Path(".accelerator/report-template.md"),
}
OVERRIDE_SECTIONS: dict[TemplateTier, str] = {
    "user": "Method",
    "config": "Scope",
}


def _override(tier: TemplateTier) -> str:
    plugin = (repo_root() / "templates" / f"{REPORT_TEMPLATE}.md").read_text()
    section = OVERRIDE_SECTIONS[tier]
    inserted = f"## {section}\n[The {section.lower()} of this report.]\n\n"
    return plugin.replace("## Sources\n", inserted + "## Sources\n", 1)


def _config(setup: Setup) -> str:
    lines = []
    if setup.breadth is not None:
        lines += ["research:", "  topic:", f"    breadth: {setup.breadth}"]
    if "config" in setup.templates:
        lines += ["templates:", f"  {REPORT_TEMPLATE}: {OVERRIDES['config']}"]
    return (
        "".join(f"{line}\n" for line in ["---", *lines, "---"]) if lines else ""
    )


def build_project(setup: Setup, root: Path, evals: Path) -> Path:
    """Lay the scenario's project out under ``root`` and return its set."""
    shutil.copytree(evals / "project", root, dirs_exist_ok=True)
    if setup.variant != "base":
        shutil.copytree(
            evals / "variants" / setup.variant, root, dirs_exist_ok=True
        )
    for mutation in setup.mutations:
        mutation.apply(root / SET)
    for tier in setup.templates:
        override = root / OVERRIDES[tier]
        override.parent.mkdir(parents=True, exist_ok=True)
        override.write_text(_override(tier))
    if config := _config(setup):
        (root / CONFIG).parent.mkdir(parents=True, exist_ok=True)
        (root / CONFIG).write_text(config)
    return root / SET
