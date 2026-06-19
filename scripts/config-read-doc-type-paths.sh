#!/usr/bin/env bash
set -euo pipefail

# Resolve each schema doc-type's configured corpus directory and emit one
# machine-readable `type<TAB>resolved-dir` line per doc-type (no markdown
# legend — consumers parse with `IFS=$'\t' read -r`). The type→path-key link
# comes from config-defaults.sh (DOC_TYPE_NAMES / DOC_TYPE_PATH_KEYS); each key
# is resolved through config-read-value.sh, honouring .accelerator/config.md
# then config.local.md, falling back to the registry default.
#
# Usage:
#   config-read-doc-type-paths.sh [project-root]
#
# config-read-value.sh resolves config strictly from its CWD (it has no root
# parameter). When a project-root argument is given, this script runs each
# config read inside a `( cd "$root" && … )` subshell so resolution is against
# the corpus root, not the caller's CWD. With no argument it resolves against
# the current directory (the validator's behaviour).
#
# Value hardening — the resolved dirs are consumed directly by the matcher and,
# in the migration, scope in-place mutation, so degenerate values are
# neutralised here:
#   - A present-but-blank key (config-read-value.sh returns "") falls back to the
#     registry default and emits a stderr note. Blanking a path does NOT disable
#     a doc-type (unsupported) — all 13 rows are always emitted.
#   - A value failing an assert_safe_relpath-equivalent check (a `..` segment, a
#     leading `/`, or `.`/empty) aborts non-zero, naming the key — stopping a
#     traversal/absolute override from widening the in-place mutation set.
#   - A value containing a tab or newline (which would corrupt the TSV line)
#     aborts non-zero, naming the key.
#   - Each surviving dir is normalised: leading `./` stripped, trailing `/`
#     stripped, repeated `/` collapsed — Phase-2 longest-match / segment
#     anchoring assume clean dirs.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=config-common.sh
source "$SCRIPT_DIR/config-common.sh"

# Byte-stable across host locales (parity with the validator's LANG=C
# discipline) — the tab/newline scan and `tr -s` operate on bytes.
export LC_ALL=C

ROOT="${1:-}"

TAB=$'\t'
NL=$'\n'

# Resolve one path key honouring config; against ROOT when given, else CWD.
resolve_key() {
  local key="$1" default="$2"
  if [ -n "$ROOT" ]; then
    (cd "$ROOT" && "$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "$default")
  else
    "$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "$default"
  fi
}

# Registry default for a bare path key (matched as "paths.<key>" in PATH_KEYS).
default_for_key() {
  local key="$1" i
  for i in "${!PATH_KEYS[@]}"; do
    if [ "${PATH_KEYS[$i]}" = "paths.${key}" ]; then
      printf '%s' "${PATH_DEFAULTS[$i]}"
      return 0
    fi
  done
  return 0
}

# Normalise a resolved dir: collapse repeated slashes, strip a leading ./ and a
# trailing /. No ${var//} slash replacement (bash-3.2 / macOS hazard); tr -s is
# byte-safe under LC_ALL=C.
normalise_dir() {
  local d="$1"
  d="$(printf '%s' "$d" | tr -s '/')"
  d="${d#./}"
  d="${d%/}"
  printf '%s' "$d"
}

for i in "${!DOC_TYPE_NAMES[@]}"; do
  type="${DOC_TYPE_NAMES[$i]}"
  key="${DOC_TYPE_PATH_KEYS[$i]}"
  default="$(default_for_key "$key")"
  raw="$(resolve_key "$key" "$default")"

  if [ -z "$raw" ]; then
    printf '%s\n' \
      "config-read-doc-type-paths.sh: note: paths.${key} is blank; using default '${default}' (blanking a path does not disable a doc-type)" >&2
    raw="$default"
  fi

  case "$raw" in
    *"$TAB"* | *"$NL"*)
      printf '%s\n' \
        "config-read-doc-type-paths.sh: error: paths.${key} value contains a tab or newline" >&2
      exit 1
      ;;
  esac

  case "$raw" in
    '' | . | .. | / | /* | */.. | ../* | */../* | */./*)
      printf '%s\n' \
        "config-read-doc-type-paths.sh: error: paths.${key} resolves to an unsafe path: ${raw}" >&2
      exit 1
      ;;
  esac

  printf '%s\t%s\n' "$type" "$(normalise_dir "$raw")"
done
