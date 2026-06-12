#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

WRITE_CONFIG="$SCRIPT_DIR/write-visualiser-config.sh"

# Sourced for config_enumerate_templates so the discovered-set assertions below
# derive the expected keys the same way the launcher does. Sourced *after*
# WRITE_CONFIG is assigned: config-common.sh reassigns SCRIPT_DIR to the plugin
# scripts/ dir, which would otherwise repoint WRITE_CONFIG.
source "$PLUGIN_ROOT/scripts/config-common.sh"

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

# ─── 20. editor: no env, no config → key absent ──────────────────────────────
echo "Test: no editor configured → editor/editor_project keys absent from config.json"
PROJ20="$TMPDIR_BASE/t-editor-absent"
make_project "$PROJ20"
OUT20_FILE="$TMPDIR_BASE/out20.json"
run_config "$PROJ20" >"$OUT20_FILE"
assert_json_eq "editor key absent (null)" ".editor" "null" "$OUT20_FILE"
assert_json_eq "editor_project key absent (null)" ".editor_project" "null" "$OUT20_FILE"
OUT20_TEXT="$(cat "$OUT20_FILE")"
assert_not_contains "no editor key emitted when unset" "$OUT20_TEXT" '"editor"'

# ─── 21. editor: config value, no env → config-over-default ──────────────────
echo "Test: visualiser.editor + editor_project in config → emitted verbatim"
PROJ21="$TMPDIR_BASE/t-editor-config"
make_project "$PROJ21"
write_config "$PROJ21" 'visualiser:
  editor: cursor
  editor_project: myrepo'
OUT21_FILE="$TMPDIR_BASE/out21.json"
run_config "$PROJ21" >"$OUT21_FILE"
assert_json_eq "editor reflects config" ".editor" "cursor" "$OUT21_FILE"
assert_json_eq "editor_project reflects config" ".editor_project" "myrepo" "$OUT21_FILE"

# ─── 22. editor: env over config ─────────────────────────────────────────────
echo "Test: ACCELERATOR_VISUALISER_EDITOR(_PROJECT) override config (env-over-config)"
PROJ22="$TMPDIR_BASE/t-editor-env"
make_project "$PROJ22"
write_config "$PROJ22" 'visualiser:
  editor: cursor
  editor_project: myrepo'
OUT22_FILE="$TMPDIR_BASE/out22.json"
ACCELERATOR_VISUALISER_EDITOR=vscode ACCELERATOR_VISUALISER_EDITOR_PROJECT=otherrepo \
  run_config "$PROJ22" >"$OUT22_FILE"
assert_json_eq "editor env wins over config" ".editor" "vscode" "$OUT22_FILE"
assert_json_eq "editor_project env wins over config" ".editor_project" "otherrepo" "$OUT22_FILE"

# ─── 23. editor: empty env falls through to config ───────────────────────────
echo "Test: empty ACCELERATOR_VISUALISER_EDITOR falls through to config"
PROJ23="$TMPDIR_BASE/t-editor-empty-env"
make_project "$PROJ23"
write_config "$PROJ23" 'visualiser:
  editor: cursor'
OUT23_FILE="$TMPDIR_BASE/out23.json"
ACCELERATOR_VISUALISER_EDITOR="" run_config "$PROJ23" >"$OUT23_FILE"
assert_json_eq "empty env does not override config" ".editor" "cursor" "$OUT23_FILE"

# ─── 24. editor: whitespace-only → key absent ────────────────────────────────
echo "Test: whitespace-only ACCELERATOR_VISUALISER_EDITOR → key absent"
PROJ24="$TMPDIR_BASE/t-editor-ws"
make_project "$PROJ24"
OUT24_FILE="$TMPDIR_BASE/out24.json"
ACCELERATOR_VISUALISER_EDITOR="   " run_config "$PROJ24" >"$OUT24_FILE"
assert_json_eq "whitespace-only editor collapses to absent" ".editor" "null" "$OUT24_FILE"

