---
title: "Bump eslint to latest minor version"
type: chore
status: ready
priority: low
---

# Bump eslint to Latest Minor Version

## Summary

Update the `eslint` package from `8.44.0` to `8.57.0` (latest minor
release in the 8.x series) to pick up bug fixes and rule improvements
without upgrading to the breaking ESLint 9 major.

## Context

ESLint 8.44.0 has been in place since June 2023. The 8.x series has
received several bug-fix and rule-improvement releases since then,
including fixes for false positives in the `no-unused-vars` rule that
currently produce intermittent noise in CI. Staying on a recent minor
keeps the codebase compatible with the ESLint 9 migration path when that
is scheduled.

## Requirements

1. Update `eslint` in `package.json` (and `package-lock.json`) from
   `8.44.0` to `8.57.0`.
2. Run the full lint suite after the upgrade and fix any newly triggered
   rule violations (expected: zero or one).

## Acceptance Criteria

- `package.json` references `eslint@8.57.0`.
- `npm run lint` exits 0 with no new warnings or errors relative to the
  pre-upgrade baseline.
- CI passes.

## Dependencies

_None._

## Assumptions

- The `8.44.0` → `8.57.0` upgrade is semver-minor and introduces no
  breaking changes to existing rules or configuration.
- No other dependencies in the lock file have a conflicting peer-dependency
  constraint on the ESLint version.
