---
type: work-item
id: "0194"
title: "Tracker Crate and Remote Sync Engine"
date: "2026-08-05T18:18:52+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0170", "work-item:0174"]
blocked_by: ["work-item:0204"]
tags: [rust, work-items, sync, tracker]
last_updated: "2026-08-12T00:20:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0194: Tracker Crate and Remote Sync Engine

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the sync state machine in the existing `work` / `work-adapters`
pair over the `RemoteTracker` port 0204 defines, the `accelerator work
sync` command that orchestrates it against the local work-item store, and
the `--push` wiring onto 0170's `create`/`update` commands — plus the
test-migration work that holds the new engine to the bash one: the fixture
tables lifted from nine suites, a new `work-item-sync-label.golden`, and a
bash-generated baseline corpus captured while its generator still exists.
The whole pipeline is unit-tested against a fake `RemoteTracker`, so
nothing here waits on a real provider client. The binary ships beside the
bash path rather than replacing it: the cutover — retiring the scripts and
repointing the skills — belongs to 0171, which supplies the real tracker
clients that make the binary usable.

## Context

This story delivers a typed, bash-3.2-independent sync engine, tested
against the same fixture tables as the nine shell scripts it will replace,
and the narrow port that unblocks 0171. It changes nothing a user sees:
the bash path stays live and the binary ships beside it. The replacement —
and with it the explicit conflict handling users get in place of decisions
buried in bash control flow — arrives when 0171 performs the cutover.

`skills/work/scripts/work-item-sync-*.sh` implements a transactional state
machine (classify → decide → apply → baseline → label) that orchestrates the
remote tracker. This story split off from 0170 (the work-item lifecycle
subdomain — see that item's Drafting Notes) on 2026-08-05 following a work
item review that found the two efforts independently deliverable: the
sync engine builds on the already-built shared `corpus`/`config`/`store`
crates (0166), not on 0170's own CRUD implementation, whereas 0171's
client adapters need the `RemoteTracker` port to exist first.
`create`/`update --push` were originally scoped to 0170 too, calling
through this crate's port; a follow-up review found that made 0170
dependent on this crate for no good reason, since only the `--push` flag
needed the port, not the rest of 0170's CRUD surface. `--push` support (and
the `work-item-create-remote.sh`/`work-item-update-remote.sh`/
`work-item-push-decide.sh` scripts that implement it) moved here instead:
this story now wires `--push` onto 0170's `create`/`update` commands once
they exist, so 0170 itself carries no dependency on the `RemoteTracker`
port at all. The `RemoteTracker` port lives in its own minimal `tracker`
crate and the sync state machine in the existing `work` / `work-adapters`
pair; `accelerator-work` links the per-provider client adapters (0171)
in-process at its composition root and fakes the port in tests.

A 2026-08-10 pass read the source scripts directly and corrected several
claims this item had carried since the split — the classification
vocabulary, the baseline's storage format, and the size of the
characterization gap — and pulled in three pieces of designed-but-
unrecorded behaviour: `--preview` mode, interactive conflict resolution,
and the per-tracker projection seam. See Drafting Notes.

## Requirements

- Consume the `RemoteTracker` port from 0204 rather than defining it here,
  and treat its signature as frozen at 0204's acceptance. This story is
  the port's first consumer, so it is the one most likely to want the
  surface changed — but 0171's client adapters are being implemented
  against that same signature in parallel, so reopening 0204 would
  reintroduce the moving contract the split existed to remove. If the sync
  flow or the `--push` wiring needs surface the port does not offer, it
  lands as a new additive port item, not as an edit to 0204 and not as a
  local workaround here.
- Implement the pending-push marker that keeps `create --push` retries
  idempotent — written before the remote call and cleared after — rather
  than reaching for a lookup operation on the port. 0204 has no lookup
  operation precisely because this marker is the agreed mechanism; that
  decision is what makes its four-operation surface final.
- Site the sync state machine in the existing `work` / `work-adapters`
  pair rather than in `tracker`: classify and decide are pure functions
  over hashes and enums that reason about work-item state, so they belong
  in the `work` domain crate; apply and baseline touch `store`, which no
  domain crate may import, so they belong in `work-adapters` alongside
  the client adapters.

  **One boundary question is open and must be decided before classify is
  written.** The classifier takes a pre-fetched remote record, which is a
  `tracker` type — but `work_domain_imports_only_permitted`
  (`cli/pup.ron`) permits only `std`/`core`/`alloc`, `kernel::Error`,
  `corpus` and `crate`, so `work` cannot import `tracker` as it stands.
  Either widen that rule with `^tracker(::|$)`, justified as a port-crate
  allowance and probed like its siblings, or have `work-adapters`
  translate `RemoteTimestamp`/`RemoteIssue` into work-domain values at
  the boundary and keep `work` unwidened. Decide deliberately: this will
  otherwise surface as a red `pup:check` mid-implementation, where the
  cheapest fix is the one that erodes the boundary `tracker` exists to
  draw.
- Place per-tracker projection **behind** the port: adapters return an
  already-projected `RemoteIssue { updated, body }` in domain terms rather
  than raw tracker JSON. This is what makes the no-provider-types
  constraint true in substance, and it preserves by construction the
  anti-drift property `work-item-project-remote.sh` currently holds by
  convention — that the path writing a `remote_hash` and the path later
  reading it use one recipe. `work-adapters/src/project_remote.rs`
  implements the projection in Rust, but **this story does not wire it to
  the port**: it sits behind a `work` dependency, so a 0171 client
  reusing it would acquire the whole lifecycle domain, and it projects
  the `show` payload shape rather than the bulk one. Each provider client
  owns reproducing its own recipe (0171); this story consumes the
  already-projected `RemoteIssue` the port returns.
