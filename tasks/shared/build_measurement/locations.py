from pathlib import Path

from tasks.shared.paths import (
    CLI_DIR,
    CLI_TARGET_DIR,
    FRONTEND,
    MEASURE_SCRATCH_DIRNAME,
    REPO_ROOT,
)

RUN_DIRECTORY_ROOT = Path(MEASURE_SCRATCH_DIRNAME) / "build"
SESSION_LOCK = RUN_DIRECTORY_ROOT / ".lock"
BUILD_PGID = RUN_DIRECTORY_ROOT / "build.pgid"
TRASH_PREFIX = "cold-trash-"

INCREMENTAL_TOUCH = (CLI_DIR / "kernel" / "src" / "lib.rs").relative_to(
    REPO_ROOT
)
COLD_DELETIONS = (
    CLI_TARGET_DIR.relative_to(REPO_ROOT),
    (FRONTEND / "node_modules").relative_to(REPO_ROOT),
)
