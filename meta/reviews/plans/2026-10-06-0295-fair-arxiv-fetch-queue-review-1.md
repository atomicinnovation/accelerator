---
type: "plan-review"
id: "2026-10-06-0295-fair-arxiv-fetch-queue-review-1"
title: "Plan Review: Fair arXiv Fetch Queue Implementation Plan"
date: "2026-10-06T09:01:58+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-06-0295-fair-arxiv-fetch-queue"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "performance", "usability", "safety", "compatibility"]
review_number: 1
review_pass: 4
tags: ["research", "arxiv", "pacing", "fetch"]
last_updated: "2026-10-06T16:04:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Fair arXiv Fetch Queue Implementation Plan

**Verdict:** REVISE

The plan has a sound structure. Queue policy is pure and lives in `research`,
the `flock` mechanics sit behind a `FetchQueue`/`Place` port in
`research-adapters`, the serving lock and the bookkeeping lock are kept
separate, and every timing boundary in the acceptance criteria is tested on
both sides. The weak spot is the queue's own failure semantics:

- a ticket whose liveness stays `Unknown` blocks the front for good;
- degraded mode loses the 900 s cap that bounds the researcher's loop;
- bookkeeping runs before `present`, so expired mismatches are unreachable
  and rejected calls still change the queue.

Test Coverage also maps several acceptance criteria to tests that cannot
fail for the bug the criterion guards. Each issue needs a targeted edit; none
needs a redesign.

### Cross-Cutting Themes

- **`Unknown` liveness has no age backstop** (flagged by: architecture,
  safety, correctness, test-coverage). `lease.rs` pairs "`Unknown` is never
  free" with an age backstop, and the plan copies the rule without the
  backstop. A permanently unprobeable ticket lock is `Live` for good. It is
  never stamped, expired or pruned, and it holds the front against every later
  ticket.
- **Degraded mode loses the 900 s cap** (flagged by: safety, usability,
  correctness). `UnqueuedPlace` issues an unpersisted ticket. Re-presenting it
  always joins afresh, so the cap never fires and the profile's "repeat until
  anything else" loop has no Rust-enforced end.
- **The re-presentation instruction breaks when the ticket changes**
  (flagged by: correctness, usability). Expiry, pruning and degraded mode all
  return a different ticket. "Re-run the identical command with `--ticket`
  appended" then produces a second `--ticket`, which clap rejects with exit 2,
  and the profile treats exit 2 as terminal.
- **`bookkept` runs on every `is_front` poll, under a blocking lock with no
  deadline** (flagged by: performance, safety, architecture, correctness,
  code-quality). With 30 waiters that is O(N²) file operations a second,
  serialised. It ignores the call `Deadline` and has no failure channel. A
  method named like a query also stamps and prunes.
- **"Out of budget, keep your place" is carried through `Unavailable`**
  (flagged by: code-quality, architecture). `ServingTurn::paced` still returns
  `Result<(), Unavailable>`, and `Settled::OutOfTurn` reinterprets it, so the
  types say the opposite of the domain's meaning.
- **Two admission loops and a transient port method** (flagged by:
  code-quality, architecture). Phase 1 adds `admitted()` and
  `PacingGate::contended`. Phase 4 removes `contended` and writes a second
  loop inline. The OpenAlex copy cannot poll.
- **Corrupt-record deletion ignores liveness** (flagged by: safety,
  architecture). A record left by another binary version can belong to a live
  invocation, and removing its lock file silently drops that invocation from
  the queue.
- **No clock-step rule for the queue's wall times** (flagged by: safety,
  architecture). `PacingState::trusted_at` guards the pacing state against
  clock steps, but expiry and cap arithmetic have no equivalent.
- **Rejection variants are duplicated** (flagged by: code-quality,
  architecture). `Presentation`, `Joining` and `TicketRejection` each declare
  `Mismatch`/`AlreadyLive`, and `OverCap` has a different shape in two of
  them.

### Tradeoff Analysis

- **Idle arXiv time vs. no interleaving**: holding backoff under the serving
  lock leaves arXiv idle during 5xx retries. Performance accepts this because
  the work item does. No action is needed beyond the throughput assertion
  recommended below, which separates that accepted cost from a queue
  regression.
- **Degraded mode fail-open vs. fail-safe**: degraded unqueued admission
  keeps fetches working when `queue.lock` breaks. With both locks broken,
  though, Safety notes that 24 researchers can reach arXiv unlocked at once.
  Failing closed (`rate_limited`) when neither lock can be held protects
  arXiv's terms at the cost of availability. Recommend failing closed only in
  the doubly-degraded case.
- **Strict "queue unchanged" vs. opportunistic housekeeping**: a rejected
  presentation can either skip stamping and pruning, which matches the
  acceptance criterion literally, or narrow the invariant to "the presented
  ticket and the order are unchanged". The first is simpler to reason about.

### Findings

#### Major

- 🟡 **Architecture / Safety / Correctness**: `Unknown` liveness is treated
  as live with no backstop and can stall the whole queue
  **Location**: Implementation Approach: Decided behaviours; Phase 3
  `bookkept` step 2
  One ticket lock that keeps probing `Unknown` (EACCES, EMFILE, ENOLCK) holds
  the front for good. Every later ticket waits out its cap and ends in
  `lock_contention`.
- 🟡 **Safety / Usability**: Degraded unqueued mode loses the 900 s cap, so
  the re-presentation loop has no termination guarantee
  **Location**: Phase 3 Degraded mode; Phase 4 §5 Prompts
  The ticket is never persisted, so the cap check at `join` never fires, and
  every `waiting` misleadingly reports position 1.
- 🟡 **Correctness / Usability**: "Re-run with `--ticket` appended" breaks
  when a call returns a different ticket
  **Location**: Phase 4 §5 Prompts (Waiting outcome)
  Expiry, pruning or degraded mode hands back a new ticket. The next literal
  re-run then carries two `--ticket` flags, and clap exits 2.
- 🟡 **Performance**: Each 100 ms `is_front` poll runs a full O(N)
  bookkeeping pass under one blocking exclusive lock
  **Location**: Phase 3 `bookkept`/`FilePlace::is_front`; Performance
  Considerations
  At 30 waiters that is about 18k serialised opens and flocks a second. The
  departing invocation's `leave`/`step_aside` queues behind it on the handoff
  path.
- 🟡 **Correctness**: Bookkeeping prunes expired records before `present`, so
  an expired mismatch can never be detected
  **Location**: Phase 3 `bookkept`/`join` vs Phase 2 `Queue::present`
  `a_ticket_with_other_arguments_is_a_mismatch_even_when_expired` passes in
  the domain. The binary instead silently joins with the new arguments.
- 🟡 **Correctness**: Rejected presentations still change the queue through
  stamping and pruning
  **Location**: Phase 3 `bookkept` steps 3–4; Phase 4
  `a_ticket_with_other_terms_exits_2_…`
  The byte-for-byte test passes only because its fixture has nothing to stamp
  or prune.
- 🟡 **Correctness**: Ticket numbering is "largest live record" in one place
  and "maximum number" in another
  **Location**: Implementation Approach: On-disk queue vs Phase 2
  `next_number`
  The "live" reading lets a new ticket outrank an absent, unexpired one, which
  breaks priority on re-presentation.
- 🟡 **Code Quality**: `fetch_arxiv` gathers queue orchestration, admission
  and serving, with many early returns that must each settle the `Place`
  **Location**: Phase 4 §1 Domain workflow
  A missed `leave()` is silent: the ticket lingers as absent for 300 s and
  inflates positions.
