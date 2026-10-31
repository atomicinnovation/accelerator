import shutil
import subprocess
from collections.abc import Sequence
from typing import Protocol


class DiagnosticRunner(Protocol):
    """Runs an untimed probe under the ambient environment.

    Separate from the measurement runner because the farm holds only the two
    variants' tools: probes routed through it would return `unknown` always.
    """

    def __call__(self, argv: Sequence[str]) -> str: ...


def raw_diagnostic_runner(argv: Sequence[str]) -> str:
    """Like the ambient runner, but returns stdout **unstripped**.

    Recovered bytes are compared against a recorded digest, so a stripped
    trailing newline reads as corruption.
    """
    resolved = shutil.which(argv[0])
    if resolved is None:
        raise FileNotFoundError(argv[0])
    completed = subprocess.run(
        [resolved, *argv[1:]], check=True, capture_output=True, text=True
    )
    return completed.stdout


def ambient_diagnostic_runner(argv: Sequence[str]) -> str:
    resolved = shutil.which(argv[0])
    if resolved is None:
        raise FileNotFoundError(argv[0])
    completed = subprocess.run(
        [resolved, *argv[1:]], check=False, capture_output=True, text=True
    )
    return completed.stdout.strip()
