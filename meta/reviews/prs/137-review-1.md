---
type: "pr-review"
id: "137-review-1"
title: "[0230] Let the tracker own work item IDs, with local drafts promoted on sync"
date: "2026-10-07T08:51:39+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:137"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "safety", "compatibility", "security", "documentation"]
review_number: 1
pr_number: 137
tags: []
last_updated: "2026-10-07T08:51:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #137 - [0230] Let the tracker own work item IDs, with local drafts promoted on sync

**Verdict:** COMMENT

The domain core is well modelled. Promotion is a pure, durably recorded state machine, retirement planning is a pure function, the new corpus ports are small and well named, and the tests are proportional to the risk, with fault injection, kill-and-resume and real-lock concurrency. The problems are at the edges of the duplicate-issue and identity guarantees:
- Tracker keys are not checked to be tracker-key shaped before they become paths and IDs.
- The prose rewriter's token boundary disagrees with the key grammar.
- The batch journal and the HTTP transport each have a path to a second create.

Outside the core, create orchestration and exit-code policy have built up in `work-cli`, and the breaking outcome changes are missing from CHANGELOG. No finding is critical.

### Cross-Cutting Themes

- **Tracker keys are not validated as tracker keys at the identity boundary** (flagged by: security, correctness, architecture) — Keys from the tracker pass only `identifier_is_safe`, which permits `/`, `..` and `"`, before they become file paths, recovery-directory names (later passed to `remove_dir_all`) and quoted YAML. `corpus::is_tracker_key` exists but is only used to warn. The key grammar also has three definitions (`is_tracker_key`, `TRACKER_SCAN_REGEX`, `normalise_id`), and the prose rewriter's `joins_a_token` disagrees with all of them about `_`. One `TrackerKey` newtype, built only through `is_tracker_key`, would close the security, correctness and visualiser findings together.
- **Holes at the edges of the never-a-second-create guarantee** (flagged by: safety, correctness, test-coverage) — The state machine itself is sound. Around it:
  - Transports retry `POST` on 5xx.
  - The batch journal is keyed only by content digest, written without a lock, and a failed journal write does not stop the push.
  - The kill-after-send boundary in the resume test skips the `creates == 1` assertion.
  - The "any record stage" rerun test reaches only two of the five stages.
- **Exit-code protocol has four sources of truth** (flagged by: architecture, code-quality) — 1/4/70/71/75 are private constants in `work::promotion`, `work::create_batch`, `work::sync::push_decide` and `work-cli::exit_codes`, and the precedence order lives elsewhere again.
- **`IdentityWorkspace` does not actually own port assembly** (flagged by: architecture, code-quality) — `promote`, `sync` and `create` each build the stores and wire `SettlementPorts`, and `create` uses different containment roots.
- **Breaking changes and the downgrade hazard are recorded only in the PR description** (flagged by: documentation, compatibility, safety) — Neither `CHANGELOG.md`'s `### Breaking` section nor the sync guide covers these:
  - outcome keywords and exit codes that change under the legacy patterns;
  - `work resolve`'s new ambiguity and case-folding behaviour;
  - the Jira CLI's 21 → 34 change;
  - the fact that an older binary cannot read promotion records.
- **Errors swallowed on promotion paths** (flagged by: code-quality, safety) — Four places discard an error:
  - `CreatedUnwritten` loses its cause.
  - A frontmatter split failure becomes an empty body sent to the tracker.
  - The promotion-claim guard silently disables itself when config is unreadable.
  - Several cleanup errors are dropped with `.ok()` / `let _`.

### Tradeoff Analysis

- **Architecture vs delivery scope**: moving create orchestration out of `work-cli` into a `work-adapters` service is the right end state, but this PR is already about 33.5k lines. Recommendation: record it as a follow-up alongside a convergence plan for the legacy marker and promotion-record pipelines, and don't block on it.
- **Safety vs corpus tidiness**: keeping retirement recovery copies for one more run would give an undo for unversioned files, at the cost of state clutter. Recommendation: keep them, and list the rewritten paths in the output. A retirement runs unattended during `work sync`, so being able to find and undo what it changed is worth more than a tidy state directory.
- **Compatibility vs correctness in `work resolve`**: an exact-`id`-wins precedence would preserve legacy behaviour but hide genuine ambiguity. Recommendation: keep the ambiguity error under `{tracker}`, let an exact `id` match win under the numeric patterns, and record the change either way.

### Strengths

- ✅ Promotion is an explicit state machine (`PromotionStage` → `next_step`/`resume`) in the pure `work` crate. Each stage is recorded durably before its side effect. Unreadable records are kept distinct from absent ones, and a second create needs an explicit `--adopt`/`--create`.
- ✅ `plan_retirement` is a pure snapshot-in, plan-out function that re-plans correctly from all three interrupted points. `apply_retirement` checks every snapshot under per-file locks, and rollback never overwrites a file someone else has changed since.
- ✅ The new corpus ports (`ExclusiveCreate`, `RecoveryCopies`, `ExclusiveLock`/`LockName`) are small, use domain names, live in the zero-dependency `corpus` crate and follow the pup rules.
- ✅ `send_create` gives the legacy and promotion paths one classified create-and-retry policy. Only a connect failure on the first attempt counts as `NotSent`, and `TrackerError::Rejected` keeps the taxonomy closed.
- ✅ Test depth matches the risk. Every pure decision has table-driven tests. Fault injection fails at every store operation. Concurrency tests can tell a working lock from a broken one. Keyword and exit-code contracts are pinned to frozen literals, and P1–P9 each have a regression test.
- ✅ Jira paths are percent-encoded and checked for traversal, Linear identifiers go in GraphQL variables, recovery copies use `create_new`, the state directories ignore themselves, and draft suffixes come from a CSPRNG.
- ✅ Prose rewriting uses literal matching and leaves bare numeric IDs alone outside typed links, which keeps false positives unlikely.
- ✅ The CLI help is pinned by `cli_surface.golden`, the skill changes come with targeted evals, and ADR-0070 answers each of ADR-0044's objections.

