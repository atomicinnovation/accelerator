#!/usr/bin/env bash
set -euo pipefail

# Usage: pr-update-body.sh <pr-number> <body-file>
# Posts the contents of <body-file> as the body of pull request
# <pr-number> on the base (upstream) repository, using the GitHub REST
# API (PATCH /repos/{owner}/{repo}/pulls/{number}).
#
# Precondition: <body-file> MUST already have YAML frontmatter stripped.
# This helper does not strip frontmatter — that is SKILL.md's
# responsibility per the existing recipe in describe-pr/SKILL.md
# sub-steps 1-4 of step 9.
#
# Exit codes:
#   0   success
#   1   encode failed (jq could not read or encode the body file)
#   2   usage error (wrong arg count, missing body file, missing jq)
#   4   PATCH failed (gh api error against the GitHub REST endpoint)
#   other  bubbled up from pr-base-repo.sh
#          (1 = resolution failed, 2 = resolver usage / missing jq)
#
# Implementation arrives in Phase 3.

echo "pr-update-body.sh: not yet implemented (see meta/plans/2026-05-15-0059-...)" >&2
exit 1
