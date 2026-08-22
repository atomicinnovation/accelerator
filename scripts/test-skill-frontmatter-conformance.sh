#!/usr/bin/env bash
set -euo pipefail

# Producer-conformance guard for work item 0103.
#
# For each frontmatter-emitting SKILL.md this guard:
#   1. extracts the verbatim frontmatter literals the skill hard-codes
#      (type, status, producer, schema_version), keyed by (skill, type);
#   2. derives the type's enforced attribute set from the contract files
#      (templates-schema.tsv ∪ frontmatter-emission-rules.sh) and asserts the
#      composed emission (skill literals ∪ loaded-template keys) covers it;
#   3. synthesises a complete fixture (the shared emit_valid, pinned to the
#      skill's extracted status literal) and runs the REAL corpus validator over
#      it, asserting it passes.
#
# Provenance over-emission (revision/repository on a non-anchored type) and
# bare/unquoted typed-linkage are enforced directly by the corpus validator
# (work item 0105), so this guard no longer re-derives them by inspection; the
# conditional-axis section below exercises both through the real validator.
#
# Status-transition mutators (validate-plan -> plan, review-adr -> adr) are
# asserted on the status axis only: the documented target status must be a
# member of the TARGET type's status_vocab. Every review-adr target — including
# `rejected` (the proposed -> rejected transition ADR-0031 adopts) — is asserted
# uniformly on this axis.
#
# A negative self-test mutates each synthesised fixture (one mutation per axis)
# and asserts rejection with the specific diagnostic, proving the guard is wired
# rather than green-path-only. Count-gated reconciliation asserts the producer
# set cannot silently grow or shrink.
#
# Contract is SOURCED, never re-encoded: frontmatter-emission-rules.sh (the
# cross-cutting sets) + templates-schema.tsv (the per-type facts).
#
# bash 3.2-safe (no associative arrays / bash-4 constructs); LC_ALL=C so the
# `←` (U+2190) glyph in the substitute-list grammar is treated as opaque bytes
# identically under BSD and GNU tooling.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=test-helpers.sh
source "$SCRIPT_DIR/test-helpers.sh"
# shellcheck source=frontmatter-emission-rules.sh
source "$SCRIPT_DIR/frontmatter-emission-rules.sh"

export LC_ALL=C

# shellcheck disable=SC2034  # consumed by frontmatter-fixtures.sh (sourced below)
CORPUS_BIN="${ACCELERATOR_BIN:-$ROOT/bin/accelerator}"
# shellcheck source=frontmatter-fixtures.sh
source "$SCRIPT_DIR/frontmatter-fixtures.sh"

SCHEMA_TSV="$SCRIPT_DIR/templates-schema.tsv"
TEMPLATES_DIR="$ROOT/templates"
cd "$ROOT"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# ---- Contract: parse templates-schema.tsv into parallel arrays --------------
SCHEMA_TMPL=()
SCHEMA_TYPES=()
SCHEMA_ANCHORED=()
SCHEMA_EXTRAS=()
SCHEMA_STATUS=()
SCHEMA_FORBIDDEN=()
SCHEMA_LINKKEYS=()
while IFS=$'\t' read -r tmpl type anchored extras vocab forbidden linkkeys; do
  SCHEMA_TMPL+=("$tmpl")
  SCHEMA_TYPES+=("$type")
  SCHEMA_ANCHORED+=("$anchored")
  SCHEMA_EXTRAS+=("$extras")
  SCHEMA_STATUS+=("$vocab")
  SCHEMA_FORBIDDEN+=("$forbidden")
  SCHEMA_LINKKEYS+=("$linkkeys")
done < <(tail -n +2 "$SCHEMA_TSV")

