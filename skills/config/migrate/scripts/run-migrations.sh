#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNNER_SCRIPT_DIR="$SCRIPT_DIR"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../../.." && pwd)}"
source "$PLUGIN_ROOT/scripts/config-common.sh"
# shellcheck source=../../../../scripts/atomic-common.sh
source "$PLUGIN_ROOT/scripts/atomic-common.sh"

# ── 1. Resolve PROJECT_ROOT ──────────────────────────────────────────────────
PROJECT_ROOT="${PROJECT_ROOT:-$(config_project_root)}"

# ── 1a. Decisions-file env var + --decisions-file flag ──────────────────────
# Scripted decisions for interactive migrations: one decision per line
# (accept | skip | edit <value>). Reachable two ways — the documented
# --decisions-file flag (parsed below) and this env var, which the no-input
# stall advertises as the equivalent resume form. The relocated validation
# block (1b, after flag parsing) checks whichever form supplied the path, so
# failures land here with a clear message rather than deep inside the prompt
# loop. (0117 promotes the env var into --help and adds --list.)
ACCELERATOR_MIGRATE_DECISIONS_FILE="${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}"
export ACCELERATOR_MIGRATE_DECISIONS_FILE

STATE_FILE="$PROJECT_ROOT/.accelerator/state/migrations-applied"
SKIP_FILE="$PROJECT_ROOT/.accelerator/state/migrations-skipped"
# Per-run path manifest (sidecar pair). RUN_PATHS_FILE is a pure list of
# repo-relative paths this run mutated (one per line, deduped); RUN_ID_FILE
# records the base revision the run started against for the staleness gate.
# Both live under .accelerator/state/ and are deleted on full success.
RUN_PATHS_FILE="$PROJECT_ROOT/.accelerator/state/migrations-run-paths.txt"
RUN_ID_FILE="$PROJECT_ROOT/.accelerator/state/migrations-run.id"

# ── --skip / --unskip / --decisions-file / --help flags ──────────────────────
if [ $# -gt 0 ]; then
  case "$1" in
    --skip)
      if [ $# -lt 2 ]; then
        echo "Usage: run-migrations.sh --skip <migration-id>" >&2
        exit 1
      fi
      mkdir -p "$(dirname "$SKIP_FILE")"
      atomic_append_unique "$SKIP_FILE" "$2"
      echo "Skipped migration: $2"
      exit 0
      ;;
    --unskip)
      if [ $# -lt 2 ]; then
        echo "Usage: run-migrations.sh --unskip <migration-id>" >&2
        exit 1
      fi
      atomic_remove_line "$SKIP_FILE" "$2"
      echo "Unskipped migration: $2"
      exit 0
      ;;
    --decisions-file)
      if [ $# -lt 2 ]; then
        echo "Usage: run-migrations.sh --decisions-file <path>" >&2
        exit 1
      fi
      # Unlike --skip/--unskip (which exit 0), this flag sets the env var and
      # falls through to a normal migration run.
      ACCELERATOR_MIGRATE_DECISIONS_FILE="$2"
      export ACCELERATOR_MIGRATE_DECISIONS_FILE
      shift 2
      ;;
    --help | -h)
      cat >&2 <<'EOF'
Usage: run-migrations.sh [FLAG]
  --skip <id>             Mark migration <id> skipped; do not run it.
  --unskip <id>           Remove migration <id> from the skip list.
  --decisions-file <path> Scripted decisions for interactive migrations, one
                          per line: accept | skip | edit <value>. The resume
                          path the no-input stall points at.
EOF
      exit 0
      ;;
  esac
fi

# ── 1b. Validate the decisions file (env- or flag-supplied) ──────────────────
if [ -n "$ACCELERATOR_MIGRATE_DECISIONS_FILE" ]; then
  if [ -d "$ACCELERATOR_MIGRATE_DECISIONS_FILE" ]; then
    echo "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE is a directory:" \
      "$ACCELERATOR_MIGRATE_DECISIONS_FILE" >&2
    exit 1
  fi
  if [ ! -e "$ACCELERATOR_MIGRATE_DECISIONS_FILE" ]; then
    echo "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE does not exist:" \
      "$ACCELERATOR_MIGRATE_DECISIONS_FILE" >&2
    exit 1
  fi
  if [ ! -r "$ACCELERATOR_MIGRATE_DECISIONS_FILE" ]; then
    echo "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE is not readable:" \
      "$ACCELERATOR_MIGRATE_DECISIONS_FILE" >&2
    exit 1
  fi
