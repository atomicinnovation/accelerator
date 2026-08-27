"""Confirm-before-mutation gate for the repointed integration write skills.

The write skills preview resolved intent rather than a wire payload, gated by an
explicit confirm step. This guard pins the ordering the design depends on:
in every write skill that invokes an ``accelerator <provider> <mutation>``
subcommand, a confirm prompt must be **present** and lexically **precede** the
mutation invocation, and the two must not sit in the same fenced block. A
missing confirm fails (so ordering cannot pass vacuously), and a committed
reversed-body fixture proves the guard fails when the mutation comes first.
"""

import re

from tasks.shared.paths import REPO_ROOT

# The mutating subcommands per provider — the read/init verbs (show, search,
# init) are excluded, so a read skill invoking the launcher is not gated.
_MUTATION_VERBS: dict[str, frozenset[str]] = {
    "linear": frozenset(
        {"create", "update", "comment", "transition", "attach"}
    ),
}

_CONFIRM = re.compile(r"to confirm", re.IGNORECASE)
_FENCE = re.compile(r"^```", re.MULTILINE)


def _mutation_offset(text: str, provider: str) -> int | None:
    """Return the offset of the first ``accelerator <provider> <mutation>``."""
    verbs = "|".join(sorted(_MUTATION_VERBS[provider]))
    pattern = re.compile(rf"/bin/accelerator {provider} (?:{verbs})\b")
    match = pattern.search(text)
    return match.start() if match else None


def _fenced_spans(text: str) -> list[tuple[int, int]]:
    """Return the (start, end) offsets of each fenced block's interior."""
    fences = [match.start() for match in _FENCE.finditer(text)]
    return [(fences[i], fences[i + 1]) for i in range(0, len(fences) - 1, 2)]


def body_violations(text: str, provider: str, label: str) -> list[str] | None:
    """Confirm-before-mutation breaches in one body, or ``None`` if not gated.

    ``None`` means the body invokes no ``accelerator <provider> <mutation>`` —
    a read/init skill — so it is out of the gate's scope. An empty list means a
    gated body that passes.
    """
    mutation = _mutation_offset(text, provider)
    if mutation is None:
        return None
    problems: list[str] = []
    before = [
        m.start() for m in _CONFIRM.finditer(text) if m.start() < mutation
    ]
    if not before:
        problems.append(
            f"{label}: no confirm step precedes the "
            f"accelerator {provider} mutation invocation"
        )
        return problems
    for start, end in _fenced_spans(text):
        if start < mutation < end and any(
            start < offset < end for offset in before
        ):
            problems.append(
                f"{label}: the confirm step and the mutation share a "
                "fenced block"
            )
    return problems


def violations() -> list[str]:
    """Every confirm-before-mutation breach across the write skills."""
    problems: list[str] = []
    checked = 0
    for provider in _MUTATION_VERBS:
        root = REPO_ROOT / "skills/integrations" / provider
        for body in sorted(root.glob("*/SKILL.md")):
            found = body_violations(
                body.read_text(), provider, str(body.relative_to(REPO_ROOT))
            )
            if found is None:
                continue  # a read/init skill — not gated
            checked += 1
            problems.extend(found)
    if checked == 0:
        problems.append(
            "anti-vacuity: no write skill was checked — the mutation matcher "
            "found nothing"
        )
    return problems
