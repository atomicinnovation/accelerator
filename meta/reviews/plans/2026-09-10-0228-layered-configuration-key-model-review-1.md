---
type: "plan-review"
id: "2026-09-10-0228-layered-configuration-key-model-review-1"
title: "Plan Review: Layered Configuration Key Model Implementation Plan"
date: "2026-09-10T09:43:33+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-10-0228-layered-configuration-key-model"
parent: "plan:2026-09-10-0228-layered-configuration-key-model"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "safety", "usability", "standards"]
review_number: 1
review_pass: 2
tags: ["configuration", "work-management", "migration", "tracker"]
last_updated: "2026-09-10T16:38:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Layered Configuration Key Model Implementation Plan

**Verdict:** REVISE

The plan is architecturally strong and unusually well-grounded in the codebase: a single shared alias helper as the deprecation spine, seven independently-mergeable phases, a correctly-justified resolving (not ignore-and-warn) alias, and a thorough domain-language rename. It should not proceed to implementation as written. One critical and thirteen major findings cluster on four hard edges — the `resolve_scheme` sketch itself leaks a tracker prefix and drops its own required warning, the deprecation-warning plumbing does not exist as claimed and will emit contradictory messages, the `m0009` migration is under-specified across three interacting keys and perpetually nags clean repos, and the rename is materially wider than "~8 sites" (serde/visualiser DTOs, a parallel `canonicalise_id` path). Two concrete arithmetic/tooling errors (the count test, the public-api snapshots) mean Phase 1–3 as written fail their own green-build gate.

### Cross-Cutting Themes

