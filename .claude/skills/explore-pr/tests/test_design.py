import json

DIFF = {"src/a.rs": {"status": "added", "add": 1, "del": 0, "hunks": []}}
FIGURE = {
    "id": "f-drawing",
    "kind": "svg",
    "title": "Shape",
    "svg": "<svg viewBox='0 0 1 1'/>",
}
ABSTRACTION = {
    "id": "a-store",
    "name": "Store",
    "kind": "trait",
    "status": "new",
    "responsibility": "Keeps the records.",
}
DECISION = {
    "id": "d-port",
    "decision": "Reach the disk through a port",
    "alternatives": ["call std::fs directly"],
    "rationale": "Tests can record every call.",
}
DESIGN = {"summary": "One port, two adapters.", "figures": [FIGURE]}


def workspace(root, chapters, design=DESIGN):
    (root / "content").mkdir(parents=True)
    (root / "data").mkdir()
    (root / "data/diff.json").write_text(json.dumps(DIFF))
    (root / "explore.json").write_text(json.dumps({"pr": {"number": 1}}))
    (root / "content/files.json").write_text(
        json.dumps([{"path": "src/a.rs", "summary": "s", "risk": "low"}])
    )
    (root / "content/overview.json").write_text(
        json.dumps({"figures": [dict(FIGURE, id="f-over")]})
    )
    if design is not None:
        (root / "content/design.json").write_text(json.dumps(design))
    for chapter in chapters:
        (root / f"content/chapter-{chapter['id']}.json").write_text(
            json.dumps(chapter)
        )
    return root


def chapter(chapter_id, **extra):
    figure = dict(FIGURE, id=f"f-{chapter_id}")
    return {"id": chapter_id, "title": chapter_id, "figures": [figure], **extra}


def assemble(tmp_path, script, chapters, design=DESIGN):
    return script("build").assemble(workspace(tmp_path, chapters, design))


def test_a_chapter_with_design_content_raises_nothing(tmp_path, script):
    _, warnings = assemble(
        tmp_path, script, [chapter("c", abstractions=[ABSTRACTION])]
    )

    assert warnings == []


def test_a_chapter_that_says_nothing_about_design_is_reported(tmp_path, script):
    _, warnings = assemble(tmp_path, script, [chapter("lines")])

    assert any(
        "chapter lines" in w and "abstractions or decisions" in w
        for w in warnings
    )


def test_decisions_alone_are_enough_design_content(tmp_path, script):
    _, warnings = assemble(
        tmp_path, script, [chapter("c", decisions=[DECISION])]
    )

    assert warnings == []


def test_a_page_without_a_design_summary_is_reported(tmp_path, script):
    _, warnings = assemble(
        tmp_path,
        script,
        [chapter("c", abstractions=[ABSTRACTION])],
        design=None,
    )

    assert any("design" in w and "summary" in w for w in warnings)


def test_the_design_is_part_of_the_content_model(tmp_path, script):
    content, _ = assemble(
        tmp_path, script, [chapter("c", decisions=[DECISION])]
    )

    assert content["design"]["summary"] == "One port, two adapters."


def test_design_figures_are_checked_like_any_other(tmp_path, script):
    broken = {
        "id": "f-arch",
        "kind": "matrix",
        "title": "Shape",
        "rows": ["r1", "r2"],
        "cols": ["c1"],
        "cells": [["a"]],
    }

    _, warnings = assemble(
        tmp_path,
        script,
        [chapter("c", decisions=[DECISION])],
        design={"summary": "s", "figures": [broken]},
    )

    assert any("f-arch" in w and "rows of cells" in w for w in warnings)


def test_a_design_item_naming_a_missing_chapter_is_reported(tmp_path, script):
    design = dict(
        DESIGN, decisions=[dict(DECISION, id="d-top", chapter="nowhere")]
    )

    _, warnings = assemble(
        tmp_path, script, [chapter("c", decisions=[DECISION])], design=design
    )

    assert any("d-top" in w and "nowhere" in w for w in warnings)


def test_an_abstraction_or_decision_id_used_twice_is_reported(tmp_path, script):
    chapters = [
        chapter("c1", abstractions=[ABSTRACTION]),
        chapter("c2", abstractions=[ABSTRACTION]),
    ]

    _, warnings = assemble(tmp_path, script, chapters)

    assert any("duplicate design id a-store" in w for w in warnings)


def key_abstractions(tmp_path, script, chapters, design=DESIGN):
    content, _ = assemble(tmp_path, script, chapters, design)
    return [
        (entry["item"]["name"], entry["chapters"])
        for entry in content["keyAbstractions"]
    ]


def test_an_abstraction_in_the_design_and_a_chapter_is_listed_once(
    tmp_path, script
):
    design = dict(DESIGN, abstractions=[dict(ABSTRACTION, id="a-top")])

    keys = key_abstractions(
        tmp_path, script, [chapter("c", abstractions=[ABSTRACTION])], design
    )

    assert keys == [("Store", ["c"])]


