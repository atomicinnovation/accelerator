---
type: "pr-review"
id: "163-review-1"
title: "[0310] Recover from an interrupted research conduct run"
date: "2026-10-13T23:30:14+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:163"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "compatibility", "documentation", "standards"]
review_number: 1
pr_number: 163
tags: []
last_updated: "2026-10-13T23:30:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #163 - [0310] Recover from an interrupted research conduct run

**Verdict:** COMMENT

A carefully modelled change. Classifying the replaced run's last batch is a
pure, heavily unit-tested domain function (`ReplacedBatch` / `ReplacedRun`).
Planning "as if quarantined" reuses the existing marker mechanism rather than
adding a parallel path, and quarantine only ever renames, never deletes. The
one substantive gap is durability. `--start` overwrites the replaced ledger
before the skill has done the renames, so a second interruption loses the
recovery it set up. The remaining findings are minor: a module cycle, a
call order nobody enforces, a string-suffix profile match, and a JSON contract
exception that does not fail safe.

### Cross-Cutting Themes

- **Recovery is not durable or idempotent** (flagged by: architecture,
  correctness, safety) — `--start` writes the new ledger, whose plan already
  assumes the renames, in the same call that derives `quarantine`. If the
  renames are skipped, fail, or are cut off by a second usage limit, the next
  `--start` reads the new ledger. It no longer reports or quarantines the
  original invalid findings, and they stay counted in `finding_count`.
  Carrying the outstanding quarantine stems in `RunMemory` until the files are
  gone would make a rerun of `conduct` converge.
- **`pair_ending` matches the profile by string suffix** (flagged by:
  architecture, code-quality, correctness, safety, test-coverage) — it puts
  the stem's text layout outside `Stem`. It also takes the first profile in
  outline order, so with `web` and `deep-web` a correct `deep-web` finding is
  judged against `web` and quarantined. No test covers overlapping profile
  names.
- **`ledger_wording` ↔ `topic_command` cycle** (flagged by: architecture,
  code-quality, standards) — the other `*_wording` modules are leaves, and
  `ReplacedLedger` was made `pub` only so this import compiles.
- **The `ReplacedBatch` calls must run in an unenforced order** (flagged by:
  architecture, code-quality) — `judging_findings` → `quarantining` →
  `RoundPlan::of` → `report` must all use the quarantined inputs. Mixing in
  the raw inputs still compiles and quietly misclassifies nodes. This is now
  public API.
- **The additive-only JSON contract gains a field consumers must act on**
  (flagged by: compatibility, architecture, standards) — a consumer that obeys
  "ignore unknown fields" gets a plan whose premise is false. The stakes are
  low while the CLI and `SKILL.md` ship together and the surface is
  unreleased, but the contract no longer fails safe.

### Tradeoff Analysis

- **CLI never mutates files vs durable recovery**: keeping the renames in the
  skill preserves the read-only CLI boundary, which the PR states explicitly.
  The cost is that the plan's correctness now depends on an LLM following a
  multi-step procedure, and the CLI never confirms it was followed. A small
  `research topic quarantine` verb, or a ledger-carried obligation that the
  next `--run` checks, keeps the boundary mostly intact and closes the
  durability gap. Recommend the ledger-carried option: it keeps the CLI
  read-only.

### Strengths

- ✅ Classification is a pure function of the ledger, the inputs and the plan.
  About 30 focused unit tests cover every class: valid, invalid, unwritten,
  unasked, refused, plan-rejected, a misdirected tree, a rootless tree and
  normalisation-only matches.
- ✅ Planning "as if quarantined" goes through
  `RoundInputs::with_findings_quarantined` plus `Finding::marker_claim`, and
  `a_misdirected_finding_read_as_quarantined_plans_as_its_marker_would` checks
  that equivalence against the whole plan.
- ✅ Quarantine is rename-only and uses the unique-suffix rule. The skill
  stops `conduct` when a rename fails and lists what is still unquarantined,
  so any misclassification can be recovered by hand.
- ✅ Every user-facing surface drops the "safe to delete" guidance for
  `.conduct-run.json`, which stops users deleting the one record recovery
  depends on.
- ✅ `OutlineItem::stem_at` removes duplicated slug/stem allocation from the
  planner. `ReplacedLedger` separates reading the ledger from wording it.
  `start_notices` flattens `run_outstanding`.
- ✅ The tree-outranks-marker precedence in `claim_index` is visible in the
  `or_else` chain, its comment explains why, and new tests pin the
  lowest-index tie-break for each kind.
