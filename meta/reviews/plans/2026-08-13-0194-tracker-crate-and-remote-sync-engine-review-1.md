---
type: plan-review
id: "2026-08-13-0194-tracker-crate-and-remote-sync-engine-review-1"
title: "Plan Review: Tracker Crate and Remote Sync Engine Implementation Plan"
date: "2026-08-13T09:02:44+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "work-item:0194"
target: "plan:2026-08-13-0194-tracker-crate-and-remote-sync-engine"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, correctness, test-coverage, code-quality, safety, compatibility, usability, standards]
review_number: 1
review_pass: 3
tags: [rust, sync, tracker, work-items, bash-parity]
last_updated: "2026-08-13T12:05:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Tracker Crate and Remote Sync Engine Implementation Plan

**Verdict:** APPROVE (see Approval below; passes 1-3 returned REVISE)

This is a strong plan — it names its oracles, takes six non-obvious decisions up front with their costs stated, verifies write ordering on a recorded slice rather than by inference, replaces bash's `exit 99` fault seam with an injected value, and makes preview fidelity structural rather than asserted. Eight lenses nevertheless converge on five critical gaps, all of the same kind: a property the plan states as settled that its own signatures or fixtures cannot deliver. The local digest recipe is not byte-equal to bash whenever either half normalises to empty; `decide`'s third input (`dirty`) has no producer anywhere in any phase; the report contract cannot express `indeterminate`, so a total tracker outage exits 0 with empty stdout; the pending-push marker's slug identity permits binding two local items to one remote issue; and `Apply<'a>` holding `&'a mut BaselineStore<'a>` will not compile usably. Two mechanical blockers also stop Phase 1 going green: it edits the wrong build-system guard file, and the documented exit code for a malformed `--resolve` contradicts the clap parser it chooses.

### Cross-Cutting Themes

- **Stated properties without a mechanism** (flagged by: correctness, compatibility, safety, architecture, code-quality) — five load-bearing claims have no implementation path: the digest recipe's byte-equality with bash, the `dirty` decision input, the run-start epoch (`corpus::Clock` has no epoch accessor), the `indeterminate` post-terminal state, and `BaselineStore`'s read side. Each will be improvised mid-phase, where the cheapest fix is the one that erodes the property.
- **The cross-process report contract is the least-pinned surface in the plan** (flagged by: usability, architecture, compatibility, code-quality, test-coverage) — the plan freezes `--help` with a golden and both crates' public APIs with cargo-public-api, but the tab-separated report that 0171's SKILL codes against is pinned by nothing, silently renames `prompt` to `unresolved`, cannot express `indeterminate`/`remote-absent`/degraded, and collapses the retryable/terminal split into an opaque `failed`.
- **Bash-parity oracles are narrower than the property they protect** (flagged by: test-coverage, correctness, compatibility) — the corpus asserts digest equality rather than AC 29's classification stability, has no empty-body/empty-frontmatter/numeric-ADF case, and its bash provenance is guaranteed only by a manual step. Classify and decide land as a second transcription in `cli/work/tests/fixtures/` rather than a shared golden, so the two implementations can drift green.
- **Safeguards present in the live bash path are dropped without being recorded** (flagged by: safety, correctness) — the SKILL's aggregate pull-overwrite gate (25-file threshold, fail-closed when non-interactive) has no replacement and no entry in "What we're NOT doing"; the `finalise` gate's "clean completion" is undefined, so a run that merely reported conflicts can advance the timestamp past a skipped item's mtime and pull over local edits next run.
- **Seams built to be frozen are shaped so 0171 cannot satisfy them** (flagged by: architecture, code-quality, compatibility, test-coverage) — `TrackerRegistry::resolve` returning `&dyn` forces eager construction of every client; the contract harness's two headline properties are configurations of the fake and unrunnable against a real client; and crate-level exclusion cannot gate a contract binary inside a client crate whose unit tests must keep running.

### Tradeoff Analysis

- **Non-interactivity vs blast-radius bounding**: the project deliberately rejects confirm/dry-run UX in favour of VCS revert, and `sync` is rightly orders-not-questions. But the dropped bulk-overwrite gate is a *quantitative* limit, not a prompt, and remote mutations plus uncommitted local edits are the two things VCS cannot recover. Recommendation: reinstate it as a fail-closed non-interactive refusal (`--max-pulls N` / distinct exit code, zero writes), which preserves the interaction model.
- **Bash parity vs correcting bash**: the plan improves on bash in three places (dropping the mtime sentinel, `proves_unchanged_since` over `==`, `proves_unchanged_since` in the bulk-then-`show` gate where bash uses string inequality). The first two are unambiguously right. The third is a genuine behavioural divergence for null-stamped issues; record it deliberately so the lifted call-count expectations are stated in terms of the new rule rather than transcribed from bash.
- **Domain purity vs public-api churn**: `plan`/`run` and the push-decide table sit outside `work` partly because "adapter-side types churn nothing". Recommendation: accept the churn and site pure policy in the domain — the snapshot exists to make domain-surface changes deliberate, not to discourage them.
- **Rich domain types vs the frozen port**: several findings ask for stronger types (`ExternalId` in the domain, `Option<Resolution>`, a three-valued dirty). None requires reopening 0204 — all are `work`-side shapes over the port's existing vocabulary.

### Findings

#### Critical

- 🔴 **Compatibility + Correctness**: The local digest recipe is not byte-equal to bash when either half normalises to empty
  **Location**: Phase 3, Section 2: Digest computation
  `work-item-normalise.sh:113-116` composes `printf '%s\n%s\n' "$fm" "$body"` over two command substitutions, so it emits both separators unconditionally; plain concatenation diverges by one newline whenever the body or the frontmatter reduces to nothing. No frontmatter/body splitter is named at all, and none of the five corpus cases has an empty body or absent frontmatter — so the plan's own oracle for AC 29 cannot detect the divergence that mass-reclassifies at cutover.

- 🔴 **Safety + Architecture + Correctness**: `decide`'s `dirty` input has no producer, and `None` means clean
  **Location**: Phase 2, Section 3; Phase 4, Section 1
  Key discoveries notes that `work/src/file_dirty.rs` "is exactly the `--dirty` input the decision table needs" and is wired into nothing — and no phase wires it. `LocalItem` is never defined, `plan()` takes no VCS status, and Phase 4's test list has no dirty-pull case. `Option<bool>` also inverts the existing fail-safe: `file_dirty::is_dirty` maps `VcsMode::Indeterminate` to dirty precisely because VCS revert cannot recover uncommitted changes.

- 🔴 **Usability + Architecture + Compatibility**: A total tracker outage is indistinguishable from a clean no-op run
  **Location**: Phase 4, Section 5 (with Section 1)
  `fetch_all` returning `Err` marks every id `Indeterminate`, `indeterminate` decides to `Noop`, and the report emits lines only for non-`Noop` decisions — so a wholly unreachable tracker yields empty stdout and exit 0. The bash SKILL this replaces reports those items under `needs-retry:` and `remote-absent:`. `TrackerError.detail` — the sole diagnostic for a pre-flight failure — has no destination anywhere.

- 🔴 **Safety + Usability + Correctness**: A slug-keyed marker holding an `external_id` can bind a second work item to another item's remote issue
  **Location**: Phase 5, Section 1, steps 1 and 4
  Step 1 reuses a stored `external_id` with nothing verifying the marker belongs to this attempt. The plan analyses the collision hazard only in the blocking direction. Two locals then claim one remote issue and `sync` pushes both over it via the whole-content `update` contract — remote data loss no VCS revert recovers. `read` returning `Option` also fails *open* on a torn marker, permitting the duplicate `create` AC 11 exists to prevent.

- 🔴 **Safety**: The bash path's aggregate pull-overwrite gate is dropped with no bound replacing it
  **Location**: Phase 4, Section 2; What we're NOT doing
  `sync-work-items/SKILL.md:172-181` refuses before any pull write when a run would overwrite more than the shared threshold of local files, and fails safe with zero writes when non-interactive. `run` executes every non-`Noop` action with no cap and no abort threshold, and the removal is not recorded. One mis-classification cause — recipe drift, a poisoned timestamp, a provider encoding change — then overwrites every local work item file in one unattended run.

- 🔴 **Code Quality + Architecture + Correctness**: `Apply<'a>` holding `&'a mut BaselineStore<'a>` will not compile usably
  **Location**: Phase 3, Section 4: Apply
  `&'a mut T<'a>` is invariant, so constructing `Apply` borrows the store for the whole of its own lifetime — but `run` needs the baseline for planning, for applying and again for `finalise`, and the resumability tests need to read it afterwards. It also duplicates the `AtomicWrite` reference in two places. Implementation hits this immediately, where the reflex fixes (clone, `RefCell`, `Option::take`) erode the design.

#### Major

- 🟡 **Test Coverage + Standards**: Phase 1 edits the wrong build-system guard file and cannot go green
  **Location**: Phase 1, Section 4
  The plan extends `tests/unit/tasks/test_integration.py`, which holds only shell-suite count floors. The guards that govern mise task registration are in `tests/unit/tasks/test_mise.py` — `test_every_integration_task_declares_its_launcher_need` (needs a `_NO_LAUNCHER_NEEDED` entry with a reason) and the roll-up guard. Adding the task as written turns `test:unit:tasks` red with no plan step to fix it. Also: the exclusion property AC 22 cares about is verified only by a manual eyeball, and `_MANIFEST` should be pinned by a unit assertion.

- 🟡 **Architecture + Code Quality + Safety**: No epoch-seconds clock port exists
  **Location**: Phase 4, Section 2
  `corpus::Clock` exposes only `now_utc_iso()` and `filename_timestamp(format)`; no phase touches `corpus`. The persisted timestamp is the sole gate on the hash-free local short-circuit, so a derived-too-large epoch silently disables local change detection corpus-wide. Either widen the shared port (listing its public-api churn and existing-implementor fan-out) or declare a narrow sync-owned clock port.