- Implement the classifier over the full **seven**-keyword vocabulary:
  `synced`, `unsynced`, `locally-modified`, `remotely-modified`,
  `conflict`, `remote-absent`, `indeterminate`. Five carry a label
  `/list-work-items` renders per item; `remote-absent` and `indeterminate`
  are deliberately caller-handled — the listing shows only whether an
  `external_id` is present, without judging sync state, and `sync` skips
  the item. The classifier never fetches — the caller hands it a
  pre-fetched remote record.

  That read is now **two tiers of one operation**, not a choice between
  two. `fetch_all()` returns stamps only (`FetchOutcome.found` pairs an
  `ExternalId` with a `RemoteTimestamp`), because no provider's bulk
  query carries a projected body; the caller compares each stamp against
  the baseline and then calls `show()` for the minority whose stamp
  moved. A per-item mode that calls `show()` for everything remains
  available, but bulk mode is bulk-then-`show`, not bulk-instead-of.
- Implement the decision table over its three inputs — (mode × state ×
  local-dirty) → `push | pull | skip-conflict | skip-dirty | prompt |
  noop` — preserving every forbidden-write cell: a conflict or
  remote-ahead item under `--push-only` never pulls; a local-ahead item
  under `--pull-only` never pushes; an `indeterminate` or `remote-absent`
  item never writes either side; a dirty local file is never silently
  overwritten by a pull.
- Keep **every** `accelerator work` command that touches the remote
  non-interactive — `sync`, and `create`/`update` under `--push`. They
  take orders, they never ask questions: no prompting, no reading stdin
  for a decision, no blocking on input. Any judgment call the model is
  better placed to make than the binary belongs in the SKILL, which
  collects the answer and re-invokes the command with it.
- Implement conflict resolution as a two-invocation flow. On the first
  run, items the decision table routes to `prompt` (bidirectional
  `conflict`, and bidirectional dirty-pull) are **reported and skipped**
  with neither side written. The SKILL collects the user's choice and
  re-invokes with explicit resolution orders
  (`--resolve <id>=<remote|local>`), which the command applies. The
  typed-token resolver stays a real seam at the CLI boundary: `remote` →
  accept-remote and `local` → push-local after trimming and case-folding;
  empty **or any unrecognised token** → skip. The safe default must never
  be a destructive write.
- Specify the conflict report as a machine-parseable contract, since the
  SKILL consumes it across a process edge and cannot be refactored in
  step with the binary: one line per unresolved item carrying the work
  item id and its classified state, and an exit code distinct from both
  clean success and error, so the SKILL can tell "nothing to resolve"
  from "resolution needed" without parsing prose.
- Verify the two-invocation flow end to end within this story by driving
  it from a test harness rather than from a SKILL: a first run over a
  conflicting corpus produces the report, and a second run carrying
  `--resolve` orders derived from it applies them. The SKILL that will
  drive this conversationally arrives with 0171's cutover.
- Implement
  `accelerator work sync [--push-only|--pull-only] [--preview]
  [--resolve <id>=<remote|local>]...` in `accelerator-work`, orchestrating
  the classify → decide → apply → baseline → label pipeline against the
  local work-item store (via the shared `corpus`/`store` crates from 0166)
  and the active provider client, selected at the work binary's
  composition root per `work.integration` (the config key naming the
  active remote tracker provider, e.g. `jira` or `linear`). This story
  builds the selection seam and exercises it against a fake; the real
  clients it selects are 0171's deliverable.
- Implement `--preview` as a mode that runs the same classification and
  decisions but routes every mutation to a no-op: no local file write, no
  remote `create`/`update` call, **no** per-item baseline `set` and **no**
  `finalise`. A preview that advanced the global timestamp would poison
  the next real run's mtime pre-filter — the run-start epoch recorded by
  `finalise`, below which unmodified files are skipped without hashing.
- Preserve the baseline's storage contract: a single JSON document
  (`last-sync.json`, `{timestamp, items: {<id>: {remote_updated_at,
  remote_hash, local_hash}}}`) written atomically — not JSONL. Preserve
  its degrade-to-empty rule: a missing **or present-but-unparseable**
  baseline is a valid empty baseline, so a VCS-conflict-markered file
  degrades to presence-only plus a full re-hash rather than crashing.
- Preserve the resumability contract: the per-item side effect is written
  first and that item's baseline entry last; the global timestamp
  (run-START epoch) advances only on clean completion.
- Close the coverage gap: `work-item-push-decide.sh` is the **only**
  sync-side script with no test coverage today and needs a
  characterization test before the Rust replacement lands. `sync-baseline`,
  `sync-classify`, `sync-decide`, `sync-label` and `project-remote` are
  already covered by sections within `test-work-item-scripts.sh`, and
  `sync-apply`, `fetch-remote`, `create-remote` and `update-remote` have
  dedicated `test-work-item-*.sh` suites — all of these need porting only,
  not new characterization tests, and none is removed here.
- Lift the existing bash fixture data into Rust test data — the classify,
  decide, baseline and label tables from `test-work-item-scripts.sh`, and
  the `test-work-item-*.sh` cases for `sync-apply`, `fetch-remote`,
  `create-remote` and `update-remote` — and assert the Rust
  implementations against the lifted tables. The bash scripts and their
  suites stay live through this story and keep passing against the same
  tables, so both implementations are held to one oracle until 0171's
  cutover retires the bash side. No bash harness is repointed at the
  binary: each implementation is tested against the shared table, not
  against the other.
- Capture the bash-generated baseline corpus as committed test data while
  `work-item-project-remote.sh` still exists — it is the only thing that
  can produce `remote_hash` values by the original recipe, and it is the
  oracle for the classification-stability criterion that protects users
  from mass reclassification after the cutover.
