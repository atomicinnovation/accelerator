#!/usr/bin/env bash
set -euo pipefail

# SessionStart config-detection hook. Execs the launcher's config-summary
# renderer in hook mode; the launcher owns the emptiness test and the
# additionalContext envelope, and --fail-safe keeps an unreadable or
# legacy-layout config silently context-free rather than failing the session.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
ACCELERATOR="${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}"

exec "$ACCELERATOR" config summary --format=hook --fail-safe
