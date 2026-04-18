#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

REAL_CLI="$SCRIPT_DIR/../cli/accelerator-visualiser"
REAL_STUB="$SCRIPT_DIR/launch-server.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Make a relocated copy of the cli/scripts sibling pair. Mutation tests
# operate on the copy only — the real files are never touched, so a
# mid-test failure can never corrupt the working tree. This also
# exercises the wrapper's plugin-root resolution from an arbitrary
# directory, proving the cli/↔scripts/ sibling contract beyond the
# in-tree happy path.
TEMP_SKILL="$TMPDIR_BASE/skill-copy"
mkdir -p "$TEMP_SKILL/cli" "$TEMP_SKILL/scripts"
cp "$REAL_CLI" "$TEMP_SKILL/cli/accelerator-visualiser"
cp "$REAL_STUB" "$TEMP_SKILL/scripts/launch-server.sh"
chmod +x "$TEMP_SKILL/cli/accelerator-visualiser" "$TEMP_SKILL/scripts/launch-server.sh"
TEMP_CLI="$TEMP_SKILL/cli/accelerator-visualiser"
TEMP_STUB="$TEMP_SKILL/scripts/launch-server.sh"

echo "=== accelerator-visualiser CLI wrapper (Phase 1) ==="
echo ""

echo "Test: wrapper is executable"
assert_file_executable "executable bit set" "$REAL_CLI"

echo "Test: wrapper exits 0"
assert_exit_code "exits 0" 0 bash "$REAL_CLI"

echo "Test: wrapper output matches stub output"
REAL_OUTPUT=$(bash "$REAL_CLI")
STUB_OUTPUT=$(bash "$REAL_STUB")
assert_eq "wrapper output equals stub output" "$STUB_OUTPUT" "$REAL_OUTPUT"

echo "Test: wrapper works from a relocated tree"
RELOCATED_OUTPUT=$(bash "$TEMP_CLI")
assert_eq "relocated wrapper output equals stub output" "$STUB_OUTPUT" "$RELOCATED_OUTPUT"

echo "Test: wrapper actually delegates (proven by sentinel)"
# Replace the TEMP_STUB with a unique-UUID echo. If the wrapper ever
# inlined its own echo instead of exec'ing the stub, the sentinel
# would not appear. Mutation is on the copy only.
SENTINEL="delegation-sentinel-$$-$RANDOM"
cat > "$TEMP_STUB" <<EOF
#!/usr/bin/env bash
echo "$SENTINEL"
EOF
chmod +x "$TEMP_STUB"
DELEGATION_OUTPUT=$(bash "$TEMP_CLI")
assert_eq "wrapper delegated to stub" "$SENTINEL" "$DELEGATION_OUTPUT"

echo "Test: wrapper forwards arguments verbatim (incl. spaces and empty)"
# Replace TEMP_STUB with a script that echoes each argv on its own line
# so quoting regressions (exec "..." $@ vs exec "..." "$@") surface.
cat > "$TEMP_STUB" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@"
EOF
chmod +x "$TEMP_STUB"
ARG_OUTPUT=$(bash "$TEMP_CLI" --foo bar "hello world" "" "a b c")
EXPECTED_ARGS=$(printf -- "--foo\nbar\nhello world\n\na b c")
assert_eq "wrapper preserves \$@ quoting" "$EXPECTED_ARGS" "$ARG_OUTPUT"

echo "Test: wrapper works via symlink (skip if filesystem forbids symlinks)"
LINK="$TMPDIR_BASE/accelerator-visualiser-link"
if ln -s "$REAL_CLI" "$LINK" 2>/dev/null; then
  LINK_OUTPUT=$("$LINK")
  assert_eq "symlink output equals stub output" "$STUB_OUTPUT" "$LINK_OUTPUT"
  assert_exit_code "symlink exits 0" 0 "$LINK"
else
  echo "  SKIP: filesystem does not permit symlinks"
fi

echo "Test: wrapper source contains symlink-cycle guard"
# The 40-hop counter is defence-in-depth against a pathological
# symlink install. It cannot be exercised behaviourally: any cyclic
# symlink chain fails at the kernel's execve() with ELOOP before
# bash starts, so the wrapper's walk never runs via invocation.
# Instead we assert the guard is present in the source so an
# accidental removal surfaces as a test failure.
if grep -q 'HOPS.*-gt.*40' "$REAL_CLI" \
  && grep -q 'symlink loop detected' "$REAL_CLI"; then
  echo "  PASS: hop counter and loop-detected diagnostic present"
  PASS=$((PASS + 1))
else
  echo "  FAIL: hop counter or loop diagnostic missing"
  FAIL=$((FAIL + 1))
fi

echo "Test: stderr is empty on happy path"
assert_stderr_empty "no stderr output" bash "$REAL_CLI"

test_summary
