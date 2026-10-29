---
type: "pr-review"
id: "173-review-1"
title: "[0306] Cite each research source exactly once"
date: "2026-10-29T10:40:52+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:173"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses:
  - architecture
  - correctness
  - code-quality
  - test-coverage
  - documentation
  - compatibility
  - standards
review_number: 1
pr_number: 173
tags: []
last_updated: "2026-10-29T10:40:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #173 - [0306] Cite each research source exactly once

**Verdict:** COMMENT

The citing-once rule is internally consistent. Two lenses traced every example in it by hand and found no error, and **Full ties** covers exactly the cases citation order cannot separate, so the merged entry does not depend on input order. The extraction into one injected contract is sound, as are the negative fitness tests and the unchanged composer confinement. The gaps are at the edges:

- The CHANGELOG still announces the behaviour this PR removes.
- The closed retraction-mark list fails silently when a profile adds a mark.
- The loosened recipe guard fails silently on a likely mis-edit.
- The behavioural evidence was not kept, and part of it predates the last wording change.

### Cross-Cutting Themes

- **Change not announced to upgraders** (flagged by: standards, documentation, compatibility). `CHANGELOG.md:202-204` under `[Unreleased]` still says "the composer and `synthesise` carry each block byte-for-byte", and nothing records the `also` suffix or the new obligation on an `agents.composer` override. The `configure` warning is the only signal an existing override gets. One cloned from the old `agents/composer.md` will ignore the injected rule path without any error.
- **Closed retraction-mark list** (flagged by: architecture, compatibility, test-coverage). **Retraction suffixes** keeps only `retracted` and `withdrawn`, while the finding outputter still allows "any suffix the profile prescribes". `_prescribed_marks` matches only `[a-z]+`, so a multi-word or hyphenated mark would be stripped from every composed entry with CI green.
- **Recipe guard loosening** (flagged by: correctness, test-coverage, architecture, code-quality, documentation, compatibility). The guard has four gaps:
  - `extends` applies to every history entry, so a superseded recipe can grow.
  - A dropped version on the current recipe is reported as "an earlier recipe's history entry was rewritten".
  - A scalar `pulldown_cmark` silently parses to `[]`.
  - Nothing says where `DigestRecipe` lives that a green golden corpus is now enough to admit a parser version.
- **Duplicates outlive composition** (flagged by: documentation, compatibility, correctness). Three kinds of entry can keep one source cited twice on disk: depth-1 findings (the researcher writes them), pre-0306 findings, and entries where the composer dropped the URL that linked two citations. The docs don't say so, and the parsers 0281 and 0284 will add must tolerate it.
- **Rule part names are a cross-file interface** (flagged by: architecture, code-quality). Three carriers enumerate the rule's parts as closed lists. The test literals naming them are not tied to `RULE_PARTS`, so renaming a heading leaves dangling references with every test green.

### Tradeoff Analysis

- **One owner for the rule vs explicit pointers in each carrier**: architecture wants each carrier to say only that the citing-once rule is its only change, with the rule owning both closures. The current enumeration gives the LLM explicit section pointers and is what the tests pin. Recommendation: move the block closure into the rule beside its tier-text closure (`citing-once/SKILL.md:78`) and keep one pointer per carrier, since the composer reads the rule anyway.
- **Deterministic CI vs behavioural evidence**: test coverage wants the Phase 3 fixtures, oracle and checker committed. The plan chose no retained eval, keeping `mise run check` deterministic and offline. Recommendation: commit them as an on-demand eval outside `check`, which gives both.
- **Parser-upgrade friction vs baseline safety**: appending a version avoids a recipe bump that would cost older binaries their sync baselines, which compatibility credits. Architecture and compatibility both note that it now rests on the golden corpus covering every parse path a release can change. Recommendation: keep the version list, restrict growth to the latest entry, and write the joining condition down.
- **Open mark grammar vs forced rule edits**: keeping unknown marks in byte-wise order after `withdrawn` stays deterministic. A closed list forces a rule edit for each new mark, which is safe only if a test notices the new mark. Recommendation: widen the test regex now, and decide the grammar when a profile first needs a new mark.

### Strengths

- ✅ One non-invocable contract with a heading per part replaces two inline carry-forward copies. Carriers refer to parts by name, and `test_no_carrier_restates_the_citing_once_rule` reuses `RULE_SIGNATURES` to check that no carrier restates the rule.
- ✅ The rule's key table is pinned to the Rust `RECOMMENDED_KEYS` through `_key_column` and `_recommended_keys`, so block order cannot drift from what the CLI emits.
- ✅ Every inline and worked example matches the rule as written: the URL-identity table, venue and URL ordering, **Full ties**, both merged-tier examples and the three-citation worked example.
- ✅ The composer still grants only `Read` and `Write`. Its read guard grows by one named contract, and the `configure` warning states the new obligation.
- ✅ The `also` suffix is forward-compatible. Treating an existing `also` as more tiers makes re-running `synthesise` idempotent, and no shipped code parses tier text yet.
- ✅ `DigestRecipe::CURRENT` stays 1, so sync baselines stay readable across mixed binary versions. The guard still rejects a history that drops a version, and its error tells the developer what to do.
- ✅ The docs link to the generated `reference/skills/internal/citing-once` page instead of copying the rule. The stale byte-for-byte wording is gone from every shipped surface, and a test keeps it out.
- ✅ The comment policy holds. The `recipe_guard` doc comment gave way to a named `extends` predicate, and the `Cargo.toml` pin comment was rewritten rather than left stale.
- ✅ `test_the_profiles_prescribe_the_retraction_marks` stops the mark-merging test passing vacuously. Phase 3 acceptance predicted outputs before any session ran, and planted mutations to show the checker could fail.

