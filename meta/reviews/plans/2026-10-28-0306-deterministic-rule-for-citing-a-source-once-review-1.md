---
type: "plan-review"
id: "2026-10-28-0306-deterministic-rule-for-citing-a-source-once-review-1"
title: "Plan Review: Deterministic Rule for Citing a Source Once Implementation Plan"
date: "2026-10-28T15:56:04+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-28-0306-deterministic-rule-for-citing-a-source-once"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "documentation", "standards", "compatibility"]
review_number: 1
review_pass: 2
tags: ["research", "deep-research", "composer", "synthesise", "tiers"]
last_updated: "2026-10-28T16:26:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Deterministic Rule for Citing a Source Once Implementation Plan

**Verdict:** REVISE

The plan puts the citing-once rule in one path-injected contract, and both carriers refer to it by part name. Tests pin the rule's key list to the Rust constant `RECOMMENDED_KEYS` and stop the carriers restating the rule. The correctness lens checked the rule text against every acceptance criterion in the work item and found that each one resolves correctly, and the worked example checks out by hand. Three major findings stand in the way:

- The composer body hard-codes a path to the rule relative to the repo, which does not resolve in an installed plugin.
- The finding outputter's metadata-block rules still contradict the merge.
- The Phase 3 batched fixtures share DOI `10.x/y`, so the rule's own chaining would merge criteria that are meant to stay separate.

### Cross-Cutting Themes

- **Composer hard-codes the rule path** (flagged by: code-quality, compatibility, architecture, standards). The composer names its other injected contract, the finding outputter, by role, and the `${CLAUDE_PLUGIN_ROOT}` path arrives from `conduct`. The plan writes `skills/research/outputters/citing-once/SKILL.md` into step 1, mainly to satisfy `test_every_carrier_reads_the_citing_once_rule`. That path is relative to the repo, so it does not exist in a user's project once the plugin is installed.
- **Phase 3 DOI chaining** (flagged by: test-coverage, correctness). The fixtures keep criteria apart by host only. C2, C3 and S1 reuse `10.x/y` (and `10.X/Y`), so a composer that applies the rule correctly merges separate criteria into one source.
- **Docs site carries an imprecise third copy of the rule** (flagged by: code-quality, architecture, documentation, correctness). The `## Citing a source once` summary leaves out host lowercasing, `dx.doi.org` and the fact that DOIs compare case-insensitively. Only the merged-tier examples are tested, so the summary can drift from the contract.
- **Leftover carry-verbatim wording in the carriers** (flagged by: code-quality, documentation). The `synthesise` paragraph keeps "carrying each finding's tiers and recorded source domains forward". Both carriers paraphrase the rule's sentence on where tiers come from, which `RULE_SIGNATURES` does not catch.
- **Configured `agents.composer` overrides and the open tier-suffix vocabulary** (flagged by: compatibility, architecture). Nothing tells a user's composer override what the injected rule path is for. The outputter still allows "any suffix the profile prescribes", but the rule recognises only `retracted`, `withdrawn` and `also`.
- **The plan departs from the work item's conformance note without saying so, and misdescribes the discovery regexes** (flagged by: standards, test-coverage). The work item asks for conformance to list the rule file, and the plan does not say it is declining to. The plan's description of the discovery regexes also misses the unanchored `DISCOVERY_RE` in the conformance test.

### Tradeoff Analysis

- **Single source of truth vs carrier-level guards**: Code quality wants the carriers to drop their paraphrase of the "every tier comes from a citation" rule. The work item, however, explicitly requires the composer to state that every tier comes from a note. Recommendation: keep that sentence in the carriers as a requirement the work item asks for, and say in the plan that it is deliberate. It is not a restatement of the rule.
- **Docs completeness vs drift**: The documentation and correctness lenses want the docs summary made more precise, which adds detail. Code quality and architecture want it reduced to the outcomes users see, which removes detail. Recommendation: cut it down to one entry per source, the `also` suffix form with examples, and retraction marks kept, then link to the contract. This fixes the inaccuracies and the drift risk in one change.

