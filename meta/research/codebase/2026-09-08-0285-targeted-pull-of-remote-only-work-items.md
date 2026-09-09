---
type: "codebase-research"
id: "2026-09-08-0285-targeted-pull-of-remote-only-work-items"
title: "Research: Targeted Pull of Remote-Only Work Items and Resolution Normalisation"
date: "2026-09-08T19:37:42+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0285"
parent: "work-item:0285"
relates_to: ["codebase-research:2026-09-06-0257-sync-specific-work-items"]
topic: "Targeted Pull of Remote-Only Work Items and Resolution Normalisation"
tags: ["research", "codebase", "work-sync", "targeting", "resolve-targets", "tracker", "create-from-remote"]
revision: "818197a2b8f174a92092b8b52236043fcc19ed91"
repository: "accelerator"
last_updated: "2026-09-08T19:37:42+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Targeted Pull of Remote-Only Work Items and Resolution Normalisation

**Date**: 2026-09-08 19:37 UTC
**Author**: Toby Clemson
**Git Commit**: 818197a2b8f174a92092b8b52236043fcc19ed91
**Branch**: HEAD (working copy, no bookmark)
**Repository**: accelerator

## Research Question

Research the codebase for work item 0285 — extend `work sync --target` to pull a
remote-only work item that has no local file, and normalise the targeted
local/remote resolution and collision behaviour that 0257 left local-only. Find
the exact code paths the three target changes touch: (a) the no-local-match arm
must fetch by id and pull; (b) the genuine local/local collision must become an
exit-2 hard error; (c) the ordinary synced item must reconcile with no warning.

## Summary

The change is well-supported by existing machinery but pivots on **three
architectural facts**, one of which contradicts an acceptance criterion.

1. **All three target branches live in one function** — `resolve_targets`
   (`cli/work-cli/src/sync.rs:456`). Its no-local-match arm (`sync.rs:519-527`)
   emits `NoMatch` → exit 3; its `suppressed_remote` arm (`sync.rs:482-494`,
   `438-449`) records the local/local collision as a non-fatal note and lets
   local win; the ordinary synced item already falls through the same
   `Some(item)` arm with no warning. 0285 rewrites the first two and reconciles
   the third.

2. **The tracker is not available during resolution.** `resolve_targets` takes
   only the local corpus and a filesystem `resolver` closure; the tracker is
   resolved *after* selection completes (`sync.rs:699`). Threading a remote
   lookup into resolution is the central plumbing task — the no-local-match arm
   has no network access today.

3. **A targeted pull can reuse `create_from_remote` untouched**
   (`cli/work-adapters/src/sync/apply.rs:270`). It already takes only an
   `&ExternalId`, drives `tracker.show` → `author_from_remote` → baseline with
   `local_synced_at = run_start_epoch`, and bypasses discovery. The single
   `discovery_suppressed()` branch at `run.rs:786-787` is what strips targeted
   imports from pull accounting, preview, and apply in one place; a targeted
   pull must inject a non-empty `untracked` set there.

⚠️ **A `show` on a non-resolving id is a `TrackerError::Retryable`, not an
absence signal** (`cli/tracker/src/lib.rs:413-425`). The trait explicitly says
absence is established with `fetch_all`, not `show`. Acceptance criterion
"matches neither a local file nor a remote issue → exit 3" (work item lines
108-109) is therefore **not achievable through `show` alone** — a genuinely
absent remote id and a transient network fault are indistinguishable, and both
map to exit **70 (RETRYABLE)** through an engine run, never exit 3. This is the
one design decision the plan must resolve up front. See Open Questions.

## Detailed Findings

### Crate layout

The sync surface spans four crates with a clean division of labour, governed by
ADR-0045 (skills-vs-CLI) and ADR-0044 (`external_id` identity):

| Crate | Path | Responsibility |
|---|---|---|
| `work-cli` | `cli/work-cli/src/` | argv→flags, target resolution, exit codes, TSV report render |
| `work-adapters` | `cli/work-adapters/src/sync/` | engine: run orchestration, per-item apply, create, discovery, baseline |
| `work` | `cli/work/src/sync/` | pure domain: plan/classify/decide, push precondition |
| `tracker` + `linear-client` | `cli/tracker/`, `cli/linear-client/` | remote port trait + Linear implementation |

Exits 2 and 3 originate **pre-flight in `work-cli`**, before the engine runs;
they are not `RunError` variants (`sync.rs:407-418`, `694-697`). Engine-phase
failures map to a different band (70/71 tracker, 4 unresolved, 5 refused).

### Target resolution — the three branches (`cli/work-cli/src/sync.rs`)

`resolve_targets` (`sync.rs:456-550`) resolves each `--target` token against the
local corpus only:

```rust
fn resolve_targets(
    corpus: &[LocalItem],
    targets: &[String],
    resolver: &dyn Fn(&str) -> RunOutcome,
) -> Result<ResolvedTargets, Vec<TargetResolutionFailure>>
```

It builds `external_id_index` once (`sync.rs:420-433`, a
`BTreeMap<String, Vec<&LocalItem>>` keyed by `canonical_external_key` —
whitespace-stripped, ASCII-upper-cased), then dispatches per token on
`resolver(token)`. Three branches matter:

- ❌ **(a) No-local-match arm** (`sync.rs:514-527`). When the resolver returns
  `NotFound`/`Invalid` and the external-id index has no entry, it pushes
  `TargetResolutionFailure::NoMatch` → exit 3, with the message "untracked
  remote issues are imported only by a full (untargeted) sync". This is the
  arm 0285(a) inverts to a remote `show` + pull.
- ❌ **(b) Collision arm** (`sync.rs:482-494` + `suppressed_remote` at
  `438-449`). When a token resolves locally to `item` and the same token also
  keys a *different* file's `external_id` (`item.id != local_match.id`), it
  records a `Suppressed` note and lets local win — a non-fatal
  `#\ttarget\tsuppressed\t…` report line. 0285(b) turns this into a
  `TargetResolutionFailure` → exit 2 naming both files.
- ✅ **(c) Ordinary synced item** (same `Some(item)` arm). The
  `item.id != local_match.id` guard (`sync.rs:447`) already excludes the item
  from warning about itself, so the `id == external_id` case emits no note
  today. What is missing is remote reconciliation — never triggered because the
  tracker is absent from resolution.

Relevant types: `RunOutcome` (`resolve.rs:25-31`), `LocalItem`
(`fetch.rs:26-30`), `TargetResolutionFailure` (`sync.rs:385-393`), `Suppressed`
(`sync.rs:366-374`), `ResolvedTargets` (`sync.rs:376-380`). Exit mapping at
`sync.rs:407-417`; precedence `USAGE > OUTSIDE_WORKDIR > NOT_FOUND` at
`sync.rs:566-577`.

⚠️ **The tracker is nowhere in resolution.** The `resolver` closure is
`crate::resolve::resolve_with` — filesystem-only, no network (`resolve.rs:92-132`).
The tracker is resolved by `registry.resolve(&integration)` only after
`build_selection` returns (`sync.rs:699-716`), deliberately so target validation
precedes the credential check (`sync.rs:654-655`). To fetch a remote-only token,
0285 must thread the tracker (or a lookup port) into `resolve_targets`.

### Create-from-remote — the reusable pull path (`cli/work-adapters/src/sync/`)

`create_from_remote` (`apply.rs:270-309`) is the clean reuse target:

```rust
pub fn create_from_remote(
    &mut self,
    external_id: &ExternalId,
    author: &dyn LocalAuthor,
) -> Result<String, ApplyError>
```

It fetches `tracker.show(external_id)` (`apply.rs:275`), authors the local file
via `author.author_from_remote(&DiscoveredIssue { external_id, issue })`
(`apply.rs:283`), hashes what was written, then writes the baseline `Entry` with
`local_synced_at: self.run_start_epoch` (`apply.rs:297-307`). It takes only an
`&ExternalId` — no marker, no `corpus_carries` — so a targeted by-id pull routes
through it unchanged, bypassing `discover_untracked` entirely.

Id and filename allocation live inside `ConfiguredLocalAuthor::author_from_remote`
(`cli/work-cli/src/sync_author.rs:97-159`): a create lock
(`.accelerator-work-create.lockdir`), `allocate_id` (scan-highest-then-increment
over on-disk filenames, `create.rs:183-200`), `slugify` → `{id}-{slug}.md`, and
`exclusive_write` (refuses to clobber). Imported taxonomy is fixed:
`kind=task`, `priority=medium`, `status=ready`, `producer=sync-work-items`,
`external_id=Some(remote key)` (`sync_author.rs:36-39`, `126-145`). A targeted
pull-create therefore produces a byte-identical file to a discovery import
(acceptance criterion lines 95-97 satisfied by construction).

The `corpus_carries` double-binding guard (`push_precondition.rs:66-95`) is a
create-**from-local** concern only — it guards the crash window between an
`external_id` write-back and marker delete. It reads `request.corpus` (the whole
set) regardless of selection, so a narrowed targeted run still judges what the
whole corpus carries (`run.rs:914-919`).

### Run orchestration — where the pull plugs in (`cli/work-adapters/src/sync/run.rs`)

The pull bound is computed in `prepare_run` before any write:

```rust
let pulls = plan.pull_count() + untracked.len();          // run.rs:822
if pulls > request.max_pulls || pushes > request.max_pushes {
    return Err(RunError::Refused { ... });                 // run.rs:824-832
}
```

