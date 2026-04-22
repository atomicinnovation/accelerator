---
date: "2026-04-22T16:45:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, performance, safety, portability]
review_pass: 2
status: complete
---

## Plan Review: Meta Visualiser Phase 4 — SSE Hub and Notify Watcher

**Verdict:** REVISE

The plan is well-structured and follows established codebase patterns faithfully: the Settings-based debounce mirrors `lifecycle::Settings`, the `sse_hub` module is clean and minimal, and the TDD sequencing is clear. However, multiple reviewers independently identified defects in the prescribed test code that would prevent the plan's own success criteria from being met — most critically, `collect()` on an unbounded SSE response will hang indefinitely in CI. Several type-level and concurrency correctness issues in the implementation code also need addressing before the plan is ready to execute. The architecture and safety findings are generally manageable given the v1 scope.

### Cross-Cutting Themes

- **SSE test body collection hangs** (flagged by: Test Coverage, Correctness, Code Quality, Architecture) — The `hub_event_arrives_on_sse_stream` test calls `collect()` on an infinite SSE stream; it will block forever and fail CI. The plan acknowledges the risk but leaves the broken test code in place. This needs a definitive fix in the plan before implementation.
- **etag empty-string sentinel** (flagged by: Architecture, Code Quality, Correctness) — Using `String::new()` as a sentinel for "no etag" is fragile. Three lenses independently recommend `Option<String>` with `skip_serializing_if = "Option::is_none"`. The plan acknowledges both options; it should commit to the safer one.
- **Debounce coalescing test reliability** (flagged by: Test Coverage, Portability, Correctness) — The `rapid_writes_coalesce_to_one_event` test cannot reliably distinguish correct debounce behaviour from OS event batching, and its timing assumptions differ between FSEvents (macOS) and inotify (Linux).
- **Full rescan contention** (flagged by: Architecture, Performance) — `rescan()` holds an index write lock for the full scan duration; concurrent file changes can stack parallel rescans and block all API readers. Acknowledged as a v1 tradeoff but the plan does not prescribe a mitigation.
- **Resource leaks in the watcher loop** (flagged by: Performance, Correctness) — Completed `JoinHandle` entries in `pending` are never removed; combined with the unbounded `mpsc` channel (flagged by Safety, Performance), the watcher has two separate unbounded memory growth paths.

### Tradeoff Analysis

- **Safety vs. Architecture on watcher supervision**: The safety lens wants the watcher task to be self-healing (restart on panic); the architecture lens wants it integrated into the shutdown supervision tree (JoinHandle in AppState, abort on shutdown). These are complementary, not conflicting — storing the handle in AppState enables both orderly shutdown and panic detection.
- **Performance vs. Simplicity on rescan serialization**: Adding a `Semaphore(1)` to serialize concurrent rescans limits lock amplification under burst events without requiring incremental indexing. This is a small addition consistent with the v1 scope.

### Findings

#### Critical

- 🔴 **Test Coverage + Correctness + Code Quality + Architecture**: SSE body-collection test hangs indefinitely
  **Location**: Step 4: `GET /api/events` route — `hub_event_arrives_on_sse_stream` test
  The test calls `response.into_body().collect().await` on an axum `Sse` response. An SSE stream is unbounded and persistent — `collect()` blocks until the stream closes, which never happens. The test will hang on every CI run. The plan acknowledges this in a note but leaves the hanging code in place without prescribing the fix. Replace `collect()` with a single `tokio::time::timeout`-wrapped `body.frame().await` call that reads one SSE chunk and asserts it contains `"doc-changed"`.

#### Major

- 🟡 **Correctness**: TOCTOU — deletion event silently dropped when concurrent rescan precedes `pre` capture
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced` function
  The `pre` entry is captured _after_ the debounce sleep. A concurrent watcher task for a different path can fire a rescan during that window, sweeping the deleted file from the index before `pre` is captured. `pre` is then `None`, the post-rescan `get` is also `None`, and the deletion event is silently dropped. Fix: capture `pre` immediately when the notify event arrives, before the debounce sleep, and pass it into `on_path_changed_debounced` as a parameter.

- 🟡 **Test Coverage**: File deletion path is untested
  **Location**: Step 3: `watcher` module — test suite
  The most structurally unusual code path — the deletion branch that uses the pre-rescan snapshot to determine `doc_type` — has no test. Add `file_deletion_produces_doc_changed_without_etag`: write a file, wait for the watcher to register, delete it, and assert a `SsePayload::DocChanged` arrives with no `etag` field in the serialised JSON.

- 🟡 **Code Quality + Correctness + Architecture**: `etag` empty-string sentinel is a type-level invariant violation
  **Location**: Step 2: `sse_hub` module — `SsePayload::DocChanged.etag`
  Using `etag: String::new()` as a deletion sentinel relies on an implicit contract that SHA-256 etags are never empty. Nothing in the type system enforces this, and future callers constructing `DocChanged` cannot distinguish intentional absence from an accidental empty string. Use `Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]` as the plan itself suggests. Deletion events pass `etag: None`; normal events pass `etag: Some(entry.etag.clone())`.

- 🟡 **Code Quality + Correctness**: Fragile frontmatter state check via string literal
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced`, frontmatter state check
  `entry.frontmatter_state == "malformed"` compares against a magic string. A rename of this value in `indexer.rs` would silently cause the watcher to emit `DocChanged` for malformed files instead of `DocInvalid`, breaking the spec requirement without a compile error. Define a shared constant (e.g. `pub const FRONTMATTER_MALFORMED: &str = "malformed"` in `indexer.rs`) or expose `FrontmatterState` as a public enum so the watcher can match on it directly.

