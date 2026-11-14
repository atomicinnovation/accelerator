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
    assert len((SKILL / "SKILL.md").read_text().splitlines()) <= 350