- 🟡 **Safety + Correctness**: "Clean completion" is undefined, so a reported-conflict run can mask local edits next run
  **Location**: Phase 4, Section 2
  If a run whose items were merely reported and skipped counts as clean, `finalise` advances the timestamp past those files' mtimes while their baseline entries hold the old `local_hash`. The next run's pre-filter declares the local side unchanged, a genuine `conflict` degrades to `remotely-modified`, and the table pulls over the local edits. AC 10 does not catch it — it only checks files the run *mutated*.

- 🟡 **Safety + Compatibility**: Clearing the baseline entry yields `conflict`, not `indeterminate`
  **Location**: Phase 5, Section 3 (and its tests)
  `Indeterminate` is reachable only from `RemotePresence::Indeterminate` — a failed read. A cleared entry leaves both hashes absent, so both sides default to changed: `conflict`. That matters beyond wording — `conflict` is *resolvable*, so a later `--resolve <id>=remote` pulls the remote over the local for an update that may never have applied, whereas a genuinely indeterminate item is never resolvable. The Phase 5 test as written would fail.

- 🟡 **Correctness + Compatibility**: An empty-string persisted hash is not mapped to "absent"
  **Location**: Phase 3, Section 3 → Phase 2, Section 2
  Bash's presence tests are emptiness tests (`// empty` + `[ -n ]`), and an empty `remote_hash` is a routine on-disk value — the plan itself persists `""` after a failed post-push `show`. `Entry` stores `String` while `BaselineEntry` takes `Option<&str>`, and the mapping is never stated. `Some("")` changes which branch runs and breaks the `expect_*_digest_called` assertions the lifted table rests on.

- 🟡 **Compatibility + Safety**: The shared `last-sync.json` has no cross-engine contract and no wire format
  **Location**: Phase 3, Section 3
  Bash tolerates per *key* (`jq -r '.local_hash // empty'`), so a malformed entry degrades one item; a strict whole-document Rust deserialise degrades the whole baseline. `render()` is unspecified against bash's `jq -c` shape (compact, `timestamp` first, insertion-ordered keys, trailing newline), and `BTreeMap` reorders. No test writes with one engine and reads with the other. Separately, read-modify-write granularity is unspecified — bash re-reads inside every `set`, so an in-memory whole-run document clobbers a concurrent writer's entries.

- 🟡 **Compatibility**: A Unicode-aware trim in `resolve_conflict_token` converts the safe default into a destructive write
  **Location**: Phase 2, Section 3
  Bash folds and trims ASCII-only in the C locale, so `"\u{00A0}remote"` stays unrecognised and skips. Rust's `to_lowercase()`/`trim()` are Unicode-aware and would resolve it to `AcceptRemote` — overwriting a dirty local file for whitespace no human can see, on the one path whose stated invariant is that the safe default is never destructive. `work/src/normalise.rs:5-10` already documents the ASCII-only precedent.

- 🟡 **Usability + Standards**: Malformed `--resolve` is documented as exit 1 but clap's `value_parser` exits 2
  **Location**: Phase 4, Sections 4 and 5
  `parse_key_value` returns `Err(String)` to clap, which surfaces as a usage error — exit 2, the same code the plan's table assigns to flag conflicts. So the published table ships wrong, and three flavours of bad caller input land on 2, 2 and 1. `--set`/`--append`/`--remove` already exit 2 on the same malformed pair.

- 🟡 **Usability + Compatibility**: `<id>\tfailed` drops the retryable/terminal split and no exit code covers a failed run
  **Location**: Phase 4, Section 5
  The 70/71 distinction is the safety-critical one in this domain and the plan ports it into `for_tracker_error`, then discards it per item. The detail goes to unstructured stderr with no correlation to the id. The table also has no row for "completed with N failures", and no precedence rule for a run with both failures and unresolved conflicts.

- 🟡 **Architecture + Code Quality**: Sync orchestration policy is sited in `work-adapters` without the siting argument
  **Location**: Phase 4, Sections 1–2
  The two-tier read rule, the `fetch_all`-error rule and resolution application are domain policy. They land in the one crate exempt from both the public-api pin and any `RestrictImports` rule, and because `plan()` takes `&dyn RemoteTracker` and fetches itself, none of it is unit-testable as a pure function. The plan argues the `work` → `tracker` edge at length but never argues where planning lives. Push-decide's pure table likewise lands in `work-cli`.

- 🟡 **Test Coverage + Architecture + Safety**: The contract harness cannot hold real clients, and its exclusion mechanism will not compose
  **Location**: Phase 1, Sections 2 and 4
  `unaccounted_id_is_indeterminate_not_absent` and `a_failing_read_is_retryable` are configurations of the fake (`truncating`/`losing`) — a real client cannot be asked to truncate a bulk fetch, so the two properties the harness exists for are unrunnable against the implementations it must hold. `run_all` over all three shapes also asserts round-trips a `losing` tracker should fail. Crate exclusion cannot gate a contract binary inside a client crate whose unit tests must keep running, so 0171 must invent the filter anyway; nextest 0.9.138 supports a profile `default-filter`, which the plan dismissed on the premise that no `nextest.toml` exists. The harness should also fail closed (an explicit opt-in env var), not merely be excluded by a string in a Python task file.

- 🟡 **Test Coverage**: Lifted-table scope is narrower than the AC, and classify/decide are a second transcription rather than a shared oracle
  **Location**: Phase 2, Section 5
  The AC names five sources; the plan lifts classify and decide only, with no recorded narrowing for the baseline and project sections or the four dedicated suites. And unlike the label golden, the two tables that carry the state machine live only in `cli/work/tests/fixtures/`, read only by Rust — so bash and Rust can drift green, which is exactly the property "both implementations held to one oracle" depends on.

- 🟡 **Test Coverage**: The classification-stability oracle asserts digests, not classification
  **Location**: Phase 3, Section 6
  AC 29 asks that every bash-baselined item classify `synced` with neither push nor pull. Digest equality is necessary but not sufficient — a wrong branch order, a `==` where `proves_unchanged_since` belongs, or a read-back bug leaves digests identical and still mass-reclassifies. The corpus's bash provenance is also guaranteed only by a manual step, so a corpus accidentally regenerated from the Rust recipe would agree with itself.

- 🟡 **Test Coverage + Correctness**: No tests are specified for the planner and run modules, or for exit code 4 and the report shape
  **Location**: Phase 4, Sections 1–2, 5 and 7
  The riskiest error path in the story — a bulk read failure must never be mistaken for absence — has no identified test; the only named coverage is the slowest level. The non-interactivity case says the command "neither blocks nor fails", which is ambiguous against an intended exit 4, and the report shape is pinned only incidentally by the two-invocation test parsing it.

- 🟡 **Code Quality + Architecture + Compatibility**: `TrackerRegistry::resolve` returning `&dyn` cannot express 0171's clients
  **Location**: Phase 4, Section 3
  A borrowed trait object requires the registry to own a constructed client for every provider it can resolve, so resolving `linear` would construct and authenticate a Jira client too. The port's own docs describe the composition as `Box<dyn RemoteTracker>`. The seam changes signature exactly when it is meant to be stable.

- 🟡 **Code Quality**: `Digests` is infallible, so an unreadable file becomes an invisible misclassification
  **Location**: Phase 2, Section 2
  `local()` reads the file on first call; a read failure has nowhere to go but a panic or a fabricated hash, which silently classifies the item locally-changed and can push stale content over a good remote with no log line. `remote_body`'s `Option` likewise conflates "not fetched" with "could not produce".

- 🟡 **Code Quality + Correctness**: `BaselineStore` has no read seam, and the spy cannot support the crash-and-resume test
  **Location**: Phase 3, Sections 3 and 5
  The store is specified with only `path` and `writer`, yet described as read-modify-write — so reads bypass the injected seam and go to the real filesystem. The spy records `verb:path` strings and persists no bytes, but the resumability test needs the second run to read back what the first wrote. The highest-value test in the phase cannot be written against the described collaborators.

- 🟡 **Code Quality + Architecture**: `FaultPoint` and `ApplyError::Interrupted` put a test concern on the shipped surface
  **Location**: Phase 3, Section 4
  A public field whose only purpose is a test crash, plus an error variant production can never produce but every `match` must handle, and a flag argument every real call site must pass. The bash seam it replaces was doubly gated; this has no gate. A non-`None` value reaching production would mutate every remote and abandon the baseline write, looking like a legitimate `Interrupted`.

- 🟡 **Code Quality**: Four error types and `RunContext` are named but never specified, with no exit-code mapping
  **Location**: Phases 3–5
  `ApplyError`, `PlanError`, `RunError` and `RunContext` appear in every signature with no variants, fields or `From<TrackerError>` conversions, and the exit-code table is prose-only with codes distributed across hand-rolled match arms. The taxonomy that determines whether a 3am failure names the item, the operation and the provider status is the least-specified part of the plan.

- 🟡 **Usability**: `create --push`'s stdout and exit contract is unspecified and collides with `create`'s path-on-stdout
  **Location**: Phase 5, Section 2
  The outcome row must be reported but has no specified destination, while `create`'s stdout *is* the written path and `create-work-item/SKILL.md` reads it as such. Retryable maps to exit 0 here and exit 70 on `update --push`, so the two halves of one feature treat identical failures oppositely and a caller cannot tell pushed from saved-unsynced by exit code.

- 🟡 **Usability**: The 72/73 messages are the whole first-run experience, yet unspecified — and 73 conflates two different fixes
  **Location**: Phase 4, Section 3
  Until 0171, these are the only outcomes. The plan specifies codes and variants but no message text, and 73 covers both unset and unknown `work.integration`. The bash path has an exemplary what/why/fix block for the unset case; regressing to a bare code, and to a 72 that reads as "broken tool" rather than "not built yet", is a real loss.

