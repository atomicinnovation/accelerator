"""The CLI output the work skills' evals quote, held to the CLI.

An eval prompt simulates an ``accelerator work`` command by quoting what it
prints. Each refusal code a prompt quotes must exit as the built binary
exits that refusal, read from the nearest exit status the prompt gives
before it, and each quoted verdict must have the fields the binary prints
for its kind, so an eval cannot keep passing against output the CLI no
longer gives.

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


def _refusal_exit_codes(tmp_path: Path) -> dict[str, int]:
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
    return {code: run.returncode for code, run in elicited.items()}


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
    return {
        "exit_codes": _refusal_exit_codes(tmp_path),
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