- 🟡 **Architecture + Performance**: Full `rescan()` holds the index write lock on every file change, blocking all concurrent API readers
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced` function
  `rescan()` rebuilds and atomically replaces the entire index. With a 2000-file corpus (per the existing benchmark) a single rescan can take ~1 second. Five concurrent debounce tasks each call `rescan()` independently, stacking write-lock contention and serialising API reads for the full duration. The plan acknowledges this as a v1 tradeoff but does not prescribe a mitigation. Add a `Semaphore(1)` in `on_path_changed_debounced` so at most one rescan runs at a time, preventing lock amplification without requiring incremental indexing.

- 🟡 **Safety**: Watcher task panic leaves filesystem watching permanently dead
  **Location**: Step 3: `watcher` module — `spawn()` function; Step 5: `server::run()`
  The `JoinHandle` returned by `tokio::spawn` is discarded — if the watcher task panics, watching stops silently for the lifetime of the process with no operator indication. Store the handle (e.g. in `AppState` alongside the lifecycle handle) and log a prominent error on task exit. This also brings the watcher into the orderly shutdown path that the lifecycle task participates in.

- 🟡 **Architecture**: Watcher task has no cancellation path, diverging from the lifecycle supervision model
  **Location**: Step 5c: Spawn the watcher in `server::run()` / What we are NOT doing
  The lifecycle task participates in the server's shutdown sequence; the watcher task is an unlinked orphan. Mid-flight debounce tasks that survive axum's graceful shutdown can write to already-dropped state. Store the `JoinHandle` from `watcher::spawn` in `AppState` and abort it during the shutdown signal future, after `axum::serve` completes.

- 🟡 **Performance + Correctness**: Completed `JoinHandle` entries never removed from `pending` map
  **Location**: Step 3: `watcher` module — event loop `pending` HashMap
  The `pending` map is only pruned when a new event arrives for the same path. Completed handles for paths that are changed once and never again accumulate indefinitely. Before inserting a new debounce task, call `JoinHandle::is_finished()` on the existing entry and remove it if done, or perform a periodic sweep.

- 🟡 **Safety + Performance**: Unbounded `mpsc` channel allows memory exhaustion under file-change storms
  **Location**: Step 3: `watcher` module — `spawn()` function, mpsc channel creation
  A `git checkout` or build tool touching thousands of files will queue every notify event in heap memory with no backpressure. Replace `unbounded_channel` with a bounded `mpsc::channel(1024)`. In the notify callback, use `try_send` and log a warning on `TrySendError::Full`.

- 🟡 **Code Quality**: `on_path_changed_debounced` is a god function with five distinct responsibilities
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced` function
  The function sleeps the debounce, captures pre-rescan state, rescans, recomputes clusters, computes the relative path, and dispatches three SSE payload variants in a single 50-line body. Extract at least two pure helpers: `fn payload_for_entry(entry: &IndexEntry, rel: &str) -> SsePayload` and `fn deletion_payload(entry: &IndexEntry, rel: &str) -> SsePayload`. This reduces the main function to a coordinator and makes the payload logic independently testable.

- 🟡 **Test Coverage + Portability + Correctness**: Debounce coalescing test is fragile across FSEvents and inotify delivery models
  **Location**: Step 3: `watcher` module — `rapid_writes_coalesce_to_one_event` test
  With a 20ms debounce and 5ms write spacing, the test relies on all five events arriving within the debounce window. On macOS (FSEvents), the OS may coalesce writes before they reach `notify`, causing the debounce to fire after only one or two events — passing for the wrong reason. On a loaded CI machine, OS delivery latency can exceed the debounce window, causing spurious failures. Increase the debounce to 50ms, tighten write spacing to 2ms, and add a comment explaining the FSEvents coalescing behaviour.

- 🟡 **Portability**: End-to-end test relies on kernel filesystem notifications that may be unavailable in containerised CI
  **Location**: Step 6: End-to-end integration test — `sse_e2e.rs`
  The test requires inotify/FSEvents to deliver a notification within 800ms. In Docker-in-Docker or overlayfs environments, inotify watches can fail silently or be quota-limited. Add a guard: check the return value of `watcher.watch()` in the test setup and skip the test (or emit a clear message) if watching fails. Document the `fs.inotify.max_user_watches` requirement for CI operators.

#### Minor

