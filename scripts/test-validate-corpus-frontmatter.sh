#!/usr/bin/env bash
set -euo pipefail

# Test harness for scripts/validate-corpus-frontmatter.sh.
# Run: bash scripts/test-validate-corpus-frontmatter.sh
#
# Fixtures are generated inline in a tmpdir (the dominant pattern in this
# subtree, e.g. test-atomic-common.sh) rather than checked in, so the per-type
# good fixtures stay in lock-step with templates-schema.tsv automatically.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
# shellcheck source=frontmatter-emission-rules.sh
source "$SCRIPT_DIR/frontmatter-emission-rules.sh"

export LC_ALL=C

VALIDATOR="$SCRIPT_DIR/validate-corpus-frontmatter.sh"
TEMPLATE_TEST="$SCRIPT_DIR/test-template-frontmatter.sh"
SCHEMA_TSV="$SCRIPT_DIR/templates-schema.tsv"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# ---- Fixture generation ---------------------------------------------------
# Emit a minimal *valid* artifact for a schema row. Required (non-optional)
# extras get a non-empty placeholder; anchored types get the provenance bundle;
# typed-linkage keys are omitted (omit-when-empty), so no referential targets
# are needed. Extra frontmatter lines may be appended via $extra_lines (a
# newline-separated string injected before the closing fence).
emit_valid() {
  local type="$1" anchored="$2" extras="$3" vocab="$4" outfile="$5" extra_lines="${6:-}"
  local id status e
  case "$type" in
    work-item) id="0001" ;;
    adr) id="ADR-0001" ;;
    pr-description) id="0042" ;;
    *) id="fixture-$type" ;;
  esac
  status="$(printf '%s' "$vocab" | cut -d'|' -f1 | tr -d '[:space:]')"
  {
    printf -- '---\n'
    printf 'type: %s\n' "$type"
    printf 'id: "%s"\n' "$id"
    printf 'title: "Fixture %s"\n' "$type"
    printf 'date: "2026-01-01T00:00:00+00:00"\n'
    printf 'author: Fixture Author\n'
    printf 'producer: fixture\n'
    printf 'status: %s\n' "$status"
    printf 'tags: []\n'
    printf 'last_updated: "2026-01-01T00:00:00+00:00"\n'
    printf 'last_updated_by: Fixture Author\n'
    printf 'schema_version: 1\n'
    for e in $extras; do
      case " $FM_OPTIONAL_EXTRAS " in *" $e "*) continue ;; esac
      printf '%s: "x"\n' "$e"
    done
    if [ "$anchored" = "yes" ]; then
      printf 'revision: "abc123"\n'
      printf 'repository: "repo"\n'
    fi
    [ -n "$extra_lines" ] && printf '%s\n' "$extra_lines"
    printf -- '---\n\n# Fixture %s\n' "$type"
  } >"$outfile"
}

# Capture the validator's stderr+rc for a set of args.
run_validator() {
  VALIDATOR_RC=0
  VALIDATOR_ERR="$("$VALIDATOR" "$@" 2>&1 >/dev/null)" || VALIDATOR_RC=$?
}

assert_rejects() { # $1=name $2=code; remaining args = validator args
  local name="$1" code="$2"
  shift 2
  run_validator "$@"
  if [ "$VALIDATOR_RC" -ne 0 ] && grep -qF -- "$code" <<<"$VALIDATOR_ERR"; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name (rc=$VALIDATOR_RC, expected code '$code')"
    echo "$VALIDATOR_ERR" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi
}

assert_accepts() { # $1=name; remaining args = validator args
  local name="$1"
  shift
  run_validator "$@"
  if [ "$VALIDATOR_RC" -eq 0 ]; then
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name (rc=$VALIDATOR_RC)"
    echo "$VALIDATOR_ERR" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi
}

# ---- 1. One valid fixture per schema type; coverage == TSV row count -------
echo "=== Valid fixtures: one per schema type ==="
VALID_DIR="$TMP/valid"
mkdir -p "$VALID_DIR"
type_count=0
while IFS=$'\t' read -r _tmpl type anchored extras vocab _forbidden _linkkeys; do
  emit_valid "$type" "$anchored" "$extras" "$vocab" "$VALID_DIR/$type.md"
  type_count=$((type_count + 1))
done < <(tail -n +2 "$SCHEMA_TSV")

