---
type: "pr-review"
id: "148-review-1"
title: "[0295] Queue arXiv fetches fairly so a waiting call keeps its place across calls"
date: "2026-10-07T08:24:43+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:148"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "compatibility", "safety", "documentation", "usability"]
review_number: 1
pr_number: 148
tags: []
last_updated: "2026-10-07T08:24:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #148 - [0295] Queue arXiv fetches fairly so a waiting call keeps its place across calls

**Verdict:** COMMENT

The queue is well built. Its rules are a pure domain model behind narrow ports, and the lock protocol across `queue.lock`, `arxiv.lock` and the per-ticket locks traced as deadlock-free. arXiv rate safety does not depend on the queue files, so a fault in the queue costs fairness, never spacing. No lens found a critical defect. The open findings concern edge paths: a ticket whose liveness cannot be probed stalls the queue for about 1,000 s, lookup tickets and the pacing fallback have no tests, and the wording of the status contract in the CHANGELOG, `research.md` and the CLI help needs to be tightened.

### Cross-Cutting Themes

- **Unprobeable tickets stall the queue** (flagged by: correctness, safety, code-quality) — `Probe::Unknown` is treated as live in `contender()` and `presence_of()`. A `.lock` that is a directory, or that returns EACCES, holds the front until abandonment (issued + 1,000 s), and nothing is logged to say why. The PR lists the directory case as benign, but its effect is a stall of about 16 minutes for every caller, and later callers pass their 900 s cap and settle as `lock_contention`.
- **The re-presentation loop looks unbounded to the agent** (flagged by: usability, safety) — the arXiv profile says "repeat until" but never says the CLI ends the loop. A silently re-issued ticket (after expiry or a lost record) also starts a fresh 900 s cap. Agents may give up early and lose the node, or loop past the cap.
- **The status and exit contract is stated loosely** (flagged by: documentation, compatibility, usability) — "any other status" reads as covering `ok` (`research.md`) or `waiting` (CLI help, profile). `E_ARXIV_TICKET_LIVE` exits 2, which is documented as a usage error before any request, but it is a transient runtime state.
- **The mismatch error can't be used to rebuild the call** (flagged by: usability, test-coverage, documentation) — `E_ARXIV_TICKET_MISMATCH` names only the differing arguments. A verb mismatch drops the query or ID, and only the single-query branch is tested. The docs say it names "each issued value".
- **Queue-only states sit in the shared `FetchOutcome`** (flagged by: architecture, code-quality) — `Waiting` and `Rejected(TicketRejection)` force arms that cannot occur in OpenAlex paths and renderers. `Rejected` is a usage error carried as an outcome.
- **Queue knowledge is duplicated across crates** (flagged by: architecture, code-quality) — `LONGEST_INVOCATION` repeats `CALL_BUDGET`, the serving-window rule is re-derived 4 times, the `flock` retry loop is written out 3 times, and `ScratchDir` is rebuilt per adapter.

### Tradeoff Analysis

- **Changelog accuracy vs signalling to callers**: Compatibility credits the `### Breaking` placement for warning `fetch` callers. Documentation points out that `accelerator research` is itself under `### Added` in the same `[Unreleased]` section, so stable-release readers see a breaking change to a new feature. Recommendation: fold the queue into the Added entry for stable readers, and keep a prerelease-scoped breaking note only if prerelease consumers are an intended audience.
- **Failing closed vs keeping the queue moving**: treating `Unknown` as live is the safe default (Safety, Correctness), but it trades a rare phantom holder for a stall of about 16 minutes. Bounding `Unknown` liveness by `presented_ms + LONGEST_INVOCATION` keeps the safe default and only ever skips holders that cannot be real.

### Strengths

