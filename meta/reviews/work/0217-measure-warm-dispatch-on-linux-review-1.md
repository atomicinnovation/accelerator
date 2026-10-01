---
type: "work-item-review"
id: "0217-measure-warm-dispatch-on-linux-review-1"
title: "Work Item Review: Measure warm dispatch on linux"
date: "2026-09-14T13:59:07+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0217"
work_item_id: "0217"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-09-20T19:27:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Measure warm dispatch on linux

**Verdict:** REVISE

0217 is a well-structured, unusually transparent measurement task — every section is present and substantively populated, the central uncertainty (whether the darwin result transfers) is carried from Summary into a matching criterion, and known tensions are surfaced rather than hidden. It reaches REVISE not on structural gaps but on a cluster of precision defects concentrated in the Acceptance Criteria and their supporting Requirements: an unresolved keying dimension the harness cannot produce, a throughput formula off by 1000×, and several internal term mismatches (ceilings/floors, three/four provenance fields, definite "the linux key" against two candidate keys). None is critical; the count of high-confidence major findings drives the verdict.

### Cross-Cutting Themes

- **The `(architecture, SHA-extension support, libc)` keying gap** (flagged by: testability, clarity, scope) — Requirements bullet 5 and AC 6 firmly require recording against axes `PLATFORM_TABLE` does not key on, while Open Question 1 says the mechanism is unresolved and must be settled first. The same root defect reads three ways: AC 6 admits no definitive pass/fail until the question resolves (testability), the firm "must" contradicts the open question (clarity), and a resolution toward extending the harness would break the declared `kind: task` sizing (scope).
- **AC 6 throughput formula disagrees 1000× with the referenced README** (flagged by: clarity, testability) — AC 6 records `asset_bytes ÷ median_ms`; `tasks/README.md#the-measure-namespace` defines `asset_bytes / (median_ms × 1000)` for decimal MB/s. Following AC 6 literally yields bytes-per-millisecond, defeating comparison against the ~555 MB/s soft band the item itself cites.
- **Acceptance Criteria omit mandatory deliverables named elsewhere** (flagged by: completeness, testability) — the README criterion-constants block plus its lockstep unit test, and the linux replacement for the darwin `pmset` power probes, are mandatory per Technical Notes/Requirements but appear in no criterion, so a run could satisfy every AC with the README constants out of sync or the load rung recording `unknown`.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: AC 6 requires keying the harness cannot record, with no verification method until the Open Question resolves
  **Location**: Acceptance Criteria (AC 6) and Requirements
  AC 6 requires the throughput "keyed on SHA-extension support" and Requirements requires recording "against (architecture, SHA-extension support, libc)", but `PLATFORM_TABLE` keys only on `(system, machine)` and no field carries libc or SHA-extension support. The Open Question is explicitly unresolved, so there is no defined artefact a verifier can inspect.

- 🟡 **Testability**: "Gate values justified from the linux figures, not inherited from darwin" has no defined verification procedure
  **Location**: Acceptance Criteria (AC 3)
  AC 3 requires floors recorded and gate values "justified" but supplies no derivation rule. Because 0189's ceilings are deliberately round numbers, a legitimately-derived linux ceiling could land on the same 50/60/70/80 values, so a verifier cannot conclusively distinguish "justified" from "copied".

- 🟡 **Clarity**: Provenance fields described inconsistently (three vs four, and wrong "None" set)
  **Location**: Context
  Context describes three provenance fields ("chip, resolved bash, resolved shasum — which the darwin entry currently leaves None"), but Technical Notes and `_DARWIN_CALIBRATION` state four (session, chip, bash, shasum) with `session` and `chip` already recorded — only `bash` and `shasum` are unset on darwin.

- 🟡 **Clarity**: Requirement names "ceilings", its criterion names "floors"
  **Location**: Requirements / Acceptance Criteria
  Requirements bullet 4 says record "the absolute ceilings", while AC 3 says "the instrument floors are recorded and their gate values justified". These are distinct per-entry constants (C1–C4 thresholds versus the quiet-host `bash_floor_ms`/`true_floor_ms` gate), so it is unclear whether both, or which one, must be justified from linux figures.

- 🟡 **Clarity**: AC 6 throughput formula disagrees 1000× with the referenced README
  **Location**: Acceptance Criteria (AC 6)
  AC 6 records `asset_bytes ÷ median_ms`; the referenced README defines `asset_bytes / (median_ms × 1000)` for decimal MB/s, and AC 6 states no unit. The recorded figure could be off by 1000× or unit-ambiguous, defeating the ~555 MB/s comparison.

- 🟡 **Clarity**: "The linux key" is definite but two keys exist
  **Location**: Requirements / Acceptance Criteria
  Summary, Context, Requirements and AC 2 refer to "the linux key" as one entity, but Technical Notes reveals `("Linux","aarch64")` and `("Linux","x86_64")`, with only Drafting Notes designating aarch64-musl as required. AC 2 ("the linux key") and AC 6 ("aarch64-unknown-linux-musl") could be satisfied by different platforms.