fi

# ── enumerate_scoped_dirty <vcs> ─────────────────────────────────────────────
#   Emit one normalized repo-relative path per line for uncommitted changes
#   under meta/, .claude/accelerator*.md, .accelerator/. Single source of truth
#   for the pre-flight owned-check and the apply-loop manifest recording, so the
#   jj-vs-git untracked asymmetry has exactly one definition.
#
#   The git branch strips the porcelain status prefix to a bare repo-relative
#   path (so recorded paths string-match dirty paths at resume time), and
#   resolves rename porcelain (`R  old -> new`) to the *new* path via the
#   trailing `s/^.* -> //`. The jj-vs-git untracked asymmetry is deliberate: the
#   git branch keeps `grep -v '^??'` (excludes untracked, preserving the existing
#   guard's git behaviour); jj tracks created files by default and includes them.
enumerate_scoped_dirty() {
  local vcs="$1"
  if [ "$vcs" = "jj" ] && command -v jj >/dev/null 2>&1; then
    jj --no-pager diff --name-only 2>/dev/null |
      grep -E '^(meta/|\.claude/accelerator|\.accelerator/)' || true
  elif [ "$vcs" = "git" ]; then
    git -C "$PROJECT_ROOT" status --porcelain \
      "meta/" ".claude/accelerator.md" ".claude/accelerator.local.md" \
      ".accelerator/" 2>/dev/null |
      grep -v '^??' |
      sed 's/^[[:space:]]*//; s/^[A-Z?][[:space:]]*//; s/^.* -> //' || true
  fi
}

# ── manifest_record_delta <vcs> <baseline_file> ──────────────────────────────
#   Append every currently-dirty scoped path not present in <baseline_file> to
#   the run manifest, deduped (atomic_append_unique is idempotent). Paths are
#   already repo-relative (single enumeration source), so no normalization is
#   needed. Takes vcs as a parameter rather than reading an ambient global,
#   matching enumerate_scoped_dirty's signature (unbound-safe under set -u).
manifest_record_delta() {
  local vcs="$1" baseline="$2" path
  mkdir -p "$(dirname "$RUN_PATHS_FILE")"
  while IFS= read -r path; do
    [ -z "$path" ] && continue
    grep -Fxq -- "$path" "$baseline" 2>/dev/null && continue
    atomic_append_unique "$RUN_PATHS_FILE" "$path"
  done < <(enumerate_scoped_dirty "$vcs")
}

# ── current_base_revision <vcs> ──────────────────────────────────────────────
#   Emit the committed base revision the working copy sits on. For jj this is
#   the change_id of @ — STABLE while the working copy is edited (it moves only
#   on `jj new`/`jj commit`), unlike commit_id (a content hash that changes on
#   every write). For git it is HEAD, stable across uncommitted edits.
#   Migrations never commit, so this value is constant for a run's duration and
#   differs only when the operator has committed since — the staleness signal.
current_base_revision() {
  local vcs="$1"
  if [ "$vcs" = "jj" ] && command -v jj >/dev/null 2>&1; then
    jj log -r @ --no-graph --no-pager -T change_id 2>/dev/null || true
  elif [ "$vcs" = "git" ]; then
    git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || true
  fi
}

# ── clear_run_manifest ───────────────────────────────────────────────────────
#   Remove the run manifest + run-id sidecar. Called on every non-aborting exit
#   (full success and the no-pending early-exit) so a leftover manifest from a
#   prior failed run never survives a run that ends without partial state.
clear_run_manifest() {
  rm -f "$RUN_PATHS_FILE" "$RUN_ID_FILE"
}

