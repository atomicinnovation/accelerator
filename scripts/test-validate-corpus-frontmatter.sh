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

# emit_valid / run_validator / assert_rejects / assert_accepts live in the
# shared fixture helper, so this suite and the producer-conformance guard share
# one fixture authority. Sourced AFTER test-helpers.sh (PASS/FAIL counters),
# frontmatter-emission-rules.sh (FM_OPTIONAL_EXTRAS), and the VALIDATOR= set.
# shellcheck source=frontmatter-fixtures.sh
source "$SCRIPT_DIR/frontmatter-fixtures.sh"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

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

# Non-anchored type carrying the provenance bundle is rejected (the reverse
# direction of the iff). emit_valid only adds provenance for anchored types, so
# inject it via extra_lines.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-prov-nonanchored.md" \
  $'revision: "abc123"\nrepository: "repo"'
assert_rejects "non-anchored type with provenance rejected" \
  "PROVENANCE-ON-NONANCHORED" "$TMP/bad-prov-nonanchored.md"

# Single-field variant (only revision) still trips the rule.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-prov-revision-only.md" \
  'revision: "abc123"'
assert_rejects "non-anchored type with lone revision rejected" \
  "PROVENANCE-ON-NONANCHORED" "$TMP/bad-prov-revision-only.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-ownid.md" 'work_item_id: "0001"'
assert_rejects "forbidden own-id key rejected" "FORBIDDEN-OWN-ID" "$TMP/bad-ownid.md"

# Obsolete legacy linkage keys (ticket / ticket_id) — forbidden on every
# typed/type-inferable doc (the migration-completion gate's second clause; see
# OBSOLETE_LEGACY_KEYS in the validator). `ticket` is neither a work-item
# forbidden-own-id key nor a typed-linkage key, so its sole defect here is the
# obsolete-key clause.
# (a) Fires standalone — and is the ONLY violation, so a future reorder letting
#     an earlier clause short-circuit first cannot mask it.
for obs_key in ticket ticket_id; do
  emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-obsolete-$obs_key.md" "$obs_key: \"0042\""
  assert_rejects "obsolete legacy key '$obs_key' rejected" "OBSOLETE-LEGACY-KEY" "$TMP/bad-obsolete-$obs_key.md"
  run_validator "$TMP/bad-obsolete-$obs_key.md"
  if grep -qF "FAIL: 1 frontmatter violation" <<<"$VALIDATOR_ERR"; then
    echo "  PASS: obsolete key '$obs_key' is the sole violation (standalone observability)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: obsolete key '$obs_key' not standalone; got: $VALIDATOR_ERR"
    FAIL=$((FAIL + 1))
  fi
done

# (b) Clean fixture (no obsolete key) passes.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-no-obsolete.md"
assert_accepts "fixture without obsolete keys accepted" "$TMP/ok-no-obsolete.md"

# (c) Negative discrimination — a CURRENT foreign-reference work_item_id: (and a
# typed parent: "work-item:NNNN") must NOT be flagged obsolete. The fixture is a
# fully-valid PLAN: a work-item would trip FORBIDDEN-OWN-ID on work_item_id: and
# mask this assertion. Pins the key-vs-reference distinction the whole gate rests
# on, so a future widening of OBSOLETE_LEGACY_KEYS to work_item_id is caught here.
emit_valid plan yes "reviewer" "draft | ready | in-progress | done" "$TMP/ok-foreign-wiid.md" 'work_item_id: "0042"'
assert_accepts "foreign work_item_id: on plan not flagged obsolete" "$TMP/ok-foreign-wiid.md"
emit_valid plan yes "reviewer" "draft | ready | in-progress | done" "$TMP/ok-typed-parent.md" 'parent: "work-item:0042"'
assert_accepts "typed parent: work-item ref not flagged obsolete" "$TMP/ok-typed-parent.md"

# (d) Coverage boundary — an obsolete key in an untyped/unmapped doc is NOT
# flagged OBSOLETE-LEGACY-KEY: validate_file returns at the INVALID-TYPE guard
# before the obsolete-key check is reached. (The boundary the gate documents is
# the SKIP on untyped docs; a typeless file is itself an INVALID-TYPE violation,
# so we assert the skip directly rather than via assert_accepts.)
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/boundary-untyped.md" 'ticket: "0042"'
sed -i.bak '/^type: work-item$/d' "$TMP/boundary-untyped.md"
run_validator "$TMP/boundary-untyped.md"
if [ "$VALIDATOR_RC" -ne 0 ] &&
  grep -qF "INVALID-TYPE" <<<"$VALIDATOR_ERR" &&
  ! grep -qF "OBSOLETE-LEGACY-KEY" <<<"$VALIDATOR_ERR"; then
  echo "  PASS: obsolete key in untyped doc skipped (INVALID-TYPE precedes the check)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: untyped-doc boundary not as expected (rc=$VALIDATOR_RC): $VALIDATOR_ERR"
  FAIL=$((FAIL + 1))
