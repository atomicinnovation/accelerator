#!/usr/bin/env bash
set -euo pipefail

# Regenerates expected.json for each work-item-sync-baseline case-* fixture
# from the live bash scripts — never from the Rust port. This corpus is the
# classification-stability oracle a Rust rewrite of the digest recipes must
# reproduce byte for byte.
#
# Must be regenerated while work-item-project-remote.sh still exists and
# implements the recipe this corpus pins; frozen thereafter, since the
# script that generates the corpus and the script the corpus is meant to
# hold to the same standard cannot both be replaced by the same change
# without the comparison becoming circular.
#
# Each case directory holds:
#   local.md           the work item (when the case exercises the local
#                       recipe)
#   remote.json         the tracker `show` payload (when the case exercises
#                       the remote recipe)
#   integration.txt     "jira" or "linear" (remote cases only)
#   expected.json        this script's output
#
# For a local.md that does not open with a fence or is never closed, bash
# exits non-zero and prints nothing; expected.json then records
# {"error": true} rather than a hash, since there is nothing else to assert.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPTS_DIR/../../.." && pwd)"
NORMALISE="$SCRIPTS_DIR/work-item-normalise.sh"
PROJECT="$SCRIPTS_DIR/work-item-project-remote.sh"
# shellcheck source=scripts/hash-common.sh
source "$PLUGIN_ROOT/scripts/hash-common.sh"

for case_dir in "$SCRIPT_DIR"/case-*/; do
  case_dir="${case_dir%/}"
  name="$(basename "$case_dir")"

  if [ -f "$case_dir/local.md" ]; then
    if local_hash=$(bash "$NORMALISE" "$case_dir/local.md" 2>/dev/null | hash_sha256_stdin); then
      printf '{"local_hash":"%s"}\n' "$local_hash" >"$case_dir/expected.json"
    else
      printf '{"error":true}\n' >"$case_dir/expected.json"
    fi
    echo "regenerated $name (local)"
  fi

  if [ -f "$case_dir/remote.json" ]; then
    integration=$(cat "$case_dir/integration.txt")
    updated=$(bash "$PROJECT" --integration "$integration" updated <"$case_dir/remote.json")
    remote_hash=$(
      bash "$PROJECT" --integration "$integration" body <"$case_dir/remote.json" |
        bash "$NORMALISE" --stdin | hash_sha256_stdin
    )
    printf '{"integration":"%s","updated":"%s","remote_hash":"%s"}\n' \
      "$integration" "$updated" "$remote_hash" >"$case_dir/expected.json"
    echo "regenerated $name (remote)"
  fi
done
