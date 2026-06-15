---
type: plan-review
id: "2026-06-15-0047-core-skills-sync-integration-review-1"
title: "Plan Review: Core Skills Sync Integration"
date: "2026-06-15T22:56:45+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-15-0047-core-skills-sync-integration"
target: "plan:2026-06-15-0047-core-skills-sync-integration"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, safety, standards, compatibility, usability]
review_number: 1
review_pass: 3
tags: [work-management, integrations, sync, list-work-items, create-work-item]
last_updated: "2026-06-16T07:20:17+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Core Skills Sync Integration

**Verdict:** REVISE

The plan is architecturally disciplined and unusually well-grounded: it pins the
central `external_id`/presence-based-classification decision up front, establishes
a clean content-in/identifier-out contract with caller-side writeback, introduces
a dispatcher seam that decouples `/create-work-item` from per-tracker scripts, and
preserves the extensibility slot 0051 needs. However, three independent themes
converge on the same real-world hazard — **duplicate remote issues** — and two
foundational concerns (whether the test net actually runs in CI, and whether the
ANSI colour mechanism renders at all on the target surface) undercut the plan's
own definition of done. These are correctable with targeted edits, not a
rewrite, but they should be resolved before implementation.

### Cross-Cutting Themes

- **Non-idempotent create + retry → duplicate remote issues** (flagged by:
  correctness, safety, architecture, compatibility) — Four lenses independently
  converge here. The new retry-once loop (Phase 4 §3) retries a non-idempotent
  remote create without distinguishing a pre-create transport failure (safe to
  retry) from a post-create response/validation failure (retry creates a
  duplicate). The same hazard is reached two other ways: a Write failure *after*
  a successful create strands a remote issue with no local linkage, and items
  already pushed under the old `work_item_id` flow bypass the (now presence-based)
  already-synced guard and get re-created. The plan inherits the Linear
  precedent's explicit loud-fail, no-auto-retry stance — and then quietly departs
  from it.