- ✅ The ledger's on-disk format is unchanged. Literal fixtures, with and
  without fields the reader does not know, show that ledgers from earlier
  prereleases are still read.

### General Findings

- 🔵 **Architecture**: Recovery responsibility is split across CLI planning
  and renames in the skill's prose; consider a small CLI verb for the mutation.
- 🔵 **Correctness**: Between the CLI judging a finding and the skill renaming
  it, an agent still in flight from the replaced run can rewrite the finding
  validly, which would quarantine a valid finding. Re-check the file before
  renaming, or document this window beside the existing note on in-flight
  writes.
- 🔵 **Compatibility**: The index-claim precedence change alters the stems
  planned for prerelease sets holding both a lower marker and a higher tree.
  This is intended, but the CHANGELOG does not say so.
- 🔵 **Documentation**: The CHANGELOG omits that a caller of `--start` other
  than `conduct` must rename the `quarantine` paths itself.

### Additional Findings

- 🔵 `docs-site/src/content/docs/research.md:518` — stderr line only asks for
  renames when `quarantine` is non-empty (documentation)
- 🔵 `cli/research/src/conduct/replaced_run.rs:249` — `owned_finding` and
  `unowned_finding` return different shapes (code-quality)
- 🔵 `cli/research-cli/src/ledger_wording.rs:30` — "a hand run" in the
  replaced notice is unclear (code-quality)
