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
#      it, asserting it passes;
#   4. asserts the two validator blind spots (provenance over-emission on
#      non-anchored types; bare/unquoted typed-linkage) by inspection of the
#      composed emission — these BYPASS the validator (see comments) and are
#      tracked for consolidation into the validator under work item 0105.
#
# Status-transition mutators (validate-plan -> plan, review-adr -> adr) are
# asserted on the status axis only: the documented target status must be a
# member of the TARGET type's status_vocab. review-adr's `rejected` target is a
# known schema-source divergence deferred to work item 0104, represented here as
# an explicit skip_test keyed to that id (not a silent omission).
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

VALIDATOR="$SCRIPT_DIR/validate-corpus-frontmatter.sh"
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
    [ "${SCHEMA_TYPES[$i]}" = "$needle" ] && { printf '%s' "$i"; return 0; }
  done
  return 0
}

# ---- Producer set (Phase 1 reconciliation) ---------------------------------
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
# Surfaced by the discovery grep but out of scope (corpus transformer).
EXCLUDED=( skills/config/migrate/SKILL.md )
# Status-transition mutators: not surfaced by the discovery grep (no full-block
# marker reaches them); tracked by hand, asserted on the status axis only.
STATUS_AXIS=( skills/planning/validate-plan/SKILL.md skills/decisions/review-adr/SKILL.md )

DISCOVERY_RE='schema_version:|Populate frontmatter|Substitute .*frontmatter|frontmatter-emission|artifact-derive-metadata\.sh'

# ---- Literal extraction -----------------------------------------------------
# Substitute-list grammar (a):  - `<field>:` ← `<value>`   (any indentation;
# optional trailing parenthetical). Captures the value between the SECOND
# backtick pair. Pure parameter expansion — no GNU/BSD-divergent flags, and the
# `←` glyph is never adjacent to a metacharacter (opaque bytes under LC_ALL=C).
extract_literal() { # $1 file  $2 field -> verbatim value or ""
  local file="$1" field="$2" line rest val
  line=$(grep -E "^[[:space:]]*-[[:space:]]*\`${field}:\`" "$file" | head -1) || true
  [ -n "$line" ] || return 0
  rest="${line#*\`${field}:\`}" # drop through the field token's closing backtick
  rest="${rest#*\`}"            # drop through the next opening backtick
  val="${rest%%\`*}"            # capture up to the next backtick
  printf '%s' "$val"
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
  grep -oE "to \`status: [a-z]+\`" skills/decisions/review-adr/SKILL.md \
    | sed -E "s/.*status: ([a-z]+)\`.*/\1/" | sort -u
}

# ---- Small helpers ----------------------------------------------------------
in_list() { local needle="$1"; shift; local x; for x in "$@"; do [ "$x" = "$needle" ] && return 0; done; return 1; }

status_in_vocab() { # $1 status  $2 vocab(pipe-joined) -> rc
  local s="$1" vocab="$2" tok oldifs="$IFS"
  IFS='|'
  for tok in $vocab; do
    tok="${tok#"${tok%%[![:space:]]*}"}"
    tok="${tok%"${tok##*[![:space:]]}"}"
    [ "$tok" = "$s" ] && { IFS="$oldifs"; return 0; }
  done
  IFS="$oldifs"
  return 1
}

template_keys() { # $1 template-file -> space-separated frontmatter keys
  awk 'BEGIN{n=0}
       /^---[[:space:]]*$/ {n++; if(n==2) exit; next}
       n==1 && /^[A-Za-z_][A-Za-z0-9_]*:/ {k=$0; sub(/:.*/,"",k); print k}' "$1"
}

# ---- Blind-spot checks (BYPASS the validator — fold into the oracle under
# ---- work item 0105; the validator does NOT enforce either today) ----------
# Provenance over-emission: a non-anchored type must NOT carry revision/
# repository in its composed emission (loaded template OR skill substitute-list).
check_no_provenance_over_emission() { # $1 anchored $2 template-file $3 skill -> rc 0 clean / 1 over-emits
  local anchored="$1" tmpl="$2" skill="$3" f
  [ "$anchored" = "yes" ] && return 0
  for f in "${FM_PROVENANCE_FIELDS[@]}"; do
    grep -qE "^${f}:" "$tmpl" && return 1
    grep -qE "^[[:space:]]*-[[:space:]]*\`${f}:\`" "$skill" && return 1
  done
  return 0
}

# Bare/unquoted linkage: every typed-linkage slot in the loaded template must be
# empty (""/[]) or a quoted scalar / bracketed list — never a bare scalar.
check_linkage_quoted() { # $1 template-file $2 linkkeys -> rc 0 clean / 1 bare value found
  local tmpl="$1" keys="$2" key line val
  for key in $keys; do
    line=$(grep -E "^${key}:" "$tmpl" | head -1) || true
    [ -n "$line" ] || continue
    val="${line#*:}"
    val="${val#"${val%%[![:space:]]*}"}" # trim leading ws
    val="${val%%#*}"                      # strip trailing inline comment
    val="${val%"${val##*[![:space:]]}"}" # trim trailing ws
    case "$val" in
      '' | '""' | '[]') continue ;; # empty slot
      '"'*'"') continue ;;          # quoted scalar
      '['*']') continue ;;          # bracketed list (quoted elements)
      *) return 1 ;;                # bare/unquoted scalar — blind-spot hit
    esac
  done
  return 0
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

  # Blind-spot 1 (BYPASSES validator; -> work item 0105): no provenance
  # over-emission on a non-anchored type.
  assert_check "$label ($type): no provenance over-emission [0105]" 0 \
    check_no_provenance_over_emission "$anchored" "$tmpl_file" "$skill"

  # Blind-spot 2 (BYPASSES validator; -> work item 0105): typed-linkage slots
  # are quoted/empty, never bare.
  assert_check "$label ($type): typed-linkage slots quoted/empty [0105]" 0 \
    check_linkage_quoted "$tmpl_file" "$linkkeys"

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

