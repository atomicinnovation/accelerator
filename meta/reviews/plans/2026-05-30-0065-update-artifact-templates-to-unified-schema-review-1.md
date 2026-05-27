---
date: "2026-05-30T06:00:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-30-0065-update-artifact-templates-to-unified-schema.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, documentation, standards, compatibility, portability]
review_pass: 4
status: complete
---

## Plan Review: Update All Artifact Templates to Unified Schema

**Verdict:** REVISE

The plan is structurally strong — TDD-first, phase-decomposed, anchored in ADR-0033, with an explicit and statically-fixed cut-line against 0066/0067/0070. However, eight lenses converge on a small set of consequential gaps: a renamed own-identity field (`work_item_id:` → `id:`) silently breaks five non-emitter consumers the plan classifies as "no action"; the Phase 1 / Phase 2 helper-output label contract is unpinned and likely to drift; the deliberately-lenient SKILL-prose test does not actually defend the population contract it nominally protects; and the Phase 11 discovery grep, as recorded, cannot reproduce the producer split the plan claims it does. Two findings are critical; ~22 are major.

### Cross-Cutting Themes

- **Metadata-helper triplication is perpetuated, not addressed** (flagged by: architecture, code-quality) — The three near-duplicate helpers (`scripts/artifact-derive-metadata.sh` + two skill-local variants) are edited in lockstep with identical changes. The plan even back-ports `inventory-metadata.sh`'s jj-first branching into the shared helper while preserving the duplication. Locks in coordinated-edit cost for every future provenance change.
- **SKILL-prose test is too lenient to defend its contract** (flagged by: architecture, code-quality, test-coverage, correctness) — The "at least one match" grep gate cannot distinguish `last_updated` from `last_updated_by`, cannot tell descriptive prose from substitution-instructing prose, and is satisfied automatically when a SKILL.md embeds the template via `config-read-template.sh`. AC8 of the work item (born-populated artifacts, no unsubstituted tokens) therefore depends entirely on manual verification.
- **Helper output label is unpinned between Phase 1 spec and Phase 2 impl** (flagged by: code-quality, correctness, compatibility, standards) — Phase 1 says the test accepts either `REVISION=` or `Current Revision Hash:`; Phase 2 emits `Current Revision:` (no "Hash", no `=`). These three strings must converge before either phase lands.
- **Renames break consumers categorised as "no action"** (flagged by: architecture, compatibility) — `work_item_id:` → `id:` on the work-item template invalidates `wip_is_work_item_file` (work-item-common.sh:447), `update-work-item`, `refine-work-item`, `list-work-items`, AND `create-work-item`'s own enrich-existing self-check. Phase 11's "Non-emitter template consumers (no action)" classification understates this as a documentation choice rather than a runtime break.
- **Discovery grep doesn't enumerate what the plan claims** (flagged by: correctness, architecture, compatibility) — The first recorded grep (`config-read-template\.sh|schema_version|last_updated|git_commit`) cannot match `extract-adrs` pre-Phase-4 (prose indirection, no `config-read-template.sh` call yet); the second (`verdict:|review_pass:|review_target:|pr_number:`) does not surface `validate-plan` (which emits `target:`/`result:`). The recorded "Producer split" therefore cannot be reproduced by running the recorded commands.
- **Status-vocabulary requirement is asserted as "presence of a comment", not "presence of the right comment"** (flagged by: test-coverage, documentation, correctness) — AC7 requires the comment to enumerate the per-type vocabulary verbatim; the regex `^status:\s+\S+\s+#\s+\S` passes on any non-empty comment. Plus the per-template source-of-truth for the vocabulary is not specified, so two implementers running Phases 3-10 in parallel will produce demonstrably different comments.
- **Phase independence is overstated** (flagged by: architecture, code-quality) — Phases 4, 5, 7, 8, 9, 10 all transitively depend on Phase 2's helper output keys. Only Phase 8 surfaces this explicitly.

### Tradeoff Analysis

