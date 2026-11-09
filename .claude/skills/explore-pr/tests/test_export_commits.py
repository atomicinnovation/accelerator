import json

COMMITS = [
    {
        "sha": "c1",
        "parents": ["p0"],
        "subject": "Add the core",
        "message": "Add the core\n\nWhy it exists.",
        "author": "a",
        "date": "2026-01-01T00:00:00Z",
    },
    {
        "sha": "m2",
        "parents": ["c1", "x9"],
        "subject": "Merge main",
        "message": "Merge main",
        "author": "a",
        "date": "2026-01-02T00:00:00Z",
    },
    {
        "sha": "c3",
        "parents": ["m2"],
        "subject": "Move and trim",
        "message": "Move and trim",
        "author": "a",
        "date": "2026-01-03T00:00:00Z",
    },
]
FILES_OF = {
    "c1": [
        {"filename": "src/core.rs", "status": "modified"},
        {"filename": "src/new.rs", "status": "added"},
    ],
    "c3": [
        {
            "filename": "src/moved.rs",
            "status": "renamed",
            "previous_filename": "src/old.rs",
        },
        {"filename": "src/core.rs", "status": "removed"},
    ],
}
BODIES = {
    ("BASE", "src/core.rs"): b"base core\n",
    ("BASE", "src/old.rs"): b"base old\n",
    ("c1", "src/core.rs"): b"c1 core\n",
    ("c1", "src/new.rs"): b"c1 new\n",
    ("c3", "src/moved.rs"): b"c3 moved\n",
}


def exported(tmp_path, script):
    export = script("export_commits")
    calls = []

    def fetch(ref, path):
        calls.append((ref, path))
        return BODIES.get((ref, path))

    listed = []

    def files_of(sha):
        listed.append(sha)
        return FILES_OF.get(sha, [])

    export.export(COMMITS, files_of, fetch, tmp_path, "BASE")
    return calls, listed


def test_each_commit_keeps_the_files_it_leaves_behind(tmp_path, script):
    exported(tmp_path, script)

    assert (tmp_path / "commits/c1/src/core.rs").read_bytes() == b"c1 core\n"
    assert (tmp_path / "commits/c1/src/new.rs").read_bytes() == b"c1 new\n"
    assert (tmp_path / "commits/c3/src/moved.rs").read_bytes() == b"c3 moved\n"
    assert not (tmp_path / "commits/c3/src/core.rs").exists()


def test_the_base_holds_each_file_as_the_first_commit_found_it(
    tmp_path, script
):
    calls, _ = exported(tmp_path, script)

    assert (tmp_path / "commits/base/src/core.rs").read_bytes() == (
        b"base core\n"
    )
    assert (tmp_path / "commits/base/src/old.rs").read_bytes() == b"base old\n"
    assert ("BASE", "src/new.rs") not in calls
    assert calls.count(("BASE", "src/core.rs")) == 1


def test_a_removed_file_is_not_fetched_after_its_commit(tmp_path, script):
    calls, _ = exported(tmp_path, script)

    assert ("c3", "src/core.rs") not in calls


def test_a_merge_commit_is_listed_but_not_exported(tmp_path, script):
    _, listed = exported(tmp_path, script)

    commits = json.loads((tmp_path / "data/commit-list.json").read_text())
    merge = next(c for c in commits if c["sha"] == "m2")
    assert merge["merge"] is True
    assert merge["files"] == []
    assert "m2" not in listed


def test_the_commit_list_keeps_the_pull_requests_order(tmp_path, script):
    exported(tmp_path, script)

    commits = json.loads((tmp_path / "data/commit-list.json").read_text())
    assert [c["sha"] for c in commits] == ["c1", "m2", "c3"]
    assert commits[0]["subject"] == "Add the core"
    assert commits[0]["message"] == "Add the core\n\nWhy it exists."
    assert commits[2]["files"] == [
        {"path": "src/moved.rs", "status": "renamed", "previous": "src/old.rs"},
        {"path": "src/core.rs", "status": "removed", "previous": None},
    ]


def test_a_commit_path_cannot_escape_its_tree(tmp_path, script):
    export = script("export_commits")
    commits = [{**COMMITS[0], "sha": "c1"}]

    export.export(
        commits,
        lambda sha: [{"filename": "../../outside.rs", "status": "added"}],
        lambda ref, path: b"x",
        tmp_path / "ws",
        "BASE",
    )

    assert not (tmp_path / "outside.rs").exists()