- ✅ The queue rules (present, expiry, cap, abandonment, `is_front_given`) are pure and I/O-free, and the adapter only gathers facts and carries out the domain's decisions. Dependencies point from `research-adapters` to the domain.
- ✅ `serve_call` takes the `ServingTurn` by value and drops it before `leave`/`step_aside`, so the lock order follows from ownership. No lock-ordering cycle exists, and `arxiv.lock` is never held together with `queue.lock`.
- ✅ Every queue write, ticket lock and unlink happens under `queue.lock`, which removes the usual flock-and-unlink race in housekeeping.
- ✅ arXiv rate safety does not depend on the queue: housekeeping never touches `arxiv.lock` or the pacing state. Stamping `last_sent` is strictly safer than before for killed calls.
- ✅ Housekeeping deletes only names that round-trip through `Ticket::parse` or carry `.tmp-`, via `ensure_contained`, so it cannot escape `arxiv-queue/` or follow symlinks.
- ✅ Degradation is deliberate: an unusable queue falls back to unqueued serving, an unlistable directory leads to a one-time report, and a filesystem without `flock` gets an unlocked turn.
- ✅ Tests sit on both sides of every threshold (300/301, 900/901, 1000/1001, 1300/1301, 32/33, 38/39 s). Real-`flock` adapter tests and real-process CLI tests check FIFO order and killed waiters.
- ✅ Every timing constant in the docs matches the code. All in-repo `fetch` consumers were updated, and an unknown-status fallback rule was added.
- ✅ `waiting` exits 0, and its stderr line gives the exact next command. Re-presentations are excluded from the call budget.

### General Findings

- 🔵 **Code Quality**: The adapter `queue.rs` (685 lines) mixes lock primitives, the record schema, housekeeping policy and `FilePlace`. Split the records and locks into submodules so the main file reads as the join/step-aside/leave protocol.
- 🔵 **Compatibility**: Pre-queue binaries sharing `arxiv.lock` bypass the queue and see more `lock_contention` during a plugin upgrade. State that fairness holds only when every session runs this release (originally anchored at `pacing.rs:376`, outside the diff hunks).
- 🔵 **Compatibility**: `waiting` reaches every arXiv caller, opted in or not. Consider adding `"reason":"waiting"` for callers keyed on `reason`.
- 🔵 **Compatibility**: The worst-case time to a settled arXiv result is now about 1,000 s across invocations, not 100 s. State it beside "Every call finishes within 100 s".
- 🔵 **Documentation**: `--ticket` is missing from the arguments paragraph of `research.md` `fetch`. The "earlier release" wording in the Logs subsection refers to releases that never shipped the log.
- 🔵 **Test Coverage**: The 30-process stress lane runs in no CI workflow. Schedule it nightly or weekly.
- 🔵 **Test Coverage**: `RecordingGate::always_busy` and `Scratch::age_file` are unused. Add the front-but-gate-busy domain test, or delete both.
- 🔵 **Correctness**: A gap between `is_front()` and `try_serve()` lets a resumed earlier ticket be overtaken for one turn. Pacing is unaffected.
- 🔵 **Correctness / Safety**: Queue timestamps have no wall-clock-step guard comparable to `PacingState::trusted_at`, so a forward stamp never expires.
- 🔵 **Code Quality**: The `Family` payload of `RequestError::TicketNotQueued` is never read.

### Additional Findings

