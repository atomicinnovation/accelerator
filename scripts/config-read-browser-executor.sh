#!/usr/bin/env bash
set -euo pipefail

# Resolves the absolute path of the Playwright executor (run.sh) for
# preloading into browser-agent contexts. Mirrors the shape of
# scripts/config-read-all-paths.sh.
#
# Note: the executor path is also referenced in the inventory-design
# SKILL.md `allowed-tools` glob. If you move run.sh, both this script and
# that glob need to update in lockstep. The `test -x` check below ensures
# this script fails loudly if the file moves without a coordinated edit.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Override hook for tests.
RUN_SH="${ACCELERATOR_BROWSER_EXECUTOR_OVERRIDE:-$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/run.sh}"

if [ ! -x "$RUN_SH" ]; then
  echo "config-read-browser-executor.sh: run.sh not found or not executable at $RUN_SH" >&2
  exit 1
fi

echo "## Browser Executor"
echo ""
echo "- browser-executor-script: $RUN_SH"