schema_index() { # $1 type -> index or ""
  local needle="$1" i
  for ((i = 0; i < ${#SCHEMA_TYPES[@]}; i++)); do
    [ "${SCHEMA_TYPES[$i]}" = "$needle" ] && {
      printf '%s' "$i"
      return 0
    }
  done
  return 0
}

# ---- Producer set -----------------------------------------------------------
EMITTERS=(
  skills/work/create-work-item/SKILL.md
  skills/work/extract-work-items/SKILL.md
  skills/work/refine-work-item/SKILL.md
  skills/work/review-work-item/SKILL.md
  skills/planning/create-plan/SKILL.md
  skills/planning/review-plan/SKILL.md
  skills/planning/validate-plan/SKILL.md
  skills/decisions/create-adr/SKILL.md
  skills/decisions/extract-adrs/SKILL.md
  skills/research/research-codebase/SKILL.md
  skills/research/research-issue/SKILL.md
  skills/design/inventory-design/SKILL.md
  skills/design/analyse-design-gaps/SKILL.md
  skills/github/describe-pr/SKILL.md
  skills/github/review-pr/SKILL.md
  skills/notes/create-note/SKILL.md
)
# Surfaced by the discovery grep but out of scope: migrate is a corpus
# transformer with no full-block emission.
EXCLUDED=(
  skills/config/migrate/SKILL.md
)
# Status-transition mutators: not surfaced by the discovery grep (no full-block
# marker reaches them); tracked by hand, asserted on the status axis only.
STATUS_AXIS=(skills/planning/validate-plan/SKILL.md skills/decisions/review-adr/SKILL.md)

DISCOVERY_RE='schema_version:|Populate frontmatter|Substitute .*frontmatter|frontmatter-emission|artifact-derive-metadata\.sh'

# ---- Literal extraction -----------------------------------------------------
# Substitute-list grammar (a):  - `<field>:` ← `<value>`   (any indentation;
# optional trailing parenthetical). Captures the value between the SECOND
# backtick pair. Pure parameter expansion — no GNU/BSD-divergent flags, and the
# `←` glyph is never adjacent to a metacharacter (opaque bytes under LC_ALL=C).
extract_literal() { # $1 file  $2 field -> verbatim value or ""
  local file="$1" field="$2" line rest val
  line=$(grep -E "^[[:space:]]*-[[:space:]]*\`${field}:\`" "$file" | head -1) || true
  if [ -z "$line" ]; then
    extract_cli_literal "$file" "$field"
    return
  fi
  rest="${line#*\`"${field}":\`}" # drop through the field token's closing backtick
  rest="${rest#*\`}"              # drop through the next opening backtick
  val="${rest%%\`*}"              # capture up to the next backtick
  printf '%s' "$val"
}

# Substitute-list grammar (b): a skill that writes via a dispatched
# `accelerator work create`/`accelerator work update` invocation carries no
# bulleted `- \`field:\` ← \`value\`` list at all — the compiled binary's own
# (separately tested) frontmatter-composition logic owns these literals.
extract_cli_literal() { # $1 file  $2 field -> verbatim value or ""
  local file="$1" field="$2"
  grep -qE 'accelerator work (create|update)' "$file" || return 0
  case "$field" in
    type) printf '%s' "work-item" ;;
    schema_version) printf '%s' "1" ;;
    status | producer)
      awk -v flag="$field" '
      /^[[:space:]]*```/ { in_block = !in_block; next }
      in_block && $0 ~ ("--" flag "[[:space:]]") {
        line = $0
        sub(".*--" flag "[[:space:]]+", "", line)
        gsub(/^"|\\$/, "", line)
        sub(/[[:space:]].*/, "", line)
        gsub(/"/, "", line)
        print line
        exit
      }
    ' "$file"
      ;;
  esac
}

# validate-plan -> plan: target status lives in prose: "status` field to `done`".
extract_validate_plan_plan_status() {
  local line rest
  line=$(grep -E "status\` field to \`" skills/planning/validate-plan/SKILL.md | head -1) || true
  [ -n "$line" ] || return 0
  rest="${line#*field to \`}"
  printf '%s' "${rest%%\`*}"
}

# review-adr -> adr: target statuses live in the "Change `status: X` to
# `status: Y`" prose; emit the set of Y targets.
extract_review_adr_targets() {
  grep -oE "to \`status: [a-z]+\`" skills/decisions/review-adr/SKILL.md |
    sed -E "s/.*status: ([a-z]+)\`.*/\1/" | sort -u
}

# ---- Small helpers ----------------------------------------------------------
in_list() {
  local needle="$1"
  shift
  local x
  for x in "$@"; do [ "$x" = "$needle" ] && return 0; done
  return 1
}

status_in_vocab() { # $1 status  $2 vocab(pipe-joined) -> rc
  local s="$1" vocab="$2" tok oldifs="$IFS"
  IFS='|'
  for tok in $vocab; do
    tok="${tok#"${tok%%[![:space:]]*}"}"
    tok="${tok%"${tok##*[![:space:]]}"}"
    [ "$tok" = "$s" ] && {
      IFS="$oldifs"
      return 0
    }
  done
  IFS="$oldifs"
  return 1
}

template_keys() { # $1 template-file -> space-separated frontmatter keys
  awk 'BEGIN{n=0}
       /^---[[:space:]]*$/ {n++; if(n==2) exit; next}
       n==1 && /^[A-Za-z_][A-Za-z0-9_]*:/ {k=$0; sub(/:.*/,"",k); print k}' "$1"
}

# A pass/fail wrapper around a check function returning rc.
assert_check() { # $1 name $2 expected_rc; remaining = command
  local name="$1" exprc="$2"
  shift 2
  local rc=0
  "$@" || rc=$?
  if [ "$rc" -eq "$exprc" ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name (rc=$rc, expected $exprc)"
    FAIL=$((FAIL + 1))
  fi
}

assert_true() { # $1 name; remaining = test command
  local name="$1"
  shift
  if "$@"; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    FAIL=$((FAIL + 1))
  fi
}

# =============================================================================
echo "=== Producer-set reconciliation (liveness gate) ==="
discovered=$(grep -rlE "$DISCOVERY_RE" skills --include='SKILL.md' | sort -u)
disc_count=$(printf '%s\n' "$discovered" | wc -l | tr -d '[:space:]')
assert_eq "discovery returns 17 producing SKILL.md files" "17" "$disc_count"
assert_eq "EMITTERS array holds 16 full-block emitters" "16" "${#EMITTERS[@]}"
allowlist=$(printf '%s\n' "${EMITTERS[@]}" "${EXCLUDED[@]}" | sort -u)
unexpected=$(comm -23 <(printf '%s\n' "$discovered") <(printf '%s\n' "$allowlist"))
assert_empty "every discovered file is in EMITTERS ∪ EXCLUDED" "$unexpected"
for s in "${STATUS_AXIS[@]}"; do
  assert_true "status-axis mutator present on disk: $s" test -f "$s"
done

# =============================================================================
echo "=== Per-(skill, type) conformance: full-block emitters ==="
processed=0
for skill in "${EMITTERS[@]}"; do
  type="$(extract_literal "$skill" type)"
  status_lit="$(extract_literal "$skill" status)"
  producer_lit="$(extract_literal "$skill" producer)"
  sv_lit="$(extract_literal "$skill" schema_version)"

  # Liveness: every claimed extraction must be non-empty (formatting-drift guard).
  if [ -z "$type" ] || [ -z "$status_lit" ] || [ -z "$producer_lit" ] || [ -z "$sv_lit" ]; then
    echo "  FAIL: $skill — empty literal extraction (type='$type' status='$status_lit' producer='$producer_lit' schema_version='$sv_lit')"
    FAIL=$((FAIL + 1))
    continue
  fi

  idx="$(schema_index "$type")"
  if [ -z "$idx" ]; then
    echo "  FAIL: $skill — extracted type '$type' is not a schema type"
    FAIL=$((FAIL + 1))
    continue
  fi
  anchored="${SCHEMA_ANCHORED[$idx]}"
  extras="${SCHEMA_EXTRAS[$idx]}"
  vocab="${SCHEMA_STATUS[$idx]}"
  linkkeys="${SCHEMA_LINKKEYS[$idx]}"
  tmpl_file="$TEMPLATES_DIR/${SCHEMA_TMPL[$idx]}"

  label="${skill#skills/}"
  label="${label%/SKILL.md}"

  # schema_version literal is the bare integer 1.
  assert_eq "$label ($type): schema_version literal is 1" "1" "$sv_lit"

  # status literal ∈ the type's vocab.
  assert_check "$label ($type): status literal '$status_lit' ∈ vocab" 0 status_in_vocab "$status_lit" "$vocab"

  # Composed-completeness: the contract-enforced attribute set ⊆ composed
  # emission (loaded-template keys ∪ the four extracted literal keys). A
  # template that silently drops a required slot fails here.
  tkeys=$(template_keys "$tmpl_file")
  # shellcheck disable=SC2086
  set -- $tkeys type status producer schema_version
  covered="$*"
  missing=""
  enforced="${FM_BASE_FIELDS[*]} status $linkkeys"
  for e in $extras; do
    case " $FM_OPTIONAL_EXTRAS " in *" $e "*) ;; *) enforced="$enforced $e" ;; esac
  done
  [ "$anchored" = "yes" ] && enforced="$enforced ${FM_PROVENANCE_FIELDS[*]}"
  for a in $enforced; do
    case " $covered " in *" $a "*) ;; *) missing="$missing $a" ;; esac
  done
  assert_empty "$label ($type): composed emission covers enforced set" "$missing"

  # Composed-acceptance: synthesise a fixture pinned to the skill's status
  # literal and run the REAL validator over it.
  fx="$TMP/accept-$type.md"
  emit_valid "$type" "$anchored" "$extras" "$status_lit" "$fx"
  assert_accepts "$label ($type): composed fixture accepted by validator" "$fx"

  processed=$((processed + 1))
