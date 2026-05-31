#!/usr/bin/env bash
# DESCRIPTION: Soft-defer before harness handshake — Phase 3 mechanical-contract test.
# INTERACTIVE: yes
set -euo pipefail
# Emit the sentinel BEFORE sourcing the harness. The runner detects it
# via exact-prefix match on stdout.
printf 'MIGRATION_RESULT: no_op_pending\n'
exit 0
