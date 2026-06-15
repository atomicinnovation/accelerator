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
  (cd "$repo" && bash "$script" "$@") >"$bash_out" 2>/dev/null || bash_rc=$?
  (cd "$repo" && "$A9R_BIN" "$subcommand" "$@") >"$a9r_out" 2>/dev/null || a9r_rc=$?
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

echo ""
echo "=== Parity differential results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "All byte-for-byte parity checks passed!"
