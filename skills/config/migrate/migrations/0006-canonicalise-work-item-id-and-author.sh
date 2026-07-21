#!/usr/bin/env bash
# DESCRIPTION: Canonicalise plan work-item -> work_item_id and research/RCA researcher -> author. Unconditional within frontmatter; body-label rewrite is anchored to the post-frontmatter / pre-first-H2 region.
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"
ACCELERATOR="${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}"

source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/scripts/log-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi
# Resolve symlinks so downstream `pwd -P` resolutions can be compared
# against PROJECT_ROOT by string prefix on platforms where /var -> /private/var
# (macOS) or similar.
if PROJECT_ROOT_CANON="$(cd "$PROJECT_ROOT" 2>/dev/null && pwd -P)"; then
  PROJECT_ROOT="$PROJECT_ROOT_CANON"
fi

# ---- Path safety ----------------------------------------------------

assert_safe_relpath() {
  local rel="$1"
  local label="$2"
  case "$rel" in
    '' | . | .. | / | /* | */.. | ../* | */../* | */./*)
      log_warn "0006: refusing dangerous $label value: $rel"
      return 1
      ;;
  esac
  # Resolve the parent through symlinks then append the basename — handles
  # missing leaves correctly while still catching symlinked parents. If the
  # parent itself does not exist (e.g. paths.plans = docs/typo-plans where
  # docs/ has not been created either), the surface-form traversal-pattern
  # check above is the only guard — that's acceptable: walk_corpus's
  # "directory does not exist" branch will then warn cleanly.
  local parent leaf abs_parent
  parent="$(dirname "$rel")"
  leaf="$(basename "$rel")"
  if abs_parent="$(cd "$PROJECT_ROOT" && CDPATH='' cd -- "$parent" 2>/dev/null && pwd -P)"; then
    local canonical="$abs_parent/$leaf"
    case "$canonical" in
      "$PROJECT_ROOT" | "$PROJECT_ROOT"/*) return 0 ;;
      *)
        log_warn "0006: $label resolves outside project root: $rel -> $canonical"
        return 1
        ;;
    esac
  fi
  # Parent doesn't exist yet — accept on the basis of the surface-form check.
  return 0
}

resolve_corpus_path() {
  local key="$1"
  local rel rc=0
  # Called inside a command substitution, so it must not exit here: return 2 on
  # a genuine read failure (the caller turns that into a fatal), 1 on an unset
  # key (the caller keeps its "skip this corpus" meaning).
  rel="$(cd "$PROJECT_ROOT" &&
    "$ACCELERATOR" config path --allow-legacy-layout "$key")" || rc=$?
  if [ "$rc" -ne 0 ]; then
    return 2
  fi
  if [ -z "$rel" ]; then
    log_warn "0006: config path returned empty for '$key' — skipping corpus"
    return 1
  fi
  if ! assert_safe_relpath "$rel" "paths.$key"; then
    log_warn "0006: skipping unsafe paths.$key — other corpora will still migrate"
    return 1
  fi
  printf '%s\n' "$rel"
}

canonicalise_rel() {
  local rel="$1"
  (cd "$PROJECT_ROOT" && CDPATH='' cd -- "$rel" 2>/dev/null && pwd -P) ||
    printf '%s\n' "$PROJECT_ROOT/$rel"
}

# ---- Awk transform --------------------------------------------------

awk_transform() {
  awk \
    -v file="$1" \
    -v has_wi="${2:-0}" \
    -v has_id="${3:-0}" \
    -v has_r="${4:-0}" \
    -v has_a="${5:-0}" \
    -v has_rb="${6:-0}" \
    -v has_ab="${7:-0}" '
    function normalise_value(line,    inner) {
      if (line ~ /^".*"$/) return line
      if (line ~ /^'\''.*'\''$/) {
        inner = substr(line, 2, length(line) - 2)
        gsub(/\\/, "\\\\", inner)
        gsub(/"/, "\\\"", inner)
        return "\"" inner "\""
      }
      return "\"" line "\""
    }
    function semantic_inner(line,    inner) {
      if (line ~ /^".*"$/) return substr(line, 2, length(line) - 2)
      if (line ~ /^'\''.*'\''$/) return substr(line, 2, length(line) - 2)
      return line
    }
    function refuses(line) {
      if (line ~ /^".*"$/ || line ~ /^'\''.*'\''$/) return 0
      if (line ~ /#/) return 1
      if (line ~ /"/) return 1
      return 0
    }

    BEGIN {
      in_frontmatter = 0
      seen_frontmatter_open = 0
      saw_first_h2 = 0
      saw_work_item = 0; saw_work_item_id = 0
      saw_researcher = 0; saw_author = 0
      saw_body_researcher = 0; saw_body_author = 0
      first_wi = ""; first_id = ""
      first_r = ""; first_a = ""
      first_rb = ""; first_ab = ""
      inner_wi = ""; inner_id = ""
      saw_wi_anywhere = 0
      saw_r_anywhere = 0
      saw_rb_anywhere = 0
      dropped_first_rb = 0
    }

    /^work-item:/ { saw_wi_anywhere = 1 }
    /^researcher:/ { saw_r_anywhere = 1 }
    /^\*\*Researcher\*\*:/ { saw_rb_anywhere = 1 }

    !seen_frontmatter_open && /^---$/ {
      seen_frontmatter_open = 1
      in_frontmatter = 1
      print; next
    }
    in_frontmatter && /^---$/ {
      in_frontmatter = 0
      print; next
    }
    /^## / { saw_first_h2 = 1 }

    in_frontmatter && /^work-item:/ {
      line = $0
      sub(/^work-item:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (refuses(line)) {
        print $0
        print "0006-REFUSE: " file " — refused work-item (unsafe value shape)" > "/dev/stderr"
        next
      }
      if (!saw_work_item) {
        first_wi = line
        inner_wi = semantic_inner(line)
        saw_work_item = 1
      }
      if (has_id == "1") { next }
      if (line == "") { print "work_item_id:"; next }
      print "work_item_id: " normalise_value(line)
      next
    }

    in_frontmatter && /^work_item_id:/ {
      line = $0
      sub(/^work_item_id:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (refuses(line)) {
        print $0
        print "0006-REFUSE: " file " — refused work_item_id (unsafe value shape)" > "/dev/stderr"
        next
      }
      if (!saw_work_item_id) {
        first_id = line
        inner_id = semantic_inner(line)
        saw_work_item_id = 1
      } else {
        if (semantic_inner(line) != inner_id) {
          print "0006-DIVERGE: " file " — multiple work_item_id values: kept first \"" inner_id "\", dropped \"" semantic_inner(line) "\"" > "/dev/stderr"
        }
        next
      }
      if (line == "") { print "work_item_id:"; next }
      print "work_item_id: " normalise_value(line)
      next
    }

    in_frontmatter && /^researcher:/ {
      line = $0
      sub(/^researcher:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_researcher) { first_r = line; saw_researcher = 1 }
      if (has_a == "1") { next }
      sub(/^researcher:/, "author:")
      print
      next
    }
    in_frontmatter && /^author:/ {
      line = $0
      sub(/^author:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_author) { first_a = line; saw_author = 1 }
      print
      next
    }

    !saw_first_h2 && /^\*\*Researcher\*\*:/ {
      line = $0
      sub(/^\*\*Researcher\*\*:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_body_researcher) { first_rb = line; saw_body_researcher = 1 }
      if (has_ab == "1" && !dropped_first_rb) { dropped_first_rb = 1; next }
      sub(/^\*\*Researcher\*\*:/, "**Author**:")
      print
      next
    }
    !saw_first_h2 && /^\*\*Author\*\*:/ {
      line = $0
      sub(/^\*\*Author\*\*:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_body_author) { first_ab = line; saw_body_author = 1 }
      print
      next
    }

    { print }

    END {
      if (saw_work_item && saw_work_item_id && inner_wi != inner_id) {
        print "0006-DIVERGE: " file " — work-item=" first_wi " vs work_item_id=" first_id " (kept work_item_id)" > "/dev/stderr"
      }
      if (saw_researcher && saw_author && first_r != first_a) {
        print "0006-DIVERGE: " file " — researcher=" first_r " vs author=" first_a " (kept author)" > "/dev/stderr"
      }
      if (saw_body_researcher && saw_body_author && first_rb != first_ab) {
        print "0006-DIVERGE: " file " — **Researcher**=" first_rb " vs **Author**=" first_ab " (kept **Author**)" > "/dev/stderr"
      }
      if (!seen_frontmatter_open && (saw_wi_anywhere || saw_r_anywhere || saw_rb_anywhere)) {
        print "0006-MALFORMED: " file " — legacy key seen but no frontmatter fence (---) detected" > "/dev/stderr"
      }
    }
  '
}

extract_frontmatter() {
  awk '/^---$/ { c++; if (c == 1) next; if (c == 2) exit } c == 1 { print }' "$1"
}

extract_pre_h2() {
  awk '/^## / { exit } { print }' "$1"
}

rewrite_file() {
  local file="$1"
  if ! grep -qE '^(work-item:|work_item_id:|researcher:|author:|\*\*Researcher\*\*:|\*\*Author\*\*:)' "$file" 2>/dev/null; then
    printf '0\n'
    return 0
  fi

  local fm pre_h2
  fm=$(extract_frontmatter "$file")
  pre_h2=$(extract_pre_h2 "$file")
  local has_wi=0 has_id=0 has_r=0 has_a=0 has_rb=0 has_ab=0
  if printf '%s\n' "$fm" | grep -q '^work-item:'; then has_wi=1; fi
  if printf '%s\n' "$fm" | grep -q '^work_item_id:'; then has_id=1; fi
  if printf '%s\n' "$fm" | grep -q '^researcher:'; then has_r=1; fi
  if printf '%s\n' "$fm" | grep -q '^author:'; then has_a=1; fi
  if printf '%s\n' "$pre_h2" | grep -q '^\*\*Researcher\*\*:'; then has_rb=1; fi
  if printf '%s\n' "$pre_h2" | grep -q '^\*\*Author\*\*:'; then has_ab=1; fi

  local tmp_out tmp_err
  tmp_out=$(mktemp)
  tmp_err=$(mktemp)
  # shellcheck disable=SC2094 # reads the input file but writes a distinct temp file (atomic-write idiom); not the same file
  awk_transform "$file" "$has_wi" "$has_id" "$has_r" "$has_a" "$has_rb" "$has_ab" \
    <"$file" >"$tmp_out" 2>"$tmp_err"

  local touched=0
  if ! cmp -s "$file" "$tmp_out"; then
    atomic_write "$file" <"$tmp_out"
    touched=1
  fi

  if [ -s "$tmp_err" ]; then
    while IFS= read -r line; do
      log_warn "0006: ${line}"
    done <"$tmp_err"
  fi
  rm -f "$tmp_out" "$tmp_err"

  printf '%s\n' "$touched"
}

# ---- Walker ---------------------------------------------------------

walk_corpus() {
  local key="$1"
  local rel rc=0
  # The `if !` form discards resolve_corpus_path's graded return, so capture it:
  # rc=2 is a read failure (fatal), rc=1 is an unresolved/unsafe key (skip).
  rel="$(resolve_corpus_path "$key")" || rc=$?
  case "$rc" in
    2) log_die "0006: config read failed for paths.$key" ;;
    1)
      echo "0006: rewrote 0 file(s) under <unresolved $key>"
      return 0
      ;;
  esac
  local abs="$PROJECT_ROOT/$rel"
  if [ ! -d "$abs" ]; then
    log_warn "0006: $key directory does not exist: $rel"
    echo "0006: rewrote 0 file(s) under $rel"
    return 0
  fi
  local rewrote=0 touched
  while IFS= read -r -d '' file; do
    touched=$(rewrite_file "$file")
    if [[ ! "${touched:-}" =~ ^[0-9]+$ ]]; then
      log_warn "0006: rewrite_file '$file' produced non-numeric touched ('${touched:-<empty>}') — treating as 0"
      touched=0
    fi
    rewrote=$((rewrote + touched))
  done < <(find "$abs" -type f -name '*.md' -print0)
  echo "0006: rewrote $rewrote file(s) under $rel"
}

# bash-3.2 has no associative arrays: track walked canons with parallel indexed
# arrays + a linear-search owner lookup (canon -> first key that recorded it).
WALKED_CANONS=()
WALKED_KEYS=()
_walked_owner() { # echoes the recorded key for a canon, empty if unseen
  local needle="$1" i
  for ((i = 0; i < ${#WALKED_CANONS[@]}; i++)); do
    if [ "${WALKED_CANONS[$i]}" = "$needle" ]; then
      printf '%s' "${WALKED_KEYS[$i]}"
      return 0
    fi
  done
  return 0 # not found: echo nothing, succeed (a failing match must not abort set -e)
}
for key in plans research_codebase research_issues; do
  raw_rel="$(cd "$PROJECT_ROOT" && "$ACCELERATOR" config path --allow-legacy-layout "$key")" ||
    log_die "0006: config read failed for paths.$key"
  if [ -z "$raw_rel" ]; then continue; fi
  canon="$(canonicalise_rel "$raw_rel")"
  owner="$(_walked_owner "$canon")"
  if [ -n "$owner" ]; then
    log_warn "0006: paths.$key aliases paths.$owner ($raw_rel -> $canon) — skipping duplicate walk"
    echo "0006: skipping duplicate walk for paths.$key"
    continue
  fi
  WALKED_CANONS+=("$canon")
  WALKED_KEYS+=("$key")
  walk_corpus "$key"
done

# ---- Userspace template overrides -----------------------------------

resolve_user_template_path() {
  local name="$1"

  # Called inside a command substitution, so a read failure returns 2 (the
  # caller turns that into a fatal) rather than exiting only this subshell.
  local tier1 rc=0
  tier1="$(cd "$PROJECT_ROOT" &&
    "$ACCELERATOR" config get --allow-legacy-layout "templates.$name")" || rc=$?
  [ "$rc" -eq 0 ] || return 2
  if [ -n "$tier1" ]; then
    if ! assert_safe_relpath "$tier1" "templates.$name"; then
      return 0
    fi
    local tier1_abs="$PROJECT_ROOT/$tier1"
    if [ -f "$tier1_abs" ]; then
      printf '%s\n' "$tier1_abs"
      return 0
    fi
    log_warn "0006: templates.$name points at missing file: $tier1 (skipping; not falling through to tier-2)"
    return 0
  fi

  local tdir_rel
  rc=0
  tdir_rel="$(cd "$PROJECT_ROOT" &&
    "$ACCELERATOR" config path --allow-legacy-layout templates)" || rc=$?
  [ "$rc" -eq 0 ] || return 2
  if [ -n "$tdir_rel" ]; then
    local tier2_abs="$PROJECT_ROOT/$tdir_rel/$name.md"
    if [ -f "$tier2_abs" ]; then
      printf '%s\n' "$tier2_abs"
    fi
  fi
}

# bash-3.2: parallel indexed arrays + linear search in place of an associative
# array keyed by resolved path (path -> first template name that recorded it).
TEMPLATE_PATHS=()
TEMPLATE_NAMES=()
_template_owner() { # echoes the recorded name for a resolved path, empty if unseen
  local needle="$1" i
  for ((i = 0; i < ${#TEMPLATE_PATHS[@]}; i++)); do
    if [ "${TEMPLATE_PATHS[$i]}" = "$needle" ]; then
      printf '%s' "${TEMPLATE_NAMES[$i]}"
      return 0
    fi
  done
  return 0 # not found: echo nothing, succeed (a failing match must not abort set -e)
}
for name in plan codebase-research rca; do
  if ! path=$(resolve_user_template_path "$name"); then
    log_die "0006: config read failed resolving template $name"
  fi
  if [ -n "$path" ]; then
    owner="$(_template_owner "$path")"
    if [ -n "$owner" ]; then
      log_warn "0006: templates.$name and templates.$owner resolve to the same file ($path) — skipping duplicate rewrite"
      continue
    fi
    TEMPLATE_PATHS+=("$path")
    TEMPLATE_NAMES+=("$name")
    touched=$(rewrite_file "$path")
    echo "0006: template $name (tier-resolved $path): touched=$touched"
  fi
done
