#!/usr/bin/env bash
# DESCRIPTION: Standalone reference fixture for the 0117 decisions bridge.
# INTERACTIVE: yes
# shellcheck disable=SC2154 # CLAUDE_PLUGIN_ROOT/PROJECT_ROOT provided by the interactive-migration harness environment
set -euo pipefail
# shellcheck source=../../../../../../../../scripts/atomic-common.sh
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
# shellcheck source=../../../../../../../../scripts/interactive-harness.sh
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

# Exactly three pinned transformations in a fixed emission order — the minimum
# that exercises one of each verb (accept / skip / edit). All rows prompt.
migration_emit_transformations() {
  harness_emit_transformation key=relates_to \
    path=meta/work/0050-example-a.md anchor=body/relates_to \
    proposed=work-item:0042 predicate_value=ambiguous
  harness_emit_transformation key=parent \
    path=meta/work/0051-example-b.md anchor=body/parent \
    proposed=work-item:0031 predicate_value=ambiguous
  harness_emit_transformation key=relates_to \
    path=meta/work/0052-example-c.md anchor=body/relates_to \
    proposed=work-item:0099 predicate_value=ambiguous
}

migration_evaluate_predicate() { return 0; } # all rows prompt

migration_validate_edit() {
  [ -n "$5" ] || {
    harness_reject "empty value not allowed"
    return 1
  }
}

# accept -> proposed; edit -> user value; skip -> not called.
# Insert `<key>: [<value>]` before the closing `---` of the file's frontmatter,
# AND append a decoupled sentinel record so AC2 can assert the decision/value
# independently of the insert mechanics (mirrors 0002-predicate's applied/log).
migration_apply_decision() {
  local key="$1" path="$2" anchor="$3" decision="$4" value="$5"
  local abs="$PROJECT_ROOT/$path"
  # Fail loudly if the target has no second '---' rather than silently no-op or
  # mis-insert (a malformed frontmatter write is a data-integrity bug, and this
  # fixture is a pattern authors may copy). POSIX-awk-safe: count '---' lines,
  # print the new key immediately before the 2nd, gawk/BSD-awk identical. Pass
  # the value via the environment (ENVIRON[]), NOT awk -v: -v assignments
  # undergo C-style escape processing (a backslash in the value would be
  # transformed), whereas ENVIRON[] is value-transparent on both BSD and gawk.
  key_line="$key: [$value]" awk '
    /^---$/ { n++; if (n == 2) { print ENVIRON["key_line"]; seen2 = 1 } }
    { print }
    END { if (!seen2) exit 3 }
  ' "$abs" >"$abs.tmp" || {
    harness_reject "no closing --- in $path"
    return 1
  }
  mv "$abs.tmp" "$abs" || {
    rm -f "$abs.tmp"
    harness_reject "rename failed for $path"
    return 1
  }
  mkdir -p "$PROJECT_ROOT/.fixture/applied"
  printf '%s\t%s\t%s\t%s\t%s\n' "$key" "$path" "$anchor" "$decision" "$value" \
    >>"$PROJECT_ROOT/.fixture/applied/log"
}

harness_run