- 🟡 **Code Quality / Architecture**: Running out of budget is encoded through
  `Unavailable` and a shared `Settled` variant
  **Location**: Implementation Approach: Ports; Phase 4 `Settled::OutOfTurn`
  The port's type says "source unavailable", while the domain means "budget
  ran out, the source is fine".
- 🟡 **Usability**: `E_ARXIV_TICKET_LIVE` gives the agent no remedy, yet the
  profile reads exit 2 as "correct the call"
  **Location**: Phase 4 §3 render.rs; §5 Prompts
  An LLM with no remedy will guess: retry in a hot loop, drop `--ticket`, or
  change the arguments.
- 🟡 **Test Coverage**: No test pins spacing from `last_finish` when it comes
  after `last_sent`
  **Location**: Phase 1 adapter tests
  A mutation that always prefers `last_sent` passes every listed test and
  breaks the arXiv 3 s rule after slow responses.
- 🟡 **Test Coverage**: The 900 s crossing criterion is mapped to a fake that
  cannot fail for the real risk
  **Location**: Phase 4 `a_ticket_crossing_900_s_while_waiting_is_still_served`
  The risk lives in `FileFetchQueue` bookkeeping, and no adapter test covers
  it.
- 🟡 **Test Coverage**: Concurrent joins are checked only by a manual diff
  read
  **Location**: Phase 3 tests / Manual Verification
  No default-lane test races joins, so a `next_number` read outside the lock
  would pass.
- 🟡 **Test Coverage**: No list of existing tests whose expected outcome
  changes or which are deleted
  **Location**: Phase 1 and Phase 4 tests
  At least eight existing or Phase 1 tests assert behaviour that Phase 4
  changes. Only one replacement is named.
- 🟡 **Test Coverage**: The re-presentation half of the mid-call exhaustion
  criterion is untested
  **Location**: Phase 4 `a_second_confirmation_the_budget_cannot_cover_…`
  No test checks that the search repeats and that cached confirmations are
  reused on the second invocation.
- 🟡 **Test Coverage**: "Within 1 s" upper bounds in default-lane real-time
  tests are likely to flake
  **Location**: Phase 1 and Phase 4 real-time binary tests
  Spawn time, the 100 ms poll and CI load all eat into a 1 s bound.
- 🟡 **Test Coverage**: The planner regression guard is a one-off manual
  check, though it is deterministic
  **Location**: Phase 5 §1 Regression guard
  `topic_outstanding.rs` already has the seams for an automated test.

#### Minor

- 🔵 **Safety / Architecture**: Corrupt-record deletion ignores liveness and
  can unlink a live holder's ticket lock
  **Location**: Phase 3 `bookkept` step 1
  Version skew makes this systematic, and the behaviour of `step_aside` and
  `leave` on a missing record is unspecified.
- 🔵 **Safety / Architecture / Correctness**: The blocking `queue.lock` take
  ignores the call deadline
  **Location**: Phase 3 `bookkept`
  A stopped holder pushes every waiter past the 120 s Bash timeout instead of
  returning `waiting`.
- 🔵 **Safety / Architecture**: Expiry and the cap use wall-clock arithmetic
  with no clock-step guard
  **Location**: Phase 2 `is_expired`, `present`
  The plan does not say what happens when `now` is earlier than a stored
  stamp.
- 🔵 **Safety**: When both locks are degraded, the serving gate fails open
  across the whole call
  **Location**: Phase 1 `try_serve` unlocked turn; Phase 3 Degraded mode
  Every `UnqueuedPlace` is front and every turn is unlocked, so 24 concurrent
  connections can reach arXiv.
- 🔵 **Code Quality / Architecture**: Admission is split across two loops,
  with a transient `PacingGate::contended`
  **Location**: Phase 1 §1; Phase 4 §1 Removals
  The end state duplicates the serving-window rule, and the OpenAlex loop's
  polling can never run.
- 🔵 **Code Quality / Architecture**: `Mismatch`/`AlreadyLive` are copied
  across three enums, and `OverCap` has inconsistent shapes
  **Location**: Phase 2 `Presentation`; Phase 3 `Joining`; Phase 4
  `TicketRejection`
- 🔵 **Code Quality**: `Place::is_front` reads as a pure query but stamps,
  prunes and deletes, and its failure path is unspecified
  **Location**: Phase 3 The `Place`
- 🔵 **Code Quality**: `Binding` flattens the typed `ArxivRequest` into
  primitives
  **Location**: Phase 2 `Binding`
  It can represent a lookup with a limit, and it calls an arXiv ID "terms".
- 🔵 **Correctness**: Stamping a killed ticket lazily at "now" can turn an
  expired ticket into a cap hit
  **Location**: Implementation Approach: End stamping
- 🔵 **Correctness**: `Ord` by number alone is inconsistent with nonce-aware
  equality
  **Location**: Phase 2 `Ticket`
- 🔵 **Correctness**: The admission check and the first pre-attempt check
  read the clock separately
  **Location**: Admission loop
  An invocation can take the serving lock and then make no request.
- 🔵 **Correctness**: A persistently slow upstream 5xx ends as
  `lock_contention` after 15 minutes
  **Location**: Decided behaviours; Phase 4 `Settled::OutOfTurn`
- 🔵 **Performance**: Fsync'd atomic writes of disposable state happen while
  contended locks are held
  **Location**: Phase 3 record writes; Phase 1 `last_sent`
- 🔵 **Performance**: The stress test and the validation run check only the
  lower bound on request gaps, never throughput
  **Location**: Phase 4 Stress; Phase 5 Validation run
- 🔵 **Performance**: The agent-turn cost of re-presentation is neither
  estimated nor measured
  **Location**: Performance Considerations
- 🔵 **Usability**: The profile does not say to use the latest ticket or to
  re-present immediately
  **Location**: Phase 4 §5 Prompts
- 🔵 **Usability**: `E_TICKET_NOT_QUEUED` breaks the error-code naming pattern
  **Location**: Phase 2 `RequestError`
- 🔵 **Usability**: `--ticket` lacks a per-argument doc comment
  **Location**: Phase 4 §3 cli.rs
- 🔵 **Usability**: The cap is "15-minute" in the reason row but "900 s"
  everywhere else
  **Location**: Phase 4 §5 reason row
- 🔵 **Compatibility**: Old and new binaries sharing `arxiv.lock` are not
  addressed
  **Location**: Migration Notes
- 🔵 **Compatibility**: The guard differential will not exercise the new
  `--ticket` invocation
  **Location**: Phase 4 Prompt structure tests
  `test_profile_invocations.py` collects only fenced commands and
  single-quoted worked queries, and the new command sits in blockquote prose.
- 🔵 **Compatibility**: No remedy row for a researcher that ends on an
  un-re-presented `waiting`
  **Location**: Phase 4 §5 reason row
- 🔵 **Compatibility**: The new third status for non-LLM callers is not
  flagged as a behaviour change
  **Location**: Phase 4 §3 CLI; §6 CHANGELOG
- 🔵 **Test Coverage**: No test that a reused number with a different nonce
  is unknown
  **Location**: Phase 2 tests
- 🔵 **Test Coverage**: Only one pairing of `present`'s verdict order is
  tested
  **Location**: Phase 2 tests
- 🔵 **Test Coverage**: `E_ARXIV_TICKET_LIVE` has no binary or domain test
  **Location**: Phase 4 binary tests
