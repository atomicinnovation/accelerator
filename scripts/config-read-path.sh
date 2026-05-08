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
    echo "config-read-path.sh: warning: unknown key '${key}' — no centralized default" >&2
  fi
fi

exec "$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "${default}"