- Wire `--push` onto 0170's `accelerator work create`/`update` commands,
  calling through this crate's `RemoteTracker` port. On `create --push`,
  create the remote issue and substitute `external_id` before the single
  write, so a successful push yields exactly one file already carrying its
  remote identity. When the remote call fails, write the file without
  `external_id` (saved unsynced) and report which of
  `work-item-create-remote.sh`'s outcome rows applied. The
  local-fallback judgment that bash resolves by prompting is not the
  binary's to make: it reports and exits, and the SKILL decides. A retry
  after a failed push must never issue a second remote `create` for the
  same work item.
- On `update --push` targeting a synced item, replace the remote issue via
  the same whole-content contract as `work-item-update-remote.sh`. When
  the remote replace call fails, **leave the local file untouched** and
  exit non-zero in both cases, surfacing that script's existing
  retryable-vs-terminal distinction: `E_DISPATCH_RETRYABLE` (provably no
  mutation, safe to retry) reports a retry-safe failure and leaves the
  baseline entry alone; `E_DISPATCH_TERMINAL` (mutation state uncertain,
  never auto-retried) additionally clears the item's baseline entry, so
  the next `sync` classifies it `indeterminate` and writes neither side
  until a human resolves it. Neither case may leave the local file
  silently diverged from a replace that may have actually applied.
- Partition tests so the default `cargo test`/`cargo nextest run` invocation
  exercises the parity/characterization suite only (no live network calls);
  gate the contract/integration suite exercising real remote calls behind a
  separate, explicitly-tagged cargo-nextest filter excluded from that
  default invocation.
- Do **not** cut the production path over to the binary in this story, and
  do not remove any bash script. `accelerator work sync` and
  `create`/`update --push` can only resolve fakes until 0171 supplies the
  real Jira and Linear clients, so retiring
  `work-item-sync-{apply,baseline,classify,decide}.sh`,
  `work-item-{fetch,project,create,update}-remote.sh` and
  `work-item-push-decide.sh` here would leave the user-facing sync and
  push flows non-functional between this story and 0171 — against the
  epic's rule that the plugin stays working at every step. The cutover —
  script removal, SKILL-caller repointing, the sync SKILL's conversational
  conflict flow, and the `_EXPECTED_WORK_SUITES` decrement — is 0171's,
  performed once its clients land. This story ships the binary complete
  and tested; 0171 flips the switch.
- Deliver the conflict report as a contract the cutover can code against,
  even though no SKILL consumes it yet: the binary side (the report shape,
  the exit code, and `--resolve` handling) lands here, and the
  conversational half that renders and re-invokes lands in 0171 alongside
  the SKILL repointing that gives it a caller.
- Do **not** remove `work-item-sync-label.sh` or `work-item-normalise.sh`
  in this story — both have live consumers outside the sync engine (see
  Technical Notes). `work-item-normalise.sh`'s behaviour is already ported
  (`work/src/normalise.rs`); the script stays for its remaining shell
  consumers, gated by the existing
  `test-fixtures/work-item-normalise.golden`. `work-item-sync-label.sh`
  needs porting, with the script likewise left in place, and needs a new
  `test-fixtures/work-item-sync-label.golden` covering each of the seven
  classified states — no label fixture exists today, only a section
  inside `test-work-item-scripts.sh` that 0171's cutover deletes. Neither
  script
  is re-pointed at the binary via a shim: both keep an independent bash
  implementation, and each duplication is held safe by testing **both**
  implementations against its shared golden fixture rather than by a
  process hop.

## Acceptance Criteria

- [ ] Given a local item and a fake `RemoteTracker` record, when
      `accelerator work sync` runs its classify → decide → apply →
      baseline → label pipeline, then the seven-keyword classification
      (`synced` / `unsynced` / `locally-modified` / `remotely-modified` /
      `conflict` / `remote-absent` / `indeterminate`) matches
      `work-item-sync-classify.sh` on its own fixture table, lifted into
      Rust test data and asserted against both implementations while the
      script remains live. This includes the
      first-sync-on-dirty case, where an item carrying an `external_id`
      but no baseline entry is judged on all three of remote-updated-at,
      remote hash and local hash rather than on presence alone, so absent
      hashes count as changed and the item surfaces as `conflict` rather
      than being masked as `synced`.
- [ ] Given the same pipeline, the per-item write sequence honours the
      "side effect first, baseline write last" resumability contract —
      verified by a test harness that captures write-call order via a fake
      store and asserts the baseline write occurs strictly after the
      corresponding side effect for every classified state, and by
      simulating a crash between the two (porting the bash fault seam
      `ACCELERATOR_TEST_MODE=1` + `WORK_SYNC_FAIL_AFTER=side-effect`) and
      asserting a re-run produces the same terminal state as an
      uninterrupted run.
- [ ] Given `accelerator work sync --push-only` or `--pull-only`, then the
      three-input decision table (mode × classified-state × local-dirty)
      forbids writes in the disallowed direction, matching every
      forbidden-write cell in `work-item-sync-decide.sh`: conflict and
      remote-ahead never pull under `--push-only`; local-ahead never
      pushes under `--pull-only`; `indeterminate` and `remote-absent`
      never write either side.
- [ ] Given a `remotely-modified` item whose local file is dirty, then a
      bidirectional run routes to `prompt` and a `--pull-only` run routes
      to `skip-dirty` — in neither case is the dirty local file
      overwritten.
- [ ] Given a bidirectional run over a `conflict` item and no resolution
      orders, then the item is reported as awaiting resolution and neither
      side is written; the command exits without prompting and without
      reading stdin — verified by running it with stdin closed and
      asserting it neither blocks nor fails. The report emits one
      machine-parseable line per unresolved item carrying the work item id
      and its classified state, and the exit code is distinct from both
      clean success and error, so a caller can distinguish "nothing to
      resolve" from "resolution needed" without parsing prose.