- 🔵 **Performance**: `compute_clusters` clones all `IndexEntry` values while write lock is held
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced`, clusters recomputation
  `indexer.all().await` and `compute_clusters(...)` run while the `clusters` write lock is held, cloning every entry twice. Compute the new clusters _before_ acquiring the write lock, then swap the pre-built result in. This reduces the write-lock window to a pointer swap.

- 🔵 **Test Coverage**: No test verifies the JSON wire format of `SsePayload`
  **Location**: Step 2: `sse_hub` module — tests
  The `#[serde(tag = "type", rename_all = "kebab-case")]` and `#[serde(rename = "docType")]` annotations, plus the `skip_serializing_if` deletion omission, are not exercised by any test. Add a `#[test]` that calls `serde_json::to_string` on each variant and asserts the exact JSON matches the spec wire format, including the absent-etag case.

- 🔵 **Test Coverage**: Lagged-consumer test does not verify recovery after lag
  **Location**: Step 2: `sse_hub` module — `slow_consumer_gets_lagged_error` test
  After asserting `Err(RecvError::Lagged(_))`, the test ends without verifying that subsequent `recv()` calls succeed. Broadcast one more event after the lag and assert `rx.recv().await.is_ok()` to document the recovery semantics.

- 🔵 **Test Coverage**: E2E test uses production debounce (100ms) with no way to reduce it
  **Location**: Step 6: End-to-end integration test — `sse_e2e.rs`
  Unlike the unit tests, the E2E test cannot pass a shorter debounce because `server::run()` hardcodes `Settings::DEFAULT`. Consider exposing a `run_with_settings` entry point for tests, or extend the deadline to 2000ms to give enough headroom on slow CI runners.

- 🔵 **Safety**: Rescan failure silently suppresses the SSE event with no client signal
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced`, rescan failure path
  When `rescan()` returns an error, the function returns early and the client receives no notification. Document this as a known gap and ensure the Phase 5 client-side listener has a periodic poll fallback to recover from missed events.

- 🔵 **Safety**: Index and cluster stores are briefly out of sync after rescan
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced`, clusters write after rescan
  Between `rescan()` completing and the `clusters` write lock being acquired, the two stores hold different generations of data. For a local developer tool this is a transient rendering glitch, not data loss — note it as a known limitation.

- 🔵 **Safety**: Missing watched directories silently skipped with no recovery path
  **Location**: Step 3: `watcher` module — `spawn()` directory watching setup
  Directories absent at startup are skipped but never retried. If a configured doc-type directory is created after the server starts, its events are never received. Add this to the "What we are NOT doing" section so it is a conscious decision.

- 🔵 **Architecture**: `watcher::spawn` discards the doc-type mapping, causing silent drops on deletion for unindexed files
  **Location**: Step 3: `watcher` module — `spawn` function signature
  Passing `Vec<PathBuf>` discards the `HashMap<String, PathBuf>` doc-type keys. For files deleted before ever being indexed, `pre` is `None` and the event is silently dropped. Consider passing `HashMap<String, PathBuf>` so `doc_type` can be derived from configuration rather than index state.

- 🔵 **Code Quality**: E2E test `_ => break` arm silently passes on network errors
  **Location**: Step 6: End-to-end integration test — `sse_e2e.rs`
  The wildcard arm handles `Ok(Err(e))` (reqwest error), `Ok(Ok(None))` (stream close), and `Err(_)` (timeout) identically, all producing the same misleading "expected doc-changed event within 800ms" failure message. Separate the arms to give actionable CI diagnostics.

- 🔵 **Portability**: Incomplete platform backend enumeration in plan comment
  **Location**: Step 1: Dependencies — `notify` crate description
  The comment reads "FSEvents on macOS, inotify on Linux" — omitting Windows (ReadDirectoryChangesW). Update to include all three backends plus a note that Windows is not a deployment target.

#### Suggestions

- 🔵 **Architecture**: Template directory watching mentioned in overview but absent from implementation
  **Location**: Overview / Desired end state; Step 5c
  The overview says "Template tier-1 and tier-2 directories are also watched (per spec D9)" but Step 5c only collects `cfg.doc_paths`. Either explicitly defer this to a follow-on phase in "What we are NOT doing", or add `cfg.templates` path collection to Step 5c.

- 🔵 **Portability**: Relative path in SSE payloads uses OS path separator
  **Location**: Step 3: `watcher` module — `on_path_changed_debounced` — `rel` computation
  `to_string_lossy()` produces backslashes on Windows. Add an explicit `/`-join via `.components()` for future-proofing, with a comment noting the intent.

- 🔵 **Performance**: Broadcast channel capacity of 256 has no client resync signal on lag
  **Location**: Step 2: `sse_hub` module; Step 5: `AppState::build`
  When a client lags, it silently misses events. Note for Phase 5 that the client should perform a full index re-fetch on receiving a `Lagged` count, rather than assuming its view remains current.

### Strengths

