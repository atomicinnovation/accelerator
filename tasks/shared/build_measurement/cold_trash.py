from dataclasses import dataclass
from pathlib import Path

TRASH_PREFIX = "cold-trash-"


@dataclass(frozen=True, slots=True)
class UnremovedTrash:
    path: Path
    size_bytes: int
    cause: str


@dataclass(frozen=True, slots=True)
class FoundTrash:
    path: Path
    size_bytes: int
