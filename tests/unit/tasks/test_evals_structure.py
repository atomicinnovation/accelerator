"""Guard the structural integrity of every ``evals/`` pair under ``skills/``.

Ported from the retired evals-structure shell guard (and its
evals-structure-self meta-test cases). For each
``evals.json`` found beneath a search root, the guard asserts:

1. a ``benchmark.json`` sits alongside it;
2. both files are valid JSON;
3. every eval id in ``evals.json`` has a ``with_skill`` run in
   ``benchmark.json``;
4. ``run_summary.with_skill.pass_rate.mean`` is present and at least 0.9.

The 0.9 floor (rather than 1.0) tolerates one known historical benchmark
(``clarity-lens``) committed at mean ~0.95 while still catching genuinely bad
benchmarks. Synthetic ``tmp_path`` trees exercise each branch, the carried
fixture directories pin the self-test expectations, and one live-tree
assertion proves the shipped ``skills/`` tree passes.
"""

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "evals-structure"

PASS_RATE_FLOOR = 0.9


def _is_valid_json(path: Path) -> bool:
    try:
        json.loads(path.read_text())
    except json.JSONDecodeError, OSError:
        return False
    return True


def _missing_eval_ids(evals_file: Path, benchmark_file: Path) -> list[int]:
    evals = json.loads(evals_file.read_text())
    benchmark = json.loads(benchmark_file.read_text())
    eval_ids = {e["id"] for e in evals.get("evals", [])}
    benchmark_ids = {
        run["eval_id"]
        for run in benchmark.get("runs", [])
        if run.get("configuration") == "with_skill"
    }
    return sorted(eval_ids - benchmark_ids)


def _mean_pass_rate(benchmark_file: Path) -> float | None:
    benchmark = json.loads(benchmark_file.read_text())
    return (
        benchmark.get("run_summary", {})
        .get("with_skill", {})
        .get("pass_rate", {})
        .get("mean")
    )


def _pair_violations(evals_file: Path, rel: str) -> list[str]:
    benchmark_file = evals_file.parent / "benchmark.json"
    rel_benchmark = f"{Path(rel).parent.as_posix()}/benchmark.json"

    if not benchmark_file.is_file():
        return [f"{rel}: missing benchmark.json — expected at {rel_benchmark}"]

    if not _is_valid_json(evals_file):
        return [f"{rel}: evals.json is not valid JSON"]

    if not _is_valid_json(benchmark_file):
        return [f"{rel_benchmark}: benchmark.json is not valid JSON"]

    found: list[str] = []

    missing = _missing_eval_ids(evals_file, benchmark_file)
    if missing:
        ids = " ".join(str(m) for m in missing)
        found.append(
            f"{rel}: eval IDs missing from benchmark.json with_skill runs: "
            f"{ids}"
        )

    mean = _mean_pass_rate(benchmark_file)
    if mean is None:
        found.append(
            f"{rel_benchmark}: run_summary.with_skill.pass_rate.mean not found"
        )
    elif float(mean) < PASS_RATE_FLOOR:
        found.append(
            f"{rel_benchmark}: pass_rate.mean = {mean} is below the "
            f"{PASS_RATE_FLOOR} threshold"
        )

    return found


def violations(root: Path) -> list[str]:
    """Every structural violation across ``evals.json`` files beneath root."""
    found: list[str] = []
    for evals_file in sorted(root.rglob("evals.json")):
        rel = evals_file.relative_to(root).as_posix()
        found.extend(_pair_violations(evals_file, rel))
    return found


