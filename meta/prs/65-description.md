---
type: pr-description
id: "65"
title: "[0194] Tracker crate and remote sync engine"
date: "2026-08-13T19:46:29+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0194"
parent: "work-item:0194"
relates_to: ["work-item:0170", "work-item:0171", "work-item:0174", "work-item:0204"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/65"
pr_number: 65
tags: [rust, work-items, sync, tracker, nextest, cargo-pup]
revision: "62cd925385c541254d9d0de518eaed3b5191e64d"
repository: "accelerator"
last_updated: "2026-08-13T19:46:29+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# [0194] Tracker crate and remote sync engine

## Summary

Remote-tracker sync exists today as nine bash scripts under `skills/work/scripts/`. This PR lands their Rust replacement against the `RemoteTracker` port 0204 froze: a pure state machine in `work::sync`, persistence and apply in `work_adapters::sync`, and an `accelerator work sync` command wired end to end. Nothing is removed and no user-facing path changes — the bash engine stays live beside this one, and 0171 performs the cutover.

One consequence is worth stating up front: no provider client exists yet, so `work sync` against any configured tracker exits 72 (recognised, no client built) or 73 (unset or unrecognised). The command is complete; what it talks to is 0171's.

## Changes

- **`cli/tracker-test-support/`, a recording fake plus a contract harness every implementation must satisfy.** Rust has no compile-time assertion that a trait method is *required*, so an implementor that quietly inherits a default body is invisible to both the port's types and a trait-object test; per-operation behavioural assertions are the only available check. Every harness function takes a `ContractSubject` rather than a bare `&dyn RemoteTracker`, because truncation and a lost write are conditions an implementation must be *induced* into: the fake configures itself, and a real client will nominate ids through `unaccountable_id`/`unreadable_id` rather than being asked to misbehave.
- **Contract suites are excluded from the default test run by binary name, behind two independent gates.** `cli/.config/nextest.toml` sets `default-filter = 'not binary(=contract)'` — the exact-match form, since bare `binary(contract)` is a substring predicate that would also capture a future `contract_helpers`. Filtering by binary rather than by crate keeps a contract crate's own unit tests running in the default pass. The second gate is `ACCELERATOR_TRACKER_CONTRACT=1`, owned by the harness itself: `run_all` *errors* rather than skips when it is unset, and returns a count of properties executed the caller must assert non-zero, because a count assertion alone cannot distinguish a closed gate from an empty run. Belt and braces, because a filter is one line of config standing between a plain `cargo nextest run` and a harness that will make live remote calls once real clients exist. `mise run test:integration:tracker-contract` selects the opt-in profile and sets the variable; `tests/unit/tasks/test_nextest_filter.py` pins the filter's exact spelling and that `tasks/test/cli.py` passes neither `--profile` nor `--ignore-default-filter`.
- **The pure state machine lands in `work::sync`: classify, decide, label, plan, push_decide, push_precondition and state.** No filesystem, no subprocess, no tracker call — those live in `work-adapters`. `classify` defaults each side to changed when it cannot prove otherwise, so a missing baseline or an unreadable stamp yields a conflict a human resolves rather than a silent overwrite. `plan` reads its `--resolve` map without validating it, so a stale or misdirected order is inert rather than an error.
- **`Dirtiness` is three-valued, and the third value is why some golden rows are Rust-only.** `Unknown` decides as `Dirty` everywhere, since VCS revert cannot recover the uncommitted working-copy changes an overwrite would destroy. The bash harness has only a two-valued test, so it would read `unknown` as clean — the opposite of the fail-safe. Those rows sit in `[DECIDE_RUST_ONLY]` in the shared golden and are labelled as inexpressible there, not as a disagreement between two implementations over a shared input.
- **Conflict-token resolution folds and trims in ASCII only.** A Unicode-aware `to_lowercase()`/`trim()` resolves a leading U+00A0 to `AcceptRemote`, turning the deliberately safe default into a local overwrite for whitespace no human can see. The golden marks that row `(nbsp)` because it is otherwise invisible in review.
- **`work_adapters::sync` holds the imperative half: digest recipes, the baseline document and its store, the fetch shell, per-item apply, the pending-push marker, and the whole-corpus run.** The digest path carries its own fence recogniser rather than reusing `document::fence::split`, which compares the exact bytes `---` and caps its scan at 1 MiB — a fence carrying a trailing space would make it treat the whole file as body, silently promoting a routine `last_updated` restamp into a content change.
- **Reads are bulk-then-`show`: one `fetch_all`, then a `show` only for ids whose stamp does not prove them unchanged.** `--per-item-reads` opts out entirely. A `fetch_all` pre-flight failure marks every present id `Indeterminate` and is carried through to the report, because discarding it turns a misconfigured token into a whole-corpus "nothing to do".
- **Apply writes the baseline entry last, and `pull` derives both hashes from post-overwrite state.** A failed `update` therefore leaves the entry unset and the next run reclassifies from scratch; a failed post-push `show` still writes the entry, with both remote fields empty and `RemoteTimestamp::NotRead` — the only place that variant is written. Deriving either hash from pre-pull content would self-corrupt the baseline into a phantom `locally-modified` on the next run.
- **The baseline re-reads before every render, and `finalise_run` blanks-then-advances as one operation.** Holding one in-memory document for a whole run would widen the lost-update window from a single write to the entire run while the bash engine is still live beside this one. The blank-then-advance ordering is load-bearing and a two-call API could be called in the wrong order or half-called; both mutations reach one document before the single write, so a failure loses neither in isolation. The read side is injected as well as the write side — with a write-only seam, a spy would see the writes while reads still came from disk, so successive `set` calls would each start from the pre-run document.
- **Write bounds refuse before any write, in preview as well as apply.** `--max-pulls` and `--max-pushes` default to 25 and exit 5. Preview refuses too: a preview that reported an over-threshold plan and exited 0 would mispredict exactly the run with the largest blast radius.
- **A pending-push marker makes `create --push` survive a crash between the remote create and the local write.** Its decision table is sited in the domain because a wrong branch binds two local work items to one remote issue, which neither VCS revert nor a re-run recovers from. `Absent` and `Unreadable` stay distinct, so a crash mid-write cannot read as "no previous attempt" and re-issue a non-idempotent create. The request fingerprint is hashed length-prefixed rather than concatenated, since undelimited `title + body + kind` is not injective. A second guard asks whether the corpus already carries the marker's `external_id`, closing the hazard the fingerprint alone cannot: a crash between the local write and the marker delete leaves a `Created` marker whose fingerprint still matches, and a re-run would then write a second file carrying the same id.
- **`accelerator work sync` is non-interactive and its report on stdout is authoritative.** Tab-separated `id action state detail` rows, with `--push-only`, `--pull-only`, `--preview`, `--resolve`, `--per-item-reads` and the two bounds. `awaiting_human` is derived from the report rather than stored, because a stored field could disagree with what was printed and yield a run that reports conflicts and exits 0. The exit-code taxonomy lives in one module and the `--help` text spells it out, including that a 71 run may also carry conflicts, so the report must be checked regardless of exit code.
- **`--push` on `create` and `update`, with deliberately different failure semantics.** `create --push` still writes the file either way; `update --push` leaves the local file untouched, exiting 70 unchanged or 71 with the baseline entry cleared. `sync` also warns about outstanding pending-push markers after a run.
- **Both implementations read one oracle rather than two transcriptions.** The classify table, the decide, label and push-decide goldens live under `skills/work/scripts/test-fixtures/` and are looped by the bash suites and the Rust suites alike, so a row cannot be edited on one side and left stale on the other. The baseline corpus is captured from live bash by its own `regenerate.sh`, never from the Rust recipe, and `sync_baseline_shellout_parity` shells the live scripts to assert the corpus still matches them right now — without it, a corpus regenerated from the port would agree with itself and the whole suite would become vacuous.
- **cargo-pup and cargo-public-api learn the two new crates.** `work` may now import `tracker` (a zero-dependency port crate, so the edge drags no transitive graph into the domain); `tracker-test-support` may import `tracker` alone, with a probe pair in `tests/integration/pup/test_import_rule.py` driven against crates literally so named, so the hyphen-to-underscore matcher is exercised directly and the compliant control actually imports something. `tracker-test-support` joins `vcs-test-support` as public-api-exempt under a shared reason.
- **Every comment on the branch was then held to the repository's comment policy.** Citations of bash scripts that are gone or that the cutover removes, plan-phase and acceptance-criterion references, and comments restating what the code already says are removed; the sweep also covers pre-existing occurrences across the workspace. Two claims that had gone stale are corrected — `filter_frontmatter_keys` and `project_remote` both have callers now.

## Context

- Work item: `meta/work/0194-tracker-crate-and-remote-sync-engine.md`
- Plan: `meta/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine.md`
- Plan review: `meta/reviews/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine-review-1.md`
- Research: `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- Depends on the port frozen in #62 (0204). The cutover — removing the nine migrated scripts and repointing the skills — is 0171's.

## Testing

- [x] `mise run check` exits 0 across all four components.
- [x] `mise run test:integration:tracker-contract` — 3 properties, all passing, with the env gate open.
- [x] The full Rust workspace under `cargo nextest run`: 2014 of 2017 pass.
- [x] The `bash-parity` suites under `--all-features`, which the default run skips: 159 pass, including `sync_baseline_shellout_parity::every_case_matches_live_bash_right_now`, which proves the committed corpus still matches the live scripts.
- [x] The bash suites the shared goldens now drive: `test-work-item-scripts.sh` (121) and `test-work-item-create-remote.sh` (46).
- [x] Comment-sweep collateral re-run: `hooks/test-vcs-detect.sh` (68), `test-skill-frontmatter-population.sh` (170), `test-skill-frontmatter-conformance.sh` (96), `test-jira-init-flow.sh` (48).
- [ ] Three tests fail only under full parallel load and pass in isolation in about a second each: `api_smoke::api_surface_is_fully_reachable_against_fixture_meta` and two `github::octocrab_client` body-update tests. Each takes 25–30s in a 2017-test run. Every changed line in those crates is a comment, verified mechanically, so these look like the existing local-HTTP-server starvation flakes rather than regressions — but I did not run the suite at `main` to confirm that directly.
- [ ] `work sync` against a real tracker. Not reachable: no provider client is wired, so every provider resolves not-available.

## Notes for Reviewers

**The acceptance criteria moved during implementation, in one place worth checking.** A terminal push failure clears the item's baseline entry, and the work item previously said the next `sync` would classify it `indeterminate`. It classifies `conflict`: a cleared entry leaves both hashes absent, both sides default to changed, and the 2x2 verdict is `conflict`. `indeterminate` is reachable only from a failed or truncated remote *read*, so no local bookkeeping can produce it. The safety property is unchanged — neither side is written under a bidirectional run — but `conflict` is *resolvable*, so a later `--resolve <id>=remote` can pull the remote over the local for an update that may never have applied. That was accepted rather than mitigated, because the alternative persists an uncertainty flag into a baseline schema the live bash engine also reads.

**Three fixture families were deliberately not lifted**, and the work item now records why: the bridge-script tables belong to 0171's clients, the baseline section's file-atomicity cases cannot be expressed as a table and became typed unit tests plus the bash-generated corpus, and the fixtures live under `skills/work/scripts/test-fixtures/` rather than in Rust-only test data so one oracle holds both implementations while the bash path stays live.

**`cli_sync.rs` covers less than it looks like it should.** `accelerator-work` is bin-only, so a subprocess cannot inject a fake `TrackerRegistry`; the suite covers provider selection, usage errors and non-interactivity, and the fake-tracker scenarios — the two-invocation conflict loop, classification stability end to end, the write-bounds boundaries — need a `[lib]` target `work_adapters::sync::run` can be driven through. Same constraint bounds `cli_create_push.rs` and `cli_update_push.rs`.

**One piece of dead duplication to decide on.** `work_cli::exit_codes::for_tracker_error` is `#[allow(dead_code)]` while `create.rs` hand-rolls an identical `dispatch_code_for_tracker_error`. The stale comment claiming the wiring lands later has been removed, but the duplication is a code fix I left alone.

**Two commits you may want to treat separately.** `Sync tracker-test-support's lockfile entry with the workspace version` is its own change: the crate was committed pinned at `1.24.0-pre.36` while `cli/Cargo.toml` declares `pre.38`, so a `--locked` build could not resolve. Abandoning that commit drops the fix without touching anything else. The comment sweep is likewise isolated, and in `hooks/test-vcs-detect.sh` it changes eight lines of printed test output, not only comments, because the criterion numbers were also test labels.

**The branch is named `0194-remote-sync-engine`, not `0194-tracker-crate-and-remote-sync-engine`.** That name belongs to merged PR #50, and reusing it would attach this PR to a branch GitHub already considers merged.
