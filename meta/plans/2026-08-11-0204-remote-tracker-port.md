---
type: plan
id: "2026-08-11-0204-remote-tracker-port"
title: "RemoteTracker Port Implementation Plan"
date: "2026-08-11T15:58:59+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0204"
parent: "work-item:0204"
derived_from: ["codebase-research:2026-08-11-0204-remote-tracker-port"]
tags: [rust, tracker, sync, port, cargo-pup]
revision: "1b7e6583aacac3f08ea2b0c03635192f557290e1"
repository: "accelerator"
last_updated: "2026-08-11T22:57:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# RemoteTracker Port Implementation Plan

## Overview

Build `cli/tracker/` — a zero-dependency domain crate holding the
`RemoteTracker` trait, its vocabulary types and its error taxonomy, plus the
cargo-pup rule and the tests that hold the surface frozen. The crate ships no
runtime behaviour. Its purpose is to be a stable, cheap-to-reach milestone that
unblocks 0171's provider clients and 0194's sync engine simultaneously, so
neither waits on the other.

## Current State Analysis

`cli/` is a 24-member Cargo workspace with a settled hexagonal shape: a
bare-named domain crate, a `<name>-adapters` sibling, and a `<name>-cli`
composition root owning the `accelerator-<name>` binary. `tracker` joins as a
domain crate with deliberately no adapter sibling — the sync state machine
lands in `work`/`work-adapters` (0194) and the provider clients in their own
crates (0171).

Remote tracker sync exists today only in bash, under
`skills/work/scripts/`. Four scripts define the behaviour this port freezes a
contract against:

- `work-item-bridge-codes.sh` — the exit-code taxonomy, sourced by every bridge
- `work-item-create-remote.sh` / `work-item-update-remote.sh` — the mutating
  bridges and their failure classification
- `work-item-fetch-remote.sh` — single-item and bulk retrieval
- `work-item-project-remote.sh` — the body projection recipe
- `work-item-sync-classify.sh` / `work-item-sync-baseline.sh` — the consumers
  of the timestamps and the fetch partition

None of this has a Rust port. `cli/work-adapters/src/project_remote.rs` is the
sole exception and it is unused in production.

Enforcement in the workspace is membership-derived: `cargo fmt --all`,
`clippy --workspace`, `cargo deny` and `nextest --workspace` all take scope
from `[workspace].members`. Only cargo-pup is per-crate, and **there is no
coverage guard** — a new crate ships with zero architectural enforcement until
its `pup.ron` rule is written by hand, and nothing notices the omission.

### Key Discoveries

- **The package name must be the bare directory name.** `cli/vcs/Cargo.toml:2`
  is `name = "vcs"`; `accelerator-vcs` is the binary at `cli/vcs-cli/`. The
  reason is at `tasks/README.md:454-459` — cargo-pup rules match on whole crate
  names, so a domain crate renamed `accelerator-tracker` would silently stop
  matching its rule.
- **`thiserror` is unreachable and unnecessary.** Every domain and adapter
  error type hand-writes `Display` and `std::error::Error`. The exact precedent
  for a crate with no `kernel` dependency is `cli/document/src/error.rs:1-42` —
  hand-written `Display`, empty `impl std::error::Error for X {}`, and no
  `From<X> for kernel::Error`.
- **One single item per `use` line, fully `crate::`-qualified.** Not a rustfmt
  setting — cargo-pup resolves a grouped `use a::{b, c}` to an empty module
  name, which no `allowed_only` regex matches, so the rule rejects it
  (`cli/pup.ron:131-134`, pinned by
  `tests/integration/pup/test_import_rule.py:593-600`). `use super::` and
  `use self::` fail for the same reason. Test targets are exempt in practice:
  `tasks/pup.py:17` runs bare `cargo +nightly pup` with no `--tests`.
- **The two anchor positions differ by convention, not by meaning.**
  `matches:` is written `"^<crate>($|::)"` and `allowed_only` entries
  `"^<path>(::|$)"` throughout `cli/pup.ron`, but the two alternations accept
  exactly the same strings — order decides only which branch a backtracking
  engine tries first, never whether a match exists. Copy the house form for
  consistency; do not expect a swapped order to be detectable.
- **The house `Display`-test location conflicts with AC 10.** The recipe at
  `cli/corpus/src/store.rs:81-148` puts one test per `Display` arm in an inline
  `#[cfg(test)] mod tests`, which 0204 forbids in `tracker/src/`. `Display` is
  public surface, so the tests move to `tracker/tests/` intact.
- **`warnings = "deny"` plus clippy pedantic and nursery**
  (`cli/Cargo.toml:133-147`). Two consequences: `missing_errors_doc` demands a
  `/// # Errors` section on all four trait methods, and `missing_const_for_fn`
  (nursery) will demand `const fn` on the two `new` constructors. `as_str`
  cannot be const — `String::as_str` is not const-stable — so the asymmetry is
  expected, not drift. `#[must_use]` on the value-returning inherent methods is
  **house style, not lint-enforced**: `cli/Cargo.toml:147` sets
  `must_use_candidate = "allow"` alongside `module_name_repetitions`. It is
  applied here for consistency with ~260 existing sites (e.g.
  `cli/vcs/src/lib.rs:27`).
- **Clippy lints test targets too.** `tasks/lint/cli.py:5-15` runs
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
  and `expect_used`/`unwrap_used` are cherry-picked to `warn`. Every integration
  test in the workspace that calls `expect` therefore carries a file-level
  `#![allow(clippy::expect_used, clippy::unwrap_used)]` —
  `cli/verify/tests/verify.rs:4`, `cli/config-adapters/tests/config_reader.rs:5`,
  `cli/launcher/tests/resolution.rs:5` and five more. It is required, not
  optional.
- **`fn as_str(&self) -> &str` has no precedent in the workspace.** Every
  existing `as_str` returns `&'static str` from a match over variants; none
  borrows from `self` (the closest, `cli/visualiser/server/src/frontmatter.rs:12`,
  is `&self`-taking but still returns `&'static str`). House style also prefers
  validating constructors (`Key::parse`) over infallible `new`. The
  `new(String)` + `as_str(&self)` pair is a deliberate departure, frozen by the
  work item. It stays non-`const` because `&self.0` reaches `&str` through a
  `Deref` call that is not const-evaluable — not because `String::as_str` is
  unstable in const, which it has not been since 1.87.
- **`cargo public-api` does not exist in this repository yet — but the lane it
  needs already does.** It is not a `mise.toml` pin, task, CI job or lockfile
  entry. It does *not*, however, require a new toolchain: `deps:install:pup`
  already provisions a rustup-managed pinned nightly
  (`PUP_NIGHTLY = "nightly-2026-01-22"`, `tasks/shared/rust.py:6-7`), invoked as
  `cargo +<nightly>`. `tasks/README.md:257-291` says mise cannot pin two rust
  toolchains — which is *why* the nightly is rustup-managed, not a reason a
  second nightly consumer is impossible.
- **`cargo-public-api` couples more weakly than `cargo-pup` does.** cargo-pup
  carries a `rustc_private` driver, so its binary only loads under the nightly
  it was *built* against — hence the `PUP_NIGHTLY`/`PUP_VERSION` matched pair.
  cargo-public-api has no driver: it builds on stable and shells out to nightly
  `rustdoc` for JSON. The only version coupling is rustdoc-JSON format ↔ tool
  version, so it can share the existing pinned nightly rather than forcing a
  third toolchain.
- **The nightly lane's isolation is guarded, and the guard is name-based.**
  `tests/unit/tasks/test_workflows.py:349-402` asserts that exactly one CI job
  consumes the nightly and that it is `check-architecture`, detecting consumers
  by the markers `("pup:check", "deps:install:pup", "+nightly")`. A second
  nightly task added to the same job is fine, but the marker list must learn its
  name or a future stable-lane leak of that task goes undetected.
- **`--profile minimal` may not carry `rustdoc`.** `deps:install:pup` installs
  the nightly with `--profile minimal` plus `rustc-dev`, `rust-src` and
  `llvm-tools-preview` (`tasks/deps.py:64-66`) — none of which is rustdoc.
  Confirm empirically whether the minimal profile ships the rustdoc binary; if
  not, the install step gains a component.
- **`work-item-bridge-codes.sh` defines four dispatch codes, not two** — 70/71
  retryable/terminal, plus 72 `E_DISPATCH_NOT_AVAILABLE` and 73
  `E_DISPATCH_UNRECOGNISED`. The two-class `TrackerError` remains correct: 72
  and 73 are dispatch-routing outcomes that resolve *above* the port, at the
  composition root selecting `Box<dyn RemoteTracker>` from `work.integration`.
  If the config names `trello` there is no client and no port call to make.
- **The retryable/terminal rule turns on mutation, not transmission.**
  `work-item-bridge-codes.sh:9` defines 70 as "failure provably BEFORE any
  remote **mutation**", and `work-item-create-remote.sh:100-104` spells the set
  out: "Retryable = provably no issue created (arg / validation / auth /
  **4xx-reject** / rate-limit / unresolvable-config)", mapping Jira codes
  11-15, 17, 19, 22 and 34 there. That set deliberately mixes both sides of the
  wire: 15, 17 and 22 (`E_BAD_SITE`, `E_REQ_BAD_PATH`, `E_REQ_NO_CREDS` —
  `jira-request.sh:10-26`) never leave the machine, while 11, 12, 13, 14, 19 and
  34 reached the tracker and were rejected. Both are retryable, which is exactly
  why the test is the mutation rather than the transmission — the narrower rule
  would reclassify every 400, 401 and 429 as terminal, so the sync would refuse
  to retry calls that provably changed nothing. The catch-all
  `*) return "$E_DISPATCH_TERMINAL"` still holds for everything unproven
  (`:99-111`, `work-item-update-remote.sh:51-72`) — that conservative default is
  the part a client implementer will get wrong, so both halves belong in the
  doc comment.
- **Timestamps are cache keys, not clocks.** `work-item-sync-classify.sh:177`
  compares with raw `[ "$a" = "$b" ]`. An unequal stamp means "go hash the
  body", never "remote is newer". Real values differ by provider — Linear
  `"2026-06-21T00:06:10.647Z"`, Jira `"2026-07-09T08:00:00.000+0000"` (numeric
  offset, no colon) — and a `chrono`/`time` round-trip would rewrite `+0000` to
  `+00:00`.
- **`""` is a legal stored timestamp.** `work-item-project-remote.sh:68,79`
  default to empty via `// ""`, and `work-item-sync-apply.sh:121` leaves it
  empty when a post-push `show` fails. A validating constructor would reject
  data already on users' disks.
- **The bulk fetch is a three-way partition guarding one invariant.**
  `work-item-fetch-remote.sh:21-25` returns `found`/`absent`/`indeterminate`,
  where **absent is only ever drawn from a provably complete fetch**.
  `work-item-sync-classify.sh:127-147` routes the two to different
  user-visible states. Linear's bulk path caps at 250 issues team-wide
  (`_WIFR_LINEAR_LIMIT=250`) against roughly 180 synced items in this repo
  today, so truncation is live rather than hypothetical.
- **The bulk fetch carries no body, and cannot.** Its normalised contract is
  `{ "found": { "<key>": { "updated": "<iso|null>" } }, … }` — a timestamp per
  key and nothing else — with `show` reserved as "the per-item full-fidelity
  read returning the issue's body + updated timestamp (the genuinely-changed
  minority)" (`work-item-fetch-remote.sh:20-40`). Both adapters build the found
  map as `{updated: (… // null)}` (`:126-128`, `:169-171`), and Linear's bulk
  selection set requests no `description` field at all. The two-tier read is
  the design, not an omission: bodies are fetched only for the minority whose
  stamp moved. Both adapters also `unique` the requested key set, so duplicate
  ids are collapsed rather than being an error.
- **The projected body is two lines, with no blank between them.**
  `work-item-project-remote.sh:73,84` is `printf '%s\n%s\n' "$summary" "$desc"`
  for both providers — title line, then description. Only Jira's description is
  canonicalised (`jq -cS`, key-sorted and compact, because ADF is structured);
  Linear's Markdown passes through verbatim. `work-item-normalise.sh` trims per
  line and strips only *trailing* blanks, so an interior blank line inserted by
  a client survives into the hash.
- **There is a third read mode.** `work-item-fetch-remote.sh` exposes plain
  `search [filter-flags…]` alongside `search --keys` and `show`: the unkeyed
  discovery path that `/sync-work-items` uses to list remote issues with no
  local work item. It is not key-scoped, so `fetch_all(ids)` cannot express it.
- **Read failures never carry the terminal class.**
  `work-item-fetch-remote.sh:42-48`: "A read mutates nothing, so 71
  (terminal-may-have-mutated) does not apply here — any underlying read failure
  collapses to 70 (the caller degrades to presence-only)."
  `work-item-bridge-codes.sh` records the same as "Read bridges never emit
  this." The retryable/terminal mapping is therefore **operation-scoped**, the
  same asymmetry the Linear-code-34 trap shows on the write side.
- **The empty stamp must never compare equal to another empty stamp.**
  `work-item-sync-classify.sh:177` short-circuits to unchanged only under
  `[ -n "$base_remote_updated" ] && [ "$a" = "$b" ]`, so an unknown baseline
  never matches an unknown remote. A derived `PartialEq` on a `String` newtype
  loses that guard silently.
- **Fixtures are always `CARGO_MANIFEST_DIR`-rooted**, never cwd-relative — 27
  call sites. Reaching outside the crate uses the same env var plus `..`
  (`cli/vcs-cli/tests/detect_goldens.rs:24-31`). That is the mechanism the
  parity fixture needs to read the bash script.
- **The pup probe harness drives the *shipped* `cli/pup.ron`.**
  `tests/integration/pup/test_import_rule.py:218-224` explains why: a synthetic
  workspace whose crate is literally named `config` exercises the real
  `^config($|::)` regex, so a typo in or deletion of the shipped rule fails the
  test. It runs only in the nightly architecture lane, never in the test
  roll-up.

## Desired End State

`cli/tracker/` exists, is a workspace member, is governed by a cargo-pup rule
proven by an automated probe pair that exercises both its denials and its
permissions, and exposes six public items and nothing else. `mise run` is green
end to end. 0171 and 0194 can both begin against a signature that cannot drift
— including its trait methods and its derives — without a test going red.

One thing the crate deliberately does not guarantee: that a client's
`FetchOutcome` is a total partition. The type cannot enforce it, AC 10 forbids
the constructor that could, and the contract test that would hold each client
to it is 0194's. That is a stated gap, not an oversight — see Handoffs.

Verification: `mise run` exits 0; `mise run public-api:check` holds the surface
against its committed snapshot; `mise run pup:check` exits 0;
`mise run test:integration:pup` exits 0 and its `tracker` probe pair fails when
the rule is deleted or either anchor mistyped; `cargo nextest run --workspace`
builds and runs `tracker/tests/`.

## Deviations From The Work Item

### Already reconciled into 0204

Four deviations were settled during the first planning pass and are already
carried by `meta/work/0204-remote-tracker-port.md`, whose Drafting Notes record
the reopening. They are listed here so a reader of the plan alone knows what the
frozen block says and why, not as outstanding work.