- 🔵 **Test Coverage**: "`Unknown` liveness counts as live" is a stated
  decision with no test
  **Location**: Phase 3 tests
- 🔵 **Test Coverage**: Some domain assertions only read back values the fake
  was scripted to return
  **Location**: Phase 4 domain tests (`ScriptedQueue`)

#### Suggestions

- 🔵 **Architecture**: State the lock hierarchy and the order of dropping the
  turn and leaving the queue
  **Location**: Admission loop; Phase 3 The `Place`
- 🔵 **Architecture**: The queue and outcome types look generic but are
  arXiv-specific
  **Location**: Phase 4 `FetchOutcome`; Phase 3 `FetchQueue`
- 🔵 **Code Quality**: `Attempts` carries `turn` and `spacing` as a data clump
  **Location**: Phase 1 §1
- 🔵 **Code Quality**: `FileFetchQueue` takes two `ScratchDir`s, one
  misleadingly named `contention_log`
  **Location**: Phase 3 Construction
- 🔵 **Safety**: Orphan lock and temp files are never pruned, and `remove`
  skips the containment check
  **Location**: Phase 3 Scratch helpers
- 🔵 **Usability**: The mismatch error names the differing arguments but not
  the values the ticket was issued for
  **Location**: Phase 4 §3 render.rs
- 🔵 **Compatibility**: `research-adapters` needs `rand = { workspace = true }`
  **Location**: Phase 3 §3
- 🔵 **Test Coverage**: Ticket-parsing edge cases and the new `ScratchDir`
  helpers are thinly tested
  **Location**: Phase 2 tests; Phase 3 Scratch helpers

### Strengths

- ✅ Functional core, imperative shell: `research::sources::queue` is pure
  policy over fixed `SystemTime`s, and all file and `flock` effects sit behind
  the `FetchQueue`/`Place` port, enforced by cargo-pup.
- ✅ The short-held `queue.lock` is separate from the long-held `arxiv.lock`.
  Per-ticket flocks give kernel-maintained liveness, so a killed invocation
  frees its place without PID logic.
- ✅ `ServingTurn` is an RAII guard and `Place` methods consume `Box<Self>`,
  so the type system enforces "release exactly once".
- ✅ The ticket-lock opens and unlinks all happen under `queue.lock`, which
  avoids the race where one process locks an inode another has just unlinked.
  No path takes `arxiv.lock` while holding `queue.lock`.
- ✅ Boundary arithmetic falls out of the existing inclusive
  `Deadline::admits_attempt_after`, and every threshold pair (32/33, 38/39,
  300/301, 900/901, 950/400) is tested on both sides.
- ✅ Whole-call admission moves `confirmations.recall` under the lock, which
  closes 0280's duplicate-OAI gap. Spacing from `last_sent` closes the gap
  left by a call killed mid-request.
- ✅ The domain timeline (`Served`, `Attempt`, `Slept`, `Released` from
  `Drop`) checks that backoff runs inside the turn, not just how often it
  runs.
- ✅ The stdout contract only grows: `waiting` joins the `status`-tagged
  union, `StoredState` reads in both directions, and rollback leaves only
  inert files.
- ✅ The 900 s cap is enforced in Rust, not left to the prompt, and
  re-presentations are correctly left out of the profile's call cap.
- ✅ The domain vocabulary (ticket, binding, standing, presence, place, turn,
  front, expiry, cap) mirrors the work item's terms.

### Recommended Changes

1. **Add an age backstop for `Unknown` liveness** (addresses: `Unknown`
   liveness with no backstop; "`Unknown` counts as live" untested)
   In `Queue`, treat a standing as absent once `now − issued_at` exceeds the
   cap plus one call budget, whatever its presence, and report it once. Add a
   policy test and an adapter test with an unprobeable `.lock`.
2. **Bound the loop in degraded mode** (addresses: degraded mode loses the
   cap; doubly-degraded fail-open)
   With no persistent queue, degraded mode returns `lock_contention` when the
   budget runs out rather than `waiting`. When neither lock can be held, it
   returns `rate_limited` rather than serving unlocked. Add tests for both.
3. **Reword the Waiting outcome and make the CLI tolerant** (addresses: the
   appended `--ticket` breaks; profile does not say latest ticket or
   immediately; guard differential misses `--ticket`)
   Say "replace any `--ticket` with the `ticket` from the latest `waiting`
   output, straight away". Add a fenced example command so the differential
   collects it. Have the attended stub change the ticket once.
4. **Make `is_front` cheap and deadline-bounded** (addresses: O(N)
   bookkeeping per poll; blocking lock with no deadline; `is_front` side
   effects and failure path)
   `is_front` lists file names and probes only lower-numbered `.lock` files,
   stopping at the first live one, with no JSON reads. Stamping and pruning
   move to `join`, `step_aside` and `leave`. Take `queue.lock` through a
   non-blocking poll bounded by `Deadline`. Specify the failure policy as
   "fail open to front, with a diagnostic". Correct the load estimate in
   Performance Considerations.
5. **Run `present` before housekeeping, and skip housekeeping on rejection**
   (addresses: expired mismatch unreachable; rejected presentations change
   the queue)
   Evaluate `present` on the unpruned snapshot. Stamp and prune only on
   `Join`, `Resume` or `OverCap`. Add a binary fixture with an unstamped or
   expired neighbour to the byte-for-byte mismatch test.
6. **Fix the numbering rule and ticket identity** (addresses: inconsistent
   `next_number`; `Ord` by number alone; reused number with another nonce)
   Use one rule: one more than the largest number among unpruned records.
   `Eq`/`Ord` cover `(number, nonce)`, and queue order is an explicit
   comparison by number. Add tests for both.
7. **Give running out of budget its own type, and use one admission
   function** (addresses: `Unavailable` reused; two admission loops;
   `fetch_arxiv` exit paths; duplicate rejection enums)
   `ServingTurn::paced → Result<(), OutOfBudget>`, and `until_settled` yields
   `Settled::OutOfBudget`, which each family maps explicitly. Use one
   `admitted(gate, clock, deadline, ready)` shared by both families, and drop
   `PacingGate::contended`. Split `fetch_arxiv` into join → admit → `serve_call`
   with a single settle. Define `TicketRejection` once.
8. **Give `E_ARXIV_TICKET_LIVE` a remedy** (addresses: no remedy; no test)
   The message ends "wait for that call to finish, then re-present the
   ticket", and the profile names it. Add a binary test.
9. **Close the test gaps** (addresses: the Test Coverage majors)
   - `spacing_runs_from_the_last_finish_when_it_follows_the_last_send`.
   - An adapter test for crossing 900 s while live.
   - A concurrent-join thread test.
   - A two-invocation mid-call exhaustion test that asserts 2 query hits and
     1 OAI hit.
   - An automated planner-guard test in `topic_outstanding.rs`.
   - A per-phase table of existing tests that are rewritten, deleted or kept.
10. **Replace the 1 s real-time upper bounds** (addresses: likely flakes)
    Move the promptness checks into deterministic adapter tests. In the
    default lane, keep real-time bounds of 2–3 s from instants the test
    controls, or move them to the stress lane.
11. **Settle the smaller semantics** (addresses: lazy stamping into a cap
    hit; slow 5xx misattributed; clock steps; corrupt records)
    - Stamp at `min(now, presented_ms + maximum invocation life)`.
    - Persist the last retryable reason so the cap reports it.
    - Saturate when `now` is earlier than a stored stamp.
    - Remove corrupt records only when their lock is `Free`.