### General Findings

- 🟡 **Standards, Documentation**: `CHANGELOG.md` `[Unreleased]` contradicts the shipped behaviour, because lines 202-204 still say the composer and `synthesise` carry each block byte-for-byte. Reword that 0348 bullet. Then add an entry for citing once that covers:
  - the `also` suffix, with examples;
  - that retraction marks are kept;
  - that an `agents.composer` override must read the citing-once rule;
  - that existing findings stay valid.
- 🟡 **Test Coverage**: the behavioural acceptance evidence was not kept. The 36 sessions' fixtures, predicted entries, oracle and checker live in a scratchpad. After the next edit to the rule or a carrier, or a model change, the behaviour cannot be checked again. Commit them as an on-demand eval beside the rule (e.g. `skills/research/outputters/citing-once/evals/`), outside `mise run check`.
- 🔵 **Correctness**: `templates/topic-research-synthesis.md:26-27`, which the PR does not touch, still tells the Findings section to carry "each source's reputation tier and recorded domain forward". With merged tiers, the prose can call a source tier-1 while its Sources entry reads `tier-3 (also tier-1)`. Point it at the Sources entry, or drop the tier clause.
- 🔵 **Compatibility**: pre-0306 and post-0306 findings coexist on disk indefinitely. Record in 0281 and 0284 that tier-text parsing must accept both forms and tolerate duplicate entries.
- 🔵 **Architecture**: no code owns the tier-text grammar, and its consumers are multiplying. When 0284 adds the first parser, give the grammar one owner in `cli/` whose test cases are the rule's examples.

### Additional Findings

- 🔵 `docs-site/src/content/docs/research.md:789` — Docs don't say depth-1 findings stay unmerged (documentation)
- 🔵 `skills/config/configure/SKILL.md:281` — Existing composer overrides get a new required input with no upgrade signal (compatibility)
- 🔵 `skills/research/outputters/citing-once/SKILL.md:170` — Worked example and URL table are unchecked against the rule (test-coverage)
- 🔵 `agents/composer.md:29` — The composer's "Never add a source" invariant is unpinned (test-coverage)
- 🔵 `cli/work-adapters/tests/golden_digests.rs:327` — Length check and any-version match have no distinguishing case (test-coverage)
- 🔵 `tests/unit/tasks/test_research_structure.py:1506` — Carrier readers, ids and origins live in parallel tuples paired by position (code-quality)
- 🔵 `tests/unit/tasks/test_research_structure.py:1576` — No-restatement check omits the finding outputter (test-coverage)
- 🔵 `skills/research/outputters/finding-outputter/SKILL.md:84` — Finding outputter accumulates producer-specific branches (architecture)
- 🔵 `docs-site/src/content/docs/research.md:791` — "Lowest tier" is undefined on the docs page, and no reason is given (documentation)
- 🔵 `docs-site/src/content/docs/research.md:795` — Docs say "contract" where every shipped skill says "rule" (standards)
- 🔵 `cli/work-adapters/tests/golden_digests.rs:303` — Remediation message doesn't name `recipe.json` (documentation)
- 🔵 `skills/research/outputters/citing-once/SKILL.md:32` — Sentence-case headings differ from sibling contracts (standards)

---
*Review generated by /review-pr*

## Inline Comments

### `skills/research/research-topic/SKILL.md:647` — Read-tool fix for synthesise is pinned only by its wording, not re-run
**Severity**: major | **Confidence**: medium | **Lens**: test-coverage

🟡 **Test Coverage**

The new wording "Read the citing-once rule with the Read tool" fixes a behaviour seen in 8 of 9 Phase 3 `synthesise` sessions, which read the rule with `Bash` `cat`. Its only regression check is `test_synthesise_reads_the_citing_once_rule_with_the_read_tool`, which pins the phrase. The earlier wording already began "Read the citing-once rule at …" and still produced `cat`. Nobody has checked whether the added words change the behaviour, and the PR's Testing checklist leaves this item unticked.

**Impact**: If the model keeps using `cat`, a user whose permissions don't allow it gets a permission prompt partway through `synthesise`, and no test or recorded run would show it.

**Suggestion**: Re-run the S1 fixture (3 sessions) against this wording. Confirm from the `stream-json` logs that each session reads the rule with `Read` and makes no `Bash` call. Then tick the checklist item and record the result in the plan's As built notes.