- ✅ The `sse_hub` module is exemplary — single-responsibility, minimal public interface, directly testable in isolation without filesystem dependencies.
- ✅ The configurable `Settings` debounce pattern correctly mirrors `lifecycle::Settings`, keeping tests fast without test-only conditionals in production code.
- ✅ The four `sse_hub` unit tests cover all meaningful broadcast channel boundary conditions: happy path, fan-out, zero-subscriber no-panic, and lagged-consumer error.
- ✅ Placing `SsePayload` in `sse_hub.rs` as the canonical wire-format type is correct layering — the hub owns the event contract.
- ✅ Moving `_watcher` into the async task body is the correct RAII pattern for keeping the `RecommendedWatcher` alive.
- ✅ The pre-rescan capture of `indexer.get(&path)` for the deletion doc_type is explicitly planned and correctly positioned (despite the TOCTOU window, the intent is right).
- ✅ The plan explicitly scopes out what is not being done (recursive watching, `FileDriver::watch`, incremental indexing, `CancellationToken`), preventing scope creep and documenting future decision points.
- ✅ The `KeepAlive::default()` on the SSE response prevents proxy-timeout-induced reconnection storms on idle connections.
- ✅ The watcher tests use `tempfile` and real filesystem mutations rather than mocking `notify`, giving genuine OS-level integration confidence.
- ✅ The five-step TDD sequencing with per-step success criteria is well-ordered and prevents compilation errors during progressive development.

### Recommended Changes

1. **Fix the `hub_event_arrives_on_sse_stream` test** (addresses: SSE body-collection test hangs)
   Replace `collect()` with `tokio::time::timeout(Duration::from_millis(500), response.into_body().frame()).await`, assert the chunk contains `"doc-changed"`. Prescribe this in the plan as the required implementation, not an afterthought.

2. **Change `etag` to `Option<String>`** (addresses: etag empty-string sentinel)
   Update `SsePayload::DocChanged.etag` to `Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]`. Update all callers: deletion passes `etag: None`, normal events pass `etag: Some(entry.etag.clone())`. Remove the `String::is_empty` annotation.

3. **Capture `pre` before the debounce sleep** (addresses: TOCTOU deletion event)
   Pass the pre-event index snapshot as a parameter to `on_path_changed_debounced` (captured immediately on notify event receipt), rather than re-fetching it after the sleep.

4. **Add the deletion path test** (addresses: file deletion path untested)
   Add `file_deletion_produces_doc_changed_without_etag` to the Step 3 watcher test suite.

5. **Switch to bounded mpsc channel** (addresses: unbounded mpsc, memory exhaustion)
   Use `tokio::sync::mpsc::channel(1024)` and `try_send` in the notify callback with a warn log on full.

6. **Store and supervise the watcher JoinHandle** (addresses: watcher task panic, no cancellation path)
   Return or store the `JoinHandle` from `watcher::spawn` in `AppState` (alongside the existing lifecycle mechanism). Log a prominent error on task exit and abort the handle during shutdown.

7. **Add shared constant for `"malformed"` state** (addresses: fragile frontmatter string comparison)
   Define `pub const FRONTMATTER_MALFORMED: &str = "malformed"` in `indexer.rs` and use it in `watcher.rs`. Or expose `FrontmatterState` as a public enum.

8. **Add a `Semaphore(1)` to serialize concurrent rescans** (addresses: full rescan contention)
   Wrap the `rescan()` call with a single-permit semaphore so concurrent debounce tasks queue rather than stacking parallel write-lock holds.

9. **Extract payload helpers from `on_path_changed_debounced`** (addresses: god function)
   Extract `fn payload_for_entry(entry: &IndexEntry, rel: &str) -> SsePayload` and `fn deletion_payload(entry: &IndexEntry, rel: &str) -> SsePayload`.

10. **Fix debounce coalescing test timing** (addresses: coalescing test fragility)
    Use 50ms debounce, 2ms write spacing. Add a comment on FSEvents vs inotify coalescing behaviour.

11. **Add a CI guard to the E2E test** (addresses: E2E test in containerised CI)
    Check `watcher.watch()` return value and skip the test if watching fails. Document inotify watch count requirements.

12. **Add `SsePayload` JSON wire-format test** (addresses: no wire-format test)
    Add a `#[test]` in `sse_hub.rs` asserting exact JSON output for each variant.

13. **Clarify template watching scope** (addresses: template watching gap)
    Add template watching to "What we are NOT doing" explicitly, or add `cfg.templates` collection to Step 5c.

14. **Compute clusters outside the write lock** (addresses: compute_clusters holding write lock)
    Call `indexer.all().await` and `compute_clusters()` before acquiring `clusters.write()`, then swap in the result.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan introduces a clean two-module decomposition (sse_hub, watcher) that integrates naturally with the existing AppState and lifecycle patterns. The broadcast hub is a sensible, minimal abstraction and the debounce-via-configurable-Settings mirrors the established lifecycle pattern well. The main architectural concern is that the watcher performs a full rescan of the entire document corpus on every single file change event, coupling a per-file notification path to an O(N) I/O operation — this is an acknowledged design choice but the implications for other concurrent readers are not addressed. A secondary concern is the absence of any cancellation or supervision for the watcher task, which diverges from how other background tasks in the system are managed.

