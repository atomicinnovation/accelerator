#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

REAL_CLI="$SCRIPT_DIR/../cli/accelerator-visualiser"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Make a relocated copy of the cli binary into a cli/scripts sibling pair.
# TEMP_STUB starts as a simple sentinel so invocation tests work without
# a full project tree (the real dispatcher delegates to launch-server.sh
# which requires find_repo_root).
TEMP_SKILL="$TMPDIR_BASE/skill-copy"
mkdir -p "$TEMP_SKILL/cli" "$TEMP_SKILL/scripts"
cp "$REAL_CLI" "$TEMP_SKILL/cli/accelerator-visualiser"
chmod +x "$TEMP_SKILL/cli/accelerator-visualiser"
TEMP_CLI="$TEMP_SKILL/cli/accelerator-visualiser"
TEMP_STUB="$TEMP_SKILL/scripts/visualiser.sh"

INITIAL_SENTINEL="cli-wrapper-initial-sentinel"
cat > "$TEMP_STUB" << SENTINELEOF
#!/usr/bin/env bash
echo "$INITIAL_SENTINEL"
SENTINELEOF
chmod +x "$TEMP_STUB"

echo "=== accelerator-visualiser CLI wrapper ==="
echo ""

echo "Test: wrapper is executable"
assert_file_executable "executable bit set" "$REAL_CLI"

echo "Test: wrapper exits 0 (via relocated tree with sentinel stub)"
assert_exit_code "exits 0" 0 bash "$TEMP_CLI"

echo "Test: wrapper works from a relocated tree"
RELOCATED_OUTPUT=$(bash "$TEMP_CLI") || true
assert_eq "relocated wrapper delegates to sentinel stub" "$INITIAL_SENTINEL" "$RELOCATED_OUTPUT"

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
# Restore a sentinel stub before the symlink test (it was replaced above).
LINK_SENTINEL="symlink-sentinel-$$"
cat > "$TEMP_STUB" << LINKEOF
#!/usr/bin/env bash
echo "$LINK_SENTINEL"
LINKEOF
chmod +x "$TEMP_STUB"
LINK="$TMPDIR_BASE/accelerator-visualiser-link"
if ln -s "$TEMP_CLI" "$LINK" 2>/dev/null; then
  LINK_OUTPUT=$("$LINK") || true
  assert_eq "symlink output equals sentinel" "$LINK_SENTINEL" "$LINK_OUTPUT"
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

echo "Test: stderr is empty on happy path (sentinel stub)"
# Restore a quiet sentinel so the wrapper produces no stderr.
cat > "$TEMP_STUB" <<'EOF'
#!/usr/bin/env bash
echo "ok"
EOF
chmod +x "$TEMP_STUB"
assert_stderr_empty "no stderr output" bash "$TEMP_CLI"

test_summary
