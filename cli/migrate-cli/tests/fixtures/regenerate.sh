#!/usr/bin/env bash
# Deliberately no -e: many captures below are EXPECTED to exit non-zero (that
# is the golden being captured — refusals, stalls, validation failures). Each
# invocation's exit code is captured explicitly into exit-code rather than
# relying on errexit, which would abort the whole run on the first such case.
set -uo pipefail

# Regenerate the bash-golden fixture matrix for accelerator-migrate (0172,
# Phase 0). Drives the real, still-live bash migration engine
# (skills/config/migrate/scripts/run-migrations.sh) as a black box and
# captures its observable artefacts (stdout, stderr, exit code, ledger/state
# files, and the resulting corpus tree where relevant) into this directory's
# fixture subdirectories, so the Rust port has a byte-level oracle.
#
# No Rust code exists yet at this point in the plan — this script's only job
# is faithful capture. Fixture-tree-building helpers below are adapted
# directly from the existing bash test suites (test-migrate.sh,
# test-migrate-interactive.sh) rather than reinvented, per this phase's own
# "modelled on the committed test driver" instruction.
#
# Determinism guarantees, mirroring hooks/test-fixtures/vcs-detect/regenerate.sh:
#   - TMPDIR is explicitly /tmp (or realpath-resolved).
#   - GIT_CEILING_DIRECTORIES scopes git's upward discovery to the capture
#     sandbox, preventing accidental walks into the accelerator's own .git.
#   - Every fixture's own CAPTURE-SOURCE.txt pins the bash-source revision so
#     the comparison harness can detect a stale golden.
#
# Linux is the canonical capture host; macOS captures may diverge in path
# normalisation (masked via masks.toml) but are useful for local iteration.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
DRIVER="$PLUGIN_ROOT/skills/config/migrate/scripts/run-migrations.sh"
BASH_MIGRATIONS_DIR="$PLUGIN_ROOT/skills/config/migrate/migrations"
BASH_TEST_FIXTURES="$PLUGIN_ROOT/skills/config/migrate/scripts/test-fixtures"
INTERACTIVE_FIXTURES="$BASH_TEST_FIXTURES/interactive"

TMPDIR=/tmp
WORK=$(mktemp -d "$TMPDIR/migrate-capture-XXXXXX")
export GIT_CEILING_DIRECTORIES="$WORK"
trap 'rm -rf "$WORK"' EXIT

# ── Capture helpers ──────────────────────────────────────────────────────────

# capture_source <fixture-dir>
#   Write CAPTURE-SOURCE.txt into a fixture directory: the bash-source
#   revision this golden was captured against, timestamp, host.
capture_source() {
  local out_dir="$1"
  {
    printf 'skills/config/migrate/scripts/run-migrations.sh: '
    (cd "$PLUGIN_ROOT" && git log -n1 --format=%H skills/config/migrate/scripts/run-migrations.sh 2>/dev/null ||
      jj log -r 'latest(::@ & file("skills/config/migrate/scripts/run-migrations.sh"))' --no-graph -T 'commit_id' 2>/dev/null ||
      echo UNKNOWN)
    printf '\nskills/config/migrate/scripts/interactive-lib.sh: '
    (cd "$PLUGIN_ROOT" && git log -n1 --format=%H skills/config/migrate/scripts/interactive-lib.sh 2>/dev/null ||
      jj log -r 'latest(::@ & file("skills/config/migrate/scripts/interactive-lib.sh"))' --no-graph -T 'commit_id' 2>/dev/null ||
      echo UNKNOWN)
    printf '\nCaptured: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'Host: %s\n' "$(uname -s)"
  } >"$out_dir/CAPTURE-SOURCE.txt"
}

# snapshot_state <fixture-dir> <repo>
#   Copy the post-run .accelerator/state/ ledger/manifest files verbatim.
snapshot_state() {
  local out_dir="$1" repo="$2"
  mkdir -p "$out_dir/state"
  [ -d "$repo/.accelerator/state" ] && cp -R "$repo/.accelerator/state/." "$out_dir/state/" || true
}