**Strengths**:
- The sse_hub module is cleanly scoped to a single responsibility: broadcast fan-out. Its public interface (new/subscribe/broadcast) is minimal and exposes no internal channel details to callers, making it straightforward to replace the underlying transport if needed.
- The Settings pattern for configurable debounce duration directly mirrors lifecycle::Settings, maintaining strong architectural consistency across background tasks and making the watcher unit-testable without production timing dependencies.
- Placing SsePayload as the canonical wire-format type in sse_hub.rs rather than in watcher.rs or api/events.rs is correct layering — the hub owns the event contract, and both the watcher (producer) and SSE handler (consumer) depend on the hub module rather than on each other.
- The plan explicitly acknowledges what is not being done (no incremental index, no FileDriver.watch method, no CancellationToken), which is honest architectural scoping that prevents scope creep and makes the tradeoffs visible.
- The watcher's spawn function signature takes concrete types (Arc<Indexer>, Arc<SseHub>, Arc<RwLock<Vec<LifecycleCluster>>>) that are already in AppState, so wiring into run() is a straight pass-through with no new ownership gymnastics.
- The doc_type derivation fallback for deleted files (capture pre-rescan entry before calling rescan) is a thoughtful handling of the deletion race that avoids silent drops for a common case.

**Findings**:
- **Major** (high confidence) — "Full rescan on every file change holds a write lock on all index entries, blocking all concurrent API reads" — Location: Step 3: watcher module — on_path_changed_debounced function — The `on_path_changed_debounced` function calls `Indexer::rescan()` on every file change event. `rescan()` replaces the entire `entries` HashMap under a write lock. While the plan explicitly acknowledges this is intentional for v1, it does not address the architectural consequence: every in-flight API request will be blocked until the full rescan write completes, and vice versa. With a large corpus, a sustained burst of file changes could cause periodic API latency spikes.
- **Major** (high confidence) — "Watcher task has no cancellation path, diverging from the lifecycle supervision model" — Location: Step 5c / What we are NOT doing — The lifecycle task participates in orderly teardown; the watcher task is an unlinked orphan. When `axum::serve` completes its graceful shutdown, in-flight debounce tasks may proceed against partially-cleaned-up state.
- **Minor** (high confidence) — "Empty-string sentinel for absent etag is a leaking internal encoding detail" — Location: Step 2b: sse_hub.rs — SsePayload::DocChanged etag field — The empty-string approach leaks an encoding decision. `Option<String>` with `skip_serializing_if = "Option::is_none"` makes the type-level contract explicit.
- **Minor** (medium confidence) — "watcher::spawn accepts raw Vec<PathBuf> rather than the configured doc_paths map, discarding doc-type mapping needed for deletion events" — Location: Step 3b: watcher.rs — spawn function signature — For files deleted before ever being indexed, `pre` is `None` and the deletion event is silently dropped.
- **Minor** (medium confidence) — "SSE route test uses collect() on an infinite stream, risking a test hang that the plan itself flags as likely" — Location: Step 4a: integration test hub_event_arrives_on_sse_stream — The plan documents the risk but defers the fix. Should be prescribed in the plan.
- **Suggestion** (medium confidence) — "Template directory watching is specified in the overview but absent from the watcher implementation" — Location: Overview / Desired end state — Either explicitly defer to a follow-on phase or add `cfg.templates` path collection to Step 5c.

### Code Quality

**Summary**: The plan is well-structured and follows established codebase patterns faithfully. The `sse_hub` module is clean and minimal. The `watcher` module has one notable design issue — `on_path_changed_debounced` does too much for a single function. There are also two correctness-adjacent quality issues: the `etag` field using `String::is_empty` as a deletion sentinel is a code smell, and the SSE integration test for `hub_event_arrives_on_sse_stream` is acknowledged as likely broken by the plan itself.

**Strengths**:
- The `sse_hub` module is exemplary — small, single-responsibility, and directly testable in isolation.
- Adopting `lifecycle::Settings` pattern for configurable debounce is the right call.
- The `watcher::spawn` function correctly moves `_watcher` into the async task to keep the `RecommendedWatcher` alive.
- Error paths in the watcher are logged with structured fields and never silently swallowed, matching the codebase's observability conventions.
- The plan explicitly names what is out of scope, which prevents scope creep.
- The deletion path correctly captures the pre-rescan entry to recover `doc_type`.
- The implementation sequence is clearly ordered to avoid compilation errors.

**Findings**:
- **Major** (high confidence) — "`on_path_changed_debounced` is a god function with five distinct responsibilities" — Location: Step 3: watcher module — Extract at least two helpers: `fn payload_for_entry` and `fn deletion_payload`.
- **Major** (high confidence) — "Empty string used as a deletion sentinel is primitive obsession" — Location: Step 2: sse_hub module — Use `Option<String>` with `skip_serializing_if = "Option::is_none"`.
- **Major** (high confidence) — "The plan acknowledges the second SSE route test will likely hang, but prescribes it anyway" — Location: Step 4: GET /api/events route — Prescribe `body.frame().await` with a `tokio::time::timeout` wrapper as the canonical approach.
- **Minor** (high confidence) — "Frontmatter state checked by string comparison against a magic literal" — Location: Step 3: watcher module — Define a named constant or expose `FrontmatterState` as a public enum.
- **Minor** (medium confidence) — "`watcher::spawn` has a 6-argument signature that signals a missing abstraction" — Location: Step 3: watcher module — Consider a `WatcherContext` grouping struct.
- **Minor** (medium confidence) — "E2E test uses `_ => break` arm, silently passing on network errors" — Location: Step 6: End-to-end integration test — Separate error/timeout/closed arms for actionable diagnostics.

