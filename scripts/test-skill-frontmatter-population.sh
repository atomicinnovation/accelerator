#!/usr/bin/env bash
set -euo pipefail

# SKILL-prose population test. For each row in skills-schema.tsv, asserts
# that the consuming SKILL.md instructs the model to populate every
# mandatory unified base field (and provenance fields, when applicable).
#
# The assertion accepts a field as "populated" when its name appears in
# one of two instruction contexts inside the SKILL.md:
#
#   1. Fenced-block context — the field name appears as a YAML key
#      (^<field>:) inside a triple-backtick fenced code block that is NOT
#      a `!`config-read-template.sh ...`` template-inclusion line.
#   2. Imperative-instruction context — the field name appears in a line
#      that contains one of [Ss]ubstitute|[Pp]opulate|[Ss]et|[Ww]rite|[Ee]mit
#      AND that line lies inside a section whose heading matches
#      (persistence|metadata|frontmatter|populate|capture metadata|step \d).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=test-helpers.sh
source "$SCRIPT_DIR/test-helpers.sh"
cd "$ROOT"

# Pin locale so the awk helpers' tolower()/[[:alpha:]]/(fill|omit) matching
# behaves deterministically regardless of host locale (parity with
# test-template-frontmatter.sh).
export LC_ALL=C

echo "=== SKILL.md frontmatter population prose ==="

SKILLS_TSV="$SCRIPT_DIR/skills-schema.tsv"

# Allowlists for the Phase 11 discovery assertion. Together they must cover
# every SKILL.md surfaced by Pass A or Pass B (see Phase 11 of the plan).
IN_SCOPE_PRODUCERS=(
  skills/work/create-work-item/SKILL.md
  skills/work/extract-work-items/SKILL.md
  skills/work/refine-work-item/SKILL.md
  skills/planning/create-plan/SKILL.md
  skills/github/describe-pr/SKILL.md
  skills/decisions/create-adr/SKILL.md
  skills/decisions/extract-adrs/SKILL.md
  skills/research/research-codebase/SKILL.md
  skills/research/research-issue/SKILL.md
  skills/design/inventory-design/SKILL.md
  skills/design/analyse-design-gaps/SKILL.md
  skills/planning/review-plan/SKILL.md
  skills/work/review-work-item/SKILL.md
  skills/github/review-pr/SKILL.md
  skills/planning/validate-plan/SKILL.md
)
NON_EMITTER_TEMPLATE_CONSUMERS=(
  skills/work/update-work-item/SKILL.md
  skills/work/list-work-items/SKILL.md
)

# Self-check: every TSV row (including the header) must have exactly
# four tab-separated fields. Header is row 1 and is skipped by the
# data loop below.
if ! awk -F'\t' 'NF != 4 { print "ERROR: " FILENAME ":" NR " has " NF " fields, expected 4"; bad=1 } END { exit (bad ? 1 : 0) }' "$SKILLS_TSV"; then
  echo "  FAIL: skills-schema.tsv field-count self-check"
  FAIL=$((FAIL + 1))
  test_summary
  exit 1
fi
echo "  PASS: skills-schema.tsv field-count self-check"
PASS=$((PASS + 1))

# Returns the body of the SKILL.md with `!`config-read-template.sh...`` lines
# stripped (those are template inclusion directives, not prose).
strip_template_directives() {
  local file="$1"
  grep -v '^!`.*config-read-template\.sh' "$file"
}

# Check fenced-block context: the field appears as a YAML key inside a
# triple-backtick fenced code block.
in_fenced_block() {
  local file="$1" field="$2"
  awk -v field="$field" '
    /^```/ { in_block = !in_block; next }
    in_block {
      pattern = "^" field ":"
      if ($0 ~ pattern) { found=1; exit }
    }
    END { exit (found ? 0 : 1) }
  ' "$file"
}

# Heading predicate shared by both section detectors below. Defined ONCE so
# the two helpers locate the same sections and a future vocabulary change is
# a one-line edit. It LOCATES a Populate-frontmatter-ish section; it does NOT
# enforce the literal "Populate frontmatter" heading (that is a separate
# assertion, see the reviewer literal-heading check below).
POPULATE_HEADING_RE='persistence|metadata|frontmatter|populate|capture metadata|step [0-9]'

# Check imperative-instruction context: inside a section whose heading
# matches the persistence-related allowed list, both an imperative verb
# (Substitute|Populate|Set|Write|Emit) and a colon-anchored field
# reference must appear. The verb and the field do not need to be on the
# same line — the canonical persistence-step snippet introduces the verb
# on a leading "Substitute every field below" line and lists the fields
# as bullets beneath.
in_imperative_section() {
  local file="$1" field="$2"
  awk -v field="$field" -v headingre="$POPULATE_HEADING_RE" '
    function flush() {
      if (in_section && has_verb && has_field) { found = 1 }
      has_verb = 0
      has_field = 0
    }
    /^#/ {
      flush()
      heading = tolower($0)
      in_section = (heading ~ headingre)
      next
    }
    in_section {
      pat = "(^|[ \t]|`|\\*)" field ":"
      verbs = "[Ss]ubstitute|[Pp]opulate|[Ss]et|[Ww]rite|[Ee]mit"
      if ($0 ~ pat) has_field = 1
      if (match($0, verbs)) has_verb = 1
    }
    END {
      flush()
      exit (found ? 0 : 1)
    }
  ' "$file"
}

