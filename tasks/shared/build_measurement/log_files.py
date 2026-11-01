import io
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import IO

from tasks.shared.build_measurement.timeline import StampedLine, stamped_text
from tasks.shared.clock import Clock


def open_log(path: Path) -> IO[str]:
    return path.open("w", buffering=1, encoding="utf-8", newline="\n")


@dataclass(slots=True)
class StampedLog:
    file: IO[str]
    lines: list[StampedLine] = field(default_factory=list)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def append(self, line: StampedLine) -> None:
        with self.lock:
            self.lines.append(line)
            self.file.write(stamped_text(line) + "\n")

    def follow(self, pipe: IO[bytes], clock: Clock, epoch: float) -> None:
        # newline="\n": a lone \r (a cargo progress bar redrawn under a pty)
        # must not end a line, or the next segment loses its [task] prefix.
        text = io.TextIOWrapper(
            pipe, encoding="utf-8", errors="replace", newline="\n"
        )
        for raw in text:
            self.append(
                StampedLine(clock.now() - epoch, raw.removesuffix("\n"))
            )

    def close(self) -> tuple[StampedLine, ...]:
        with self.lock:
            self.file.close()
            return tuple(self.lines)
