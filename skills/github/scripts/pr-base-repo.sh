#!/usr/bin/env bash
set -euo pipefail

# Usage: pr-base-repo.sh <pr-number>
# Prints "<owner>/<name>" of the base (upstream) repository for the given
# pull request to stdout. Cross-fork-safe: resolves via
# `gh pr view --json baseRepository`, not `gh repo view`. Used by
# describe-pr (for PATCHing the body), review-pr, and respond-to-pr.
#
# Exit codes:
#   0  success
#   1  resolution failed (auth, network, 404, malformed JSON, ...)
#   2  usage error (wrong arg count, missing jq, ...)
#
# Conventions:
# - Cross-fork-safe: resolves via `gh pr view --json baseRepository`.
#   `gh repo view` returns the local checkout's repo (the fork, for
#   contributors), which is wrong for cross-fork PR operations.
# - Preserves the underlying gh stderr on failure so callers see the
#   real cause; emits a conditional `gh repo set-default` remediation
#   only when the captured stderr matches the known phrase.
# - Validates that owner.login and name are non-empty so a degenerate
#   gh response can't smuggle "null/null" downstream.
#
# Invocation: must be run as a subprocess (e.g. via command
# substitution or direct execution). The EXIT trap on the internal
# err_file would clobber a caller's own EXIT trap if this script were
# `source`d. All current callers spawn a subshell, which is safe.

if [ $# -ne 1 ]; then
  echo "Usage: pr-base-repo.sh <pr-number>" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "pr-base-repo.sh: jq is required (install via Homebrew, apt, or mise)" >&2
  exit 2
fi

pr_number="$1"

# Resolve the base (upstream) repo. Capture stderr to a tempfile so we
# can replay it on failure rather than silently substituting our own
# remediation hint.
err_file=$(mktemp)
trap 'rm -f "$err_file"' EXIT

if ! payload=$(gh pr view "$pr_number" --json baseRepository 2>"$err_file"); then
  if [ -s "$err_file" ]; then
    cat "$err_file" >&2
  fi
  echo "pr-base-repo.sh: could not resolve base repo for PR #$pr_number." >&2
  if grep -q "no default remote repository" "$err_file"; then
    echo "  Run 'gh repo set-default' and select the appropriate repository." >&2
  fi
  exit 1
fi

# Pre-validate that gh returned parseable JSON. Without this, a
# malformed response (HTML auth-nag, plain-text proxy error) reaches
# the jq calls below and produces opaque parse errors instead of the
# helper's own remediation.
if ! jq -e . >/dev/null 2>&1 <<<"$payload"; then
  echo "pr-base-repo.sh: gh returned non-JSON output for PR #$pr_number." >&2
  echo "  Raw payload: $payload" >&2
  exit 1
fi

owner=$(jq -r '.baseRepository.owner.login // ""' <<<"$payload")
name=$(jq -r '.baseRepository.name // ""' <<<"$payload")

if [ -z "$owner" ] || [ -z "$name" ]; then
  echo "pr-base-repo.sh: baseRepository.owner.login or .name was empty/null in gh response." >&2
  echo "  Raw payload: $payload" >&2
  exit 1
fi

printf '%s/%s\n' "$owner" "$name"
