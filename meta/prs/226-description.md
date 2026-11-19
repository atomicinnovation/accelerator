---
type: "pr-description"
id: "226"
title: "[0218] Revise and accept ADR-0074 on pruning only a private cache root"
date: "2026-11-16T16:09:51+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0218"
parent: "work-item:PP-747"
relates_to: ["adr:ADR-0074", "adr:ADR-0072", "work-item:PP-944", "work-item:PP-945", "work-item:PP-946"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/226"
pr_number: 226
tags: ["adr", "cache", "launcher", "retention", "security"]
revision: "302f92af00137c467be9947d0d970f114022d310"
repository: "accelerator"
last_updated: "2026-11-16T16:09:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0218] Revise and accept ADR-0074 on pruning only a private cache root

## Summary

ADR-0074 was proposed alongside #170, which built 0218's claim-gated
pruning. It decides that the launcher prunes a relocated cache root only
when a walk from the root up to `/` shows that no other user can write
anywhere on that path. This PR revises the ADR after review and accepts it.

The review checked the ADR's technical claims against the code in
`cli/launcher/src/launch/outbound/resolve/`. Most held. Three went further
than the code, and one obvious alternative was missing. This PR changes
only the ADR, not the code.

## Changes

### A missing option, now considered

- **Option 4: remove only through descriptors.** The launcher would open
  each directory from `/` down with `O_NOFOLLOW` and remove entries
  relative to the open handle, as std's `remove_dir_all` does. A swapped
  name could then never redirect a removal. This is the textbook answer
  to a swap between the check and the removal, and it would keep shared
  roots prunable.
- **Why we pass on it.** It stops the redirection, but it would still
  prune a shared root while ignoring every other user's claims, so it
  would still evict their live versions. A shared root is only worth
  pruning once its users' claims and writes can be trusted, which is the
  work of epic 0340. Until then, refusing costs less than rewriting every
  removal around directory handles.
- The chosen option, the walk, is renumbered from 4 to 5.

### Claims narrowed to what the code does

- **What needs a `SafePruneRoot`.** The ADR said every removal in a
  relocated root takes one. In the code, only the prune and reap entry
  points require one. The launcher deletes its own temporary files while
  writing an entry without one: the probe, the temporary pointer, the
  temporary archive and the partial download. The ADR now says "every
  prune and reap" and states that this temporary-file cleanup is not
  gated.
- **Volumes that ignore ownership.** The check reads macOS's
  `MNT_IGNORE_OWNERSHIP` flag and always answers false elsewhere. The ADR
  now says it is macOS-only and why Linux needs no equivalent: Linux
  enforces the modes a volume reports.
- **Remedies.** A refusal because a directory cannot be examined, or is
  not a directory, names the directory but offers no fix. The drivers,
  the decision and the positive consequence now say "where one exists",
  and a new negative records the gap.
- **The suggested per-user directory.** The ADR quoted
  `ACCELERATOR_CACHE_DIR=<shared>/$USER`, which the refusal never prints.
  It now uses the refusal's own wording: "a per-user directory on a
  native filesystem".
- **Bookkeeping directories.** These are `claims/`, `trees/` and
  `trees/claims/`. The ADR said they share the walk's "verdict", but they
  skip the volume check. It now says they are judged "by the rules the
  walk applies to a root".

### Behaviour now recorded

- The root is canonicalised once, and a symlink anywhere on the walk
  refuses.
- Tree claims are written without the walk, into a `trees/claims/` that
  passes the root rules. Writing a claim removes nothing.

### Links and status

- Shared-root follow-up: epic 0340 and stories 0341 (use a group-shared
  root safely) and 0342 (prune one) are added to `relates_to` and
  References. The "never pruned" negative now points to them.
- The `getent` rule is trimmed to "without trusting the caller's
  environment and under a deadline". The specifics belong to the plan,
  and the fixed-path detail survives in the NixOS negative.
- The status moves from `proposed` to `accepted`, so the ADR is now
  immutable.

## Context

- ADR: `meta/decisions/ADR-0074-prune-only-a-relocated-root-no-other-user-can-write-to.md`
- It narrows: `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
- Work item: `meta/work/PP-747-bound-cache-root-growth.md`
- Implementation: #170 (merged)
- Shared-root follow-up: `meta/work/PP-944-shared-cache-roots-on-multi-user-systems.md`,
  `meta/work/PP-945-use-a-group-shared-cache-root-safely.md` and
  `meta/work/PP-946-prune-a-group-shared-cache-root.md`

## Testing

- [x] `accelerator corpus frontmatter validate` passes on the ADR in both
      its revised and its accepted state.
- [x] The revised claims were checked against the code; the call sites
      are summarised under Changes. Each claim was confirmed or reworded
      to match.

## Notes for Reviewers

- **Is the reason for passing on Option 4 right?** That is the substantive
  judgement in this PR. A reader may ask why not combine Options 2 and 4:
  descriptor-based removal that also honours other users' claims. The
  ADR's answer is that claims and writes in a shared root can't be trusted
  before 0341, and 0342 is essentially that combination plus shared locks.
- **The ADR is accepted.** Further changes need a new ADR that supersedes
  it.
- **`last_updated` was not bumped.** It still reads 2026-11-09, the date
  the ADR was created.