### Test Coverage

**Summary**: The plan demonstrates strong testing discipline with TDD applied throughout and co-located unit tests for each module. Coverage is broadly proportional to risk. However, two meaningful gaps stand out: `hub_event_arrives_on_sse_stream` will hang, and there are no tests covering the file-deletion path through the watcher.

**Strengths**:
- TDD discipline is enforced throughout — tests are written before implementation in every step.
- The configurable-debounce Settings struct is a pragmatic and correct solution for avoiding sleep-heavy, flaky timing tests.
- The sse_hub unit tests cover all four meaningful boundary conditions for a broadcast channel.
- The watcher tests use tempfile for isolation and drive real filesystem mutations rather than mocking notify.
- The end-to-end test follows the same server-info.json poll pattern established in server.rs.
- The plan explicitly acknowledges the collect()-hangs risk and suggests a frame-by-frame alternative.

**Findings**:
- **Critical** (high confidence) — "SSE body-collection test will hang indefinitely as written" — Location: Step 4: GET /api/events route — hub_event_arrives_on_sse_stream test — Replace `collect()` with a single-frame read inside a `tokio::time::timeout`.
- **Major** (high confidence) — "File deletion path is untested" — Location: Step 3: watcher module — test suite — Add `file_deletion_produces_doc_changed_without_etag` test.
- **Major** (medium confidence) — "Debounce coalescing test has a structural race that may produce false positives" — Location: Step 3: watcher module — rapid_writes_coalesce_to_one_event test — Increase debounce to 50ms, tighten write spacing to 2ms.
- **Minor** (high confidence) — "Lagged-consumer test does not verify message delivery resumes after lag" — Location: Step 2: sse_hub module — After asserting Lagged, broadcast one more event and assert recv is Ok.
- **Minor** (medium confidence) — "No test verifies the JSON wire format of SsePayload" — Location: Step 2: sse_hub module — Add a test asserting exact JSON strings for each variant.
- **Minor** (medium confidence) — "E2E test uses production debounce (100ms), making it sensitive to CI timing" — Location: Step 6: End-to-end integration test — Either extend deadline to 2000ms or expose a test-only entry point.

### Correctness

**Summary**: The plan is logically well-structured and the core SSE broadcast/subscription mechanics are sound. Two issues stand out: the `hub_event_arrives_on_sse_stream` test will hang because `collect()` blocks indefinitely on a non-terminating SSE stream, and there is a TOCTOU window in `on_path_changed_debounced` where a deletion event can be silently dropped if a concurrent rescan runs between the `pre` capture and the one triggered by the debounce handler.

**Strengths**:
- The per-path debounce using JoinHandle::abort() is correct — aborting the previous handle and spawning a new one properly resets the debounce window.
- Keeping the RecommendedWatcher alive inside the spawned task via `let _watcher = watcher` is the correct ownership pattern.
- The pre-rescan capture of `indexer.get(&path)` before calling `rescan()` is the right approach for obtaining `doc_type` on deleted files.
- Using `broadcast::channel` semantics (slow consumers get Lagged) is correct for SSE fan-out.
- The `slow_consumer_gets_lagged_error` test correctly validates channel-drop behaviour.
- Concurrent calls to `rescan()` are safe because rescan builds a fresh HashMap and atomically swaps it.

**Findings**:
- **Critical** (high confidence) — "`collect()` on an SSE response body hangs indefinitely" — Location: Step 4: GET /api/events route — hub_event_arrives_on_sse_stream test — Replace with `frame().await` + timeout.
- **Major** (high confidence) — "TOCTOU: deletion event silently dropped when concurrent rescan precedes `pre` capture" — Location: Step 3: watcher module — on_path_changed_debounced — Capture `pre` immediately on notify event receipt, before the debounce sleep.
- **Major** (high confidence) — "Empty-string sentinel for absent etag is an incorrect invariant" — Location: Step 2: sse_hub module — Use `Option<String>`.
- **Major** (medium confidence) — "Fragile string comparison `== \"malformed\"` instead of enum matching" — Location: Step 3: watcher module — on_path_changed_debounced — Define shared constant or public enum.
- **Minor** (high confidence) — "`pending` HashMap retains completed JoinHandles indefinitely" — Location: Step 3: watcher module — spawn function — Prune on insert with `is_finished()`.
- **Minor** (medium confidence) — "Coalesce test cannot distinguish correct debounce from OS event batching" — Location: Step 3: watcher module — rapid_writes_coalesce_to_one_event — Add a clarifying comment noting the inherent non-determinism.

### Performance

**Summary**: The plan's watcher design has three meaningful performance concerns: a full `rescan()` on every file change, a `JoinHandle` map that leaks completed handles, and an unbounded mpsc channel. For a local-developer tool with typical meta directories (tens to low hundreds of files), the rescan cost is likely acceptable and is explicitly acknowledged as a v1 trade-off. The handle leak and unbounded channel are fixable with minimal effort.