12. **Tidy the docs and migration notes** (addresses: the Compatibility and
    Usability minors)
    - Migration Notes: a mixed-version window is possible.
    - Add a reason row for a `waiting` status that is never re-presented.
    - The CHANGELOG and `FETCH_HELP` flag the third status.
    - `--ticket` gets a doc comment.
    - Use "900 s" consistently.
    - Rename `E_TICKET_NOT_QUEUED` to `E_RESEARCH_USAGE` or
      `E_OPENALEX_TICKET_NOT_QUEUED`.
    - Add `rand = { workspace = true }`.
    - Add a throughput assertion to the stress test and idle-time figures to
      the validation run.

## Per-Lens Results

### Architecture

**Summary**: Structurally the plan is sound. Pure queue policy in `research`,
`flock` mechanics behind the port in `research-adapters`, and separate
serving and bookkeeping locks. The main risk is resilience: `Unknown`
liveness counts as live without the age backstop `lease.rs` pairs it with.
Smaller issues are port types with slightly wrong meaning, admission split in
two, and unspecified lock ordering and clock-step defences.

**Strengths**: the functional core and imperative shell are cleanly applied;
the short-held `queue.lock` is separate from the long-held `arxiv.lock`; the
`try_serve` → `ServingTurn` RAII reshape; the explicit reversal of 0280's
backoff-outside-the-lock decision; and domain language that follows the work
item's terms.

**Findings**:

- 🟡 major / medium — *Phase 3 `bookkept` step 2; Decided behaviours* —
  **Liveness `Unknown` treated as live with no backstop can stall the whole
  queue.** `lease.rs` documents that `Unknown` falls through to an age
  backstop, and the queue has none. Suggest treating an `Unknown` standing as
  absent past the cap, with a policy test.
- 🔵 minor / high — *Ports; Phase 4 `Settled::OutOfTurn`* —
  **`ServingTurn::paced` signals an out-of-budget refusal through
  `Unavailable`.** Suggest `Result<(), OutOfTurn>` or a
  `Paced::{Ran, BudgetShort}` enum.
- 🔵 minor / high — *Phase 1 §1; Phase 4 §1* — **Admission logic ends up
  split across two loops, with transient port scaffolding.** Decide the end
  state up front and avoid adding `contended` only to remove it.
- 🔵 minor / medium — *Phase 3 port and `bookkept`* — **The queue port has no
  failure channel or deadline after join.** Poll `queue.lock` non-blocking
  within the deadline, and specify the mid-wait failure behaviour.
- 🔵 minor / medium — *Phase 2 `present`/`is_expired`* — **Cross-process
  wall-clock arithmetic lacks the clock-step guard the pacing state has.**
  Define how future-dated stamps are treated, in the pure policy.
- 🔵 minor / medium — *Phase 3 `bookkept` step 1* — **Removing a corrupt
  record also removes a ticket lock that may be held.** Probe first, and
  consider a record `schema_version`.
- 🔵 suggestion / medium — *Admission loop; The `Place`* — **State the lock
  hierarchy and the order of dropping the turn and leaving.**
- 🔵 suggestion / low — *Phase 4 `FetchOutcome`; Phase 3 `FetchQueue`* —
  **The queue and outcome types look generic but are arXiv-specific.**
  Either name them for arXiv or reuse one `TicketRejection` type.

### Code Quality

**Summary**: The domain and adapter split is sensible, ownership expresses
lock lifetimes, and the vocabulary is strong. The risks are in Phase 4:
`fetch_arxiv` picks up orchestration with many exit paths, rejection variants
repeat across three enums, and `Unavailable` and `Settled` are reused for
"out of budget, keep your place".

**Strengths**: the queue policy is pure; consuming `Box<Self>` enforces
release exactly once; domain naming has a manual review gate; the deadline
arithmetic is reused; each phase ends green, with checks on comments; and
the degraded modes always report a diagnostic.

**Findings**:

- 🟡 major / high — *Phase 4 §1* — **`fetch_arxiv` gathers queue
  orchestration, admission and serving, with many early returns that must
  each settle the `Place`.** Split it into join → admit → `serve_call`, with
  one settle match.
- 🟡 major / medium — *Phase 4 `Settled::OutOfTurn`; Ports* — **"Out of
  budget, keep your place" is encoded through `Unavailable` and a shared
  `Settled` variant whose meaning depends on the source family.** Introduce
  `OutOfBudget` and map it per family.
- 🔵 minor / high — *Phase 2/3/4 enums* — **`Mismatch` and `AlreadyLive` are
  copied across three enums, and `OverCap` has inconsistent shapes.** Define
  `TicketRejection` once.
- 🔵 minor / medium — *Phase 3 The `Place`* — **`Place::is_front` reads as a
  pure query but stamps, prunes and deletes, and its failure path is
  unspecified.** Rename it or document the effect, and state the failure
  policy.
- 🔵 minor / medium — *Phase 2 `Binding`* — **`Binding` flattens the typed
  `ArxivRequest` into `verb + terms + Option<Limit>`.** Mirror the request as
  a sum type, and name `query`, `id` and `--limit`.
- 🔵 minor / medium — *Phase 1 §1; Phase 4 §1* — **Two admission polling
  loops, plus a transient `PacingGate::contended`.** Use one loop
  parameterised by readiness.
- 🔵 suggestion / medium — *Phase 1 `Attempts`* — **`Attempts` carries
  `turn` and `spacing` as a data clump.** Expose `spacing()` on the turn.
- 🔵 suggestion / low — *Phase 3 Construction* — **`FileFetchQueue` takes two
  `ScratchDir`s, one misleadingly named `contention_log`.** Introduce a
  `ContentionLog` value.

### Test Coverage

**Summary**: The test plan is strong: layered, with boundary pairs on both
sides, red-first lists per phase and a coverage table. The gaps:

- Some criteria are mapped to fakes that cannot fail.
- The spacing rule is under-pinned.
- Concurrent joins are checked only by hand.
- The rewritten and deleted tests are not listed.
- The "within 1 s" real-time bounds are likely to flake.

**Strengths**: every timing threshold is tested on both sides; the timeline
assertions show what runs inside the turn; the layers match the risk; the
mismatch check is byte-for-byte; the stress lane follows the opt-in pattern;
and the prompt changes have structural tests.

**Findings**:

- 🟡 major / high — *Phase 1 adapter tests* — **No test pins "later of
  `last_finish` and `last_sent`" when `last_finish` is the later one.** Add
  `spacing_runs_from_the_last_finish_when_it_follows_the_last_send`.
- 🟡 major / high — *Phase 4 domain* — **The 900 s crossing criterion is
  mapped to a fake that cannot fail for the real risk.** Add an adapter test
  that resumes a ticket at 899 s and advances the clock.
- 🟡 major / medium — *Phase 3* — **Concurrent joins are checked only by a
  manual diff read.** Add an N-thread join test that asserts numbers 1..=N.
- 🟡 major / high — *Phase 1 and Phase 4* — **No list of existing tests whose
  expected outcome changes or which are deleted.** Name each one, including
  `a_confirmation_inherits_what_remains_of_the_deadline` and the `passes()`
  users.
- 🟡 major / medium — *Phase 4 domain* — **The re-presentation half of the
  mid-call exhaustion criterion is untested.** Add a two-invocation test
  asserting 2 query hits and 1 OAI hit.
