"""The `SessionStart` vcs-detect hook: registered once, and run end to end.

The hook command runs through a copy of `bin/accelerator` into the launcher
built from this tree, fetched from a stub release server and verified against
a per-run key. The launcher's own verification of `accelerator-vcs` is not
traversed: it checks against the committed key embedded at build time, which
no per-run key can satisfy. Its `dispatch_failure_policy.rs` covers that.
"""

import filecmp
import json
import subprocess
from pathlib import Path

from tests.integration.support.installation import (
    REPO_ROOT,
    VERSION,
    Installation,
    download_log,
    run_bootstrap,
)
from tests.integration.support.installation_fixtures import NewInstallation
from tests.support.artefacts import claim_task_built

pytest_plugins = ("tests.integration.support.installation_fixtures",)

HOOKS_JSON = REPO_ROOT / "hooks/hooks.json"

_DETECT_ARGS = (
    "vcs",
    "detect",
    "--format=hook",
    "--fail-safe",
    "--descriptive",
)
_DETECT_COMMAND = "${CLAUDE_PLUGIN_ROOT}/bin/accelerator " + " ".join(
    _DETECT_ARGS
)
_GIT_REFERENCE_OPENING = (
    "This repository uses git as its version control system."
)
_BOUNDARY_BLOCK_MARKERS = (
    "WORKSPACE BOUNDARY DETECTED",
    "do not edit files in",
    "do not run VCS commands against",
    "do not grep, find, or research files in",
)
_SILENCING_THE_OVERRIDE_INFO_LINE = {"ACCELERATOR_LOG": "error"}


def _launcher_and_signature(alias: str) -> list[str]:
    return [f"accelerator-{alias}", f"accelerator-{alias}.minisig"]


def _served_assets(installation: Installation) -> list[str]:
    return [url.rsplit("/", 1)[-1] for url in download_log(installation.server)]


def _cached_launcher(installation: Installation, alias: str) -> Path:
    return installation.root / f"bin/accelerator-launcher-{VERSION}-{alias}"


def _assert_outside_any_checkout(directory: Path) -> None:
    ancestors_vcs_detection_walks = directory.parents[:-1]
    enclosing = [
        ancestor
        for ancestor in ancestors_vcs_detection_walks
        if (ancestor / ".git").exists() or (ancestor / ".jj").exists()
    ]
    assert not enclosing, (
        f"{directory} sits inside a checkout at {enclosing[0]}; move the "
        "temporary root (TMPDIR or --basetemp) outside it"
    )


def _vcs_override_env() -> dict[str, str]:
    return {"ACCELERATOR_VCS_BIN": str(claim_task_built("accelerator-vcs"))}


def _detect(
    installation: Installation,
    downloader: Path,
    *,
    env: dict[str, str] | None = None,
    cwd: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    return run_bootstrap(
        installation.root,
        installation.server,
        downloader,
        args=_DETECT_ARGS,
        extra_env=env,
        cwd=cwd,
    )


def test_launcher_dispatch_smoke_on_a_plain_non_repo_dir(
    new_installation: NewInstallation,
    downloader: Path,
    host_platform: str,
    tmp_path: Path,
) -> None:
    launcher = claim_task_built("accelerator")
    installation = new_installation(launcher_source=launcher)
    project = tmp_path / "project"
    project.mkdir()
    _assert_outside_any_checkout(project)

    result = _detect(
        installation,
        downloader,
        env=_vcs_override_env() | _SILENCING_THE_OVERRIDE_INFO_LINE,
        cwd=project,
    )

    assert result.returncode == 0, result.stderr
    assert result.stderr == ""
    assert _served_assets(installation) == _launcher_and_signature(
        host_platform
    )
    assert filecmp.cmp(
        _cached_launcher(installation, host_platform), launcher, shallow=False
    ), "the cached launcher is not the one served"
    hook = json.loads(result.stdout)["hookSpecificOutput"]
    assert hook["hookEventName"] == "SessionStart"
    context = hook["additionalContext"]
    assert context.startswith(_GIT_REFERENCE_OPENING), context
    for marker in _BOUNDARY_BLOCK_MARKERS:
        assert marker not in context


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
