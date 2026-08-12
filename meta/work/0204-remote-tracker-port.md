---
type: work-item
id: "0204"
title: "RemoteTracker Port"
date: "2026-08-10T16:34:11+00:00"
author: Toby Clemson
producer: review-plan
status: done
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture", "codebase-research:2026-08-11-0204-remote-tracker-port"]
blocks: ["work-item:0171", "work-item:0194"]
tags: [rust, tracker, sync, port]
last_updated: "2026-08-12T00:30:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0204: RemoteTracker Port

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the `tracker` crate — the `RemoteTracker` trait, the value types
`ExternalId`, `RemoteIssue` and `RemoteTimestamp`, the `TrackerError`
type, and the cargo-pup rule that keeps the crate narrow — and nothing
else. It holds no logic. Its whole purpose is to be a stable,
cheap-to-reach milestone that unblocks two larger stories at once: 0171's
per-provider client adapters and 0194's sync engine both build against
this signature, and neither should wait on the other for it.

## Context

Accelerator syncs work items to a remote tracker — Jira or Linear — and
that sync is moving from the bash bridge scripts into the Rust CLI. The
port defined here is the seam between the two halves of that move: the
sync engine on one side, the per-provider HTTP clients on the other,
neither knowing the other's types.

Split out of 0194 on 2026-08-10 after its third review pass. As 0194 then
stood, it carried this crate as the first of four phases while also
carrying the sync state machine, the `accelerator work sync` command and
the `--push` wiring — and its own Dependencies section conceded that
0171's real blocking milestone was "only the port signature (end of Phase
A), not this item's full acceptance gate". A downstream story blocked on a
fragment of another story is a coupling the dependency graph cannot
express: in practice 0171 either waits for work it does not need, or
starts against an unaccepted branch whose signature can still churn
underneath it. (0194 has since been restructured, so those phase labels
describe its pre-split shape only.)

Three review lenses independently recommended the extraction. The crate is
the right size for it — a trait, three value types, an error type and a
lint rule, with no runtime behaviour to test — so the milestone is cheap
to reach and easy to hold stable once reached.

## Requirements

- Implement the `tracker` crate as the port and its vocabulary only, with
  no provider-specific or HTTP types anywhere in its public API. The
  public API is exactly these six items and nothing else:
  `RemoteTracker`, `ExternalId`, `RemoteIssue`, `RemoteTimestamp`,
  `FetchOutcome` and `TrackerError`.
