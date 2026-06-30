#!/usr/bin/env bash
# Drift guard for the master skill index (docs/skills/README.md) and the
# per-skill subsections it deep-links to. Derives the user-invokable skill set
# from SKILL.md frontmatter (the canonical source) and asserts the docs track
# it. Auto-discovered by tasks/test/helpers.py (executable scripts/test-*.sh)
# and run under `mise run test:integration:config`.
#
# Guards five invariants:
#   (a) every user-invokable skill is referenced in README.md via its
#       /<name> invocation;
#   (b) no internal (user-invocable: false) skill is referenced there;
#   (c) the invokable set numbers exactly 46 (liveness — fails loudly on an
#       enumeration/exclusion regression);
#   (d) every index deep link <page>.md#<name> resolves to a real
#       `### `/<name>`` heading on its target page;
#   (e) each skill's index gloss AND its home-page description reproduce the
#       first sentence of its SKILL.md description verbatim (whitespace-
#       normalised), so a reworded description can't drift past the docs.
#
# Bash 3.2 floor: no associative arrays, no ${var,,}. Parallel data lives in a
# temp facts file and space-separated name lists.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

SKILLS_DIR="$PLUGIN_ROOT/skills"
DOCS_SKILLS_DIR="$PLUGIN_ROOT/docs/skills"
INDEX="$DOCS_SKILLS_DIR/README.md"
EXPECTED_INVOKABLE_COUNT=46

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
FACTS="$WORK_DIR/facts.tsv" # invokable: name<TAB>first_sentence
: >"$FACTS"

# --- frontmatter facts: print "name<TAB>user-invocable<TAB>folded-description"
# Reads ONLY the leading ---…--- block (so configure's body-level `name:`
# config-key examples are ignored). Folds a multi-line / `>` block-scalar
# description into one line; parse_fm-style first-line capture would lose the
# continuation lines.
skill_facts() {
  awk '
    BEGIN { n = 0; desc = ""; capt = 0; ui = ""; name = "" }
    /^---[[:space:]]*$/ { n++; if (n == 2) exit; next }
    n == 1 {
      if ($0 ~ /^name:/) { v = $0; sub(/^name:[[:space:]]*/, "", v); name = v }
      else if ($0 ~ /^user-invocable:/) {
        v = $0; sub(/^user-invocable:[[:space:]]*/, "", v); ui = v
      }
      else if ($0 ~ /^description:/) {
        v = $0; sub(/^description:[[:space:]]*/, "", v)
        sub(/^>[[:space:]]*$/, "", v); sub(/^>[[:space:]]*/, "", v)
        desc = v; capt = 1; next
      }
      else if (capt == 1) {
        if ($0 ~ /^[A-Za-z_-]+:/) { capt = 0 }
        else {
          line = $0; sub(/^[[:space:]]+/, "", line)
          if (desc == "") { desc = line } else { desc = desc " " line }
        }
      }
    }
    END { printf "%s\t%s\t%s\n", name, ui, desc }
  ' "$1"
}

# First sentence of a folded description: strip a surrounding double-quoted
# scalar, protect `e.g.` / `i.e.`, truncate at the first ". ", restore.
compute_first() {
  local d="$1"
  case "$d" in '"'*'"')
    d="${d#\"}"
    d="${d%\"}"
    ;;
  esac
  d="${d//e.g./e<DOT>g<DOT>}"
  d="${d//i.e./i<DOT>e<DOT>}"
  local first="${d%%. *}"
  [ "$first" != "$d" ] && first="$first."
  first="${first//e<DOT>g<DOT>/e.g.}"
  first="${first//i<DOT>e<DOT>/i.e.}"
  printf '%s' "$first"
}

# Collapse all whitespace runs to single spaces; trim ends.
norm() { tr '\n' ' ' | tr -s ' ' | sed 's/^ //; s/ $//'; }

