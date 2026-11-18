import json


def added(*lines):
    return {"hunks": [{"o": 0, "n": 1, "l": [f"+{line}" for line in lines]}]}


def modified(*rows):
    return {"hunks": [{"o": 1, "n": 1, "l": list(rows)}]}


def lineage_of(script, earlier, current):
    return script("build").lineage_of(earlier, current)


def test_a_file_the_later_head_left_alone_is_the_same(script):
    diff = {"a.rs": added("x", "y")}

    assert lineage_of(script, diff, diff) == {"a.rs": "same"}


def test_lines_pushed_down_by_new_ones_are_carried_to_where_they_are(script):
    earlier = {"a.rs": added("x", "y", "z")}
    current = {"a.rs": added("n1", "n2", "x", "y", "z")}

    assert lineage_of(script, earlier, current) == {
        "a.rs": {"new": [[1, 3, 3]], "old": [], "changed": True}
    }


def test_a_rewritten_line_is_not_carried(script):
    earlier = {"a.rs": added("x", "y", "z")}
    current = {"a.rs": added("x", "Y", "z")}

    assert lineage_of(script, earlier, current) == {
        "a.rs": {"new": [[1, 1, 1], [3, 3, 1]], "old": [], "changed": True}
    }


def test_removed_lines_are_carried_through_the_base(script):
    earlier = {"a.rs": modified(" a", "-b", " c")}
    current = {"a.rs": modified(" top", " a", "-b", " c")}

    assert lineage_of(script, earlier, current) == {
        "a.rs": {"new": [], "old": [[2, 3, 1]], "changed": False}
    }


def test_a_file_whose_changes_only_moved_has_not_changed(script):
    earlier = {"a.rs": modified(" a", "+b", " c")}
    current = {"a.rs": modified(" top", " a", "+b", " c")}

    assert lineage_of(script, earlier, current)["a.rs"]["changed"] is False


def test_a_line_that_is_no_longer_a_change_is_not_carried(script):
    earlier = {"a.rs": modified(" a", "+b", "+c")}
    current = {"a.rs": modified(" a", " b", "+c")}

    assert lineage_of(script, earlier, current) == {
        "a.rs": {"new": [[3, 3, 1]], "old": [], "changed": True}
    }


def test_a_file_the_earlier_head_did_not_change_has_no_lineage(script):
    current = {"a.rs": added("x"), "b.rs": added("y")}

    assert lineage_of(script, {"a.rs": added("x")}, current) == {"a.rs": "same"}


def workspace(root, head, diff):
    (root / "content").mkdir(parents=True, exist_ok=True)
    (root / "data").mkdir(exist_ok=True)
    (root / "data/diff.json").write_text(json.dumps(diff))
    manifest = {"pr": {"number": 1, "head": head}} if head else {"pr": {}}
    (root / "explore.json").write_text(json.dumps(manifest))
    return root


def test_a_build_remembers_the_diff_at_its_head(tmp_path, script):
    build = script("build")
    diff = {"a.rs": added("x")}

    build.main([str(workspace(tmp_path, "sha-one", diff))])

    kept = json.loads((tmp_path / "data/heads/sha-one.json").read_text())
    assert kept == diff


def test_a_build_without_a_head_remembers_nothing(tmp_path, script):
    build = script("build")

    build.main([str(workspace(tmp_path, None, {"a.rs": added("x")}))])

    assert not (tmp_path / "data/heads").exists()


def test_the_page_carries_the_lineage_from_every_earlier_head(tmp_path, script):
    build = script("build")
    build.main([str(workspace(tmp_path, "sha-one", {"a.rs": added("x")}))])
    workspace(tmp_path, "sha-two", {"a.rs": added("n", "x")})
    build.main([str(tmp_path)])

    model, _ = build.assemble(tmp_path)

    assert model["lineage"] == {
        "sha-one": {"a.rs": {"new": [[1, 2, 1]], "old": [], "changed": True}}
    }


def test_a_first_build_has_no_lineage(tmp_path, script):
    build = script("build")
    build.main([str(workspace(tmp_path, "sha-one", {"a.rs": added("x")}))])

    model, _ = build.assemble(tmp_path)

    assert model["lineage"] == {}