# ── 2. Pre-flight: clean working tree check ──────────────────────────────────
# RESUME is set to 1 only by the guarded-resume branch (Phase 3). Initialise it
# unconditionally here, not inside the FORCE-guarded block: the FORCE path skips
# that whole block, so an in-block default would leave RESUME unset and the
# fresh-run guard would fail under `set -u`. A FORCE run stays RESUME=0 and
# correctly mints a fresh run-id + truncates (FORCE is a brand-new run).
RESUME=0
# VCS detection is hoisted out of the FORCE-guarded block so `vcs` is computed
# unconditionally (cheap, side-effect-free) and visible to the FORCE path, the
# non-FORCE path, and the apply-loop manifest helpers.
vcs=""
if [ -d "$PROJECT_ROOT/.jj" ]; then
  vcs="jj"
elif [ -d "$PROJECT_ROOT/.git" ]; then
  vcs="git"
fi

if [ -z "${ACCELERATOR_MIGRATE_FORCE:-}" ]; then
  dirty=$(enumerate_scoped_dirty "$vcs")

  if [ -n "$dirty" ]; then
    # Detect any in-flight interactive session logs among the dirty paths
    # and emit a distinct, named message that names the resume command and
    # the explicit discard command. Prevents jj-abandon-in-confusion.
    dirty_session_logs=$(printf '%s\n' "$dirty" |
      grep -E '\.accelerator/state/migrations-[0-9a-z-]+-session\.jsonl' |
      sed 's/^[[:space:]]*//; s/^[A-Z?][[:space:]]*//' ||
      true)
    if [ -n "$dirty_session_logs" ]; then
      echo "Found in-flight interactive migration session(s):" >&2
      while IFS= read -r path; do
        [ -z "$path" ] && continue
        # Resolve the absolute path so the user can copy/paste rm commands.
        abs="$path"
        case "$abs" in /*) ;; *) abs="$PROJECT_ROOT/$path" ;; esac
        decision_count=0
        if [ -f "$abs" ]; then
          decision_count=$(wc -l <"$abs" 2>/dev/null | tr -d ' ' || echo 0)
        fi
        echo "  $path  ($decision_count decisions recorded)" >&2
      done <<<"$dirty_session_logs"
      echo "" >&2
      echo "To resume: re-run /accelerator:migrate (the session log is read on entry;" >&2
      echo "you will be prompted only for un-decided transformations)." >&2
      while IFS= read -r path; do
        [ -z "$path" ] && continue
        abs="$path"
        case "$abs" in /*) ;; *) abs="$PROJECT_ROOT/$path" ;; esac
        decision_count=0
        if [ -f "$abs" ]; then
          decision_count=$(wc -l <"$abs" 2>/dev/null | tr -d ' ' || echo 0)
        fi
        echo "To discard: rm $path  (loses $decision_count decisions)" >&2
      done <<<"$dirty_session_logs"
      echo "" >&2
      # Pick the right status command for the detected VCS. git is the
      # fallback when no VCS is detected (or the binary is missing) — it
      # is the more widely-installed of the two.
      case "$vcs" in
        jj) status_cmd='jj status' ;;
        *) status_cmd='git status' ;;
      esac
      echo "If the above does not match what you expected, run \`$status_cmd\`" >&2
      echo "to see all uncommitted changes before proceeding." >&2
      exit 1
    fi
    echo "Error: dirty working tree — uncommitted changes detected in meta/," \
      ".claude/accelerator*.md, or .accelerator/." >&2
    echo "Commit or discard those changes first, or set" \
      "ACCELERATOR_MIGRATE_FORCE=1 to skip this check." >&2
    exit 1
  fi
fi

# ── 3. Read state files ──────────────────────────────────────────────────────
applied_ids=()
if [ -f "$STATE_FILE" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] && applied_ids+=("$line")
  done <"$STATE_FILE"
fi

skipped_ids=()
if [ -f "$SKIP_FILE" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] && skipped_ids+=("$line")
  done <"$SKIP_FILE"
fi

# ── 4. Glob bundled migrations ───────────────────────────────────────────────
MIGRATIONS_DIR="${ACCELERATOR_MIGRATIONS_DIR:-$PLUGIN_ROOT/skills/config/migrate/migrations}"
migration_files=()
while IFS= read -r -d '' f; do
  migration_files+=("$f")
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 \
  2>/dev/null | sort -z) || true

# ── 5. Warn about unknown applied or skipped IDs ─────────────────────────────
known_ids=()
for f in "${migration_files[@]+"${migration_files[@]}"}"; do
  known_ids+=("$(basename "$f" .sh)")
done

for applied_id in "${applied_ids[@]+"${applied_ids[@]}"}"; do
  found=0
  for known_id in "${known_ids[@]+"${known_ids[@]}"}"; do
    [ "$applied_id" = "$known_id" ] && found=1 && break
  done
  if [ "$found" -eq 0 ]; then
    echo "[warning] migrations-applied references unknown migration" \
      "$applied_id — preserved on rewrite" >&2
  fi
done

for skipped_id in "${skipped_ids[@]+"${skipped_ids[@]}"}"; do
  found=0
  for known_id in "${known_ids[@]+"${known_ids[@]}"}"; do
    [ "$skipped_id" = "$known_id" ] && found=1 && break
  done
  if [ "$found" -eq 0 ]; then
    echo "[warning] migrations-skipped references unknown migration" \
      "$skipped_id — preserved on rewrite" >&2
  fi
done

# Cross-state inconsistency: ID in both applied and skipped
for applied_id in "${applied_ids[@]+"${applied_ids[@]}"}"; do
  for skipped_id in "${skipped_ids[@]+"${skipped_ids[@]}"}"; do
    if [ "$applied_id" = "$skipped_id" ]; then
      echo "[warning] migration $applied_id appears in BOTH .migrations-applied" \
        "and .migrations-skipped — applied takes precedence" >&2
    fi
  done
done

# ── 6. Compute pending list ──────────────────────────────────────────────────
pending_files=()
for f in "${migration_files[@]+"${migration_files[@]}"}"; do
  id="$(basename "$f" .sh)"
  is_applied=0
  for applied_id in "${applied_ids[@]+"${applied_ids[@]}"}"; do
    [ "$applied_id" = "$id" ] && is_applied=1 && break
  done
  is_skipped=0
  for skipped_id in "${skipped_ids[@]+"${skipped_ids[@]}"}"; do
    [ "$skipped_id" = "$id" ] && is_skipped=1 && break
  done
  if [ "$is_applied" -eq 0 ] && [ "$is_skipped" -eq 0 ]; then
    pending_files+=("$f")
  fi
done

# ── 7. Print preview ─────────────────────────────────────────────────────────
if [ ${#pending_files[@]} -eq 0 ]; then
  echo "No pending migrations."
  if [ "${#skipped_ids[@]}" -gt 0 ]; then
    echo "Skipped: $(printf '%s ' "${skipped_ids[@]}")"
  fi
  # A "nothing to do" invocation ends without partial state — clear any stale
  # manifest a prior failed run may have left behind.
  clear_run_manifest
  exit 0
fi

# Pre-run banner: surface the destructive-write expectation before any
# migration runs, so users facing future destructive migrations see the
# warning consistently. Pre-flight already enforces the clean-tree rule.
echo "About to apply ${#pending_files[@]} migration(s):"
for f in "${pending_files[@]}"; do
  id="$(basename "$f" .sh)"
  description=$(grep '^# DESCRIPTION:' "$f" | head -1 |
    sed 's/^# DESCRIPTION:[[:space:]]*//')
  echo "  $id — $description"
  echo "    To skip: bash $0 --skip $id"