**1 → `fetch_all` gained a partition type and a key argument.** The original
`fetch_all(&self) -> Result<Vec<(ExternalId, RemoteIssue)>, TrackerError>`
cannot express "the fetch was incomplete", so the caller's only route to absence
is `requested − returned` — exactly the unsound inference the bash path is built
to avoid, against a provider that truncates at 250 against ~180 items. The port
takes a `FetchOutcome` carrying the three-way partition, and takes the ids to
scope the retrieval as the bash bulk mode does. The surface is six items, not
five.

**2 → AC 1's `cargo public-api` snapshot was replaced by a self-reading surface
test, and that has now been reversed — see deviation 8.** The tool is adopted
after all. This entry is kept only so the reversal is legible; the reasoning
that follows was wrong on both counts.

**3 → the pup rule omits the `^kernel::Error(::|$)` allowance.** With no
`[dependencies]` table `use kernel::Error;` cannot compile, so the line would be
inert and would misdescribe the crate. Dropped, with the reason recorded in
`pup.ron`.

**4 → the parity fixture enumerates all four dispatch codes.** The script
defines four, and a two-code fixture cannot fail when it gains a fifth — which
is precisely what AC 6 asks of it. The fixture carries all four, asserts exactly
two map onto `TrackerError`'s classes, and records that 72 and 73 resolve above
the port.

### Requiring a further 0204 edit

The deviations below were found by plan review and settled before writing.
Each changes the
block 0204 declares verbatim frozen, so **0204 must be edited before this is
implemented** — and, unlike the four above, so must 0194's and 0171's
descriptions of the port, which still say five items.

**5 → the bulk partition carries a timestamp, not a whole issue.**
`FetchOutcome.found` becomes `Vec<(ExternalId, RemoteTimestamp)>`;
`RemoteIssue` stays as `show`'s return type only. The bulk contract this ports
returns a timestamp per key and nothing else, reserving bodies for the
genuinely-changed minority, and Linear's bulk query has no `description` field
to project from. A mandatory `body` on the bulk arm leaves a conforming client
two options: fabricate `String::new()`, which by `RemoteIssue.body`'s own doc
comment reclassifies every synced item as remotely modified; or issue a `show`
per id, which destroys the bulk design and collides with 0194's own criterion of
zero `show` calls in bulk mode. Dropping the field makes the wrong answer
unrepresentable rather than merely discouraged, and it retires the
Linear-`description` constraint that was otherwise being handed to 0171. This is
the same class of defect as deviation 1 and the second finding that could
otherwise have made the port produce a wrong answer.

**6 → four derives are added, and `ExternalId` gains `Display`.**
`TrackerError` derives `Clone, PartialEq, Eq`, matching every other error type
in the workspace (`document::DocumentError`, `corpus::StoreError`,
`config::ConfigError`, `work::UpdateError`, `collaboration::ForgeApiError`);
without `PartialEq` no consumer can `assert_eq!` over a `Result` returning it,
which both 0171's and 0194's tests will want. `ExternalId` derives `Hash` and
implements `Display`: 0194 joins `FetchOutcome.found` against its local work
items on every sync run, and without `Hash` it must linear-scan or re-key by
`String`, discarding the newtype exactly where it earns its place. `Display`
removes `as_str()` from every format site, following `config::Key`
(`cli/config/src/key.rs:9-38`). `RemoteTimestamp` gains neither — the no-ordering
rule stands, and its equality carries a caveat recorded on the type.

**Consequence for AC 10 and for Requirements**: `impl Display for ExternalId` is
a function body in `src/`. Two places must move, not one — AC 10's list of
permitted bodies, and the Requirements sentence "`Display` and `Error` on
`TrackerError` are the sole permitted impls with bodies", which sits *inside*
the block declared verbatim frozen.

**7 → both `new` constructors are `const fn`.** Nursery's
`missing_const_for_fn` under `warnings = "deny"` is expected to reject `pub fn
new` as 0204 writes it. `const` is also a forward commitment — removing it later
breaks any consumer calling it in a const context — so it belongs in the frozen
block rather than in the implementation.

**Confirm this one empirically before editing 0204**, and apply the answer
uniformly. `FixedTracker`'s `holding`, `truncating` and `losing` have the same
shape — `Drop`-carrying parameters moved into a struct literal — and clippy
lints test targets too, so either all of them are `const fn` or none need to be.
The two positions cannot both be right.

`missing_const_for_fn` has historically declined to fire on functions whose
parameters carry a `Drop` impl, which `String` does. Compile the two newtypes against the workspace lint
profile first: if the lint is silent, the deviation is a gratuitous forward
commitment and should be dropped rather than justified on a compulsion that does
not exist. The same check settles whether `as_str` is demanded as `const`
(it is not — see Key Discoveries) before the block is declared final a second
time.

**8 → the surface is pinned by `cargo public-api`, not by a self-reading test.**
This reverses deviation 2 and restores AC 1's original instrument.

The self-reading test was tried and failed three review passes. Each pass found
a different source shape it mishandled — first trait methods and derives, then
owner attribution and rustfmt-wrapped signatures, then brace-struct variants,
empty `impl` blocks, attribute ordering, and argument lines lifted out of a
`write!` body. The defects were not careless; they are what happens when a
contract is pinned by line-shape heuristics over text, because such a parser
cannot be verified by reading it — only by tracing every shape the source might
take, which is precisely the work it was meant to save.

`cargo public-api` reads rustdoc JSON, so it never sees function bodies,
formatting or attribute order. Every one of those defects is absent by
construction rather than by patch. Three further properties matter here:

- **Derives are pinned semantically.** They surface as the impls they generate
  (`impl Clone for ExternalId`, `impl PartialOrd for RemoteTimestamp`), so the
  derive 0204 forbids is caught as an added impl rather than as a changed
  literal.
- **The snapshot is format-independent.** rustfmt cannot move it, which retires
  the whole reconcile-after-formatting step and the class of false diffs it
  produced.
- **It is still hand-writable.** Output is one fully-qualified item per line,
  sorted — mechanical to author from 0204's frozen block, so the snapshot still
  starts red rather than characterising whatever was built.

The cost is a build-system change: a pinned tool, an install task, a check task,
CI wiring and edits to three committed guards. Phase 1 §10 carries
it. The earlier objection — a third toolchain — was wrong twice over: the
nightly already exists, and cargo-public-api has no `rustc_private` driver, so
unlike cargo-pup it does not need to be built against the toolchain it invokes.

**Consequence**: AC 1 must be rewritten. It currently specifies a golden under
`tracker/tests/fixtures/` read by a test that parses `src/lib.rs`, and
explicitly says `cargo public-api` "is deliberately not used".

## What We're NOT Doing

- No provider clients, no HTTP, no `tracker-adapters` — 0171 and the work item
  both exclude them.
- No sync state machine, no `accelerator work sync`, no baseline store, no
  classifier — 0194 owns all of it.
- **Not moving `cli/work-adapters/src/project_remote.rs`.** It is written,
  unused in production, and on the wrong side of the boundary this crate draws:
  `work-adapters` depends on `work`, so a 0171 client reusing it would pull in
  the whole lifecycle domain. It also projects the **`show`** payload shape
  (`/fields/…`, `/data/issue/…`) while the bulk payloads are shaped differently
  (`.issues[].fields`, `.data.issues.nodes[]`), so it is not reusable for
  `fetch_all` even setting the dependency aside. Recorded here and handed to
  0171 rather than resolved.
- **Not adding `description` to Linear's bulk GraphQL selection set.**
  `linear-search-flow.sh:157-165` requests
  `nodes { id identifier title updatedAt state { name } assignee { name } }` —
  no description at all, and adding one interacts with Linear's complexity cap
  (exit 36, classified retryable). Deviation 5 removes the need: the bulk arm
  returns a `RemoteTimestamp`, which `updatedAt` already supplies, so no client
  has to widen the query. Recorded because the shape is the reason the partition
  carries a stamp rather than an issue.
- **Not enforcing `FetchOutcome`'s totality in the type.** A
  `FetchOutcome::partition(requested, found, retrieval_was_complete)`
  constructor would make an unsound absence unrepresentable, but AC 10 forbids a
  function body in `src/` beyond the ones it names, and that criterion is
  deliberate. The invariant stays a documented obligation here and passes to
  0194's parameterised contract test — see Handoffs. The cost is stated rather
  than hidden: 0171's clients are written before any mechanism exists to catch a
  violation.
- **Not porting the read bridge's unkeyed `search` mode.** The untracked-
  discovery path forwards user filter flags to the tracker and returns whatever
  matches, with no local key to scope by, so it cannot be expressed as
  `fetch_all(ids)`. It is a listing and discovery concern rather than a sync-
  engine one, and the port's four operations are declared final — so leaving it
  unrecorded would mean rediscovering it at 0171's cutover, when adding a fifth
  operation is a new work item. Handed to 0171 explicitly — see Handoffs.
- **Not porting the create bridge's `--dry-run` preview.**
  `work-item-create-remote.sh:12-20` resolves and previews the tracker's target
  fields without creating anything, so `/create-work-item` can make an informed
  push offer and fail early on an unresolvable Jira project. It is a
  field-resolution capability rather than one of the three bridge operations,
  and 0194's `--preview` is a different thing (it routes mutations to no-ops and
  makes no port call). Left above the port and handed to 0171 — see Handoffs.
- No `#[non_exhaustive]` on `TrackerError` — a compile-breaking third class is
  the property both consumers want. This matches `document::DocumentError` and
  deliberately diverges from `corpus::StoreError` and `config::ConfigError`.
- **No `[dev-dependencies]` either.** AC 9 says "no dependencies at all";
  strictly `[dev-dependencies]` is a separate table, but nothing planned needs
  one (no I/O, so no `tempfile`; no parameterised-test crate exists in the
  workspace anyway). Both tables stay absent, stated so a reviewer and an
  implementer read AC 9 the same way.
- No probe-pair backfill for `work_adapters::filesystem` or `migrate_adapters`.
  Phase 3 covers the four whole-crate domain rules including their crate-specific
  allowances; those two module-scoped adapter rules are shaped differently, do
  not fit the same writer, and remain unprobed as follow-up.

## Implementation Approach

Three phases, each independently mergeable. Phase 1 lands a governed, compiling
crate carrying the vocabulary but no port; Phase 2 adds the port and the
freeze; Phase 3 backfills probe coverage for four rules this crate does not
touch. Phase 1 deliberately carries `tracker`'s own pup probe pair so the
enforcement rule merges together with the proof that it works, rather than
arriving a phase later.

One caveat on independence: Phases 1 and 3 both edit
`tests/integration/pup/test_import_rule.py`, and Phase 3 renames constants that
Phase 1's probe uses. They are independent in *subject* — no shared source, no
shared crate — but not conflict-free. Merge Phase 1 first and rebase Phase 3
onto it, and re-run the tracker probe pair after Phase 3 lands.

Test-first throughout, and the ordering is load-bearing rather than decorative:
for a crate whose whole deliverable is its tests, writing them second produces
tests shaped to fit what was built, which is how a freeze ends up pinning an
implementation instead of a contract. **Every numbered step below is in the
order it should be performed**, tests ahead of the source they drive, with the
expected red named at each step. The surface golden in particular is
hand-written from 0204's frozen block *before* `src/lib.rs` exists, so it starts
red; capturing it from the implementation's own output is the procedure for a
later deliberate change, never for the first commit.

---

## Phase 1: The Crate, Its Vocabulary, and Its Enforcement Rule

### Overview

Register a zero-dependency `tracker` crate, govern it with a cargo-pup rule
proven by an automated probe pair that exercises both what the rule denies and
what it permits, and land the four vocabulary types — tests first. Close with
the add-a-library-crate checklist that stops the next crate rediscovering all
of this. No trait yet.

### Changes Required

#### 1. Workspace registration

**File**: `cli/Cargo.toml`
**Changes**: append `tracker` to `[workspace].members`, then regenerate the
lockfile.

```toml
members = ["launcher", "kernel", "verify", "document", "config", "config-adapters", "corpus", "corpus-adapters", "corpus-cli", "vcs", "vcs-adapters", "vcs-cli", "vcs-test-support", "store", "visualiser/server", "work", "work-adapters", "work-cli", "collaboration", "github", "collaboration-cli", "migrate", "migrate-adapters", "migrate-cli", "tracker"]
```

`lint:cli:check` runs clippy with `--locked` (`tasks/lint/cli.py:15`), so an
unsynced `Cargo.lock` reddens `cli:check` as an apparent clippy failure. Sync it
with the house command:

```
cargo metadata --manifest-path cli/Cargo.toml --format-version 1
```

Two commands are wrong here and both are tempting. `cargo update -p tracker`
resolves its spec against packages already in the lockfile, where `tracker` is
not yet present. And `cargo generate-lockfile` re-resolves the whole ~360-package
closure, floating every caret-bounded dependency — `tasks/CLAUDE.md:9` bans it by
name, `tasks/version.py:62-65` explains why, and
`tests/unit/tasks/test_version.py:88` asserts no task ever invokes it. A
zero-dependency crate registration must not carry a workspace-wide dependency
bump.

#### 2. Crate manifest

**File**: `cli/tracker/Cargo.toml` (new)
**Changes**: the `cli/vcs/Cargo.toml` shape with no dependency tables at all.
This is the workspace's first zero-dependency member. Inheriting `version` is
load-bearing — `tasks/build.py:102-129` only catches a *mismatch*, so a
hardcoded literal passes today and breaks at the next workspace bump.

```toml
[package]
name = "tracker"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true
```

#### 3. The cargo-pup rule

**File**: `cli/pup.ron`
**Changes**: add a whole-crate `RestrictImports` rule after the `work` domain
rule, matching the sibling shape minus the `kernel::Error` line.

```
        // The whole tracker crate is domain: the port a remote issue tracker
        // satisfies, and nothing else. Unlike its sibling domain rules it
        // omits the kernel::Error allowance — the crate declares no
        // dependencies, so such an import could not compile.
        Module((
            name: "tracker_domain_imports_only_permitted",
            matches: Module("^tracker($|::)"),
            rules: [
                RestrictImports(
                    allowed_only: Some([
                        "^(std|core|alloc)(::|$)",
                        "^crate(::|$)",
                    ]),
                    denied: None,
                    severity: Error,
                ),
            ],
        )),
```

Copy each anchor verbatim: `matches:` is the **resolved module path** and takes
`"^<crate>($|::)"`, while `allowed_only` entries match the **literal `use`-path
text** and take `"^<path>(::|$)"`. The alternation order differs between the
two positions (`cli/pup.ron:1-8`).

#### 4. The pup probe pair — AC 9, automated

**File**: `tests/integration/pup/test_import_rule.py`
**Changes**: append a probe section driving the shipped `cli/pup.ron` against a
synthetic workspace whose crate is literally named `tracker`, importing one
literally named `work`. AC 9 requires the pair be automated and re-runnable
rather than a one-off demonstration.

The compliant control carries **real imports drawn from the permit list**,
following `_LIBRARY_COMPLIANT` at `test_import_rule.py:549-552`. A control with
no `use` statement proves only that nothing was rejected — a corrupted anchor
(`^crate` misspelt, the `^` dropped, `^(std|core|alloc)` narrowed to `^std$`)
would pass the violation case, the control and `pup:check` alike. The rule must
be shown to permit as well as to deny.

