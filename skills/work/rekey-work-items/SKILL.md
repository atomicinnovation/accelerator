---
name: rekey-work-items
description: Converge every work item's ID onto the configured work.id_pattern,
  renaming files and rewriting references across meta/. Use after changing
  work.id_pattern or work.key, when existing work items should take IDs in the
  new shape, or to finish or undo an interrupted re-key.
argument-hint: "[--per-item-reads]"
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator work *)
  - Bash(accelerator corpus frontmatter validate *)
---

# Re-key Work Items

!`accelerator config context --skill rekey-work-items --fail-safe`
!`accelerator config agents --fail-safe`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Active integration**: !`accelerator config work integration --fail-safe`
**Local ID prefix**: !`accelerator config work key --fail-safe`
**ID pattern**: !`accelerator config work id_pattern --fail-safe`
**Work items directory**: !`accelerator config path work --fail-safe`

`/rekey-work-items` gives every work item an `id` in the shape **ID pattern**
describes, whatever shape it has now. Each re-keyed item is renamed, its old
`id` joins its `aliases`, and references to it across every corpus root
follow. The `accelerator work rekey` engine owns every decision: which items
change, the order fresh numbers follow, collisions, and recovery. This skill
previews, asks, then applies or discards.

A preview records a **proposed run plan** under `.accelerator/state/rekey/`,
which version control ignores. The apply re-keys exactly the previewed
mapping, refusing if any item's identity changed since, and records its
progress first, so an interrupted run is finished by running it again. Under
a `{tracker}` pattern the preview reads the tracker for current keys; the
apply makes no tracker call. The apply refuses over uncommitted changes in
the corpus, because version control is how a re-key is undone.

Pass `--per-item-reads` through to the preview when the user gives it.

## The override note

`ACCELERATOR_FORCE` makes `work rekey` skip its safety checks: the apply's
working-copy check, and the discard's check that every path the run wrote is
restored. **Never set it**, by prefix, `export` or `env`: overriding a
refusal is the user's decision, made outside this skill.

Whenever a `work rekey` call's stderr carries a line starting
`warning: ACCELERATOR_FORCE is set`, on any exit, tell the user, before any
question, that `ACCELERATOR_FORCE` is set in their environment and the next
apply or discard will skip its safety check. From then on, for the rest of
this invocation, never offer **Undo it**: say the override would discard the
run without checking the restore, and ask the user to unset
`ACCELERATOR_FORCE` and run `/rekey-work-items` again.

## Step 1: Preview

```bash
accelerator work rekey --preview [--per-item-reads]
```

Branch on the outcome:

- **Exit 0 with mapping lines** on stdout, one `<old> → <new>` per changing
  item: go to Step 2.
- **Exit 0 with no lines**: every item already matches the pattern. Report
  that there is nothing to re-key and stop.
- **`E_REKEY_IN_PROGRESS`**, after the stderr line
  `the run in progress has still to re-key:`: an earlier run is unfinished.
  Show the entries it still has to re-key, printed on stdout. Use the
  `AskUserQuestion` tool with **Finish it** (run `accelerator work rekey`,
  then act on Step 3's outcomes) and **Undo it** (the Undo flow below).
  Under the override note, offer **Finish it** and **Stop** instead.
- **`E_REKEY_IN_PROGRESS` saying a re-key is running (pid `<n>`)**: another
  session is re-keying. Relay that and stop.
- **`E_REKEY_PLAN_UNREADABLE`**: relay the printed remedy and any
  half-applied evidence lines as they are. Never offer to finish the run, and
  never run the forced discard the remedy names. Stop.
- **Another exit 4**: surface the code, its printed remedy and the paths
  listed under it. Nothing was recorded. Stop.
- **Exit 70**: the tracker was unreachable; nothing was recorded. Retry once
  it is reachable.
- **Exit 72-74**: the tracker cannot be selected or configured. Relay the
  printed message, point at `/accelerator:configure`, and stop; nothing was
  recorded.
- **Exit 1** with `E_IDENTITY_RUN_BUSY`, `E_REKEY_STATE_UNWRITABLE`,
  `E_REKEY_IDENTITY_RUN_LOCK_FAILED`, `E_REKEY_CORPUS_INACCESSIBLE` or
  `E_DRAFT_ID_EXHAUSTED`: relay the printed remedy and stop.

## Step 2: Confirm

Print the mapping lines as they are, in a `text` fence, with the override
note if it applies.

With no dirty-path warning, use the `AskUserQuestion` tool:

1. **Yes, re-key** — rename the listed items and rewrite their references
2. **No, discard** — discard the plan; nothing changes

When stderr carries `warning: the working copy has uncommitted changes`,
name the paths listed under it and say the apply will refuse until they are
committed or reverted. Then use the `AskUserQuestion` tool:

1. **Keep the plan and stop** — commit or revert the paths first
2. **Discard** — discard the plan; nothing changes

## Step 3: Apply

On **Yes, re-key**:

```bash
accelerator work rekey
```

On exit 0, each stdout line is `<old> → <new>` and the re-keyed file's
path, separated by a tab. Report each re-keyed item, then go to Step 5. On a
non-zero exit, act on the outcome table; any lines printed before the
refusal are entries that were applied, and they stand.

| Code | What to do |
|---|---|
| `E_REKEY_PREVIEW_STALE` | The work items changed since the preview; name what changed and offer to run `/rekey-work-items` again for a fresh preview |
| `E_REKEY_DIRTY_WORKING_COPY` | Name the paths, say the plan is kept as it was (proposed or in progress), and stop |
| `E_REKEY_WORKING_COPY_UNPROBEABLE` | Name the failure, say the plan is kept as it was, and stop |
| `E_REKEY_ENTRY_ROLLED_BACK` | Say an entry was rolled back; offer to run `accelerator work rekey` again to resume |
| `E_REKEY_ENTRY_REFUSED`, `E_REKEY_RESTORE_INCOMPLETE` | Relay the printed remedy, then offer the Undo flow, unless the override note was shown |
| `E_REKEY_NOT_REVERTED` | The Undo flow, unless the override note was shown |
| `E_REKEY_PLAN_UNREADABLE`, `E_IDENTITY_RUN_BUSY` | Relay the printed remedy and stop |
| Any other code | Surface the code and its printed remedy, and stop |

## Step 4: Discard or keep

On **No, discard** or **Discard**:

```bash
accelerator work rekey --discard
```

Report that no work item changed.

On **Keep the plan and stop**: say that once the paths are committed or
reverted, `/rekey-work-items` previews again and asks, or
`accelerator work rekey` applies this plan if no item's identity changed
meanwhile.

## Undo flow

Never offer it under the override note.

```bash
accelerator work rekey --discard
```

On exit 0 the run plan is gone; report that the re-key was undone. On
`E_REKEY_NOT_REVERTED`, some paths the run wrote still differ from their
state when it started. Show the paths under each `restore from <revision>`
heading with that revision, and the paths under `remove`. The skill does not
restore them: ask the user to restore and remove them, and to say when that
is done. Then run `--discard` again. If it refuses again, show only the paths
still listed and repeat.

## Step 5: Validate

Validate each re-keyed item, using the path each applied line prints:

```bash
accelerator corpus frontmatter validate --file <each re-keyed item path>
```

If any invocation exits non-zero, that item violates the canonical
frontmatter standard; report the emitted violation before completing.

!`accelerator config instructions rekey-work-items --fail-safe`
