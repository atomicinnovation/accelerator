#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

ENSURE="$SCRIPT_DIR/ensure-playwright.sh"

echo "=== ensure-playwright.sh: structural ==="

assert_file_exists "ensure-playwright.sh exists" "$ENSURE"
assert_file_executable "ensure-playwright.sh is executable" "$ENSURE"

echo ""
echo "=== ensure-playwright.sh: platform guard ==="

# Simulate a Windows-like OSTYPE by wrapping the call with env override.
# On Windows OSTYPE is msys, mingw*, or cygwin*. We test via the mock exit mode
# since actually changing OSTYPE from a running bash is unreliable.
assert_exit_code "rejects unknown platform via OSTYPE override" 2 \
  env OSTYPE=msys ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 \
  bash "$ENSURE"

assert_stderr_contains "platform rejection mentions OSTYPE" "OSTYPE" \
  env OSTYPE=msys ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 \
  bash "$ENSURE"

echo ""
echo "=== ensure-playwright.sh: Node version check ==="

assert_stderr_contains "rejects when node missing — names install URL" "nodejs.org" \
  env PATH="/usr/bin:/bin" \
  bash "$ENSURE"

assert_exit_code "rejects when node missing" 10 \
  env PATH="/usr/bin:/bin" \
  bash "$ENSURE"

assert_stderr_contains "node-missing emits ACCELERATOR_DOWNGRADE_REASON=node-missing" \
  "ACCELERATOR_DOWNGRADE_REASON=node-missing" \
  env PATH="/usr/bin:/bin" \
  bash "$ENSURE"

echo ""
echo "=== ensure-playwright.sh: mock install paths ==="

MOCK_CACHE="$(mktemp -d)"
trap 'rm -rf "$MOCK_CACHE"' EXIT

# Both _OK flags: fast path that just touches expected files and writes sentinel
assert_exit_code "mock install succeeds with both OK flags" 0 \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 \
  bash "$ENSURE"

# Verify sentinel was written
LOCKHASH="$(sha256sum "$SCRIPT_DIR/playwright/package-lock.json" 2>/dev/null | cut -c1-8 || shasum -a 256 "$SCRIPT_DIR/playwright/package-lock.json" | cut -c1-8)"
SENTINEL="$MOCK_CACHE/$LOCKHASH/.bootstrap-sentinel"

assert_file_exists "sentinel written after mock install" "$SENTINEL"

assert_stderr_contains "sentinel contains lockhash" "$LOCKHASH" \
  bash -c "jq -r .lockhash \"$SENTINEL\" >&2"

# Second run with sentinel present: exits fast (< 5 seconds)
assert_exit_code "second run exits 0 (sentinel short-circuits)" 0 \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 \
  bash "$ENSURE"

echo ""
echo "=== ensure-playwright.sh: mock failure paths ==="

MOCK_CACHE_FAIL="$(mktemp -d)"
trap 'rm -rf "$MOCK_CACHE_FAIL"' EXIT

# Simulated npm ci failure
assert_exit_code "exits 14 on simulated npm ci failure" 14 \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_FAIL" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT=42 \
  bash "$ENSURE"

assert_stderr_contains "npm failure names NPM_CONFIG_REGISTRY" "NPM_CONFIG_REGISTRY" \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_FAIL" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT=42 \
  bash "$ENSURE"

assert_stderr_contains "npm failure emits ACCELERATOR_DOWNGRADE_REASON=bootstrap-failed" \
  "ACCELERATOR_DOWNGRADE_REASON=bootstrap-failed" \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_FAIL" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT=42 \
  bash "$ENSURE"

MOCK_CACHE_FAIL2="$(mktemp -d)"
trap 'rm -rf "$MOCK_CACHE_FAIL2"' EXIT

# Simulated playwright install failure
assert_exit_code "exits 15 on simulated playwright install failure" 15 \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_FAIL2" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_EXIT=42 \
  bash "$ENSURE"

assert_stderr_contains "playwright failure names PLAYWRIGHT_DOWNLOAD_HOST" "PLAYWRIGHT_DOWNLOAD_HOST" \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_FAIL2" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_EXIT=42 \
  bash "$ENSURE"

assert_stderr_contains "playwright failure emits ACCELERATOR_DOWNGRADE_REASON=bootstrap-failed" \
  "ACCELERATOR_DOWNGRADE_REASON=bootstrap-failed" \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_FAIL2" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_EXIT=42 \
  bash "$ENSURE"

echo ""
echo "=== ensure-playwright.sh: lockhash namespacing ==="

MOCK_CACHE_NS="$(mktemp -d)"
trap 'rm -rf "$MOCK_CACHE_NS"' EXIT

# Run with real package-lock.json — verify namespace directory matches expected hash
assert_exit_code "lockhash namespace install succeeds" 0 \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_NS" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 \
  bash "$ENSURE"

assert_dir_exists "lockhash namespace directory created" "$MOCK_CACHE_NS/$LOCKHASH"

echo ""
echo "=== ensure-playwright.sh: stale sweep (opt-in) ==="

MOCK_CACHE_SWEEP="$(mktemp -d)"
trap 'rm -rf "$MOCK_CACHE_SWEEP"' EXIT

# Create two stale sentinel entries with completed_at > 90 days ago
OLD_DATE="2025-01-01T00:00:00Z"
mkdir -p "$MOCK_CACHE_SWEEP/aaaaaaaa"
printf '{"lockhash":"aaaaaaaa","playwright_version":"1.40.0","completed_at":"%s"}' "$OLD_DATE" \
  > "$MOCK_CACHE_SWEEP/aaaaaaaa/.bootstrap-sentinel"
mkdir -p "$MOCK_CACHE_SWEEP/bbbbbbbb"
printf '{"lockhash":"bbbbbbbb","playwright_version":"1.41.0","completed_at":"%s"}' "$OLD_DATE" \
  > "$MOCK_CACHE_SWEEP/bbbbbbbb/.bootstrap-sentinel"

# Without sweep flag: stale dirs are preserved
assert_exit_code "sweep disabled by default: stale dirs preserved" 0 \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_SWEEP" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 \
  bash "$ENSURE"

assert_dir_exists "stale dir aaaaaaaa preserved without sweep flag" "$MOCK_CACHE_SWEEP/aaaaaaaa"
assert_dir_exists "stale dir bbbbbbbb preserved without sweep flag" "$MOCK_CACHE_SWEEP/bbbbbbbb"

# With sweep flag: stale dirs removed, active preserved
assert_exit_code "sweep enabled: exits 0" 0 \
  env ACCELERATOR_PLAYWRIGHT_CACHE="$MOCK_CACHE_SWEEP" \
  ACCELERATOR_PLAYWRIGHT_MOCK_NPM_OK=1 \
  ACCELERATOR_PLAYWRIGHT_MOCK_PLAYWRIGHT_OK=1 \
  ACCELERATOR_PLAYWRIGHT_SWEEP=1 \
  bash "$ENSURE"

assert_dir_not_exists "stale dir aaaaaaaa removed by sweep" "$MOCK_CACHE_SWEEP/aaaaaaaa"
assert_dir_not_exists "stale dir bbbbbbbb removed by sweep" "$MOCK_CACHE_SWEEP/bbbbbbbb"
assert_dir_exists "active lockhash dir preserved by sweep" "$MOCK_CACHE_SWEEP/$LOCKHASH"

test_summary
