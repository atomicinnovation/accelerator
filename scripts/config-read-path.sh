#!/usr/bin/env bash
set -euo pipefail

# Reads a path configuration value.
# Usage: config-read-path.sh <path_key> [default]
#
# When [default] is omitted or empty (both treated identically), the
# plugin-standard default for the key is looked up from config-defaults.sh.
# An explicit non-empty [default] takes precedence (backward compatible).
#
# If the key is not found in PATH_KEYS and no explicit [default] is provided,
# a warning is printed to stderr and the output is empty. Callers that need
# a non-empty fallback for unknown keys should supply an explicit [default].
#
# Path keys and their plugin-standard defaults are defined in config-defaults.sh.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=config-defaults.sh
source "$SCRIPT_DIR/config-defaults.sh"
# shellcheck source=vcs-common.sh
# Sourced for find_repo_root, used by the migration-aware warning's
# cheap-gate below.
source "$SCRIPT_DIR/vcs-common.sh"

key="${1:-}"
if [ -z "$key" ]; then
  echo "Usage: config-read-path.sh <path_key> [default]" >&2
  exit 1
fi

if [ -n "${2:-}" ]; then
  default="${2}"
else
  default=""
  for i in "${!PATH_KEYS[@]}"; do
    if [ "${PATH_KEYS[$i]}" = "paths.${key}" ]; then
      default="${PATH_DEFAULTS[$i]}"
      break
    fi
  done
  if [ -z "$default" ]; then
    case "$key" in
      design_inventories|design_gaps)
        # Defensive: a caller (skill author, external script) is still
        # invoking the legacy bare key. The in-tree call sites were
        # renamed; this only fires for out-of-tree callers.
        echo "config-read-path.sh: warning: key '${key}' was renamed by migration 0004 to 'research_${key}'; run /accelerator:migrate" >&2
        ;;
      *)
        echo "config-read-path.sh: warning: unknown key '${key}' — no centralized default" >&2
        ;;
    esac
  fi
fi

# Pre-migration user check: when the canonical key is requested, probe
# the user's config for the legacy alias. If present, their override is
# silently being ignored — emit a warning naming the ignored key.
case "$key" in
  research_design_inventories|research_design_gaps)
    legacy="${key#research_}"
    project_root="$(find_repo_root 2>/dev/null || true)"
    if [ -n "$project_root" ] && \
       grep -qF "$legacy" \
         "$project_root/.accelerator/config.md" \
         "$project_root/.accelerator/config.local.md" 2>/dev/null; then
      legacy_value=$(bash "$SCRIPT_DIR/config-read-value.sh" "paths.${legacy}" "" 2>/dev/null || true)
      if [ -n "$legacy_value" ]; then
        echo "config-read-path.sh: warning: your config sets 'paths.${legacy}' (renamed by migration 0004 to 'paths.${key}'); the legacy override is being ignored. Run /accelerator:migrate" >&2
      fi
    fi
    ;;
esac

exec "$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "${default}"