- 🟡 major / medium — *Real-time binary tests* — **"Within 1 s" upper bounds
  in default-lane real-time tests are likely to flake.** Use instants the
  test controls, prove promptness deterministically, and keep 2–3 s smoke
  bounds.
- 🟡 major / medium — *Phase 5 §1* — **The planner regression guard is a
  one-off manual check, though it is deterministic.** Add a
  `topic_outstanding.rs` test.
- 🔵 minor / medium — *Phase 2* — **No test that a reused number with a
  different nonce is unknown.**
- 🔵 minor / medium — *Phase 2* — **Only one pairing of `present`'s verdict
  order is tested.** Add mismatch+cap, mismatch+live and live+cap.
- 🔵 minor / high — *Phase 4 binary* — **`E_ARXIV_TICKET_LIVE` has no binary
  or domain test.**
- 🔵 minor / medium — *Phase 3* — **"`Unknown` liveness counts as live" is a
  stated decision with no test.**
- 🔵 minor / medium — *Phase 4 `ScriptedQueue`* — **Some domain assertions
  only read back scripted values.** Back position 2 with a binary test, and
  assert `leave` exactly once on every settled outcome.
- 🔵 suggestion / medium — *Phase 2; Phase 3* — **Ticket-parsing edge cases
  and the `ScratchDir` helpers are thinly tested.**

### Correctness

**Summary**: The core concurrency design holds up. Ticket-lock opens and
unlinks happen under `queue.lock`, lock ordering is deadlock-free, and the
boundary arithmetic is right. The weaker parts are bookkeeping semantics:

- pruning before `present` makes the expired-mismatch rule dead and breaks
  "queue unchanged";
- numbering is described inconsistently;
- `Unknown` is treated as live with no backstop;
- appending `--ticket` breaks on a changed ticket.

**Strengths**: no flock-on-unlinked-inode race; deadlock-free lock ordering;
correct boundaries against the inclusive `<=`; spacing from
`max(last_finish, last_sent)`; and `present` checks expiry before the cap.

**Findings**:

- 🟡 major / high — *Phase 3 `bookkept`/`join` vs Phase 2 `present`* —
  **Bookkeeping prunes expired records before `present`, so an expired
  mismatch can never be detected.**
- 🟡 major / medium — *Phase 3 steps 3–4; Phase 4 mismatch test* —
  **Rejected presentations still change the queue through stamping and
  pruning.**
- 🟡 major / medium — *On-disk queue vs `next_number`* — **Ticket numbering is
  "largest live record" in one place and "maximum number" in another.**
- 🟡 major / medium — *Decided behaviours; Phase 3 step 2* — **A ticket whose
  liveness is persistently `Unknown` blocks the front for good, with no
  backstop.** Also specify what `join` does if locking the new ticket fails.
- 🟡 major / medium — *Phase 4 §5; Phase 3 Degraded mode* — **"Re-run the
  identical command with `--ticket` appended" breaks when a call returns a
  different ticket.** Say "replace", or use `overrides_with`.
- 🔵 minor / medium — *End stamping; `present`* — **Lazy "now" stamping of
  killed tickets can turn an expired ticket into a cap hit.** Stamp at
  `min(now, presented_ms + maximum life)`.
- 🔵 minor / medium — *Phase 2 `Ticket`* — **`Ord` by number alone is
  inconsistent with nonce-aware equality.**
- 🔵 minor / medium — *Phase 3 `bookkept`* — **Blocking acquisition of
  `queue.lock` has no deadline.**
- 🔵 minor / low — *Admission loop* — **The admission check and the first
  pre-attempt check read the clock separately.** A ticket can be served
  without making a request, and a resumed ticket can lose one turn.
- 🔵 minor / low — *Decided behaviours; `Settled::OutOfTurn`* — **A
  persistently slow upstream failure ends as `lock_contention` after 15
  minutes.** Persist the last retryable reason on the ticket.

### Performance

**Summary**: Admission and serving throughput are sound, and spacing hides
most of the handoff delay. The main risk is that every 100 ms `is_front`
runs full O(N) bookkeeping under one blocking lock, which is O(N²) in
aggregate. Smaller costs: fsync on disposable state, the agent-turn cost of
re-presentation, and stress assertions that check only lower bounds.

**Strengths**: `recall` under the lock removes duplicate OAI requests;
spacing from `last_sent` hides the handoff; skipping absent tickets protects
throughput; pruning keeps the directory bounded; and the accepted idle-lock
costs are stated.

**Findings**:

- 🟡 major / high — *Phase 3 `bookkept`/`is_front`; Performance
  Considerations* — **Each 100 ms `is_front` poll runs a full O(N)
  bookkeeping pass under one blocking exclusive lock.** List names, probe
  only lower `.lock` files, and move housekeeping to join, step-aside and
  leave.
- 🔵 minor / medium — *Phase 3 writes; Phase 1 `last_sent`* — **Fsync'd
  atomic writes of disposable state happen while contended locks are held.**
- 🔵 minor / medium — *Stress; Validation* — **The stress test and the
  validation run check only the lower bound on request gaps, never
  throughput.** Add a maximum-gap or total-duration bound, and record idle
  time.
- 🔵 minor / medium — *Prompts; Performance Considerations* — **The
  agent-turn cost of re-presentation is neither estimated nor measured.**

### Usability

**Summary**: The main consumer, the LLM researcher, gets a simple contract:
`waiting` exits 0, the ticket is a plain word, and the cap is enforced in
Rust. The friction is at the edges. `E_ARXIV_TICKET_LIVE` gives no remedy,
the profile does not say that the ticket can change, degraded mode loses the
cap, and error codes and units are inconsistent.

**Strengths**: `waiting` fits the `status`-tagged shape; re-presenting is a
small change to the command; the cap is enforced in Rust; the mismatch error
names the differing arguments and gives a recovery; the reason row has the
right remedy; and the contract is tested by an attended model check.

**Findings**:

- 🟡 major / high — *Phase 4 §3, §5* — **`E_ARXIV_TICKET_LIVE` gives the
  agent no remedy, yet exit 2 tells it to "correct the call".**
- 🔵 minor / medium — *Phase 4 §5* — **The profile does not say the ticket can
  change, or that the agent should re-present at once.**
- 🔵 minor / medium — *Phase 3 Degraded mode* — **Degraded mode silently
  removes the cap the agent's loop depends on.**
- 🔵 minor / high — *Phase 2 `RequestError`* — **`E_TICKET_NOT_QUEUED`
  breaks the existing error-code naming pattern.**
- 🔵 minor / high — *Phase 4 §3 cli.rs* — **`--ticket` is added without a
  per-argument doc comment.**
- 🔵 minor / high — *Phase 4 §5, §6* — **The cap is called "15-minute" in the
  reason row but "900 s" everywhere else.**
- 🔵 suggestion / medium — *Phase 4 §3* — **The mismatch error names the
  differing arguments but not the values the ticket was issued for.**

### Safety

**Summary**: Protection of arXiv's one-connection, 3 s rule is strong: the
serving `flock` stays the only gate on requests, and `last_sent` closes the
kill gap. The gaps are in the queue's failure handling:

- degraded mode loses the cap that guarantees the loop ends;
- `Unknown` liveness has no backstop;
- corrupt-record deletion ignores liveness;
- the bookkeeping lock ignores the call deadline.

