import json

HEAD = [
    "fn store(record: Record) {",
    "    let path = claim_path(record);",
    "    disk.write(path, record);",
    "    journal.append(record);",
    "}",
]
DIFF = {
    "src/a.rs": {
        "status": "added",
        "add": len(HEAD),
        "del": 0,
        "hunks": [{"o": 0, "n": 1, "l": [f"+{line}" for line in HEAD]}],
    }
}
FIGURE = {
    "id": "f-drawing",
    "kind": "svg",
    "title": "Shape",
    "svg": "<svg viewBox='0 0 1 1'/>",
}
DESIGN = {"summary": "One port, two adapters.", "figures": [FIGURE]}
DECISION = {
    "id": "d-port",
    "decision": "Reach the disk through a port",
    "alternatives": ["call std::fs directly"],
    "rationale": "Tests can record every call.",
}
ABSTRACTION = {
    "id": "a-store",
    "name": "Store",
    "kind": "trait",
    "status": "new",
    "responsibility": "Keeps the records.",
}
INVARIANT = {"id": "c-i1", "text": "A record is journalled once."}
CHECK = {
    "question": "Does any writer bypass the port?",
    "problem": "A direct `std::fs` call outside the adapter.",
}
STOP = {
    "id": "c-s1",
    "file": "src/a.rs",
    "start": 1,
    "end": 5,
    "anchor": "fn store(",
    "title": "The store",
    "carries": ["d-port"],
    "fit": "Where the port decision is enforced for every writer.",
    "why": "A recorded write is the only way a test can see the order.",
}


def chapter(chapter_id="c", **fields):
    figure = dict(FIGURE, id=f"f-{chapter_id}")
    return {
        "id": chapter_id,
        "title": chapter_id,
        "figures": [figure],
        "decisions": [DECISION],
        "stops": [STOP],
        **fields,
    }


def assemble(root, script, chapters, design=DESIGN):
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
    (root / "content/design.json").write_text(json.dumps(design))
    for each in chapters:
        (root / f"content/chapter-{each['id']}.json").write_text(
            json.dumps(each)
        )
    return script("build").assemble(root)


def warnings_for(root, script, *chapters, design=DESIGN):
    _, warnings = assemble(root, script, list(chapters), design)
    return warnings


def stop_warnings(root, script, **fields):
    return warnings_for(root, script, chapter(stops=[dict(STOP, **fields)]))


def without(stop, *fields):
    return {key: value for key, value in stop.items() if key not in fields}


def test_a_stop_that_says_why_and_what_it_carries_raises_nothing(
    tmp_path, script
):
    assert warnings_for(tmp_path, script, chapter()) == []


def test_a_stop_that_does_not_say_why_is_reported(tmp_path, script):
    warnings = warnings_for(
        tmp_path, script, chapter(stops=[without(STOP, "why")])
    )

    assert any("stop c-s1" in w and "no why" in w for w in warnings)


def test_a_stop_that_still_narrates_its_code_in_a_note_is_reported(
    tmp_path, script
):
    warnings = stop_warnings(tmp_path, script, note="Writes one record.")

    assert any(
        "stop c-s1" in w and "note is no longer shown" in w for w in warnings
    )


def test_a_stop_with_a_single_free_form_check_is_reported(tmp_path, script):
    warnings = stop_warnings(tmp_path, script, check="Look at the port.")

    assert any(
        "stop c-s1" in w and "check is no longer shown" in w for w in warnings
    )


def test_a_stop_that_is_evidence_for_nothing_is_reported(tmp_path, script):
    warnings = warnings_for(
        tmp_path,
        script,
        chapter(
            decisions=[],
            abstractions=[ABSTRACTION],
            stops=[without(STOP, "carries")],
        ),
    )

    assert any("stop c-s1" in w and "carries nothing" in w for w in warnings)


def test_a_stop_carrying_something_the_page_does_not_declare_is_reported(
    tmp_path, script
):
    warnings = stop_warnings(tmp_path, script, carries=["d-port", "d-lost"])

    assert any("stop c-s1" in w and "unknown d-lost" in w for w in warnings)


def test_a_stop_may_carry_an_abstraction_an_invariant_or_a_risk(
    tmp_path, script
):
    risk = {"id": "c-r1", "level": "medium", "text": "A torn write."}
    stop = dict(
        STOP, carries=["d-port", "a-store", "c-i1", "c-r1"], checks=[CHECK]
    )

    warnings = warnings_for(
        tmp_path,
        script,
        chapter(
            abstractions=[ABSTRACTION],
            invariants=[INVARIANT],
            risks=[risk],
            stops=[stop],
        ),
    )

    assert warnings == []


def test_a_stop_may_carry_what_the_design_or_another_chapter_declares(
    tmp_path, script
):
    design = dict(DESIGN, decisions=[dict(DECISION, id="d-top")])
    other = chapter(
        "o",
        decisions=[],
        abstractions=[ABSTRACTION],
        stops=[dict(STOP, id="o-s1", carries=["a-store", "d-top"])],
    )
    first = chapter(stops=[dict(STOP, carries=["d-port", "a-store"])])

    assert warnings_for(tmp_path, script, first, other, design=design) == []