- 🔵 `cli/research/src/sources/request.rs:6` — `request` and `queue` modules depend on each other (architecture)
- 🔵 `cli/research/src/sources/fetch.rs:584` — serving-window rule re-derived in 4 places (architecture)
- 🔵 `cli/research/src/sources/fetch.rs:386` — `served_when` / `serve_call` / `served` near-synonyms (code-quality)
- 🔵 `cli/research-adapters/src/queue.rs:190` — duplicated non-blocking `flock` retry loops (code-quality)
- 🔵 `cli/research-adapters/src/queue.rs:373` — housekeeping repeated in 3 of 4 join arms (code-quality)
- 🔵 `cli/research-adapters/src/queue.rs:417` — absent contender given a fabricated `issued_at` (code-quality)
- 🔵 `cli/research/src/sources/fetch.rs:202` — ticket rejection modelled as a `FetchOutcome` (code-quality, architecture)
- 🔵 `cli/research-adapters/src/queue.rs:255` — future-`schema_version` records deleted by older binaries (compatibility)
- 🔵 `docs-site/src/content/docs/research.md:204` — "Killed calls" bullet says the ticket both loses and keeps its place (documentation)
- 🔵 `docs-site/src/content/docs/research.md:116` — mismatch row says "each issued value" (documentation)
- 🔵 `meta/plans/2026-09-26-0283-recursive-finding-deepening.md:169` — rescope sentence repeated 8 times, with no link to the 0295 artefacts (documentation)
- 🔵 `skills/research/profiles/arxiv-profile/SKILL.md:85` — `E_ARXIV_TICKET_LIVE` guidance split, with a branch that cannot happen (usability)
- 🔵 `cli/research-adapters/src/pacing.rs:255` — no test reads a legacy pacing state without `last_sent_ms` (test-coverage)
- 🔵 `cli/research-adapters/tests/queue.rs:605` — `own()`-patience tests depend on wall-clock timing (test-coverage)
- 🔵 `cli/research-adapters/tests/queue.rs:785` — read-only-directory test fails when run as root (test-coverage)
- 🔵 `cli/research-adapters/src/pacing.rs:57` — 100 ms killed-send margin is small next to connection-setup latency (safety)
- 🔵 `docs-site/package.json:27` — lockfile churn (astro 7.3.6, `compiler-rs` 0.5) goes beyond the advisory patch (compatibility)
- 🔵 `cli/research/src/sources/queue.rs:21` — `LONGEST_INVOCATION` duplicates `CALL_BUDGET` (architecture) *(posted inline)*
- 🔵 `cli/research/src/sources/queue.rs:111` — `Binding` mirrors `ArxivRequest` (code-quality)
- 🔵 `cli/research/src/sources/queue.rs:172` — `value_of` empty-string sentinel (code-quality)
- 🔵 `cli/research/src/sources/queue.rs:400` — port doc comments restate signatures and placement (documentation)
- 🔵 `cli/research-adapters/src/queue.rs:315` — `read_record` validates via a throwaway `Standing` (code-quality)
- 🔵 `cli/research-adapters/src/queue.rs:655` — `Limit` rebuilt through a string round trip (code-quality)
- 🔵 `cli/research-adapters/src/scratch.rs:131` — `replace_transient` vs `replace_without_sync` (code-quality)
- 🔵 `cli/research-cli/src/main.rs:285` — research `ScratchDir` rebuilt per adapter (code-quality)
- 🔵 `cli/research-cli/src/main.rs:292` — composition root owns `"arxiv-queue"` (architecture)
- 🔵 `cli/research/src/sources/fetch.rs:199` — shared `FetchOutcome` gains arXiv-only variants (architecture)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `CHANGELOG.md:65-74` — Breaking entry describes a feature that has not had a stable release, and the Added entry for it is now stale
**Severity**: major | **Confidence**: medium | **Lens**: documentation

🟡 **Documentation**

This entry sits under `### Breaking` in `[Unreleased]`, but the `accelerator research` command family it changes is itself new in the same section (`### Added`, line 87). No stable release up to 1.23.0 shipped `research fetch`. A reader upgrading from 1.23.0 therefore sees a breaking change to something listed as new. The claim that `arxiv-contention.log` counts are "not comparable with earlier releases" refers to releases that never had the log. The Added entry also still says a throttled source "degrades to `status: \"unavailable\"`", and it omits `--ticket` and its three `E_ARXIV_TICKET_*` exit-2 codes.

**Suggestion**: For stable-release readers, fold the queue into the Added `accelerator research` entry: list `ok`/`unavailable`/`waiting`, `--ticket` and its exit-2 codes, and drop the "earlier releases" comparison. If prerelease consumers are an intended audience, scope the breaking note explicitly ("since 1.24.0-pre…") and still update the Added entry.

---

### `docs-site/src/content/docs/research.md:96-98` — "Any other status" can be read as treating `ok` as unavailable
**Severity**: major | **Confidence**: high | **Lens**: documentation

🟡 **Documentation**

"…repeat until the status is anything else. A caller should treat any other status as `unavailable`…" Read in sequence, "any other status" refers back to "anything else", meaning anything other than `waiting`, and that includes `ok`. This is the forward-compatibility rule for custom `fetch` callers, so a literal reader could discard successful results.

**Suggestion**: Name the known set: "Treat any status other than `ok`, `unavailable` or `waiting` as `unavailable`, reporting it verbatim as the reason." Move the sentence next to the status/exit table.

---

### `cli/research-adapters/src/queue.rs:643-662` — No test persists and resumes a lookup ticket
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

Every adapter and CLI queue test issues a `search` binding. No test writes a `StoredBinding::Lookup` record, reads it back through `to_binding`/`ArxivId::parse`, or re-presents a lookup ticket. If lookup persistence broke (for example a dropped version suffix or a changed serde tag), `read_record` would set the record aside, housekeeping would remove it, and every lookup re-presentation would silently `Join` at the back of the queue. No test would fail.

