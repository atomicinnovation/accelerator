"""Guard that every ``/create-work-item`` eval passes in full, as written.

The evals-structure guard holds a skill's mean ``with_skill`` pass rate at
0.9, which would let several evals fail outright. Every eval in
``evals.json`` needs a ``with_skill`` run with a ``pass_rate`` of 1.0, graded
against the expectations the eval holds now, so an eval edited without
re-running the benchmark is flagged.

The benchmark also records the ``skill_digest`` of ``SKILL.md`` and
``evals.json`` it was last brought up to date against. Choosing which evals
an edit affects is left to whoever re-runs them, but an edit after which no
eval was re-run is flagged.
"""

import hashlib
import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]


def _evals_dir(root: Path) -> Path:
    return root / "skills" / "work" / "create-work-item" / "evals"


def skill_digest(root: Path) -> str:
    digest = hashlib.sha256()
    digest.update((_evals_dir(root).parent / "SKILL.md").read_bytes())
    digest.update(b"\0")
    digest.update((_evals_dir(root) / "evals.json").read_bytes())
    return f"sha256:{digest.hexdigest()}"


def failing_runs(root: Path) -> list[str]:
    evals = json.loads((_evals_dir(root) / "evals.json").read_text())["evals"]
    benchmark = json.loads((_evals_dir(root) / "benchmark.json").read_text())
    runs = benchmark.get("runs", [])
    current = skill_digest(root)
    if benchmark.get("metadata", {}).get("skill_digest") != current:
        return [
            f"skill changed since benchmark: re-run the evals it affects, "
            f"then record skill_digest {current}"
        ]
    with_skill = {
        run["eval_id"]: run
        for run in runs
        if run.get("configuration") == "with_skill"
    }
    found = []
    for evaluation in evals:
        eval_id = evaluation["id"]
        run = with_skill.get(eval_id)
        if run is None:
            found.append(f"eval {eval_id}: no with_skill run")
            continue
        pass_rate = run.get("result", {}).get("pass_rate")
        if pass_rate is None or pass_rate < 1.0:
            found.append(f"eval {eval_id}: pass_rate {pass_rate}")
            continue
        graded = [
            expectation["text"] for expectation in run.get("expectations", [])
        ]
        if graded != evaluation["expectations"]:
            found.append(f"eval {eval_id}: graded against other expectations")
    return found


def _write(root: Path, evals: list[dict], runs: list[dict]) -> None:
    directory = _evals_dir(root)
    directory.mkdir(parents=True, exist_ok=True)
    (directory.parent / "SKILL.md").write_text("# Create work item\n")
    (directory / "evals.json").write_text(json.dumps({"evals": evals}))
    (directory / "benchmark.json").write_text(
        json.dumps(
            {"metadata": {"skill_digest": skill_digest(root)}, "runs": runs}
        )
    )


def _eval(eval_id: int, expectations: list[str]) -> dict:
    return {"id": eval_id, "expectations": expectations}


def _run(
    eval_id: int, configuration: str, pass_rate: float | None, graded: list[str]
) -> dict:
    result = {} if pass_rate is None else {"pass_rate": pass_rate}
    return {
        "eval_id": eval_id,
        "configuration": configuration,
        "result": result,
        "expectations": [{"text": text, "passed": True} for text in graded],
    }


def test_every_eval_in_the_file_is_judged_on_its_current_expectations(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path,
        [_eval(eval_id, ["a"]) for eval_id in (1, 2, 3, 4, 5)],
        [
            _run(1, "with_skill", 1.0, ["a"]),
            _run(2, "with_skill", 0.75, ["a"]),
            _run(3, "with_skill", None, ["a"]),
            _run(4, "old_skill", 1.0, ["a"]),
            _run(5, "with_skill", 1.0, ["an earlier wording"]),
        ],
    )
    assert failing_runs(tmp_path) == [
        "eval 2: pass_rate 0.75",
        "eval 3: pass_rate None",
        "eval 4: no with_skill run",
        "eval 5: graded against other expectations",
    ]


def test_every_create_work_item_eval_passes_fully() -> None:
    assert failing_runs(REPO_ROOT) == []


def test_a_skill_changed_since_the_benchmark_is_flagged(tmp_path: Path) -> None:
    _write(tmp_path, [_eval(1, ["a"])], [_run(1, "with_skill", 1.0, ["a"])])
    skill = _evals_dir(tmp_path).parent / "SKILL.md"
    skill.write_text(skill.read_text() + "A reworded step.\n")

    assert failing_runs(tmp_path) == [
        f"skill changed since benchmark: re-run the evals it affects, then "
        f"record skill_digest {skill_digest(tmp_path)}"
    ]


def test_an_eval_file_changed_since_the_benchmark_is_flagged(
    tmp_path: Path,
) -> None:
    _write(tmp_path, [_eval(1, ["a"])], [_run(1, "with_skill", 1.0, ["a"])])
    evals = _evals_dir(tmp_path) / "evals.json"
    evals.write_text(evals.read_text() + "\n")

    assert failing_runs(tmp_path)[0].startswith("skill changed since benchmark")