def test_a_decision_no_stop_carries_is_reported(tmp_path, script):
    unshown = dict(DECISION, id="d-unshown")

    warnings = warnings_for(
        tmp_path, script, chapter(decisions=[DECISION, unshown])
    )

    assert any(
        "decision d-unshown" in w and "no stop carries it" in w
        for w in warnings
    )


def test_a_decision_in_a_chapter_without_a_tour_needs_no_stop(tmp_path, script):
    assert warnings_for(tmp_path, script, chapter(stops=[])) == []


def test_a_high_risk_no_stop_carries_is_reported(tmp_path, script):
    risk = {"id": "c-r1", "level": "high", "text": "Deletes a live version."}

    warnings = warnings_for(tmp_path, script, chapter(risks=[risk]))

    assert any("risk c-r1" in w and "no stop carries it" in w for w in warnings)


def test_a_high_risk_without_an_id_cannot_be_carried_and_is_reported(
    tmp_path, script
):
    risk = {"level": "high", "text": "Deletes a live version."}

    warnings = warnings_for(tmp_path, script, chapter(risks=[risk]))

    assert any("high risk" in w and "no id" in w for w in warnings)


def test_a_lesser_risk_needs_no_stop(tmp_path, script):
    risk = {"id": "c-r1", "level": "medium", "text": "A slow prune."}

    assert warnings_for(tmp_path, script, chapter(risks=[risk])) == []


def test_a_stop_that_names_a_risk_without_a_check_is_reported(tmp_path, script):
    warnings = stop_warnings(
        tmp_path, script, risk="A bypassed port is never recorded."
    )

    assert any("stop c-s1" in w and "no checks" in w for w in warnings)


def test_a_stop_that_carries_a_risk_without_a_check_is_reported(
    tmp_path, script
):
    risk = {"id": "c-r1", "level": "low", "text": "A torn write."}
    stop = dict(STOP, carries=["d-port", "c-r1"])

    warnings = warnings_for(
        tmp_path, script, chapter(risks=[risk], stops=[stop])
    )

    assert any("stop c-s1" in w and "no checks" in w for w in warnings)


def test_a_stop_that_names_a_risk_and_how_to_check_it_raises_nothing(
    tmp_path, script
):
    warnings = stop_warnings(
        tmp_path,
        script,
        risk="A bypassed port is never recorded.",
        checks=[CHECK],
    )

    assert warnings == []


def test_a_check_that_does_not_say_what_a_problem_looks_like_is_left_out(
    tmp_path, script
):
    vague = {"question": "Is the port used?"}

    model, warnings = assemble(
        tmp_path, script, [chapter(stops=[dict(STOP, checks=[vague, CHECK])])]
    )

    assert any(
        "stop c-s1" in w and "check 1 has no problem" in w for w in warnings
    )
    assert model["chapters"][0]["stops"][0]["checks"] == [CHECK]


def test_a_check_that_asks_nothing_is_left_out(tmp_path, script):
    silent = {"problem": "A direct call."}

    warnings = stop_warnings(tmp_path, script, checks=[silent])

    assert any(
        "stop c-s1" in w and "check 1 has no question" in w for w in warnings
    )


def test_a_line_to_focus_on_inside_the_stop_raises_nothing(tmp_path, script):
    focus = [{"line": 4, "why": "Journalled after the write, not before."}]

    assert stop_warnings(tmp_path, script, focus=focus) == []


def test_a_line_to_focus_on_outside_the_stop_is_reported(tmp_path, script):
    focus = [{"line": 9, "why": "Journalled after the write."}]

    warnings = stop_warnings(tmp_path, script, focus=focus)

    assert any(
        "stop c-s1" in w and "focus line 9" in w and "1-5" in w
        for w in warnings
    )


def test_a_focus_that_names_no_line_is_left_out(tmp_path, script):
    focus = [{"why": "Journalled after the write."}]

    model, warnings = assemble(
        tmp_path, script, [chapter(stops=[dict(STOP, focus=focus)])]
    )

    assert any(
        "stop c-s1" in w and "focus 1 has no line" in w for w in warnings
    )
    assert model["chapters"][0]["stops"][0]["focus"] == []


def test_a_why_that_mostly_names_its_own_lines_is_noted_not_refused(
    tmp_path, script
):
    narrated = (
        "`claim_path` gives the `path`, then `disk.write` stores the "
        "`record` and `journal.append` logs it."
    )

    warnings = stop_warnings(tmp_path, script, why=narrated)

    [note] = [w for w in warnings if "stop c-s1" in w]
    assert "restates its own lines" in note
    assert type(note).__name__ == "Note"


def test_a_why_that_names_one_thing_from_its_lines_is_not_noted(
    tmp_path, script
):
    pointed = "`journal.append` comes last so a failed write leaves no entry."

    assert stop_warnings(tmp_path, script, why=pointed) == []
