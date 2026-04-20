#!/usr/bin/env bash
# Visualiser-specific bash test-harness helpers. Source from test-*.sh scripts:
#
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test-helpers.sh"

# Spawn a child that exits immediately, wait for it, and echo its PID.
# Used to obtain a deterministically-dead PID for stale-lifecycle tests.
spawn_and_reap_pid() {
  sh -c 'exit 0' &
  local dead_pid=$!
  wait "$dead_pid" 2>/dev/null || true
  echo "$dead_pid"``
}

# Write an executable fake-visualiser binary to <out-path>. When invoked
# with --config <path>, it reads tmp_path from the config JSON, binds an
# ephemeral port on 127.0.0.1, writes server-info.json + server.pid
# atomically, and parks until SIGTERM. Uses Python 3 (required by mise env).
make_fake_visualiser() {
  local out_path="$1"
  cat > "$out_path" << 'FAKE_VISUALISER_EOF'
#!/usr/bin/env python3
import argparse, json, os, signal, stat, sys
from http.server import HTTPServer, BaseHTTPRequestHandler

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', required=True)
    args, _ = parser.parse_known_args()

    config = json.load(open(args.config))
    tmp_path = config['tmp_path']
    os.makedirs(tmp_path, mode=0o700, exist_ok=True)

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            body = b'fake-visualiser-ok\n'
            self.send_response(200)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        def log_message(self, *a):
            pass

    server = HTTPServer(('127.0.0.1', 0), Handler)
    port = server.server_address[1]
    my_pid = os.getpid()

    pid_path = os.path.join(tmp_path, 'server.pid')
    info_path = os.path.join(tmp_path, 'server-info.json')

    with open(pid_path, 'w') as f:
        f.write(str(my_pid) + '\n')
    os.chmod(pid_path, stat.S_IRUSR | stat.S_IWUSR)

    info = {
        'version': '0.0.0-fake', 'pid': my_pid, 'start_time': None,
        'host': '127.0.0.1', 'port': port,
        'url': 'http://127.0.0.1:{}'.format(port),
        'log_path': os.path.join(tmp_path, 'server.log'),
        'tmp_path': tmp_path,
    }
    tmp_info = os.path.join(tmp_path, '.server-info.json.tmp')
    with open(tmp_info, 'w') as f:
        json.dump(info, f, indent=2)
        f.write('\n')
    os.chmod(tmp_info, stat.S_IRUSR | stat.S_IWUSR)
    os.rename(tmp_info, info_path)

    def on_sigterm(signum, frame):
        server.shutdown()
        sys.exit(0)

    signal.signal(signal.SIGTERM, on_sigterm)
    server.serve_forever()

main()
FAKE_VISUALISER_EOF
  chmod +x "$out_path"
}

# Variant of make_fake_visualiser that ignores SIGTERM so stop-server.sh
# must escalate to SIGKILL. Used to test the forced-sigkill path.
make_unkillable_fake_visualiser() {
  local out_path="$1"
  make_fake_visualiser "$out_path"
  # Replace the on_sigterm handler with SIG_IGN.
  sed -i.bak \
    's/signal.signal(signal.SIGTERM, on_sigterm)/signal.signal(signal.SIGTERM, signal.SIG_IGN)/' \
    "$out_path"
  rm -f "${out_path}.bak"
}

# Walk <base-dir>/**/server.pid and SIGKILL every recorded PID.
# Used in EXIT traps — catches disowned + reparented fakes that pkill -P misses.
reap_visualiser_fakes() {
  local base="$1"
  while IFS= read -r pid_file; do
    local pid
    pid=$(tr -cd '0-9' < "$pid_file" 2>/dev/null || true)
    [ -n "$pid" ] && kill -9 "$pid" 2>/dev/null || true
  done < <(find "$base" -name "server.pid" 2>/dev/null)
}
