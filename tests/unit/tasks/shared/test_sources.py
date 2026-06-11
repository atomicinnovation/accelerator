from pathlib import Path

from tasks.shared.sources import _keep, shell_sources


def _write(path: Path, text: str = "#!/usr/bin/env bash\n") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


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
    def test_keeps_fixtures_and_helpers_excludes_only_workspaces(self, tmp_path: Path):
        _write(tmp_path / "scripts/normal.sh")
        _write(tmp_path / "scripts/test-helpers.sh")
        _write(tmp_path / "scripts/test-fixtures/seed.sh")
        _write(tmp_path / "workspaces/ws.sh")
        # A non-shell file must not appear regardless.
        _write(tmp_path / "scripts/readme.md", "x\n")

        # workspaces/ is the one permanent exclusion; fixtures + helpers are kept.
        assert shell_sources(root=tmp_path) == [
            "scripts/normal.sh",
            "scripts/test-fixtures/seed.sh",
            "scripts/test-helpers.sh",
        ]

    def test_includes_extensionless_cli_script(self):
        # The visualiser CLI is a bash script with no .sh extension, so the walk's
        # `.sh` filter never matches it — it must be appended explicitly. Runs
        # against the real repo root where the script exists on disk.
        sources = shell_sources()
        assert (
            "skills/visualisation/visualise/cli/accelerator-visualiser" in sources
        )

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

        assert shell_sources(root=tmp_path) == ["a.sh", "skills/x/scripts/deep.sh"]

    def test_no_gitignore_present_is_tolerated(self, tmp_path: Path):
        _write(tmp_path / "scripts/keep.sh")

        assert shell_sources(root=tmp_path) == ["scripts/keep.sh"]
