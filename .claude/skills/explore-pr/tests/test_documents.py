import json
import re
from pathlib import Path

SKILL = Path(__file__).resolve().parent.parent
DOCUMENTS = [SKILL / "SKILL.md", *sorted((SKILL / "reference").glob("*.md"))]
BUNDLED = re.compile(
    r"(?<![\w/.-])((?:scripts|assets|reference|examples)/[\w./-]*\w)"
)
PRINCIPLES = ("## Design before lines", "## Pictures first")


def test_every_bundled_file_a_document_names_exists():
    named = {
        (document.name, path)
        for document in DOCUMENTS
        for path in BUNDLED.findall(document.read_text())
    }

    missing = sorted(
        f"{document}: {path}"
        for document, path in named
        if not (SKILL / path).exists()
    )
    assert named
    assert missing == []


def test_each_principle_is_set_out_in_the_content_model_alone():
    setting_out = {
        heading: [
            document.name
            for document in DOCUMENTS
            if heading in document.read_text().splitlines()
        ]
        for heading in PRINCIPLES
    }

    assert set(setting_out) == set(PRINCIPLES)
    assert all(found == ["content-model.md"] for found in setting_out.values())


def test_the_skill_stays_short_enough_to_load_whole():
    assert len((SKILL / "SKILL.md").read_text().splitlines()) <= 360


def step(number):
    skill = (SKILL / "SKILL.md").read_text()
    start = skill.index(f"\n### {number}. ")
    return skill[start : skill.index("\n### ", start + 1)]


def test_the_shape_step_offers_three_depths_and_what_each_costs():
    rows = [line.strip() for line in step(3).splitlines()]
    table = [row for row in rows if row.startswith("| **")]

    assert [row.split("|")[1].strip() for row in table] == [
        "**Quick**",
        "**Standard**",
        "**Deep**",
    ]
    assert "subagent" in step(3)


def test_the_steps_that_cost_most_follow_the_agreed_depth():
    for number in (4, 5, 6, 7):
        assert "depth" in step(number).lower(), number


def test_the_evals_are_well_formed_and_cover_choosing_a_depth():
    evals = json.loads((SKILL / "evals/evals.json").read_text())["evals"]

    ids = [each["id"] for each in evals]
    assert len(ids) == len(set(ids))
    assert all(each["prompt"] and each["assertions"] for each in evals)
    assert any("depth" in each["expected_output"].lower() for each in evals)
