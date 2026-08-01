---
type: pr-description
id: "31"
title: "Refine the migration engine subdomain work item and record its review"
date: "2026-08-01T13:18:04+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
relates_to: ["work-item:0172"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/31"
pr_number: 31
tags: [work-item, review, migrate, rust, planning]
revision: "45e5c1a2566bcc714f3abba4b0406b1b89c15ff1"
repository: "accelerator"
last_updated: "2026-08-01T13:18:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Refine the migration engine subdomain work item and record its review

Revises work item 0172 through a four-pass review and records the review
artifact alongside it.

## Why

0172 is the highest-risk port on the 0136 spine — it retires ~12.5k lines of the
plugin's most stateful bash, including a 984-line FIFO/fd concurrency library and
seven numbered migrations. It went into review as a `draft` whose acceptance
criteria could not have gated that work.

## What changed

The substantive corrections come from reading the primary sources directly —
`skills/config/migrate/SKILL.md` and work items 0119, 0178, 0180 and 0167 —
rather than paraphrasing them. Four claims in earlier drafts were wrong:

- **`--list` scope.** It dry-emits *every* pending interactive transformation,
  segmented by a `# migration <id>` header with `<position>` restarting per
  migration. It is the *decisions file* that is scoped per-migration, not
  `--list`.
- **The 0119 guarded-resume contract was inverted.** The message listing every
  owned dirty path is a resume affordance emitted when the pre-flight *proceeds*
  at exit 0; the refusal path asserts no affordance plus the
  `ACCELERATOR_MIGRATE_FORCE` hint.
- **The legacy read path.** `ACCELERATOR_MIGRATION_MODE` was deliberately dropped
  by 0178 and stays unhonoured with a retained negative test. 0167's
  per-invocation `--allow-legacy-layout` read flag replaces it.
- **The per-run path manifest** is a sidecar pair at
  `.accelerator/state/migrations-run-paths.txt` plus a run-id file, and an *empty*
  manifest is valid — an interactive interrupt before any mechanical delta leaves
  one, and requiring non-empty would make that resume unreachable.

Beyond the corrections, the item now captures the documented lifecycle contract
that earlier drafts omitted (the `no_op_pending` soft deferral, the preview
banner, unknown-ID preservation, the dry-apply validation pass, predicate
routing, sticky skip, source drift), scopes the parity gate to repointable
suites, and commits to mapping every retiring assertion rather than the
remainder alone — a stated departure from the pattern 0167 set, with a measured
trigger at 400 assertions.

## Couplings recorded

Two cross-item couplings that were invisible from both ends:

- **0182's plugin-root rename** — its `no CLAUDE_ under cli/` guard, the
  index-3 `hooks.json` entry, and allowlist entries enumerating files this story
  deletes.
- **0183's SessionStart advisory fix** — it targets the same hook this story
  ports and deletes, so whichever lands first invalidates the other.

Also recorded: `--allow-legacy-layout` appears in none of 0167's own criteria
(only in a note 0167 wrote onto 0178), so 0167 could close without shipping what
these migrations depend on.

## Scope

0172 moves `draft` → `ready` at `priority: high`. Assertion-grade detail — the
fixture × artefact table, exact normalisation tokens, the depth-floor extraction
procedure — is deliberately deferred to `/create-plan`, following the precedent
0119's own review set when it deferred "assertion-grade detail … to be settled in
/create-plan".

## Review

Four passes are recorded in
`meta/reviews/work/0172-migration-engine-subdomain-review-1.md`. The verdict is
APPROVE, set by the author after a bounded close-out following pass 4; the
document records that explicitly, since no fifth lens pass ran against the
closed-out state.