**Strengths**: whole-call admission prevents interleaving; `last_sent`
spacing; the queue orders requests while only the serving lock permits them;
every queue mutation happens under `queue.lock` with atomic writes; stamping
is conservative; a mismatch writes nothing; re-presentations are bounded in
normal operation; and pruning is narrowly scoped.

**Findings**:

- 🟡 major / medium — *Phase 3 Degraded mode; Phase 4 Prompts* — **Degraded
  unqueued mode loses the 900 s cap, so the researcher's re-presentation loop
  has no termination guarantee.** Put the issue time in the ticket, or fail
  terminal in degraded mode, and add a profile ceiling.
- 🟡 major / medium — *Phase 3 `bookkept` steps 2–4; Decided behaviours* —
  **A ticket whose liveness stays `Unknown` blocks the queue for good, with
  no age backstop.**
- 🔵 minor / medium — *Phase 3 step 1* — **Corrupt-record deletion ignores
  liveness and can unlink a live holder's ticket lock.** Read only files
  whose names parse as tickets, and probe before removing.
- 🔵 minor / medium — *Phase 3 `bookkept`* — **The blocking bookkeeping lock
  ignores the call deadline.**
- 🔵 minor / medium — *Phase 1 `try_serve`; Phase 3 Degraded mode* — **When
  `arxiv.lock` cannot be locked, the serving gate fails open across the whole
  call.**
- 🔵 minor / low — *Phase 2* — **Expiry and the cap use wall-clock
  arithmetic with no guard against clock steps.**
- 🔵 suggestion / medium — *Phase 3 Scratch helpers* — **Orphan lock and temp
  files are never pruned, and `remove` skips the containment check.**

### Compatibility

**Summary**: The plan is mostly sound. `waiting` is an added variant of the
tagged union, `--ticket` needs no guard changes, the state file reads in both
directions, and rollback is clean. The gaps are unacknowledged edges:
mixed-version sharing of `arxiv.lock`, a wrong claim about guard-differential
coverage, a missing reason row, and non-LLM callers.

**Strengths**: the stdout contract only grows; the state file reads in both
directions; the queue state is isolated; the ticket form is guard-safe; the
narrowed log meaning is called out; the public-API snapshot is regenerated
deliberately; and the OpenAlex contract is pinned.

**Findings**:

- 🔵 minor / medium — *Migration Notes* — **Old and new binaries sharing
  `arxiv.lock` are not addressed.**
- 🔵 minor / high — *Phase 4 Prompt structure* — **The guard differential
  will not exercise the new `--ticket` invocation.** Add a fenced example.
- 🔵 minor / medium — *Phase 4 §5* — **No remedy row for a researcher that
  ends on an un-re-presented `waiting`.**
- 🔵 minor / medium — *Phase 4 §3, §6* — **The behaviour change for non-LLM
  callers of `fetch arxiv` should be flagged as such.**
- 🔵 suggestion / high — *Phase 3 §3* — **`research-adapters` needs a `rand`
  dependency the plan does not list.**

---
*Review generated by /accelerator:review-plan*


## Re-Review (Pass 2) — 2026-10-06T14:25:01+00:00

**Verdict:** REVISE

All eight lenses ran again against the revised plan and the updated work
item. 40 of the 46 first-pass majors and minors are resolved, and 6 are
partly resolved. The revisions introduce 3 new majors. Each is narrow,
and none needs a redesign.

### Previously Identified Issues

- 🟡 **Architecture / Safety / Correctness**: `Unknown` liveness has no backstop — Partially resolved. `ABANDONED_AFTER` bounds parsed standings, but a set-aside or unreadable record behind a `Held`/`Unknown` lock still blocks the adapter's `is_front` for good.
- 🟡 **Safety / Usability**: Degraded mode loses the 900 s cap — Resolved (`SteppedAside::Unkept` → `lock_contention`).
- 🟡 **Correctness / Usability**: Appended `--ticket` breaks on a changed ticket — Resolved.
- 🟡 **Performance**: `is_front` runs full bookkeeping under a blocking lock — Resolved (lock-free `is_front`, deadline-bounded `queue.lock`).
- 🟡 **Correctness**: Expired mismatch unreachable — Resolved.
- 🟡 **Correctness**: Rejected presentations mutate the queue — Resolved.
- 🟡 **Correctness**: Inconsistent numbering rule — Resolved.
- 🟡 **Code Quality**: `fetch_arxiv` exit paths — Resolved (join → admit → `serve_call` → single settle).
- 🟡 **Code Quality / Architecture**: Out-of-budget encoded as `Unavailable` — Resolved (`OutOfBudget`).
- 🟡 **Usability**: `E_ARXIV_TICKET_LIVE` has no remedy — Resolved.
- 🟡 **Test Coverage**: Spacing from `last_finish` — Resolved.
- 🟡 **Test Coverage**: 900 s crossing only against a fake — Resolved.
- 🟡 **Test Coverage**: Concurrent joins only by manual read — Resolved.
- 🟡 **Test Coverage**: No list of changed tests — Partially resolved. The tables leave out two OpenAlex refusal tests in `openalex_fetch.rs:365-399`.
- 🟡 **Test Coverage**: Re-presentation after mid-call exhaustion — Resolved.
- 🟡 **Test Coverage**: Flaky 1 s bounds — Resolved.
- 🟡 **Test Coverage**: Planner guard manual only — Partially resolved. The new automated test cannot pass as written (see new issues).
- 🔵 **Correctness**: Lazy stamping turns expiry into a cap hit — Partially resolved. Written stamps are bounded, but `present` judges an unstamped presented ticket as never expired.
- 🔵 **Correctness / Safety**: Clock steps — Partially resolved. The claim that "the age backstop bounds a future stamp" is false when `issued_at` itself is in the future.
- 🔵 **Performance**: Fsync on disposable state — Partially resolved. Queue records skip fsync, but the per-attempt `last_sent` write still fsyncs inside the turn.
- 🔵 **Safety**: Orphan lock and temp files — Partially resolved. Orphan locks are pruned, but `.tmp-*` files from killed writers are not.
- 🔵 **Architecture**: Transient `contended` scaffolding — Accepted as the price of a green Phase 1; downgraded to a suggestion.
- 🔵 All other first-pass minors and suggestions (Compatibility ×5, Usability ×6, Code Quality ×6, Architecture ×5, Safety ×4, Performance ×2, Test Coverage ×6) — Resolved.

### New Issues Introduced

#### Major

- 🟡 **Correctness / Safety**: Liveness probes briefly take the ticket lock, so lock-free `is_front` polls can make a resume fail or read as already live. `probe_liveness` takes `LOCK_EX|LOCK_NB`, and 24 waiters probe outside `queue.lock`. A resuming `join` can then see its own presented ticket as `Held` (a spurious `E_ARXIV_TICKET_LIVE`) or fail to take its own lock, and the plan does not say what happens next.
- 🟡 **Code Quality / Architecture**: The front-of-queue rule is implemented twice. `FilePlace::is_front` reimplements the rule in the adapter, and `Queue::is_front` has no production caller.
- 🟡 **Test Coverage**: The planner-guard test cannot pass as written. `outstanding --limit` is an `Option` with no default; the 24 comes from `research.topic.concurrency` and reaches the binary as `--limit {concurrency}`.

#### Minor