- 🟡 **Usability + Code Quality**: An unrecognised `--resolve` token skips silently
  **Location**: Phase 4, Section 5
  `--resolve 0194=remotee` produces stdout, stderr and an exit code identical to passing no order at all, so a SKILL looping until exit 0 can re-invoke forever with the same typo. The stale-*id* case does get a warning; the bad-*token* case does not.

- 🟡 **Compatibility**: The corpus cannot detect jq-versus-`serde_json` rendering divergence
  **Location**: Phase 3, Section 6
  Key order is safe (both sort by byte order), but number rendering (`serde_json` round-trips through `f64`; jq 1.7 preserves the literal) and escaping (jq emits ``, `serde_json` raw) are not. No planned case carries a fractional number, an exponent, a large integer or a control character in the ADF. The existing `work-item-project-remote/case-*` fixtures are also asserted against no Rust test.

- 🟡 **Compatibility**: Two bash-side environment constraints are unbudgeted
  **Location**: Phase 2, Section 6; Phase 3, Section 6
  No bash suite under `skills/work/scripts/` reads a `.golden` today, so the label loop is new machinery landing on a suite that tracks failures in shell variables — a `cat | while read` pipeline runs in a subshell and silently discards every increment, and `mapfile` is bash 4 and banned. The new generator is a tracked entrypoint `.sh`, so the executable-bit invariant, ShellCheck, shfmt and the bashisms linter all apply.

- 🟡 **Standards**: A Rust binary's exit codes do not belong in `EXIT_CODES.md` as scoped
  **Location**: Phase 4, Section 5
  That file's preamble scopes it to `skills/work/scripts/` helpers whose codes are `readonly E_*=NN` constants, with the document derived. Adding a binary's codes contradicts both the scope and the source-of-truth rule. Either amend the preamble in the same change (naming `dispatch_codes.rs` as a second authority) or document on the binary's own surface.

#### Minor

- 🔵 **Correctness**: Three factual slips in the cited oracle shapes — the decide table is 7 states × 3 modes with `dirty` sub-splitting only `remotely-modified` (not "8-row" / "8×3"); the bash harness passes `--dirty` on five rows, not two (the semantic claim still holds); and the token cases run to `:464`, not `:459`.
- 🔵 **Correctness**: `Subject.mtime: Option<u64>` requires the caller to have already stat'ed the file — the very syscall the plan's justification says the lazy port skips. Either move `mtime` behind the port or drop the stat half of the claim.
- 🔵 **Correctness**: "Execute each non-`Noop` action" admits executing a `Prompt`. Four of six variants are non-`Noop` and non-executable; restate as an exhaustive match naming which feed `unresolved` versus `reported`.
- 🔵 **Correctness**: Plan-then-apply does *not* make AC 8 hold by construction — preview and the real run are separate invocations re-deriving from freshly stat'ed mtimes and a fresh `fetch_all`. Keep the shared derivation as design, describe AC 8 as an asserted set comparison.
- 🔵 **Correctness**: The fake has no `failing_create`, no created-then-terminal variant, and `Call::Create` omits the body — so ACs 11 and 12 are unbuildable against it as specified.
- 🔵 **Correctness + Test Coverage**: The run-start-before-mtime assertion is vacuous under an arbitrarily early injected clock, and at one-second granularity `mtime == timestamp` is skipped by the inclusive pre-filter. Add an equal-second boundary row.
- 🔵 **Correctness**: The bulk-then-`show` gate uses `proves_unchanged_since` where bash uses raw string inequality — a real divergence for null-stamped issues. Record it, and state AC 18's call counts in terms of the new rule.
- 🔵 **Architecture**: The `tracker` allowance is crate-wide on `work` and cannot later be narrowed to `work::sync`; note the breadth in the rule's comment.
- 🔵 **Architecture + Usability**: `Action` gets no keyword `Display`/parse pair, so the wire vocabulary is hand-rolled in `work-cli` — including the `Prompt` → `unresolved` rename, giving one concept three names.
- 🔵 **Architecture + Safety**: The pending-push marker is a second durable store with no lifecycle owner — nothing reaps, lists or reports markers, and a retained one permanently blocks that title. It carries no timestamp or failure detail, so a stale marker cannot be triaged, and it sits under a VCS-tracked path so it can be committed and propagated.
- 🔵 **Code Quality**: `label` uses `kernel::Error` for states the type could exclude, and `classify_external_id` returns a 7-variant enum for a 2-variant answer.
- 🔵 **Code Quality**: Naming collisions and mode proliferation — struct `Apply` beside `RunMode::Apply`, and three "mode" types (`Mode`, `ReadMode`, `RunMode`).
- 🔵 **Code Quality**: Ids stay stringly-typed (`Option<&str>`, `BTreeMap<String, _>`) despite `tracker::ExternalId` now being importable, and absence has two spellings (`None` and `Some("")`).
- 🔵 **Code Quality + Safety**: Degrade-to-empty on an unparseable baseline is silent, and the first `set` overwrites the evidence — one parse failure becomes a corpus-wide reclassification with nothing pointing at the cause.
- 🔵 **Test Coverage**: Replacing the inline label assertions drops two live arms — the default composed mode (`work-item-sync-label.sh <id>`, the arm `/list-work-items` actually invokes) and `--label bogus` → exit 1.
- 🔵 **Test Coverage**: The push-decide characterization leaves `--write-failed` with a non-zero code uncovered, so a port that tests `write_failed` before the code still passes every row.
- 🔵 **Test Coverage**: `--exclude tracker-test-support` plus `-E 'binary(contract)'` means the crate's own `#[cfg(test)]` tests run in neither invocation.
- 🔵 **Usability**: Exit codes are documented only in a bash-scripts file, not in `--help`; 4 (resolution needed, not an error) is the least discoverable part of the contract.
- 🔵 **Usability**: Report lines have variable arity (2 fields, 3 for `unresolved`), against the fixed-arity precedent in `migrate-cli --list`.
- 🔵 **Usability**: Flag names mislead — `--per-item` does not say per item what, `--preview`'s help implies no remote calls (planning still fetches), and `--resolve`'s help omits `skip`.
- 🔵 **Standards**: The golden's exit-1 rows use a trailing empty field where every sibling golden uses an explicit `(empty)` sentinel — invisible in review and lost to any trailing-whitespace trim.
- 🔵 **Standards**: `generate.sh` + `README.md` departs from convention: regenerators are named `regenerate.sh` with the provenance in a header comment block, and no `README.md` exists anywhere under any `test-fixtures/` tree.
- 🔵 **Standards**: The corpus directory is not named for its generating script and its cases lack the `case-` prefix; an extensionless `integration` file mixes conventions.
- 🔵 **Standards**: `Digests` is the only plural-named port in the workspace (`Clock`, `IdScanner`, `AtomicWrite`, `RemoteTracker` are all singular roles).
- 🔵 **Standards**: Phases 2 and 3 touch shell but list no `mise run scripts:check` — the one component with no autofixer.
- 🔵 **Standards**: The `tasks/test/cli.py` comment and the corpus README name work item 0171, against the explicit rule that comments must not carry work-item references.
- 🔵 **Standards**: `--per-item` is missing from the Desired End State usage string, so the frozen `cli_surface.golden` diff is not fully pre-declared.
- 🔵 **Compatibility**: The label golden under-specifies the input space — bash strips a *combined* `[[:space:]"']` class from both ends, so `'PROJ-1'` and mixed quote/whitespace runs need rows; a "strip quotes then trim" port would pass as written.
- 🔵 **Compatibility**: Widening `work`'s rule puts `tracker`'s types into `work`'s *pinned* surface, coupling two previously independent pins — `tracker`'s next additive change reddens `work`'s snapshot with no edit in `work`.
- 🔵 **Compatibility**: Adding `"tracker"` to `_ALL_EXTRA_CRATES` materialises a synthetic crate the shipped `tracker_domain_imports_only_permitted` rule also matches; and the hyphen-to-underscore matcher for `tracker-test-support` is unverified precedent — the compliant control must import `tracker` or a silently-unmatched rule passes.

#### Suggestions