**Suggestion**: Mirror `a_resumed_ticket_keeps_its_number_and_clears_its_end` for `ArxivRequest::Lookup(ArxivId::parse("2608.21129v2"))`, asserting the stored `verb`/`id` and that the resumed ticket keeps its number.

---

### `cli/research-adapters/src/pacing.rs:140-155` — The pacing gate's fallback when the filesystem cannot lock is untested
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

When `arxiv.lock` cannot be opened or locked, `try_serve` hands out an unlocked turn and reports `pacing without the lock`. No test covers either branch. If this returned `None` by mistake (an easy slip, since `None` now means "busy"), `admitted` would poll until out of budget, and every arXiv call would end `waiting` and then `lock_contention` with the suite still green.

**Suggestion**: Create `arxiv.lock` as a directory, as the queue tests already do for `queue.lock`. Assert that `try_serve()` is `Some`, that the attempt is still spaced, and that one diagnostic names `arxiv.lock`.

---

### `cli/research-adapters/src/queue.rs:417-430` — An unprobeable ticket ahead stalls the queue until abandonment (Correctness, Safety, Code Quality)
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, safety, code-quality

🔵 **Correctness**

`contender()` and `presence_of()` treat `Probe::Unknown` as live. `probe()` returns `Unknown` for a `.lock` that is a directory, or for EACCES, EMFILE and similar errors. One such ticket blocks every later `is_front()` until issued + 1,000 s. Callers behind it pass their 900 s cap and settle as `lock_contention`, and re-presenting the ticket itself gets `E_ARXIV_TICKET_LIVE`. That is a stall of about 16 minutes for every caller, not just the repeated housekeeping report the PR describes.

🔵 **Safety**

Failing closed is right, but a real holder can never outlive one invocation. Bounding `Unknown` liveness by the record's `presented_ms + LONGEST_INVOCATION`, rather than issue + `ABANDONED_AFTER`, would only ever skip phantom holders.

🔵 **Code Quality**

Both `Err(_)` arms in `probe` (around lines 166–186) drop the errno, so a stuck queue gives no clue that the cause is an unreadable lock rather than a live call. Carry the error in `Probe::Unknown`, or report once per ticket as `report_unlistable_once` does.

---

### `skills/research/profiles/arxiv-profile/SKILL.md:98-103` — The re-presentation loop has no visible end (Usability, Safety)
**Severity**: minor | **Confidence**: medium | **Lens**: usability, safety

🔵 **Usability**

"Repeat until the status is anything else" never tells the agent that the CLI ends the loop: after the 900 s cap, the call settles as `unavailable`/`lock_contention`. `waiting` is also absent from the "End in exactly one of these" outcome list. LLM subagents given an open-ended "until" often stop after a few tries, which loses the node, the outcome this PR exists to prevent.

🔵 **Safety**

The cap does not hold across the loop. `Queue::present` silently issues a fresh ticket, with a fresh cap, when the presented one is expired, unreadable or missing (for example after an agent turn longer than 300 s or a lost unsynced record). The loop can therefore run well past 900 s, and re-presentations don't count against the call budget.

**Suggestion**: Add one sentence such as "The CLI settles this within about 15 minutes; never stop while the status is `waiting`." Pair it with a hard ceiling that doesn't depend on the ticket persisting, for example "re-present at most 12 times, then report Unavailable with reason `waiting`".

---

### `cli/research-cli/src/cli.rs:22-30` — Help text: fallback rule precedes `waiting`, and exit 2 now covers a transient state (Documentation, Compatibility)
**Severity**: minor | **Confidence**: medium | **Lens**: documentation, compatibility

🔵 **Documentation**

The help says "treat any other status as unavailable" right after `ok`/`unavailable` and before it introduces `waiting`. The arXiv profile's Unavailable bullet has the same order. An agent acting on the first rule that applies could treat `waiting` as unavailable. Enumerate the set ("other than ok, unavailable or waiting") and put the rule after all three statuses.

🔵 **Compatibility**

Exit 2 is documented as "usage error, before any request". `E_ARXIV_TICKET_LIVE` reflects transient queue state, and the identical command can succeed once the other process returns. Callers mapping exit 2 to "fix the arguments, never retry as-is" will handle it wrongly. Either use a distinct exit code for queue-state rejections, or state in the help text and the exit-code table that exit 2 can be transient for `E_ARXIV_TICKET_LIVE`.

