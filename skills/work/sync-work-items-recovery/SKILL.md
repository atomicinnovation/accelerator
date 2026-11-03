---
name: sync-work-items-recovery
description: Recovery procedures for work sync failures that need a person —
  terminal rows, refused pending pushes and refused orders. Read by
  sync-work-items — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Sync Work Items Recovery

`/sync-work-items` reads this when a run reports a `terminal` row, a refused
pending push or an `order-refused` row. Its **Read the run's output** section
holds the quoting, files-and-arguments and confirmation rules; every step here
follows them. Each `work sync` order below is a discrete argv token, run only
after the user confirms it.

## Terminal recovery

A `terminal` row means something for the item may have landed, locally or in
the tracker, so a re-run is never offered as the fix. Key the recovery on the
row's state, the `error:` line's operation and the cause's step prefix (`after
<step>: `), read after any `created <KEY>; ` at the start of the cause:

| Row state | Operation, prefix | Recovery |
|---|---|---|
| any | `baseline-set`, `after pull` | The file holds the remote content. The next sync records its baseline (counted as synced, with no row), or reports a conflict to resolve with `<id>=remote`. |
| any | `hash-local` or `baseline-set`, `after push` | The tracker holds your local content. The next sync records its baseline, or reports a conflict to resolve with `<id>=local`. |
| `unsynced` | `baseline-set`, `after author` or `after link` | The item and its issue match; the next sync settles it. |
| any | `update`, `retitle` | Check whether the tracker issue now shows your local content, then sync. |
| `unsynced` | `read-local` or `hash-local`, `after author` | Under the confirmation rule, ask the user to move the malformed file aside (files rule), then sync to import it again. |
| `unsynced` | `read-local` or `hash-local`, `after link` | Repair the file, keeping its `external_id` line, then sync. |
| `unsynced` | `marker-write`, `link-external-id`, `create` | Follow the pending-push recovery below. |
| any | other | Show the line and ask the user to inspect the file and the tracker issue before syncing. |

## Order refusals

An `order-refused` row (`<id>\tfailed\t-\torder-refused`, or the item's own
state when the order was refused while applying) carries a fixed cause on its
`error:` line. Quote it, then take the next step it names:

| Cause | Next step |
|---|---|
| `issue <KEY> not found; check the key` | Check the key with the user; offer `adopt:<KEY>` again only with a corrected key. |
| `issue <KEY> moved to <NEW>; …` | Offer `--resolve <id>=adopt:<NEW>`. |
| `issue <KEY> moved to a key this tracker does not use; …` | Ask the user to check the key in the tracker. |
| `<KEY> is already carried by <carrier>` | Ask the user which item is the real one. |
| `<id> already carries external_id <KEY>; …` | Stop and show both keys and the item path; the user checks the tracker and edits `external_id` by hand. |
| `marker records <KEY>; …` | Offer `--resolve <id>=adopt:<KEY>` with that key. |
| `already written by <carrier>; …` | Ask the user which item is the real one; offer no order. |
| `orders settle pending pushes; run without --pull-only` | Offer the same order without `--pull-only`. |
| `<id> is a draft; promote it with …` | Offer `accelerator work promote <id>`. |
| `<id> is not in this run's --target` | Offer the order again with `--target <id>`. |
| `no work item <id> in this run` | Check the id with the user. |
| `<id> was deferred by identity settlement; …` | Settle the item's identity row first (see `/sync-work-items` Step 5). |

An adopt on an incomplete corpus is not an `order-refused` row: its class is
`unparseable` or `retryable`, and the `#\tunidentified` advice applies. An
adopt that linked over different content says so on its own run with a `note:
<id>: adopt:` line; `/sync-work-items` Step 3 handles the conflict that
follows.

## Pending-push recovery

A refused pending push is a `terminal` row whose `error:` line reads `pending
push refused (<reason>): …`. The marker is at
`<paths.integrations>/<work.integration>/pending-push/<id>.json`, built from
`accelerator config path integrations`, the **Active integration** and a
canonical id; read it with the Read tool. Every branch is settled by one `work
sync` order, run only after the user confirms it:

- Show what the run reported: the reason token, the marker path, any carrier,
  and any `created <KEY>` at the start of this run's cause, the only key that
  counts as known. Quote the marker's contents only when the user asks to see
  them.
- `--resolve <id>=adopt:<KEY>` links an issue that exists. The CLI checks the
  key's shape, that the item carries no other `external_id`, that no other
  work item carries the key, and that the issue exists under that key, then
  links it and removes the marker. When the issue's title or body differ from
  the item's, the next sync reports a conflict to resolve as usual. Run it
  with `--preview` first and show the user the issue title its
  `#\tadopt\t<id>\t<KEY>\t<title>` record prints before confirming.
- `--resolve <id>=create` discards the marker and creates the issue in the
  same run, for an item the user has confirmed has no issue.
- Key known (a `created <KEY>` cause, or no refusal after one): offer
  `adopt:<KEY>`.
- `prior-attempt-unknown`: ask the user to search the tracker for the item's
  title first; offer `adopt:<KEY>` with a key they supply, or `create` when
  they found none.
- `already-written`: name the carrier the cause reports; never offer `adopt`
  or `create`, which the CLI refuses; ask the user which item is the real one.
- `fingerprint-mismatch`: the marker records the key of an issue that was
  created, and the item changed since. Offer `adopt:<that key>` first (the
  next run shows any difference as a conflict); the CLI refuses `create`
  unless that issue no longer exists.
- `marker-unreadable`: show the path; offer `create` only after the user has
  searched the tracker and found no issue, otherwise `adopt:<KEY>`.

Each order is a separate run:

```bash
accelerator work sync --target <id> --resolve <id>=adopt:<KEY>
```