fi

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-empty.md" 'parent: ""'
assert_rejects "empty-placeholder key rejected" "EMPTY-PLACEHOLDER" "$TMP/bad-empty.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-bare-linkage.md" 'parent: "0030"'
assert_rejects "bare-number linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-bare-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-path-linkage.md" 'parent: "meta/work/0030-foo.md"'
assert_rejects "path-shape linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-path-linkage.md"

# Genuinely *unquoted* linkage values (distinct from the quoted-malformed
# fixtures above, which the legacy loop already caught). These produce zero/
# partial tokens under the old tokenizer and so escaped BAD-LINKAGE-SHAPE.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-unquoted-linkage.md" 'parent: 0030'
assert_rejects "unquoted (bare) linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-unquoted-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-unquoted-path-linkage.md" 'parent: meta/work/0030-foo.md'
assert_rejects "unquoted path linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-unquoted-path-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-bracket-unquoted-linkage.md" 'parent: [plan:0042]'
assert_rejects "bracketed-but-unquoted linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-bracket-unquoted-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-mixed-list-linkage.md" 'relates_to: ["plan:0001", plan:0002]'
assert_rejects "mixed list with one unquoted element rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-mixed-list-linkage.md"

# Accept-side fixtures guarding the rewrite's new comma-split path directly (the
# only existing bracketed-list accept fixture, ok-dotted-linkage, is single-
# element, so multi-element splitting is otherwise unverified).
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-multi-list-linkage.md" 'relates_to: ["adr:0001", "adr:0002"]'
assert_accepts "multi-element quoted list accepted" "$TMP/ok-multi-list-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-spaced-list-linkage.md" 'relates_to: ["adr:0001",   "adr:0002"]'
assert_accepts "irregularly-spaced quoted list accepted" "$TMP/ok-spaced-list-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-comment-linkage.md" 'parent: "work-item:0001" # inverse note'
assert_accepts "quoted ref with trailing inline comment accepted" "$TMP/ok-comment-linkage.md"

# set -f glob suppression: an UNQUOTED glob-bearing value must reject with the
# LITERAL token, regardless of CWD contents — proving the comma-split does not
# pathname-expand. Run from a directory seeded with files that WOULD match.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-glob-linkage.md" 'parent: plan-*'
mkdir -p "$TMP/globdir"
: >"$TMP/globdir/plan-1.md"
: >"$TMP/globdir/plan-2.md"
glob_rc=0
glob_err="$(cd "$TMP/globdir" && "$VALIDATOR" "$TMP/bad-glob-linkage.md" 2>&1 >/dev/null)" || glob_rc=$?
if [ "$glob_rc" -ne 0 ] &&
  grep -qF -- "BAD-LINKAGE-SHAPE" <<<"$glob_err" &&
  grep -qF -- "plan-*" <<<"$glob_err"; then
  echo "  PASS: unquoted glob value rejects with literal token (globbing suppressed)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: glob-bearing linkage not deterministic (rc=$glob_rc): $glob_err"
  FAIL=$((FAIL + 1))
fi

# No-double-flag invariant: an empty linkage value yields EMPTY-PLACEHOLDER
# *only*, never an additional BAD-LINKAGE-SHAPE. Cover BOTH empty forms — the
# quoted-empty `parent: ""` (inner-skip branch) and the bracketed-empty
# `relates_to: []` (post-bracket-strip empty $rest, a different branch).
assert_absent "empty quoted linkage does not double-flag" "BAD-LINKAGE-SHAPE" "$TMP/bad-empty.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-empty-list.md" 'relates_to: []'
assert_rejects "empty-list placeholder rejected" "EMPTY-PLACEHOLDER" "$TMP/bad-empty-list.md"
assert_absent "empty-list linkage does not double-flag" "BAD-LINKAGE-SHAPE" "$TMP/bad-empty-list.md"

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

# adr status: rejected is a vocab member (ADR-0031 lifecycle); validator accepts.
# NOTE: this is the ONLY corpus-validator coverage for `rejected` — the generic
# per-type valid-fixture loop (:37-44) cannot reach it (emit_valid pins status to
# the first vocab token, `proposed`). Do not treat this fixture as redundant.
emit_valid adr no decision_makers "rejected" "$TMP/ok-adr-rejected.md"
assert_accepts "adr status: rejected accepted" "$TMP/ok-adr-rejected.md"

# adr status: a non-member near-miss (`reject`) still rejects after the widening.
# Same single-token idiom — pass the non-member token directly, no sed.
emit_valid adr no decision_makers "reject" "$TMP/bad-adr-status.md"
assert_rejects "adr non-member status rejected" "BAD-STATUS" "$TMP/bad-adr-status.md"

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

# ---- 6. Config-driven allowlist scope -------------------------------------
echo "=== Allowlist scope (config-driven) ==="

