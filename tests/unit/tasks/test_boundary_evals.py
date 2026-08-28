"""Guard the review lens boundary evals, ported from
``scripts/test-boundary-evals.sh``.

Each lens ships a ``boundary_benchmark.json`` recording a negative-output
regression run — proof the lens does not produce findings in a peer lens's
domain. This asserts every such benchmark exists and records a 100% pass
rate (every expectation across every run passed).

Synthetic ``tmp_path`` trees exercise each branch, plus a live-tree assertion
that the five shipped benchmarks pass.
"""

import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

LENSES = ("clarity", "completeness", "dependency", "scope", "testability")


def _benchmark_path(root: Path, lens: str) -> Path:
    return (
        root
        / "skills"
        / "review"
        / "lenses"
        / f"{lens}-lens"
        / "evals"
        / "boundary_benchmark.json"
    )


def violations(root: Path) -> list[str]:
    """Every missing-benchmark or non-100%-pass violation across the lenses."""
    found: list[str] = []
    for lens in LENSES:
        benchmark = _benchmark_path(root, lens)
        if not benchmark.is_file():
            found.append(
                f"{lens} boundary_benchmark.json not found at {benchmark}"
            )
            continue

        data = json.loads(benchmark.read_text())
        for run in data.get("runs", []):
            eval_name = run.get("eval_name", "?")
            for expectation in run.get("expectations", []):
                if not expectation.get("passed", False):
                    text = expectation.get("text", "?")
                    found.append(
                        f"{lens} boundary eval '{eval_name}': expectation not "
                        f"passed: {text}"
                    )
    return found


def _write_benchmark(root: Path, lens: str, passed_flags: list[bool]) -> None:
    path = _benchmark_path(root, lens)
    path.parent.mkdir(parents=True, exist_ok=True)
    expectations = [
        {"text": f"expectation {index}", "passed": flag}
        for index, flag in enumerate(passed_flags)
    ]
    payload = {
        "runs": [{"eval_name": f"{lens}-eval", "expectations": expectations}]
    }
    path.write_text(json.dumps(payload))


def _write_all_passing(root: Path) -> None:
    for lens in LENSES:
        _write_benchmark(root, lens, [True, True, True])


def test_all_passing_benchmarks_yield_no_violations(tmp_path: Path) -> None:
    _write_all_passing(tmp_path)
    assert violations(tmp_path) == []


def test_missing_benchmark_is_flagged(tmp_path: Path) -> None:
    _write_all_passing(tmp_path)
    _benchmark_path(tmp_path, "scope").unlink()
    flagged = violations(tmp_path)
    assert any("scope boundary_benchmark.json not found" in v for v in flagged)


def test_failing_expectation_is_flagged(tmp_path: Path) -> None:
    _write_all_passing(tmp_path)
    _write_benchmark(tmp_path, "clarity", [True, False, True])
    flagged = violations(tmp_path)
    assert any(
        "clarity boundary eval" in v and "expectation not passed" in v
        for v in flagged
    )


def test_every_failing_expectation_is_reported(tmp_path: Path) -> None:
    _write_all_passing(tmp_path)
    _write_benchmark(tmp_path, "dependency", [False, True, False])
    flagged = [v for v in violations(tmp_path) if v.startswith("dependency")]
    assert len(flagged) == 2


def test_missing_passed_key_counts_as_failed(tmp_path: Path) -> None:
    _write_all_passing(tmp_path)
    path = _benchmark_path(tmp_path, "testability")
    path.write_text(
        json.dumps(
            {
                "runs": [
                    {
                        "eval_name": "testability-eval",
                        "expectations": [{"text": "no passed key"}],
                    }
                ]
            }
        )
    )
    assert any(v.startswith("testability") for v in violations(tmp_path))


def test_absent_runs_key_is_tolerated(tmp_path: Path) -> None:
    _write_all_passing(tmp_path)
    _benchmark_path(tmp_path, "completeness").write_text(json.dumps({}))
    assert violations(tmp_path) == []


def test_the_real_boundary_benchmarks_pass() -> None:
    assert violations(REPO_ROOT) == []