- 🟡 **Dependency**: Release cut carrying 0216 is an uncaptured prerequisite between "0216 lands" and running the measurement
  **Location**: Dependencies
  AC 6 measures throughput "under the `sha2` 0.11 hardware backend 0216 ships" and Requirements says "No build: the shipped musl artefact is fetched and verified". For the fetched artefact to contain 0216's backend, a signed release postdating 0216 must be cut — an epic-0136 action 0189 named. Dependencies captures only "Sequence after 0216 lands" (landing-in-tree, not release-published).

#### Minor

- 🔵 **Scope**: Summary understates scope relative to Requirements and Acceptance Criteria
  **Location**: Summary
  The Summary frames the work as "run the harness and record the figures", but Requirements/AC also carry the 0216-deferred aarch64-musl throughput (AC 6) and the ceiling-justification work, making it easy to close the item on the harness run alone.

- 🔵 **Scope**: Task-sizing is contingent on an unresolved harness-extension fork
  **Location**: Open Questions
  Context asserts this "runs a task rather than re-authoring a script", but Requirement 5 + Open Question 1 admit a resolution that "extend[s] the harness — scope beyond a measurement task", so `kind: task` is contingent on that fork.

- 🔵 **Dependency**: Release base URL / network egress is an external coupling named only in Requirements
  **Location**: Requirements
  Requirements names "network egress to the release base URL" and fetch-and-verify of the musl artefact, but the external release-hosting service and its reachability are not tracked in Dependencies as an availability coupling that yields no verdict when down.

