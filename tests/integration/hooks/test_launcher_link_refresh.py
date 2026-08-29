"""Tests for `hooks/launcher-link-refresh.sh`.

Python rather than a `hooks/test-*.sh` harness, per ADR-0048: Python is the test
language for the non-Rust surfaces, shell wrappers included. The two bash suites
still under `hooks/` predate that decision.

The hook self-locates unconditionally, so there is no environment seam for the
plugin root — the root under test is wherever the *hook file* sits. Each case
that needs a particular root therefore builds a fixture installation and copies
the hook into it, which is more setup than exporting a variable and better
fidelity: it exercises the resolution production actually performs.

`_run` builds a complete explicit environment rather than inheriting one. That
matters even though the hook never reads a plugin root: a leaked
`ACCELERATOR_CACHE_DIR` would redirect the unverified-log path, and a leaked
`CLAUDE_PLUGIN_DATA` would defeat the inertness cases.
"""

import json
import os
import shutil
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
HOOK = REPO_ROOT / "hooks/launcher-link-refresh.sh"
HOOKS_JSON = REPO_ROOT / "hooks/hooks.json"

HOOK_COMMAND = "${CLAUDE_PLUGIN_ROOT}/hooks/launcher-link-refresh.sh"

_BASH = "/bin/bash" if Path("/bin/bash").exists() else "bash"


@dataclass(frozen=True)
class Result:
    returncode: int
    stdout: str
    stderr: str

    @property
    def notice(self) -> str:
        """The `systemMessage` from the single JSON object on stdout."""
        return json.loads(self.stdout)["systemMessage"]

    @property
    def stdout_objects(self) -> int:
        decoder = json.JSONDecoder()
        text, count, index = self.stdout.strip(), 0, 0
        while index < len(text):
            _, index = decoder.raw_decode(text, index)
            count += 1
            while index < len(text) and text[index].isspace():
                index += 1
        return count


def make_root(path: Path) -> Path:
    """A fixture installation carrying its own copy of the hook.

    The *physical* path is returned because the hook resolves its own root with
    `pwd -P`: on macOS `tmp_path` is under /private/var while its logical form
    is /var, so a logical path here would fail every link comparison on that
    leg alone. `CLAUDE_PLUGIN_DATA` is deliberately never canonicalised — the
    hook composes it verbatim, and so must the assertions.
    """
    (path / "hooks").mkdir(parents=True)
    (path / "bin").mkdir()
    shutil.copy(HOOK, path / "hooks/launcher-link-refresh.sh")
    launcher = path / "bin/accelerator"
    launcher.write_text("#!/bin/sh\nexit 0\n")
    launcher.chmod(0o755)
    return path.resolve()


def _run(root: Path, *, plugin_data: str | None = None, **env: str) -> Result:
    """Invoke the hook under a minimal, explicit environment."""
    composed = {"PATH": env.pop("PATH", os.environ["PATH"])}
    if plugin_data is not None:
        composed["CLAUDE_PLUGIN_DATA"] = plugin_data
    composed.update(env)
    completed = subprocess.run(
        [_BASH, str(root / "hooks/launcher-link-refresh.sh")],
        capture_output=True,
        text=True,
        env=composed,
        cwd=env.get("_CWD", str(root)),
        check=False,
    )
    return Result(completed.returncode, completed.stdout, completed.stderr)


def snapshot(tree: Path) -> list[str]:
    """One line per entry plus its link target.

    A name-only listing cannot see a re-pointed symlink or a rewritten file,
    which is exactly what "created **or modified**" has to cover.
    """
    out = []
    for path in [*sorted(tree.rglob("*")), tree]:
        target = str(path.readlink()) if path.is_symlink() else ""
        digest = ""
        if path.is_file() and not path.is_symlink():
            digest = path.read_bytes().hex()
        out.append(f"{path}|{target}|{digest}")
    return sorted(out)


def mode_of(path: Path) -> int:
    return stat.S_IMODE(path.lstat().st_mode)


def link_target(path: Path) -> str:
    return str(path.readlink()) if path.is_symlink() else ""