Note what this does *not* catch: swapping an alternation's order
(`^crate($|::)` for `^crate(::|$)`) changes nothing, because the two regexes
accept the same strings. Use one of the corruptions above when demonstrating
the control's discriminating power.

```python
# --- The tracker domain rule ---
#
# Driven against a workspace whose crates are literally named `tracker` and
# `work`, so the shipped `^tracker($|::)` regex is exercised directly. The
# violation is the one the crate exists to prevent: a port crate reaching for
# the lifecycle domain would make every provider client depend on it
# transitively.

_TRACKER_WORKSPACE = """\
[workspace]
resolver = "2"
members = ["tracker", "work"]
"""

_TRACKER_MANIFEST = """\
[package]
name = "tracker"
version = "0.0.0"
edition = "2021"
license = "MIT"

[lib]
path = "src/lib.rs"

[dependencies]
work = { path = "../work" }
"""

_WORK_MANIFEST = """\
[package]
name = "work"
version = "0.0.0"
edition = "2021"
license = "MIT"

[lib]
path = "src/lib.rs"
"""

_TRACKER_LIB = "pub mod port;\n\npub struct Marker;\n"
_WORK_LIB = "pub struct WorkItem;\n"

_TRACKER_PORT_VIOLATION = (
    "use work::WorkItem;\n\npub fn make() -> WorkItem {\n    WorkItem\n}\n"
)
_TRACKER_PORT_COMPLIANT = (
    "use std::path::Path;\n\n"
    "use crate::Marker;\n\n"
    "pub fn make(path: &Path) -> (usize, Marker) {\n"
    "    (path.as_os_str().len(), Marker)\n"
    "}\n"
)

def _write_tracker_probe(root: Path, port_body: str) -> None:
    (root / "Cargo.toml").write_text(_TRACKER_WORKSPACE)

    tracker_src = root / "tracker/src"
    tracker_src.mkdir(parents=True, exist_ok=True)
    (root / "tracker/Cargo.toml").write_text(_TRACKER_MANIFEST)
    (tracker_src / "lib.rs").write_text(_TRACKER_LIB)
    (tracker_src / "port.rs").write_text(port_body)

    work_src = root / "work/src"
    work_src.mkdir(parents=True, exist_ok=True)
    (root / "work/Cargo.toml").write_text(_WORK_MANIFEST)
    (work_src / "lib.rs").write_text(_WORK_LIB)

def test_real_tracker_rule_rejects_importing_the_work_domain(
    tmp_path: Path,
) -> None:
    _require_tools()
    _write_tracker_probe(tmp_path, _TRACKER_PORT_VIOLATION)
    result = _pup("--pup-config", str(CLI_PUP_RON), cwd=tmp_path)
    output = _ANSI.sub("", result.stdout + result.stderr)
    assert result.returncode != 0, output
    assert "is not allowed" in output, output
    assert "tracker_domain_imports_only_permitted" in output, output

def test_real_tracker_rule_permits_std_and_crate_imports(
    tmp_path: Path,
) -> None:
    _require_tools()
    _write_tracker_probe(tmp_path, _TRACKER_PORT_COMPLIANT)
    result = _pup("--pup-config", str(CLI_PUP_RON), cwd=tmp_path)
    assert result.returncode == 0, _ANSI.sub("", result.stdout + result.stderr)
```

The control's name says what a green run establishes. `_TRACKER_PORT_COMPLIANT`
imports one `std::` path and one `crate::` path, so both `allowed_only` anchors
must match for it to pass — which is what makes a typo in either one visible.

#### 5. Fixtures

**File**: `cli/tracker/tests/fixtures/dispatch-codes.txt` (new)
**Changes**: all four codes with their port mapping, preceded by the reasoning
0204 requires the fixture to record. Format is deliberately flat so parsing
needs only `std`; the reader skips blank lines and lines whose first non-space
character is `#`.

```
# The bash dispatch taxonomy, held against TrackerError's two classes.
#
# 70 and 71 are the two classes the port expresses. 72 and 73 are
# dispatch-routing outcomes that resolve ABOVE the port, at the composition
# root selecting Box<dyn RemoteTracker> from work.integration: if the config
# names a tracker with no client there is no port call to make.
#
# This pins the taxonomy's membership. Which class a given wire condition
# maps to is operation-scoped — see TrackerError's doc comment.
E_DISPATCH_RETRYABLE=70 Retryable
E_DISPATCH_TERMINAL=71 Terminal
E_DISPATCH_NOT_AVAILABLE=72 above-the-port
E_DISPATCH_UNRECOGNISED=73 above-the-port
```

**File**: `cli/tracker/tests/fixtures/remote-updated-at.txt` (new)
**Changes**: one real stamp per provider, one per line. The Linear value comes
from `.accelerator/state/integrations/linear/last-sync.json` (a tracked,
bash-written baseline); the Jira value from
`skills/integrations/jira/scripts/test-fixtures/scenarios/apply-push-204-show.json:17`.
Committing only the Linear format would leave the `+0000` shape — the one the
work item is most worried about — untested.

```
2026-06-21T00:06:10.647Z
2026-07-09T08:00:00.000+0000
```

#### 6. Vocabulary and error tests (red)

Written before `src/lib.rs` exists, so the first run fails to compile with
`error[E0432]: unresolved import` / `can't find crate for 'tracker'`. That is
the red; §7 is what turns it green.

**File**: `cli/tracker/tests/vocabulary.rs` (new)

```rust
//! The value types hold their bytes and nothing else: construction and
//! read-back are lossless, and comparison is exact.

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;

type TestError = Box<dyn Error>;

fn stamps() -> Result<Vec<String>, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/remote-updated-at.txt");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    Ok(text.lines().map(str::to_owned).collect())
}

#[test]
fn the_fixture_covers_both_incompatible_provider_formats(
) -> Result<(), TestError> {
    let stamps = stamps()?;
    assert!(
        stamps.iter().any(|stamp| stamp.ends_with('Z')),
        "no Linear-shaped stamp in the fixture: {stamps:?}"
    );
    assert!(
        stamps.iter().any(|stamp| stamp.contains("+0000")),
        "no Jira-shaped stamp in the fixture: {stamps:?}"
    );
    assert!(
        stamps.iter().all(|stamp| !stamp.trim().is_empty()),
        "a blank fixture row round-trips trivially: {stamps:?}"
    );
    Ok(())
}

#[test]
fn every_committed_stamp_survives_a_round_trip_byte_identically(
) -> Result<(), TestError> {
    for stamp in stamps()? {
        let issue = RemoteIssue {
            updated: RemoteTimestamp::new(stamp.clone()),
            body: String::new(),
        };
        assert_eq!(
            issue.updated.as_str(),
            stamp,
            "stamp {stamp} did not survive the round trip"
        );
    }
    Ok(())
}

#[test]
fn stamps_differing_only_in_whitespace_compare_unequal() {
    let stamp = RemoteTimestamp::new("2026-06-21T00:06:10.647Z".to_owned());
    let padded = RemoteTimestamp::new("2026-06-21T00:06:10.647Z ".to_owned());
    assert_ne!(stamp, padded);
}

#[test]
fn the_empty_stamp_is_a_legal_value() {
    assert_eq!(RemoteTimestamp::new(String::new()).as_str(), "");
}

#[test]
fn two_unknown_stamps_compare_equal_and_must_not_be_read_as_unchanged() {
    // The trap this pins is the derived `PartialEq`, not a bug: both empty
    // stamps mean "unknown", so a caller comparing them learns nothing.
    let unknown = RemoteTimestamp::new(String::new());
    assert_eq!(unknown, RemoteTimestamp::new(String::new()));
    assert!(unknown.as_str().is_empty(), "check this before comparing");
}

#[test]
fn an_external_id_returns_the_bytes_it_was_given() {
    let id = ExternalId::new("ENG-1".to_owned());
    assert_eq!(id.as_str(), "ENG-1");
}

#[test]
fn an_external_id_displays_without_reaching_for_its_bytes() {
    let id = ExternalId::new("ENG-1".to_owned());
    assert_eq!(format!("{id}"), "ENG-1");
}

#[test]
fn external_ids_key_a_map() {
    let mut index = HashMap::new();
    index.insert(ExternalId::new("ENG-1".to_owned()), "local-0204");
    assert_eq!(
        index.get(&ExternalId::new("ENG-1".to_owned())),
        Some(&"local-0204")
    );
}
```

`the_fixture_covers_both_incompatible_provider_formats` guards the guard: the
round-trip loop asserts nothing when the fixture is empty, so an edit to a data
file could otherwise disable the crate's most load-bearing invariant silently.
The `Display` and `HashMap` tests exist because both are deviations from the
original frozen block (deviation 6) — an unused derive is one a later reader
deletes.

**File**: `cli/tracker/tests/errors.rs` (new)

```rust
//! The error taxonomy: two classes, closed, held 1:1 against the bash
//! dispatch codes that remain authoritative until the bridges are retired.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use tracker::TrackerError;

type TestError = Box<dyn Error>;

const ABOVE_THE_PORT: &str = "above-the-port";

fn read(relative: &str) -> Result<String, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()).into())
}

/// The class a dispatch code maps onto, as the fixture records it.
#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    Class(String),
    AboveThePort,
}

#[derive(Debug)]
struct DispatchCode {
    number: String,
    resolution: Resolution,
}

/// The name and numeric value one shell line declares, if it declares one.
fn declaration(line: &str) -> Option<(String, String)> {
    let at = line.find("E_DISPATCH_")?;
    let (name, rest) = line[at..].split_once('=')?;
    let value = rest.trim().trim_matches(|c| c == '"' || c == '\'');
    let number: String =
        value.chars().take_while(char::is_ascii_digit).collect();
    if number.is_empty() {
        return None;
    }
    Some((name.trim().to_owned(), number))
}

fn codes_declared_by_the_bash_taxonomy(
) -> Result<BTreeMap<String, String>, TestError> {
    let script = read("../../skills/work/scripts/work-item-bridge-codes.sh")?;
    let mut declared = BTreeMap::new();
    for line in script.lines().map(str::trim) {
        if line.starts_with('#') || !line.contains("E_DISPATCH_") {
            continue;
        }
        let (name, number) = declaration(line).ok_or_else(|| {
            format!("could not read a dispatch declaration from: {line}")
        })?;
        declared.insert(name, number);
    }
    Ok(declared)
}

fn codes_recorded_by_the_fixture(
) -> Result<BTreeMap<String, DispatchCode>, TestError> {
    let fixture = read("tests/fixtures/dispatch-codes.txt")?;
    fixture
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (declaration, resolution) = line
                .split_once(' ')
                .ok_or_else(|| format!("malformed fixture row: {line}"))?;
            let (name, number) = declaration
                .split_once('=')
                .ok_or_else(|| format!("malformed fixture row: {line}"))?;
            let resolution = if resolution == ABOVE_THE_PORT {
                Resolution::AboveThePort
            } else {
                Resolution::Class(resolution.to_owned())
            };
            Ok((
                name.to_owned(),
                DispatchCode {
                    number: number.to_owned(),
                    resolution,
                },
            ))
        })
        .collect()
}

/// The variant name `Debug` prints, which is the identifier itself — so a
/// rename propagates here instead of being absorbed by a match arm.
fn class_of(error: &TrackerError) -> String {
    let rendered = format!("{error:?}");
    rendered
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn retryable() -> TrackerError {
    TrackerError::Retryable {
        detail: String::new(),
    }
}

fn terminal() -> TrackerError {
    TrackerError::Terminal {
        detail: String::new(),
    }
}

#[test]
fn the_fixture_enumerates_exactly_the_codes_the_bash_taxonomy_declares(
) -> Result<(), TestError> {
    let declared = codes_declared_by_the_bash_taxonomy()?;
    let recorded: BTreeMap<String, String> = codes_recorded_by_the_fixture()?
        .into_iter()
        .map(|(name, code)| (name, code.number))
        .collect();
    assert_eq!(
        recorded, declared,
        "the bash dispatch taxonomy and tests/fixtures/dispatch-codes.txt \
         disagree — update the fixture deliberately, and check whether \
         TrackerError's two classes still cover it. A reformatted declaration \
         in work-item-bridge-codes.sh reads the same way here as a changed one."
    );
    Ok(())
}

#[test]
fn each_dispatch_code_maps_onto_the_class_it_names() -> Result<(), TestError> {
    let recorded = codes_recorded_by_the_fixture()?;
    for (name, expected) in [
        ("E_DISPATCH_RETRYABLE", retryable()),
        ("E_DISPATCH_TERMINAL", terminal()),
    ] {
        let code = recorded
            .get(name)
            .ok_or_else(|| format!("the fixture does not record {name}"))?;
        assert_eq!(
            code.resolution,
            Resolution::Class(class_of(&expected)),
            "{name} maps onto the wrong TrackerError class"
        );
    }
    Ok(())
}

#[test]
fn exactly_two_dispatch_codes_reach_the_port() -> Result<(), TestError> {
    let recorded = codes_recorded_by_the_fixture()?;
    let mapped = recorded
        .values()
        .filter(|code| matches!(code.resolution, Resolution::Class(_)))
        .count();
    assert_eq!(
        mapped, 2,
        "the port expresses two classes; every other code must be recorded as \
         resolving above it"
    );
    Ok(())
}

#[test]
fn each_class_routes_to_a_distinct_outcome() {
    // A closed-set guard: fails to compile if a variant is added or removed
    // without this match arm list moving with it.
    let outcome = |error: TrackerError| match error {
        TrackerError::Retryable { .. } => "retry",
        TrackerError::Terminal { .. } => "surface",
    };
    assert_eq!(
        outcome(TrackerError::Retryable {
            detail: String::new()
        }),
        "retry"
    );
    assert_eq!(
        outcome(TrackerError::Terminal {
            detail: String::new()
        }),
        "surface"
    );
}

#[test]
fn a_tracker_error_is_usable_as_a_std_error() {
    let boxed: Box<dyn Error> = Box::new(retryable());
    assert!(boxed.source().is_none());
}

#[test]
fn a_retryable_failure_says_nothing_changed_remotely() {
    assert_eq!(
        TrackerError::Retryable {
            detail: "linear: create ENG-2 failed, connection refused"
                .to_owned()
        }
        .to_string(),
        "tracker call failed with no remote change: linear: create ENG-2 \
         failed, connection refused"
    );
}

#[test]
fn a_terminal_failure_says_the_remote_state_is_unknown() {
    assert_eq!(
        TrackerError::Terminal {
            detail: "jira: create PROJ-? failed, response lost".to_owned()
        }
        .to_string(),
        "tracker call failed and a remote change may have applied, so the \
         remote state is unknown: jira: create PROJ-? failed, response lost"
    );
}
```

`each_class_routes_to_a_distinct_outcome` carries the only inline comment in the
crate's tests. It survives the repo's low comment tolerance for the reason
`cli/vcs/src/classify.rs:585-586` does: without it the body reads as a no-op.
The separate closed-set test the first draft carried is dropped — it asserted a
strict subset of what this one does.

