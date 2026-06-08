#!/usr/bin/env bash
# DESCRIPTION: Unify the meta/ corpus to the ADR-0033/0034 schema — base fields, identity, provenance, status reconciliation, fence-less backfill, and (interactive) body-section typed linkage.
# INTERACTIVE: yes
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"

source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/scripts/log-common.sh"
source "$PLUGIN_ROOT/scripts/interactive-harness.sh"

# Byte-stable text processing across environments (parity with the launcher's
# locale safety): the cmp -s idempotency gate and [:alnum:]/[:space:] classes
# must interpret bytes identically everywhere.
export LC_ALL=C LANG=C

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi
if PROJECT_ROOT_CANON="$(cd "$PROJECT_ROOT" 2>/dev/null && pwd -P)"; then
  PROJECT_ROOT="$PROJECT_ROOT_CANON"
fi

SCHEMA_TSV="$PLUGIN_ROOT/scripts/templates-schema.tsv"
STATUS_MAP_TSV="$PLUGIN_ROOT/scripts/status-legacy-map.tsv"
FRAG_AWK="$PLUGIN_ROOT/skills/config/migrate/scripts/frontmatter-frag.awk"
BODY_AWK="$PLUGIN_ROOT/skills/config/migrate/scripts/0007-frontmatter-rewrite.awk"

REFUSE_COUNT=0
MALFORMED_COUNT=0