- [ ] Given a `sync` run over a two-conflict corpus, then a test harness
      parses the report, constructs one `--resolve` order per reported id,
      and a second invocation applies exactly those orders — closing the
      two-invocation loop without a SKILL and without stdin.
- [ ] Given a re-invocation carrying `--resolve <id>=<token>`, then
      `remote` accepts the remote, `local` pushes the local, and an empty
      **or unrecognised** token skips without writing either side — token
      matching is case-insensitive and whitespace-trimmed.
- [ ] Given `accelerator work sync --preview` over a set spanning every
      classified state, then no mutation occurs on any of three
      observables: the baseline document is byte-identical before and
      after (so neither a per-item `set` nor `finalise` ran), every local
      work item file's content and mtime are unchanged, and the fake
      `RemoteTracker` records zero `create` and zero `update` calls.
- [ ] Given the same preview, then a real run immediately following it
      over the same fixture set applies exactly the action set the preview
      reported, item for item — so plan fidelity is asserted against
      behaviour rather than claimed.
- [ ] Given a run that fails partway, then the baseline's global timestamp
      is unchanged from its pre-run value; given a clean run, the
      persisted timestamp is the epoch captured at run **start** (verified
      against an injected clock) and is strictly earlier than the mtime of
      any file the run mutated — so the next run's mtime pre-filter cannot
      skip an item this run wrote.
- [ ] Given a baseline file that is missing, and separately one that is
      present but unparseable (e.g. carrying VCS conflict markers), then
      both are treated as a valid empty baseline: reads yield nothing and
      succeed, and the run degrades to a full re-hash rather than failing.
- [ ] Given a fresh work item directory, when `accelerator work create
      --push` runs, then it allocates the next ID per the configured
      pattern and, when the remote create call via the wired (or, in unit
      tests, faked) `RemoteTracker` port succeeds, writes exactly one
      local file with `external_id` already substituted. When the remote
      call fails, the file is written without `external_id` (saved
      unsynced) and the command reports which of
      `work-item-create-remote.sh`'s outcome rows applied, exiting without
      prompting and without reading stdin — the local-fallback judgment is
      the SKILL's, not the binary's.
- [ ] Given a `create --push` whose remote call failed terminally after
      the remote issue was in fact created, when the command is re-run for
      the same work item, then it issues no second `create` call against
      the fake tracker.
- [ ] Given a work item file, when `accelerator work update --push`
      targets a synced item, then the remote issue is replaced via the
      same whole-content contract as `work-item-update-remote.sh`. When
      the remote replace call fails, the local file is left untouched and
      the command exits non-zero in both cases, surfacing that script's
      retryable-vs-terminal distinction with a different consequence for
      each: on `E_DISPATCH_RETRYABLE` (provably no mutation, safe to
      retry) the baseline entry is left intact, and on
      `E_DISPATCH_TERMINAL` (mutation state uncertain, never
      auto-retried) the item's baseline entry is cleared, so a subsequent
      `sync` classifies it `indeterminate` and writes neither side.
- [ ] A characterization test captures `work-item-push-decide.sh`'s
      pre-port behaviour — every documented `--code` × `--attempt`
      combination plus `--write-failed`, and at least one error path
      (non-integer arguments) — and passes against the unmodified script.
- [ ] The fixture tables behind `test-work-item-sync-apply.sh`,
      `test-work-item-fetch-remote.sh`, `test-work-item-create-remote.sh`,
      `test-work-item-update-remote.sh` and the classify, decide, baseline,
      label and project sections of `test-work-item-scripts.sh` are lifted
      into committed Rust test data, and both implementations pass against
      them: the Rust suite against the lifted tables, and the bash suites
      unchanged against the scripts, which remain in place. The Rust suite
      makes no live network call.
- [ ] The bash-generated baseline corpus is committed under
      `test-fixtures/` while `work-item-project-remote.sh` still exists, so
      the classification-stability check below survives 0171's removal of
      its generator.
- [ ] Given a sync over N items in bulk mode, then the fake tracker
      records exactly one `fetch_all` call and at most one `show` call
      per item whose stamp differs from its baseline — zero when nothing
      changed; given per-item mode, exactly N `show` calls and no
      `fetch_all`. Either way the classifier itself records no calls, so
      it is shown never to fetch on its own behalf.
- [ ] `work-item-sync-label.sh` and its Rust port are both asserted
      against a new `test-fixtures/work-item-sync-label.golden` covering
      each of the seven classified states, and `work-item-normalise.sh`
      and `work/src/normalise.rs` against the existing
      `work-item-normalise.golden`, so neither deliberate duplication can
      drift undetected.
- [ ] Remote calls are exercised only by a separate, explicitly-tagged
      contract/integration suite, gated behind a cargo-nextest filter
      excluded from the default `cargo test`/`cargo nextest run`
      invocation. This story delivers the harness — a shared
      `RemoteTracker` contract test parameterised over implementations,
      asserting round-trip `create` → `show`, whole-content `update` →
      `show`, and two properties the port documents but cannot enforce:
      that `fetch_all`'s partition is total over the requested ids, with
      an unaccounted id landing in `indeterminate` rather than `absent`;
      and that a failing read yields `Retryable`, never `Terminal`. Both
      are obligations on every client that no test in `tracker` can hold
      — 0204 hands them here explicitly, and
      `tracker/tests/port.rs::partitions_totally` is the shape to lift.
      This story passes the harness against the fake; running it against
      each real client is 0171's criterion, not this one's.
- [ ] Projection lives behind the port: the fake and any adapter this
      story wires return domain `RemoteIssue` values, never raw tracker
      JSON, so no provider-shaped data reaches the state machine.