Two things here were got wrong in the previous draft and are worth stating so
they are not reintroduced.

`class_of` reads the variant name out of `Debug` output rather than returning a
literal from a match arm. A match returning `"Retryable"` looks like derivation
but is not: renaming the variant changes only the *pattern*, leaves the literal
standing, and the test stays green. `Debug` prints the identifier, so a rename
propagates.

The mapping is asserted per code, not as a set. Comparing sorted class names
proves only that the fixture uses both names — swapping the two rows so that
70 reads `Terminal` and 71 reads `Retryable` leaves the multiset unchanged and
passes. That inversion is the most damaging error the fixture could carry: a
client told 70 is terminal would refuse to retry a call that provably never
left the machine, and would retry a create that may have duplicated an issue.
`each_dispatch_code_maps_onto_the_class_it_names` pins the pairing;
`exactly_two_dispatch_codes_reach_the_port` keeps the closed-set guard.

#### 7. The vocabulary types (green)

**File**: `cli/tracker/src/lib.rs` (new)
**Changes**: a single file. The crate is small enough that one file is the
natural home — `cli/vcs/src/lib.rs` holds its types directly for the same
reason — and it keeps AC 10's "no `#[cfg(test)]` module in `tracker/src/`"
trivial to police. The Phase 2 surface test does not depend on it: that test
walks `src/` recursively, so splitting a module out later narrows nothing.

```rust
//! The port a remote issue tracker satisfies, and the vocabulary it speaks.
//!
//! The provider clients that implement it and the sync engine that calls it
//! both live elsewhere; this crate is the seam between them and holds no
//! logic. It deliberately has no `-adapters` sibling.

use std::fmt::Display;
use std::fmt::Formatter;

/// The identifier a remote tracker gave an issue.
///
/// The same value the local work item carries in its `external_id`
/// frontmatter field, taken as opaque: the port does not parse, validate or
/// interpret the string.
///
/// Opaque to the port is not opaque to the client. The value is written
/// unquoted into a work item's YAML frontmatter, so an implementation must
/// reject an identifier it cannot safely persist — control characters, a
/// newline, a leading `---` or `#` — rather than returning it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalId(String);

impl ExternalId {
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ExternalId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A tracker's own last-modified stamp, held verbatim.
///
/// A cache key, not a clock: an unequal stamp means the body must be
/// re-hashed, never that the remote is newer. Hence no `PartialOrd` or `Ord`,
/// and no conversion surface beyond construction and read-back.
///
/// The bytes must survive unchanged — providers emit mutually incompatible
/// formats (see `tests/fixtures/remote-updated-at.txt` for the committed set),
/// and a date-library round-trip would rewrite a numeric offset, reclassifying
/// every item whose baseline the bash sync path already wrote.
///
/// The empty string is a legal value with two sources: a tracker that reports
/// no timestamp for an issue, and a post-push read that failed. `new`
/// therefore validates nothing. Both mean *unknown*.
///
/// Beware the consequence: `==` reports two empty stamps as equal, and that
/// must not be read as "unchanged". Check for emptiness before comparing, as
/// the sync classifier does — comparing two unknowns and concluding a match
/// classifies an item whose baseline was never written as already synced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTimestamp(String);

impl RemoteTimestamp {
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a tracker reports about one issue, in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteIssue {
    /// The tracker's own last-modified stamp, stored as `remote_updated_at`
    /// in the sync baseline. The two names refer to one value.
    pub updated: RemoteTimestamp,
    /// The already-projected domain body: the issue's title line, then its
    /// description, with no blank line between them and a trailing newline.
    /// A structured description is canonicalised first — key-sorted and
    /// compact — so equal content hashes equally; a Markdown one is carried
    /// verbatim.
    ///
    /// An absent description is where the two providers diverge and where a
    /// client is most likely to guess wrong: a structured one projects as the
    /// literal token `null`, a Markdown one as an empty line. Neither is
    /// inferable from a JSON deserialiser's natural output, and either wrong
    /// choice reclassifies every such item.
    ///
    /// The value is the *un-normalised* projection. The caller normalises
    /// before hashing.
    ///
    /// This is **not** the body a caller supplies when pushing: it carries the
    /// title line as well, so a push followed by a read is not the identity.
    ///
    /// Projection sits behind the port, so reproducing the recipe exactly is
    /// the implementing client's obligation. A body differing by so much as
    /// whitespace reclassifies every synced item as remotely modified, and an
    /// interior blank line survives normalisation. The bash recipe
    /// (`work-item-project-remote.sh`) is the current reference
    /// implementation; the contract above outlives it.
    pub body: String,
}

/// A failure reported by a remote tracker.
///
/// Two classes, closed deliberately: `#[non_exhaustive]` is absent so that
/// adding a third is a compile-breaking change for every consumer, which is
/// the property both consumers want.
///
/// The classes divide on one question: **could a remote change have
/// happened?** That makes classification operation-scoped, not a property of
/// the wire condition — the same provider status falls either way depending on
/// what was attempted, so a client must classify per call rather than from one
/// status table. A read cannot mutate, so a read never produces `Terminal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerError {
    /// No remote change occurred, provably.
    ///
    /// For a mutating call the test is the *mutation*, not the transmission:
    /// a request that never left the machine qualifies, and so does one the
    /// tracker received and rejected before applying anything.
    ///
    /// The test is provability, not a list of statuses. A rejection qualifies
    /// only where the provider's protocol makes it provable, and that varies
    /// by operation as well as by provider: the same wire condition can be
    /// provable on `create` and unprovable on `update` against one tracker.
    /// A single status-to-class table is therefore wrong — classify per
    /// operation, and when in doubt use `Terminal`.
    ///
    /// For a read it is the only class, because there was nothing to mutate;
    /// the caller degrades rather than repeating blindly.
    Retryable {
        /// What failed, for a human reading a sync report. State the
        /// provider, the operation, the external id where one is known, and
        /// the underlying status or exit code.
        detail: String,
    },
    /// A remote change may have happened, and which is unknowable.
    ///
    /// The conservative default **for mutating calls**: a failure belongs in
    /// `Retryable` only when the absence of a remote change is *provable*, so
    /// a lost or unparseable response, a 5xx, or a connection dropped after
    /// the request went out all belong here — the tracker may have applied it.
    /// Reads never produce this class.
    Terminal {
        /// What failed, in the same shape `Retryable` asks for, and
        /// additionally whether a remote mutation may have applied — that is
        /// what the reader has to act on.
        detail: String,
    },
}

impl Display for TrackerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retryable { detail } => write!(
                formatter,
                "tracker call failed with no remote change: {detail}"
            ),
            Self::Terminal { detail } => write!(
                formatter,
                "tracker call failed and a remote change may have applied, so \
                 the remote state is unknown: {detail}"
            ),
        }
    }
}

impl std::error::Error for TrackerError {}
```

Both `Display` messages state **mutation safety**, not a retry instruction, and
the distinction took two attempts to get right. The first draft said "failed
unrecoverably" for `Terminal`, which tells a user the opposite of what the class
means — the bash bridge says "an issue may exist — do NOT retry"
(`work-item-create-remote.sh:243-245`). The second draft fixed that arm but made
`Retryable` assert "the request was sent before it failed, so it is safe to
retry", which is false for a read: every read failure is `Retryable`, including
a permanently deleted issue that `show`'s own doc warns against looping on. A
message stating what did or did not change is true for both operation classes;
one prescribing a retry is not. These strings reach a user through the sync
report.

#### 8. The structural guards `cargo public-api` cannot give

**File**: `cli/tracker/tests/structure.rs` (new)

Three of the crate's invariants are invisible to rustdoc JSON: a `#[cfg(test)]`
module is not public API and is not compiled for rustdoc; a manifest is not code
at all; and a workspace member list is another file entirely. All three are
criteria — AC 10's no-test-module half, AC 9's absent dependency tables, and
AC 10's absent `tracker-adapters` member — and the last two had no guard of any
kind once the textual golden went away. This file is all three, and it costs
little: it is already reading files under `CARGO_MANIFEST_DIR`.

It lands in Phase 1 because that is where `src/lib.rs` and `Cargo.toml` ship,
and the phases are independently mergeable.

```rust
//! The crate ships no behaviour and no dependencies, and has no adapter
//! sibling. None of that is visible to rustdoc JSON, so it is checked here.

use std::error::Error;
use std::path::PathBuf;

type TestError = Box<dyn Error>;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sources() -> Result<Vec<PathBuf>, TestError> {
    let mut paths = Vec::new();
    let mut pending = vec![manifest_dir().join("src")];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                paths.push(path);
            }
        }
    }
    if paths.is_empty() {
        return Err("found no sources under src/".into());
    }
    paths.sort();
    Ok(paths)
}

fn declares_a_test_module(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("#[cfg("))
        .any(|line| line.contains("test"))
}

#[test]
fn the_crate_declares_no_test_module() -> Result<(), TestError> {
    for path in sources()? {
        let source = std::fs::read_to_string(&path)?;
        assert!(
            !declares_a_test_module(&source),
            "{} declares a test module; the crate's tests live in tests/",
            path.display()
        );
    }
    Ok(())
}

#[test]
fn the_manifest_declares_no_dependencies() -> Result<(), TestError> {
    let manifest = std::fs::read_to_string(manifest_dir().join("Cargo.toml"))?;
    for table in ["[dependencies]", "[dev-dependencies]"] {
        assert!(
            !manifest.contains(table),
            "tracker declares {table}; the crate's whole value is that a \
             client of this port acquires nothing else with it"
        );
    }
    Ok(())
}

#[test]
fn the_workspace_lists_no_adapter_sibling() -> Result<(), TestError> {
    let workspace = manifest_dir().join("..").join("Cargo.toml");
    let manifest = std::fs::read_to_string(&workspace)?;
    assert!(
        !manifest.contains("tracker-adapters"),
        "the port is deliberately the workspace's first domain crate with no \
         -adapters sibling; provider clients live in their own crates"
    );
    Ok(())
}
```

Matching `#[cfg(` … `test` rather than the literal `#[cfg(test)]` catches
`#[cfg(all(test, …))]` and rustfmt-spaced variants, and reading only
attribute-prefixed lines stops the string inside a doc comment false-positiving.
The recursive walk keeps the guard from narrowing if the crate grows a module in
directory form; the empty-result error stops a broken walk passing vacuously.

AC 10's remaining half — no function body beyond the named ones — stays a manual
check, because counting permitted bodies mechanically would pin more than the
criterion means.

#### 9. The add-a-library-crate checklist

**File**: `tasks/README.md`
**Changes**: a new top-level `## Registering a library crate` section, placed
**after** the whole existing `## Registering a dispatched sub-binary` section
(i.e. after its trailing prose, before `## CI job → local command`).

`tasks/README.md:458-459` says a crate carrying domain modules "may also owe a
`cli/pup.ron` rule; that is the generic add-a-Rust-crate surface, not part of
this checklist" — and that generic surface is documented nowhere. This plan's
central discovery is that enforcement is membership-derived *except* cargo-pup,
so a new crate ships with no architectural enforcement until someone writes its
rule by hand and nothing notices the omission. Writing that down is what stops
the hazard reopening on the next library crate.

**The placement is load-bearing, not stylistic.**
`tests/unit/tasks/test_registration_docs.py` slices from
`## Registering a dispatched sub-binary` to the **next `## ` heading**, splits
the slice on `^\d+\. `, and asserts exactly thirteen items each carrying one of
`**[PR]**`/`**[release]**`/`**[author]**`. A `###` subsection with numbered
steps inside that region breaks the count; a `##` inserted before the existing
section's trailing prose strips the names that guard asserts on. Appending a
sibling section after the whole thing leaves the slice untouched. Use bullets
rather than a numbered list so the item-splitting regex cannot reach it either.

```markdown
## Registering a library crate

A plain library crate — no dispatch token, no binary, no launcher wiring — owes
four things, plus a fifth when other crates build against its surface.
`cli/tracker/` is the worked example.

- **Workspace membership.** Add the directory to `[workspace].members` in
  `cli/Cargo.toml`, then sync the lockfile with `cargo metadata
  --manifest-path cli/Cargo.toml --format-version 1` (the minimal update, never
  `cargo generate-lockfile`). Clippy runs `--locked`, so an unsynced lockfile
  surfaces as an unrelated clippy failure.
- **Inherited manifest fields.** `version`, `edition`, `rust-version`,
  `license` and `publish` are all `.workspace = true`, and `[lints] workspace =
  true` opts the crate into the shared pedantic/nursery set. A hardcoded
  version passes the coherence check today and breaks at the next bump; a
  missing `[lints]` table silently exempts the crate from every lint the rest
  of the workspace is held to.
- **A `cli/pup.ron` rule.** Nothing derives architectural enforcement from
  membership, so a crate without a rule has none and no check reports it
  missing.
- **A probe pair** in `tests/integration/pup/test_import_rule.py`, driving the
  shipped `cli/pup.ron` against a synthetic workspace named for the crate: a
  violation case and a compliant control that imports something the permit list
  must admit. There is no coverage guard for `pup.ron`, so a rule deleted or
  mistyped is otherwise silent, and a control with no imports proves only that
  nothing was rejected.
- **A public-API snapshot**, when the crate's surface is one other crates build
  against. `public-api:check` names each crate explicitly, so a new crate is
  exempt from the surface pin until `tasks/public_api.py` learns it. The
  snapshot lives at `<crate>/tests/fixtures/public-api.txt` and is regenerated
  with `mise run public-api:update`.

Then run `mise run deny:check`.
```

Three index sentences point at `tasks/README.md` and describe it as carrying one
checklist; each needs a few words so the new section is discoverable by the
reader it is written for — a session that reads `CLAUDE.md` first.

- `tasks/README.md:6-7` — "carries **the** checklist for registering a
  dispatched sub-binary" becomes "carries the checklists for registering a
  dispatched sub-binary and a library crate".
- `tasks/CLAUDE.md` — its pointer at
  `tasks/README.md#registering-a-dispatched-sub-binary` gains the sibling
  anchor. **Additive only**: `test_registration_docs.py`'s `_RESOLVES` asserts
  that exact string still appears, so append rather than rewrite.