# ── Path safety (reused shape from 0006) ─────────────────────────────────────
assert_safe_relpath() {
  local rel="$1" label="$2"
  case "$rel" in
    '' | . | .. | / | /* | */.. | ../* | */../* | */./*)
      log_warn "0007: refusing dangerous $label value: $rel" >&2
      return 1
      ;;
  esac
  return 0
}

# ── Schema lookups from templates-schema.tsv ─────────────────────────────────
# Echo a tab-separated row for a doc-type, or empty.
schema_row() {
  awk -F'\t' -v t="$1" 'NR > 1 && $2 == t { print; exit }' "$SCHEMA_TSV"
}
anchored_for_type() { schema_row "$1" | cut -f3; }
extras_for_type() { schema_row "$1" | cut -f5; }
status_vocab_for_type() { schema_row "$1" | cut -f5- | cut -f1; } # 5th col
# Status vocab is column 5; re-extract cleanly.
status_vocab_of() { schema_row "$1" | awk -F'\t' '{print $5}'; }

own_id_key_for_type() {
  case "$1" in
    work-item) echo work_item_id ;;
    adr) echo adr_id ;;
    *) echo "" ;;
  esac
}

# Space-joined "legacy=canonical" status pairs for a type (for awk -v).
status_map_for_type() {
  awk -F'\t' -v t="$1" 'NR > 1 && $1 == t { printf "%s=%s ", $2, $3 }' "$STATUS_MAP_TSV"
}

# ── Location → doc-type (exhaustive; reviews by subdir) ──────────────────────
infer_type_from_path() {
  case "$1" in
    */work/*) echo work-item ;;
    */plans/*) echo plan ;;
    */decisions/*) echo adr ;;
    */research/codebase/*) echo codebase-research ;;
    */research/issues/*) echo issue-research ;;
    */research/design-gaps/*) echo design-gap ;;
    */research/design-inventories/*) echo design-inventory ;;
    */reviews/plans/*) echo plan-review ;;
    */reviews/work/*) echo work-item-review ;;
    */reviews/prs/*) echo pr-review ;;
    */validations/*) echo plan-validation ;;
    */notes/*) echo note ;;
    *) echo "" ;;
  esac
}

out_of_scope() {
  case "$1" in
    */specs/* | */talks/* | */global/*) return 0 ;;
    *) return 1 ;;
  esac
}

stem_of() { local b; b="$(basename "$1")"; printf '%s' "${b%.md}"; }

# Identity stem for a file, type-aware. Nested-manifest types (design-inventory,
# whose manifest is always `inventory.md` under a dated directory) take the
# PARENT DIRECTORY name as their stem — mirroring the indexer's slug source —
# so distinct inventories don't all collapse to the id "inventory".
derive_stem() { # $1=file $2=type
  case "$2" in
    design-inventory) basename "$(dirname "$1")" ;;
    *) stem_of "$1" ;;
  esac
}

# Strict leading-fence test (no trailing whitespace tolerance — by design).
has_strict_fence() {
  awk 'NR == 1 { exit ($0 == "---" ? 0 : 1) } END { if (NR == 0) exit 1 }' "$1"
}

# Existing frontmatter value of a key (first fence region), or empty.
fm_get() {
  awk -v k="$1" '
    NR == 1 && $0 != "---" { exit }
    $0 == "---" { c++; if (c == 2) exit; next }
    c == 1 && index($0, k ":") == 1 {
      v = $0; sub("^" k ":[ \t]*", "", v); sub(/[ \t]+$/, "", v); print v; exit
    }' "$2"
}

fm_inner() {
  local v="$1"
  case "$v" in
    '"'*'"') v="${v#\"}"; v="${v%\"}" ;;
    "'"*"'") v="${v#\'}"; v="${v%\'}" ;;
  esac
  printf '%s' "$v"
}

# ── VCS author resolution (jj then git; LANG=C; Unknown on absence/failure) ──
# Sets RESOLVED_AUTHOR; emits a counted DIVERGE diagnostic on lookup *failure*
# (distinct from a genuine empty history → plain Unknown).
resolve_author() {
  local relpath="$1" author=""
  if [ -d "$PROJECT_ROOT/.jj" ] && command -v jj >/dev/null 2>&1; then
    author="$(cd "$PROJECT_ROOT" && LANG=C jj --no-pager log --no-graph \
      -r "latest(::@ & files(\"$relpath\"))" -T 'author.name()' 2>/dev/null | head -1 || true)"
  elif [ -d "$PROJECT_ROOT/.git" ] && command -v git >/dev/null 2>&1; then
    author="$(cd "$PROJECT_ROOT" && LANG=C git log --format='%an' -1 -- "$relpath" 2>/dev/null | head -1 || true)"
  fi
  if [ -n "$author" ]; then
    RESOLVED_AUTHOR="$author"
  else
    RESOLVED_AUTHOR="Unknown"
  fi
}

# Sets RESOLVED_REVISION to the short commit/change id that last touched the
# file, or "" when no VCS / no history is available.
resolve_revision() {
  local relpath="$1" rev=""
  if [ -d "$PROJECT_ROOT/.jj" ] && command -v jj >/dev/null 2>&1; then
    rev="$(cd "$PROJECT_ROOT" && LANG=C jj --no-pager log --no-graph \
      -r "latest(::@ & files(\"$relpath\"))" -T 'commit_id.short()' 2>/dev/null | head -1 || true)"
  elif [ -d "$PROJECT_ROOT/.git" ] && command -v git >/dev/null 2>&1; then
    rev="$(cd "$PROJECT_ROOT" && LANG=C git log --format='%h' -1 -- "$relpath" 2>/dev/null | head -1 || true)"
  fi
  RESOLVED_REVISION="$rev"
}

# ── Corpus enumeration ───────────────────────────────────────────────────────
META_REL="meta"
META_ABS="$PROJECT_ROOT/$META_REL"

corpus_files() {
  [ -d "$META_ABS" ] || return 0
  find "$META_ABS" -type f -name '*.md' -print0
}

# ── Step 0: read-only precondition pre-pass (zero mutations) ─────────────────
precondition_prepass() {
  local f type id own seen_ids="" dup=0 refused=0
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_strict_fence "$f" || continue
    type="$(fm_inner "$(fm_get type "$f")")"
    [ -n "$type" ] || type="$(infer_type_from_path "$f")"
    if [ "$type" = "work-item" ]; then
      if [ -z "$(fm_get kind "$f")" ]; then
        log_warn "0007-REFUSE: $f — work-item missing kind: (run migration 0005 first)" >&2
        refused=1
      fi
      own="$(fm_inner "$(fm_get work_item_id "$f")")"
      local fid; fid="$(printf '%s' "$(stem_of "$f")" | grep -oE '^[0-9]+' || true)"
      if [ -n "$own" ] && [ -n "$fid" ] && [ "$own" != "$fid" ]; then
        log_warn "0007-REFUSE: $f — own work_item_id '$own' != filename id '$fid'" >&2
        refused=1
      fi
    else
      # Foreign work_item_id must already be quoted (0006 guarantee).
      local raw; raw="$(fm_get work_item_id "$f")"
      if [ -n "$raw" ]; then
        case "$raw" in
          '"'*'"' | "'"*"'") : ;;
          *)
            log_warn "0007-REFUSE: $f — foreign work_item_id unquoted: $raw (run migration 0006 first)" >&2
            refused=1
            ;;
        esac
      fi
    fi
    # Post-rewrite id (own-id key → id, else existing id, else stem). The
    # duplicate check is keyed on the TYPED reference (type:id), not the bare
    # id, because the reference namespace is per-type: a plan and its driving
    # research legitimately share a YYYY-MM-DD-NNNN-slug stem (→ same bare id)
    # but resolve to distinct typed refs (plan:… vs codebase-research:…). A true
    # collision is two artifacts of the SAME type with the same id (e.g. the
    # 0032 work-item carrying work_item_id "0031", colliding with 0031).
    case "$type" in
      work-item | adr) id="$(fm_inner "$(fm_get "$(own_id_key_for_type "$type")" "$f")")" ;;
      *) id="" ;;
    esac
    [ -n "$id" ] || id="$(fm_inner "$(fm_get id "$f")")"
    [ -n "$id" ] || id="$(derive_stem "$f" "$type")"
    local typed_id="${type}:${id}"
    case "$seen_ids" in
      *"|${typed_id}|"*) log_warn "0007-REFUSE: $f — duplicate post-rewrite id '$typed_id'" >&2; refused=1 ;;
    esac
    seen_ids="${seen_ids}|${typed_id}|"
  done < <(corpus_files)
  [ "$refused" -eq 0 ] || return 1
  return 0
}

# ── Step 1: backfill fence-less files + the partial-fence note ───────────────
backfill_file() {
  local f="$1" type stem h1 title date author rel iso tmp
  type="$(infer_type_from_path "$f")"
  stem="$(derive_stem "$f" "$type")"
  rel="${f#"$PROJECT_ROOT"/}"
  # Title from first H1, else humanised stem.
  h1="$(awk '/^# / { sub(/^# /, ""); print; exit }' "$f" || true)"
  title="${h1:-$stem}"
  # Date from a leading YYYY-MM-DD filename prefix, else Unknown→skip date seed.
  date="$(printf '%s' "$stem" | grep -oE '^[0-9]{4}-[0-9]{2}-[0-9]{2}' || true)"
  if [ -n "$date" ]; then iso="${date}T00:00:00+00:00"; else iso=""; fi
  resolve_author "$rel"
  author="$RESOLVED_AUTHOR"
  local revision=""
  if [ "$(anchored_for_type "$type")" = "yes" ]; then
    resolve_revision "$rel"
    revision="$RESOLVED_REVISION"
    if [ -z "$revision" ]; then
      log_warn "0007-DIVERGE[author-lookup-failed]: $rel — no VCS revision for anchored backfill" >&2
    fi
  fi

  tmp="$(mktemp)"
  {
    printf -- '---\n'
    printf 'type: %s\n' "$type"
    printf 'id: "%s"\n' "$stem"
    printf 'title: "%s"\n' "$title"
    [ -n "$iso" ] && printf 'date: "%s"\n' "$iso"
    printf 'author: %s\n' "$author"
    if [ "$type" = "note" ]; then
      printf 'producer: create-note\n'
      printf 'status: captured\n'
      printf 'topic: "%s"\n' "$title"
    fi
    printf 'tags: []\n'
    if [ "$(anchored_for_type "$type")" = "yes" ]; then
      [ -n "$revision" ] && printf 'revision: "%s"\n' "$revision"
      printf 'repository: "%s"\n' "$(basename "$PROJECT_ROOT")"
    fi
    [ -n "$iso" ] && printf 'last_updated: "%s"\n' "$iso"
    printf 'last_updated_by: %s\n' "$author"
    printf 'schema_version: 1\n'
    printf -- '---\n\n'
    # Original body. If the file had a partial fence (note with trailing-ws
    # fence), strip its existing pseudo-frontmatter block first.
    if awk 'NR==1 && /^---[[:space:]]*$/{found=1} END{exit(found?0:1)}' "$f"; then
      awk 'BEGIN{c=0} /^---[[:space:]]*$/{c++; next} c>=2{print} c<2{next}' "$f"
    else
      cat "$f"
    fi
  } >"$tmp"
  atomic_write "$f" <"$tmp"
  rm -f "$tmp"
}

run_backfill() {
  # Backfill any file lacking a STRICT leading fence. The strict test (== "---",
  # no trailing whitespace) is what catches the partial-fence note whose `---`
  # carries trailing whitespace — a loose detector would skip it. The backfill
  # SHAPE (note vs plan vs other) is chosen by location inside backfill_file, so
  # notes are still backfilled to the note shape "by location". An
  # already-conforming (strict-fenced) note is left for the rewrite pass, so
  # re-runs do not re-resolve VCS and stay idempotent.
  local f
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_strict_fence "$f" || backfill_file "$f"
  done < <(corpus_files)
}

# ── Step 2: deterministic awk rewrite over the now-fenced corpus ─────────────
rewrite_file() {
  local f="$1" type anchored own extras vocab smap stem idstem repo
  type="$(fm_inner "$(fm_get type "$f")")"
  [ -n "$type" ] || type="$(infer_type_from_path "$f")"
  [ -n "$type" ] || return 0
  anchored=0; [ "$(anchored_for_type "$type")" = "yes" ] && anchored=1
  own="$(own_id_key_for_type "$type")"
  vocab="$(status_vocab_of "$type")"
  smap="$(status_map_for_type "$type")"
  stem="$(derive_stem "$f" "$type")"
  case "$type" in
    work-item) idstem="$(printf '%s' "$stem" | grep -oE '^[0-9]+' || echo "$stem")" ;;
    *) idstem="$stem" ;;
  esac
  repo="$(basename "$PROJECT_ROOT")"

  local has_type=0 has_id=0 has_tags=0 has_schema=0 has_lu=0 has_lub=0
  local has_date=0 has_author=0 has_producer=0 has_revision=0 has_repository=0 has_title=0
  [ -n "$(fm_get type "$f")" ] && has_type=1
  [ -n "$(fm_get id "$f")" ] && has_id=1
  [ -n "$(fm_get title "$f")" ] && has_title=1
  [ -n "$(fm_get tags "$f")" ] && has_tags=1
  [ -n "$(fm_get schema_version "$f")" ] && has_schema=1
  [ -n "$(fm_get last_updated "$f")" ] && has_lu=1
  [ -n "$(fm_get last_updated_by "$f")" ] && has_lub=1
  [ -n "$(fm_get date "$f")" ] && has_date=1
  [ -n "$(fm_get author "$f")" ] && has_author=1
  [ -n "$(fm_get producer "$f")" ] && has_producer=1
  [ -n "$(fm_get revision "$f")" ] && has_revision=1
  [ -n "$(fm_get repository "$f")" ] && has_repository=1

  # Derive the "hard" base fields a fenced file may lack — same sources as the
  # fence-less backfill: H1 → title, VCS → author/revision, filename → date.
  local rel="${f#"$PROJECT_ROOT"/}"
  local title_default="" author_default="" date_default="" revision_default=""
  if [ "$has_title" -eq 0 ]; then
    title_default="$(awk 'f && /^# /{sub(/^# /,""); print; exit} /^---[[:space:]]*$/{c++; if(c==2)f=1}' "$f" | tr -d '"')"
    [ -n "$title_default" ] || title_default="$stem"
  fi
  if [ "$has_author" -eq 0 ]; then
    resolve_author "$rel"; author_default="$RESOLVED_AUTHOR"
  fi
  if [ "$has_date" -eq 0 ]; then
    local _d; _d="$(printf '%s' "$stem" | grep -oE '^[0-9]{4}-[0-9]{2}-[0-9]{2}' || true)"
    [ -n "$_d" ] && date_default="${_d}T00:00:00+00:00"
  fi
  if [ "$anchored" -eq 1 ] && [ "$has_revision" -eq 0 ]; then
    resolve_revision "$rel"; revision_default="$RESOLVED_REVISION"
  fi

  local tmp_out tmp_err
  tmp_out="$(mktemp)"; tmp_err="$(mktemp)"
  awk -f "$FRAG_AWK" -f "$BODY_AWK" \
    -v file="$f" -v type="$type" -v anchored="$anchored" -v own_id_key="$own" \
    -v id_from_stem="$idstem" -v repo_name="$repo" \
    -v statusvocab="$vocab" -v statusmap="$smap" \
    -v title_default="$title_default" -v author_default="$author_default" \
    -v date_default="$date_default" -v revision_default="$revision_default" \
    -v has_type="$has_type" -v has_id="$has_id" -v has_title="$has_title" -v has_tags="$has_tags" \
    -v has_schema="$has_schema" -v has_lu="$has_lu" -v has_lub="$has_lub" \
    -v has_date="$has_date" -v has_author="$has_author" \
    -v has_producer="$has_producer" -v has_revision="$has_revision" \
    -v has_repository="$has_repository" \
    <"$f" >"$tmp_out" 2>"$tmp_err"

  if [ -s "$tmp_err" ]; then
    grep -c '0007-REFUSE' "$tmp_err" >/dev/null 2>&1 &&
      REFUSE_COUNT=$((REFUSE_COUNT + $(grep -c '0007-REFUSE' "$tmp_err")))
    grep -c '0007-MALFORMED' "$tmp_err" >/dev/null 2>&1 &&
      MALFORMED_COUNT=$((MALFORMED_COUNT + $(grep -c '0007-MALFORMED' "$tmp_err")))
    while IFS= read -r line; do log_warn "0007: $line" >&2; done <"$tmp_err"
  fi
  if ! cmp -s "$f" "$tmp_out"; then atomic_write "$f" <"$tmp_out"; fi
  rm -f "$tmp_out" "$tmp_err"
}

run_rewrite() {
  local f
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_strict_fence "$f" || continue   # backfill already fenced these
    rewrite_file "$f"
  done < <(corpus_files)
}

# ── Self-validation (structural, file-list mode) ─────────────────────────────
VALIDATOR="$PLUGIN_ROOT/scripts/validate-corpus-frontmatter.sh"
self_validate_structural() {
  local files=()
  local f
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    files+=("$f")
  done < <(corpus_files)
  [ "${#files[@]}" -gt 0 ] || return 0
  bash "$VALIDATOR" "${files[@]}" >&2
}
self_validate_referential() {
  bash "$VALIDATOR" "$META_ABS" >&2
}

# ── Interactive hooks: body-section typed linkage ───────────────────────────
PARSER="$PLUGIN_ROOT/scripts/linkage-parser.sh"
MERGE_AWK="$PLUGIN_ROOT/skills/config/migrate/scripts/frontmatter-merge.awk"

# Cardinality of a linkage key (single vs list), per ADR-0034.
linkage_card() {
  case "$1" in
    parent | target | source | superseded_by) echo single ;;
    *) echo list ;;
  esac
}

# A typed reference value: doc-type:id (or pr:<n>). Bare/path shapes rejected.
LINKAGE_REF_RE='^(work-item|plan|adr|pr|codebase-research|issue-research|pr-description|design-inventory|design-gap|plan-validation|plan-review|work-item-review|pr-review|note):[A-Za-z0-9-]+$'

# Run the body-section parser over every fenced in-scope artifact and emit one
# transformation per reference. resolved-band → predicate routes mechanical;
# ambiguous-band → predicate routes to the interactive prompt. The parser's
# stdout is captured here and re-emitted as TX frames; only TX lines reach
# stdout (parser diagnostics go to its stderr → the runner's stderr capture).
migration_emit_transformations() {
  local f rel recs src key target anchor band
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_strict_fence "$f" || continue
    rel="${f#"$PROJECT_ROOT"/}"
    recs="$(bash "$PARSER" "$f" 2>/dev/null || true)"
    [ -n "$recs" ] || continue
    while IFS=$'\t' read -r src key target anchor band; do
      [ -n "$key" ] && [ -n "$target" ] || continue
      harness_extras_set band "$band"
      harness_extras_set linkage_key "$key"
      harness_emit_transformation \
        key="${rel}#${anchor}" path="$rel" anchor="$anchor" \
        proposed="${key}=${target}" predicate_value="$band" \
        display="Proposed linkage: ${key}: \"${target}\"
Section anchor: ${anchor}
Band: ${band}"
    done <<<"$recs"
  done < <(corpus_files)
}

# Prompt only ambiguous-band references; resolved apply mechanically.
migration_evaluate_predicate() {
  [ "$(harness_field band)" = "ambiguous" ]
}

# Reject an edited value that is not a "<linkage-key>=<typed-ref>" pair with a
# vocabulary key and a doc-type:id target.
migration_validate_edit() {
  local key="$1" path="$2" anchor="$3" proposed="$4" user_value="$5"
  case "$user_value" in
    *=*) : ;;
    *) harness_reject "edit must be <linkage-key>=<doc-type:id>"; return 1 ;;
  esac
  local lk="${user_value%%=*}" tr="${user_value#*=}"
  case "$(linkage_card "$lk")" in single | list) : ;; *) harness_reject "unknown linkage key '$lk'"; return 1 ;; esac
  if ! printf '%s' "$tr" | grep -qE "$LINKAGE_REF_RE"; then
    harness_reject "target '$tr' is not a typed doc-type:id reference"
    return 1
  fi
  return 0
}

# Insert the typed linkage (canonical side) into the artifact's frontmatter.
migration_apply_decision() {
  local key="$1" path="$2" anchor="$3" decision="$4" value="$5"
  local lk="${value%%=*}" tr="${value#*=}"
  [ -n "$lk" ] && [ -n "$tr" ] || return 0
  local abs="$PROJECT_ROOT/$path" tmp
  [ -f "$abs" ] || return 0
  tmp="$(mktemp)"
  awk -f "$MERGE_AWK" -v lkey="$lk" -v lval="$tr" -v card="$(linkage_card "$lk")" \
    <"$abs" >"$tmp"
  cmp -s "$abs" "$tmp" || atomic_write "$abs" <"$tmp"
  rm -f "$tmp"
}

# Confirm the recorded linkage is present (drives resume DRIFT recovery).
migration_verify_applied() {
  local key="$1" path="$2" anchor="$3" outcome="$4" proposed="$5" user="$6"
  local value="${user:-$proposed}"
  local lk="${value%%=*}" tr="${value#*=}"
  [ -n "$lk" ] && [ -n "$tr" ] || return 0
  local abs="$PROJECT_ROOT/$path"
  [ -f "$abs" ] || return 1
  grep -qF -- "\"$tr\"" "$abs"
}

migration_session_log_path() {
  printf '.accelerator/state/migrations-%s-session.jsonl\n' "${MIGRATION_ID:-0007-unify-meta-corpus-frontmatter}"
}

# ── Orchestration: (0) pre-pass → (1) backfill → (2) rewrite → (3) harness ───
# Everything before harness_run goes to stderr — the runner parses this
# migration's stdout as the interactive frame stream.
{
  if ! precondition_prepass; then
    log_warn "0007: precondition pre-pass refused — zero files mutated" >&2
    exit 1
  fi
  run_backfill
  run_rewrite
  if [ "$REFUSE_COUNT" -gt 0 ] || [ "$MALFORMED_COUNT" -gt 0 ]; then
    log_warn "0007: $REFUSE_COUNT REFUSE / $MALFORMED_COUNT MALFORMED — failing" >&2
    exit 1
  fi
  self_validate_structural
} >&2

harness_run

# Stage-2: full validation (incl. referential integrity) AFTER harness_run so
# the interactive apply path's writes are covered. Non-zero exit here makes the
# runner withhold the ledger entry.
self_validate_referential