### General Findings

- 🟡 **Documentation / Compatibility**: CHANGELOG.md has no `### Breaking` entries for the outcome and exit-code changes, `work resolve` behaviour, the Jira CLI exit code, or the downgrade hazard. It also has no `### Added` entry for `{tracker}`, `promote`, `create-batch`, `--no-promote` or `aliases`.
- 🔵 **Architecture**: two parallel durable-create pipelines (the legacy `pending_push` marker and `PromotionRecord`) now coexist, with no planned convergence.
- 🔵 **Code Quality**: `(integrations_root, integration)` is passed around as an untyped pair. An `IntegrationState` value type would fit it.
- 🔵 **Code Quality**: many new `///` comments on private items restate the code, contrary to the repository's comment policy.
- 🔵 **Code Quality / Safety**: best-effort cleanups discard their errors without a trace (`remove_file(...).ok()`, `let _ = records.remove`, `retirement_outstanding(...).unwrap_or(true)`).
- 🔵 **Safety**: promotion and retirement records carry no version field, so an older binary fails with no guidance. The error should name the minimum binary version.
- 🔵 **Test Coverage**: live tracker behaviour (`locate`, moved issues, `NotSent`/`Rejected`) is verified only against hand-written fixtures until `test:integration:tracker-contract` runs.
- 🔵 **Security**: a manifest's `body_file` accepts absolute paths and `..`. A prompt-injected `extract-work-items` run could push a local secret to the tracker.

### Additional Findings

- 🟡 `cli/work-cli/src/promote.rs:201` — identity port assembly is copied across `promote`, `sync` and `create` (architecture, code-quality)
- 🟡 `cli/work/src/promotion.rs:229` — the exit-code taxonomy is restated as private magic numbers (architecture, code-quality)
- 🟡 `skills/work/sync-work-items/SKILL.md:422` — the promotion and identity-pass rendering has no evals (test-coverage)
- 🔵 `cli/work-adapters/src/promotion.rs:629` — a key not shaped like a tracker key only warns, then promotion proceeds (security)
- 🔵 `cli/work/src/retirement.rs:558` — IDs are written into YAML double quotes without escaping (security)
- 🔵 `cli/work-adapters/src/promotion.rs:564` — `--adopt` records the key the user typed, not the tracker's canonical key (correctness)
- 🔵 `cli/work-adapters/src/retirement.rs:364` — a resumed retirement reports `RolledBack` while steps from the earlier attempt remain, and drops its record (correctness)
- 🔵 `cli/work/src/retirement.rs:315` — the bare-filename path spelling breaks sibling links from other drafts (correctness)
- 🔵 `cli/work/src/retirement.rs:586` — appending an alias to a zero-indented YAML block list produces invalid YAML (correctness)
- 🔵 `cli/work-cli/src/create_batch.rs:314` — a failed batch-journal write does not stop the create it guards (safety)
- 🔵 `cli/work-cli/src/batch_journal.rs:165` — the batch journal is read-modify-write with no lock (safety)
- 🔵 `cli/work-adapters/src/retirement.rs:531` — a successful retirement deletes its recovery copies straight away and doesn't list the rewritten files (safety)
- 🔵 `cli/work-adapters/src/sync/pending_push.rs:412` — the downgrade hazard is stated only in the PR description (compatibility)
- 🔵 `cli/work-cli/src/resolve.rs:129` — `work resolve` now looks identities up case-insensitively first, under every pattern (compatibility)
- 🔵 `cli/jira-client/src/mutation.rs:89` — the standalone Jira CLI exit code for invalid requests changes from 21 to 34 (compatibility)
- 🔵 `cli/corpus/src/lock.rs:12` — the documented lock order includes a create lock that the port doesn't model (architecture)
- 🔵 `cli/tracker/src/lib.rs:819` — the tracker-owned-ID capability is keyed on provider names, and the key grammar is duplicated (architecture)
- 🔵 `cli/work-adapters/src/retirement.rs:60` — retirement ports depend on the concrete `BaselineStore` and on the `sync` module (architecture)
- 🔵 `cli/work-adapters/src/promotion.rs:93` — report-row presentation types live in the promotion service module (architecture)
- 🔵 `cli/work-adapters/src/promotion.rs:707` — `CreateOutcomeUnknown` is also the catch-all for internal invariant violations (code-quality)
- 🔵 `cli/work/src/retirement.rs:30` — "retire onto a tracker key" is rebuilt field by field in seven places (code-quality)
- 🔵 `cli/work-cli/src/promote.rs:34` — CLI command modules depend on each other's internals (code-quality)
- 🔵 `cli/work-cli/src/sync_author.rs:103` — the promotion-claim guard silently disables itself when config can't be read (code-quality)
- 🔵 `cli/work-cli/src/sync_author.rs:116` — the "which keys do live promotions claim" rule is implemented three times (code-quality)
- 🔵 `cli/work-cli/src/create.rs:1172` — `promotion_outcome` layers four mappings over one enum (code-quality)
- 🔵 `cli/work-cli/src/promote.rs:227` — errors are shown to users in `Debug` format (code-quality)
- 🔵 `cli/work/src/retirement.rs:577` — a hand-rolled YAML line editor is embedded in the retirement planner (code-quality)
- 🔵 `cli/work-cli/src/create_batch.rs:290` — `Batch::create` mixes journalling, creation and parent-link annotation (code-quality)
- 🔵 `cli/work-adapters/tests/promotion.rs:963` — the hard-coded `0..=10` kill range does not check that it reached every operation (test-coverage)
- 🔵 `cli/work-cli/src/create.rs:2178` — the "any record stage" rerun test covers only `Attempted` and `Created` (test-coverage)
- 🔵 `cli/work-cli/tests/keyword_exit_codes.rs:104` — the frozen reason/exit table is not checked against the skill's table (test-coverage)
- 🔵 `cli/work-cli/src/batch_journal.rs:314` — the 30-day retention boundary is not pinned (test-coverage)
- 🔵 `docs-site/src/content/docs/guides/sync-work-items.mdx:178` — "never rewritten" leaves out that the next sync pushes the key H1 or raises a conflict (documentation)
- 🔵 `cli/work-cli/src/cli.rs:93` — the `next-number` help under `{tracker}` suggests its draft IDs are reserved (documentation)
- 🔵 `cli/work-cli/tests/fixtures/cli_surface.golden:185` — `work list --help` describes the Sync column condition wrongly (documentation)
- 🔵 `docs-site/src/content/docs/guides/configuration-cookbook.md:174` — the cookbook overstates where an invalid pattern is refused, and calls `--project` "unused" when it is rejected (documentation)
- 🔵 `skills/work/create-work-item/SKILL.md:602` — the new outcome-table rows lose their indentation and break the table (documentation)
- 🔵 `skills/work/create-work-item/SKILL.md:553` — "draft" now means two different things on the same push gate (documentation)
- 🔵 `skills/work/refine-work-item/SKILL.md:194` — "continue at step 3" reads as the failure branch, and the warning override is out of order (documentation)
- 🔵 `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md:8` — the body still says `**Status**: Accepted` (documentation)
- 🔵 `skills/work/create-work-item/evals/benchmark.md:45` — evals 35–36 are missing from the results list (documentation)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/corpus/src/references.rs:267-269` — Token boundary ignores `_`, so retiring one key rewrites a different key
**Severity**: major | **Confidence**: high | **Lens**: correctness

🟡 **Correctness**

`joins_a_token` (`references.rs:106`) treats only ASCII alphanumerics and `-` as token characters. `corpus::is_tracker_key` accepts `_` in the project prefix, and `an_underscore_key_is_distinctive_and_rewritten_in_prose` confirms that `MY_PROJ-12` counts as distinctive.

That means retiring `PROJ-12` → `ENG-3` also matches inside `MY_PROJ-12`, because the preceding `_` is not a token character, and rewrites it to `MY_ENG-3`. The same happens at the trailing edge: `PP-760_notes` becomes `ENG-42_notes`. `without_ids` uses the same matcher, so content-digest normalisation is affected too.

**Impact**: In a Jira tenant with projects such as `PROJ` and `MY_PROJ`, a key change or promotion silently corrupts typed links and prose references to an unrelated issue across `meta/`.

**Suggestion**: Add `b'_'` to `joins_a_token`, and add a test that `MY_PROJ-12` survives retiring `PROJ-12`.

---

### `cli/work-adapters/src/sync/identity_settlement.rs:549-553` — Key from the tracker becomes a filesystem path and a `remove_dir_all` target without shape validation
**Severity**: major | **Confidence**: medium | **Lens**: security

🟡 **Security**

Under `{tracker}`, the identity pass retires an item's `id` onto `RemoteIssue.key`. Before that key gets here, it is checked only by `identifier_is_safe`, which rejects control characters, a leading `---` and a leading `#`, but accepts `/`, `..`, `"` and a leading `/`. `new_id` is then used directly to build three things:
- the target file, `work_dir.join("{id}-{slug}.md")` (a leading `/` makes `Path::join` discard `work_dir` entirely);
- `retirement-records/{old}--{new}.json`;
- `retirement-recovery/{old}--{new}`, which `finish_retirement` and `roll_back` pass to `fs::remove_dir_all`.

