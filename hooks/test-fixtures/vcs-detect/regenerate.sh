#!/usr/bin/env bash
set -euo pipefail

# Regenerate AC5 golden snapshots for vcs-detect.sh.
#
# Pre-conditions:
#   - jj, git, jq, realpath on PATH (via `mise install` from repo root)
#   - hooks/vcs-detect.sh and scripts/vcs-common.sh are in the pre-0058
#     state (verified against CAPTURE-SOURCE.txt by the AC5 test).
#
# Determinism guarantees:
#   - TMPDIR is explicitly /tmp (or realpath-resolved). macOS
#     /var/folders -> /private/var symlinks are normalised so the
#     resulting JSON does not embed host-specific path artefacts.
#   - GIT_CEILING_DIRECTORIES scopes git's upward discovery to the
#     temp dir, preventing accidental walks into the accelerator's
#     own .git or any ancestor repo.
#   - Fixture builders match the test-suite fixture builders exactly
#     (same `jj git init` invocation, same `git init -q + commit
#     --allow-empty` shape) so capture and replay produce identical
#     stdout.
#
# Linux is the canonical capture host. Snapshots captured on macOS
# may diverge in path normalisation; CI runs Linux only.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
HOOK="$PLUGIN_ROOT/hooks/vcs-detect.sh"

TMPDIR=/tmp
WORK=$(mktemp -d "$TMPDIR/vcs-detect-capture-XXXXXX")
export GIT_CEILING_DIRECTORIES="$WORK"
trap 'rm -rf "$WORK"' EXIT

# Main jj workspace.
WORKDIR="$WORK/main-jj" && mkdir -p "$WORKDIR" && (cd "$WORKDIR" && jj git init --quiet)
(cd "$WORKDIR" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK") \
  > "$SCRIPT_DIR/main-jj-workspace.json"

# Main git checkout (with one empty commit, matching the test fixture).
WORKDIR="$WORK/main-git" && mkdir -p "$WORKDIR"
(cd "$WORKDIR" && git init -q && git config user.email t@e.x && git config user.name T)
(cd "$WORKDIR" && git commit --allow-empty -q -m init)
(cd "$WORKDIR" && CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$HOOK") \
  > "$SCRIPT_DIR/main-git-checkout.json"

# Record source provenance so the AC5 test can verify the snapshots
# match the production code state they were captured against.
{
  printf 'hooks/vcs-detect.sh: '
  (cd "$PLUGIN_ROOT" && git log -n1 --format=%H hooks/vcs-detect.sh 2>/dev/null \
    || jj log -r 'latest(::@ & file("hooks/vcs-detect.sh"))' --no-graph -T 'commit_id' 2>/dev/null \
    || echo UNKNOWN)
  printf '\nscripts/vcs-common.sh: '
  (cd "$PLUGIN_ROOT" && git log -n1 --format=%H scripts/vcs-common.sh 2>/dev/null \
    || jj log -r 'latest(::@ & file("scripts/vcs-common.sh"))' --no-graph -T 'commit_id' 2>/dev/null \
    || echo UNKNOWN)
  printf '\nCaptured: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'Host: %s\n' "$(uname -s)"
} > "$SCRIPT_DIR/CAPTURE-SOURCE.txt"

echo "Captured snapshots into $SCRIPT_DIR"
