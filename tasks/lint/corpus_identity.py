"""Guard: keep work-item identity out of the general-purpose corpus crates.

``cli/corpus`` and ``cli/corpus-adapters`` take work-item IDs through the
``IdConvention`` and ``LinkageIds`` ports; the scheme, its grammar and its
scanners live in the ``work`` crates. A corpus source naming one of those items
has pulled work-item identity back in.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

_CORPUS_SOURCES = ("cli/corpus/src", "cli/corpus-adapters/src")

_WORK_ITEM_IDENTITY = (
    "WorkItemIdScheme",
    "SchemeConvention",
    "IdScanner",
    "IdScan",
    "IdOwnership",
    "TRACKER_TOKEN",
    "canonical_draft_id",
    "references_key",
    "is_key_token",
    "is_tracker_key",
    "validate_id_pattern",
    "IdPatternError",
    "CanonicaliseError",
    "compile_scan_regex",
    "compile_format_string",
    "parse_full_id",
    "pattern_max_number",
    "canonicalise_id",
    "PatternCanonicaliser",
    "RegexScanner",
    "WorkItemIdCanonicaliser",
)

_NAMES = re.compile(r"\b(?:" + "|".join(_WORK_ITEM_IDENTITY) + r")\b")


def violations(root: Path) -> list[str]:
    """Repo-relative paths of the corpus sources naming work-item identity."""
    return sorted(
        path.relative_to(root).as_posix()
        for source in _CORPUS_SOURCES
        for path in (root / source).rglob("*.rs")
        if _NAMES.search(path.read_text())
    )


@task
def check(context: Context) -> None:
    """Fail if a corpus crate names a work-item identity item."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "work-item identity belongs in the work crates; the corpus crates "
            "take IDs through the IdConvention and LinkageIds ports:\n  "
            + "\n  ".join(offenders),
            code=1,
        )
