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
        "kind": "tree",
        "root": "src/",
        "items": [{"path": "core.rs", "where": "src/core.rs:99"}],
    }
    figure = {"id": "f-cmp", "kind": "compare", "after": after}

    warnings = warnings_for(tmp_path, script, overview={"figures": [figure]})

    assert about(
        warnings, "overview", "figures[f-cmp].after.items[0]", "outside 1-3"
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


def test_a_module_map_box_linked_to_a_missing_chapter_is_reported(
    tmp_path, script
):
    modmap = {"nodes": [{"id": "m", "label": "M", "chapter": "nope"}]}

    warnings = warnings_for(tmp_path, script, [chapter()], modmap=modmap)

    assert about(warnings, "module map", "modules[m]", "missing chapter nope")


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


def test_a_test_a_long_file_may_hold_out_of_sight_is_noted_not_refused(
    tmp_path, script
):
    build = script("build")
    (tmp_path / "content").mkdir(parents=True)
    (tmp_path / "data").mkdir()
    partly_carried = {
        "tests/long.rs": {
            "status": "modified",
            "full": False,
            "add": 1,
            "del": 0,
            "hunks": [{"o": 99, "n": 99, "l": ["+fn seen_here() {}"]}],
        }
    }
    (tmp_path / "data/diff.json").write_text(json.dumps(partly_carried))
    (tmp_path / "explore.json").write_text(json.dumps({"pr": {"number": 1}}))
    cited = chapter(
        tests=[
            {"file": "tests/long.rs", "name": "seen_here"},
            {"file": "tests/long.rs", "name": "far"},
        ]
    )
    (tmp_path / "content/chapter-claims.json").write_text(json.dumps(cited))

    _, warnings = build.assemble(tmp_path)

    (noted,) = [w for w in warnings if "far" in w]
    assert isinstance(noted, build.Note)
    assert "too long to carry whole; unverified" in noted
    assert not [w for w in warnings if "seen_here" in w]


REVIEW = [
    {
        "id": "R1-06",
        "severity": "major",
        "summary": "Drops an unreadable claim.",
        "file": "src/core.rs",
        "start": 1,
        "end": 3,
    }
]


def observation(observation_id, **fields):
    return {
        "id": observation_id,
        "severity": "minor",
        "level": "implementation",
        "title": "A claim can go stale",
        "summary": "It ages out early.",
        **fields,
    }


def register(script, chapters=(), design=None, findings=REVIEW):
    return script("build").findings_register(
        findings, list(chapters), design or {}, DIFF
    )


def named(findings, finding_id):
    return next(f for f in findings if f["id"] == finding_id)


def test_a_prior_finding_comes_from_the_review(script):
    (prior,) = register(script)["findings"]

    assert prior["source"] == "review"


def test_an_observation_becomes_a_finding_where_it_points(script):
    seen = observation("P-claims-01", where="other/core.rs:1")
    claims = chapter(observations=[seen])

    found = named(register(script, [claims])["findings"], "P-claims-01")

    assert found["source"] == "reading-pass"
    assert found["chapter"] == "claims"
    assert (found["file"], found["start"], found["end"]) == (
        "src/other/core.rs",
        1,
        1,
    )
    assert found["kind"] == "inline"


def test_an_observation_without_a_place_is_general(script):
    claims = chapter(observations=[observation("P-claims-01")])

    found = named(register(script, [claims])["findings"], "P-claims-01")

    assert found["kind"] == "general"


def test_the_design_pass_raises_observations_of_its_own(script):
    design = {"observations": [observation("D-01")]}

    found = named(register(script, design=design)["findings"], "D-01")

    assert (found["source"], found["chapter"]) == ("design-pass", None)


def test_the_same_observation_from_two_passes_is_one_finding(script):
    first = chapter(observations=[observation("P-claims-01")])
    second = dict(
        chapter(
            observations=[observation("P-store-02", sameAs=["P-claims-01"])]
        ),
        id="store",
    )

    registered = register(script, [first, second])

    ids = [f["id"] for f in registered["findings"]]
    assert ids == ["R1-06", "P-claims-01"]
    assert named(registered["findings"], "P-claims-01")["alsoRaisedIn"] == [
        "store"
    ]
    assert registered["aliases"] == {"P-store-02": "P-claims-01"}


def test_a_repeat_of_a_prior_finding_belongs_in_its_check(script):
    repeat = observation("P-claims-01", sameAs=["R1-06"])

    warnings = register(script, [chapter(observations=[repeat])])["warnings"]

    assert about(warnings, "P-claims-01", "sameAs names R1-06", "findingChecks")


def test_a_link_to_a_finding_that_does_not_exist_is_reported(script):
    links = observation("P-claims-01", extends="R9-99", distinctFrom=["R8"])

    warnings = register(script, [chapter(observations=[links])])["warnings"]

    assert about(warnings, "P-claims-01", "extends names unknown finding R9-99")
    assert about(warnings, "distinctFrom names unknown finding R8")


def test_unlinked_findings_on_the_same_lines_are_reported(script):
    near = observation("P-claims-01", where="src/core.rs:2-5")

    warnings = register(script, [chapter(observations=[near])])["warnings"]

    assert (
        "R1-06 and P-claims-01 cite overlapping lines of src/core.rs (2-3); "
        "link them with sameAs, extends or distinctFrom" in warnings
    )


def test_findings_on_the_same_lines_may_say_how_they_relate(script):
    for link in ({"extends": "R1-06"}, {"distinctFrom": ["R1-06"]}):
        near = observation("P-claims-01", where="src/core.rs:2", **link)

        warnings = register(script, [chapter(observations=[near])])["warnings"]

        assert about(warnings, "overlapping") == [], link


def test_two_prior_findings_on_the_same_lines_are_left_alone(script):
    twice = [*REVIEW, dict(REVIEW[0], id="R1-07")]

    assert register(script, findings=twice)["warnings"] == []


def test_old_notes_are_still_findings_under_their_old_names(script):
    old = [{"level": "major", "text": "Stale.", "where": "src/core.rs:2"}]

    registered = register(script, [chapter(ownNotes=old)])

    found = named(registered["findings"], "N-claims-1")
    assert (found["severity"], found["summary"]) == ("major", "Stale.")
    assert registered["renames"] == {"own:claims-1": "finding:N-claims-1"}
    assert about(registered["warnings"], "ownNotes is the old name")
    assert all(
        type(w).__name__ == "Note"
        for w in about(registered["warnings"], "ownNotes")
    )


def test_an_observation_may_be_cited_wherever_a_finding_may(tmp_path, script):
    seen = observation("P-claims-01")
    risks = [{"level": "low", "text": "t", "refs": ["P-claims-01"]}]

    warnings = warnings_for(
        tmp_path, script, [chapter(observations=[seen], risks=risks)]
    )

    assert about(warnings, "unknown finding") == []


def test_an_observation_s_severity_and_level_are_checked(tmp_path, script):
    odd = observation("P-claims-01", severity="huge", level="vibe")

    warnings = warnings_for(tmp_path, script, [chapter(observations=[odd])])

    assert about(warnings, "observations[P-claims-01]", 'severity "huge"')
    assert about(warnings, "observations[P-claims-01]", 'level "vibe"')
