from tasks.shared.dev.circus import ArbiterSpec, render_circus_ini


def _spec(**overrides) -> ArbiterSpec:
    base = dict(
        endpoint_socket="/tmp/acc-dev-abc/e.sock",
        pubsub_socket="/tmp/acc-dev-abc/p.sock",
        pidfile="/dev/dir/circusd.pid",
        dev_dir="/dev/dir",
        server_bin="/bin/accelerator-visualiser",
        config_path="/dev-server/config.json",
        npm_bin="/usr/local/bin/npm",
        frontend="/repo/frontend",
        frontend_port=54321,
        server_info_path="/dev-server/server-info.json",
    )
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

    def test_server_cmd_omits_log_file_flag(self):
        ini = render_circus_ini(_spec())
        assert "--log-file" not in ini

    def test_visualiser_info_path_set_for_frontend(self):
        spec = _spec()
        ini = render_circus_ini(spec)
        assert f"VISUALISER_INFO_PATH = {spec.server_info_path}" in ini
