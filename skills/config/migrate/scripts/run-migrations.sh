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

# ── --skip / --unskip / --decisions-file / --list / --help flags ─────────────
# A while/shift loop (not a single-leading-flag if/case): --list and
# --decisions-file fall through to a run, so more than one flag may precede it,
# and an unrecognised flag/positional must be REJECTED (was silently ignored).
LIST_MODE=""
while [ $# -gt 0 ]; do
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
    --unapply)
      if [ $# -lt 2 ]; then
        echo "Usage: run-migrations.sh --unapply <migration-id>" >&2
        exit 1
      fi
      atomic_remove_line "$STATE_FILE" "$2"
      echo "Unapplied migration: $2"
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
    --list)
      LIST_MODE=1
      shift
      ;;
    --help | -h)
      # Route the explicit help path to STDOUT (GNU/POSIX convention) so an
      # agent's `--help | grep` finds the promoted env var; usage-on-error
      # messages stay on stderr.
      cat <<'EOF'
Usage: run-migrations.sh [FLAG]
  --skip <id>             Mark migration <id> skipped; do not run it.
  --unskip <id>           Remove migration <id> from the skip list.
  --unapply <id>          Remove migration <id> from the applied ledger so a
                          half-applied migration can be re-run.
  --list                  Dry-emit pending interactive transformations, one
                          tab-delimited line each, then exit (no mutation):
                            <pos>\t<key>\t<proposed>\t<path>:<field>
                          With >1 pending interactive migration, output is
                          segmented by a `# migration <id>` header and <pos>
                          restarts at 1 per migration.
  --decisions-file <path> Scripted decisions for interactive migrations, one
                          per line: accept | skip | edit <value>. The resume
                          path the no-input stall points at.

Environment:
  ACCELERATOR_MIGRATE_DECISIONS_FILE=<path>
                          Same as --decisions-file: newline-delimited verbs
                          (accept | skip | edit <value>) matched to pending
                          transformations by emission order. Validated up front
                          (dry-apply); a rejected edit, unknown verb, or wrong
                          count fails closed naming the position, corpus
                          unmutated.
EOF
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      echo "Run with --help for usage." >&2
      exit 1
      ;;
  esac
done

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

# ── refuse_dirty_tree ────────────────────────────────────────────────────────
#   Emit the canonical dirty-tree refusal + FORCE hint and exit 1. The single
#   definition of this message: both the guarded-resume `else` arm and the
#   original refusal site call it, so the text cannot drift between them.
refuse_dirty_tree() {
  echo "Error: dirty working tree — uncommitted changes detected in meta/," \
    ".claude/accelerator*.md, or .accelerator/." >&2
  echo "Commit or discard those changes first, or set" \
    "ACCELERATOR_MIGRATE_FORCE=1 to skip this check." >&2
  exit 1
}

# ── is_session_log <repo-relative-path> ──────────────────────────────────────
#   True ONLY for the canonical interactive session log. Used by the detector
#   and the resume affordance (which run `wc -l` for the decision count); a
#   looser match would mislabel a stderr.log / resume-state.tmp as a decisions
#   log with a bogus count.
is_session_log() {
  case "$1" in
    .accelerator/state/migrations-[0-9a-z]*-session.jsonl) return 0 ;;
  esac
  return 1
}

# ── is_session_artifact <repo-relative-path> ─────────────────────────────────
#   True for ANY runner-managed interactive session artifact preserved across a
#   failure: the log, the stderr capture, or the resume-state tmp. Used by the
#   owned-check (a preserved stderr.log under jj must not defeat resume). Shares
#   is_session_log's migrations-<id>- id-class so the two recognisers agree on
#   what counts as this-run's. FIFOs (migrations-<id>-{r2m,m2r}.fifo) are
#   omitted: neither git nor jj tracks named pipes, so they never appear in
#   enumerate_scoped_dirty.
is_session_artifact() {
  case "$1" in
    .accelerator/state/migrations-[0-9a-z]*-session.jsonl) return 0 ;;
    .accelerator/state/migrations-[0-9a-z]*-stderr.log) return 0 ;;
    .accelerator/state/migrations-[0-9a-z]*-resume-state.tmp) return 0 ;;
  esac
  return 1
}