# snapshot_tree <fixture-dir> <repo>
#   Copy the post-run meta/ + .accelerator/ + .claude/ tree verbatim, for
#   corpus-state byte-parity comparisons.
snapshot_tree() {
  local out_dir="$1" repo="$2"
  mkdir -p "$out_dir/result-tree"
  for d in meta .accelerator .claude; do
    [ -e "$repo/$d" ] && cp -R "$repo/$d" "$out_dir/result-tree/" || true
  done
}

# ── Shared fixture inputs, adapted from test-migrate.sh / test-migrate-interactive.sh ──

# All bundled mechanical (non-INTERACTIVE) migrations, copied once — mirrors
# test-migrate.sh's MECHANICAL_MIGRATIONS_DIR.
MECHANICAL_DIR="$WORK/mechanical-migrations"
mkdir -p "$MECHANICAL_DIR"
for f in "$BASH_MIGRATIONS_DIR"/[0-9][0-9][0-9][0-9]-*.sh; do
  if ! grep -qE '^# INTERACTIVE:[[:space:]]*yes$' < <(head -5 "$f"); then
    cp "$f" "$MECHANICAL_DIR/"
  fi
done
# Default every subsequent invocation to the mechanical-only set (mirrors
# test-migrate.sh:29's global export) so an individual migration's fixture
# capture isn't derailed by 0007 (interactive) also being pending. Sections
# that need a narrower or different set override this inline.
export ACCELERATOR_MIGRATIONS_DIR="$MECHANICAL_DIR"

only_dir_for() { # only_dir_for <migration-basename.sh...> -> echoes new dir path
  local d
  d=$(mktemp -d "$WORK/only-XXXXXX")
  local f
  for f in "$@"; do
    cp "$BASH_MIGRATIONS_DIR/$f" "$d/"
  done
  printf '%s\n' "$d"
}

# setup_old_repo: temp dir with the pre-0001 legacy ticket structure, no VCS
# dir (find_repo_root falls back to $PWD; VCS clean-tree check is skipped).
# Verbatim adaptation of test-migrate.sh:63-75.
setup_old_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$WORK/repo-XXXXXX")
  mkdir -p "$repo_dir/meta/tickets"
  printf -- '---\nticket_id: 0001\n---\n\n# 0001: Foo\n' >"$repo_dir/meta/tickets/0001-foo.md"
  mkdir -p "$repo_dir/meta/reviews/tickets"
  printf -- '---\ntype: work-item-review\n---\n\n# foo-review-1\n' \
    >"$repo_dir/meta/reviews/tickets/foo-review-1.md"
  mkdir -p "$repo_dir/.claude"
  printf -- '---\npaths:\n  tickets: meta/tickets\n---\n' \
    >"$repo_dir/.claude/accelerator.md"
  echo "$repo_dir"
}

setup_0002_repo() {
  local repo
  repo=$(mktemp -d "$WORK/m0002-XXXXXX")
  cp -R "$BASH_TEST_FIXTURES/0002/." "$repo/"
  mkdir -p "$repo/.git" "$repo/meta" "$repo/.accelerator/state"
  printf '0001-rename-tickets-to-work\n0004-restructure-meta-research-into-subject-subcategories\n' \
    >"$repo/.accelerator/state/migrations-applied"
  printf '%s\n' "$repo"
}

setup_0003_repo() {
  local repo
  repo=$(mktemp -d "$WORK/m0003-XXXXXX")
  cp -R "$BASH_TEST_FIXTURES/0003/." "$repo/"
  mkdir -p "$repo/.accelerator/state"
  printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
    >"$repo/.accelerator/state/migrations-applied"
  printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n' \
    >"$repo/meta/.migrations-applied"
  printf '%s\n' "$repo"
}

