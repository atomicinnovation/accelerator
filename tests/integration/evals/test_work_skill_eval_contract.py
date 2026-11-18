"""The CLI output the work skills' evals quote, held to the CLI.

An eval prompt simulates an ``accelerator work`` command by quoting what it
prints. Each refusal code a prompt quotes must exit as the built binary
exits that refusal, read from the nearest exit status the prompt gives
before it, each refusal or warning a prompt quotes as a block must read as
a line the binary prints for its code, up to the paths, quoted values and
numbers in it, and each quoted verdict must have the fields the binary
prints for its kind, so an eval cannot keep passing against output the CLI
no longer gives.

``NOT_ELICITED`` names the codes this lane does not draw from the binary,
since eliciting them needs a live identity-run lock, a re-key run plan, a
push marker in the serialiser's own format, or a batch create. Every other
quoted code is elicited, so a code in neither fails the lane.
"""

import json
import os
import re
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
WORK = Path(
    os.environ.get(
        "ACCELERATOR_WORK_BIN", REPO_ROOT / "cli/target/debug/accelerator-work"
    )
)
SKILLS = (
    "create-work-item",
    "update-work-item",
    "review-work-item",
    "refine-work-item",
    "extract-work-items",
    "list-work-items",
    "sync-work-items",
)
EXIT_STATUS = re.compile(r"\bexit(?:s|ing)? (\d+)\b")
REFUSAL_CODE = re.compile(r"\bE_[A-Z][A-Z_]*[A-Z]\b")
NOT_ELICITED = frozenset(
    {
        "E_IDENTITY_RUN_BUSY",
        "E_REKEY_IN_PROGRESS",
        "E_PUSH_PENDING",
        "E_PUSH_MARKER_UNREADABLE",
        "E_PUSH_FINGERPRINT_MISMATCH",
        "E_DRAFT_EXISTS",
    }
)
VERDICT_KINDS = ("exact", "other-key", "corrected")
QUOTED_BLOCK = re.compile(
    r"(?m)^(Error: |warning: )?(E_[A-Z][A-Z_]*[A-Z]): [^\n]*(?:\n    [^\n]*)*"
)


def _prompts() -> list[str]:
    return [
        evaluation["prompt"]
        for skill in SKILLS
        for evaluation in json.loads(
            (REPO_ROOT / "skills/work" / skill / "evals/evals.json").read_text()
        )["evals"]
    ]


def _is_a_warning(prompt: str, start: int) -> bool:
    line_start = prompt.rfind("\n", 0, start) + 1
    return prompt[line_start:start].endswith("warning: ")


def quoted_refusals(prompt: str) -> list[tuple[str, int | None]]:
    """Each refusal code in ``prompt`` with the exit status quoted nearest
    before it, or ``None`` when no status precedes it. A code quoted in a
    ``warning:`` line is no refusal."""
    statuses = [
        (status.end(), int(status.group(1)))
        for status in EXIT_STATUS.finditer(prompt)
    ]
    return [
        (
            code.group(0),
            next(
                (
                    exit_code
                    for end, exit_code in reversed(statuses)
                    if end <= code.start()
                ),
                None,
            ),
        )
        for code in REFUSAL_CODE.finditer(prompt)
        if not _is_a_warning(prompt, code.start())
    ]


def normalised(line: str) -> str:
    """``line`` with its paths, quoted values and numbers made generic, and
    without the ``Error:`` or ``warning:`` a caller prints it after."""
    line = re.sub(r"^(?:Error: |warning: )", "", line.strip())
    line = re.sub(r"'[^']*'", "'…'", line)
    line = re.sub(r"meta/[^\s`'),]+", "<path>", line)
    return re.sub(r"\d+", "#", line)


def _form(prefix: str | None) -> str:
    return "warning" if prefix == "warning: " else "refusal"


def quoted_blocks(prompt: str) -> list[tuple[tuple[str, str], list[str]]]:
    """Each refusal or warning ``prompt`` quotes on a line of its own, by
    form and code, as its normalised lines: the line naming the code and
    each indented line it lists beneath."""
    return [
        (
            (_form(block.group(1)), block.group(2)),
            [normalised(line) for line in block.group(0).split("\n")],
        )
        for block in QUOTED_BLOCK.finditer(prompt)
    ]


def printed_lines(stderr: str) -> dict[tuple[str, str], set[str]]:
    """Each normalised line of ``stderr`` under the form and code it belongs
    to: those of the line naming a code, or of the line it is listed
    beneath."""
    lines: dict[tuple[str, str], set[str]] = {}
    owner = None
    for line in stderr.splitlines():
        if not line.startswith("    "):
            named = REFUSAL_CODE.search(line)
            owner = (
                (
                    _form(
                        "warning: " if line.startswith("warning: ") else None
                    ),
                    named.group(0),
                )
                if named
                else None
            )
        if owner:
            lines.setdefault(owner, set()).add(normalised(line))
    return lines


