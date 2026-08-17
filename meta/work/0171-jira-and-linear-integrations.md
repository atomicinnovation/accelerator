---
type: work-item
id: "0171"
title: "Jira and Linear Integrations"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: epic
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
blocks: ["work-item:0174"]
relates_to: ["work-item:0170", "work-item:0194", "work-item:0204", "work-item:0174", "work-item:0165", "work-item:0203"]
tags: [rust, jira, linear, integrations, reqwest, sync]
last_updated: "2026-08-17T11:52:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-192"
---

# 0171: Jira and Linear Integrations

**Kind**: Epic
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the `jira-client` and `linear-client` adapter crates over the frozen
`RemoteTracker` port (`reqwest`/serde replacing `jq`/`curl`) and the thin
`accelerator-jira` / `accelerator-linear` binaries above them, so the standalone
integration skills and the work-item sync engine share one implementation per
provider.

Then retire both bash surfaces those clients replace — the jira and linear
script clusters under `skills/integrations/`, and the work-item cutover 0194
deliberately left undone — and give `/sync-work-items` the conversational
conflict flow that makes its conflict report actionable. The four children carry
the inventory and the acceptance criteria; see Scope and Decomposition.

## Context

`skills/integrations/jira/scripts/` (22 prod) and `linear/scripts/` (12 prod)
implement create/update/comment/transition/search/show/attach/init flows,
Atlassian Document Format (ADF)↔markdown conversion, Jira Query Language (JQL)
and GraphQL, and auth, shelling out to `jq`/`curl`. Both have Python mock HTTP
servers for tests. The intended shape is one shared client crate per provider,
consumed by both the standalone binary and the `tracker` sync engine — neither
crate exists yet.

Two things are degraded for plugin users until this lands. `/sync-work-items`
can detect a conflict but cannot resolve one, because 0194's binary is
non-interactive and nothing yet closes the report → prompt → resolve loop. And
every provider behaviour — auth, projection, error classification — exists twice
per provider, once in bash and once in Rust, so a fix to either drifts from the
other until the bash half is gone.