- The repo-root `CLAUDE.md` — a different sentence ("carries the
  **thirteen-point** checklist…"), so it needs its own wording that keeps
  "thirteen-point" bound to the dispatch checklist.

`tasks/README.md:458-459` also declines to document this surface ("that is the
generic add-a-Rust-crate surface, not part of this checklist"). Repoint it at
the new section, or the README declares the surface undocumented two paragraphs
above the section documenting it.

#### 10. The `cargo-public-api` lane

**Files**: `tasks/shared/rust.py`, `tasks/deps.py`, `tasks/public_api.py` (new),
`tasks/__init__.py`, `mise.toml`, `.github/workflows/main.yml`,
`tests/unit/tasks/test_workflows.py`, `tests/unit/tasks/test_mise.py`,
`tests/unit/tasks/test_deps.py`, `tests/unit/tasks/test_rust.py`,
`tasks/README.md`

The surface pin needs a tool. It mirrors the cargo-pup lane step for step,
because that lane already solved the same problems — a pinned nightly, an
install with a presence probe, and confinement to one CI job.

**The name.** `public-api:check`, backed by `tasks/public_api.py`. The two
existing standalone Rust gates are named for the tool that implements them
(`deny:check` → cargo-deny, `pup:check` → cargo-pup), and `api:` would both
break that and collide with the visualiser's HTTP API, which `mise.toml:173`
already proxies. It also keeps one spelling across the lane, beside
`deps:install:public-api` and `PUBLIC_API_VERSION`.

**The pin.** `PUBLIC_API_VERSION` joins `tasks/shared/rust.py` beside
`PUP_NIGHTLY`/`PUP_VERSION`. It is *not* a matched pair with the nightly in the
way cargo-pup's is: cargo-public-api has no `rustc_private` driver, so it builds
on stable and only shells out to nightly `rustdoc`. Reuse `PUP_NIGHTLY` for that
rustdoc invocation, and **update that constant's comment** — it currently
records a single consumer. If the pinned nightly's rustdoc-JSON format turns out
to be outside the installed tool's supported range, introduce a separate
`PUBLIC_API_NIGHTLY` rather than dragging `PUP_NIGHTLY` to satisfy two
upstreams — cargo-pup's pin is the more constrained of the two and must not be
moved for this.

**Provisioning.** Prefer a `mise.toml [tools]` pin resolved through an
`aqua:`/`ubi:` backend with a regenerated `mise.lock`, which is how every
third-party binary here is pinned *except* cargo-pup — and cargo-pup is the
exception only because a `cargo:` backend would build it against stable and it
would fail to load, which does not apply to a tool with no driver.
`mise.toml:41-43` records that from-source build as "an accepted unverified
surface", singular; adding a second needs its own justification.

If no publishable binary exists, fall back to `deps:install:public-api` modelled
on `deps.install_pup` (`tasks/deps.py:52-100`): a presence probe on
`cargo public-api --version` doing whole-token equality against
`PUBLIC_API_VERSION` (substring matching would false-match `0.4.10` against
`0.4.1`), then `cargo install cargo-public-api --version <pin> --locked`. Follow
that task's failure contract too — `warn=True, pty=False` on each step, then
`raise Exit(...)` naming the pin — which
`test_deps.py::test_nightly_install_failure_reraised_as_exit_naming_pin` pins
deliberately. Record the second accepted exception in `mise.toml`'s settings
comment and in `tasks/README.md`.

**Confirm the minimal profile ships `rustdoc`** — `deps:install:pup` installs
with `--profile minimal` plus three components, none of which is rustdoc. If it
is absent, add the component there *and* extend
`test_deps.py::_PUP_COMPONENTS`, which pins that set.

**The check.** `tasks/public_api.py`, mirroring `tasks/pup.py`: run the tool
from `cli/`, capture stdout, compare against the committed snapshot, and fail
via `raise Exit(...)` naming the snapshot path and `mise run public-api:update`.
The invocation is approximately

```
cargo public-api --toolchain <nightly> \
  --omit blanket-impls,auto-trait-impls -p tracker
```

The `--omit` list is load-bearing. Default output carries every blanket and
auto-trait impl — `impl Send for ExternalId`, `impl<T, U> Into<U> for T where U:
From<T>`, and dozens more — which no one can transcribe from 0204's block, so
the snapshot could only ever be captured from the implementation. That is the
characterisation snapshot deviation 8 exists to avoid. Omitting those two
categories leaves the items and their *derive-generated* impls, which is exactly
what the freeze needs and is genuinely hand-writable. Do **not** reach for
`--simplified` instead: it additionally omits `auto-derived-impls`, discarding
the derive coverage this tool was chosen for.

Fail the check when the snapshot is missing or empty rather than writing it —
a check that regenerates on absence retires the freeze the first time someone
deletes the file.

**Regeneration.** `public-api:update` writes the snapshot from the same
invocation, sharing one command-building helper with the check so the two cannot
drift. Without it, the only workflow the lane exists to support after the first
commit — a deliberate, reviewed surface change — has no path, and a
hand-reconstructed command that differs by a flag produces a snapshot that never
goes green.

**Wiring.** Both tasks get a `description` (every one of ~100 tasks in
`mise.toml` has one; `mise tasks` is the discovery surface).
`public-api:check` takes `depends = ["deps:install:public-api"]`, joins the
aggregate `check` and the bare `default` beside `pup:check`, and gets a step in
the `check-architecture` CI job. That job's comment "This is the ONLY nightly
consumer" becomes "the only nightly-consuming job" — two tasks now share it.

**Registration.** `tasks/__init__.py` builds the invoke namespace by hand: every
module appears both in the alphabetical `from . import (...)` block and in an
`ns.add_collection(Collection.from_module(...))` line. Without both, `mise run
public-api:check` fails with "No idea what 'public-api.check' is!" — and since
the task joins `check` and `default`, that breaks every local run.

**Three guards must learn the new tasks.** Each exists to stop exactly the drift
this change could introduce:

- `test_workflows.py:356` — `_NIGHTLY_MARKERS` gains `public-api:check` and
  `deps:install:public-api`, so a future leak into a stable-lane job is caught.
  Its positive half (`:386-390`, asserting `check-architecture` runs both pup
  tasks) gains a matching assertion for the new step, so the CI step cannot be
  dropped silently.
- `test_mise.py:18` — `_CHECK_GATES` gains `public-api:check`. Its stated
  purpose is that "a gate cannot be silently unwired from the read-only
  CI-mirror", and the surface pin is the plan's single most load-bearing check.
- `test_deps.py` / `test_rust.py` — the new install and check tasks get the
  same leaf-branch coverage their cargo-pup equivalents have, including the
  version-probe case the pin's own comment calls out (`0.4.1` must not match
  `0.4.10`) and a missing-snapshot case.

**Also update the docs that describe this lane as cargo-pup-only.** Five
committed statements become false:

- `tasks/README.md:43-47` — the standalone-gate enumeration.
- `tasks/README.md:257-291` — the `### Rust nightly lane (cargo-pup)` section
  ("Only `pup:check` and `test:integration:pup` consume it"), including its
  bumping-the-pin bullet, which must gain a step: after a `PUP_NIGHTLY` bump,
  re-verify `PUBLIC_API_VERSION` against the new nightly's rustdoc-JSON format,
  regenerate the snapshot, and read the diff as toolchain-induced before
  accepting it.
- `tasks/README.md:476-486` — the CI-job table, whose rule is that each CI job
  mirrors a single `mise run` task.
- The repo-root `CLAUDE.md:35-36` — "Rust enforcement beyond `cli:check`
  (cargo-deny, cargo-pup)". This is the always-loaded orientation file, so a
  session that reads it will not otherwise learn the gate exists.
- `tests/unit/tasks/test_mise.py`'s module docstring, edited in the same change
  as `_CHECK_GATES`.

#### 11. The public-API snapshot, first instalment

**File**: `cli/tracker/tests/fixtures/public-api.txt` (new)

Hand-written from 0204's amended Requirements block **before** `src/lib.rs`
exists, covering the four vocabulary items this phase ships. It sits under
`tests/fixtures/` with the crate's other committed data — every such artefact in
the workspace does, and 0204's AC 1 says so — rather than at the crate root,
which is cargo-public-api's own convention but would be this workspace's only
root-level data file. The file is read by a Python task, not by cargo, so the
tool's layout is not binding.

The file has two halves, and only one of them is hand-writable. Say so plainly,
because an earlier draft claimed the whole thing was and that claim would have
collapsed on first run.

**Hand-written, from 0204's block**: the item declarations, their fields and
variants, the four trait method signatures, and the `impl <Trait> for <Type>`
lines the derives generate. These are exactly the contract, and they are what
must start red — a mistyped signature or a dropped derive shows up here.

**Captured, once**: the *methods* those derives generate — `pub fn
tracker::ExternalId::clone(&self) -> tracker::ExternalId`, `hash<__H:
core::hash::Hasher>(&self, state: &mut __H)`, marker impls like
`StructuralPartialEq`, and `core::error::Error` where 0204 writes
`std::error::Error`. Their names and paths are chosen by the derive expansion
and the tool's renderer, not by 0204, so they cannot be predicted — roughly
forty lines for six items. Capturing them costs nothing in freeze strength:
adding `PartialOrd` later still appears as new impl *and* method lines against
the committed file.

So: **run the tool once against an existing workspace crate before authoring**,
to learn the rendering conventions (path qualification, derive-method spelling,
which marker impls appear). Then hand-write the contract half, run
`public-api:check`, and reconcile. A difference in the hand-written half is a
mismatch to resolve in favour of 0204's block; a difference in the captured half
is transcription to accept. Knowing which half a line belongs to is what keeps
this a pin rather than a snapshot.

One thing the file cannot carry at all: `#[must_use]` is an attribute rather
than API shape, so it does not appear. Those four attributes stay a manual read.

Phase 2 extends this file with `FetchOutcome` and `RemoteTracker`.

### Success Criteria

#### Automated Verification

- [x] `tracker` compiles as a workspace member: `mise run cli:check`
- [x] The snapshot was hand-written from 0204's amended block and was red before
      `src/lib.rs` existed
- [x] `mise run public-api:check` passes, and the four vocabulary items plus
      their derive-generated impls appear in
      `cli/tracker/tests/fixtures/public-api.txt`
- [x] `mise run public-api:update` regenerates that file byte-identically —
      check and update share one invocation, so they cannot drift
- [x] The structural guards pass: `cd cli && cargo nextest run -p tracker`
      covers no test module, no dependency tables, no adapter sibling
- [x] `mise run test:unit:tasks` passes — §10 edits three guards
      (`test_workflows.py`, `test_mise.py`, and the new task coverage in
      `test_deps.py`/`test_rust.py`) and §9 edits a region
      `test_registration_docs.py` parses; `build-system:check` does not run
      tests
- [x] The lockfile is current — `cli:check` runs clippy `--locked`, so a stale
      `Cargo.lock` fails here rather than silently
- [x] Vocabulary and error tests pass:
      `cd cli && cargo nextest run -p tracker`
- [x] Both committed stamps round-trip byte-identically, including the Jira
      `+0000` form, and the fixture-shape test fails if either form is removed
- [x] `mise run pup:check` passes, i.e. the real `tracker` crate satisfies its
      own rule. This is a positive check only — a compliant crate cannot
      demonstrate the rule's discriminating power, which is what the probe pair
      is for
- [x] `mise run test:integration:pup` passes — the mise task the
      `check-architecture` CI lane runs. Prefer it over raw pytest: the suite
      *skips* rather than fails when cargo-pup is absent, so
      `uv run pytest … -k tracker` can exit 0 having asserted nothing unless
      `mise run deps:install:pup` has run. If using the raw form for the inner
      loop, confirm it reports passed rather than skipped
- [x] `mise run deny:check` passes with the new member
- [x] `mise run build-system:check` passes (the probe additions are Python)

#### Manual Verification

These are one-shot mutation checks: they establish that the guards discriminate,
but they are performed once and never re-run, so record the outcome of each in
the implementation notes rather than leaving a later reader to assume it.

- [x] The parity test fails when a fifth code is added to
      `work-item-bridge-codes.sh` — add one temporarily, in a shape the parser
      does not expect (a trailing comment) as well as the usual one
- [x] The parity test fails when a `TrackerError` variant is renamed — it reads
      the name from `Debug`, so the rename must propagate rather than being
      absorbed by a match arm
- [x] The parity test fails when the `Retryable`/`Terminal` resolution words are
      swapped between the 70 and 71 rows — not when the lines are reordered,
      since the fixture is keyed by name
- [x] Deleting the `tracker` rule from `cli/pup.ron` makes the probe pair fail
- [x] Corrupting an `allowed_only` anchor makes the compliant control fail —
      misspell the path segment, or narrow `^(std|core|alloc)(::|$)` to
      `^std$`. Two mutations do **not** work: a swapped alternation
      (`^crate($|::)`) is the same regex, and dropping the `^` *widens* the
      pattern rather than narrowing it, so nothing is rejected
- [x] `cargo public-api` renders parameter *names*, not only types — the
      same-typed `create(title, body)` swap is caught by the snapshot alone, so
      confirm this before relying on it (run the tool once against any existing
      workspace crate)

- [x] The `pup.ron` rule's `matches:` and `allowed_only:` anchors are
      character-identical to the sibling rules apart from the crate name and
      the omitted `kernel::Error` line
- [x] The committed `Cargo.lock` diff contains only the `tracker` package entry
      and its member line — no unrelated dependency movement
- [x] `TrackerError::Terminal`'s doc comment states that `Retryable` requires
      provable absence of a remote *change* — not of transmission — and that
      everything unproven is terminal. This is the part a client author will
      otherwise get wrong, so it is checked here rather than assumed
- [x] `RemoteIssue.body`'s doc comment states the projection contract, names it
      as distinct from `create`/`update`'s `body` parameter, and assigns
      reproduction to the implementing client
- [x] `RemoteTimestamp`'s doc comment states that the empty stamp means
      *unknown* and must not be read as equal to another empty stamp
- [x] `dispatch-codes.txt` records why 72 and 73 resolve above the port
- [x] `TrackerError` carries no `#[non_exhaustive]`
- [x] No doc comment in `src/lib.rs` names an item this phase does not declare
      — `FetchOutcome` and the trait arrive in Phase 2 and their cross-
      references arrive with them

**Implementation notes (recorded 2026-08-12):**

- Deviation 7 confirmed empirically: `missing_const_for_fn` under the
  workspace's `warnings = "deny"` DOES fire on `pub fn new(value: String) ->
  Self`, contrary to the plan's "historically declined to fire on a
  `Drop`-carrying parameter" hypothesis. `const fn` on both `new`
  constructors is therefore required, not a gratuitous forward commitment —
  confirmed by temporarily reverting to non-`const` and observing clippy
  reject it, then restoring.
- `cargo public-api` 0.52.0 omits function parameter names **by default**
  (a behaviour change from the version the plan assumed) — the
  `create(title, body)`-swap guard the plan relies on requires the
  `--include function-parameter-names` flag explicitly. Added it to both
  `public-api:check` and `public-api:update`'s shared invocation in
  `tasks/public_api.py`; without it the two-parameter-swap case would not
  have been caught.
- The pinned nightly's `--profile minimal` install already ships `rustdoc`
  bundled with the `rustc` component — no separate component needed, and no
  install-task change was required.
- `cargo-public-api` has no published GitHub release binary assets, so
  `deps:install:public-api` builds it from source via `cargo install`,
  mirroring `deps:install:pup`'s presence-probe pattern; recorded as a second
  accepted unverified surface in `mise.toml`'s `[settings]` comment.

Every check that concerns `lib.rs` sits in this phase, not Phase 2, because
that is where the file ships and the phases are independently mergeable. No
automated test reads doc text, so these are the only gate the comments have.

---

## Phase 2: The Port

### Overview

Write the golden and the port tests first, then add `RemoteTracker` and
`FetchOutcome` to satisfy them. The verification set that freezes the surface: a
private fake, a `Box<dyn RemoteTracker>` exercise, the exhaustive-consumer
signature probe, the partition-totality test, and the extended public-API
snapshot.

### Changes Required

#### 1. The snapshot, extended (red)

**File**: `cli/tracker/tests/fixtures/public-api.txt`
**Changes**: add the entries for `FetchOutcome` and `RemoteTracker`, again
hand-written from 0204's amended Requirements block before the trait exists, so
`public-api:check` is red until §3 lands.

`RemoteTracker`'s four methods appear as fully-qualified trait items with whole
signatures — the parameter order that a same-typed swap of `create`'s `title`
and `body` would change is pinned here, and pinned independently of how rustfmt
happens to wrap the declaration.

#### 2. The port tests (red)

**File**: `cli/tracker/tests/port.rs` (new)
**Changes**: the fake and the guards. Being an integration test, it links
against `tracker` as an external consumer and sees only the public API, so it
stops compiling if any signature widens — the guard a separate probe crate
would have given, without a second manifest to register.

```rust
//! The port as an external consumer sees it: a fake implementation that
//! stops compiling if any signature moves, exercised through a trait object
//! because the sync engine's composition root holds one.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;

struct FixedTracker {
    known: Vec<(ExternalId, RemoteIssue)>,
    unprovable: Vec<ExternalId>,
    lossy: Vec<ExternalId>,
}

impl FixedTracker {
    fn holding(known: Vec<(ExternalId, RemoteIssue)>) -> Self {
        Self {
            known,
            unprovable: Vec::new(),
            lossy: Vec::new(),
        }
    }

    fn truncating(
        known: Vec<(ExternalId, RemoteIssue)>,
        unprovable: Vec<ExternalId>,
    ) -> Self {
        Self {
            known,
            unprovable,
            lossy: Vec::new(),
        }
    }

    /// Ids whose write is acknowledged by nothing — the response is lost, so
    /// the fake cannot say whether the mutation applied.
    fn losing(
        known: Vec<(ExternalId, RemoteIssue)>,
        lossy: Vec<ExternalId>,
    ) -> Self {
        Self {
            known,
            unprovable: Vec::new(),
            lossy,
        }
    }

    fn issue(&self, id: &ExternalId) -> Option<&RemoteIssue> {
        self.known
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, issue)| issue)
    }
}

