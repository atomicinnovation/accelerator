#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

# Source the validate-source.sh helpers without invoking main
# shellcheck disable=SC1091
source "$SCRIPT_DIR/validate-source.sh"

# Helper: call a function, capture its exit code and stdout safely under set -euo pipefail
call_capturing() {
  local out_var="$1" rc_var="$2"
  shift 2
  local _out _rc
  _out="$("$@" 2>/dev/null)" && _rc=$? || _rc=$?
  printf -v "$out_var" '%s' "$_out"
  printf -v "$rc_var" '%d' "$_rc"
}

echo "=== validate-source.sh: canonicalise_host ==="

call_capturing result rc canonicalise_host '[::1]:8080'
assert_eq "canonicalise_host strips brackets and port from [::1]:8080" "::1" "$result"

call_capturing result rc canonicalise_host '[fe80::1%eth0]:443'
assert_eq "canonicalise_host strips zone-id from fe80::1%eth0" "fe80::1" "$result"

call_capturing result rc canonicalise_host 'LOCALHOST.'
assert_eq "canonicalise_host lowercases and strips trailing dot from LOCALHOST." "localhost" "$result"

call_capturing result rc canonicalise_host '127.0.0.1:8080'
assert_eq "canonicalise_host strips port from 127.0.0.1:8080" "127.0.0.1" "$result"

call_capturing result rc canonicalise_host 'user:pass@example.com'
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: canonicalise_host rejects userinfo (user:pass@example.com)"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: canonicalise_host rejects userinfo — expected exit 1, got 0"; FAIL=$((FAIL+1)); }

call_capturing result rc canonicalise_host '2130706433'
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: canonicalise_host rejects decimal-encoded 2130706433"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: canonicalise_host rejects decimal-encoded — expected exit 1, got 0"; FAIL=$((FAIL+1)); }

call_capturing result rc canonicalise_host '0x7f000001'
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: canonicalise_host rejects hex-encoded 0x7f000001"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: canonicalise_host rejects hex-encoded — expected exit 1, got 0"; FAIL=$((FAIL+1)); }

call_capturing result rc canonicalise_host '0177.0.0.1'
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: canonicalise_host rejects octal-encoded 0177.0.0.1"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: canonicalise_host rejects octal-encoded — expected exit 1, got 0"; FAIL=$((FAIL+1)); }

echo ""
echo "=== validate-source.sh: is_localhost_default ==="

call_capturing result rc is_localhost_default "localhost"
[[ "$rc" -eq 0 ]] && \
  { echo "  PASS: is_localhost_default localhost returns 0"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: is_localhost_default localhost returns 0 — got $rc"; FAIL=$((FAIL+1)); }

call_capturing result rc is_localhost_default "127.0.0.1"
[[ "$rc" -eq 0 ]] && \
  { echo "  PASS: is_localhost_default 127.0.0.1 returns 0"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: is_localhost_default 127.0.0.1 returns 0 — got $rc"; FAIL=$((FAIL+1)); }

call_capturing result rc is_localhost_default "127.0.0.2"
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: is_localhost_default 127.0.0.2 returns 1"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: is_localhost_default 127.0.0.2 returns 1 — got $rc"; FAIL=$((FAIL+1)); }

call_capturing result rc is_localhost_default "10.0.0.1"
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: is_localhost_default 10.0.0.1 returns 1"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: is_localhost_default 10.0.0.1 returns 1 — got $rc"; FAIL=$((FAIL+1)); }

echo ""
echo "=== validate-source.sh: classify_internal boundary cases ==="

call_capturing result rc classify_internal '172.15.255.255'
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: classify_internal 172.15.255.255 returns 1 (just below RFC1918)"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 172.15.255.255 returns 1 — got classification: $result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '172.16.0.0'
[[ "$rc" -eq 0 && "$result" == "RFC1918" ]] && \
  { echo "  PASS: classify_internal 172.16.0.0 returns RFC1918"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 172.16.0.0 returns RFC1918 — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '172.31.255.255'
[[ "$rc" -eq 0 && "$result" == "RFC1918" ]] && \
  { echo "  PASS: classify_internal 172.31.255.255 returns RFC1918"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 172.31.255.255 returns RFC1918 — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '172.32.0.0'
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: classify_internal 172.32.0.0 returns 1 (just above RFC1918)"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 172.32.0.0 returns 1 — got classification: $result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '127.0.0.2'
[[ "$rc" -eq 0 && "$result" == "loopback" ]] && \
  { echo "  PASS: classify_internal 127.0.0.2 returns loopback"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 127.0.0.2 returns loopback — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '::1'
[[ "$rc" -eq 0 && "$result" == "loopback" ]] && \
  { echo "  PASS: classify_internal ::1 returns loopback"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal ::1 returns loopback — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '::ffff:127.0.0.1'
[[ "$rc" -eq 0 && "$result" == "loopback" ]] && \
  { echo "  PASS: classify_internal ::ffff:127.0.0.1 returns loopback"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal ::ffff:127.0.0.1 returns loopback — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '0.0.0.0'
[[ "$rc" -eq 0 && "$result" == "wildcard" ]] && \
  { echo "  PASS: classify_internal 0.0.0.0 returns wildcard"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 0.0.0.0 returns wildcard — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '::'
[[ "$rc" -eq 0 && "$result" == "wildcard" ]] && \
  { echo "  PASS: classify_internal :: returns wildcard"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal :: returns wildcard — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal 'fe80::1'
[[ "$rc" -eq 0 && "$result" == "link-local" ]] && \
  { echo "  PASS: classify_internal fe80::1 returns link-local"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal fe80::1 returns link-local — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '169.254.169.254'
[[ "$rc" -eq 0 && "$result" == "link-local" ]] && \
  { echo "  PASS: classify_internal 169.254.169.254 returns link-local"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 169.254.169.254 returns link-local — got rc=$rc result=$result"; FAIL=$((FAIL+1)); }

call_capturing result rc classify_internal '8.8.8.8'
[[ "$rc" -ne 0 ]] && \
  { echo "  PASS: classify_internal 8.8.8.8 returns 1 (public)"; PASS=$((PASS+1)); } || \
  { echo "  FAIL: classify_internal 8.8.8.8 returns 1 (public) — got classification: $result"; FAIL=$((FAIL+1)); }

test_summary
