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
#   1   encode failed (jq could not read or encode the body file) OR
#       bubbled up from pr-base-repo.sh (resolution failed)
#   2   usage error (wrong arg count, missing body file, missing jq) OR
#       bubbled up from pr-base-repo.sh (resolver usage / missing jq)
#   4   PATCH failed (gh api error against the GitHub REST endpoint)
#
# Conventions:
# - Body encoded as JSON via `jq -Rs '{body: .}'` to a tempfile so
#   encode and PATCH failures can be distinguished cleanly.
# - PATCH targets the base (upstream) repo per the shared resolver.
# - Explicit `--method PATCH` (gh api's default is GET, or POST when a
#   body is present — relying on the default is brittle).

if [ $# -ne 2 ]; then
  echo "Usage: pr-update-body.sh <pr-number> <body-file>" >&2
  exit 2
fi

pr_number="$1"
body_file="$2"

if [ ! -f "$body_file" ]; then
  echo "pr-update-body.sh: body file not found: $body_file" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "pr-update-body.sh: jq is required (install via Homebrew, apt, or mise)" >&2
  exit 2
fi

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
resolver="$script_dir/../../scripts/pr-base-repo.sh"

# Capture the resolver's exit code explicitly so callers can
# distinguish resolver-usage (2) from resolver-resolution (1) failures.
if base_repo=$("$resolver" "$pr_number"); then
  :
else
  resolver_rc=$?
  # Resolver already emitted its own stderr (preserved gh stderr +
  # conditional hint). Re-emit a contextual line and propagate the
  # resolver's exit code verbatim.
  echo "pr-update-body.sh: base-repo resolution failed for PR #$pr_number." >&2
  exit "$resolver_rc"
fi

# Allocate one tempdir for all stage artefacts so a single trap
# covers cleanup unconditionally, including any mktemp-failure path.
stage_dir=$(mktemp -d)
trap 'rm -rf "$stage_dir"' EXIT
payload_file="$stage_dir/payload"
encode_err="$stage_dir/encode.err"
patch_err="$stage_dir/patch.err"

if ! jq -Rs '{body: .}' <"$body_file" >"$payload_file" 2>"$encode_err"; then
  if [ -s "$encode_err" ]; then
    cat "$encode_err" >&2
  fi
  echo "pr-update-body.sh: encode failed for $body_file (could not read or JSON-encode the file)." >&2
  exit 1
fi

if ! gh api --method PATCH "repos/$base_repo/pulls/$pr_number" --input "$payload_file" 2>"$patch_err"; then
  if [ -s "$patch_err" ]; then
    cat "$patch_err" >&2
  fi
  echo "pr-update-body.sh: PATCH failed for repos/$base_repo/pulls/$pr_number." >&2
  exit 4
fi
