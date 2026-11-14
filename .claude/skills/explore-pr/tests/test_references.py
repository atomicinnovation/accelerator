import json

DIFF = {
    "src/core.rs": {
        "status": "modified",
        "add": 1,
        "del": 1,
        "hunks": [{"o": 1, "n": 1, "l": [" fn a() {", "-old", "+new", " }"]}],
    },
    "src/other/core.rs": {
        "status": "added",
        "add": 1,
        "del": 0,
        "hunks": [{"o": 0, "n": 1, "l": ["+fn b() {}"]}],
    },
    "src/gone.rs": {
        "status": "removed",
        "add": 0,
        "del": 2,
        "hunks": [{"o": 1, "n": 0, "l": ["-fn c() {", "-}"]}],
    },
    "tests/core_test.rs": {
        "status": "added",
        "add": 2,
        "del": 0,
        "hunks": [{"o": 0, "n": 1, "l": ["+fn claims_twice() {", "+}"]}],
    },
}


def chapter(**fields):
    return {"id": "claims", "title": "Claims", "stops": [], **fields}


def warnings_for(root, script, chapters=(), **content):
    (root / "content").mkdir(parents=True)
    (root / "data").mkdir()
    (root / "data/diff.json").write_text(json.dumps(DIFF))
    (root / "explore.json").write_text(json.dumps({"pr": {"number": 1}}))
    for item in chapters:
        (root / f"content/chapter-{item['id']}.json").write_text(
            json.dumps(item)
        )
    for name, value in content.items():
        (root / f"content/{name.replace('_', '-')}.json").write_text(
            json.dumps(value)
        )
    _, warnings = script("build").assemble(root)
    return warnings


def about(warnings, *fragments):
    return [w for w in warnings if all(f in w for f in fragments)]


def test_a_where_inside_a_changed_file_raises_nothing(tmp_path, script):
    risks = [{"level": "low", "text": "t", "where": "src/core.rs:3"}]

    warnings = warnings_for(tmp_path, script, [chapter(risks=risks)])

    assert about(warnings, "where") == []


def test_a_where_past_the_end_of_its_file_is_reported(tmp_path, script):
    risks = [{"level": "low", "text": "t", "where": "src/core.rs:4"}]

    warnings = warnings_for(tmp_path, script, [chapter(risks=risks)])

    assert about(
        warnings, "chapter claims", "risks[0]", "src/core.rs:4", "outside 1-3"
    )


def test_a_where_range_must_end_inside_its_file(tmp_path, script):
    risks = [{"level": "low", "text": "t", "where": "src/core.rs:2-9"}]

    warnings = warnings_for(tmp_path, script, [chapter(risks=risks)])

    assert about(warnings, "src/core.rs:2-9", "outside 1-3")


def test_a_where_in_a_removed_file_is_checked_against_its_base(
    tmp_path, script
):
    risks = [
        {"level": "low", "text": "t", "where": "src/gone.rs:2"},
        {"level": "low", "text": "t", "where": "src/gone.rs:3"},
    ]

    warnings = warnings_for(tmp_path, script, [chapter(risks=risks)])

    assert about(warnings, "src/gone.rs:2") == []
    assert about(warnings, "src/gone.rs:3", "outside 1-2")


def test_a_where_that_is_not_a_path_and_line_is_reported(tmp_path, script):
    decisions = [{"id": "d-one", "decision": "d", "where": "the core module"}]

    warnings = warnings_for(tmp_path, script, [chapter(decisions=decisions)])

    assert about(warnings, "decisions[d-one]", "is not path:line")


def test_a_where_in_an_unchanged_file_is_reported_as_unverified(
    tmp_path, script
):
    design = {"principles": [{"principle": "p", "where": "src/old.rs:10"}]}

    warnings = warnings_for(tmp_path, script, design=design)

    assert about(
        warnings, "design", "principles[0]", "src/old.rs", "not a changed file"
    )


def test_a_where_may_name_a_changed_file_by_a_unique_suffix(tmp_path, script):
    risks = [
        {"level": "low", "text": "t", "where": "core_test.rs:2"},
        {"level": "low", "text": "t", "where": "core_test.rs:3"},
    ]

    warnings = warnings_for(tmp_path, script, [chapter(risks=risks)])

    assert about(warnings, "core_test.rs:2") == []
    assert about(
        warnings, "core_test.rs:3", "outside 1-2 of tests/core_test.rs"
    )


def test_a_where_suffix_matching_several_files_is_reported(tmp_path, script):
    risks = [{"level": "low", "text": "t", "where": "core.rs:1"}]

    warnings = warnings_for(tmp_path, script, [chapter(risks=risks)])

    assert about(
        warnings, "core.rs:1", "more than one", "src/core.rs, src/other/core.rs"
    )


def test_a_where_on_a_figure_inside_a_comparison_is_checked(tmp_path, script):
    after = {
        "id": "f-after",
        "kind": "flow",
        "nodes": [{"id": "a", "label": "A", "where": "src/core.rs:99"}],
        "edges": [],
    }
    figure = {"id": "f-cmp", "kind": "compare", "after": after}

    warnings = warnings_for(tmp_path, script, overview={"figures": [figure]})

    assert about(
        warnings, "overview", "figures[f-cmp].after.nodes[a]", "outside 1-3"
    )