# (a) An arbitrary unknown subtree is skipped: a corpus carrying valid files
# under configured dirs PLUS junk under unconfigured subtrees validates clean.
ALLOW="$TMP/allow-ok/meta"
mkdir -p "$ALLOW/work" "$ALLOW/announcements" "$ALLOW/random"
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$ALLOW/work/0001-real.md"
# Junk that WOULD be flagged if it were ever validated (no fence / no type).
printf '# Announcement\n\nnot a schema artifact\n' >"$ALLOW/announcements/news.md"
printf -- '---\nfoo: bar\n---\n# Random\n' >"$ALLOW/random/whatever.md"
assert_accepts "unknown subtrees skipped (allowlist) — corpus validates clean" "$ALLOW"

# (b) A paths.work override is honoured: a malformed file under the CONFIGURED
# custom dir IS flagged, while an equivalently-malformed file left at the now-
# unconfigured default meta/work/ is skipped. Asserting BOTH halves proves the
# override actually resolved rather than being silently ignored.
OVR_REPO="$TMP/allow-override"
mkdir -p "$OVR_REPO/.accelerator" "$OVR_REPO/meta/custom-work" "$OVR_REPO/meta/work"
cat >"$OVR_REPO/.accelerator/config.md" <<'EOF'
---
paths:
  work: meta/custom-work
---
EOF
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$OVR_REPO/meta/custom-work/0001-bad.md"
sed -i.bak '/^title: /d' "$OVR_REPO/meta/custom-work/0001-bad.md"
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$OVR_REPO/meta/work/0002-bad.md"
sed -i.bak '/^title: /d' "$OVR_REPO/meta/work/0002-bad.md"
rm -f "$OVR_REPO"/meta/custom-work/*.bak "$OVR_REPO"/meta/work/*.bak
ovr_rc=0
ovr_err="$(cd "$OVR_REPO" && "$VALIDATOR" "$OVR_REPO/meta" 2>&1 >/dev/null)" || ovr_rc=$?
if [ "$ovr_rc" -ne 0 ] &&
  grep -qF "custom-work/0001-bad.md" <<<"$ovr_err" &&
  grep -qF "MISSING-BASE-FIELD" <<<"$ovr_err" &&
  ! grep -qF "meta/work/0002-bad.md" <<<"$ovr_err"; then
  echo "  PASS: paths.work override flags configured custom dir, skips default meta/work"
  PASS=$((PASS + 1))
else
  echo "  FAIL: paths.work override not honoured (rc=$ovr_rc): $ovr_err"
  FAIL=$((FAIL + 1))
fi

# (c) The referential-integrity index is scoped to configured dirs: a typed ref
# to a target that lives in an out-of-scope subtree is unresolved (the target is
# never indexed) -> DANGLING-REF. Confirms the index build honours the allowlist.
SCOPED="$TMP/allow-scoped/meta"
mkdir -p "$SCOPED/reviews/work" "$SCOPED/random"
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$SCOPED/random/9999-target.md"
sed -i.bak 's/^id: "0001"$/id: "9999"/' "$SCOPED/random/9999-target.md"
emit_valid work-item-review no "reviewer verdict lenses review_number review_pass work_item_id" "complete" \
  "$SCOPED/reviews/work/0001-review-1.md" 'target: "work-item:9999"'
rm -f "$SCOPED"/random/*.bak "$SCOPED"/reviews/work/*.bak
assert_rejects "typed ref to out-of-scope target is DANGLING-REF (index scoped)" \
  "DANGLING-REF" "$SCOPED"

# (d) The doc-type table is resolved ONCE per run regardless of corpus size: a
# counting wrapper around the resolver is spawned exactly once over a multi-file
# corpus (not per file, not per walk pass).
ONCE="$TMP/allow-once/meta"
mkdir -p "$ONCE/work" "$ONCE/plans"
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$ONCE/work/0001-a.md"
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$ONCE/work/0002-b.md"
sed -i.bak 's/^id: "0001"$/id: "0002"/' "$ONCE/work/0002-b.md"
emit_valid plan yes "reviewer" "draft | ready | in-progress | done" "$ONCE/plans/2026-01-01-c.md"
rm -f "$ONCE"/work/*.bak "$ONCE"/plans/*.bak
COUNTER="$TMP/resolver-count"
: >"$COUNTER"
WRAP="$TMP/resolver-wrap.sh"
cat >"$WRAP" <<WRAPEOF
#!/usr/bin/env bash
echo x >>"$COUNTER"
exec "$SCRIPT_DIR/config-read-doc-type-paths.sh" "\$@"
WRAPEOF
chmod +x "$WRAP"
DOC_TYPE_PATHS_RESOLVER="$WRAP" "$VALIDATOR" "$ONCE" >/dev/null 2>&1 || true
assert_eq "doc-type table resolved exactly once per run (multi-file corpus)" \
  "1" "$(grep -c x "$COUNTER")"

test_summary
