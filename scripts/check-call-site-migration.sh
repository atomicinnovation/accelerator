#!/usr/bin/env bash
set -euo pipefail

# Gate for the 0167 config-command call-site migration. Proves that no retained
# file still *invokes* a removal-set config script (Grep A) and that no SKILL.md
# still names one under scripts/ (Grep B), and confines --allow-legacy-layout to
# the migration engine. The corpus is fixed here so a scope chosen at
# verification time cannot be narrowed until it passes.
#
# Grep A-functional is gated to zero OUTSIDE a PENDING_PHASE7 allowlist: the
# removal set is not deleted until Phase 7, so the four Rust dependants, the
# config-common template resolver, and the superseded shell suites/shims still
# invoke removal-set scripts until Phase 7 §2-§3 deletes or repoints them. Each
# allowlisted file carries a known-positive floor below (it MUST still contain a
# functional reference), so Phase 7 empties the allowlist rather than leaving it
# to rot.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ── The removal set (Phase 7 §1), by basename ────────────────────────────────
REMOVAL_SET_BASENAMES="
config-read-value.sh
config-read-path.sh
config-read-all-paths.sh
config-read-doc-type-paths.sh
config-read-work.sh
config-read-agents.sh
config-read-agent-name.sh
config-read-context.sh
config-read-review.sh
config-read-skill-context.sh
config-read-skill-instructions.sh
config-read-template.sh
config-list-template.sh
config-show-template.sh
config-eject-template.sh
config-diff-template.sh
config-reset-template.sh
config-dump.sh
config-summary.sh
"

# The removal set itself, plus config-common (0174) and the browser executor
# (0173), are excluded from the Grep A corpus: they are retained members whose
# self-references are not this story's to remove.
REMOVAL_SET_PATHS="
scripts/config-read-value.sh
scripts/config-read-path.sh
scripts/config-read-all-paths.sh
scripts/config-read-doc-type-paths.sh
scripts/config-read-work.sh
scripts/config-read-agents.sh
scripts/config-read-agent-name.sh
scripts/config-read-context.sh
scripts/config-read-review.sh
scripts/config-read-skill-context.sh
scripts/config-read-skill-instructions.sh
scripts/config-read-template.sh
scripts/config-list-template.sh
scripts/config-show-template.sh
scripts/config-eject-template.sh
scripts/config-diff-template.sh
scripts/config-reset-template.sh
scripts/config-dump.sh
scripts/config-summary.sh
skills/config/init/scripts/init.sh
"

# Files whose functional removal-set references belong to a later phase, not to
# this call-site cutover. Each MUST still contain a functional reference (the
# known-positive floor below), so the later phase empties this list rather than
# leaving it to rot. These are Phase 7 §2-§3 (deletions and cli/ repoints);
# Phase 6 already re-homed hooks/config-detect.sh onto the bootstrap path.
PENDING_PHASE7="
cli/config/src/catalogue.rs
cli/config-adapters/tests/parity.rs
cli/corpus-adapters/tests/common/mod.rs
scripts/config-common.sh
scripts/test-config.sh
scripts/test-config-read-doc-type-paths.sh
"

# Enumerate candidate files: tracked shell/rust/markdown under the repo, minus
# build artefacts, vendored deps, the immutable release record, and migration
# prose. cli/ stays IN the corpus.
list_corpus() {
  find . \
    \( -path './.git' -o -path './node_modules' -o -path '*/node_modules' \
    -o -path './cli/target' -o -path './meta' -o -path './docs' \
    -o -path '*/.jj' \) -prune -o \
    -type f \( -name '*.sh' -o -name '*.rs' -o -name '*.md' \) -print |
    sed 's#^\./##' |
    grep -vxF 'CHANGELOG.md'
}

is_excluded_path() { # $1 = repo-relative path — removal set or retained sibling
  case "$1" in
    scripts/config-common.sh) return 1 ;; # kept in corpus; Phase-7 refs allowlisted
    # The gates that DEFINE the removal set enumerate it as data, not as calls.
    scripts/check-inventory.sh | scripts/check-call-site-migration.sh) return 0 ;;
  esac
  printf '%s\n' "$REMOVAL_SET_PATHS" | grep -qxF "$1" && return 0
  case "$1" in
    scripts/config-read-browser-executor.sh) return 0 ;; # 0173 owns it
    scripts/test-shims/*) return 0 ;;                    # deleted with the suite
  esac
  return 1
}

is_pending_phase7() { # $1 = repo-relative path
  printf '%s\n' "$PENDING_PHASE7" | grep -qxF "$1"
}

# A line references a removal-set script *functionally* (an invocation or a
# resolved path) rather than merely mentioning its name in prose. Comments are
# classified as mentions. bash-3.2 safe.
line_is_comment() { # $1 = raw line
  case "$(printf '%s' "$1" | sed 's/^[[:space:]]*//')" in
    '#'* | '//'* | '*'*) return 0 ;;
  esac
  return 1
}