# Check that the named field's OWN bullet, inside a Populate-frontmatter-ish
# section (located via the shared $POPULATE_HEADING_RE), carries a whole-word
# fill/omit guidance keyword. The keyword is bound to the field's own bullet
# window (from the bullet naming the field until the next bullet / heading /
# EOF), not to the section as a whole — otherwise one field's note would
# satisfy the check for every field. POSIX classes keep matching identical
# under BSD awk (macOS) and gawk (CI). Returns 0 only when the field's bullet
# carries its own fill/omit guidance.
in_populate_section_with_guidance() {
  local file="$1" field="$2"
  awk -v field="$field" -v headingre="$POPULATE_HEADING_RE" '
    BEGIN { fieldpat = "(^|[[:space:]]|`|\\*)" field ":" }
    # Commit a satisfied attribution window (the field bullet carried its own
    # fill/omit guidance) and reset it. Called wherever a window closes —
    # next bullet, heading boundary, AND EOF.
    function flush() {
      if (in_section && tracking && saw) found = 1
      tracking = 0; saw = 0
    }
    /^#/ {
      flush()
      heading = tolower($0)
      in_section = (heading ~ headingre)
      next
    }
    in_section {
      if ($0 ~ /^[[:space:]]*[-*]/) flush()   # a new bullet closes the prior window
      # Arm only ONCE per window (!tracking guard): a continuation line that
      # re-mentions the field key must not reset saw and drop a satisfied
      # window.
      if (!tracking && $0 ~ fieldpat) { tracking = 1; saw = 0 }
      if (tracking && $0 ~ /(^|[^[:alpha:]])(fill|omit)([^[:alpha:]]|$)/) saw = 1
    }
    END { flush(); exit (found ? 0 : 1) }
  ' "$file"
}

# Counts reviewer producers that carry a literal "Populate frontmatter"
# heading. Reviewer rows are discriminated from the TSV (they declare
# `target` in fields_to_assert), not a hardcoded path list; gated on == 4
# after the loop so the assertion cannot go inert.
reviewer_heading_pass=0

# Iterate each skill row, skipping the header (row 1). Process
# substitution (rather than a pipe) keeps the loop in the parent shell
# so PASS/FAIL counter increments persist.
while IFS=$'\t' read -r skill_path producer_name fields omit_when_empty; do
  echo "--- $skill_path (producer=$producer_name) ---"
  if [ ! -f "$skill_path" ]; then
    echo "  FAIL: $skill_path — SKILL.md not found"
    FAIL=$((FAIL + 1))
    continue
  fi

  stripped=$(mktemp)
  strip_template_directives "$skill_path" >"$stripped"

  for field in $fields; do
    if in_fenced_block "$stripped" "$field" ||
      in_imperative_section "$stripped" "$field"; then
      echo "  PASS: $skill_path: instructs population of '$field'"
      PASS=$((PASS + 1))
    else
      echo "  FAIL: $skill_path: no instruction to populate '$field'"
      FAIL=$((FAIL + 1))
    fi
  done

  # Omit-when-empty fields (ADR-0040): each must appear in a
  # Populate-frontmatter section AND carry its own fill/omit guidance note.
  # `-` sentinel = no omit-when-empty fields on this row (skipped).
  for fld in $omit_when_empty; do
    [ "$fld" = "-" ] && continue
    if in_populate_section_with_guidance "$stripped" "$fld"; then
      echo "  PASS: $skill_path: instructs population of omit-when-empty field '$fld' with fill/omit guidance"
      PASS=$((PASS + 1))
    else
      echo "  FAIL: $skill_path: omit-when-empty field '$fld' missing or lacks fill/omit guidance in Populate frontmatter section"
      FAIL=$((FAIL + 1))
    fi
  done

  # Reviewer producers (rows declaring `target` in fields_to_assert) must
  # carry a literal `#`-prefixed "Populate frontmatter" heading — enforces
  # AC #3 in the test itself, not just via the awk detector's broad predicate.
  case " $fields " in
    *" target "*)
      if grep -qE '^#+[[:space:]]+Populate frontmatter[[:space:]]*$' "$skill_path"; then
        echo "  PASS: $skill_path: carries a literal 'Populate frontmatter' heading"
        reviewer_heading_pass=$((reviewer_heading_pass + 1))
        PASS=$((PASS + 1))
      else
        echo "  FAIL: $skill_path: reviewer producer lacks a literal 'Populate frontmatter' heading"
        FAIL=$((FAIL + 1))
      fi
      ;;
  esac

  rm -f "$stripped"
done < <(tail -n +2 "$SKILLS_TSV")

