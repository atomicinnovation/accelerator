#!/usr/bin/env bash
set -euo pipefail

# Test harness for scripts/linkage-parser.sh.
# Run: bash scripts/test-linkage-parser.sh
#
# Run under the macOS system bash (/bin/bash, 3.2) at least once in CI replay:
# the spike-fix keyword boundaries must classify identically on the BSD
# grep/awk toolchain (no \b), which the hyphen-boundary assertions below pin.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
# shellcheck source=linkage-parser.sh
source "$SCRIPT_DIR/linkage-parser.sh"

export LC_ALL=C

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# Parse a file (forcing source_type) and return the full TSV output.
parse() { lp_parse_file "$1" "${2:-}"; }

# Field (1-5) of the record whose target_ref (col 3) equals $2, from output $1.
field_for_target() {
  printf '%s\n' "$1" | awk -F'\t' -v t="$2" -v c="$3" '$3 == t { print $c; exit }'
}

assert_rc0() { # name; cmd...
  local name="$1"
  shift
  if "$@"; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name (expected rc 0)"
    FAIL=$((FAIL + 1))
  fi
}
assert_rc1() { # name; cmd...
  local name="$1"
  shift
  if "$@"; then
    echo "  FAIL: $name (expected non-zero rc)"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  fi
}

# ---- 1. Spike fix #1: template-path blocklist (AC-11) ----------------------
echo "=== Spike fix #1: template-path blocklist ==="
assert_rc0 "ADR-NNNN.md is a template path" lp_is_template_path "meta/decisions/ADR-NNNN.md"
assert_rc0 "YYYY-MM-DD-topic.md is a template path" lp_is_template_path "meta/notes/YYYY-MM-DD-topic.md"
assert_rc0 "{number}-description.md is a template path" lp_is_template_path "meta/work/{number}-description.md"
assert_rc1 "real path is not a template path" lp_is_template_path "meta/work/0030-foo.md"

mkdir -p "$TMP/meta/plans"
cat >"$TMP/meta/plans/2026-01-01-0001-tpl.md" <<'EOF'
## References
- The template is `meta/decisions/ADR-NNNN-description.md` and should not link
- A real ref `meta/work/0030-real.md`
EOF
out="$(parse "$TMP/meta/plans/2026-01-01-0001-tpl.md")"
assert_not_contains "template path produces no linkage record" "$out" "ADR-NNNN"
assert_contains "the sibling real ref is still parsed" "$out" "work-item:0030"

# ---- 2. Spike fix #2: tightened blocks keyword (AC-11, no \b) ---------------
echo "=== Spike fix #2: blocks keyword (hyphen/underscore boundary) ==="
assert_rc0 "'Blocks:' label matches" lp_has_blocks_keyword "Blocks: 0034"
assert_rc0 "'this blocks the other' matches" lp_has_blocks_keyword "this blocks the other"
assert_rc1 "'code-block' does NOT match (hyphen boundary)" lp_has_blocks_keyword "see the code-block example"
assert_rc1 "'code_block' does NOT match (underscore boundary)" lp_has_blocks_keyword "a code_block here"

mkdir -p "$TMP/meta/work"
cat >"$TMP/meta/work/0050-cb.md" <<'EOF'
## Dependencies
- The code-block rendering is unrelated to dependencies
EOF
out="$(parse "$TMP/meta/work/0050-cb.md")"
assert_empty "code-block prose yields no blocks linkage" "$out"

# ---- 3. Spike fix #3: sibling → relates_to (AC-11) -------------------------
echo "=== Spike fix #3: sibling → relates_to ==="
assert_rc0 "'Sibling:' label matches" lp_has_sibling_keyword "Sibling: meta/plans/x.md"
assert_rc1 "'siblings-list' compound does NOT match" lp_has_sibling_keyword "siblings-list here"
mkdir -p "$TMP/meta/plans"
cat >"$TMP/meta/plans/2026-01-02-0002-sib.md" <<'EOF'
## References
- Sibling component plans: `meta/plans/2026-01-03-0003-other.md`
EOF
out="$(parse "$TMP/meta/plans/2026-01-02-0002-sib.md")"
assert_eq "sibling ref keyed relates_to" "relates_to" "$(field_for_target "$out" "plan:2026-01-03-0003-other" 2)"

# ---- 4. Band-classification set across the five headers (AC-6/AC-8) --------
echo "=== Band classification across five headers ==="
mkdir -p "$TMP/meta/work" "$TMP/meta/plans" "$TMP/meta/decisions"
# work-item Dependencies, explicit Blocks label, deterministic pair → resolved.
cat >"$TMP/meta/work/0060-bands.md" <<'EOF'
## Dependencies
- Blocks: 0061
EOF
out="$(parse "$TMP/meta/work/0060-bands.md")"
assert_eq "work-item Blocks → resolved" "resolved" "$(field_for_target "$out" "work-item:0061" 5)"

# plan References, explicit Source → resolved parent.
cat >"$TMP/meta/plans/2026-02-01-0062-bands.md" <<'EOF'
## References
- Source: `meta/work/0063-owning.md`
EOF
out="$(parse "$TMP/meta/plans/2026-02-01-0062-bands.md")"
assert_eq "plan Source → resolved" "resolved" "$(field_for_target "$out" "work-item:0063" 5)"

# plain path in ## References with no prose hint → ambiguous (section default).
cat >"$TMP/meta/plans/2026-02-02-0064-bands.md" <<'EOF'
## References
- `meta/plans/2026-02-03-0065-bare.md`
EOF
out="$(parse "$TMP/meta/plans/2026-02-02-0064-bands.md")"
assert_eq "unhinted ## References ref → ambiguous" "ambiguous" "$(field_for_target "$out" "plan:2026-02-03-0065-bare" 5)"