### Findings

#### Major

- 🟡 **Code Quality / Compatibility / Architecture / Standards**: Composer body hard-codes the rule path, breaking the injected-contract pattern
  **Location**: Phase 2, §3 The composer; §1 `test_every_carrier_reads_the_citing_once_rule`
  Step 1 names `skills/research/outputters/citing-once/SKILL.md` literally so that a test can find it. That path does not resolve from an installed plugin, and it duplicates the path `conduct` injects.
- 🟡 **Documentation**: Finding outputter's metadata-block rules contradict the citing-once rule
  **Location**: Phase 1, §3 The `also` suffix in the finding outputter
  The composer reads `finding-outputter/SKILL.md` first, and its **Sources Metadata Block** section (`:84-103`) says block lines are "unchanged and in emitted order" and that a slot line is never filled "from anything else". The plan edits only the tier paragraph, so the composer receives two contracts that disagree.
- 🟡 **Test Coverage / Correctness**: Batched fixtures share DOI `10.x/y`, so criteria chain into one source
  **Location**: Phase 3: Setup and Fixtures (C2, C3, S1)
  Separate hosts do not keep criteria apart when they share a DOI, because DOI matches chain. Criteria such as "two URLs with two DOIs stay two" or "untouched single citation" become impossible to evaluate.

#### Minor

- 🔵 **Compatibility / Architecture**: Configured `agents.composer` overrides are not told about the new input contract
  **Location**: Phase 2, §2 Inject the rule path
  The spawn passes the rule path without saying what it is for. Neither the configure skill's composer-override guidance nor the docs mention it.
- 🔵 **Compatibility / Architecture**: The rule closes the tier-suffix vocabulary while profiles keep it open
  **Location**: Phase 1, §2 Retraction suffixes
  Under the rule, a future suffix that a profile prescribes has no defined merge behaviour.
- 🔵 **Correctness**: "each followed by `; `" literally produces `tier-3 (retracted; )`
  **Location**: Phase 1, §2 Retraction suffixes
  Read literally, the separator follows every mark, even when no `also` list follows. That contradicts the rule's own examples.
- 🔵 **Correctness / Documentation**: Full ties scope and tie equality are ambiguous
  **Location**: Phase 1, §2 Full ties
  The rule does not limit Full ties to the group at the front of citation order. "Tie on venue" could also be read as equality after case-folding.
- 🔵 **Correctness**: Extracting a DOI from a URL does not handle a trailing slash or percent-encoding
  **Location**: Phase 1, §2 Carrying a DOI
  `doi.org/10.x/y/` yields `10.x/y/`, which does not match `doi: 10.x/y`.
- 🔵 **Documentation / Correctness**: "Every citation in the documents you read" is too broad
  **Location**: Phase 1, §2 preamble
  Taken literally, this would include the example citations in the outputter and in the rule file, and the worked example's `brandur.org` host overlaps the fixtures.
- 🔵 **Code Quality / Architecture / Documentation / Correctness**: The docs summary is an untested, imprecise third copy of the rule
  **Location**: Phase 2, §5 Docs site
  It omits host lowercasing, `dx.doi.org` and case-insensitive DOI comparison. It also simplifies how the representative citation is chosen.
- 🔵 **Documentation**: The docs point at a repo path rather than the generated reference page, and use wording that will date
  **Location**: Phase 2, §5 Docs site
  Link to `reference/skills/internal/citing-once.md`, and drop "is new".
- 🔵 **Code Quality / Documentation**: Retained "carrying ... forward" wording, plus partial restatements in the carriers
  **Location**: Phase 2, §3 and §4
  The `synthesise` paragraph keeps a carry-verbatim signal immediately before the merge instruction.
- 🔵 **Standards / Test Coverage**: The departure from the work item's conformance note is implicit, and the discovery-regex description is inaccurate
  **Location**: Key Discoveries
  `test_conformance.py:68` is unanchored and also matches `frontmatter-emission`, `Populate frontmatter` and `Substitute ... frontmatter`.
