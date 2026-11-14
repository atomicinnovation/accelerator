import json

HEAD = [
    "fn a() {",
    "    claim();",
    "}",
    "",
    "fn b() {",
    "    claim();",
    "}",
    "fn c() {}",
]
DIFF = {
    "src/core.rs": {
        "status": "added",
        "add": len(HEAD),
        "del": 0,
        "hunks": [{"o": 0, "n": 1, "l": [f"+{line}" for line in HEAD]}],
    },
    "src/gone.rs": {
        "status": "removed",
        "add": 0,
        "del": 2,
        "hunks": [{"o": 1, "n": 0, "l": ["-fn old() {", "-}"]}],
    },
}


def stop(**fields):
    return {"id": "s1", "file": "src/core.rs", "start": 1, "end": 3, **fields}


def row(**fields):
    return {"id": "L1", "file": "src/core.rs", "line": 2, **fields}


def workspace(root, stops=(), ledger=()):
    (root / "content").mkdir(parents=True)
    (root / "data").mkdir()
    (root / "data/diff.json").write_text(json.dumps(DIFF))
    (root / "explore.json").write_text(json.dumps({"pr": {"number": 1}}))
    chapter = {"id": "first", "title": "First", "stops": list(stops)}
    (root / "content/chapter-first.json").write_text(json.dumps(chapter))
    (root / "content/ledger-a.json").write_text(json.dumps(list(ledger)))
    return root


def anchor_warnings(root, script, **content):
    _, warnings = script("build").assemble(workspace(root, **content))
    return [w for w in warnings if "anchor" in w]


def stops_of(root):
    return json.loads((root / "content/chapter-first.json").read_text())[
        "stops"
    ]


def test_a_stop_whose_anchor_is_on_its_first_line_raises_nothing(
    tmp_path, script
):
    assert (
        anchor_warnings(tmp_path, script, stops=[stop(anchor="fn a()")]) == []
    )


def test_an_anchor_matches_whatever_the_spacing(tmp_path, script):
    stops = [stop(anchor="  fn   a()\t{ ")]

    assert anchor_warnings(tmp_path, script, stops=stops) == []


def test_a_stop_without_an_anchor_is_reported(tmp_path, script):
    warnings = anchor_warnings(tmp_path, script, stops=[stop()])

    assert warnings == ["stop s1: no anchor; quote the code on line 1"]


def test_a_blank_anchor_is_no_anchor(tmp_path, script):
    warnings = anchor_warnings(tmp_path, script, stops=[stop(anchor="  ")])

    assert warnings == ["stop s1: no anchor; quote the code on line 1"]


def test_a_stop_may_anchor_on_a_later_line_of_its_range(tmp_path, script):
    stops = [stop(anchor="claim();", anchorLine=2)]

    assert anchor_warnings(tmp_path, script, stops=stops) == []


def test_an_anchor_line_outside_its_stop_is_reported(tmp_path, script):
    stops = [stop(anchor="fn b()", anchorLine=5)]

    warnings = anchor_warnings(tmp_path, script, stops=stops)

    assert warnings == ["stop s1: anchor line 5 is outside its lines 1-3"]


def test_an_anchor_found_once_elsewhere_has_moved(tmp_path, script):
    stops = [stop(start=2, end=4, anchor="fn b()")]

    warnings = anchor_warnings(tmp_path, script, stops=stops)

    assert warnings == [
        "stop s1: anchor moved from line 2 to line 5 (+3); re-anchor it"
    ]


def test_an_anchor_found_in_several_other_places_is_ambiguous(tmp_path, script):
    stops = [stop(anchor="claim();")]

    warnings = anchor_warnings(tmp_path, script, stops=stops)

    assert warnings == [
        "stop s1: anchor is not on line 1 but on lines 2, 6; "
        "choose a line only it has"
    ]


def test_an_anchor_the_file_no_longer_has_is_reported(tmp_path, script):
    stops = [stop(anchor="fn z()")]

    warnings = anchor_warnings(tmp_path, script, stops=stops)

    assert warnings == [
        "stop s1: anchor not found in src/core.rs; "
        "the code changed, so re-read it"
    ]


def test_a_stop_on_removed_code_is_anchored_in_the_base(tmp_path, script):
    held = stop(file="src/gone.rs", side="old", end=2, anchor="fn old()")
    lost = stop(id="s2", file="src/gone.rs", side="old", end=2, anchor="fn a()")

    warnings = anchor_warnings(tmp_path, script, stops=[held, lost])

    assert warnings == [
        "stop s2: anchor not found in src/gone.rs; "
        "the code changed, so re-read it"
    ]