impl RemoteTracker for FixedTracker {
    fn create(
        &self,
        _title: &str,
        _body: &str,
        _kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        Ok(ExternalId::new(format!("ENG-{}", self.known.len() + 1)))
    }

    fn update(
        &self,
        id: &ExternalId,
        _title: &str,
        _body: &str,
    ) -> Result<(), TrackerError> {
        if self.lossy.contains(id) {
            return Err(TrackerError::Terminal {
                detail: format!(
                    "jira: update {id} failed, response lost after send"
                ),
            });
        }
        if self.issue(id).is_some() {
            return Ok(());
        }
        Err(TrackerError::Retryable {
            detail: format!(
                "jira: update {id} rejected, HTTP 404 no such issue"
            ),
        })
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        self.issue(id).cloned().ok_or_else(|| TrackerError::Retryable {
            detail: format!("fake: show {id} failed, connection refused"),
        })
    }

    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError> {
        let mut outcome = FetchOutcome {
            found: Vec::new(),
            absent: Vec::new(),
            indeterminate: Vec::new(),
        };
        let mut requested: Vec<&ExternalId> = ids.iter().collect();
        requested.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        requested.dedup();
        for id in requested {
            match self.issue(id) {
                Some(issue) => {
                    outcome.found.push((id.clone(), issue.updated.clone()));
                }
                None if self.unprovable.contains(id) => {
                    outcome.indeterminate.push(id.clone());
                }
                None => outcome.absent.push(id.clone()),
            }
        }
        Ok(outcome)
    }
}

const JIRA_STAMP: &str = "2026-07-09T08:00:00.000+0000";

fn issue(stamp: &str, body: &str) -> RemoteIssue {
    RemoteIssue {
        updated: RemoteTimestamp::new(stamp.to_owned()),
        body: body.to_owned(),
    }
}

fn known() -> Vec<(ExternalId, RemoteIssue)> {
    vec![(
        ExternalId::new("ENG-1".to_owned()),
        issue(JIRA_STAMP, "Pushed title\nPushed description\n"),
    )]
}

#[test]
fn all_four_operations_are_reachable_through_a_trait_object() {
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::holding(known()));

    let created = tracker
        .create("Add remote tracker port", "Body text\n", "story")
        .expect("the fake always creates");
    assert_eq!(created.as_str(), "ENG-2");

    let id = ExternalId::new("ENG-1".to_owned());
    assert_eq!(tracker.update(&id, "Title", "Body text\n"), Ok(()));
    assert_eq!(
        tracker.show(&id).expect("the fake holds ENG-1").body,
        "Pushed title\nPushed description\n"
    );
    let outcome = tracker.fetch_all(&[id.clone()]).expect("the fake fetches");
    assert_eq!(
        outcome.found,
        vec![(id, RemoteTimestamp::new(JIRA_STAMP.to_owned()))]
    );
}

/// A compile-time echo of the surface pin, on the stable lane.
///
/// `public-api:check` runs only on the nightly architecture job, so a nightly
/// break or a rustdoc-JSON skew takes the freeze offline while every stable
/// check stays green. Exhaustive destructuring costs nothing and means an added
/// or removed public field reddens `cargo nextest` too.
#[test]
fn every_public_field_is_accounted_for() {
    let tracker = FixedTracker::holding(known());
    let id = ExternalId::new("ENG-1".to_owned());

    let RemoteIssue { updated, body } =
        tracker.show(&id).expect("the fake holds ENG-1");
    let FetchOutcome {
        found,
        absent,
        indeterminate,
    } = tracker.fetch_all(&[id]).expect("the fake fetches");

    assert_eq!(updated.as_str(), JIRA_STAMP);
    assert!(!body.is_empty());
    assert_eq!(found.len(), 1);
    assert!(absent.is_empty() && indeterminate.is_empty());
}

#[test]
fn a_failed_read_is_retryable_because_it_mutated_nothing() {
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::holding(Vec::new()));
    let missing = ExternalId::new("ENG-404".to_owned());

    assert_eq!(
        tracker.show(&missing),
        Err(TrackerError::Retryable {
            detail: "fake: show ENG-404 failed, connection refused".to_owned()
        })
    );
}

#[test]
fn a_rejected_write_is_retryable_because_nothing_was_modified() {
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::holding(Vec::new()));
    let missing = ExternalId::new("ENG-404".to_owned());

    assert_eq!(
        tracker.update(&missing, "Title", "Body"),
        Err(TrackerError::Retryable {
            detail: "jira: update ENG-404 rejected, HTTP 404 no such issue"
                .to_owned()
        })
    );
}

#[test]
fn a_write_whose_response_was_lost_is_terminal() {
    let id = ExternalId::new("ENG-1".to_owned());
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::losing(known(), vec![id.clone()]));

    assert_eq!(
        tracker.update(&id, "Title", "Body"),
        Err(TrackerError::Terminal {
            detail: "jira: update ENG-1 failed, response lost after send"
                .to_owned()
        })
    );
}

fn partitions_totally(outcome: &FetchOutcome, requested: &[ExternalId]) {
    let mut reported: Vec<&ExternalId> = outcome
        .found
        .iter()
        .map(|(id, _)| id)
        .chain(outcome.absent.iter())
        .chain(outcome.indeterminate.iter())
        .collect();
    let reported_count = reported.len();
    reported.sort_by_key(|id| id.as_str().to_owned());
    reported.dedup();
    assert_eq!(
        reported_count,
        reported.len(),
        "an id was reported more than once across the three vectors"
    );

    let mut expected: Vec<&ExternalId> = requested.iter().collect();
    expected.sort_by_key(|id| id.as_str().to_owned());
    expected.dedup();
    assert_eq!(
        reported, expected,
        "the partition does not cover exactly the requested ids"
    );
}

#[test]
fn a_bulk_fetch_partitions_every_requested_id_exactly_once() {
    let present = ExternalId::new("ENG-1".to_owned());
    let gone = ExternalId::new("ENG-9".to_owned());
    let unseen = ExternalId::new("ENG-7".to_owned());
    let requested = [present.clone(), gone.clone(), unseen.clone()];
    let tracker = FixedTracker::truncating(known(), vec![unseen.clone()]);

    let outcome = tracker
        .fetch_all(&requested)
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &requested);
    assert_eq!(
        outcome.found.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![&present]
    );
    assert_eq!(outcome.absent, vec![gone]);
    assert_eq!(outcome.indeterminate, vec![unseen]);
}

#[test]
fn a_duplicated_id_is_partitioned_once() {
    let id = ExternalId::new("ENG-1".to_owned());
    let tracker = FixedTracker::holding(known());

    let outcome = tracker
        .fetch_all(&[id.clone(), id.clone()])
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &[id]);
    assert_eq!(outcome.found.len(), 1);
}

#[test]
fn an_empty_request_makes_no_call_and_yields_an_empty_outcome() {
    let tracker = FixedTracker::holding(known());

    let outcome = tracker
        .fetch_all(&[])
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &[]);
}

#[test]
fn an_unprovable_id_is_indeterminate_rather_than_absent() {
    let unseen = ExternalId::new("ENG-7".to_owned());
    let requested = [unseen.clone()];
    let tracker =
        FixedTracker::truncating(Vec::new(), vec![unseen.clone()]);

    let outcome = tracker
        .fetch_all(&requested)
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &requested);
    assert!(
        outcome.absent.is_empty(),
        "a truncated fetch must not report absence"
    );
    assert_eq!(outcome.indeterminate, vec![unseen]);
}
```

Three things here are deliberate rather than incidental. The fake synthesises an
identifier instead of echoing the title, because this file is the only worked
example a 0171 client author has and echoing teaches a title-is-an-identifier
conflation the real contract does not have. The `Err` cases are exercised
through the trait object rather than constructed inline, so the
`Result<_, TrackerError>` half of every signature — the half both consumers
branch on — is covered end to end.

The two `update` failures model the classification rather than merely producing
one of each class. A rejected write (the tracker answered, and answered "no")
is `Retryable`, because nothing was modified; a lost response is `Terminal`,
because it might have been. Getting this backwards in the worked example would
teach the exact mistake the taxonomy exists to prevent — and an earlier draft
did, returning `Terminal` for a rejection. And `partitions_totally` is a reusable
property check rather than three positional assertions: it is the shape 0194's
parameterised contract test needs, so writing it here means that test inherits
a definition rather than re-deriving one.

Note what these tests do **not** establish. They prove the fake honours the
partition, not that the contract is enforceable — `FetchOutcome` has three
public fields and no constructor, so a real client can return an unsound
partition and nothing in this crate will notice. That gap is recorded under
What We're NOT Doing and handed to 0194.

#### 3. The partition type and the trait (green)

**File**: `cli/tracker/src/lib.rs`
**Changes**: append.

```rust
/// What a bulk retrieval could establish about each requested issue.
///
/// The partition is total over the requested ids: every distinct id appears in
/// exactly one of the three vectors. Duplicates in `ids` are ignored — the
/// request is a set, as both bash adapters treat it — an empty request yields
/// an empty outcome and makes no remote call, and the three vectors are
/// unordered, so a caller indexes rather than zips.
///
/// `absent` carries the weight. An id belongs there only when the retrieval
/// was provably complete — a truncated page, an exhausted rate limit or a
/// partial failure puts its unseen ids in `indeterminate` instead. An
/// implementation that cannot distinguish the two must report every unseen id
/// as indeterminate: inferring absence from a fetch that may have been cut
/// short is what makes a sync delete an issue that still exists.
///
/// Nothing here enforces totality — the type cannot, and this crate ships no
/// logic. It is an obligation on every implementation, held by the shared
/// contract test that lives with the sync engine. Until that exists,
/// `tracker/tests/port.rs::partitions_totally` is the check to copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchOutcome {
    /// The issues the retrieval accounted for, each paired with the id it was
    /// requested under — bulk payloads carry no other way to associate a
    /// record with a local work item.
    ///
    /// A stamp, not an issue: bulk retrieval establishes *whether* an issue
    /// changed, and `show` fetches the body for the minority that did. No
    /// provider's bulk query returns a projected body, so a `RemoteIssue` here
    /// could only ever be filled with a fabricated one.
    ///
    /// An issue the tracker returns without a timestamp still belongs here,
    /// paired with an empty `RemoteTimestamp`. Never drop it: an id missing
    /// from a complete retrieval reads as absence, so filtering out the
    /// null-stamped entries reports a live issue as deleted.
    pub found: Vec<(ExternalId, RemoteTimestamp)>,
    /// Provably gone from the tracker. Only ever drawn from a complete
    /// retrieval.
    pub absent: Vec<ExternalId>,
    /// Not accounted for, and the retrieval could not prove why.
    pub indeterminate: Vec<ExternalId>,
}

/// The operations a remote issue tracker exposes to the sync engine.
///
/// Synchronous and `&self`-taking so the trait stays dyn-compatible: the sync
/// engine selects its client at composition time from the `work.integration`
/// config key and holds it as `Box<dyn RemoteTracker>`. A native async fn in a
/// trait is not object-safe, and the crate's import rule forbids the
/// `async-trait` dependency that would restore it.
///
/// Every call blocks until it resolves. Timeouts, retries and backoff are
/// wholly the implementing client's responsibility — they live inside the bash
/// bridges today, i.e. already behind this seam — and a client must bound its
/// own calls, because a caller has no way to.
pub trait RemoteTracker {
    /// Creates a new remote issue and returns the identifier assigned to it.
    ///
    /// An identifier that cannot be safely written back to frontmatter is a
    /// [`TrackerError::Terminal`] failure, not an `Ok`: the issue exists
    /// remotely, so a repeat would duplicate it, and the caller must be told
    /// rather than handed a value that would corrupt the file.
    ///
    /// `title` and `body` are the local work item's, not a projected body.
    /// `kind` is the work item's `kind` value, taken opaquely: mapping it onto
    /// a Jira issue type or its Linear equivalent is the implementing client's
    /// business, and the empty string means "use the tracker's configured
    /// default", which is what the bash bridge does with an omitted `--kind`.
    ///
    /// # Errors
    ///
    /// [`TrackerError::Retryable`] only when it is provable that no issue was
    /// created — which includes a tracker-side rejection, where the provider's
    /// protocol makes the rejection provable. A remote create is not
    /// idempotent, so once the request may have been *applied* the failure is
    /// [`TrackerError::Terminal`]: a repeat would duplicate the issue.
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError>;

    /// Replaces the remote issue's whole content.
    ///
    /// Returns nothing: the caller that needs the resulting stamp for its sync
    /// baseline reads it back with `show`.
    ///
    /// # Errors
    ///
    /// [`TrackerError::Retryable`] only when it is provable that nothing was
    /// modified; otherwise [`TrackerError::Terminal`]. The operation is
    /// idempotent, so the hazard is not duplication but not knowing whether it
    /// landed.
    ///
    /// The two operations' provable sets are **not nested in either
    /// direction** for at least one provider — each is narrower than the other
    /// on some conditions — so derive each from that operation's own mapping
    /// rather than carrying a classification across.
    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError>;

