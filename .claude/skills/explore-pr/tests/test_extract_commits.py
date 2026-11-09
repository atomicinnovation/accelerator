def lines(*words):
    return "".join(f"{w}\n" for w in words)


def commit(sha, *files, parents=("p",)):
    return {
        "sha": sha,
        "subject": f"commit {sha}",
        "message": f"commit {sha}",
        "author": "a",
        "date": "d",
        "merge": len(parents) > 1,
        "files": [
            {"path": p, "status": s, "previous": prev} for p, s, prev in files
        ],
    }


def write(root, path, text):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)


def traced(tmp_path, script, base, versions, listed):
    extract = script("extract_commits")
    diff_of = script("extract_diff")
    commits = tmp_path / "commits"
    for path, text in base.items():
        write(commits / "base", path, text)
    for (sha, path), text in versions.items():
        write(commits / sha, path, text)
    head = {}
    for (_, path), text in versions.items():
        head[path] = text
    for c in listed:
        for f in c["files"]:
            if f["status"] == "removed":
                head.pop(f["path"], None)
            if f["previous"]:
                head.pop(f["previous"], None)
    renamed = {
        f["previous"]: f["path"]
        for c in listed
        for f in c["files"]
        if f["previous"]
    }
    base_tree = {renamed.get(p, p): t for p, t in base.items()}
    for path, text in base_tree.items():
        write(tmp_path / "base", path, text)
    for path, text in head.items():
        write(tmp_path / "head", path, text)
    paths = sorted(set(base_tree) | set(head))
    diff = diff_of.diff_tree(tmp_path / "base", tmp_path / "head", paths)
    return extract.trace(commits, listed, diff)["commits"]


def file_of(commits, sha, path):
    entry = next(c for c in commits if c["sha"] == sha)
    return next(f for f in entry["files"] if f["path"] == path)


def changes(file):
    return [line for h in file["hunks"] for line in h["l"] if line[0] in "+-"]


SUPERSEDED = (
    {"a.rs": lines("a", "b", "c", "d", "e")},
    {
        ("c1", "a.rs"): lines("a", "b", "C1", "d", "e", "X"),
        ("c2", "a.rs"): lines("a", "b", "C2", "d", "e", "X"),
    },
    [
        commit("c1", ("a.rs", "modified", None)),
        commit("c2", ("a.rs", "modified", None)),
    ],
)


def test_a_line_that_survives_to_head_takes_its_place_in_the_whole_diff(
    tmp_path, script
):
    commits = traced(tmp_path, script, *SUPERSEDED)

    c1 = file_of(commits, "c1", "a.rs")
    assert changes(c1) == ["-c", "+C1", "+X"]
    assert c1["fate"] == ["old:3", "~c2", "new:6"]


def test_a_line_removed_after_an_earlier_commit_added_it_names_that_commit(
    tmp_path, script
):
    commits = traced(tmp_path, script, *SUPERSEDED)

    c2 = file_of(commits, "c2", "a.rs")
    assert changes(c2) == ["-C1", "+C2"]
    assert c2["fate"] == ["^c1", "new:3"]


def test_a_commits_hunks_keep_three_lines_of_context(tmp_path, script):
    base = {"a.rs": lines(*[f"l{n}" for n in range(1, 21)])}
    after = lines(*[f"l{n}" if n != 10 else "L10" for n in range(1, 21)])

    commits = traced(
        tmp_path,
        script,
        base,
        {("c1", "a.rs"): after},
        [commit("c1", ("a.rs", "modified", None))],
    )

    hunk = file_of(commits, "c1", "a.rs")["hunks"][0]
    assert (hunk["o"], hunk["n"]) == (7, 7)
    assert hunk["l"] == [
        " l7",
        " l8",
        " l9",
        "-l10",
        "+L10",
        " l11",
        " l12",
        " l13",
    ]


