#!/usr/bin/env bash
set -euo pipefail

# Byte-identical mechanical-path snapshot test.
#
# Captures the migrated artefact tree, redacted stdout/stderr, and the exit
# code of the runner against each bundled migration's fixture, then asserts
# the post-change runner produces byte-identical output. The snapshots live
# in test-fixtures/mechanical-snapshots/ and must be checked in so this test
# fails if a future change accidentally alters the mechanical-path output.
#
# To (re)generate snapshots after an intentional change: run with the env var
# ACCELERATOR_MIGRATE_SNAPSHOT_REGEN=1.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

RUNNER="$SCRIPT_DIR/run-migrations.sh"
FIXTURES_ROOT="$SCRIPT_DIR/test-fixtures"
SNAPSHOTS_ROOT="$SCRIPT_DIR/test-fixtures/mechanical-snapshots"

REGEN="${ACCELERATOR_MIGRATE_SNAPSHOT_REGEN:-0}"

if [ "$REGEN" != "1" ]; then
  assert_dir_exists "snapshot tree is checked in" "$SNAPSHOTS_ROOT"
fi

mkdir -p "$SNAPSHOTS_ROOT"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# This is the MECHANICAL-path snapshot test (see header). Interactive migrations
# (# INTERACTIVE: yes — e.g. 0007) drive the runner through a FIFO handshake and
# run a whole-corpus self-validation, so they are out of scope here and are
# covered by test-migrate-0007.sh / test-migrate-interactive.sh. Pin the default
# migrations dir to the non-interactive set so the checked-in snapshots stay
# stable as interactive migrations are added.
MIGRATIONS_DIR="$SCRIPT_DIR/../migrations"
MECHANICAL_MIGRATIONS_DIR="$TMPDIR_BASE/mechanical-migrations"
mkdir -p "$MECHANICAL_MIGRATIONS_DIR"
for _m in "$MIGRATIONS_DIR"/[0-9][0-9][0-9][0-9]-*.sh; do
  if ! head -5 "$_m" | grep -qE '^# INTERACTIVE:[[:space:]]*yes$'; then
    cp "$_m" "$MECHANICAL_MIGRATIONS_DIR/"
  fi
done
export ACCELERATOR_MIGRATIONS_DIR="$MECHANICAL_MIGRATIONS_DIR"

# Each per-migration fixture exposes a `seed.sh` that populates a fresh
# PROJECT_ROOT and (optionally) writes state under .accelerator/state/. The
# snapshot captures the migrated tree, redacted streams, and exit code.

# Redact volatile content from captured streams: absolute paths to the
# sandbox tempdir, ISO 8601 timestamps, and PID-like numerics in our own
# diagnostic lines.
redact_stream() {
  local sandbox="$1"
  sed \
    -e "s|$sandbox|<SANDBOX>|g" \
    -e "s|/var/folders/[^/]*/[^/]*/T/[^/[:space:]]*|<TMPDIR>|g" \
    -e "s|/tmp/[^/[:space:]]*|<TMPDIR>|g" \
    -e 's/[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}T[0-9]\{2\}:[0-9]\{2\}:[0-9]\{2\}Z/<REDACTED>/g' \
    -e 's/pid=[0-9]\+/pid=<REDACTED>/g'
}

run_snapshot() {
  local id="$1"
  local seed="$FIXTURES_ROOT/$id/seed.sh"
  local snap_dir="$SNAPSHOTS_ROOT/$id"

  if [ ! -f "$seed" ]; then
    skip_test "snapshot $id" "no seed.sh under test-fixtures/$id/"
    return 0
  fi

  local sandbox="$TMPDIR_BASE/$id"
  mkdir -p "$sandbox"
  bash "$seed" "$sandbox"

  local stdout_file="$TMPDIR_BASE/$id.stdout"
  local stderr_file="$TMPDIR_BASE/$id.stderr"
  local rc=0
  PROJECT_ROOT="$sandbox" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATE_FORCE=1 \
    bash "$RUNNER" >"$stdout_file" 2>"$stderr_file" || rc=$?

  local files_list="$TMPDIR_BASE/$id.files"
  (cd "$sandbox" && find . -type f | sort |
    xargs -I{} sh -c 'printf "%s  %s\n" "$(sha256sum < "{}" | cut -d" " -f1)" "{}"') \
    >"$files_list"

  redact_stream "$sandbox" <"$stdout_file" >"$TMPDIR_BASE/$id.stdout.redacted"
  redact_stream "$sandbox" <"$stderr_file" >"$TMPDIR_BASE/$id.stderr.redacted"

  if [ "$REGEN" = "1" ]; then
    mkdir -p "$snap_dir"
    cp "$files_list" "$snap_dir/files.sha256"
    cp "$TMPDIR_BASE/$id.stdout.redacted" "$snap_dir/stdout"
    cp "$TMPDIR_BASE/$id.stderr.redacted" "$snap_dir/stderr"
    printf '%s\n' "$rc" >"$snap_dir/exit-code"
    echo "  REGEN: snapshot $id captured"
    return 0
  fi

  if [ ! -d "$snap_dir" ]; then
    echo "  FAIL: snapshot $id — no checked-in snapshot at $snap_dir"
    echo "    To capture: ACCELERATOR_MIGRATE_SNAPSHOT_REGEN=1 bash $0"
    FAIL=$((FAIL + 1))
    return 0
  fi

  if diff -u "$snap_dir/files.sha256" "$files_list" >/dev/null; then
    echo "  PASS: snapshot $id — artefact tree byte-identical"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: snapshot $id — artefact tree differs"
    diff -u "$snap_dir/files.sha256" "$files_list" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi

  if diff -u "$snap_dir/stdout" "$TMPDIR_BASE/$id.stdout.redacted" >/dev/null; then
    echo "  PASS: snapshot $id — stdout byte-identical"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: snapshot $id — stdout differs"
    diff -u "$snap_dir/stdout" "$TMPDIR_BASE/$id.stdout.redacted" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi

  if diff -u "$snap_dir/stderr" "$TMPDIR_BASE/$id.stderr.redacted" >/dev/null; then
    echo "  PASS: snapshot $id — stderr byte-identical"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: snapshot $id — stderr differs"
    diff -u "$snap_dir/stderr" "$TMPDIR_BASE/$id.stderr.redacted" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  fi

  local recorded_rc
  recorded_rc=$(cat "$snap_dir/exit-code")
  assert_eq "snapshot $id — exit code matches" "$recorded_rc" "$rc"
}

# Iterate over every fixture directory that has a seed.sh. Snapshots are
# per-fixture (not per-migration) so a fixture covering multiple migrations
# at once is still recorded as one snapshot.
for fixture_dir in "$FIXTURES_ROOT"/*/; do
  fid=$(basename "$fixture_dir")
  case "$fid" in
    mechanical-snapshots) continue ;;
    interactive) continue ;;
    bash-shims) continue ;;
  esac
  run_snapshot "$fid"
done

test_summary