def _repository(tmp_path: Path, name: str, work: str) -> Path:
    root = tmp_path / name
    (root / ".jj").mkdir(parents=True)
    (root / "meta/work").mkdir(parents=True)
    (root / ".accelerator").mkdir()
    (root / ".accelerator/config.md").write_text(f"---\nwork:\n{work}---\n")
    return root


def _run(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(WORK), *args],
        cwd=root,
        env={**os.environ, "ACCELERATOR_PLUGIN_ROOT": str(REPO_ROOT)},
        capture_output=True,
        text=True,
        check=False,
    )


def _elicited(tmp_path: Path) -> dict[str, subprocess.CompletedProcess[str]]:
    keyed = '  id_pattern: "{key}-{number:04d}"\n  key: PP\n'
    numeric = '  id_pattern: "{number:04d}"\n'
    held = _repository(tmp_path, "held", numeric)
    (held / "meta/work/0400-held.md").write_text('---\nid: "0400"\n---\n')
    off_pattern = _repository(tmp_path, "off-pattern", keyed)
    (off_pattern / "meta/work/0001-x.md").write_text('---\nid: "0001"\n---\n')
    malformed = _repository(tmp_path, "malformed", numeric)
    (malformed / "meta/work/0399.md").write_text('---\nid: "0399"\n---\n')
    elicited = {
        "E_ID_WITH_PROJECT": _run(
            _repository(tmp_path, "project", keyed),
            "create",
            "Item",
            "task",
            "medium",
            "--id",
            "XY-0400",
            "--project",
            "XY",
            "--dry-run",
        ),
        "E_ID_TRACKER_OWNED": _run(
            _repository(
                tmp_path,
                "tracker",
                '  id_pattern: "{tracker}"\n  integration: jira\n',
            ),
            "id",
            "check",
            "--",
            "0400",
        ),
        "E_ID_DRAFT_RESERVED": _run(
            _repository(tmp_path, "draft", numeric),
            "id",
            "check",
            "--",
            "draft-k7mq3x",
        ),
        "E_ID_PATTERN_MISMATCH": _run(
            _repository(tmp_path, "mismatch", numeric),
            "id",
            "check",
            "--",
            "foo",
        ),
        "E_ID_TAKEN": _run(held, "id", "check", "--", "0400"),
        "E_REKEY_REQUIRED": _run(off_pattern, "id", "check", "--", "0400"),
        "E_WORK_ITEM_MALFORMED": _run(malformed, "id", "check", "--", "0400"),
        "E_WORK_ID_PATTERN_INVALID": _run(
            _repository(tmp_path, "invalid", '  id_pattern: "{bogus}"\n'),
            "resolve",
            "0041",
        ),
        "E_RESOLVE_INVALID": _run(
            _repository(tmp_path, "unresolvable", numeric),
            "resolve",
            "--",
            "a_b!",
        ),
    }
    for code, run in elicited.items():
        assert code in run.stderr, f"{code} not elicited: {run.stderr}"
    return elicited


def _variants(tmp_path: Path) -> list[subprocess.CompletedProcess[str]]:
    """Runs printing the other forms a quoted refusal or warning takes."""
    keyed = '  id_pattern: "{key}-{number:04d}"\n  key: PP\n'
    two_off = _repository(tmp_path, "two-off-pattern", keyed)
    (two_off / "meta/work/0001-x.md").write_text('---\nid: "0001"\n---\n')
    (two_off / "meta/work/0002-y.md").write_text('---\nid: "0002"\n---\n')
    tracker = _repository(
        tmp_path,
        "tracker-off",
        '  id_pattern: "{tracker}"\n  integration: jira\n',
    )
    (tracker / "meta/work/0042-search.md").write_text('---\nid: "0042"\n---\n')
    one_off = _repository(tmp_path, "one-off-pattern", keyed)
    (one_off / "meta/work/0001-x.md").write_text('---\nid: "0001"\n---\n')
    malformed = _repository(
        tmp_path, "two-malformed", '  id_pattern: "{number:04d}"\n'
    )
    (malformed / "meta/work/0399.md").write_text('---\nid: "0399"\n---\n')
    (malformed / "meta/work/0398-cafe.md").write_bytes(
        b'---\nid: "0398"\ntitle: Caf\xe9\n---\n'
    )
    return [
        _run(two_off, "id", "check", "--", "0400"),
        _run(tracker, "resolve", "--", "meta/work/0042-search.md"),
        _run(one_off, "list"),
        _run(malformed, "id", "check", "--", "0400"),
    ]


def _verdict_field_counts(tmp_path: Path) -> dict[str, set[int]]:
    root = _repository(
        tmp_path, "verdicts", '  id_pattern: "{key}-{number:04d}"\n  key: PP\n'
    )
    counts: dict[str, set[int]] = {kind: set() for kind in VERDICT_KINDS}
    for requested in ("PP-0400", "XY-0400", "400", "xy-400"):
        run = _run(root, "id", "check", "--", requested)
        assert run.returncode == 0, run.stderr
        fields = run.stdout.strip().split("\t")
        counts[fields[0]].add(len(fields))
    return counts


