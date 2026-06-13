"""Unit tests for the Docker visual-regression command + host-server lifecycle.

`docker_visual_command` is asserted on its parsed token list (not substrings),
and `run_against_host_server` is exercised through its injected seams (a local
launcher/handle double + the shared `FakeProcs`/`FakeClock`) so all four
spawn/poll/teardown paths run with no live node process.
"""

import shlex
from itertools import pairwise
from pathlib import Path

import pytest

from tasks.shared.clock import Clock
from tasks.shared.dev.host_server import (
    HostServerError,
    run_against_host_server,
)
from tasks.shared.playwright import (
    BROWSER_LOCALE,
    CHROMIUM_CHANNEL,
    E2E_LANG,
    PLAYWRIGHT_PLATFORM,
    resolved_playwright_version,
)
from tasks.test.e2e import docker_visual_command
from tests.unit.tasks.shared.doubles import FakeClock, FakeProcs

# ─── docker_visual_command ───────────────────────────────────


def _env_map(tokens: list[str]) -> dict[str, str]:
    """Collect `-e KEY=VALUE` pairs into a dict."""
    env: dict[str, str] = {}
    for flag, value in pairwise(tokens):
        if flag == "-e" and "=" in value:
            key, val = value.split("=", 1)
            env[key] = val
    return env


def _volumes(tokens: list[str]) -> list[str]:
    return [value for flag, value in pairwise(tokens) if flag == "-v"]


class TestDockerVisualCommand:
    def _tokens(self, **kw) -> list[str]:
        defaults = {
            "base_url": "http://host.docker.internal:7799",
            "image": "mcr.microsoft.com/playwright:v1.59.1-noble",
            "update": False,
        }
        defaults.update(kw)
        return shlex.split(docker_visual_command(**defaults))

    def test_carries_required_docker_flags(self):
        tokens = self._tokens()
        assert f"--platform={PLAYWRIGHT_PLATFORM}" in tokens
        assert "--ipc=host" in tokens
        assert "--add-host=host.docker.internal:host-gateway" in tokens
        # -w /work as adjacent tokens
        assert tokens[tokens.index("-w") + 1] == "/work"

    def test_mounts_only_the_frontend_dir(self):
        from tasks.shared.paths import FRONTEND

        volumes = self._volumes_for()
        assert f"{FRONTEND}:/work" in volumes

    def _volumes_for(self, **kw):
        return _volumes(self._tokens(**kw))

    def test_node_modules_anonymous_mask_by_default(self):
        volumes = self._volumes_for()
        assert "/work/node_modules" in volumes
        assert not any(v.startswith("pw-node-modules-") for v in volumes)

    def test_cache_deps_uses_version_keyed_named_volume(self):
        volumes = self._volumes_for(cache_deps=True)
        version = resolved_playwright_version()
        expected = f"pw-node-modules-{version}:/work/node_modules"
        assert expected in volumes
        assert "/work/node_modules" not in volumes

    def test_env_carries_origin_channel_locale_and_lang(self):
        env = _env_map(self._tokens())
        assert env["CI"] == "1"
        assert env["BASE_URL"] == "http://host.docker.internal:7799"
        assert env["CHROMIUM_CHANNEL"] == CHROMIUM_CHANNEL
        assert env["PLAYWRIGHT_LOCALE"] == BROWSER_LOCALE
        assert env["LANG"] == E2E_LANG
        assert env["LC_ALL"] == E2E_LANG

    def test_no_drift_env_values_equal_shared_constants(self):
        # The container's channel/locale/lang come from the single shared
        # source — proving one declaration site per no-drift value.
        env = _env_map(self._tokens())
        assert env["CHROMIUM_CHANNEL"] == CHROMIUM_CHANNEL
        assert env["PLAYWRIGHT_LOCALE"] == BROWSER_LOCALE
        assert env["LANG"] == E2E_LANG == env["LC_ALL"]

    def test_bash_payload_is_npm_ci_then_compare_without_locale_gen(self):
        tokens = self._tokens()
        assert tokens[-3:][:2] == ["bash", "-c"]
        payload = tokens[-1]
        assert payload == (
            "npm ci && npx playwright test --config "
            "playwright.docker.config.ts --project visual-regression"
        )
        assert "locale-gen" not in payload

    def test_update_appends_update_snapshots_only_when_set(self):
        assert "--update-snapshots" not in self._tokens(update=False)[-1]
        payload = self._tokens(update=True)[-1]
        assert payload.endswith("--update-snapshots")
        assert payload.startswith("npm ci && npx playwright test")


# ─── run_against_host_server ─────────────────────────────────


class FakeServerHandle:
    """Launch handle whose poll() returns None until a configurable exit."""

    def __init__(self, pid, *, exit_code=None, exit_after_polls=0):
        self.pid = pid
        self.exit_code = exit_code
        self.exit_after_polls = exit_after_polls
        self.polls = 0

    def poll(self):
        self.polls += 1
        if self.exit_code is not None and self.polls > self.exit_after_polls:
            return self.exit_code
        return None