def _write(path: Path, payload: object | str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = payload if isinstance(payload, str) else json.dumps(payload)
    path.write_text(text)


def _valid_evals() -> dict:
    return {
        "skill_name": "demo",
        "evals": [
            {"id": 1, "name": "scenario-a"},
            {"id": 2, "name": "scenario-b"},
        ],
    }


def _valid_benchmark(mean: float = 1.0, eval_ids: tuple[int, ...] = (1, 2)):
    return {
        "runs": [
            {"eval_id": i, "configuration": "with_skill", "run_number": 1}
            for i in eval_ids
        ],
        "run_summary": {"with_skill": {"pass_rate": {"mean": mean}}},
    }


def _pair(root: Path, name: str, evals: object, benchmark: object | None):
    directory = root / name / "evals"
    _write(directory / "evals.json", evals)
    if benchmark is not None:
        _write(directory / "benchmark.json", benchmark)


def test_missing_benchmark_is_flagged(tmp_path: Path) -> None:
    _pair(tmp_path, "demo", _valid_evals(), None)
    assert any("missing benchmark.json" in v for v in violations(tmp_path))


def test_malformed_evals_json_is_flagged(tmp_path: Path) -> None:
    _pair(tmp_path, "demo", '{ "evals": [ { "id": 1', _valid_benchmark())
    assert any(
        "evals.json is not valid JSON" in v for v in violations(tmp_path)
    )


def test_malformed_benchmark_json_is_flagged(tmp_path: Path) -> None:
    _pair(tmp_path, "demo", _valid_evals(), "{ not json")
    assert any(
        "benchmark.json is not valid JSON" in v for v in violations(tmp_path)
    )


def test_missing_eval_ids_is_flagged(tmp_path: Path) -> None:
    _pair(tmp_path, "demo", _valid_evals(), _valid_benchmark(eval_ids=(1,)))
    assert any(
        "eval IDs missing from benchmark.json with_skill runs" in v
        for v in violations(tmp_path)
    )


def test_baseline_only_run_does_not_satisfy_a_scenario(tmp_path: Path) -> None:
    benchmark = {
        "runs": [
            {"eval_id": 1, "configuration": "with_skill", "run_number": 1},
            {"eval_id": 2, "configuration": "baseline", "run_number": 1},
        ],
        "run_summary": {"with_skill": {"pass_rate": {"mean": 1.0}}},
    }
    _pair(tmp_path, "demo", _valid_evals(), benchmark)
    assert any(
        "eval IDs missing from benchmark.json with_skill runs" in v
        for v in violations(tmp_path)
    )


def test_missing_pass_rate_mean_is_flagged(tmp_path: Path) -> None:
    benchmark = {
        "runs": [
            {"eval_id": 1, "configuration": "with_skill", "run_number": 1},
            {"eval_id": 2, "configuration": "with_skill", "run_number": 1},
        ],
        "run_summary": {"with_skill": {}},
    }
    _pair(tmp_path, "demo", _valid_evals(), benchmark)
    assert any("pass_rate.mean not found" in v for v in violations(tmp_path))


def test_low_pass_rate_is_flagged(tmp_path: Path) -> None:
    _pair(tmp_path, "demo", _valid_evals(), _valid_benchmark(mean=0.83))
    assert any("is below the 0.9 threshold" in v for v in violations(tmp_path))


def test_pass_rate_exactly_at_floor_passes(tmp_path: Path) -> None:
    _pair(tmp_path, "demo", _valid_evals(), _valid_benchmark(mean=0.9))
    assert violations(tmp_path) == []


def test_a_valid_pair_has_no_violations(tmp_path: Path) -> None:
    _pair(tmp_path, "demo", _valid_evals(), _valid_benchmark())
    assert violations(tmp_path) == []


def test_an_empty_root_has_no_violations(tmp_path: Path) -> None:
    assert violations(tmp_path) == []


def test_fixture_valid_pair_has_no_violations() -> None:
    assert violations(FIXTURES / "valid-pair") == []


def test_fixture_missing_benchmark_has_violations() -> None:
    assert violations(FIXTURES / "missing-benchmark") != []


def test_fixture_scenario_name_mismatch_has_violations() -> None:
    assert violations(FIXTURES / "scenario-name-mismatch") != []


def test_fixture_low_pass_rate_has_violations() -> None:
    assert violations(FIXTURES / "low-pass-rate") != []


def test_fixture_malformed_json_has_violations() -> None:
    assert violations(FIXTURES / "malformed-json") != []


def test_the_real_skills_tree_passes() -> None:
    assert violations(REPO_ROOT / "skills") == []
