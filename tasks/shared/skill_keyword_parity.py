"""Doc-vs-binary keyword parity for the repointed integration skills.

The jira/linear write and read bodies branch on a structured stdout *keyword*
rather than a bash exit integer. This guard binds the two
representations: every outcome keyword a repointed ``SKILL.md`` body branches on
must exist in the sub-binary's own declared keyword set (parsed from its
``keywords.rs``), the match must be non-vacuous (an under-matching extractor
cannot pass silently), and no body may reintroduce a bash exit integer in an
outcome branch or a dropped ``--print-payload``/``--describe`` preview.

The binary's ``keywords.rs`` is the authority: a keyword renamed there without
the body following it, or a stale keyword left in a body, fails here.
"""

import re
from pathlib import Path

from tasks.shared.paths import REPO_ROOT

# The repointed bodies and the crate whose keyword set is their authority.
_PROVIDERS: dict[str, Path] = {
    "linear": REPO_ROOT / "cli/linear-cli/src/keywords.rs",
    "jira": REPO_ROOT / "cli/jira-cli/src/keywords.rs",
}

# A keyword literal in a `keyword(self)` projection: `=> "created",`.
_KEYWORD_LITERAL = re.compile(r'=>\s*"([a-z][a-z0-9-]*)"')

# A backticked token on a branch-cue line is an intended keyword reference. The
# cues are the shapes the bodies actually use: "outcome is `x`", "the `x`
# keyword", "On `x`,", and a bullet "- **`x`**".
_BRANCH_CUE = re.compile(r"outcome|keyword|^\s*-\s+\*\*`|On `")
_BACKTICKED = re.compile(r"`([a-z][a-z0-9-]*)`")

# Bash exit integers must not reappear in an outcome branch.
_EXIT_INTEGER = re.compile(r"\b[Ee]xits?\s+`?\d{1,3}`?")
_DROPPED_PREVIEW = re.compile(r"--print-payload|--describe")

# Tokens that are backticked and lower-kebab but are never outcome keywords, so
# a branch-cue line mentioning one is not a keyword reference to check.
_NOT_KEYWORDS = frozenset(
    {"outcome", "y", "n", "external_id", "init-linear", "init-jira"}
)


def declared_keywords(keywords_rs: Path) -> set[str]:
    """Return the keyword literals a crate's ``keywords.rs`` projects."""
    return set(_KEYWORD_LITERAL.findall(keywords_rs.read_text()))


def _bodies(provider: str) -> list[Path]:
    root = REPO_ROOT / "skills/integrations" / provider
    return sorted(root.glob("*/SKILL.md"))


def referenced_keywords(text: str) -> set[str]:
    """Return the outcome keywords a body branches on, from its cue lines."""
    referenced: set[str] = set()
    for line in text.splitlines():
        if not _BRANCH_CUE.search(line):
            continue
        for token in _BACKTICKED.findall(line):
            if token not in _NOT_KEYWORDS:
                referenced.add(token)
    return referenced


def violations() -> list[str]:
    """Every doc-vs-binary keyword parity breach, one message per breach."""
    problems: list[str] = []
    total_matches = 0
    for provider, keywords_rs in _PROVIDERS.items():
        declared = declared_keywords(keywords_rs)
        for body in _bodies(provider):
            text = body.read_text()
            rel = body.relative_to(REPO_ROOT)
            for referenced in sorted(referenced_keywords(text)):
                if referenced in declared:
                    total_matches += 1
                else:
                    problems.append(
                        f"{rel}: branches on keyword '{referenced}' not in "
                        f"{keywords_rs.relative_to(REPO_ROOT)}'s declared set "
                        f"{sorted(declared)}"
                    )
            exit_match = _EXIT_INTEGER.search(text)
            if exit_match:
                problems.append(
                    f"{rel}: cites a bash exit integer "
                    f"('{exit_match.group(0).strip()}') — branch on the keyword"
                )
            if _DROPPED_PREVIEW.search(text):
                problems.append(
                    f"{rel}: references a dropped --print-payload/--describe "
                    "preview"
                )
    if total_matches == 0:
        problems.append(
            "anti-vacuity: no repointed body branched on any declared "
            "keyword — the extractor matched nothing, so the guard is vacuous"
        )
    return problems