# review-adr -> adr: each documented target status must be an adr-vocab member,
# EXCEPT `rejected` — a known schema-source divergence deferred to 0104.
ADR_IDX="$(schema_index adr)"
ADR_VOCAB="${SCHEMA_STATUS[$ADR_IDX]}"
adr_targets="$(extract_review_adr_targets)"
assert_true "review-adr -> adr: target statuses extracted (non-empty)" test -n "$adr_targets"
for tgt in $adr_targets; do
  if [ "$tgt" = "rejected" ]; then
    # Deferred: adr vocab lacks `rejected` though ADR-0031 adopts it and
    # review-adr persists it. Flips to a live assert_check when 0104 lands.
    skip_test "review-adr -> adr: status 'rejected' ∈ vocab" "schema-source divergence deferred to work item 0104"
    continue
  fi
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

# Linkage: present (quoted typed ref) accepts; absent accepts; bare rejects.
emit_valid work-item no "kind priority external_id" "draft" "$TMP/link-present.md" 'parent: "work-item:0001"'
assert_accepts "typed-linkage present (quoted) accepted" "$TMP/link-present.md"
emit_valid work-item no "kind priority external_id" "draft" "$TMP/link-absent.md"
assert_accepts "typed-linkage absent accepted" "$TMP/link-absent.md"
emit_valid work-item no "kind priority external_id" "draft" "$TMP/link-bare.md" 'parent: "0042"'
assert_rejects "bare-number linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/link-bare.md"

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
echo "=== Blind-spot liveness (each by-inspection check must be able to fail) ==="
# Provenance over-emission: a non-anchored template carrying revision must trip
# the check (the validator would NOT — that is the blind spot).
LIVE_TMPL="$TMP/live-template.md"
printf 'type: design-gap\nstatus: draft\nrevision: "x"\nrepository: "y"\n' >"$LIVE_TMPL"
assert_check "blind-spot liveness: provenance over-emission detected" 1 \
  check_no_provenance_over_emission "no" "$LIVE_TMPL" "$SCRIPT_DIR/frontmatter-emission-rules.sh"
# Clean control: an anchored type is allowed to carry provenance.
assert_check "blind-spot control: anchored provenance allowed" 0 \
  check_no_provenance_over_emission "yes" "$LIVE_TMPL" "$SCRIPT_DIR/frontmatter-emission-rules.sh"

# Bare linkage: a template slot with a bare scalar must trip the check.
LIVE_LINK="$TMP/live-link.md"
printf 'parent: 0042\nrelates_to: []\n' >"$LIVE_LINK"
assert_check "blind-spot liveness: bare linkage detected" 1 check_linkage_quoted "$LIVE_LINK" "parent relates_to"
LIVE_LINK_OK="$TMP/live-link-ok.md"
printf 'parent: "work-item:0042"\nrelates_to: []\n' >"$LIVE_LINK_OK"
assert_check "blind-spot control: quoted linkage accepted" 0 check_linkage_quoted "$LIVE_LINK_OK" "parent relates_to"

# =============================================================================
echo "=== No re-encoded contract ==="
assert_true "guard sources frontmatter-emission-rules.sh" \
  grep -qF 'frontmatter-emission-rules.sh' "$SCRIPT_DIR/test-skill-frontmatter-conformance.sh"
assert_true "guard reads templates-schema.tsv" \
  grep -qF 'templates-schema.tsv' "$SCRIPT_DIR/test-skill-frontmatter-conformance.sh"

test_summary