---

### `cli/work-adapters/tests/golden_digests.rs:276-282` — Rewritten-entry check lets a superseded recipe grow and misnames a current-recipe edit
**Severity**: minor | **Confidence**: high | **Lens**: correctness, test-coverage, code-quality, documentation

🔵 **Correctness**

`extends` applies to every recorded entry. So after a recipe bump, a history like `[r1: [0.13.4, 0.13.5, 0.14.0], r2: [0.14.0]]` passes although r1 was edited, and r1 is never checked against the lock again. The PR notes call this harmless today. It still contradicts this branch's own error text and the invariant `recorded_history` documents.

🔵 **Test Coverage**

No case in `the_recipe_guard` covers a version appended to an earlier recipe. The behaviour is neither pinned as intended nor rejected.

🔵 **Code Quality, Documentation**

This branch now fires for three causes, all reported as "an earlier recipe's history entry was rewritten":

- a shortened history;
- a changed recipe number;
- a dropped or replaced parser version.

The last cause also hits the *current* recipe: the new "recorded version dropped" case is recipe 1 with recipe 1 current. And replacing a version instead of appending one is now the likeliest mistake in the parser-bump workflow this PR introduces.

**Suggestion**: Allow `extends` only for the latest entry, require every earlier entry to equal its recorded value, and report the two failures separately:

```rust
history.iter().zip(recorded).enumerate().all(|(at, (entry, recorded))| {
    if at + 1 == history.len() {
        extends(entry, recorded)
    } else {
        entry == recorded
    }
})
```

Add a failing case `"version added to an earlier recipe"` with `vec![record(1, &["0.13.4", "0.13.6"]), record(2, &["0.13.5"])]`. Give the dropped-version failure its own message, such as "recipe {n} no longer lists pulldown-cmark {v}; append parser versions, never replace one".

---

### `skills/research/outputters/citing-once/SKILL.md:70-79` — Closed mark list silently drops any tier mark a profile adds later
**Severity**: minor | **Confidence**: high | **Lens**: architecture, compatibility, test-coverage

🔵 **Architecture**

**Retraction suffixes** names only the two marks today's profiles prescribe: `retracted` from `openalex-profile` and `withdrawn` from `arxiv-profile`. Meanwhile `finding-outputter/SKILL.md:81-82` still lets tier text carry "any suffix the profile prescribes". So the generic merge rule now depends on vocabulary the profiles own. Line 78 makes **Merge rule** and **Retraction suffixes** the only permitted changes to tier text, so any other mark is dropped, even from a source cited once.

🔵 **Compatibility**

Suppose a profile later prescribes `tier-2 (preprint)` or `tier-3 (expression of concern)`. That mark would be stripped from every composed finding and dossier.

🔵 **Test Coverage**

`_prescribed_marks` (`tests/unit/tasks/test_research_structure.py:1471`) matches only `[a-z]+`. The guard meant to catch this misses multi-word and hyphenated marks, which are the likeliest shapes of a new one.

**Suggestion**: Widen the regex to capture the whole suffix, `` r"`tier-[123] \(([^)`]+)\)`" ``, split on `; `, so every new mark forces a rule edit. Optionally, let the rule keep any non-`also` part it does not name, in byte-wise order after `withdrawn` and before the `also` list. That stays deterministic without the rule listing each profile's vocabulary.

---

### `cli/work-adapters/tests/golden_digests.rs:218-226` — `pulldown_cmark` parses leniently and no longer matches its field name
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, test-coverage, standards

🔵 **Code Quality, Test Coverage**

`entries(entry, "pulldown_cmark")` is `as_array().cloned().unwrap_or_default()`, a helper built for optional collections. The pre-PR scalar shape, which is the natural thing to write for a `recipe: 2` entry, becomes `[]` silently, and so does a mistyped or missing key. The code it replaced panicked instead, and the sibling `recipe` field still uses `expect`. The guard then fails with a message that points away from the malformed JSON: either "recorded under pulldown-cmark , but 0.13.6 is locked" or "an earlier recipe's history entry was rewritten".

🔵 **Standards**

Every other fixture key in this file maps one-to-one onto a field of the same name. `pulldown_cmark` now holds a list that is read into `pulldown_cmark_versions`.

**Suggestion**: Parse strictly:

```rust
entry["pulldown_cmark"]
    .as_array()
    .unwrap_or_else(|| panic!("pulldown_cmark is a list of versions in {entry}"))
```

Then rename the fixture key to `pulldown_cmark_versions` while there is only one history entry, or keep the Rust field named `pulldown_cmark`.

---

### `tests/unit/tasks/test_research_structure.py:1539-1547` — Rule-part names referenced across three shipped files are not tied to `RULE_PARTS`
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