- 🔵 `cli/research/src/conduct/replaced_run.rs:40` — one concept goes by four
  names: replaced, unfinished, did not end, interrupted (standards)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/research-cli/src/topic_command.rs:284-294` — Replaced ledger is overwritten before the quarantine renames land (Architecture, Correctness, Safety)
**Severity**: major | **Confidence**: medium | **Lens**: architecture, correctness, safety

🟡 **Architecture / Correctness / Safety**

This call derives `replaced_run.quarantine` from the old ledger. It then writes
the new run's ledger (`RunLedger::start` + `record`), whose plan and claims
already assume the renames. From then on, the obligation to quarantine those
findings lives only in the transient `--start` JSON.

Suppose the skill does not complete the renames: a second usage limit (the
exact scenario this PR targets), a closed terminal, a failed rename that stops
`conduct`, or a model that skips the step. The next `--start` then reads the
*new* ledger, whose last batch holds the re-offered stems at fresh indexes.
The original invalid findings are never judged or reported again, stay on
disk, and stay counted in `finding_count`. A misdirected `02-b-web.md`
answering `A?` would also sit beside the fresh `02-a-web`.

**Suggestion**: Make the outstanding quarantine durable. For example, carry
the unrenamed `quarantine` stems in the new ledger's `RunMemory`, keep
re-emitting them (or fail the next `--run`) while any still exists, and drop
them once they are gone. Add a test that runs `--start` twice without the
renames and asserts that the second run still reports and quarantines the
original paths.

---

### `cli/research/src/topic/outline.rs:111-117` — `pair_ending` matches the first profile by string suffix (Correctness, Safety, Test Coverage, Architecture, Code Quality)
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, safety, test-coverage, architecture, code-quality

🔵 **Correctness / Safety / Test Coverage**

`find(|profile| text.ends_with(&format!("-{profile}")))` returns the first
profile in outline order. `Stem::admits_profile` allows hyphens, so with
`profiles: web, deep-web` the stem `01-a-deep-web` matches `web`. On this
reworded-question fallback, `owned_finding` then judges a correct
`(A, deep-web)` finding against `(A, web)`, classes it `invalid` and
quarantines it. Existing tests use only `web`/`openalex`/`arxiv`, so nothing
pins the result.

🔵 **Architecture / Code Quality**

The match also re-parses the stem's `<index>-<slug>-<profile>` layout outside
`Stem`, which owns that format (`parse`, `allocated`, `admits_profile`).

**Suggestion**: Add a `Stem` query (e.g. `stem.ends_with_profile(profile)`)
and prefer the longest matching profile, or parse the profile segment off
unambiguously. Add a test case with overlapping profiles.

---

### `cli/research-cli/src/ledger_wording.rs:6` — `ledger_wording` and `topic_command` depend on each other (Architecture, Code Quality, Standards)
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality, standards

🔵 **Architecture / Code Quality / Standards**

`ledger_wording` imports `crate::topic_command::ReplacedLedger`, and
`topic_command` imports `ledger_wording`. The sibling `concurrency_wording`
and `plan_wording` depend only on domain types or leaf modules, never on the
command that calls them. `ReplacedLedger` was also made `pub`, and sits among
private helpers mid-file, only so that this import works.

**Suggestion**: Make `corrupt_notice` parameterless and keep the match on
`ReplacedLedger` in `topic_command`. Alternatively, move `ReplacedLedger`
into its own module, or beside `RunLedger` in the `research` crate.

---

### `cli/research/src/conduct/replaced_run.rs:103-125` — The `ReplacedBatch` call order is enforced only by convention (Architecture, Code Quality)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

Callers must run `judging_findings(inputs)`, then `quarantining(inputs)`,
then `RoundPlan::of(quarantined)`, then `report(quarantined, plan)`. Passing
the raw inputs, or a plan drawn from them, to `report` still compiles and
quietly misclassifies nodes. The sequence is repeated in `run_outstanding`
and in this file's test `report` helper, and it is now public API (see
`public-api.txt`).

**Suggestion**: Make the sequence type-driven. One option: `quarantining`
consumes the batch and returns a `QuarantinedRound` that alone draws the plan
and the report. Another: a single `ReplacedBatch::recover(...) -> (RoundInputs,
RoundPlan, ReplacedRun)` that both `run_outstanding` and the tests call.

---

### `docs-site/src/content/docs/research.md:470-473` — The additive-only contract now has a field consumers must not ignore (Compatibility, Architecture, Standards)
**Severity**: minor | **Confidence**: medium | **Lens**: compatibility, architecture, standards

🔵 **Compatibility / Architecture / Standards**

The contract promises that "consumers ignore fields they do not know", and
then makes an exception for `replaced_run.quarantine`. A consumer that obeys
the general rule gets a plan drawn against a filesystem that does not exist:
fresh indexes are allocated while the invalid findings stay on disk and stay
counted. The exception is documented, but the contract no longer fails safe.
The same change also removes the readable-ledger entry from `warnings`, and
the CHANGELOG does not mention that.

**Suggestion**: The surface is unreleased, so this is cheap to fix now.
Either keep the plan valid when the field is ignored (see the durability
comment on `topic_command.rs`), or name the exception as an explicit rule.
Say what goes wrong if the paths are not renamed, and list the `warnings`
change in the CHANGELOG.

---

### `cli/research-cli/tests/topic_outstanding.rs:1227-1235` — Plan equivalence does not hold for a schema-invalid finding that names its question (Test Coverage)
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

This test covers only a misdirected finding, and compares only `pairs`. For a
schema-invalid finding, `Finding::invalid` has no `answer`, so its
`marker_claim` is `IndexClaim(n, None)`. The on-disk marker reader in
`research-adapters/src/topic.rs` reads the marker's frontmatter `question` and
yields `IndexClaim(n, Some("A?"))`. So "plans as if already renamed" differs
from the plan after the rename for that class. Within a run, the ledger's own
claims hide the difference.

**Suggestion**: Parameterise the test over a misdirected finding and a
schema-invalid finding whose frontmatter still names its question. Compare
`items` as well as `pairs`, including a plan drawn without ledger claims.
Then either have `marker_claim` read the stray question, or narrow the
documented claim.

---

### `CONTRIBUTING.md:27-32` — "Every unverified launch is logged" overstates what is logged (Documentation)
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

`bin/accelerator` writes `.accelerator-unverified.log` only for the
dev-launcher exec (`ACCELERATOR_LAUNCHER_BIN`) and for integrity failures.
`ACCELERATOR_<SUB>_BIN` overrides, handled in `override_path_from`, emit only
a `tracing::info!` event. The log also lives under `ACCELERATOR_CACHE_DIR`
when that is set. A contributor checking the log to confirm that their
`ACCELERATOR_RESEARCH_BIN` build ran will find no entry.

**Suggestion**: Narrow the claim to launcher launches and the cache
directory, or make sub-binary overrides write to the same log.

---

### `docs-site/src/content/docs/research.md:513-517` — `replaced_run` entries can carry `null` `question`/`profile` (Documentation)
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

`unowned_finding` returns `invalid` entries with no pair, and `unasked`
entries with no pair when nothing was written, which `topic_command.rs`
renders as `null`. `SKILL.md` handles this case, but this CLI reference does
not mention it, so a script written from it may assume both fields are
always strings.

**Suggestion**: Append "…; both are `null` when no outline pair owns the stem
and the finding names none".

---

### `cli/research-cli/src/topic_command.rs:558-563` — `PlanOutput::replaced_run` method shares its name with the field (Code Quality)
**Severity**: suggestion | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

`rendered["replaced_run"] = self.replaced_run(replaced)` sits inside
`if let Some(replaced) = self.replaced_run`, so the same identifier names both
a field and a rendering method. The other renderers (`pair`, `absolute`) do
not shadow fields.

**Suggestion**: Rename the method to describe what it renders, e.g.
`replaced_run_json`.

---

### `cli/research-cli/src/topic_command.rs:261-263` — A readable replaced ledger no longer produces a `warnings` entry (Compatibility)
**Severity**: suggestion | **Confidence**: medium | **Lens**: compatibility

🔵 **Compatibility**

Before this change, `--start` over a readable ledger added a `warnings`
entry. Now only a corrupt ledger does, so a consumer that watched `warnings`
to detect a replaced run loses that signal without a deprecation step. The
risk is low because `SKILL.md` changes in lockstep and the surface is
unreleased.

**Suggestion**: Record the removal in the CHANGELOG entry, or keep a short
`warnings` entry alongside `replaced_run` for one release.

---

## Per-Lens Results

### Architecture

**Summary**: The change fits the functional-core / imperative-shell split.
Classification is pure domain logic in `research::conduct::replaced_run`,
quarantine is a transformation of `RoundInputs`, and all I/O stays in
`topic_command`. The main weakness is durability: the obligation to
quarantine travels only in transient `--start` JSON, and the ledger holding
the evidence is overwritten in that same call. Smaller issues: a module cycle,
a three-step protocol whose order nothing enforces, and stem layout leaking
into `OutlineItem`.

**Strengths**: Classification is a pure function with about 30 tests;
quarantine planning reuses the marker mechanism; the CLI keeps its no-rename
boundary; `stem_at` removes duplication; the domain names are rich.

**Comments**:
- `cli/research-cli/src/topic_command.rs:284-297` — major/medium — Recovery
  is not durable: the replaced ledger is overwritten before the quarantine
  obligation is discharged.
- `cli/research-cli/src/ledger_wording.rs:6` — minor/high — Circular module
  dependency between `ledger_wording` and `topic_command`.
- `cli/research/src/conduct/replaced_run.rs:103-125` — minor/medium — The
  three-step `ReplacedBatch` protocol has an order nothing enforces.
- `cli/research/src/topic/outline.rs:111-117` — minor/medium — `pair_ending`
  parses the stem's string form.
- `docs-site/src/content/docs/research.md:471-473` — minor/medium — The
  additive-only contract carries a field consumers must act on.

**General Findings**:
- suggestion — Recovery responsibility is split across CLI planning and
  renames in the skill's prose; consider a `research topic quarantine` verb.

### Code Quality

**Summary**: Clear domain types and small, well-named functions; the private
`Pending`/`Verdict` states keep the two-phase judging tidy. Moderate concerns:
a module cycle, the call order callers must replay, and a stringly-typed
profile match.

**Strengths**: Domain naming; `stem_at` extraction; the `ReplacedLedger` enum
separates reading from wording; `start_notices` flattens `run_outstanding`;
the `claim_index` precedence is visible; `Pending::Judged`/`AwaitingPlan`
makes the phasing explicit.

**Comments**:
- `cli/research-cli/src/ledger_wording.rs:6` — minor/high — Cyclic dependency.
- `cli/research-cli/src/topic_command.rs:284-294` — minor/medium — Temporal
  coupling across judge, quarantine, plan and report.
- `cli/research/src/topic/outline.rs:111-117` — minor/medium — `pair_ending`
  matches by string suffix rather than through `Stem`.
- `cli/research-cli/src/topic_command.rs:558-563` — suggestion/high — The
  method shares its name with the field.
- `cli/research/src/conduct/replaced_run.rs:249-278` — suggestion/low —
  `owned_finding`/`unowned_finding` return different shapes.
- `cli/research-cli/src/ledger_wording.rs:30-34` — suggestion/low — "a hand
  run" is unclear.

**General Findings**: none.

### Test Coverage

**Summary**: Thorough tests, clearly written test-first. About 30 unit tests
cover every outcome class. Integration tests pin the JSON contract, the
stderr ordering, the corrupt/absent ledger cases and forward compatibility.
Two narrow gaps: plan equivalence for schema-invalid findings, and
overlapping profile suffixes.

**Strengths**: Every verdict path is tested; whole-plan equivalence is checked
for a misdirected finding; renames are simulated across runs; literal ledger
fixtures; an explicit regression assertion on the removed warning; the stderr
order is pinned; a real-NFKC integration test.

**Comments**:
- `cli/research-cli/tests/topic_outstanding.rs:1227-1235` — minor/medium —
  Plan equivalence is tested only for a misdirected finding (confirmed by the
  orchestrator: the in-memory `marker_claim` carries `None` where the on-disk
  marker reader carries the frontmatter question).
- `cli/research/src/topic/outline.rs:111` — minor/medium — `pair_ending` is
  untested with hyphen-suffix profiles.

**General Findings**: none.

### Correctness

**Summary**: The classification logic is sound and well bounded. Quarantine is
applied before `RoundPlan::of` and `Observed::of`, so the new ledger matches
the disk once the renames land. The main gap is that recovery cannot be
repeated after a second interruption.

**Strengths**: Plan/claims/seen consistency; validity means "answers its
pair"; ownership comes from the replaced ledger's own claims; every ledger
state is explicit; the tree-before-marker change is correct and tested.

**Comments**:
- `cli/research-cli/src/topic_command.rs:284-294` — major/medium — The
  replaced ledger is overwritten before the renames are confirmed.
- `cli/research/src/topic/outline.rs:111-117` — minor/low — `pair_ending`
  picks the first profile, not the longest.

**General Findings**:
- suggestion — Between judging and renaming, an agent still in flight from
  the replaced run can rewrite an invalid finding validly.

### Safety

**Summary**: Careful. Recovery only renames, uses unique suffixes, and stops
on a failed rename. The residual risks are recoverable: renames that are
never checked, and the profile-suffix edge case.

**Strengths**: Rename-only quarantine; fails safe on a partial rename; a
corrupt ledger never yields a quarantine list; "safe to delete" removed; the
report is printed before any mutation; unasked findings are left in place.

**Comments**:
- `cli/research-cli/src/topic_command.rs:284-294` — minor/medium — The start
  plan assumes renames that the CLI never checks were done.
- `cli/research/src/topic/outline.rs:111-117` — minor/low — The suffix
  fallback can pick the wrong profile and quarantine a valid finding.

**General Findings**: none.

### Compatibility

**Summary**: The CLI and `SKILL.md` ship together and the surface is
unreleased, so the bundled consumer cannot break. The ledger format is
unchanged. The one real concern is the exception to the additive-only
contract.

**Strengths**: The ledger format is untouched; `replaced_run` only appears on
`--start`; the corrupt case keeps its `warnings` entry; hand runs still get a
stderr line; the public-API fixture is updated; the `CONTRIBUTING` override
naming matches the launcher.

**Comments**:
- `docs-site/src/content/docs/research.md:471-473` — minor/medium — Ignoring
  `replaced_run` breaks the additive-only contract.
- `cli/research-cli/src/topic_command.rs:261-263` — suggestion/medium — A
  readable replaced ledger no longer produces a `warnings` entry.

**General Findings**:
- suggestion — The index-claim precedence change alters the stems planned for
  existing prerelease sets; mention it in the CHANGELOG.

### Documentation

**Summary**: The user-facing docs are updated consistently, and the new
comments are sparse and explain why. There are three inaccuracies:
`CONTRIBUTING` overstates logging, the docs omit the `null`
`question`/`profile` case, and the stderr wording is described too broadly.

**Strengths**: "Safe to delete" is corrected everywhere; the contract
exception is stated openly; the stderr/`warnings` split matches the code; the
`SKILL.md` report is precisely specified and pinned by structure tests; the
doc comments explain intent.

**Comments**:
- `CONTRIBUTING.md:27-32` — minor/high — "Every unverified launch is logged"
  overstates what is logged.
- `docs-site/src/content/docs/research.md:513-517` — minor/high — `null`
  `question`/`profile` is undocumented.
- `docs-site/src/content/docs/research.md:518-521` — suggestion/medium — The
  stderr line only asks for renames when `quarantine` is non-empty.

**General Findings**:
- suggestion — The CHANGELOG omits the rename obligation for callers of
  `--start` other than `conduct`.

### Standards

**Summary**: Closely follows the conventions: the `*_wording` pattern,
`pub(crate)` helpers, behaviour-sentence test names, 80 columns, and comment
discipline. Three small consistency issues.

**Strengths**: Follows the wording-module pattern; the public API grows only
deliberately; gerund constructors match `Schedule::starting`; comments explain
why; `start_notices` is extracted; the CHANGELOG and docs are updated
together.

**Comments**:
- `cli/research-cli/src/ledger_wording.rs:6` — minor/medium — Mutual
  dependency; `ReplacedLedger` is placed mid-file.
- `cli/research/src/conduct/replaced_run.rs:40` — suggestion/low — One
  concept goes by four names across code, JSON, `SKILL.md` and docs.
- `docs-site/src/content/docs/research.md:470-472` — suggestion/medium — The
  additive-only contract gains an exception consumers must act on.

**General Findings**: none.
