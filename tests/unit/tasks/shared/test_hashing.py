import os
from pathlib import Path

import pytest

from tasks.shared.hashing import TreeChanges, compute_sha256, tree_digest

_FIXTURES = Path(__file__).resolve().parent.parent / "fixtures"


class TestComputeSha256:
    def test_empty_file(self, tmp_path: Path):
        f = tmp_path / "empty"
        f.write_bytes(b"")
        assert compute_sha256(f) == (
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )

    def test_hello_newline(self, tmp_path: Path):
        f = tmp_path / "hello"
        f.write_bytes(b"hello\n")
        assert compute_sha256(f) == (
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        )

    def test_missing_path_raises(self, tmp_path: Path):
        with pytest.raises(FileNotFoundError):
            compute_sha256(tmp_path / "nonexistent")

    def test_output_is_lowercase(self):
        result = compute_sha256(_FIXTURES / "tiny_binary.bin")
        assert result == result.lower()

    def test_idempotent(self):
        fixture = _FIXTURES / "tiny_binary.bin"
        assert compute_sha256(fixture) == compute_sha256(fixture)


def _tree(root: Path) -> Path:
    (root / "repo/.git").mkdir(parents=True)
    (root / "repo/.git/index").write_bytes(b"index")
    (root / "repo/README").write_text("readme")
    return root


class TestTreeDigest:
    def test_is_stable_across_calls(self, tmp_path: Path):
        tree = _tree(tmp_path)

        assert tree_digest(tree) == tree_digest(tree)

    def test_a_content_change_changes_the_value(self, tmp_path: Path):
        tree = _tree(tmp_path)
        before = tree_digest(tree)

        (tree / "repo/README").write_text("changed")

        assert tree_digest(tree).value != before.value

    def test_a_mode_change_changes_the_value(self, tmp_path: Path):
        tree = _tree(tmp_path)
        before = tree_digest(tree)

        (tree / "repo/README").chmod(0o755)

        assert tree_digest(tree).value != before.value

    @pytest.mark.parametrize(
        "change",
        [
            lambda tree: (tree / "repo/poison.gitconfig").write_text("x"),
            lambda tree: (tree / "repo/README").unlink(),
            lambda tree: (tree / "repo/empty").mkdir(),
        ],
        ids=["added file", "removed file", "added empty directory"],
    )
    def test_adding_or_removing_an_entry_changes_the_value(
        self, tmp_path: Path, change
    ):
        tree = _tree(tmp_path)
        before = tree_digest(tree)

        change(tree)

        assert tree_digest(tree).value != before.value

    def test_a_timestamp_change_alone_changes_nothing(self, tmp_path: Path):
        tree = _tree(tmp_path)
        before = tree_digest(tree)

        os.utime(tree / "repo/README", (1, 1))
        os.utime(tree / "repo", (1, 1))

        assert tree_digest(tree) == before

    def test_a_symlink_is_hashed_by_its_target(self, tmp_path: Path):
        tree = _tree(tmp_path)
        (tree / "link").symlink_to("repo/README")
        before = tree_digest(tree)

        (tree / "link").unlink()
        (tree / "link").symlink_to("repo")

        assert tree_digest(tree).changes_since(before) == TreeChanges(
            added=(), removed=(), modified=("link",)
        )

    def test_a_fifo_is_digested_without_opening_it(self, tmp_path: Path):
        tree = _tree(tmp_path)
        before = tree_digest(tree)

        os.mkfifo(tree / "repo/pipe")

        assert tree_digest(tree).changes_since(before) == TreeChanges(
            added=("repo/pipe",), removed=(), modified=()
        )

    def test_changes_name_each_added_removed_and_modified_path(
        self, tmp_path: Path
    ):
        tree = _tree(tmp_path)
        before = tree_digest(tree)

        (tree / "repo/poison.gitconfig").write_text("x")
        (tree / "repo/README").unlink()
        (tree / "repo/.git/index").write_bytes(b"rewritten")

        assert tree_digest(tree).changes_since(before) == TreeChanges(
            added=("repo/poison.gitconfig",),
            removed=("repo/README",),
            modified=("repo/.git/index",),
        )

    def test_an_unchanged_tree_has_no_changes(self, tmp_path: Path):
        tree = _tree(tmp_path)

        assert tree_digest(tree).changes_since(
            tree_digest(tree)
        ) == TreeChanges(added=(), removed=(), modified=())
