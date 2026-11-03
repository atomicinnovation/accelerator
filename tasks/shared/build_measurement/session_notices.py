"""What a session tells the operator about the checkout it leaves behind."""

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class UnremovedTrash:
    path: Path
    size_bytes: int
    cause: str


@dataclass(frozen=True, slots=True)
class FoundTrash:
    path: Path
    size_bytes: int


@dataclass(frozen=True, slots=True)
class DevLauncherNeedsRebuild:
    pass


type SessionNotice = UnremovedTrash | FoundTrash | DevLauncherNeedsRebuild


def dev_launcher_notices(
    *, deleting: bool, marker_present: bool
) -> tuple[DevLauncherNeedsRebuild, ...]:
    return (DevLauncherNeedsRebuild(),) if deleting and marker_present else ()
