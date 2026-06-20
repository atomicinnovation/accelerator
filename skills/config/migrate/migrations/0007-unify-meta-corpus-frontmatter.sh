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
# Single source for path→doc-type classification + out-of-scope (previously a
# byte-identical copy lived here and in the validator).
source "$PLUGIN_ROOT/scripts/doc-type-inference.sh"
# Shared loader (load_doc_type_table) for the config-driven doc-type allowlist
# injected into the classifier above. Sourced by literal path like the classifier
# itself; the resolver it spawns is overridable via DOC_TYPE_PATHS_RESOLVER.
source "$PLUGIN_ROOT/scripts/doc-type-table.sh"
# Cross-cutting schema rules: fm_assert_schema_columns (column-order guard) and
# FM_OPTIONAL_EXTRAS (the optional-extra carve-out, consumed in the required-
# extras backfill). NB: a shipped migration must reproduce its historical output
# forever, but the SPECIFIC facts 0007 consumes — the column ORDER and the
# optional-extra set for the types 0007 touches — are contractually stable
# (those extras are *required*, not optional), so this dependency on the evolving
# single source is safe and is regression-guarded by the test suite.
source "$PLUGIN_ROOT/scripts/frontmatter-emission-rules.sh"

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

# Resolve-and-inject the config-driven doc-type allowlist ONCE, against the
# canonicalised corpus root (not the migration's CWD), so BOTH the scope gate
# (out_of_scope) and the type derivation (infer_type_from_path — the type
# written into regenerated frontmatter) are config-aware and consistent with the
# validator. Non-fatal here; the orchestration block's pre-mutation guard fails
# closed on a wholesale resolution failure before anything is mutated.
DOC_TYPE_TABLE_OK=1
load_doc_type_table "$PROJECT_ROOT" || DOC_TYPE_TABLE_OK=0
# type<TAB>dir snapshot of the injected table for the awk path_to_typed (-v
# doc_type_table channel) so the linkage-value classifier is config-aware in step
# with the shell-side scope/derivation. Records are joined by the ASCII Record
# Separator 0x1E (NOT a newline — the one-true-awk rejects a newline in a -v
# value; 0x1E cannot occur in a type name or a path).
DOC_TYPE_RS=$'\x1e'
DOC_TYPE_TSV=""
for _dti in "${!DOC_TYPE_INJECTED_NAMES[@]}"; do
  _dt_rec="$(printf '%s\t%s' "${DOC_TYPE_INJECTED_NAMES[$_dti]}" "${DOC_TYPE_INJECTED_DIRS[$_dti]}")"
  DOC_TYPE_TSV="${DOC_TYPE_TSV:+$DOC_TYPE_TSV$DOC_TYPE_RS}$_dt_rec"
done

# Overridable (test-only seam, mirrors the validator's) so a fixture can prove
# the forbidden-key drop and required-extras backfill are schema-driven.
SCHEMA_TSV="${SCHEMA_TSV:-$PLUGIN_ROOT/scripts/templates-schema.tsv}"
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
# Space-joined extras for a type (schema TSV col 4); "-" → empty. (Was wrongly
# cut -f5 — status_vocab — and dead; now correct and live in the extras backfill.)
extras_for_type() {
  local v
  v="$(schema_row "$1" | cut -f4)"
  [ "$v" = "-" ] && v=""
  printf '%s' "$v"
}
# Status vocab is column 5.
status_vocab_of() { schema_row "$1" | awk -F'\t' '{print $5}'; }

own_id_key_for_type() {
  case "$1" in
    work-item) echo work_item_id ;;
    adr) echo adr_id ;;
    *) echo "" ;;
  esac
}

# Space-joined forbidden own-id keys for a type (schema TSV col 6); "-" → empty.
forbidden_keys_for_type() {
  local v
  v="$(schema_row "$1" | cut -f6)"
  [ "$v" = "-" ] && v=""
  printf '%s' "$v"
}

# Canonicalise a legacy artifact-type alias to its ADR-0033 type (mirrors the
# awk's canonical_type). Used wherever the shell looks a present type: up in the
# schema table — the raw legacy alias has no row, so vocab/extras/anchored would
# misfire (e.g. a `type: validation` file's `status: complete` DIVERGEing).
canonical_type() {
  case "$1" in
    validation) echo plan-validation ;;
    *) echo "$1" ;;
  esac
}