- 🔵 **Standards**: Second copy of the recommended keys in a different format
  **Location**: Phase 1, §2 Block order
  The rule file uses a numbered list where the outputter uses a table, so the tests need a second parser, `_numbered_keys`.
- 🔵 **Test Coverage**: Rule-file tests leave several required decisions unpinned
  **Location**: Phase 1, §1 `RULE_SIGNATURES`
  The tests do not pin chaining, `dx.doi.org`, case-insensitive DOI comparison, a missing venue sorting last, ignoring the suffix, `also` counting as more tiers, or "Never add a key".
- 🔵 **Test Coverage**: The composer's read-scope guard and `synthesise` reading the rule first are untested
  **Location**: Phase 2, §1
  Nothing checks that "Never read repository files beyond …" admits the rule.
- 🔵 **Test Coverage**: No test checks that the rule file itself drops byte-for-byte wording, and there is no `synthesise` counterpart to the tier-provenance test
  **Location**: Phase 2, §1 `test_no_carrier_keeps_a_block_byte_for_byte`
  The final acceptance criterion covers the rule file as well as the carriers.
- 🔵 **Test Coverage**: Single runs of heavily batched fixtures are weaker than 0348's three-runs-all-pass method
  **Location**: Phase 3: Fixtures
  C3 packs about 15 criteria into one run.
- 🔵 **Test Coverage**: Diffing whole C5 Sources sections flags entry order, which the rule does not fix
  **Location**: Testing Strategy, Manual step 2
  Compare entries per source.
- 🔵 **Code Quality**: The rule path string is duplicated across two constants
  **Location**: Phase 1, §1
  Derive `CITING_ONCE` from `CITING_ONCE_PATH`.

#### Suggestions

- 🔵 **Code Quality**: The test name and its contents diverge, and rule-part names are spelt out again as literals
  **Location**: Phase 2, §1 `test_every_carrier_confines_its_changes_to_the_rule`
  Split out the slot-line assertion into its own test.
- 🔵 **Architecture**: The rule file's description lists its consumers
  **Location**: Phase 1, §2 frontmatter
  Describe what the contract decides, not who reads it.
- 🔵 **Compatibility**: Give future tier-text parsers a compact grammar
  **Location**: Phase 1, §2 tier-text paragraph
  This gives 0281 and 0284 one form to implement against.
- 🔵 **Standards**: A rule file in `outputters/` without the `-outputter` name
  **Location**: Phase 1, §2
  This is a justified departure, because the work item prescribes the path.
- 🔵 **Documentation**: "Canonical" is used for values but not for order, the reverse of the work item
  **Location**: Phase 1, §2 Block order
  State outright that values are copied exactly, with no case, format or prefix change.

### Strengths

- ✅ One non-invocable contract replaces two inline copies and reuses the established mechanism for injecting a contract's path. Each heading matches a part named in the work item, so the carriers can refer to parts by name.
- ✅ The rule's key list is tested against `RECOMMENDED_KEYS` through `_recommended_keys()`, so when 0349 adds keys, any drift fails loudly.
- ✅ `test_no_carrier_restates_the_citing_once_rule` turns "name parts, restate none" from a convention into an enforced rule.
- ✅ Every acceptance criterion resolves correctly under the rule text, including the named venue `GitHub` beating `github.com` in the eugene-khyst case. The worked example checks out by hand.
- ✅ Each phase starts red. Stale byte-for-byte tests are replaced rather than left contradicting the new ones, and the proposed tests reuse `_prose`, `_section` and `_split` idiomatically.
- ✅ The researcher's "copied unchanged" contract is explicitly left alone, which keeps a clear split: researchers record, composers and `synthesise` merge.
- ✅ The plan verified that no CLI or visualiser code parses tier text, so the `also` form breaks nothing that has shipped. Findings written before the rule stay valid.
- ✅ Phase 3 covers nearly every behavioural criterion, keeps the nine-run and six-run determinism repetitions, and checks the `stream-json` logs for the read and for no `Bash` calls.