The bolded part names (`**Merge rule**`, `**Retraction suffixes**`, `**Metadata merge**`, `**Full ties**`, `**Block order**`) are the interface between the citing-once rule and the three files that refer to it: `agents/composer.md`, `synthesise`, and the finding outputter. These tests spell the names out as fresh literals, here and in `test_the_finding_outputter_defers_composed_blocks_to_the_rule`, independent of `RULE_PARTS`. Suppose someone renames `Full ties` to `Tie-breaks` in the rule and in `RULE_PARTS` but not in the carriers. Every test still passes, and the carriers point the model at a section that no longer exists.

**Suggestion**: Name the two groups once and check them against `RULE_PARTS`:

```python
TIER_TEXT_PARTS = ("Merge rule", "Retraction suffixes")
BLOCK_PARTS = ("Metadata merge", "Full ties", "Block order")
assert set(TIER_TEXT_PARTS + BLOCK_PARTS) <= set(RULE_PARTS)
```

Then build the carrier and outputter phrases from those tuples.

---

### `tests/unit/tasks/test_research_structure.py:1413-1431` — Several rule decisions survive mutation of `RULE_SIGNATURES`
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`RULE_SIGNATURES` pins some decisions only by a key term, which still appears if the rule is reversed, and leaves others unpinned. Each of these edits to `citing-once/SKILL.md` passes every test:

- reversing "A named venue sorts before a bare domain";
- reordering "by tier, lowest first, then by venue, then by URL";
- flipping "Tiers rank `tier-3` lowest";
- deleting the representative-citation or first-carrier sentence;
- dropping "drop its fragment" or "drop the path's trailing slash".

"case-folded, then byte-wise" appears under both Venue and **Full ties**, so it can be removed from either one without a test failing.

**Impact**: These decisions drive source identity and representative choice, which 0281 and 0284 will consume.

**Suggestion**: Add phrases that fix each decision's direction, e.g. `"A named venue sorts before a bare domain"`, `"by tier, lowest first, then by venue, then by URL"`, `"The first citation in citation order is the representative citation"` and `"drop the path's trailing slash"`. The same tuple feeds `test_no_carrier_restates_the_citing_once_rule`, so longer phrases strengthen both checks.

---

### `agents/composer.md:31-34` — Carriers enumerate the rule's internal parts as closed lists
**Severity**: minor | **Confidence**: high | **Lens**: architecture

🔵 **Architecture**

The composer lists the rule's parts as closed sets: "**Merge rule** and **Retraction suffixes** are the only changes … **Metadata merge**, **Full ties** and **Block order** are the only changes". `synthesise` repeats this at `research-topic/SKILL.md:652-655`, and so does the finding outputter at `:111-113`. The rule already owns the tier-text closure (`citing-once/SKILL.md:78`) but not the block closure, so its internal structure has spread into every carrier and into `test_every_carrier_confines_its_changes_to_the_rule`.

**Impact**: Renaming, splitting or adding a part means editing the rule, three carriers and the tests together. 0281's `report` is planned to apply the rule and would copy the sentence a fourth time.

**Suggestion**: Put the metadata-block closure in the rule next to line 78. Each carrier then says only that the citing-once rule is the only change it makes to tier text or a metadata block, and the closure sentences are pinned once, in the rule's tests.

---

### `skills/research/outputters/citing-once/SKILL.md:54-58` — DOI taken from a `doi.org` URL is not percent-decoded, but the CLI percent-encodes those URLs
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

**Carrying a DOI** takes the raw path of a `doi.org` or `dx.doi.org` URL as the DOI. The CLI's citation URLs are percent-encoded, though: `Doi::url()` encodes everything outside the RFC 3986 unreserved set and `/` (`cli/research/src/sources/request.rs:401-408`), while `Doi::parse` percent-decodes. So `10.1016/S0140-6736(20)30183-5` is cited as `https://doi.org/10.1016/S0140-6736%2820%2930183-5`. Suppose two citations of that work carry no `doi` key, one with an encoded URL and one without. They yield different DOIs and different normalised URLs, and stay two sources. The work item relies on this path, because pre-0348 findings carry a DOI only through their URL.

**Impact**: When `synthesise` re-merges pre-0348 findings, or merges web citations without DOI metadata, a source whose DOI contains `(`, `)`, `;`, `:`, `<` or `>` can be cited twice.

**Suggestion**: Percent-decode the path before comparing, e.g. "that URL's path, without its leading slash and percent-decoded, is the DOI". Add an example pair: `doi.org/10.1016/S0140-6736%2820%2930183-5` and `doi: 10.1016/s0140-6736(20)30183-5`.

---

### `skills/research/outputters/citing-once/SKILL.md:103-105` — Merged entries drop non-representative URLs, so `synthesise` can re-split a source the composer merged
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

A merged entry keeps only the representative citation's URL, and **Metadata merge** keeps only the first carrier's `doi`. The URLs, and any second DOI, that linked the other citations into the group are discarded. Example: a composer merges `doi.org/10.x/y` (tier-3) with `pub.example/p` plus `doi: 10.x/y` (tier-1) and keeps the doi.org URL. Another finding cites `https://pub.example/p/` with no block. That citation matches neither the kept URL nor a DOI. So applying the rule twice, in the composer and then in `synthesise`, can group citations differently from applying it once over all of them.