# ── dirty_tree_fully_owned <vcs> <dirty> ─────────────────────────────────────
#   Return 0 iff the manifest + run-id sidecar are usable, the recorded base
#   revision still equals the current one, AND every line in <dirty> is either a
#   runner-managed bookkeeping file or present in the manifest. Fail-closed: any
#   unusable manifest/sidecar, or a revision mismatch, returns 1 (→ refuse).
#   Always invoked as an `if` condition, which suspends `set -e` for the body —
#   so an internal grep no-match returns non-zero without aborting the script,
#   and each explicit `|| return 1` is an intended refusal.
dirty_tree_fully_owned() {
  local vcs="$1" dirty="$2" path recorded current
  # Usability gate (mirror launcher-helpers.sh identity gate).
  [ -r "$RUN_ID_FILE" ] && [ -s "$RUN_ID_FILE" ] || return 1 # run-id non-empty
  # The manifest must EXIST but may be EMPTY: an in-flight interactive interrupt
  # that ran before any mechanical delta leaves an empty manifest, yet its
  # session log is owned-by-pattern (Phase 4). Requiring non-empty (`-s`) here
  # would make that resume unreachable. The per-path loop is the sole ownership
  # authority — an empty manifest + a dirty mechanical path still refuses.
  [ -r "$RUN_PATHS_FILE" ] || return 1
  # Staleness: the recorded base revision must equal the current one. They
  # differ only when the operator has committed since the failed run (the
  # working copy has moved on) — the "different run" case AC4 requires we refuse.
  recorded=$(head -n1 "$RUN_ID_FILE")
  current=$(current_base_revision "$vcs")
  [ -n "$current" ] && [ "$recorded" = "$current" ] || return 1
  # Runner-managed bookkeeping files are implicitly owned; derive their
  # repo-relative forms from the path variables (no hard-coded literals).
  local rel_applied="${STATE_FILE#"$PROJECT_ROOT/"}"
  local rel_skipped="${SKIP_FILE#"$PROJECT_ROOT/"}"
  local rel_paths="${RUN_PATHS_FILE#"$PROJECT_ROOT/"}"
  local rel_id="${RUN_ID_FILE#"$PROJECT_ROOT/"}"
  while IFS= read -r path; do
    [ -z "$path" ] && continue
    case "$path" in
      "$rel_applied" | "$rel_skipped" | "$rel_paths" | "$rel_id") continue ;;
    esac
    # A current-run interactive session artifact is owned by pattern (gated by
    # the base-revision check above, so a stale-run artifact is NOT owned). The
    # session log stays out of the mechanical manifest; this is where the two
    # resume axes share one ownership decision.
    is_session_artifact "$path" && continue
    grep -Fxq -- "$path" "$RUN_PATHS_FILE" || return 1
  done <<<"$dirty"
  return 0
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