done
assert_eq "all 16 full-block emitters processed" "16" "$processed"

# =============================================================================
echo "=== Status-axis mutators ==="
# validate-plan -> plan: a passing plan's status must be a plan-vocab member.
PLAN_IDX="$(schema_index plan)"
PLAN_VOCAB="${SCHEMA_STATUS[$PLAN_IDX]}"
vp_plan_status="$(extract_validate_plan_plan_status)"
assert_true "validate-plan -> plan: status literal extracted (non-empty)" test -n "$vp_plan_status"
assert_check "validate-plan -> plan: status '$vp_plan_status' ∈ plan vocab" 0 \
  status_in_vocab "$vp_plan_status" "$PLAN_VOCAB"
vp_fx="$TMP/vp-plan.md"
emit_valid plan yes reviewer "$vp_plan_status" "$vp_fx"
assert_accepts "validate-plan -> plan: status fixture accepted" "$vp_fx"

# review-adr -> adr: each documented target status must be an adr-vocab member.
ADR_IDX="$(schema_index adr)"
ADR_VOCAB="${SCHEMA_STATUS[$ADR_IDX]}"
adr_targets="$(extract_review_adr_targets)"
assert_true "review-adr -> adr: target statuses extracted (non-empty)" test -n "$adr_targets"
for tgt in $adr_targets; do
  assert_check "review-adr -> adr: status '$tgt' ∈ adr vocab" 0 status_in_vocab "$tgt" "$ADR_VOCAB"
  adr_fx="$TMP/adr-$tgt.md"
  emit_valid adr no decision_makers "$tgt" "$adr_fx"
  assert_accepts "review-adr -> adr: status '$tgt' fixture accepted" "$adr_fx"