**Impact**: The dossier can list one source twice even though some finding linked every citation of it. 0281's `report` inherits the same behaviour.

**Suggestion**: Either state that identity is judged only over the entries the carrier reads and accept the gap, or keep what formed the group in the merged entry, for example every distinct DOI. Record the choice so 0281 inherits it deliberately.

---

### `cli/Cargo.toml:117-118` — Parser-upgrade policy now rests on the golden corpus alone
**Severity**: minor | **Confidence**: medium | **Lens**: compatibility, architecture

🔵 **Compatibility**

This comment and the guard's new message drop the rule that a pulldown-cmark release arrives with a recipe bump. Any release now joins recipe 1 if the golden pairs still hash identically. But persisted sync baselines hash pulldown-cmark's parse, and the corpus is only a sample. A release that changes how a construct outside the corpus parses would move existing baselines without a recipe change; this comment itself calls SoftBreak offsets observed behaviour. The next sync would then report spurious changes or conflicts. 0.13.5 itself looks safe, provided the RUSTSEC-2026-0414 fix is confined to wikilink handling, which is never enabled.

🔵 **Architecture**

A recipe used to mean "digests under this exact parser". It now means "digests the fixtures show unchanged across these parsers". That is a reasonable trade, but the assumption is not written where the recipe's contract lives: `DigestRecipe::CURRENT` in `work-adapters/src/sync/baseline.rs:22-25` says only "Bumped by any change that moves a recorded digest".

**Suggestion**: State the joining condition in full, here and in the `DigestRecipe` doc comment: the upstream diff touches only disabled extensions or non-parse code, and every golden digest still holds.

---

## Per-Lens Results

### Architecture

**Summary**: The PR moves the citing-once behaviour into one non-invocable contract, injected by path, which both carriers read by role. It keeps the composer confined to `Read` and `Write`, and adds negative fitness tests so the carriers cannot drift back to restating the rule. The remaining architectural pressure comes from how the design will evolve. Each carrier lists the rule's internal parts, the rule hard-codes mark vocabulary that the profiles own, and the finding outputter is collecting producer-specific branches. All three get worse once 0281's `report` becomes a third carrier. On the CLI side, the digest-recipe guard now quietly treats one recipe as covering several parser versions.

**Strengths**:
- A single shared contract replaces two inline copies of the carry-forward rule, reusing the `${CLAUDE_PLUGIN_ROOT}` path-injection pattern; the composer refers to it by role.
- Boundary fitness functions (`test_no_carrier_restates_the_citing_once_rule`, `test_no_contract_keeps_a_block_byte_for_byte`) guard against rule content leaking back into carriers.
- The fourth copy of the recommended-key table is pinned to `RECOMMENDED_KEYS`. The rule therefore stays self-contained for `synthesise`, which never reads the outputter, and the copies cannot drift.
- The composer's security boundary is unchanged, and `configure` documents the new obligation on an override.
- Treating an existing `also` suffix as more tiers makes re-running `synthesise` idempotent, so existing sets need no migration.
- The docs link to the generated reference page instead of keeping a third paraphrase.
- The recipe history now records a list of parser versions per recipe, so a parser security patch no longer forces a digest-recipe bump.

**Comments**:
- `agents/composer.md:31-34` (RIGHT) — minor, high — Carriers enumerate the rule's internal parts as closed lists. Inline above.
- `skills/research/outputters/citing-once/SKILL.md:70-79` (RIGHT) — minor, medium — Generic merge rule hard-codes profile-owned suffix vocabulary. Merged into the inline "Closed mark list" comment.
- `skills/research/outputters/finding-outputter/SKILL.md:84-87` (RIGHT) — suggestion, medium — Finding outputter accumulates producer- and artefact-specific branches. The outputter claims to own "the finding's shape end to end", but now describes what a dossier does (an artefact it does not own, and which `synthesise` never reads from it) and how a composer writes blocks (already stated in `composer.md` and the rule). Keep this section to the artefact's grammar with the two examples, and let `composer.md` and the rule own production.
- `cli/work-adapters/tests/golden_digests.rs:298-309` (RIGHT) — minor, medium — Digest recipe semantics shift implicitly to fixture-proven parser equivalence. Merged into the inline `Cargo.toml:117-118` comment (the policy part) and the inline `golden_digests.rs:276-282` comment (the earlier-recipe extension part).

**General Findings**:
- suggestion — The tier-text grammar has no code owner as its consumers multiply. 0281's `report` and `ask`, and 0284's set-detail page, will each consume `tier-3 (retracted; withdrawn; also tier-1)`. When 0284 adds the first parser, give the grammar one owner in `cli/` whose test cases are the rule's examples.

### Correctness

**Summary**: The citing-once rule holds together. Every inline example checks out against the rule as written: the URL-identity table, the venue and URL ordering examples, the **Full ties** examples, both merged-tier examples and the worked example. **Full ties** covers exactly the cases citation order cannot tell apart, so title, venue, URL and each metadata value are decided the same way regardless of input order. The remaining gaps are at the edges. A DOI read from a URL is not percent-decoded, although the CLI percent-encodes the doi.org URLs it emits. Merged entries drop the URLs of non-representative citations, so `synthesise` cannot always re-link a source across findings. And the loosened recipe guard, which the PR already acknowledges, has a simple tightening.