- Define the port with these signatures verbatim. They are the contract
  0171 and 0194 build against, so they are stated here rather than left to
  the implementer — an unstated signature is how a frozen signature stops
  being frozen.

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  pub struct ExternalId(String);

  impl ExternalId {
      pub const fn new(value: String) -> Self;
      pub fn as_str(&self) -> &str;
  }

  impl std::fmt::Display for ExternalId { /* ... */ }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct RemoteTimestamp(String);

  impl RemoteTimestamp {
      pub const fn new(value: String) -> Self;
      pub fn as_str(&self) -> &str;
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct RemoteIssue {
      pub updated: RemoteTimestamp,
      pub body: String,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct FetchOutcome {
      pub found: Vec<(ExternalId, RemoteTimestamp)>,
      pub absent: Vec<ExternalId>,
      pub indeterminate: Vec<ExternalId>,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum TrackerError {
      Retryable { detail: String },
      Terminal { detail: String },
  }

  impl std::fmt::Display for TrackerError { /* ... */ }
  impl std::error::Error for TrackerError {}

  pub trait RemoteTracker {
      fn create(&self, title: &str, body: &str, kind: &str)
          -> Result<ExternalId, TrackerError>;

      fn update(&self, id: &ExternalId, title: &str, body: &str)
          -> Result<(), TrackerError>;

      fn show(&self, id: &ExternalId)
          -> Result<RemoteIssue, TrackerError>;

      fn fetch_all(&self, ids: &[ExternalId])
          -> Result<FetchOutcome, TrackerError>;
  }
  ```

  This block is the whole of the freeze: six public items, their fields,
  variants, derives, inherent methods, method signatures and trait impls.
  Three impls carry bodies and no others may: `Display` and `Error` on
  `TrackerError` — without them the type is not usable as an error — and
  `Display` on `ExternalId`, which keeps `as_str()` out of every format
  site in both consumers. They are named here so the no-logic rule below
  has an unambiguous edge.

  `const` on the two `new` constructors is expected rather than settled:
  nursery's `missing_const_for_fn` under `warnings = "deny"` should reject
  the plain form, but the lint has historically not fired on a parameter
  carrying a `Drop` impl, which `String` does. Confirm before
  implementation; if it does not fire, drop `const` from both rather than
  keeping a forward commitment nothing compels.

  `ExternalId` holds the same value the local work item carries in its
  `external_id` frontmatter field. The port takes it as opaque: it does
  not parse, validate or interpret the string.

  The four operations mirror the bash bridge scripts:
  `create` is `work-item-create-remote.sh`; `update` is
  `work-item-update-remote.sh` and is a whole-content replace returning
  nothing on success; `show` and `fetch_all` are the single-item and
  key-scoped bulk modes of `work-item-fetch-remote.sh`. That script's
  third mode — the unkeyed discovery `search` — has no port operation
  and is deliberately left above the port; 0171 owns its fate at
  cutover.
  `fetch_all` pairs each stamp with its `ExternalId` because the bulk mode
  has no other way to associate a record with a local work item. It is
  key-scoped and returns a partition rather than a flat list, because the
  bash bulk mode is both — see the `FetchOutcome` requirement below.
- Define `FetchOutcome` as a total three-way partition over the ids
  passed to `fetch_all`: every distinct requested id appears in exactly
  one of `found`, `absent` and `indeterminate`. Duplicates in the request
  are ignored, an empty request yields an empty outcome, and the three
  vectors are unordered — callers index rather than zip.

  `found` carries a `RemoteTimestamp`, not a `RemoteIssue`. Bulk
  retrieval establishes *whether* an issue changed; `show` fetches the
  body for the minority that did. No provider's bulk query returns a
  projected body — Linear's selection set has no `description` field at
  all — so a `RemoteIssue` here could only ever be filled with a
  fabricated one, and a fabricated body reclassifies every synced item.
  An issue the tracker returns without a timestamp still belongs in
  `found`, paired with an empty `RemoteTimestamp`; dropping it would make
  a live issue read as absent.

  `absent` carries the weight. An
  id belongs there only when the retrieval was provably complete; a
  truncated page, an exhausted rate limit or a partial failure puts its
  unseen ids in `indeterminate` instead, and an implementation that
  cannot tell the two apart must report every unseen id as
  indeterminate. A partial retrieval is therefore an `Ok` whose unproven
  ids are indeterminate, never an `Err`. This mirrors
  `work-item-fetch-remote.sh`, whose own contract states that absent is
  only ever drawn from a complete fetch, and whose consumer
  `work-item-sync-classify.sh` routes the two classes to different
  user-visible states. A flat `Vec` would force the caller to compute
  absence as requested minus returned, which is exactly the unsound
  inference the bash path exists to avoid — and the risk is not
  hypothetical: Linear's bulk path caps at 250 issues team-wide against
  roughly 180 synced items today.
- Keep the trait synchronous, taking `&self`, and dyn-compatible. 0194
  selects the active client at its composition root from the
  `work.integration` config key, which needs `Box<dyn RemoteTracker>`;
  and the `RestrictImports` rule below forbids the `async-trait`
  dependency that would otherwise be needed to keep an async trait
  object-safe. Fixing this after 0171 has begun would break the freeze,
  so it is pinned now.
- Define `TrackerError` as a closed two-class enum, deliberately *not*
  `#[non_exhaustive]`: adding a third class must be a compile-breaking
  change for consumers, which is the opposite of what `#[non_exhaustive]`
  provides. The two classes are retryable, meaning no remote mutation
  provably occurred and the call is safe to repeat; and terminal, meaning
  the remote mutation state is unknown or the failure is permanent. The
  rule is asymmetric and the asymmetry must reach the doc comment:
  retryable requires provable absence of a remote *change* — not of
  transmission — and everything unproven is terminal. The distinction is
  load-bearing. `work-item-bridge-codes.sh:9` scopes code 70 to "failure
  provably BEFORE any remote mutation", and the Jira retryable set
  includes 4xx rejects, auth failures and rate limits that plainly
  reached the tracker and were refused. A transmission-based reading
  would reclassify all of those as terminal, so the sync would refuse to
  retry calls that provably changed nothing.

  Classification is therefore operation-scoped rather than a property of
  the wire condition, and the per-operation mapping tables are
  authoritative where they are more conservative than the rule. Every
  bash bridge closes with a catch-all
  `*) return "$E_DISPATCH_TERMINAL"`. That conservative default is the
  part a client author will otherwise get wrong. The two classes
  correspond one-for-one to `E_DISPATCH_RETRYABLE` and
  `E_DISPATCH_TERMINAL` as defined in `work-item-bridge-codes.sh`, which
  remains authoritative until 0171 retires it. Both consumers need the
  distinction: 0194's `update --push` gives each class a different local
  outcome, and 0171's clients are what raise it. A port whose error
  cannot express it forces every consumer to reconstruct it from the
  provider detail the port exists to hide.
- Commit a parity fixture enumerating all four of
  `work-item-bridge-codes.sh`'s dispatch codes, and a test asserting that
  exactly two of them map 1:1 onto `TrackerError`'s classes. It must fail
  if either side gains, loses or renames a code. Four rather than two:
  the script defines `E_DISPATCH_NOT_AVAILABLE` (72) and
  `E_DISPATCH_UNRECOGNISED` (73) alongside the retryable and terminal
  pair, and a fixture holding only two cannot fail when the script gains
  a fifth — which is the property this criterion exists to give. The two
  extra codes are dispatch-routing outcomes that resolve above the port,
  at the composition root selecting the client from `work.integration`:
  if the config names an unbuilt tracker there is no `RemoteTracker` to
  call. The fixture records that reasoning. This is the artefact 0171's
  criterion to delete the script "and its parity fixture" refers to, so
  it is owned here rather than left to 0194.
- Define `RemoteTimestamp` as an opaque newtype over `String` holding the
  tracker's own last-modified stamp verbatim, compared by equality and
  never parsed or ordered — hence no `PartialOrd`/`Ord` derive and no
  conversion surface beyond construction and byte read-back. It is the
  value persisted as `remote_updated_at` in the sync baseline — the two
  names refer to one thing. Preserving the exact bytes is what keeps
  items whose baselines the bash sync path already wrote classifying as
  `synced` after the port lands; a lossy conversion at that boundary
  silently reclassifies them.
- Commit one real `remote_updated_at` string per provider under
  `tracker/tests/fixtures/` as this item's own round-trip input. Both
  formats, not one: the providers emit incompatible shapes — Linear
  `2026-06-21T00:06:10.647Z` and Jira `2026-07-09T08:00:00.000+0000`,
  whose numeric offset carries no colon — and a fixture holding only the
  Linear form leaves the shape a date-library round-trip would silently
  rewrite untested. Take the Linear value from the tracked bash-written
  baseline at `.accelerator/state/integrations/linear/last-sync.json` and
  the Jira value from the integration test fixtures under
  `skills/integrations/jira/scripts/test-fixtures/`. These are opaque
  strings, so capturing them needs no tracker credentials and no tenant —
  this item stays genuinely unblocked and does not wait on the baseline
  corpus 0194 commits.
- Specify `RemoteIssue.body` as the already-projected domain body, not
  raw tracker JSON — the output of the projection recipe
  `work-item-project-remote.sh` defines. Projection sits behind the port,
  so each of 0171's clients owns reproducing its provider's recipe
  exactly. A body differing by so much as whitespace reclassifies every
  synced item as `remotely-modified`, which is why the obligation belongs
  in the contract rather than only in the consumer.
- Treat `create`'s `kind` as an opaque caller-supplied string that the
  port does not interpret. Mapping it onto a Jira issue type or its
  Linear equivalent is each client's business (0171). This is what keeps
  the one parameter carrying work-domain meaning from dragging `work`
  across the boundary.
- Keep `tracker` free of any dependency on `work`. The crate exists to
  give 0171's clients a narrow edge; the moment it needs work-item
  identity types, 0171 pulls the whole lifecycle domain in transitively
  and the crate stops earning its place. The signatures above are
  expressible in `&str`, `String`, `Vec` and the crate's own types, so
  the invariant is achievable — and it must be enforced, not assumed.
- Add the cargo-pup whole-crate `RestrictImports` rule to `pup.ron`,
  matching the shape used for `config`, `corpus`, `vcs` and `work` but
  permitting only `std`/`core`/`alloc` and `crate`. Those four siblings
  also permit `kernel::Error`; `tracker` drops that line because it
  declares no dependencies at all, so `use kernel::Error;` could not
  compile and the allowance would misdescribe the crate. `TrackerError`
  is crate-local, which is what makes the empty dependency list possible.
- Prove the rule rather than assert it. Commit a probe pair in
  `tests/integration/pup/test_import_rule.py` that drives the shipped
  `cli/pup.ron` against a synthetic workspace whose crates are literally
  named `tracker` and `work` — a violation case importing a `work` type,
  and a compliant positive control — matching the pattern the `config`
  rule already uses. There is no coverage guard for `pup.ron`, so a rule
  that is deleted or mistyped is otherwise silent; an automated probe is
  also re-runnable, where a one-off manual demonstration is not.
- Ship no logic and no adapter. `tracker` is deliberately the workspace's
  first domain crate without a matching `-adapters` sibling: the sync
  state machine lives in `work`/`work-adapters` (0194) and the provider
  clients in their own crates (0171).
- Carry the verification artefacts as part of this crate rather than as a
  second workspace member: a fake `RemoteTracker` and a consumer
  exercising both error classes, plus a signature probe, all living in
  `tracker/tests/`. An integration test there links against `tracker` as
  an external consumer, so it sees only the public API and stops
  compiling if any signature widens — the same guard a probe crate would
  give, without a second manifest to register. These fixtures are private
  to this item's verification; the shared, reusable fake and the
  parameterised `RemoteTracker` contract test are 0194's deliverable.
- Give the port no lookup operation. `create --push` retry idempotency is
  resolved locally in `work` instead, so the four-operation surface is
  final at acceptance rather than provisional. The mechanism and its
  implementation belong to 0194 — see Dependencies.

## Acceptance Criteria

- [x] The `tracker` crate exists in the `cli/` workspace and compiles.
      `tracker/src/lib.rs` declares exactly six `pub` items —
      `RemoteTracker`, `ExternalId`, `RemoteIssue`, `RemoteTimestamp`,
      `FetchOutcome` and `TrackerError` — each carrying only the derives,
      fields, variants, inherent methods and trait impls given in the
      Requirements block, and nothing else. `cargo public-api` pins that
      surface against a committed snapshot at
      `cli/tracker/tests/fixtures/public-api.txt`, checked by
      `mise run public-api:check` and regenerated by
      `mise run public-api:update`. It reads rustdoc JSON, so the pin is
      immune to source formatting and catches a derive semantically, as
      the impls it generates. The tool reuses the pinned nightly the
      cargo-pup lane already provisions; it needs no new toolchain.
      The snapshot's contract half — declarations, fields, variants,
      signatures and `impl <Trait> for <Type>` lines — is hand-written
      from the Requirements block before `src/lib.rs` exists, so it
      starts red; the derive-generated method lines, whose names the
      expansion chooses, are captured once.
- [x] The four trait methods match the signatures in the Requirements
      block exactly, including `fetch_all`'s
      `(&self, ids: &[ExternalId]) -> Result<FetchOutcome, TrackerError>`
      — verified by an integration test in `tracker/tests/` that
      implements `RemoteTracker` and therefore stops compiling if any
      signature changes. `RemoteTracker` declares exactly four methods,
      so a fifth operation cannot be added additively without failing the
      public-API snapshot. None of the four carries a default body —
      confirm whether the snapshot distinguishes a provided from a
      required trait method, and if it does not, record that clause as
      unguarded rather than assuming it is covered. **Confirmed
      unguarded (2026-08-12)**: giving an existing method a default body
      while the fake still overrides it renders byte-identical
      `cargo public-api` output and leaves `port.rs` compiling — neither
      guard catches it. Not fixable within this crate's no-logic
      constraint; recorded for 0171/0194.
- [x] A test proves `fetch_all`'s partition is total and that an
      incomplete retrieval never reads as absence: given a fake that
      cannot account for one requested id, every requested id appears in
      exactly one of `found`, `absent` and `indeterminate`, and the
      unaccounted id lands in `indeterminate` with `absent` empty.
- [x] A test constructs `Box<dyn RemoteTracker>` from the fake and invokes
      all four operations through it, so the trait is object-safe and
      usable from 0194's composition root. Making the trait async or
      otherwise dyn-incompatible fails this test.
- [x] `TrackerError` declares exactly two variants and is not
      `#[non_exhaustive]`, demonstrated by an integration test in
      `tracker/tests/` whose match over it has no wildcard arm and routes
      each class to a distinct outcome. Adding a third class is therefore
      a compile-breaking change for every consumer.
- [x] A committed fixture under `tracker/tests/fixtures/` enumerates all
      four of `work-item-bridge-codes.sh`'s dispatch codes and records
      which of them resolve above the port. One test reads the script
      itself and fails if the two sides disagree; a second asserts that
      exactly two codes map 1:1 onto `TrackerError`'s classes. Adding,
      removing or renaming a code on either side fails the build.
- [x] `RemoteIssue.updated` is a `RemoteTimestamp`; a test round-trips
      every `remote_updated_at` value committed under
      `tracker/tests/fixtures/` through the field and back out
      byte-identically. The fixture holds one real stamp per provider —
      Linear's `Z`-suffixed form and Jira's numeric-offset
      `+0000` form — because a single-format fixture leaves the shape most
      at risk of a lossy conversion untested. The empty string is covered
      too: it is what the sync path stores when a post-push read fails, so
      `new` must not validate. And
      `RemoteTimestamp` derives no `PartialOrd`/`Ord` and exposes no
      parsing or conversion method beyond `new` and `as_str`, so two
      values differing only in whitespace compare unequal.
- [x] `RemoteIssue.body`'s doc comment states that the value is the
      already-projected domain body per the `work-item-project-remote.sh`
      recipe, and that reproducing it per provider is the implementing
      client's obligation — giving 0171's projection-fidelity criterion a
      referent in the contract rather than only in its own text.
- [x] `tracker` does not depend on `work`, enforced mechanically rather
      than by review: its `Cargo.toml` declares neither a
      `[dependencies]` nor a `[dev-dependencies]` table, and the cargo-pup
      `RestrictImports` rule in `pup.ron` permits only
      `std`/`core`/`alloc` and `crate`. Enforcement is demonstrated, not
      asserted, and by an automated probe pair rather than a one-off: a
      synthetic crate named `tracker` importing one named `work` fails the
      shipped rule by name, and a compliant control — carrying real
      `std::` and `crate::` imports, so a green run means "evaluated and
      allowed" rather than "nothing was imported" — passes. Deleting the
      rule from `pup.ron`, or corrupting either permit anchor, makes the
      pair fail. The pair runs under `mise run test:integration:pup`, not
      `mise run pup:check`: the latter is a different task that checks
      the real workspace positively and cannot demonstrate the rule's
      discriminating power. Both sit on the `check-architecture` CI job,
      which `cli:check` does not cover.
- [x] The crate carries no behavioural logic: `tracker/src/` contains no
      `#[cfg(test)]` module and no function body other than the four
      inherent methods, the two `Display` impls and the `Error` impl
      named in Requirements, and the workspace manifest lists no
      `tracker-adapters` member. The first and third are checked by
      `tracker/tests/structure.rs`, which also asserts the absent
      dependency tables of the criterion above — none of them is visible
      to rustdoc JSON, so the public-API snapshot cannot see them. The
      no-extra-function-body half stays a manual read.
- [x] `mise run cli:check`, `mise run pup:check`, `mise run deny:check`
      and `mise run public-api:check` all pass with the new crate
      registered, and the `tracker/tests/` fixtures are built and run by
      the workspace's `cargo nextest run` invocation rather than being
      excluded from it.

## Open Questions

None outstanding. The one question that could have reopened the frozen
surface — whether `create --push` retry idempotency needs a lookup
operation — is settled in Requirements: it is resolved locally in `work`
via a pending-push marker, and the port stays at four operations.

## Dependencies

- No blockers, and every artefact this item copies or registers against
  already exists. The `cli/` workspace and the `kernel` crate came from
  the foundation items (0163/0164); 0166 delivered the shared config,
  corpus and store crates, none of which this crate uses; the `pup.ron`
  rule shape this item copies was established by the `config`, `corpus`,
  `vcs` and `work` subdomain stories (0178/0179/0169/0170); and the
  registration checklist comes from 0187. All are complete. The crate
  holds no logic that could depend on anything else.
- Blocks: 0171 (Jira and Linear Integrations) — specifically its client
  adapter crates and thin binaries, which `impl RemoteTracker`. That half
  of 0171 waits on nothing but this item. 0171's cutover half — the
  script removal, skill repointing, conversational conflict flow and
  contract-suite run — remains blocked by 0194, as 0171's own
  `blocked_by` records.
- Blocks: 0194 (Tracker Crate and Remote Sync Engine) — its state machine
  and `sync` command call through this port. Two obligations pass to it
  with the unblock: implementing the pending-push marker that keeps
  `create --push` retries idempotent without a port lookup, and building
  the shared reusable fake and the parameterised `RemoteTracker` contract
  test. 0194 must record both, so the reason the port has no lookup
  operation is written where the alternative would be built.
- Reverse coupling on 0194: 0194 describes itself as the port's first
  consumer and design driver, so it is the item most likely to want the
  surface changed. The signature is frozen at this item's acceptance, and
  any later need is an additive change carried as a new item rather than
  a reopening of 0204 — otherwise 0171 is again building against a
  moving contract, which is what the split existed to prevent. 0194's
  Requirements record the same protocol, so both halves of the split
  respond to unmet surface needs the same way.
- 0194 owns the sync baseline storage contract (`last-sync.json` and its
  `remote_updated_at` values, written today by the live bash sync path).
  `RemoteTimestamp` must round-trip the values already on users' disks;
  0194's classification-stability criterion is what would otherwise catch
  a mismatch, and it runs after this signature is frozen. This item does
  not wait on 0194's baseline corpus: it commits a single captured
  `remote_updated_at` string of its own, which needs no tracker
  credentials and no tenant to obtain.
- `work-item-bridge-codes.sh` remains the authoritative
  retryable/terminal taxonomy in the interim. This item owns the parity
  fixture holding `TrackerError`'s two classes to it — carried as a
  requirement and a criterion here, not only as a note — so 0171's
  criterion to delete the script "and its parity fixture" has an artefact
  to refer to, and 0194 should not build a second one.
- External systems: Jira REST and Linear GraphQL. Both must be able to
  satisfy the frozen signature, and their constraints shaped it —
  `fetch_all` returns an owned `Vec` rather than a cursor because bulk
  retrieval and per-tenant rate limits are the client's concern to
  handle behind the port, not the caller's.
- Parent: epic 0136.

## Assumptions

- The four operations are sufficient for both consumers. The one case
  that looked like it might need a fifth — `create --push` retry
  idempotency — is decided against the port: 0194 resolves it locally
  with a pending-push marker. If some later need does emerge, it is
  additive and lands as a new item, not as a reopening of this one.
- Several bash-bridge capabilities are deliberately left above the port
  rather than ported: the unkeyed discovery `search` mode, the create
  bridge's `--dry-run` field-resolution preview, the update bridge's
  `--dry-run` payload validation (what `/sync-work-items --preview` uses
  today), the identifier-safety check that keeps a malformed remote key
  out of YAML frontmatter, per-call timeouts, and the pagination cap.
  Each is a client or caller obligation, not a port operation. 0171 owns
  deciding, at cutover, whether each is re-sited, dropped or carried as
  an additive item — see its Requirements.

## Technical Notes

- `tracker` is an addition to a live workspace, not a green field: `work`
  and `work-adapters` already exist and are substantially built from 0170.
- The alternatives were weighed when this crate was still part of 0194
  and are unchanged: putting the port in `work` is simplest but makes
  every 0171 client depend on the whole lifecycle domain — the one thing
  the crate exists to prevent; a full `tracker` + `tracker-adapters` pair
  matches the workspace's usual shape but adds two crates and a second
  home for adapter code `work-adapters` already hosts.
- Of the registration checklist 0187 added at
  `tasks/README.md#registering-a-dispatched-sub-binary`, only the steps
  that apply to a plain library crate are in scope: workspace membership,
  cargo-deny/cargo-pup coverage, and — new with this item — a public-API
  snapshot. This crate has no dispatch token, no binary and no launcher
  wiring, so the dispatch-specific steps do not apply. The library-crate
  steps are being written up as their own `## Registering a library
  crate` section, with `cli/tracker/` as the worked example.
- The fake is deliberately built twice — a private one here, a shared
  reusable one in 0194. Shipping the shared fake from `tracker` would
  either put test-support code in a crate whose emptiness is
  mechanically policed, or add the `test-support` feature and second
  consumer this item exists to avoid. The duplication is a four-method
  stub against a signature that cannot drift, so it costs less than the
  coupling would.
- One constraint the port cannot resolve passes to 0171 with the unblock:
  `cli/work-adapters/src/project_remote.rs` already implements the
  projection recipe in Rust, but it sits behind a `work` dependency and
  projects the `show` payload shape rather than the bulk one, so no
  client can reuse it as it stands. This item does not move it; 0171 owns
  the answer.

  A second constraint has been retired. Linear's bulk GraphQL selection
  set (`linear-search-flow.sh:157-165`) requests no `description` field,
  which would have made `RemoteIssue.body` unobtainable from its bulk
  query — but `FetchOutcome.found` now carries a `RemoteTimestamp`, which
  `updatedAt` already supplies, so no client needs to widen the query or
  meet Linear's complexity cap.
- One trap for a client author: Linear code 34 is retryable on `create`
  but terminal on `update` (`work-item-update-remote.sh:59-65`). The same
  wire condition maps to two classes depending on the operation, so a
  single status-to-class table is wrong. Nor are the two operations'
  provable sets nested in either direction — Linear codes 18, 23, 25, 27
  and 29 run the other way, retryable on `update` and terminal on
  `create` — so each must be derived from its own table. `TrackerError`'s
  two classes are operation-agnostic, but their *application* is not, and
  the doc comments say so.
- The house recipe puts one `Display` test per arm in an inline
  `#[cfg(test)] mod tests` (`cli/corpus/src/store.rs:81-148`), which the
  no-logic criterion forbids here. `Display` is public surface, so those
  tests move to `tracker/tests/` unchanged. The deviation is deliberate,
  not an oversight.

## Drafting Notes

- Split from 0194 on 2026-08-10 following work item review 2 pass 3
  (`meta/reviews/work/0194-tracker-crate-and-remote-sync-engine-review-2.md`),
  in which the clarity, scope and dependency lenses independently
  recommended it. The trigger was structural rather than about size alone:
  0194 recorded 0171's blocking milestone as the end of its own Phase A,
  which no dependency graph can represent, so the edge was either
  over-blocking or fiction someone had to manage informally.
- Three gaps the review found in 0194's version of this crate are fixed
  here rather than carried over: `fetch_all()` had no signature at all
  while the signature was what 0171 waited on; the port's error type never
  said whether it distinguishes retryable from terminal failure, which
  both consumers depend on; and `RemoteIssue.updated` was never typed nor
  connected to the baseline's `remote_updated_at`, the field it becomes.
- Review 1 of this item (2026-08-10) found the first draft had restated
  those three gaps as instructions to the implementer rather than closing
  them, so the verbatim trait definition and the four types were written
  into Requirements. Four decisions were settled at the same time, each
  because leaving it open would have unfrozen the signature after 0171
  began: the trait is synchronous and dyn-compatible (`async-trait` is a
  dependency the pup rule forbids, and native async fn in traits is not
  object-safe, but 0194's composition root needs
  `Box<dyn RemoteTracker>`); `TrackerError` is crate-local rather than a
  widening of the shared `kernel::Error`; `RemoteTimestamp` is opaque over
  `String` so existing bash-written baselines round-trip byte-identically;
  and retry idempotency is resolved in `work`, keeping the port at four
  operations.
- The signature probe is an integration test in `tracker/tests/` rather
  than the separate probe crate the first draft's criteria implied. An
  integration test links against the crate as an external consumer, so it
  sees only the public API and catches widening just as a probe crate
  would — without a second manifest to register, and without contradicting
  this item's own one-crate framing.
- Review 1 pass 2 (2026-08-10) corrected three defects the first revision
  introduced. `TrackerError` had been specified as both "closed" and
  `#[non_exhaustive]`, which are opposites — `#[non_exhaustive]` exists
  to let variants be added *without* breaking consumers, and it also
  makes the wildcard-free match the criterion demanded impossible for a
  `tracker/tests/` consumer; the attribute is dropped, since a
  compile-breaking third class is the property both consumers want.
  `ExternalId` and `RemoteTimestamp` were named as frozen public items
  but never defined, so the freeze did not cover the type carrying remote
  identity; all five items are now given verbatim, derives and inherent
  methods included. And enforcement was attributed to `cli:check`, which
  runs workspace rustfmt and clippy only — cargo-pup lives on the
  separate nightly `pup:check` lane, so the criteria now name it.
- Planning on 2026-08-11
  (`meta/plans/2026-08-11-0204-remote-tracker-port.md`, backed by
  `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`)
  reopened the frozen block across three passes, deliberately and before
  either consumer began. Eight changes in total, each cheap now and
  expensive after acceptance. `fetch_all` could not express "the fetch
  was incomplete", so it gains `FetchOutcome` and an `ids` parameter and
  the surface
  becomes six items — the only one of the four that could have made the
  port give a wrong answer, and live rather than theoretical because
  Linear truncates at 250 against roughly 180 synced items. AC 1 named
  `cargo public-api`, which is absent from this repository and would be
  its third Rust toolchain — the same criterion had been written into
  three work items without the tooling ever landing — so a self-reading
  surface golden replaces it. The pup rule's `kernel::Error` allowance is
  dropped as inert. And the parity fixture holds all four dispatch codes
  rather than two, because the script defines four and a two-code fixture
  cannot fail when a fifth arrives.

  Four more followed from plan review (five passes, verdict APPROVE on
  2026-08-11, `meta/reviews/plans/2026-08-11-0204-remote-tracker-port-review-1.md`).
  `FetchOutcome.found` carries a `RemoteTimestamp` rather than a
  `RemoteIssue`: no provider's bulk query returns a projected body, so
  the original shape could only have been satisfied by fabricating one,
  which would reclassify every synced item. `TrackerError` gains
  `Clone, PartialEq, Eq` and `ExternalId` gains `Hash` and `Display` —
  every sibling error type in the workspace derives the first set, no
  consumer could `assert_eq!` over a `Result` without it, and 0194 joins
  on `ExternalId` every sync run. Both `new` constructors become
  `const fn`, pending the lint check recorded in Requirements. And AC 1's
  self-reading golden becomes a `cargo public-api` snapshot: the
  hand-rolled parser failed three review passes on four distinct source
  shapes, because a contract pinned by line-shape heuristics over text
  cannot be verified by reading it. The tool reads rustdoc JSON and needs
  no new toolchain — the earlier objection that it would mean a third was
  wrong twice over, since the nightly already exists and cargo-public-api
  has no `rustc_private` driver to build against it.

  The freeze protocol itself is unchanged, and applies from this revision
  onward.
- Implemented 2026-08-12 against
  `meta/plans/2026-08-11-0204-remote-tracker-port.md`, all three phases,
  `mise run` green end to end. One acceptance criterion resolved to a
  documented gap rather than a pass: giving an existing trait method a
  default body is confirmed unguarded by both `port.rs` and
  `public-api:check` — recorded on that criterion above and handed to
  0171/0194 rather than worked around, since fixing it would need a
  constructor or test AC 10 forbids. 0194 and 0171 have been told their
  descriptions of the port were stale (five items, no `FetchOutcome`) —
  see their own Drafting Notes.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Implementation research: `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`
- Plan: `meta/plans/2026-08-11-0204-remote-tracker-port.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Split from: `meta/work/0194-tracker-crate-and-remote-sync-engine.md`
- ADRs: ADR-0045, ADR-0052, ADR-0053