- [ ] The sync state machine is sited per the crate boundary: classify
      and decide live in `work`; apply and baseline live in
      `work-adapters`. No `store` dependency is added to any domain
      crate. The `work` → `tracker` edge is resolved deliberately and
      recorded — either `work`'s import rule is widened with
      `^tracker(::|$)` and probed like its siblings, or `work-adapters`
      translates at the boundary and `work` stays unwidened.
- [ ] Given `work.integration` set to `jira` and separately to `linear`,
      the composition root resolves the corresponding client; given the
      key unset or naming an unknown provider, the command fails with a
      named error rather than defaulting silently. Verified against fakes
      registered under both names — the seam, not the real clients.
- [ ] Given a fixture corpus whose `remote_hash` baselines were written by
      the bash projection path, when the ported sync runs against matching
      remote records, then every such item classifies as `synced` and
      neither a push nor a pull is issued — so moving projection behind
      the port cannot mass-reclassify a real user's synced items.
- [ ] No bash script is removed and no SKILL is repointed: every script
      named in Requirements is still present and still invoked by its
      existing callers at the end of this story, and
      `_EXPECTED_WORK_SUITES` is unchanged, and the existing bash suites
      for every one of those scripts still pass — so the sync and `--push`
      flows a user exercises are demonstrably the ones they had before. No
      skill, script or user-facing entry point invokes the binary.

## Open Questions

None outstanding. Both questions carried into this item were resolved on
2026-08-10 — the command's interaction model and the shared-script
duplication — and their answers are recorded in Requirements, Acceptance
Criteria and Drafting Notes.

## Dependencies

- Blocked by: 0204 (the `RemoteTracker` port), split out of this item on
  2026-08-10. It is a trait, four value types (`ExternalId`,
  `RemoteIssue`, `RemoteTimestamp`), the `TrackerError` type and a lint
  rule, with no logic — so it is cheap to discharge, but the state
  machine and both `--push` flows compile against it, so it lands first.
- No other blockers. 0166 (shared crates) and 0187 (generalises the
  sub-binary registration surface) were done as of 2026-08-05, and 0170
  (the work-item lifecycle subdomain), which gated the `--push` wiring,
  was validated done on 2026-08-07.
- Phase ordering within this item: A (state machine) is startable once
  0204 lands; B (the `sync` command) depends on A, since it orchestrates
  the pipeline A builds; C (`--push` wiring) depends on A too — its
  terminal-failure path writes through A's baseline code — and on 0170's
  `create`/`update` commands, which now exist. There is no removal phase
  — see the cutover handover below.
- Hands the cutover to 0171. This story leaves the bash path live and
  ships the binary beside it, because `sync` and `--push` cannot resolve a
  real tracker until 0171's clients exist and 0171 is blocked by this
  story. Retiring the scripts here would leave the user-facing flows dead
  in the interval. 0171 therefore owns: removing the nine migrated
  scripts and their suites, repointing `sync-work-items`,
  `create-work-item`, `list-work-items` and `EXIT_CODES.md` at
  `accelerator work …`, adding the sync SKILL's conversational conflict
  flow, and decrementing `_EXPECTED_WORK_SUITES`.
- Does **not** block 0171 any more. Its client adapters wait on 0204's
  port signature, which is now its own item — so 0171 no longer waits on
  this story's state machine, command surface or test migration, none of
  which it needs. The two can run in parallel once 0204 lands.
- Reverse coupling on 0171: this story builds and verifies the *seams*
  that need a real client, never the clients themselves. The
  composition-root provider selection is exercised against fakes
  registered under both provider names, and the tagged contract suite runs
  against the fake. Three obligations pass to 0171 and need matching
  acceptance criteria there: running that contract suite against each real
  client, reproducing the jira and linear projection recipes exactly
  (gated by the corpus this story commits — see Assumptions), and the
  cutover above. Nothing in this item's own acceptance gate waits on 0171.
- External systems: Jira and Linear. Only the tagged contract/integration
  suite touches them, and only from 0171 onward; it needs API credentials
  and a sandbox project or workspace per provider, and `fetch_all()`
  exposes it to per-tenant rate limits on large corpora. The default
  invocation makes no network call, so nothing in this story's own
  verification depends on that provisioning.
- Relates to 0174 (Retire Shell Tooling and CI Guards): the deliberate
  bash/Rust duplication this story leaves behind
  (`work-item-sync-label.sh`, `work-item-normalise.sh`) retires there,
  once `/list-work-items` and the two integration scripts are ported.
- **Inherited from 0204's frozen port** (plan review, verdict APPROVE
  2026-08-11). Four obligations the port documents but cannot enforce,
  and which no test in `tracker` can hold:
  - `FetchOutcome`'s totality. The type has three public `Vec` fields and
    no validating constructor — 0204's no-logic criterion forbids one —
    so an unsound partition is representable. The contract test above is
    the only mechanism that will ever catch it, and 0171's clients are
    written before it exists.
  - The empty-`RemoteTimestamp` trap. An empty stamp means *unknown*, and
    derived `PartialEq` reports two unknowns as equal. Comparing them and
    concluding "unchanged" marks an item whose baseline was never written
    as already synced — `work-item-sync-classify.sh:177` guards this with
    `[ -n "$base_remote_updated" ] &&` before its equality short-circuit,
    and the Rust classifier must reproduce that guard.
  - The normalisation recipe. `RemoteIssue.body` is the *un-normalised*
    projection; this story owns normalising before hashing, and must
    reproduce `work-item-normalise.sh` exactly — per-line trim, trailing
    blanks stripped, interior blank lines surviving into the hash. If the
    two sides disagree by a byte, every synced item reclassifies.
  - A `show` per pushed item. `create` returns only an `ExternalId` and
    `update` returns `()`, so the post-push baseline write needs a read
    back. This matches the bash path and is accepted, not overlooked.
- Parent: epic 0136.

## Assumptions