**Strengths**:
- Every worked and inline example agrees with the rule text. Each was traced by hand: the URL table, the venue, URL and **Full ties** examples, and the worked example's merged tier, representative citation, merged keys and canonical key order.
- **Full ties** is exactly the set of cases citation order cannot separate, so every choice comes out the same whatever the input order.
- URL normalisation lowercases the host before dropping `www.`. DOI extraction works on the normalised URL, so `www.doi.org`, trailing slashes, queries and fragments need no extra rules.
- The claim that a retracted or withdrawn source is always tiered `tier-3` checks out against the openalex and arxiv profiles, so the mark-ordering rule cannot conflict with **Merge rule**.
- The rewritten recipe guard still rejects a history that drops or reorders a version, or has an empty version list. The `history.len() < recorded.len()` check keeps the prefix semantics of the old `starts_with`.

**Comments**:
- `skills/research/outputters/citing-once/SKILL.md:54-58` (RIGHT) — minor, medium — DOI taken from a doi.org URL is not percent-decoded. Inline above.
- `skills/research/outputters/citing-once/SKILL.md:103-105` (RIGHT) — minor, medium — Merged entries drop non-representative URLs. Inline above.
- `cli/work-adapters/tests/golden_digests.rs:276-282` (RIGHT) — minor, high — Recipe guard lets an earlier recipe's version list be extended. Merged into the inline rewritten-entry comment.

**General Findings**:
- minor — `templates/topic-research-synthesis.md:26-27` still asks the Findings prose to carry each source's single tier forward. A source can now have several tiers, which only the Sources entry's merged tier text resolves, so the dossier can contradict itself. Point it at the Sources entry, or drop the tier clause.

### Code Quality

**Summary**: The shipped surface is well structured for maintenance. One non-invocable contract has a heading per part, the carriers refer to parts by name, and mechanical guards stop them restating the rule. The key table is pinned to `RECOMMENDED_KEYS`. The reworked recipe guard is split into a named `extends` predicate and gives an actionable error. The findings are all minor. The recipe-history parser silently accepts a malformed `pulldown_cmark` value that the old code rejected. The "earlier recipe rewritten" message now covers mistakes made in the current recipe. In the structure tests, cross-file part-name references are not tied to `RULE_PARTS`, and each carrier's attributes sit in parallel tuples paired by position.

**Strengths**:
- One injected contract with one `##` heading per part, and carriers that refer to it by name. `test_no_carrier_restates_the_citing_once_rule` reuses `RULE_SIGNATURES` as a check against restating the rule.
- The key table is checked against Rust `RECOMMENDED_KEYS` through the existing helpers.
- The `CARRIERS` decorator and the reuse of `_prose`, `_section` and `_split` keep the new test block consistent with the module.
- The recipe guard reads in domain terms, and the locked-version error states the remedy.
- `the_recipe_guard` gains both the new happy path and its matching failure.
- No explanatory comments were added, and the `Cargo.toml` comment was updated.

**Comments**:
- `cli/work-adapters/tests/golden_digests.rs:218-226` (RIGHT) — minor, high — Recipe-history parsing silently turns a malformed `pulldown_cmark` into an empty list. Merged into the inline lenient-parsing comment.
- `cli/work-adapters/tests/golden_digests.rs:276-282` (RIGHT) — minor, medium — "Earlier recipe rewritten" message now also covers mistakes in the current recipe's entry. Merged into the inline rewritten-entry comment.
- `tests/unit/tasks/test_research_structure.py:1506-1510` (RIGHT) — suggestion, medium — Carrier readers, ids and origins live in parallel tuples paired by position. `CARRIER_READERS`, `CARRIER_NAMES` and `("a note", "a finding")` are kept in step by hand. Only one `strict=True` zip catches a length mismatch, and nothing catches a wrong order. Keep each carrier's attributes together with `pytest.param(..., id=...)` or a small `NamedTuple`.
- `tests/unit/tasks/test_research_structure.py:1539-1547` (RIGHT) — minor, high — Rule-part names referenced across three shipped files are not tied to `RULE_PARTS`. Inline above.

**General Findings**: none.

### Test Coverage

**Summary**: The new structure tests are well built and follow the repo's conventions. They pin the citing-once rule's headings, key table and wiring, and stop the composer and `synthesise` from restating the rule. The recipe guard gained relevant cases. The main gap is that the shipped behaviour, an LLM merging citations deterministically, rests only on one-off scratchpad sessions that cannot be re-run. The final `synthesise` Read-tool fix has not been run behaviourally at all. Several rule decisions, and two branches of the rewritten recipe guard, would survive plausible mutations with CI green.