### Recommended Changes

1. **Refer to the rule by role in the composer** (addresses: Composer body hard-codes the rule path; Read-scope guard untested)
   Write "the injected citing-once rule" in the description, read scope, step 3 and the guard sentence, with no path. Assert the path only in the `compose` spawn test and in `synthesise`. For the composer, assert the role name in both the "Read first" step and the read-scope guard.
2. **Scope the finding outputter's metadata-block rules** (addresses: Finding outputter contradicts the citing-once rule)
   In **Sources Metadata Block**, limit "copied unchanged and in emitted order" to a researcher writing from CLI output. Add one sentence saying a composed finding's blocks follow the rule's **Metadata merge**, **Full ties** and **Block order**, and pin that sentence with a test.
3. **Isolate Phase 3 criteria by DOI as well as host** (addresses: Batched fixtures share DOI `10.x/y`; Single runs of batched fixtures)
   Give each criterion its own DOI (`10.c01/y` and so on), keeping the case variation under test. Consider running C3 and S1 three times, or splitting C3 into smaller fixtures.
4. **Tighten the rule's wording** (addresses: `; ` separator; Full ties scope; DOI extraction; "documents you read"; "Canonical")
   - Make `; ` separate the parts of a suffix rather than follow each mark.
   - Limit Full ties to citations tied byte for byte with the representative citation, or with a key's first carrier.
   - Take the DOI from the normalised path.
   - Scope gathering to "the notes or findings you cite from".
   - Move the worked example off `brandur.org`.
5. **Decide the fate of unknown tier suffixes, and tell `agents.composer` overrides about the rule** (addresses: Closed suffix vocabulary; Overrides not told)
   Either state an order for marks the rule does not list, or narrow the outputter to the closed set. Label the injected path in the spawn list, and add one line to the configure skill's guidance on composer overrides.
6. **Trim the docs section to outcomes and link to the reference page** (addresses: Docs third copy; Docs path and dated wording)
7. **Clean up the carriers' leftover wording** (addresses: "carrying ... forward")
   Drop the "carrying ... forward" clause from `synthesise`. Keep the tier-provenance sentence, mark it as deliberate, and add a `synthesise` counterpart test.
8. **Widen the structure tests** (addresses: Unpinned decisions; Rule file byte-for-byte untested; Path constant duplication; Test name divergence)
   - Add the missing signatures.
   - Parametrise the stale-phrase test over `CITING_ONCE`.
   - Derive `CITING_ONCE` from `CITING_ONCE_PATH`.
   - Split the slot-line test into its own test.
   - Consider a table for the key list so `_key_column` can be reused.
9. **Record the conformance departure and correct the discovery-regex note** (addresses: Implicit departure; Inaccurate description)
   Add the departure to "What We're NOT Doing", and restate the constraint as "matches none of the three discovery passes", citing each pass's `file:line`.
10. **Compare C5 per source** (addresses: Diffing whole C5 Sources sections)

## Per-Lens Results

### Architecture

**Summary**: The plan puts the citing-once rule in one path-injected contract that both carriers read. This reuses the existing injection pattern and enforces the boundary with negative tests and a key list pinned to Rust. Smaller coupling points remain: a hard-coded location in the composer, a description that names its consumers, a third paraphrase on the docs site, and a closed vocabulary of tier suffixes.

**Strengths**:
- One shared contract replaces two inline copies, using the existing path-injection mechanism.
- `test_no_carrier_restates_the_citing_once_rule` enforces the boundary.
- The key list is pinned to `RECOMMENDED_KEYS`.
- Each heading matches a part named in the work item.
- The researcher-side rule is left explicitly alone.
- The plan gives reasons for departing from the work item on conformance.