`untracked.len()` folds create-from-remote imports into the pull dimension
(`run.rs:819-821`); `RunError::Refused` breaks out `new_local_files` /
`new_remote_issues` for the operator message (exit 5, `sync.rs:812-828`).

⚠️ **One branch gates everything targeted.** `discovery_suppressed()`
(`run.rs:138-141`) is true for `ItemSelection::Targeted`, and `prepare_run`
short-circuits `untracked` to empty at `run.rs:786-787` before scope resolution.
That single branch simultaneously removes targeted imports from pull accounting
(`run.rs:822`), the preview loop (`run.rs:886-892`), and the apply loop
(`run.rs:929-940`). For 0285, a targeted pull-create must produce a **non-empty,
targeted `untracked` set** at that branch — not a discovery search — so it flows
through accounting, preview (zero writes, `Action::CreateFromRemote` /
`NotApplied`), and apply unchanged.

The full-sync (untargeted) write set is structurally the same code path: for
`ItemSelection::All`, `reconciled() == corpus`, and the only two selection-keyed
branches (`discovery_suppressed()` and `advance_document`, `run.rs:232`) both
take their original `All` value. This is what keeps an untargeted run
byte-identical (acceptance criterion lines 125-127) — provided the targeted
injection does not perturb the `All` path.

### Tracker port — `show` semantics (`cli/tracker/src/lib.rs`, `cli/linear-client/src/client.rs`)

`RemoteTracker::show` (`lib.rs:430`): `fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError>`.
Linear implements it as a single by-id GraphQL query
`issue(id: $id) { id identifier title updatedAt description }`
(`client.rs:55-60`, `515-553`, `617`). `RemoteIssue { updated, body }`
(`lib.rs:107-132`) is exactly the type `author_from_remote` consumes, so a
`show` result feeds the create path identically to discovery.

⚠️ **Absence is not discoverable through `show`** (`lib.rs:413-425`): "a
`RemoteIssue` or an error are the only outcomes"; a non-resolving id is a
`TrackerError::Retryable` (a read mutates nothing, so never `Terminal`). The
contract directs callers to establish absence with `fetch_all`
(`lib.rs:451`, `FetchOutcome`). Through a sync run, a retryable failure becomes
`ItemOutcome::Failed` → exit **70 RETRYABLE** (`sync.rs:240-268`,
`exit_codes.rs:69-75`), never exit 3.

The config key `work.integration` selects the implementation in
`ConfiguredTrackers::resolve` (`tracker_registry.rs:155-189`): `linear` →
`LinearClient`, `jira` → `JiraClient`, `trello`/`github-issues` → `NotAvailable`,
`""` → `Unset`.

### Skill rendering (`skills/work/sync-work-items/SKILL.md`)

The skill is prose-only — no formatter module; it consumes the engine's stdout
TSV report as authoritative and re-expresses it. Four passages must change:

- **Line 84-86** — "a targeted pull reaches only items already tracked locally"
  becomes factually wrong; rewrite to say a remote-only target is fetched by id
  and pulled.
- **Line 91-92** — narrow exit 3 from "no local id, path, or external_id" to
  "no local file **nor** remote issue".
- **Line 274 / 291-296** — admit a *targeted* create-from-remote into the
  `pulled-untracked:` group and preview, counted against `--max-pulls`.
- **Line 82-85 / 95-98 / 287-289** — remove the non-fatal "Suppressed remote
  match" note; the ordinary synced item reconciles silently, and the genuine
  local/local collision becomes an exit-2 usage error naming both files.

## Code References

- `cli/work-cli/src/sync.rs:456-550` — `resolve_targets`, the three target branches
- `cli/work-cli/src/sync.rs:438-449` — `suppressed_remote`, the collision detector (→ exit 2)
- `cli/work-cli/src/sync.rs:514-527` — no-local-match arm (`NoMatch` → exit 3), the (a) inversion point
- `cli/work-cli/src/sync.rs:420-433` — `external_id_index` construction and keying
- `cli/work-cli/src/sync.rs:699-716` — tracker resolved *after* selection (the plumbing gap)
- `cli/work-cli/src/sync.rs:755-758` — `Scope` → `ItemSelection` bridge
- `cli/work-cli/src/sync.rs:198-238` — `render_report` TSV serialiser
- `cli/work-cli/src/sync.rs:240-268` — `exit_code_for_report` (engine-phase codes)
- `cli/work-cli/src/exit_codes.rs:55-75` — code taxonomy (USAGE=2, RESOLVE_NOT_FOUND=3, 70/71 tracker)
- `cli/work-adapters/src/sync/apply.rs:270-309` — `create_from_remote` (reusable pull path)
- `cli/work-adapters/src/sync/apply.rs:297-307` — baseline `Entry` with `local_synced_at` watermark
- `cli/work-adapters/src/sync/run.rs:786-787` — `discovery_suppressed()` gate (the single injection point)
- `cli/work-adapters/src/sync/run.rs:822-832` — pull accounting and `--max-pulls` refusal
- `cli/work-adapters/src/sync/run.rs:883-908` — preview branch (zero-write would-create rows)
- `cli/work-adapters/src/sync/run.rs:929-940` — apply-mode `create_from_remote` call site
- `cli/work-cli/src/sync_author.rs:97-159` — `author_from_remote` (id/filename allocation, taxonomy)
- `cli/work/src/sync/push_precondition.rs:66-95` — `corpus_carries` double-binding guard
- `cli/tracker/src/lib.rs:413-430` — `show` contract (absence not discoverable)
- `cli/tracker/src/lib.rs:451` — `fetch_all` / `FetchOutcome` (the absence-capable read)
- `cli/linear-client/src/client.rs:55-60,515-553,617` — Linear `show` by-id query
- `cli/work-cli/tests/cli_sync_targets.rs` — targeted-sync exit-code tests to extend
- `skills/work/sync-work-items/SKILL.md:82-98,274,287-296` — rendering to revise