def test_an_abstraction_two_chapters_declare_lists_both_chapters(
    tmp_path, script
):
    chapters = [
        chapter("c1", abstractions=[dict(ABSTRACTION, id="a-1")]),
        chapter("c2", abstractions=[dict(ABSTRACTION, id="a-2")]),
    ]

    keys = key_abstractions(tmp_path, script, chapters)

    assert keys == [("Store", ["c1", "c2"])]


def test_names_that_differ_only_in_markup_are_the_same_abstraction(
    tmp_path, script
):
    chapters = [
        chapter("c1", abstractions=[dict(ABSTRACTION, id="a-1")]),
        chapter(
            "c2", abstractions=[dict(ABSTRACTION, id="a-2", name="`Store`")]
        ),
    ]

    keys = key_abstractions(tmp_path, script, chapters)

    assert keys == [("Store", ["c1", "c2"])]


def test_the_first_declaration_describes_the_key_abstraction(tmp_path, script):
    design = dict(
        DESIGN,
        abstractions=[
            dict(ABSTRACTION, id="a-top", responsibility="Owns every record.")
        ],
    )

    content, _ = assemble(
        tmp_path, script, [chapter("c", abstractions=[ABSTRACTION])], design
    )

    [entry] = content["keyAbstractions"]
    assert entry["item"]["responsibility"] == "Owns every record."


def test_a_key_abstraction_works_with_every_declarations_collaborators(
    tmp_path, script
):
    chapters = [
        chapter(
            "c1",
            abstractions=[
                dict(ABSTRACTION, id="a-1", collaborators=["Clock", "Disk"])
            ],
        ),
        chapter(
            "c2",
            abstractions=[
                dict(ABSTRACTION, id="a-2", collaborators=["Disk", "Log"])
            ],
        ),
    ]

    content, _ = assemble(tmp_path, script, chapters)

    [entry] = content["keyAbstractions"]
    assert entry["item"]["collaborators"] == ["Clock", "Disk", "Log"]


def test_a_collaborator_given_with_its_kind_is_drawn_with_that_kind(
    tmp_path, script
):
    collaborator = {"name": "Clock", "kind": "trait"}
    chapters = [
        chapter(
            "c", abstractions=[dict(ABSTRACTION, collaborators=[collaborator])]
        )
    ]

    content, _ = assemble(tmp_path, script, chapters)

    [entry] = content["keyAbstractions"]
    assert entry["item"]["collaborators"] == ["Clock"]
    assert content["collaboratorKinds"] == {"Clock": "trait"}


def test_a_collaborator_that_is_not_an_abstraction_needs_a_kind(
    tmp_path, script
):
    chapters = [
        chapter("c", abstractions=[dict(ABSTRACTION, collaborators=["Clock"])])
    ]

    _, warnings = assemble(tmp_path, script, chapters)

    assert any("Clock" in w and "kind" in w for w in warnings)


def test_a_collaborator_that_is_an_abstraction_needs_no_kind(tmp_path, script):
    clock = dict(ABSTRACTION, id="a-clock", name="Clock")
    store = dict(ABSTRACTION, collaborators=["Clock"])
    chapters = [chapter("c", abstractions=[store, clock])]

    _, warnings = assemble(tmp_path, script, chapters)

    assert not any("Clock" in w for w in warnings)


def test_a_design_abstraction_owned_by_a_chapter_lists_that_chapter_first(
    tmp_path, script
):
    design = dict(
        DESIGN, abstractions=[dict(ABSTRACTION, id="a-top", chapter="c2")]
    )
    chapters = [
        chapter("c1", abstractions=[ABSTRACTION]),
        chapter("c2", decisions=[DECISION]),
    ]

    keys = key_abstractions(tmp_path, script, chapters, design)

    assert keys == [("Store", ["c2", "c1"])]


def test_distinct_abstractions_keep_the_design_then_tour_order(
    tmp_path, script
):
    design = dict(
        DESIGN, abstractions=[dict(ABSTRACTION, id="a-top", name="Port")]
    )
    chapters = [
        chapter("c1", abstractions=[ABSTRACTION]),
        chapter("c2", abstractions=[dict(ABSTRACTION, id="a-2", name="Disk")]),
    ]

    keys = key_abstractions(tmp_path, script, chapters, design)

    assert keys == [("Port", []), ("Store", ["c1"]), ("Disk", ["c2"])]


STOP = {
    "id": "c-s1",
    "file": "src/a.rs",
    "start": 1,
    "end": 1,
    "title": "The store",
    "note": "Writes one record.",
}


def test_a_stop_that_does_not_place_itself_in_the_change_is_reported(
    tmp_path, script
):
    _, warnings = assemble(
        tmp_path,
        script,
        [chapter("c", abstractions=[ABSTRACTION], stops=[STOP])],
    )

    assert any("stop c-s1" in w and "bigger picture" in w for w in warnings)


def test_a_stop_that_places_itself_in_the_change_raises_nothing(
    tmp_path, script
):
    stop = dict(STOP, fit="Carries the port decision for every writer.")

    _, warnings = assemble(
        tmp_path,
        script,
        [chapter("c", abstractions=[ABSTRACTION], stops=[stop])],
    )

    assert not [w for w in warnings if "bigger picture" in w]