**Impact**: A key such as `PP-1/../../../..` points the recovery directory at the repository root. A compromised or misconfigured tracker endpoint, or a spoofed response, can then make `work sync` write outside `meta/work/` and recursively delete outside `.accelerator/state/`. The same unchecked key reaches promotion (`promotion.rs:629` only warns on an odd shape) and the unescaped YAML scalars in `retirement.rs:558`.

**Suggestion**: Refuse any key for which `corpus::is_tracker_key` is false at the point it becomes an identity: `KeyChange::RetireKey`, a promotion's `Retirement`, and `--adopt`. Ideally, make that a `TrackerKey` newtype that can only be built through the check. As defence in depth, have `Retirement::recovery_dir` and `file_for` reject separators and `..`.

---

### `cli/work-cli/src/batch_journal.rs:135-146` — Journal rows keyed only by content digest: identical batch entries overwrite each other
**Severity**: major | **Confidence**: medium | **Lens**: correctness

🟡 **Correctness**

`record` replaces whichever row has the same `content_digest`. That digest is `request_digest(title, body, kind)`, so it does not include the entry's `ref` or `parent`.

Take a manifest with two entries that share title, kind and body, for example "Update docs" under two different epics. The second entry's journal row overwrites the first. On a rerun, the first entry claims the surviving row and is reported as the second entry's item, and its children are linked to that item's ID. The second entry finds no row and is created again.

**Impact**: Resuming a partially failed pushed batch can create a duplicate tracker issue and attach children to the wrong parent.

**Suggestion**: Key rows by `(batch fingerprint, ref)`, falling back to the digest only for legacy rows whose `batch` is empty.

Safety raised two related issues: a failed journal write does not stop the push (`create_batch.rs:314`), and the journal is read-modify-write with no lock (`batch_journal.rs:165`).

---

### `cli/jira-client/src/transport.rs:333-342` — Transport still retries non-idempotent creates on 5xx, beneath the duplicate-prevention guarantees
**Severity**: major | **Confidence**: medium | **Lens**: safety

🟡 **Safety**