**Strengths**:
- The key table is checked against `RECOMMENDED_KEYS` rather than a copied list.
- `test_the_profiles_prescribe_the_retraction_marks` stops the mark-merging test passing vacuously over an empty set.
- `RULE_SIGNATURES` does double duty: each phrase must appear in the rule and must be absent from the carriers.
- `MERGED_TIER_EXAMPLES` is shared across the rule, finding-outputter and docs tests.
- The carrier tests are parametrised with named ids, so a failure names the carrier that drifted.
- The recipe guard gained "version added" and "recorded version dropped" cases. Combined with `every_golden_side_digests_to_its_recorded_hash`, a parser version is accepted only while every digest still holds.
- Phase 3 acceptance was rigorous for a one-off. It checked determinism across note and finding orders, used an oracle that predicted outputs first, and planted mutations to show the checker could fail.

**Comments**:
- `skills/research/research-topic/SKILL.md:647` (RIGHT) — major, medium — Read-tool fix is pinned only by its wording, not re-run. Inline above.
- `tests/unit/tasks/test_research_structure.py:1413-1431` (RIGHT) — minor, high — Several rule decisions survive mutation of `RULE_SIGNATURES`. Inline above.
- `skills/research/outputters/citing-once/SKILL.md:170-176` (RIGHT) — minor, medium — Worked example and URL table are unchecked against the rule. The plan's Python oracle reproduced both, but it stayed in the scratchpad. Commit it as a test that derives each URL row's verdict and recomputes the worked example's merged entry.
- `tests/unit/tasks/test_research_structure.py:1471` (RIGHT) — minor, high — The prescribed-mark regex misses multi-word or hyphenated marks. Merged into the inline "Closed mark list" comment.
- `cli/work-adapters/tests/golden_digests.rs:263-268` (RIGHT) — minor, high — Extension of a non-latest recipe entry has no pinning case. Merged into the inline rewritten-entry comment.
- `cli/work-adapters/tests/golden_digests.rs:327-328` (RIGHT) — suggestion, high — The length check and the any-version match have no distinguishing case. Deleting `history.len() < recorded.len()` passes every test, and so does narrowing `any` to `last`. Make `recorded` a per-case column, add a two-recorded/one-history case, and add a case `"locked at an earlier recorded version"` that expects `true`.
- `cli/work-adapters/tests/golden_digests.rs:218` (RIGHT) — suggestion, medium — A scalar `pulldown_cmark` entry parses silently to an empty list. Merged into the inline lenient-parsing comment.
- `agents/composer.md:29-31` (RIGHT) — minor, medium — The composer's "Never add a source" invariant is unpinned. Assert it in `_composer()`, and consider whether `synthesise` should state and pin the same rule.
- `tests/unit/tasks/test_research_structure.py:1576-1583` (RIGHT) — suggestion, medium — The no-restatement check omits the finding outputter, which every composer reads and which now paraphrases the suffix rule. Add it to the parametrisation.

**General Findings**:
- major — Behavioural acceptance evidence is not kept or re-runnable. The 36 sessions' fixtures, predicted entries, oracle and checker live in a scratchpad, and the plan says "No retained eval". The repo already has a per-skill eval convention (`skills/work/*/evals/`). Commit the C1–C5 and S1–S2 fixtures and the checker as an on-demand eval beside the rule.

### Documentation

**Summary**: The documentation this PR ships is accurate and consistent. The citing-once contract is the one place the rule is written down, and the carriers name its parts without restating them. Every worked, ordering and normalisation example was re-traced against the rule's text without finding an error. The main gap is outside the diff. The CHANGELOG's `[Unreleased]` entry still says the composer and `synthesise` carry each metadata block byte-for-byte, which is exactly the behaviour this PR removes. It also has no entry for the new `also` tier-text form or the new requirement on composer overrides. The other findings are small: the user docs leave a few questions open, and one recipe-guard diagnostic is now misleading.

**Strengths**:
- The rule lives in one contract, and tests pin both its headings and that the carriers don't restate it.
- The rule is easy to check: a URL table, explicit sort examples and a worked example, all verified by hand.
- The stale byte-for-byte wording is gone from every shipped surface (grep-checked), and a test keeps it out.
- The docs link to the generated reference page, which the generated index already lists.
- The `configure` override warning closes the gap a custom-agent author would otherwise hit.
- The recipe guard's doc comment was replaced by a named `extends` helper.

**Comments**:
- `docs-site/src/content/docs/research.md:789-790` (RIGHT) — minor, medium — The docs don't say that depth-1 findings stay unmerged. A user who sees a source listed twice in a depth-1 finding will read it as a bug. Add one sentence saying the researcher writes depth-1 findings, so only `synthesise` merges their duplicates.
- `docs-site/src/content/docs/research.md:791-793` (RIGHT) — suggestion, medium — "Lowest tier" is undefined on the docs page, and no reason is given. Readers may take "lowest" to mean `tier-1`. Gloss it as the weakest tier and give the reason, for example that a disagreement between tiers stays visible.
- `cli/work-adapters/tests/golden_digests.rs:282` (RIGHT) — minor, medium — The guard message says "an earlier recipe" when the current recipe's entry was rewritten. Merged into the inline rewritten-entry comment.
- `cli/work-adapters/tests/golden_digests.rs:303-307` (RIGHT) — suggestion, low — The remediation message doesn't say where to record the new parser version. Name `tests/fixtures/golden-digests/recipe.json` and its `pulldown_cmark` list in the message.

