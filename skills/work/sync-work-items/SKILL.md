---
name: sync-work-items
description: Reconcile local work items in meta/work/ with the active remote
  tracker named by work.integration. Use when the user wants to sync, push, or
  pull work items to or from Jira or Linear, preview what a sync would change, or
  reconcile divergent local and remote state.
argument-hint: "[--push-only|--pull-only] [--preview] [--all] [filter-flags…]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---

# Sync Work Items

**Active integration**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh integration`
**Default project code**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh default_project_code`
**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work`

`/sync-work-items` reconciles the local work items under the work directory with
the remote tracker named by `work.integration`. It is **on-demand** (never
background), operates against **exactly one** integration per invocation, and
writes can affect remote state — which a local VCS revert **cannot** recover —
so a `--preview` mode is provided to inspect the plan before any side effect.

The safety-critical orchestration lives in tested scripts, not this prose:
`work-item-sync-decide.sh` owns the (mode × state) decision table,
`work-item-sync-classify.sh` owns change detection, and `work-item-sync-apply.sh`
owns the per-item commit sequence (side-effect first, baseline last). This skill
parses arguments, renders decisions, and runs the prompts/gates around them.

## Step 0: Config gate and prerequisites

**Config gate.** The **Active integration** read above gates the whole skill.
`config-read-work.sh integration` exits 0 with a **blank line** when nothing is
configured, so branch on the **string**. If it is empty, print a clear,
actionable error and stop — do not guess a tracker:

```
/sync-work-items needs an active remote tracker, but `work.integration` is not
configured.

  What: the `work.integration` setting selects which remote your work items sync
        with. It is currently unset.
  Why:  sync reads and writes that tracker's API; with no tracker there is
        nothing to reconcile against.
  Fix:  set `work.integration` to one of `jira`, `linear`, `trello`, or
        `github-issues` via /accelerator:configure, then re-run.
```

`<sys>` for every script below is this configured value — never re-derived.
`trello` and `github-issues` are not built yet; the bridges report
"not available" (exit 72) for them, which you surface as a clear message.

**Prerequisites.** Before any remote call, confirm `jq` (with `-S` support),
`sha256sum` or `shasum`, and the repo's VCS binary (`git` or `jj`) are present.
If one is missing, name it and how to obtain it (same what/why/how shape as the
config gate) rather than failing later with a raw `command not found` or a
silently divergent hash.

## Step 1: Parse mode and filters

Resolve the directional mode through the decision script so the
mutually-exclusive rule is enforced in one tested place:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-sync-decide.sh mode \
  [--push-only] [--pull-only]
```

It prints `bidirectional` (the default when neither flag is given), `push-only`,
or `pull-only`, and **errors** (exit 2) if both directional flags are supplied —
surface that error and stop. Other arguments:

- `--preview` — report the full set of intended changes (push, pull, conflict,
  push-unsynced, untracked-pull) **without** any local write or remote API
  write, and **without** touching `last-sync.json`. Combinable with any
  directional flag.
- `--all` — for the untracked-remote pull (Step 4 / Phase 8), drop only the
  implicit `work.default_project_code` scope; user filters still apply.
- remaining flags are tracker `search-*` filter flags, forwarded verbatim.

Omitting the directional flags means **bidirectional**. Example:
`/sync-work-items --push-only --preview` previews only the local→remote pushes.

**Capture the run-start epoch now** (before reading any item) — it becomes the
baseline's global `timestamp` on clean completion, so a file edited *during* the
run is re-hashed (not wrongly short-circuited) on the next run.

## Step 2: Build the remote pre-filter map

Collect the non-empty `external_id`s of the local work items (synced items) and
fetch their remote state in **one** bulk call:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-fetch-remote.sh \
  --integration <sys> search --keys <comma-separated external_ids>