tsv_rows=$(($(wc -l <"$SCHEMA_TSV") - 1))
assert_eq "valid fixture per TSV row (count matches TSV)" "$tsv_rows" "$type_count"
assert_accepts "validator accepts every per-type valid fixture (file-list mode)" "$VALID_DIR"/*.md

# Unquoted-but-present author stays conforming (guards a future tightening of
# the shared helper from silently failing the corpus).
emit_valid work-item no "kind priority external_id" "draft | ready" "$TMP/unquoted-author.md"
assert_accepts "unquoted-but-present author: line accepted" "$TMP/unquoted-author.md"

# ---- 2. Failure-mode fixtures ---------------------------------------------
echo "=== Failure-mode fixtures ==="
BASE_EXTRAS="kind priority external_id"
BASE_VOCAB="draft | ready | in-progress | review | done | blocked | abandoned"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-unquoted-id.md"
sed -i.bak 's/^id: "0001"$/id: 0001/' "$TMP/bad-unquoted-id.md"
assert_rejects "unquoted id rejected" "UNQUOTED-ID" "$TMP/bad-unquoted-id.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-missing-base.md"
sed -i.bak '/^title: /d' "$TMP/bad-missing-base.md"
assert_rejects "missing base field rejected" "MISSING-BASE-FIELD" "$TMP/bad-missing-base.md"

emit_valid plan yes "reviewer" "draft | ready | in-progress | done" "$TMP/bad-gitcommit.md" 'git_commit: "deadbeef"'
assert_rejects "git_commit present rejected" "FORBIDDEN-PROVENANCE" "$TMP/bad-gitcommit.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-ownid.md" 'work_item_id: "0001"'
assert_rejects "forbidden own-id key rejected" "FORBIDDEN-OWN-ID" "$TMP/bad-ownid.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-empty.md" 'parent: ""'
assert_rejects "empty-placeholder key rejected" "EMPTY-PLACEHOLDER" "$TMP/bad-empty.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-bare-linkage.md" 'parent: "0030"'
assert_rejects "bare-number linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-bare-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-path-linkage.md" 'parent: "meta/work/0030-foo.md"'
assert_rejects "path-shape linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-path-linkage.md"

# A typed ref whose id is a version-numbered stem (dots) is accepted.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-dotted-linkage.md" 'relates_to: ["plan:2026-06-04-changelog-1.21.0-cleanup"]'
assert_accepts "dotted typed-ref id accepted (version-numbered stem)" "$TMP/ok-dotted-linkage.md"

# A note: target is a valid typed ref (work-item extracted from a note —
# ADR-0034 work-item|source|note).
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-note-source.md" 'source: "note:2026-04-29-some-note"'
assert_accepts "note: typed-ref target accepted" "$TMP/ok-note-source.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-status.md"
sed -i.bak 's/^status: .*/status: bogus/' "$TMP/bad-status.md"
assert_rejects "bad status rejected" "BAD-STATUS" "$TMP/bad-status.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-schemaver.md"
sed -i.bak 's/^schema_version: 1$/schema_version: "1"/' "$TMP/bad-schemaver.md"
assert_rejects "non-integer schema_version rejected" "BAD-SCHEMA-VERSION" "$TMP/bad-schemaver.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-date.md"
sed -i.bak 's/^date: .*/date: "2026-01-01"/' "$TMP/bad-date.md"
assert_rejects "date-only (non-ISO-timestamp) rejected" "BAD-TIMESTAMP" "$TMP/bad-date.md"

# Both ISO offset forms (Z and +00:00) accepted.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-zulu.md"
sed -i.bak 's/^date: .*/date: "2026-01-01T21:38:10Z"/' "$TMP/ok-zulu.md"
assert_accepts "Z (zulu) ISO timestamp accepted" "$TMP/ok-zulu.md"

# No-fence file.
printf '# just a heading\n\nbody\n' >"$TMP/no-fence.md"
assert_rejects "no-fence file rejected" "NO-FENCE" "$TMP/no-fence.md"

# Unknown type.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-type.md"
sed -i.bak 's/^type: work-item$/type: nonsense/' "$TMP/bad-type.md"
assert_rejects "unknown type rejected" "INVALID-TYPE" "$TMP/bad-type.md"