# Space-joined "legacy=canonical" status pairs for a type (for awk -v).
status_map_for_type() {
  awk -F'\t' -v t="$1" 'NR > 1 && $1 == t { printf "%s=%s ", $2, $3 }' "$STATUS_MAP_TSV"
}

# infer_type_from_path / out_of_scope are sourced from
# scripts/doc-type-inference.sh (single source, shared with the validator).

stem_of() {
  local b
  b="$(basename "$1")"
  printf '%s' "${b%.md}"
}

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
    '"'*'"')
      v="${v#\"}"
      v="${v%\"}"
      ;;
    "'"*"'")
      v="${v#\'}"
      v="${v%\'}"
      ;;
  esac
  printf '%s' "$v"
}

# Empty-placeholder test, mirroring the awk is_empty_val and the validator's
# EMPTY-PLACEHOLDER rule. Keep the three in lockstep. Used by the required-extras
# backfill so a present-but-empty placeholder ("" / []) counts as absent (the awk
# would otherwise drop it via omit-when-empty, leaving the file MISSING-EXTRA).
fm_is_empty_val() { case "$1" in '' | '""' | '[]') return 0 ;; *) return 1 ;; esac }

# Echo the default value for a required extra, or empty if none derivable.
# CRITICAL: every command substitution here must succeed under `set -euo
# pipefail` — an unguarded pipe whose grep finds no match exits non-zero and
# aborts the whole migration mid-rewrite (the exact permanent-stall this fixes).
# Hence the `|| true` guards.
extra_default() { # $1=extra-name $2=file $3=stem $4=title
  local n
  case "$1" in
    topic)
      # ← title, with embedded quotes stripped (parity with title_default's
      # `tr -d '"'`; an unescaped " in a double-quoted scalar is invalid YAML).
      printf '%s' "$4" | tr -d '"'
      ;;
    pr_number)
      # PR-anchored number: digits of a genuine pr-/PR- *segment* — the `pr` token
      # must be at start-of-stem or preceded by a hyphen, so a `pr` embedded in a
      # word (expr-3, improve-2) does NOT match.
      n="$(printf '%s' "$3" | grep -oE '(^|-)[Pp][Rr]-?[0-9]+' | grep -oE '[0-9]+' | head -1 || true)"
      # Leading-number fallback ONLY for a stem that is NOT date-prefixed (e.g.
      # 240-description → 240). A date-prefixed, pr-token-less stem
      # (2026-06-17-summary, 2026-06-17-0114-foo) has no derivable PR number →
      # stays empty, so the builder breadcrumbs it rather than fabricating a part.
      if [ -z "$n" ]; then
        case "$3" in
          [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]*) : ;; # date-prefixed → no fallback
          *) n="$(printf '%s' "$3" | grep -oE '^[0-9]+' | head -1 || true)" ;;
        esac
      fi
      printf '%s' "$n"
      ;;
    review_number) printf '1' ;;
    review_pass) printf '1' ;; # parity with review_number
    sequence) printf '1' ;;    # design-inventory ordinal default
    # screenshots_incomplete defaults true (conservative — an inventory whose
    # flag was never set is treated as NOT vouched complete, per Migration Notes).
    screenshots_incomplete) printf 'true' ;;
    verdict) printf 'unknown' ;; # sentinel
    lenses) printf 'unknown' ;;  # sentinel (emitted as a list)
    # No derivable default → the backfill loop stamps the `unknown` string
    # sentinel for these string/enum extras (see the no-derivable-default branch).
    *) printf '' ;;
  esac
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