Every repository input this story consumes is on disk; its non-repository
prerequisites — the credentialed tracker target and the two provider services —
are listed in Dependencies. 0204 introduced the `tracker` crate and froze the
`RemoteTracker` port inside it; 0194 added the sync engine to the same crate and
left three artefacts behind for this story (confirm they exist rather than
trusting 0194's status field — see Dependencies): the contract harness at `cli/tracker-test-support/src/contract.rs`
(parameterised over implementations via `ContractSubject`, run by `mise run
test:integration:tracker-contract`), the bash-generated baseline corpus under
`skills/work/scripts/test-fixtures/`, and `accelerator work sync` with its
machine-parseable conflict report. The bash path stays the production path until
this story lands.

## Scope

This item is the unit of value — **one Rust implementation per provider, in
production** — and it is delivered entirely by its four children. Requirements and
acceptance criteria live in them.

**Precedence: the children are normative.** Each child's Requirements and
Acceptance Criteria are the contract its implementer builds to and its reviewer
accepts against. This item deliberately holds no duplicate copy, because it had
one and it drifted: two clauses were silently lost between the parent and a child
within a single revision (a per-flow fixture-provenance obligation, and the
qualifier that a recorded assertion count is context rather than a bar). One
normative home per obligation removes that failure mode.

What the whole delivers, as narrative rather than contract:

- `jira-client` and `linear-client` over the frozen `RemoteTracker` port, each
  reproducing its provider's projection recipe, error classification and request
  bounds, wired into `accelerator-work`'s composition root — **0210**.
- `accelerator-jira` and `accelerator-linear` exposing the eight flows per
  provider, registered as dispatched sub-binaries, replacing both bash script
  clusters — **0211**.
- The work-item cutover: eighteen scripts deleted, the fixture corpus relocated,
  eleven parity tests converted, the three work skills repointed, both suite
  floors removed, the dirty guard preserved, the three port-less bridge
  capabilities discharged — **0212**.
- The conversational conflict loop that makes `accelerator work sync`'s conflict
  report actionable — **0213**.

Every parent obligation was audited into exactly one child before this section
replaced the duplicated lists: 28 requirements and 31 criteria, all assigned. The
audit found exactly one gap — the three port-less bridge capabilities, which no
child had claimed — now owned by 0212.

## Decomposition

This item stays the single coherent unit of value — one Rust implementation per
provider, in production — and delegates delivery to four children, each with its
own status, requirements and acceptance criteria. The Requirements and Acceptance
Criteria above remain the whole picture; each child owns the subset named here
and restates it, so an increment can be planned, merged and accepted on its own.

| Child | Carries | Blocked by |
|---|---|---|
| 0210 | Client crates, composition root, oracle transcriptions, contract run | — |
| 0212 | Work-item cutover, fixtures, tests, floors, guard, port-less capabilities | 0210 |
| 0211 | Provider binaries, registration, integration cluster retirement | 0210, 0212 |
| 0213 | Conversational conflict flow | — |

- **0210 — Provider Client Crates over the RemoteTracker Port.** `jira-client`,
  `linear-client`, the composition-root wiring, `deny.toml` clearance, the four
  mapping tables, bounded calls, the identifier-safety check, whole-corpus offline
  projection fidelity, the ADF/JQL/GraphQL construction assertions (they pin this
  child's code, so they moved here from 0211), and the contract harness against
  both real clients. Deletes nothing, and transcribes at named paths the three
  oracles the later children destroy: the four `curl` tables, the ADF node-type
  inventory, and the baseline carrying the eleven tests' fixture cases, their
  assertion counts and the pre-change fixture file count.
- **0212 — Work-Item Script Cutover.** The eighteen `work-item-*.sh` files, the
  fixture relocation, the eleven test conversions, the three work-skill
  repoints, the work suite floor, the dirty guard, the `E_DISPATCH_*`
  consolidation, the `tracker` doc-comment sweep — and the **three port-less
  bridge capabilities**, since this is the child that deletes the scripts
  carrying them. It also owns the whole-repository `jq`/`curl` equality
  assertion, because the work skills are repointed here.
- **0211 — Integration Binaries and Bash Cluster Retirement.**
  `accelerator-jira` and `accelerator-linear` with eight flows each, their exit-
  code contract, per-flow request and **stdout** goldens, registration and the
  release manifest, then both script clusters, their suites, the mock servers,
  the integrations floor and the seven jira/linear `SHELL_LIBRARIES` entries.
- **0213 — Conversational Conflict Resolution Flow.** The conversational half in
  `sync-work-items/SKILL.md`. Gated by none of this item's Open Questions and by
  no credentialed target, so it can land first — and should, since
  `/sync-work-items` cannot resolve a conflict today.

**Ordering.** 0210 → 0212 → 0211, with 0213 free. Two constraints force it.

First, 0210's bash is the only oracle for projection and classification fidelity,
so it precedes every deletion.

Second, 0212 precedes 0211 because the coupling between the two deletion sets runs
in **one direction only**. Four of the five suites 0212 deletes —
`test-work-item-create-remote.sh`, `-update-remote.sh`, `-fetch-remote.sh` and
`-sync-apply.sh` — reach into
`skills/integrations/{jira,linear}/scripts/test-helpers/` and `.../test-fixtures/`
for the Python mock servers and their scenario fixtures. The clusters invoke no
`work-item-*.sh` in return: verified by grep, the two references to
`work-item-sync-label.sh` at `linear-create-flow.sh:304` and
`jira-resolve-fields.sh:140` are comments about shared normalisation, not calls.
Deleting the work suites first removes the only outside consumers of the clusters'
test assets, after which 0211 retires both clusters whole.

⚠️ This ordering was **reversed on 2026-08-17** after the earlier rationale proved
false. The decomposition originally put 0211 first on the belief that those two
comment references were live callers; in that order 0211 would have broken all
four suites and the `_EXPECTED_WORK_SUITES` floor at its own merge boundary.

Each child's boundary is its own atomicity unit: an "in the same change" clause in
a child's Requirements binds that child's change, not the set. Each child must
leave `mise run` green at its own merge point, per epic 0136's stay-functional
rule, and each carries a criterion saying so.

## Open Questions

Five things must be settled before pickup, each changing what is in scope: the
three questions below, plus the two ⚠️-marked assumptions that bound the size of
0211 and 0212 respectively. Owner: Toby Clemson.

- Where the credentialed Jira/Linear target's secrets live — CI secrets or
  local-only. This decides whether the real-client contract run is a CI gate or
  a manual pre-merge check, and therefore whether a broken client can reach
  `main`. The CI answer also adds a workflow change to the scope; the local-only
  answer does not.
- The fate of the three open port-less bridge capabilities: unkeyed discovery
  `search`, the create `--dry-run` field-resolution preview, the update
  `--dry-run` payload validation. Re-site above the port, drop, or file as an
  additive port item. Only the first two are options **inside** this story:
  0204 is done and frozen, and its own protocol says a later surface need lands
  as a **new** additive work item, so choosing that fate for any capability
  means filing that item (parented to 0136, blocking this one) and holding this
  story at `draft` until it exists. The identifier-safety check is **not**
  among these three; it is decided.
- Whether `skills/work/scripts/EXIT_CODES.md` is rewritten in place for the Rust
  exit codes or folded into the CLI's own docs. The latter is preferred and
  removes `skills/work/scripts/` entirely; the former keeps a directory holding
  one file.

## Decisions

One entry per decision, each carrying its state. Several acceptance criteria are
discharged in part by an entry here — always paired with an independent
assertion, never by the entry alone. Three states appear: *open* means blocked on
an Open Question and must be answered **before pickup**; *pending* means the
answer is produced by doing the work and is recorded as it lands; *decided* means
settled and closed.

- Unkeyed discovery `search` — *open*.
- Create `--dry-run` field-resolution preview — *open*.
- Update `--dry-run` payload validation — *open*.
- Contract-run execution route (CI job and secrets, or manual step and evidence
  location) — *open*. Durable beyond this story: site the answer in
  `tasks/README.md` and point here.
- `EXIT_CODES.md` siting: rewritten in place, or folded into the CLI docs with
  `skills/work/scripts/` removed — *open*.
- The two binaries' exit-code contract and its document of record — *open*.
  Durable: it belongs in the CLI's own docs, not here.
- Copyleft status of the `wiremock-rs` and `rustls` dependency trees, and
  whether 0203 therefore becomes a release-path dependency — *pending*.
- `linear-graphql.sh` classified as a production script or a library entry —
  *pending*.
- Consumer sweep result for the eighteen deleted work scripts (the grep command
  and its empty output) — *pending*.
- Reverse cross-cluster sweep result: references from `skills/work/scripts/` into
  either integration cluster or into the two Python mock servers, with each one's
  resolution — *pending* (0211).
- Flow-coverage mapping: each of the 22 Jira and 12 Linear production scripts to a
  named subcommand or a recorded internal-helper classification — *pending*
  (0211).
- Conflict-flow walkthrough evidence, one run per fixture including the clean
  exit-`0` case — *pending* (0213).
- Pre-deletion transcriptions: the four `curl` exit-code tables, the ADF node-type
  inventory, and the per-test assertion-count and fixture-case baseline for the
  eleven converted tests — *pending*.
- Fixtures deleted for having no consumer, with the reason each has none —
  *pending*.
- Per-flow fixture capture source (credentialed target or mock-served) —
  *pending*.
- Cross-skill `jq`/`curl` `allowed-tools` audit result — *pending*.
- Conflict-flow walkthrough evidence — *pending*.
- Identifier-safety check — **decided**: carried forward, an unsafe identifier
  is a `Terminal` failure.

## Dependencies

- `blocked_by` is deliberately empty because every declared upstream is `done`.
  Five items declare an edge into this one: 0166 (shared config, corpus and
  store crates), 0169 (VCS subdomain and hooks migration), 0187 (sub-binary
  registration surface) and 0204 (the `RemoteTracker` port) all carry `blocks:
  work-item:0171` and all read `done`; 0194 (the `tracker` crate and sync
  engine) blocks the cutover half in prose only. Registration follows the
  checklist at `tasks/README.md#registering-a-dispatched-sub-binary`; this
  story adds **two** dispatch tokens (`jira` and `linear`), it does not
  generalise the surface.
- ⚠️ **0194's status needs confirming at pickup, not assuming.** This item
  treats 0194 as complete, but the 0194 record visible from the `build-system`
  workspace reads `status: ready` with no criteria ticked — the flip to `done`
  is in commit `c03f2448c6`, which is not an ancestor of this workspace's
  working copy. Confirm against the artefacts themselves (does
  `cli/tracker-test-support/src/contract.rs` exist, does `accelerator work
  sync` run) rather than against a status field, since the whole cutover half
  consumes them. If 0194 is genuinely incomplete, restore an explicit
  `blocked_by` edge for that half.
- Two non-work-item prerequisites remain open — see the next two entries.
- **Prerequisite, not yet discharged**: a credentialed tracker target. A
  scratch Jira project, a Linear team and API tokens must exist, and — if the
  Open Question resolves to CI — the secrets must be installed on the
  repository, which is an action outside this change. **Toby Clemson owns
  provisioning it** — it is external account administration with no work item of
  its own, and it gates 0210's contract run and 0212's corpus run alike. It gates more than two
  criteria: the ordering rule below requires the contract harness to pass
  before any bash is deleted, so an unprovisioned target blocks **every**
  deletion, relocation and repointing in the story. Provision it before the
  first deletion, or the change strands with both implementations live — the
  half-migrated state 0194 was restructured to avoid.
- **External systems**: Jira REST and Linear GraphQL. The network-touching
  contract lane's success is contingent on both services being reachable, and
  on staying inside per-tenant rate limits, Linear's query-complexity cap and
  its 250-issue bulk truncation. A red contract run is therefore not
  automatically a defect in the change, and an API deprecation on either side
  can break a passing client later.
- **Ordering within the change**: the clients must implement `RemoteTracker`,
  reproduce the projection recipes and pass the contract harness *before* the
  bash scripts are deleted, the corpus relocated or the skills repointed. The
  bash scripts and their generated corpus are the only oracle for projection
  and classification fidelity; deleting them first discards the guard that
  makes the cutover safe.
- **New dependency trees**: `wiremock-rs`, `rustls` and their transitive
  dependencies must clear `deny.toml`'s licence and advisory policy. Any
  allowance or exception needed is part of this change, not a follow-up.
- **Release pipeline**: two new dispatched binaries join the per-platform
  upload set and the minisign-signed `manifest.json` contract 0165 owns. A
  binary that builds and registers locally but is absent from the manifest is
  undiscoverable until a user's launcher tries to fetch it. If the copyleft
  check in the registration requirement fires, 0203 (MPL-2.0 attribution
  artefact, still `ready`) becomes a release-path dependency — a licence
  failure, not a build one, so it must be settled at planning rather than at
  release.
- **Blocks 0174** (Retire Shell Tooling and CI Guards), but the edge is now held
  at **child** level: 0211 clears the integrations floor and seven library
  entries, 0212 clears the work floor and the eighth, so 0174's `blocked_by` names
  0211 and 0212 rather than this item. A blocker lookup from 0174 therefore lands
  on the increments that do the work instead of on a parent that performs none.
  This item retains its own `blocks: 0174` as the rolled-up view.
- The frozen port signature is six items: the trait, `ExternalId`,
  `RemoteIssue`, `RemoteTimestamp`, `FetchOutcome` and `TrackerError`.
  `fetch_all` returns `FetchOutcome.found` as `Vec<(ExternalId,
  RemoteTimestamp)>` — a stamp per key rather than a projected issue — so each
  client's bulk-mode query needs no `description`/body field at all. Linear's
  selection set in particular can stay as narrow as `linear-search-flow.sh`'s
  today, with no need to widen it against Linear's complexity cap.
- `RemoteTimestamp` is a three-variant enum, not a `String` newtype. A client
  maps a tracker's stamp to `Reported(bytes)` verbatim and a blank or null one to
  `NotReported` — never to `Reported("")`, which means nothing. The third
  variant, `NotRead`, is unreachable through the port: a client cannot return it,
  because `show` and `fetch_all` either answer or fail.
- A client that quietly drops an operation once the trait gains a
  default-bodied method is undetectable by 0204's own guards (`cargo public-api`
  and the signature-probe test). Nothing catches that mistake below the shared
  contract harness actually exercising all four operations against the real
  client.
- Carries 0194's cutover. 0194 shipped the Rust sync engine beside the live bash
  path without retiring it, because `sync` and `create`/`update --push` could only
  resolve fakes until these clients existed. This story inherits four obligations
  from it: the script removal and skill repointing, the sync SKILL's
  conversational conflict flow, running the shared contract harness against real
  clients, and per-provider projection fidelity against the committed corpus.
- 0170 (the work-item lifecycle subdomain) is a **discharged** dependency, not
  merely a relation: the skill repointing consumes `accelerator work create`
  and `list`, which 0170 delivers, and `/list-work-items` is why the read path
  must stay bounded. 0170 is `done`, so nothing is blocked; it carries no
  dependency on these clients or the port, and 0194 wires `--push` onto its
  `create`/`update` commands separately.
- 0174 (Retire Shell Tooling and CI Guards) also relates in the other
  direction: this story now clears the whole `work-item-*.sh` surface itself,
  so 0174 inherits none of it. 0174 was updated on 2026-08-17 to say so and to
  record the split of floor and `SHELL_LIBRARIES` ownership between the two, so
  neither a floor nor a library entry can be cleared twice or missed.
- Parent: epic 0136.

## Assumptions

- `wiremock` can express both providers' error shapes faithfully enough to
  exercise all four exit-code mapping tables — including Linear's
  partial-success GraphQL responses, where a `200` carries an `errors` array.
  If it cannot, the affected cases move to the credentialed target rather than
  going uncovered — the transcribed-fixture criterion requires a class
  assertion for every row whichever lane provides it.
- The async-to-sync bridge in the test layer is a one-off pattern, not a
  per-test cost, and does not force the production clients async.
- The four bash exit-code mapping tables are the authority over
  `TrackerError`'s doc comment wherever the two disagree.
- The committed goldens are a sufficient oracle once their bash generators are
  gone — no case in the corpus depends on regenerating it.
- The Rust surface already covers everything the three previously-deferred
  scripts do (`sync-label` → `cli/work/src/sync/label.rs`, `normalise` →
  `cli/work/src/normalise.rs`, `file-dirty` → `working_copy_status.rs` /
  `dirty_paths.rs`), so blanket deletion needs repointing rather than new
  behaviour.

  ⚠️ This assumption and the flow-enumeration one below each **bound the size**
  of one half of the item, and neither is confirmed. Both must be checked before
  planning, alongside the three Open Questions: if the flow list is short, more
  flows need migrating and fixturing; if a Rust replacement is missing, blanket
  deletion means new behaviour rather than repointing.
- The eight flows per provider are the whole user-facing surface of the two
  integration script clusters — no flow exists in bash that the enumerated
  eight plus ADF↔markdown, JQL and GraphQL construction do not cover. Worth
  confirming against both script directories before planning commits to the
  per-flow fixture criterion.

## Technical Notes

- Source bash: `skills/integrations/jira/scripts/` (`jira-common`, `jira-auth`,
  `jira-jql`, `jira-body-input`, `jira-custom-fields`, flows) and
  `skills/integrations/linear/scripts/` (`linear-common`, `linear-auth`,
  `linear-graphql.sh`, flows).
- `reqwest` + rustls keeps the clients musl-static-friendly; no native-tls.
- `wiremock-rs` is the mock layer: `MockServer::start()` binds a random local
  port in-process, servers are pooled across tests and shut down on drop, and
  instances must not be shared between tests. `tokio` and `reqwest` are already
  workspace dependencies and `cli/github` already uses tokio, so no new runtime
  enters the workspace.
- Crate ownership, stated once: 0204 introduced the `tracker` crate together
  with the frozen port; 0194 added the sync engine inside it. References to "the
  `tracker` crate" throughout mean that one crate.
- Contract harness: `cli/tracker-test-support/src/contract.rs`, parameterised
  via `ContractSubject`; excluded from the default test run and driven by
  `mise run test:integration:tracker-contract` (`tasks/test/integration.py`,
  `tracker_contract`).
- Baseline corpus currently at `skills/work/scripts/test-fixtures/`; its Rust
  consumers resolve it by repo-root-relative path today, which is what the
  relocation removes.

## Drafting Notes

- Treated as the Phase 8 story; kept as one grouped item.
- Updated 2026-08-05: 0170 split into 0170 (lifecycle CRUD) and 0194
  (`tracker` crate and remote sync engine). All references to "the tracker port
  from 0170" point at 0194's line of work.
- Updated 2026-08-10: absorbed the cutover from 0194, whose original final phase
  retired the bash sync and bridge scripts — but the real clients replacing them
  are this story's deliverable, so the user-facing `sync` and `--push` flows
  would have been dead between the two, against epic 0136's rule that the plugin
  stays functional at every step. An interim adapter shelling out to the bridge
  scripts was weighed and rejected: throwaway work needing its own retirement,
  for a window this story closes anyway.
- Updated 2026-08-10: the port moved out of 0194 into 0204 so the client crates
  would wait on a trait and its vocabulary rather than a whole sync engine.
- Updated 2026-08-17, enrichment pass. 0194 is complete, so every work-item
  blocker is discharged.
  Five decisions taken: keep Jira, Linear and the cutover as one item now that
  sequencing no longer argues for a split; test with `wiremock-rs` and keep
  Python out of the `cli/` lane; relocate the fixture corpus into the Rust test
  tree rather than leaving it beside the skills; delete **every**
  `work-item-*.sh` and `test-work-item-*.sh` by the end of this story rather
  than deferring three to 0174; and provision the credentialed tracker target
  before the cutover lands. (That last decision originally read "during
  implementation"; the 2026-08-17 review pass re-sited it as a prerequisite,
  since it is external-account administration with no work item of its own.)
- The enrichment found eleven Rust tests resolving paths under
  `skills/work/scripts/`, which no requirement or criterion had accounted for.
  Deleting the scripts breaks the build; deleting the tests discards the
  cutover's only regression guard. Converting them is now explicit, and the
  count is eleven rather than seven precisely because deletion went blanket —
  `normalise_parity`, `sync_label_parity`, `cli_diff_parity` and
  `diff_shellout_parity` pin against scripts the earlier scope retained.
- Blanket deletion also promoted three things from footnotes to requirements:
  the suite floor is removed outright rather than decremented,
  `test-work-item-scripts.sh` dies whole rather than in part, and the
  dirty-work-item overwrite guard at `sync-work-items/SKILL.md:137` needs an
  explicit Rust replacement rather than quietly vanishing.
- `--resolve` takes three tokens (`remote|local|skip`), not the two this item
  previously specified, and `sync` reports conflicts on exit `4` and `71` alike.
  Corrected against `cli/work-cli/src/cli.rs`.
- `producer` left as `extract-work-items`: it records where the item came from,
  and this pass enriched it rather than created it.
- Reviewed 2026-08-17 (review 1, verdict REVISE) and revised in place. The
  review's scope critical — split the two clients and the cutover into
  siblings — was considered and **declined**: the item stays whole. Everything
  else was applied. The substantive changes: the integration script clusters are
  now a requirement rather than an assumption, with the eight flows per provider
  enumerated and both `SKILL.md` sets repointed in the body, not just their
  frontmatter; the primary client criterion no longer verifies against the bash
  suites it deletes, but against committed request/response fixtures captured
  from those flows before deletion; the four exit-code tables and the
  timeout/page-cap bounds gained criteria with named threshold values; the
  identifier-safety check was lifted out of the four-capability decision (whose
  list enumerated three) into its own four-case criterion, leaving three genuinely
  open fates; the contract run now covers all four port operations and needs a
  named execution route; the corpus criterion gained a seeding procedure and the
  absent-description case; and Dependencies gained the credentialed-target
  prerequisite, both external systems, the intra-change ordering rule, the
  `deny.toml` and release-manifest couplings and an explicit `blocks: 0174`.
- The review reported 0194's status as `ready`, contradicting this item. That is
  workspace divergence: the flip to `done` is in commit `c03f2448c6`, verified
  not to be an ancestor of this workspace's working copy. Dependencies now says
  to confirm 0194 against its artefacts rather than its status field, since a
  divergence in either direction is possible.
- Children 0210-0213 reviewed 2026-08-17 (one pass, five lenses over all four as a
  set). Every finding was applied. The pass found one **critical**, raised
  independently by three lenses and verified directly: the three port-less bridge
  capabilities — unkeyed discovery `search` and the two `--dry-run` behaviours —
  had landed in no child, while 0212 deletes the scripts carrying them. All four
  children could have been accepted green while `/sync-work-items` lost remote-issue
  discovery and `--preview` lost live push validation. That is the same
  partition failure that killed `## Increments`, reproduced in a second structure;
  it is now 0212's, and a full audit (28 requirements, 31 criteria) confirmed it was
  the only gap.
- Other fixes from that pass. 0211's exit-code mapping is now anchored to the
  retiring bash flow's codes or `tracker`'s existing values rather than to a
  document the same child authors — it previously could not fail. The per-flow
  fixture *provenance* obligation, silently dropped in the carve-out, is restored,
  so sixteen fixtures cannot quietly pin the new clients to the mock servers 0211
  deletes. The whole-repository `jq`/`curl` equality assertion moved to 0212, since
  0211 could not satisfy it before the work skills were repointed. The
  ADF/JQL/GraphQL assertions moved to 0210, where the code they pin lives. 0210
  gained a whole-corpus offline projection criterion — the only window in which the
  bash corpus exists — and its three transcriptions now have named paths and are
  scoped to its own merge, so the siblings' "verify against 0210's baseline"
  criteria resolve to files. 0213 gained a Context, an Assumptions entry, a
  stub-on-`PATH` walkthrough with a stated pass predicate and three named fixtures
  (including the clean exit-`0` case), and an automated skills-lane guard — 0212
  edits the same file, so a manual-only check would not have survived it.
- 0211 also gained a **reverse** cross-cluster sweep. The pass-3 reordering was
  justified by references running from the integration clusters into
  `skills/work/scripts/`; nobody had checked the other direction, where the
  surviving work-item bridges and their three suites may drive the two Python mock
  servers 0211 deletes. Unswept, the chosen order could create the mirror of the
  break it was chosen to prevent.
- Structural changes from that pass: this item is now `kind: epic` (nested under
  0136, following the 0145-under-0192 precedent) with 0210-0212 as `story` and 0213
  as `task`, since three children are multi-week efforts that `task` mis-sizes. The
  duplicated Requirements and Acceptance Criteria were removed in favour of a
  `## Scope` section stating that **the children are normative** — the duplication
  had already drifted twice in one revision, losing a provenance clause and an
  assertion-count qualifier, and both losses surfaced as findings. The 0174 edge
  moved to child level: 0174's `blocked_by` now names 0211 and 0212, the increments
  that actually clear its floors and library entries.
- Not applied, and deliberately: the scope lens's recommendation to split 0210 and
  0211 along the **provider seam** (jira and linear as separate children). It would
  halve the two largest children and let the providers proceed in parallel, and the
  seam is real — separate crates, binaries, clusters and skill sets, sharing only
  the port, the composition root and the async-to-sync test pattern. It is not
  applied because it doubles the child count on a decomposition the author has
  already ruled on twice, and because the shared items would need an arbitrary home
  in whichever provider lands first. Worth revisiting if 0211's unconfirmed
  flow-enumeration assumption comes back short.
- Reviewed a third time 2026-08-17 (review 1, pass 3, REVISE on major count with
  no criticals). The decisive finding was that `## Increments`, added in pass 2 to
  mitigate the scope critical, had become the largest single source of majors: it
  contradicted the Requirements' "in the same change" clauses, did not partition
  the requirement set, left the composition-root wiring unassigned, and — the
  live break — put the deletion of `work-item-sync-label.sh` in an increment
  before the one retiring the two integration-cluster scripts that still call it.
  Resolved at the source by reifying the four increments as children 0210–0213
  parented to this item, each with its own status, requirements and criteria,
  following the 0166 → 0178/0179/0180 precedent. Ordering is now 0210 → 0211 →
  0212 with 0213 free, and 0211 precedes 0212 precisely so the sync-label callers
  die before the script does. The independent pass-3 findings were applied too:
  the contract-run criterion demands an enforcing route rather than a recorded
  one (writing down "a broken client can reach `main`" used to satisfy it); the
  consumer sweep names a definite repository-wide grep instead of excluding the
  only two references it cited; the binaries gained an exit-code contract with a
  document of record and stdout goldens for the sixteen subcommands; the
  absent-description rule gained an offline criterion so the item's highest risk
  is not verified only in the unprovisioned credentialed lane; the timeout
  criterion's self-contradictory window became a T-relative assertion plus a
  defaults check; and "all four port operations" now enumerates four operations
  with the two cross-operation obligations listed separately.
- The scope lens's position, stated across all three passes, was that this item
  is epic-scale for a `story`. Splitting into siblings was declined twice; the
  child decomposition is the third answer, and it keeps 0171 as the single unit
  of value while giving each increment its own acceptance gate.
- Reviewed again 2026-08-17 (review 1, pass 2, still REVISE — no criticals
  remained, but the major count kept the verdict). Applied: the four mapping
  tables now pair to a named provider and operation and are transcribed to a
  committed fixture before deletion, alongside the ADF node-type inventory and
  the per-test assertion baseline — three oracles that previously existed only
  in the code being deleted; the tautological `jq`/`curl` audit and the
  escapable "any migrated production script" both became equality assertions;
  the work-skill repointing, the two binaries' own CLI surface, sub-binary
  registration with the release manifest, and the `RemoteTimestamp` blank/null
  rule each gained the criterion they lacked; the timeout criterion gained a
  two-sided window and an override seam so it cannot be waved through or turn
  into 100s of wall clock; the fixture-deletion and drop-decision escape
  clauses were closed; and the additive-port-item branch was moved out of scope
  entirely — 0204 is frozen, so that fate means a new blocking item, not a
  dependency on a done one. An `## Increments` section named four ordered,
  individually mergeable changes — superseded by the pass-3 child decomposition
  above, which reifies those four as 0210–0213.
- Three Open Questions must close before pickup — the secrets siting, the three
  port-less capability fates, and the `EXIT_CODES.md` siting. Each of the first two
  changes what is in scope, so sizing is not settled until they are answered. That
  originally held this item at `draft`.
- Superseded 2026-08-17: the five items were moved to `ready` with those three
  questions **still open**, alongside the two ⚠️ size-bounding assumptions in 0211
  and 0212. They are carried into planning rather than treated as pickup gates, so
  the first planning session must answer them before committing to a shape. The
  five reviews were accepted at the same time with their open findings recorded
  rather than resolved — see each review's Acceptance section for what remains
  known-imperfect, including three self-contradictions introduced during the fix
  rounds (0211's mock-server deferral, 0211's `jq`/`curl` survivor set, 0213's stub
  seam) and two latent gaps better closed by implementing 0210 than by more
  specification.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Related: 0194, 0204, 0170, 0174
- Review: `meta/reviews/work/0171-jira-and-linear-integrations-review-1.md`
- ADRs: ADR-0045, ADR-0046, ADR-0053