- **The `resolve_scheme` sketch is wrong in two ways at once** (flagged by: correctness, code-quality) — it sets `key: prefix.value` unconditionally (leaking a prefix into no-`{key}` recognition paths, violating AC #7) and constructs the scheme dropping `prefix.deprecation` (silently discarding the AC #8/#9 warning). The plan's own code sample demonstrates both bugs.
- **The deprecation-warning plumbing does not exist as described** (flagged by: usability, correctness, compatibility) — `render::emit` has no dedup, so "identical warnings are deduped at emit" is false; and a tracker-backed legacy repo emits two *differently-worded* warnings for one legacy field ("set `work.key`" vs "set `jira.project_key`"), which dedup could never collapse and which read as contradictory.
- **`m0009` is under-specified where it is most dangerous** (flagged by: correctness, safety, compatibility) — the three interacting keys (legacy, `work.key`, scope key) have no read-before-write ordering, the mixed-state guard's pairs are unnamed, and `work.key` materialisation is contradictory (bullet 1 renames unconditionally, bullet 3 conditions on `{project}`).
- **`m0009` as the highest-id migration perpetually nags clean repos** (flagged by: safety, usability) — `NoOpPending` never advances the ledger, so every fresh or already-migrated repo sits behind the registry forever and the SessionStart nag never clears.
- **The removal window is advisory, not structural** (flagged by: compatibility, architecture, usability, safety) — nothing forces `m0009` before the 1.25.0 code-delete, and `REMOVAL_RELEASE` is a hardcoded literal that drifts if the schedule slips.
- **The rename is wider than the plan scopes** (flagged by: code-quality, standards) — serde DTOs in the visualiser (`RawWorkItemConfig`, `WorkItemConfigBody`/`defaultProjectCode`) and a parallel `canonicalise_id` resolution path are outside the compiler's reach and unlisted; public-api snapshots and the count test break the per-phase green gate.

### Tradeoff Analysis

- **Loud failure vs a silent, un-actionable nag**: making `m0009` the highest migration return `NoOpPending` on clean repos keeps the "nothing to do" semantics honest but produces a permanent nag that trains users to ignore genuine prompts. Recommendation: return `Applied` (record the ledger) when there is genuinely nothing to migrate — "already up to date" is terminal here, not pending.
- **Team-file rewrite vs personal-file rewrite for `m0009`**: rewriting team `config.md` (like `m0004`) fixes the repo once for everyone but lands a committed diff that a colleague or CI still on the pre-1.24.0 plugin cannot read (`{key}` is a hard `UnknownToken`). The plan should state this forward-only, mixed-version hazard as a conscious choice rather than leave it implicit.

### Findings

#### Critical

- 🔴 **Compatibility**: Removal in 1.25.0 is unenforced — unmigrated configs break on upgrade
  **Location**: Migration Notes; Phase 7 (m0009)
  The one-release window is structural, not version-gated: 1.25.0 deletes the alias and the `{project}` token, and the only safety net is an advisory SessionStart nag. A user who upgrades 1.24.0 → 1.25.0 while still carrying `work.default_project_code`/`{project}` hits the exact `E_NO_PROJECT`/`BadKeyValue`/`UnknownToken` failures the resolving alias was built to prevent, with no runtime guard. Suggestion: the 1.25.0 change must refuse to start with a clear `/accelerator:migrate` error on detecting legacy keys, rather than removing recognition outright — record that as the enforced contract now.

#### Major

- 🟡 **Correctness**: `resolve_scheme` sets the prefix unconditionally, leaking a tracker prefix into no-`{key}` recognition paths (AC #7)
  **Location**: Phase 3, Section 3
  The sketch gates only the *error* on `references_key`; `key: prefix.value` is set whenever a prefix resolves. For a legacy bare-numeric tracker repo (`{number:04d}` + `work.default_project_code: PP`, `expected_integration: None` so it always resolves), `scheme.key` becomes `Some("PP")`, and `normalise_id`/`extract_id` (which branch on the field, not the pattern) key `0001-foo.md` as `PP-0001`, corrupting `filter`/`resolve`/`cluster`/the indexer. Fix: gate the field — `key: references_key(&id_pattern).then(...).flatten()`.

- 🟡 **Code Quality**: The field rename crosses serde boundaries the compiler will not catch
  **Location**: Phase 3, Section 2
  `cli/visualiser/server/src/config.rs` (`RawWorkItemConfig.default_project_code`, `Deserialize`) and `.../api/work_item_config.rs` (`WorkItemConfigBody`, serialised `defaultProjectCode` to the React frontend) are separate structs the struct-field rename leaves untouched. The "~8 sites" enumeration omits them plus `cluster.rs`, `slug.rs`, `resolve.rs:380/387`. Renaming the launcher's emitted JSON key without coordinating `RawWorkItemConfig` breaks deserialisation silently. Fix: enumerate the serde DTOs and the launcher→visualiser wire key as an explicit coordinated decision.

- 🟡 **Code Quality**: A parallel prefix-resolution path in `canonicalise_id` is not updated
  **Location**: Phase 3, Section 3
  `cli/work-cli/src/canonicalise_id.rs:33` resolves the same prefix independently via `effective_nonempty(config, "work.default_project_code")`, and no phase touches it. `accelerator work canonicalise-id` would keep reading the legacy key, bypassing `work.key` and the alias. Fix: fold this read into the shared resolver or list it explicitly as a rename/alias site.

- 🟡 **Correctness / Code Quality**: The deprecation warning is computed then discarded; `resolve_scheme` has no warnings channel
  **Location**: Implementation Approach; Phase 3, Section 3
  `AliasedScalar` carries `deprecation`, but the sketched `resolve_scheme` returns `Ok(WorkItemIdScheme { id_pattern, key: prefix.value })`, dropping it — contradicting the prose claim that warnings route through `ScalarView`/`render::emit`. The AC #8/#9 warning becomes a per-caller obligation each of the ~4 consumers can silently forget. Fix: make the warning channel structural — return it alongside the scheme or emit inside the helper.

- 🟡 **Usability / Correctness / Compatibility**: Deprecation warnings duplicate and contradict; "deduped at emit" is false
  **Location**: Phase 3 vs Phases 4–5
  `render::emit` (`render/mod.rs:43`) iterates and prints — there is no dedup. On a tracker-backed legacy repo the same `work.default_project_code` is read by `resolve_scheme` (warns "set `work.key`") and by the scope path (warns "set `jira.project_key`"), in different crates returning different types, so they never reach a common dedup point and name different keys. The manual-verification "prints one deprecation warning" is unachievable as designed. Fix: suppress the prefix-path warning when the pattern omits `{key}`; coalesce to a single warning naming the correct target.

- 🟡 **Correctness / Safety / Compatibility**: `m0009` is under-specified across three interacting keys
  **Location**: Phase 7, Section 1
  Bullet 1 renames `work.default_project_code`→`work.key` (removing the source), bullet 2 needs to *read* that source for the scope key, and bullet 3 materialises `work.key` only where `{project}` was referenced — contradicting bullet 1's unconditional rename and risking a TOCTOU within the migration. The mixed-state guard's key pairs are unnamed, and scope-key materialisation ("insert where absent") is not stated to protect an init-written scope key. A bare-numeric tracker repo gets a spurious `work.key`. Fix: read legacy value + `work.integration` once up front, write from the captured value, condition `work.key` materialisation on the pattern, and enumerate the guarded pairs.

- 🟡 **Safety**: `init-jira`/`init-linear` overwrite a user-set scope key with no backup or confirmation
  **Location**: Phase 6
  Phase 6 has `init-linear` call `set(&key, value, Level::Team)` "overwriting any existing value" with no `.bak` and (unlike `init-jira`'s interactive Step 5) no offer. A user who hand-set `linear.team_key` then re-runs `/init-linear` to refresh the catalogue silently loses it to whatever discovery resolves. Fix: back up or confirm before overwriting a non-empty scope key; make Linear match Jira's interactive offer. (AC #12/#13 require overwrite, but not destruction of a deliberately-set value without recourse.)

- 🟡 **Safety / Usability**: `m0009` `NoOpPending` perpetually nags clean repos
  **Location**: Phase 7, Section 1
  `NoOpPending` does not append to the ledger (`lifecycle.rs:117`), and `m0009` becomes the highest-id migration, so any repo with nothing to migrate sits at `highest_applied < highest_available` forever and the SessionStart nag fires every session, never cleared by running migrate. The plan's "the nag needs no change" is wrong for a highest-id no-op. Fix: return `Applied` when genuinely up to date.

- 🟡 **Compatibility**: A config carrying both keys is silently un-warned yet cannot migrate
  **Location**: Implementation Approach; Phase 7 mixed-state guard
  When both `work.key` and `work.default_project_code` are set, the helper returns the canonical value with no warning (unlike the `paths.rs` precedent, which warns even when ignoring), *and* `m0004`'s mixed-state guard aborts the migration — so the user gets no deprecation signal and cannot clean up, reaching 1.25.0 still carrying the legacy key. Fix: emit a "legacy key present and ignored" warning, and treat a redundant legacy key beside a correct canonical value as a removable no-op, not a hard abort.

- 🟡 **Test Coverage**: Discovery-scope ACs (#4, #7) assert behaviour, not an observable emitted filter
  **Location**: Phase 5 (AC #4); Phase 3 (AC #7)
  The criteria describe "scopes from the scope key" in prose but never assert the concrete `SearchScope.project` value or the Linear `{team:{id:{eq:UUID}}}` filter that 0220 pinned — a rigour regression the research flagged. A mis-scoping defect (0220's bug class) passes every named test. The `RecordingTracker::search` scaffolding is `unimplemented!`, so this assertion must be built. Fix: record and assert the scope passed to `search` for the divergent and tracker-backed arms.

- 🟡 **Test Coverage**: Config-file layering AC #8 (personal-over-team split keys) has no named test
  **Location**: Testing Strategy / all phases
  Work-item AC #8 — scope key in team `config.md`, `work.key` in personal `config.local.md`, `{key}` using the personal value while discovery scopes from the team value — is one of the two headline "layered" requirements yet appears in no phase or the Testing Strategy. Fix: add an integration test mirroring `personal_overrides_team()`.

- 🟡 **Usability**: The canonical config reference still teaches the deprecated keys and denies the new ones
  **Location**: Phases 4–6 (skill prose) — omission of `skills/config/configure/SKILL.md`
  The plan updates only integration-skill prose. `skills/config/configure/SKILL.md` — the one place a user learns config keys — still lists only `default_project_code`, documents `{project}` not `{key}`, and at lines 748–750 actively asserts the new keys do not exist. A developer configuring manually is taught the deprecated vocabulary. Fix: add explicit edits to `configure/SKILL.md` (key tables, the `{key}` DSL token, the recognised-key lists) and sweep `work.*` prose under `skills/work/*`.

- 🟡 **Standards**: The catalogue count-test arithmetic is wrong (58 should be 56)
  **Location**: Phase 1, Section 1
  The count test sums PATH/TEMPLATE/WORK/REVIEW/AGENT/VISUALISER keys — `EXTRA_KEYS` is deliberately *not* counted. Adding `work.key` bumps it by one (55→56); the two `EXTRA_KEYS` additions do not change the sum. As written, Phase 1 asserts 58 against an actual 56 and fails its own `cargo test -p config` gate. Fix: target 56, rename `fifty_six`.

- 🟡 **Standards**: Public-api snapshots for `config` and `corpus` are not regenerated
  **Location**: Phases 2 & 3
  Both are pinned crates (`_PINNED_CRATES`); Phase 3 adds public items to `config` and Phases 2–3 rename a public field/variants on `corpus`. `public-api:check` is not in `cli:check` (the per-phase gate), so drift passes the inner loop but reddens the full `mise run` — breaking the phase-green guarantee. Fix: add a snapshot-regeneration step and list `public-api:check` in those phases' criteria.

#### Minor

- 🔵 **Architecture / Code Quality**: The `{key}`/`{project}` recognition is duplicated across two walled-off crates with no shared home
  **Location**: Phase 2
  `references_key` and the spelling pair land independently in `work_item_pattern.rs` and `work_item_id.rs`, kept correct only by the parity test. Define the predicate once per crate and route every branch through it.

- 🔵 **Test Coverage**: The plan's AC numbering does not map to the work item's criteria
  **Location**: Success Criteria, Phases 3–7
  The plan cites AC #8 for the tracker-backed legacy read (work-item AC #9), AC #13 for init overwrite (AC #12), and never references work-item AC #8. This breaks traceability and masks the AC #8 gap above. Fix: renumber to match, add a traceability table.

- 🔵 **Test Coverage**: The `work.integration`-vs-section-presence disagreement is tested only manually
  **Location**: Phases 4–5
  The research asks for an automated both-sections-present test; the plan leaves it under Manual Verification. Promote it to an automated integration test.

- 🔵 **Test Coverage**: `m0009` E2E omits arms present in the `migration_0002` model
  **Location**: Phase 7
  No named `NoOpPending`, no-mutation-on-abort, `.0009.bak` creation, or end-to-end `{project}`→`{key}` rewrite arm — the highest-risk irreversible paths. Mirror the model's full arm set.

- 🔵 **Safety**: The `.0009.bak` backup of `config.local.md` is world-readable, bypassing the 0600 guard
  **Location**: Phase 7
  `backup_config_once` writes via `ctx.write` (`fresh_mode = 0o666 & !umask`, typically 0644), so a plaintext copy of the personal config lands group/world-readable next to the 0600-protected original. Force 0600 on a personal-file sidecar.

- 🔵 **Compatibility**: The migrated config is unreadable by any pre-1.24.0 plugin
  **Location**: Phases 2 & 7
  Once `m0009` rewrites team `config.md`, `{key}` is a hard `UnknownToken` and the new keys are unknown to older plugins — so a colleague, CI runner, or plugin rollback on the older version fails to mint IDs. Document the forward-only, mixed-version hazard as a conscious choice.

- 🔵 **Compatibility / Usability**: `REMOVAL_RELEASE = "1.25.0"` is a hardcoded literal that drifts on a schedule slip
  **Location**: Phase 3, Section 1
  The plan's own note says removal shifts if 1.24.0 slips, but nothing ties the string to the shipped version. Tie it to `CARGO_PKG_VERSION`'s next minor (or a test), or soften the wording and keep `/accelerator:migrate` primary.

- 🔵 **Compatibility**: `trello`/`github-issues` integrations have no scope-key target
  **Location**: Phases 4–5, 7
  `work.integration` accepts four values but the alias and `m0009` dispatch handle only `jira`/`linear`. Confirm the other two carry no scope-key dependency and state the deliberate coverage, or extend the dispatch.

- 🔵 **Correctness**: `references_key` false-positives on escaped `{{key}}` literals
  **Location**: Phase 2, Section 1
  `pattern.contains("{key}")` matches the literal text inside an escaped `{{key}}`, spuriously triggering the `work.key`-required error. Detect the token via the brace-aware walk, not a substring `contains`.

- 🔵 **Correctness**: Legacy `work.default_project_code` now outranks the catalogue `/team/key`
  **Location**: Phase 5, Section 1
  Placing the deprecated field above the currently-authoritative catalogue means a stale legacy value wins over a freshly re-init'd team key. Confirm the intended precedence.

#### Suggestions

- 🔵 **Architecture**: No named "active tracker scope key" resolver — 0229 must reuse the inline `sync.rs` dispatch
  **Location**: Phase 5, Section 2
  Extract `resolve_active_scope_key(config)` so `sync.rs`, the integration reads, and 0229's pull-scope layer consume one abstraction rather than duplicating the `work.integration` branch.

- 🔵 **Correctness**: Linear discovery scope in `sync.rs` omits the catalogue fallback that `auth.rs` adds
  **Location**: Phase 5, Section 2
  A catalogue-only Linear repo resolves the scope to `None` in `sync.rs` while `auth.rs` resolves it fine — two divergent answers, risking the 0220 regression. Have `sync.rs` call the same catalogue-aware `team_key` resolver.

- 🔵 **Usability**: `work_key_required` error text is under-specified and must disambiguate `{key}` from the scope key
  **Location**: Phase 3, Section 3
  Give it `bad_integration`-level richness: name the `{key}` token, state it resolves from `work.key` (not the scope key) and why, and say where to set it. A generic "work.key is required" invites the user to reach for `jira.project_key` — the forbidden fallback.

- 🔵 **Architecture / Usability**: `work.key` reads ambiguously beside the self-describing scope keys; "key" is overloaded across four new names
  **Location**: Desired End State / Phase 1
  `work.key`, `{key}`, `jira.project_key`, `linear.team_key` all read as "the key" while their purpose is independence. If not locked by the story, consider `work.id_prefix`; at minimum teach the contrast with a worked example.

- 🔵 **Standards**: New module `legacy_alias.rs` needs its `pub mod` declaration and `pub use` re-exports called out
  **Location**: Phase 3, Section 1
  State `pub mod legacy_alias;` in `lib.rs` and the intended re-exports; confirm no crate-registration checklist is owed (correct — it is a module, not a new crate).

- 🔵 **Standards**: Elision markers (`// ... unchanged ...`) in snippets risk becoming production comments
  **Location**: Phases 1 & 2
  Replace inline elision comments with prose outside the fence, per the no-comments convention.

### Strengths

- ✅ The shared alias helper lives in `config` (which all three consumers already depend on, itself depending only on `kernel`), so the deprecation machinery adds no dependency edges and the 1.25.0 removal is a delete-one-helper operation.
- ✅ The resolving alias is correctly chosen over ignore-and-warn: the scope key has no benign default, so ignoring would produce a hard-migrate outage — the justification is sound and documented.
- ✅ Seven independently-mergeable phases, each leaving `mise run` green, ordered so each alias lands with its first consumer and no phase breaks an existing config.
- ✅ The ownership inversion (integration crates read only their own section) establishes the clean one-way `work → integration` dependency that serves eventual independent packaging.
- ✅ Thorough domain-language rename (`TokenKind::Project`→`Key`, `MissingProject`→`MissingKey`, `ParsedId.project`→`key`) matching the story's conceptual split.
- ✅ The `{key}`/`{project}` parity strategy (byte-identical scan/format/parse across both implementations, plus an intact unknown-token set) is mutation-resistant.
- ✅ The alias gate closes the previously-untested "both sections present" case by dispatching on `work.integration`, not section presence.
- ✅ `m0009` inherits `m0004`'s proven safeguards: once-only `.bak` sidecar before first mutation, atomic per-file writes, fail-closed mixed-state abort.

### Recommended Changes

1. **Fix the `resolve_scheme` sketch** (addresses: AC #7 prefix leak, warning discarded) — gate the field with `references_key(&id_pattern).then(...).flatten()`, and give `resolve_scheme` a warnings channel that surfaces `prefix.deprecation` through `ScalarView`/`render::emit`.
2. **Make the deprecation warning single and correct** (addresses: duplicate/contradictory warnings, false dedup claim) — suppress the prefix-path warning when the pattern omits `{key}`, add a real dedup point (or emit once from a single alias-resolution site), and drop the unsubstantiated "deduped at emit" claim.
3. **Specify `m0009` fully** (addresses: three-key ordering, mixed-state guard, perpetual nag) — read legacy value + `work.integration` once up front and write from the captured value; enumerate the guarded key pairs (guard only a true `work.default_project_code` vs `work.key` collision; treat an init-written or already-correct canonical key as authoritative insert-where-absent); condition `work.key` materialisation on the pattern; return `Applied` when nothing is to be done; force 0600 on a personal-file `.bak`.
4. **Enforce the removal window** (addresses: critical unmigrated-break, `REMOVAL_RELEASE` drift, mixed-version hazard) — record that the 1.25.0 change must refuse to start with a `/accelerator:migrate` error on detecting legacy keys; tie `REMOVAL_RELEASE` to `CARGO_PKG_VERSION`; document the forward-only team-file rewrite.
5. **Widen the rename scope explicitly** (addresses: serde/visualiser DTOs, parallel `canonicalise_id`, public-api, count test) — enumerate the visualiser serde DTOs and the launcher→visualiser wire key; fold `canonicalise_id.rs`'s prefix read into the shared resolver; add public-api snapshot regeneration to Phases 2–3; correct the count test to 56.
6. **Protect init-written config** (addresses: init overwrite destroys a user value) — back up or confirm before overwriting a non-empty scope key; make Linear's writeback match Jira's interactive offer.
7. **Close the test gaps** (addresses: unpinned scope filter, missing AC #8 test, AC misnumbering, manual-only cases) — assert the concrete emitted `SearchScope`/Linear filter; add the personal-over-team split-key test; renumber ACs to the work item and add a traceability table; promote the both-sections-present and legacy-renders-`PP-0001` cases to automated tests.
8. **Update the canonical config docs and add the missing helpers** (addresses: `configure/SKILL.md` omission, `references_key` escaping, catalogue precedence, module declaration) — edit `configure/SKILL.md`; make `references_key` brace-aware; confirm the catalogue-vs-legacy precedence; declare `pub mod legacy_alias;`.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound and well-grounded — the shared alias helper is the spine, placed in a crate all consumers already depend on, with no circular dependencies, and phases keep `mise run` green while landing each alias with its first consumer. The core ownership inversion is correct and directionally strong for eventual independent packaging. Main risks are cohesion: the "resolve the active tracker's scope key" concept is left inline in `sync.rs` rather than named and extractable (0229 will need it), the prefix-token vocabulary is duplicated across two walled-off crates, and the temporary integration→work re-coupling during the window is understated.

**Strengths**:
- The shared alias helper sits in `config` (depends only on `kernel`); no new dependency edges, no cycles; centralised `REMOVAL_RELEASE` makes removal a delete-one-helper operation.
- Ownership inversion establishes a clean one-way `work → integration` dependency serving independent packaging.
- Seven independently-mergeable phases, each green, ordered so no phase breaks an existing config.
- Strong domain modelling: internal token vocabulary renamed to key-centric terms.

**Findings**:
- 🟡 major (medium): No named "active tracker scope key" resolver — Phase 5's `work.integration` dispatch is left inline in `sync.rs`, so 0229 (and any third tracker in `WORK_INTEGRATION_VALUES`) must re-implement it. Extract `resolve_active_scope_key(config)`.
- 🔵 minor (high): Prefix-token vocabulary duplicated across `work_item_pattern.rs` and `work_item_id.rs` with no shared home; `references_key` defined twice. Define the predicate once in `corpus` and reuse.
- 🔵 minor (medium): Deprecation-window re-coupling understated — Phase 4/5 call sites literally pass `"work.default_project_code"`, so integration crates *do* name a `work.*` key until 1.25.0. State this as a window-scoped exception.
- 🔵 minor (medium): 1.25.0 alias removal has no enforced guarantee the migration ran — only the SessionStart nag; a 1.23→1.25 jump silently loses the scope key. Record the accepted tradeoff and the intended 1.25.0 safeguard.
- 🔵 suggestion (low): `work.key` reads ambiguously beside `jira.project_key`/`linear.team_key`; consider `work.id_prefix` or make the "local ID prefix" role unmistakable.

### Correctness

**Summary**: Logically well-structured with green-preserving ordering, but the central resolution logic has a gap: `resolve_scheme` sets the prefix unconditionally without gating on `references_key`, leaking a tracker prefix into ID-recognition paths and contradicting AC #7. Further concerns: the Linear discovery-scope read in `sync.rs` omits the catalogue fallback `auth.rs` adds (0220 regression risk), and `m0009`'s read-before-write ordering and mixed-state handling across three keys are ambiguous. The alias gate itself is sound and the "both sections present" case is correctly closed.

**Strengths**:
- The alias gate correctly closes the "both sections present" disagreement case.
- Correctly identifies `references_key` must land in both implementations and that the `{project}`-substring guards must widen.
- The `{key}`-required rule is an explicit precondition, not a silent fallback (AC #2).
- Migration idempotency anchored on a `NoOpPending`-when-absent gate.

**Findings**:
- 🟡 major (high): Scheme prefix set unconditionally — leaks `PP` into `normalise_id`/`extract_id` for no-`{key}` patterns, corrupting the index (AC #7). Gate the field, not just the error.
- 🟡 major (medium): Linear discovery scope resolved without the catalogue fallback used by `auth.rs` — catalogue-only repos lose their scope in `sync.rs`, risking 0220 regression. Share one resolver.
- 🟡 major (medium): `m0009` read-before-write ordering and ambiguous `work.key` materialisation — bullets 1 and 3 both target `work.key` contradictorily; bullet 1 can delete the value bullet 2 needs. Read once, write from the captured value.
- 🟡 major (medium): Mixed-state guard underspecified across three interacting keys; scope-key materialisation may clobber an init-written key. Enumerate the guarded pairs.
- 🔵 minor (medium): Two differently-worded deprecation warnings for one legacy field; the dedup claim does not hold. Suppress/coalesce.
- 🔵 minor (medium): `resolve_scheme` discards the computed deprecation warning. Thread it out.
- 🔵 minor (low): `references_key` substring match false-positives on escaped `{{key}}`. Use the brace-aware walk.
- 🔵 minor (low): Legacy field now outranks the catalogue `/team/key`; a stale value beats a re-init'd catalogue. Confirm precedence.

### Code Quality

**Summary**: Well-structured, strongly TDD-framed, and uses DI (`&dyn ConfigAccess`) throughout; the core rename genuinely improves domain language. Main risks: the field/token rename is materially under-scoped (crosses serde DTO boundaries the compiler won't police, misses a parallel resolution path), the load-bearing deprecation warning is returned as easily-dropped data with no channel on `resolve_scheme`, and the helper signature is a transposable data clump. Addressable without redesign but should be pinned before implementation.

**Strengths**:
- Strict red-green-refactor stated per phase.
- The rename splits a genuinely overloaded field into two well-named domain concepts.
- Good testability: helper and resolvers take `&dyn ConfigAccess`.
- Clean phasing; each phase independently mergeable and green.
- The plan itself is comment-clean.

**Findings**:
- 🔴 major (high): Field rename under-scoped, crosses serde boundaries — visualiser `RawWorkItemConfig` / `WorkItemConfigBody` (`defaultProjectCode`) are separate structs the rename leaves unchanged; wire-key rename would break deserialisation silently. Enumerate the DTOs and the wire key.
- 🟡 major (high): Parallel prefix-resolution path in `canonicalise_id.rs:33` not updated — `work canonicalise-id` keeps reading the legacy key, bypassing the alias. Fold into the shared resolver.
- 🟡 major (medium): Load-bearing deprecation warning returned as droppable data; `resolve_scheme`'s sketched signature has no warnings channel and its body discards `prefix.deprecation`. Make the channel structural.
- 🔵 minor (medium): Helper signature is a transposable data clump — adjacent `canonical`/`deprecated` `&str`, a constant `removal_release` argument (YAGNI), and an overloaded `Option` `None`. Drop the const arg; split into two intent-revealing entry points or a typed request.
- 🔵 minor (medium): `{key}`/`{project}` recognition triplicated across walled-off crates. Route each branch through one per-crate predicate; keep the parity test as the enforced contract.

### Test Coverage

**Summary**: Disciplined red-green-refactor with a well-shaped pyramid — unit for helper/token, integration for scope resolution and init writeback, real-binary `m0009` E2E on the `migration_0002` model. The `{key}`/`{project}` parity strategy and the alias gate matrix are strong. The two most important gaps: discovery-scope ACs (#4, #7) are described behaviourally rather than pinned to an observable emitted filter (a rigour regression against 0220), and the config-file layering criterion has no named test.

**Strengths**:
- Rigorous Phase 2 parity coverage (byte-identical output across both implementations, unknown-token set intact).
- The full alias-helper gate matrix is named (canonical-wins, legacy-fallback-with-warning, gate match/mismatch/absent, empty-both).
- The `m0009` E2E is anchored on `migration_0002.rs` (real binary, TempDir, ledger pre-seeded through 0008), with idempotency and mixed-state arms.
- Appropriate pyramid balance.

**Findings**:
- 🟡 major (high): Discovery-scope ACs assert behaviour, not the emitted filter — a mis-scoping defect (0220's class) passes every test; `RecordingTracker::search` is `unimplemented!`, so the assertion must be built. Record and assert the scope.
- 🟡 major (high): Config-file layering AC #8 (personal-over-team split keys) has no named test at any level. Add one mirroring `personal_overrides_team()`.
- 🟡 major (medium): AC-to-test numbering does not map to the work item's criteria (plan's #8/#13 vs work-item #9/#12; work-item #8 never referenced). Renumber and add a traceability table.
- 🟡 major (medium): The `work.integration`-vs-section-presence disagreement is tested only manually. Promote to an automated integration test.
- 🔵 minor (medium): Single-emission of the deprecation warning asserted nowhere. Assert exactly once per invocation.
- 🔵 minor (medium): `m0009` E2E omits `NoOpPending`, no-mutation-on-abort, `.0009.bak`, and end-to-end id_pattern-rewrite arms present in the model.
- 🔵 minor (medium): Legacy "still renders `PP-0001`" is only manually verified. Add an automated mint test for both arms.
- 🔵 minor (low): Jira init writeback/reporting lacks an automated criterion (asymmetric with Linear). Add `cargo test -p jira-cli`.

### Compatibility

**Summary**: Unusually careful about the read-time deprecation contract — the `{project}`→`{key}` synonym is byte-identical across both implementations, the resolving alias correctly avoids a hard-migrate window, and phase ordering keeps every config working. The central risk is release coupling: removal in 1.25.0 is a pure code-delete with no semver gate and no enforcement that the migration ran first. Two narrower gaps: the both-keys-present case is silently un-warned and blocks the migration, and a migrated config is unreadable by any pre-1.24.0 plugin.

**Strengths**:
- The `{project}`→`{key}` synonym is byte-identical, mandated in both implementations, with parity tests.
- The resolving-alias choice is correctly justified against the ignore-and-warn precedent.
- Phase ordering keeps existing configs resolving at each merge point, preserving 0220.
- Gating on `work.integration` closes the both-sections-present case.

**Findings**:
- 🔴 critical (high): Removal in 1.25.0 unenforced — a 1.24→1.25 jump without migrating breaks tracker tooling and minting silently. Have 1.25.0 refuse to start with a `/accelerator:migrate` error.
- 🟡 major (high): A config with both keys is silently un-warned (canonical wins, no warning) yet the mixed-state guard aborts the migration — the user cannot clean up and reaches 1.25.0 still carrying the legacy key. Warn on ignored-legacy; treat redundant legacy as a removable no-op.
- 🟡 major (medium): Migrated config is unreadable by pre-1.24.0 plugins; the team-`config.md` rewrite lands a committed diff that breaks mixed-version teams/CI/rollback. Document the forward-only hazard.
- 🔵 minor (medium): `trello`/`github-issues` have no scope-key target in the alias or `m0009`. Confirm no dependency or extend the dispatch.
- 🔵 minor (medium): `REMOVAL_RELEASE` hardcoded, decoupled from `CARGO_PKG_VERSION`; a slip misnames the release. Derive or assert it.
- 🔵 minor (medium): Two differently-worded warnings for one legacy key on a single command; dedup will not collapse them. Coalesce or dedup on the key name.

### Safety

**Summary**: The migration inherits strong safety properties from `m0004`/`m0001` — a once-only `.0009.bak` before first mutation, a fail-closed mixed-state abort, atomic per-file writes, and a resolving alias that avoids a hard-migrate window. The largest residual risks are outside the migration: Phase 6's init writeback destroys a user-set scope key with no backup or confirmation, and the mixed-state guard is under-specified so a legitimate init-then-migrate config could abort or be mishandled. Two operational gaps: `NoOpPending` never advancing the ledger (perpetual nag) and the personal-config `.bak` written world-readable.

**Strengths**:
- The resolving alias is an explicit safety choice avoiding a hard-migrate outage.
- `m0009` models `m0004`'s once-only `.bak` and fail-closed abort.
- Atomic temp-file-plus-rename writes; per-file backup means no torn single-file write.
- Idempotency and mixed-state-aborts pinned as explicit success criteria.

**Findings**:
- 🔴 major (high): `init-jira`/`init-linear` overwrite a user-set scope key with no backup or confirmation (Linear is an unconditional `set()`, asymmetric with Jira's interactive Step 5). Back up or confirm; match Jira's offer.
- 🟡 major (medium): Mixed-state guard key pairs unspecified — a valid `jira.project_key` (init-written) alongside a lingering legacy value could abort a common config or overwrite the init key. Guard only the true rename collision; treat an init key as authoritative.
- 🟡 major (medium): `NoOpPending` on the new highest-id migration → perpetual SessionStart nag on clean repos. Return `Applied` when nothing to do.
- 🔵 minor (medium): `.0009.bak` of `config.local.md` written 0644, defeating the 0600 guard on personal config at rest. Force 0600 on the sidecar.
- 🔵 minor (low): Cross-file half-migration leaves a mixed effective config until re-run; recovery leans on per-transform idempotency, not just the whole-migration no-op. Add a partial-failure test.

### Usability

**Summary**: Core config ergonomics are right — canonical-wins-no-warning steady state, an explicit required-key error, and init writeback removing a manual step. Three real gaps: the "deduped at emit" claim is unsubstantiated (`render::emit` has no dedup) and a tracker-backed legacy repo emits two contradictory remediation strings for one field; `m0009` as the highest no-op migration perpetually nags every clean/fresh repo; and the plan leaves the canonical config reference (`configure/SKILL.md`) teaching the deprecated key names and asserting the new ones do not exist.

**Strengths**:
- The warning ends with the actionable "Run /accelerator:migrate".
- The required-key rule fails loudly rather than borrowing the scope key as a prefix.
- A fully migrated repo is completely quiet.
- Init writeback shortens time-to-first-success.

**Findings**:
- 🟡 major (high): `m0009` as the highest migration perpetually nags clean repos (`NoOpPending` never records the ledger). Return `Applied` when up to date.
- 🟡 major (high): Warnings duplicate and contradict — `render::emit` has no dedup, and the prefix path ("set `work.key`") and scope path ("set `jira.project_key`") emit different strings from different crates. Single dedup point; one combined warning.
- 🟡 major (high): `skills/config/configure/SKILL.md` still teaches `default_project_code`/`{project}` and (lines 748–750) denies the new keys exist. Add explicit edits and sweep `skills/work/*` prose.
- 🟡 major (medium): `work_key_required` error text under-specified; a user could reach for `jira.project_key`/`linear.team_key` — the forbidden fallback. Give it `bad_integration`-level guidance.
- 🔵 minor (medium): "key" overloaded across `work.key`, `{key}`, `jira.project_key`, `linear.team_key`, inviting a wrong mental model. Teach the contrast with a worked example.
- 🔵 minor (medium): Hardcoded "1.25.0" may misname the actual removal. Soften or tie to the actual removal.

### Standards

**Summary**: Strongly aligned with naming and structural conventions — the key-centric rename, the `mod.rs`/`registry.rs` migration registration with a conforming id, and catalogue/`effective_nonempty` reads all follow established patterns. But it mis-states the count-test arithmetic and omits two "keep-in-sync" obligations pinned Rust crates carry — the public-api snapshots for `config` and `corpus` — plus the module declaration for `legacy_alias.rs`. These sync gaps undermine the plan's own "each phase leaves `mise run` green" guarantee.

**Strengths**:
- Domain-language rename thorough and consistent across token kinds, predicates, error variants, and the scheme field.
- `m0009` registration matches convention exactly; the note that the nag and `hooks.json` need no change is correct.
- New config keys added in the right groups matching the stringly-typed catalogue pattern.
- Helper named in domain terms; `REMOVAL_RELEASE` centralised.

**Findings**:
- 🔴 major (high): Count-test arithmetic wrong — `EXTRA_KEYS` is not summed, so the target is 56 (not 58); as written Phase 1 fails its own gate. Target 56, rename `fifty_six`.
- 🟡 major (high): Public-api snapshots for the pinned crates `config` and `corpus` not regenerated; `public-api:check` is outside `cli:check`, so drift reddens the full `mise run`. Add a regeneration step and list `public-api:check`.
- 🔵 minor (medium): New `legacy_alias.rs` needs `pub mod legacy_alias;` in `lib.rs` and `pub use` re-exports called out; correctly does NOT trigger the library-crate checklist (it is a module, not a new crate).
- 🔵 suggestion (low): Elision markers (`// ... unchanged ...`) risk becoming production comments; move to prose outside the fence.

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** COMMENT

The revision closes the entire initial finding set: the one critical, all thirteen majors, and the minors are resolved (one — Jira init writeback — resolved as a documented manual-scope decision rather than new automation). The re-review's eight lenses surfaced **two new majors and a set of minors, all second-order consequences of the fixes themselves**, and both majors plus the recurring minors were addressed in the same pass. The plan is now acceptable for implementation; the residual items are minor polish left as accepted tradeoffs, noted below.

### Previously Identified Issues

All resolved unless marked otherwise.

- 🔴 **Compatibility**: 1.25.0 removal unenforced — **Resolved** (Migration Notes now mandate a hard-refuse-with-migrate-error contract on the follow-up release; `REMOVAL_RELEASE` drift test added).
- 🟡 **Correctness**: `resolve_scheme` prefix leak (AC #7) — **Resolved** (field gated via `pattern_uses_key.then_some(...).flatten()`, with a field-gate assertion).
- 🟡 **Correctness**: Linear `sync.rs` scope without catalogue fallback — **Resolved** (routed through the Section 1 catalogue-aware resolver; catalogue-only parity criterion).
- 🟡 **Correctness**: `m0009` read-before-write ordering / ambiguous `work.key` — **Resolved** (read-once snapshot; `work.key` materialised only when the captured pattern referenced the prefix).
- 🟡 **Correctness**: Mixed-state guard underspecified — **Resolved** (guarded pairs enumerated; init-written / equal keys are removable redundancies, not aborts).
- 🟡 **Code Quality**: Rename crosses serde boundaries — **Resolved** (visualiser DTOs + `defaultProjectCode` wire key called out as an explicit decision).
- 🟡 **Code Quality**: Parallel `canonicalise_id` path — **Resolved** (folded onto the shared resolver).
- 🟡 **Code Quality / Correctness**: Warning returned as droppable data — **Resolved** (`resolve_scheme` returns `Resolved<T>`; single dedup collection point).
- 🟡 **Usability / Correctness / Compatibility**: Duplicate/contradictory warnings, false "deduped at emit" — **Resolved** (dedup on the deprecated key name; prefix-path warning suppressed when `{key}` absent; exactly-once criterion).
- 🟡 **Safety**: Init overwrite destroys a user-set key — **Resolved** (Linear given a confirmed-overwrite path matching Jira).
- 🟡 **Safety / Usability**: `m0009` `NoOpPending` perpetual nag — **Resolved** (`Applied` on a no-op run; ledger-advances/nag-silent criterion).
- 🟡 **Compatibility**: Both-keys silently un-warned yet unmigratable — **Resolved** (ignored-legacy warning; redundant legacy removed, not aborted).
- 🟡 **Compatibility**: Migrated config unreadable by older plugins — **Resolved** (forward-only mixed-version hazard documented with `.bak` recovery).
- 🟡 **Test Coverage**: Discovery-scope ACs not pinned to an emitted filter — **Resolved** (recorded `SearchScope` / `{team:{id:{eq:UUID}}}` assertion).
- 🟡 **Test Coverage**: AC #8 layering untested; AC numbering; both-sections manual-only — **Resolved** (AC #8 test, corrected AC traceability table, automated both-sections test).
- 🟡 **Usability**: `configure/SKILL.md` stale — **Resolved** (updated across Phases 3–5).
- 🟡 **Usability**: `work_key_required` under-specified — **Resolved** (enriched to `bad_integration` richness; disambiguates `{key}` from the scope key).
- 🔴 **Standards**: Count-test arithmetic (58→56) — **Resolved** (corrected to 56 with reasoning).
- 🟡 **Standards**: Public-api snapshots not regenerated — **Resolved** (criteria added to Phases 2–3).
- 🔵 **Minors** (helper signature, token duplication, `references_key` escaping, catalogue precedence, `0600` sidecar, partial-failure, single-emission, trello/github coverage, `REMOVAL_RELEASE` drift, `legacy_alias` declaration, elision comments, module registration) — **Resolved**.
- 🔵 **Test Coverage**: Jira init writeback automated criterion — **Resolved as documented decision** (the Jira section write is skill-driven and manually verified; recorded as a deliberate asymmetry with Linear's binary-level coverage).

### New Issues Introduced (by the fixes) — addressed this pass

- 🟡 **Correctness / Architecture / Test Coverage**: `resolve_active_scope_key` default arm (trello/github-issues/unset) was unspecified, risking a regression against the old unconditional `sync.rs:853` read — **Addressed**: the default arm now preserves the current `work.default_project_code` read, with a `work.integration: trello` sync-scope-unchanged test.
- 🟡 **Usability / Safety / Test Coverage**: The new confirmed-overwrite init flow had unspecified non-interactive behaviour — **Addressed**: non-interactive default is fail-safe (preserve + report), a `--force` flag performs the unattended overwrite AC #12 describes, driven through an injectable confirmer seam.
- 🔵 **Compatibility / Correctness / Usability**: Key-name dedup could show one remediation name for a dual-destination legacy key — **Addressed**: the surviving message leads with `Run /accelerator:migrate` rather than a single `set '<canonical>'`.
- 🔵 **Safety**: `0600` guarded only the `.bak`, not the rewritten `config.local.md` — **Addressed**: both the sidecar and the rewritten live file are forced to `0600`, with an assertion.
- 🔵 **Test Coverage / Compatibility**: `REMOVAL_RELEASE` "next minor" over a `-pre.N` version was ambiguous — **Addressed**: the test normalises to the release minor and asserts a later-minor lower bound.
- 🔵 **Code Quality / Standards**: The sketch shadowed `references_key` with a bool — **Addressed**: renamed to `pattern_uses_key`.
- 🔵 **Architecture**: The shared `references_key` bound one crate's tokeniser — **Addressed**: only the spelling set is shared; each crate walks its own tokeniser; the parity test asserts agreement.

### Residual — accepted, not changed

- 🔵 **Architecture** (minor): The `config` crate's helper reads `work.integration` internally, giving a low-level crate a work-domain concept. Accepted: `config` already holds domain catalogue knowledge, and passing the integration value through four call sites is not clearly better.
- 🔵 **Code Quality** (minor): The helper's adjacent `canonical`/`deprecated` `&str` params are transposable. Accepted: the `AliasedScalar` return names the result; a `KeyAlias` newtype is optional polish.
- 🔵 **Test Coverage** (minor, low): No recorded-scope assertion on the Jira dispatch arm specifically (the Linear filter is pinned). Left for implementation; Jira scope resolution is covered at the resolver level in Phase 4.

### Assessment

The plan is in good shape and ready to implement. The pass-2 computed verdict was COMMENT: no critical or major findings remained open, and the two majors the fixes introduced were themselves resolved. The three residual minors are accepted tradeoffs or implementation-level polish, not blockers.

**Reviewer decision (2026-09-10):** APPROVED. With all findings resolved or accepted, the reviewer approves the plan for implementation; the plan status is set to `ready`. The frontmatter `verdict` reflects this approval.

---
*Re-review generated by /accelerator:review-plan*