```

It returns `{ "found": {<key>:{updated}}, "absent": [...], "indeterminate": [...] }`
— the adapter picked the per-tracker strategy, so you never branch on tracker. If
the bridge exits non-zero (remote unreachable / timed out), treat **every** key
as `indeterminate`: nothing is written, and each affected item is reported as
needs-retry. Resolve the baseline path and global timestamp once:

```
BASE=$(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-sync-baseline.sh path)
TS=$(jq -r '.timestamp // 0' "$BASE" 2>/dev/null || echo 0)
```

## Step 3: Reconcile synced items

For each local item with a non-empty `external_id`, emitting
`processing item k of N` progress as you go:

1. **Classify** with the shared engine. Derive `--remote-status` from the
   pre-filter map (`found`→`present` with `--remote-updated`; `absent`→`absent`;
   `indeterminate` or degraded→`indeterminate`). For a `found` key whose
   `updated` differs from the baseline entry's `remote_updated_at`, fetch the
   body (`work-item-fetch-remote.sh … show --external-id <key>`), project +
   canonicalise it with `work-item-project-remote.sh --integration <sys> body`,
   write it to a temp file, and pass `--remote-body-file`:

   ```
   STATE=$(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-sync-classify.sh \
     --file <path> --external-id <key> \
     --baseline "$(work-item-sync-baseline.sh get <id>)" --timestamp "$TS" \
     --remote-status <present|absent|indeterminate> \
     [--remote-updated <iso>] [--remote-body-file <tmp>])
   ```

2. **For a `remotely-modified` item, test the local file's cleanliness** before
   any overwrite (the recovery model is VCS revert, which cannot recover
   uncommitted working-copy changes):

   ```
   if work-item-file-dirty.sh <path>; then DIRTY=1; else DIRTY=0; fi
   ```

3. **Decide the action** deterministically:

   ```
   ACTION=$(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-sync-decide.sh \
     decide --mode <mode> --state "$STATE" --dirty "$DIRTY")
   ```

   - `push` → `work-item-sync-apply.sh push` (the **update** bridge, not create:
     the issue already exists). Extract the item's title and body, write the body
     to a temp file, and pass `--integration <sys> --external-id <key> --id <id>
     --file <path> --title <t> --body-file <tmp>`.
   - `pull` → reconstruct the local file (keep the local frontmatter — `id`,
     `external_id`, and the other authored fields — replacing title/body from the
     projected remote), write it to a temp file, project the remote body, and run
     `work-item-sync-apply.sh pull --id <id> --file <path>
     --new-content-file <tmp> --remote-updated <iso> --remote-body-file <proj>`.
     Emit `<id>: local replaced from remote` so the overwrite is visible and
     revertable.
   - `skip-dirty` → skip the pull, **warn** and list the `id` (a dirty local file
     is never silently overwritten). Reported under `needs-retry`.
   - `skip-conflict` → report the `id` under `conflicts-skipped` and write
     neither side. This is the directional-mode outcome (`--push-only` /
     `--pull-only`): resolving a conflict needs a write the mode forbids, so it
     is reported and skipped with **no** prompt.
   - `prompt` → bidirectional conflict resolution (see "Conflict resolution"
     below). The dirty-pull route (`remotely-modified` + dirty in bidirectional)
     also returns `prompt` and is resolved the same way.
   - `noop` → nothing to do (synced, or a forbidden-write cell, or
     `indeterminate`/`remote-absent`). Report `indeterminate` items under
     `needs-retry` and `remote-absent` items under `remote-absent` (never push to
     a non-existent issue).

4. **Aggregate pull-overwrite gate.** If the number of local files a run will
   overwrite from remote exceeds the shared threshold (**25**, the same constant
   the untracked-pull gate uses), pin and evaluate **before any pull write**:

   ```
   N local files will be overwritten from remote. Proceed? [y/N]
   ```

   It **fails safe**: empty input, a non-interactive context, or any non-`y`
   answer aborts the entire pull-overwrite class with **zero** writes and a
   non-zero exit. Never proceed on no answer.

5. **Terminal push handling.** A 71/terminal code from the update bridge is
   **never** auto-retried (a resent request could apply twice on a
   response-uncertain failure): report the item as
   needs-manual-reconciliation and leave its baseline entry **unset** (the apply
   helper already does this), so the next run re-classifies authoritatively.

`work-item-sync-apply.sh` performs each item's side-effect, then sets that id's
baseline entry **last** (per-item resumability). Re-running after a mid-run
interruption is idempotent: reconciled items match their baseline and are skipped.

### Conflict resolution (bidirectional only)

When `decide` returns `prompt`, resolve the conflict interactively. First render a
**section-grouped** diff so a large item stays reviewable — local is the `-`
baseline side, remote is the `+` side (the recommended/default-accept side):

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-section-diff.sh \
  <local-file> <remote-reconstructed-file>
```

Then prompt with a **typed token** (not a `y/n` keystroke — a reflexive Enter
must never discard local edits, and this avoids colliding with the `[y/N]`
polarity used by the batch-push and untracked-pull gates). Pin the exact string:

```
Conflict on <id> (<external_id>). Recommended: keep remote.
Type 'remote' to OVERWRITE your local edits with the remote version,
'local' to push your local version to the remote, or
'skip' to leave both unchanged and resolve it later. [remote/local/skip]
No default — Enter (or an unrecognised entry) re-asks once, then skips.
```

Read the raw input and map it through the tested entry point (never re-derive the
mapping in prose):

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-sync-decide.sh \
  resolve-conflict-token "<raw input>"