**Strengths**:
- The per-path debounce design correctly aborts and replaces in-flight tasks, avoiding redundant rescan work during rapid file saves.
- The broadcast channel's fixed capacity with explicit Lagged error handling is a well-reasoned design.
- The KeepAlive::default() on the SSE response avoids proxy-timeout-induced reconnection storms.
- The Settings struct with configurable debounce keeps tests fast without test-only conditionals.
- Choosing RecommendedWatcher over polling is the right I/O efficiency choice.

**Findings**:
- **Major** (high confidence) — "Full `rescan()` reads every file across all doc types on every single file change" — Location: Step 3: watcher module — on_path_changed_debounced — Add a Semaphore(1) guard to serialise concurrent rescans.
- **Major** (high confidence) — "Completed `JoinHandle` entries are never removed from the `pending` map" — Location: Step 3: watcher module — event loop pending HashMap — Prune completed handles with `is_finished()` before each insert.
- **Minor** (high confidence) — "Unbounded mpsc channel between notify callback and the async task removes all backpressure" — Location: Step 3: watcher module — Use bounded channel(1024) with try_send.
- **Minor** (medium confidence) — "`compute_clusters` clones all `IndexEntry` values while the write lock is held" — Location: Step 3: watcher module — Compute outside the lock, swap in the result.
- **Minor** (medium confidence) — "Broadcast channel capacity of 256 may be insufficient under concurrent slow SSE subscribers" — Location: Step 2: sse_hub module — Note for Phase 5 that clients should re-fetch on Lagged.

### Safety

**Summary**: The plan introduces a background filesystem watcher with sound core data safety mechanisms: rescan uses an atomic write-lock swap and the pre-rescan capture for deletion doc_type is explicitly planned. The main safety concerns are an unrecoverable watcher task (no restart on panic), an unbounded mpsc channel creating a memory exhaustion path, and a brief data consistency window between the index and cluster writes.

**Strengths**:
- The rescan() implementation uses an atomic write-lock swap, so readers never observe a partially-updated index.
- The pre-rescan capture of the pre-deletion entry is explicitly planned and correctly placed.
- Missing watched directories are skipped with a warning rather than causing a hard failure.
- The broadcast channel uses a fixed-capacity buffer with Lagged semantics — slow SSE consumers are shed rather than blocking the broadcaster.
- The plan inherits the lifecycle::Settings pattern for configurable debounce.

**Findings**:
- **Major** (high confidence) — "Watcher task panic leaves filesystem watching permanently dead" — Location: Step 3: watcher module — spawn() function — Store the JoinHandle; log a prominent error on task exit.
- **Major** (high confidence) — "Unbounded mpsc channel allows memory exhaustion under file-change storms" — Location: Step 3: watcher module — spawn() function — Replace with bounded channel(1024) and try_send.
- **Minor** (high confidence) — "Rescan failure silently suppresses the SSE event with no client signal" — Location: Step 3: watcher module — on_path_changed_debounced rescan failure path — Document for Phase 5 client resync.
- **Minor** (medium confidence) — "Index and cluster stores are briefly out of sync after rescan" — Location: Step 3: watcher module — clusters write after rescan — Low-priority note; compute clusters outside the write lock.
- **Minor** (medium confidence) — "Missing watched directories silently skipped with no recovery path" — Location: Step 3: watcher module — spawn() directory watching setup — Add to "What we are NOT doing".

### Portability

**Summary**: The plan is well-suited to the project's macOS and Linux deployment targets. The existing codebase already uses Unix-only APIs, so the new dependencies do not introduce new platform coupling beyond what already exists. However, the plan's backend description is incomplete, one test's coalescing assertion makes a behavioural assumption that differs between FSEvents and inotify, and the end-to-end test's reliance on kernel filesystem notifications in containerised CI is undocumented.

**Strengths**:
- The plan correctly uses `notify::RecommendedWatcher` with automatic backend selection.
- `RecursiveMode::NonRecursive` is applied consistently and its semantics are uniform across FSEvents and inotify.
- The Settings struct with configurable debounce makes the watcher testable without OS-level timing.
- Missing directories are skipped with a warning rather than panicking.
- The watcher task keeps `_watcher` alive for the task lifetime correctly across all notify backends.

**Findings**:
- **Minor** (high confidence) — "Incomplete platform backend enumeration in plan comment" — Location: Step 1: Dependencies — Update to include ReadDirectoryChangesW and note Windows is not a target.
- **Major** (medium confidence) — "Coalescing test assertion is fragile across FSEvents vs inotify delivery models" — Location: Step 3: watcher module — rapid_writes_coalesce_to_one_event test — Document FSEvents coalescing behaviour; adjust timing.
- **Major** (medium confidence) — "End-to-end test relies on kernel filesystem notifications that may be unavailable in containerised CI" — Location: Step 6: End-to-end integration test — Add guard checking watcher.watch() return value; document inotify watch count requirements.
- **Minor** (high confidence) — "Relative path in SSE payloads uses OS path separator via to_string_lossy()" — Location: Step 3: watcher module — on_path_changed_debounced rel computation — Add explicit forward-slash join for future-proofing.