done
echo ""
echo "Migrations rewrite files and may make repo-wide changes; commit"
echo "your working tree before running so VCS revert is available as"
echo "rollback. The pre-flight will refuse to run on a dirty tree"
echo "unless ACCELERATOR_MIGRATE_FORCE=1 is set."
echo ""

# Source the interactive library. Both paths run on bash 3.2+.
# shellcheck source=interactive-lib.sh
source "$RUNNER_SCRIPT_DIR/interactive-lib.sh"

# ── Establish run identity + baseline before the apply loop ──────────────────
# On a fresh run (RESUME=0 — Phase 3 sets RESUME=1 for a guarded resume) mint a
# new run-id (the current base revision) and truncate the manifest, so a prior
# failed run's manifest cannot survive a clean start. An empty base revision
# (unborn git HEAD, or absent/failed VCS) writes the sidecar empty, which
# disables guarded resume for the run (fail-closed, by design).
if [ "$RESUME" -ne 1 ]; then
  mkdir -p "$(dirname "$RUN_ID_FILE")"
  current_base_revision "$vcs" | atomic_write "$RUN_ID_FILE"
  : | atomic_write "$RUN_PATHS_FILE" # truncate to empty
fi
# The baseline is the dirty set to EXCLUDE from this run's manifest. Fresh clean
# run: empty (the pre-flight guaranteed a clean tree). FORCE run: the foreign
# dirt the run must not claim ownership of. Guarded resume: empty — re-assert
# ownership of every still-dirty path on each step (self-healing across
# successive failures, rather than depending on the prior append surviving).
BASELINE_FILE=$(mktemp) ||
  {
    echo "migrate: cannot create baseline temp file" >&2
    exit 1
  }