**Findings**:
- 🔵 minor (high) — Phase 2 §3 / §1: **Composer definition hard-codes the rule's location, against the injection pattern.** Elsewhere the composer refers to injected contracts by role. The literal path couples the agent definition to the rule file's location and blurs which path is authoritative. Refer to the rule by role, and assert the path only in the spawn test.
- 🔵 minor (medium) — Phase 2 §2: **Citing behaviour moves into the default agent's prose, not the spawn contract.** A custom composer receives the path with no directive. Label the injected path with its role, or update the configure skill's override guidance.
- 🔵 minor (medium) — Phase 1 §2 Retraction suffixes: **The rule closes the tier-suffix vocabulary while profiles keep it open.** Generalise the treatment of marks, or add a test that every suffix a profile prescribes is named in the rule.
- 🔵 minor (medium) — Phase 2 §5: **The docs site keeps an untested third paraphrase of the rule.** Reduce it to the outcomes users see, or extend the docs test.
- 🔵 suggestion (medium) — Phase 1 §2 frontmatter: **The rule file's description lists its consumers.** Describe what the contract decides instead.

### Correctness

**Summary**: Every requirement and acceptance criterion in 0306 resolves correctly under the rule text, the worked example is right, and every string the tests assert is present. The main defect is that the Phase 3 batched fixtures reuse DOI `10.x/y`, so chaining merges separate criteria. There are smaller wording issues in the retraction separator, the scope of Full ties, and which citations to gather.

**Strengths**:
- The worked example was traced by hand and checks out, including the block order doi, openalex_id, published, accessed.
- Every composer and `synthesise` criterion resolves correctly, including the case-fold-then-byte-wise and `GitHub` vs `github.com` cases.
- Normalisation lowercases the host before dropping `www.`, and never rewrites a cited URL.
- Treating an `also` suffix as more tiers makes `synthesise` idempotent.
- The 19 keys match `RECOMMENDED_KEYS` exactly.
- Every positive and negative test string is present or absent as intended after `_prose`.

**Findings**:
- 🟡 major (high) — Phase 3 Setup and Fixtures: **Batched fixtures share DOI 10.x/y, so the rule's chaining merges separate criteria.** C2, C3 and S1 all chain. Give each criterion its own DOI prefix, or run the DOI criteria as separate compositions.
- 🔵 minor (high) — Phase 1 §2 Retraction suffixes: **"each followed by `; `" literally produces `tier-3 (retracted; )`.** Make the separator sit between parts.
- 🔵 minor (medium) — Phase 1 §2 Full ties: **Full ties does not say it only applies to the group at the front of citation order.** A tie group lower in citation order could override the representative citation. Scope it explicitly.
- 🔵 minor (low) — Phase 1 §2 Carrying a DOI: **Extracting a DOI from a URL path does not handle a trailing slash, query or percent-encoding.** Use the normalised path.
- 🔵 minor (medium) — Phase 2 §5: **The docs summary of URL identity leaves out host lowercasing.** It contradicts the first row of the table.
- 🔵 suggestion (low) — Phase 1 §2 opening: **"Every citation in the documents you read" includes the rule's and the outputter's example citations.** Scope it to the notes or findings, and move the worked example off `brandur.org`.

### Test Coverage

**Summary**: Structure tests cover the shape and the wiring, and the one-off headless runs cover behaviour. The work is red-first, and nearly every criterion maps to a fixture. The weaknesses are that DOIs are shared across the batched fixtures, and that the rule-file tests sample only some of the decisions.

**Strengths**:
- Phases start red, and the replaced and deleted tests are named.
- The key list is checked against the Rust constant.
- Carrier tests are parametrised, and there is an inverse test that the carriers do not restate the rule.
- Assertions use `_prose` and exact heading membership.
- Phase 3 keeps the determinism repetitions and checks the logs.
- The unchanged allowlists act as an implicit guard.