---

### `cli/research-cli/src/render.rs:51-66` — Mismatch error can't be used to rebuild the call, and only one branch is tested (Usability, Test Coverage)
**Severity**: minor | **Confidence**: high | **Lens**: usability, test-coverage

🔵 **Usability**

`E_ARXIV_TICKET_MISMATCH` names only the differing arguments. On a verb mismatch, `Binding::differing` returns just `[Verb]`, so the message omits the query or ID the ticket was bound to. Limits render as `limit 10`, not `--limit 10`. An agent told to "re-present it with that call" may fall back to "drop --ticket" and lose its place. When the verb differs, render the whole issued call as typed, for example `search 'attention heads' --limit 10`.

🔵 **Test Coverage**

Only the single `query '…'` branch is tested. The `verb`/`id`/`--limit` branch and the `, ` join for several differing arguments are never rendered in a test. Add cases for a limit mismatch, a search-vs-lookup mismatch and a query+limit mismatch.

---

### `cli/research/src/sources/queue.rs:21` — `LONGEST_INVOCATION` silently duplicates the CLI's `CALL_BUDGET`
**Severity**: minor | **Confidence**: high | **Lens**: architecture

🔵 **Architecture**

`ABANDONED_AFTER`, the stamping in `latest_end` and the adapter's stale-temp sweep all assume no call outlives 100 s, but nothing ties `LONGEST_INVOCATION` to `CALL_BUDGET` in `research-cli/src/main.rs`. If someone raises the budget, live calls could be stamped as ended or treated as abandoned, breaking FIFO order with no failing test.

**Suggestion**: Derive one from the other, as the contention log now derives its message from `queue::CAP`. Alternatively, assert `CALL_BUDGET <= LONGEST_INVOCATION` in a test.

---

### `cli/research-adapters/src/queue.rs:298-303` — A resumed ticket that fails to take its place loses its `waiting` outcome
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

`take_place` returns `Joining::Unqueued` when `own()` or `write_record` fails. On the `Presentation::Resume` path the ticket's record still holds its place, but the call now runs unqueued. If it runs out of budget, it reports `Unavailable` (`QueueUnusable` `lock_contention`) instead of `Waiting` with the still-valid ticket. The caller stops re-presenting, so a transient hiccup turns a delayed node into a lost one.

**Suggestion**: When the presented ticket's record exists, return a variant that carries the ticket, so an out-of-budget result still becomes `FetchOutcome::Waiting`. Alternatively, retry `write_record` once before giving up the place.

---

## Per-Lens Results

### Architecture

**Summary**: The PR keeps a clean functional core / imperative shell split. The queue rules are pure domain code, the file-backed adapter, nonce randomness and serde schema stay in `research-adapters`, and dependencies point from adapters to the domain. `ServingTurn` makes whole-call admission a type, and the lock order follows from ownership. The weaknesses are hidden couplings: `LONGEST_INVOCATION` duplicates `CALL_BUDGET`, the serving-window rule is re-derived in four places, and `request`/`queue` depend on each other.

**Strengths**: The pure queue model is unit-testable without a filesystem. The dependency direction is right (`rand`, `serde`, `rustix` stay in adapters). The `try_serve`/`paced` split makes whole-call admission a type. The `ContentionLog` port with a typed `Contention` enum moved out of pacing. Degradation is deliberate. The `store` refactor keeps both write contracts explicit.

**Comments**:
- 🔵 minor/high `cli/research/src/sources/queue.rs:21` — `LONGEST_INVOCATION` silently duplicates `CALL_BUDGET`. Derive one from the other, or assert the relation.
- 🔵 minor/medium `cli/research/src/sources/fetch.rs:584` — The serving-window rule is re-derived in `admitted`, `until_settled`, `Bound::ServingWindow` and `FileArxivQueue::own`. Introduce a domain `ServingWindow` and pass it to `ArxivQueue::join` instead of a raw `spacing`.
- 🔵 minor/high `cli/research/src/sources/request.rs:6` — `request` imports `queue::Ticket` while `queue` imports request types, and ticket errors live in `RequestError`. Keep the dependency one-way, with a `TicketError` in `queue`.
- 🔵 suggestion/medium `cli/research-cli/src/main.rs:292` — The composition root hard-codes `"arxiv-queue"`. Let `FileArxivQueue` nest its own directory.
- 🔵 suggestion/low `cli/research/src/sources/fetch.rs:199-202` — The shared `FetchOutcome` gains arXiv-only variants. Acceptable for two sources; consider `ArxivOutcome` if more appear.