- 🔵 **Code Quality**: The config-composition preamble is copied a sixth and seventh time; extract `composed_context()` as Phase 4's first step.
- 🔵 **Code Quality**: `RunOutcome.unresolved` duplicates a filterable subset of `reported` — two fields that must agree can disagree; expose an iterator instead.
- 🔵 **Test Coverage**: Only Phase 1 names a red step; reorder Phases 2–5 to put the table/golden before the module it drives, and name each lifted row's derivation (bash line, or live invocation).
- 🔵 **Safety**: `--resolve` orders carry no staleness check against the state the human saw — let the order optionally carry the reported state and refuse on disagreement.
- 🔵 **Usability**: `sync` is whole-corpus only; consider `--only <id>` so a single item can be re-checked without a full `fetch_all`.
- 🔵 **Architecture**: Express `FaultPoint` as an injected post-side-effect hook, or `#[doc(hidden)]` the field.
- 🔵 **Standards**: Refresh three drifted line anchors (`sha2` is `cli/Cargo.toml:63`; the pinned/exempt split is `:15-26` and `:38-43`; `vcs-test-support`'s exemption is `:62-65`), and state that `"tracker-test-support"` goes after `"tracker"` with a trailing comma added.
- 🔵 **Standards**: Several success-criteria bullets and two Rust signatures exceed 80 columns; pre-wrap the signatures in rustfmt's one-parameter-per-line form.

### Strengths

- ✅ Six non-obvious decisions are taken up front with their costs stated — the pup widening (with the named alternative costed against it), the lossy `NotReported`/`NotRead` round trip, exit code 4 verified against the codes `accelerator work` has actually taken, plan-then-apply, exclusion-by-crate, and the accepted default-bodied-trait-method gap.
- ✅ Insisting on `proves_unchanged_since` over `==`, with a `(NotReported, NotReported)` table row that fails under `==`, closes the exact trap the port documents — encoding the invariant in a test rather than a comment.
- ✅ Dropping bash's `9999999999` mtime sentinel for `Option<u64>` is right, and the plan explains why carrying it would wrongly short-circuit above that value.
- ✅ The lazy `Digests` port is a genuine architectural seam, not a performance note: it preserves both bash short-circuits and makes them directly assertable via `expect_local_digest_called`, which the bash suite can only infer.
- ✅ Apply ordering is asserted on a recorded `verb:path` slice rather than inferred, and the fault seam is an injected value returning an error rather than an env read plus `exit 99` — strictly better than the original, since post-crash state becomes observable.
- ✅ Row-count assertions on every lifted table and golden section guard against a silently-emptied table passing green.
- ✅ `--preview` is defined by what it must not do, with the poisoned-pre-filter reason stated, and verified on three independent observables including file mtimes.
- ✅ `pull` deriving both hashes from post-overwrite state matches the bash oracle, and the plan names the phantom-`locally-modified` self-corruption that deriving either from pre-pull content would cause.
- ✅ Capturing the bash-generated corpus while its generator still exists correctly identifies that the anti-drift oracle has a shorter life than the property it protects.
- ✅ The `E_DISPATCH_*` duplication is anchored the right way round — Rust constants asserted against bash with a count assertion, so a fifth bash code reddens the Rust side.
- ✅ The registration checklist is enumerated correctly and completely, and the `_TEST_SUPPORT` shared-constant refactor follows `tasks/public_api.py`'s own `_ADAPTER`/`_COMPOSITION_ROOT` idiom. The pup probe-table edits are exactly right, and the new crate carries a pup rule the precedent crate lacks.
- ✅ Fail-safe defaults are chosen deliberately in several places: `fetch_all` errors write nothing, contradictory `--resolve` orders are an error rather than last-wins, stale orders warn rather than apply, and provider selection fails closed.
- ✅ Shipping the registry empty makes the coexistence window architecturally inert — no Rust path can write `last-sync.json` in production before 0171.
- ✅ Rust file and test naming follows established convention throughout (`cli_<command>.rs`, `*_parity.rs`), and the JSON-table-plus-count-assertion shape has a real in-repo precedent.

### Recommended Changes

1. **Specify the digest recipe as the exact bash composition, and widen the corpus to cover it** (addresses: digest byte-equality, jq rendering divergence, classification-stability oracle)
   State `digest::local` as `fm.trim_end_matches('\n') + "\n" + body.trim_end_matches('\n') + "\n"`, name the frontmatter/body splitter and its no-frontmatter and unclosed-frontmatter behaviour, and add corpus cases for: no frontmatter, empty body, all-blank body, an all-`IGNORE_KEYS` file, and ADF carrying a fractional number, an exponent, a large integer and a control character. Extend `sync_baseline_corpus.rs` to assert AC 29's actual property — load the corpus into a real `last-sync.json`, run the pipeline against matching `RecordingTracker` records, assert every item `synced` with zero `create`/`update`. Add a `bash-parity`-gated test shelling the live scripts while they exist, so provenance is CI-enforced rather than manual.

2. **Wire the dirty input, and make it three-valued** (addresses: no `dirty` producer, the fail-safe inversion)
   Define `LocalItem`, add a `WorkingCopyStatus` seam to Phase 3 or 4 feeding `work::file_dirty::is_dirty` (fetched once per run for the jj whole-tree shape, per path for git), and replace `Option<bool>` with `Clean`/`Dirty`/`Unknown` where `Unknown` is treated as dirty. Add an end-to-end test asserting a dirty `remotely-modified` file is byte-identical after both a bidirectional and a `--pull-only` run.

3. **Make the report contract complete, pinned and typed** (addresses: silent outage, `failed` collapsing 70/71, variable arity, `Prompt`/`unresolved`, exit codes)
   Emit a line for every classified item including `indeterminate` and `remote-absent`; put the classified state in a third field on *every* line (fixed arity); add the `TrackerError` class as a fourth field on `failed`; carry `fetch_all`'s `detail` into `RunOutcome` and print it on stderr with a distinct non-zero exit. Give `Action` its own keyword `Display`/parse pair in `work::sync::decide` so one place owns the wire vocabulary. Commit a report golden. Correct the exit-code table: malformed `--resolve` is 2 (clap), add a row for per-item failures, and state precedence when a run has both failures and conflicts.

4. **Fail closed on the pending-push marker** (addresses: cross-item id reuse, torn-marker duplicate create, marker lifecycle)
   Record a fingerprint of the request (`sha256(title + body + kind)`) plus a timestamp and the failure class; reuse the stored `external_id` only on a fingerprint match, and refuse with a named error otherwise. Make `read` return `Result<Option<_>, MarkerError>` and treat unparseable content as "a previous attempt of unknown outcome" — never as absence. Write it through `AtomicWrite`. Model the two states as an enum (`Attempted` / `Created`) rather than an `Option<String>`. Decide whether the marker directory is VCS-tracked, and give markers a discovery path (surfaced in `sync`'s report or a small read command) plus a documented clearing procedure.

5. **Reinstate the bulk-overwrite bound as a fail-closed refusal** (addresses: the dropped SKILL gate)
   Refuse to execute a plan whose pull count exceeds the shared threshold unless an explicit `--max-pulls N` / `--allow-bulk-overwrite` is passed; exit with a distinct code having written nothing. This keeps the binary orders-not-questions while preserving the existing quantitative limit. Add it to Phase 4's test list.

6. **Define "clean completion" and the post-terminal state** (addresses: `finalise` masking local edits, `indeterminate` unreachable)
   Withhold `finalise` whenever any item ended `unresolved`, skipped, indeterminate, remote-absent or failed, and test that a run with one skipped conflict leaves the timestamp untouched *and* that the next run still detects the local modification. Separately, either correct Phase 5's claim to `conflict` (and its test), or introduce an explicit marker the planner reads to force `RemotePresence::Indeterminate` — do not rely on a state the classifier cannot reach from local data.

7. **Fix the compile-blocking and green-blocking mechanics** (addresses: `Apply` lifetimes, wrong guard file, empty-string hashes, ASCII trim, `BaselineStore` read seam)
   Split `Apply`'s lifetimes (or have it own the store and hand it back) and drop the duplicated `writer`. Retarget the build-system edit at `tests/unit/tasks/test_mise.py` (`_NO_LAUNCHER_NEEDED` entry with a reason, roll-up membership, `depends = ["deps:install:python"]`), and add a unit assertion that `_MANIFEST` still carries the exclusion. State that an empty-string persisted hash reads back as `None`, with table rows for both sides. Specify `to_ascii_lowercase()` and `trim_matches(char::is_ascii_whitespace)` with a U+00A0 row proving `Skip`. Give `BaselineStore` an injected read side and make the test double a single in-memory store that both records order and retains content.

8. **Reshape the two seams 0171 must inherit** (addresses: registry borrow, harness inducibility, exclusion mechanism)
   Return `Result<Box<dyn RemoteTracker>, SelectionError>` from `resolve`, and box the fakes in tests so the test shape matches production. Give the contract harness an induction seam (a `ContractSubject` supplying a definitely-unknown id and a way to force a partial fetch) and separate the conformance set from shape-specific cases. Verify nextest's `default-filter` at the pinned version and prefer `not binary(contract)` in a `cli/nextest.toml`; either way have the harness fail closed behind an explicit opt-in env var, and drop the `-E` filter so the crate's own lib tests run somewhere.

9. **Specify the shared `last-sync.json` contract and test it bidirectionally** (addresses: tolerance granularity, wire format, RMW granularity)
   State per-entry tolerance (unknown fields ignored, a bad field empty, only that entry discarded — never the document), pin `render()` to `jq -c`'s shape, and add a parity test writing with bash and reading with Rust and vice versa. State that each mutation re-reads immediately before rendering (matching bash), or take the existing mkdir-lock contract around the run. Make degradation observable — return it alongside the value, warn on stderr naming the path, and preserve the unparseable content aside before the first write.

10. **Specify the error taxonomy, the messages and the missing clock** (addresses: unspecified errors, 72/73 message quality, no epoch port)
    Enumerate `ApplyError`/`PlanError`/`RunError`/`RunContext` with their `From<TrackerError>`/`From<StoreError>` conversions, each variant carrying the item id and the operation, and define one `exit_code()` function the tests assert against. Pin the 72/73 message strings in the plan and assert them (name the config key, the recognised set and `/accelerator:configure` for unset; echo the offending value for unknown; say "recognised but not built yet" for 72), and consider distinguishing unset from unknown at the variant level. Decide the epoch seam explicitly — widen `corpus::Clock` (listing the snapshot and implementor fan-out) or declare a narrow sync-owned port — and specify that a derivation failure yields no advance rather than a fallback value.

11. **Close the remaining oracle gaps and record the narrowings** (addresses: lift scope, second transcription, dropped label arms, push-decide gap)
    Move the classify, decide and push-decide tables into `skills/work/scripts/test-fixtures/` in the established `|`-delimited golden shape and loop the bash suites over them (`while IFS='|' read -r … done < "$GOLDEN"` — a redirect, never a pipeline, or the failure counter is lost to a subshell). Add a `[DEFAULT]` section and a `bogus|1|(empty)` row to the label golden so the bash loop is a strict superset of what it replaces, plus `[CLASSIFY]` rows for `'PROJ-1'`, `  "PROJ-1"  `, `"  "` and `'"`. Add `--code 70 --attempt 1 --write-failed` → `retry` and `--code 71 … --write-failed` → `loud-terminal`. Record explicitly which AC-named sources are not being lifted and why. Add `cli/work-adapters/tests/sync_plan.rs` covering the `fetch_all`-error branch, the partition mapping, and each resolution override.

12. **Fold in the conventions and corrections** (addresses: the minor and suggestion sets)
    Rename to `regenerate.sh` with the provenance in its header comment (no `README.md` under `test-fixtures/`, no work-item numbers in comments), name the corpus directory for its generating script with `case-` prefixes, use the `(empty)` sentinel, rename `Digests` to a singular role noun before the snapshot is first generated, add `mise run scripts:check` to Phases 2 and 3, add `[--per-item]` to the Desired End State, correct the three drifted line anchors and the decide-table shape (7 states × 3 modes), extract `composed_context()`, and restate AC 8 as an asserted set comparison rather than a structural guarantee.

## Per-Lens Results

### Architecture

**Summary**: Unusually strong on boundary reasoning — the `work` → `tracker` widening is argued against a named alternative, the domain/adapter split follows the settled pattern, plan-then-apply makes AC 8 structural, and the lazy `Digests` port is a genuine seam. Two structural gaps undercut it: the orchestration layer is sited in `work-adapters` without the siting argument the work item demanded, putting real domain policy in the one crate exempt from both the pup rule and the public-API pin and untestable as a pure function; and two declared pipeline inputs (the run-start epoch, the per-item `dirty` flag) have no provider anywhere, with the epoch requiring a widening of a shared pinned domain port. On resilience the contract cannot distinguish a healthy no-op run from a wholly unreachable tracker, and defines no exit code for partial per-item failure.

**Findings**: 5 major (orchestration siting; no epoch clock port; `dirty` has no producer and `LocalItem` undefined; crate-level exclusion does not compose and nextest's `default-filter` was dismissed on a false premise; unreachable tracker degrades to silent success plus no code for per-item failure), 4 minor (report grammar pinned by nothing and `Prompt`/`unresolved` split across two places; `resolve` returning a borrow forces eager construction; `&'a mut BaselineStore<'a>`; the marker as an unowned second store), 2 suggestions (crate-wide `tracker` allowance cannot be narrowed; `FaultPoint` as a public field).

### Correctness

**Summary**: Rigorous about the traps that matter — `proves_unchanged_since` over `==`, the dropped mtime sentinel, `pull`'s post-overwrite hashes, the two short-circuits through a lazy port — and nearly every bash line citation checks out. Remaining risk is concentrated in three stated-as-settled properties that are not pinned: the local digest recipe is not byte-equal when either half normalises to empty, the persisted empty-string hashes are not mapped back to bash's "absent" semantics, and "clean completion" is undefined even though a conflict-reporting run can mask a local edit next run. A second cluster is structural: `plan()`'s signature can neither supply `decide`'s `dirty` input nor carry the fetched body into apply, and the marker degrades open.

**Findings**: 5 major (digest composition; `finalise` clean-completion; `plan()` cannot supply `dirty` or carry the fetched payload, forcing a second `show` against AC 18; marker fails open and the delete-after-write window; empty-string hash presence), 8 minor (`Subject.mtime` requires an eager stat; "execute each non-`Noop`" admits executing `Prompt`; AC 8 is not structural; the fake cannot express ACs 11/12; `BaselineStore` read seam plus the lifetime; the same-second mtime boundary; the bulk gate's `proves_unchanged_since`-versus-string-inequality divergence; three factual slips in the cited table shapes).

### Test Coverage

**Summary**: Unusually test-literate — it names its oracles, pins row counts, asserts call order on a slice, injects the fault seam, and adds four classify rows that would kill real mutants. Weaknesses concentrate in three places: the lifted-table strategy is narrower than the AC and is a second transcription rather than a shared oracle; the classification-stability oracle stops at digest equality; and the test-infrastructure plumbing is partly wrong — the named build-system test file contains no such assertions and the actual guard is never touched, so Phase 1 cannot be green as written. The contract harness also cannot exercise its two headline properties against anything but a specially-shaped fake.

**Findings**: 9 major (lift scope narrower than the AC with no recorded deviation; classify/decide as a second transcription; stability oracle asserts digests not behaviour; wrong build-system guard file plus unguarded exclusion; harness properties unrunnable against real implementations; no tests for planner/run error paths; exit code 4 and the report shape unasserted; the label golden drops two live bash arms; corpus provenance manual-only), 3 minor (the vacuous timestamp assertion; `--write-failed` with a non-zero code uncovered; the crate's own tests run nowhere), 1 suggestion (only Phase 1 names a red step).

### Code Quality

**Summary**: Rigorous for a port-and-parity story — the pure state machine is cleanly separated from I/O, ports are injected, plan-then-apply makes preview fidelity structural. Weak spots are all in the proposed Rust API shapes: an invariant-lifetime `&'a mut BaselineStore<'a>` that will not compile usably, an infallible `Digests` port that forces I/O errors to be swallowed, a test-only `FaultPoint`/`Interrupted` pair on the shipped surface, and four error types named but never specified despite carrying every diagnostic the engine emits at 3am. Domain expressiveness also slips: stringly-typed ids despite `ExternalId` now being importable, `kernel::Error` for states the type could exclude, and pure decision tables split across crates for snapshot-churn reasons rather than domain ones.

**Findings**: 1 critical (`Apply` lifetimes), 10 major (infallible `Digests`; `FaultPoint` on the shipped surface; `BaselineStore` read seam and the recording-only spy; no epoch accessor on `corpus::Clock`; four unspecified error types and no exit-code mapping; `fetch_all`'s `detail` discarded; `PendingPush::read` fails open; pure policy sited outside the domain; stringly-typed ids and two spellings of absence; silent degrade-to-empty; `resolve` returning a borrow), 3 minor (`label`/`classify_external_id` signatures; naming collisions and mode proliferation; silent bad token), 2 suggestions (the copied composition preamble; `RunOutcome.unresolved` duplication).

### Safety

**Summary**: Strong on write-ordering — side-effect-first/baseline-last is pinned by an order-asserting spy, the fault seam is injected and returns an error, `--preview` withholds `finalise` for a stated reason, degrade-to-empty is preserved, and the bash corpus is captured as the mass-reclassification oracle. Two safeguards present in the live bash path are nevertheless dropped or unspecified: the local-dirty input is never wired (and `Option<bool>` inverts the existing fail-safe-to-dirty convention), and the SKILL's aggregate pull-overwrite gate has no equivalent, so nothing bounds how many local files one run can overwrite. The marker's slug identity also admits a reuse path binding one remote issue to a different work item — the one class of damage neither VCS revert nor a re-run can undo.

**Findings**: 3 critical (`dirty` never wired and `None` means clean; the dropped aggregate pull-overwrite gate; slug-keyed marker reuse), 6 major (undefined "clean completion" masking skipped items' local edits; no epoch seam and the corpus-wide consequence of a too-large epoch; unspecified RMW granularity widening the lost-update window to a whole run; the terminal clear does not produce `indeterminate`; crate exclusion as the only barrier to live mutations; a crash during `create` indistinguishable from a terminal failure with nothing to triage it), 2 minor (`FaultPoint` as an ungated public field; degrading then rewriting destroys the evidence silently), 1 suggestion (`--resolve` carries no staleness check).

### Compatibility

**Summary**: Careful about the contracts that matter — the lossy round trip is named as a decision, `proves_unchanged_since` is proven by a failing row, the `E_DISPATCH_*` constants are anchored to bash with a count assertion, and exit 4 is picked against the codes actually taken. Risk concentrates where bytes must agree across two live implementations: the digest recipes (the stated composition diverges from bash's `printf '%s\n%s\n'` whenever either side is empty, and no splitter is named) and the `last-sync.json` wire format (per-entry tolerance, key order, compactness and a bash↔Rust round trip all unspecified and untested). Secondary: the report is declared frozen but pinned by nothing and renames `prompt`; a Unicode-aware trim turns a safe skip into a destructive pull; and `TrackerRegistry` cannot express lazily-constructed clients.

**Findings**: 1 critical (digest composition and the unnamed splitter), 7 major (no cross-engine baseline contract or wire format; Unicode trim in `resolve_conflict_token`; the unpinned report plus the `prompt`/`unresolved` rename and the collapsed failure class; the terminal clear cannot yield `indeterminate`; empty-string hash presence; jq-versus-`serde_json` number and escape rendering plus no `project_remote` parity test; `resolve` returning a borrow), 4 minor (the label golden's under-specified input space; the two bash environment constraints — subshell-swallowed loop and executable bit; `tracker` types entering `work`'s pinned surface; the pup probe generator's synthetic-crate coupling and the unverified hyphen matcher).

### Usability

**Summary**: Strong on the two DX properties that matter most: `sync` is genuinely non-interactive (closed-stdin test, orders-not-questions help check), and plan-then-apply makes `--preview` fidelity structural. Weak spots are all at the process edge the SKILL parses — the report can only express non-`Noop` decisions, so a total `fetch_all` outage produces empty stdout and exit 0; per-item failures collapse the retryable/terminal split; and `create --push`'s stdout and exit contract is left unspecified while `update --push`'s is spelled out. Error-message *content* for 72/73 — the only outcomes anyone will see for this story's lifetime — is never specified beyond "a named error".

**Findings**: 2 critical (report cannot express indeterminate/remote-absent so an outage looks clean; slug-keyed marker can silently attach a second item to another's remote issue), 5 major (`create --push` stdout/exit unspecified and colliding with the path contract, plus create-0/update-70 asymmetry; `failed` drops the retryable/terminal split and no exit code covers a failed run; 72/73 message content unspecified and 73 conflating two fixes; unrecognised token skips silently; caller-input errors split across 1 and 2 with the documented 1 unreachable), 4 minor (exit codes documented only in a bash file, not `--help`; variable line arity; three misleading flag helps; three names for one outcome), 1 suggestion (no single-item scoping).

### Standards

**Summary**: Unusually convention-literate: it enumerates all five obligations of the library-crate checklist correctly, mirrors `vcs-test-support` and goes further by adding a pup rule that crate lacks, lifts the shared-reason-constant style into `_TEST_SUPPORT`, and gets both pup probe-table edits exactly right. Gaps concentrate in the build-system registration surface (it names the wrong guard file, so `test:unit:tasks` fails until `test_mise.py` is updated), the exit-code contract (the `--resolve` parse-failure code contradicts the chosen `value_parser`), and fixture-tree conventions (empty-field sentinel, generator naming, an unprecedented README under `test-fixtures/`). None is structural; all are mechanical.

**Findings**: 3 major (wrong build-system guard file; the exit-1/exit-2 contradiction; `EXIT_CODES.md` scope and source-of-truth rule), 7 minor (the `(empty)` sentinel; `regenerate.sh` naming and the unprecedented README; corpus directory and case naming; `Digests` as the only plural port name; no `scripts:check` in Phases 2–3; work-item numbers in comments against the explicit rule; `--per-item` missing from the declared surface), 2 suggestions (three drifted line anchors plus `members` placement and the trailing comma; over-80-column bullets and signatures).

## Plan Revision — 2026-08-13

All twelve recommended changes were applied to the plan (now 2646 lines, up from
1608). Three decisions were taken by the author where the finding admitted more than
one resolution:

- **Pull-overwrite gate** — reinstated as a fail-closed refusal (`--max-pulls`,
  default 25, exit 5, zero writes) rather than deferred to 0171. The interactive half
  follows the conversational flow into 0171.
- **Planner siting** — split into a pure `work::sync::plan` plus a thin
  `work_adapters::sync::fetch` shell, accepting the public-api churn, rather than
  recording the `work-adapters` siting with its costs.
- **Post-terminal state** — `conflict` accepted and the wording corrected, rather than
  forcing `indeterminate` via a new marker or widening the baseline schema.

Three cited facts were verified against source before editing, and all three confirmed
the finding: `tests/unit/tasks/test_mise.py` carries both registration guards (`:146`,
`:180`); `corpus::Clock` has no epoch accessor (`cli/corpus/src/metadata.rs:15-18`);
and `EXIT_CODES.md`'s preamble scopes it to `readonly E_*=NN` script constants. A
fourth was tested empirically and **contradicted the plan's premise**: nextest 0.9.138
does accept a profile `default-filter`, so the stated reason for rejecting a filter
("there is no `nextest.toml`") was a non-reason. The mechanism was changed from
`--exclude tracker-test-support` to `default-filter = 'not binary(contract)'` in a new
`cli/.config/nextest.toml`, which additionally composes to 0171's client crates and
stops silencing `tracker-test-support`'s own lib tests.

Structural changes beyond the findings' literal asks:

- `Implementation Approach` grew from six decisions to twelve, absorbing the epoch
  seam, the overwrite bound, three-valued dirtiness, ASCII-only token handling, the
  empty-string-hash rule, the report contract, the failure class and the
  post-terminal state.
- `ItemDigests` (renamed from `Digests`, now fallible and carrying `mtime`),
  `Dirtiness`, `SyncDirection`, `RetrievalStrategy`, `RenderableState`,
  `SyncPresence`, `RunClock`, `ContractSubject`, `Degradation` and `RequestFingerprint`
  are new or reshaped types; `Apply` became `ItemApplier<'ctx, 'store>` with
  independent lifetimes; `FaultPoint` and `ApplyError::Interrupted` were removed in
  favour of inducing the crash through a spy writer.
- Four new test files were added (`sync_plan.rs`, `sync_fetch.rs`,
  `sync_baseline_shellout_parity.rs`, `project_remote_parity.rs`), plus a report
  golden and five new corpus cases covering the empty and numeric/control-character
  shapes where the recipes actually diverge.
- The classify/decide/push-decide tables moved from `cli/work/tests/fixtures/` to
  `skills/work/scripts/test-fixtures/` so both implementations share one oracle.

Not addressed, deliberately: item selection for `sync` (`--only <id>`) was recorded as
an explicit non-goal rather than implemented — it is new surface rather than parity,
and the work item's command specification does not carry it.

⚠️ Unverified: none of the proposed Rust signatures has been compiled, and the
`tracker_test_support_imports_only_permitted` hyphen-to-underscore matcher is flagged
in the plan as something to check rather than a confirmed behaviour.

## Re-Review (Pass 2) — 2026-08-13

**Verdict:** REVISE

All eight lenses re-ran against the revised plan. **Nothing regressed to "still present"** — every one of the 32 aggregated prior findings is resolved (~75 of 102 per-lens instances) or partially resolved (~27), and the partials are mostly scope gaps rather than failed fixes. But the pass surfaced **2 critical and ~24 major new findings, the large majority of them defects in the revision itself** rather than pre-existing issues the first pass missed. Two of the twelve recommended changes were implemented in ways that do not work, and one introduced a regression the plan's own text warns against.

### Previously Identified Issues

Per-lens resolution of the prior pass:

| Lens | Resolved | Partial | Still present |
|---|---|---|---|
| Architecture | 7 | 4 | 0 |
| Correctness | 8 | 5 | 0 |
| Test Coverage | 10 | 3 | 0 |
| Code Quality | 12 | 5 | 0 |
| Safety | 9 | 3 | 0 |
| Compatibility | 10 | 2 | 0 |
| Usability | 9 | 3 | 0 |
| Standards | 10 | 2 | 0 |

All six prior criticals are resolved: the digest recipe is now specified (though wrongly justified — see below), `dirty` is wired as three-valued `Dirtiness`, the report names every classified item, the marker is fingerprinted, the pull gate is reinstated as `--max-pulls`, and `ItemApplier` takes independent lifetimes.

Notable partials: the AC-19 lift scope still omits the four dedicated bash suites with no recorded deviation; `tracker-test-support` has no named lib test of its own; `ItemDigests` is still plural where every other port in the workspace is singular; and `RunError`/`MarkerError` remain unspecified.

### New Issues Introduced

#### Critical — both in the digest recipe, both verified against source

- 🔴 **Compatibility + Correctness**: The no-frontmatter parity claim is inverted.
  I claimed bash "emits a leading `\n` that concatenation omits". It does not — `config_extract_frontmatter` (`scripts/config-common.sh:76-85`) hits `NR == 1 && !/^---[[:space:]]*$/ { exit }` → `END { if (!closed) exit 1 }`, and under `set -euo pipefail` the pipeline assignment aborts the script. Verified empirically: `bash work-item-normalise.sh nofm.md` produces no output and **exit 1**. So `digest::local` must return `Err` for a file that does not open with a fence (not only for unclosed frontmatter), the Key discoveries bullet is wrong, and the corpus case I specified cannot have an `expected.json` — `regenerate.sh` would have nothing to record and the `bash-parity` shellout no expectation to assert.

- 🔴 **Compatibility + Correctness**: The named splitter does not implement the delimiter attributed to it.
  The plan specifies "a `---[[:space:]]*$` closing delimiter", citing `document`'s split. `cli/document/src/fence.rs:41` is `if &raw[..first_line_end] != b"---"` and `:70` is `== b"---"` — exact bytes modulo CRLF. A fence written `--- ` is frontmatter-plus-body to bash and all-body to `document`, so the `IGNORE_KEYS` filter never runs, `last_updated`/`revision` restamps become content changes, and `local_hash` diverges silently. `document` also caps its scan at 1 MiB where awk has none.

#### Major — regressions introduced by the revision

- 🟡 **Code Quality + Architecture + Correctness + Safety** (4 lenses): `RunContext<'a>` reintroduces `&'a mut BaselineStore<'a>` — the exact invariant-lifetime trap Phase 3 §4 now carries a warning against, and worse, `'a` is unified across all ten fields so it will be inferred as the composition-root lifetime.
- 🟡 **Test Coverage + Compatibility + Correctness** (3 lenses): the shared classify table cannot drive bash. Rows carry fabricated digest strings (`lh-xyz`), an injected `mtime` and `expect_*_called` flags; `work-item-sync-classify.sh` computes both hashes itself from real content via `work-item-normalise.sh | hash_sha256_stdin` and has no seam for injected digests or call observation. `Dirtiness::Unknown` has no bash spelling. The row-count assertion counts rows in the file, not rows exercised, so a bash loop that skips them still passes — the two-transcriptions failure the shared table was introduced to prevent.
- 🟡 **Test Coverage + Compatibility + Architecture** (3 lenses): the contract task will not build. `--workspace --all-features` without `--exclude accelerator-visualiser` triggers `cli/visualiser/server/build.rs`'s assertion that `frontend/dist/index.html` exists under `embed-dist`; the profile filter suppresses the visualiser's tests, not its build. The mise task also lacks the `build:frontend:stub` edge every workspace-wide Rust task carries — so `test:integration:tracker-contract`, the roll-up, and the `mise run` done-gate all fail on a clean checkout.
- 🟡 **Correctness**: the marker's delete-after-write window binds two locals to one remote issue. A crash between step 4's file write and the marker delete leaves a `Created` marker whose fingerprint still **matches**, so the re-run passes the guard, reuses the id, allocates a *new* work-item number and writes a *second* file carrying the same `external_id`. The fingerprint guard does not cover this; the fix is to check the local corpus for an item already carrying that id.
- 🟡 **Correctness**: "clean completion" overshoots into unreachability. `remote-absent` and `indeterminate` are *sticky* — nothing a later sync does clears them — so one deleted remote issue freezes the global timestamp permanently and disables the mtime pre-filter corpus-wide, the very optimisation the lazy `ItemDigests` port, its fixture rows and the Performance section exist to preserve.
- 🟡 **Safety**: the bound is asymmetric. `--max-pulls` protects local files; the push direction has none, and recipe drift → stale `local_hash` → `locally-modified` → `Push` → whole-content `update` on every remote issue. `--push-only` is unbounded, and the pre-push remote content exists nowhere locally.
- 🟡 **Safety + Correctness + Compatibility**: the rename-aside is a write that precedes the zero-write refusal, is undefined under `--preview`, uses a fixed filename that clobbers earlier evidence, has no test, and is a filesystem mutation the live bash engine neither performs nor expects (it removes conflict markers from a tracked path, reading as a resolution the user never made).
- 🟡 **Usability**: exit-code precedence hides code 4. 70 beats 4, so a run with one conflict and one retryable failure exits 70 — "safe to retry" — and a status-branching SKILL loops forever without surfacing the conflict, defeating the work item's explicit requirement.
- 🟡 **Correctness**: no exit code covers skipped items. Row 0 is "clean; nothing unresolved, skipped or failed" but 4 is scoped to the `Prompt` subset, so `skip-dirty` maps nowhere and `exit_code(&RunOutcome)` is unimplementable as specified.
- 🟡 **Usability**: a `fetch_all` failure maps to exit 1, where `EXIT_CODES.md` already defines a read-bridge failure as 70/safe-to-retry — the taxonomy is reused everywhere else and abandoned for the most common transient failure.
- 🟡 **Correctness + Architecture + Standards** (3 lenses): `push_decide` is sited in two crates (table says `work_cli`, Phase 5 says `work::sync`), and the domain siting is unreachable — the table's input is a numeric dispatcher code whose constants live in `work-cli/src/exit_codes.rs`, which `work_domain_imports_only_permitted` forbids `work` from importing.
- 🟡 **Architecture + Safety + Usability** (3 lenses): Phase 5 claims `sync` lists outstanding markers; Phase 4 froze stdout as fixed-arity lines carrying "nothing else", byte-compared against a golden, with no Phase 5 edit to `sync.rs` or the golden. The marker's gitignore is likewise asserted but appears in no phase's changes — and nothing currently ignores `meta/integrations/`, so under jj's auto-snapshot a marker is committed the moment it is written.
- 🟡 **Test Coverage + Compatibility**: the report golden is byte-compared with no ordering contract. `reported` derives from corpus enumeration order; AC 9's own check is a *set* comparison. Either a darwin/linux flake or an accidentally frozen order.
- 🟡 **Usability**: nothing in the new surface can push an existing unsynced item — `unsynced` collapses to `noop` in all directions, `update --push` refuses a target with no `external_id`, `create --push` only pushes what it creates. Bash SKILL Step 4 covers this today; absent from the plan and from the non-goals.
- 🟡 **Test Coverage**: the fingerprint reuse-guard test can pass vacuously — if it varies the *title*, the two requests land on different marker paths and the assertion holds against an implementation that does not detect the hazard. It must hold the title constant and vary body or kind.
- 🟡 **Correctness**: AC references are off by 1–5 throughout (the work item has 25 criteria) and internally inconsistent — the created-then-terminal case is "AC 12" in Phase 1 and "AC 11" in Phase 5; "AC 10" names two different criteria.
- 🟡 **Test Coverage + Architecture**: the `ACCELERATOR_TRACKER_CONTRACT` gate makes the suite silently vacuous — nothing asserts the assertions ran, and the only check verifies the *skip*.

#### Minor and suggestions

Stale destination-table rows (`dispatch_codes` → `exit_codes`, `work_cli::push_decide` → `work::sync`); `binary(contract)` is a substring matcher, so `[profile.contract]` would pull in a future `contract_helpers` (use `binary(=contract)`); "fixed arity" is stated twice but the contract is 3-or-4 fields; `noop\tsynced` lines make stdout O(corpus) inside a SKILL context window; `RequestFingerprint` concatenates three fields undelimited — the same non-injectivity the plan documents for `digest::local`, in the guard whose job is proving two requests identical; the `--max-pulls` tests cannot distinguish `>` from `>=` at the boundary and `--max-pulls 0` is undefined; `--max-pulls` is not evaluated under `--preview`, so a preview promises pulls the run refuses (violating AC 9 for that plan); the exec-bit criterion asserts a guard that exempts `test-fixtures` paths; two encodings for shared tables where only classify needs JSON; the new `.config/` convention is recorded nowhere shared; the `conflict`-not-`indeterminate` decision contradicts AC 14 without flagging it for amendment; 42 `⚠️` markers where the sibling 0204 plan has none; sentence-case headings against the template's Title Case.

### Assessment

The revision moved the plan a long way — the state machine, the ports, the error taxonomy, the report contract and the parity oracles are all materially better specified, and no prior finding regressed. But it is not ready for implementation. Two of the twelve changes need redoing rather than refining: the digest recipe rests on a false premise about bash's no-frontmatter behaviour and names a splitter with the wrong fence class, and the shared classify table is not executable by the implementation it was relocated to serve. A third, `RunContext`, reintroduces a trap the same revision documents. And three mechanisms — the contract task, the marker's containment, the exit-code mapping — are specified in ways that cannot run as written.

The pattern is worth naming: the first pass found gaps in what the plan *said*; this pass found gaps in what the plan's own fixes *do*. Several were only catchable by executing the bash oracle or reading the cited Rust rather than trusting the plan's description of either — which is the same discipline the plan now demands of its own lifted fixtures.

Recommended next step: a third pass over the digest/splitter parity, the classify table's schema, `RunContext`, the contract task invocation, the marker's write-to-delete window, the finalise gate's sticky states, and the exit-code table — then re-review only those.

## Plan Revision — Pass 3 (2026-08-13)

All 2 critical and ~24 major findings from pass 2 were addressed; the plan is now 2977
lines. Both criticals were verified against source before acting, and both confirmed
the finding rather than the plan.

**The two criticals.** `digest::local` now returns `Err` for a file that does not open
with a fence and for an unclosed frontmatter, because
`bash work-item-normalise.sh` over a body-only file was run and produces no output and
**exit 1** — the previous claim that bash "emits a leading `\n`" was false. The
corresponding corpus case became an *error-parity* case (expected failure, no
`expected.json` hash), since the generator has nothing else to record. And the digest
path now carries **its own** fence recogniser matching `^---[[:space:]]*$` with no scan
cap: `cli/document/src/fence.rs:41` compares the exact bytes `b"---"` and stops at
1 MiB, so a fence with a trailing space would have made the whole file body, skipped
the `IGNORE_KEYS` filter, and turned a routine restamp into a silent
`locally-modified`. The `document` edge on `work-adapters` was removed accordingly.

**Regressions from pass 2, fixed.** `RunContext` is gone — its ten fields split into
`SyncPorts` (injected collaborators) and `SyncRequest` (run parameters, now including
`mode`), with the store passed as a **separate `&mut` argument** so the `&'a mut T<'a>`
trap cannot recur and neither struct needs `&mut` at all. The contract task now reuses
`tasks/test/cli.py`'s `_MANIFEST` (carrying `--exclude accelerator-visualiser`, without
which `--all-features` trips the visualiser's `embed-dist` build assertion) and gains
the `build:frontend:stub` and `deps:install:rust-components` edges. The shared classify
table was recast around **content** rather than digests — symbolic
`from-content`/`stale`/`absent` hashes, `mtime_offset` instead of an absolute epoch,
`applies_to` per row, `rust_only` for call-observability — because
`work-item-sync-classify.sh` computes hashes from real content and no input makes a
file's normalised sha256 equal `lh-xyz`. Each side now asserts the count of rows *it
ran*, closing the back door where a bash loop could skip every inexpressible row and
still satisfy a whole-file count.

**Newly closed.** The finalise gate became per-item — blanking the `local_hash` of
unreconciled items and then advancing — because withholding on any unreconciled item
would have let one deleted remote issue freeze the timestamp permanently, since
`remote-absent` and `indeterminate` are sticky. `--max-pushes` mirrors `--max-pulls`
(recipe drift drives `Push`, and remote pre-state exists nowhere locally), both
evaluated under `--preview`. Exit codes were reworked three ways: 4 now covers every
item awaiting a human (not just conflicts, so `skip-dirty` maps somewhere and
`exit_code` is implementable), 4 beats 70 (a status-branching caller would otherwise
loop forever on a mixed run without surfacing the conflict), and a `fetch_all`
pre-flight failure is 70 per `EXIT_CODES.md`'s read-bridge convention rather than
generic 1. The report gained a stated ascending-id ordering, genuinely fixed four-field
arity with `-` as placeholder, and a counted trailer for `synced` items instead of
O(corpus) noise. The marker gained a corpus check (closing the write-to-delete window
that the fingerprint guard could not, since the fingerprint still matches) and a
length-prefixed digest. `push_decide` takes a bare `u8`, since the pup rule forbids
`work` from importing `work-cli`'s constants and an enum could not express the `99`
row. The rename-aside was **dropped** — it mutated a tracked path mid-merge, diverged
from bash, and preceded the zero-write refusal.

**Also.** AC references renumbered against the real 25 (they were off by 1–5 and
internally inconsistent); the stale destination-table rows and `dispatch_codes_parity.rs`
corrected; `binary(=contract)` exact-match form; `run_all` returns an executed-property
count the binary asserts non-zero; the filter guard moved to its own
`test_nextest_filter.py` per the repo's artefact-guard precedent, with the new
`.config/` and `tests/contract.rs` conventions recorded in `tasks/README.md`; the
`.gitignore` entry for markers made an explicit file change; the decide and push-decide
tables moved to pipe-delimited goldens (only classify needs JSON); the vacuous exec-bit
criterion dropped and the fixture exemption stated; boundary-comparison and
degradation tests added; the AC 14 contradiction flagged for amendment; the
unsynced-push hole recorded as an explicit non-goal; headings normalised to Title Case;
and the ⚠️ markers cut from 59 to 26, keeping only traps whose failure mode is silent.

⚠️ Still unverified: no proposed Rust signature has been compiled, and the
`tracker_test_support_imports_only_permitted` hyphen matcher remains flagged in the
plan as something to check rather than a confirmed behaviour.

## Re-Review (Pass 3) — 2026-08-13

**Verdict:** REVISE

Four lenses re-ran (code-quality, correctness, test-coverage, safety) — the ones whose pass-2 findings the two closing verifications bear on. Resolution was strong: **21 of 45 prior per-lens findings resolved, 17 partial, 4 still present**, and every pass-2 critical is closed. But the pass surfaced **21 new findings (0 critical, 16 major)**, and the dominant cause is a single mechanical failure rather than a design problem.

### Previously Identified Issues

| Lens | Resolved | Partial | Still present |
|---|---|---|---|
| Code Quality | 2 | 6 | 1 |
| Correctness | 8 | 5 | 0 |
| Test Coverage | 7 | 2 | 2 |
| Safety | 4 | 2 | 1 |

Both pass-2 criticals (the digest premise, the fence class) are resolved and were re-verified against source by the correctness lens independently. `RunContext`, the contract task's build, the marker's write-to-delete window, the classify fixture's executability, the sticky-state finalise deadlock and the AC renumbering are all closed.

Still present after three passes: AC 16's four unlifted bash suites (now recorded as a deviation), `tracker-test-support`'s absent lib tests (now specified), `RunOutcome.failures`' stringly-keyed join, and `--resolve` staleness (accepted residual).

### New Issues Introduced

**The pattern: decisions taken in the Implementation Approach did not propagate into Phase 4.** Four of the five highest-severity findings are the same defect, each flagged by two or three lenses independently:

| Decision taken | Phase 4 still said |
|---|---|
| Blank unreconciled hashes, then finalise | §7: "a run that reports a conflict… does not finalise" |
| Bound both directions, both modes | §2: "under `RunMode::Apply`… `Pull` count exceeds `max_pulls`" |
| "4 beats 70, not the reverse" | table column: `70 … beats 4` |
| `fetch_all` failure is 70, not 1 | §7: "exactly 1 on a `fetch_all` pre-flight failure" |

Under TDD the stale test wins, so the first would have shipped the design the plan explicitly rejected.

**A genuine defect in the pass-3 finalise fix** (safety + correctness): blanking `local_hash` for **dirty-only** items is wrong. `skip-dirty` and the dirty-pull `Prompt` are reachable only from `remotely-modified`, whose local side classified *unchanged* — so the stored hash is accurate, not stale. Blanking it makes the next run say `conflict`, whose decision is `Prompt`, which blanks again: the item sticks at `unresolved` **forever, even after the user commits**. `Dirtiness::Unknown` decides as dirty for every item, so one failed VCS probe does this corpus-wide — and a manufactured `conflict` is resolvable, so `--resolve <id>=local` then pushes never-modified local content over a newer remote. Also unspecified: a failed blank write followed by a successful `finalise` reinstates the original hazard in full.

**Self-contradictions introduced in pass 3**: the synced-summary line `#\tsynced\t142` broke the four-field arity contract it sits inside; `unresolved()` named two different sets (the `Prompt` wire keyword and the wider exit-4 population); and `run_all`'s non-vacuity counter contradicted its own "skips when unset" manual check.

**Undefined types with load-bearing dependents**: `RunError` (three exit codes depend on it), `ApplyError`'s class accessor (the report's safety-critical fourth field), `BaselineStore`'s entire `impl` block (four call shapes), and `GatheredFacts` (whose borrowed element type could not be owned without self-borrowing).

**The `ExternalId` claim survived three passes.** Flagged by architecture in pass 2, acknowledged in discussion, never edited — and re-flagged by code-quality and correctness in pass 3.

### Assessment

The plan is materially better than at pass 1 and every load-bearing mechanism is now specified, but the process has a visible signature: **each pass fixes the previous pass's findings and introduces a smaller set of new ones, concentrated in the sections the fixes touched.** Pass 1 → 6 criticals. Pass 2 → 2 criticals, both in pass-1 fixes. Pass 3 → 0 criticals, 16 majors, mostly propagation failures from pass-2 fixes. The severity is converging; the count is not converging as fast, and the recurring failure mode is now editorial (decision recorded in one section, not carried to the two others that restate it) rather than analytical.

Two things changed the error profile materially and are worth doing earlier next time: **compiling the signatures** (which caught a third instance of a lifetime trap two review passes had missed, and forced three phantom types to be defined) and **executing the bash oracle** (which inverted a stated parity premise). Both found defects no amount of reading would have.

## Plan Revision — Pass 4 (2026-08-13)

Plan now 3188 lines. All four propagation contradictions fixed at source; the blanking rule narrowed to items whose local side classified changed, with a failed blank now suppressing the advance; `finalise_run(blank, epoch)` made one operation so the ordering cannot be half-applied; the exit-code table given one total order (**71 > 4 > 70 > 0**, with 1/2/5/72/73 terminal and exclusive) and the lossiness stated; code 4 widened to cover `remote-absent`/`indeterminate`; the summary line re-arity'd to `#\tsummary\tsynced\t142` and excluded from AC 9's set comparison; `unresolved()` renamed `awaiting_human()`; `run_all` returning `ContractRun::{Skipped, Ran{properties}}` and failing closed when the env var is absent; `applies_to` counts pinned to committed floors rather than derived circularly; `RunError`, `ApplyError::class()`, `BaselineStore`'s full `impl` and an owned `GatheredFacts` with a `plan_inputs()` borrowed view all declared; `RemoteFacts.body` dropped (no reader — the digest port carries it); the `ExternalId` claim replaced with an explicit emptiness collapse at `LocalItem` construction, pinned by a classify row and a construction test; a `SelectionError` under `create --push` routed through `push_decide` to `local-save` rather than exiting 72 (the only path exercisable before 0171); the absent-mtime row marked `["rust"]` with the sentinel reason; `mtime_offset`'s bash realisation stated; `tracker-test-support` lib tests specified; AC 7/11 misreferences corrected; and AC 16's four unlifted suites recorded as a deviation with the reason.

**Verified, not asserted**: the `gather` → `plan` seam — stubbed out of the pass-3 compile check and the source of one finding — now compiles end to end with owned facts and on-demand borrowed views, as does the empty-string-to-`None` conversion at the entry boundary.

### Remaining Four Gaps — Closed

Plan now 3287 lines. The four gaps left open at the end of pass 4 are closed, two of them compile-checked rather than reasoned about:

- **The pending-push decision is sited in the domain.** `work::sync::push_precondition` takes `(&MarkerState, request_digest, &dyn Fn(&ExternalId) -> bool)` and returns `PushPrecondition::{Proceed, ReuseId, Refuse(RefusalReason)}`; `PendingPush` and `RequestFingerprint` move with it, leaving `path`/`read`/`render`/`outstanding` (JSON and filesystem) in `work-adapters`. `create.rs` executes the returned decision, and the marker tests drive the function directly. The plan's own argument for siting `push_decide` in the domain applies with more force to a five-branch table whose wrong branch causes unrecoverable remote data loss.

- **The corpus check refuses rather than quietly succeeding.** This changed behaviour, not just wording. The earlier text deleted the marker and exited 0 "having changed nothing" — wrong for the one case the fingerprint provably cannot distinguish: two deliberate creates with identical title, body and kind are *indistinguishable requests*, so a genuine second create would find the first item, report success, and create nothing. It now refuses naming both paths, handing the operator the only decision they can make. `RefusalReason::AlreadyWritten` added; stdout specified for the reuse branch (line 1 path, line 2 `write-once\t<id>`).

- **The marker warning has a change site.** `pending_push::outstanding()` enumerates the directory and `work-cli/src/sync.rs` prints one named warning per marker on stderr. Previously the behaviour was asserted in Phase 5's prose while the change site sat in Phase 4 — before `pending_push` exists to enumerate — so it belonged to neither phase and nothing tested it. Since no SKILL invokes the binary in this story, this is the only way an operator learns a marker is blocking a title.

- **The stringly-keyed join is gone.** `RunOutcome` becomes `RunReport { reported: Vec<ReportedItem>, … }` where `ReportedItem { planned, outcome: ItemOutcome }`, so a whole report line renders from one record and `ApplyError::class()` supplies field 4 directly. The rename also resolves a collision: `work-cli` already uses `RunOutcome` for three per-command result enums. `RemoteFacts.body` dropped in the same pass — it had no reader, since the digest port carries the body.

Five new tests specified: `push_precondition`'s five branches as a domain unit test, the already-written refusal, the reuse branch's stdout, `sync`'s marker warning, and the `jj status` check for a written marker.

**Compile-checked**: the `ItemOutcome`/`ReportedItem` fold with a working `render()` that needs no lookup, and `push_precondition`'s full match — both build at edition 2021 / MSRV 1.90.

No findings from pass 3 remain open. The plan has not been re-reviewed since these edits.

## Approval

**Verdict set to APPROVE by the author, 2026-08-13**, superseding the REVISE verdicts of
passes 1-3. Every finding raised across the three passes is resolved, partially resolved
with the residual recorded, or explicitly accepted as a non-goal; no finding remains open.

Two things a later reader should know rather than infer.

The plan was **not re-reviewed after the final round of edits** — the four gaps closed in
that round (the pending-push precondition's siting, the corpus check's refusal, the marker
warning's change site, and the `RunReport` fold) carry compile evidence for the two that
are type-shaped, but no lens has read them. On the record of this review, a fourth pass
would likely find a handful of further editorial defects, concentrated in the sections
those edits touched; the reviewer's recommendation was to implement Phase 1 instead, on
the grounds that a compiler and a test runner have been more productive than further
reading.

Two items need action **outside** this plan. AC 14 and the Requirements bullet at
`meta/work/0194-…md:236-246` say a cleared baseline entry classifies `indeterminate`; that
is unreachable, and the criterion should be corrected to `conflict`. AC 16 names four
bash suites whose tables this story deliberately does not lift, recorded as a non-goal
with the reason; that criterion should be narrowed. Both are work-item edits, not plan
edits, and a validator reading either criterion literally would mark a correct
implementation failed.

---
*Review generated by /accelerator:review-plan*
