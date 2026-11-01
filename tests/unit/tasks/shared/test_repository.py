import pytest

from tasks.shared.repository import (
    NotARepositoryError,
    RepositoryKind,
    repository_kind,
)


def test_a_directory_with_only_git_is_git(tmp_path):
    (tmp_path / ".git").mkdir()
    assert repository_kind(tmp_path) is RepositoryKind.GIT


def test_a_directory_with_only_jj_is_jj(tmp_path):
    (tmp_path / ".jj").mkdir()
    assert repository_kind(tmp_path) is RepositoryKind.JJ


def test_a_directory_with_both_is_colocated_jj(tmp_path):
    (tmp_path / ".jj").mkdir()
    (tmp_path / ".git").mkdir()
    assert repository_kind(tmp_path) is RepositoryKind.COLOCATED_JJ


def test_a_directory_with_neither_is_refused(tmp_path):
    with pytest.raises(NotARepositoryError, match="not a git or jj"):
        repository_kind(tmp_path)
