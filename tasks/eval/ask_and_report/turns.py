"""What one turn of a conversation did, as the turn loop and judges read it."""

import hashlib
import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.eval.headless.transcript import Event, Init, Result, Text

UNTRACKED = ".git"


@dataclass(frozen=True)
class FileState:
    digest: str
    text: str


type Snapshot = Mapping[str, FileState]


def snapshot(root: Path) -> dict[str, FileState]:
    """Every file under ``root`` but the repository the harness creates."""
    taken: dict[str, FileState] = {}
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if not path.is_file() or relative.parts[0] == UNTRACKED:
            continue
        content = path.read_bytes()
        taken[relative.as_posix()] = FileState(
            digest=hashlib.sha256(content).hexdigest(),
            text=content.decode(errors="replace"),
        )
    return taken


def changed(before: Snapshot, after: Snapshot) -> list[str]:
    return sorted(
        path
        for path in before.keys() | after.keys()
        if before.get(path) != after.get(path)
    )


def save_snapshot(taken: Snapshot, path: Path) -> None:
    path.write_text(
        json.dumps(
            {
                name: {"digest": state.digest, "text": state.text}
                for name, state in taken.items()
            },
            indent=2,
        )
    )


def load_snapshot(path: Path) -> dict[str, FileState]:
    return {
        name: FileState(**state)
        for name, state in json.loads(path.read_text()).items()
    }


def main_text(events: Sequence[Event]) -> str:
    return "\n".join(
        event.text
        for event in events
        if isinstance(event, Text) and not event.by_subagent
    )


@dataclass(frozen=True)
class TurnRecord:
    """One turn as it ended: what it was sent, logged, and left on disk.

    ``writes`` holds the set-relative paths the turn declares it may write,
    and ``earlier`` the events of each turn before it, in order.
    """

    number: int
    reply: str | None
    writes: tuple[str, ...]
    events: tuple[Event, ...]
    logged: bool
    exit_status: str | None
    before: Snapshot
    after: Snapshot
    initial: Snapshot
    earlier: tuple[tuple[Event, ...], ...]

    @property
    def text(self) -> str:
        return main_text(self.events)

    @property
    def init(self) -> Init | None:
        return next((e for e in self.events if isinstance(e, Init)), None)

    @property
    def result(self) -> Result | None:
        return next((e for e in self.events if isinstance(e, Result)), None)


def errored(record: TurnRecord) -> str | None:
    """Name the infrastructure failure that errored a turn, if one did."""
    if not record.logged:
        return "no log"
    if record.exit_status is None:
        return "no exit status"
    if record.exit_status == "timeout":
        return "timeout"
    if record.exit_status != "0":
        return f"exit code {record.exit_status}"
    if record.init is None:
        return "no init event"
    if record.result is not None and record.result.is_error:
        return "is_error"
    return None