## Re-Review (Pass 2) — 2026-04-22

**Verdict:** COMMENT

### Previously Identified Issues

#### Architecture (6 findings)
- 🟡 **Full rescan holds write lock** — Resolved (Semaphore(1) serialises rescans; acknowledged as v1 tradeoff)
- 🟡 **Watcher task has no cancellation path** — Resolved (JoinHandle returned + supervisor task in Step 5c)
- 🔵 **Empty-string etag sentinel** — Resolved (changed to `Option<String>`)
- 🔵 **spawn discards doc-type mapping** — Partially resolved (pre-capture handles most cases; fast-create-then-delete still silent)
- 🔵 **SSE test uses collect()** — Resolved then regressed: broadcast-before-subscribe is incorrect for `tokio::sync::broadcast` → **fixed in pass 2 edits** (broadcast now happens after handler subscribes)
- 🔵 **Template watching gap** — Resolved (explicitly deferred in "What we are NOT doing")

#### Code Quality (6 findings)
- 🟡 **God function** — Partially resolved (`payload_for_entry` extracted; function still has 8 params — acceptable for v1, noted for future `WatcherContext` struct)
- 🟡 **Empty-string etag** — Resolved (`Option<String>`)
- 🟡 **SSE test hangs** — Resolved (uses `frame()` + timeout)
- 🔵 **Frontmatter magic string** — Resolved (`FRONTMATTER_MALFORMED` constant)
- 🔵 **6-argument spawn signature** — Still present (acceptable for v1)
- 🔵 **E2E test wildcard break** — Resolved (separate arms for network error, stream close, timeout)

#### Test Coverage (6 findings)
- 🔴 **SSE collect() hangs** — Resolved (uses `frame()` + timeout; broadcast-after-subscribe fixed in pass 2)
- 🟡 **Deletion path untested** — Resolved (new `file_deletion_produces_doc_changed_without_etag` test)
- 🟡 **Coalescing test race** — Resolved (50ms debounce, 2ms writes, explanatory comment)
- 🔵 **Lagged recovery not tested** — Resolved (recovery assertion added)
- 🔵 **No wire-format test** — Resolved (new `sse_payload_json_wire_format` test)
- 🔵 **E2E CI timing** — Resolved (2000ms deadline, inotify requirements documented)

#### Correctness (6 findings)
- 🔴 **collect() hangs** — Resolved (uses `frame()` + timeout)
- 🟡 **TOCTOU deletion race** — Resolved (pre captured before debounce sleep in event loop)
- 🟡 **Empty-string etag invariant** — Resolved (`Option<String>`)
- 🟡 **Fragile frontmatter string** — Resolved (`FRONTMATTER_MALFORMED` constant)
- 🔵 **Pending map leak** — Resolved (`retain` with `is_finished()`)
- 🔵 **Coalesce test non-determinism** — Resolved (improved timing, explanatory comment)

#### Performance (5 findings)
- 🟡 **Full rescan per change** — Partially resolved (Semaphore(1) prevents pile-up; per-event cost unchanged by design)
- 🟡 **Pending map leak** — Resolved (`retain` eviction)
- 🔵 **Unbounded mpsc** — Resolved (bounded 1024 + `try_send`)
- 🔵 **Clusters under write lock** — Resolved (computed outside lock, swap-only under lock)
- 🔵 **Broadcast capacity 256** — Partially resolved (lag path tested and logged; capacity unchanged)

#### Safety (5 findings)
- 🟡 **Watcher panic undetected** — Resolved (JoinHandle supervised, prominent error logged)
- 🟡 **Unbounded mpsc** — Resolved (bounded 1024)
- 🔵 **Rescan failure silent** — Still present (log-only, no client signal; acceptable for v1 developer tool)
- 🔵 **Index/cluster sync window** — Still present (sub-ms window; negligible for this tool's criticality)
- 🔵 **Missing dirs no recovery** — Still present (no retry; acceptable, should add warn log for non-existent dirs)

#### Portability (4 findings)
- 🔵 **Incomplete notify comment** — Resolved (Windows backend + platform note added in pass 2)
- 🟡 **Coalescing test fragile** — Resolved (50ms/2ms timing, FSEvents comment)
- 🟡 **E2E test in containers** — Partially resolved (documented but no skip guard; acceptable for now)
- 🔵 **OS path separator** — Resolved (`.replace('\\', "/")` added in pass 2)

### New Issues Introduced

- 🟡 **Architecture + Correctness + Test Coverage**: Broadcast-before-subscribe in `hub_event_arrives_on_sse_stream` test — `tokio::sync::broadcast` does not replay past messages to new subscribers, so the pre-broadcast event would be silently discarded. **Fixed in pass 2**: test now broadcasts after the handler has subscribed via `oneshot`.

### Assessment

All critical and major findings from the initial review have been addressed. The one new critical issue (broadcast-before-subscribe) was identified and fixed during this pass. The remaining items are minor observations appropriate for a v1 developer tool: the 8-parameter function signature, the 6-argument spawn signature, the rescan failure silence, and the E2E test lacking a programmatic CI skip guard. None of these warrant a REVISE verdict — the plan is ready for implementation.
