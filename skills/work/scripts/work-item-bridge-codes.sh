#!/usr/bin/env bash
# work-item-bridge-codes.sh — the single canonical exit-code taxonomy shared by
# the work → integrations bridges (create / fetch / update) and the push/sync
# decision scripts. This file is SOURCED, never executed: it owns the
# E_DISPATCH_* namespace so the taxonomy has exactly one definition rather than a
# hand-copied block per script.
#
# The numeric values are a hard contract (asserted by a unit test):
#   70  E_DISPATCH_RETRYABLE      failure provably BEFORE any remote mutation
#                                 (arg/validation/auth/connect) — safe to retry.
#                                 For a READ bridge there is nothing to mutate,
#                                 so 70 simply means "read failed / degrade".
#   71  E_DISPATCH_TERMINAL       failure AT/AFTER a mutation (request sent,
#                                 response lost or invalid) — NOT safe to
#                                 auto-retry. Read bridges never emit this (a read
#                                 mutates nothing).
#   72  E_DISPATCH_NOT_AVAILABLE  tracker recognised but the operation is not
#                                 built yet (trello / github-issues).
#   73  E_DISPATCH_UNRECOGNISED   <sys> not in {linear,jira,trello,github-issues}
#                                 or empty — fail closed.
#
# Guarded against double-source: a caller that transitively sources this twice
# (e.g. a test that sources two bridges) must not trip `readonly` re-declaration.

# These constants are consumed by the scripts that source this file, not used
# within it — SC2034 (appears unused) is a false positive for a sourced library.
# shellcheck disable=SC2034
if [ -z "${_WORK_ITEM_BRIDGE_CODES_SOURCED:-}" ]; then
  _WORK_ITEM_BRIDGE_CODES_SOURCED=1
  readonly E_DISPATCH_RETRYABLE=70
  readonly E_DISPATCH_TERMINAL=71
  readonly E_DISPATCH_NOT_AVAILABLE=72
  readonly E_DISPATCH_UNRECOGNISED=73
fi
