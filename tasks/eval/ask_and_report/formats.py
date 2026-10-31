"""The fixed lines research-topic's ask and report verbs print.

Each template is written as SKILL.md writes it, so one string serves both
the structure tests that find it in SKILL.md and the judges that match it.
"""

import re
from typing import Literal

type Offer = Literal["accept", "pick", "proposal", "reject"]
type RefusalGate = Literal["status", "freshness", "conducted"]

STATUS_REFUSAL = (
    "{verb} refused: {slug} is {status}; it needs synthesised or complete. "
    "{recovery}"
)
STATUS_RECOVERIES = {
    "briefed": (
        "Run outline {slug}, then conduct {slug}, then synthesise {slug}."
    ),
    "outlined": "Run conduct {slug}, then synthesise {slug}.",
    "researching": (
        "Run conduct {slug} to finish any outstanding research, then "
        "synthesise {slug}."
    ),
}
FRESHNESS_REFUSAL = (
    "{verb} refused: {slug}'s synthesis is stale: {mismatches}. "
    "Run conduct {slug}, then synthesise {slug}."
)
CONDUCTED_REFUSAL = "{verb} refused: {slug} is not fully conducted."
HELD_BACK = (
    "Held back by a profile the brief does not name or no skill installs:"
)
HELD_BACK_ITEM = "- {question}: {reasons}"
HELD_BACK_RECOVERY = (
    "Add the profile to the brief's `source_profiles`, or fix its name in "
    "the outline."
)
STILL_TO_RESEARCH = "Still to research:"
UNANSWERED_ITEM = "- {question} ({profile})"
CONDUCT_THEN_SYNTHESISE = "Run conduct {slug}, then synthesise {slug}."
REFUSALS: dict[RefusalGate, str] = {
    "status": STATUS_REFUSAL,
    "freshness": FRESHNESS_REFUSAL,
    "conducted": CONDUCTED_REFUSAL,
}

COVERAGE = "Coverage: {answered|partial|unanswered}"
COVERED = "Covered:"
GAPS = "Gaps:"
COVERAGE_ITEM = "- {part}: {source} ({tier text}), {source} ({tier text})"
UNSUPPORTED_GAP = "- {part}: no source"

EQUIVALENT_REPORTS = "Equivalent reports:"
EQUIVALENT_REPORT = "{n}. reports/{file}.md — {question}"

PROPOSED_ROUND = "Proposed Round {N+1}:"
PROPOSED_ITEM = "- [ ] {question} — profiles: {profile}, {profile}"

PROMPTS: dict[Offer, str] = {
    "accept": (
        "Reply accept to write reports/{new}.md, reject to propose research "
        "instead, or decline to write nothing."
    ),
    "pick": (
        "Reply rewrite 1 (or another listed number) to rewrite that report "
        "in place, new to write reports/{new}.md, reject to propose research "
        "instead, or decline to write nothing."
    ),
    "proposal": (
        "Reply confirm to append it to outline.md and reopen {slug}, or "
        "decline."
    ),
    "reject": (
        "Reply reject to propose research if this does not answer your "
        "question."
    ),
}

APPENDED_ROUND = (
    "Appended Round {N+1} and reopened {slug} to researching. Next: run "
    "conduct {slug}, then synthesise {slug}, then {verb} {slug} {question} "
    "again."
)
WROTE_REPORT = "Wrote reports/{file}.md (coverage {coverage})."

_PLACEHOLDER = re.compile(r"\{([^{}]+)\}")
_LIST = re.compile(r"\{([^{}]+)\}(?:, \{\1\})+")
_FENCE = re.compile(r"^(```|~~~)")


def normalised(line: str) -> str:
    """Read a line as its text, without Markdown emphasis, code or quoting."""
    line = line.strip()
    while line.startswith(">"):
        line = line[1:].lstrip()
    for marker in ("**", "__", "`"):
        line = line.replace(marker, "")
    if line.startswith("* "):
        line = "- " + line[2:]
    return re.sub(r"\*(\S(?:[^*]*\S)?)\*", r"\1", line)


def lines(text: str) -> list[str]:
    """Every non-empty line of ``text``, normalised, with fences dropped."""
    return [
        normalised(line)
        for line in text.splitlines()
        if line.strip() and not _FENCE.match(line.strip())
    ]


def _group(placeholder: str) -> str:
    name = re.sub(r"\W+", "_", placeholder).strip("_")
    return name if name[:1].isalpha() else f"p_{name}"


def pattern(template: str, **known: str) -> re.Pattern[str]:
    """Match a line filled in from ``template``, its ``known`` values fixed.

    Every other placeholder captures what fills it, under its name with
    non-word characters as ``_``. A placeholder repeated as a comma list
    captures a list of one or more, and one named twice must repeat.
    """
    template = _LIST.sub(lambda found: "{" + found[1] + "}", template)
    parts: list[str] = []
    seen: set[str] = set()
    at = 0
    template = normalised(template)
    for found in _PLACEHOLDER.finditer(template):
        parts.append(re.escape(template[at : found.start()]))
        name = _group(found[1])
        if name in known:
            parts.append(re.escape(known[name]))
        elif name in seen:
            parts.append(f"(?P={name})")
        else:
            choices = found[1].split("|")
            body = (
                "|".join(map(re.escape, choices)) if len(choices) > 1 else ".+?"
            )
            parts.append(f"(?P<{name}>{body})")
            seen.add(name)
        at = found.end()
    parts.append(re.escape(template[at:]))
    return re.compile("".join(parts))


def matches(template: str, line: str, **known: str) -> re.Match[str] | None:
    return pattern(template, **known).fullmatch(normalised(line))