- `reqwest` in the work binary (via the client adapters 0171 provides) is
  acceptable — it is already workspace-wide via the launcher, resolved by
  the work↔integrations coupling decision (research doc
  `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`,
  Open Question 2).
- The active provider client is faked at the `RemoteTracker` port boundary
  for this story's own unit tests; real Jira/Linear wiring is exercised
  once 0171 lands, via the separate contract/integration suite.
- 0170's `create`/`update` CLI signatures (flags and arguments) are stable
  once implemented, so this story's `--push` wiring doesn't need to
  renegotiate them; if 0170's signatures change after landing, that wiring
  may need rework.
- Moving projection behind the port does not change any persisted
  `remote_hash`: the adapters are expected to reproduce the existing
  recipe exactly (jira — summary line then the description in Atlassian
  Document Format through key-sorted `jq -S`; linear — title line then
  Markdown description verbatim). If a projected body differs by even
  whitespace, every synced item reclassifies as `remotely-modified` on
  the first run after the migration. The golden fixtures gate the
  projection function; the bash-written-baseline criterion above gates
  the property that actually matters.

## Technical Notes

- **Crate boundary.** `work` and `work-adapters` already exist and are
  substantially built (0170's crates: `create`, `update`, `show`,
  `resolve`, `next_number`, `section_diff`, `tags`, `normalise`,
  `own_identity`, `template_hints`), so this story extends a live pair
  rather than starting one. The `tracker` crate and the reasoning for its
  narrowness now live in 0204.
- **Phasing.** Three slices, all downstream of 0204. Two slices this
  story used to carry have left it: the port is now 0204, and script
  removal plus SKILL repointing are 0171's cutover:
  - **A** — the state machine: classifier and decision table into `work`;
    baseline and apply into `work-adapters` (atomic single-JSON writes,
    degrade-to-empty, side-effect-first resumability, the fault seam).
    The `work-item-push-decide.sh` characterization test lands here too,
    in A rather than C, so it is written against the untouched script
    ahead of the `--push` wiring it protects.
  - **B** — `accelerator work sync`, `--preview`, the report-and-resolve
    conflict flow (`--resolve`) and its machine-parseable report
    contract, the composition root's provider selection per
    `work.integration`, and the `work-item-sync-label.sh` port with its
    new `work-item-sync-label.golden` fixture — the pipeline's fifth
    stage has to work here for the command to run end to end.
  - **C** — `--push` wiring onto 0170's `create`/`update`.
- Source bash: `work-item-sync-{apply,baseline,classify,decide}.sh`,
  `work-item-fetch-remote.sh` (the read counterpart to
  `work-item-create-remote.sh`; `work-item-sync-apply.sh` calls it
  directly), `work-item-project-remote.sh` (the per-tracker projection
  seam — **already migrated** to `work-adapters/src/project_remote.rs`,
  which no caller uses yet because sync still shells out to the script;
  this story needs the wiring only — the translation is done and the
  removal is 0171's), and —
  moved from 0170 alongside the `--push` wiring they implement —
  `work-item-create-remote.sh`, `work-item-update-remote.sh` and
  `work-item-push-decide.sh`.
- **Shared scripts that are not removable here.**
  `work-item-sync-label.sh` has three live consumers outside sync —
  `skills/work/list-work-items/SKILL.md`,
  `skills/integrations/linear/scripts/linear-create-flow.sh`, and
  `skills/integrations/jira/scripts/jira-resolve-fields.sh` — so removing
  it would require porting `/list-work-items` and two integration
  scripts, well outside this story. `work-item-normalise.sh` is likewise
  reached from the sync SKILL and from `work-item-project-remote.sh`.
  Both stay, with no shim — the decision and its rationale are in
  Drafting Notes (2026-08-10); each keeps its own bash implementation,
  held to a golden fixture shared with the Rust side. Note that after the
  cutover moved to 0171, *every* script in this story's scope survives it;
  these two differ in surviving 0171's cutover as well, because their
  consumers sit outside the sync engine.
- **Parity fixtures already on disk**: `test-fixtures/`
  `work-item-normalise.golden` and `work-item-project-remote.golden`,
  plus the classify/decide/baseline/label/project sections inside
  `test-work-item-scripts.sh`. Every one of them gates a duplication that
  survives this story, since nothing is removed here; the label fixture is
  the only one that does not yet exist.
- The exit-code taxonomy (`E_DISPATCH_*`) is owned by one sourced
  definition shared by every bridge and both decision scripts —
  `work-item-bridge-codes.sh`. The Rust side needs the same single
  ownership, and until 0171 ports the Jira and Linear bridges the
  taxonomy has two implementations: `work-item-bridge-codes.sh` stays
  authoritative in the interim, and the Rust definition is asserted
  against it by fixture so the retryable-vs-terminal semantics the
  `--push` criteria depend on cannot drift.
- The sync flow depends on config + baseline store + the integration
  clients (0171); this story only requires the port to exist, not a
  concrete provider implementation.
- The `--push` wiring onto `create`/`update` extends 0170's
  already-implemented command definitions in `accelerator-work` with a
  `--push` flag that calls through this crate's port — it's a follow-on
  change to 0170's code, not a rewrite of it.
- Both this story and 0170 are now fully independent for their own
  respective scopes (the tracker crate/sync command here, the CRUD
  commands there), so either could scaffold/extend the `accelerator-work`
  binary and its composition root first. Registration follows the same
  checklist 0187 adds at
  `tasks/README.md#registering-a-dispatched-sub-binary` that 0170's
  Technical Notes also references — whichever story lands first does the
  actual registration; the other just finds it already done.

## Drafting Notes

- Split from 0170 on 2026-08-05 following work item review 1
  (`meta/reviews/work/0170-work-item-subdomain-and-sync-engine-review-1.md`),
  which found the lifecycle CRUD and sync engine independently deliverable.
  This item carries the `tracker` crate and `sync` command; 0170 keeps the
  CRUD lifecycle ops. Because 0170's `--push` flows and 0171's clients both
  build against this crate's port, this item precedes both rather than
  depending on either — the reverse of the sequencing first assumed during
  the split discussion. **Superseded** by the 2026-08-05 revision two notes
  below, which absorbed `--push` and reversed the direction again.
- Revised following work item review 1
  (`meta/reviews/work/0194-tracker-crate-and-remote-sync-engine-review-1.md`):
  added a concrete verification mechanism for the resumability criterion,
  clarified that Dependencies' blocking milestone is the port signature
  compiling rather than this item's full Acceptance Criteria gate, added a
  Technical Notes cross-reference to the 0187 registration checklist, added
  Requirements bullets mirroring the test-partitioning and
  removal/floor-decrement criteria, and glossed several previously-undefined
  terms (`work.integration`, the decision table, the Q2 reference).
- Revised 2026-08-05, following a review discussion: absorbed `--push`
  support for 0170's `create`/`update` commands (and the
  `work-item-create-remote.sh`/`work-item-update-remote.sh`/
  `work-item-push-decide.sh` scripts that implement it) from 0170, to
  remove 0170's dependency on this crate entirely. This story now depends
  on 0170 instead, but only for the new `--push`-wiring ACs; the `tracker`
  crate and `sync` command remain independent of 0170, as before.
