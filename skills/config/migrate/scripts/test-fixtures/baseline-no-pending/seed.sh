#!/usr/bin/env bash
# Seed a sandbox PROJECT_ROOT representing a fully-upgraded consumer repo
# whose state file already records every bundled migration. The runner
# should report "No pending migrations." and exit 0.
#
# Used by test-migrate-snapshot.sh to byte-identical-snapshot the
# mechanical-path "nothing to do" output.

set -euo pipefail

SANDBOX="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../../../.." && pwd)"
MIGRATIONS_DIR="$PLUGIN_ROOT/skills/config/migrate/migrations"

mkdir -p "$SANDBOX/.accelerator/state"
mkdir -p "$SANDBOX/.git"

# Mark every bundled mechanical migration as applied so the runner has
# nothing pending. The IDs come from the bundled migrations directory —
# enumerating here keeps the fixture decoupled from the runner. Interactive
# migrations (# INTERACTIVE: yes — e.g. 0007) are out of scope for this
# mechanical-path snapshot (the harness pins the runner to the non-interactive
# set), so they are excluded here too, keeping the seeded ledger in lock-step.
for f in "$MIGRATIONS_DIR"/[0-9][0-9][0-9][0-9]-*.sh; do
  grep -qE '^# INTERACTIVE:[[:space:]]*yes$' < <(head -5 "$f") && continue
  basename "$f" .sh
done >"$SANDBOX/.accelerator/state/migrations-applied"