# Sets RESOLVED_DATE to the YYYY-MM-DD the file was first authored (VCS), or ""
# when no VCS / no history. Used as the date fallback for files (e.g.
# work-items) whose filename carries no date prefix.
resolve_date() {
  local relpath="$1" d=""
  if [ -d "$PROJECT_ROOT/.jj" ] && command -v jj >/dev/null 2>&1; then
    d="$(cd "$PROJECT_ROOT" && LANG=C jj --no-pager log --no-graph \
      -r "latest(::@ & files(\"$relpath\"))" -T 'author.timestamp().format("%Y-%m-%d")' 2>/dev/null | head -1 || true)"
  elif [ -d "$PROJECT_ROOT/.git" ] && command -v git >/dev/null 2>&1; then
    d="$(cd "$PROJECT_ROOT" && LANG=C git log --format='%ad' --date=format:'%Y-%m-%d' -1 -- "$relpath" 2>/dev/null | head -1 || true)"
  fi
  RESOLVED_DATE="$d"
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
  local f type id own seen_ids="" refused=0
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_strict_fence "$f" || continue
    type="$(fm_inner "$(fm_get type "$f")")"
    [ -n "$type" ] || type="$(infer_type_from_path "$f")"
    type="$(canonical_type "$type")"
    if [ "$type" = "work-item" ]; then
      if [ -z "$(fm_get kind "$f")" ]; then
        log_warn "0007-REFUSE: $f — work-item missing kind: (run migration 0005 first)" >&2
        refused=1
      fi
      own="$(fm_inner "$(fm_get work_item_id "$f")")"
      local fid
      fid="$(printf '%s' "$(stem_of "$f")" | grep -oE '^[0-9]+' || true)"
      if [ -n "$own" ] && [ -n "$fid" ] && [ "$own" != "$fid" ]; then
        log_warn "0007-REFUSE: $f — own work_item_id '$own' != filename id '$fid'" >&2
        refused=1
      fi
    else
      # Foreign work_item_id must already be quoted (0006 guarantee).
      local raw
      raw="$(fm_get work_item_id "$f")"
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
      *"|${typed_id}|"*)
        log_warn "0007-REFUSE: $f — duplicate post-rewrite id '$typed_id'" >&2
        refused=1
        ;;
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
  # Title from first H1, else humanised stem. Trim surrounding whitespace: a
  # column-padded source line would otherwise yield a quoted scalar with
  # trailing spaces, which crashes the visualiser's YAML parser (libyml panics
  # on a quoted flow scalar ending in trailing whitespace).
  h1="$(awk '/^# / { sub(/^# /, ""); sub(/[[:space:]]+$/, ""); print; exit }' "$f" || true)"
  title="${h1:-$stem}"
  # Strip embedded quotes — an unescaped " inside a double-quoted scalar is
  # invalid YAML (parity with the fenced rewrite's title_default path). Applies
  # to both the title: and the topic: emitted below.
  title="$(printf '%s' "$title" | tr -d '"')"
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
    fi
    case "$type" in
      note | codebase-research | issue-research)
        # title is already quote-stripped above (parity with the fenced path).
        printf 'topic: "%s"\n' "$title"
        ;;
    esac
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
  type="$(canonical_type "$type")"
  [ -n "$type" ] || return 0
  anchored=0
  [ "$(anchored_for_type "$type")" = "yes" ] && anchored=1
  own="$(own_id_key_for_type "$type")"
  local forbidden
  forbidden="$(forbidden_keys_for_type "$type")"
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
  local has_priority=0
  [ -n "$(fm_get priority "$f")" ] && has_priority=1

  # Derive the "hard" base fields a fenced file may lack — same sources as the
  # fence-less backfill: H1 → title, VCS → author/revision, filename → date.
  local rel="${f#"$PROJECT_ROOT"/}"
  local title_default="" author_default="" date_default="" revision_default=""
  if [ "$has_title" -eq 0 ]; then
    title_default="$(awk 'f && /^# /{sub(/^# /,""); print; exit} /^---[[:space:]]*$/{c++; if(c==2)f=1}' "$f" | tr -d '"')"
    [ -n "$title_default" ] || title_default="$stem"
  fi
  if [ "$has_author" -eq 0 ]; then
    resolve_author "$rel"
    author_default="$RESOLVED_AUTHOR"
  fi
  if [ "$has_date" -eq 0 ]; then
    local _d
    _d="$(printf '%s' "$stem" | grep -oE '^[0-9]{4}-[0-9]{2}-[0-9]{2}' || true)"
    if [ -n "$_d" ]; then
      date_default="${_d}T00:00:00+00:00"
    else
      # No date in the filename (e.g. work-items NNNN-slug) — fall back to the
      # VCS-authored date.
      resolve_date "$rel"
      [ -n "$RESOLVED_DATE" ] && date_default="${RESOLVED_DATE}T00:00:00+00:00"
    fi
  fi
  if [ "$anchored" -eq 1 ] && [ "$has_revision" -eq 0 ]; then
    resolve_revision "$rel"
    revision_default="$RESOLVED_REVISION"
  fi

  # Build a SINGLE packed channel of required-and-absent extras to backfill, one
  # `name=value` record per extra, separated by an ASCII Unit Separator (0x1F)
  # that cannot occur in a single-line YAML scalar (safe under LC_ALL=C) — NOT a
  # printable `;`, which a topic (an arbitrary user H1) could legitimately carry.
  # One packed -v (parsed by a generic awk emit loop) instead of one -v per extra
  # keeps a future schema extra from needing a new awk parameter.
  local US backfill_extras="" ex dv cur_title
  US=$'\x1F'
  # Resolved current title: prefer the file's own title:, else the H1/stem default.
  cur_title="$(fm_inner "$(fm_get title "$f")")"
  [ -n "$cur_title" ] || cur_title="$title_default"
  for ex in $(extras_for_type "$type"); do
    case " $FM_OPTIONAL_EXTRAS " in *" $ex "*) continue ;; esac # optional → skip
    fm_is_empty_val "$(fm_get "$ex" "$f")" || continue          # present & non-empty → skip
    dv="$(extra_default "$ex" "$f" "$stem" "$cur_title")"
    dv="${dv//$US/}" # defence-in-depth: strip any stray US byte from the value
    if [ -z "$dv" ]; then
      # Underivable string/enum required extra → write the `unknown` sentinel
      # (parity with the verdict/lenses contract) so self_validate_structural
      # sees a present value rather than aborting on MISSING-EXTRA; breadcrumb
      # each stamped file so the sentinel write is auditable, not silent (0118).
      # Numeric/boolean extras never reach here — they get typed defaults in
      # extra_default. See Migration Notes for the dual sentinel-source rationale.
      dv='unknown'
      log_warn "0007-DIVERGE[backfill-sentinel]: $f — required extra '$ex' has no derivable default; stamped 'unknown'" >&2
    fi
    backfill_extras="${backfill_extras:+$backfill_extras$US}${ex}=${dv}"
  done

  local tmp_out tmp_err
  tmp_out="$(mktemp)"
  tmp_err="$(mktemp)"
  awk -f "$FRAG_AWK" -f "$BODY_AWK" \
    -v file="$f" -v type="$type" -v anchored="$anchored" -v own_id_key="$own" \
    -v doc_type_table="$DOC_TYPE_TSV" \
    -v forbidden="$forbidden" -v backfill_extras="$backfill_extras" \
    -v id_from_stem="$idstem" -v repo_name="$repo" \
    -v statusvocab="$vocab" -v statusmap="$smap" \
    -v title_default="$title_default" -v author_default="$author_default" \
    -v date_default="$date_default" -v revision_default="$revision_default" \
    -v has_type="$has_type" -v has_id="$has_id" -v has_title="$has_title" -v has_tags="$has_tags" \
    -v has_priority="$has_priority" \
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
    has_strict_fence "$f" || continue # backfill already fenced these
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
  # Pin CWD to PROJECT_ROOT so the spawned validator resolves the same allowlist
  # that scoped the mutation (even though file-list mode does not filter by it).
  (cd "$PROJECT_ROOT" && bash "$VALIDATOR" "${files[@]}") >&2
}
self_validate_referential() {
  # Pin CWD to PROJECT_ROOT so the whole-corpus self-check observes the SAME
  # doc-type allowlist that drove the mutation — a CWD != PROJECT_ROOT invocation
  # cannot make it validate a different file set than was mutated.
  (cd "$PROJECT_ROOT" && bash "$VALIDATOR" "$META_ABS") >&2
}

