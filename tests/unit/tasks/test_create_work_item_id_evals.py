"""Guard that every ``/create-work-item --id`` eval passes in full.

The evals-structure guard holds a skill's mean ``with_skill`` pass rate at
0.9, which would let several of these evals fail outright. Each eval 39 to
72 needs a ``with_skill`` run, and every such run a ``pass_rate`` of 1.0.
"""

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

EXPLICIT_ID_EVALS = range(39, 73)


def _benchmark_path(root: Path) -> Path:
    return (
        root
        / "skills"
        / "work"
        / "create-work-item"
        / "evals"
        / "benchmark.json"
    )


def failing_runs(root: Path) -> list[str]:
    runs = json.loads(_benchmark_path(root).read_text()).get("runs", [])
    with_skill = [
        run
        for run in runs
        if run.get("configuration") == "with_skill"
        and run.get("eval_id") in EXPLICIT_ID_EVALS
    ]
    found = [
        f"eval {eval_id}: no with_skill run"
        for eval_id in EXPLICIT_ID_EVALS
        if not any(run["eval_id"] == eval_id for run in with_skill)
    ]
    found.extend(
        f"eval {run['eval_id']}: pass_rate {pass_rate}"
        for run in with_skill
        if (pass_rate := run.get("result", {}).get("pass_rate")) is None
        or pass_rate < 1.0
    )
    return found


def _run(eval_id: int | str, configuration: str, result: dict) -> dict:
    return {
        "eval_id": eval_id,
        "configuration": configuration,
        "result": result,
    }


def _write_benchmark(root: Path, runs: list[dict]) -> None:
    path = _benchmark_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"runs": runs}))


def test_a_failing_explicit_id_run_is_flagged(tmp_path: Path) -> None:
    passing = [
        _run(eval_id, "with_skill", {"pass_rate": 1.0})
        for eval_id in EXPLICIT_ID_EVALS
        if eval_id not in (40, 41, 42)
    ]
    _write_benchmark(
        tmp_path,
        [
            *passing,
            _run(40, "with_skill", {"pass_rate": 0.75}),
            _run(41, "with_skill", {}),
            _run(42, "old_skill", {"pass_rate": 1.0}),
            _run(43, "old_skill", {"pass_rate": 0.5}),
            _run(12, "with_skill", {"pass_rate": 0.5}),
        ],
    )
    assert sorted(failing_runs(tmp_path)) == [
        "eval 40: pass_rate 0.75",
        "eval 41: pass_rate None",
        "eval 42: no with_skill run",
    ]


def test_every_explicit_id_eval_passes_fully() -> None:
    assert failing_runs(REPO_ROOT) == []