class TestRefresh:
    def test_a_first_refresh_creates_the_link_and_says_nothing(
        self, tmp_path: Path
    ) -> None:
        root = make_root(tmp_path / "v1")
        result = _run(root, plugin_data=str(tmp_path / "data"))

        assert result.returncode == 0
        assert link_target(tmp_path / "data/bin/accelerator") == str(
            root / "bin/accelerator"
        )
        assert result.stdout == ""
        assert result.stderr == ""

    def test_a_second_root_repoints_the_link_and_names_both(
        self, tmp_path: Path
    ) -> None:
        v1 = make_root(tmp_path / "v1")
        v2 = make_root(tmp_path / "v2")
        data = tmp_path / "data"
        _run(v1, plugin_data=str(data))

        userbin = tmp_path / "userbin"
        userbin.mkdir()
        (userbin / "accelerator").symlink_to(data / "bin/accelerator")

        result = _run(v2, plugin_data=str(data))

        assert result.returncode == 0
        assert link_target(data / "bin/accelerator") == str(
            v2 / "bin/accelerator"
        )
        assert str(v1 / "bin/accelerator") in result.stderr
        assert str(v2 / "bin/accelerator") in result.stderr
        # The two-hop design's central claim: the user's own hop never moved,
        # so it still executes after the plugin-owned hop was re-pointed.
        assert (
            subprocess.run(
                [str(userbin / "accelerator")], check=False
            ).returncode
            == 0
        )


class TestDestinationGuards:
    def test_a_regular_file_is_refused_and_left_untouched(
        self, tmp_path: Path
    ) -> None:
        root = make_root(tmp_path / "v1")
        data = tmp_path / "data"
        (data / "bin").mkdir(parents=True)
        (data / "bin/accelerator").write_text("user content\n")

        result = _run(root, plugin_data=str(data))

        assert result.returncode == 0
        assert (data / "bin/accelerator").read_text() == "user content\n"
        # An actionable, persistent state the hook cannot repair, so it must
        # arrive as a systemMessage rather than on stderr.
        assert "is not a symlink" in result.notice
        assert "[accelerator]" not in result.stderr

    def test_a_symlink_to_a_directory_is_replaced_not_written_into(
        self, tmp_path: Path
    ) -> None:
        # `mv -f` onto a symlink-to-directory writes *inside* it and reports
        # success, so this is the case that fails if the clear step is dropped.
        root = make_root(tmp_path / "v1")
        data = tmp_path / "data"
        (data / "bin").mkdir(parents=True)
        elsewhere = tmp_path / "elsewhere"
        elsewhere.mkdir()
        (data / "bin/accelerator").symlink_to(elsewhere)

        result = _run(root, plugin_data=str(data))

        assert result.returncode == 0
        assert link_target(data / "bin/accelerator") == str(
            root / "bin/accelerator"
        )
        assert list(elsewhere.iterdir()) == []

    def test_a_stale_staging_path_is_refused(self, tmp_path: Path) -> None:
        # The staging path carries the hook child's pid, unknowable in advance,
        # so the hook is invoked through a wrapper that execs it — the child
        # then inherits a pid the fixture chose and can seed the path first.
        root = make_root(tmp_path / "v1")
        data = tmp_path / "data"
        (data / "bin").mkdir(parents=True)
        wrapper = tmp_path / "wrapper.sh"
        wrapper.write_text(
            "#!/usr/bin/env bash\n"
            ': >"$2/bin/accelerator.new.$$"\n'
            'exec bash "$1"\n'
        )

        completed = subprocess.run(
            [
                _BASH,
                str(wrapper),
                str(root / "hooks/launcher-link-refresh.sh"),
                str(data),
            ],
            capture_output=True,
            text=True,
            env={"PATH": os.environ["PATH"], "CLAUDE_PLUGIN_DATA": str(data)},
            check=False,
        )

        assert completed.returncode == 0
        assert "stale staging path" in completed.stderr
        assert not (data / "bin/accelerator").exists()


