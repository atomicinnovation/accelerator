FILES = {
    ("BASE", "src/kept.rs"): b"old kept\n",
    ("HEAD", "src/kept.rs"): b"new kept\n",
    ("HEAD", "src/added.rs"): b"added\n",
    ("BASE", "src/gone.rs"): b"gone\n",
    ("BASE", "src/before.rs"): b"moved\n",
    ("HEAD", "src/after.rs"): b"moved and edited\n",
}
ENTRIES = [
    {"filename": "src/kept.rs", "status": "modified"},
    {"filename": "src/added.rs", "status": "added"},
    {"filename": "src/gone.rs", "status": "removed"},
    {
        "filename": "src/after.rs",
        "status": "renamed",
        "previous_filename": "src/before.rs",
    },
    {"filename": "src/missing.rs", "status": "modified"},
]


def fetched():
    calls = []

    def fetch(ref, path):
        calls.append((ref, path))
        return FILES.get((ref, path))

    return fetch, calls


def test_each_side_holds_the_files_that_exist_on_it(tmp_path, script):
    export = script("export_trees")
    fetch, _ = fetched()

    export.export(ENTRIES, fetch, tmp_path, "BASE", "HEAD")

    assert (tmp_path / "base/src/kept.rs").read_bytes() == b"old kept\n"
    assert (tmp_path / "head/src/kept.rs").read_bytes() == b"new kept\n"
    assert not (tmp_path / "base/src/added.rs").exists()
    assert not (tmp_path / "head/src/gone.rs").exists()


def test_a_renamed_file_keeps_its_old_body_under_its_new_name(tmp_path, script):
    export = script("export_trees")
    fetch, _ = fetched()

    export.export(ENTRIES, fetch, tmp_path, "BASE", "HEAD")

    assert (tmp_path / "base/src/after.rs").read_bytes() == b"moved\n"
    assert not (tmp_path / "base/src/before.rs").exists()


def test_no_side_is_fetched_where_the_status_rules_it_out(tmp_path, script):
    export = script("export_trees")
    fetch, calls = fetched()

    export.export(ENTRIES, fetch, tmp_path, "BASE", "HEAD")

    assert ("BASE", "src/added.rs") not in calls
    assert ("HEAD", "src/gone.rs") not in calls


def test_the_changed_file_list_keeps_the_pull_requests_order(tmp_path, script):
    export = script("export_trees")
    fetch, _ = fetched()

    export.export(ENTRIES, fetch, tmp_path, "BASE", "HEAD")

    assert (tmp_path / "data/changed-files.txt").read_text().splitlines() == [
        "src/kept.rs",
        "src/added.rs",
        "src/gone.rs",
        "src/after.rs",
        "src/missing.rs",
    ]


def test_a_path_cannot_escape_its_tree(tmp_path, script):
    export = script("export_trees")
    hostile = [{"filename": "../../outside.rs", "status": "added"}]

    export.export(hostile, lambda ref, path: b"x", tmp_path / "ws", "B", "H")

    assert not (tmp_path / "outside.rs").exists()