## Architecture Insights

- **Resolution is filesystem-pure by design.** 0257 deliberately kept the
  tracker out of `resolve_targets` so target validation is credential-independent
  and runs before any network contact (`sync.rs:654-655`). 0285 breaks that
  invariant intentionally; the plan should decide whether the remote lookup lives
  *in* resolution (enabling a pre-flight exit 3) or *after* it in the engine
  (pushing remote-absent to exit 70). This choice drives the exit-code semantics.
- **`ItemSelection::Targeted` is a single lever.** It gates discovery, preview
  imports, apply imports, and document-watermark advance from one enum. A
  targeted pull must extend `Targeted` to carry a set of confirmed remote-only
  ids for import, rather than always forcing `untracked = empty`.
- **The create path is already id-driven, not discovery-driven.**
  `create_from_remote(&ExternalId, …)` means the plan needs a way to inject an
  explicit id list, not to re-run `search`. This aligns with 0285's "by-id fetch,
  no search" assumption (work item lines 154-156).
- **Two exit-code bands, two phases.** Codes 2/3/6 are resolution-phase
  (`work-cli`, pre-engine); 70/71/4/5 are engine-phase. Which phase performs the
  remote lookup determines which band a remote-absent target lands in.

## Historical Context

- `meta/research/codebase/2026-09-06-0257-sync-specific-work-items.md` — the
  closest prior art; analyses `resolve_targets`, `create_from_remote`,
  `external_id_index` as built by 0257 (the code 0285 extends).
- `meta/plans/2026-09-06-0257-sync-specific-work-items.md` — 0257's plan; the
  exit-3 abort, discovery suppression, and pull-bound accounting 0285 modifies
  all originate here.
- `meta/research/codebase/2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear.md`
  — discovery suppression and targeted-vs-discovery gating on Linear.
- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md` — the tracker
  port surface, including `show` by-id fetch semantics.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` —
  `external_id` as remote identity; the basis for resolution and collision keys.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — why resolution
  and exit codes live in the CLI, not the skill.
- `meta/work/0146-work-item-sync-enhancements.md` — parent epic; `0229` (pull
  scope) and `0255` (chunk merge) touch the same engine path (work item lines
  146-150).

## Related Research

- `meta/research/codebase/2026-09-06-0257-sync-specific-work-items.md` (direct predecessor)
- `meta/research/codebase/2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear.md`
- `meta/research/codebase/2026-08-18-0213-conversational-conflict-resolution-flow.md`
- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`

## Open Questions

- ❓ **Exit 3 for a remote-absent target is not reachable via `show`.**
  Acceptance criterion (lines 108-109) requires "neither local nor remote → exit
  3", but `show` cannot distinguish an absent id from a transient fault — both
  are `TrackerError::Retryable`. Resolve one of: (i) use `fetch_all([id])`
  (`lib.rs:451`), which reports absence, to gate the pull and produce a clean
  exit 3; or (ii) accept remote-absent → exit 70 and amend the criterion. Option
  (i) is the only path that honours the criterion as written. This is the
  headline decision for the plan.
- ❓ **Where does the remote lookup run — resolution or engine?** A pre-flight
  exit 3 requires threading the tracker into `resolve_targets` (breaking the
  credential-independence invariant, `sync.rs:654-655`). An engine-phase lookup
  keeps resolution pure but pushes remote-absent into the 70 band. The two are
  coupled to the question above.
- ❓ **Watermark on re-run.** Acceptance criterion (lines 119-121) requires a
  completed targeted pull to advance `local_synced_at` and not re-pull. The
  baseline `Entry` is written on create (`apply.rs:297-307`), so re-run
  classification should treat it as synced — confirm the second-run path with
  the item now local resolves via the ordinary synced-item arm (c), not the pull
  arm.
