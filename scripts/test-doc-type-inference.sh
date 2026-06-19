#!/usr/bin/env bash
set -euo pipefail

# Test harness for scripts/doc-type-inference.sh — the injected-table allowlist
# classifier and its backward-compatible fallback.
# Run: bash scripts/test-doc-type-inference.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
export LC_ALL=C

# shellcheck source=doc-type-inference.sh
source "$SCRIPT_DIR/doc-type-inference.sh"

GOLDEN="$SCRIPT_DIR/test-fixtures/doc-type-inference/fallback-golden.txt"

# infer_type_from_path captured as a plain string (newline stripped).
infer() { infer_type_from_path "$1"; }
# "out"/"in" for out_of_scope.
scope() { if out_of_scope "$1"; then echo out; else echo in; fi; }

# ---- 1. Injected-table allowlist matching ----------------------------------
echo "=== Injected-table matching ==="
DOC_TYPE_INJECTED_NAMES=(work-item plan pr-description pr-review codebase-research)
DOC_TYPE_INJECTED_DIRS=(meta/work meta/plans meta/prs meta/reviews/prs meta/research/codebase)
DOC_TYPE_TABLE_INJECTED=1

assert_eq "relative path: meta/work -> work-item" "work-item" "$(infer meta/work/0001-x.md)"
assert_eq "absolute-prefixed path: meta/work -> work-item" \
  "work-item" "$(infer /abs/proj/meta/work/0001-x.md)"
assert_eq "most-specific match: meta/reviews/prs -> pr-review (not pr-description)" \
  "pr-review" "$(infer /abs/proj/meta/reviews/prs/42-review-1.md)"
assert_eq "most-specific match holds for relative path too" \
  "pr-review" "$(infer meta/reviews/prs/42-review-1.md)"
assert_eq "research/codebase classified (not shadowed)" \
  "codebase-research" "$(infer meta/research/codebase/2026-01-01-x.md)"
# Segment-boundary safety: meta/prs must not match meta/prs-archive.
assert_eq "segment boundary: meta/prs-archive does NOT match meta/prs" \
  "" "$(infer meta/prs-archive/x.md)"
assert_eq "segment boundary keeps prs-archive out of scope" \
  "out" "$(scope meta/prs-archive/x.md)"
# Allowlist skip: an unknown subtree is out of scope.
assert_eq "unknown subtree -> empty type" "" "$(infer meta/whatever/x.md)"
assert_eq "unknown subtree -> out of scope" "out" "$(scope meta/whatever/x.md)"
assert_eq "configured subtree -> in scope" "in" "$(scope meta/work/0001-x.md)"

# ---- 2. Glob metacharacter in a configured dir matched literally -----------
echo "=== Glob metacharacter literal-matching ==="
DOC_TYPE_INJECTED_NAMES=(note)
DOC_TYPE_INJECTED_DIRS=('meta/star*dir')
DOC_TYPE_TABLE_INJECTED=1
assert_eq "literal '*' in dir matches the literal path" \
  "note" "$(infer 'meta/star*dir/x.md')"
assert_eq "literal '*' in dir does NOT glob-match other text" \
  "" "$(infer 'meta/starANYTHINGdir/x.md')"

# ---- 3. Equal-length tie resolves by array order ---------------------------
echo "=== Equal-length tie (deterministic by array order) ==="
DOC_TYPE_INJECTED_NAMES=(type-a type-b)
DOC_TYPE_INJECTED_DIRS=(meta/tie meta/tie)
DOC_TYPE_TABLE_INJECTED=1
assert_eq "two types on same dir -> first-in-array wins" \
  "type-a" "$(infer meta/tie/x.md)"

# ---- 4. Fallback mode reproduces the checked-in golden ---------------------
echo "=== Fallback mode (golden-pinned) ==="
unset DOC_TYPE_TABLE_INJECTED
assert_file_exists "fallback golden fixture present" "$GOLDEN"
# Re-derive over the golden's own path column so the path list cannot drift from
# the captured expectations.
regen="$(while IFS=$'\t' read -r p _t _s; do
  [ -n "$p" ] || continue
  printf '%s\t%s\t%s\n' "$p" "$(infer "$p")" "$(scope "$p")"
done <"$GOLDEN")"
assert_eq "fallback classifications reproduce the golden snapshot" \
  "$(cat "$GOLDEN")" "$regen"

test_summary