- **Lenient gate vs. test maintenance cost**: tightening the SKILL-prose grep to require an instruction context (a fenced code block or imperative verb near the field) increases test maintenance but is the only way to make AC8 actually enforceable. The current "lenient by design" stance trades off real contract enforcement for prose freedom — the plan should make that trade-off explicit if kept.
- **Single coordinated story vs. multiple small stories**: the work-item own-id rename (`work_item_id:` → `id:`) plus its five consumer updates could either be folded into 0065 (expanding scope) or split into a separate story (preserving 0065's "templates-only" framing). The plan currently does neither — it does the rename without the consumer updates.

### Findings

#### Critical

- 🔴 **Compatibility**: Renaming `work_item_id:` → `id:` breaks every script that parses own-identity, but only the two producer skills are updated
  **Location**: Phase 3 + Phase 11 ("non-emitter template consumers (no action)")
  After Phase 3 lands, `wip_is_work_item_file` (work-item-common.sh:447) will mark every new work-item file as not-a-work-item; `list-work-items`, `update-work-item`, `refine-work-item` all rely on this predicate or on direct `work_item_id:` reads. `create-work-item`'s own enrich-existing path also calls `work-item-read-field.sh work_item_id` as an integrity check — new work items become un-enrichable. The plan classifies these consumers as "no action."

- 🔴 **Compatibility**: `validate-plan`'s inline frontmatter will continue overwriting the new template block — post-0065 validations still ship the legacy shape
  **Location**: Phase 6 (lines 660-665)
  Phase 6 explicitly leaves `validate-plan/SKILL.md:134-145` untouched, but the inline emission there is the *whole* file write. Between 0065 and 0066, every new validation report emits `skill: validate-plan`/no `id:`/no `producer:`/no `schema_version:` despite the template carrying the unified block. The handoff is correctly stated but the consequence ("born unified for 8 of 9 templates, not 9 of 9") is understated.

#### Major

- 🟡 **Architecture / Code Quality**: Triplicated metadata helpers are perpetuated rather than consolidated
  **Location**: Phase 2
  Three near-identical helpers receive the same set of edits. The plan even back-ports `inventory-metadata.sh`'s jj-first branching into the shared helper. Every future provenance change pays a 3× edit tax.

- 🟡 **Architecture**: Phase independence claim is overstated — helper-output keys are a shared dependency
  **Location**: Implementation Approach
  Phases 4, 5, 7, 8, 9, 10 all transitively depend on Phase 2's helper output keys; only Phase 8 calls this out. The lenient SKILL-prose test won't catch mis-ordered phases.

- 🟡 **Architecture / Code Quality / Test Coverage / Correctness**: SKILL-prose test gate is too lenient to defend the population contract
  **Location**: Phase 1 §2; Implementation Approach
  "At least one match" grep can't distinguish substitution instructions from descriptive prose, conflates `last_updated`/`last_updated_by`, and is satisfied automatically by templates rendered into SKILL.md via `config-read-template.sh`. AC8 then rides on manual verification.

- 🟡 **Code Quality**: Silent failures from `jj log … 2>/dev/null || echo ""` mask real errors under `set -euo pipefail`
  **Location**: Phase 2 §1 (example bash)
  Inside the outer `jj root` guard, an inner `jj log` failure silently produces empty REVISION; the helper-output test only exercises the happy path so the regression is invisible.

- 🟡 **Code Quality**: `[ -n "$X" ] && echo …` interacts badly with `set -e`
  **Location**: Phase 2 §1 (example bash)
  A future edit moving one of these `&&` chains to script-end will exit the script non-zero whenever the variable is empty — turning a graceful fallback into a hard failure.

- 🟡 **Code Quality**: Three overlapping sources of truth for the schema table will drift
  **Location**: Phase 1 + work item Schema Reference
  The same per-template contract is encoded in (a) the work-item Schema Reference, (b) the TEMPLATES array, (c) the SKILL.md grep table — with no derivation between them.

- 🟡 **Correctness / Compatibility / Standards**: Helper output label is unpinned between Phase 1 and Phase 2
  **Location**: Phase 1 §3 + Phase 2 §1
  Phase 1 spec accepts `REVISION=` or `Current Revision Hash:`; Phase 2 emits `Current Revision:`. The three strings must converge before either lands.

- 🟡 **Test Coverage / Documentation / Correctness**: Status-comment regex passes on any non-empty comment, leaving AC7 unenforced
  **Location**: Phase 1 §1
  AC7 requires the comment enumerate the per-type vocabulary verbatim; the regex `^status:\s+\S+\s+#\s+\S` accepts `status: x # y`. Plus the per-template vocabulary source-of-truth is not specified.

- 🟡 **Test Coverage**: AC8 (non-tokenised artifact output) is verified only manually
  **Location**: Testing Strategy
  The single most behavioural acceptance criterion has no automated regression protection. Several updated skills (`extract-work-items`, `extract-adrs`, `analyse-design-gaps`, `inventory-design`) are not even in the manual sample.

- 🟡 **Test Coverage / Portability**: Helper test covers only the git fallback, never the jj-first branch
  **Location**: Phase 1 §3
  The temp-repo setup uses `git init`; the jj branch is exercised incidentally if jj is installed, never as an asserted case.

- 🟡 **Test Coverage**: Own-identity-vs-foreign-reference assertion is underspecified
  **Location**: Phase 1 §1
  The "no `<type>_id:` for own identity" rule conflates with legitimate foreign references (plan template's `work_item_id`). Without a per-row `forbidden_own_id_key` column the assertion will either over-fire or under-fire.

- 🟡 **Correctness**: Phase 11 first discovery grep cannot match `extract-adrs` pre-Phase-4
  **Location**: Phase 11
  `extract-adrs/SKILL.md` currently has none of the four tokens (`config-read-template.sh|schema_version|last_updated|git_commit`); the recorded "Producer split" lists it as in-scope. The grep is not reproducible.

- 🟡 **Correctness**: Second discovery grep cannot find `validate-plan`
  **Location**: Phase 11
  `validate-plan` emits `target:`/`result:`, neither matched by `verdict:|review_pass:|review_target:|pr_number:`. The recorded 0066-owned set cannot be produced by the recorded command.

- 🟡 **Correctness**: Discovery completeness assertion will fail immediately on legitimate non-emitter consumers
  **Location**: Phase 11 §2
  `update-work-item`, `refine-work-item`, `list-work-items` all call `config-read-template.sh` and would match the first grep, but are not in the in-scope or 0066-owned union the assertion allows.

- 🟡 **Documentation**: Inline-comment voice and density inconsistent across the nine template specifications
  **Location**: Phases 3–10
  work-item.md has substantive contract comments on nearly every field; plan.md / pr-description.md / design-gap.md are sparse and largely attributional. No declared convention.

- 🟡 **Documentation**: Per-type status-vocabulary lookup is under-specified for the implementer
  **Location**: Implementation Approach, Phases 3–10
  Several templates have no existing status comment. Implementers are told to "reproduce verbatim" without per-template source-of-truth pointers; the plan's own examples use inconsistent formats.

- 🟡 **Documentation**: SKILL.md prose edits described too loosely for consistent output
  **Location**: Implementation Approach + Phases 3–10
  Phase 5 introduces a named "Step 4b: Capture metadata"; Phases 7-10 refer to "the metadata-substitution step" variably. No canonical snippet provided.

- 🟡 **Standards**: `test:unit:templates` task body breaks the invoke-delegation convention
  **Location**: Phase 1 §4
  Every existing mise task delegates to `invoke <ns>.<task>` or a single one-liner; the proposed triple-quoted bash heredoc has no precedent.

- 🟡 **Compatibility**: `supersedes:` shape change (scalar → typed-linkage list) is silently breaking
  **Location**: Phase 4
  No discovery of consumers reading the legacy shape; no transition window. Mixed-shape corpus until 0070.

- 🟡 **Compatibility**: `pr_title:` migration is not coordinated with `review-pr` (0066)
  **Location**: Phase 7
  `review-pr/SKILL.md:458` still writes `pr_title:` inline; post-0065 PR descriptions emit `title:` while post-0065 PR reviews emit `pr_title:`. Cross-artifact field divergence within the same release.

- 🟡 **Compatibility**: Discovery pass mislabels consumer skills as "no action"
  **Location**: Phase 11
  The two recorded greps find producers only; the consumer-side breakage surface (any script reading `work_item_id`, `adr_id`, `pr_title`, `skill:`, `supersedes:`) is invisible to the pass.

- 🟡 **Portability**: Temp-repo `git commit --allow-empty` requires `user.email`/`user.name` config that CI and fresh dev machines often lack
  **Location**: Phase 1 §3
  `mise run test:unit:templates` will fail on identity-unset environments before any helper assertion runs.

#### Minor

- 🔵 **Architecture**: Phase 6 creates a transiently inconsistent template (`validation.md` carries unified frontmatter, but `validate-plan` ignores it until 0066). Add a tracking comment so readers understand the dead-block.
- 🔵 **Architecture**: ADR `supersedes:` shape change introduces corpus divergence the plan does not surface as a risk.
- 🔵 **Architecture**: Phase 11 discovery assertion couples to a static SKILL.md inventory — risk of becoming a tripwire on legitimate new skills.
- 🔵 **Architecture**: Helper output is human-readable prose rather than a machine-parseable key/value format — locked in further by Phase 1 assertions.
- 🔵 **Code Quality**: Pipe-delimited TEMPLATES rows are primitive-obsession; use parallel arrays or TSV.
- 🔵 **Code Quality**: Provenance keying contract under-specified (`REVISION=` vs `Current Revision Hash:`) — defer-to-implementer language weakens the RED-first guarantee.
- 🔵 **Code Quality**: Discovery-pass assertion belongs in its own test, not appended to SKILL-prose driver (single-responsibility erosion).
- 🔵 **Code Quality**: Three sequential `bash scripts/...` invocations in the mise task short-circuit on first failure.
- 🔵 **Code Quality**: Discovery grep tokens are fragile — `git_commit` disappears post-Phase-2; future producers without those tokens are invisible.
- 🔵 **Code Quality**: Status-line regex won't match the proposed `validation.md` status if the implementer omits the inline comment.
- 🔵 **Test Coverage**: `date`, `last_updated`, `tags` field-shape rules from ADR-0033 are not type-checked.
- 🔵 **Test Coverage**: Concrete regex for the `schema_version: 1` integer-vs-string distinction is left to the implementer.
- 🔵 **Test Coverage**: RED baseline is asserted by manual exit-code inspection only; no per-row "expected to FAIL pre-edit" marker.
- 🔵 **Test Coverage**: validation.md's frontmatter contract is asserted only at template level, with no field-population test in 0065.
- 🔵 **Test Coverage**: Discovery-pass test patterns drift-prone — define grep patterns in a single sourced location.
- 🔵 **Correctness**: ISO timestamp regex would falsely match a labelled REVISION line if it ever contained an ISO-like substring — anchor to the label.
- 🔵 **Correctness**: `parent` is a corpus-wide linkage key but absent from Phase 1 base-field assertions.
- 🔵 **Correctness**: Status-comment regex requires whitespace after `#` (`\s+`) — should be `\s*`.
- 🔵 **Correctness**: Research templates' status vocabulary may not be `complete`-only — confirm pre-edit set.
- 🔵 **Correctness**: Plan template's `work_item_id` value-shape (id-only vs `doc-type:id`) is not asserted.
- 🔵 **Documentation**: Discovery Pass Record appended to the work item — likely to rot; consider plan or research location.
- 🔵 **Documentation**: "What We're NOT Doing" omits read-side consumers, template-body preservation, and ADR-0028 supersession.
- 🔵 **Documentation**: References footer is incomplete (missing ADR-0028, ADR-0034 explicit path, epic 0045, `skills/config/configure/SKILL.md`).
- 🔵 **Documentation**: Phase 5's "Step 4b" naming not propagated as a convention to Phases 7-10.
- 🔵 **Standards**: Test driver lacks the `cd "$ROOT"` pattern established by `test-format.sh`.
- 🔵 **Standards**: Filename-timestamp format `%Y-%m-%d_%H-%M-%S` differs from sibling helpers' `%Y-%m-%d-%H%M%S` — preserved verbatim without justification.
- 🔵 **Standards**: Inline comment density inconsistent with several existing templates.
- 🔵 **Standards**: `depends`-list TOML edit shape (single-line vs multi-line) unspecified.
- 🔵 **Standards**: Plan's own frontmatter still uses `skill:` and `work_item_id:` — flag intentionally in NOT Doing.
- 🔵 **Standards**: Phase 11 grafts discovery onto SKILL-prose population test, mixing concerns.
- 🔵 **Compatibility**: ADR own-id rename is shielded by visualiser filename fallback — record this as the rationale.
- 🔵 **Compatibility**: Helper output relabel is self-contained today but the test and helper labels should be pinned to one literal.
- 🔵 **Portability**: `git init` default branch differs by git version — pass `--initial-branch=main`.
- 🔵 **Portability**: `jj root` short-circuit when jj is installed but cwd is non-jj — acceptable but worth documenting.
- 🔵 **Portability**: jj workspace with empty change at `@` may emit non-pushable commit_id; consider `@-` for jj.
- 🔵 **Portability**: Frontmatter parser assumes LF line endings — normalise CRLF or set `.gitattributes`.
- 🔵 **Portability**: Test driver should `unset GIT_DIR GIT_WORK_TREE JJ_CONFIG` and set `HOME=$tmpdir` for isolation.
- 🔵 **Portability**: Prerequisite tooling (bash 4+, jj/git, date semantics) is not documented.

#### Suggestions

- 🔵 **Correctness**: Empty `target: ""` in validation template is not a valid ADR-0034 reference at template level — comment or omit.
- 🔵 **Correctness**: Pin the exact helper-output label (`Current Revision:`) in both Phase 1 and Phase 2 so the implementer has one target.
- 🔵 **Documentation**: Document the lenient-by-design choice for the status-comment regex explicitly.

### Strengths

- ✅ Strict-TDD framing: Phase 1 lands three failing drivers wired into a new `test:unit:templates` mise task; every later phase has a concrete RED→GREEN gate.
- ✅ ADR-0033 is treated as the single source of truth ("on discrepancy, ADR-0033 wins") — the right dependency direction.
- ✅ Cut-line with 0066/0067/0070 is statically fixed and named, eliminating runtime-negotiated scope.
- ✅ Phase 11's reproducible discovery grep concept (recorded into a durable artifact) is a thoughtful guard against future inline producers slipping the contract.
- ✅ Phase 4 reintroduces an explicit `config-read-template.sh adr` call in `extract-adrs`, removing a prose indirection and restoring the canonical template-loader pattern.
- ✅ ADR own-identity rename is partially shielded by filename fallback in the visualiser (indexer.rs:1098, wiki-links.ts:103-115) — a meaningful resilience the plan can record as the rationale for not adding a transitional fallback.
- ✅ Test drivers follow established conventions: `set -euo pipefail`, sourcing `scripts/test-helpers.sh`, `test_summary`, matching the existing `test-*.sh` corpus.
- ✅ ISO timestamp format `+00:00` as a literal in the format string is portable across BSD/GNU `date`.
- ✅ Body content of templates is explicitly preserved untouched — minimum change to achieve the schema goal.
- ✅ ADR-0034 typed-linkage list shape for `supersedes` (`["adr:ADR-NNNN"]`) is correctly captured.
- ✅ The work-item Schema Reference table provides 1:1 row-to-acceptance traceability for the contract.

### Recommended Changes

1. **Decide work-item own-id rename scope and either expand Phase 3 or split the rename out** (addresses: critical compatibility finding on `work_item_id:` → `id:`)
   The current plan renames the producer-side field but leaves five consumer scripts (`wip_is_work_item_file`, `update-work-item`, `refine-work-item`, `list-work-items`, `create-work-item`'s enrich-existing self-check) reading `work_item_id`. Either (a) expand Phase 3 to add transitional `id:`-first/`work_item_id:`-fallback reads to those consumers, or (b) defer the template rename to a coordinated story that includes the consumers. The plan must record the chosen approach.

2. **Add a Migration Notes paragraph stating that plan-validation is the single "born-unified" exception until 0066 lands** (addresses: critical compatibility finding on validate-plan inline override)
   Make the consequence explicit so readers don't infer that all post-0065 artifacts carry the unified shape. Also confirm ordering: 0066 must ship close behind 0065, or the gap window grows.

3. **Pin the helper output label in both Phase 1 and Phase 2 to one literal string** (addresses: helper label drift)
   Recommend: `Current Revision: <hash>` (matching the existing `Current Date/Time (UTC): <iso>` pattern). Drop the "or `REVISION=` or `Current Revision Hash:`" alternative from Phase 1's spec.

4. **Tighten the SKILL-prose test or drop the "contract" framing** (addresses: lenient-gate findings from architecture/code-quality/test-coverage/correctness)
   Either (a) require the field name within an instruction context — fenced code block, or within N lines of a `## Step` heading containing `Populate|Substitute|Set` — or (b) accept the leniency but rename the test (e.g., `test-skill-frontmatter-smoke.sh`) and explicitly document that AC8 verification rides on integration/manual checks.

5. **Re-record Phase 11's Discovery Pass with greps that actually enumerate the claimed producer split** (addresses: Phase 11 grep findings)
   The first grep cannot match `extract-adrs` pre-Phase-4; the second cannot match `validate-plan`; and the discovery completeness assertion will fail on legitimate non-emitter consumers. Either run the discovery *after* Phase 4 has added the explicit `config-read-template.sh adr` call, broaden the grep tokens (add `target:|result:` to catch validate-plan), and enumerate the non-emitter consumer allowlist in the assertion.

6. **Strengthen the status-comment contract and name per-template vocabulary sources** (addresses: AC7 enforcement gap)
   Add an `expected_status_vocabulary` column to the TEMPLATES table (e.g., work-item.md → `draft | ready | in-progress | review | done | blocked | abandoned`) and assert the comment substring matches the pinned string. For each per-phase template that lacks a current status comment, name the source-of-truth file (e.g., codebase-research.md → `research-codebase/SKILL.md` persistence section).

7. **Provide a canonical SKILL.md persistence-step snippet for Phases 3-10 to paste** (addresses: documentation finding on SKILL.md prose looseness)
   Add a "Canonical persistence-step prose snippet" block under Implementation Approach with template parameters (`{producer-name}`, `{provenance? yes/no}`). Have Phases 3-10 reference it rather than re-describing the substitution per phase.

8. **Fix the `git init && git commit --allow-empty` portability issue in the helper test driver** (addresses: portability finding)
   Use `git -c user.email=test@example.com -c user.name=Test -c init.defaultBranch=main commit ...` or set the `GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL` env vars. Also `unset GIT_DIR GIT_WORK_TREE JJ_CONFIG` and isolate `HOME` to the temp dir.

9. **Make Phase 2 a stated prerequisite of Phases 4, 5, 7, 8, 9, 10** (addresses: architecture finding on phase independence)
   Update the Implementation Approach paragraph that claims Phases 3-10 are mutually independent — they are file-independent, but Phases 4/5/7/8/9/10 are runtime-dependent on Phase 2's helper output keys.

10. **Coordinate `pr_title:` → `title:` migration with `review-pr` (0066)** (addresses: pr_title compatibility finding)
    Either keep `pr_title:` present-but-deprecated in describe-pr's output for one release, or confirm 0066 ships in lockstep so cross-artifact PR field naming is consistent at every release boundary.

11. **Address (or defer with a follow-up issue) the metadata-helper triplication** (addresses: architecture/code-quality finding)
    Either collapse the two skill-local helpers into thin wrappers over the shared one (with a `--filename-format=` flag) or open a follow-up story and link it from Migration Notes. Don't perpetuate the triplication silently.

12. **Choose ONE source of truth for the schema table** (addresses: code-quality finding on three drift surfaces)
    Either extract the table to `scripts/templates-schema.tsv` consumed by both test drivers, or add a markdown-table-parsing assertion that ties the test back to the work-item Schema Reference.

13. **Convert the mise task to invoke-delegation form** (addresses: standards finding)
    Add `tasks/test/unit.py:templates` that runs the three drivers, then in `mise.toml` use `run = "invoke test.unit.templates"`. Matches every other test:unit:* task.

14. **Add explicit Migration Notes paragraphs for the breaking renames and a consumer-discovery pass** (addresses: compatibility findings on `supersedes`, `pr_title`, `skill`, mislabelled consumers)
    Run a sibling consumer-discovery grep (`rg -n 'work_item_id|adr_id|pr_title|skill:|supersedes:|git_commit' skills --glob '**/SKILL.md' --glob '**/*.sh'`) and audit each hit. Record findings in Migration Notes.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan demonstrates strong architectural thinking — a clearly defined cut-line between this story and 0066/0067/0070, a documented identity/provenance contract anchored in ADR-0033, and a deliberate strategy of pushing schema enforcement into executable tests rather than prose. Three structural weaknesses: (1) three near-duplicate metadata helpers are perpetuated rather than consolidated; (2) the claim of phase independence is overstated — Phases 4/5/7/8/9/10 share a runtime dependency on Phase 2's helper output keys; (3) the SKILL-prose test is so lenient (literal grep) that it does not actually defend the architectural boundary it nominally protects.

**Strengths**: clear named cut-line; ADR-0033 as single source of truth; test-driven contract externalisation; reproducible discovery grep; explicit template-loader for extract-adrs.

**Findings**: 3 major (helper triplication; overstated independence; lenient SKILL-prose test), 4 minor (validation.md transient inconsistency; ADR supersedes corpus divergence; discovery-pass tripwire risk; prose-shaped helper output).

### Code Quality

**Summary**: TDD framing is clear but several specific code-quality concerns exist: three near-duplicate metadata helpers being modified in parallel; the example bash uses `2>/dev/null` to silence inner jj/git failures (regression vector under `set -e`); `[ -n ] && echo` constructs are brittle under `set -e`; hardcoded tables (TEMPLATES, SKILL.md table, work-item Schema Reference) duplicate the schema contract across three places with no derivation between them.

**Strengths**: explicit RED→GREEN per phase; deliberate phase decoupling; named mise task wired into aggregator; reuses `test-helpers.sh`; thoughtful discovery-pass-as-test idea.

**Findings**: 4 major (helper triplication; silent jj/git failures; `[ -n ] && echo` set -e brittleness; three sources of truth), 6 minor (primitive-obsession in TEMPLATES rows; provenance keying ambiguity; discovery assertion responsibility erosion; sequential mise commands short-circuit; lenient grep gate; discovery grep token fragility), 1 around status-line regex burdening implementers.

### Test Coverage

**Summary**: The plan is structurally test-driven and the three-layer test design covers the right surfaces, but each layer has gaps. The most consequential: status-vocabulary content is not asserted (AC7 left to prose only); the SKILL-prose gate is so lenient that descriptive prose can satisfy it without instructing population; AC8 (non-tokenised output) is verified manually only; and the helper test never exercises the jj branch.

**Strengths**: test-first ordering with explicit RED baseline; wired into umbrella `test:unit`; 1:1 row-to-acceptance traceability via TEMPLATES table; mutation-resistant negative regex for `%Z` timestamps; Phase 11 discovery assertion.

**Findings**: 5 major (status-comment regex doesn't enforce vocabulary; lenient SKILL-prose gate; AC8 manual only; jj branch not exercised; underspecified own-id assertion), 5 minor (no timestamp/tags shape assertion; schema_version regex not pinned; RED baseline manual; validation.md template-only; discovery patterns drift-prone).

### Correctness

**Summary**: The plan is internally consistent in most respects, but several precise issues exist: Phase 11's recorded discovery greps cannot enumerate the producer set the plan claims (extract-adrs has no matching tokens pre-Phase-4; validate-plan does not match the second grep); the SKILL-prose assertion conflates `last_updated`/`last_updated_by`; the discovery completeness assertion will fail on legitimate non-emitter consumers.

**Strengths**: explicit RED baseline; correct identification of describe-pr as in-scope hybrid; correct ADR-0034 list shape for `supersedes`; correct id/foreign-`<type>_id` distinction across all nine templates.

**Findings**: 3 major (first discovery grep cannot match extract-adrs; second cannot find validate-plan; SKILL-prose conflates `last_updated*`), 6 minor (ISO regex anchoring; missing `parent` assertion; `\s+` vs `\s*` after `#`; research status vocabulary unverified; plan `work_item_id` value-shape not asserted; discovery completeness allowlist incomplete), 2 suggestion (empty `target` validity; helper label pinning).

### Documentation

**Summary**: The plan is well-organised but the documentation artifacts it produces are specified at uneven levels of prescriptiveness. Inline-comment voice and density vary across the nine templates; per-type status-comment requirement defers vocabulary lookup to the implementer without naming source-of-truth per type; SKILL.md prose edits are described as "add prose instructing the model to substitute" without a canonical snippet.

**Strengths**: Schema Reference as authoritative table; documentation contracts tied to automated tests; explicit scope handoffs; reproducible discovery grep.

**Findings**: 3 major (inline comment voice inconsistent; status vocab lookup underspecified; SKILL.md prose loose), 4 minor (Discovery Pass Record location; NOT Doing omissions; References footer incomplete; Step 4b not propagated as convention), 1 suggestion (lenient regex documented choice).

### Standards

**Summary**: Mostly aligned with repo conventions but the `test:unit:templates` mise task diverges from the established invoke-delegation pattern; every other task in `mise.toml` delegates to a Python invoke task or runs a single one-liner, while the plan proposes a triple-quoted bash heredoc with no precedent. The `producer:` rename is thoroughly applied; the explicit `config-read-template.sh adr` reintroduction honours the template-loader convention.

**Strengths**: test drivers match established convention; canonical template-loader restored in extract-adrs; reproducibility grep matches project practice; task name follows `test:unit:<area>` pattern; correct integration with `test:unit` aggregator.

**Findings**: 1 major (mise task body breaks invoke-delegation convention), 7 minor (`cd "$ROOT"` pattern missing; helper output keys/labels mismatch; filename-timestamp format divergence; comment density inconsistency; depends-list TOML edit shape; plan's own frontmatter still legacy; discovery in SKILL-prose driver).

### Compatibility

**Summary**: The plan introduces a breaking producer-side schema change while explicitly leaving the corpus and several consumers untouched. The resulting divergence window has at least one definite break: every script reading `work_item_id:` from a work-item file (refine-work-item, update-work-item, list-work-items, work-item-common.sh's `wip_is_work_item_file`, and create-work-item's enrich-existing self-check) will fail on artifacts produced by the new template. Other contract changes (visualiser `read_ref_keys`, helper output labels) are mostly resilient via filename fallback or have no external consumers, but the plan does not enumerate these consumer-impact checks.

**Strengths**: explicit acknowledgment of corpus migration boundary; foreign-reference key preserved; ADR own-id rename shielded by visualiser filename fallback; helper output label change is self-contained today; Phase 11 discovery test guards against future divergence.

**Findings**: 2 critical (`work_item_id:` → `id:` breaks consumers; validate-plan inline overrides template), 3 major (supersedes shape break; pr_title not coordinated with review-pr; discovery pass mislabels consumers), 2 minor (ADR filename-fallback rationale; helper output label pinning).

### Portability

**Summary**: ISO format choice is portable across BSD/GNU `date`, and the jj-first / git-fallback structure is sound. However, the test driver's `git init && git commit --allow-empty -m init` requires `user.email`/`user.name` config that CI runners and fresh dev machines often lack. The default-branch name differs by git version. The jj-first probe should be hardened against jj-installed-but-non-jj-cwd cases.

**Strengths**: portable ISO timestamp format; sound jj-first / git-fallback structure; explicit `else` arm for missing VCS; `date -u` eliminates host TZ reliance.

**Findings**: 1 major (`git commit` requires identity), 6 minor (`git init` default branch; jj-installed-non-jj-cwd; jj empty-change at @; CRLF line endings; env isolation; tooling prerequisites undocumented).

---

## Re-Review (Pass 2) — 2026-05-30

**Verdict:** REVISE

Both critical findings from pass 1 are **Resolved**. Of the ~23 prior majors, 16 are resolved (most via explicit edits, several via documented acknowledgement-of-tradeoff with follow-up tracking). 4 prior majors remain partially or fully present, and the edits introduce 4 new majors — including one real bug in the proposed test-driver code (`HOME` reassignment not exported). The plan is structurally close to ready but needs another small pass to land cleanly.

### Previously Identified Issues

#### Critical — both resolved

- ✅ **Compatibility**: `work_item_id:` → `id:` consumer breakage — **Resolved**. Phase 3 §4 now prescribes a transitional dual-key read fallback at `wip_is_work_item_file` and `work-item-read-field.sh`, with bidirectional fallback and a removal milestone tied to 0070. Prose updates in the four consuming SKILL.md files are enumerated. New success criteria (negative-path and bidirectional) added.
- ✅ **Compatibility**: validate-plan inline override during 0065→0066 gap — **Resolved** (via documentation). Phase 6 now carries an explicit caveat block; Migration Notes names this as "the single 'born unified' exception" with explicit ordering guidance for 0066. Acknowledged-tradeoff with a bounded window.

#### Major — resolution status (selected)

- ✅ **Architecture / Code Quality**: Triplicated metadata helpers — **Resolved** (acknowledged tradeoff). Phase 2 §1 now carries an explicit "On the triplication smell" paragraph naming the duplication, justifying scope deferral, and recording a follow-up consolidation story.
- ✅ **Architecture**: Phase independence overstated — **Resolved**. Implementation Approach rewritten to enumerate ordering constraints (Phase 1 first; Phase 2 before 4/5/7/8/9/10; Phase 11 last) with explicit reasoning.
- ✅ **Architecture / Code Quality / Test Coverage / Correctness**: SKILL-prose test too lenient — **Resolved**. Phase 1 §2 now requires fenced-block or imperative-instruction context, with word-boundary disambiguation between `last_updated` and `last_updated_by` and explicit exclusion of template-only inclusion.
- 🟡 **Code Quality**: Silent `jj log … 2>/dev/null || echo ""` failure mask — **Still present**. Phase 2 §1 example bash still contains the silent-fallback pattern under `set -euo pipefail`. The whole-helper test will still pass on a non-VCS path because empty REVISION is allowed there.
- ✅ **Code Quality**: `[ -n ] && echo` under `set -e` — **Resolved in effect** (POSIX exempts the test command in an AND-OR list); pattern retained but the actual exit-on-failure risk for this specific shape is lower than originally framed.
- ✅ **Code Quality**: Three sources of truth for schema table — **Resolved**. `scripts/templates-schema.tsv` is now the single source consumed by both drivers, with an explicit cross-check against the work-item Schema Reference table.
- ✅ **Correctness / Compatibility / Standards**: Helper output label drift between Phase 1 and Phase 2 — **Resolved**. Phase 1 §3 and Phase 2 §1 both now pin `Current Revision:` (no "Hash", no `=`), with label-anchored regex.
- ✅ **Test Coverage / Documentation / Correctness**: Status-comment vocabulary enforcement — **Resolved**. TSV column 5 carries the verbatim vocabulary; `grep -F` enforces AC7.
- 🟡 **Test Coverage**: AC8 (non-tokenised artifact output) verified only manually — **Still present**. Testing Strategy still classifies these tests as predominantly manual. No automated assertion catches a regression that ships `{{placeholder}}` tokens.
- ✅ **Test Coverage / Portability**: Helper test git-only coverage — **Resolved**. New coverage matrix exercises both git (always) and jj (skip-if-absent) branches.
- ✅ **Test Coverage**: Own-identity-vs-foreign-reference assertion underspecified — **Resolved**. TSV `forbidden_own_id_key` column resolves the ambiguity (plan.md keeps `work_item_id:` as foreign reference; only work-item.md and adr.md carry populated forbidden keys).
- ✅ **Correctness**: Phase 11 discovery greps (extract-adrs, validate-plan) — **Resolved**. Timing is now post-Phase-10 explicitly; Pass B grep adds `target:|result:` to surface validate-plan; allowlist union includes non-emitter template consumers.
- ✅ **Documentation**: Status vocab lookup underspecified — **Resolved** via per-template authority table in Phase 1 §1.
- 🟡 **Documentation**: Canonical SKILL.md prose snippet — **Partially resolved**. The snippet now exists under Implementation Approach, but Phases 3, 4, 8, 9, 10 still describe persistence prose in their own ad-hoc voice without literally referencing the snippet. Phase 5 retains "Step 4b" naming; Phase 7's cross-reference points at it but Phase 5 itself does not re-route through the snippet.
- ✅ **Standards**: Mise task heredoc — **Resolved** via `invoke test.unit.templates` delegation form.
- ✅ **Compatibility**: `supersedes:` shape break — **Resolved** via Migration Notes (no known consumers; mixed-shape acceptable until 0070).
- ✅ **Compatibility**: `pr_title` coordination with review-pr — **Resolved** via Phase 7 cross-artifact note + Migration Notes paragraph.
- ✅ **Compatibility**: Discovery pass mislabels consumers — **Resolved** via Phase 11 consumer-side sweep grep.
- ✅ **Portability**: `git commit` identity / default branch / env isolation — **Resolved** via hermetic isolation pattern.

### New Issues Introduced

The edits introduce 4 new majors and ~14 new minors:

#### New Major Findings

- 🟡 **Portability**: `HOME` reassignment is not exported, so git subprocesses still see the host's `~/.gitconfig` — Phase 1 §3 isolation pattern uses `HOME="$tmpdir"` as a bare assignment. Bash does NOT propagate bare assignments to child processes; the subsequent `git ... init` call therefore reads `$HOME/.gitconfig` from the developer's real home directory, defeating the stated isolation goal. **Fix**: change to `export HOME="$tmpdir"` (and also export `XDG_CONFIG_HOME`).
- 🟡 **Correctness**: `\s` / `\S` escapes are PCRE/ripgrep, not POSIX — Phase 1 §1 and §3 use `^schema_version: 1(\s+#.*)?$` and `^Current Revision:\s+\S+$`. POSIX `grep -E` does not interpret `\s`; under BSD/macOS grep these assertions would silently fail to match valid lines. **Fix**: either replace with `[[:space:]]+` / `[^[:space:]]+` everywhere, or state explicitly that the test driver uses `rg` / `grep -P`.
- 🟡 **Correctness**: Phase 1 cross-check reads `meta/work/0065-...md` but plan claims it does not touch `meta/` — Phase 1 §1 specifies a Schema Reference table parser pointed at the work-item file; Phase 11 §1 also *writes* (appends Discovery Pass Record) to the same file. Both contradict the "Touching any file under `meta/`" clause in NOT Doing. **Fix**: narrow the NOT Doing rule to "no edits to existing meta/ artifacts" carving out the explicit Phase 11 append and Phase 1 read, or move the Schema Reference cross-check off the work-item file.
- 🟡 **Test Coverage**: Helper-test isolation uses a single `trap EXIT` but coverage matrix runs multiple helpers — Phase 1 §3 snippet sets `trap "rm -rf '$tmpdir'" EXIT` then `cd "$tmpdir/repo"`. Bash's `trap` is global; re-using the pattern across the coverage matrix's six invocations either overwrites the trap or leaks tmpdirs. **Fix**: scope the isolation as a per-invocation function (subshell + per-call tmpdir cleanup), not a global trap.

#### Notable New Minor Findings

- 🔵 **Documentation**: Inline-comment voice and density still inconsistent across the nine template specifications — the new canonical persistence snippet doesn't extend to template YAML comment-density convention. Phase 3 comments every field; Phases 9-10 carry no per-field comments.
- 🔵 **Documentation**: Canonical persistence snippet defined but Phases 3-10 don't actually reference it — same finding as the partial-resolution above, called out as a remaining doc issue.
- 🔵 **Architecture / Code Quality**: TSV-vs-markdown-table cross-check is a fragile parser — the assertion that parses lines starting with `|` between header and row-count heuristic is brittle to innocuous markdown reformatting. Either generate the markdown table from the TSV, or drop the cross-check.
- 🔵 **Standards**: `tasks/test/unit.py:templates` function spec diverges from sibling style — untyped `ctx`, bare relative paths, undeclared `Exit` import, fail-collect pattern not used by `visualiser`/`frontend`. Should mirror sibling style.
- 🔵 **Standards**: Plan's own frontmatter still uses `skill:` / `work_item_id:` — the plan ships a schema it does not itself follow.
- 🔵 **Standards**: Filename-timestamp format `%Y-%m-%d_%H-%M-%S` (shared) vs `%Y-%m-%d-%H%M%S` (skill-local) still divergent — same label `Timestamp For Filename:`, different shapes.
- 🔵 **Compatibility**: `wip_is_work_item_file` predicate semantics weakened by accepting any `id:` — a future ADR dropped in the work-items dir would now pass the predicate.
- 🔵 **Portability**: `jj init` invocation not pinned across jj CLI version shifts.
- 🔵 **Portability**: TSV is whitespace-fragile (tabs vs spaces); `.gitattributes` or editorconfig rule recommended.
- 🔵 **Test Coverage**: `date`, `last_updated`, `tags` field shapes still presence-checked only at template level; `pr_number:` bare-integer rule unasserted; re-run regen semantics untested.
- 🔵 **Correctness**: Phase 1 §2 fenced-block exclusion mentions "lines inside the resulting rendered template" but the test reads a static file — phantom exclusion.
- 🔵 **Correctness**: TSV rows with empty trailing `forbidden_own_id_key` column are brittle under naive readers; use `-` sentinel or assert `NF == 6`.
- 🔵 **Documentation**: Discovery Pass Record still in the work item rather than the plan.
- 🔵 **Documentation**: "Step 4b" naming convention still not propagated.

### Assessment

The plan has improved substantially. Both criticals resolved, the producer-side schema contract is now genuinely enforceable, the test surface is well-structured, and the cut-line discipline against 0066/0067/0070 is precise. However, the verdict stays **REVISE** because:

1. Three of the new majors are real bugs in the proposed code/contract that a future implementer will hit immediately: `HOME` not exported (will silently leak host config), `\s/\S` not POSIX (will silently fail on BSD grep), and the global `trap EXIT` (will leak tmpdirs across iterations).
2. The "Phase 1 reads `meta/`" inconsistency is small but undermines the NOT Doing rule that other parts of the plan rely on.
3. The canonical SKILL.md snippet is a great addition but lands incompletely — per-phase sections need a one-line "apply the canonical snippet with these parameters" pointer to actually close the documentation review concern.
4. `AC8 manual-only` and the silent jj failure pattern persist from pass 1; both are accepted limitations but should be documented as such if not addressed.

Recommend one more small pass focusing on those four items, then APPROVE.

### Suggested next-pass changes (small)

1. `export HOME="$tmpdir"` (one-character fix to Phase 1 §3) and add `export XDG_CONFIG_HOME=...`.
2. Replace `\s` / `\S` with `[[:space:]]+` / `[^[:space:]]+` in the four regex pin-points, OR state explicitly that the test driver uses `rg` and reject `grep -E`.
3. Rewrite Phase 1 §3 isolation as `run_helper_in_clean_repo()` function (subshell + per-call tmpdir + per-call cleanup) instead of a global `trap EXIT`.
4. Carve out the Phase 1 read and Phase 11 append from the NOT Doing rule (one sentence under What We're NOT Doing or under Phase 1's intro).
5. In each per-template phase (3, 4, 8, 9, 10), replace the ad-hoc "add prose substituting…" paragraph with a single pointer: "Apply the canonical persistence-step snippet (Implementation Approach) with `{type-literal} = …`, `{producer-name} = …`, `{initial-status} = …`, provenance bundle = yes|no." Update Phase 5's Step 4b to use the snippet's naming.

---

## Re-Review (Pass 3) — Spot-check — 2026-05-30

**Verdict:** COMMENT

Narrow spot-check across the four lenses that had remaining majors after pass 2 (portability, correctness, code-quality, documentation). All four pass-2 majors are now **Resolved**. Six minor observations remain — none block implementation; the plan is ready.

### Previously Identified Issues

#### Pass-2 majors — all resolved

- ✅ **Portability**: `HOME="$tmpdir"` not exported — **Resolved**. Phase 1 §3's `run_helper_in_clean_repo()` function uses `export HOME="$tmpdir"` and `export XDG_CONFIG_HOME="$tmpdir/.config"` inside a subshell, with an inline comment justifying the export.
- ✅ **Test Coverage**: Global `trap EXIT` for multi-iteration cleanup — **Resolved**. `trap "rm -rf '$tmpdir'" EXIT` now lives inside the per-invocation subshell; the plan explicitly notes "per-invocation function — NOT a global trap".
- ✅ **Correctness**: `\s`/`\S` PCRE escapes — **Resolved**. All regex pin-points replaced with `[[:space:]]` / `[^[:space:]]`; the new `^schema_version: 1([[:space:]]+#.*)?$` correctly accepts both `schema_version: 1` and `schema_version: 1 # comment` under `grep -E`.
- ✅ **Correctness**: Phase 1 reads `meta/` despite NOT Doing rule — **Resolved**. NOT Doing section now explicitly carves out the Phase 1 read and Phase 11 append with bounded scope ("These carve-outs do not extend to any other `meta/` file").
- ✅ **Code Quality**: Silent jj failure mask — **Resolved**. Phase 2 §1 example bash no longer suppresses jj errors inside the outer jj-root guard; an inline comment explains the rationale.
- ✅ **Documentation**: Canonical snippet not propagated to per-phase sections — **Resolved**. Phases 3, 4, 5, 7, 8, 9, 10 all now read "apply the canonical persistence-step snippet with these parameters: …" with phase-specific `{braces}` parameter substitutions.
- ✅ **Documentation**: Step 4b naming not propagated — **Resolved**. Phase 5 renamed to "Step 5: Populate frontmatter" with explicit annotation that this supersedes the bespoke Step 4b naming.
- ✅ **Test Coverage**: AC8 manual-only — **Resolved via documentation**. Testing Strategy now contains an "Accepted limitation: AC8 verification is manual-only" subsection with rationale and a follow-up path.

### New / Remaining Minor Findings

Six minor observations from the spot-check. All are polish-level and can land in 0065 or be deferred to a follow-up:

- 🔵 **Portability**: Prerequisite tooling versions still undocumented (bash, mise, invoke, git, jj >= 0.20, awk, mktemp, GNU/BSD date semantics). A short Prerequisites subsection would help fresh-clone onboarding.
- 🔵 **Portability**: TSV whitespace fragility has no safeguard. Recommend adding `awk 'NF != 6 { exit 1 }'` self-check at the top of the test driver, plus an `.editorconfig` rule pinning tabs for `*.tsv`.
- 🔵 **Correctness**: TSV trailing-empty-column ambiguity — rows with empty `forbidden_own_id_key` (column 6) are indistinguishable from 5-field rows after an editor's `trim_trailing_whitespace` pass. Recommend using `-` sentinel or asserting `NF == 6`.
- 🔵 **Correctness**: Phase 1 §2 fenced-block exclusion still references "lines inside the resulting rendered template" — that rendered content doesn't exist statically in the SKILL.md, so the exclusion clause is unimplementable as written. Recommend dropping the clause; only the directive line itself needs exclusion.
- 🔵 **Correctness**: Phase 5 plan template uses `work_item_id: "{work-item reference, if any}"` while peers (pr-description, codebase-research) use `work_item_id: ""`. Normalise to the empty-string + optional-comment shape for consistency.
- 🔵 **Documentation**: Inline YAML comment voice/density still varies across template blocks (Phase 3 dense; Phases 5/7/8/9/10 sparser). No comment-style policy was added to Implementation Approach. Worth adding a one-paragraph policy or accepting the divergence explicitly.

### Assessment

The plan is **ready for implementation**. Pass 1 surfaced 2 criticals and ~22 majors; pass 2 confirmed 16/22 resolved and 4 new majors introduced by the edits themselves; pass 3 confirms all those new majors plus the persistent carry-overs are now resolved. The six remaining minors are observations: each is a single-line documentation or robustness improvement that doesn't change the plan's structure, contract, or implementability.

Recommend the implementer pick up the four most useful minors during implementation (TSV `NF == 6` self-check; drop the phantom rendered-template clause; normalise plan.md's `work_item_id` placeholder; add a one-paragraph inline-comment policy) and defer the prerequisite-tooling documentation to a follow-up if it's not already covered by `mise.toml`'s toolchain pin.

---

## Re-Review (Pass 4) — All minors addressed — 2026-05-30

**Verdict:** APPROVE

All six minor observations from pass 3 have been addressed in the plan:

- ✅ **Portability**: Prerequisites subsection added under Implementation Approach naming bash, mise, invoke, git, awk, grep/rg, mktemp, date, gh, and optional jj >= 0.20 with version notes. Confirms toolchain is already pinned in `mise.toml` for everything except `jj`.
- ✅ **Portability**: TSV whitespace fragility — Phase 1 §1 now uses a literal `-` sentinel in the `forbidden_own_id_key` column for the six rows that previously had empty trailing fields, plus a recommendation to add an `.editorconfig` rule pinning `indent_style = tab` and `trim_trailing_whitespace = false` for `*.tsv`.
- ✅ **Correctness**: TSV trailing-empty-column — resolved by the `-` sentinel above; test driver now includes a self-check (`awk -F'\t' 'NF != 6 { exit 1 }'`) that fails fast on any row drift.
- ✅ **Correctness**: Phantom rendered-template clause — clarified in Phase 1 §2 to explicitly note that the directive line is the only thing statically present (rendered template content is a runtime expansion that does not exist in the file at grep-time).
- ✅ **Correctness**: Plan template `work_item_id` placeholder — normalised to `work_item_id: ""` with an updated comment matching the pr-description / codebase-research shape.
- ✅ **Documentation**: Inline-comment policy — new "Template inline-comment convention" subsection under Implementation Approach pinning which fields require comments (`type`, `status`, `schema_version`, foreign references, typed-linkage keys), which are recommended (per-type extras with non-obvious vocabulary), and which are omitted (self-evident base fields).

### Final assessment

The plan has progressed through four review passes:

| Pass | Verdict | Criticals | Majors | Notes |
|------|---------|-----------|--------|-------|
| 1 | REVISE | 2 | ~22 | Initial review across 8 lenses |
| 2 | REVISE | 0 | 4 (new) | 16/22 majors resolved; 4 new majors introduced |
| 3 | COMMENT | 0 | 0 | All majors resolved; 6 minors remain |
| 4 | **APPROVE** | 0 | 0 | All minors addressed |

The plan is ready for implementation. The detailed test-driven contract is enforceable, the consumer-side breakage surface is covered by the Phase 3 §4 fallback, the producer-side cut-line with 0066/0067/0070 is statically fixed, and the canonical persistence-step snippet ensures consistent SKILL.md edits across the seven parallelisable per-template phases.
