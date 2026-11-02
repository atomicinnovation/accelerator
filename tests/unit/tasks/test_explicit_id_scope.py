"""Guards that only ``/create-work-item`` passes an explicit ``--id`` to an
``accelerator work`` command.

``/extract-work-items`` and every other skill let the CLI allocate IDs.
Synthetic ``tmp_path`` trees exercise each branch, plus a real-tree
assertion.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

PERMITTED_SKILL = Path("skills/work/create-work-item")

EXPLICIT_ID = re.compile(
    r"accelerator work\b.*?\s--id(?:[\s=]|$)", re.MULTILINE
)


def _joined_continuations(text: str) -> str:
    return re.sub(r"\\\n", " ", text)


def violations(root: Path) -> list[str]:
    skills = root / "skills"
    return sorted(
        path.relative_to(root).as_posix()
        for path in skills.rglob("SKILL.md")
        if not path.relative_to(root).is_relative_to(PERMITTED_SKILL)
        and EXPLICIT_ID.search(_joined_continuations(path.read_text()))
    )


def _skill(root: Path, rel: str, body: str) -> None:
    path = root / rel / "SKILL.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_a_continued_command_carrying_an_id_is_flagged(
    tmp_path: Path,
) -> None:
    _skill(
        tmp_path,
        "skills/work/extract-work-items",
        '```\naccelerator work create "T" task medium \\\n  --id 0400\n```\n',
    )
    assert violations(tmp_path) == ["skills/work/extract-work-items/SKILL.md"]


def test_create_work_item_and_other_flags_are_not_flagged(
    tmp_path: Path,
) -> None:
    _skill(
        tmp_path,
        "skills/work/create-work-item",
        "accelerator work create \"T\" task medium --id='0400'\n",
    )
    _skill(
        tmp_path,
        "skills/work/other",
        'accelerator work create "T" task medium --id-file ids.txt\n',
    )
    assert violations(tmp_path) == []


def test_only_create_work_item_passes_an_explicit_id() -> None:
    assert violations(REPO_ROOT) == []
