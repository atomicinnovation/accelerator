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

echo "=== SKILL.md frontmatter population prose ==="

SKILLS_TSV="$SCRIPT_DIR/skills-schema.tsv"

# Allowlists for the Phase 11 discovery assertion. Together they must cover
# every SKILL.md surfaced by Pass A or Pass B (see Phase 11 of the plan).
IN_SCOPE_PRODUCERS=(
  skills/work/create-work-item/SKILL.md
  skills/work/extract-work-items/SKILL.md
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
  skills/work/refine-work-item/SKILL.md
  skills/work/update-work-item/SKILL.md
  skills/work/list-work-items/SKILL.md
)

# Self-check: every TSV row (including the header) must have exactly
# three tab-separated fields. Header is row 1 and is skipped by the
# data loop below.
if ! awk -F'\t' 'NF != 3 { print "ERROR: " FILENAME ":" NR " has " NF " fields, expected 3"; bad=1 } END { exit (bad ? 1 : 0) }' "$SKILLS_TSV"; then
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

# Check imperative-instruction context: inside a section whose heading
# matches the persistence-related allowed list, both an imperative verb
# (Substitute|Populate|Set|Write|Emit) and a colon-anchored field
# reference must appear. The verb and the field do not need to be on the
# same line — the canonical persistence-step snippet introduces the verb
# on a leading "Substitute every field below" line and lists the fields
# as bullets beneath.
in_imperative_section() {
  local file="$1" field="$2"
  awk -v field="$field" '
    function flush() {
      if (in_section && has_verb && has_field) { found = 1 }
      has_verb = 0
      has_field = 0
    }
    /^#/ {
      flush()
      heading = tolower($0)
      if (heading ~ /persistence|metadata|frontmatter|populate|capture metadata|step [0-9]/) {
        in_section = 1
      } else {
        in_section = 0
      }
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

# Iterate each skill row, skipping the header (row 1). Process
# substitution (rather than a pipe) keeps the loop in the parent shell
# so PASS/FAIL counter increments persist.
while IFS=$'\t' read -r skill_path producer_name fields; do
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

  rm -f "$stripped"
done < <(tail -n +2 "$SKILLS_TSV")

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

test_summary
