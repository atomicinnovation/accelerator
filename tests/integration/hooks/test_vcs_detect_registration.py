"""The two non-detection guards the retired vcs-detect shell harness carried.

VCS-detection behaviour itself is mirrored in `cli/vcs-adapters` and
`cli/vcs-cli`. These two are not: an end-to-end launcher-dispatch smoke through
the real `bin/accelerator` wrapper, and `hooks/hooks.json` SessionStart
registration integrity.
"""

import json
import os
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
LAUNCHER = REPO_ROOT / "bin/accelerator"
HOOKS_JSON = REPO_ROOT / "hooks/hooks.json"

_DETECT_COMMAND = (
    "${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs detect "
    "--format=hook --fail-safe --descriptive"
)

_PROHIBITIONS = (
    "do not edit files in",
    "do not run VCS commands against",
    "do not grep, find, or research files in",
)


def _dispatch_env(cache_dir: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.setdefault(
        "ACCELERATOR_VCS_BIN",
        str(REPO_ROOT / "cli/target/debug/accelerator-vcs"),
    )
    env.setdefault("ACCELERATOR_PLUGIN_ROOT", str(REPO_ROOT))
    # The sub-binary override emits an INFO diagnostic that production hooks
    # (which carry no override) never do. Quieten to errors so the empty-stderr
    # contract tests the production path's cleanliness, not the test override's.
    env["ACCELERATOR_LOG"] = "error"
    # Without an isolated cache the bootstrap stages its launcher tree into the
    # plugin root's own bin/, which the concurrently-running entrypoint suite's
    # bin/-untouched backstop then flags. Redirect staging into the test's tmp.
    env["ACCELERATOR_CACHE_DIR"] = str(cache_dir)
    return env


def test_launcher_dispatch_smoke_on_a_plain_non_repo_dir(tmp_path) -> None:
    env = _dispatch_env(tmp_path / "cache")
    vcs_bin = Path(env["ACCELERATOR_VCS_BIN"])
    assert vcs_bin.is_file(), (
        f"the compiled accelerator-vcs is absent at {vcs_bin}; build it "
        "(build:cli:dev) — this lane fails rather than fetching a release"
    )

    result = subprocess.run(
        [
            str(LAUNCHER),
            "vcs",
            "detect",
            "--format=hook",
            "--fail-safe",
            "--descriptive",
        ],
        cwd=tmp_path,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, (
        f"non-repo dispatch exited {result.returncode}\n{result.stderr}"
    )
    assert result.stderr == "", f"expected empty stderr, got: {result.stderr!r}"
    if result.stdout.strip():
        json.loads(result.stdout)
    for phrase in _PROHIBITIONS:
        assert phrase not in result.stdout, (
            f"non-repo output names a boundary prohibition: {phrase!r}"
        )
    assert "WORKSPACE BOUNDARY DETECTED" not in result.stdout


def _session_start_entries() -> list[dict]:
    data = json.loads(HOOKS_JSON.read_text(encoding="utf-8"))
    return data["hooks"]["SessionStart"]


def test_hooks_json_vcs_detect_registration_integrity() -> None:
    matches = [
        entry
        for entry in _session_start_entries()
        if entry.get("hooks")
        and entry["hooks"][0].get("command") == _DETECT_COMMAND
    ]
    assert len(matches) == 1, (
        f"expected exactly one SessionStart vcs-detect entry, found "
        f"{len(matches)}"
    )
    entry = matches[0]
    assert entry.get("matcher") == "", "vcs-detect matcher must be empty"
    assert len(entry["hooks"]) == 1, "vcs-detect must register exactly one hook"
    assert entry["hooks"][0].get("type") == "command"