done

# =============================================================================
echo "=== Conditional-axis coverage (both branches, per AC4) ==="
# Provenance: present (anchored) accepts; absent (non-anchored) accepts;
# anchored-missing rejects.
emit_valid plan yes reviewer "draft" "$TMP/prov-present.md"
assert_accepts "provenance present (anchored plan) accepted" "$TMP/prov-present.md"
emit_valid work-item no "kind priority external_id" "draft" "$TMP/prov-absent.md"
assert_accepts "provenance absent (non-anchored work-item) accepted" "$TMP/prov-absent.md"
emit_valid plan yes reviewer "draft" "$TMP/prov-missing.md"
sed '/^revision: /d; /^repository: /d' "$TMP/prov-missing.md" >"$TMP/prov-missing2.md"
assert_rejects "provenance missing on anchored type rejected" "MISSING-PROVENANCE" "$TMP/prov-missing2.md"
# Over-emission: a non-anchored type carrying provenance rejects (the reverse
# direction folded into the validator under work item 0105). Pairs with the
# prov-absent accept above (non-anchored-with-no-provenance).
emit_valid work-item no "kind priority external_id" "draft" "$TMP/prov-overemit.md" \
  $'revision: "x"\nrepository: "y"'
assert_rejects "provenance on non-anchored type rejected" \
  "PROVENANCE-ON-NONANCHORED" "$TMP/prov-overemit.md"

# Linkage: present (quoted typed ref) accepts; absent accepts; bare rejects.
emit_valid work-item no "kind priority external_id" "draft" "$TMP/link-present.md" 'parent: "work-item:0001"'
assert_accepts "typed-linkage present (quoted) accepted" "$TMP/link-present.md"
emit_valid work-item no "kind priority external_id" "draft" "$TMP/link-absent.md"
assert_accepts "typed-linkage absent accepted" "$TMP/link-absent.md"
emit_valid work-item no "kind priority external_id" "draft" "$TMP/link-bare.md" 'parent: 0042'
assert_rejects "bare (unquoted) linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/link-bare.md"

# Omit-when-empty: present-and-valid accepts; absent accepts; empty rejects.
emit_valid work-item no "kind priority external_id" "draft" "$TMP/owe-present.md" 'external_id: "JIRA-1"'
assert_accepts "omit-when-empty key present-and-valid accepted" "$TMP/owe-present.md"
emit_valid work-item no "kind priority external_id" "draft" "$TMP/owe-absent.md"
assert_accepts "omit-when-empty key absent accepted" "$TMP/owe-absent.md"
emit_valid work-item no "kind priority external_id" "draft" "$TMP/owe-empty.md" 'external_id: ""'
assert_rejects "EMPTY-PLACEHOLDER (non-tags key emitted \"\") rejected" "EMPTY-PLACEHOLDER" "$TMP/owe-empty.md"