class TestDataBinGuards:
    def test_a_symlink_to_a_directory_is_refused(self, tmp_path: Path) -> None:
        # The flavour matters: a dangling symlink or a regular file reaches the
        # mkdir guard instead and produces a different diagnostic.
        root = make_root(tmp_path / "v1")
        data = tmp_path / "data"
        data.mkdir()
        real_bin = tmp_path / "real-bin"
        real_bin.mkdir()
        (data / "bin").symlink_to(real_bin)

        result = _run(root, plugin_data=str(data))

        assert result.returncode == 0
        assert "is not a plain directory" in result.stderr
        assert list(real_bin.iterdir()) == []

    def test_a_regular_file_is_refused(self, tmp_path: Path) -> None:
        # Without this case the guard's second clause has no coverage.
        root = make_root(tmp_path / "v1")
        data = tmp_path / "data"
        data.mkdir()
        (data / "bin").write_text("not a directory\n")

        result = _run(root, plugin_data=str(data))

        assert result.returncode == 0
        assert "is not a plain directory" in result.stderr
        assert (data / "bin").read_text() == "not a directory\n"

    def test_an_existing_directory_keeps_its_mode(self, tmp_path: Path) -> None:
        root = make_root(tmp_path / "v1")
        data = tmp_path / "data"
        (data / "bin").mkdir(parents=True)
        (data / "bin").chmod(0o700)

        result = _run(root, plugin_data=str(data))

        assert result.returncode == 0
        assert mode_of(data / "bin") == 0o700

    @pytest.mark.skipif(
        os.getuid() == 0, reason="mode bits are advisory for uid 0"
    )
    def test_an_unwritable_directory_reports_and_stages_nothing(
        self, tmp_path: Path
    ) -> None:
        root = make_root(tmp_path / "v1")
        data = tmp_path / "data"
        (data / "bin").mkdir(parents=True)
        (data / "bin").chmod(0o555)
        try:
            result = _run(root, plugin_data=str(data))

            assert result.returncode == 0
            assert "may be stale" in result.stderr
            assert list((data / "bin").iterdir()) == []
        finally:
            # Restore before tmp_path teardown, which cannot remove it either.
            (data / "bin").chmod(0o755)


class TestLauncherValidation:
    def test_a_removed_launcher_is_refused(self, tmp_path: Path) -> None:
        # The state after an upgrade deletes the old directory.
        root = make_root(tmp_path / "v1")
        (root / "bin/accelerator").unlink()

        result = _run(root, plugin_data=str(tmp_path / "data"))

        assert result.returncode == 0
        assert "launcher not executable" in result.stderr
        assert not (tmp_path / "data/bin/accelerator").exists()

    def test_a_non_executable_launcher_is_refused(self, tmp_path: Path) -> None:
        root = make_root(tmp_path / "v1")
        (root / "bin/accelerator").chmod(0o644)

        result = _run(root, plugin_data=str(tmp_path / "data"))

        assert result.returncode == 0
        assert "launcher not executable" in result.stderr


class TestInertness:
    def test_an_unset_plugin_data_is_inert(self, tmp_path: Path) -> None:
        # Asserted on the hook's decision, not on the absence of a /bin entry:
        # / is unwritable on both CI legs and SIP-protected on macOS, so an
        # absence assertion would pass whether or not the guard exists.
        root = make_root(tmp_path / "v1")

        result = _run(root)

        assert result.returncode == 0
        assert "CLAUDE_PLUGIN_DATA unavailable" in result.stderr
        assert result.stdout == ""

    def test_a_relative_plugin_data_is_inert(self, tmp_path: Path) -> None:
        # Composing against a relative value would put the link inside the
        # user's project directory.
        root = make_root(tmp_path / "v1")
        cwd = tmp_path / "cwd"
        cwd.mkdir()
        before = snapshot(cwd)

        result = _run(root, plugin_data="./data", _CWD=str(cwd))

        assert result.returncode == 0
        assert "CLAUDE_PLUGIN_DATA unavailable" in result.stderr
        assert snapshot(cwd) == before


