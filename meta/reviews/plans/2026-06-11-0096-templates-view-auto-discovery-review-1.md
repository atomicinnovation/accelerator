---
type: plan-review
id: "2026-06-11-0096-templates-view-auto-discovery-review-1"
title: "Plan Review: Templates View Auto-Discovers Available Templates Implementation Plan"
date: "2026-06-12T00:57:07+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-11-0096-templates-view-auto-discovery"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, correctness, test-coverage, code-quality, portability, compatibility]
review_number: 1
review_pass: 2
tags: [visualiser, templates, plan-review]
last_updated: "2026-06-12T09:25:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Templates View Auto-Discovers Available Templates Implementation Plan

**Verdict:** REVISE

The plan is architecturally sound and unusually well-researched: it correctly
identifies the single leverage point (the launcher's hardcoded roster), exploits
an already-data-driven downstream chain that all six lenses independently
verified needs zero production change, and chooses the right build-time
discovery seam. The reservations are concentrated, not structural — three major
findings cluster around two themes (an unverified fourth tier key on the
riskiest edit, and a lost fail-fast guarantee in the new discovery loop) plus a
genuine traceability gap for AC #6. None invalidates the approach; all are
addressable with small, local edits to the plan. A REVISE pass to close the
`config_override_source` coverage hole, restore fail-fast in
`build_templates_json`, and correct the test-surface/line-number drift will make
this ready to implement.

### Cross-Cutting Themes

- **`config_override_source` — the fourth tier key — is verified nowhere**
  (flagged by: test-coverage, compatibility, correctness, code-quality). The
  real `template_tier` emits four keys (`config_override`, `user_override`,
  `plugin_default`, `config_override_source`), and the last feeds the view's
  Tier 1 description text. But the Phase 1 fixture uses a three-key shape (omits
  it, relying on `#[serde(default)]`), and the rewritten `config_contract.rs`
  asserts only the key-set + `plugin_default` suffix. So the jq restructure —
  the plan's own "structurally non-trivial" edit — could drop or mis-populate
  `config_override_source` and pass every planned test.

- **`build_templates_json` silently loses fail-fast under `set -euo pipefail`**
  (flagged by: code-quality, portability). The proposed
  `printf '%s\t%s\n' "$name" "$(template_tier "$name")"` runs the resolver
  inside a command substitution feeding `printf`; a `template_tier` failure does
  not abort the script (printf still succeeds with an empty field), unlike
  today's `ADR="$(template_tier adr)"` assignment which `set -e` catches. This
  contradicts the plan's own "fail loudly at boot" claim in Migration Notes.

- **Test-surface conflation and line-number drift** (flagged by: correctness,
  code-quality, test-coverage, compatibility). The plan repeatedly cites
  `config.rs:433` for the fixture assertion (live location is ~`460`), and its
  "real config emits 13 doc_paths, missing `review_work`" framing inverts which
  side is stale: `config_contract.rs:47` *already* asserts `doc_paths.len()==13`
  and includes `review_work`; only the static fixture (`config.valid.json` /
  `config.rs`) pins 12. An implementer following the citations edits the wrong
  region or distrusts a passing contract assertion.

- **The hand-synced fixture reintroduces the very drift the plan removes**
  (flagged by: architecture, code-quality). Phase 1 expands `config.valid.json`
  to a hand-maintained 13-entry list pinned to a literal `13`, while making the
  contract test drift-proof. Single-source-of-truth then holds for production
  but not for the fixture, which must be hand-edited on the next template added.

### Tradeoff Analysis

- **Fixture realism vs. drift-proofing**: Bumping the fixture to a literal 13
  (a deliberate decision during planning) improves how representative the sample
  config looks, but architecture and code-quality note it re-creates a manual
  sync point. Recommendation: keep the bump but reframe the fixture's role
  explicitly — it exercises the *deserializer shape*, not the live set;
  `config_contract.rs` is the authoritative generator-shape/contract test. This
  resolves the tension without reverting the decision.

- **Adding `config_override_source` to the fixture (fidelity) vs. minimal
  fixture (forgiving deserialization sample)**: compatibility/correctness want
  the fixture to mirror the four-key wire shape; the minimal-sample view says it
  only needs to exercise the deserializer. Recommendation: do not rely on the
  fixture for this — close the gap where it matters by asserting
  `config_override_source` at the *launcher-unit* level (both the `null` case
  and a populated config-override case), which covers the production emission
  directly.

### Findings

#### Critical

_None._

#### Major

- 🟡 **test-coverage**: `config_override_source` is dropped from the fixture and asserted by no test
  **Location**: Phase 1 §2 (fixture refresh) & Phase 2 §1 (contract test rewrite)
  `TemplateTiers` has four fields; `template_tier` emits all four and
  `config_override_source` drives the view's Tier 1 description. The fixture uses
  three keys and the contract test asserts only key-set + `plugin_default`
  suffix, so a regression in the jq restructure that dropped or mis-populated
  this key would pass every planned test while quietly breaking the view.

- 🟡 **code-quality / portability**: `build_templates_json` loses fail-fast under `set -euo pipefail`
  **Location**: Phase 2 §3(b) (discovery builder)
  `printf '%s\t%s\n' "$name" "$(template_tier "$name")"` swallows a
  `template_tier` failure — `printf` still succeeds with an empty second field,
  and the pipeline status reflects only the trailing `jq`. This regresses the
  current `ADR="$(template_tier adr)"` form, where `set -e` aborts on a failed
  resolution, and undercuts the plan's "fail loudly at boot" guarantee.

- 🟡 **test-coverage**: AC #6 (tier-presence indicators) has no automated coverage and the plan does not credit the resolver tests that do cover it
  **Location**: Phase 2 §2 (launcher-unit coverage) & Testing Strategy
  The launcher-unit cases assert only the `plugin_default`/`user_override` path
  *strings* (which `template_tier` emits unconditionally), never the
  `present`/`active` logic that drives the indicators, and AC #6 is routed to
  Manual Verification. The indicator behaviour is in fact already covered by the
  name-agnostic resolver tests in `templates.rs`
  (`only_plugin_default_present_picks_plugin_default_active`,
  `all_three_tiers_present_picks_config_override_as_active`,
  `user_override_wins_when_config_override_absent`) — but the plan never makes
  that connection, so AC #6's traceability reads as a hole.

#### Minor

- 🔵 **correctness**: Contract-test `read_dir` filter does not mirror the helper's regular-file (`-f`) guard
  **Location**: Phase 2 §1 (rewritten `config_contract.rs`)
  `config_enumerate_templates` applies `[ -f "$f" ] || continue`; the in-test
  `read_dir(...).filter(extension == "md")` does not. A future directory or
  symlink named `*.md` under `templates/` would make `expected` a superset of
  `actual` and fail the assertion even though the launcher is correct. Add
  `&& p.is_file()` to mirror the guard. (Latent given the curated dir.)

- 🔵 **correctness / code-quality**: Tab-delimited splice assumes tab-free basenames and single-line tier JSON, undocumented
  **Location**: Phase 2 §3(b)
  `split("\t")` over `printf '%s\t%s\n'` is correct only while basenames contain
  no tab and `template_tier` stays single-line (`jq -nc`). Both hold today;
  neither is asserted. Note the invariants in the builder comment and
  cross-reference the `jq -nc` dependency from `template_tier`.

- 🔵 **correctness / code-quality / test-coverage / compatibility**: Test-surface conflation & drifted line numbers
  **Location**: Current State Analysis blast-radius table, Phase 1 §3 header, What We're NOT Doing, References
  Cites `config.rs:433` (live ~`460`); frames `doc_paths`==12 as the "real
  config missing review_work" when `config_contract.rs:47` already asserts 13.
  Attribute `templates.len()==8` to `config_contract.rs:72` and the stale
  `doc_paths.len()==12` solely to the `config.rs` fixture test.

- 🔵 **architecture / code-quality**: Hand-synced fixture reintroduces the drift class the plan removes
  **Location**: Phase 1 §2
  The fixture becomes a hand-maintained 13-entry duplicate pinned to a literal
  count; single-source-of-truth holds for the launcher but not here. Reframe the
  fixture's role (deserializer sample, not live mirror) or assert
  shape/representative-key invariants rather than an exact count.

- 🔵 **test-coverage**: The config-override-only exclusion case is green against the unchanged script
  **Location**: Phase 2 Overview (red→green claim) & §2
  `zzz-fake` was never in the old roster, so `has("zzz-fake") == false` already
  passes before the change — the blanket "written first and fail (red)" claim is
  inaccurate for this case. Label it a characterisation/lock test, or strengthen
  it so the new behaviour (not the old roster's incidental absence) is what
  bites.

- 🔵 **test-coverage**: AC #3/#4 (add/remove through the script) are never round-tripped through `write-visualiser-config.sh`
  **Location**: Testing Strategy / Manual Testing Steps
  Add/remove is covered at the helper-count level and via set==directory, but no
  automated test mutates a `*.md` and re-runs the production script. Defensible
  via the set==directory tautology — state that transitive reasoning explicitly,
  or add a launcher-unit case asserting the emitted template count equals the
  real `config_enumerate_templates` count.

- 🔵 **architecture**: Contract test couples to a fixed relative directory depth shared with the launcher's `PLUGIN_ROOT`
  **Location**: Phase 2 §1 (`CARGO_MANIFEST_DIR/../../../../templates`)
  The drift-proof test now enforces the contract via two independently-maintained
  depth constants (this `../../../../` and the launcher's `PLUGIN_ROOT`) that
  must agree. Acceptable given the stable layout; note it as a known fragility or
  resolve relative to the script's `PLUGIN_ROOT`.

- 🔵 **architecture**: Phase-ordering rationale overstates a non-existent dependency
  **Location**: Implementation Approach / Phase 1 Overview
  Phase 2's script change depends on no Phase 1 artifact (disjoint test
  surfaces). The "characterise the primitive before the feature depends on it"
  framing is decorative; reframe as independent refactors presented Phase-1-first
  for narrative clarity.

- 🔵 **compatibility**: `note.user_override` assertion pins a candidate path, not a resolved-present one
  **Location**: Phase 2 §2
  `make_project` never creates `.accelerator/templates/`, so the assertion relies
  on `template_tier` emitting the user-override candidate unconditionally
  (presence decided server-side). Correct today; add a one-line comment so the
  pinned contract is unambiguous.

#### Suggestions

- 🔵 **architecture**: Add an explicit note that K=0 (empty `templates/`) yields a valid empty map and an empty view with no boot error — intended graceful degradation, not a silent fault. **Location**: Migration Notes / Phase 2 §3(b).

- 🔵 **test-coverage / portability**: K=0 is verified only at the helper level; consider one assertion that pipes empty enumeration through the same `jq -Rn 'reduce inputs ...'` fold (or an empty-dir launcher-unit case) to lock the splice's empty-input behaviour. **Location**: Phase 2 §3(b).

- 🔵 **test-coverage**: The existing enumerate test uses a substring `assert_contains "research"` that `codebase-research` happens to satisfy; the plan over-credits it as exact 13-key membership. Correct the characterisation or tighten to exact-name membership while Phase 1 already edits the block. **Location**: Current State Analysis / `test-config.sh:5015`.

### Strengths

- ✅ Correctly locates the single architectural leverage point: every downstream
  consumer (`config.rs` `HashMap<String, TemplateTiers>`, the map-iterating
  `TemplateResolver`, the `/api/templates` verbatim passthrough, the React
  `.map` with no allow-list/switch) was independently verified name-agnostic and
  untouched — textbook "change one seam" design.
- ✅ Reuses `config_enumerate_templates` and `template_tier` unchanged rather
  than reimplementing, so the three-tier resolution (ADR-0017) and the
  `deny_unknown_fields` per-template shape are preserved by construction.
- ✅ The rewritten `config_contract.rs` derives its expected set from the same
  `templates/` directory the launcher scans, making the end-to-end contract
  drift-proof instead of re-pinning a literal roster in the test.
- ✅ The blast-radius correction is the product of genuine verification:
  `config_contract.rs` really is the only breaking test, and
  `test-launch-server.sh:87-92` genuinely survives (its six names remain in the
  discovered set, and `template_tier` emits the same paths unconditionally).
- ✅ K=0 boundary reasoning is correct (`reduce inputs` over zero lines → `{}`),
  the jq trailing-comma edit is syntactically clean, and the
  `CARGO_MANIFEST_DIR/../../../../templates` path resolves to the right
  directory.
- ✅ The `rca`-only glyph gap, the name-agnostic frontend, and the
  non-breaking/non-verifying e2e fixture (`start-server.mjs`) are all confirmed
  precisely against the live checkout.
- ✅ Count/add/remove coverage is correctly placed at the helper-unit level where
  K is directly controllable; phasing keeps each merge unit green.

### Recommended Changes

1. **Assert `config_override_source` at the launcher-unit level** (addresses:
   "config_override_source verified nowhere"). In Phase 2 §2 add a case that a
   discovered template's `config_override_source` is `null` with no override,
   and one where a `.accelerator/config.md` template override populates it —
   exercising all four keys the production jq emits. Optionally also add the key
   to the new fixture entries so `config.valid.json` mirrors the wire shape.

2. **Restore fail-fast in `build_templates_json`** (addresses: "loses fail-fast
   under set -euo pipefail"). Capture the tier on its own line before printing:
   `local tier; tier="$(template_tier "$name")" || return 1; printf '%s\t%s\n' "$name" "$tier"`,
   and document the propagation contract in the comment. Optionally assert in the
   jq reduce that each `$v` parses to an object.

3. **Connect AC #6 to its real coverage** (addresses: "AC #6 traceability gap").
   In the Testing Strategy, state that the `present`/`active` indicator behaviour
   is covered by the pre-existing, name-agnostic `templates.rs` resolver tests
   and that discovery adds no tier-presence code, so those tests apply unchanged
   to every discovered name.

4. **Fix the test-surface conflation and line numbers** (addresses: "test-surface
   conflation & drifted line numbers"). Update citations to `config.rs:~460`
   (`parses_valid_config` / `templates.len()==8` fixture pin), attribute the
   live `templates.len()==8` script-contract assertion to `config_contract.rs:72`,
   and correct the `doc_paths` note to say the *fixture* is stale at 12 while
   `config_contract.rs:47` already asserts 13.

5. **Mirror the helper's `-f` guard in the contract test** (addresses: "read_dir
   filter does not mirror -f guard"). Add `&& p.is_file()` to the `read_dir`
   filter so the in-test derivation matches `config_enumerate_templates` exactly.

6. **Tighten the red→green and drift framings** (addresses: "override-only case
   is green", "phase-ordering overstates dependency", "hand-synced fixture
   drift"). Label the override-only exclusion as a green-under-both lock test,
   reframe the phase ordering as independent refactors, and add a one-line note
   that the fixture exercises the deserializer shape while `config_contract.rs`
   is the authoritative generator/contract test.

7. **Document the load-bearing invariants** (addresses: "tab-split assumptions",
   "candidate path", "K=0 graceful degradation"). Note the tab-free-basename +
   single-line-tier-JSON invariants in the builder, comment that `user_override`
   is an unconditional candidate path, and state that an empty `templates/`
   yields a valid empty map / empty view with no boot error.

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound — correctly identifies the single leverage
point and exploits an already-data-driven downstream chain that needs zero
production change; the build-time discovery seam and its tradeoff are chosen and
acknowledged well, and the per-template shape contract is preserved by reusing
`template_tier`. Main reservations: single-source-of-truth is only partially
achieved (the contract test is drift-proof but the fixture remains a hand-synced
duplicate), and the two-phase ordering rationale overstates a dependency that
does not exist.

**Strengths**:
- Textbook "change one seam": the only roster-imposing component is the
  launcher's config-assembly; the whole chain past `config.json` is verified
  name-agnostic and untouched.
- Reuses `config_enumerate_templates` + `template_tier` unchanged, preserving
  ADR-0017 three-tier resolution and the `deny_unknown_fields` shape by
  construction.
- Build-time seam is the right boundary (the runtime watcher watches content,
  not membership); the relaunch-to-reflect tradeoff is explicit.
- Contract test derives its expected set from disk → genuinely drift-proof.
- Scope boundaries (leaving `TEMPLATE_KEYS` to 0029, partial-fix-is-intentional)
  are explicit and coherent.

**Findings**:
- 🔵 minor (high): Single-source-of-truth achieved for the launcher but not for
  the deserialization fixture, which remains a hand-synced duplicate pinned to a
  literal 13 — the same drift class, relocated into a test fixture. (Phase 1 §2)
- 🔵 minor (medium): The phases are genuinely independent, but "Phase 1
  characterises the primitive before the feature depends on it" overstates a
  dependency that does not exist (Phase 2's script change consumes no Phase 1
  artifact). (Implementation Approach / Phase 1 Overview)
- 🔵 minor (medium): The drift-proof contract test couples to a fixed relative
  directory depth (`../../../../templates`) that must stay in lockstep with the
  launcher's `PLUGIN_ROOT` computation; an acceptable test-only coupling worth
  noting. (Phase 2 §1)
- 🔵 suggestion (high): `deny_unknown_fields` is well-leveraged as a fail-loud
  guardrail; the K=0 empty-map path deserves an explicit "empty view is
  expected-and-valid" boot-behaviour note. (Migration Notes / Phase 2 §3(b))

### Correctness

**Summary**: Core logic is sound — the `jq -Rn 'reduce inputs ...'` splice
correctly yields `{}` at K=0 and one entry per template otherwise; the
tab-delimited encoding is safe given `template_tier`'s `jq -nc` single-line
output; the `--argjson`/trailing-comma edits are syntactically clean; and the
`CARGO_MANIFEST_DIR/../../../../templates` path resolves to the directory the
launcher scans. Main gaps are minor: the contract test's `read_dir` filter does
not mirror the helper's `-f` guard, and the tab-split's basename/single-line
assumptions are undocumented (both prevented by the curated-templates domain).

**Strengths**:
- K=0 handled correctly: empty loop → empty stdin → `reduce inputs` returns the
  initial `{}` (valid empty object, not error/null).
- Tab-delimiter splice safe in the common case (`jq -nc` compact, single-line).
- jq `-n` edits syntactically clean — the replacement preserves the trailing
  comma before `work_item`.
- Contract-test path is correct (four `..` from the server crate reach repo
  root).
- Fixture/assertion consistency sound (leaving `doc_paths.len()==12` is correct
  for the static fixture); server alphabetical sort verified, so config.json key
  order is irrelevant; `config_override_source` is `#[serde(default)]`.

**Findings**:
- 🔵 minor (high): Contract test `read_dir` filter lacks the helper's `-f` /
  `is_file()` guard — a future non-regular `*.md` entry would fail the
  set-equality assertion though the launcher is correct. (Phase 2 §1)
- 🔵 minor (medium): Tab-split silently assumes tab-free basenames and single-line
  tier JSON; hold today, neither guarded — note the invariants. (Phase 2 §3(b))
- 🔵 minor (high): Cited line numbers drifted — `config.rs:433` vs live ~`460`
  for the fixture assertion; would misdirect the implementer. (Current State /
  References)
- 🔵 suggestion (medium): Fixture entries omit `config_override_source` while the
  live script always emits it; deserialization passes (default) but the fixture
  no longer mirrors the wire shape. (Phase 1 §2)

### Test Coverage

**Summary**: Well-structured pyramid (count/add/remove at helper-unit,
set-equality at launcher-integration + Rust-contract, deserialization at
Rust-unit), and the core red→green and blast-radius claims are accurate.
However, several acceptance criteria map loosely: AC #6 (tier-presence) is
"wiring inputs" only and never connected to the existing `templates.rs` resolver
tests that cover it; `config_override_source` is dropped from the fixture and
asserted nowhere; one "red" test is in fact green against the unchanged script.

**Strengths**:
- The blast-radius table is genuinely verified (`config_contract.rs` is the only
  breaking test; `test-launch-server.sh` survives).
- Count/add/remove correctly placed at the helper-unit level where K is
  controllable.
- The rewritten contract test re-scans `templates/` in-test → drift-proof,
  avoiding a hardcoded-roster-in-the-test anti-pattern.
- TDD coupling note (script + `config_contract.rs` must land together) is
  correct; phasing keeps each merge unit green.

**Findings**:
- 🟡 major (high): AC #6 (tier-presence indicators) has no automated coverage and
  the plan does not credit the existing name-agnostic `templates.rs` resolver
  tests that cover the `present`/`active` logic. (Phase 2 §2 / Testing Strategy)
- 🟡 major (high): `config_override_source` (the fourth tier key, feeding the
  view's Tier 1 description) is dropped from the fixture and asserted by no test;
  the riskiest edit could regress it undetected. (Phase 1 §2 / Phase 2 §1)
- 🔵 minor (high): The config-override-only exclusion test is green against the
  unchanged script (`zzz-fake` was never in the old roster), contradicting the
  blanket red→green claim. (Phase 2 Overview / §2)
- 🔵 minor (medium): AC #3/#4 (add/remove through the script) are inferred from
  set-equality + helper counts, never round-tripped through
  `write-visualiser-config.sh`. (Testing Strategy / Manual Testing Steps)
- 🔵 minor (medium): The "real config missing `review_work`" framing contradicts
  the live `config_contract.rs:46-65`, which already asserts 13 and includes
  `review_work`; only the fixture is stale. (Blast-radius table / What We're NOT
  Doing)
- 🔵 suggestion (low): K=0 behaviour of the jq reduce is verified only by reading,
  not by a test exercising the fold. (Phase 2 §3(b))
- 🔵 suggestion (medium): The existing enumerate test uses a substring
  `assert_contains "research"`; the plan over-credits it as exact 13-key
  membership. (Current State / `test-config.sh:5015`)

### Code Quality

**Summary**: Well-structured plan; `build_templates_json` is cleanly designed,
well-named, and matches the file's comment density and jq conventions. The main
concern is that the discovery loop's command-substitution-inside-`printf`
pattern silently loses the fail-fast property the existing per-template scalar
assignments have under `set -euo pipefail`. The contract-test and shell-test
rewrites are otherwise idiomatic and readable.

**Strengths**:
- `build_templates_json` is a focused, well-named single-responsibility helper
  matching existing block style.
- Collapsing eight `--argjson` flags + a literal object into one `--argjson
  templates` is a real DRY/maintainability win.
- Reuses the established `for KEY in $(config_enumerate_templates ...)` idiom
  from sibling scripts.
- The drift-proof contract test is a clean, maintainable design.
- New shell test cases follow existing harness conventions.

**Findings**:
- 🟡 major (high): The discovery loop's `"$(template_tier "$name")"` inside
  `printf` swallows resolution failures under `set -euo pipefail`, regressing the
  current fail-fast assignment form. (Phase 2 §3(b))
- 🔵 minor (medium): Even setting aside the masking, error propagation through the
  `for ... done | jq` pipeline is non-obvious; document or restructure the
  fail-fast contract. (Phase 2 §3(b))
- 🔵 minor (high): The Current State table conflates the two test surfaces —
  `templates.len()==8` is `config_contract.rs:72`; the stale `doc_paths==12` is
  the `config.rs` fixture test, not the contract test. (Phase 1 / Current State)
- 🔵 minor (medium): The hand-edited three-key fixture diverges from the
  four-key generator output — the drift class this work item removes; note the
  fixture's role or add a clarifying comment. (Phase 1 §2)
- 🔵 suggestion (low): The tab-delimited `reduce` splice is load-bearing on the
  `jq -nc` single-line invariant; cross-reference the dependency from
  `template_tier`. (Phase 2 §3(b))

### Portability

**Summary**: The shell additions stay within the repo's bash 3.2-safe
vocabulary: the `for name in $(config_enumerate_templates ...)` idiom already
ships in two siblings under the same `set -euo pipefail` with no SC2046
suppression; jq 1.7.1 supports every construct used; and the `printf '\t'` / jq
`split("\t")` tab handling plus the Rust `read_dir` + PathBuf join are portable
across the darwin+linux CI matrix. The one risk worth surfacing is the `set -e`
masking of the inner `$(template_tier ...)` failure.

**Strengths**:
- The discovery idiom is proven in `config-list-template.sh:21` and
  `config-eject-template.sh:121` under `set -euo pipefail`, passing ShellCheck
  cleanly on space-free basenames.
- No bash-4 constructs introduced (no mapfile/associative arrays/nullglob/case
  modification) → passes the bashisms denylist.
- jq pinned at 1.7.1; `-Rn`/`reduce inputs`/`split`/`fromjson`/destructuring all
  available since 1.6.
- The Rust rewrite uses cross-platform `Path::join`/`extension`/`file_stem` and
  derives the set from disk.

**Findings**:
- 🔵 minor (high): `printf '%s\t%s\n' "$name" "$(template_tier "$name")"` does
  not abort on an inner resolution failure under `set -euo pipefail`; capture the
  tier into a variable first. (Phase 2 §3(b))
- 🔵 suggestion (medium): The K=0 path is exercised only at the helper level, not
  end-to-end through `build_templates_json`; consider an empty-dir launcher-unit
  assertion. (Phase 2 §3(b))

### Compatibility

**Summary**: A low-risk, additive change from a contract standpoint: the
per-template wire shape is preserved exactly (Phase 2 reuses `template_tier`
unchanged), and the whole downstream chain is genuinely name-agnostic as claimed
— all verified against the live checkout. The only contract that changes is the
*set* of names, unconstrained by every consumer; the `rca`-only glyph claim is
correct and degrades gracefully. The residual concern is the three-key fixture
shape diverging from the four-key script output, which masks rather than
verifies the full contract.

**Strengths**:
- Four-key shape preserved trivially and exactly via the unchanged
  `template_tier`; only the key *set* changes.
- Name-agnostic claim verified end-to-end (`HashMap`, passthrough,
  `TemplateSummary`, single `.map`, no allow-list/switch).
- `rca`-only glyph gap confirmed precisely; row still renders.
- e2e claim holds (`start-server.mjs` builds its own fixture, never runs the
  launcher); no e2e spec asserts template names/counts.
- Backward compatibility for config-overrides preserved (discovery keys on
  basename; `template_tier` still reads `templates.<name>`).

**Findings**:
- 🔵 minor (high): Phase 1 fixture entries omit `config_override_source` (the
  fourth key the launcher always emits); `parses_valid_config` exercises the
  forgiving three-key shape, not the real wire contract. (Phase 1 §2)
- 🔵 minor (high): The "real config emits 13 doc_paths, missing review_work" note
  conflates two surfaces — `config_contract.rs:47` already asserts 13; only the
  fixture (`config.rs`) pins 12. (What We're NOT Doing)
- 🔵 minor (medium): The `note.user_override` assertion pins an *unconditional
  candidate* path (the project never creates `.accelerator/templates/`); add a
  clarifying comment. (Phase 2 §2)

---

## Re-Review (Pass 2) — 2026-06-12T09:25:43+00:00

**Verdict:** APPROVE

The revised plan resolves all three major findings and effectively every minor
across the six re-run lenses. The agents verified each fix against the live
checkout (the `config_override_source` provenance trace, the `set -e`/`pipefail`
propagation chain, the `templates.rs` resolver-test names, the corrected line
numbers, and that the enriched fixture has no cross-test blast radius). The
re-review surfaced only small NEW items, all minor/suggestion; the two actionable
ones were applied in this pass, leaving no outstanding actionable findings.

### Previously Identified Issues

- 🟡 **test-coverage**: `config_override_source` dropped from fixture / asserted nowhere — **Resolved** (fixture carries the 4th key on all 13 entries; `parses_valid_config` asserts it; launcher-unit null + populated cases added).
- 🟡 **code-quality / portability**: `build_templates_json` loses fail-fast under `set -euo pipefail` — **Resolved** (tier captured in its own assignment; propagation chain verified correct against the live script).
- 🟡 **test-coverage**: AC #6 (tier-presence) uncredited coverage — **Resolved** (named `templates.rs` resolver tests `:377/:395/:418/:436`; new "Pre-existing coverage relied upon" subsection).
- 🔵 **correctness**: contract-test `read_dir` filter vs `-f` guard — **Resolved** (`p.is_file()` added).
- 🔵 **correctness / code-quality**: tab-split invariants undocumented — **Resolved** (invariants noted in the builder comment).
- 🔵 **correctness / code-quality / test-coverage / compatibility**: line-number & test-surface drift — **Resolved** (citations corrected: `config.rs:460/456/452`, `config_contract.rs:46/72`; doc_paths framing fixed).
- 🔵 **architecture / code-quality**: hand-synced fixture reintroduces drift — **Resolved** (fixture reframed as a deserializer-shape sample; `config_contract.rs` named authoritative).
- 🔵 **test-coverage**: override-only exclusion green-under-both vs red→green claim — **Resolved** (labelled a characterisation/lock test in three places).
- 🔵 **architecture**: phase-ordering overstated a dependency — **Resolved** (reframed as independent, either-order).
- 🔵 **compatibility**: `note.user_override` candidate-path undocumented — **Resolved** (NOTE added).
- 🔵 **code-quality**: pipeline exit-status propagation non-obvious — **Resolved** (explained in the comment).
- 🔵 **test-coverage**: AC #3/#4 not round-tripped through the script — **Partially resolved / accepted** (helper-level + set==directory coverage; end-to-end add/remove remains manual by design — reasonable given the contract test re-derives the set each run).
- 🔵 **architecture**: contract-test directory-depth coupling to `PLUGIN_ROOT` — **Still present, deliberately accepted** (test-only, self-announcing on failure; agent agreed the tradeoff is reasonable).
- 🔵 **test-coverage / portability / architecture**: K=0 fold not exercised end-to-end — **Accepted via documentation** (unreachable for the fixed plugin `PLUGIN_ROOT`; helper-level K=0 covered; Migration Notes document graceful degradation).
- 🔵 **test-coverage**: existing substring `assert_contains "research"` over-credited — **Resolved** (promoted from "optional" to a definite exact-line tightening in Phase 1 §1).

### New Issues Introduced

- 🔵 **code-quality** (minor): the `set -e` propagation chain is sensitive to a future `local tier="$(…)"` collapse that would re-mask the failure — **Addressed this pass** (added a guard note to the builder comment).
- 🔵 **test-coverage** (minor): the populated config-override case asserted only `config_override_source`, not `config_override` itself — **Addressed this pass** (added the sibling `config_override == "custom/rca.md"` assertion).
- 🔵 **architecture** (suggestion): the contract test re-implements the helper's filter in Rust (a second, language-divergent definition of "what is a template") — **Accepted** (intrinsic to a cross-language contract test; the "mirror the helper" comment is the right mitigation; revisit only if the discovery rule grows beyond a plain `*.md` glob).
- 🔵 **compatibility / portability** (suggestions): clearances only — the enriched fixture remains a valid `TemplateTiers` deserialization with single-consumer blast radius, and the new shell idioms are bash-3.2/BSD-portable and ShellCheck-clean. No action.

### Assessment

The plan is ready to implement. All majors are resolved, the framing and line
references are accurate against the live checkout, and the two actionable new
minors were applied in this pass. The remaining items are explicitly-accepted,
low-risk tradeoffs (test-only directory coupling; manual end-to-end add/remove;
documented-unreachable K=0), each verified reasonable by the re-review.

---
*Review generated by /accelerator:review-plan*