# ── Build the basename alternation once ──────────────────────────────────────
BN_ALT="$(printf '%s' "$REMOVAL_SET_BASENAMES" | grep . | sed 's/\./\\./g' |
  paste -sd '|' -)"
# Functional shapes: basename preceded by a path separator or opening quote, or
# reached through an invocation keyword / resolver helper.
FUNC_RE="([/\"']($BN_ALT))|((bash|exec|source|require_script|\.join\(|\.arg\()[^\n]*($BN_ALT))"

FUNCTIONAL_HITS=""
MENTION_COUNT=0
PENDING_SEEN=""

while IFS= read -r file; do
  [ -n "$file" ] || continue
  is_excluded_path "$file" && continue
  # grep the file for any removal-set basename; classify each hit.
  while IFS= read -r hit; do
    [ -n "$hit" ] || continue
    lineno="${hit%%:*}"
    text="${hit#*:}"
    if line_is_comment "$text"; then
      MENTION_COUNT=$((MENTION_COUNT + 1))
      continue
    fi
    if printf '%s' "$text" | grep -qE "$FUNC_RE"; then
      if is_pending_phase7 "$file"; then
        PENDING_SEEN="$PENDING_SEEN$file
"
      else
        FUNCTIONAL_HITS="$FUNCTIONAL_HITS$file:$lineno:$text
"
      fi
    else
      MENTION_COUNT=$((MENTION_COUNT + 1))
    fi
  done < <(grep -nE "($BN_ALT)" "$file" 2>/dev/null || true)
done < <(list_corpus)

FAILED=0

# ── Grep A-functional: zero outside the allowlist ────────────────────────────
if printf '%s' "$FUNCTIONAL_HITS" | grep -q .; then
  echo "FAIL: retained files functionally reference a removal-set script:" >&2
  printf '%s' "$FUNCTIONAL_HITS" | grep . | sed 's/^/  /' >&2
  FAILED=1
else
  echo "PASS: Grep A-functional — no removal-set invocations outside the Phase-7 allowlist"
fi

# ── Known-positive floor: every allowlisted file must still be a real hit ─────
while IFS= read -r pf; do
  [ -n "$pf" ] || continue
  if ! printf '%s' "$PENDING_SEEN" | grep -qxF "$pf"; then
    echo "FAIL: PENDING_PHASE7 entry no longer has a functional reference: $pf" >&2
    echo "      Phase 7 has repointed/deleted it — remove it from PENDING_PHASE7." >&2
    FAILED=1
  fi
done < <(printf '%s\n' "$PENDING_PHASE7" | grep .)

# ── Grep A-mention: reported, not gated ──────────────────────────────────────
echo "INFO: Grep A-mention — $MENTION_COUNT non-functional reference(s) to removal-set names (reported, not gated)"

# ── Grep B: no removal-set config- script named under scripts/ in a SKILL.md ──
# The only permitted `scripts/config-` matches are the browser executor (0173)
# and config-common (0174).
GREPB="$(grep -rn 'scripts/config-' --include=SKILL.md skills/ 2>/dev/null |
  grep -vE 'scripts/config-read-browser-executor\.sh|scripts/config-common\.sh' ||
  true)"
if [ -n "$GREPB" ]; then
  echo "FAIL: a SKILL.md still names a removal-set config- script under scripts/:" >&2
  printf '%s\n' "$GREPB" | sed 's/^/  /' >&2
  FAILED=1
else
  echo "PASS: Grep B — no removal-set config- script named in any SKILL.md"
fi

# ── --allow-legacy-layout confinement ────────────────────────────────────────
BADFLAG="$(grep -rln 'allow-legacy-layout' --include='*.sh' skills scripts 2>/dev/null |
  grep -vE '^skills/config/migrate/migrations/|^scripts/doc-type-table\.sh$' |
  grep -vE 'test-|check-call-site-migration\.sh$' || true)"
if [ -n "$BADFLAG" ]; then
  echo "FAIL: --allow-legacy-layout appears outside migrations/ and doc-type-table.sh:" >&2
  printf '%s\n' "$BADFLAG" | sed 's/^/  /' >&2
  FAILED=1
else
  echo "PASS: --allow-legacy-layout confined to the migration engine"
fi

exit "$FAILED"