# =============================================================================
echo "=== Negative self-test: per-axis fixture mutation (wiring proof) ==="
# Mutate one axis at a time; assert the mutation is not a no-op, then assert the
# validator rejects with the specific diagnostic. A green-path-only guard would
# pass these vacuously.
BASE="$TMP/neg-base.md"
emit_valid work-item no "kind priority external_id" "draft | ready" "$BASE"
mut_n=0
assert_axis_mutation() { # $1 desc $2 code $3 sed_expr
  local desc="$1" code="$2" expr="$3"
  mut_n=$((mut_n + 1))
  local out="$TMP/neg-mut-$mut_n.md"
  sed "$expr" "$BASE" >"$out"
  if cmp -s "$BASE" "$out"; then
    echo "  FAIL: $desc — mutation was a no-op"
    FAIL=$((FAIL + 1))
    return
  fi
  assert_rejects "$desc" "$code" "$out"
}
assert_axis_mutation "axis=type    -> INVALID-TYPE" "INVALID-TYPE" 's/^type: work-item$/type: nonsense/'
assert_axis_mutation "axis=status  -> BAD-STATUS" "BAD-STATUS" 's/^status: .*/status: bogus/'
assert_axis_mutation "axis=extra   -> MISSING-EXTRA" "MISSING-EXTRA" '/^kind: /d'
assert_axis_mutation "axis=schema_version -> BAD-SCHEMA-VERSION" "BAD-SCHEMA-VERSION" 's/^schema_version: 1$/schema_version: "1"/'

# =============================================================================
echo "=== No re-encoded contract ==="
assert_true "guard sources frontmatter-emission-rules.sh" \
  grep -qF 'frontmatter-emission-rules.sh' "$SCRIPT_DIR/test-skill-frontmatter-conformance.sh"
assert_true "guard reads templates-schema.tsv" \
  grep -qF 'templates-schema.tsv' "$SCRIPT_DIR/test-skill-frontmatter-conformance.sh"

# =============================================================================
# Design skill, agent and docs structure
# =============================================================================

PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INIT="$PLUGIN_ROOT/skills/config/init/SKILL.md"
CONFIGURE="$PLUGIN_ROOT/skills/config/configure/SKILL.md"
DOCS_INTERNALS="$PLUGIN_ROOT/docs-site/src/content/docs/internals.md"
DOCS_CONFIGURATION="$PLUGIN_ROOT/docs-site/src/content/docs/configuration.md"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Foundation: init SKILL.md ==="

assert_contains "init lists research_design_inventories path key" \
  "$(cat "$INIT")" "research_design_inventories"
assert_contains "init lists research_design_gaps path key" \
  "$(cat "$INIT")" "research_design_gaps"
# Derive the expected count from the Path Resolution list rather than
# hardcoding it (the marker's correctness is invariant-tested in
# test-config.sh; here we just need a value that auto-tracks the file).
EXPECTED_DIR_COUNT=$(grep -cE '^\*\*[A-Za-z][^*]* directory\*\*:' "$INIT")
assert_contains "init declares directory count via marker" \
  "$(cat "$INIT")" "<!-- DIR_COUNT:${EXPECTED_DIR_COUNT} -->"
assert_contains "init summary lists design inventories directory" \
  "$(cat "$INIT")" "{design inventories directory}"
assert_contains "init summary lists design gaps directory" \
  "$(cat "$INIT")" "{design gaps directory}"

echo ""

echo "=== Foundation: configure SKILL.md ==="

assert_contains "configure paths table includes research_design_inventories" \
  "$(cat "$CONFIGURE")" "research_design_inventories"
assert_contains "configure paths table includes research_design_gaps" \
  "$(cat "$CONFIGURE")" "research_design_gaps"

echo ""

echo "=== Design key call sites use canonical research_design_* form ==="
assert_exit_code "no SKILL.md or agent uses bare design_(inventories|gaps)" 1 \
  bash -c "grep -rE --exclude-dir=node_modules --exclude-dir=target 'config path design_(inventories|gaps)\\b' \"$PLUGIN_ROOT/skills\" \"$PLUGIN_ROOT/agents\""

echo ""

echo "=== Foundation: docs ==="