**Findings**:
- 🟡 major (high) — Phase 3 Setup and Fixtures: **Batched fixtures isolate criteria by host but share DOIs, so criteria chain into one source.** Extend isolation to identifiers.
- 🔵 minor (medium) — Phase 1 `RULE_SIGNATURES`: **Rule-file tests leave several required decisions unpinned.** Add signatures for chaining, `dx.doi.org`, case-insensitive comparison, a missing venue sorting last, ignoring the suffix, `also` as more tiers, "Never add a key" and "lowest tier".
- 🔵 minor (medium) — Phase 2 §1: **The composer's read-scope admission of the rule is untested.** Assert that the guard sentence names the rule. Check that `synthesise` reads the rule before it writes.
- 🔵 minor (high) — Phase 2 §1: **The acceptance criterion's "rule file retains no byte-for-byte or never-reorder instruction" is not tested against the rule file.** Parametrise over `CITING_ONCE`, and add a `synthesise` tier-provenance test.
- 🔵 minor (medium) — Phase 3 Fixtures: **Single runs of heavily batched fixtures are weaker than 0348's three-runs-all-pass method.** Run C3 and S1 three times, or split them.
- 🔵 minor (low) — Testing Strategy step 2: **Diffing whole C5 Sources sections may flag behaviour the rule does not fix.** Compare per source.
- 🔵 suggestion (medium) — Key Discoveries: **The description of the discovery regexes is inaccurate for conformance.** Restate the constraint accurately, and note the departure.

### Code Quality

**Summary**: The single-source-of-truth design is sound, and the tests reuse the existing helpers. The main problem is that the composer would hard-code a repo-relative path to satisfy a test. Smaller issues are partial restatements of the rule, a duplicated path constant, and a third copy in the docs.

**Strengths**:
- One contract with one heading per part.
- The key list is pinned to `_recommended_keys()`.
- `_numbered_keys` matches the `_key_column` idiom.
- The `CARRIERS` decorator removes repetition.
- The test that carriers do not restate the rule is mechanical.
- There are no explanatory comments.

**Findings**:
- 🟡 major (high) — Phase 2 §3 / §1: **Composer body hard-codes the rule path, breaking the injected-contract pattern.** Refer to the rule by role, and test the path only in the spawn and in `synthesise`.
- 🔵 minor (medium) — Phase 2 §3 / §4: **Carriers partially restate the rule despite the "restate none" goal.** Either drop the paraphrases or mark them as deliberate.
- 🔵 minor (medium) — Phase 2 §5: **The docs summary is a third, simplified copy of the rule with no drift guard.** Keep it to a sentence or two plus the examples and a link.
- 🔵 minor (high) — Phase 1 §1: **The rule path string is duplicated across two constants.** Derive one from the other.
- 🔵 suggestion (medium) — Phase 2 §1: **The test name and its contents diverge, and rule-part names are spelt out again as literals.** Split out the slot-line test.
- 🔵 suggestion (low) — Phase 2 §4: **The retained "carrying each finding's tiers ... forward" reads against the new merge behaviour.** Drop or reword it.

### Documentation

**Summary**: Drift is designed out well, and the rule file mostly reads unambiguously, with a worked example that checks out. The main gap is the finding outputter's Sources Metadata Block section, which still says to copy blocks unchanged. The docs summary has small inaccuracies, wording that will date, and link-convention issues.

**Strengths**:
- The rule has one home, and the carriers name its parts.
- Headings match the work item's part names.
- Abstract statements are paired with examples, and the worked example traces correctly.
- The docs update is pinned by tests and links to the contract.
- The plan accounts for the discovery regexes and the generated reference page.

**Findings**:
- 🟡 major (high) — Phase 1 §3: **The finding outputter's metadata-block rules contradict the citing-once rule for the composer.** Limit them to a researcher writing from CLI output, and point composed findings to the rule.
- 🔵 minor (medium) — Phase 1 §2 Full ties: **"Tie on venue and URL" is ambiguous given case-folded sort keys.** Say the citations are identical as recorded.
- 🔵 minor (low) — Phase 1 §2 preamble: **"Every citation in the documents you read" is broader than intended.**
- 🔵 minor (medium) — Phase 2 §4: **The retained "carrying each finding's tiers ... forward" sits awkwardly beside the merge instruction.**
- 🔵 minor (high) — Phase 2 §5: **The docs summary of source identity omits host lowercasing and `dx.doi.org`.**
- 🔵 minor (high) — Phase 2 §5: **The docs point at a repo path rather than the generated reference page, and use wording that will date.** Link `reference/skills/internal/citing-once.md`.
- 🔵 suggestion (medium) — Phase 1 §2 Block order: **"Canonical" is used for values but not for order, the reverse of the work item and docs.**