- 🔵 **Correctness**: The settle step maps `Unkept` to `lock_contention` even when `last_retryable` is set, which misattributes an upstream failure in degraded mode.
- 🔵 **Correctness**: The `queue.lock` poll bound is under-specified. If it is measured through `admits_attempt_after`, `step_aside` and `leave` give up without trying; if through raw remaining time, `join` can spend the whole budget and then end a valid ticket terminally.
- 🔵 **Correctness**: "The caller's own ticket is `Live`" is ambiguous for `join` and would reject every resume.
- 🔵 **Correctness**: A timed-out `step_aside` drops `last_retryable`, so the cap can report a stale reason.
- 🔵 **Safety**: Write and remove failures inside bookkeeping are unspecified. A failed record write on `join` could reopen an unbounded loop.
- 🔵 **Architecture**: The "cap after upstream failure is not contention" rule is decided in both the adapter and the domain.
- 🔵 **Architecture**: `is_front` has no failure channel when the directory cannot be listed, so it fails open silently.
- 🔵 **Code Quality**: Overlapping names: `OutOfBudget`/`OutOfTurn` for one concept, and `Served` for both a turn and an outcome.
- 🔵 **Code Quality**: The degraded `UnqueuedPlace` is returned as `Joining::Queued`.
- 🔵 **Compatibility**: The profile, `research.md` and the Pacing docs omit the upstream-reason carve-out from `lock_contention`.
- 🔵 **Compatibility**: One `lock_contention` cause covers two conditions (cap vs unusable queue) with one remedy.
- 🔵 **Usability**: The reworded call budget drops the guard-block and usage-error exemptions and reuses "request".
- 🔵 **Usability**: The waiting summary line says "after 0 attempts".
- 🔵 **Test Coverage**: `seed_ticket` cannot set `last_retryable` or `schema_version`, and the upstream-failure cap has no binary test.
- 🔵 **Test Coverage**: Turn-before-`leave` ordering is not asserted, and the `leave` timeout, unreadable-record and vanished-lock paths are untested.

#### Suggestions

- 🔵 **Compatibility**: Number tickets from all ticket-named files, so version-skewed records cannot collide.
- 🔵 **Usability**: Make "treat an unknown status as not yet settled" concrete for scripts.
- 🔵 **Usability**: Make the `E_ARXIV_TICKET_LIVE` remedy actionable without seeing the other fetch.
- 🔵 **Performance**: Cache `issued_at` in `FilePlace` rather than re-reading it every poll.
- 🔵 **Safety**: Widen the fail-open docs to cover per-call lock errors and lost spacing, and log unlocked turns.
- 🔵 **Test Coverage**: The kill tests are duplicated across `arxiv_pacing.rs` and `arxiv_queue.rs`, and `arxiv_pacing.rs` is missing from the Phase 4 table.

### Assessment

The first pass's structural problems are fixed. The queue's failure modes
are bounded, bookkeeping is ordered correctly, the types say what the
domain means, and the test plan pins the boundaries. The three new majors
are targeted:

- probe with `LOCK_SH` and retry the owner's own lock briefly;
- move the front decision into one pure domain function that the adapter
  feeds;
- split the planner-guard test into the config-default assertion plus a
  `--limit 24` binary test.

Most of the new minors are one-line specification gaps in Phase 3's
failure paths. One more short revision round should bring the plan to
COMMENT or APPROVE.


## Re-Review (Pass 3) — 2026-10-06T14:43:26+00:00

**Verdict:** REVISE

All eight lenses ran against the plan as revised after pass 2. Every major
and minor finding from pass 2 is resolved or reduced to a suggestion. The
design-level lenses (Architecture, Correctness, Safety, Performance) found
no major issue. The three new majors all come from Test Coverage, and all
are about whether specific Phase 3 adapter tests can be written as
specified. None of them affects the design.

### Previously Identified Issues

- 🟡 **Correctness / Safety**: Probes take the ticket lock — Resolved (`LOCK_SH` probes, owner retry).
- 🟡 **Code Quality / Architecture**: Front rule implemented twice — Resolved (`is_front_given`); `Queue::is_front` remains without a production caller (now a minor).
- 🟡 **Test Coverage**: Planner-guard test cannot pass — Resolved (`--limit 24` test plus the catalogue assertion).
- 🔵 **Architecture / Safety / Correctness**: `Unknown` backstop for unreadable records — Resolved (mtime fallback).
- 🔵 **Correctness**: Unstamped presented ticket judged unexpired — Resolved (effective end).
- 🔵 **Correctness**: `Unkept` ignores `last_retryable` — Resolved (`contention_or`).
- 🔵 **Correctness**: `queue.lock` poll bound — Resolved.
- 🔵 **Correctness**: "The caller's own ticket is `Live`" ambiguity — Resolved.
- 🔵 **Correctness**: Timed-out `step_aside` drops its reason — Resolved (resume clears it).
- 🔵 **Correctness / Safety**: Backward clock-step claim — Resolved (corrected and accepted).
- 🔵 **Safety**: Bookkeeping write and remove failures — Resolved.
- 🔵 **Architecture**: Contention decided in two layers — Resolved (domain `ContentionLog` port).
- 🔵 **Architecture**: Unlistable directory fails open silently — Resolved.
- 🔵 **Code Quality**: Overlapping names — Partially resolved. `OutOfTurn` is gone, but "settled" still has two meanings (`Settled::OutOfBudget` against `ServedCall::Settled`).
- 🔵 **Code Quality**: Degraded place returned as `Queued` — Resolved (`Joining::Unqueued`).
- 🔵 **Compatibility**: Upstream carve-out missing from the docs — Resolved.
- 🔵 **Compatibility**: One cause for two conditions — Partially resolved. The remedy clause is adequate, but the contention log does not record which condition fired.
- 🔵 **Usability**: Call-budget wording; "after 0 attempts" — Resolved.
- 🔵 **Test Coverage**: OpenAlex refusal tests missing from the table; no seeding for `last_retryable`; no binary test of the upstream cap; turn-before-`leave` order; duplicated kill tests — Resolved.
- 🔵 **Performance**: `last_sent` fsync — Resolved. `is_front` re-reads — Resolved (cache).
- 🔵 **Safety**: `.tmp-*` files and the fail-open docs — Resolved.

### New Issues Introduced

#### Major

- 🟡 **Test Coverage / Correctness**: Tests that age files by modification time ignore the test clock's wall origin. The adapter `RecordingClock` starts at `UNIX_EPOCH + 1_800_000_000 s`, so every freshly seeded file reads as months old. The abandonment and `.tmp-*` tests cannot pass as written, and `seed_raw_record` cannot set a modification time.
- 🟡 **Test Coverage**: The failed-record-write test cannot target a record name that contains a random nonce. `FileArxivQueue` has no seam for the nonce.
- 🟡 **Test Coverage**: The concurrent-join thread test runs bounded lock polling on a clock that never waits. `RecordingClock` is an `Rc` (not `Send`), and its virtual sleeps exhaust the poll budget within milliseconds of real time, so the test would flake.

#### Minor