- Revised 2026-08-10 after reading the source scripts directly rather than
  relying on the split-time summary. Four claims were wrong and are
  corrected above: (1) the classification is **seven** keywords, not five,
  and three of the five names previously used (`local-ahead`,
  `remote-ahead`, `in-sync`) were never the vocabulary — the real ones are
  `locally-modified`, `remotely-modified`, `synced`; (2) the baseline is a
  single atomically-written JSON document, **not** JSONL; (3) the decision
  table takes **three** inputs (mode × state × dirty), not two; (4) the
  earlier "four untested scripts" claim (and the five-script list that
  succeeded it) was wrong — `test-work-item-scripts.sh` already covers
  `sync-baseline`, `sync-classify`, `sync-decide`, `sync-label` and
  `project-remote` in dedicated sections, leaving **`push-decide` as the
  only genuinely uncovered script**. The characterization scope shrinks
  accordingly.
- The same pass pulled in behaviour that was designed in bash but
  unrecorded here: `--preview` mode and its no-mutation invariant,
  interactive conflict resolution (`prompt` / `skip-conflict` /
  `skip-dirty` and the token resolver's safe default), the baseline's
  degrade-to-empty rule, the `finalise` global-timestamp step, and
  `work-item-project-remote.sh` as the anti-drift projection seam.
- Decided 2026-08-10: projection moves **behind** the `RemoteTracker`
  port rather than staying a separate domain-side transformation as bash
  has it. Rationale in Requirements; the risk it carries is recorded in
  Assumptions.
- Decided 2026-08-10, re-taken 2026-08-10 after work item review 2: kept
  as one story with explicit phases. The original reason for
  revisiting — the `--push` slice stalling on 0170 — was discharged when 0170
  completed on 2026-08-07, so the decision was re-taken on size grounds
  instead. The item is large (a new crate, a state machine, a command,
  `--push` wiring, a SKILL flow, and a nine-script retirement), and review
  2 flagged that it matches the shape the epic has already split twice
  (0169 into four, 0173 into three). Two later splits — the cutover to
  0171 and the port to 0204 — have since removed both detachable slices,
  leaving three phases that build one thing: the state machine, the
  command that drives it, and the `--push` wiring whose terminal-failure
  path writes through the state machine's baseline code. Revisit if any
  single phase grows past its own plan.
- Split again 2026-08-10, following work item review 2 pass 3: the
  `tracker` crate left this item to become 0204. The clarity, scope and
  dependency lenses each arrived at it independently, and the argument was
  structural rather than about size. This item's own Dependencies section
  had recorded 0171's blocking milestone as "the port signature (end of
  Phase A), not this item's full acceptance gate" — a dependency no graph
  can express, leaving 0171 to either wait for work it does not need or
  build against an unaccepted branch whose signature could still move. The
  port is a trait, four value types, an error type and a lint rule, so
  it makes a cheap item and a milestone that is easy to hold stable once
  reached. Three defects the review found in the port survive as 0204's
  requirements rather than being carried here: `fetch_all()` had no
  signature at all, the error type never said whether it distinguishes
  retryable from terminal failure, and `RemoteIssue.updated` was untyped
  and never connected to the baseline's `remote_updated_at`. 0204's own
  review has since settled the port's shape — synchronous and
  dyn-compatible, a crate-local two-class `TrackerError`, and no lookup
  operation — so this item consumes a signature that is final rather than
  one it must finish designing. This item blocks 0171's cutover half only;
  0171's client adapters and 0194 are siblings over 0204 and run in
  parallel.
- Decided 2026-08-10, after inspecting the workspace rather than taking
  the split-time framing at face value: `tracker` is kept, but narrowed to
  the port and its vocabulary, and the sync state machine moves into the
  existing `work` / `work-adapters` pair. The original framing put the
  whole state machine in `tracker`, which fell down on three counts —
  classification reasons about work-item state and belongs with it;
  baseline persistence needs `store`, which no domain crate may import,
  so it would have forced a fourth `tracker-adapters` crate; and the
  workspace's settled pattern is a domain crate plus one `-adapters`
  sibling, with no precedent for a third crate per subdomain. The narrow
  port survives all three because it holds no logic. Alternatives weighed
  and rejected: putting the port in `work` (simplest, but makes every
  0171 client depend on the whole lifecycle domain — the one thing the
  crate exists to prevent), and `tracker` + `tracker-adapters` as a full
  pair (consistent with precedent, but two new crates and a second home
  for adapter code that `work-adapters` already hosts).
