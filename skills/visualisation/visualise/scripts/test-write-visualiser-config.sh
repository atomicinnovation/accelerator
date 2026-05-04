#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

WRITE_CONFIG="$SCRIPT_DIR/write-visualiser-config.sh"

TMPDIR_BASE="$(mktemp -d)"
ORIG_DIR="$PWD"
trap 'cd "$ORIG_DIR"; rm -rf "$TMPDIR_BASE"' EXIT

# Build a minimal project: needs .jj so config-read scripts find the repo root.
make_project() {
  local d="$1"
  mkdir -p "$d/.jj" "$d/.claude" "$d/meta/tmp"
  : > "$d/meta/tmp/.gitignore"
}

# Write an accelerator.md with given frontmatter body into a project.
write_config() {
  local proj="$1"
  local body="$2"
  printf -- "---\n%s\n---\n" "$body" > "$proj/.claude/accelerator.md"
}

run_config() {
  local proj="$1"
  shift
  (
    cd "$proj"
    "$WRITE_CONFIG" \
      --plugin-version "0.0.0-test" \
      --project-root "$proj" \
      --tmp-dir "$proj/meta/tmp/visualiser" \
      --log-file "$proj/meta/tmp/visualiser/server.log" \
      "$@"
  )
}

echo "=== test-write-visualiser-config.sh ==="
echo ""

# ─── 1. Default paths (no overrides) ──────────────────────────────────────────
echo "Test: default config produces doc_paths.work and doc_paths.review_work"
PROJ1="$TMPDIR_BASE/t-default"
make_project "$PROJ1"
# No accelerator.md — use pure defaults
OUT1_FILE="$TMPDIR_BASE/out1.json"
run_config "$PROJ1" > "$OUT1_FILE"
assert_json_eq "doc_paths.work is meta/work" ".doc_paths.work" "$PROJ1/meta/work" "$OUT1_FILE"
assert_json_eq "doc_paths.review_work is meta/reviews/work" ".doc_paths.review_work" "$PROJ1/meta/reviews/work" "$OUT1_FILE"
OUT1_TEXT="$(cat "$OUT1_FILE")"
assert_not_contains "doc_paths must not contain tickets key" "$OUT1_TEXT" '"tickets"'

# ─── 2. Pre-migration project (paths.tickets set, no paths.work) ──────────────
echo "Test: pre-migration project (paths.tickets without paths.work) → non-zero exit with migrate hint"
PROJ2="$TMPDIR_BASE/t-premigration"
make_project "$PROJ2"
write_config "$PROJ2" "paths:
  tickets: meta/old-tickets"
# Should exit non-zero and emit a migrate hint to stderr
STDERR2=""
EXIT2=0
STDERR2="$(cd "$PROJ2" && "$WRITE_CONFIG" \
    --plugin-version "0.0.0-test" \
    --project-root "$PROJ2" \
    --tmp-dir "$PROJ2/meta/tmp/visualiser" \
    --log-file "$PROJ2/meta/tmp/visualiser/server.log" \
    2>&1 >/dev/null)" || EXIT2=$?
assert_eq "non-zero exit for pre-migration project" "1" "$EXIT2"
assert_contains "stderr names the migration" "$STDERR2" "migrate"

# ─── 3. paths.work override is reflected in config ────────────────────────────
echo "Test: paths.work override reflected in doc_paths.work"
PROJ3="$TMPDIR_BASE/t-override"
make_project "$PROJ3"
write_config "$PROJ3" "paths:
  work: meta/items"
OUT3_FILE="$TMPDIR_BASE/out3.json"
run_config "$PROJ3" > "$OUT3_FILE"
assert_json_eq "doc_paths.work reflects override" ".doc_paths.work" "$PROJ3/meta/items" "$OUT3_FILE"

echo ""
test_summary