if [ "$reviewer_heading_pass" -eq 4 ]; then
  echo "  PASS: reviewer literal-heading count (4)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: reviewer literal-heading count is $reviewer_heading_pass, expected 4"
  FAIL=$((FAIL + 1))
fi

# Phase 11 discovery assertion: every SKILL.md surfaced by Pass A or Pass B
# must appear in one of the three allowlists. The patterns are kept here
# (rather than in the work item) so the test and the work-item's recorded
# Discovery Pass Record share a single source of truth (the work-item
# references this script by name).
echo "--- Discovery pass: every emitting/template-consuming SKILL is allowlisted ---"
DISCOVERY_PATTERNS=(
  'config-read-template\.sh'
  '^[[:space:]]*producer:'
  '^[[:space:]]*schema_version:'
  '^[[:space:]]*verdict:'
  '^[[:space:]]*review_pass:'
  '^[[:space:]]*review_target:'
  '^[[:space:]]*target:'
  '^[[:space:]]*result:'
  '^[[:space:]]*pr_number:'
)

discovered=$(
  for pat in "${DISCOVERY_PATTERNS[@]}"; do
    grep -rlE "$pat" skills --include='SKILL.md' 2>/dev/null || true
  done | sort -u
)

allowlist=$(
  printf '%s\n' \
    "${IN_SCOPE_PRODUCERS[@]}" \
    "${NON_EMITTER_TEMPLATE_CONSUMERS[@]}" |
    sort -u
)

unexpected=$(comm -23 <(printf '%s\n' "$discovered") <(printf '%s\n' "$allowlist"))
if [ -z "$unexpected" ]; then
  echo "  PASS: every discovered SKILL.md is allowlisted"
  PASS=$((PASS + 1))
else
  echo "  FAIL: SKILL.md files surfaced by discovery pass but not categorised:"
  echo "$unexpected" | sed 's/^/    /'
  FAIL=$((FAIL + 1))
fi

# ---------------------------------------------------------------------------
# Liveness self-test for in_populate_section_with_guidance. Because every
# omit_when_empty column is `-` until Phases 3-6 populate it, the loop above
# may iterate zero times — so without this the helper could ship never having
# been observed to reject bad input. Runs on every invocation (guarded inline,
# not a sibling file). Gated on the exact PASS count.
# ---------------------------------------------------------------------------
echo "--- Self-test: in_populate_section_with_guidance liveness ---"
selftest_pass=0
st_guidance() {
  # $1 desc, $2 field, $3 expected_rc (0 accept / 1 reject), $4 file
  local desc="$1" field="$2" exprc="$3" file="$4" rc=0
  in_populate_section_with_guidance "$file" "$field" || rc=$?
  if [ "$rc" -eq "$exprc" ]; then
    echo "  PASS: self-test $desc"
    selftest_pass=$((selftest_pass + 1))
  else
    echo "  FAIL: self-test $desc (got rc=$rc, expected $exprc)"
    FAIL=$((FAIL + 1))
  fi
}

st_dir=$(mktemp -d)

# 1. field WITH its own fill/omit bullet → accepted (0).
cat > "$st_dir/with.md" <<'STEOF'
### Populate frontmatter

- `parent:` ← the parent ref. Fill when named; otherwise omit the key.
STEOF
st_guidance "a field with a fill/omit bullet is accepted" parent 0 "$st_dir/with.md"

# 2. field named WITHOUT any fill/omit note → rejected (1).
cat > "$st_dir/without.md" <<'STEOF'
### Populate frontmatter

- `parent:` ← the parent ref. Set it to the parent work item id.
STEOF
st_guidance "a field without a fill/omit note is rejected" parent 1 "$st_dir/without.md"

# 3. fill/omit note on a DIFFERENT field's bullet → rejected (per-field binding).
cat > "$st_dir/crossfield.md" <<'STEOF'
### Populate frontmatter

- `parent:` ← the parent ref. Set it to the parent work item id.
- `source:` ← the source ref. Fill when explicit; otherwise omit.
STEOF
st_guidance "a fill/omit note on a different field's bullet is rejected" parent 1 "$st_dir/crossfield.md"

# 4. fill/omit only as a buried substring (backfill) → rejected (whole-word).
cat > "$st_dir/buried.md" <<'STEOF'
### Populate frontmatter

- `parent:` ← the parent ref. We backfill this during reconciliation.
STEOF
st_guidance "a buried fill/omit substring (backfill) is rejected" parent 1 "$st_dir/buried.md"

# 5. proper guidance under a bold lead-in (no `#` heading) → rejected.
cat > "$st_dir/bold.md" <<'STEOF'
**Populate frontmatter**:

- `parent:` ← the parent ref. Fill when named; otherwise omit the key.
STEOF
st_guidance "guidance under a bold lead-in (no # heading) is rejected" parent 1 "$st_dir/bold.md"

rm -rf "$st_dir"

if [ "$selftest_pass" -eq 5 ]; then
  echo "  PASS: guidance-helper liveness self-test count (5)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: guidance-helper liveness self-test count is $selftest_pass, expected 5"
  FAIL=$((FAIL + 1))
fi

test_summary