- 🔵 **Dependency**: Directed ordering/consumer edges live only in prose; frontmatter carries only undirected `relates_to`
  **Location**: Frontmatter: relates_to
  "Runs after 0216" and "Consumed by 0219" are directed couplings, but frontmatter records 0216 and 0219 only under `relates_to`, with no `blocked_by`/`blocks` edges (contrast 0189's `blocked_by`), so the ordering is invisible to graph/scheduling tooling.

- 🔵 **Testability**: AC 6 cannot confirm the hardware backend engaged, and its formula disagrees with the canonical one
  **Location**: Acceptance Criteria (AC 6) and Technical Notes
  AC 6 gives no procedure to confirm the hardware path engaged rather than the soft backend — the only signal (clearing the ~555 MB/s band) lives in Technical Notes without a threshold — and its formula differs 1000× from the README's MB/s derivation.

- 🔵 **Testability**: "Completes a valid session" does not state its pass condition
  **Location**: Acceptance Criteria (AC 1)
  AC 1 requires a "valid session" but leans on harness semantics it does not restate; the harness records invalidated sessions too (branch 5b), so a drifted or teardown-failed run producing a committed record could be argued to meet AC 1.

- 🔵 **Testability**: Verifiable, mechanically-forced outcomes are absent from the Acceptance Criteria
  **Location**: Technical Notes / Requirements (no matching criterion)
  The README criterion-constants block + lockstep test, and the linux replacement for the darwin `pmset` power probes ("or the load rung records `unknown`"), are concrete testable outcomes with no criterion, so a run could pass every AC while leaving them unaddressed.

- 🔵 **Clarity**: Firm "must record against (arch, SHA-ext, libc)" contradicts the Open Question
  **Location**: Requirements
  Requirements bullet 5 and AC 6 read as settled ("must", "keyed on"), while Open Question 1 says the mechanism is unresolved and directs "Resolve before authoring the entry". (Drafting Notes acknowledge this; the primary sections still read as settled.)

- 🔵 **Clarity**: "Open Question 1" reference does not match the Open Questions section
  **Location**: Drafting Notes
  Drafting Notes say "Open Question 1 resolved to a native-aarch64 Colima VM", but the sole Open Question is the axes-keying question, not the VM/host choice (which now sits in Assumptions), so the cross-reference misleads.

- 🔵 **Clarity**: 0216 stated both as "done" and as not yet landed
  **Location**: Dependencies / Drafting Notes
  Dependencies frames 0216 as future ("Sequence after 0216 lands") while Drafting Notes states "0216 is done and explicitly deferred it" and Requirements/AC use present tense ("0216 ships"), so a reader cannot tell whether this item is ready to start.

#### Suggestions

- 🔵 **Completeness**: README constants + lockstep test update is a mandatory deliverable with no acceptance criterion
  **Location**: Acceptance Criteria
  Technical Notes say registering an entry "also means updating the README criterion-constants block and its lockstep unit test", but no criterion captures it; a reader using the AC as the definition of done would not see the obligation (the CI lockstep guard lowers, but does not close, the risk).

- 🔵 **Testability**: "Whether the darwin result transferred" has no threshold and constrains attribution to two causes
  **Location**: Acceptance Criteria (AC 4)
  "Transferred" has no defined magnitude, and constraining attribution to "spawn cost or digest backend" leaves no verifiable path if a third factor dominates, so the statement can be written to pass regardless of the figures.

- 🔵 **Clarity**: Core symbols (G, B, C1–C6, branch 7) used without local definition
  **Location**: Summary / Context
  `G`, `B`, the cells C1–C6 and "branch 7" appear throughout without introduction; their meanings live only in 0189's Latency Criterion, so the reader must have 0189 open to parse even the Summary.

### Strengths

- ✅ Every standard and optional section is present and substantively populated, the frontmatter is intact with a recognised `kind` and appropriate `draft` status, and the central open question is carried from Summary into a dedicated criterion (AC 4).
- ✅ Context explains the motivation precisely — darwin-x64 and linux-arm64 have no CI lane, and two named effects (spawn cost, digest backend) push the ratio in opposite directions — so transfer is correctly framed as an open question rather than an assumption.
- ✅ The one-off-versus-recurring boundary against 0219 is stated and reasoned about (0219 consumes this entry as its calibration source), and the 0216 hand-off is precisely justified in Drafting Notes rather than left implicit.
- ✅ Known tensions are surfaced, not hidden — the keying mismatch is captured as an Open Question and echoed in Drafting Notes rather than silently rewritten.
- ✅ AC 2 and AC 5 are fully enumerable and verifiable: AC 2 names the four provenance fields and requires a "calibrated" honesty note; AC 5 is a well-formed conditional (Perl absent → C3/C4/C6 not applicable, branch 7, with reason).
- ✅ The required host configuration (native aarch64 Colima VM, never cross-arch emulation) is named precisely across Assumptions, Technical Notes and Drafting Notes, with the reason it matters to timing and SHA-2 detection.

### Recommended Changes

1. **Resolve Open Question 1, then rephrase Requirement 5 and AC 6 to name the concrete artefact a verifier reads** (addresses: "AC 6 requires keying the harness cannot record", "Firm 'must record against…' contradicts the Open Question", "Task-sizing is contingent on an unresolved harness-extension fork")
   Settle whether the SHA-extension/libc facts ride in the free-text `chip`/`shasum` provenance under an architecture-only key, or the harness is extended. Then make the Requirements/AC wording conditional on — or consistent with — that resolution, and if it lands on extending the harness, split the keying change into its own work item so this stays `kind: task`.

2. **Fix the AC 6 throughput formula and state its unit** (addresses: "AC 6 throughput formula disagrees 1000× with README", "AC 6 cannot confirm the hardware backend engaged")
   Align AC 6 to `asset_bytes / (median_ms × 1000)` decimal MB/s per the README, and add an engagement threshold (e.g. "throughput ≥ 555 MB/s, confirming the ARMv8 SHA-2 path engaged") so the recorded figure is both correctly-scaled and self-verifying.

3. **Reconcile the ceilings/floors terminology across Requirements and AC 3, and state the derivation rule** (addresses: "Requirement names 'ceilings', its criterion names 'floors'", "'Gate values justified' has no defined verification procedure")
   Use consistent terms, say explicitly whether the absolute ceilings, the instrument floors, or both must be justified from linux figures, give the derivation rule (e.g. "bootstrap upper bound rounded up to the next 10 ms with ≥20% headroom, printed beside each ceiling"), and state whether a provisional VM-scoped ceiling satisfies AC 3.

4. **Reconcile the provenance-field count in Context with Technical Notes** (addresses: "Provenance fields described inconsistently")
   Enumerate all four fields (session, chip, bash, shasum) in Context and state that only `bash` and `shasum` are unset on darwin.

5. **Disambiguate "the linux key" in the primary sections** (addresses: "'The linux key' is definite but two keys exist")
   Name Linux/aarch64-musl as the required key and Linux/x86_64 as optional in Requirements/AC, so AC 2 and AC 6 unambiguously refer to the same platform.

6. **Capture the release-cut prerequisite and the release-hosting coupling in Dependencies, and promote directed edges to frontmatter** (addresses: "Release cut carrying 0216 is an uncaptured prerequisite", "Release base URL / network egress named only in Requirements", "Directed edges live only in prose")
   Add a Dependencies note that this item is blocked on a published, minisign-signed release whose version includes 0216's backend (naming the epic-0136 release owner), list the release base URL as an external availability coupling, and add `blocked_by: ["work-item:0216"]` plus a blocks edge to 0219.

7. **Add criteria for the mechanically-forced deliverables and tighten the soft criteria** (addresses: "Verifiable outcomes absent from AC", "README constants + lockstep test has no criterion", "'Completes a valid session' does not state its pass condition", "'Whether the darwin result transferred' has no threshold")
   Add criteria for the README constants + lockstep test passing and the load rung recording a real power state (not `unknown`); make AC 1's validity condition an explicit field read (e.g. `validity == VALID`, not branch 5); and give AC 4 a concrete transfer test with attribution allowed beyond the two named causes.

8. **Reconcile 0216's status and add a one-line pointer to 0189's symbol definitions** (addresses: "0216 stated both as 'done' and not yet landed", "'Open Question 1' reference does not match the Open Questions section", "Core symbols used without local definition")
   State 0216's status consistently, fix the "Open Question 1" cross-reference in Drafting Notes, and add a one-line pointer on first use that G, B, C1–C6 and the branch labels are defined in 0189's Latency Criterion.

## Per-Lens Results

### Clarity

**Summary**: The work item is dense but generally precise, with rich, consistent domain language and unusually transparent handling of several known tensions. The main clarity risks are internal inconsistencies across sections: the calibration-provenance field count and which darwin fields are recorded (Context vs Technical Notes), a ceilings-versus-floors term mismatch between Requirements and Acceptance Criteria, and a throughput formula in AC 6 that disagrees by 1000× with the referenced README. "The linux key" is also used as a single definite entity while two candidate keys exist, resolved only in the later notes.

**Strengths**:
- Transparent about the "(architecture, SHA-extension support, libc)" keying tension, capturing it as an explicit Open Question and Drafting Note rather than silently rewriting the requirement.
- The scope boundary against 0219 (one-off operator run vs recurring scheduled lane) is stated clearly and unambiguously.
- Assumptions and Technical Notes precisely name the required host configuration (native aarch64 Colima VM, never cross-arch emulation) and explain why it matters to timing and SHA-2 detection.

**Findings**:
- 🟡 major, high confidence — **Provenance fields described inconsistently (three vs four, and wrong "None" set)** (Context). Context describes three fields left `None` on darwin; Technical Notes and `_DARWIN_CALIBRATION` state four fields (session, chip, bash, shasum) with only `bash`/`shasum` unset. Risks an implementer treating `chip` as unrecorded or overlooking `session`.
- 🟡 major, high confidence — **Requirement names "ceilings", its criterion names "floors"** (Requirements / Acceptance Criteria). Requirements bullet 4 says "absolute ceilings"; AC 3 says "instrument floors". These are distinct constants (C1–C4 thresholds vs `bash_floor_ms`/`true_floor_ms`), so it is unclear which must be justified from linux figures.
- 🟡 major, high confidence — **AC 6 throughput formula disagrees 1000× with referenced README** (Acceptance Criteria). AC 6 uses `asset_bytes ÷ median_ms`; README defines `asset_bytes / (median_ms × 1000)` for decimal MB/s. Literal AC 6 yields bytes-per-millisecond, defeating the ~555 MB/s comparison.
- 🟡 major, medium confidence — **"The linux key" is definite but two keys exist** (Requirements / Acceptance Criteria). Only Drafting Notes designate aarch64-musl as required; AC 2 ("the linux key") and AC 6 ("aarch64-unknown-linux-musl") could be satisfied by different platforms.
- 🔵 minor, medium confidence — **Firm "must record against (arch, SHA-ext, libc)" contradicts the Open Question** (Requirements). The firm "must" reads settled while Open Question 1 says the mechanism is unresolved.
- 🔵 minor, high confidence — **"Open Question 1" reference does not match the Open Questions section** (Drafting Notes). The Drafting Note describes the VM decision; the sole Open Question is the axes-keying one.
- 🔵 minor, medium confidence — **0216 stated both as "done" and as not yet landed** (Dependencies / Drafting Notes). "Sequence after 0216 lands" versus "0216 is done and explicitly deferred it" versus present-tense "0216 ships".
- 🔵 suggestion, medium confidence — **Core symbols (G, B, C1–C6, branch 7) used without local definition** (Summary / Context). Their meanings live only in 0189's Latency Criterion; no pointer at point of use.

### Completeness

**Summary**: Work item 0217 is a `task` and is exceptionally complete: every standard section is present and substantively populated, the frontmatter is intact with a recognised kind and an appropriate `draft` status, and the central uncertainty (whether the darwin result transfers) is carried from Summary into a matching acceptance criterion. The only gap worth noting is that at least one mandatory deliverable described in Technical Notes is not reflected in the Acceptance Criteria's definition of done.

**Strengths**:
- Summary is a single, unambiguous action statement that names the subject of the work.
- Context thoroughly explains the motivation (no CI lane for darwin-x64/linux-arm64; two named effects push the ratio opposite ways) so transfer cannot be assumed.
- All optional sections carry real content: Open Questions, Dependencies (0216/0189/0205/0219/0136 with rationale), Assumptions (native-aarch64 VM), Technical Notes.
- Six specific acceptance criteria define done, and the central open question has a dedicated criterion (AC 4).
- Frontmatter is complete and correct (recognised kind, draft status, priority, parent/derived_from/relates_to/tags populated).
- Known tensions surfaced rather than hidden (keying mismatch as Open Question echoed in Drafting Notes).

**Findings**:
- 🔵 suggestion, medium confidence — **README constants + lockstep test update is a mandatory deliverable with no acceptance criterion** (Acceptance Criteria). Technical Notes require updating the README criterion-constants block and its lockstep unit test in the same change, but no criterion captures it; the CI lockstep guard lowers but does not close the definitional gap.

### Dependency

**Summary**: The core work-item ordering chain (0216 → 0217 → 0219) is explicitly and directionally captured in the Dependencies prose, with the scope boundary against 0219 deliberately negotiated and the 0216 hand-off (AC 6) precisely justified — a well-mapped item. The main gaps are an implied prerequisite that sits between "0216 lands" and "this measurement can run" (a published, signed release actually carrying 0216, an epic-0136 action 0189 named but this item does not), an external-hosting coupling (release base URL / network egress) named only in Requirements, and directed edges that live only in prose with no machine-readable frontmatter.

**Strengths**:
- Upstream ordering on 0216 captured with direction and rationale, tied to the specific deferred deliverable (aarch64-musl throughput, AC 6).
- Downstream consumer 0219 named as "Consumed by", identifying this item's entry as 0219's calibration source.
- Scope boundary with 0219 explicitly negotiated in Drafting Notes, including the fallback that lane-wiring belongs here if the boundary is wrong.
- Measurement/decomposition prerequisites (tool set, digest backends, musl target/linker, signed release, network egress) enumerated in detail, with the native-aarch64-guest prerequisite thorough across Assumptions/Technical Notes/Drafting Notes.

**Findings**:
- 🟡 major, medium confidence — **Release cut carrying 0216 is an uncaptured prerequisite between "0216 lands" and running the measurement** (Dependencies). For the fetched artefact to contain 0216's backend, a signed release postdating 0216 must be cut (an epic-0136 action 0189 named); Dependencies captures only "Sequence after 0216 lands", risking a scheduling surprise.
- 🔵 minor, medium confidence — **Release base URL / network egress is an external coupling named only in Requirements** (Requirements). The external release-hosting service and its reachability are not tracked in Dependencies as an availability coupling that yields no verdict when down.
- 🔵 minor, medium confidence — **Directed ordering/consumer edges live only in prose; frontmatter carries only undirected `relates_to`** (Frontmatter: relates_to). No `blocked_by`/`blocks` edges (contrast 0189's `blocked_by`), so ordering is invisible to graph/scheduling tooling.

### Scope

**Summary**: Work item 0217 is a well-bounded measurement task centred on producing one calibrated linux platform entry from a run of the committed harness, with clean, explicitly-stated boundaries against its siblings (0219 owns the recurring lane; x86_64 is a non-discharging secondary target). Two mild scope signals remain: the Summary describes a narrower scope than the Requirements/Acceptance Criteria (which also carry a distinct throughput deliverable deferred from the closed 0216), and the "measurement task, not script re-authoring" sizing is contingent on Open Question 1 not forcing a harness extension. Neither rises to a delivery-risk level.

**Strengths**:
- Clean one-off vs recurring boundary with 0219: this item produces the calibrated entry as a single operator run; the recurring lane is left to 0219.
- The required target scope is a single platform entry — Linux/aarch64-musl required, Linux/x86_64 explicitly secondary and non-discharging — so two entries are not silently bundled.
- The hand-off boundary with the closed 0216 is stated and reasoned about in Drafting Notes rather than left implicit.

**Findings**:
- 🔵 minor, medium confidence — **Summary understates scope relative to Requirements and Acceptance Criteria** (Summary). The Summary omits the 0216-deferred aarch64-musl throughput (AC 6) and the ceiling-justification work, making it easy to close the item on the harness run alone.
- 🔵 minor, medium confidence — **Task-sizing is contingent on an unresolved harness-extension fork** (Open Questions). Requirement 5 + Open Question 1 admit a resolution that extends the harness ("scope beyond a measurement task"), on which `kind: task` depends.

### Testability

**Summary**: As a measurement/operator task, 0217 is strongest where it names concrete, inspectable artefacts (a committed record, four enumerated provenance fields, branch-7 not-applicable outcomes) and weakest where success rests on judgement ("gate values justified", "whether the darwin result transferred") or on a keying dimension the harness cannot currently produce. Two acceptance criteria (AC 3 and AC 6) lack a definitive pass/fail procedure. Several mechanically-forced, highly verifiable outcomes (README constants + lockstep test, load-rung power probes) appear in Requirements/Technical Notes but in no criterion.

**Strengths**:
- AC 2 is fully enumerable and verifiable: it names the four provenance fields and requires the session reported "calibrated" rather than as context.
- AC 5 is a well-formed conditional criterion tied to a concrete harness behaviour (Perl absent → C3/C4/C6 not applicable, branch 7, with reason).
- AC 1 anchors on a concrete artefact (a record committed under `meta/measurements/`).
- Criteria are grounded in observable harness outputs (branch numbers, closure semantics, provenance notes, persisted `asset_bytes`), and AC 6 supplies an explicit derivation formula.

**Findings**:
- 🔴 (reported major) high confidence — **AC 6 requires keying the harness cannot record, with no verification method until the Open Question resolves** (Acceptance Criteria AC 6 / Requirements). `PLATFORM_TABLE` keys only on `(system, machine)`; no field carries libc or SHA-extension support, and the Open Question is unresolved, so no defined artefact a verifier can inspect.
- 🟡 major, medium confidence — **"Gate values justified from the linux figures, not inherited from darwin" has no defined verification procedure** (Acceptance Criteria AC 3). No derivation rule; a legitimately-derived linux ceiling could land on the same round 50/60/70/80 values, so "justified" versus "copied" is unresolvable.
- 🔵 minor, medium confidence — **AC 6 cannot confirm the hardware backend engaged, and its formula disagrees with the canonical one** (AC 6 / Technical Notes). No engagement threshold in the criterion (the ~555 MB/s signal lives in Technical Notes without a threshold); formula differs 1000× from the README's MB/s derivation.
- 🔵 minor, medium confidence — **"Completes a valid session" does not state its pass condition** (AC 1). The harness records invalidated sessions too (branch 5b); a drifted/teardown-failed run producing a committed record could be argued to meet AC 1.
- 🔵 minor, medium confidence — **Verifiable, mechanically-forced outcomes are absent from the Acceptance Criteria** (Technical Notes / Requirements). README constants + lockstep test and the `pmset` replacement ("or the load rung records `unknown`") are testable outcomes with no criterion.
- 🔵 suggestion, medium confidence — **"Whether the darwin result transferred" has no threshold and constrains attribution to two causes** (AC 4). No defined magnitude for "transferred"; attribution limited to spawn cost or digest backend leaves no verifiable path if a third factor dominates.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-20

**Verdict:** REVISE

Every finding from pass 1 that was targeted is resolved — a clean sweep across
all five lenses. Pass 2 is REVISE on the major count (3 ≥ 2), not on any
unresolved pass-1 issue: the three majors are all second-order — two are AC 3
wording introduced by the pass-1 fix (a rounding-rule/headroom-band conflict and
still-underspecified floors), and one is a pre-existing `reverify`-versus-
`verifier::sha256_hex` term/unit conflation the fixes did not touch. All are
cleanly fixable.

### Previously Identified Issues

- 🟡 **Clarity**: Provenance fields inconsistent (three vs four) — Resolved
- 🟡 **Clarity**: Requirement "ceilings" vs criterion "floors" — Resolved
- 🟡 **Clarity**: AC 6 throughput formula 1000× — Resolved
- 🟡 **Clarity**: "The linux key" definite but two exist — Resolved
- 🔵 **Clarity**: Firm "must record against…" vs Open Question — Resolved
- 🔵 **Clarity**: "Open Question 1" cross-reference mismatch — Resolved
- 🔵 **Clarity**: 0216 stated both done and not-landed — Resolved
- 🔵 **Clarity**: Core symbols (G, B, C1–C6, branch 7) undefined — Resolved
- 🟡 **Testability**: AC 6 keying unverifiable — Resolved
- 🟡 **Testability**: AC 3 "justified" no procedure — Partially resolved
  (ceilings gained a rule; floors still lack one — see new findings)
- 🔵 **Testability**: AC 6 HW-engagement / formula — Resolved (formula fixed;
  engagement threshold added but now flagged approximate — see new findings)
- 🔵 **Testability**: AC 1 "valid session" undefined — Resolved
- 🔵 **Testability**: Mechanical outcomes absent from AC — Resolved (AC 7, AC 8)
- 🔵 **Testability**: AC 4 no transfer threshold — Resolved (numeric test added;
  attribution clause now flagged — see new findings)
- 🟡 **Dependency**: Release-cut prerequisite uncaptured — Resolved
- 🔵 **Dependency**: Release base URL not in Dependencies — Resolved
- 🔵 **Dependency**: Directed edges prose-only — Resolved
- 🔵 **Scope**: Summary understates scope — Still present (broadened: now the
  AC 7/AC 8 code and doc/test changes)
- 🔵 **Scope**: Task-sizing contingent on harness fork — Resolved
- 🔵 **Completeness**: README/lockstep AC missing — Resolved (AC 7)

### New Issues Introduced

#### Major

- 🟡 **Clarity** (Requirement 5 / AC 6): `reverify` ms-per-MB and
  `verifier::sha256_hex` MB/s are different terms (parent vs sub-operation) with
  reciprocal units, but the item reads as one measurement — state whether one
  figure or two, under which name and unit.
- 🟡 **Testability** (AC 3): the ceiling rounding rule and the ~18–30% headroom
  band can conflict with no tie-break (UB 49 → 50 ≈ 2%; next is 60 ≈ 33%). Make
  the rounding rule authoritative; demote the band to an observation.
- 🟡 **Testability** (AC 3 / Requirement 4): the instrument floors are still
  required "justified" with no derivation rule, unlike the ceilings.

#### Minor / Suggestions

- 🔵 **Clarity** (Requirement 5): "architecture-only platform key
  `(system, machine)`, not the OS name" is self-contradictory — the tuple
  includes `system` = OS. Describe the key as `(system, machine)` (OS + arch).
- 🔵 **Testability** (AC 6): "~555 MB/s" is approximate — a boundary reading has
  no verdict. Use an exact floor.
- 🔵 **Testability** (AC 4): the "or another factor the evidence points to"
  attribution clause is always satisfiable — scope it or mark it narrative.
- 🔵 **Testability** (AC 2 / AC 6): the `musl` libc fact is required in
  provenance but no criterion verifies it appears.
- 🔵 **Completeness** (Frontmatter): `last_updated` predates the body's
  2026-09-15 resolution notes.
- 🔵 **Clarity** (Context, low confidence): the no-entry "raises, no record"
  framing sits awkwardly beside inherited branch-7 "records context"; likely
  already disambiguated by Technical Notes' two-levels note.
- 🔵 **Dependency** (suggestion): the `rustup target add` musl-toolchain fetch is
  not reflected in Dependencies.
- 🔵 **Completeness** (suggestion): `status` remains `draft` though the item
  reads review-ready (status is a separate workflow step, not changed in review).

### Assessment

The pass-1 revision cleared every finding it targeted. The three remaining
majors are refinements of the fixes rather than structural problems: make AC 3's
rounding rule authoritative, give the floors their own derivation rule, and name
the two digest measurements (`reverify` and `verifier::sha256_hex`) distinctly.
Those three edits, plus the minor keying-axis and `musl`-provenance wording,
should take the item to APPROVE.

## Re-Review (Pass 3) — 2026-09-20

**Verdict:** REVISE

Strong convergence. The three pass-2 majors and most pass-2 minors are resolved,
and every resolved finding stayed resolved. The two remaining majors are both
AC-level precision points introduced by the pass-2 fixes; the verdict trips on
the major count (2), not on any structural problem. Everything else is minor or
suggestion, clustered on a few spots.

### Previously Identified Issues (pass 2 → pass 3)

- 🟡 **Clarity**: `reverify` vs `verifier::sha256_hex` conflated — Resolved
- 🟡 **Testability**: AC 3 rounding-rule / headroom-band conflict — Resolved
- 🟡 **Testability**: AC 3 floors no derivation — Partially resolved (derivation
  added but unquantified — see new majors)
- 🔵 **Clarity**: keying-axis self-contradiction — Resolved
- 🔵 **Testability**: AC 6 `~555` approximate — Resolved (`≥ 700 MB/s`)
- 🔵 **Testability**: AC 4 attribution always-satisfiable — Resolved
- 🔵 **Testability**: `musl` not verified by any AC — Resolved (AC 2)
- 🔵 **Completeness**: `last_updated` stale — Resolved
- 🔵 **Scope**: Summary understates scope — Still present (softened to suggestion)
- 🔵 **Dependency**: `rustup` toolchain not in Dependencies — Still present (sug.)

### New Issues Introduced

#### Major

- 🟡 **Testability** (AC 3): the instrument-floor gate derivation is unquantified
  ("headroom to spare") where the ceiling rule carries a precise band. Give the
  floors a defined band (e.g. 40–80% above the measured `FLOOR_SAMPLES` median,
  bracketing darwin's 46–75%).
- 🟡 **Testability** (AC 4): "within 10% of darwin's recorded figure" does not
  name which darwin recording; 0189 records several (0205's 42.28 vs the
  authoritative 35.531, ~19% apart). Pin the reference to 0189's Validation
  Results (35.531 / 38.230 / 51.496 / 55.291; C5 1.3260 [1.3236, 1.3279]).

#### Minor / Suggestions

- 🔵 **Clarity / Testability** (AC 4): the trailing "or" makes C5's test
  ambiguous — state the rule per cell.
- 🔵 **Clarity / Testability / Completeness** (AC 8): "load rung" conflates
  `cpu_count_rung` with `power_state`, and "a real power state" admits a
  meaning-free pass — name the `power_state` field, its linux source, and the
  acceptable values.
- 🔵 **Clarity / Testability** (Requirement 5): `reverify` is reported as flat ms
  (not per-MB) per README, and the `reverify` recording has no AC — fix the unit
  and either add an AC or fold the digest recording into AC 6.
- 🔵 **Clarity / Scope** (Summary): still under-describes the deliverable set
  (calibrated entry, ceilings/floors, README/lockstep, power probes).
- 🔵 **Dependency** (minor): the real remaining blocker (the release cut) has no
  machine-readable frontmatter edge.
- 🔵 **Suggestions**: HWCAP undefined; Context "correct behaviour … resolves"
  phrasing; `status: draft` though review-ready; `rustup` toolchain in
  Dependencies; AC 8 names no concrete linux probe.

### Assessment

Two rounds of revision took the item from 7 majors to 2, every resolved finding
holding. The two remaining majors are one-line precision fixes (pin AC 4's
darwin reference; quantify the AC 3 floor band), and the minors cluster on three
spots (AC 4, AC 8, Requirement 5's `reverify` unit). This is the point of
diminishing returns — each pass surfaces finer wording against a structurally
sound item. Fixing the two majors and the AC 4 / AC 8 / `reverify`
clarifications settles the substance; the rest is optional polish.

## Re-Review (Pass 4) — 2026-09-20

**Verdict:** REVISE

Every pass-3 finding that was actioned is resolved — but three new majors
appeared, each an interaction between the pass-3-round fixes and the item's own
branches (the VM assumption, the Perl-absent branch, and the two darwin figure
tables). The major count is not converging: passes 2→3→4 ran 3, 2, 3 majors.
This is the diminishing-returns signature of an iterative review loop — each
round's added specificity creates a new edge with the item's other specific
claims. The substance has been implementation-ready since pass 2.

### Previously Identified Issues (pass 3 → pass 4)

- 🟡 **Testability**: AC 3 floor gate unquantified — Resolved (40–80% band; a new
  minor notes the pre/post floor basis)
- 🟡 **Testability**: AC 4 darwin reference not fixed — Resolved (pinned to 0189
  Validation Results)
- 🔵 **Clarity/Testability**: AC 4 trailing-"or" ambiguity — Resolved (per-cell;
  a new suggestion notes the C5 disjunct is now redundant)
- 🔵 **Clarity/Testability/Completeness**: AC 8 "load rung"/weak pass — Resolved
  (`power_state` named; a new major notes the VM-no-source case)
- 🔵 **Clarity/Testability**: Requirement 5 `reverify` unit/AC — Resolved (folded
  into AC 6)
- 🔵 **Clarity/Scope**: Summary understates scope — Resolved (broadened)
- 🔵 **Completeness**: `last_updated` stale — Resolved
- 🔵 **Dependency**: release-cut edge / `rustup` — no longer flagged

### New Issues Introduced

#### Major

- 🟡 **Clarity** (AC 3): the "~18–30% band" reproduces darwin's 50/60/70/80 only
  against 0189's *Cells* table (0205 base figures), but AC 4 quotes the
  *Validation Results* medians — two darwin referents that yield different
  ceilings. Fix: make the linux rule self-contained (smallest multiple of 10 ms
  giving ≥18% headroom over the linux measured statistic), dropping the
  darwin-reproduction claim.
- 🟡 **Testability** (AC 8): `power_state` has no defined outcome when the guest
  exposes no power source — which the item's own Assumptions (Colima VM) make
  likely — unlike AC 5's explicit not-applicable branch. Fix: add a fallback
  mirroring AC 5.
- 🟡 **Testability** (AC 3 vs AC 5): AC 3 derives C1–C4 unconditionally, but AC 5
  sends C3/C4 not-applicable when Perl is absent — no rule says which governs.
  Fix: scope AC 3's C3/C4 clause to "when the fallback farm is buildable, else
  per AC 5".

#### Minor / Suggestions

- 🔵 **Clarity**: "AC" collides with alternating current in AC 8
  ("AC/battery/charge"); "decomposing the term set" undefined; Requirements/ACs
  are unnumbered yet cross-referenced by number; Context "correct behaviour …
  resolves" phrasing.
- 🔵 **Testability**: AC 4 attribution still narrative/subjective; AC 3 floor
  basis (pre- vs post-sampling) and the "~" band boundary; AC 4's C5 interval
  disjunct is redundant under the ±10% band.
- 🔵 **Scope**: AC 6 throughput is an independently-failable gate (state the
  closure rule); x86_64 optional edge; AC 8 power-probe is separable.
- 🔵 **Dependency**: branch-7 gating cells (C3/C4) need a recorded owner
  acceptance to close (per 0189); 0215/0191 could stale the darwin baseline AC 4
  compares against.
- 🔵 **Completeness**: term-set decomposition has no AC pinning it as a
  deliverable; `status: draft`.

### Assessment

Four rounds have resolved roughly a dozen findings, every actioned one holding.
The major count is oscillating (3 → 2 → 3), not falling, because each round's
specificity fixes create fresh edge-interactions — the classic infinite-polish
dynamic of an LLM review loop against a structurally sound artefact. The three
pass-4 majors are all bounded and fixable in one focused round (self-contained
AC 3 rule; AC 8 and AC 3/AC 5 branch fallbacks), but a pass 5 would predictably
surface more. Recommendation: apply the pass-4 majors if desired, then accept —
the item has been implementation-ready in substance since pass 2.

## Approval — 2026-09-20

**Verdict: APPROVE** (owner decision, Toby Clemson).

The three pass-4 majors were applied — AC 3's ceiling rule made self-contained,
AC 3's C3/C4 clause scoped to the fallback-farm-buildable branch (else per
AC 5), and AC 8 given a VM-no-power-source fallback. With those closed, the work
item is accepted for implementation. No pass-5 verification was run: the
major-count trend (7 → 3 → 2 → 3, every actioned finding resolved) showed the
loop surfacing edge-interactions of its own fixes rather than unresolved
substance. The residual low suggestions (numbered AC anchors, the
`rustup`/branch-7 dependency notes, the 0215/0191 baseline caveat) are recorded
above for an implementer to weigh in context.
