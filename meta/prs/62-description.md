---
type: "pr-description"
id: "62"
title: "[0204] RemoteTracker Port"
date: "2026-08-12T13:30:41+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0204"
parent: "work-item:0204"
relates_to: ["work-item:0136", "work-item:0171", "work-item:0194"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/62"
pr_number: 62
tags: ["rust", "tracker", "sync", "port", "cargo-pup", "cargo-public-api"]
revision: "b75aeb8808a810f8c4220453aa846d186aec6136"
repository: "accelerator"
last_updated: "2026-08-12T13:30:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0204] RemoteTracker Port

## Summary

Remote-tracker sync exists today only in bash, under `skills/work/scripts/`. Both halves of its Rust replacement — 0171's Jira and Linear clients and 0194's sync engine — need the same contract, and whichever was built first would have defined it as a side effect. This PR lands that contract on its own: `cli/tracker/`, a zero-dependency domain crate holding the `RemoteTracker` port, its vocabulary and its error taxonomy, shipping no runtime behaviour and existing to be frozen. Holding it frozen turned out to need an instrument the repository did not have, so the PR also adds a `cargo-public-api` lane — which pins ten crates, not one, and found a real leak in `kernel` on its first run.

## Changes

- **`cli/tracker/`, a domain crate with no dependencies and no adapter sibling.** Six public items: `RemoteTracker` (`create`, `update`, `show`, `fetch_all`), `ExternalId`, `RemoteIssue`, `RemoteTimestamp`, `TrackerError` and `FetchOutcome`. No `[dependencies]` table and no `[dev-dependencies]` either, so `Display` and `std::error::Error` are hand-written as every other error type in the workspace does. The sync state machine lands in `work`/`work-adapters` (0194) and the provider clients in their own crates (0171); nothing here.
- **`fetch_all` returns a three-way partition rather than what it happened to find.** `FetchOutcome` carries `found`, `absent` and `indeterminate`, so a truncated bulk retrieval is representable. The alternative — a `Vec` the caller diffs against what it requested — infers absence from a short response, which is exactly the unsound step the bash path is built to avoid against a provider that truncates at 250 against ~180 items. `found` pairs each id with a `RemoteTimestamp` and not a whole issue: the bulk contract returns a stamp per key, and a mandatory body would force a conforming client either to fabricate an empty one (which by `RemoteIssue.body`'s own contract reclassifies every synced item as remotely modified) or to issue a `show` per id, destroying the bulk design.
- **`RemoteTimestamp` names its two unknowns.** `NotRead` and `NotReported` are distinct from `Reported(String)`, and `proves_unchanged_since` only answers true for a reported pair. Two unknown stamps compare equal under the derived `PartialEq`, which is the trap `work-item-sync-classify.sh` guards against by hand today; the type documents it and a test pins it, but the guard itself is 0194's to write.
- **The surface is pinned by `cargo-public-api`, replacing a self-reading test that failed three review passes.** Each pass found a different source shape the text parser mishandled — trait methods, derives, owner attribution, rustfmt-wrapped signatures, brace-struct variants, argument lines lifted out of a `write!` body. `cargo public-api` reads rustdoc JSON instead, so formatting and attribute order are invisible to it and a forbidden derive is caught semantically, as the impl it generates. `--include function-parameter-names` is load-bearing: without it a same-typed parameter swap (`create`'s `title`/`body`) renders identically.
- **The lane pins ten crates and forces a decision about every other member.** `_PINNED_CRATES` covers every domain crate plus the shared libraries (`collaboration`, `config`, `corpus`, `document`, `kernel`, `migrate`, `store`, `tracker`, `vcs`, `work`); `_EXEMPT_MEMBERS` records, with a reason each, why adapters, composition roots, `verify` and `vcs-test-support` are not pinned. A coverage guard in `tests/unit/tasks/test_rust.py` fails until a new workspace member appears in one of the two, so the omission that cargo-pup rules suffer from — silent absence of enforcement — cannot happen here.
- **The pin's first finding: `kernel::Error` was exposing `tracing_subscriber`'s parse error.** `#[error(...)] #[from] tracing_subscriber::filter::ParseError` put a foreign type in the public surface of the crate every other one depends on, so a consumer had to depend on `tracing-subscriber` to construct or match that variant. `LogFilter` now carries a `String`, `thiserror` is gone from `kernel` (and `tracing-subscriber` from `launcher`'s dev-dependencies), and `Display` plus an empty `Error` impl are hand-written with tests per arm — including one asserting the error chain ends here.
- **The nightly lane is now shared rather than owned by cargo-pup.** `PUP_NIGHTLY` becomes `RUST_NIGHTLY` behind a new `deps:install:nightly` task that every nightly-lane step depends on, directly or through its tool's install task, so the toolchain is provisioned once and two steps cannot race on `~/.rustup`. The two tools couple to it with different strength, and `tasks/shared/rust.py` and `tasks/README.md` both say so: cargo-pup carries a `rustc_private` driver, so `PUP_VERSION` and `RUST_NIGHTLY` move together; cargo-public-api has no driver, builds on stable and is coupled only through the rustdoc-JSON format.
- **CI gains a step, not a job.** `public-api:check` joins `check-architecture` — the sole nightly-consuming job — beside `pup:check` and `test:integration:pup`, because a job of its own would provision the same toolchain twice. It is wired into both the aggregate `check` and the bare `default` task, and `tests/unit/tasks/test_workflows.py`'s name-based isolation guard learns its markers.
- **A cargo-pup rule for `tracker`, and probe pairs backfilled for four rules that had none.** The rule permits only `std`/`core`/`alloc` and `crate`, deliberately omitting the `^kernel::Error(::|$)` allowance its sibling domain rules carry — with no dependencies table such an import cannot compile, so the line would be inert and would misdescribe the crate. Phase 3 then backfills the missing probes for `corpus`, `vcs`, `work` and `migrate`, taking `tests/integration/pup/test_import_rule.py` from 16 to 39 cases: a violation, a compliant control, `kernel::Error`, each rule's own extra allowance, and cross-rejection of an allowance that belongs to a different rule.
- **`tasks/README.md` gains a "Registering a library crate" checklist** — five points, with `cli/tracker/` as the worked example — alongside the existing thirteen-point sub-binary one, plus a rewritten nightly-lane section documenting the per-tool coupling and how to read a snapshot diff caused by a dependency bump rather than a first-party edit.
- **Handoff edits to 0136, 0171, 0194 and 0203.** 0171's and 0194's descriptions of the port were stale in different ways (both understated the item count; 0171's end to end) and now name the real six-item signature; `blocked_by: work-item:0204` is cleared from both now that the port is accepted. 0171 also picks up the four exit-code mapping tables, the identifier-safety check and the four bridge capabilities left above the port. The epic's child list gains 0203 and 0204.

## Context

Implements `work-item:0204` (RemoteTracker Port), split out of `work-item:0194` on 2026-08-10 so the contract could be reviewed without the sync engine attached. Researched in `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`, planned in `meta/plans/2026-08-11-0204-remote-tracker-port.md`, reviewed at work-item and plan level (`meta/reviews/work/0204-remote-tracker-port-review-1.md`, `meta/reviews/plans/2026-08-11-0204-remote-tracker-port-review-1.md`), and validated in `meta/validations/2026-08-11-0204-remote-tracker-port-validation.md` (result: pass, no undocumented deviations).

Sits under the `work-item:0136` Rust CLI migration epic. Unblocks `work-item:0171` (Jira and Linear client adapters, plus the work-item cutover) and `work-item:0194` (sync engine) simultaneously — that simultaneity is the whole reason the split happened.

Eight deviations from the work item's frozen block were settled across three planning passes, all recorded in the plan and reconciled into 0204 before implementation began. The two that changed the shipped signature: `fetch_all` gained `FetchOutcome` and a key argument, and `found` carries a timestamp rather than an issue. The reversal worth naming is deviation 8 — an earlier pass had rejected `cargo public-api` on the premise that it meant a third toolchain, which was wrong twice over.

## Testing

- [x] `mise run check` — the full read-only CI mirror across frontend, server, cli, deny, pup, public-api, build-system and scripts — exits 0 at the branch tip.
- [x] `cargo nextest run -p tracker` — 29/29 across `vocabulary.rs`, `errors.rs`, `port.rs`, `structure.rs`. `port.rs` drives a fake through a `dyn RemoteTracker` because the sync engine's composition root will hold one, so any signature move stops it compiling; `structure.rs` checks what rustdoc JSON cannot see (no dependencies, no behaviour, no adapter sibling).
- [x] `mise run public-api:check` — all ten pinned crates match their committed snapshots. The `tracker` snapshot was hand-written from 0204's frozen block before `src/lib.rs` existed, so it started red rather than characterising whatever was built.
- [x] `mise run test:integration:pup` — 39/39, the four backfilled rules included. Each probe drives the shipped `cli/pup.ron` against a synthetic workspace, so a deleted rule or a mistyped anchor reddens it.
- [x] `mise run test:unit:tasks` — 757/757, covering the new `deps:install:public-api` presence probe (token equality on the version line, not a substring match), the pinned-or-exempt coverage guard, and the nightly-lane isolation guard.
- [x] `mise run deny:check`, `mise run build-system:check` — clean.
- [x] Every one-shot mutation check the plan specifies was performed with the expected red observed: rule deletion, anchor corruption, variant rename, parameter swap, derive addition, dependency-table addition.
- [ ] **One guard is confirmed absent rather than passing.** A fifth required trait method given a default body is caught by neither `public-api:check` nor `port.rs` — a client would silently inherit it instead of failing to compile. Established by mutation, recorded, and handed to 0171/0194 rather than papered over.
- [ ] Not verified this session: the Linux CI lane, and a `RUST_NIGHTLY` bump against `PUBLIC_API_VERSION`'s rustdoc-JSON support (the procedure is documented in `tasks/README.md`; nothing exercises it until the next bump).

## Notes for Reviewers

- **`FetchOutcome`'s totality is prose, not a type invariant.** A `FetchOutcome::partition(requested, found, complete)` constructor would make an unsound absence unrepresentable, but AC 10 forbids function bodies in `src/` beyond the ones it names, and that criterion is deliberate — the crate is a contract, not a library. The cost is stated rather than hidden: 0171's clients get written before any mechanism exists to catch a violation, and 0194 must widen its contract test (currently `create`→`show` and `update`→`show` only) to include a `fetch_all` case, or the designated catcher will not catch it.
- **The bash oracle and the frozen contract disagree about `Retryable`, and the code follows the oracle.** 0204's Requirements say retryable requires *provable* absence of transmission; `work-item-bridge-codes.sh` scopes 70 to "before any remote mutation", and Jira's retryable set includes 4xx rejects that plainly transmitted. Classification is therefore documented as operation-scoped on `TrackerError`, and 0171 is told to port the four mapping tables verbatim rather than reason from the rule and arrive somewhere more precise. Linear code 34 is the worked example: retryable on `create`, terminal on `update`.
- **One post-validation change is worth a second look.** `tracker/tests/errors.rs` originally parsed `skills/work/scripts/work-item-bridge-codes.sh` to hold the taxonomy 1:1 against the bash declarations. That reached out of `cli/` into a tree the bridges' own retirement will delete, so the test now pins the code numbers in its fixture and asserts the two classes against them directly. It trades a live oracle for a committed one, deliberately — say if you would rather keep the coupling until 0171 cuts over.
- **`kernel` losing `thiserror` is a wider blast radius than the crate it sits in.** It is the crate everything depends on, so the change touches every consumer's error surface, and `swallow_under_fail_safe`'s test in `launcher` had to stop constructing a real `ParseError` to build a `LogFilter`. Behaviour is unchanged (the rendered message still reads `invalid log filter: …`), but the variant now carries what the parser said rather than the parser's own type.
- **`work`'s pup rule does not permit `tracker`.** The sync state machine lands in `work`, so 0194 will hit this mid-implementation, where the cheapest fix — widening the rule — is also the one that erodes the boundary. Flagged in the plan's handoffs rather than pre-resolved here, because bridging through `work-adapters` is the alternative and it is 0194's call.
- **Five doc-comment lines in `cli/tracker/src/lib.rs` run to 81 characters.** `rustfmt` does not reflow doc comments (`wrap_comments` is off), so nothing in the repo catches this. Left as-is; call it if you want it wrapped.
- **`work_adapters::filesystem` and `migrate_adapters` remain the only pup rules with no probe pair.** Both are module-scoped rather than whole-crate, do not fit the parameterised writer Phase 3 uses, and stay unprobed as follow-up.
- **History reviews commit by commit**: frozen-surface reopening → crate, vocabulary and pup rule → port trait and partition type → probe backfill → handoff edits → the public-API pin and the `kernel` leak it found → validation → the bash-oracle decoupling and comment trim.