# ── Corpus identity index (for existence-checking resolved inferences) ───────
# Newline-delimited set of "type:id" identities over the (post-rewrite) corpus,
# resolved by the same rule the migration uses (own-id → id → derived stem).
CORPUS_INDEX=""
build_corpus_index() {
  local f type id
  while IFS= read -r -d '' f; do
    out_of_scope "$f" && continue
    has_strict_fence "$f" || continue
    type="$(fm_inner "$(fm_get type "$f")")"
    [ -n "$type" ] || type="$(infer_type_from_path "$f")"
    type="$(canonical_type "$type")"
    [ -n "$type" ] || continue
    case "$type" in
      work-item | adr) id="$(fm_inner "$(fm_get "$(own_id_key_for_type "$type")" "$f")")" ;;
      *) id="" ;;
    esac
    [ -n "$id" ] || id="$(fm_inner "$(fm_get id "$f")")"
    [ -n "$id" ] || id="$(derive_stem "$f" "$type")"
    CORPUS_INDEX="${CORPUS_INDEX}${type}:${id}"$'\n'
  done < <(corpus_files)
}
corpus_index_has() { grep -qxF -- "$1" <<<"$CORPUS_INDEX"; }

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
LINKAGE_REF_RE='^(work-item|plan|adr|pr|codebase-research|issue-research|pr-description|design-inventory|design-gap|plan-validation|plan-review|work-item-review|pr-review|note):[A-Za-z0-9.-]+$'

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
    # shellcheck disable=SC2034  # `src` is the leading TSV column, read past but unused here
    while IFS=$'\t' read -r src key target anchor band; do
      [ -n "$key" ] && [ -n "$target" ] || continue
      # Existence-check resolved inferences: a resolved-band target that does
      # not resolve to a real artifact (e.g. a year/date mis-parsed from prose,
      # like "work-item:2026") is NOT applied mechanically — skip + DIVERGE
      # rather than write a dangling edge. pr: is tolerated. Ambiguous-band
      # refs are emitted regardless (the human gates them).
      if [ "$band" = "resolved" ]; then
        case "$target" in
          pr:*) : ;;
          *:*) corpus_index_has "$target" ||
            {
              log_warn "0007-DIVERGE[reverse-orphan]: $rel — resolved $key target '$target' resolves to no artifact; skipped" >&2
              continue
            } ;;
        esac
      fi
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
    *)
      harness_reject "edit must be <linkage-key>=<doc-type:id>"
      return 1
      ;;
  esac
  local lk="${user_value%%=*}" tr="${user_value#*=}"
  case "$(linkage_card "$lk")" in single | list) : ;; *)
    harness_reject "unknown linkage key '$lk'"
    return 1
    ;;
  esac
  if ! printf '%s' "$tr" | grep -qE "$LINKAGE_REF_RE"; then
    harness_reject "target '$tr' is not a typed doc-type:id reference"
    return 1
  fi
  return 0
}