def test_a_renamed_file_traces_onto_its_head_path(tmp_path, script):
    commits = traced(
        tmp_path,
        script,
        {"old.rs": lines("p", "q")},
        {("c1", "new.rs"): lines("p", "Q")},
        [commit("c1", ("new.rs", "renamed", "old.rs"))],
    )

    moved = file_of(commits, "c1", "new.rs")
    assert moved["headPath"] == "new.rs"
    assert moved["fate"] == ["old:2", "new:2"]


def test_a_line_removed_and_restored_is_unchanged_overall(tmp_path, script):
    commits = traced(
        tmp_path,
        script,
        {"a.rs": lines("a", "b", "c")},
        {("c1", "a.rs"): lines("a", "c"), ("c2", "a.rs"): lines("a", "b", "c")},
        [
            commit("c1", ("a.rs", "modified", None)),
            commit("c2", ("a.rs", "modified", None)),
        ],
    )

    assert file_of(commits, "c1", "a.rs")["fate"] == ["="]
    assert file_of(commits, "c2", "a.rs")["fate"] == ["="]


def test_a_deleted_file_traces_its_lines_onto_the_base(tmp_path, script):
    commits = traced(
        tmp_path,
        script,
        {"gone.rs": lines("g", "h")},
        {},
        [commit("c1", ("gone.rs", "removed", None))],
    )

    gone = file_of(commits, "c1", "gone.rs")
    assert gone["fate"] == ["old:1", "old:2"]
    assert (gone["add"], gone["del"]) == (0, 2)


def test_lines_of_a_file_a_later_commit_rewrites_are_superseded(
    tmp_path, script
):
    commits = traced(
        tmp_path,
        script,
        {},
        {("c1", "n.rs"): lines("n1", "n2"), ("c2", "n.rs"): lines("n1", "N2")},
        [
            commit("c1", ("n.rs", "added", None)),
            commit("c2", ("n.rs", "modified", None)),
        ],
    )

    assert file_of(commits, "c1", "n.rs")["fate"] == ["new:1", "~c2"]
    assert file_of(commits, "c2", "n.rs")["fate"] == ["^c1", "new:2"]


def test_a_merge_commit_carries_no_files(tmp_path, script):
    commits = traced(
        tmp_path,
        script,
        {"a.rs": lines("a")},
        {("c1", "a.rs"): lines("A")},
        [
            commit("c1", ("a.rs", "modified", None)),
            {**commit("m2", parents=("c1", "x")), "files": []},
        ],
    )

    merge = next(c for c in commits if c["sha"] == "m2")
    assert merge["merge"] is True
    assert merge["files"] == []


def test_lines_of_different_files_in_one_commit_keep_apart(tmp_path, script):
    commits = traced(
        tmp_path,
        script,
        {},
        {
            ("c1", "a.rs"): lines("a1", "a2"),
            ("c1", "b.rs"): lines("b1", "b2"),
            ("c2", "b.rs"): lines("B1", "b2"),
        },
        [
            commit("c1", ("a.rs", "added", None), ("b.rs", "added", None)),
            commit("c2", ("b.rs", "modified", None)),
        ],
    )

    assert file_of(commits, "c1", "a.rs")["fate"] == ["new:1", "new:2"]
    assert file_of(commits, "c1", "b.rs")["fate"] == ["~c2", "new:2"]


def test_an_identical_line_the_whole_diff_chose_instead_takes_the_change(
    tmp_path, script
):
    commits = traced(
        tmp_path,
        script,
        {"a.rs": lines("{", "{", "y")},
        {("c1", "a.rs"): lines("{", "y"), ("c2", "a.rs"): lines("{", "x")},
        [
            commit("c1", ("a.rs", "modified", None)),
            commit("c2", ("a.rs", "modified", None)),
        ],
    )

    assert file_of(commits, "c1", "a.rs")["fate"] == ["old:2"]
    assert file_of(commits, "c2", "a.rs")["fate"] == ["old:3", "new:2"]