**General Findings**: none.

### Correctness

**Summary**: The lock protocol is sound. Every queue write, ticket lock and unlink happens under `queue.lock`, probes are non-blocking and shared, `arxiv.lock` and `queue.lock` are never held together, and `serve_call` drops the turn before `leave`/`step_aside`. The front rule always lets the lowest live ticket through, and probe-then-delete is safe. The edge cases are an unprobeable ticket stalling the queue, and a failed resume that drops to the unqueued path.

**Strengths**: A single lock covers the queue. Lock guards are bound to named variables, so drop order is correct. There is no lock-ordering cycle. The front rule always lets someone through. Expiry, cap and abandonment fit together, and `end_of` only moves ends earlier. Pacing handles a missing or older finish and same-millisecond stamps.

**Comments**:
- 🔵 minor/medium `cli/research-adapters/src/queue.rs:417-430` — An unprobeable ticket ahead blocks the queue until abandonment, about 16 minutes. Treat a non-regular-file lock path as not live, or bound `Unknown` more tightly.
- 🔵 minor/medium `cli/research-adapters/src/queue.rs:298-303` — A resumed ticket that fails `own()`/`write_record` loses its `waiting` outcome. Carry the ticket through, or retry the write.

**General Findings**:
- 🔵 suggestion — A gap between `is_front()` and `try_serve()` allows a one-turn FIFO inversion. Pacing is unaffected. Document it, or re-check after `try_serve`.
- 🔵 suggestion — Queue timestamps lack a wall-clock-step guard comparable to `PacingState::trusted_at`.

### Test Coverage

**Summary**: The queue is test-driven at every layer: boundary tests on both sides of every threshold, `Timeline` interleaving assertions, real-`flock` adapter tests and real-process CLI tests. The gaps are the lookup-ticket round trip, the unlocked fallback of the pacing gate, legacy pacing-state reads, the mismatch-message branches and two timing-dependent adapter tests.

**Strengths**: Boundary tests sit on both sides of every threshold. `Timeline` pins the turn being dropped before `leave`/`step_aside`. Rejection tests snapshot every queue file. Adapter tests run against real `flock` semantics. Real-process CLI tests check FIFO order and killed waiters. The stress lane is opt-in and guarded in `test_mise.py`. `replace_without_sync` has direct tests.

**Comments**:
- 🟡 major/high `cli/research-adapters/src/queue.rs:643-662` — No test persists and resumes a lookup ticket.
- 🟡 major/high `cli/research-adapters/src/pacing.rs:140-155` — The unlocked fallback of the pacing gate is untested.
- 🔵 minor/high `cli/research-adapters/src/pacing.rs:255` — No test reads a pacing state written before `last_sent_ms`.
- 🔵 minor/high `cli/research-cli/src/render.rs:51-63` — The mismatch message is tested only for a single query mismatch.
- 🔵 minor/medium `cli/research-adapters/tests/queue.rs:605-650` — The `own()`-patience tests rely on wall-clock timing. Move the failure case to `RecordingClock`.
- 🔵 minor/low `cli/research-adapters/tests/queue.rs:785-790` — The read-only-directory test fails when run as root.

**General Findings**:
- 🔵 minor — The stress lane runs in no CI workflow. Schedule it.
- 🔵 suggestion — `RecordingGate::always_busy` and `Scratch::age_file` are unused.

### Code Quality

**Summary**: The domain/adapter split keeps the fetch workflow testable with in-memory fakes, and errors degrade with diagnostics throughout. The costs are concentrated in the 685-line adapter `queue.rs` (four responsibilities, repeated `flock` loops) and in the near-synonymous `served_when`/`serve_call`/`served`, plus some modelling smells that let states the domain never has be represented.

**Strengths**: The domain is pure. `Queue::present` reads as domain guard clauses. Domain language is rich throughout. `Result<Settled<T>, OutOfBudget>` separates budget exhaustion. The `store` extract-function refactor is sound. Degradation is reported, not swallowed. Comments are mostly "why" comments.

