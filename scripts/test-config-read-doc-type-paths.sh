#!/usr/bin/env bash
set -euo pipefail

# Test harness for scripts/config-read-doc-type-paths.sh (the doc-type directory
# resolver) and the type→path-key registry coherence guard.
# Run: bash scripts/test-config-read-doc-type-paths.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SHIM_DIR="$SCRIPT_DIR/test-shims"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
export LC_ALL=C

# The doc-type resolver is exercised through the compiled launcher via its shim.
# Build it once if the caller did not supply a path.
if [ -z "${ACCELERATOR_BIN:-}" ]; then
  cargo build --quiet --manifest-path "$PLUGIN_ROOT/cli/Cargo.toml" \
    --bin accelerator
  ACCELERATOR_BIN="$PLUGIN_ROOT/cli/target/debug/accelerator"
fi
export ACCELERATOR_BIN

RESOLVER="$SHIM_DIR/config-read-doc-type-paths.sh"
SCHEMA_TSV="$SCRIPT_DIR/templates-schema.tsv"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# Resolved dir for a given type out of a TSV blob.
dir_of() { printf '%s\n' "$1" | awk -F'\t' -v t="$2" '$1 == t { print $2 }'; }
row_count() { printf '%s\n' "$1" | grep -c .; }

# Write a minimal .accelerator/config.md overriding a single paths.<key>.
write_config() { # $1=repo $2=key $3=value (may be empty)
  mkdir -p "$1/.accelerator"
  {
    printf -- '---\n'
    printf 'paths:\n'
    printf '  %s: %s\n' "$2" "$3"
    printf -- '---\n'
  } >"$1/.accelerator/config.md"
}

# ---- 1. Default resolution -------------------------------------------------
echo "=== Default resolution (no config) ==="
DEFREPO="$TMP/default"
mkdir -p "$DEFREPO"
def_out="$("$RESOLVER" "$DEFREPO")"
assert_eq "default repo emits 13 rows" "13" "$(row_count "$def_out")"
assert_eq "default work-item -> meta/work" "meta/work" "$(dir_of "$def_out" work-item)"
assert_eq "default note -> meta/notes" "meta/notes" "$(dir_of "$def_out" note)"
assert_eq "default pr-review -> meta/reviews/prs" "meta/reviews/prs" "$(dir_of "$def_out" pr-review)"
assert_eq "default codebase-research -> meta/research/codebase" \
  "meta/research/codebase" "$(dir_of "$def_out" codebase-research)"
# Non-schema path keys are excluded from the allowlist.
assert_not_contains "excludes meta/global" "$def_out" "meta/global"
assert_not_contains "excludes templates dir" "$def_out" ".accelerator/templates"
assert_not_contains "excludes tmp dir" "$def_out" ".accelerator/tmp"
assert_not_contains "excludes integrations dir" "$def_out" ".accelerator/state/integrations"

# ---- 2. Config override (paths.work) ---------------------------------------
echo "=== paths.work override ==="
OVR="$TMP/override"
write_config "$OVR" work "custom/work-items"
ovr_out="$("$RESOLVER" "$OVR")"
assert_eq "paths.work override reflected" "custom/work-items" "$(dir_of "$ovr_out" work-item)"
assert_eq "non-overridden plan stays default" "meta/plans" "$(dir_of "$ovr_out" plan)"
# CWD-form (no root arg) honours config too — the validator's invocation path.
cwd_out="$(cd "$OVR" && "$RESOLVER")"
assert_eq "CWD-form resolution honours config" "custom/work-items" "$(dir_of "$cwd_out" work-item)"

# ---- 3. Empty value falls back to the registry default ---------------------
echo "=== blank paths.work falls back to default ==="
EMP="$TMP/empty"
write_config "$EMP" work ""
emp_out="$("$RESOLVER" "$EMP" 2>/dev/null)"
assert_eq "blank paths.work falls back to meta/work" "meta/work" "$(dir_of "$emp_out" work-item)"
assert_eq "blank paths.work still emits 13 rows" "13" "$(row_count "$emp_out")"
emp_err="$("$RESOLVER" "$EMP" 2>&1 >/dev/null)"
assert_contains "blank key emits a coercion note" "$emp_err" "paths.work is blank"

# ---- 4. Trailing-slash override is normalised ------------------------------
echo "=== trailing-slash normalisation ==="
TS="$TMP/trailing"
write_config "$TS" work "custom/work/"
ts_out="$("$RESOLVER" "$TS")"
assert_eq "trailing slash stripped" "custom/work" "$(dir_of "$ts_out" work-item)"

# ---- 5. Path-safety rejection ---------------------------------------------
echo "=== path-safety rejection ==="
BAD="$TMP/unsafe"
write_config "$BAD" work "../escape"
bad_rc=0
bad_err="$("$RESOLVER" "$BAD" 2>&1 >/dev/null)" || bad_rc=$?
assert_neq "traversal (..) path aborts non-zero" "0" "$bad_rc"
assert_contains "traversal error names the key" "$bad_err" "paths.work"

ABS="$TMP/abs"
write_config "$ABS" work "/etc/passwd"
abs_rc=0
"$RESOLVER" "$ABS" >/dev/null 2>&1 || abs_rc=$?
assert_neq "absolute path aborts non-zero" "0" "$abs_rc"

# ---- 6. Registry coherence guard ------------------------------------------
echo "=== type->path-key registry coherence ==="
# shellcheck source=config-defaults.sh
source "$SCRIPT_DIR/config-defaults.sh"

assert_eq "DOC_TYPE_NAMES and DOC_TYPE_PATH_KEYS are equal length (index-coupled)" \
  "${#DOC_TYPE_NAMES[@]}" "${#DOC_TYPE_PATH_KEYS[@]}"

tsv_types="$(awk -F'\t' 'NR > 1 { print $2 }' "$SCHEMA_TSV" | sort)"
names_sorted="$(printf '%s\n' "${DOC_TYPE_NAMES[@]}" | sort)"
assert_eq "DOC_TYPE_NAMES equals the TSV type column (both directions)" \
  "$tsv_types" "$names_sorted"

missing_keys=""
for k in "${DOC_TYPE_PATH_KEYS[@]}"; do
  found=0
  for pk in "${PATH_KEYS[@]}"; do
    [ "$pk" = "paths.$k" ] && found=1 && break
  done
  [ "$found" -eq 1 ] || missing_keys="$missing_keys $k"
done
assert_empty "every DOC_TYPE_PATH_KEYS entry is a member of PATH_KEYS" "$missing_keys"

test_summary
