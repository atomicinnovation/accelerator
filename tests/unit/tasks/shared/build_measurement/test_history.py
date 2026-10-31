import shlex
from collections.abc import Callable, Sequence
from pathlib import Path

import pytest

from tasks.shared.build_measurement.errors import HistoryError
from tasks.shared.build_measurement.history import (
    GitHistory,
    JjHistory,
    checked_run,
)
from tasks.shared.build_measurement.provenance import (
    History,
    measured_commit,
    working_copy_matches,
)
from tests.unit.tasks.shared.build_measurement.repositories import (
    GitRepository,
    JjRepository,
    isolate,
)

_KEPT_RECORD = {"meta/measurements/build_x.json": "{}"}

type Repository = JjRepository | GitRepository


def _jj(root: Path) -> tuple[Repository, Callable[..., History]]:
    return JjRepository(root), JjHistory


def _git(root: Path) -> tuple[Repository, Callable[..., History]]:
    return GitRepository(root), GitHistory


@pytest.fixture(params=[_jj, _git], ids=["jj", "git"])
def repository(request, monkeypatch, tmp_path):
    isolate(monkeypatch, tmp_path)
    repo, history = request.param(tmp_path / "repo")
    return repo, history(repo.root)


def _write(root: Path, relative: str, content: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def test_an_uncommitted_kept_record_names_the_same_commit_and_matches(
    repository,
):
    repo, history = repository
    code = repo.commit({"a.txt": "a"}, "code")
    _write(repo.root, *next(iter(_KEPT_RECORD.items())))
    assert measured_commit(history) == code
    assert working_copy_matches(history, code)


def test_a_committed_kept_record_names_the_same_commit_and_matches(
    repository,
):
    repo, history = repository
    code = repo.commit({"a.txt": "a"}, "code")
    repo.commit(_KEPT_RECORD, "record")
    assert measured_commit(history) == code
    assert working_copy_matches(history, code)


def test_an_edit_outside_measurements_does_not_match(repository):
    repo, history = repository
    code = repo.commit({"a.txt": "a"}, "code")
    _write(repo.root, "a.txt", "edited")
    assert not working_copy_matches(history, code)


def test_an_untracked_file_outside_measurements_does_not_match(repository):
    repo, history = repository
    code = repo.commit({"a.txt": "a"}, "code")
    _write(repo.root, "new.txt", "new")
    assert not working_copy_matches(history, code)


def test_a_clean_merge_is_measured_by_its_difference_from_its_first_parent(
    repository,
):
    repo, history = repository
    base = repo.commit({"a.txt": "a"}, "base")
    side = repo.commit({"b.txt": "b"}, "side")
    repo.checkout(base)
    main = repo.commit({"c.txt": "c"}, "main")
    merge = repo.merge(main, side)
    assert measured_commit(history) == merge
    assert working_copy_matches(history, merge)


def test_a_merged_working_copy_follows_its_first_parent(monkeypatch, tmp_path):
    isolate(monkeypatch, tmp_path)
    repo = JjRepository(tmp_path / "repo")
    base = repo.commit({"a.txt": "a"}, "base")
    left = repo.commit({"b.txt": "b"}, "left")
    repo.checkout(base)
    repo.commit({"c.txt": "c"}, "right, the newer")
    right = repo.commit({"d.txt": "d"}, "right tip")
    repo.checkout(left, right)
    assert measured_commit(JjHistory(repo.root)) == left


def test_a_gitignored_run_directory_does_not_break_the_match(repository):
    repo, history = repository
    code = repo.commit({".gitignore": "/.accelerator-measure/\n"}, "ignore")
    _write(repo.root, ".accelerator-measure/build/record.json", "{}")
    assert working_copy_matches(history, code)


def test_a_failing_command_names_its_argv_and_stderr(repository):
    repo, history = repository
    repo.commit({"a.txt": "a"}, "code")
    with pytest.raises(HistoryError) as refused:
        history.changed_paths("0" * 40)
    assert "0" * 40 in shlex.join(refused.value.argv)
    assert refused.value.stderr
    assert refused.value.stderr in str(refused.value)
    assert shlex.join(refused.value.argv) in str(refused.value)


def _subcommand(argv: Sequence[str]) -> str:
    return next(arg for arg in argv[1:] if not arg.startswith("-"))


def test_the_adapters_only_issue_read_only_commands(monkeypatch, tmp_path):
    isolate(monkeypatch, tmp_path)
    issued: list[Sequence[str]] = []

    def recording_run(argv: Sequence[str], *, cwd: Path) -> str:
        issued.append(argv)
        return checked_run(argv, cwd=cwd)

    for build, adapter in (
        (JjRepository, JjHistory),
        (GitRepository, GitHistory),
    ):
        repo = build(tmp_path / adapter.__name__)
        code = repo.commit({"a.txt": "a"}, "code")
        history = adapter(repo.root, run=recording_run)
        commit = measured_commit(history)
        working_copy_matches(history, commit)
        assert commit == code

    assert {_subcommand(argv) for argv in issued} <= {
        "log",
        "diff",
        "rev-list",
        "diff-tree",
        "ls-files",
    }