assert_contains "internals meta/ table lists design-inventories/" \
  "$(cat "$DOCS_INTERNALS")" "design-inventories/"
assert_contains "internals meta/ table lists design-gaps/" \
  "$(cat "$DOCS_INTERNALS")" "design-gaps/"
assert_contains "configuration template keys include design-inventory" \
  "$(cat "$DOCS_CONFIGURATION")" "design-inventory"
assert_contains "configuration template keys include design-gap" \
  "$(cat "$DOCS_CONFIGURATION")" "design-gap"

echo ""

echo "=== Browser agents ==="

LOC="$PLUGIN_ROOT/agents/browser-locator.md"
ANA="$PLUGIN_ROOT/agents/browser-analyser.md"

assert_file_exists "browser-locator.md exists" "$LOC"
assert_file_exists "browser-analyser.md exists" "$ANA"

# Extract the tools: field from YAML frontmatter, sort items, join with comma.
# Handles both single-line "tools: a, b, c" and wrapped continuation lines.
# Strips leading whitespace (from YAML block-scalar continuation lines).
extract_tools() {
  local file="$1"
  # Extract text between first and second --- (the frontmatter)
  # Find the tools: line, then collect it plus any continuation lines
  awk '
    /^---/ { fm++; next }
    fm == 1 && /^tools:/ { line = $0; in_tools = 1; next }
    fm == 1 && in_tools && /^  / { line = line " " $0; next }
    fm == 1 && in_tools { in_tools = 0 }
    fm == 2 { exit }
    END { print line }
  ' "$file" |
    sed 's/^tools:[[:space:]]*//' |
    sed 's/^>[[:space:]]*//' |
    tr ',' '\n' |
    sed 's/^[[:space:]]*//' |
    sed 's/[[:space:]]*$//' |
    grep -v '^$' |
    sort |
    tr '\n' ',' |
    sed 's/,$//'
}

LOC_TOOLS="$(extract_tools "$LOC")"
ANA_TOOLS="$(extract_tools "$ANA")"

assert_eq "browser-locator declares exactly Bash as its tool" \
  "Bash" \
  "$LOC_TOOLS"
assert_eq "browser-analyser declares exactly Bash as its tool" \
  "Bash" \
  "$ANA_TOOLS"
assert_not_contains "browser-locator declares no mcp__playwright__ tools" \
  "$LOC_TOOLS" "mcp__playwright__"
assert_not_contains "browser-analyser declares no mcp__playwright__ tools" \
  "$ANA_TOOLS" "mcp__playwright__"

echo ""

echo "=== executor evaluate payload allowlist ==="

ANA_BODY="$(cat "$ANA")"
for forbidden in "fetch" "XMLHttpRequest" "document.cookie" \
  "localStorage" "sessionStorage" "indexedDB" \
  "eval" "innerHTML" "window.open"; do
  assert_contains "browser-analyser body forbids $forbidden in executor evaluate" \
    "$ANA_BODY" "$forbidden"
done

echo ""

echo "=== .mcp.json ==="

assert_exit_code ".claude-plugin/.mcp.json does not exist (MCP path removed)" 1 \
  test -e "$PLUGIN_ROOT/.claude-plugin/.mcp.json"

echo ""

echo "=== inventory-design: skill structure ==="

SKILL="$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md"
assert_file_exists "inventory-design SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: inventory-design"
assert_contains "argument-hint declares positional source-id and location" \
  "$(cat "$SKILL")" 'argument-hint: "[source-id] [location]'
assert_contains "disable-model-invocation true" \
  "$(cat "$SKILL")" "disable-model-invocation: true"
assert_contains "argument-hint includes --allow-internal flag" \
  "$(cat "$SKILL")" "--allow-internal"
assert_contains "argument-hint includes --allow-insecure-scheme flag" \
  "$(cat "$SKILL")" "--allow-insecure-scheme"
assert_not_contains "allowed-tools contains no mcp__playwright__ entries" \
  "$(cat "$SKILL")" "mcp__playwright__"
assert_not_contains "allowed-tools grants no inventory-design scripts glob" \
  "$(cat "$SKILL")" "/skills/design/inventory-design/scripts/"
assert_contains "loads config context" \
  "$(cat "$SKILL")" "accelerator config context"
assert_contains "loads agent names" \
  "$(cat "$SKILL")" "accelerator config agents"
assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "accelerator config instructions inventory-design"
assert_contains "Agent Names defaults include browser-locator" \
  "$(cat "$SKILL")" "accelerator:browser-locator"