### Standards

**Summary**: The plan follows the project's conventions closely: the outputter frontmatter shape, the test helpers, test naming and the parametrised carriers. It departs from local conventions in a few places without saying so: the hard-coded path, a second format for the key list, the conformance note, and an inaccurate description of the discovery regexes.

**Strengths**:
- The frontmatter matches the sibling outputters exactly.
- The tests follow the existing conventions.
- The injection test is extended rather than duplicated.
- Key order is read from the Rust constant.
- Not listing the rule file in the allowlists is consistent with the allowlists' own comments.

**Findings**:
- 🔵 minor (high) — Key Discoveries / Integration Tests: **The departure from the work item's "conformance must list it" note is implicit.** Add it to What We're NOT Doing, and update the work item.
- 🔵 minor (high) — Key Discoveries: **The discovery-regex description does not match the conformance guard.** `DISCOVERY_RE` is unanchored and matches the frontmatter-emission markers.
- 🔵 minor (medium) — Phase 2 §3: **The composer names the rule by a hard-coded repo-relative path, unlike the outputter.**
- 🔵 minor (medium) — Phase 1 §2 Block order: **A second copy of the recommended keys in a different format.** Use a table so `_key_column` can be reused.
- 🔵 suggestion (low) — Phase 1 §2: **A rule file in `outputters/` without the `-outputter` name.** This is a justified departure.

### Compatibility

**Summary**: The change is prose-only and adds to what consumers already handle. No shipped CLI or visualiser code parses tier text, and findings written before 0306 stay valid. The main risk is that the composer prose names a repo-relative path that does not resolve in an installed plugin. Smaller gaps are composer overrides and a closed grammar for tier text.

**Strengths**:
- The reviewer confirmed that nothing in `cli/` parses tier text.
- The key list is pinned to the Rust constant.
- Sets written before 0306 stay valid, and `synthesise` re-merges them.
- The researcher contract is unchanged.
- The skill injects the path through `${CLAUDE_PLUGIN_ROOT}`.

**Findings**:
- 🟡 major (medium) — Phase 2 §3: **The composer prose names a repo-relative rule path that does not resolve in an installed plugin.** Agent bodies do not expand `${CLAUDE_PLUGIN_ROOT}`, so a failed read may leave the composer working without the rule. Refer to the rule by its injected role instead.
- 🔵 minor (medium) — Phase 2: **Configured `agents.composer` overrides are not told about the new input contract.** Update the override guidance in the configure skill and the docs.
- 🔵 minor (medium) — Phase 1 §2 / §3: **The tier-text grammar is closed to `retracted` and `withdrawn`, while the outputter still allows any suffix a profile prescribes.** Define how unknown marks are handled, or narrow the outputter.
- 🔵 suggestion (low) — Phase 1 §2: **Give future tier-text parsers a compact grammar to target.**

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-28T16:22:19+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Code Quality / Compatibility / Architecture / Standards**: Composer body hard-codes the rule path — Resolved
- 🟡 **Documentation**: Finding outputter's metadata-block rules contradict the citing-once rule — Resolved
- 🟡 **Test Coverage / Correctness**: Batched fixtures share DOI `10.x/y` — Resolved
- 🔵 **Compatibility / Architecture**: `agents.composer` overrides not told about the input contract — Resolved
- 🔵 **Compatibility / Architecture**: Rule closes the tier-suffix vocabulary while profiles keep it open — Resolved
- 🔵 **Correctness**: "each followed by `; `" literally produces `tier-3 (retracted; )` — Resolved
- 🔵 **Correctness / Documentation**: Full ties scope and tie equality ambiguous — Resolved
- 🔵 **Correctness**: DOI extraction from a URL ignores trailing slash and query — Resolved
- 🔵 **Documentation / Correctness**: "Every citation in the documents you read" too broad — Resolved
- 🔵 **Code Quality / Architecture / Documentation / Correctness**: Docs summary is an untested third copy — Resolved
- 🔵 **Documentation**: Docs point at a repo path and use dated wording — Resolved
- 🔵 **Code Quality / Documentation**: Retained "carrying … forward" wording — Resolved
- 🔵 **Standards / Test Coverage**: Conformance departure implicit; discovery-regex description inaccurate — Resolved
- 🔵 **Standards**: Second copy of the recommended keys in a different format — Resolved
- 🔵 **Test Coverage**: Rule-file tests leave decisions unpinned — Resolved
- 🔵 **Test Coverage**: Composer read scope and `synthesise` read order untested — Resolved
- 🔵 **Test Coverage**: Rule file not tested for stale byte-for-byte wording; no `synthesise` provenance test — Resolved
- 🔵 **Test Coverage**: Single runs of batched fixtures — Resolved
- 🔵 **Test Coverage**: Whole-section C5 diff — Resolved
- 🔵 **Code Quality**: Rule path string duplicated — Resolved
- 🔵 **Code Quality**: Test name and contents diverge; part names as literals — Partially resolved (slot-line test split; part names still literals)
- 🔵 **Architecture**: Rule file description lists its consumers — Still present (accepted: matches sibling outputters)
- 🔵 **Standards**: Rule file in `outputters/` without `-outputter` name — Still present (accepted: path set by work item)