@pytest.fixture(scope="module")
def cli(tmp_path_factory: pytest.TempPathFactory) -> dict:
    assert WORK.exists(), f"build {WORK} first (mise run build:cli:work:dev)"
    tmp_path = tmp_path_factory.mktemp("eval-contract")
    elicited = _elicited(tmp_path)
    lines: dict[tuple[str, str], set[str]] = {}
    for run in [*elicited.values(), *_variants(tmp_path)]:
        for owner, printed in printed_lines(run.stderr).items():
            lines.setdefault(owner, set()).update(printed)
    return {
        "exit_codes": {code: run.returncode for code, run in elicited.items()},
        "lines": lines,
        "verdict_fields": _verdict_field_counts(tmp_path),
    }


def test_a_quoted_code_takes_the_exit_status_quoted_nearest_before_it() -> None:
    assert quoted_refusals(
        "Simulate `x` exiting 4 with this stderr:\n\nError: E_ID_TAKEN: …"
    ) == [("E_ID_TAKEN", 4)]
    assert quoted_refusals(
        "`x` exiting 1 with this stderr:\n\nwaiting for y\nError: "
        "E_IDENTITY_RUN_BUSY: …"
    ) == [("E_IDENTITY_RUN_BUSY", 1)]
    assert quoted_refusals(
        "Each exits 1 with the stderr given:\n\n1. a: Error: "
        "E_ID_PATTERN_MISMATCH: …\n2. b: Error: E_ID_DRAFT_RESERVED: …"
    ) == [("E_ID_PATTERN_MISMATCH", 1), ("E_ID_DRAFT_RESERVED", 1)]
    assert quoted_refusals("exiting 1 with stderr `E_RESOLVE_INVALID: …`") == [
        ("E_RESOLVE_INVALID", 1)
    ]
    assert quoted_refusals("Relay E_ID_TAKEN.") == [("E_ID_TAKEN", None)]
    assert (
        quoted_refusals(
            "exiting 0 with this stderr:\n\nwarning: E_REKEY_REQUIRED: …"
        )
        == []
    )


def test_a_quoted_block_is_read_as_its_generic_lines() -> None:
    assert quoted_blocks(
        "stderr:\n\nError: E_WORK_ITEM_MALFORMED: 2 work items cannot be "
        "read. Fix each one:\n    meta/work/0399.md has `id` '0399'. Rename "
        "it to `0399-<slug>.md`\n\nSimulate"
    ) == [
        (
            ("refusal", "E_WORK_ITEM_MALFORMED"),
            [
                "E_WORK_ITEM_MALFORMED: # work items cannot be read. Fix "
                "each one:",
                "<path> has `id` '…'. Rename it to `#-<slug>.md`",
            ],
        )
    ]
    assert quoted_blocks("warning: E_REKEY_REQUIRED: 1 item\n") == [
        (("warning", "E_REKEY_REQUIRED"), ["E_REKEY_REQUIRED: # item"])
    ]
    assert quoted_blocks("relay `E_ID_TAKEN: …` as is") == []


def test_a_printed_line_belongs_to_the_code_above_it() -> None:
    assert printed_lines(
        "warning: E_XY: 2 bad:\n    a.md bad\nother\n    stray\n"
    ) == {("warning", "E_XY"): {"E_XY: # bad:", "a.md bad"}}


def test_every_quoted_block_reads_as_lines_the_cli_prints(cli: dict) -> None:
    for prompt in _prompts():
        for (form, code), lines in quoted_blocks(prompt):
            if code in NOT_ELICITED:
                continue
            assert (form, code) in cli["lines"], (form, code)
            for line in lines:
                assert line in cli["lines"][form, code], (form, code, line)


def test_every_quoted_code_is_elicited_or_named_as_not(cli: dict) -> None:
    quoted = {
        code for prompt in _prompts() for code, _ in quoted_refusals(prompt)
    }
    assert quoted, "the evals quote no refusal"
    assert quoted <= cli["exit_codes"].keys() | NOT_ELICITED, (
        quoted - cli["exit_codes"].keys() - NOT_ELICITED
    )


def test_every_quoted_refusal_exits_as_the_cli_exits_it(cli: dict) -> None:
    for prompt in _prompts():
        for code, exit_code in quoted_refusals(prompt):
            if code in NOT_ELICITED:
                continue
            assert exit_code == cli["exit_codes"][code], (code, exit_code)


def test_every_quoted_verdict_has_the_fields_its_kind_prints(cli: dict) -> None:
    quoted = [
        span
        for prompt in _prompts()
        for span in prompt.split("`")[1::2]
        if span.split("\t")[0] in VERDICT_KINDS and "\t" in span
    ]
    assert quoted, "the evals quote no verdict"
    for verdict in quoted:
        fields = verdict.split("\t")
        assert len(fields) in cli["verdict_fields"][fields[0]], verdict
        assert all(fields), verdict