def test_a_ledger_row_is_anchored_on_the_line_of_its_call(tmp_path, script):
    rows = [
        row(anchor="claim();"),
        row(id="L2"),
        row(id="L3", line=4, anchor="fn b()"),
    ]

    warnings = anchor_warnings(tmp_path, script, ledger=rows)

    assert warnings == [
        "ledger L2: no anchor; quote the code on line 2",
        "ledger L3: anchor moved from line 4 to line 5 (+1); re-anchor it",
    ]


def test_reanchoring_moves_a_stop_with_its_anchor(tmp_path, script):
    moved = stop(start=2, end=4, anchor="fn b()")
    root = workspace(tmp_path, stops=[moved])

    report = script("build").reanchor(root)

    assert report == ["stop s1: moved +3 to lines 5-7"]
    assert stops_of(root) == [stop(start=5, end=7, anchor="fn b()")]


def test_reanchoring_moves_an_anchor_line_with_its_stop(tmp_path, script):
    moved = stop(start=4, end=6, anchor="fn c()", anchorLine=5)
    root = workspace(tmp_path, stops=[moved])

    script("build").reanchor(root)

    assert stops_of(root) == [
        stop(start=7, end=9, anchor="fn c()", anchorLine=8)
    ]


def test_reanchoring_moves_a_ledger_row_and_the_range_read_with_it(
    tmp_path, script
):
    rows = [row(line=4, start=3, end=4, anchor="fn b()"), row(anchor="fn c()")]
    root = workspace(tmp_path, ledger=rows)

    report = script("build").reanchor(root)

    assert report == [
        "ledger L1: moved +1 to line 5",
        "ledger L1: moved +6 to line 8",
    ]
    assert json.loads((root / "content/ledger-a.json").read_text()) == [
        row(line=5, start=4, end=5, anchor="fn b()"),
        row(line=8, anchor="fn c()"),
    ]


def test_reanchoring_leaves_what_it_cannot_place_alone(tmp_path, script):
    stops = [
        stop(anchor="claim();"),
        stop(id="s2", anchor="fn z()"),
        stop(id="s3"),
        stop(id="s4", anchor="fn a()"),
    ]
    root = workspace(tmp_path, stops=stops)
    written = (root / "content/chapter-first.json").read_text()

    report = script("build").reanchor(root)

    assert report == []
    assert (root / "content/chapter-first.json").read_text() == written


def test_the_command_line_reanchors_before_it_builds(tmp_path, script, capsys):
    moved = stop(start=2, end=4, anchor="fn b()", fit="f")
    root = workspace(tmp_path, stops=[moved])

    script("build").main([str(root), "--reanchor"])

    streams = capsys.readouterr()
    assert "reanchored stop s1: moved +3 to lines 5-7" in streams.out
    assert "anchor" not in streams.err
    assert stops_of(root)[0]["start"] == 5


PARTLY_CARRIED = {
    "tests/long.rs": {
        "status": "modified",
        "full": False,
        "add": 1,
        "del": 0,
        "hunks": [
            {"o": 99, "n": 99, "l": [" fn near() {", "+    claim();", " }"]}
        ],
    }
}


def partly_carried(root, script, stops):
    workspace(root, stops=stops)
    (root / "data/diff.json").write_text(json.dumps(PARTLY_CARRIED))
    _, warnings = script("build").assemble(root)
    return [w for w in warnings if "anchor" in w or "not in the page" in w]


def test_a_stop_on_lines_a_long_file_does_not_carry_says_they_cannot_be_shown(
    tmp_path, script
):
    far = stop(file="tests/long.rs", start=20, end=24, anchor="fn far()")

    warnings = partly_carried(tmp_path, script, [far])

    assert warnings == [
        "stop s1: lines 20-24 of tests/long.rs are not in the page: the file "
        "is too long to carry whole and these are far from any change; "
        "cite lines the diff holds"
    ]


def test_a_stop_on_lines_a_long_file_does_carry_is_anchored_as_usual(
    tmp_path, script
):
    near = stop(file="tests/long.rs", start=99, end=101, anchor="fn near()")

    assert partly_carried(tmp_path, script, [near]) == []