# ---- 3. Referential integrity (whole-corpus mode) -------------------------
echo "=== Referential integrity ==="
OK_CORPUS="$TMP/corpus-ok/meta"
mkdir -p "$OK_CORPUS/work" "$OK_CORPUS/reviews/work" "$OK_CORPUS/reviews/prs"
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$OK_CORPUS/work/0030-target.md"
sed -i.bak 's/^id: "0001"$/id: "0030"/' "$OK_CORPUS/work/0030-target.md"
emit_valid work-item-review no "reviewer verdict lenses review_number review_pass work_item_id" "complete" \
  "$OK_CORPUS/reviews/work/0030-review-1.md" 'target: "work-item:0030"'
emit_valid pr-review no "reviewer verdict lenses review_number pr_number" "complete" \
  "$OK_CORPUS/reviews/prs/42-review-1.md" 'target: "pr:42"'
rm -f "$OK_CORPUS"/work/*.bak "$OK_CORPUS"/reviews/work/*.bak "$OK_CORPUS"/reviews/prs/*.bak
assert_accepts "resolved typed ref + pr: tolerated (clean corpus)" "$OK_CORPUS"

BAD_CORPUS="$TMP/corpus-bad/meta"
mkdir -p "$BAD_CORPUS/work" "$BAD_CORPUS/reviews/work"
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$BAD_CORPUS/work/0030-target.md"
sed -i.bak 's/^id: "0001"$/id: "0030"/' "$BAD_CORPUS/work/0030-target.md"
emit_valid work-item-review no "reviewer verdict lenses review_number review_pass work_item_id" "complete" \
  "$BAD_CORPUS/reviews/work/0099-review-1.md" 'target: "work-item:9999"'
rm -f "$BAD_CORPUS"/work/*.bak "$BAD_CORPUS"/reviews/work/*.bak
assert_rejects "dangling typed ref flagged (whole-corpus)" "DANGLING-REF" "$BAD_CORPUS"

# ---- 4. Single-source guard -----------------------------------------------
# Flip a rule (drop work-item from FM_SOURCE_TYPE_RE) in a tampered copy of the
# shared helper, point BOTH surfaces at it, and assert both change behaviour —
# proving they consult the one helper, not divergent copies.
echo "=== Single-source guard (shared emission-rules helper) ==="
TAMPERED="$TMP/tampered-rules.sh"
sed 's/work-item|//' "$SCRIPT_DIR/frontmatter-emission-rules.sh" >"$TAMPERED"

# A work-item linkage ref the UNTAMPERED validator accepts...
emit_valid plan yes "reviewer" "draft | ready | in-progress | done" "$TMP/ref-fixture.md" 'parent: "work-item:0030"'
assert_accepts "untampered: work-item: linkage ref accepted" "$TMP/ref-fixture.md"

# ...is rejected once work-item leaves the shared vocab.
guard_rc=0
guard_err="$(FM_EMISSION_RULES="$TAMPERED" "$VALIDATOR" "$TMP/ref-fixture.md" 2>&1 >/dev/null)" || guard_rc=$?
if [ "$guard_rc" -ne 0 ] && grep -qF "BAD-LINKAGE-SHAPE" <<<"$guard_err"; then
  echo "  PASS: validator behaviour flips with tampered shared helper"
  PASS=$((PASS + 1))
else
  echo "  FAIL: validator did NOT flip with tampered helper (rc=$guard_rc)"
  FAIL=$((FAIL + 1))
fi

# The template-shape test also consults the same helper: untampered green,
# tampered red (its linkage-slot regex no longer matches work-item: comments).
tmpl_ok_rc=0
"$TEMPLATE_TEST" >/dev/null 2>&1 || tmpl_ok_rc=$?
assert_eq "untampered: template-shape test passes" "0" "$tmpl_ok_rc"
tmpl_bad_rc=0
FM_EMISSION_RULES="$TAMPERED" "$TEMPLATE_TEST" >/dev/null 2>&1 || tmpl_bad_rc=$?
assert_neq "template-shape test behaviour flips with tampered helper" "0" "$tmpl_bad_rc"

# ---- 5. Sanity: the real (migrated) corpus validates clean ----------------
# Post-0007 the corpus is unified-schema; this both proves the validator reads
# real files and guards against the migrated corpus regressing.
echo "=== Sanity: real (migrated) corpus validates clean ==="
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
if [ -d "$ROOT/meta/work" ]; then
  sanity_rc=0
  "$VALIDATOR" "$ROOT/meta" >/dev/null 2>&1 || sanity_rc=$?
  assert_eq "migrated corpus validates clean (validator reads real files)" "0" "$sanity_rc"
else
  skip_test "real-corpus sanity check" "meta/work not present"
fi

test_summary
