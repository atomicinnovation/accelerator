import json


def write(root, path, text):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)


def numbered(count):
    return "".join(f"line {n}\n" for n in range(1, count + 1))


def test_a_modified_source_file_keeps_its_whole_body_as_context(
    tmp_path, script
):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(base, "src/app.rs", numbered(40))
    write(head, "src/app.rs", numbered(40).replace("line 20\n", "line XX\n"))

    diff = extract.diff_tree(base, head, ["src/app.rs"])

    entry = diff["src/app.rs"]
    assert entry["status"] == "modified"
    assert (entry["add"], entry["del"]) == (1, 1)
    assert len(entry["hunks"]) == 1
    hunk = entry["hunks"][0]
    assert (hunk["o"], hunk["n"]) == (1, 1)
    assert hunk["l"][0] == " line 1"
    assert "-line 20" in hunk["l"]
    assert "+line XX" in hunk["l"]
    assert hunk["l"][-1] == " line 40"


def test_added_and_deleted_files_are_classified_by_which_side_exists(
    tmp_path, script
):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(base, "old.py", "a\nb\n")
    write(head, "new.py", "c\n")

    diff = extract.diff_tree(base, head, ["old.py", "new.py"])

    assert diff["old.py"]["status"] == "deleted"
    assert diff["old.py"]["del"] == 2
    assert diff["new.py"]["status"] == "added"
    assert diff["new.py"]["hunks"][0]["l"] == ["+c"]


def test_a_lockfile_keeps_only_a_few_lines_of_context(tmp_path, script):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(base, "Cargo.lock", numbered(100))
    write(head, "Cargo.lock", numbered(100).replace("line 50\n", "line X\n"))

    hunk = extract.diff_tree(base, head, ["Cargo.lock"])["Cargo.lock"]["hunks"][
        0
    ]

    assert hunk["o"] == 45
    assert len([line for line in hunk["l"] if line[0] == " "]) == 10


def test_a_binary_file_is_flagged_and_carries_no_hunks(tmp_path, script):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    (head / "img").mkdir(parents=True)
    (head / "img/logo.png").write_bytes(b"\x89PNG\x00\x01")

    entry = extract.diff_tree(base, head, ["img/logo.png"])["img/logo.png"]

    assert entry["binary"] is True
    assert entry["hunks"] == []


def test_a_removed_line_starting_with_dashes_is_not_taken_for_a_header(
    tmp_path, script
):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(base, "q.sql", "-- comment\nselect 1;\n")
    write(head, "q.sql", "select 1;\n")

    lines = extract.diff_tree(base, head, ["q.sql"])["q.sql"]["hunks"][0]["l"]

    assert lines == ["--- comment", " select 1;"]


def test_the_command_line_writes_json_for_every_listed_path(tmp_path, script):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(head, "a.md", "x\n")
    listing = tmp_path / "files.txt"
    listing.write_text("a.md\n\n")
    out = tmp_path / "diff.json"

    extract.main([str(base), str(head), str(listing), str(out)])

    assert list(json.loads(out.read_text())) == ["a.md"]


def changed_at(count, line):
    return numbered(count).replace(f"line {line}\n", "line XX\n")


def test_a_long_source_file_still_keeps_its_whole_body(tmp_path, script):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(base, "tests/sync_create.rs", numbered(6000))
    write(head, "tests/sync_create.rs", changed_at(6000, 3000))

    entry = extract.diff_tree(base, head, ["tests/sync_create.rs"])[
        "tests/sync_create.rs"
    ]

    assert entry["full"] is True
    assert entry["hunks"][0]["l"][0] == " line 1"
    assert entry["hunks"][-1]["l"][-1] == " line 6000"


def test_a_file_too_long_to_carry_whole_keeps_only_what_is_near_a_change(
    tmp_path, script
):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(base, "src/generated.rs", numbered(25_000))
    write(head, "src/generated.rs", changed_at(25_000, 12_000))

    entry = extract.diff_tree(base, head, ["src/generated.rs"])[
        "src/generated.rs"
    ]

    assert entry["full"] is False
    assert len(entry["hunks"]) == 1
    assert entry["hunks"][0]["l"][0] == " line 11995"
    assert entry["hunks"][0]["l"][-1] == " line 12005"


def test_a_lockfile_keeps_only_what_is_near_a_change_however_short(
    tmp_path, script
):
    extract = script("extract_diff")
    base, head = tmp_path / "base", tmp_path / "head"
    write(base, "Cargo.lock", numbered(40))
    write(head, "Cargo.lock", changed_at(40, 20))

    entry = extract.diff_tree(base, head, ["Cargo.lock"])["Cargo.lock"]

    assert entry["full"] is False
    assert entry["hunks"][0]["l"][0] == " line 15"