**Comments**:
- 🔵 minor/high `cli/research-adapters/src/queue.rs:190-231` — The non-blocking `flock` retry loop is duplicated in `bookkept` and `own` (and `try_serve`).
- 🔵 minor/medium `cli/research-adapters/src/queue.rs:373-401` — Housekeeping is called in 3 of 4 join arms, and the `Rejected` exception is unexplained.
- 🔵 minor/medium `cli/research-adapters/src/queue.rs:417-430` — An absent `Contender` gets a fabricated `issued_at`. Model it as an `Absent | Live { issued_at }` enum.
- 🔵 minor/medium `cli/research-adapters/src/queue.rs:166-186` — Probe failures collapse to `Unknown` with no diagnostic.
- 🔵 minor/high `cli/research/src/sources/fetch.rs:386-423` — `served_when`/`serve_call`/`served` are near-synonyms.
- 🔵 minor/medium `cli/research/src/sources/fetch.rs:202` — Ticket rejection is modelled as a `FetchOutcome`.
- 🔵 suggestion/medium `cli/research-adapters/src/queue.rs:315-323` — `read_record` validates via a throwaway `Standing`.
- 🔵 suggestion/high `cli/research-adapters/src/queue.rs:655-665` — `Limit` is rebuilt through a string round trip. Add `TryFrom<u8>`.
- 🔵 suggestion/medium `cli/research/src/sources/queue.rs:111-145` — `Binding` mirrors `ArxivRequest`. Make it a newtype.
- 🔵 suggestion/medium `cli/research/src/sources/queue.rs:172-187` — `value_of` uses an empty-string sentinel.
- 🔵 suggestion/medium `cli/research-adapters/src/scratch.rs:131` — The no-fsync contract has two names: `replace_transient` and `replace_without_sync`.
- 🔵 suggestion/high `cli/research-cli/src/main.rs:285-297` — The research `ScratchDir` is built once but still rebuilt per adapter.

**General Findings**:
- 🔵 minor — The adapter `queue.rs` combines four responsibilities. Split records and locks into submodules.
- 🔵 suggestion — The `Family` payload of `TicketNotQueued` is never read.

### Compatibility

**Summary**: The PR knowingly breaks the `fetch` output contract and handles it well: a `### Breaking` entry, every in-repo consumer updated, and an unknown-status fallback rule. No Rust code parses the fetch JSON, and the public-API changes are additive or internal. The remaining risks are mixed-version scratch directories, future record schemas, and exit 2 now covering a transient state.

**Strengths**: The CHANGELOG placement follows convention. The forward-compatibility rule is added in the help text, `research.md` and both profiles. All consumers are updated. The pacing state stays readable across versions (`#[serde(default)]`, `rename`). The contention-log format change is documented. `atomic_write` keeps its fsyncs. `--ticket` is optional and rejected on OpenAlex.

**Comments**:
- 🔵 minor/medium `cli/research-cli/src/cli.rs:25-30` — Exit 2 now covers a transient runtime conflict (`E_ARXIV_TICKET_LIVE`).
- 🔵 minor/medium `cli/research-adapters/src/queue.rs:255-266` — Records with a future `schema_version` are treated as unreadable and deleted by older binaries. Keep newer-version records, or version the directory.
- 🔵 suggestion/medium `docs-site/package.json:27` — Lockfile churn reaches beyond the `sharp`/`source-map-js` patch (astro 7.3.6, `compiler-rs` 0.5, `es-module-lexer` 3).

**General Findings**:
- 🔵 minor — Pre-queue binaries sharing `arxiv.lock` bypass the queue and see more `lock_contention` (moved from `pacing.rs:376-392`, outside the diff hunks).
- 🔵 minor — `waiting` is opt-out for callers outside the plugin. Consider adding `"reason":"waiting"`.
- 🔵 suggestion — Fixed-budget arXiv calls can now span many invocations. State the new worst case.

### Safety

**Summary**: The queue is well contained. Deletion is limited to strict ticket names inside `arxiv-queue/`, every wait loop is bounded, and arXiv rate safety does not depend on the queue files, so faults cost fairness, not spacing. The concerns are low severity: the agent loop's lack of its own ceiling, `Unknown` probes wedging fairness, and a thin killed-send margin.

