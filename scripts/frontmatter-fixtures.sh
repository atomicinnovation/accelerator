#!/usr/bin/env bash
# Shared frontmatter fixture synthesiser + validator-assertion helpers.
#
# Single source for the "synthesise a minimal valid artifact, then drive the
# corpus validator over it" pattern. Sourced by BOTH:
#   - scripts/test-validate-corpus-frontmatter.sh   (validator behaviour suite)
#   - scripts/test-skill-frontmatter-conformance.sh (producer-conformance guard)
# so the two suites share one fixture authority — a schema tightening lands in
# one place, not two.
#
# Named WITHOUT a `test-` prefix so the run_shell_suites() discovery glob
# (scripts/**/test-*.sh) never tries to execute it as a suite.
#
# Pure functions, no top-level side effects, bash 3.2-safe (no associative
# arrays). Preconditions the sourcing suite must satisfy BEFORE calling these:
#   - source scripts/test-helpers.sh            (PASS / FAIL counters)
#   - source scripts/frontmatter-emission-rules.sh  (FM_OPTIONAL_EXTRAS)
#   - set VALIDATOR=<path to validate-corpus-frontmatter.sh>

# ---- Fixture generation ---------------------------------------------------
# Emit a minimal *valid* artifact for a schema row. Required (non-optional)
# extras get a non-empty placeholder; anchored types get the provenance bundle;
# typed-linkage keys are omitted (omit-when-empty), so no referential targets
# are needed. Extra frontmatter lines may be appended via $extra_lines (a
# newline-separated string injected before the closing fence).
#
# $4 (vocab) is split on `|` and the FIRST token becomes the status value, so a
# caller can pin a specific status by passing it as a single-token "vocab"
# (e.g. "done") — this is how the conformance guard injects a skill's extracted
# status literal.
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
      # shellcheck disable=SC2154 # FM_OPTIONAL_EXTRAS populated by the sourced frontmatter-emission-rules.sh (see header preconditions)
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
  # shellcheck disable=SC2154 # VALIDATOR set by the caller before sourcing (see header preconditions)
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
    # shellcheck disable=SC2001 # anchored whole-line sed indent that ${var//.../...} cannot express
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
    # shellcheck disable=SC2001 # anchored whole-line sed indent that ${var//.../...} cannot express
    echo "$VALIDATOR_ERR" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi
}