def test_a_breaking_change_is_checked_where_it_is_and_in_the_changelog(
    tmp_path, script
):
    breaking = [
        {"id": "B1", "where": "src/core.rs:8", "changelog": "src/core.rs:9"}
    ]

    warnings = warnings_for(tmp_path, script, breaking=breaking)

    assert about(warnings, "breaking", "[B1]", "where src/core.rs:8")
    assert about(warnings, "breaking", "[B1]", "changelog src/core.rs:9")


def test_a_test_reference_found_in_a_changed_file_raises_nothing(
    tmp_path, script
):
    invariants = [
        {"id": "i1", "pinnedBy": ["tests/core_test.rs::claims_twice"]}
    ]

    warnings = warnings_for(tmp_path, script, [chapter(invariants=invariants)])

    assert about(warnings, "test ") == []


def test_a_pinning_test_its_file_does_not_declare_is_reported(tmp_path, script):
    invariants = [{"id": "i1", "pinnedBy": ["tests/core_test.rs::claims_once"]}]

    warnings = warnings_for(tmp_path, script, [chapter(invariants=invariants)])

    assert about(
        warnings,
        "chapter claims",
        "invariants[i1]",
        "test claims_once",
        "not found in tests/core_test.rs",
    )


def test_a_test_named_through_its_modules_is_found_by_its_last_name(
    tmp_path, script
):
    invariants = [
        {"id": "i1", "pinnedBy": ["core_test.rs::tests::claims_twice"]}
    ]

    warnings = warnings_for(tmp_path, script, [chapter(invariants=invariants)])

    assert about(warnings, "test ") == []


def test_a_test_in_an_unchanged_file_is_reported_as_unverified(
    tmp_path, script
):
    row = {
        "id": "L1",
        "file": "src/core.rs",
        "line": 2,
        "tests": ["tests/old_test.rs::holds"],
    }

    warnings = warnings_for(tmp_path, script, ledger=[row])

    assert about(
        warnings, "ledger", "[L1]", "tests/old_test.rs", "not a changed file"
    )


def test_a_chapter_test_is_checked_by_name_and_line(tmp_path, script):
    tests = [
        {"file": "tests/core_test.rs", "name": "claims_twice", "line": 1},
        {"file": "tests/core_test.rs", "name": "claims_twice", "line": 7},
        {"file": "tests/core_test.rs", "name": "missing", "line": 1},
    ]

    warnings = warnings_for(tmp_path, script, [chapter(tests=tests)])

    assert len(about(warnings, "test ")) == 2
    assert about(warnings, "tests[1]", "line 7 is outside 1-2")
    assert about(warnings, "tests[2]", "test missing not found")


def test_a_file_assigned_to_a_missing_chapter_is_reported(tmp_path, script):
    files = [{"path": "src/core.rs", "chapter": "nope", "summary": "s"}]

    warnings = warnings_for(tmp_path, script, [chapter()], files=files)

    assert about(warnings, "file src/core.rs", "missing chapter nope")


def test_a_file_assigned_to_a_chapter_that_exists_raises_nothing(
    tmp_path, script
):
    files = [{"path": "src/core.rs", "chapter": "claims", "summary": "s"}]

    warnings = warnings_for(tmp_path, script, [chapter()], files=files)

    assert about(warnings, "missing chapter") == []


def test_a_figure_box_linked_to_a_missing_chapter_is_reported(tmp_path, script):
    figure = {
        "id": "f1",
        "kind": "flow",
        "nodes": [{"id": "a", "label": "A", "chapter": "nope"}],
        "edges": [],
    }

    warnings = warnings_for(tmp_path, script, [chapter(figures=[figure])])

    assert about(warnings, "figures[f1].nodes[a]", "missing chapter nope")


def test_a_module_map_box_linked_to_a_missing_chapter_is_reported(
    tmp_path, script
):
    modmap = {"nodes": [{"id": "m", "label": "M", "chapter": "nope"}]}

    warnings = warnings_for(tmp_path, script, [chapter()], modmap=modmap)

    assert about(warnings, "module map", "nodes[m]", "missing chapter nope")


def test_prose_naming_a_stop_that_exists_raises_nothing(tmp_path, script):
    stop = {"id": "claims-s1", "file": "src/core.rs", "start": 1, "end": 2}
    overview = {"lede": "Start at `claims-s1`, in chapter `claims`."}

    warnings = warnings_for(
        tmp_path, script, [chapter(stops=[stop])], overview=overview
    )

    assert about(warnings, "unknown") == []


def test_prose_naming_a_stop_its_chapter_does_not_have_is_reported(
    tmp_path, script
):
    stop = {
        "id": "claims-s1",
        "file": "src/core.rs",
        "start": 1,
        "end": 2,
        "note": "Continues in claims-s7 and `claims-s7`.",
    }

    warnings = warnings_for(tmp_path, script, [chapter(stops=[stop])])

    assert len(about(warnings, "unknown stop claims-s7")) == 1
    assert about(warnings, "chapter claims", "stops[claims-s1].note")


def test_prose_naming_a_chapter_that_does_not_exist_is_reported(
    tmp_path, script
):
    design = {
        "summary": "s",
        "signOff": "See chapters `claims` and `retention`.",
    }

    warnings = warnings_for(tmp_path, script, [chapter()], design=design)

    assert about(warnings, "design", "signOff", "unknown chapter retention")
    assert about(warnings, "unknown chapter claims") == []


def test_code_after_the_word_chapter_that_is_no_reference_is_left_alone(
    tmp_path, script
):
    overview = {"lede": "Each chapter `README.md` and chapter `Clock` differ."}

    warnings = warnings_for(tmp_path, script, [chapter()], overview=overview)

    assert about(warnings, "unknown chapter") == []