setup_0004_repo() {
  local fixture="$1" repo
  repo=$(mktemp -d "$WORK/m0004-XXXXXX")
  cp -R "$BASH_TEST_FIXTURES/0004/$fixture/." "$repo/"
  mkdir -p "$repo/.accelerator/state"
  printf '0001-rename-tickets-to-work\n0002-rename-work-items-with-project-prefix\n0003-relocate-accelerator-state\n' \
    >"$repo/.accelerator/state/migrations-applied"
  printf '%s\n' "$repo"
}

setup_0005_repo() {
  local scenario="$1" repo
  repo=$(mktemp -d "$WORK/m0005-XXXXXX")
  cp -R "$BASH_TEST_FIXTURES/0005/$scenario/." "$repo/"
  echo "$repo"
}

setup_0006_repo() {
  local scenario="$1" repo
  repo=$(mktemp -d "$WORK/m0006-XXXXXX")
  cp -R "$BASH_TEST_FIXTURES/0006/$scenario/." "$repo/"
  echo "$repo"
}

setup_sandbox() { # setup_sandbox <name> — matches test-migrate-interactive.sh:21-27
  local name="$1" sandbox
  sandbox=$(mktemp -d "$WORK/$name-XXXXXX")
  mkdir -p "$sandbox/.git" "$sandbox/.accelerator/state"
  printf '%s\n' "$sandbox"
}

seed_predicate_sandbox() { # test-migrate-interactive.sh:360-369
  local sandbox="$1"
  shift
  mkdir -p "$sandbox/.fixture"
  : >"$sandbox/.fixture/transformations"
  local line
  for line in "$@"; do
    printf '%s\n' "$line" >>"$sandbox/.fixture/transformations"
  done
}

seed_bridge_corpus() { # test-migrate-interactive.sh:1552-1557
  local sbx="$1"
  mkdir -p "$sbx/meta/work"
  printf -- '---\nid: "0050"\n---\n' >"$sbx/meta/work/0050-example-a.md"
  printf -- '---\nid: "0051"\n---\n' >"$sbx/meta/work/0051-example-b.md"
  printf -- '---\nid: "0052"\n---\n' >"$sbx/meta/work/0052-example-c.md"
}

BRIDGE_DIR="$INTERACTIVE_FIXTURES/0006-decisions-bridge/migrations"
PREDICATE_DIR="$INTERACTIVE_FIXTURES/0002-predicate/migrations"

# gr_int_repo <vcs> — jj/git repo with an empty owned manifest seeded at the
# current base revision. Adapted from test-migrate-interactive.sh:1319-1332.
gr_base_rev() {
  local repo="$1" vcs="$2"
  if [ "$vcs" = jj ]; then
    (cd "$repo" && jj log -r @ --no-graph --no-pager -T change_id 2>/dev/null)
  else
    git -C "$repo" rev-parse HEAD
  fi
}

gr_int_repo() {
  local vcs="$1" repo
  repo=$(mktemp -d "$WORK/gr-int-$vcs-XXXXXX")
  mkdir -p "$repo/.accelerator/state" "$repo/meta"
  if [ "$vcs" = jj ]; then
    (cd "$repo" && jj git init --quiet)
  else
    git -C "$repo" init -q
    git -C "$repo" -c user.email=t@t -c user.name=T commit -q --allow-empty -m init
  fi
  : >"$repo/.accelerator/state/migrations-run-paths.txt"
  gr_base_rev "$repo" "$vcs" >"$repo/.accelerator/state/migrations-run.id"
  printf '%s\n' "$repo"
}

echo "Capturing accelerator-migrate bash goldens into $SCRIPT_DIR"

# ══════════════════════════════════════════════════════════════════════════
# Mechanical / ledger fixtures
# ══════════════════════════════════════════════════════════════════════════