assert_contains "Agent Names defaults include browser-analyser" \
  "$(cat "$SKILL")" "accelerator:browser-analyser"

echo ""

echo "=== inventory-design: evals ==="

EVALS="$PLUGIN_ROOT/skills/design/inventory-design/evals/evals.json"
BENCH="$PLUGIN_ROOT/skills/design/inventory-design/evals/benchmark.json"
assert_file_exists "evals.json exists" "$EVALS"
assert_file_exists "benchmark.json exists" "$BENCH"
assert_eq "evals.json is valid JSON" "$(jq empty "$EVALS" 2>&1)" ""
assert_eq "benchmark.json is valid JSON" "$(jq empty "$BENCH" 2>&1)" ""

echo ""

echo "=== analyse-design-gaps: skill structure ==="

SKILL="$PLUGIN_ROOT/skills/design/analyse-design-gaps/SKILL.md"
assert_file_exists "analyse-design-gaps SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: analyse-design-gaps"
assert_contains "argument-hint two positional ids" \
  "$(cat "$SKILL")" 'argument-hint: "[current-source-id] [target-source-id]"'
assert_contains "instructs cue-phrase prose" \
  "$(cat "$SKILL")" "we need"
assert_contains "skill body invokes the cue-phrase audit subcommand" \
  "$(cat "$SKILL")" "accelerator design audit-cue-phrases"

echo ""

assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "accelerator config instructions analyse-design-gaps"
echo "=== analyse-design-gaps: evals ==="

EVALS="$PLUGIN_ROOT/skills/design/analyse-design-gaps/evals/evals.json"
BENCH="$PLUGIN_ROOT/skills/design/analyse-design-gaps/evals/benchmark.json"
assert_file_exists "evals.json exists" "$EVALS"
assert_file_exists "benchmark.json exists" "$BENCH"
assert_eq "evals.json is valid JSON" "$(jq empty "$EVALS" 2>&1)" ""
assert_eq "benchmark.json is valid JSON" "$(jq empty "$BENCH" 2>&1)" ""

echo ""

echo "=== browser-locator links contract ==="
assert_contains "browser-locator body documents the links command" \
  "$(cat "$LOC")" "accelerator design executor links"
assert_contains "browser-locator body uses pathname as route identifier" \
  "$(cat "$LOC")" "Use \`pathname\`"
assert_contains "browser-locator body restricts route names to links output" \
  "$(cat "$LOC")" "Routes come from"
assert_contains "browser-locator body requires same_origin filter" \
  "$(cat "$LOC")" "same_origin: true"
assert_not_contains "browser-analyser body does NOT advertise the links command" \
  "$(cat "$ANA")" "accelerator design executor links"

echo ""

# =============================================================================
# Design script references resolve
# =============================================================================
# A call site left pointing at a deleted script fails here, at merge, rather
# than at run time — where the symptom was a headless Chromium held until its
# idle timeout, with nothing to attribute it to. The migration that emptied
# these directories moved three call sites and missed two.

echo "=== Design script references resolve ==="

DESIGN_REFERRERS="$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md
$PLUGIN_ROOT/skills/design/analyse-design-gaps/SKILL.md
$PLUGIN_ROOT/agents/browser-locator.md
$PLUGIN_ROOT/agents/browser-analyser.md"

# Call sites only. `Bash(...)` lines are permission grants, checked against
# their call sites below rather than against the filesystem, and a path ending
# in `/` is a grant's glob prefix rather than a file anyone invokes.
design_script_paths() {
  grep -v 'Bash(' "$1" |
    grep -oE '\$\{CLAUDE_PLUGIN_ROOT\}/skills/design/[A-Za-z0-9_./-]+' |
    grep '/scripts/' |
    grep -v '/$' |
    sort -u
}

# The path a grant or call site names, with the plugin-root placeholder removed
# so a failure message reads as a repository path. Prefix removal rather than
# `${var//}`: the pattern carries both braces and slashes, which is where the
# bash 3.2 floor has bitten before.
without_plugin_root() {
  printf '%s\n' "${1#\$\{CLAUDE_PLUGIN_ROOT\}/}"
}

while IFS= read -r referrer; do
  [ -f "$referrer" ] || continue
  for reference in $(design_script_paths "$referrer"); do
    relative="$(without_plugin_root "$reference")"
    assert_file_exists "$(basename "$referrer") names an existing $relative" \
      "$PLUGIN_ROOT/$relative"
  done
done <<EOF
$DESIGN_REFERRERS
EOF