    /// Reads one remote issue in full, including its projected body.
    ///
    /// Absence is not discoverable here — a `RemoteIssue` or an error are the
    /// only outcomes. Establish that an id is gone with `fetch_all`, whose
    /// partition distinguishes provable absence from an unproven miss, and do
    /// not build a retry loop around a `show` that may be reading a deleted
    /// issue.
    ///
    /// # Errors
    ///
    /// Always [`TrackerError::Retryable`]. A read mutates nothing, so the
    /// terminal class — which means "a mutation may have applied" — cannot
    /// arise; the bash read bridge collapses every read failure to the
    /// retryable code for the same reason.
    ///
    /// Read the class as "nothing changed remotely", not as "call again". A
    /// deleted issue fails here indefinitely, so the caller degrades to
    /// presence-only rather than looping.
    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError>;

    /// Reads the requested issues in bulk, partitioned by what the retrieval
    /// could establish.
    ///
    /// Returns stamps, not bodies. This is the cheap first tier of a two-tier
    /// read: compare each stamp against the sync baseline, then call `show`
    /// for the minority that moved.
    ///
    /// # Errors
    ///
    /// Always [`TrackerError::Retryable`], and only on a **pre-flight** failure
    /// — one that stops any request being constructed, such as unresolvable
    /// credentials or a requested id the client cannot safely embed in its
    /// query.
    ///
    /// Once a request has been attempted, every outcome is an `Ok`. A partial
    /// retrieval puts its unproven ids in `indeterminate`; so does a total
    /// transport failure, which is an `Ok` with every id indeterminate rather
    /// than an `Err`. Degrading per id beats failing a whole sync run, and the
    /// partition can already say it.
    ///
    /// This deliberately differs from the current Linear bridge, which returns
    /// a retryable exit for any bulk-search failure — a client porting that
    /// adapter must move transport failures into the partition rather than
    /// carrying the `Err` across.
    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError>;
}
```

Both read operations state that their failures are always `Retryable`.
`work-item-fetch-remote.sh:42-48` is explicit that "a read mutates nothing, so
71 (terminal-may-have-mutated) does not apply here", and a client author who
returned `Terminal` from a failed read would have 0194 surface a hard error
where the bash path degrades to presence-only and exits 0. The first draft's
`show` doc said the opposite.

There is no reconcile-after-formatting step. rustdoc JSON carries API shape, not
source text, so `mise run fix` cannot move the snapshot — which was the whole
reason the textual golden needed one.

### Success Criteria

#### Automated Verification

- [ ] The crate compiles with the port added: `mise run cli:check`
- [ ] All port tests pass: `cd cli && cargo nextest run -p tracker`
- [ ] The trait is dyn-compatible — `all_four_operations_are_reachable_through_a_trait_object`
      fails to compile if the trait is made async or otherwise object-unsafe
- [ ] Both error classes are observed coming back from a port call, not just
      constructed — and on the axis that defines them:
      `a_rejected_write_is_retryable_because_nothing_was_modified` and
      `a_write_whose_response_was_lost_is_terminal`
- [ ] `mise run public-api:check` passes against the hand-written snapshot
- [ ] Running `mise run fix` does not move the snapshot
- [ ] `mise run pup:check` and `mise run deny:check` pass
- [ ] The full local CI mirror is green: `mise run`

#### Manual Verification

The mutation checks below are one-shot: they establish that the freeze
discriminates, then never run again. Record each outcome in the implementation
notes.

- [ ] The snapshot was hand-written from 0204's amended Requirements block, and
      was red before `src/lib.rs` gained the port
- [ ] Renaming any trait method or changing any parameter **type** breaks
      `port.rs` at compile time
- [ ] Swapping `create`'s `title` and `body` parameters — same type, so the
      fake still compiles — fails `public-api:check`
- [ ] Adding a seventh public item fails `public-api:check`
- [ ] Adding a **fifth trait method with a default body** fails
      `public-api:check` — the additive change `port.rs` cannot catch
- [ ] Giving one of the **four existing** methods a default body is caught by
      something. This is the more dangerous half of AC 2 — it lets a client
      silently not implement an operation — and neither guard obviously sees
      it: the fake overrides all four so it still compiles, and rustdoc JSON
      may not distinguish a provided from a required trait method. Confirm
      empirically; **if nothing catches it, record AC 2's default-body clause
      as unguarded** rather than leaving it implied
- [ ] Adding `PartialOrd, Ord` to `RemoteTimestamp` fails `public-api:check`
      — it surfaces as two added impls, not as a changed literal
- [ ] Renaming `TrackerError::Retryable` fails `public-api:check` — variants
      are items in rustdoc JSON, which the textual golden could not see
- [ ] Deleting `impl std::error::Error for TrackerError {}` fails
      `public-api:check` and `a_tracker_error_is_usable_as_a_std_error`
- [ ] Adding a `#[cfg(test)]` module to `src/`, a `[dev-dependencies]` table
      to the manifest, or a `tracker-adapters` member to the workspace each
      fails its guard in `structure.rs`
- [ ] Removing a `#[must_use]` fails **nothing** — attributes are not API shape.
      Confirm this is so, and that the four attributes are present, by reading
- [ ] `fetch_all`'s doc comment states that a partial retrieval is an `Ok` with
      `indeterminate` ids, never an `Err`, and that it returns stamps rather
      than bodies
- [ ] `show`'s doc comment states that absence is not discoverable through it
- [ ] Both read operations' `# Errors` sections state that a read failure is
      always `Retryable`, and say the class means "nothing changed" rather than
      "call again"
- [ ] `src/lib.rs` declares exactly six public items
- [ ] `src/` contains no function body beyond the four inherent methods, the
      two `Display` impls and the `Error` impl — AC 10's no-behaviour half,
      which `structure.rs` deliberately does not cover

The `RemoteIssue.body` and `#[non_exhaustive]` checks moved to Phase 1, where
`lib.rs` ships.

---

## Phase 3: Pup Probe Backfill

### Overview

Backfill probe pairs for the four domain rules that today have only
`test_real_cli_pup_ron_loads` — a parse check that proves nothing about
discriminating power. Subject-independent of Phases 1 and 2, but it edits the
same file as Phase 1's probe pair and renames constants that probe uses, so it
rebases onto Phase 1 rather than merging in parallel.

### Changes Required

**File**: `tests/integration/pup/test_import_rule.py`
**Changes**: a parametrised section covering `corpus`, `vcs`, `work` and
`migrate`.

The four rules share a **baseline** shape — the whole-crate domain allowance of
`std`/`core`/`alloc`, `kernel::Error` and `crate` — but two of them widen it,
each with a comment in `pup.ron` recording the widening as deliberate:
`work_domain_imports_only_permitted` additionally permits `^corpus(::|$)`
(`cli/pup.ron:96-102`) for `WorkItemIdScheme`/`IdScanner`, and
`migrate_domain_imports_only_permitted` additionally permits `^corpus(::|$)`
and `^document(::|$)` (`cli/pup.ron:177-184`). The writer is therefore
parameterised by **crate, rule name and extra allowances**, not by crate alone.
Probing only the shared prefix would leave the two rules most likely to be
edited — and whose widenings are the ones a reviewer would want justified —
exactly as unguarded after the backfill as before it.

Three baseline cases per crate — an outbound violation (importing a sibling
adapter crate), a positive control, and the narrowed `kernel::Error` allowance
— plus one positive case per extra allowance. Four crates, three extras:
fifteen cases.

**The control must import.** `_CONFIG_SERVICE_COMPLIANT`
(`test_import_rule.py:277`) is `pub fn make() -> u8 { 0 }` with no `use`
statement, so reusing it as-is would give all four backfilled rules the weak
control Phase 1 §4 argues against — leaving `^(std|core|alloc)(::|$)` and
`^crate(::|$)`, the two anchors every domain rule shares, exercised by nothing.
The renamed `_DOMAIN_SERVICE_COMPLIANT` is therefore *rewritten*, not merely
renamed, to the `_TRACKER_PORT_COMPLIANT` shape: one `std::` import and one
`crate::` import.

Two call sites cannot take that body as-is, and both would fail for reasons
unrelated to the rule under test. `version::core`'s rule permits only
`^crate::version::core(::|$)`, so a bare `use crate::Marker;` is a *violation*
there — leave that probe on its own subtree-scoped body. And the writer must
emit a `lib.rs` declaring the item the control imports: `_CONFIG_LIB` is
`pub mod service;` with no `Marker`, so the import would not compile. Give the
domain-rule writer a lib body carrying both `pub mod service;` and
`pub struct Marker;`, as `_TRACKER_LIB` already does — **and add
`pub struct Marker;` to `_CONFIG_LIB` too**, or the existing
`test_real_config_rule_passes_a_compliant_service` fails on a compile error
rather than on anything about the rule it probes. The addition is inert for
`_write_config_store_probe`, which shares that body.

```python
# --- The remaining whole-crate domain rules ---
#
# Each is driven against a workspace whose crate is literally named for it,
# under the shipped cli/pup.ron, so deleting or mistyping a rule fails here.
# corpus and vcs carry config's rule shape; work and migrate widen it, and
# the third element pins what each widening permits.

_DOMAIN_RULES = [
    ("corpus", "corpus_domain_imports_only_permitted", ()),
    ("vcs", "vcs_domain_imports_only_permitted", ()),
    ("work", "work_domain_imports_only_permitted", ("corpus",)),
    (
        "migrate",
        "migrate_domain_imports_only_permitted",
        ("corpus", "document"),
    ),
]
```

The writer emits a workspace of `<crate>`, `adapters`, `kernel` and one member
per extra allowance, reusing `_ADAPTERS_MANIFEST`, `_ADAPTERS_LIB`,
`_KERNEL_MANIFEST`, `_KERNEL_LIB` and `_KERNEL_LOGGING` already defined in the
file. The three shared service bodies move with it: `_CONFIG_SERVICE_VIOLATION`,
`_CONFIG_SERVICE_COMPLIANT` and `_CORE_KERNEL_ERROR` are renamed
`_DOMAIN_SERVICE_VIOLATION`, `_DOMAIN_SERVICE_COMPLIANT` and
`_DOMAIN_KERNEL_ERROR`, with their existing `config` and `version::core` call
sites updated in the same change. They are about to describe six crates, and a
failing `migrate` probe built from something called `_CONFIG_SERVICE_VIOLATION`
reads as a copy-paste mistake rather than deliberate sharing.

The tests are
`@pytest.mark.parametrize(("crate", "rule", "extras"), _DOMAIN_RULES)` over that
writer — the tuple form every multi-name parametrisation in this repo uses
(`test_registration_docs.py:129`, `test_integration.py:94`) — asserting
`"is not allowed"` and the rule name on the violation, exit 0 on the control,
exit 0 on the `kernel::Error` import, and exit 0 on each extra allowance.

**Also the converse.** Positive cases alone detect a widening being *lost*,
never one being *gained* — pasting `^document(::|$)` into
`vcs_domain_imports_only_permitted` would pass every case above. Two more
parametrised families close that:

- each crate **rejects** every extra allowance not in its own tuple — `vcs`
  rejects both `corpus` and `document`, `corpus` rejects `document`, `work`
  rejects `document`, `migrate` rejects neither (four cases);
- each crate rejects `kernel::logging`, proving the allowance is the narrowed
  `^kernel::Error` and not a bare `^kernel` (four cases). This is the analogue
  of the existing `test_core_importing_kernel_infra_is_rejected`, which exists
  for exactly this reason against the synthetic RON.

Boundary erosion is the failure the backfill exists to catch, and it is the
direction a reviewer unblocking an import would push. Twenty-three cases in
total, not fifteen.

Note the cost: twenty-three additional cargo-pup invocations, each compiling a small
synthetic workspace. They land on `test:integration:pup` (`mise.toml:293`), not
on `pup:check` (`mise.toml:525`, `cargo +nightly pup` over the real workspace),
which gains no work from this phase.

`test:integration:pup` is in neither the aggregate `check` nor the bare
`default`, so a full local run is unaffected — but it **does gate every pull
request**: `.github/workflows/main.yml` triggers on `pull_request`, and the
`check-architecture` job runs it unconditionally. "Nightly" in this repo names
the pinned Rust *toolchain* the lane uses, not a schedule. So the coverage is
real feedback in review rather than next-day feedback, and the cost is paid per
PR push rather than once a night. Twenty-three synthetic-workspace compilations is a
material addition on that budget; if it proves too slow, share one workspace
build per crate across its cases before dropping any case.

`work_adapters::filesystem` and `migrate_adapters` remain unprobed. Both are
module-scoped rules of a different shape from the domain four, so they do not
fit this writer; they are recorded as follow-up rather than bundled in.

### Success Criteria

#### Automated Verification

- [ ] All twenty-three new cases pass: `mise run test:integration:pup`
- [ ] The rewritten shared control leaves the existing `config` and
      `version::core` probes green
- [ ] Phase 1's tracker probe pair still passes after the constant renames
- [ ] `mise run build-system:check` passes (ruff, pyrefly)
- [ ] `mise run pup:check` still passes

#### Manual Verification

- [ ] Each violation case fails for the right rule — delete one rule from
      `cli/pup.ron` and confirm only that crate's cases fail
- [ ] Removing `^corpus(::|$)` from the `work` rule fails `work`'s
      extra-allowance case and nothing else. Same for `^document(::|$)` on
      `migrate`
- [ ] The parametrised writer produces the same workspace shape the bespoke
      `config` probe does for a crate with no extra allowances, so a reader can
      see the two are equivalent
- [ ] `check-architecture` runtime is acceptable — this lane runs per PR, not
      overnight

---

## Testing Strategy

### Unit Tests

There are none, by design. AC 10 forbids a `#[cfg(test)]` module in
`tracker/src/`, and the crate has no behaviour to unit-test. Everything the
house style would normally put inline — the per-arm `Display` assertions from
the `cli/corpus/src/store.rs:81-148` recipe — moves to `tracker/tests/` and
works unchanged, because `Display` is public surface.

### Integration Tests

`tracker/tests/` holds four binaries, each its own crate linking against
`tracker` as an external consumer:

- `vocabulary.rs` — the fixture-shape guard, byte-identical round-trip over both
  real provider stamp formats, whitespace inequality, the empty-string case,
  and the `Display`/`Hash` surface deviation 6 adds
- `errors.rs` — per-arm `Display` messages, the wildcard-free closed-set match,
  the four-code parity fixture against the live bash script with its expected
  classes derived from the enum
- `port.rs` — the fake, the trait object, both error classes returned *through*
  a port call, the signature probe, the reusable partition-totality check, and
  the truncation case that must not report absence
- `structure.rs` — the three invariants rustdoc JSON cannot see: no
  `#[cfg(test)]` module (AC 10), no dependency tables (AC 9), no
  `tracker-adapters` member (AC 10)

The public surface itself is pinned outside the test suite, by
`mise run public-api:check` against `cli/tracker/tests/fixtures/public-api.txt`
— a build-system check rather than a Rust test, because AC 9 forbids the
`[dev-dependencies]` table a crate-local diff harness would need.