- 🔵 **Test Coverage / Correctness**: The owner's 20 ms retry has no test that can exercise it. `probing_never_blocks_an_owner_or_another_prober`, as worded, contradicts the retry bound.
- 🔵 **Test Coverage**: Two existing-test claims are wrong. `paced` now writes the pacing state twice, and the Phase 1 timeline and outcome tests change once queue events join the timeline.
- 🔵 **Correctness**: Abandonment can make a ticket's end later and delay its expiry. A ticket issued 1050 s ago that ended 400 s ago becomes `OverCap` rather than `Join`.
- 🔵 **Performance / Architecture**: The fixed 20 ms `own()` window can turn a resume into `Unqueued` under scheduler load. That folds a per-ticket ownership failure into whole-queue unusability (`QueueUnusable`).
- 🔵 **Performance**: Front-first contender order probes every absent ticket ahead of the one being served. Walking from the nearest ticket ahead gives the same answer more cheaply.
- 🔵 **Usability / Compatibility**: The `E_ARXIV_TICKET_LIVE` remedy of three tries is used up in seconds because the agent cannot wait, and it invents a "reason `lock_contention`" that the CLI never produces.
- 🔵 **Usability**: "Then end on that status" in the Waiting outcome reads as ending the research after one settled call.
- 🔵 **Compatibility**: The `research.md` exit-code row (:92) and deadline paragraph (:110-112) are not in the update list, and there is no "status table".
- 🔵 **Safety**: Forward wall-clock steps and host sleep break the 100 s invocation-life assumption. Document it, as for backward steps.
- 🔵 **Code Quality**: `Queue::is_front` has no production caller.
- 🔵 **Code Quality**: "Settled" has two meanings. `until_settled` could return `Result<Settled<T>, OutOfBudget>`.

#### Suggestions

- 🔵 **Code Quality**: Spacing is reported by both `PacingGate` and `ServingTurn`.
- 🔵 **Usability / Compatibility**: Name the `Contention` kind in each `arxiv-contention.log` line, and point the reason row at the log rather than at stderr.
- 🔵 **Compatibility**: Say where the no-fsync write lives (likely a new `store` function), and add the `store` public-API snapshot to Phase 1.

### Assessment

The design has converged. Architecture, Correctness, Safety and Performance
report no majors, and every behavioural question from passes 1 and 2 is
settled. The pass 3 majors are confined to Phase 3's adapter test harness:

- seed modification times from the injected clock with `File::set_modified`;
- add a nonce source seam to `FileArxivQueue`;
- run the concurrent-join test on a clock that really sleeps, with one
  queue per thread.

The minors are short wording or specification edits. The one behavioural
change is to let abandonment only cap a ticket's end, never push it later.
A revision that addresses these should reach COMMENT or APPROVE without
another full pass.


## Re-Review (Pass 4) — 2026-10-06T15:43:43+00:00

**Verdict:** COMMENT

All eight lenses ran against the plan as revised after pass 3. Every
pass-3 finding is resolved or narrowed. One major remains, below the
threshold of three: a single Phase 3 test still cannot reach the code
path it targets. The plan is acceptable as it stands, and the findings
below are refinements.

### Previously Identified Issues

- 🟡 **Test Coverage / Correctness**: Modification-time tests ignore the test clock's wall origin — Resolved (`File::set_modified` from the test clock).
- 🟡 **Test Coverage**: The failed-record-write test cannot target a nonce-named record — Partially resolved. The nonce seam exists, but any ticket-named obstacle is read as an unparsed ticket and moves the new number past it.
- 🟡 **Test Coverage**: The concurrent-join test runs on a clock that never waits — Resolved (`SystemClock`, one queue per thread).
- 🔵 **Test Coverage / Correctness**: Owner retry untested — Partially resolved. Tests exist, but on a fresh `Join` the held lock is an orphan that housekeeping prunes; they must use the resume path.
- 🔵 **Test Coverage**: Existing-test claims — Resolved.
- 🔵 **Correctness**: Abandonment extends an end — Resolved.
- 🔵 **Performance / Architecture**: Fixed 20 ms `own()` window — Resolved, but see the new `own()` bound issue below.
- 🔵 **Performance**: Front-first probe order — Resolved.
- 🔵 **Usability / Compatibility**: `E_ARXIV_TICKET_LIVE` remedy — Resolved in substance; wording issues remain (below).
- 🔵 **Usability**: "End on that status" — Resolved.
- 🔵 **Compatibility**: `research.md` sections missed — Resolved.
- 🔵 **Safety**: Forward steps and host sleep — Resolved (documented and accepted).
- 🔵 **Code Quality**: `Queue::is_front`, two meanings of "settled", spacing on two ports — Resolved.
- 🔵 **Usability / Compatibility**: Contention log kinds; `store` location of the no-fsync write — Resolved.

### New Issues Introduced

#### Major

- 🟡 **Test Coverage / Correctness**: The failed-record-write test still cannot produce a write failure (as above). Use the resume path: seed an absent record and its lock, make the directory read-only, and re-present. Alternatively, add a record-writer seam.

#### Minor

- 🔵 **Architecture / Performance / Safety**: `own` polls for the whole of `join`'s bound while holding `queue.lock`. A stuck prober would stall the whole queue. Give `own` its own short bound and name the ticket locks in the lock-order paragraph.
- 🔵 **Architecture**: The queue adapter re-derives the serving window from `pacing.rs`'s private `SPACING`. Let the domain pass the window to `join`.
- 🔵 **Correctness**: Phase 4 layer 1 says `Unqueued` ends through `contention_or` at join, which contradicts the flow where it proceeds to admission.
- 🔵 **Correctness / Test Coverage**: The probe's open mode is unspecified. A read-only open makes a directory-as-lock probe `Free`, not `Unknown`. Specify read and write without create, as in `lease.rs`.
- 🔵 **Test Coverage**: The 38/39 s backoff tests do not say how a 6 s wait arises after one attempt. `BACKOFF[0]` is 3 s, so the first response needs `Retry-After: 6`.
- 🔵 **Test Coverage**: `RecordingGate` cannot see the clock's `Slept` events, and `is_inside` means "inside an attempt". Give the gate and clock one shared timeline, redefine `is_inside` as "turn alive", and record `is_inside` on `recall`.
- 🔵 **Test Coverage**: The real-time re-presentation tests have no synchronisation point before release. Wait for the resumed record or a `Held` probe.
- 🔵 **Safety**: `store::replace_without_sync` could erode `atomic_write`'s durability through shared helpers. Keep `atomic_write`'s syncs on its own path, and document both contracts in the `store` module doc.
- 🔵 **Code Quality**: `OutOfBudget`'s public `last_retryable` field has a split-ownership contract.
- 🔵 **Code Quality**: The `join` text still says the nonce comes from `rand`, and `Nonce` has no named constructor.
- 🔵 **Usability**: The `E_ARXIV_TICKET_LIVE` fallback names Failed, which the profile defines for exit 1, and the CLI's stderr remedy says to re-present.
- 🔵 **Usability**: `queue_unusable` has no next step for the operator.

#### Suggestions

- 🔵 **Code Quality**: `OutOfBudget` names three things; rename `contention_or` for the decision it makes.
- 🔵 **Usability**: "Line counts stay comparable" contradicts the CHANGELOG.
- 🔵 **Compatibility**: The contention-log format change is missing from the Migration Notes and the Pacing docs.
- 🔵 **Compatibility**: Add the unknown-status rule to the profiles as well as `FETCH_HELP`.

### Assessment

The plan is ready for implementation. The architecture is settled, and
Correctness, Safety, Performance, Architecture, Usability and
Compatibility report no majors. The remaining major is a test-setup
correction confined to one Phase 3 test. Every minor is a one- or
two-sentence specification fix. Most of them (the `own()` bound, the
probe open mode, the Phase 4 layer 1 wording, the backoff setup and the
shared timeline) would cost an implementer a red-test detour if left
unfixed, so applying them before Phase 1 starts is worthwhile. No
further full pass is needed.

### Final Verdict

APPROVE. All the pass 4 findings were applied to the plan without a
further pass, and the plan is marked ready for implementation.