`send_create` retries "only when the first failure proves no issue was created". Underneath it, `Transport::execute` still retries every request, `POST /issue` included, on 429 or any 5xx (`is_retryable_status`, `500..600`). The Linear transport retries mutations on 5xx too.

A 502 or 504 from a gateway can arrive after Jira has already created the issue. The transport then sends a second create and returns `Created(<second key>)`, so the promotion record never learns that the first issue exists. The new `send_failure` treats a connect failure after such a response as possibly applied, but by then the retry has already gone out.

This retry behaviour predates this PR, but the PR's guarantees now rest on it.

**Impact**: The tracker can end up with an orphaned duplicate issue that nothing locally records. Preventing exactly that is the job of `E_PUSH_PENDING` and `remote-may-exist`.

**Suggestion**: Make the retry policy depend on the operation. For mutating requests, retry only on 429 and on statuses that prove the request was not applied, and classify any other 5xx as outcome-unknown. Add a test that a create answered with 502 is sent exactly once.

---

### `cli/work-cli/src/create.rs:878-891` — Write failure after a successful tracker create is swallowed without a cause
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

The item write is retried with `.or_else(|_| write_new(...))`, which discards the first error. If the retry also fails, `written.is_err()` produces `CreatedUnwritten` with `cause: None`, which discards the second error too.

**Impact**: This is the one state that needs a person to step in, because an issue exists that no local file carries. It is also the one state where the cause (permissions, disk full, containment, an existing file) is lost.

**Suggestion**: Keep the last error and pass it through as the cause. Consider naming the retry, for example `write_new_retrying_once`.

---

### `cli/work-adapters/src/promotion.rs:653-660` — Frontmatter split failure silently becomes an empty body sent to the tracker
**Severity**: major | **Confidence**: medium | **Lens**: code-quality

🟡 **Code Quality**

`split_frontmatter_and_body(&promoted).unwrap_or_default()` turns a parse failure into an empty body, which is then passed to `tracker.update(key, title, &body)`. `retire` does the same at line 700, and `digest::local(c).unwrap_or_default()` at line 699 hashes the failure as `""`.

**Impact**: A malformed draft overwrites the remote description with nothing, or records a baseline hash of the empty string, with no trace in the output.

**Suggestion**: Return the failure as a `NotPromoted` through the existing `store_failure`/`unreadable` helpers. Defaulting silently is never right when the value is about to be written to a remote system.

---

### `cli/work-adapters/tests/promotion.rs:974-980` — Kill-after-send branch skips the no-second-create assertion
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

When `die_at == 1`, meaning the run is killed after the create is sent but before its key is recorded, the loop `continue`s right after checking `EarlierAttemptUnconfirmed`. It skips `assert_eq!(creates(&tracker), 1)`. That is the boundary where a duplicate could be created, and the only one that does not check the create count. The validation report counts this test as covering the unwritten `a_create_killed_after_sending_…` test.

**Impact**: A regression that sends a create from the resumed path, or from the recommended `--adopt`/`--create` recovery, would still pass.

**Suggestion**: Assert `creates == 1` and an unchanged draft before the `continue`. Optionally, go on to `--adopt` the key and assert the promotion completes with `creates == 1`.

---

### `cli/visualiser/server/tests/compose_contract.rs:186-204` — Visualiser cannot identify drafts or tracker keys with digit/underscore prefixes under `{tracker}`
**Severity**: major | **Confidence**: high | **Lens**: compatibility

🟡 **Compatibility**

This test only checks that the visualiser config builds under `{tracker}`. The indexer still takes each item's identity from `WorkItemIdScheme::normalise_id`, which accepts only an all-letter prefix followed by an all-digit suffix. So `draft-k7mq3x`, `MY_PROJ-7` and `ABC2-15` all normalise to `None` and drop out of `work_item_by_id`, even though `is_tracker_key` accepts the last two. The visualiser's file listing is also non-recursive, so `meta/work/drafts/` is never indexed.

**Impact**: Under `{tracker}`, drafts never appear in the visualiser. Items from projects whose keys contain digits or underscores lose their ID, and typed links to them do not resolve.

**Suggestion**: Make `normalise_id` accept `is_tracker_key` and `DraftId` tokens when ownership is `{tracker}`. Decide whether `drafts/` should be indexed, and extend this test to assert that a `MY_PROJ-7` item and a draft each get a `work_item_id`.

---

### `skills/work/extract-work-items/SKILL.md:662-668` — Exit-1 guidance contradicts the "never re-run" rule for unpushed batches
**Severity**: major | **Confidence**: high | **Lens**: documentation

🟡 **Documentation**

Step 7 says that on exit 1 you should call `create-batch` again with the same manifest. Three lines earlier, the skill warns that without `--push`, a rerun writes every item again, because only a pushed batch is journalled.

**Impact**: An agent that follows step 7 after a declined batch fails partway will create duplicate drafts for every item that was already written.

**Suggestion**: Limit the rerun advice to pushed batches. For an unpushed batch, rebuild the manifest from only the entries that were not printed, or report the partial state.

---

### `cli/work-cli/src/create.rs:988` — Create orchestration and outcome policy live in the CLI crate, not an application service
**Severity**: major | **Confidence**: high | **Lens**: architecture

🟡 **Architecture**

`work-cli/src/create.rs` now holds application logic, much of it through direct `std::fs` calls rather than ports:
- duplicate-create detection (`pending_create`);
- the legacy marker push pipeline;
- recording the baseline after a create;
- the `NotPromoted` → `PushOutcome` policy (`create_outcome_of`);
- the draft-then-promote flow.

`create_batch.rs`, `promote.rs` and `sync.rs` call back into it.

**Impact**: The rules that prevent a second create are split between the binary and `work-adapters`. They can only be tested through the CLI's filesystem wiring, and no other front end can reuse them.

