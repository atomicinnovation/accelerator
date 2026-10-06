from pathlib import Path

import pytest

from tests.integration.support.characterisation import Repository

PERSONAL = ".accelerator/config.local.md"

_TEAM = "---\nwork:\n  integration: jira\n---\n"
_PERSONAL = "---\njira:\n  token_cmd: echo personal\n---\n"


def _configure(root: Path) -> None:
    (root / ".accelerator/tmp").mkdir(parents=True, exist_ok=True)
    (root / ".accelerator/tmp/.gitignore").write_text("*\n")
    (root / ".accelerator/config.md").write_text(_TEAM)
    personal = root / PERSONAL
    personal.write_text(_PERSONAL)
    personal.chmod(0o600)


def _summary(run, binaries, cwd: Path, *, with_vcs_binary: bool = True):
    env = (
        {"ACCELERATOR_VCS_BIN": str(binaries.subbinary("vcs"))}
        if with_vcs_binary
        else {}
    )
    result = run(
        binaries.launcher,
        "config",
        "summary",
        "--format=hook",
        cwd=cwd,
        env=env,
    )
    assert result.stdout.count("\n") <= 1, "stdout is more than one JSON line"
    return result


@pytest.mark.parametrize(
    "with_vcs_binary",
    [True, False],
    ids=["with-vcs-binary", "without-vcs-binary"],
)
@pytest.mark.parametrize("tracking", ["tracked", "untracked"])
def test_summary_reports_the_personal_files_tracking(
    repository: Repository,
    binaries,
    run,
    matches_golden,
    tracking,
    with_vcs_binary,
):
    _configure(repository.root)
    if tracking == "tracked":
        repository.track(PERSONAL)
    else:
        repository.untrack(PERSONAL)

    matches_golden(
        _summary(
            run, binaries, repository.root, with_vcs_binary=with_vcs_binary
        )
    )


@pytest.mark.vcs_specific
def test_summary_outside_any_repository(
    hermetic, fixture_root, binaries, run, matches_golden
):
    project = fixture_root / "plain"
    project.mkdir()
    _configure(project)

    matches_golden(_summary(run, binaries, project))


@pytest.mark.vcs_specific
@pytest.mark.parametrize("tracking", ["tracked", "untracked"])
def test_summary_in_a_nested_git_repository(
    hermetic, fixture_root, binaries, run, matches_golden, tracking
):
    outer = Repository.create("git", fixture_root / "outer", hermetic)
    outer.write("README.md", "outer\n")
    outer.commit()
    inner = Repository.create("git", outer.root / "inner", hermetic)
    _configure(inner.root)
    if tracking == "tracked":
        inner.track(PERSONAL)
    else:
        inner.untrack(PERSONAL)

    matches_golden(_summary(run, binaries, inner.root))


@pytest.mark.vcs_specific
def test_summary_with_a_corrupted_git_index(
    hermetic, fixture_root, binaries, run, matches_golden
):
    repository = Repository.create("git", fixture_root / "corrupt", hermetic)
    _configure(repository.root)
    repository.track(PERSONAL)
    (repository.root / ".git/index").write_bytes(b"DIRC-not-an-index")

    matches_golden(_summary(run, binaries, repository.root))


@pytest.mark.vcs_specific
@pytest.mark.parametrize("tracking", ["tracked", "untracked"])
def test_summary_in_a_colocated_jj_repository(
    hermetic, fixture_root, binaries, run, matches_golden, tracking
):
    repository = Repository.create(
        "colocated-jj", fixture_root / "colocated", hermetic
    )
    _configure(repository.root)
    if tracking == "tracked":
        repository.track(PERSONAL)
    else:
        repository.untrack(PERSONAL)

    matches_golden(_summary(run, binaries, repository.root))
