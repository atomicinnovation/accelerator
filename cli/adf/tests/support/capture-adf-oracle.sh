#!/usr/bin/env bash
# Deliberately no -e: the render/assemble abort and reject cases exit non-zero
# by design (that is the oracle status being frozen), so each driver's exit
# code is captured explicitly rather than aborting the whole run.
set -uo pipefail

# capture-adf-oracle.sh — freeze the bash ADF pipeline's observed output.
#
# Walks every case directory under tests/fixtures/adf/, runs the matching bash
# driver over the case input, and writes the observed result beside it:
#   oracle.out         raw stdout bytes
#   oracle.err         raw stderr bytes (the notice case reads this)
#   oracle-status.txt  the driver's exit status
# render-* cases run jira-adf-to-md.sh unseeded over adf.json; assemble-* cases
# run jira-md-to-adf.sh with JIRA_ADF_LOCALID_SEED=1 over input.md.
#
# The frozen corpus is the differential's oracle once the drivers are deleted.
# Kept for provenance and regeneration: re-run against the bash drivers at the
# revision recorded in oracle-manifest.txt; the corpus is never regenerated
# from this crate's own output.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
FIXTURES="$SCRIPT_DIR/../fixtures/adf"
ADF_TO_MD="$PLUGIN_ROOT/skills/integrations/jira/scripts/jira-adf-to-md.sh"
MD_TO_ADF="$PLUGIN_ROOT/skills/integrations/jira/scripts/jira-md-to-adf.sh"

fail() {
  printf 'capture-adf-oracle: %s\n' "$1" >&2
  exit 1
}

command -v bash >/dev/null 2>&1 || fail "bash is required"
command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v awk >/dev/null 2>&1 || fail "awk is required"
[ -x "$ADF_TO_MD" ] || fail "missing driver: $ADF_TO_MD"
[ -x "$MD_TO_ADF" ] || fail "missing driver: $MD_TO_ADF"
[ -d "$FIXTURES" ] || fail "missing fixtures: $FIXTURES"

driver_revision() {
  local path="$1"
  (cd "$PLUGIN_ROOT" &&
    jj log -r "latest(::@ & files(\"$path\"))" --no-graph -T commit_id 2>/dev/null ||
    git log -n1 --format=%H "$path" 2>/dev/null ||
    echo UNKNOWN)
}

printf 'capture-adf-oracle: freezing the bash ADF oracle\n'
printf '  jira-adf-to-md.sh: %s\n' "$(driver_revision skills/integrations/jira/scripts/jira-adf-to-md.sh)"
printf '  jira-md-to-adf.sh: %s\n' "$(driver_revision skills/integrations/jira/scripts/jira-md-to-adf.sh)"
printf '  captured: %s on %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -s)"
printf '\n'

captured=0
for case_dir in "$FIXTURES"/*/; do
  case_dir="${case_dir%/}"
  name="$(basename "$case_dir")"

  case "$name" in
    render-*)
      input="$case_dir/adf.json"
      driver="$ADF_TO_MD"
      seed=""
      ;;
    assemble-*)
      input="$case_dir/input.md"
      driver="$MD_TO_ADF"
      seed="1"
      ;;
    *)
      fail "unclassifiable case: $name (expected render-*/assemble-*)"
      ;;
  esac

  [ -f "$input" ] || fail "$name: missing input $input"

  if [ -n "$seed" ]; then
    JIRA_ADF_LOCALID_SEED="$seed" bash "$driver" \
      <"$input" >"$case_dir/oracle.out" 2>"$case_dir/oracle.err"
  else
    bash "$driver" \
      <"$input" >"$case_dir/oracle.out" 2>"$case_dir/oracle.err"
  fi
  status=$?
  printf '%s\n' "$status" >"$case_dir/oracle-status.txt"

  printf '  %-52s status=%s bytes=%s\n' \
    "$name" "$status" "$(wc -c <"$case_dir/oracle.out" | tr -d ' ')"
  captured=$((captured + 1))
done

printf '\ncapture-adf-oracle: froze %s cases\n' "$captured"
