from pathlib import Path

from tasks.shared.paths import MEASURE_SCRATCH_DIRNAME

RUN_DIRECTORY_ROOT = Path(MEASURE_SCRATCH_DIRNAME) / "build"
SESSION_LOCK = RUN_DIRECTORY_ROOT / ".lock"
BUILD_PGID = RUN_DIRECTORY_ROOT / "build.pgid"
