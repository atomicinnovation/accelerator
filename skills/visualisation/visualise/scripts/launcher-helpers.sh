#!/usr/bin/env bash
# Shared helpers for launch-server.sh and stop-server.sh. Never
# executed directly — sourced only (no exec bit).

die_json() {
  echo "$1" >&2
  exit 1
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

download_to() {
  local url="$1" dest="$2"
  if command -v curl >/dev/null 2>&1; then
    if [ -n "${ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD:-}" ]; then
      curl -fsSL --retry 3 --max-redirs 3 --max-filesize 33554432 -o "$dest" "$url" 2>/dev/null
    else
      curl -fsSL --proto '=https' --tlsv1.2 --retry 3 --max-redirs 3 \
        --max-filesize 33554432 -o "$dest" "$url" 2>/dev/null
    fi
  elif command -v wget >/dev/null 2>&1; then
    wget -q --tries=3 --max-redirect=3 -O "$dest" "$url"
  else
    return 127
  fi
}

ppid_of() {
  local pid="$1"
  if [ -r "/proc/$pid/status" ]; then
    awk '/^PPid:/ {print $2}' "/proc/$pid/status"
  elif command -v ps >/dev/null 2>&1; then
    ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' '
  else
    return 1
  fi
}

start_time_of() {
  local pid="$1"
  if [ -r "/proc/$pid/stat" ] && [ -r "/proc/stat" ]; then
    local tail; tail="$(sed -E 's/.*\) //' "/proc/$pid/stat")"
    local starttime_ticks; starttime_ticks="$(echo "$tail" | awk '{print $20}')"
    local hz; hz="$(getconf CLK_TCK 2>/dev/null || echo 0)"
    [ "$hz" -gt 0 ] || return 1
    local btime; btime="$(awk '/^btime / {print $2}' /proc/stat)"
    echo $(( btime + starttime_ticks / hz ))
  elif command -v ps >/dev/null 2>&1 && [ "$(uname -s)" = "Darwin" ]; then
    local out; out="$(ps -p "$pid" -o lstart= 2>/dev/null | tr -s ' ' ' ' | sed 's/^ //;s/ $//')"
    [ -n "$out" ] || return 1
    date -j -f "%a %b %d %H:%M:%S %Y" "$out" +%s 2>/dev/null
  else
    return 1
  fi
}

# Atomically write server-stopped.json to $path with the given $reason.
# Uses a hidden tempfile in the same dir for an atomic rename.
write_server_stopped() {
  local path="$1" reason="$2"
  local dir; dir="$(dirname "$path")"
  mkdir -p "$dir"
  local tmp; tmp="$(mktemp "$dir/.server-stopped.XXXXXX")"
  local ts; ts="$(date +%s 2>/dev/null || echo null)"
  jq -nc --arg reason "$reason" --argjson timestamp "$ts" \
    '{reason:$reason,timestamp:$timestamp,written_by:"stop-server.sh"}' > "$tmp"
  chmod 0600 "$tmp" 2>/dev/null || true
  mv "$tmp" "$path"
}

# stop_server_status — read-only probe of the lifecycle files.
# Expects $INFO, $PID_FILE to be set by the caller.
stop_server_status() {
  if [ ! -f "$INFO" ]; then
    jq -nc '{status:"not_running"}'
    return 0
  fi
  local pid url start_time alive
  pid="$(tr -cd '0-9' < "$PID_FILE" 2>/dev/null || echo '')"
  url="$(jq -r '.url // empty' "$INFO" 2>/dev/null || true)"
  start_time="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  alive=false
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    if [ -z "$start_time" ] || \
       [ "$(start_time_of "$pid" 2>/dev/null || echo '')" = "$start_time" ]; then
      alive=true
    fi
  fi
  jq -nc \
    --arg status "$([ "$alive" = true ] && echo running || echo stale)" \
    --arg url "$url" \
    --argjson pid "${pid:-null}" \
    '{status:$status,url:$url,pid:$pid}'
}

# stop_server_stop — SIGTERM → 2s grace → SIGKILL escalation.
# Expects $INFO, $PID_FILE, $STOPPED to be set by the caller.
stop_server_stop() {
  if [ ! -f "$PID_FILE" ]; then
    jq -nc '{status:"not_running"}'
    return 0
  fi

  local pid expected_start current_start
  pid="$(tr -cd '0-9' < "$PID_FILE")"
  expected_start="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"

  if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
    rm -f "$PID_FILE" "$INFO"
    jq -nc '{status:"stopped",note:"pid was already dead"}'
    return 0
  fi

  # PID identity check: refuse to kill an unrelated process that
  # happens to hold a recycled PID.
  if [ -n "$expected_start" ]; then
    current_start="$(start_time_of "$pid" 2>/dev/null || echo '')"
    if [ "$current_start" != "$expected_start" ]; then
      jq -nc --argjson pid "$pid" \
        --arg expected "$expected_start" \
        --arg actual "${current_start:-unknown}" \
        '{status:"refused",reason:"pid identity mismatch — not killing an unrelated process",pid:$pid,expected_start_time:$expected,actual_start_time:$actual}'
      rm -f "$PID_FILE" "$INFO"
      return 1
    fi
  fi

  kill "$pid" 2>/dev/null || true
  local i
  for i in $(seq 1 20); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.1
  done

  local forced=false
  if kill -0 "$pid" 2>/dev/null; then
    forced=true
    kill -9 "$pid" 2>/dev/null || true
    sleep 0.1
  fi
  if kill -0 "$pid" 2>/dev/null; then
    jq -nc --argjson pid "$pid" \
      '{status:"failed",error:"process still running after SIGKILL",pid:$pid}'
    return 1
  fi

  # Post-shutdown invariant: server-stopped.json must exist.
  # If the Rust server handled SIGTERM, it already wrote this.
  # If not (fake server, or SIGKILL), we synthesise it here.
  if [ "$forced" = true ] || [ ! -f "$STOPPED" ]; then
    write_server_stopped "$STOPPED" "forced-sigkill"
  fi

  rm -f "$PID_FILE" "$INFO"
  if [ "$forced" = true ]; then
    jq -nc '{status:"stopped",forced:true}'
  else
    jq -nc '{status:"stopped"}'
  fi
}
