"""Producer-run frontmatter-validation coverage guard (0221 AC #6).

Every in-scope producer skill must run ``accelerator corpus frontmatter
validate`` on the document it wrote or edited, as its final persistence step.
The enforcement is producer-run (there is no CI conformance lane), so this
static check is the sole guarantee that coverage stays complete: each of the
closed in-scope set must carry the invocation *inside a fenced ``bash`` block*
— an executable step, not prose, a comment, or an ``allowed-tools`` glob — and
a discovery pass flags any frontmatter-emitting skill that is neither in-scope
nor explicitly out-of-scope. The bash-fence requirement is what gives the
static check its mutation-catching power across the skills' varied final-step
headings.

The discovery pass covers only frontmatter-*emitting* skills. Six in-scope
skills edit an existing document rather than emitting a fresh one
(``stress-test-*``, ``sync-work-items``, ``conduct-spike``, ``review-adr``,
``implement-plan``); they carry no emission marker, so they are held only by
the closed in-scope list, while the discovery pass guards against a new
*emitter* being added without the step.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

INVOCATION = "accelerator corpus frontmatter validate"

# The closed in-scope producer set (21). Each must run the validate step.
IN_SCOPE_PRODUCERS: tuple[str, ...] = (
    "skills/work/create-work-item/SKILL.md",
    "skills/work/extract-work-items/SKILL.md",
    "skills/work/refine-work-item/SKILL.md",
    "skills/work/update-work-item/SKILL.md",
    "skills/work/stress-test-work-item/SKILL.md",
    "skills/work/sync-work-items/SKILL.md",
    "skills/work/review-work-item/SKILL.md",
    "skills/research/conduct-spike/SKILL.md",
    "skills/research/research-codebase/SKILL.md",
    "skills/research/research-issue/SKILL.md",
    "skills/decisions/create-adr/SKILL.md",
    "skills/decisions/extract-adrs/SKILL.md",
    "skills/decisions/review-adr/SKILL.md",
    "skills/planning/create-plan/SKILL.md",
    "skills/planning/review-plan/SKILL.md",
    "skills/planning/validate-plan/SKILL.md",
    "skills/planning/stress-test-plan/SKILL.md",
    "skills/planning/implement-plan/SKILL.md",
    "skills/notes/create-note/SKILL.md",
    "skills/design/inventory-design/SKILL.md",
    "skills/design/analyse-design-gaps/SKILL.md",
)

# Frontmatter-emitting skills that are deliberately out of scope: the PR skills
# write no meta/ artefact, and list-work-items is read-only.
OUT_OF_SCOPE_EMITTERS: tuple[str, ...] = (
    "skills/github/describe-pr/SKILL.md",
    "skills/github/review-pr/SKILL.md",
    "skills/work/list-work-items/SKILL.md",
)

_BASH_FENCE = re.compile(r"^[ \t]*```\s*(bash|sh|console)\b")

# Markers that identify a frontmatter-emitting skill, for the discovery pass.
_EMISSION_MARKERS: tuple[re.Pattern[str], ...] = (
    re.compile(r"accelerator config template "),
    re.compile(r"^[ \t]*producer:"),
    re.compile(r"^[ \t]*schema_version:"),
    re.compile(r"^[ \t]*verdict:"),
    re.compile(r"^[ \t]*review_pass:"),
    re.compile(r"^[ \t]*target:"),
)


def validate_step_in_a_bash_fence(text: str) -> bool:
    """The validate invocation appears inside a fenced ``bash`` block."""
    in_bash_fence = False
    fence_has_invocation = False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            if in_bash_fence:
                if fence_has_invocation:
                    return True
                in_bash_fence = False
            else:
                in_bash_fence = bool(_BASH_FENCE.match(line))
                fence_has_invocation = False
            continue
        if in_bash_fence and INVOCATION in line:
            fence_has_invocation = True
    return False


def _emits_frontmatter(text: str) -> bool:
    return any(
        marker.search(line)
        for marker in _EMISSION_MARKERS
        for line in text.splitlines()
    )


def _discovered_emitters(root: Path) -> set[str]:
    discovered: set[str] = set()
    for path in (root / "skills").rglob("SKILL.md"):
        if _emits_frontmatter(path.read_text()):
            discovered.add(path.relative_to(root).as_posix())
    return discovered


def coverage_violations(root: Path) -> list[str]:
    found: list[str] = []
    for skill in IN_SCOPE_PRODUCERS:
        path = root / skill
        if not path.is_file():
            found.append(f"{skill}: SKILL.md not found")
            continue
        if not validate_step_in_a_bash_fence(path.read_text()):
            found.append(
                f"{skill}: no '{INVOCATION}' invocation in a fenced block "
                "within a persistence section"
            )
    allowlist = set(IN_SCOPE_PRODUCERS) | set(OUT_OF_SCOPE_EMITTERS)
    found.extend(
        f"{skill}: frontmatter-emitting skill neither in-scope nor "
        "listed out-of-scope"
        for skill in sorted(_discovered_emitters(root) - allowlist)
    )
    return found


# --------------------------------------------------------------------------
# Synthetic branch tests.
# --------------------------------------------------------------------------


def test_an_invocation_in_a_bash_fence_is_accepted() -> None:
    body = "```bash\naccelerator corpus frontmatter validate --file x.md\n```\n"
    assert validate_step_in_a_bash_fence(body)


def test_an_invocation_outside_a_fence_is_rejected() -> None:
    body = "Run accelerator corpus frontmatter validate --file x.md.\n"
    assert not validate_step_in_a_bash_fence(body)


def test_an_invocation_in_a_non_bash_fence_is_rejected() -> None:
    body = "```yaml\naccelerator corpus frontmatter validate --file x.md\n```\n"
    assert not validate_step_in_a_bash_fence(body)


def test_an_invocation_in_an_allowed_tools_glob_is_rejected() -> None:
    body = "  - Bash(accelerator corpus frontmatter validate *)\n"
    assert not validate_step_in_a_bash_fence(body)


def test_a_missing_skill_is_flagged(tmp_path: Path) -> None:
    assert any("SKILL.md not found" in v for v in coverage_violations(tmp_path))


def test_a_discovered_emitter_outside_the_allowlist_is_flagged(
    tmp_path: Path,
) -> None:
    rogue = tmp_path / "skills/rogue/new/SKILL.md"
    rogue.parent.mkdir(parents=True, exist_ok=True)
    rogue.write_text("```yaml\nproducer: new\n```\n")
    assert any("neither in-scope" in v for v in coverage_violations(tmp_path))


# --------------------------------------------------------------------------
# Live tree.
# --------------------------------------------------------------------------


def test_the_real_skills_tree_has_full_coverage() -> None:
    assert coverage_violations(REPO_ROOT) == []