# plain ## Related Research ref, no explicit hint → ambiguous (section default).
cat >"$TMP/meta/plans/2026-02-04-0066-bands.md" <<'EOF'
## Related Research
- `meta/research/codebase/2026-02-04-rr.md`
EOF
out="$(parse "$TMP/meta/plans/2026-02-04-0066-bands.md")"
assert_eq "unhinted ## Related Research ref → ambiguous" "ambiguous" "$(field_for_target "$out" "codebase-research:2026-02-04-rr" 5)"

# adr Historical Context supersedes → resolved.
cat >"$TMP/meta/decisions/ADR-0067-bands.md" <<'EOF'
## Historical Context
- Supersedes `meta/decisions/ADR-0026-old.md`
EOF
out="$(parse "$TMP/meta/decisions/ADR-0067-bands.md")"
assert_eq "adr supersedes → resolved" "resolved" "$(field_for_target "$out" "adr:ADR-0026" 5)"

# ---- 5. Resolved-band golden-target set (AC-8 correctness) -----------------
echo "=== Resolved-band golden targets (emitted value) ==="
# Each resolved ref's emitted target equals its hand-verified doc-type:id.
assert_eq "golden: plan Source → work-item:0063" "work-item:0063" \
  "$(field_for_target "$(parse "$TMP/meta/plans/2026-02-01-0062-bands.md")" "work-item:0063" 3)"
assert_eq "golden: work-item Blocks → work-item:0061" "work-item:0061" \
  "$(field_for_target "$(parse "$TMP/meta/work/0060-bands.md")" "work-item:0061" 3)"
assert_eq "golden: adr supersedes → adr:ADR-0026" "adr:ADR-0026" \
  "$(field_for_target "$(parse "$TMP/meta/decisions/ADR-0067-bands.md")" "adr:ADR-0026" 3)"
# A plan target keeps its FULL stem (not plan:NNNN).
cat >"$TMP/meta/plans/2026-03-01-0070-golden.md" <<'EOF'
## References
- Sibling: `meta/plans/2026-05-13-0055-sidebar-activity-feed.md`
EOF
out="$(parse "$TMP/meta/plans/2026-03-01-0070-golden.md")"
assert_eq "golden: plan target uses full stem" "plan:2026-05-13-0055-sidebar-activity-feed" \
  "$(field_for_target "$out" "plan:2026-05-13-0055-sidebar-activity-feed" 3)"

# ---- 6. Prose disambiguation: Source → parent / derived_from / source ------
echo "=== Source: disambiguation (parent / derived_from / source) ==="
# shellcheck disable=SC2016 # single-quoted markdown arg; backticks are literal text passed verbatim to lp_infer_key, intentionally not command substitution
assert_eq "Source→parent for work-item target" "parent" \
  "$(printf '%s' "$(lp_infer_key plan '## References' '- Source: `meta/work/0042-x.md`' work-item)" | cut -f1)"
# shellcheck disable=SC2016 # single-quoted markdown arg; backticks are literal text passed verbatim to lp_infer_key, intentionally not command substitution
assert_eq "Source→derived_from for codebase-research target" "derived_from" \
  "$(printf '%s' "$(lp_infer_key plan '## References' '- Source: `meta/research/codebase/x.md`' codebase-research)" | cut -f1)"
# shellcheck disable=SC2016 # single-quoted markdown arg; backticks are literal text passed verbatim to lp_infer_key, intentionally not command substitution
assert_eq "Source→derived_from for issue-research target" "derived_from" \
  "$(printf '%s' "$(lp_infer_key plan '## References' '- Source: `meta/research/issues/x.md`' issue-research)" | cut -f1)"
assert_eq "Source→source for non-meta target" "source" \
  "$(printf '%s' "$(lp_infer_key plan '## References' '- Source: https://example.com/spec' '')" | cut -f1)"

# ---- 7. pr: tolerance (AC) -------------------------------------------------
echo "=== pr: tolerance ==="
mkdir -p "$TMP/meta/reviews/prs"
cat >"$TMP/meta/reviews/prs/42-review-1.md" <<'EOF'
## References
- Reviews `pr:42`
EOF
out="$(parse "$TMP/meta/reviews/prs/42-review-1.md")"
assert_contains "pr: reference is emitted (tolerated)" "$out" "pr:42"
run_rc=0
parse "$TMP/meta/reviews/prs/42-review-1.md" >/dev/null 2>&1 || run_rc=$?
assert_eq "pr: reference never errors the parser" "0" "$run_rc"

# ---- 8. Known-ambiguous set (AC-9) -----------------------------------------
echo "=== Known-ambiguous references ==="
# A bare 4-digit number on a Related: label has no determinate target type and
# no single-valued pairing → ambiguous (routed to the hook in Phase 3).
cat >"$TMP/meta/work/0080-amb.md" <<'EOF'
## Dependencies
- Related: 0030
EOF
out="$(parse "$TMP/meta/work/0080-amb.md")"
assert_eq "bare-number Related → ambiguous" "ambiguous" "$(field_for_target "$out" "0030" 5)"

# work-item derived-from-able bare number: (work-item, derived_from) admits both
# note and work-item targets → genuinely multi-candidate → ambiguous. Emulated
# via a bare number with no explicit single-valued pairing.
cat >"$TMP/meta/work/0081-amb.md" <<'EOF'
## References
- `meta/notes/2026-01-01-some-note.md`
EOF
out="$(parse "$TMP/meta/work/0081-amb.md")"
assert_eq "unhinted note ref on work-item → ambiguous" "ambiguous" \
  "$(field_for_target "$out" "note:2026-01-01-some-note" 5)"

test_summary