- **ANSI colour on a model-rendered markdown surface** (flagged by: usability,
  standards, code-quality) — Three lenses flag that `/list-work-items` output is
  emitted by the model as a markdown table into the Claude Code conversation, not
  streamed to a TTY. Raw `\033[32m` escapes will likely render as literal
  gibberish or be stripped. The codebase has zero ANSI precedent; the only
  terminal-awareness idiom (migrate's TTY-detect) is not adopted. The
  distinct-*text* requirement already carries the signal; colour needs a
  surface-appropriate mechanism or must be confirmed against the real surface.

- **The test net may not actually gate** (flagged by: test-coverage) — The named
  Linear/Jira/work suites are not wired into any `mise`/CI task, new suites land
  in the same unwired subtrees, and shell-suite discovery silently skips files
  without the exec bit. As written, `mise run check` (the plan's stated "done")
  runs almost none of the plan's regression net.

- **Dispatcher contract + prose state machine under-specified** (flagged by:
  architecture, correctness, code-quality, standards, test-coverage) — The
  dispatcher wraps two heterogeneous scripts (bare-identifier vs `{id,key,self}`
  JSON; overlapping 100-band exit codes meaning different things) without a
  normalised output/exit-code taxonomy, and the five-outcome retry/fallback state
  machine lives entirely in SKILL.md prose with no scripted test. The
  retry-vs-fallback safety decision (theme 1) depends on exactly the exit-code
  distinction the dispatcher does not yet define.

### Tradeoff Analysis

- **Presence-based classification (read side) vs. trust on the write side**: The
  presence-based rule is correct and robust for *classification* in
  `/list-work-items`, but reused unchanged as the Linear *already-synced write
  guard* it now trusts any non-empty `external_id` (incl. hand-typed/malformed/
  quoted-empty) to mean "a real remote issue exists." Recommendation: keep the
  read-side classifier purely presence-based, but normalise (strip quotes/
  whitespace) and optionally sanity-check on the write-side guard — the two
  consumers have different trust requirements.

- **Defer migration of legacy `work_item_id` (0064) vs. ship a transitional
  guard now**: Fully deferring is cleaner scope-wise, but the guard regression
  and the misclassification both take effect the moment Phase 1 lands, before any
  migration exists, and both lead to duplicate-push risk. Recommendation: add a
  transitional read of remote-format `work_item_id` in the guard/classifier until
  0064 ships, or pull the one-line migration into scope.

### Findings

#### Critical

- 🔴 **Correctness / Safety / Architecture**: Retry-once on push failure can
  create duplicate remote issues
  **Location**: Phase 4, Section 3: Push state machine + defer-write
  The retry loop retries a non-idempotent create without constraining *which*
  failures are safe. A failure surfaced after the remote actually created the
  issue (timeout/parse-error after `issueCreate`) produces a second issue on
  retry — the exact mode the Linear precedent's loud no-retry stance prevents.

- 🔴 **Correctness / Safety**: Write-failure-after-create strands a remote issue
  with no local linkage
  **Location**: Phase 4, Section 3: defer-write success branch
  The "no file until resolve" invariant has no defined behaviour if the single
  Write fails *after* a successful create returned a validated identifier. The
  remote issue exists, the key is known, but nothing is on disk — and a naive
  re-run duplicates it. The file-first path surfaces `E_CREATE_WRITEBACK_FAILED`
  loudly here; this path is silent.

- 🔴 **Test Coverage**: Named shell suites are not wired into the CI gate
  **Location**: Testing Strategy; Phase 1/2/4 Automated Verification
  `test-linear-create.sh`, `test-jira-create.sh`, `test-work-item-scripts.sh`,
  and the two new suites live in subtrees that `test:integration` does not run.
  `mise run check` (the stated definition of done) executes none of them, so the
  bulk of the regression net only runs when a human runs each `bash` line by hand.

- 🔴 **Test Coverage**: Tree-fence change breaks a wired byte-for-byte equality
  test
  **Location**: Phase 3, Change 4: Tree fence fixture
  `test-hierarchy-format.sh` (in the wired `scripts` subtree) asserts the
  `canonical-tree-fence` block is byte-identical between `list-work-items` and
  `refine-work-item` (which has no integration gate). Appending a label to one
  fence breaks this test, and a static prose example cannot be simultaneously
  integration-on, integration-off, and identical across two skills. The plan also
  misidentifies the test.

#### Major

- 🟡 **Architecture**: Dispatcher error-surface contract across two heterogeneous
  integrations is under-specified
  **Location**: Phase 4, Sections 1 & 3
  Linear (bare identifier, 100–109 band) and Jira (`{id,key,self}` JSON,
  overlapping 100–107 band) are normalised into one dispatcher contract that is
  never defined. Without it the caller re-couples to tracker-specific codes — the
  very thing the dispatcher abstracts — and the retry/fallback decision can't be
  made safely.

- 🟡 **ANSI rendering** (flagged by: usability, standards, code-quality): raw
  ANSI escapes will not render as colour on a model-emitted markdown table
  **Location**: Phase 3, Section 3 / Desired End State
  Output reaches the Claude Code conversation as markdown, not a TTY. Escapes
  likely show as literal `[32m…[0m` gibberish. Zero ANSI precedent in the
  codebase; migrate's TTY-detect idiom not adopted. Also raised as a code-quality
  concern: escape literals scattered across two render surfaces (table + fence)
  with no single owner.

- 🟡 **Correctness**: Phase 3 contradicts itself on how `external_id` is read
  **Location**: Phase 3, Section 2 vs. Performance Considerations
  §2 says read via `work-item-read-field.sh external_id` (exit 1 when absent, no
  bridge); Performance Considerations says it's parsed from the same `awk` pass
  (empty-string when absent). Different absent-field semantics → misbranching risk
  and a contradicted "no extra per-file spawn" claim.

- 🟡 **Compatibility / Safety**: Items pushed under old `work_item_id` bypass the
  already-synced guard → duplicate creation; and render as unsynced
  **Location**: Migration Notes; Phase 1 §2; Phase 3 §2
  The guard regression and the false-negative classification both take effect when
  Phase 1 lands, before the deferred 0064 migration exists. Combined, they invite
  a re-push that duplicates an already-synced remote issue.

- 🟡 **Correctness**: Already-synced guard's "non-empty" boundary is imprecise
  (quoted-empty / whitespace)
  **Location**: Phase 1, Section 2
  The current guard trims surrounding quotes/whitespace; the plan doesn't state
  the new `external_id` presence check does. `external_id: ""` or a quote-only
  value reads as non-empty under a naive `[[ -n ]]` and wrongly classifies.

- 🟡 **Code Quality / Test Coverage**: The five-outcome retry/fallback state
  machine lives entirely in unverifiable prose
  **Location**: Phase 4, Section 3
  Net-new branching (accept→success/retry/fallback; decline; unbuilt) with the
  highest-risk ACs (write-once, no-partial-file, retry-then-fallback) has no
  scripted regression test — only the dispatcher beneath it is tested, and no eval
  case is named.

- 🟡 **Standards**: New Jira exit codes are not pinned to the documented namespace
  **Location**: Phase 2, Section 3
  Jira already owns the full 100–107 band (107 = `E_CREATE_BAD_ASSIGNEE`), only
  108–109 reserved. "Document any new resolver exit codes" doesn't assign them,
  risking collision with the per-integration namespace discipline.

- 🟡 **Test Coverage**: Linear no-file create-and-return mode is uncovered
  **Location**: Phase 1, Change 2; Phase 4, Change 4
  The mode the dispatcher actually calls is not asserted to print a bare
  identifier, make the GraphQL call, and perform *no* file I/O. Existing cases are
  all file-first; the defer-write invariant could break only in real use.

- 🟡 **Test Coverage**: New suites must carry the exec bit or CI silently skips
  them
  **Location**: Phase 4, Change 4
  Shell-suite discovery requires `**/test-*.sh` AND `os.X_OK`. A suite committed
  without `chmod +x` vanishes from CI with no failure — false green, compounding
  the wiring gap.

- 🟡 **Usability**: Retry / "run /sync-work-items later" messaging is
  under-specified for error actionability
  **Location**: Phase 4, Section 3
  The prompt/message text is never drafted; the dispatcher surfaces a generic
  error. A user can't tell transient-vs-config failure, and `/sync-work-items`
  (0051) doesn't exist yet — telling users to run an unbuilt command is a dead end.

#### Minor

- 🔵 **Architecture / Code Quality**: kind→type and project resolution risks
  duplication across the user-facing Jira mode and the dispatcher's Jira branch;
  make the read-only resolver script non-optional with a single source of truth.
  **Location**: Phase 2 §1–2 / Phase 4 §1

- 🔵 **Correctness / Safety**: `config_upsert_frontmatter_field` insert branch
  funnels *every* non-zero exit (no-frontmatter/unclosed/duplicate, not just
  absent) into insert; must fail closed on malformed input and run the same
  integrity re-check as the replace path.
  **Location**: Phase 1, Section 1

- 🔵 **Architecture**: Already-synced guard weakens from format-validation to mere
  presence — a write-side trust shift worth acknowledging.
  **Location**: Phase 1, Section 2

- 🔵 **Correctness / Usability**: Jira `--project` may be unresolvable on the push
  path (bare-numeric `id`, unset `default_project_code`); resolve/validate *before*
  the push offer as a non-retryable decline-equivalent outcome.
  **Location**: Phase 4 §1 / Phase 2 §1

- 🔵 **Safety / Standards**: Dispatcher should fail closed on an unrecognised/empty
  `--integration` value (refuse, distinct code), with a test.
  **Location**: Phase 4, Section 1

- 🔵 **Standards / Compatibility**: Linear `EXIT_CODES.md` is *derived* — the
  `readonly E_*=NN` constants and adjacent comments in the script are the source of
  truth (with a greppable equality check); update both, plus the 100–109
  range-summary note, not just the table descriptions.
  **Location**: Phase 1, Section 4

- 🔵 **Standards**: `create-linear-issue` line `:10` is inside the YAML
  `description` frontmatter — reword the discovery-bearing description, not just
  body refs.
  **Location**: Phase 1, Section 3

- 🔵 **Standards**: The dispatcher's own exit-code namespace (not-available +
  pass-through policy) is undocumented; give it an EXIT_CODES contract consistent
  with the per-area convention.
  **Location**: Phase 4, Section 1

- 🔵 **Test Coverage**: Jira already-synced refusal and end-to-end `external_id`
  writeback are only covered indirectly via the shared helper; add explicit Jira
  cases.
  **Location**: Phase 2, Change 1

- 🔵 **Test Coverage**: Existing Linear cases (3, 5, the byte-identical-remainder
  case) and the EXIT_CODES equality check must be re-pointed `work_item_id` →
  `external_id`; list them so the assertion migration isn't implicit.
  **Location**: Phase 1, Change 4

- 🔵 **Usability**: "Not available" message for trello/github-issues needs concrete,
  non-dead-end wording (cite 0049/0050, reassure local save).
  **Location**: Phase 4, Section 1

- 🔵 **Usability**: Jira silent default-to-Task on unknown kind and silent project
  source should be surfaced in the preview.
  **Location**: Phase 2 & 4

- 🔵 **Usability**: The push offer adds a second consecutive confirmation to a path
  that had none; state both outcomes inline so decline-still-saves is clear.
  **Location**: Phase 4, Section 2

#### Suggestions

- 🔵 **Architecture / Code Quality**: Make the optional companion ADR
  (id / external_id / retired-`work_item_id`-remote-key model) non-optional so the
  deferred 0064 canonicalisation has a stable reference point.
  **Location**: References

- 🔵 **Code Quality**: State the integration gate once as a named convention
  ("integration-configured := non-empty output of `config-read-work.sh
  integration`") referenced from each phase, rather than re-deriving it at three
  call sites.
  **Location**: Phase 3 §1 / Phase 4 §2

- 🔵 **Code Quality**: Define the create-and-return contract as "prints a bare
  validated identifier on stdout" for *every* integration, pushing the Jira `.key`
  extraction down so the dispatcher routes uniformly.
  **Location**: Phase 1 §2 / Phase 4 §1

- 🔵 **Usability**: Signpost the relationship between `/create-work-item` push and
  the standalone `/create-{jira,linear}-issue` skills; name the concrete recovery
  skill in the decline/fallback messaging.
  **Location**: Desired End State / Phase 4

### Strengths

- ✅ Decision discipline: the `external_id` vs `work_item_id` vs `id` collision is
  resolved up front in the Overview, so downstream readers never reverse-engineer
  which key means what.
- ✅ Clean contract boundary: integration primitive = content-in/validated-
  identifier-out, no writeback; user-facing skill = file-driven, caller-side
  `external_id` writeback — removes the file-lifecycle coupling baked into today's
  `linear-create-flow.sh`.
- ✅ The dispatcher is an open-closed extension seam: adding Trello/GitHub create
  paths is a modification of the dispatcher, not the calling skill.
- ✅ Defer-write-once eliminates partial-frontmatter mutation on the happy path and
  sidesteps the replace-only limitation of `config_set_frontmatter_field`.
- ✅ Phase independence is well-reasoned (3 independent; 1/2 mutually independent;
  4 layers on 1+2), each green on its own, limiting blast radius.
- ✅ The `status → {label, colour}` lookup is a genuine data-driven seam 0051 can
  extend without a call-site edit.
- ✅ `config_upsert_frontmatter_field` is composed, not duplicated — delegates to
  the replace path and adds only the insert branch, preserving injection-safety.
- ✅ `atomic_write` + post-write integrity re-check carried forward; presence-based
  rule verified against the `PROJ-0042` misclassification it replaces.
- ✅ Fail-safe defaults respected: unconfigured `work.integration` reproduces
  today's behaviour exactly; the strict `y`/`Y`-only gate is reused.
- ✅ Migration is no-op and conservative: items without `external_id` render as
  unsynced (never falsely "synced").
- ✅ The distinct-*text*-and-colour requirement is asserted, protecting the
  no-colour / colour-blind case.

### Recommended Changes

1. **Make the retry idempotency-safe** (addresses: retry duplicate;
   write-failure-after-create; guard bypass) — Constrain auto-retry to failure
   classes that provably occurred *before* the remote mutation (arg/auth/connect-
   refused). Route any at-or-after-`issueCreate` failure — and any Write failure
   after a successful create — to the existing loud non-idempotent guidance (print
   the identifier; "may have been created, run sync / set `external_id` by hand;
   do not blindly re-run"). This requires the dispatcher to distinguish pre-create
   from post-create failures via distinct exit codes.

2. **Define the dispatcher's normalised contract** (addresses: dispatcher error
   surface; state machine; Jira namespace) — Specify one stdout convention (bare
   identifier for every tracker) and a dispatcher-owned exit-code taxonomy
   (retryable-transport / terminal / not-available / unrecognised) that maps each
   integration's native codes into it, documented in an EXIT_CODES file under
   `skills/work/scripts/`. Fail closed on unrecognised `--integration`.

3. **Resolve the ANSI/rendering-surface question** (addresses: ANSI won't render)
   — Confirm whether `/list-work-items` output reaches a colour-capable TTY. If
   not (markdown chat), use a markdown-native distinction (emoji/badge + distinct
   label text) and keep the lookup's style field surface-appropriate; if a TTY is
   the target, adopt the migrate TTY-detect idiom and document the new convention.
   Either way, prefer a single helper owning the `status → {label, style}` table
   over escape literals in two prose render surfaces.

4. **Wire the tests into CI and prove they run** (addresses: unwired suites;
   exec bit; tree fence) — Add a Phase 0 step registering `test.integration` tasks
   for `skills/work` and `skills/integrations` (+ `mise.toml` rollup entries),
   `chmod +x` every new `test-*.sh` with a count-floor guard, and verify the suites
   appear in `mise run test:integration` output. Resolve the
   `canonical-tree-fence` coupling explicitly: decide whether the label belongs in
   the shared fence at all; if so change both SKILL.md fences in lockstep (or
   scope the equality test), and name `test-hierarchy-format.sh` + the
   refine-work-item coupling in the success criterion.

5. **Pick one `external_id` reader and define absent-field semantics**
   (addresses: Phase 3 contradiction; quoted-empty boundary) — Either parse from
   the existing `awk` pass (empty/absent → unsynced) or mandate
   `work-item-read-field.sh` (exit 1 → unsynced, accept the per-file spawn) — not
   both. Define "non-empty `external_id`" precisely as "length > 0 after stripping
   surrounding quotes and whitespace" and apply it identically in the Linear guard,
   the Jira guard, and the list classifier.

6. **Close the legacy `work_item_id` gap or ship the migration** (addresses: guard
   bypass; unsynced false-negative) — Either add a transitional read of
   remote-format `work_item_id` to the guard and classifier until 0064 lands, or
   pull the one-line `work_item_id` → `external_id` migration into this scope. If
   left deferred, state the duplicate-push risk explicitly for 0051's batch push.

7. **Specify the test net for the no-file mode and the state machine** (addresses:
   no-file mode uncovered; prose state machine; Jira writeback) — Add an explicit
   Linear no-file-mode case (bare identifier, GraphQL captured, input file
   byte-unchanged), specify eval cases (or push logic into a testable seam) for
   each create-work-item AC transition, and add Jira already-synced-refusal +
   end-to-end `external_id` writeback cases.

8. **Tighten the helper and EXIT_CODES edits** (addresses: insert fail-closed;
   derived EXIT_CODES; description frontmatter) — Make the upsert insert branch run
   only on genuine field-absence in well-formed frontmatter (propagate
   malformed/duplicate failures), update the script-side `readonly E_*` constants
   + comments + 100–109 range-summary note alongside the derived Linear table, and
   reword the `create-linear-issue` `description` frontmatter (not just body refs).

## Per-Lens Results

### Architecture

**Summary**: Architecturally strong — pins the central decision, establishes a
clean content-in/identifier-out contract boundary, introduces a dispatcher seam
that decouples `/create-work-item` from per-tracker scripts, and preserves the
0051 extension seam. Main gaps: resilience semantics of the retry-then-fallback
state machine (idempotency under partial failure), an under-specified dispatcher
error-surface contract across two heterogeneous integration scripts, and a latent
divergence in the already-synced guard's trust model now that `external_id` is
presence-tested rather than format-validated.

**Strengths**: Clear contract boundary removing file-lifecycle coupling; dispatcher
as an open-closed indirection point; defer-write single disk-mutation with clean
rollback; well-reasoned phase independence; data-driven status lookup as the 0051
seam; id/external_id split that matches the domain.

**Findings**:
- 🟡 (medium) *Phase 4 §1 & §3* — Dispatcher error-surface contract across two
  heterogeneous integrations is under-specified. Linear (bare identifier, 100–109
  band) and Jira (`{id,key,self}` JSON, overlapping 100–107 band) are not
  normalised into one contract; the caller re-couples to tracker-specific
  codes/shapes, and the retry-vs-fallback decision depends on distinguishing
  transport from validation failure. Specify a single stdout convention + a
  dispatcher-owned exit-code taxonomy.
- 🟡 (medium) *Phase 4 §3* — Retry semantics lack an idempotency guarantee against
  duplicate remote creates. The retry-once loop wraps a non-idempotent create; a
  post-create failure that is retried produces a duplicate — the mode the Linear
  `E_CREATE_WRITEBACK_FAILED` stance is designed to prevent. Constrain retry to
  provably-pre-create failures.
- 🔵 (high) *Phase 1 §2* — Already-synced guard weakens from format-validation to
  mere presence; it now trusts any non-empty `external_id` (incl. hand-typed/
  malformed). Acknowledge the tradeoff or keep a lightweight write-side sanity
  check.
- 🔵 (medium) *Phase 4 §1* — kind→type and project resolution risks duplication
  across the user-facing Jira mode and the dispatcher branch; make the read-only
  resolver script non-optional.
- 🔵 (low) *Phase 4 §1* — The dispatcher's cross-category exec relies on an
  implicit allowed-tools trust assumption; document it as the single sanctioned
  work→integrations bridge and treat integration invocation signatures as a
  published, tested contract.
- 🔵 suggestion *References / Migration Notes* — `work_item_id`'s three meanings
  are narrowed but not resolved; make the optional companion ADR non-optional so
  the deferred 0064 canonicalisation has a stable reference.

### Code Quality

**Summary**: Unusually disciplined for a shell/skill codebase: a single canonical
contract, the `external_id` decision pinned early, the push state machine isolated
behind a dispatcher seam. Main maintainability risks: ANSI colour introduced as a
cross-cutting concern in markdown prose with no script-level encapsulation, and
the retry-then-fallback state machine living entirely in SKILL.md prose. The new
upsert helper is well-shaped, but its awk insert logic is left as a sketch and is
the most error-prone unit.

**Strengths**: Decision discipline on the key collision; upsert composed not
duplicated; data-driven status lookup as a real seam; dispatcher bounds skill-body
complexity; defer-write-once eliminates the replace-only limitation on the hot path.

**Findings**:
- 🟡 (high) *Phase 4 §3* — The five-exit retry/fallback state machine lives entirely
  as SKILL.md prose, unverifiable by unit test. Push branching below into the
  dispatcher (distinct exit codes per outcome) and enumerate the explicit outcome
  table.
- 🟡 (medium) *Phase 3 §3* — ANSI introduced as a cross-cutting concern but emitted
  as inline escape literals in two render surfaces with no single owner. Consider a
  small read-only helper owning the `status → {label, ANSI}` table with localised
  TTY-detection.
- 🔵 (high) *Phase 1 §1* — The upsert awk insert is left as a `...` sketch — the
  highest-risk unit (fail-closed on unclosed frontmatter, insert only in range,
  env-passed value). Specify it to reuse the replace awk's range-tracking states
  and identical integrity re-check.
- 🔵 (medium) *Phase 2 §1* — The Jira skill takes on a second, file-aware mode
  additively; keep the resolution logic in the read-only resolver script so the
  skill body stays a thin dispatcher between two clearly separated modes.
- 🔵 (medium) *Phase 1 §2 / Phase 4 §1* — The dispatcher carries per-integration
  response parsing (bare stdout vs `.key` from JSON) — mild feature envy. Define
  the contract as "bare validated identifier on stdout" for every integration.
- 🔵 suggestion *Phase 3 §1 / Phase 4 §2* — State the empty-string integration gate
  once as a named convention referenced from each phase.

### Test Coverage

**Summary**: TDD-disciplined and enumerates a sensible per-phase test set, but
rests on a flawed assumption that its named suites form part of the CI gate — the
existing Linear/Jira/work suites are not wired into any mise/invoke task, and the
two new suites land in the same unwired subtrees, so `mise run check` will not run
them. Two concrete gaps also stand out: the canonical-tree-fence assertion couples
list-work-items to refine-work-item byte-for-byte (the plan edits only one), and
the retry-once-then-fallback create-work-item state machine is left entirely to
model-driven prose with no scripted test.

**Strengths**: TDD mandated red-first; the upsert test matrix is well-chosen and
mutation-resistant; success criteria phrased independent of palette/count; the
PROJ-0042 classification edge case explicitly asserted; the upsert helper's tests
land in `test-config.sh` which *is* wired into CI.

**Findings**:
- 🔴 (high) *Testing Strategy; Phase 1/2/4* — Named suites are not wired into CI;
  `tasks/test/integration.py` runs `run_shell_suites` only for scripts,
  visualisation, decisions, hooks, migrate, github — no linear/jira/work. Add a
  Phase 0 wiring step.
- 🔴 (high) *Phase 3, Change 4* — `test-hierarchy-format.sh` asserts the tree fence
  is byte-identical between list-work-items and refine-work-item (and is in the CI
  gate). Appending a label breaks it; a static example can't be on/off/identical at
  once. Plan also misidentifies the test.
- 🟡 (high) *Phase 4, Change 3* — The highest-risk new behaviour (retry/fallback,
  no-partial-file, write-once) has no scripted regression test; only the dispatcher
  is tested and no eval case is named.
- 🟡 (medium) *Phase 1 Change 2; Phase 4 Change 4* — The Linear no-file
  create-and-return mode (what the dispatcher calls) is uncovered; existing cases
  are all file-first. Add a bare-identifier / no-file-I/O assertion.
- 🟡 (high) *Phase 4, Change 4* — Shell suites need the exec bit (`os.X_OK`) or
  they silently vanish from CI. State `chmod +x` + a count-floor guard.
- 🔵 (medium) *Phase 1, Change 4* — Existing Linear cases (3, 5, byte-identical
  remainder) and the EXIT_CODES equality check must be re-pointed `work_item_id` →
  `external_id`; list them.
- 🔵 (medium) *Phase 2, Change 1* — Jira already-synced refusal and end-to-end
  writeback are only covered indirectly via the shared helper; add explicit Jira
  cases.

### Correctness

**Summary**: Logically well-structured; presence-based classification, the
empty-string config gate, and the defer-write single-write invariant are sound.
The most serious gaps are in the create-work-item push state machine: the
retry-once contradicts the non-idempotency stance inherited from the Linear
precedent and can create duplicates, and the "no file until resolve" invariant has
an unhandled remote-create-succeeds-but-Write-fails mode. There is also an internal
contradiction in Phase 3 about how `external_id` is read.

**Strengths**: Correctly identifies the exit-0-empty-line gate; presence-based rule
robust under any id_pattern and verified against PROJ-0042; defer-write correctly
sidesteps the replace-only limitation; lookup-consulted-once preserves the 0051
extension invariant.

**Findings**:
- 🔴 (high) *Phase 4 §3* — Retry-once on push failure can create duplicate remote
  issues; only retry provably-pre-create failures, route post-create failures to
  loud manual recovery. Requires dispatcher pre/post-create exit-code distinction.
- 🔴 (medium) *Phase 4 §3 success branch* — No defined behaviour if the single
  Write fails after a successful create; remote issue exists with no local linkage.
  Add a loud Write-failure-after-create branch reusing the
  `E_CREATE_WRITEBACK_FAILED` stance.
- 🟡 (high) *Phase 3 §2 vs Performance Considerations* — Two inconsistent
  external_id read mechanisms (work-item-read-field.sh exit-1 vs awk empty-string)
  with different absent-field semantics. Pick one.
- 🟡 (medium) *Phase 1 §2* — Already-synced guard "non-empty" boundary doesn't
  state quote/whitespace trimming; `external_id: ""` misclassifies. Define
  precisely and apply identically across guard/guard/classifier.
- 🔵 (medium) *Phase 1 §1* — Upsert funnels every non-zero exit (incl. malformed/
  duplicate) into insert; should fail closed and only insert on genuine absence.
- 🔵 (medium) *Phase 4 §1 / Phase 2* — Jira `--project` may be unresolvable on the
  push path (bare-numeric id, unset default_project_code); resolve/validate before
  the offer as a non-retryable decline-equivalent.

### Safety

**Summary**: Governs a non-idempotent remote create paired with a writeback that
can fail. The dominant accidental-harm risk is duplicate remote issues when a
create succeeds but identifier capture fails — the plan preserves the loud-fail
stance for the file-first path but introduces two new paths (deferred single-write,
dispatcher) where this must be re-proven, not assumed. Local data-loss risk is low
(atomic writes, deferred-write avoids partial mutation); recovery via VCS revert /
re-sync is appropriate.

**Strengths**: Defer-the-write eliminates partial-frontmatter hazard; atomic_write +
integrity re-check carried forward; loud non-idempotent stance preserved on the
file-first path; migration is no-op fail-safe; fail-safe defaults throughout.

**Findings**:
- 🔴 (high) *Phase 4 §3 retry path* — Retry-once can create duplicate remote issues
  on a post-create network hiccup; restrict retry to provably-pre-mutation failures
  and fall through to loud manual recovery otherwise (dispatcher must distinguish
  pre/post-send via exit codes).
- 🟡 (medium) *Phase 4 §3 success branch* — Deferred single-write success path
  loses the loud writeback-failure protection the file-first path keeps; specify the
  post-create Write-failure behaviour and add a manual-verification step.
- 🔵 (high) *Phase 1 §1* — The upsert insert branch must carry the same integrity
  re-check and fail-closed guarantees as replace; an off-by-one insert before the
  closing `---` is exactly where corruption would land. Add a unit test.
- 🔵 (high) *Migration Notes* — Items pushed under old `work_item_id` silently read
  as unsynced, inviting duplicate re-push; treat remote-format `work_item_id` as a
  distinct needs-migration signal or ship the one-line migration.
- 🔵 (medium) *Phase 4 §1* — Dispatcher fail-safe for unrecognised/empty
  `--integration` is unspecified; fail closed with a distinct code + test.

### Standards

**Summary**: Largely well-aligned with project conventions: `config_upsert_*`
mirrors `config_set_*`, the dispatcher is correctly placed under the already-allowed
`skills/work/scripts/*` prefix (no allowed-tools widening), and the env-passed
injection-safe value contract is carried forward. Main gaps: the ANSI-colour output
mechanism is unprecedented and may not be valid for the SKILL.md model-output
surface, and the Jira exit-code namespace additions and the Linear `readonly E_*`
constant-vs-table source-of-truth discipline are under-specified.

**Strengths**: Helper name/contract mirrors the established convention; dispatcher
placement + the no-widening reasoning is sound (single tool call; sub-exec is a
child process); push-offer reuses the fail-safe y/Y gate; empty-string gate matches
the script's documented behaviour.

**Findings**:
- 🟡 (medium) *Phase 3 §3 / Desired End State* — Raw ANSI escapes emitted into a
  markdown table that is consumed as model conversation text, not a TTY; zero
  codebase precedent; escapes may surface as literals. Confirm the surface; use a
  markdown-native convention or the migrate TTY-detect idiom.
- 🟡 (high) *Phase 2 §3* — New Jira resolver/guard exit codes not pinned to the
  reserved 108–109 band; risk of collision with the existing 100–107 create codes.
  Update both the Codes table and the Phase 4 namespace summary.
- 🔵 (high) *Phase 1 §4* — Linear EXIT_CODES.md is derived; the `readonly E_*=NN`
  constants are the source of truth (greppable equality check). Update script-side
  constants/comments alongside the table.
- 🔵 (medium) *Phase 1 §3* — `create-linear-issue` `:10` is inside the
  discovery-bearing `description` frontmatter; reword it, not just body refs.
- 🔵 (medium) *Phase 4 §1 / Phase 2 §2* — The dispatcher's own exit-code namespace
  (not-available + pass-through policy) is undocumented; give it an EXIT_CODES
  contract consistent with the per-area convention.

### Compatibility

**Summary**: Largely sound on contract stability: the `external_id` writeback, the
upsert helper, and the Jira file mode are additive, and the 0051 contract is
forward-compatible. Central risk is the deferred migration: items already pushed
under the old Linear flow carry the remote key in `work_item_id`, and after this
change they misclassify as unsynced *and* bypass the already-synced guard, enabling
duplicate creation. Secondary: the guard's trigger condition silently changes shape
(format-test → presence-test).

**Strengths**: `external_id` is an existing omit-by-default slot (no consumer
breaks); the Jira change is explicitly additive; uses A/B of `work_item_id` are
correctly scoped out (visualiser aggregation + read-field bridge intact); the 0051
contract is inherited cleanly; defer-write avoids a contract change on the create
path.

**Findings**:
- 🟡 (high) *Migration Notes; Phase 1 §2* — Items pushed under old `work_item_id`
  bypass the already-synced guard (now presence-based on `external_id`), enabling
  duplicate remote creation the moment Phase 1 lands, before any migration. Keep a
  transitional `work_item_id` read in the guard or pull the migration into scope.
- 🟡 (high) *Migration Notes; Phase 3 §2* — Those same items render as unsynced
  (false negative), which combined with the guard regression can prompt a
  duplicating re-push. Ship the migration or classify remote-format `work_item_id`
  as synced during the transition.
- 🔵 (medium) *Phase 1 §2 & §4* — The guard trigger changes shape, not just field
  name (`E_CREATE_ALREADY_SYNCED` now fires on presence, not remote-format). State
  this in EXIT_CODES + prose and assert the new trigger in tests.
- 🔵 (medium) *Phase 4 §1* — The dispatcher not-available contract for unbuilt
  trackers should be a stable named code so 0049/0050 extend it additively.
- 🔵 suggestion *Phase 1 §4* — Update the EXIT_CODES 100–109 range-summary note (not
  just per-code descriptions) and extend the `! grep work_item_id` check to cover
  EXIT_CODES.md.

### Usability

**Summary**: Strong on consistency and progressive disclosure: one
read/preview/confirm/create/write-back contract across both create skills, the
proven fail-safe y/N gate reused, defer-write leaving no orphan file, graceful
degradation for unbuilt trackers. Two significant DX risks: the synced/unsynced
label relies on raw terminal ANSI in a surface rendered as a markdown table in the
conversation (so colour will likely render as escape gibberish or be stripped), and
the retry-once-then-fallback messaging and the unbuilt-tracker message are
under-specified. The distinct-text accessibility requirement is correctly captured.

**Strengths**: One contract across both create skills; reuses the fail-safe y/N
gate; defer-write = forgiving lifecycle with no orphans; graceful degradation;
data-driven status slot; distinct text AND colour required (protects no-colour
case).

**Findings**:
- 🟡 (high) *Phase 3 §3* — ANSI escapes in a model-rendered markdown table won't
  render as colour and may show as literal gibberish. Pin the surface; use a
  markdown-native distinction or gate ANSI behind TTY-detection.
- 🟡 (medium) *Phase 4 §3* — Retry and "run /sync-work-items later" messaging is
  under-specified; surface the underlying error, distinguish transient vs config
  failure, and reconcile the guidance with 0051 being unbuilt.
- 🔵 (medium) *Phase 4 §1* — The "not available" message for trello/github-issues
  needs concrete, non-dead-end wording (cite 0049/0050, reassure local save).
- 🔵 (medium) *Phase 2 & 4* — Silent default-to-Task on unknown kind and silent
  project source should be surfaced in the preview.
- 🔵 (high) *Phase 4 §2* — The push offer adds a second consecutive confirmation;
  state both outcomes inline so decline-still-saves-locally is clear.
- 🔵 suggestion *Desired End State / Phase 4* — Signpost the relationship between
  `/create-work-item` push and the standalone create skills; name the recovery skill
  in the fallback messaging.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-15

**Verdict:** REVISE

The revision is a strong, faithful response: **all four original critical
findings are resolved**, and the great majority of majors and minors are
addressed. The verdict stays REVISE because the edits surfaced **one new
critical** (a pseudocode bug introduced in the upsert helper), plus a cluster of
majors that are mostly *refinements on the areas just edited* (an unhandled retry
transition, the pre/post-create boundary's ambiguous-timeout case, the
state-machine test still not gating, and a couple of spots where stale pre-edit
wording survived). None require structural rework — they are tightening passes on
the new mechanisms.

### Previously Identified Issues

- 🔴 **Correctness/Safety/Architecture**: Retry-once duplicate creation —
  **Resolved.** The §3 outcome table + the Linear pre/post-create exit-code split
  + the dispatcher taxonomy confine retry to `retryable-transport`. (Two narrower
  residuals below.)
- 🔴 **Correctness/Safety**: Write-failure-after-create — **Resolved.** Added as
  an explicit outcome-table row with loud non-idempotent guidance.
- 🔴 **Test Coverage**: Suites not wired into CI — **Resolved.** Phase 0 wires
  `skills/work` + `skills/integrations` into `run_shell_suites` with an exec-bit
  count-floor guard. (State-machine eval gating residual below.)
- 🔴 **Test Coverage**: Tree-fence byte-identical coupling — **Resolved.** Phase 3
  §4 keeps the shared fence label-free; the equality assertion is preserved.
- 🟡 **ANSI rendering** (usability/standards/code-quality) — **Mostly resolved.**
  Switched to markdown-native glyph+text everywhere except two stale spots (Phase
  3 Overview + References slot still say "ANSI-colour"/"colour") — see New Issues.
- 🟡 **Correctness**: Phase 3 read-mechanism contradiction — **Resolved.** Single
  reader (`work-item-read-field.sh`, exit-1 = unsynced); Performance
  Considerations corrected. (One residual: exit-1 also fires for malformed files
  — see New Issues.)
- 🟡 **Correctness**: quoted-empty boundary — **Resolved.** Shared normalisation
  defined once; applied across guards + classifier.
- 🟡 **Code Quality/Test Coverage**: prose state machine untested — **Partially
  resolved.** §4 names eval transition cases but leaves the testable-seam fallback
  conditional on whether evals gate; reviewers confirm the eval harness does *not*
  currently gate — see New Issues.
- 🟡 **Standards**: Jira exit-code namespace — **Resolved.** Pinned to the
  reserved 108–109 band; both EXIT_CODES sections updated.
- 🟡 **Test Coverage**: Linear no-file mode uncovered — **Resolved.** Explicit
  no-file-mode case added.
- 🟡 **Test Coverage**: exec bit on new suites — **Resolved.** Phase 0 guard +
  "created executable".
- 🟡 **Usability**: retry/fallback messaging — **Mostly resolved.** Both outcomes
  inline; points at `/create-<tracker>-issue`. Residual: recovery guidance is
  inconsistent across the three failure rows — see New Issues.
- 🟡 **Compatibility/Safety**: legacy `work_item_id` guard bypass /
  misclassification — **Resolved (moot).** Verified against the changelog: Linear
  is unreleased, so no in-the-wild item carries a remote-key `work_item_id`.
- 🔵 Minors (resolver non-optional, derived EXIT_CODES lockstep, description
  frontmatter, project-validated-before-offer, dispatcher fail-closed, not-
  available wording, Jira preview transparency, push-offer two-outcomes, named
  gate convention, companion ADR) — **Resolved.**

### New Issues Introduced

- 🔴 **Correctness** (also flagged major by code-quality, minor by
  safety/standards/compatibility): *Phase 1 §1* — the `config_upsert_frontmatter_field`
  pseudocode branches on a constant `E_FM_FIELD_ABSENT` that does not exist;
  `config_set_frontmatter_field` collapses all four error modes (no-frontmatter /
  unclosed / field-absent / duplicate) to a uniform `return 1`. As sketched the
  helper cannot distinguish field-absent and would **fail open** (insert into
  malformed/duplicate frontmatter) — the exact property the revision's prose
  blockquote promises to guarantee. The fix is to make the blockquote's
  re-detect-via-`config_extract_frontmatter` path the *primary* instruction (or
  refactor `config_set_frontmatter_field` to surface distinct codes as a stated
  prerequisite) and delete the misleading skeleton.
- 🟡 **Correctness**: *Phase 4 §3* — the `retryable-transport` row enumerates only
  "retry → success" and "retry → retryable-transport"; it omits the case where the
  single retry returns `terminal-post-create` (first attempt pre-mutation, retry
  sent the mutation and lost the response). State that the retry result re-enters
  the taxonomy (a retry → terminal-post-create routes to the no-retry loud-guidance
  row).
- 🟡 **Safety**: *Phase 1 §2 / Phase 4 §1* — the pre/post-create boundary doesn't
  classify the ambiguous window: a read-timeout or connection-reset *after* the
  request is sent. If mapped to `retryable-transport` it reopens the duplicate-
  create risk. Specify that any failure where the request was (or may have been)
  transmitted maps to `terminal-post-create`; only provably-before-send failures
  are retryable. Add a "response dropped after create" mock test.
- 🟡 **Test Coverage**: *Phase 4 §4* — the create-work-item `evals/` harness has no
  `mise` task and does not gate in CI, so the safety-critical state-machine
  transitions would ship with no automated regression protection. Make the
  testable-seam fallback unconditional (extract the code→action mapping into a
  thin seam the dispatcher suite drives).
- 🟡 **Code Quality**: *Phase 3 §2* — `work-item-read-field.sh` returns exit 1
  uniformly for file-not-found / no-frontmatter / unclosed / field-absent, not just
  field-absent. Mapping every exit 1 to "unsynced" would misclassify a malformed
  file as a valid unsynced item (Step 2 already skip-and-warns those). Scope the
  "exit 1 → unsynced" rule to files that passed Step 2's validity check.
- 🟡 **Architecture**: *Phase 3 §2* — the existing Step 2 scan already emits every
  frontmatter line in one awk pass, so the mandated per-file `work-item-read-field.sh`
  call is a redundant second reader on the same path (chosen to honour AC #4's
  wording). Consider folding `external_id` into the single-pass scan as the
  authoritative reader and reconciling AC #4 in the follow-up `/update-work-item`.
- 🟡 **Compatibility**: *Phase 4 §1* — "bare *validated* identifier" is
  under-specified across trackers: Linear's `^[A-Z][A-Z0-9]*-[0-9]+$` regex would
  reject GitHub (`owner/repo#42`) / Trello (opaque) identifiers. State that
  format validation is *per-tracker* (each integration validates its own shape)
  and the dispatcher does only a tracker-agnostic safety check (non-empty, single
  line, no control/injection chars).
- 🟡 **Test Coverage**: *Phase 1 §4* — the plan claims `test-linear-create.sh`
  already greps the `readonly E_*` constants for value-parity with EXIT_CODES.md;
  that mechanism does not exist in the suite today. Either add it as new work or
  drop the claim and rely on the `! grep work_item_id` check.
- 🔵 **Code Quality/Standards/Usability**: residual stale wording — Phase 3
  *Overview* still says `status → {label, ANSI-colour}` and References still says
  `status → {label, colour}`; reword to `{label, glyph}` to match §3.
- 🔵 **Test Coverage**: *Phase 1 success criteria* — the "byte-identical-remainder"
  case must switch its exclusion to `external_id` AND start from a fixture with no
  `external_id` line so it proves *insertion* (not replacement); Case 5 must keep a
  deterministic (duplicate-line) fail-closed trigger, not a permission-based one.
- 🔵 **Architecture/Code Quality**: minor — active integration has two sources of
  truth (config gate vs dispatcher `--integration` arg; state the gate is the sole
  source); `.key` extraction placement ("resolver or thin wrapper") should commit
  to the thin wrapper to keep the resolver single-responsibility.
- 🔵 **Standards**: suggestion — pin the dispatcher taxonomy to concrete
  `readonly E_*=NN` integers in a declared band, matching the Jira/Linear
  EXIT_CODES convention.
- 🔵 **Safety/Usability**: the loud terminal-post-create guidance should name the
  saved file's absolute path and prefer `/create-<tracker>-issue <path>` over a
  manual frontmatter edit; standardise "push later" recovery across all non-success
  rows.
- 🔵 **Test Coverage**: Phase 0 now pulls the python3 mock servers into CI —
  confirm `python3` is provisioned and note the mock startup-timeout given known
  shell-suite parallel-load flakiness.

### Assessment

The plan is materially stronger and the hard structural decisions (idempotency
boundary, CI wiring, defer-write outcomes, markdown-native rendering) are sound
and settled. What remains is a **tightening pass**, not rework: fix the one
pseudocode bug the edits introduced (the `E_FM_FIELD_ABSENT` skeleton), close the
two small idempotency-boundary gaps (retry re-enters the taxonomy; classify the
ambiguous post-send timeout as terminal), commit the state-machine logic to a
gating testable seam, scope the `work-item-read-field.sh` exit-1 rule to
validated files, make the dispatcher's identifier validation per-tracker, and
sweep the residual ANSI wording. Once those land the plan should reach APPROVE
without another structural review.

## Re-Review (Pass 3) — 2026-06-16

**Verdict:** APPROVE

Final verification pass over the five lenses that raised the pass-2 findings
(correctness, safety, test-coverage, architecture, compatibility). **All pass-2
findings are confirmed resolved**, and the structural decisions verified sound
against the actual codebase. The pass found **one new bug introduced by the pass-2
edits** — the `E_FM_FIELD_ABSENT` fix had substituted a different phantom symbol,
`config_get_frontmatter_field` (no such shared getter exists) — plus a handful of
minor refinements. All were fixed in this round. No critical or major issue
remains outstanding; verdict moves to APPROVE.

### Previously Identified Issues (pass-2 New Issues)

- 🔴 **Correctness**: `E_FM_FIELD_ABSENT` phantom constant — **Resolved**, then a
  follow-on bug (`config_get_frontmatter_field` phantom *function*, flagged by
  correctness/compatibility/safety/architecture this pass) was caught and
  **fixed** — the presence check now greps the `config_extract_frontmatter` output
  with `^key:` anchoring; a note records how to add a real getter if preferred.
- 🟡 **Correctness**: retry re-enters the taxonomy — **Resolved & verified** (bounded
  single retry, no loop; child results 0 / terminal-post-create / retryable all
  handled).
- 🟡 **Safety**: ambiguous post-send timeout — **Resolved & verified** (maps to
  terminal-post-create end-to-end; conservative and correct).
- 🟡 **Test Coverage**: state machine in a gating testable seam — **Resolved**, with
  a refinement applied: the seam input now includes a post-dispatcher
  write-result flag so the `Write-failure-after-success` row (a local failure after
  dispatcher `0`) is expressible and unit-tested, not just dispatcher exit codes.
- 🟡 **Architecture**: list-work-items dual reader — **Resolved & verified** (single
  authoritative awk-fold reader); a residual Performance-Considerations
  contradiction was **fixed** to match.
- 🟡 **Compatibility**: per-tracker identifier validation — **Resolved & verified**;
  refined so the dispatcher's tracker-agnostic safety check explicitly permits
  `/`, `#`, `@` (GitHub/Trello identifiers) and rejects only what breaks unquoted
  YAML.
- 🟡 **Test Coverage**: EXIT_CODES parity claim corrected; byte-identical proves
  insertion — **Resolved**, with an added requirement to assert the inserted line's
  *position* inside the fence (remainder-equality alone doesn't).
- 🔵 Count-floor sizing, python3 CI note, residual ANSI wording, Linear preview,
  AC #7 reconciliation, `.key` thin wrapper, dispatcher single-source-of-truth +
  code pinning — **all resolved**.

### New Issues Introduced

- None outstanding. The one bug this pass surfaced (the `config_get_frontmatter_field`
  phantom) was fixed in-round, along with the minor refinements above.

### Assessment

The plan is sound and ready for implementation. The hard problems —
non-idempotent-create idempotency boundary, CI-gating of the test net,
defer-write outcome completeness, markdown-native rendering, and forward-
compatibility for 0049/0050 — are all settled and verified against the codebase.
Two cross-cutting implementation watch-items (not blockers) carry into execution:
(1) the upsert helper and the Phase 3 classifier both need a frontmatter
field-presence/read primitive that does not exist yet — author it once (a shared
getter or a documented inline grep) and share it; (2) the work-item-push-decide
seam is the single gating guard against duplicate remote issues, so its per-row
unit tests are the most important new tests to land. **Verdict: APPROVE.**
