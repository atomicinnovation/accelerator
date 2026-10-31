from pathlib import Path
from typing import IO


def open_log(path: Path) -> IO[str]:
    return path.open("w", buffering=1, encoding="utf-8", newline="\n")