**Suggestion**: Move `pending_create`, the legacy push and `create_tracker_keyed_item` into a `work-adapters` creation service expressed against ports. Move the `NotPromoted` → `PushOutcome` mapping into `work::promotion`. Given how large the PR already is, a follow-up work item is a reasonable way to do this.

---

## Per-Lens Results

### Architecture

**Summary**: Structurally sound. The pure cores (`next_step`, `plan_retirement`, `corpus::references`) are cleanly split from adapter services that drive small new corpus ports. The concerns are about where policy now lives: create orchestration and outcome policy in the `work-cli` binary, an exit-code protocol restated in four places, a lock hierarchy only partly modelled by the port, and knowledge of which trackers support tracker-owned IDs spread across crates.

**Strengths**:
- Promotion is an explicit, durably recorded state machine in the pure `work` crate. The adapter only carries out the steps it chooses.
- `plan_retirement` is pure (snapshot in, plan out), and applying the plan is left to `work-adapters`.
- `ExclusiveCreate`, `RecoveryCopies`, `ExclusiveLock`/`LockName` and `HeldLock` are small domain ports in the zero-dependency `corpus` crate, as the pup rules require.
- `RemoteTracker::locate` → `Located::{Found, NotFound}` separates a definite "not found" from a failed read, and `RemoteIssue.key` makes a moved issue visible in a typed way.
- `work_adapters::remote_create::send_create` gives the legacy and promotion paths one shared create-and-retry policy.
- `TrackerError::Rejected` keeps the taxonomy closed, and the dispatch-code oracle test is extended.
- `IdentityWorkspace` is a deliberate attempt to centralise port assembly.

**Comments**:
- 🟡 major/high `cli/work-cli/src/create.rs:988` — Create orchestration and outcome policy live in the CLI crate, not an application service. (Body as in inline comments.)
- 🟡 major/medium `cli/work/src/promotion.rs:229-233` — Exit-code taxonomy restated as private magic numbers in several domain modules. 1/4/70/71/75 are defined in `work::promotion`, `work::sync::push_decide`, `work::create_batch` and `work-cli::exit_codes`, with the precedence `71 > 4 > 75 > 74 > 70` defined elsewhere. Suggestion: a single domain exit-code vocabulary, re-exported by `work-cli::exit_codes`, with the precedence kept beside it.
- 🔵 minor/high `cli/corpus/src/lock.rs:12-15` — The documented global lock order includes a create lock the port does not model. `LockName` has only `Retirement` and `ForFile`. The create lock is `work-cli::create::LOCK_FILE_NAME`, taken through the concrete `acquire`, and `sync_author.rs` reaches into it. Suggestion: add `LockName::Create`.
- 🔵 minor/medium `cli/tracker/src/lib.rs:819-824` — `supports_tracker_owned_ids(&str)` puts provider names into the port. The key grammar is duplicated in `is_tracker_key` and `TRACKER_SCAN_REGEX`, and `IdPatternError::TrackerNeedsJiraOrLinear` hard-codes the provider names. Suggestion: decide the capability at the composition root, and give the grammar a single definition.
- 🔵 minor/high `cli/work-cli/src/promote.rs:201-210` — `IdentityWorkspace` leaves store wiring to each caller, and the containment roots differ between `create` and `promote`/`sync`. Suggestion: have it own the stores and expose assembled ports.
- 🔵 minor/medium `cli/work-adapters/src/retirement.rs:60` — `RetirementPorts.baseline` is the concrete `&BaselineStore`, and `WorkingCopyStatus` is borrowed from `crate::sync::fetch`. Suggestion: a narrow `BaselineRename` port, and move the status port to a neutral module.
- 🔵 suggestion/medium `cli/work-adapters/src/promotion.rs:93-150` — `PromotionRow`/`Detail`/`DetailSource` presentation types live in the service module. Suggestion: a separate `promotion_report` module.

**General Findings**:
- 🔵 minor — Two parallel durable-create pipelines (the legacy `pending_push` marker and `PromotionRecord`) coexist, with no convergence plan recorded. Record one in ADR-0070 or a follow-up work item.

### Correctness

**Summary**: The promotion state machine, the record-before-act ordering and retirement resumption hold up well, and no recorded stage leads to a second create. The most serious problem is the prose rewriter's token boundary, which ignores `_` and so corrupts other keys. The batch journal keyed by digest can mix up identical entries and create duplicates. The remaining findings are edge-case data handling and inaccurate reporting.

**Strengths**:
- `next_step` is total and pure over (record state, mode). An unreadable record is never read as "no create sent", and an adopt naming a different key stops.
- Each stage is recorded before its side effect. Retitling is idempotent after a crash, thanks to the ID-normalised draft comparison.
- `plan_retirement` re-plans from all three interrupted points, and exempts the half-written target from collision checks only when it carries the old ID as an alias.
- `apply_retirement` checks snapshots under per-file locks, undo refuses to overwrite files changed since, and recovery copies are write-once.
- Draft minting is bounded at 16 draws, rejects all-digit suffixes, and checks ids, aliases and IDs minted earlier in the same batch.
- `parents_first` detects every cycle, including self-parenting, and duplicate or dangling refs are refused when the manifest is parsed.

