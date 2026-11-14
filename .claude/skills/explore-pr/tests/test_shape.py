import json

DIFF = {
    "src/core.rs": {
        "status": "added",
        "add": 3,
        "del": 0,
        "hunks": [{"o": 0, "n": 1, "l": ["+fn a() {", "+    b();", "+}"]}],
    },
}
STOP = {
    "id": "s1",
    "file": "src/core.rs",
    "start": 1,
    "end": 3,
    "anchor": "fn a()",
    "fit": "f",
}
DRAWING = {"id": "f1", "kind": "svg", "svg": "<svg viewBox='0 0 1 1'/>"}
SOUND_CHAPTER = {
    "id": "first",
    "title": "First",
    "figures": [DRAWING],
    "decisions": [{"id": "d1", "decision": "d"}],
    "stops": [STOP],
}
SOUND = {
    "chapter-first": SOUND_CHAPTER,
    "overview": {"figures": [dict(DRAWING, id="f0")]},
    "design": {"summary": "s"},
    "files": [{"path": "src/core.rs", "summary": "s", "risk": "high"}],
}


def workspace(root, raw=None, **content):
    (root / "content").mkdir(parents=True)
    (root / "data").mkdir()
    (root / "data/diff.json").write_text(json.dumps(DIFF))
    (root / "explore.json").write_text(json.dumps({"pr": {"number": 1}}))
    for name, value in {**SOUND, **content}.items():
        (root / f"content/{name}.json").write_text(json.dumps(value))
    for name, text in (raw or {}).items():
        (root / f"content/{name}.json").write_text(text)
    return root


def assembled(root, script, raw=None, **content):
    return script("build").assemble(workspace(root, raw, **content))


def chapter(**fields):
    return {"chapter-first": {**SOUND_CHAPTER, **fields}}


def test_sound_content_raises_nothing(tmp_path, script):
    _, warnings = assembled(tmp_path, script)

    assert warnings == []


def test_a_file_that_is_not_json_is_reported_and_left_out(tmp_path, script):
    model, warnings = assembled(
        tmp_path, script, raw={"chapter-second": '{"id": "second",'}
    )

    assert [c["id"] for c in model["chapters"]] == ["first"]
    assert any(
        w.startswith("chapter-second.json: not valid JSON") and "line 1" in w
        for w in warnings
    )


def test_a_file_of_the_wrong_kind_is_reported_and_left_out(tmp_path, script):
    model, warnings = assembled(
        tmp_path, script, ledger={"id": "L1"}, overview=[1], breaking="none"
    )

    assert model["ledger"] == []
    assert model["breaking"] == []
    assert "ledger.json: must be a JSON array" in warnings
    assert "overview.json: must be a JSON object" in warnings
    assert "breaking.json: must be a JSON array" in warnings


def test_a_manifest_that_is_not_json_is_reported(tmp_path, script):
    root = workspace(tmp_path)
    (root / "explore.json").write_text("{")

    _, warnings = script("build").assemble(root)

    assert any(w.startswith("explore.json: not valid JSON") for w in warnings)


def test_a_chapter_without_an_id_is_reported_and_left_out(tmp_path, script):
    model, warnings = assembled(
        tmp_path, script, **{"chapter-odd": {"title": "Odd"}}
    )

    assert [c["id"] for c in model["chapters"]] == ["first"]
    assert "chapter-odd.json: chapter has no id; left out" in warnings


def test_a_stop_missing_what_places_it_is_reported_and_left_out(
    tmp_path, script
):
    stops = [STOP, {"id": "s2", "file": "src/core.rs", "end": 3}]

    model, warnings = assembled(tmp_path, script, **chapter(stops=stops))

    assert [s["id"] for s in model["chapters"][0]["stops"]] == ["s1"]
    assert "chapter first: stop s2 has no start; left out" in warnings


def test_a_line_number_that_is_not_a_whole_number_is_reported(tmp_path, script):
    stops = [dict(STOP, start="1"), dict(STOP, id="s2", anchorLine=1.5)]

    model, warnings = assembled(tmp_path, script, **chapter(stops=stops))

    assert model["chapters"][0]["stops"] == []
    assert (
        "chapter first: stop s1 start must be a whole number; left out"
        in warnings
    )
    assert (
        "chapter first: stop s2 anchorLine must be a whole number; left out"
        in warnings
    )


def test_an_item_without_an_id_is_named_by_its_position(tmp_path, script):
    stops = [STOP, {"file": "src/core.rs", "start": 1, "end": 2}]

    _, warnings = assembled(tmp_path, script, **chapter(stops=stops))

    assert "chapter first: stop 2 has no id; left out" in warnings


def test_a_ledger_row_without_its_line_is_reported_and_left_out(
    tmp_path, script
):
    rows = [{"id": "L1", "file": "src/core.rs"}, "L2"]

    model, warnings = assembled(tmp_path, script, ledger=rows)

    assert model["ledger"] == []
    assert "ledger.json: ledger row L1 has no line; left out" in warnings
    assert "ledger.json: ledger row 2 is not an object; left out" in warnings


def test_a_file_summary_without_a_path_is_reported(tmp_path, script):
    files = [*SOUND["files"], {"summary": "s"}]

    _, warnings = assembled(tmp_path, script, files=files)

    assert "files.json: file summary 2 has no path; left out" in warnings


def test_a_finding_check_without_an_id_is_reported(tmp_path, script):
    checks = [{"verdict": "confirmed"}]

    model, warnings = assembled(
        tmp_path, script, **chapter(findingChecks=checks)
    )

    assert model["chapters"][0]["findingChecks"] == []
    assert "chapter first: finding check 1 has no id; left out" in warnings


