from enum import StrEnum, unique
from pathlib import Path


class NotARepositoryError(RuntimeError):
    pass


@unique
class RepositoryKind(StrEnum):
    GIT = "git"
    JJ = "jj"
    COLOCATED_JJ = "colocated-jj"

    @property
    def is_jj(self) -> bool:
        return self is not RepositoryKind.GIT


def repository_kind(path: Path) -> RepositoryKind:
    has_jj = (path / ".jj").is_dir()
    has_git = (path / ".git").exists()
    if has_jj:
        return RepositoryKind.COLOCATED_JJ if has_git else RepositoryKind.JJ
    if has_git:
        return RepositoryKind.GIT
    raise NotARepositoryError(f"{path} is not a git or jj repository")
