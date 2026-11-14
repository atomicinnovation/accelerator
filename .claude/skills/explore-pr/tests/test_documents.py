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


GUIDE = "draw-diagram/reference/choosing.md"
KIND_DRAWN = re.compile(r"= \{ kind: '([a-z0-9-]+)'")


def kinds_the_kit_draws():
    notations = sorted(SKILL.parent.glob("draw-*/notation.js"))
    return [KIND_DRAWN.search(each.read_text()).group(1) for each in notations]


def document(name):
    return (SKILL / name).read_text()


def test_the_contract_lists_every_kind_the_kit_draws():
    contract = document("reference/content-model.md")
    kind_line = next(
        line for line in contract.splitlines() if line.startswith('  "kind": ')
    )
    listed = kind_line.split('"')[3].split("|")

    assert kinds_the_kit_draws()
    assert [kind for kind in kinds_the_kit_draws() if kind not in listed] == []
    for kind in kinds_the_kit_draws():
        assert f"| `{kind}` |" in contract, kind


def test_whoever_chooses_a_figure_is_sent_to_the_guide():
    for name in (
        "SKILL.md",
        "reference/content-model.md",
        "reference/reading-pass-brief.md",
        "reference/design-pass-brief.md",
    ):
        assert GUIDE in document(name), name
    assert (SKILL.parent / GUIDE).exists()


def test_the_briefs_keep_no_guide_of_their_own():
    for name in ("reference/reading-pass-brief.md", "SKILL.md"):
        assert "| `class-diagram` |" not in document(name), name


def test_no_document_still_describes_the_figures_as_they_were():
    for each in DOCUMENTS:
        text = each.read_text()
        assert "seven kinds" not in text, each.name
        assert "In pictures" not in text, each.name
    assert "layered `flow`" not in document("reference/design-pass-brief.md")


def test_the_evals_cover_choosing_a_figure_from_the_change():
    evals = json.loads(document("evals/evals.json"))["evals"]
    expected = " ".join(
        assertion["text"] for each in evals for assertion in each["assertions"]
    )

    assert "data flow diagram" in expected
    assert "entity-relationship diagram" in expected
    assert "flow, sequence, tree, matrix" not in expected