**Strengths**: Rate safety does not depend on the queue. Housekeeping deletion is tightly scoped. A live ticket's files are protected. Every wait loop is bounded by the deadline. Corrupt records self-heal. Stamping `last_sent` strictly improves on the previous behaviour. The stress lane uses `MockHTTPServer`, not real arXiv.

**Comments**:
- 🔵 minor/medium `skills/research/profiles/arxiv-profile/SKILL.md:98-103` — The `waiting` re-presentation loop has no ceiling of its own.
- 🔵 suggestion/low `cli/research-adapters/src/queue.rs:155-162` — An `Unknown` probe wedges fairness for longer than the ticket cap.
- 🔵 suggestion/low `cli/research-adapters/src/pacing.rs:57-61` — The 100 ms killed-send margin is small next to connection-setup latency.

**General Findings**:
- 🔵 suggestion — Queue liveness and expiry use the wall clock, so a sleep or clock step can expire a live ticket. This affects fairness only.

### Documentation

**Summary**: The docs match the code closely: every timing constant, error code and the stderr line agree. The problems are in the pieces the PR asked a second reader to check. The CHANGELOG places a feature that has never had a stable release under `### Breaking`, the "any other status" sentence in `research.md` is ambiguous, and the 0283 annotations repeat one sentence eight times without links.

**Strengths**: Timing claims match the code. Each contention-log kind is paired with a next step. The validation follow-ups fixed comments that would have drifted. The Limits subsection is candid. The opt-in lanes are documented. The `waiting` outcome in the arXiv profile is clear and concrete.

**Comments**:
- 🟡 major/medium `CHANGELOG.md:65-74` — The Breaking entry describes a feature that has not had a stable release, and the Added entry is stale.
- 🟡 major/high `docs-site/src/content/docs/research.md:96-98` — "Any other status" can be read as treating `ok` as unavailable.
- 🔵 minor/medium `cli/research-cli/src/cli.rs:22-30` — The help puts the fallback rule before `waiting` (the profile does the same).
- 🔵 minor/medium `docs-site/src/content/docs/research.md:204-205` — The "Killed calls" bullet says the ticket both loses and keeps its place.
- 🔵 minor/high `docs-site/src/content/docs/research.md:116` — The mismatch row says "each issued value", but only differing ones are named.
- 🔵 minor/high `meta/plans/2026-09-26-0283-recursive-finding-deepening.md:169-170` — The rescope sentence is repeated 8 times with no link to the 0295 artefacts.
- 🔵 suggestion/low `cli/research/src/sources/queue.rs:400-402` — Some port doc comments restate signatures or make placement claims.

**General Findings**:
- 🔵 suggestion — `--ticket` is missing from the `fetch` arguments paragraph in `research.md`.
- 🔵 minor — The Logs subsection attributes timestamp-only lines to "an earlier release".

### Usability

**Summary**: The `waiting`/`--ticket` protocol is easy for an agent to follow: exit 0, a JSON document that describes itself, a stderr line giving the exact next command, and a fenced example in the profile. The friction is in edge paths: an incomplete mismatch message, split `E_ARXIV_TICKET_LIVE` guidance, no visible end to the loop, and silent re-issue of a ticket after expiry.

**Strengths**: `waiting` exits 0 and the JSON carries everything needed to continue. The stderr line gives the exact next action. The Waiting outcome is concrete and pinned by structure tests. Re-presentations are excluded from the call budget. The unknown-status fallback gives a safe default. `E_ARXIV_TICKET_MALFORMED` shows a valid example. The triage tables pair each failure with a next step. The stress lane is easy to find and run.

**Comments**:
- 🔵 minor/high `cli/research-cli/src/render.rs:51-66` — The mismatch error omits the full issued call on a verb mismatch and uses bare words instead of flag names.
- 🔵 minor/medium `skills/research/profiles/arxiv-profile/SKILL.md:85-96` — The `E_ARXIV_TICKET_LIVE` guidance is split across two paragraphs, and its "wait for the other fetch" branch cannot happen.
- 🔵 minor/medium `skills/research/profiles/arxiv-profile/SKILL.md:98-103` — The loop gives the agent no sign that it ends.
- 🔵 suggestion/medium `cli/research-cli/src/cli.rs:24-29` — Nothing in the help or output shows when a ticket was silently re-issued after expiry.

**General Findings**: none.