**General Findings**:
- major — CHANGELOG `[Unreleased]` still says the composer and `synthesise` carry blocks byte-for-byte. Amend that entry, and add one for citing once (`also` suffix, retraction marks kept, the composer-override requirement, existing findings stay valid).

### Compatibility

**Summary**: Seen through the compatibility lens, this PR is mostly additive and safe. The `also` suffix is a forward-compatible extension, no shipped code parses tier text, and keeping `DigestRecipe::CURRENT` at 1 preserves cross-version baseline readability. The dependency changes are benign. The three minor concerns are all about contracts that will cause trouble later rather than now:

- existing composer overrides get a new required input, and nothing tells their authors;
- the parser-upgrade policy now treats a green golden corpus as sufficient evidence;
- the closed suffix list silently drops any mark a profile adds later.

**Strengths**:
- Leaving `DigestRecipe::CURRENT` at 1 keeps sync baselines readable across mixed binary versions. The extended guard still rejects a history that drops a version.
- The tier-text change is forward-compatible, and a grep of `cli/` confirms nothing parses Sources tier text.
- The `tempfile` re-resolution to `getrandom 0.4.3` adds no new platform or target surface. pulldown-cmark 0.13.5 declares `rust-version = 1.71.1`, below the workspace's 1.90.0.
- The rule's key table is pinned to `RECOMMENDED_KEYS`.
- The composer still grants only `Read` and `Write`. The `PreToolUse` guard does not block the new plugin-root read, and the change needs nothing beyond the v2.1.144 floor.

**Comments**:
- `skills/config/configure/SKILL.md:281-283` (RIGHT) — minor, medium — Existing composer overrides get a new required input with no upgrade signal. An override cloned from the old `agents/composer.md` will silently ignore the injected path. Put the requirement in a release note, or have `accelerator config agents` print a reminder when `agents.composer` is overridden.
- `cli/Cargo.toml:117-118` (RIGHT) — minor, medium — The parser-upgrade policy now relies on the golden corpus alone. Inline above, merged with architecture.
- `skills/research/outputters/citing-once/SKILL.md:70-79` (RIGHT) — minor, high — The closed suffix grammar silently drops any tier mark a profile adds later. Merged into the inline "Closed mark list" comment.

**General Findings**:
- suggestion — Sources written before 0306 stay on disk, so the planned tier-text parsers must accept both forms. Record this in 0281 and 0284: treat the suffix as `; `-separated parts with an optional final `also` list separated by `, `, and tolerate duplicate entries.

### Standards

**Summary**: The shipped surface mostly follows the project's conventions. The new contract copies its siblings' frontmatter and description shape, and the docs links use the established internal-reference form. The tests reuse the module's helpers and naming, and prose stays within 80 columns. The main gap is change management. The `[Unreleased]` CHANGELOG still says the composer and `synthesise` carry each metadata block byte-for-byte, and nothing records the `also` suffix or the requirement on `agents.composer` overrides. The other findings are smaller naming inconsistencies.

**Strengths**:
- The `citing-once` frontmatter matches its sibling outputters, so the docs generator files the page under Internal reference without extra wiring.
- The rule is injected by `${CLAUDE_PLUGIN_ROOT}` path, alongside the finding outputter, and the composer refers to it by role.
- The docs links target the `internal/` path that `output_path()` generates.
- The new tests follow the module's naming and helper conventions.
- The comment policy is followed in `golden_digests.rs` and `Cargo.toml`.
- Prose stays within 80 columns; the only longer line is the literal worked-example entry.
- The third-party notices were regenerated with the lockfile bump.

**Comments**:
- `cli/work-adapters/tests/golden_digests.rs:218` (RIGHT) — minor, medium — The JSON key `pulldown_cmark` holds a list but no longer matches its Rust field name. Merged into the inline lenient-parsing comment.
- `docs-site/src/content/docs/research.md:795` (RIGHT) — suggestion, medium — The docs call it the `citing-once` contract, while every shipped surface calls it the citing-once rule. Use "rule" in both docs sentences (here and at line 758).
- `skills/research/outputters/citing-once/SKILL.md:32` (RIGHT) — suggestion, low — The section headings use sentence case, unlike every sibling injected contract, which uses Title Case H2s. Either Title-case the part names (with `RULE_PARTS` and the carriers), or accept the local inconsistency.

**General Findings**:
- major — CHANGELOG not updated, and its Unreleased entry now contradicts the new citing behaviour. Reword the 0348 bullet. Then add bullets for the `also` suffix and mark ordering, and for the `agents.composer` override requirement, filed under Changed (or Breaking, if overrides are treated as a public contract).

---
*Review generated by /accelerator:review-pr*