- Recorded 2026-08-10: `work` and `work-adapters` are already
  substantially built, and `work-adapters/src/project_remote.rs` is
  already a working port of `work-item-project-remote.sh` — written with
  the same adapter-siting rationale this story adopted independently. The
  earlier assumption that all of this was unported bash was wrong;
  Requirements and Technical Notes now distinguish porting work from
  wiring work.
- Decided 2026-08-10, resolving the interaction-model question, and
  extended after review 2: the CLI takes orders rather than asking
  questions, and this holds for **every** command that touches the remote,
  not just `sync`. Conflict resolution needs a judgment call the model is
  better placed to make than the binary, so the SKILL owns the prompt and
  `accelerator work sync` stays strictly non-interactive — it reports
  conflicts and skips them, then applies explicit `--resolve` orders on a
  second invocation. `create`/`update --push` inherit the same rule: bash
  resolves the local-fallback question by prompting, and the Rust side
  reports and exits instead, leaving the decision to the SKILL. This
  settles the
  decide-boundary question underneath it: `mode` and `decide` lose their
  external callers once the binary orchestrates the pipeline and collapse
  into internals, while the token resolver stays a real seam at the CLI
  boundary, because the SKILL now feeds it across a process edge. The
  original framing of this question (function visibility versus
  testability) was the wrong altitude — the answer follows from who owns
  the prompt.
- Decided 2026-08-10, resolving the shared-script question: no shim.
  `work-item-sync-label.sh` and `work-item-normalise.sh` keep independent
  bash implementations, and both they and the Rust port are held to one
  shared set of golden fixtures. A shim would have removed the
  duplication outright, but at the cost of a shell→binary hop on the
  `/list-work-items` path — per-item, and ~30ms per warm spawn after
  0186, so a large listing pays seconds. The precedent settles it:
  `work/src/normalise.rs` and `work-item-normalise.sh` already coexist
  this way, gated by `test-fixtures/work-item-normalise.golden`, and that
  arrangement is working.
- Scoped out 2026-08-10: removal of `work-item-sync-label.sh` and
  `work-item-normalise.sh`, both of which have consumers outside the sync
  engine. Recorded as an explicit non-goal rather than silently widening
  this story into `/list-work-items` and the integration scripts; the
  residual duplication retires under 0174.
- Decided 2026-08-10, resolving review 2 pass 2's critical finding: the
  **cutover moves to 0171**. Making the phase ordering explicit exposed
  that the old Phase E retired the bash bridges while the real Jira and
  Linear clients were still 0171's deliverable — and 0171 is blocked by
  this story, so the user-facing `sync` and `--push` flows would have been
  dead in the interval, against epic 0136's stay-functional-at-every-step
  rule. This story now ships the binary complete and tested beside the
  live bash path, changing no user-visible behaviour; 0171 flips the
  switch once its clients exist. Consequences recorded above: the phase
  list drops to A–D, the parity strategy becomes two implementations held
  to one lifted fixture table rather than a transient pre-removal oracle,
  the bash-generated baseline corpus must be committed here while its
  generator still exists, the sync SKILL's conversational half follows the
  cutover into 0171 (a test harness drives the two-invocation loop here
  instead), and the contract suite's real-client half becomes 0171's
  criterion. The alternative — an interim adapter shelling out to the
  existing bridge scripts — was rejected as throwaway work that would
  itself need retiring, for a window 0171 closes anyway.
- Revised following work item review 2
  (`meta/reviews/work/0194-tracker-crate-and-remote-sync-engine-review-2.md`),
  which returned REVISE on thirteen major findings, nearly all of them
  loose ends left by the 2026-08-10 correction pass. Six substantive
  decisions were taken and the rest were mechanical reconciliation:
  - **Parity strategy.** The bash suites were both the acceptance oracle
    and the removal target, and the item used "ported", "repointed" and
    "removed" for the same files. Settled: the bash suites are a
    transient pre-removal oracle, the fixture tables lift into Rust test
    data, and no bash harness is repointed at the binary.
  - **`create`/`update --push` interaction model.** Non-interactivity was
    scoped only to `sync`, while `decline` and `confirmed-local-fallback`
    implied a prompt. Settled: every remote-touching command is
    non-interactive and the SKILL owns the judgment.
  - **`update --push` failure outcomes.** The item asked the implementer
    to define the outcome its own criterion verified. Settled: the local
    file is never written, both cases exit non-zero, and
    `E_DISPATCH_TERMINAL` additionally clears the baseline entry so the
    next `sync` classifies the item `indeterminate`.
  - **Label anti-drift gate.** The no-shim decision cited shared golden
    fixtures, but none covered labelling. Settled: a new
    `test-fixtures/work-item-sync-label.golden` over the seven states,
    created alongside the label port.
  - **0171 coupling.** Composition-root wiring, the contract suite and
    projection fidelity all needed 0171's adapters while Dependencies
    claimed no blockers. Settled: this item builds and verifies the seams
    against fakes, and per-provider fidelity becomes 0171's requirement.
  - **Size.** Re-taken and kept whole — see the phasing note above.
  Mechanically: cleared the stale `blocked_by: 0170` (done 2026-08-07),
  stated the intra-item phase ordering, added the
  SKILL-caller repointing that script removal implies, specified the
  conflict report as a machine-parseable contract, added criteria for
  `finalise`, bulk-vs-`show` orchestration, provider selection and
  post-migration classification stability, broadened the `--preview`
  no-mutation check to three observables, split the no-provider-types
  criterion into its two checks, enumerated the bridge scripts behind the
  port's four operations, and glossed the terms review 2 found undefined.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Split from: `meta/work/0170-work-item-subdomain-and-sync-engine.md`
- ADRs: ADR-0045, ADR-0052, ADR-0053