# --list is a dry, read-only emit: it skips the WHOLE pre-flight (the manifest /
# run-id setup, the RESUME state, the guarded-resume branch, and the in-flight
# session-log steer 0119 added inside it). Because --list excludes already-
# decided keys via the resume filter, it notes any in-flight session log on
# stderr (below, in the --list branch) rather than re-deriving it here.
if [ -z "$LIST_MODE" ] && [ -z "${ACCELERATOR_MIGRATE_FORCE:-}" ]; then
  dirty=$(enumerate_scoped_dirty "$vcs")

  if [ -n "$dirty" ]; then
    # Owned-check FIRST: when every dirty path is owned by this run (mechanical
    # manifest paths AND current-run interactive session artifacts), resume
    # WITHOUT ACCELERATOR_MIGRATE_FORCE=1. 0069's replay-on-entry resumes any
    # in-flight interactive migration, the applied ledger skips completed ones,
    # and the mechanical tail re-runs. A NOT-owned tree (foreign dirt, or a
    # stale/foreign session log) falls to today's behaviour: steer in-flight
    # session logs to the structured resume/discard scaffold, else refuse.
    if dirty_tree_fully_owned "$vcs" "$dirty"; then
      RESUME=1
      echo "Resuming over this run's own partial migration output:" >&2
      while IFS= read -r path; do
        [ -z "$path" ] && continue
        if is_session_log "$path"; then
          # Resolve the absolute path so the decision count + rm command are
          # accurate and copy/pasteable.
          abs="$path"
          case "$abs" in /*) ;; *) abs="$PROJECT_ROOT/$path" ;; esac
          decision_count=0
          if [ -f "$abs" ]; then
            decision_count=$(wc -l <"$abs" 2>/dev/null | tr -d ' ' || echo 0)
          fi
          echo "  $path" >&2
          echo "    interactive migration — resuming: replays $decision_count" \
            "decided transformation(s) and re-prompts only undecided ones" >&2
          echo "    (with no decisions channel it re-stalls — resume" \
            "non-interactively via --decisions-file)." >&2
          echo "    To discard instead: rm $abs  (loses $decision_count decisions)" >&2
        else
          echo "  $path" >&2
        fi
      done <<<"$dirty"
      # fall through past the refusal into "Read state files"
    else
      # NOT fully owned. Detect in-flight interactive session logs among the
      # dirty paths and emit the structured resume/discard scaffold (prevents
      # jj-abandon-in-confusion); reuse is_session_log so the detector and the
      # owned-check agree on what counts as a session log.
      dirty_session_logs=""
      while IFS= read -r path; do
        [ -z "$path" ] && continue
        if is_session_log "$path"; then
          dirty_session_logs="${dirty_session_logs}${path}"$'\n'
        fi
      done <<<"$dirty"
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
      refuse_dirty_tree
    fi
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

# Source the interactive library here (sourcing only defines functions, so it is
# safe to do before the preview banner / early exits). The --list and dry-apply
# surfaces below are defined here. Both paths run on bash 3.2+.
# shellcheck source=interactive-lib.sh
source "$RUNNER_SCRIPT_DIR/interactive-lib.sh"

# ── --list: dry-emit pending interactive transformations ─────────────────────
# Read-only enumeration, before the "No pending migrations." early exit so an
# empty corpus prints the empty sentinel rather than the preview banner.
if [ -n "$LIST_MODE" ]; then
  # Identify pending interactive migrations up front so we know whether to
  # segment. Mechanical migrations are NOT run in list mode (they would mutate).
  int_files=()
  for f in "${pending_files[@]+"${pending_files[@]}"}"; do
    is_interactive_migration "$f" && int_files+=("$f")
  done
  multi=0
  [ "${#int_files[@]}" -gt 1 ] && multi=1
  if [ "$multi" -eq 1 ]; then
    # stderr only (stdout stays parseable data); fires only in the multi case,
    # so the single-migration stderr-clean guarantee is unaffected.
    echo "Note: ${#int_files[@]} interactive migrations pending; resume one at a" \
      "time with --decisions-file per '# migration <id>' section — a single" \
      "multi-migration decisions file is not yet supported." >&2
  fi
  # Because --list excludes already-decided keys via the resume filter, note any
  # in-flight session on stderr (never on the parseable stdout stream) so the
  # read-only path is not silent about partial state.
  for f in "${int_files[@]+"${int_files[@]}"}"; do
    id="$(basename "$f" .sh)"
    sess="$PROJECT_ROOT/.accelerator/state/migrations-${id}-session.jsonl"
    if [ -s "$sess" ]; then
      echo "Note: migration $id has an in-flight session log; --list shows only" \
        "the remaining (undecided) transformations. Re-run" \
        "/accelerator:migrate to resume, or rm $sess to discard." >&2
    fi
  done
  emitted=0
  for f in "${int_files[@]+"${int_files[@]}"}"; do
    id="$(basename "$f" .sh)"
    enumerate_interactive_transformations "$f" "$id" || exit 1
    [ "${#LIST_ENTRIES[@]}" -eq 0 ] && continue
    # Segment ONLY when >1 pending, so the single-migration case stays bare
    # canonical lines; positions RESTART at 1 per migration to match the
    # per-migration decisions file.
    [ "$multi" -eq 1 ] && printf '# migration %s\n' "$id"
    pos=0
    for entry in "${LIST_ENTRIES[@]}"; do
      pos=$((pos + 1))
      emitted=$((emitted + 1))
      # entry = key<TAB>path<TAB>anchor<TAB>proposed (fields already guarded
      # against embedded TAB/newline at enumerate time, so the split is safe).
      IFS=$'\t' read -r k p a v <<<"$entry"
      printf '%s\t%s\t%s\t%s:%s\n' "$pos" "$k" "$v" "$p" "$a"
    done
  done
  [ "$emitted" -eq 0 ] && echo "no pending transformations"
  exit 0
fi

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

# ── Fail-closed validation of the decisions file (no-mutation dry-apply) ─────
# When a decisions file is supplied, validate it against each pending
# interactive migration BEFORE the live apply loop. A non-zero return / FAIL
# aborts here, so a malformed decisions file (rejected edit, unknown verb, too
# few/too many) never falls through to a mutating run — the corpus stays
# unmutated. read_decision's silent unknown-verb pass-through is left intact but
# is now unreachable for the decisions-file path (dry-apply rejects first).
if [ -n "$ACCELERATOR_MIGRATE_DECISIONS_FILE" ]; then
  for f in "${pending_files[@]+"${pending_files[@]}"}"; do
    id="$(basename "$f" .sh)"
    if is_interactive_migration "$f"; then
      dry_apply_interactive_migration "$f" "$id" || exit 1
    fi
  done
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
