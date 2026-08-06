import pytest

from tasks.shared.dev.circus import (
    ArbiterSpec,
    CircusSupervisor,
    SupervisorUnreachableError,
    render_circus_ini,
)


def _spec(**overrides) -> ArbiterSpec:
    base = {
        "endpoint_socket": "/tmp/acc-dev-abc/e.sock",
        "pubsub_socket": "/tmp/acc-dev-abc/p.sock",
        "pidfile": "/dev/dir/circusd.pid",
        "dev_dir": "/dev/dir",
        "server_bin": "/bin/accelerator-visualiser",
        "project_root": "/repo/proj",
        "npm_bin": "/usr/local/bin/npm",
        "frontend": "/repo/frontend",
        "frontend_port": 54321,
        "server_info_path": "/dev-server/server-info.json",
    }
    base.update(overrides)
    return ArbiterSpec(**base)


class TestRenderCircusIni:
    def test_both_watchers_present(self):
        ini = render_circus_ini(_spec())
        assert "[watcher:server]" in ini
        assert "[watcher:frontend]" in ini

    def test_stop_children_and_graceful_timeout_on_both(self):
        ini = render_circus_ini(_spec())
        assert ini.count("stop_children = true") == 2
        assert ini.count("graceful_timeout = 2") == 2

    def test_autostart_false_on_frontend_only(self):
        ini = render_circus_ini(_spec())
        # server autostarts, frontend does not (ordering gate).
        assert "autostart = true" in ini
        assert ini.count("autostart = false") == 1
        frontend_section = ini.split("[watcher:frontend]")[1]
        assert "autostart = false" in frontend_section

    def test_respawn_false_on_both(self):
        ini = render_circus_ini(_spec())
        assert ini.count("respawn = false") == 2

    def test_ipc_endpoint_and_pubsub_and_pidfile_interpolated(self):
        spec = _spec()
        ini = render_circus_ini(spec)
        assert f"endpoint = ipc://{spec.endpoint_socket}" in ini
        assert f"pubsub_endpoint = ipc://{spec.pubsub_socket}" in ini
        assert f"pidfile = {spec.pidfile}" in ini

    def test_frontend_cmd_carries_port_and_strictport(self):
        spec = _spec()
        ini = render_circus_ini(spec)
        assert f"--port {spec.frontend_port} --strictPort" in ini
        assert f"--prefix {spec.frontend}" in ini

    def test_server_cmd_is_serve_from_project_root(self):
        spec = _spec()
        ini = render_circus_ini(spec)
        server_section = ini.split("[watcher:server]")[1].split(
            "[watcher:frontend]"
        )[0]
        # Model-1: `serve` from working_dir (the project root); no --config and
        # no --log-file (the server composes both from .accelerator/*.md).
        assert f"cmd = {spec.server_bin} serve --owner-pid 0" in server_section
        assert f"working_dir = {spec.project_root}" in server_section
        assert "--config" not in server_section
        assert "--log-file" not in ini

    def test_server_stream_captures_bootstrap_log_not_server_log(self):
        # The server writes its composed server.log itself and /dev/null's its
        # stdout, so circus captures only the pre-redirect stderr to a separate
        # bootstrap log — never the same file.
        spec = _spec()
        ini = render_circus_ini(spec)
        server_section = ini.split("[watcher:server]")[1].split(
            "[watcher:frontend]"
        )[0]
        assert f"{spec.dev_dir}/server.bootstrap.log" in server_section
        assert f"{spec.dev_dir}/server.log" not in server_section

    def test_frontend_stream_captures_frontend_log(self):
        spec = _spec()
        ini = render_circus_ini(spec)
        frontend_section = ini.split("[watcher:frontend]")[1]
        assert f"{spec.dev_dir}/frontend.log" in frontend_section

    def test_visualiser_info_path_set_for_frontend(self):
        spec = _spec()
        ini = render_circus_ini(spec)
        assert f"VISUALISER_INFO_PATH = {spec.server_info_path}" in ini


class _StubClient:
    """Stands in for circus's `CircusClient`: records calls, replays replies."""

    def __init__(self, replies: dict[str, dict]) -> None:
        self._replies = replies
        self.calls: list[tuple[str, dict]] = []

    def send_message(self, command: str, **props) -> dict:
        self.calls.append((command, props))
        return self._replies.get(command, {"status": "ok"})


def _supervisor(replies: dict[str, dict]) -> CircusSupervisor:
    # Bypass __init__ so no real endpoint is dialled: the adapter's whole job
    # is translating circus's wire shapes, which is what these pin.
    sup = object.__new__(CircusSupervisor)
    sup._client = _StubClient(replies)
    return sup


class TestCircusSupervisorStart:
    def test_an_accepted_start_returns(self):
        sup = _supervisor({"start": {"status": "ok"}})
        sup.start("frontend")
        assert sup._client.calls == [("start", {"name": "frontend"})]

    def test_a_refused_start_raises_rather_than_passing_silently(self):
        # circus answers a rejected command with an error *payload*, not an
        # exception. Swallowing it left the caller polling a watcher that was
        # never asked to run until its deadline — the "did not become active"
        # failure with an empty frontend.log.
        sup = _supervisor(
            {"start": {"status": "error", "reason": "arbiter is stopping"}}
        )

        with pytest.raises(SupervisorUnreachableError) as excinfo:
            sup.start("frontend")

        assert "arbiter is stopping" in str(excinfo.value)
        assert "frontend" in str(excinfo.value)

    def test_a_refused_start_without_a_reason_still_raises(self):
        sup = _supervisor({"start": {"status": "error"}})

        with pytest.raises(SupervisorUnreachableError):
            sup.start("frontend")