**Comments**:
- 🟡 major/high `cli/corpus/src/references.rs:267-269` — Token boundary ignores `_`. (Body as in inline comments.)
- 🟡 major/medium `cli/work-cli/src/batch_journal.rs:135-146` — Journal rows keyed only by content digest. (Body as in inline comments.)
- 🔵 minor/medium `cli/work-adapters/src/promotion.rs:564-587` — `--adopt` records the key the user typed (`named`), not `issue.key`. Because comparisons ignore case, `pp-900` becomes a permanent lowercase id, and a moved issue gets promoted onto the stale key. Suggestion: store `issue.key.clone()` and check collisions against it.
- 🔵 minor/low `cli/work/src/retirement.rs:586-605` — `append_alias` on a zero-indented block list (`aliases:\n- "0230"`) produces invalid YAML, and `Region::after` in `references.rs` treats such entries as ordinary frontmatter. Suggestion: recognise `- ` items directly under the key, or normalise the list to the inline form.
- 🔵 minor/medium `cli/work/src/retirement.rs:315-329` — The bare `draft-x-slug.md` spelling is rewritten to `PP-900-slug.md` regardless of directory, which breaks relative sibling links from other drafts. Suggestion: spellings that depend on the referencing file's directory (`../PP-900-slug.md`).
- 🔵 minor/medium `cli/work-adapters/src/retirement.rs:364-383` — `roll_back` on a resumed plan undoes only this attempt's steps but reports `RolledBack`, and `finish_locked` then removes the record. A baseline-only resume failure also orphans the old baseline entry. Suggestion: a distinct still-incomplete outcome, and keep the record.

**General Findings**: none.

### Code Quality

**Summary**: The domain core is pure, named in the domain's language and port-injected. The weaknesses are in the wiring: errors swallowed on promotion paths, "retire onto a key" and the identity port assembly rebuilt by hand, and CLI command modules depending on each other's internals.

**Strengths**:
- `PromotionStage` → `PromotionStep` is exhaustively matched, and the `promote` loop is a thin interpreter over it.
- `create.rs` replaces a 200-line `try_run`/`execute_push` with a `CreationContext` and three named strategies chosen by `(IdOwnership, push)`.
- Ad-hoc `read_dir` and frontmatter scraping is replaced by the shared `FilesystemWorkItemFiles`/`identities`/`resolve_identity` abstractions.
- The new seams (`SuffixDraws`, `CreationStore`, `probe_status`, `PromotionRecords`, `RetirementRecords`) are injected, so flows can be driven with fault-injecting stores.
- `exit_code_for_report` is now a single `max_by_key(severity)` fold.

**Comments**:
- 🟡 major/high `cli/work-cli/src/create.rs:878-891` — Write failure after a successful create is swallowed without a cause. (Body as in inline comments.)
- 🟡 major/medium `cli/work-adapters/src/promotion.rs:653-660` — Frontmatter split failure becomes an empty body. (Body as in inline comments.)
- 🟡 major/medium `cli/work-cli/src/promote.rs:201-226` — Identity port assembly is copied across `promote`, `sync` and `create`. Suggestion: `workspace.settlement(ownership) -> IdentityPorts`.
- 🔵 minor/high `cli/work-adapters/src/promotion.rs:707-714` — `CreateOutcomeUnknown` (`remote-may-exist`) is also the catch-all for impossible record states. Suggestion: a `RecordInconsistent { stage }` variant, or restructure so those states can't arise.
- 🔵 minor/high `cli/work/src/retirement.rs:30-34` — `Retirement { new_external_id: Some(new_id) }` is built by hand at seven sites. Suggestion: `Retirement::onto_key`, and drop the `Option` if nothing needs `None`.
- 🔵 minor/high `cli/work-cli/src/promote.rs:34-39` — Command modules depend on each other's internals (`sync::severity`, `create::blocked_remedy`, `identity_workspace` ↔ `create`). Suggestion: move the shared pieces into `exit_codes`, `promotion_report` and a `layout` module.
- 🔵 minor/high `cli/work-cli/src/sync_author.rs:103-110` — `pending_push_location` turns config errors into `None` via `.ok()?`, which disables `refuse_keys_held_by_promotions`. The copy in `create.rs` propagates the error instead. Suggestion: one helper that returns a `Result`.
- 🔵 minor/medium `cli/work-cli/src/sync_author.rs:116-174` — The "which keys do live promotions claim" rule is implemented three times (`sync_author`, `create::pending_create`, `identity_settlement::keys_held_by_promotions`), each treating unreadable records differently. Suggestion: `PromotionRecords::live_claims`.
- 🔵 minor/medium `cli/work-cli/src/create.rs:1172-1256` — `promotion_outcome` runs to about 85 lines, layers four mappings, and has `unreachable!` for `Previewed`. Suggestion: one exhaustive `CreateReport::from`.
- 🔵 minor/high `cli/work-cli/src/promote.rs:227-234` — `internal(format!("{other:?}"))` (and `identity_settlement.rs:523`) shows users `Debug` output. Suggestion: use `Display`.
- 🔵 minor/high `cli/work/src/promotion.rs:229-233` — Exit codes are copied as private constants into domain modules, and the parity test only checks the CLI table.
- 🔵 suggestion/medium `cli/work/src/retirement.rs:577-612` — A line-based YAML editor of about 170 lines sits in the planner and reuses `tags::parse_current_tags`. Suggestion: a `work::frontmatter_lines` module.
- 🔵 suggestion/medium `cli/work-cli/src/create_batch.rs:290-373` — `Batch::create` mixes journalling, creation and parent annotation, and has two near-identical `JournalEntry` literals. Suggestion: a builder plus a helper.

**General Findings**:
- 🔵 minor — `(integrations_root, integration)` is an untyped pair passed through about seven functions. Introduce an `IntegrationState` value type.
- 🔵 minor — Many new `///` comments on private items restate the code (`CreationStore`, `pending create`, batch and run-error types, fields in `identity_settlement.rs`/`promotion.rs`). Prune them.
- 🔵 minor — Best-effort side effects discard their errors: `remove_file(marker).ok()` ×4, `let _ = records.remove`, `let _ = records.save` in `rewind`, and `retirement_outstanding(...).unwrap_or(true)`. At least emit a `tracing::warn!`.

### Test Coverage

