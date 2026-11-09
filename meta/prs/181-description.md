---
type: "pr-description"
id: "181"
title: "[0338] Make linear update send only the fields it is given"
date: "2026-11-06T12:48:12+00:00"
author: "Phil Helm"
producer: "describe-pr"
status: "complete"
work_item_id: "0338"
parent: "work-item:0338"
relates_to: ["work-item:0146", "work-item:0048", "work-item:0296"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/181"
pr_number: 181
tags: ["linear", "cli", "update"]
revision: "0538df52402f7c1d3b442f220c7752e8ade237be"
repository: "accelerator-git"
last_updated: "2026-11-06T12:48:12+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# [0338] Make linear update send only the fields it is given

## Summary

`accelerator linear update` wrote fields it was never given and dropped ones
it was. Without `--title` it sent an empty title, which Linear refused (exit
34). Without `--description` it sent an empty description, which Linear
accepted, silently erasing the issue's body. `--assignee-id` and `--priority`
were parsed but never sent, and `--state` sent only the state, discarding
every other flag.

`linear update` now sends one `issueUpdate` whose input holds exactly the
flags given, with `--state` resolved through the catalogue into the same
input.

```diff
 run_update(args)
-  if --state
-    transition(key, state)                  # every other flag dropped
-    print issue JSON with "outcome"
-    return
-  update_op(key, title ?? "", description ?? "")
+  state_id = resolve_state(--state)?       # 122 / 123 before any request
+  update_op(key, UpdateFields {            # only the flags given
+    title, description, state_id, assignee_id, priority
+  })
   print "updated\t<KEY>"
```

## Evidence

The `issueUpdate` input each command sends. "After" is what `flow_update.rs`
asserts. "Before" is read from `main`'s `run_update`:

| Command | Before | After |
| --- | --- | --- |
| `--title x` | `{title: "x", description: ""}`, body erased | `{title: "x"}` |
| `--description x` | `{title: "", description: "x"}`, exit 34 | `{description: "x"}` |
| `--priority 1` | `{title: "", description: ""}`, exit 34 | `{priority: 1}` |
| `--state "In Progress" --title x` | state only, title dropped | `{stateId: "state-ip-uuid", title: "x"}` |

Against real Linear on PP-938, `--description`, `--title` and `--priority`
each changed only their own field.

## Merge Danger

**Door:** two-way

A revert restores the old input and output. Nothing is migrated or stored.

**Blast Radius:** narrow

- **Scripts calling `linear update --state`.** They get `updated\t<KEY>`
  instead of JSON. Some exit codes change too: a bare 401 exits 20, not 11,
  and a malformed identifier such as `#BLA-1` exits 34 before sending.
- **Scripts calling `linear update`.** `--assignee-id` and `--priority` now
  reach Linear, so a value Linear refuses, such as `--assignee-id ""`, exits
  34 where it used to succeed.
- **`work sync`, `work update --push` and promotion.** These now go through
  `update_op`. The existing pins and the new titled `update` pin hold their
  input byte-for-byte.

## Changes

### `linear-client`

- **One update operation.** A new `update` module holds `UpdateFields`, a
  value of optional `title`, `description`, `state_id`, `assignee_id` and
  `priority`, and `update_op(&ExternalId, &UpdateFields)`. Its input carries
  only the fields set. It refuses an update that names no field before any
  request. This follows Jira's `UpdateFields` and `update_op`.
- **`retitle_op` is gone.** The tracker port's `update` and `retitle` both
  delegate to `update_op`, and send byte-for-byte the input they sent before.
  `work sync`, `work update --push` and promotion are unchanged. A new exact
  pin for a titled `update`, beside the existing pins, guards this.
- `call_op` and `refuse_identifier_op` become `pub(crate)`, so `update.rs`
  can own its document as `transition.rs` does.

### `linear update`

- **Only the flags given.** `run_update` builds `UpdateFields` from the flags
  with `as_deref()`, so an absent flag is absent from the input. An empty one,
  such as `--description ""`, is sent as typed and clears the field.
- **`--state` joins the same update.** The state is resolved before any
  request. An unknown name exits 122 and an ambiguous one exits 123, with the
  existing `E_TRANSITION_STATE_…` stderr lines, as before.
- **One success shape.** Every success prints `updated\t<KEY>`, with or
  without `--state`. Every failure prints `linear update failed` with the
  port-failure exit code.
- `UpdateArgs::names_no_field` replaces the five-way `is_none()` chain behind
  the `E_UPDATE_NO_OPS` refusal (exit 111).

### Skill and CHANGELOG

- **`update-linear-issue`.**
  - Step 2 shows a field given an empty value as cleared.
  - Step 4 drops the JSON `outcome` case.
  - Step 4 no longer claims that every failure left the issue unwritten. A
    timeout can follow a write Linear applied, so the skill now suggests
    `/show-linear-issue` to check.
- **`CHANGELOG.md`.** `[Unreleased]` gains a `Fixed` entry for the erased
  description and a `Breaking` entry for `--state`'s output and failure
  reporting.

### Breaking

`linear update --state` used to print the issue as JSON with
`"outcome": "updated"`. It now prints `updated\t<KEY>`, like every other
update. Its failures now exit with the codes a plain update uses, which
differ for some responses, such as a 401 without a JSON body. An identifier
the shared rule refuses is now refused before anything is sent. The only
caller in the repo is `update-linear-issue`, which this PR updates.

## Context

- Work item: `meta/work/0338-linear-update-overwrites-fields-it-was-not-given.md`
  (PP-921)
- Research:
  `meta/research/codebase/2026-11-05-0338-linear-update-overwrites-unspecified-fields.md`
- Plan: `meta/plans/2026-11-05-0338-linear-update-sends-only-given-fields.md`
- Reviews: `meta/reviews/work/0338-…-review-1.md` and
  `meta/reviews/plans/2026-11-05-0338-…-review-1.md`
- Validation:
  `meta/validations/2026-11-05-0338-linear-update-sends-only-given-fields-validation.md`

## Testing

- [x] `cli/linear-client/tests/update.rs` (new) checks the exact input for:
  - each field alone, and all five together;
  - an empty description.

  It also checks the refusals and failures:
  - an empty update and a refused identifier send nothing;
  - a 200 response carrying GraphQL `errors[]` is a bad request.
- [x] `cli/linear-cli/tests/flow_update.rs` runs the real binary against a mock
  server.
  - It checks the exact input for every acceptance criterion.
  - It runs `--state` with `--title` over the current and legacy catalogue
    shapes.
  - It checks the 122 and 123 refusals, with and without a catalogue, all
    sending nothing.
  - A failed `--state` update prints `linear update failed` and exits 34.
  - A `--state` update fails with the same exit code as a plain update against
    a bare 401.
- [x] The existing pins in `contract_offline.rs` and `port.rs` pass unedited.
- [x] `mise run --continue-on-error`: every task passes except the two
  date-expired advisory checks described below.
- [x] Against real Linear on 2026-11-05, on scratch issue PP-938 (since
  archived), with the release binary built from `91d7fa12`:
  - `--description`, `--title` and `--priority` each change only their own
    field.
  - `--state` with `--priority` and `--assignee-id` changes all three in one
    update.
  - An unknown state exits 122 and leaves the issue unchanged.
  - `--assignee-id ""` exits 34 with `linear update failed`. Linear refuses it,
    and the assignee is unchanged.
  - `--title ""` exits 34 in the same way. `--description ""` exits 0 and
    clears the description.
- [x] `/update-linear-issue <KEY> --title "…"` end to end, run on 2026-11-06
  from this checkout with `claude --plugin-dir` and the branch's local builds:
  the preview confirms a title-only change, and `/show-linear-issue` shows the
  description survived.
- [x] `/update-linear-issue <KEY> --description ""` previews the description
  as cleared.

## Notes for Reviewers

- **Date-expired advisory checks.** Two checks fail on `main` too, and are
  unrelated to this change. Their review-by dates of 2026-11-03 have passed:
  - `docs:audit:check` fails on the `GHSA-ch52-4w7c-c8xp` ignore in
    `docs-site/audit-ignores.toml`.
  - `test:integration:deny` fails on the `RUSTSEC-2026-0118` and
    `RUSTSEC-2026-0119` cargo-deny ignores.

  They need re-checking upstream in a separate change.
- **What isn't changed:**
  - `linear update failed` gives no detail about what went wrong.
  - `--priority` and `--assignee-id` are not validated locally, so Linear's
    refusal shows up as exit 34.
  - There is no way to unassign an issue.
  - `linear transition` and its JSON output are unchanged.
- **Ambiguous state tested offline only.** The live catalogue has no
  ambiguous active state name, so the 123 refusal is proven only by the flow
  test.