def _make_launcher(
    procs, *, handle, port_file, port=None, child_pid=None, register=True
):
    captured: dict = {}

    def launcher(argv, *, env, cwd):
        captured["argv"] = argv
        captured["env"] = env
        captured["cwd"] = cwd
        if register:
            procs.add(handle.pid, 1000.0)
            if child_pid is not None:
                procs.add(child_pid, 1001.0, parent=handle.pid)
        if port is not None:
            Path(port_file).write_text(port)
        return handle

    return launcher, captured


def _clock() -> tuple[Clock, FakeClock]:
    fc = FakeClock()
    return Clock(sleep=fc.sleep, now=fc.now), fc


class TestRunAgainstHostServer:
    def test_ready_path_invokes_on_ready_with_port_and_reaps(self, tmp_path):
        procs = FakeProcs()
        handle = FakeServerHandle(4242)
        port_file = tmp_path / ".e2e-port"
        launcher, captured = _make_launcher(
            procs,
            handle=handle,
            port_file=port_file,
            port="7799",
            child_pid=4243,
        )
        clock, _ = _clock()
        seen: list[str] = []

        run_against_host_server(
            server_bin=Path("/bin/accelerator-visualiser"),
            on_ready=seen.append,
            frontend=tmp_path,
            launcher=launcher,
            killer=procs,
            clock=clock,
            env={},
        )

        assert seen == ["7799"]
        # spawn env single-sources the host server's locale + bind override
        assert captured["env"]["E2E_SERVER_HOST"] == "0.0.0.0"
        assert captured["env"]["LANG"] == E2E_LANG
        assert captured["env"]["LC_ALL"] == E2E_LANG
        assert (
            captured["env"]["ACCELERATOR_VISUALISER_BIN"]
            == "/bin/accelerator-visualiser"
        )
        # node leader and its Rust child both reaped through ProcessOps
        assert 4242 in procs.terminated
        assert 4243 in procs.terminated
        # .e2e-port cleaned up
        assert not port_file.exists()

    def test_server_exits_early_short_circuits_with_exit_code(self, tmp_path):
        procs = FakeProcs()
        handle = FakeServerHandle(4242, exit_code=3, exit_after_polls=0)
        port_file = tmp_path / ".e2e-port"
        launcher, _ = _make_launcher(
            procs, handle=handle, port_file=port_file, port=None
        )
        clock, _ = _clock()
        seen: list[str] = []

        with pytest.raises(HostServerError, match=r"exited \(code 3\)"):
            run_against_host_server(
                server_bin=Path("/bin/x"),
                on_ready=seen.append,
                frontend=tmp_path,
                launcher=launcher,
                killer=procs,
                clock=clock,
                env={},
            )
        assert seen == []  # on_ready never reached

    def test_port_never_published_times_out(self, tmp_path):
        procs = FakeProcs()
        handle = FakeServerHandle(4242)
        port_file = tmp_path / ".e2e-port"
        launcher, _ = _make_launcher(
            procs, handle=handle, port_file=port_file, port=None
        )
        clock, _ = _clock()

        with pytest.raises(HostServerError, match="did not publish"):
            run_against_host_server(
                server_bin=Path("/bin/x"),
                on_ready=lambda _p: None,
                frontend=tmp_path,
                launcher=launcher,
                killer=procs,
                clock=clock,
                env={},
                readiness_timeout=1.0,
            )

    def test_terminate_times_out_then_kills_leader_and_child(self, tmp_path):
        procs = FakeProcs()
        handle = FakeServerHandle(4242)
        port_file = tmp_path / ".e2e-port"

        def launcher(argv, *, env, cwd):
            procs.add(4242, 1000.0, ignore_sigterm=True)  # survives SIGTERM
            procs.add(4243, 1001.0, parent=4242)
            Path(port_file).write_text("7799")
            return handle

        clock, _ = _clock()

        run_against_host_server(
            server_bin=Path("/bin/x"),
            on_ready=lambda _p: None,
            frontend=tmp_path,
            launcher=launcher,
            killer=procs,
            clock=clock,
            env={},
            grace_kill=1.0,
        )

        # leader ignored SIGTERM, so it escalated to SIGKILL; child died on TERM
        assert 4242 in procs.terminated and 4242 in procs.killed
        assert 4243 in procs.terminated

    def test_already_exited_leader_makes_teardown_a_no_op(self, tmp_path):
        procs = FakeProcs()
        handle = FakeServerHandle(4242)
        port_file = tmp_path / ".e2e-port"
        # register=False: the leader is gone by teardown time (never in procs)
        launcher, _ = _make_launcher(
            procs,
            handle=handle,
            port_file=port_file,
            port="7799",
            register=False,
        )
        clock, _ = _clock()
        seen: list[str] = []

        # No raise despite the leader being absent from the process table.
        run_against_host_server(
            server_bin=Path("/bin/x"),
            on_ready=seen.append,
            frontend=tmp_path,
            launcher=launcher,
            killer=procs,
            clock=clock,
            env={},
        )
        assert seen == ["7799"]
        assert 4242 not in procs.killed  # nothing live to escalate to
