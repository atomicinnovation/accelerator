#!/usr/bin/env bash
# Raw-byte differential parity gate: the config-read bash scripts vs. the a9r
# binary, compared byte-for-byte.
#
# Why this exists separately from test-config.sh: every assertion there captures
# output via `$(…)`, which strips ALL trailing newlines — so a port emitting
# zero, one, or three trailing newlines passes `assert_eq` identically. Because
# config-read stdout is injected verbatim into prompts via the `!` preprocessor,
# trailing-newline fidelity is contract-critical and otherwise invisible to the
# gate. This suite writes each backend's raw stdout to a file and `cmp`s them
# (newline included), the only assertion form that actually proves
# "byte-for-byte".
#
# Runs the comparison only when A9R_BIN names the built binary (it compares the
# two backends directly). With A9R_BIN unset it is a no-op success, so it stays
# inert in the bash-only suite run and the twice-run wiring drives the real
# comparison with A9R_BIN exported.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -z "${A9R_BIN:-}" ]; then
  echo "test-config-parity.sh: A9R_BIN unset — nothing to compare (this gate"
  echo "diffs bash vs a9r; the bash-only run has no second backend). Skipping."
  exit 0
fi

if [ ! -x "$A9R_BIN" ]; then
  echo "test-config-parity.sh: A9R_BIN set but not an executable file: $A9R_BIN" >&2
  exit 1
fi

READ_VALUE="$SCRIPT_DIR/config-read-value.sh"
READ_PATH="$SCRIPT_DIR/config-read-path.sh"
READ_TEMPLATE="$SCRIPT_DIR/config-read-template.sh"
# Plugin root (parent of scripts/) for the config-read-template a9r subcommand,
# which cannot derive it from the binary path. The bash impl reads its own
# SCRIPT_DIR/..; export the same value to the a9r side so both resolve the same
# plugin-default templates dir.
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PASS=0
FAIL=0

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local d
  d=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$d/.git"
  echo "$d"
}

# diff_cmd <name> <repo> <subcommand> args…
# Runs <subcommand> args… under both backends from <repo>, capturing raw stdout
# to files (NO command substitution, so trailing newlines survive), and asserts
# both the exit code and the raw bytes match.
diff_cmd() {
  local name="$1" repo="$2" subcommand="$3"
  shift 3
  local script
  case "$subcommand" in
    config-read-value) script="$READ_VALUE" ;;
    config-read-path) script="$READ_PATH" ;;
    config-read-template) script="$READ_TEMPLATE" ;;
    *)
      echo "  FAIL: $name (unknown subcommand: $subcommand)"
      FAIL=$((FAIL + 1))
      return
      ;;
  esac
  local bash_out a9r_out bash_rc a9r_rc
  bash_out="$TMPDIR_BASE/bash.out"
  a9r_out="$TMPDIR_BASE/a9r.out"
  bash_rc=0
  a9r_rc=0
  # Force the bash impl on this side: the script is now a shim that would
  # itself route to a9r when A9R_BIN is set, collapsing the differential to
  # a9r-vs-a9r. A9R_FORCE_BASH=1 pins the genuine bash backend so the compare
  # is bash-impl vs a9r.
  (cd "$repo" && A9R_FORCE_BASH=1 bash "$script" "$@") >"$bash_out" 2>/dev/null || bash_rc=$?
  (cd "$repo" && ACCELERATOR_PLUGIN_ROOT="$PLUGIN_ROOT" "$A9R_BIN" "$subcommand" "$@") \
    >"$a9r_out" 2>/dev/null || a9r_rc=$?
  if [ "$bash_rc" != "$a9r_rc" ]; then
    echo "  FAIL: $name — exit code differs: bash=$bash_rc a9r=$a9r_rc"
    FAIL=$((FAIL + 1))
    return
  fi
  if cmp -s "$bash_out" "$a9r_out"; then
    echo "  PASS: $name (byte-identical stdout, exit $bash_rc)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name — stdout differs byte-for-byte (exit $bash_rc):"
    echo "    bash:"
    xxd "$bash_out" | sed 's/^/      /'
    echo "    a9r:"
    xxd "$a9r_out" | sed 's/^/      /'
    FAIL=$((FAIL + 1))
  fi
}

echo "=== config-read byte-for-byte differential (bash vs a9r) ==="
echo ""

# --- found value (top-level) ------------------------------------------------
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
name: my-name
---
FIXTURE
diff_cmd "found top-level value" "$REPO" config-read-value name default

# --- found sectioned value --------------------------------------------------
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
agents:
  reviewer: my-reviewer
---
FIXTURE
diff_cmd "found sectioned value" "$REPO" config-read-value agents.reviewer fallback

# --- quote-stripped value ---------------------------------------------------
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
name: "quoted value"
---
FIXTURE
diff_cmd "one quote layer stripped" "$REPO" config-read-value name default

# --- not-found, non-empty default (echoes default, exit 0) ------------------
REPO=$(setup_repo)
diff_cmd "not-found echoes default" "$REPO" config-read-value missing.key the-default

# --- not-found, EMPTY default (bare newline, exit 0) — the trailing-newline
#     fidelity case that command substitution would mask -----------------------
REPO=$(setup_repo)
diff_cmd "not-found empty default (bare newline)" "$REPO" config-read-value missing.key ""

# --- config-read-path: defaults-table hit -----------------------------------
REPO=$(setup_repo)
diff_cmd "path default-table value" "$REPO" config-read-path plans

# --- config-read-path: explicit default -------------------------------------
REPO=$(setup_repo)
diff_cmd "path explicit default" "$REPO" config-read-path plans custom/plans

# --- config-read-path: configured override ----------------------------------
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat >"$REPO/.accelerator/config.md" <<'FIXTURE'
---
paths:
  plans: my/plans
---
FIXTURE
diff_cmd "path configured override" "$REPO" config-read-path plans meta/plans

# --- config-read-template: plugin-default (tier 3), fence-wrapped -----------
REPO=$(setup_repo)
diff_cmd "template plugin default (fenced)" "$REPO" config-read-template plan

# --- config-read-template: user template (tier 2), already-fenced passthrough
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator/templates"
# shellcheck disable=SC2016 # literal markdown fence text, not command substitution
printf '```markdown\n# Already Fenced\n```\n' >"$REPO/.accelerator/templates/plan.md"
diff_cmd "template user override (no double-wrap)" "$REPO" config-read-template plan

# --- config-read-template: unknown name → error + available list, exit 1 ----
REPO=$(setup_repo)
diff_cmd "template unknown name (error+list)" "$REPO" config-read-template nope-not-a-template

echo ""
echo "=== Parity differential results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "All byte-for-byte parity checks passed!"
