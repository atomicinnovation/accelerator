import os
from pathlib import Path

from tasks.shared.sources import (
    _BUILD_OUTPUT,
    SURVIVING_SHELL_SOURCES,
    walk_files,
)


def _write(path: Path, text: str = "#!/usr/bin/env bash\n") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


class TestWalkFiles:
    def test_yields_every_suffix_repo_relative(self, tmp_path: Path):
        # Suffix filtering belongs to the caller, not the walk.
        _write(tmp_path / "a.sh")
        _write(tmp_path / "pkg/b.py", "x\n")
        _write(tmp_path / "pkg/c.md", "x\n")

        assert sorted(walk_files(tmp_path)) == ["a.sh", "pkg/b.py", "pkg/c.md"]

    def test_prunes_gitignored_directories(self, tmp_path: Path):
        _write(tmp_path / ".gitignore", "cli/target/\n")
        _write(tmp_path / "keep.sh")
        _write(tmp_path / "cli/target/debug/build.sh")

        assert sorted(walk_files(tmp_path)) == [".gitignore", "keep.sh"]

    def test_prunes_gitignored_file_patterns(self, tmp_path: Path):
        _write(tmp_path / ".gitignore", "*.generated.sh\n")
        _write(tmp_path / "real.sh")
        _write(tmp_path / "thing.generated.sh")

        assert sorted(walk_files(tmp_path)) == [".gitignore", "real.sh"]

    def test_prunes_every_build_output_entry_without_a_gitignore(
        self, tmp_path: Path
    ):
        # No .gitignore is written, so any survivor proves the prune tuple —
        # not the ignore spec — is what removed these.
        _write(tmp_path / "keep.sh")
        for name in _BUILD_OUTPUT:
            _write(tmp_path / name / "nested" / "x.sh")

        assert sorted(walk_files(tmp_path)) == ["keep.sh"]

    def test_prunes_the_frontend_dist_bundle_at_its_real_depth(
        self, tmp_path: Path
    ):
        # The built SPA is ignored only by cli/visualiser/frontend/.gitignore,
        # which _ignore_spec never reads — the root file's `/dist/` is
        # root-anchored and does not match at this depth.
        _write(tmp_path / ".gitignore", "/dist/\n")
        _write(
            tmp_path / "cli/visualiser/frontend/dist/assets/bundle.js", "x\n"
        )
        _write(tmp_path / "cli/visualiser/frontend/src/main.ts", "x\n")

        assert sorted(walk_files(tmp_path)) == [
            ".gitignore",
            "cli/visualiser/frontend/src/main.ts",
        ]

    def test_prune_replaces_rather_than_extends_the_defaults(
        self, tmp_path: Path
    ):
        _write(tmp_path / "dist/x.sh")
        _write(tmp_path / "custom/y.sh")

        assert sorted(walk_files(tmp_path, prune=("custom",))) == ["dist/x.sh"]
        assert (
            sorted(walk_files(tmp_path, prune=(*_BUILD_OUTPUT, "custom"))) == []
        )

    def test_subtree_scopes_the_walk_but_not_the_ignore_spec(
        self, tmp_path: Path
    ):
        # The spec must still be read at `repo` and matched repo-relative: a
        # spec read at the subtree would test `target/` against `cli/target/`,
        # match nothing, and descend into the whole build tree.
        _write(tmp_path / ".gitignore", "cli/target/\n")
        _write(tmp_path / "outside.sh")
        _write(tmp_path / "cli/keep.sh")
        _write(tmp_path / "cli/target/debug/build.sh")

        assert sorted(walk_files(tmp_path, subtree="cli")) == ["cli/keep.sh"]

    def test_never_descends_into_vcs_metadata(self, tmp_path: Path):
        _write(tmp_path / "keep.sh")
        _write(tmp_path / ".git/hooks/pre-commit.sh")
        _write(tmp_path / ".jj/working_copy/snapshot.sh")

        assert sorted(walk_files(tmp_path)) == ["keep.sh"]


class TestSurvivingShellSources:
    _REPO_ROOT = Path(__file__).resolve().parents[4]

    def test_names_exactly_the_two_thin_shell_survivors(self):
        assert SURVIVING_SHELL_SOURCES == (
            "bin/accelerator",
            "hooks/launcher-link-refresh.sh",
        )

    def test_every_survivor_exists_on_disk(self):
        for rel in SURVIVING_SHELL_SOURCES:
            assert (self._REPO_ROOT / rel).is_file(), (
                f"{rel} is listed as a survivor but absent"
            )

    def test_every_survivor_is_tracked_executable(self):
        # Claude Code invokes both as bare commands; the executable bit
        # bin/accelerator depends on is the one property the retired exec-bit
        # invariant still needs to hold for these two.
        for rel in SURVIVING_SHELL_SOURCES:
            path = self._REPO_ROOT / rel
            assert os.access(path, os.X_OK), f"{rel} must be executable (0755)"

    def test_every_survivor_is_a_backticked_token_in_the_readme(self):
        # The README documents the set; SURVIVING_SHELL_SOURCES defines it. A
        # backtick-token check tolerates prose formatting while catching a
        # dropped survivor — it is not a strict equality against parsed prose.
        readme = (self._REPO_ROOT / "tasks/README.md").read_text(
            encoding="utf-8"
        )
        for rel in SURVIVING_SHELL_SOURCES:
            assert f"`{rel}`" in readme, (
                f"{rel} must appear as a backticked token in tasks/README.md"
            )