```

It returns one action:

- `accept-remote` → resolve as a **pull**: overwrite the local file from the
  remote via `work-item-sync-apply.sh pull` (Phase 6 ordering, incl. the
  post-write `remote_hash`).
- `push-local` → resolve as a **push**: push the local version via
  `work-item-sync-apply.sh push`, and emit an **override-log** line to the
  summary naming the item, e.g. `OVERRIDE <id> (<external_id>): pushed local→remote`.
- `skip` → report under `conflicts-skipped`, write **nothing**.

There is deliberately **no Enter default**: 'Recommended: keep remote' steers the
choice but still requires typing the word, so a reflexive Enter (empty input) or
any unrecognised token re-asks **once**, then resolves to `skip` — never to a
destructive write.

## Step 4: Unsynced push offer and untracked pull

### Unsynced push offer

For each local item with **no** `external_id` (never pushed), offer a push using
**one** pinned grammar (per-item `[y/N]` with the fast-path keys surfaced in the
string so they are discoverable):

```
Push <id> "<title>" to <tracker>? [y/N]  (a = push all remaining, d = decline all remaining)
```

- `a` / `d` touch only **un-decided** items and **never resurrect** declines.
- **Accepted** → push via the **create** bridge:

  ```
  ${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-create-remote.sh \
    --integration <sys> --title <t> --kind <kind> --body-file <tmp>
  ```

  Substitute the returned key into the item's `external_id` line **in memory**,
  then write the whole item (frontmatter incl. `external_id` + body) in a
  **single** `atomic_write`, so the file never exists half-linked.
  `work-item-push-decide.sh` governs retry/terminal handling exactly as
  `/create-work-item` does (a 71/terminal is never auto-retried; the returned
  key, if any, is preserved with loud guidance).
- **Declined** → untouched.
- Under `--preview`: report the intended pushes via the create bridge's
  `--dry-run`; make no write.

### Untracked remote pull

Fetch remote issues via the read bridge, forwarding the user's filter flags
verbatim:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-fetch-remote.sh \
  --integration <sys> search [filter-flags…]
```

- **Default scope** is `work.default_project_code` — for jira this is the search
  flow's own default project, so plain `search` is already scoped; for linear the
  team is catalogue-fixed (single-team), so there is no project scope.
- `--all` forwards the tracker's `--all-projects` primitive (jira only), dropping
  **only** the project clause; any user filters (e.g. `--label`) still apply.
- Compute the **untracked** set: remote issues whose key is **not** already held
  by any local item's `external_id` (a held key is already tracked — never create
  a duplicate).

**Blast-radius gate.** When the untracked set exceeds the shared threshold
(**25** — the same constant the pull-overwrite gate in Step 3 uses), pin, and
evaluate **before any creation write**:

```
N untracked remote issues will be created. Proceed? [y/N]
```

It **fails safe**: empty input, a non-interactive context, or any non-`y` answer
aborts the untracked-pull class with **zero** creations and a non-zero exit. This
stops a mis-scoped `--all` or an automation-flooded project from flooding the
work directory and exhausting IDs.

**Allocate the whole batch up front** — never per item in a loop (which would
hand every pulled item the same number until each file lands):

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-next-number.sh --count N
```

For each issue, build the full frontmatter (incl. `external_id` = remote key and
the allocated `id`) and body in memory, write it in a **single** `atomic_write`,
then record its baseline entry (`work-item-sync-baseline.sh set <id>
<remote_updated_at> <remote_hash> <local_hash>`, with `remote_hash` from
`work-item-project-remote.sh … body | work-item-normalise.sh --stdin` over the
issue's `show` body, and `local_hash` from the just-written file). Re-validate
each allocated `id` is still free immediately before its write and **abort** the
batch on an unexpected collision rather than overwriting (single-writer
assumption). The pull is idempotent across re-runs: a created item now carries an
`external_id`, so it is no longer untracked. Under `--preview`: report the
untracked set, allocate nothing, create nothing.

## Step 5: Persist and summarise

On **clean completion** (and **never** under `--preview`), advance the global
baseline timestamp with the run-start epoch from Step 1:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-sync-apply.sh finalise \
  --timestamp <run-start-epoch>
```

Under `--preview`, run the same classification and decisions but route every
push through the update bridge's real dry-run (`--dry-run`, which forwards
`--print-payload`) and report every pull instead of writing it. **No** baseline
mutation occurs under `--preview` — neither per-item `set` nor `finalise`.

Print a summary grouped by action, listing the affected `id`s (not bare counts)
so the user can see exactly which items changed without re-running:

```
pushed:                <ids>
pulled:                <ids>
pushed-unsynced:       <ids>   (new external_id written back)
pulled-untracked:      <ids>   (remote key → new local id)
conflicts-skipped:     <ids>
overrides:             OVERRIDE <id> (<external_id>): pushed local→remote
needs-retry:           <ids>
remote-absent:         <ids>
unsynced (not pushed): <ids>   (declined)
```