# Insert the typed linkage (canonical side) into the artifact's frontmatter.
migration_apply_decision() {
  # $4 is the decision label (consumed by the caller, not needed here).
  local key="$1" path="$2" anchor="$3" value="$5"
  local lk="${value%%=*}" tr="${value#*=}"
  [ -n "$lk" ] && [ -n "$tr" ] || return 0
  local abs="$PROJECT_ROOT/$path" tmp
  [ -f "$abs" ] || return 0
  # Single-valued keys are set-if-absent-or-equal, never overwritten: a file may
  # have competing inferences for the same single key (e.g. two `source:`
  # candidates), and last-wins would be resume-order-dependent (resolved refs
  # re-apply every run; resumed ambiguous ones do not) — non-idempotent. Once
  # set, the value sticks; a conflicting candidate is kept-out and DIVERGEd.
  if [ "$(linkage_card "$lk")" = "single" ]; then
    local existing
    existing="$(fm_inner "$(fm_get "$lk" "$abs")")"
    if [ -n "$existing" ] && [ "$existing" != "$tr" ]; then
      log_warn "0007-DIVERGE[parent-conflict]: $path — $lk already '$existing'; not overwriting with '$tr'" >&2
      return 0
    fi
  fi
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
  # A single-valued key counts as applied once it is populated, even if a
  # conflicting candidate won the set-if-absent race (the relationship is
  # handled; the conflict is in the DIVERGE log). This keeps resume from
  # spuriously DRIFTing the candidate that was kept out.
  if [ "$(linkage_card "$lk")" = "single" ] && [ -n "$(fm_inner "$(fm_get "$lk" "$abs")")" ]; then
    return 0
  fi
  grep -qF -- "\"$tr\"" "$abs"
}

