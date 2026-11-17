"""The CLI output the ``/create-work-item`` evals quote, held to the CLI.

An eval prompt simulates ``work id check`` and ``work create --id`` by
quoting what they print. Each quoted ``E_ID_*`` refusal must exit as the
built binary exits that refusal, and each quoted verdict must have the
fields the binary prints for its kind, so an eval cannot keep passing
against output the CLI no longer gives.
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
EVALS = REPO_ROOT / "skills/work/create-work-item/evals/evals.json"
QUOTED_REFUSAL = re.compile(
    r"exiting (\d+) with this stderr:\s*(?:Error: )?(E_ID_[A-Z_]+):"
)
VERDICT_KINDS = ("exact", "other-key", "corrected")


def _prompts() -> list[str]:
    return [
        evaluation["prompt"]
        for evaluation in json.loads(EVALS.read_text())["evals"]
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


def test_every_quoted_id_refusal_exits_as_the_cli_exits_it(cli: dict) -> None:
    quoted = [
        (int(exit_code), code)
        for prompt in _prompts()
        for exit_code, code in QUOTED_REFUSAL.findall(prompt)
    ]
    assert quoted, "the evals quote no E_ID_* refusal"
    for exit_code, code in quoted:
        assert exit_code == cli["exit_codes"][code], code


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