**Summary**: Strong and proportional to the risk: table-driven tests of every pure decision, fault injection at every store operation, kill-and-resume loops, real-lock concurrency, and frozen exit-code tables. The gaps are narrow. The kill-after-send boundary skips the create-count assertion. The kill range is hard-coded. The "any record stage" test reaches two of the five stages. `sync-work-items` has no evals.

**Strengths**:
- Every pure domain decision has table-driven tests across all of its variants.
- Fault injection uses the real filesystem rather than mocks.
- Frozen literal tables pin the keyword and exit-code contracts, including the dual codes that depend on `issue_exists`.
- The concurrency tests can tell a working lock from a broken one.
- P1–P9 each have a named regression test, and the transport classification is tested at two levels.
- Reference-rewriter tests pin token boundaries, case folding, alias exclusion and bare-number handling.

**Comments**:
- 🟡 major/high `cli/work-adapters/tests/promotion.rs:974-980` — The kill-after-send branch skips the no-second-create assertion. (Body as in inline comments.)
- 🟡 major/medium `skills/work/sync-work-items/SKILL.md:422-435` — The new promotion and identity-pass rendering (rows, `#\tdetail` lines matched by id, a 13-row reason/recovery table) has no evals. Suggestion: evals mirroring extract-work-items 9–10, mixing promoted, not-promoted and key-changed rows.
- 🔵 minor/medium `cli/work-adapters/tests/promotion.rs:963` — The `0..=10` kill range is hard-coded and does not check that it reached every operation. Suggestion: loop until the first run is not killed.
- 🔵 minor/high `cli/work-cli/src/create.rs:2178` — `rerunning_a_create_sends_no_second_create_at_any_record_stage` covers only `Attempted` and `Created`. Suggestion: parameterise the test over every `PromotionStage`.
- 🔵 minor/medium `cli/work-cli/tests/keyword_exit_codes.rs:104-119` — `FROZEN_NOT_PROMOTED` is not checked against the SKILL.md reason table. Suggestion: a doc-parity test following the `skill_doc_worked_example.rs` precedent.
- 🔵 suggestion/high `cli/work-cli/src/batch_journal.rs:314` — The 30-day retention boundary (`<=`) is not pinned. Suggestion: add test entries exactly at 30 days and at 30 days + 1s.

**General Findings**:
- 🔵 minor — `locate()`, moved-issue detection and `NotSent`/`Rejected` are verified only against hand-written fixtures. Run `test:integration:tracker-contract` with moved-issue and 404 `locate` cases before merging.

### Safety

**Summary**: Careful overall: durable fsynced records before each create, retirement guarded by locks with snapshot checks and rollback, recovery copies for files version control cannot restore, and unreadable state treated as a failure. The remaining risks are at the edges: transports retry creates on 5xx, a failed batch-journal write doesn't stop the push, and a successful retirement discards its recovery copies straight away.

**Strengths**:
- An `Attempted` record is written durably before each create, `Unreadable` is kept distinct from `Absent`, and a second create needs an explicit `--adopt`/`--create`.
- Only a connect failure on the first attempt counts as `NotSent`.
- Per-file locks and snapshot checks run before the first step, and rollback restores only paths whose bytes still match what the retirement wrote.
- Recovery copies are also taken when dirtiness is `Unknown`, use `create_new` with fsync, and are never overwritten on resume.
- Retirement and promotion records roll forward after a Ctrl-C.
- Distinctive-only prose rewriting with whole-token boundaries.
- `--max-pushes`/`--max-pulls` ceilings are checked before anything is applied.
- Malformed records are reported as errors, never silently skipped.

**Comments**:
- 🟡 major/medium `cli/jira-client/src/transport.rs:333-342` — The transport retries non-idempotent creates on 5xx. (Body as in inline comments.)
- 🔵 minor/high `cli/work-cli/src/create_batch.rs:314-327` — A failed journal write is only stashed in `journal_failure`, and the create goes ahead. A rerun can then create a duplicate. Suggestion: make the callback fallible and abort before `promote`.
- 🔵 minor/low `cli/work-cli/src/batch_journal.rs:165-182` — The journal is read-modify-write with no lock, so overlapping `create-batch --push` runs lose entries. Suggestion: hold a lockdir for the journal's whole lifetime.
- 🔵 minor/medium `cli/work-adapters/src/retirement.rs:531-537` — A successful retirement deletes its recovery copies straight away and doesn't list the rewritten files. Suggestion: keep the copies for one more run and report the rewritten paths.

**General Findings**:
- 🔵 minor — The downgrade hazard is noted only in the PR description. Add a version field to the records, name the minimum binary version in the error, and add a changelog line.

### Compatibility

**Summary**: `{tracker}` is opt-in and legacy IDs stay as they are, but contracts change under the numeric patterns too (outcome keywords, exit 75, `work resolve`, the Jira CLI codes), and none of it is recorded in CHANGELOG. The visualiser cannot identify drafts, or keys whose prefix has digits or underscores. The new on-disk formats are forward-compatible, and the contract pins are thorough.

**Strengths**:
- `keyword_exit_codes.rs` pins every keyword and reason to a literal code.
- `aliases` is optional, and older validators ignore it.
- `Attempted`/`Created` promotion records keep the legacy marker shape, and unreadable files are reported individually.
- `{tracker}` is opt-in, there is `--no-promote`, and the cookbook warns that everyone needs 1.24.0.
- The public-API fixtures, the help golden and the exit-code parity test were updated alongside the code.