echo "  all-pending/"
REPO=$(setup_old_repo)
OUT="$SCRIPT_DIR/all-pending"
mkdir -p "$OUT"
(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$MECHANICAL_DIR" \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
snapshot_tree "$OUT" "$REPO"
capture_source "$OUT"

echo "  0001/"
REPO=$(setup_old_repo)
OUT="$SCRIPT_DIR/0001"
mkdir -p "$OUT"
ONLY_0001=$(only_dir_for 0001-rename-tickets-to-work.sh)
(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$ONLY_0001" \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
snapshot_tree "$OUT" "$REPO"
capture_source "$OUT"

echo "  0002/"
REPO=$(setup_0002_repo)
OUT="$SCRIPT_DIR/0002"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
snapshot_tree "$OUT" "$REPO"
capture_source "$OUT"

echo "  0003/"
REPO=$(setup_0003_repo)
OUT="$SCRIPT_DIR/0003"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
snapshot_tree "$OUT" "$REPO"
capture_source "$OUT"

echo "  0004/"
REPO=$(setup_0004_repo default-layout)
OUT="$SCRIPT_DIR/0004"
mkdir -p "$OUT"
MIGRATION_0004="$BASH_MIGRATIONS_DIR/0004-restructure-meta-research-into-subject-subcategories.sh"
(PROJECT_ROOT="$REPO" \
  bash "$MIGRATION_0004" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
snapshot_tree "$OUT" "$REPO"
capture_source "$OUT"

echo "  0005/"
REPO=$(setup_0005_repo default-layout)
OUT="$SCRIPT_DIR/0005"
mkdir -p "$OUT"
ONLY_0005=$(only_dir_for 0005-rename-work-item-type-to-kind.sh)
(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$ONLY_0005" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
snapshot_tree "$OUT" "$REPO"
capture_source "$OUT"

echo "  0006/"
REPO=$(setup_0006_repo default-layout)
OUT="$SCRIPT_DIR/0006"
mkdir -p "$OUT"
ONLY_0006=$(only_dir_for 0006-canonicalise-work-item-id-and-author.sh)
(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$ONLY_0006" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
snapshot_tree "$OUT" "$REPO"
capture_source "$OUT"

# ══════════════════════════════════════════════════════════════════════════
# 0007/ — real interactive migration, no decisions channel → structured stall
# ══════════════════════════════════════════════════════════════════════════

echo "  0007/"
ONLY_0007=$(only_dir_for 0007-unify-meta-corpus-frontmatter.sh)
REPO=$(mktemp -d "$WORK/m0007-stall-XXXXXX")
mkdir -p "$REPO/meta/work"
printf -- '---\ntype: work-item\nid: "0001"\nwork_item_id: "0001"\ntitle: "Foo"\ndate: "2026-01-01T00:00:00+00:00"\nauthor: Fixture Author\nproducer: create-work-item\nstatus: ready\nkind: story\npriority: medium\ntags: []\nlast_updated: "2026-01-01T00:00:00+00:00"\nlast_updated_by: Fixture Author\nschema_version: 1\n---\n\n# 0001: Foo\n\n## References\n\n- See work-item:0002 for background.\n' \
  >"$REPO/meta/work/0001-foo.md"
printf -- '---\ntype: work-item\nid: "0002"\nwork_item_id: "0002"\ntitle: "Bar"\ndate: "2026-01-01T00:00:00+00:00"\nauthor: Fixture Author\nproducer: create-work-item\nstatus: ready\nkind: story\npriority: medium\ntags: []\nlast_updated: "2026-01-01T00:00:00+00:00"\nlast_updated_by: Fixture Author\nschema_version: 1\n---\n\n# 0002: Bar\n' \
  >"$REPO/meta/work/0002-bar.md"
OUT="$SCRIPT_DIR/0007"
mkdir -p "$OUT"
(cd "$REPO" && ACCELERATOR_MIGRATIONS_DIR="$ONLY_0007" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
capture_source "$OUT"

# ══════════════════════════════════════════════════════════════════════════
# interactive/ — scripted decision sequences against fixture interactive migrations
# ══════════════════════════════════════════════════════════════════════════

echo "  interactive/doc-example/"
SBX=$(setup_sandbox "doc-example")
DEC=$(mktemp "$WORK/dec-doc-example-XXXXXX")
printf 'edit \nedit 0123-renamed\nskip\n' >"$DEC"
OUT="$SCRIPT_DIR/interactive/doc-example"
mkdir -p "$OUT"
(ACCELERATOR_MIGRATIONS_DIR="$INTERACTIVE_FIXTURES/doc-example/migrations" \
  PROJECT_ROOT="$SBX" \
  ACCELERATOR_MIGRATE_FORCE=1 ACCELERATOR_MIGRATE_DECISIONS_FILE="$DEC" \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$SBX"
capture_source "$OUT"

echo "  interactive/accept-verb/"
SBX=$(setup_sandbox "accept-verb")
seed_predicate_sandbox "$SBX" "k1|f|a|original-value|ambiguous|p"
DEC=$(mktemp "$WORK/dec-accept-XXXXXX")
printf 'accept\n' >"$DEC"
OUT="$SCRIPT_DIR/interactive/accept-verb"
mkdir -p "$OUT"
(ACCELERATOR_MIGRATIONS_DIR="$PREDICATE_DIR" \
  PROJECT_ROOT="$SBX" \
  ACCELERATOR_MIGRATE_FORCE=1 ACCELERATOR_MIGRATE_DECISIONS_FILE="$DEC" \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$SBX"
capture_source "$OUT"

echo "  interactive/three-decision/"
SBX=$(setup_sandbox "three-decision")
seed_predicate_sandbox "$SBX" \
  "k1|f1|a|v1|ambiguous|p" \
  "k2|f2|a|v2|ambiguous|p" \
  "k3|f3|a|v3|ambiguous|p"
mkdir -p "$SBX/.accelerator/state"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
cat >"$LOG" <<'EOF'
{"transformation_key":"k1","schema_version":1,"outcome":"accepted","proposed_value":"v1","timestamp":"2026-05-30T12:00:00Z"}
{"transformation_key":"k2","schema_version":1,"outcome":"edited","proposed_value":"v2","user_value":"custom","timestamp":"2026-05-30T12:00:00Z"}
EOF
DEC=$(mktemp "$WORK/dec-three-XXXXXX")
printf 'accept\n' >"$DEC"
OUT="$SCRIPT_DIR/interactive/three-decision"
mkdir -p "$OUT"
(ACCELERATOR_MIGRATIONS_DIR="$PREDICATE_DIR" \
  PROJECT_ROOT="$SBX" \
  ACCELERATOR_MIGRATE_FORCE=1 ACCELERATOR_MIGRATE_DECISIONS_FILE="$DEC" \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$SBX"
capture_source "$OUT"

echo "  interactive/validator-rejecting/"
SBX=$(setup_sandbox "validator-rejecting")
seed_predicate_sandbox "$SBX" "k1|f|a|original-value|ambiguous|p"
DEC=$(mktemp "$WORK/dec-reject-XXXXXX")
printf 'edit \nedit recovered\n' >"$DEC"
OUT="$SCRIPT_DIR/interactive/validator-rejecting"
mkdir -p "$OUT"
(ACCELERATOR_MIGRATIONS_DIR="$PREDICATE_DIR" \
  PROJECT_ROOT="$SBX" \
  ACCELERATOR_MIGRATE_FORCE=1 ACCELERATOR_MIGRATE_DECISIONS_FILE="$DEC" \
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$SBX"
capture_source "$OUT"

echo "  interactive/foreign-dirty-path/ + interactive/two-owned-dirty-paths/ (git)"
# Both adapted from test-migrate-interactive.sh's near-miss (1469-1496) and
# mixed-owned (1499-1540) cases: a manifest listing meta/work/mech.md plus an
# owned session log; one variant dirties ALSO a path outside the manifest
# (foreign, refuses), the other dirties only manifest-owned paths (resumes).
build_dirty_repo() {
  local name="$1"
  local repo
  repo=$(mktemp -d "$WORK/$name-XXXXXX")
  mkdir -p "$repo/meta/work" "$repo/.accelerator/state"
  printf 'base\n' >"$repo/meta/work/mech.md"
  git -C "$repo" init -q
  git -C "$repo" -c user.email=t@t -c user.name=T add .
  git -C "$repo" -c user.email=t@t -c user.name=T commit -qm init
  printf '%s\n' "$repo"
}

REPO=$(build_dirty_repo "foreign-dirty")
rev=$(git -C "$REPO" rev-parse HEAD)
printf 'meta/work/mech.md\n' >"$REPO/.accelerator/state/migrations-run-paths.txt"
printf '%s\n' "$rev" >"$REPO/.accelerator/state/migrations-run.id"
printf 'x\n' >>"$REPO/meta/work/mech.md"
git -C "$REPO" add meta/work/mech.md
printf 'x\n' >"$REPO/meta/work/foreign.md"
git -C "$REPO" add meta/work/foreign.md
OUT="$SCRIPT_DIR/interactive/foreign-dirty-path"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
capture_source "$OUT"

REPO=$(build_dirty_repo "two-owned")
rev=$(git -C "$REPO" rev-parse HEAD)
LOG="$REPO/.accelerator/state/migrations-0099-session.jsonl"
printf '{"transformation_key":"k1","schema_version":1,"outcome":"accepted","proposed_value":"v1","timestamp":"2026-05-30T12:00:00Z"}\n' >"$LOG"
git -C "$REPO" add "$LOG"
printf 'meta/work/mech.md\n' >"$REPO/.accelerator/state/migrations-run-paths.txt"
printf '%s\n' "$rev" >"$REPO/.accelerator/state/migrations-run.id"
printf 'x\n' >>"$REPO/meta/work/mech.md"
git -C "$REPO" add meta/work/mech.md
OUT="$SCRIPT_DIR/interactive/two-owned-dirty-paths"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
snapshot_state "$OUT" "$REPO"
capture_source "$OUT"

# ══════════════════════════════════════════════════════════════════════════
# manifest-states/ — the four fail-closed states, git-backed
# ══════════════════════════════════════════════════════════════════════════

echo "  manifest-states/{absent,empty,unreadable,stale}/"
manifest_state_repo() {
  local repo
  repo=$(mktemp -d "$WORK/manifest-state-XXXXXX")
  mkdir -p "$repo/meta/work" "$repo/.accelerator/state"
  printf 'base\n' >"$repo/meta/work/dirty.md"
  git -C "$repo" init -q
  git -C "$repo" -c user.email=t@t -c user.name=T add .
  git -C "$repo" -c user.email=t@t -c user.name=T commit -qm init
  printf 'x\n' >>"$repo/meta/work/dirty.md"
  git -C "$repo" add meta/work/dirty.md
  printf '%s\n' "$repo"
}

REPO=$(manifest_state_repo)
OUT="$SCRIPT_DIR/manifest-states/absent"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
capture_source "$OUT"

REPO=$(manifest_state_repo)
: >"$REPO/.accelerator/state/migrations-run-paths.txt"
rev=$(git -C "$REPO" rev-parse HEAD)
printf '%s\n' "$rev" >"$REPO/.accelerator/state/migrations-run.id"
: >"$REPO/.accelerator/state/migrations-run.id" # empty sidecar (overwrites the valid one above)
OUT="$SCRIPT_DIR/manifest-states/empty"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
capture_source "$OUT"

REPO=$(manifest_state_repo)
: >"$REPO/.accelerator/state/migrations-run-paths.txt"
rev=$(git -C "$REPO" rev-parse HEAD)
printf '%s\n' "$rev" >"$REPO/.accelerator/state/migrations-run.id"
chmod 000 "$REPO/.accelerator/state/migrations-run.id"
OUT="$SCRIPT_DIR/manifest-states/unreadable"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
chmod 644 "$REPO/.accelerator/state/migrations-run.id" # restore so cleanup can remove it
capture_source "$OUT"

REPO=$(manifest_state_repo)
: >"$REPO/.accelerator/state/migrations-run-paths.txt"
printf 'stale-revision-sentinel\n' >"$REPO/.accelerator/state/migrations-run.id"
OUT="$SCRIPT_DIR/manifest-states/stale"
mkdir -p "$OUT"
(cd "$REPO" &&
  bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
capture_source "$OUT"

# ══════════════════════════════════════════════════════════════════════════
# list/ — --list output shape
# ══════════════════════════════════════════════════════════════════════════

echo "  list/single-pending/"
SBX=$(setup_sandbox "list-single")
seed_bridge_corpus "$SBX"
OUT="$SCRIPT_DIR/list/single-pending"
mkdir -p "$OUT"
(ACCELERATOR_MIGRATIONS_DIR="$BRIDGE_DIR" \
  PROJECT_ROOT="$SBX" \
  bash "$DRIVER" --list >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
capture_source "$OUT"

echo "  list/multi-pending/"
# Two already-committed interactive fixture migrations (0006-decisions-bridge,
# 0002-predicate), copied in rather than authored inline — lint:claude-
# coupling:check forbids this file (it lives under cli/) from naming the
# legacy plugin-root variable a self-authored migration sourcing
# interactive-harness.sh would need to read.
MULTI_DIR=$(mktemp -d "$WORK/multi-migs-XXXXXX")
cp "$BRIDGE_DIR/0006-decisions-bridge.sh" "$MULTI_DIR/0006-decisions-bridge.sh"
cp "$PREDICATE_DIR/0002-predicate.sh" "$MULTI_DIR/0002-predicate.sh"
SBX=$(setup_sandbox "list-multi")
seed_bridge_corpus "$SBX"
seed_predicate_sandbox "$SBX" "k1|f|a|original-value|ambiguous|p"
OUT="$SCRIPT_DIR/list/multi-pending"
mkdir -p "$OUT"
(ACCELERATOR_MIGRATIONS_DIR="$MULTI_DIR" \
  PROJECT_ROOT="$SBX" \
  bash "$DRIVER" --list >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
echo "$?" >"$OUT/exit-code"
capture_source "$OUT"

# ══════════════════════════════════════════════════════════════════════════
# decisions-file/ — dry-apply validation, fail-closed
# ══════════════════════════════════════════════════════════════════════════

run_bridge_capture() { # <name> <decisions-content>
  local name="$1" content="$2"
  local SBX OUT DEC
  SBX=$(setup_sandbox "decfile-$name")
  seed_bridge_corpus "$SBX"
  DEC=$(mktemp "$WORK/dec-$name-XXXXXX")
  printf '%s' "$content" >"$DEC"
  OUT="$SCRIPT_DIR/decisions-file/$name"
  mkdir -p "$OUT"
  (ACCELERATOR_MIGRATIONS_DIR="$BRIDGE_DIR" \
    PROJECT_ROOT="$SBX" \
    ACCELERATOR_MIGRATE_FORCE=1 ACCELERATOR_MIGRATE_DECISIONS_FILE="$DEC" \
    bash "$DRIVER" >"$OUT/stdout" 2>"$OUT/stderr" </dev/null)
  echo "$?" >"$OUT/exit-code"
  snapshot_tree "$OUT" "$SBX"
  capture_source "$OUT"
}

echo "  decisions-file/too-few/"
run_bridge_capture "too-few" $'accept\nskip\n'

echo "  decisions-file/too-many/"
run_bridge_capture "too-many" $'accept\nskip\nedit work-item:0100\naccept\n'

echo "  decisions-file/unknown-verb/"
run_bridge_capture "unknown-verb" $'accept\nfrobnicate\nedit work-item:0100\n'

echo "  decisions-file/rejected-edit-no-recovery/"
run_bridge_capture "rejected-edit-no-recovery" $'accept\nskip\nedit\n'

echo "  decisions-file/blank-crlf-comments/"
run_bridge_capture "blank-crlf-comments" $'accept\r\n\r\n# a comment line\r\nskip\r\nedit work-item:0100\r\n'

echo ""
echo "Capture complete. Verify with: find $SCRIPT_DIR -maxdepth 3 -name exit-code | sort"
