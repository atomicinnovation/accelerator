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
# Implementation arrives in Phase 2.

echo "pr-base-repo.sh: not yet implemented (see meta/plans/2026-05-15-0059-...)" >&2
exit 1