class TestUnverifiedLog:
    def _seed(self, root: Path) -> Path:
        log = root / "bin/.accelerator-unverified.log"
        log.write_text("2026-01-01T00:00:00Z pid=1 bad signature\n")
        return log

    def test_a_non_empty_log_is_reported(self, tmp_path: Path) -> None:
        root = make_root(tmp_path / "v1")
        log = self._seed(root)

        result = _run(root, plugin_data=str(tmp_path / "data"))

        assert result.returncode == 0
        assert str(log) in result.notice
        assert "[accelerator]" not in result.stderr

    def test_an_absent_log_produces_no_notice(self, tmp_path: Path) -> None:
        root = make_root(tmp_path / "v1")

        assert _run(root, plugin_data=str(tmp_path / "data")).stdout == ""

    def test_a_zero_length_log_produces_no_notice(self, tmp_path: Path) -> None:
        root = make_root(tmp_path / "v1")
        (root / "bin/.accelerator-unverified.log").touch()

        assert _run(root, plugin_data=str(tmp_path / "data")).stdout == ""

    def test_two_notices_emit_exactly_one_json_object(
        self, tmp_path: Path
    ) -> None:
        # Two systemMessage objects would be invalid output, so this is the
        # case that pins the NOTICE accumulator.
        root = make_root(tmp_path / "v1")
        self._seed(root)
        data = tmp_path / "data"
        (data / "bin").mkdir(parents=True)
        (data / "bin/accelerator").write_text("user content\n")

        result = _run(root, plugin_data=str(data))

        assert result.returncode == 0
        assert result.stdout_objects == 1
        assert "an unverified launcher was recorded" in result.notice
        assert "is not a symlink" in result.notice

    def test_without_jq_the_notices_degrade_to_stderr(
        self, tmp_path: Path
    ) -> None:
        # Shadowed rather than stripped from PATH: macOS 15 ships /usr/bin/jq,
        # so dropping every directory holding a jq also drops dirname, mkdir
        # and ln, and the hook would fail before reaching the jq fallback.
        root = make_root(tmp_path / "v1")
        self._seed(root)
        shadow = tmp_path / "shadow"
        shadow.mkdir()
        (shadow / "jq").write_text("#!/bin/sh\nexit 127\n")
        (shadow / "jq").chmod(0o755)

        result = _run(
            root,
            plugin_data=str(tmp_path / "data"),
            PATH=f"{shadow}:{os.environ['PATH']}",
        )

        assert result.returncode == 0
        assert result.stdout == ""
        assert "an unverified launcher was recorded" in result.stderr


def test_nothing_outside_plugin_data_is_created_or_modified(
    tmp_path: Path,
) -> None:
    root = make_root(tmp_path / "v1")
    home = tmp_path / "home"
    home.mkdir()
    cwd = tmp_path / "cwd"
    cwd.mkdir()
    before = snapshot(home) + snapshot(cwd)

    result = _run(
        root,
        plugin_data=str(tmp_path / "data"),
        HOME=str(home),
        _CWD=str(cwd),
    )

    assert result.returncode == 0
    assert snapshot(home) + snapshot(cwd) == before
    assert not (home / ".local").exists()


class TestRegistration:
    """Selected by command content, never by index.

    This suite selects the SessionStart entry by its command string, never by
    index, so a `vcs detect` entry can move within the array without breaking
    it — no positional coupling on hooks.json's SessionStart array remains
    anywhere, and this must not reintroduce one.
    """

    def _group(self) -> dict:
        registered = json.loads(HOOKS_JSON.read_text())
        groups = [
            group
            for group in registered["hooks"]["SessionStart"]
            if any(hook["command"] == HOOK_COMMAND for hook in group["hooks"])
        ]
        assert len(groups) == 1, groups
        return groups[0]

    def test_the_hook_is_a_session_start_group(self) -> None:
        assert self._group()["matcher"] == ""

    def test_the_group_holds_exactly_one_command_hook(self) -> None:
        hooks = self._group()["hooks"]
        assert len(hooks) == 1
        assert hooks[0]["type"] == "command"
        # The full literal: a bare endswith match would pass for a relative or
        # wrongly-prefixed command.
        assert hooks[0]["command"] == HOOK_COMMAND


def test_the_hook_reads_no_plugin_root_from_the_environment() -> None:
    assert "CLAUDE_PLUGIN_ROOT" not in HOOK.read_text()