# Text of a `### `/<name>`` subsection on a docs page (up to the next ###/## or
# EOF). The heading may carry an inline argument hint — `### `/<name> [args]`` —
# so match the `/<name>` prefix up to the space or closing backtick that follows.
section_text() {
  awk -v name="$2" '
    BEGIN { pat = "^### `/" name "[ `]" }
    $0 ~ pat { grab = 1; next }
    grab && (/^### / || /^## /) { exit }
    grab { print }
  ' "$1"
}

# --- Enumerate SKILL.md, pruning vendored deps + a frontmatter-less fixture ---
INVOKABLE_NAMES=""
INTERNAL_NAMES=""
INVOKABLE_COUNT=0
EMPTY_NAME_FILES=""

while IFS= read -r skill; do
  # skill_facts emits exactly "name<TAB>ui<TAB>desc". Split on tab with
  # parameter expansion, NOT `IFS=$'\t' read` — tab is IFS-whitespace, so read
  # collapses the empty ui field of invokable skills and swallows the desc.
  facts_line="$(skill_facts "$skill")"
  tab=$'\t'
  name="${facts_line%%"$tab"*}"
  facts_rest="${facts_line#*"$tab"}"
  ui="${facts_rest%%"$tab"*}"
  desc="${facts_rest#*"$tab"}"
  if [ -z "$name" ]; then
    EMPTY_NAME_FILES="$EMPTY_NAME_FILES $skill"
    continue
  fi
  if [ "$ui" = "false" ]; then
    INTERNAL_NAMES="$INTERNAL_NAMES $name"
  else
    INVOKABLE_NAMES="$INVOKABLE_NAMES $name"
    INVOKABLE_COUNT=$((INVOKABLE_COUNT + 1))
    printf '%s\t%s\n' "$name" "$(compute_first "$desc")" >>"$FACTS"
  fi
done < <(find "$SKILLS_DIR" -name SKILL.md \
  -not -path '*/node_modules/*' -not -path '*/test-fixtures/*' | sort)

echo "=== Enumeration & liveness ==="
assert_empty "no SKILL.md with empty/absent frontmatter name" "$EMPTY_NAME_FILES"
assert_eq "invokable set is exactly $EXPECTED_INVOKABLE_COUNT skills" \
  "$EXPECTED_INVOKABLE_COUNT" "$INVOKABLE_COUNT"

assert_file_exists "master index docs/skills/README.md exists" "$INDEX"

# Membership check shared with the negative self-test. Returns 1 if any
# invokable token is missing or any internal token leaks into the index.
NAME_BOUNDARY='([^A-Za-z0-9-]|$)'
check_membership() {
  local idx="$1" v=0 n
  for n in $INVOKABLE_NAMES; do
    grep -Eq "/${n}${NAME_BOUNDARY}" "$idx" || v=$((v + 1))
  done
  for n in $INTERNAL_NAMES; do
    grep -Eq "/${n}${NAME_BOUNDARY}" "$idx" && v=$((v + 1))
  done
  return $((v > 0 ? 1 : 0))
}

if [ -f "$INDEX" ]; then
  IDX_CONTENT="$(cat "$INDEX")"
  IDX_NORM="$(norm <"$INDEX")"
else
  IDX_CONTENT=""
  IDX_NORM=""
fi

echo ""
echo "=== Index membership (all invokable present, no internal leaked) ==="
for n in $INVOKABLE_NAMES; do
  assert_matches_regex "index references /$n" \
    "/${n}${NAME_BOUNDARY}" "$IDX_CONTENT"
done
for n in $INTERNAL_NAMES; do
  assert_not_matches_regex "index omits internal skill $n" \
    "/${n}${NAME_BOUNDARY}" "$IDX_CONTENT"
done

echo ""
echo "=== Deep-link anchor resolution + description match (all 46) ==="
while IFS=$'\t' read -r name first; do
  [ -n "$name" ] || continue
  # Parse the index deep link `](<page>.md#<name>)` for this skill.
  target="$(grep -oE "\]\([^)]*#${name}\)" "$INDEX" 2>/dev/null | head -1 |
    sed -E 's/^\]\(//; s/\)$//' || true)"
  if [ -z "$target" ]; then
    assert_eq "index deep-links $name to a #${name} anchor" "found" "missing"
    continue
  fi
  page="${target%#*}"
  anchor="${target#*#}"
  assert_eq "deep-link anchor for $name equals its name" "$name" "$anchor"
  resolved="$DOCS_SKILLS_DIR/$page"
  if [ -f "$resolved" ]; then
    assert_matches_regex "anchor #$name resolves to a ### heading on $page" \
      "^### \`/${name}[ \`]" "$(cat "$resolved")"
    sect="$(section_text "$resolved" "$name" | norm)"
    nfirst="$(printf '%s' "$first" | norm)"
    assert_contains "home-page description for $name matches SKILL.md" \
      "$sect" "$nfirst"
    assert_contains "index gloss for $name matches SKILL.md" \
      "$IDX_NORM" "$nfirst"
  else
    assert_eq "deep-link target page for $name exists ($page)" "found" "missing"
  fi
done <"$FACTS"

echo ""
echo "=== Negative self-test (assertions are live, not vacuous) ==="
# A clean index passes membership; a mutated one (one invokable dropped, one
# internal token injected) must fail — proving the checker isn't green-only.
BROKEN="$WORK_DIR/broken-index.md"
# First word of each space-prefixed name list (parameter expansion, no
# word-splitting): strip the leading space, then take up to the next space.
drop_tmp="${INVOKABLE_NAMES# }"
DROP_NAME="${drop_tmp%% *}"
leak_tmp="${INTERNAL_NAMES# }"
LEAK_NAME="${leak_tmp%% *}"
if [ -f "$INDEX" ]; then
  grep -vE "/${DROP_NAME}${NAME_BOUNDARY}" "$INDEX" >"$BROKEN" || true
else
  : >"$BROKEN"
fi
# shellcheck disable=SC2016 # literal backticks are intentional markdown, not command substitution
printf -- '- [`/%s`](review-system.md#%s) — leaked.\n' \
  "$LEAK_NAME" "$LEAK_NAME" >>"$BROKEN"
if check_membership "$BROKEN"; then
  assert_eq "mutated index is reported FAIL by check_membership" "fail" "pass"
else
  assert_eq "mutated index is reported FAIL by check_membership" "fail" "fail"
fi

test_summary