**Comments**:
- 🟡 major/high `cli/visualiser/server/tests/compose_contract.rs:186-204` — The visualiser cannot identify drafts or keys with digit or underscore prefixes. (Body as in inline comments.)
- 🔵 minor/medium `cli/work-cli/src/resolve.rs:129-140` — `work resolve` now looks identities up case-insensitively first, under every pattern, and gains exit 2 on ambiguity and exit 1 when `drafts/` is unreadable. Suggestion: exact-`id`-wins under the legacy patterns, and record the change in the changelog.
- 🔵 minor/medium `cli/jira-client/src/mutation.rs:89-91` — `accelerator jira create/update` exits 34 where it exited 21 for invalid requests, which differs from both the old value and the specific codes 42 and 17. Suggestion: map to the specific codes, or document the change.
- 🔵 minor/medium `cli/work-adapters/src/sync/pending_push.rs:412-418` — A later-stage promotion record makes an older binary's `outstanding` fail for the whole integration. Suggestion: document the downgrade path in the sync guide and the changelog.

**General Findings**:
- 🟡 major — Breaking CLI contract changes have no CHANGELOG entry: `created-unwritten`, exit 75 for `rejected`, the new precedence order, `E_RESOLVE_AMBIGUOUS`, `E_PUSH_PENDING`/`E_DRAFT_EXISTS`/`created-blocked`/`retirement-incomplete`, connect failures becoming retryable (70), the Jira CLI's 21 → 34, and the downgrade hazard.

### Security

**Summary**: Most of the new attack surface is handled well. The gap is that keys from the tracker become a retirement's `new_id` after only `identifier_is_safe`, which allows `/`, `..`, `"` and a leading `/`. They then build file paths, a `remove_dir_all` target and quoted YAML. `corpus::is_tracker_key` exists but never refuses a key.

**Strengths**:
- Jira paths are percent-encoded and checked for traversal.
- Linear uses GraphQL variables, and both clients run `identifier_is_safe` on returned keys.
- Recovery copies use `create_new`, and the state directories have self-ignoring `.gitignore` files.
- Draft suffixes come from a CSPRNG, and `DraftId::parse` is strict.
- `--adopt` requires the tracker to confirm the issue exists via `locate`.
- Prose rewriting uses literal matching, so there is no regex injection.

**Comments**:
- 🟡 major/medium `cli/work-adapters/src/sync/identity_settlement.rs:549-553` — A key from the tracker becomes a filesystem path and a `remove_dir_all` target without validation. (Body as in inline comments.)
- 🔵 minor/high `cli/work-adapters/src/promotion.rs:629-634` — A key not shaped like a tracker key only produces a `tracing::warn!` in `retitle_remote`, then promotion proceeds. Suggestion: refuse with a `NotPromoted` reason.
- 🔵 minor/medium `cli/work/src/retirement.rs:558` — `set_scalar`, `quoted_list`/`append_alias` and `rewrite_review_link` wrap IDs in raw double quotes, so a `"` can inject frontmatter keys. Suggestion: use the shared YAML escaper, or the `is_tracker_key` gate.

**General Findings**:
- 🔵 suggestion — A manifest's `body_file` accepts absolute paths and `..`, so a prompt-injected `extract-work-items` run could push a local secret to the tracker. Suggestion: restrict it to paths under the manifest's directory or the repository, or have the skill show the resolved paths before the push offer.

### Documentation

**Summary**: The skills, evals, CLI help golden, `exit_codes` docs, docs-site guides and ADR-0070 describe the change consistently. The largest gap is the missing CHANGELOG update. The rest are smaller accuracy problems: stale help text, a contradiction in extract-work-items, the "never rewritten" wording, and ADR-0044's body status.

**Strengths**:
- The CLI help is pinned by `cli_surface.golden`.
- The `exit_codes` module documents the 70/71/75 contract, the precedence order and the reason mapping, and the sync skill follows it.
- The skill changes come with targeted evals, and the benchmark notes say honestly where an eval first failed and what fixed it.
- list-work-items separates draft items from `status: draft`.
- ADR-0070 answers each of ADR-0044's objections, and ADR-0044's frontmatter is marked superseded.
- The added inline comments explain why, without referencing ADRs or work items.

**Comments**:
- 🟡 major/high `skills/work/extract-work-items/SKILL.md:662-668` — The exit-1 rerun advice contradicts the "never re-run unpushed" rule. (Body as in inline comments.)
- 🔵 minor/medium `docs-site/src/content/docs/guides/sync-work-items.mdx:178-182` — The `--adopt` "never rewritten" claim leaves out that the next sync pushes the key H1 or raises a conflict.
- 🔵 minor/medium `cli/work-cli/src/cli.rs:93-96` — The `next-number` help says "sequential" and hard-codes `meta/work/drafts/`. Under `{tracker}`, its draft IDs are random and not reserved.
- 🔵 minor/high `cli/work-cli/tests/fixtures/cli_surface.golden:185` — `work list --help` still ties the Sync column to `work.integration` alone. It now appears when a baseline exists or any listed item is a draft.
- 🔵 minor/medium `docs-site/src/content/docs/guides/configuration-cookbook.md:174-185` — The cookbook says an invalid pattern is "refused wherever it is read", but `work list` falls back to the default. It also calls `--project` "unused" when it is rejected (`E_PATTERN_KEY_UNUSED`), and `configure` repeats that wording.
- 🔵 minor/medium `skills/work/create-work-item/SKILL.md:602-607` — The new table rows lose their indentation, which breaks the table, and the `E_*` stderr codes sit under a "Second-line keyword" header.
- 🔵 minor/medium `skills/work/create-work-item/SKILL.md:553-558` — "draft" means both `status: draft` and a provisional-ID item on the same push gate.
- 🔵 minor/medium `skills/work/refine-work-item/SKILL.md:194-199` — "continue at step 3" reads as the failure branch, and the `{tracker}` warning wording comes after the warning has been shown.
- 🔵 minor/high `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md:8-9` — The body still says `**Status**: Accepted`.
- 🔵 suggestion/high `skills/work/create-work-item/evals/benchmark.md:45-46` — Evals 35–36 are missing from the results list.

**General Findings**:
- 🟡 major — CHANGELOG.md is not updated for the breaking outcome and exit-code changes, the downgrade hazard, or the new feature surface.
