from pathlib import Path

from tasks.shared.sources import _BUILD_OUTPUT, _keep, shell_sources, walk_files


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


class TestKeepPredicate:
    def test_keeps_a_normal_script(self):
        assert _keep("scripts/foo.sh")

    def test_keeps_fixtures_at_any_depth(self):
        # 0098 widened scope: fixtures are now linted/formatted like any script.
        assert _keep("skills/x/test-fixtures/seed.sh")
        assert _keep("test-fixtures/a.sh")

    def test_excludes_workspaces(self):
        assert not _keep("workspaces/ws/a.sh")

    def test_keeps_test_helpers(self):
        # 0098 widened scope: sourced-only helper libs are now in scope too.
        assert _keep("scripts/test-helpers.sh")


class TestShellSourcesDiscovery:
    def test_keeps_fixtures_and_helpers_excludes_only_workspaces(
        self, tmp_path: Path
    ):
        _write(tmp_path / "scripts/normal.sh")
        _write(tmp_path / "scripts/test-helpers.sh")
        _write(tmp_path / "scripts/test-fixtures/seed.sh")
        _write(tmp_path / "workspaces/ws.sh")
        # A non-shell file must not appear regardless.
        _write(tmp_path / "scripts/readme.md", "x\n")

        # workspaces/ is the one permanent exclusion; fixtures + helpers are
        # kept.
        assert shell_sources(root=tmp_path) == [
            "scripts/normal.sh",
            "scripts/test-fixtures/seed.sh",
            "scripts/test-helpers.sh",
        ]

    def test_includes_extensionless_cli_script(self):
        # The plugin entry point is a bash script with no .sh extension, so the
        # walk's `.sh` filter never matches it — it must be appended
        # explicitly. Runs against the real repo root where the script exists
        # on disk.
        sources = shell_sources()
        assert "bin/accelerator" in sources

    def test_honours_gitignored_directories(self, tmp_path: Path):
        _write(tmp_path / ".gitignore", "node_modules/\ndist/\n")
        _write(tmp_path / "scripts/keep.sh")
        # Gitignored trees (at any depth) must never be scanned — this is the
        # case that git ls-files got "for free" and a naive walk would miss.
        _write(tmp_path / "node_modules/pkg/install.sh")
        _write(tmp_path / "skills/app/node_modules/pkg/run.sh")
        _write(tmp_path / "skills/app/dist/bundle.sh")

        assert shell_sources(root=tmp_path) == ["scripts/keep.sh"]

    def test_honours_gitignored_file_patterns(self, tmp_path: Path):
        _write(tmp_path / ".gitignore", "*.generated.sh\n")
        _write(tmp_path / "scripts/real.sh")
        _write(tmp_path / "scripts/thing.generated.sh")

        assert shell_sources(root=tmp_path) == ["scripts/real.sh"]

    def test_never_descends_into_vcs_metadata(self, tmp_path: Path):
        # .git / .jj are absent from .gitignore but must never be walked.
        _write(tmp_path / "scripts/keep.sh")
        _write(tmp_path / ".git/hooks/pre-commit.sh")
        _write(tmp_path / ".jj/working_copy/snapshot.sh")

        assert shell_sources(root=tmp_path) == ["scripts/keep.sh"]

    def test_finds_scripts_in_nested_directories(self, tmp_path: Path):
        _write(tmp_path / "a.sh")
        _write(tmp_path / "skills/x/scripts/deep.sh")

        assert shell_sources(root=tmp_path) == [
            "a.sh",
            "skills/x/scripts/deep.sh",
        ]

    def test_no_gitignore_present_is_tolerated(self, tmp_path: Path):
        _write(tmp_path / "scripts/keep.sh")

        assert shell_sources(root=tmp_path) == ["scripts/keep.sh"]

    def test_prunes_build_output_the_root_gitignore_misses(
        self, tmp_path: Path
    ):
        # Before walk_files this yielded all six: the walk pruned only
        # gitignored directories, and none of these five is in the root file.
        _write(tmp_path / "scripts/keep.sh")
        _write(tmp_path / ".venv/bin/act.sh")
        _write(tmp_path / "cli/f/dist/b.sh")
        _write(tmp_path / "playwright-report/t.sh")
        _write(tmp_path / "coverage/c.sh")
        _write(tmp_path / "node_modules/p/i.sh")

        assert shell_sources(root=tmp_path) == ["scripts/keep.sh"]
