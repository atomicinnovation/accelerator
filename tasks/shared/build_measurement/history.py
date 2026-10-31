import subprocess
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from tasks.shared.build_measurement.errors import HistoryError


class HistoryCommandRunner(Protocol):
    def __call__(self, argv: Sequence[str], *, cwd: Path) -> str: ...


def checked_run(argv: Sequence[str], *, cwd: Path) -> str:
    completed = subprocess.run(
        argv, cwd=cwd, check=False, capture_output=True, text=True
    )
    if completed.returncode != 0:
        raise HistoryError(argv, completed.stderr.strip())
    return completed.stdout


def _lines(output: str) -> tuple[str, ...]:
    return tuple(line for line in output.splitlines() if line)


@dataclass(frozen=True, slots=True)
class JjHistory:
    root: Path
    run: HistoryCommandRunner = checked_run

    def first_parent_chain(self) -> Iterator[str]:
        return iter(
            self._lines(
                "--ignore-working-copy",
                "log",
                "--no-graph",
                "-r",
                "first_ancestors(first_parent(@)) ~ root()",
                "-T",
                'commit_id ++ "\\n"',
            )
        )

    def changed_paths(self, commit: str) -> tuple[str, ...]:
        return self._lines(
            "--ignore-working-copy",
            "diff",
            "--from",
            f"first_parent({commit})",
            "--to",
            commit,
            "--name-only",
        )

    def working_copy_changes(self, commit: str) -> tuple[str, ...]:
        return self._lines("diff", "--from", commit, "--to", "@", "--name-only")

    def _lines(self, *arguments: str) -> tuple[str, ...]:
        return _lines(self.run(["jj", *arguments], cwd=self.root))


@dataclass(frozen=True, slots=True)
class GitHistory:
    root: Path
    run: HistoryCommandRunner = checked_run

    def first_parent_chain(self) -> Iterator[str]:
        return iter(self._lines("rev-list", "--first-parent", "HEAD"))

    def changed_paths(self, commit: str) -> tuple[str, ...]:
        return self._lines(
            "diff-tree",
            "-r",
            "--root",
            "--no-commit-id",
            "--name-only",
            "--diff-merges=first-parent",
            commit,
        )

    def working_copy_changes(self, commit: str) -> tuple[str, ...]:
        return (
            *self._lines("diff", "--name-only", commit),
            *self._lines("ls-files", "--others", "--exclude-standard"),
        )

    def _lines(self, *arguments: str) -> tuple[str, ...]:
        return _lines(self.run(["git", *arguments], cwd=self.root))