# ─── 25. editor: custom template with :// and a space round-trips intact ─────
# Guards the macOS bash 3.2 quoting gotcha: the value carries a scheme, a
# placeholder, and an embedded space, all of which must survive verbatim.
echo "Test: custom-template editor with '://' and a space round-trips intact"
PROJ25="$TMPDIR_BASE/t-editor-template"
make_project "$PROJ25"
write_config "$PROJ25" 'visualiser:
  editor: "zed://open?path={abs}&name=My Project"'
OUT25_FILE="$TMPDIR_BASE/out25.json"
run_config "$PROJ25" >"$OUT25_FILE"
assert_json_eq "custom template round-trips with :// and space" \
  ".editor" "zed://open?path={abs}&name=My Project" "$OUT25_FILE"

# ─── templates: discovered set matches the templates/ directory ──────────────
echo "Test: templates object lists every *.md in the plugin templates/ dir"
PROJ_TD="$TMPDIR_BASE/t-templates-discovered"
make_project "$PROJ_TD"
OUT_TD="$TMPDIR_BASE/out-td.json"
run_config "$PROJ_TD" >"$OUT_TD"
EXPECTED_KEYS="$(config_enumerate_templates "$PLUGIN_ROOT" | sort | tr '\n' ' ')"
ACTUAL_KEYS="$(jq -r '.templates | keys[]' "$OUT_TD" | sort | tr '\n' ' ')"
assert_eq "templates keys match templates/ dir" "$EXPECTED_KEYS" "$ACTUAL_KEYS"

# Tier wiring flows through template_tier for a previously-hidden template.
# NOTE: user_override is the *unconditional candidate* path template_tier emits
# (make_project never creates .accelerator/templates/); the server decides
# present/absent. We pin the candidate path the launcher wires, not presence.
assert_json_eq "rca plugin_default points at plugin templates/" \
  ".templates.rca.plugin_default" "$PLUGIN_ROOT/templates/rca.md" "$OUT_TD"
assert_json_eq "note user_override points at project .accelerator/templates" \
  ".templates.note.user_override" "$PROJ_TD/.accelerator/templates/note.md" "$OUT_TD"

# Fourth tier key: config_override_source. Null with no config override — this
# guards the key the jq restructure is most likely to drop (it feeds the view's
# Tier 1 description and has no other automated check).
assert_json_eq "rca config_override_source null with no override" \
  ".templates.rca.config_override_source" "null" "$OUT_TD"

# …and populated when a config.md declares the override (exercises the 4th key
# in its non-null form, plus the provenance scan in template_tier).
echo "Test: config_override_source records the declaring config file"
PROJ_CS="$TMPDIR_BASE/t-templates-override-source"
make_project "$PROJ_CS"
mkdir -p "$PROJ_CS/custom"
echo "# custom rca" >"$PROJ_CS/custom/rca.md"
printf -- '---\ntemplates:\n  rca: custom/rca.md\n---\n' >"$PROJ_CS/.accelerator/config.md"
OUT_CS="$TMPDIR_BASE/out-cs.json"
run_config "$PROJ_CS" >"$OUT_CS"
# Pin both co-dependent halves of the populated-override shape: the path itself
# and its provenance. (Asserting only the source would miss a dropped path.)
assert_json_eq "rca config_override reflects the declared path" \
  ".templates.rca.config_override" "custom/rca.md" "$OUT_CS"
assert_json_eq "rca config_override_source names config.md" \
  ".templates.rca.config_override_source" ".accelerator/config.md" "$OUT_CS"

# ─── config-override-only template is NOT surfaced (templates/ is canonical) ──
# NOTE: a characterisation/lock test — GREEN under both the old 8-name roster and
# the new discovery (zzz-fake is in neither). It pins that .accelerator/templates/
# is not a discovery source, guarding a future change that scanned the override dir.
echo "Test: a template present only in .accelerator/templates is not surfaced"
PROJ_OO="$TMPDIR_BASE/t-templates-override-only"
make_project "$PROJ_OO"
mkdir -p "$PROJ_OO/.accelerator/templates"
echo "# fake" >"$PROJ_OO/.accelerator/templates/zzz-fake.md"
OUT_OO="$TMPDIR_BASE/out-oo.json"
run_config "$PROJ_OO" >"$OUT_OO"
assert_json_eq "override-only template absent from set" \
  '.templates | has("zzz-fake")' "false" "$OUT_OO"

echo ""
test_summary