migration_session_log_path() {
  printf '.accelerator/state/migrations-%s-session.jsonl\n' "${MIGRATION_ID:-0007-unify-meta-corpus-frontmatter}"
}

# Test-only seam (never set in production, mirrors the SCHEMA_TSV/
# DOC_TYPE_INFERENCE overrides): when sourced with this sentinel, stop here so the
# suite can unit-test the pure helpers (extra_default, forbidden_keys_for_type,
# …) without triggering the corpus orchestration / interactive harness below.
# `return` when sourced (the test path); `exit 0` is the defensive
# executed-with-sentinel fallback ShellCheck cannot see is reachable.
# shellcheck disable=SC2317
if [ "${ACCELERATOR_0007_NO_RUN:-}" = "1" ]; then return 0 2>/dev/null || exit 0; fi

# ── Orchestration: (0) pre-pass → (1) backfill → (2) rewrite → (3) harness ───
# Everything before harness_run goes to stderr — the runner parses this
# migration's stdout as the interactive frame stream.
{
  # Fail loudly (before mutating anything) if the schema's column order changed
  # under the positional cut -fN reads (forbidden col 6, extras col 4).
  fm_assert_schema_columns "$SCHEMA_TSV" || exit 1
  # Fail-closed net: a wholesale doc-type allowlist resolution failure (resolver
  # non-zero or zero rows) would otherwise classify the entire corpus
  # out-of-scope and exit 0 having migrated nothing — indistinguishable from a
  # clean idempotent re-run (the clean-tree net does not catch a no-op). Because
  # a blank config value is coerced to its registry default, a short/empty table
  # only ever signals such a failure. Abort before mutating anything.
  if [ "$DOC_TYPE_TABLE_OK" -ne 1 ]; then
    log_warn "0007: doc-type allowlist resolution failed — zero files mutated — fix .accelerator/config.md (or revert meta/ via your VCS), then re-run" >&2
    exit 1
  fi
  if ! precondition_prepass; then
    log_warn "0007: precondition pre-pass refused — zero files mutated — resolve the refusals above (or revert meta/ via your VCS), then re-run" >&2
    exit 1
  fi
  run_backfill
  run_rewrite
  if [ "$REFUSE_COUNT" -gt 0 ] || [ "$MALFORMED_COUNT" -gt 0 ]; then
    log_warn "0007: $REFUSE_COUNT REFUSE / $MALFORMED_COUNT MALFORMED — failing — revert meta/ via your VCS to recover, then re-run" >&2
    exit 1
  fi
  self_validate_structural
  # Build the identity index over the final (post-rewrite) corpus so the
  # emitter can existence-check resolved inferences before harness_run.
  build_corpus_index
  # Independent backstop for a wrong-but-non-empty scope (which the same-scope
  # self-validation cannot catch by itself): a non-empty corpus that resolved
  # zero in-scope typed files signals a mis-resolved allowlist. The resolver's
  # path-safety rejection removes the traversal/absolute mis-scope class at
  # source; this catches the residue. Recover (as for all 0007 faults) via VCS.
  guard_typed="$(printf '%s' "$CORPUS_INDEX" | grep -c . || true)"
  guard_files="$(corpus_files | tr -cd '\0' | wc -c | tr -d ' ')"
  if [ "$guard_files" -gt 0 ] && [ "$guard_typed" -eq 0 ]; then
    log_warn "0007: corpus has $guard_files file(s) but zero resolved to a configured doc-type — aborting (scope mis-resolved); revert meta/ via your VCS" >&2
    exit 1
  fi
} >&2

harness_run

# Stage-2: full validation (incl. referential integrity) AFTER harness_run so
# the interactive apply path's writes are covered. Non-zero exit here makes the
# runner withhold the ledger entry.
self_validate_referential
