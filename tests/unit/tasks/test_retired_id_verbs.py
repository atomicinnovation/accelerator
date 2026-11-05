"""Guards against a skill naming a retired ``accelerator work`` ID verb.

``work next-number`` and ``work canonicalise-id`` are now ``work id next``
and ``work id canonicalise``. ``accelerator corpus adr next-number`` is a
different verb and keeps its name. Recorded ``benchmark.json`` runs are
history, so only ``SKILL.md`` and ``evals.json`` are scanned.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

RETIRED_VERB = re.compile(r"(?<!adr )\b(?:next-number|canonicalise-id)\b")


def violations(root: Path) -> list[str]:
    skills = root / "skills"
    sources = [*skills.rglob("SKILL.md"), *skills.rglob("evals.json")]
    return sorted(
        str(path.relative_to(root))
        for path in sources
        if RETIRED_VERB.search(re.sub(r"\s+", " ", path.read_text()))
    )


def _write(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_a_wrapped_or_bare_mention_is_flagged(tmp_path: Path) -> None:
    _write(
        tmp_path, "skills/a/SKILL.md", "run `accelerator work\nnext-number`\n"
    )
    _write(tmp_path, "skills/b/evals/evals.json", '"never calls next-number"\n')
    assert violations(tmp_path) == [
        "skills/a/SKILL.md",
        "skills/b/evals/evals.json",
    ]


def test_the_adr_verb_is_not_flagged(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "skills/c/SKILL.md",
        "accelerator corpus adr\n  next-number --count 3\n",
    )
    assert violations(tmp_path) == []


def test_a_recorded_benchmark_is_not_scanned(tmp_path: Path) -> None:
    _write(tmp_path, "skills/d/evals/benchmark.json", '"work next-number"\n')
    assert violations(tmp_path) == []


def test_no_skill_names_a_retired_id_verb() -> None:
    assert violations(REPO_ROOT) == []