They are picked up by `cargo nextest run --workspace` from workspace membership
alone (`tasks/test/cli.py:6-14`), satisfying AC 11's "built and run … rather
than being excluded" for free. There are no Rust coverage floors — coverage is
report-only with no `--fail-under` (`tasks/test/cli.py:22-24`) — and the shell
suite-count floors count `test-*.sh` files, so a Rust crate does not move them.

### Manual Testing Steps

Each step mutates the tree, confirms a guard reddens, and reverts. Record the
outcomes — these establish that the freeze discriminates and they never run
again.

1. Add a fifth code to `work-item-bridge-codes.sh`, once as
   `readonly E_DISPATCH_X=74` and once as
   `readonly E_DISPATCH_X=74 # backoff`; confirm the parity test fails both
   times rather than silently dropping the commented form.
2. Rename `TrackerError::Retryable`; confirm the parity test fails. `class_of`
   reads the name from `Debug`, so fixing the match arm alone must not restore
   green.
3. Swap the `Retryable` and `Terminal` *resolution words* between the 70 and 71
   rows, leaving names and numbers in place; confirm
   `each_dispatch_code_maps_onto_the_class_it_names` fails. Reordering the two
   lines proves nothing — the fixture is read into a `BTreeMap` keyed by name,
   so line order cannot affect any assertion.
4. Add `pub fn extra(&self) {}` to `impl ExternalId`; confirm `public-api:check` fails
   naming the snapshot and the regeneration command.
5. Add a fifth `RemoteTracker` method **with a default body**; confirm
   `public-api:check` fails and `port.rs` still compiles — the additive change the
   fake cannot catch.
6. Swap `create`'s `title` and `body` parameters; confirm `public-api:check` fails and
   `port.rs` still compiles — same-typed parameters, so only the snapshot sees
   it.
7. Rename `TrackerError::Retryable`; confirm `public-api:check` fails — variants are
   items in rustdoc JSON.
8. Delete `impl std::error::Error for TrackerError {}`; confirm both
   `public-api:check` and `a_tracker_error_is_usable_as_a_std_error` fail.
9. Add `PartialOrd, Ord` to `RemoteTimestamp`; confirm `public-api:check` fails —
   two added impls, not a changed literal.
10. Rename `fetch_all` to `fetch`; confirm `port.rs` fails to compile.
11. Empty `remote-updated-at.txt`; confirm
    `the_fixture_covers_both_incompatible_provider_formats` fails rather than
    the round-trip loop passing on zero rows.
12. Add `use work::…` to `cli/tracker/src/lib.rs` (with a temporary
    `[dependencies]` entry), run `mise run pup:check`, confirm it fails naming
    `tracker_domain_imports_only_permitted`. The Phase 1 probe pair automates
    this, so it is a one-off confirmation that the automation matches reality.
13. Delete the `tracker` rule from `cli/pup.ron`; confirm the probe pair fails.
14. Corrupt an `allowed_only` anchor — misspell the path segment, or narrow
    `^(std|core|alloc)(::|$)` to `^std$` — and confirm the compliant control
    fails. Two mutations will *not* work: a swapped alternation
    (`^crate($|::)`) is the same regex, and dropping the `^` widens rather
    than narrows.

## Performance Considerations

Only one, and it is build-time. Phase 3 adds twenty-three cargo-pup invocations to
`test:integration:pup`, each compiling a synthetic workspace on the pinned
nightly (`PUP_NIGHTLY = "nightly-2026-01-22"`, `tasks/shared/rust.py:6-7`), and
cargo-pup builds from source on first run.

The cost lands on the `check-architecture` CI job, which runs on **every pull
request** — the workflow triggers on `pull_request` and the job is
unconditional. It does not land on a full local run: `test:integration:pup`
(`mise.toml:293`) is in neither the aggregate `check` nor the bare `default`,
and `pup:check` (`mise.toml:525`, in both) is a different task that gains no
work from this phase.

If per-PR runtime proves intolerable, the first lever is sharing one synthetic
workspace build per crate across its cases; only after that, dropping the
`kernel::Error` positive case per crate. Never the violation cases or the
extra-allowance cases — those are the discriminating power the phase exists to
add.

`public-api:check` adds a second nightly-lane cost: one rustdoc-JSON build of a
zero-dependency crate, which is small, plus a first-run `cargo install` of
cargo-public-api. Like `pup:check` it sits in the aggregate `check` and the bare
`default`, so the install is paid once locally and cached in CI by the same
`Swatinem/rust-cache` step the lane already has.

The crate itself adds negligible compile time — no dependencies, one source
file, four small test binaries.

## Migration Notes

None. `tracker` is additive: nothing imports it until 0171 and 0194 land, and
no existing behaviour changes. The bash sync path continues to run unmodified,
and `work-item-bridge-codes.sh` remains the authoritative taxonomy until 0171
retires it — at which point the parity fixture is the artefact 0171's criterion
to delete the script "and its parity fixture" refers to.

## Handoffs

Obligations that leave this plan explicitly, so they are not silently dropped.

**Before implementation starts**

- **0204 must be edited** to carry deviations 5, 6 and 7 (deviations 1-4 are
  already in it). The places below, not one — the frozen block is quoted and
  paraphrased across the item:
  - Requirements block: `FetchOutcome.found` becomes
    `Vec<(ExternalId, RemoteTimestamp)>`; `TrackerError` derives
    `Clone, PartialEq, Eq`; `ExternalId` derives `Hash`; both `new`
    constructors become `const fn` (pending the empirical check in deviation 7).
  - Requirements block: "`Display` and `Error` on `TrackerError` are the sole
    permitted impls with bodies" must admit `Display` on `ExternalId`, and the
    block itself should carry the impl so AC 1's "nothing else" enumeration
    covers it.
  - Requirements prose: "`fetch_all` pairs each issue with its `ExternalId`" —
    deviation 5 makes it pair a timestamp.
  - Requirements prose: "retryable requires *provable* absence of
    transmission". The oracle says otherwise — `work-item-bridge-codes.sh:9`
    scopes 70 to "before any remote **mutation**", and the Jira retryable set
    includes 4xx rejects that plainly transmitted. 0204 currently declares the
    narrower rule as frozen contract, and the plan's doc comments contradict
    it.
  - **AC 1 must be rewritten** (deviation 8). It currently specifies a golden
    under `tracker/tests/fixtures/` read by a test that parses `src/lib.rs`, and
    says `cargo public-api` "is deliberately not used — it is absent from this
    repository and installing it would mean a third Rust toolchain". All three
    clauses are now wrong: the tool is adopted, the snapshot lives at
    `cli/tracker/tests/fixtures/public-api.txt`, and it reuses the existing nightly.
  - AC 9 asserts the probe pair "runs on the nightly cargo-pup lane
    (`mise run pup:check`)". It runs under `test:integration:pup`; `pup:check`
    is a different task.
  - AC 10's list of permitted function bodies: same addition.
  - Technical Notes: the paragraph handing 0171 the Linear-`description`
    constraint is retired by deviation 5 — the bulk arm no longer asks for a
    body.
  - Drafting Notes: the planning entry says the block was reopened once, for
    "Four changes". It is now eight, across three passes.
- **0194 must be told, and it is more than a wording fix.** Its description of
  the port is stale (five items, no `FetchOutcome`), but the substantive change
  is that bulk retrieval no longer carries bodies: its acceptance criterion
  "exactly one `fetch_all` call and **zero `show` calls**" is unsatisfiable
  except on an all-unchanged corpus, and its classifier requirement frames bulk
  and per-item reads as alternatives when they are now two tiers of one read.
  Both need restating.
- **0171 must be told** — same stale five-item description, plus the retired
  Linear-`description` constraint.

**0171 inherits**

- Reproducing each provider's projection recipe for `show`'s `RemoteIssue.body`
  exactly — title line, then description, no blank line between them, Jira's
  ADF key-sorted and compact. `cli/work-adapters/src/project_remote.rs`
  implements it in Rust but sits behind a `work` dependency and projects the
  `show` payload shape, so no client can reuse it as it stands. This plan does
  not move it.
- The trap that Linear code 34 is retryable on `create` but terminal on
  `update` (`work-item-update-remote.sh:59-65`) — the same wire condition maps
  to two classes depending on the operation, so a single status-to-class table
  is wrong. Its read-side twin: a read failure is always retryable.

  **The tables are conservative, and where they disagree with the rule they
  win.** Linear codes 18, 23, 25, 27 and 29 (test-gate refusals, token
  resolution failures, insecure local permissions) are provably
  pre-transmission, yet `_linear_map_no_file_failure`'s catch-all maps them
  terminal on `create`. A client that reasoned purely from the provability rule
  would classify them retryable and diverge from today's behaviour. Port the
  tables; treat the rule as what explains them, not as licence to be more
  precise than they are. Four of them, not two — `_wicr_map_jira` and `_wiur_map_jira` for
  Jira, and `_wiur_map_linear` plus `_linear_map_no_file_failure` (inside
  `linear-create-flow.sh`) for Linear. Code 34 is the worked example of why they
  differ: `create` routes it through a pre/post-send seam that makes the
  rejection provable, while `linear-graphql.sh` propagates it on `update` with
  no such distinction, so the same status is retryable on one and terminal on
  the other.
- **Discharging the identifier-safety obligation.** Now stated on `ExternalId`
  and on `create`, so a client author meets it in the crate rather than in this
  plan — but the port cannot enforce it, so 0171 owns the implementation and
  0171's Requirements should say so.
  **The check to port.** `work-item-create-remote.sh:62-87,238-246`
  validates every returned identifier before passing it on — rejecting control
  characters, newlines, a leading `---` and a leading `#` — because the value is
  written unquoted into a work item's YAML frontmatter, and classifies a
  violation as terminal ("an issue may exist — do NOT retry"). It is the one
  tracker-agnostic check the dispatcher performs, and the dispatcher dissolves
  at the port: `ExternalId::new` is infallible by freeze, so the type cannot
  carry it. Each client owns it, and returning an identifier unsafe to persist
  is a `Terminal` failure rather than an `Ok`.
- The update bridge's `--dry-run` (`work-item-update-remote.sh:19,102-105`),
  which forwards the tracker's real `--print-payload` and is what
  `/sync-work-items --preview` uses to validate every push against the live
  tracker. 0194's `--preview` routes mutations to no-ops and makes no port call,
  so it does not discharge this.
- Bounding its own calls. The port is synchronous with no deadline or
  cancellation, so the per-request timeouts the bash bridges carry today
  (`curl --max-time 30` in `jira-request.sh`, `--max-time 60` in the Linear
  flows) and the `_WIFR_PAGE_CAP=20` pagination backstop must be reproduced
  behind the seam. A caller has no way to add them, and `/list-work-items`
  relies on the read path not hanging.
- The unkeyed `search` mode, which has no port operation. Decide at cutover
  whether untracked discovery is dropped, re-sited above the port, or carried
  as an additive item.
- The create bridge's `--dry-run` field-resolution preview, which has no port
  surface. Either re-site it above the port at cutover or carry it as an
  additive item; losing it silently costs `/create-work-item` its informed push
  offer and its early unresolvable-project failure.
- Updating `RemoteIssue.body`'s doc comment and `errors.rs`'s module doc when
  the bash bridges are retired. Both name scripts that 0171 deletes; the
  contracts they state outlive the references, but the references go stale the
  moment the files do.

**0194 inherits**

- The pending-push marker that keeps `create --push` retries idempotent without
  a port lookup.
- The shared reusable fake and the parameterised `RemoteTracker` contract test.
  The fake built here is deliberately private and duplicated.
- **Asserting `FetchOutcome`'s totality for every client.** The type cannot
  enforce it and AC 10 forbids a constructor that could, so until that contract
  test exists the invariant is prose and 0171's clients are unguarded. The
  reusable `partitions_totally` check in `port.rs` is the shape to lift.
  0194's contract-test criterion currently specifies only `create`→`show` and
  `update`→`show` round-trips, so it must be **widened** to include a
  `fetch_all` partition case — as written, the designated catcher would not
  catch the violation. 0171's real-client criterion inherits the same widening.
- **The empty-stamp trap.** `RemoteTimestamp` derives `PartialEq`, so two
  unknown stamps compare equal — and `work-item-sync-classify.sh:177` guards
  against exactly that with `[ -n "$base_remote_updated" ] &&` before its
  equality short-circuit. A classifier that compares stamps without checking
  emptiness first marks an item whose baseline was never written as already
  synced. The doc comment and
  `two_unknown_stamps_compare_equal_and_must_not_be_read_as_unchanged` carry
  the warning; the guard itself is 0194's to write.
- Deciding whether to widen `work_domain_imports_only_permitted` with
  `^tracker(::|$)` or to bridge the port through `work-adapters`. The sync state
  machine lands in `work`, whose rule does not currently permit `tracker`, so
  this surfaces as an enforcement failure mid-implementation — where the
  cheapest fix is also the one that erodes the boundary.
- The extra `show` per pushed item. `create` returns only an `ExternalId` and
  `update` returns `()`, so the post-push baseline write needs a read back. This
  matches the bash path and is accepted, not overlooked.

**Smaller**

- `work_adapters::filesystem` and `migrate_adapters` remain the only pup rules
  with no probe pair after Phase 3.

## References

- Original work item: `meta/work/0204-remote-tracker-port.md`
- Plan review driving this revision:
  `meta/reviews/plans/2026-08-11-0204-remote-tracker-port-review-1.md`
- Related research:
  `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Split from: `meta/work/0194-tracker-crate-and-remote-sync-engine.md`
- Consumer: `meta/work/0171-jira-and-linear-integrations.md`
- ADRs: ADR-0044, ADR-0045, ADR-0052, ADR-0053
- Manifest to copy: `cli/vcs/Cargo.toml:1-18`
- Error recipe with no `kernel` dependency: `cli/document/src/error.rs:1-42`
- `Display` recipe to copy verbatim: `cli/document/src/error.rs:3-24`
- Closed-set guard idiom: `cli/vcs/src/classify.rs:584-599`
- Nightly-lane task to mirror: `tasks/pup.py`, `tasks/deps.py:52-100`
- Nightly-lane isolation guard: `tests/unit/tasks/test_workflows.py:349-402`
- Registration-docs guard: `tests/unit/tasks/test_registration_docs.py`
- Fixture outside the crate: `cli/vcs-cli/tests/detect_goldens.rs:24-31`
- Fake/double house style: `cli/collaboration/src/base_repo.rs:96-198`
- String newtype with `Display`: `cli/config/src/key.rs:9-38`
- Test-file lint allow: `cli/verify/tests/verify.rs:4`
- Pup rule to copy: `cli/pup.ron:57-72`
- Pup rules that widen the baseline: `cli/pup.ron:96-102` (`work`),
  `cli/pup.ron:177-184` (`migrate`)
- Pup probe pair to copy: `tests/integration/pup/test_import_rule.py:218-310`
- Probe control with real imports:
  `tests/integration/pup/test_import_rule.py:549-552`
- Bulk read contract and its read-failure rule:
  `skills/work/scripts/work-item-fetch-remote.sh:20-48`
- Registration checklist: `tasks/README.md:322-474`
- Closest precedent plans:
  `meta/plans/2026-07-11-0179-corpus-crates-parsing-conventions.md`,
  `meta/plans/2026-08-06-0170-work-item-lifecycle-subdomain.md`