def test_a_list_given_as_something_else_is_reported_and_emptied(
    tmp_path, script
):
    model, warnings = assembled(tmp_path, script, **chapter(risks="none"))

    assert model["chapters"][0]["risks"] == []
    assert "chapter first: risks must be a list; left out" in warnings


def test_an_entry_that_is_not_an_object_is_reported_and_left_out(
    tmp_path, script
):
    risks = [{"level": "low", "text": "t"}, "another"]

    model, warnings = assembled(tmp_path, script, **chapter(risks=risks))

    assert model["chapters"][0]["risks"] == [{"level": "low", "text": "t"}]
    assert "chapter first: risks entry 2 is not an object; left out" in warnings


def test_tones_states_and_verdicts_across_the_page_are_checked(
    tmp_path, script
):
    timeline = {
        "id": "f2",
        "kind": "timeline",
        "spans": [{"from": 1, "to": 2, "label": "young", "tone": "added"}],
    }
    tree = {
        "id": "f3",
        "kind": "tree",
        "items": [{"path": "a", "state": "new"}],
    }
    design = {
        "summary": "s",
        "figures": [tree],
        "principles": [{"principle": "p", "verdict": "bent"}],
    }
    files = [{"path": "src/core.rs", "summary": "s", "risk": "critical"}]
    ci = {"checks": [{"name": "lint", "status": "red"}]}

    _, warnings = assembled(
        tmp_path,
        script,
        design=design,
        files=files,
        ci=ci,
        **chapter(figures=[timeline]),
    )

    assert any('spans[0]: tone "added" is not one of' in w for w in warnings)
    assert any('items[0]: state "new" is not one of' in w for w in warnings)
    assert any('verdict "bent" is not one of' in w for w in warnings)
    assert any(
        w.startswith('file src/core.rs: risk "critical" is not one of')
        for w in warnings
    )
    assert any('status "red" is not one of' in w for w in warnings)


def test_a_citation_the_build_cannot_check_is_a_note(tmp_path, script):
    build = script("build")
    risks = [{"level": "low", "text": "t", "where": "src/old.rs:3"}]

    _, warnings = build.assemble(workspace(tmp_path, **chapter(risks=risks)))

    (unverified,) = [w for w in warnings if "not a changed file" in w]
    assert isinstance(unverified, build.Note)


def test_a_file_without_a_summary_is_a_note(tmp_path, script):
    build = script("build")

    _, warnings = build.assemble(workspace(tmp_path, files=[]))

    (skimmed,) = [w for w in warnings if "no summary" in w]
    assert isinstance(skimmed, build.Note)


def test_a_defect_is_not_a_note(tmp_path, script):
    build = script("build")

    _, warnings = build.assemble(workspace(tmp_path, **chapter(figures=[])))

    assert warnings
    assert not any(isinstance(w, build.Note) for w in warnings)


def test_the_build_succeeds_when_only_notes_remain(tmp_path, script, capsys):
    root = workspace(tmp_path, files=[])

    status = script("build").main([str(root)])

    streams = capsys.readouterr()
    assert status == 0
    assert "note: src/core.rs: no summary" in streams.err
    assert "0 warnings, 1 note" in streams.out


def test_the_build_fails_while_a_warning_remains_but_writes_the_page(
    tmp_path, script, capsys
):
    root = workspace(tmp_path, **chapter(figures=[]))

    status = script("build").main([str(root)])

    streams = capsys.readouterr()
    assert status == 1
    assert "warning: chapter first: no figure" in streams.err
    assert "1 warning, 0 notes" in streams.out
    assert (root / "explore-pr-1.html").exists()


def test_the_hosted_build_fails_while_a_warning_remains(tmp_path, script):
    root = workspace(tmp_path / "ws", **chapter(figures=[]))

    status = script("build").main([str(root), "--hosted", str(tmp_path / "h")])

    assert status == 1
    assert (tmp_path / "h/index.html").exists()


def test_a_field_of_the_wrong_kind_is_reported_and_ignored(tmp_path, script):
    extra = {
        "risks": [{"level": "low", "text": "t", "refs": "R1-01", "where": 7}],
        "invariants": [{"id": 4, "text": "t", "pinnedBy": [["a"]]}],
        "tests": [{"file": "src/core.rs", "name": "a", "line": "1"}],
    }

    model, warnings = assembled(tmp_path, script, **chapter(**extra))

    first = model["chapters"][0]
    assert first["risks"] == [{"level": "low", "text": "t"}]
    assert first["invariants"] == [{"text": "t"}]
    assert first["tests"] == [{"file": "src/core.rs", "name": "a"}]
    assert (
        "chapter first: risks[0]: refs must be a list of text; ignored"
        in warnings
    )
    assert "chapter first: risks[0]: where must be text; ignored" in warnings
    assert any(
        "pinnedBy must be a list of text; ignored" in w for w in warnings
    )
    assert any("line must be a whole number; ignored" in w for w in warnings)


def test_a_figure_built_of_the_wrong_kinds_is_reported_not_fatal(
    tmp_path, script
):
    matrix = {
        "id": "f4",
        "kind": "matrix",
        "rows": ["r"],
        "cols": 2,
        "cells": ["x"],
    }

    _, warnings = assembled(tmp_path, script, **chapter(figures=[matrix]))

    assert any(
        "figures[f4]: cols must be a list; ignored" in w for w in warnings
    )
    assert any("figure f4: cols must be a list" in w for w in warnings)


def test_a_field_given_as_null_is_treated_as_absent(tmp_path, script):
    extra = {"risks": [{"level": "low", "text": "t", "refs": None}]}

    model, warnings = assembled(tmp_path, script, **chapter(**extra))

    assert model["chapters"][0]["risks"] == [{"level": "low", "text": "t"}]
    assert warnings == []