echo ""

# =============================================================================
# Design script grants have call sites
# =============================================================================
# The check above cannot see this class: a grant whose directory still exists
# but whose scripts have all been migrated away reads as valid while handing
# the model Bash access to a tree the skill no longer invokes.

echo "=== Design script grants have call sites ==="

# Everything after the closing frontmatter delimiter.
skill_body() {
  awk 'BEGIN { d = 0 } /^---$/ && d < 2 { d++; next } d >= 2' "$1"
}

# The literal prefix a grant authorises, i.e. the rule without its trailing
# glob, so `.../scripts/*` is satisfied by any invocation under `.../scripts/`.
granted_script_prefixes() {
  sed -n '1,/^---$/p' "$1" |
    grep -oE 'Bash\(\$\{CLAUDE_PLUGIN_ROOT\}/skills/[A-Za-z0-9_./-]+\*?\)' |
    sed -e 's|^Bash(||' -e 's|)$||' -e 's|\*$||' |
    grep '/scripts/' |
    sort -u
}

for skill in "$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md" \
  "$PLUGIN_ROOT/skills/design/analyse-design-gaps/SKILL.md"; do
  for prefix in $(granted_script_prefixes "$skill"); do
    assert_contains \
      "$(basename "$(dirname "$skill")") invokes what it grants: $(without_plugin_root "$prefix")" \
      "$(skill_body "$skill")" "$prefix"
  done
done

echo ""

# =============================================================================
# Design eval and protocol references resolve
# =============================================================================
# The JSON parsing alone never checked that a script or reason a design eval or
# the protocol names still exists, so references to deleted scripts and retired
# downgrade reasons could rot undetected. This guard closes that gap: every *.sh
# a design eval or the protocol names must resolve to a script still on disk,
# and no reason it names may have been retired from the executor's vocabulary.

echo "=== Design eval and protocol references resolve ==="

DESIGN_REFERENCE_FILES="$PLUGIN_ROOT/skills/design/inventory-design/evals/evals.json
$PLUGIN_ROOT/skills/design/inventory-design/evals/benchmark.json
$PLUGIN_ROOT/skills/design/inventory-design/PROTOCOL.md"

# The live downgrade vocabulary, read from its single source of truth (the
# key() match arms), so a reason dropped from the enum can no longer be named by
# a doc or eval that CI still passes.
DOWNGRADE_ENUM="$PLUGIN_ROOT/cli/design/src/runtime/downgrade.rs"
live_downgrade_reasons() {
  grep -oE '=> "[a-z][a-z-]*",' "$DOWNGRADE_ENUM" |
    sed -e 's|^=> "||' -e 's|",$||' |
    sort -u
}

# Append-only: every reason that has ever been in the vocabulary. The retired
# set is derived below as those no longer live, so a reason dropped from the
# enum in a future change is caught without touching this guard's logic.
REASONS_EVER="unsupported-platform loader-unresolvable glibc-too-old
runtime-libraries-missing artifact-unavailable materialisation-in-progress
executor-ping-failed cache-unwritable disk-floor-not-met
node-missing node-too-old bootstrap-failed"

LIVE_REASONS="$(live_downgrade_reasons)"

for reason in $LIVE_REASONS; do
  assert_contains "REASONS_EVER covers live reason $reason (append it)" \
    "$REASONS_EVER" "$reason"
done

while IFS= read -r reference_file; do
  [ -f "$reference_file" ] || continue
  reference_body="$(cat "$reference_file")"
  # Every *.sh a design eval or the protocol names must resolve to a script
  # still on disk; once the runtime is vendored none survives under
  # skills/design/, so a stale reference is caught here rather than at run time.
  for script in $(printf '%s\n' "$reference_body" |
    grep -oE '[A-Za-z0-9_-]+\.sh' | sort -u); do
    found="$(find "$PLUGIN_ROOT/skills/design" -name "$script" -type f 2>/dev/null |
      head -n 1)"
    assert_true \
      "$(basename "$reference_file") reference to $script resolves under skills/design" \
      test -n "$found"
  done
  # No reason it names may have been retired from the executor's vocabulary.
  for reason in $REASONS_EVER; do
    printf '%s\n' "$LIVE_REASONS" | grep -qx "$reason" && continue
    assert_not_contains \
      "$(basename "$reference_file") names no retired downgrade reason ($reason)" \
      "$reference_body" "$reason"
  done
done <<EOF
$DESIGN_REFERENCE_FILES
EOF

echo ""

test_summary
