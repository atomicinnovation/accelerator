#!/usr/bin/env bash
# set -e intentionally omitted so a failing assertion does not abort
# the harness — assertion failures tally into FAIL and the suite runs
# to completion. Mirrors the convention from sibling test scripts.
set -uo pipefail

# Real-gh smoke harness for skills/github/scripts/pr-base-repo.sh.
# Asserts every --json field the resolver requests is in the allowlist
# printed by `gh pr view --json INVALID` on the installed gh.
#
# Skipped (does not fail) if `gh` is not on PATH.
#
# Picked up automatically by run_shell_suites in
# tasks/test/helpers.py (globs **/test-*.sh under skills/github/).
#
# Rationale: the sibling PATH-stubbed harness in
# `test-pr-base-repo-scripts.sh` dispatches on `$1 $2` alone and cannot
# detect cases where the resolver requests a --json field that is not
# in the installed gh's allowlist. Work item 0071 documents the
# specific defect this catches.
#
# Why probe via `gh pr view --json INVALID` rather than scrape
# `gh pr view --help`: gh's error path emits a structured allowlist
# (`Unknown JSON field: "INVALID"\nAvailable fields: ...`) that is the
# same surface the resolver itself hits at runtime, so what passes
# this check is guaranteed to be accepted at runtime. The help text,
# by contrast, mentions short field tokens like `url` in flag
# descriptions outside the allowlist, producing false PASSes on
# real regressions.
#
# github.com-only: the probe targets the gh-cli project's own repo
# (cli/cli) to force gh past argv/repo-discovery into field validation.
# Operators with `GH_HOST` set to a GitHub Enterprise host will see
# the harness SKIP (via the marker-sanity guard below) rather than
# falsely FAIL — see the probe-invocation comment block for details.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
# shellcheck source=/dev/null
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
# Intentionally do NOT source skills/github/scripts/test-helpers.sh —
# this harness exercises the real gh, not the PATH stub.

SCRIPT="$SCRIPT_DIR/pr-base-repo.sh"

echo "=== pr-base-repo.sh real-gh smoke ==="

if ! command -v gh >/dev/null 2>&1; then
  skip_test "real-gh smoke" "gh not on PATH"
  test_summary
  exit 0
fi

# Extract every --json field the resolver requests from its source.
# Greps for `--json <token>` and emits the token. If the resolver
# evolves to request multiple fields in one call (e.g.
# `--json url,state`), the comma-split handles it. The field-name
# character class [A-Za-z][A-Za-z0-9_,]* is safe to interpolate into
# the ERE allowlist regex below without escaping.
fields=$(
  grep -oE -- "--json [A-Za-z][A-Za-z0-9_,]*" "$SCRIPT" \
    | awk '{print $2}' \
    | tr ',' '\n' \
    | sort -u
)

if [ -z "$fields" ]; then
  echo "  FAIL: real-gh smoke: no --json fields extracted from $SCRIPT"
  echo "    (resolver source may have refactored to a variable; see"
  echo "     meta/work/0071-*.md and pr-base-repo.sh header for context)"
  FAIL=$((FAIL + 1))
  test_summary
  exit 1
fi

# Probe gh's allowlist by deliberately requesting an invalid field.
# Capture stderr only — stdout is empty on this error path. Use a
# field name so unusual it cannot collide with a real gh field.
#
# Why pass `1 --repo cli/cli`: `gh pr view` validates positional args
# and resolves the repo context before validating the --json field set.
# Without a PR number we'd hit `accepts 1 arg(s)`; without --repo we'd
# hit `no git remotes found` in CI / outside git checkouts. Passing a
# known-stable public repo + an arbitrary PR number forces gh past
# the argv/repo-discovery checks and into field validation, which is
# the surface we actually want to probe. The PR may or may not exist
# on cli/cli — irrelevant, because field validation is parse-time and
# fires before network fetch.
#
# github.com-only by design: cli/cli is the gh-cli project's own repo.
# Operators with `GH_HOST` set to a GitHub Enterprise host won't have
# access to cli/cli; the probe will then hit a repo-resolution error
# *before* reaching field validation, the marker-sanity check below
# will fail to find `Unknown JSON field`, and the harness SKIPs with
# diagnostic stderr. This is the documented degradation path for GHE
# — see the GHE manual-verification step in the plan.
PROBE_FIELD='__ACCEL_PROBE__'
probe_stderr=$(gh pr view 1 --repo cli/cli --json "$PROBE_FIELD" 2>&1 1>/dev/null || true)

# Format-sanity check: the probe MUST emit gh's canonical
# `Unknown JSON field` marker. If it doesn't, gh either reached a
# different error path (auth missing, network down) or has changed
# its error format. Either way, we cannot reliably parse an allowlist,
# so SKIP rather than emit field FAILs with misleading attribution.
if ! grep -q "Unknown JSON field" <<<"$probe_stderr"; then
  skip_test "real-gh smoke" \
    "gh did not emit expected 'Unknown JSON field' marker — auth, network, or error-format issue. Captured stderr:
$probe_stderr
(see meta/work/0071-*.md and pr-base-repo.sh header for context)"
  test_summary
  exit 0
fi

# gh's "Unknown JSON field" error includes a list of valid fields,
# typically rendered as a comma-separated allowlist on subsequent
# lines. Strip the marker line, then collapse the rest into a
# single newline-delimited token stream we can grep against.
# `awk '{print $1}'` takes the first whitespace-delimited token of
# each line — gh may render the allowlist as `Specify one of: a, b, c`
# (commas converted to newlines, awk strips trailing prose) or as a
# bullet-per-line list (awk strips any non-token prefix). The first
# token per line after comma-splitting is the field identifier.
allowlist_tokens=$(
  printf '%s\n' "$probe_stderr" \
    | grep -v "Unknown JSON field" \
    | tr ',' '\n' \
    | awk '{print $1}' \
    | grep -E '^[A-Za-z][A-Za-z0-9_]*$' \
    | sort -u
)

# Control-field check: assert a known-stable gh field is in the parsed
# allowlist. If our parser is broken (gh changed its rendering, the
# stripping logic dropped real tokens), this assertion catches the
# parser bug before it manifests as misleading per-field FAILs.
CONTROL_FIELD='number'
if ! printf '%s\n' "$allowlist_tokens" | grep -qx -- "$CONTROL_FIELD"; then
  echo "  FAIL: real-gh smoke: parser sanity check — control field '$CONTROL_FIELD' not in parsed allowlist"
  echo "    (parser may be broken; gh's error format may have changed)"
  echo "    Captured stderr:"
  printf '%s\n' "$probe_stderr" | sed 's/^/      /'
  echo "    Parsed tokens:"
  printf '%s\n' "$allowlist_tokens" | sed 's/^/      /'
  echo "    (see meta/work/0071-*.md and pr-base-repo.sh header)"
  FAIL=$((FAIL + 1))
  test_summary
  exit 1
fi

for field in $fields; do
  if printf '%s\n' "$allowlist_tokens" | grep -qx -- "$field"; then
    echo "  PASS: real-gh smoke: '$field' in gh's allowlist"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: real-gh smoke: '$field' NOT in installed gh's allowlist"
    echo "    (installed gh: $(gh --version 2>/dev/null | head -n 1))"
    echo "    (resolver: $SCRIPT)"
    echo "    (see meta/work/0071-*.md and pr-base-repo.sh header;"
    echo "     a gh upgrade may have dropped this field from --json)"
    FAIL=$((FAIL + 1))
  fi
done

test_summary
