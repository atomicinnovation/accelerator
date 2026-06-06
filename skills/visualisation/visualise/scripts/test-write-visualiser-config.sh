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
  mkdir -p "$d/.jj" "$d/.accelerator/tmp"
  : >"$d/.accelerator/tmp/.gitignore"
}

# Write a config.md with given frontmatter body into a project.
write_config() {
  local proj="$1"
  local body="$2"
  mkdir -p "$proj/.accelerator"
  printf -- "---\n%s\n---\n" "$body" >"$proj/.accelerator/config.md"
}

run_config() {
  local proj="$1"
  shift
  (
    cd "$proj"
    "$WRITE_CONFIG" \
      --plugin-version "0.0.0-test" \
      --project-root "$proj" \
      --tmp-dir "$proj/.accelerator/tmp/visualiser" \
      --log-file "$proj/.accelerator/tmp/visualiser/server.log" \
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
run_config "$PROJ1" >"$OUT1_FILE"
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
  --tmp-dir "$PROJ2/.accelerator/tmp/visualiser" \
  --log-file "$PROJ2/.accelerator/tmp/visualiser/server.log" \
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
run_config "$PROJ3" >"$OUT3_FILE"
assert_json_eq "doc_paths.work reflects override" ".doc_paths.work" "$PROJ3/meta/items" "$OUT3_FILE"

# ─── 4. kanban_columns: missing → 7 defaults ─────────────────────────────────
echo "Test: missing visualiser.kanban_columns → 7 defaults in config"
PROJ4="$TMPDIR_BASE/t-kanban-default"
make_project "$PROJ4"
OUT4_FILE="$TMPDIR_BASE/out4.json"
run_config "$PROJ4" >"$OUT4_FILE"
assert_json_eq "kanban_columns has 7 entries" ".kanban_columns | length" "7" "$OUT4_FILE"
assert_json_eq "kanban_columns[0] is draft" ".kanban_columns[0]" "draft" "$OUT4_FILE"
assert_json_eq "kanban_columns[6] is abandoned" ".kanban_columns[6]" "abandoned" "$OUT4_FILE"

# ─── 5. kanban_columns: custom array → reflected in config ──────────────────
echo "Test: custom visualiser.kanban_columns reflected in config"
PROJ5="$TMPDIR_BASE/t-kanban-custom"
make_project "$PROJ5"
write_config "$PROJ5" "visualiser:
  kanban_columns: [ready, in-progress, review, done]"
OUT5_FILE="$TMPDIR_BASE/out5.json"
run_config "$PROJ5" >"$OUT5_FILE"
assert_json_eq "kanban_columns has 4 entries" ".kanban_columns | length" "4" "$OUT5_FILE"
assert_json_eq "kanban_columns[0] is ready" ".kanban_columns[0]" "ready" "$OUT5_FILE"
assert_json_eq "kanban_columns[3] is done" ".kanban_columns[3]" "done" "$OUT5_FILE"

# ─── 6. kanban_columns: empty list → non-zero exit ──────────────────────────
echo "Test: empty visualiser.kanban_columns → non-zero exit"
PROJ6="$TMPDIR_BASE/t-kanban-empty"
make_project "$PROJ6"
write_config "$PROJ6" "visualiser:
  kanban_columns: []"
EXIT6=0
STDERR6="$(cd "$PROJ6" && "$WRITE_CONFIG" \
  --plugin-version "0.0.0-test" \
  --project-root "$PROJ6" \
  --tmp-dir "$PROJ6/.accelerator/tmp/visualiser" \
  --log-file "$PROJ6/.accelerator/tmp/visualiser/server.log" \
  2>&1 >/dev/null)" || EXIT6=$?
assert_eq "empty kanban_columns exits non-zero" "1" "$EXIT6"
assert_contains "stderr mentions empty" "$STDERR6" "empty"

# ─── 7. kanban_columns: malformed (unclosed bracket) → non-zero exit ─────────
echo "Test: malformed visualiser.kanban_columns (unclosed bracket) → non-zero exit"
PROJ7="$TMPDIR_BASE/t-kanban-malformed"
make_project "$PROJ7"
write_config "$PROJ7" 'visualiser:
  kanban_columns: "[ready, in-progress"'
EXIT7=0
# shellcheck disable=SC2034 # command run for its exit status (EXIT7); captured stderr intentionally unused
STDERR7="$(cd "$PROJ7" && "$WRITE_CONFIG" \
  --plugin-version "0.0.0-test" \
  --project-root "$PROJ7" \
  --tmp-dir "$PROJ7/.accelerator/tmp/visualiser" \
  --log-file "$PROJ7/.accelerator/tmp/visualiser/server.log" \
  2>&1 >/dev/null)" || EXIT7=$?
assert_eq "malformed kanban_columns exits non-zero" "1" "$EXIT7"

# ─── 8. idle_timeout: no env, no config → key absent ─────────────────────────
echo "Test: no idle_timeout configured → key absent from config.json"
PROJ8="$TMPDIR_BASE/t-idle-absent"
make_project "$PROJ8"
OUT8_FILE="$TMPDIR_BASE/out8.json"
run_config "$PROJ8" >"$OUT8_FILE"
assert_json_eq "idle_timeout key absent (null)" ".idle_timeout" "null" "$OUT8_FILE"

# ─── 9. idle_timeout: config value, no env → config-over-default ─────────────
echo "Test: visualiser.idle_timeout in config → emitted verbatim"
PROJ9="$TMPDIR_BASE/t-idle-config"
make_project "$PROJ9"
write_config "$PROJ9" 'visualiser:
  idle_timeout: "30m"'
OUT9_FILE="$TMPDIR_BASE/out9.json"
run_config "$PROJ9" >"$OUT9_FILE"
assert_json_eq "idle_timeout reflects config" ".idle_timeout" "30m" "$OUT9_FILE"

# ─── 10. idle_timeout: numeric 0 → survives as string "0" ────────────────────
echo "Test: numeric visualiser.idle_timeout: 0 → string \"0\""
PROJ10="$TMPDIR_BASE/t-idle-zero"
make_project "$PROJ10"
write_config "$PROJ10" 'visualiser:
  idle_timeout: 0'
OUT10_FILE="$TMPDIR_BASE/out10.json"
run_config "$PROJ10" >"$OUT10_FILE"
assert_json_eq "numeric 0 token survives as string" ".idle_timeout" "0" "$OUT10_FILE"

# ─── 11. idle_timeout: mixed-case Never → passes through untouched ────────────
echo "Test: visualiser.idle_timeout: Never → passes through (case-folding is Rust's job)"
PROJ11="$TMPDIR_BASE/t-idle-never"
make_project "$PROJ11"
write_config "$PROJ11" 'visualiser:
  idle_timeout: "Never"'
OUT11_FILE="$TMPDIR_BASE/out11.json"
run_config "$PROJ11" >"$OUT11_FILE"
assert_json_eq "mixed-case Never passes through" ".idle_timeout" "Never" "$OUT11_FILE"

# ─── 12. idle_timeout: env over config ───────────────────────────────────────
echo "Test: ACCELERATOR_VISUALISER_IDLE_TIMEOUT overrides config (env-over-config)"
PROJ12="$TMPDIR_BASE/t-idle-env"
make_project "$PROJ12"
write_config "$PROJ12" 'visualiser:
  idle_timeout: "30m"'
OUT12_FILE="$TMPDIR_BASE/out12.json"
ACCELERATOR_VISUALISER_IDLE_TIMEOUT=2h run_config "$PROJ12" >"$OUT12_FILE"
assert_json_eq "env value wins over config" ".idle_timeout" "2h" "$OUT12_FILE"

# ─── 13. idle_timeout: empty env falls through to config ──────────────────────
echo "Test: empty ACCELERATOR_VISUALISER_IDLE_TIMEOUT falls through to config"
PROJ13="$TMPDIR_BASE/t-idle-empty-env"
make_project "$PROJ13"
write_config "$PROJ13" 'visualiser:
  idle_timeout: "30m"'
OUT13_FILE="$TMPDIR_BASE/out13.json"
ACCELERATOR_VISUALISER_IDLE_TIMEOUT="" run_config "$PROJ13" >"$OUT13_FILE"
assert_json_eq "empty env does not override config" ".idle_timeout" "30m" "$OUT13_FILE"

# ─── 14. idle_timeout: zero-length duration "0s" passes guard ────────────────
echo "Test: visualiser.idle_timeout: 0s → passes coarse guard, emitted verbatim"
PROJ14="$TMPDIR_BASE/t-idle-zero-s"
make_project "$PROJ14"
write_config "$PROJ14" 'visualiser:
  idle_timeout: "0s"'
OUT14_FILE="$TMPDIR_BASE/out14.json"
run_config "$PROJ14" >"$OUT14_FILE"
assert_json_eq "0s passes guard, emitted verbatim" ".idle_timeout" "0s" "$OUT14_FILE"

# ─── 15. idle_timeout: compound + spaced forms not rejected by guard ─────────
echo "Test: compound 1h30m and spaced '1h 30m' pass the digit-led guard"
PROJ15="$TMPDIR_BASE/t-idle-compound"
make_project "$PROJ15"
write_config "$PROJ15" 'visualiser:
  idle_timeout: "1h30m"'
OUT15_FILE="$TMPDIR_BASE/out15.json"
run_config "$PROJ15" >"$OUT15_FILE"
assert_json_eq "compound 1h30m emitted verbatim" ".idle_timeout" "1h30m" "$OUT15_FILE"
PROJ15B="$TMPDIR_BASE/t-idle-spaced"
make_project "$PROJ15B"
write_config "$PROJ15B" 'visualiser:
  idle_timeout: "1h 30m"'
OUT15B_FILE="$TMPDIR_BASE/out15b.json"
run_config "$PROJ15B" >"$OUT15B_FILE"
assert_json_eq "spaced '1h 30m' emitted verbatim" ".idle_timeout" "1h 30m" "$OUT15B_FILE"

# ─── 16. idle_timeout: whitespace-padded env is trimmed ──────────────────────
echo "Test: whitespace-padded env ' 8h ' is trimmed before emission"
PROJ16="$TMPDIR_BASE/t-idle-padded"
make_project "$PROJ16"
OUT16_FILE="$TMPDIR_BASE/out16.json"
ACCELERATOR_VISUALISER_IDLE_TIMEOUT=" 8h " run_config "$PROJ16" >"$OUT16_FILE"
assert_json_eq "padded env trimmed to 8h" ".idle_timeout" "8h" "$OUT16_FILE"

# ─── 17. idle_timeout: guard-accepted-but-Rust-invalid passes through ────────
echo "Test: digit-led but Rust-invalid '5 zonks' passes the coarse guard"
PROJ17="$TMPDIR_BASE/t-idle-zonks"
make_project "$PROJ17"
OUT17_FILE="$TMPDIR_BASE/out17.json"
ACCELERATOR_VISUALISER_IDLE_TIMEOUT="5 zonks" run_config "$PROJ17" >"$OUT17_FILE"
assert_json_eq "5 zonks passes coarse guard (Rust is authoritative)" ".idle_timeout" "5 zonks" "$OUT17_FILE"

# ─── 18. idle_timeout: quoted vs unquoted config scalar ──────────────────────
echo "Test: quoted and unquoted idle_timeout config forms both yield 8h"
PROJ18="$TMPDIR_BASE/t-idle-quoted"
make_project "$PROJ18"
write_config "$PROJ18" 'visualiser:
  idle_timeout: "8h"'
OUT18_FILE="$TMPDIR_BASE/out18.json"
run_config "$PROJ18" >"$OUT18_FILE"
assert_json_eq "quoted idle_timeout yields 8h" ".idle_timeout" "8h" "$OUT18_FILE"
PROJ18B="$TMPDIR_BASE/t-idle-unquoted"
make_project "$PROJ18B"
write_config "$PROJ18B" 'visualiser:
  idle_timeout: 8h'
OUT18B_FILE="$TMPDIR_BASE/out18b.json"
run_config "$PROJ18B" >"$OUT18B_FILE"
assert_json_eq "unquoted idle_timeout yields 8h" ".idle_timeout" "8h" "$OUT18B_FILE"

# ─── 19. idle_timeout: invalid shape rejected on the terminal ────────────────
echo "Test: invalid visualiser.idle_timeout: soon → non-zero exit + terminal error + no config"
PROJ19="$TMPDIR_BASE/t-idle-invalid"
make_project "$PROJ19"
write_config "$PROJ19" 'visualiser:
  idle_timeout: "soon"'
OUT19_FILE="$TMPDIR_BASE/out19.json"
EXIT19=0
STDERR19="$(cd "$PROJ19" && "$WRITE_CONFIG" \
  --plugin-version "0.0.0-test" \
  --project-root "$PROJ19" \
  --tmp-dir "$PROJ19/.accelerator/tmp/visualiser" \
  --log-file "$PROJ19/.accelerator/tmp/visualiser/server.log" \
  2>&1 >"$OUT19_FILE")" || EXIT19=$?
assert_eq "invalid idle_timeout exits non-zero" "1" "$EXIT19"
assert_contains "stderr names the bad value" "$STDERR19" "invalid visualiser.idle_timeout 'soon'"
assert_empty "no config.json emitted on invalid idle_timeout" "$(cat "$OUT19_FILE")"

echo ""
test_summary