if [ "$RESUME" -eq 1 ]; then
  : >"$BASELINE_FILE"
else
  enumerate_scoped_dirty "$vcs" >"$BASELINE_FILE" 2>/dev/null || true
fi

# ── 8. Apply each pending migration ─────────────────────────────────────────
applied_count=0
for f in "${pending_files[@]}"; do
  id="$(basename "$f" .sh)"
  echo "[${id}] running" >&2
  export PROJECT_ROOT

  # Dispatch on the # INTERACTIVE: yes header marker. The interactive path is
  # deliberately NOT recorded into the mechanical manifest — interactive
  # partial-resume is governed by the 0069 session-log scaffold (Phase 4 owns
  # the session-log axis), so a manifest entry for it would blur the two axes.
  if is_interactive_migration "$f"; then
    INTERACTIVE_APPLIED=0
    if ! run_interactive_migration "$f" "$id"; then
      rm -f "$BASELINE_FILE"
      echo "[${id}] failed" >&2
      exit 1
    fi
    if [ "${INTERACTIVE_APPLIED:-0}" -eq 1 ]; then
      applied_count=$((applied_count + 1))
    fi
    continue
  fi

  STDOUT_FILE=$(mktemp)
  if ! PROJECT_ROOT="$PROJECT_ROOT" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" ACCELERATOR_MIGRATION_MODE=1 bash "$f" >"$STDOUT_FILE" 2>&1; then
    manifest_record_delta "$vcs" "$BASELINE_FILE" # capture partial writes (AC1)
    cat "$STDOUT_FILE" >&2
    rm -f "$STDOUT_FILE" "$BASELINE_FILE"
    echo "[${id}] failed" >&2
    exit 1
  fi
  manifest_record_delta "$vcs" "$BASELINE_FILE" # capture successful writes
  # Inspect stdout for the no_op_pending sentinel
  NO_OP_PENDING=0
  if grep -qx 'MIGRATION_RESULT: no_op_pending' "$STDOUT_FILE"; then
    NO_OP_PENDING=1
  fi
  # Strip the sentinel from the user-visible output
  grep -v -x 'MIGRATION_RESULT: no_op_pending' "$STDOUT_FILE" >&2 || true
  rm -f "$STDOUT_FILE"

  if [ "$NO_OP_PENDING" -eq 1 ]; then
    echo "[${id}] no-op (stays pending)" >&2
    continue
  fi

  mkdir -p "$(dirname "$STATE_FILE")"
  atomic_append_unique "$STATE_FILE" "$id"
  echo "[${id}] applied" >&2
  applied_count=$((applied_count + 1))
done

# Full run completed without aborting — no partial state to resume over, so the
# manifest + run-id sidecar are deleted. (Guarded resume is scoped strictly to
# partial-failure re-runs.)
clear_run_manifest
rm -f "$BASELINE_FILE"

# ── 9. Summary ───────────────────────────────────────────────────────────────
echo ""
SUMMARY="applied: $applied_count"
if [ "${#skipped_ids[@]}" -gt 0 ]; then
  SUMMARY="$SUMMARY; skipped: $(printf '%s ' "${skipped_ids[@]}" | sed 's/ $//')"
fi
PENDING_REMAINING=$((${#pending_files[@]} - applied_count))
if [ "$PENDING_REMAINING" -gt 0 ]; then
  SUMMARY="$SUMMARY; pending (no-op): $PENDING_REMAINING"
fi
echo "Migration complete. $SUMMARY."
