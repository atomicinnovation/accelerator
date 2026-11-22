import re
from collections.abc import Sequence
from pathlib import Path

from invoke import Exit

from tasks.shared.selection import Selection

from .filters import node_tests_run

PLAYWRIGHT_DIR = "skills/design/inventory-design/scripts/playwright"


def tap_counts(output: str) -> dict[str, int]:
    """Parse the runner's own accounting, rather than proxying for it."""
    counts = {}
    for field in ("pass", "fail", "skipped"):
        found = re.search(rf"^# {field} (\d+)$", output, re.MULTILINE)
        if found is None:
            raise Exit(
                f"node --test emitted no '# {field}' summary line", code=1
            )
        counts[field] = int(found.group(1))
    return counts


def _test_callback_ranges(source: str) -> list[tuple[int, int]]:
    """Character ranges of every `test(...)`/`it(...)` callback body.

    Brace-matched from the callback's opening `{` to its match, so a helper
    defined at module level is outside every range — which is the whole point:
    a whole-file grep cannot tell a helper's legitimate early return from a test
    body silently reporting success.
    """
    ranges = []
    for opener in re.finditer(r"\b(?:test|it)\s*\(", source):
        arrow = source.find("=> {", opener.end())
        if arrow == -1:
            continue
        depth = 0
        for index in range(arrow + 3, len(source)):
            if source[index] == "{":
                depth += 1
            elif source[index] == "}":
                depth -= 1
                if depth == 0:
                    ranges.append((arrow + 3, index))
                    break
    return ranges


# A bare early return inside a test body reports as *passed*, not skipped, so a
# suite that gates itself on an absent runtime stays green while asserting
# nothing.
# Matches a return that yields nothing, wherever it sits on the line — the
# common shape is a guard clause (`if (!installed) return;`), not a statement
# alone on its own line.
_BARE_RETURN = re.compile(r"(?:^|[;{}\s)])return\s*(?:null\s*)?;")


def bare_returns_in_tests(source: str) -> list[str]:
    offenders = []
    for start, end in _test_callback_ranges(source):
        body = source[start:end]
        offset = source[:start].count("\n")
        for number, line in enumerate(body.split("\n"), start=offset + 1):
            # A test that calls the runner's own skip() is declaring itself
            # skipped, which the zero-skip assertion catches separately.
            if ".skip(" in line or "skip()" in line:
                continue
            if _BARE_RETURN.search(line):
                offenders.append(f"line {number}: {line.strip()}")
    return offenders


def refuse_filtered_run_that_ran_nothing(
    selection: Selection, mise_task: str, tap: str, suites: Sequence[Path]
) -> None:
    if node_tests_run(tap, suites) == 0:
        raise Exit(selection.nothing_selected(mise_task), code=1)
    print(
        "design-automation: pass-count and skip floors skipped for a "
        "filtered run"
    )