### New Issues Introduced

- 🔵 **Architecture / Correctness**: Phase 1's outputter `also` paragraph tells the unchanged composer to cite once — Phase 1 still claims no behaviour change; move the paragraph to Phase 2 with the metadata-block scoping.
- 🔵 **Code Quality / Standards**: New `PROFILES` constant duplicates `PROFILES_DIR`, `_profiles()` and `_profile_skill`, with a looser glob.
- 🔵 **Code Quality**: `synthesise` restates the rule's in-finding duplicate scope while the plan omits `also` for that reason — keep as a deliberate guard or drop.
- 🔵 **Standards**: Unwrapped ~130-column line in **Carrying a DOI**.
- 🔵 **Documentation**: Docs `Order` bullet says the CLI emits no other key, then orders other keys without saying where they come from.
- 🔵 **Test Coverage**: No guard on either carrier deleting an empty slot line.
- 🔵 **Test Coverage**: Prescribed-mark test passes vacuously if profile phrasing drifts.
- 🔵 **Test Coverage**: Phase 3 log check covers the composer reading the rule but not `synthesise`.
- 🔵 **Test Coverage** (suggestion): Three Phase 2 guard tests never go red; the "fail first" criterion overstates them.
- 🔵 **Test Coverage** (suggestion): C4 URL rows run once each.
- 🔵 **Correctness** (suggestion): DOI host test should name the normalised host.
- 🔵 **Documentation** (suggestion): "Nothing in the CLI or visualiser parses tier text" will date.
- 🔵 **Standards** (suggestion): Docs link text should be backticked; the two unit discovery passes have different marker sets.
- 🔵 **Code Quality** (suggestion): Carrier pair spelt out three times; part names not tied to `RULE_PARTS`; docs `Order` bullet restates **Block order**.
- 🔵 **Compatibility** (suggestion): Worked example keeps a schemeless URL as the merged entry's link.

### Assessment

All three major findings and every actionable minor from pass 1 are resolved; the correctness lens re-traced every acceptance criterion and the worked example against the edited rule and found them sound. What remains is minor: one phasing inconsistency in Phase 1, test-module hygiene, a few guard gaps, and docs wording. The plan is ready for implementation; the new minors are worth a quick tidy first.

**Final verdict:** APPROVE. The pass-2 minors were applied to the plan (C4 rows 1 and 5 repeated three times), and the plan is marked `ready`.
