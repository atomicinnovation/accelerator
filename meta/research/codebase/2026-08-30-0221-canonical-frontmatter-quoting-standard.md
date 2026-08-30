---
type: "codebase-research"
id: "2026-08-30-0221-canonical-frontmatter-quoting-standard"
title: "Research: Canonical Quoting Standard for All Frontmatter"
date: "2026-08-30T13:56:26+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0221"
parent: "work-item:0221"
relates_to: ["adr:ADR-0065", "adr:ADR-0033", "adr:ADR-0034", "work-item:0220", "work-item:0227"]
topic: "Canonical Quoting Standard for All Frontmatter"
tags: ["research", "codebase", "frontmatter", "corpus", "document", "validator", "migration"]
revision: "8b872872cbe2a523a90c7cfe20ba60210b8db06a"
repository: "accelerator"
last_updated: "2026-08-30T14:39:59+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Resolved Python-lint scope (retire) and the sixth-write-path question (visualiser is not a renderer producer); grounded the Rust template-shape check re-home"
schema_version: 1
---

# Research: Canonical Quoting Standard for All Frontmatter

**Date**: 2026-08-30T13:56:26+00:00
**Author**: Toby Clemson
**Git Commit**: 8b872872cbe2a523a90c7cfe20ba60210b8db06a
**Branch**: (detached — no bookmark)
**Repository**: accelerator

## Research Question

Research work item 0221 (canonical quoting standard for all frontmatter) across
the codebase: the emitter, the validator, the producer write paths, the
migration framework, and the producer skills. Additionally — and specifically —
find every test, lint, and guard that pins particular frontmatter behaviour so
it can be removed or generified as the quoting standard becomes type-driven.
Those pins are in scope of any resulting plan.

## Summary

The fix is smaller than the work item implies at its core and larger at its
edges. **The emitter change is one match arm.** Every frontmatter write in `cli/`
funnels through `document::render` → `serde_saphyr::to_string` → the
`Serialize for Yaml` impl, and the `Scalar::String` arm at
`cli/document/src/value.rs:173-175` is the single choke point — sequence
elements recurse back through it via `FlowSeq`, so one arm governs both mapping
values and list elements. Integers, booleans, and null already emit bare
intrinsically. The mechanism is serde-saphyr's per-value `DoubleQuoted<T>`
wrapper, which quotes in both block and flow position **and disables
block-scalar folding for that value** — directly removing the `>-` title-refold
and array-collapse churn that motivated PR #76. serde-saphyr's global
`quote_all` knob is the wrong tool: it prefers single quotes.

**The edges are where scope lives, and the additional ask uncovered two surfaces
the work item does not name.** First, `cli/work/src/tags.rs:47-62` carries a
*second, independent* minimal-quoting rule (`needs_quoting`/`format_tag`) for tag
items, with its own tests — dead weight once the renderer quotes every string
element, and a candidate to remove. Second, and more consequential, there is a
**parallel frontmatter-rules surface written in Python** —
`tasks/lint/frontmatter_rules.py` — carrying `ID_QUOTED_RE`,
`SCHEMA_VERSION_RE`, and `TYPED_REF_RE` field-specifically, consumed by its own
test suite (`test_frontmatter_rules.py`, `test_template_frontmatter.py`,
`test_conformance.py`). **Decision (2026-08-30): retire it** in favour of the
Rust validator. ⚠️ Retirement is not a clean delete — `test_template_frontmatter.py`
is the *only* validator of `templates/*.md`, and the Rust validator cannot
substitute (templates are a virtual doc-type the walker excludes, and their
placeholder values flood the instance-validator with false violations). Template
validation must therefore be **re-homed**, not dropped. See the follow-up
section for the full retirement map.

**The generification target is a clean, enumerable set.** In the Rust validator,
`check_id_quoting` (`UnquotedId`) and the `quoted:false` branch of
`check_linkage_shape` (`BadLinkageShape`) collapse into one general rule: a bare
scalar passes only if it is an integer/boolean/null literal or a flow collection
whose elements recurse; anything else bare is an unquoted-string violation. Two
carve-outs resist the fold and must stay as separate value constraints:
`check_schema_version` (inverse polarity — *requires* bare `1`) and
`check_timestamps` (a format check, quote-agnostic today; under the general rule
a bare ISO timestamp newly becomes a violation). The `public-api.txt` snapshots
for `cli/corpus` and `cli/document` are guards that will trip when the
`Violation` taxonomy or the value-tree surface changes.

**The migration is a mechanical migration `0008`** modelled on `m0006`/`m0007`,
reusing the existing `MigrationContext::validate_frontmatter` port for the
`meta/` self-check plus an explicit `.accelerator/config.md` pass. Three
test-maintenance tripwires apply: the `migrate` `public-api.txt`, the
per-test "all other migrations applied" isolation lists, and
`full_registry_e2e.rs`.

**Producer-skill wiring follows an existing pattern.**
`tests/unit/tasks/test_skill_frontmatter_population.py` (closed allowlist +
`skills-schema.tsv` + discovery gate) is the exact template for AC #6's static
coverage test — but its current producer set does not match the 22 named skills
(six are absent), so the new test needs its own list. The single-file
invocation `accelerator corpus frontmatter validate --file <path>` is already
supported.

## Detailed Findings

### The emitter: one choke point (`cli/document/`)

The whole render pipeline is one function deep. `render` composes the preserved
body with `emit`'s output (`cli/document/src/render.rs:19-28`); `emit` is the
sole serde-saphyr call, passing **no options** so `SerializerOptions::default()`
governs (`render.rs:36-43`). Quoting is thus entirely delegated to serde-saphyr's
minimal-quoting heuristic — plain when safe, double-quoted when ambiguous,
**block-scalar (`>`/`|`) when long or multiline** (`prefer_block_scalars:
true`). That block-scalar default is the churn source.

The value tree carries an explicit scalar discriminant, which is exactly the
signal a type-driven emitter needs. `Scalar` has five variants — `String`,
`Bool`, `Int`, `Float`, `Null` (`cli/document/src/value.rs:22-29`) — and the
`Serialize for Yaml` impl dispatches per variant (`value.rs:157-189`):

- `Scalar::String` → `serialize_str` (`value.rs:173-175`) — **the fix point.**
- `Scalar::Int`/`Bool`/`Null` → `serialize_i64`/`serialize_bool`/`serialize_unit`
  — never quote; the "leave bare" behaviour is already intrinsic.
- `Sequence` → `serde_saphyr::FlowSeq(items)` (`value.rs:177`) — forces `[...]`
  flow style; each element recurses back through the `String` arm.
- `Mapping` → block map, insertion order preserved (`value.rs:179-186`).

⚠️ Because sequences recurse through the `String` arm, wrapping that one arm in
`DoubleQuoted` covers mapping values and list elements together. serde-saphyr is
pinned `=0.0.29` (`cli/Cargo.toml:88`) and confined to `cli/document` by
convention; `DoubleQuoted<T>` (`T: AsRef<str>`) is re-exported and emits double
quotes in both positions while suppressing block folding for the value —
`cli/document/tests/document.rs` today asserts only body round-trip, never a
quoting style, so this arm has no exact-output test to update, only new ones to
add.

⚠️ `Scalar::Float` currently emits bare via `serialize_f64`. ADR-0065's closed
bare set is `{integer, boolean, null}` — floats are to be **quoted**. Quoting a
genuine float changes its parse-back type to string, so the emitter and
validator must agree on `Float`, and the plan should confirm no float-typed
frontmatter or config value exists (see Open Questions).

### The second quoting rule: `cli/work/src/tags.rs`

Tag items are pre-formatted by an independent minimal-quoting rule before they
reach the value tree. `needs_quoting`/`format_tag`/`build_canonical`
(`cli/work/src/tags.rs:47-62`) quote a tag only when it contains `,`, `:`, or
`#`. Its tests pin bare output — `mutate_tags` yielding `[a, b, c]`, `[b]`
(`tags.rs:152-171`, `:208-217`), and `tags_needing_quoting_are_quoted_on_rebuild`
asserting a plain item `z` stays bare (`tags.rs:221-229`).

Once the renderer double-quotes every string element, this rule is superseded —
tag items emit as `["a", "b", "c"]` regardless of `format_tag`. The plan should
generify or remove `needs_quoting`/`format_tag` and flip these tests. `FieldValue`
and the `tags` module surface are pinned by `cli/work/tests/fixtures/public-api.txt`
(`:3-6`, `:49`, `:707-716`), but only signature changes trip it — internal
quoting changes are private.

### The Rust validator: field-specific checks to fold (`cli/corpus/`)

The validator deliberately scans **raw frontmatter text**, not a parsed tree,
precisely so `id: "0042"` and `id: 0042` stay distinguishable
(`cli/corpus/src/frontmatter_validation/mod.rs:5-13`). A two-stage design gates
on a value-tree well-formedness classifier (`NO-FENCE`), then runs eleven
text-scanning checks. The full taxonomy is 17 codes
(`frontmatter_validation/violation.rs:73-93`).

The checks split cleanly into field-name-keyed (candidates to generify) and
type/schema-row-driven (already general, unaffected):

| Check | Code | Keyed on | Fate under general rule |
|---|---|---|---|
| `check_id_quoting` | `UNQUOTED-ID` | literal `id` | Folds — special case of "bare string is a violation" |
| `check_linkage_shape` (`quoted:false` branch) | `BAD-LINKAGE-SHAPE` | `row.typed_linkage_keys` | Folds — bare/single-quoted element is the unquoted-string case |
| `check_empty_placeholders` | `EMPTY-PLACEHOLDER` | all keys but `tags` | Adjacent — literal `""`/`[]` match, orthogonal to quoting |
| `check_schema_version` | `BAD-SCHEMA-VERSION` | literal `schema_version` | Stays — inverse polarity, *requires* bare `1` |
| `check_timestamps` | `BAD-TIMESTAMP` | `["date","last_updated"]` | Stays as format check; quoting newly enforced by general rule |

The linkage check's shape grammar (`shape::is_well_formed`,
`frontmatter_validation/shape.rs:16-27`) validates the `type:id` *value* and is a
separate concern from quoting — the `quoted:true` malformed branch stays. Note
the double-quote-only asymmetry: single-quoted linkage is treated as unquoted
(`mod.rs:170-177`), pinned by `a_single_quoted_typed_ref_is_rejected_as_unquoted`
(`mod.rs:782-789`).

⚠️ Two behaviour changes the plan must design in:

- **Timestamps.** Today `check_timestamps` strips surrounding quotes and checks
  ISO format only, so a bare valid timestamp *passes*. Under the general rule a
  bare scalar that is not int/bool/null is a violation — so a bare ISO timestamp
  newly fails. Intended (ADR-0065 quotes timestamps), but the format check and
  the quoting check must compose without double-reporting.
- **`schema_version`.** The general rule would let both `1` and `"1"` pass the
  *quoting* test (bare int passes, quoted string passes). `schema_version` must
  stay bare, so `check_schema_version` remains a standalone value constraint
  rejecting `"1"` — it is the one field where bare is required, not merely
  permitted.

Field-specific validator tests to update live in
`frontmatter_validation/mod.rs`: `an_unquoted_numeric_id_is_flagged`
(`:573-577`), the quoted/truly-empty id edge cases (`:580-602`),
`schema_version_must_be_the_bare_integer_one` (`:605-609`), the linkage suite
(`:764-830`), and the message-wording test
`bad_linkage_shape_distinguishes_quoted_and_unquoted_wording`
(`violation.rs:205-220`). The whole-repo self-check
`this_repositorys_own_corpus_is_clean` (`cli/corpus-cli/tests/frontmatter_goldens.rs:309-322`)
flips from green to red until the corpus migration lands, then back to green — it
is the natural end-to-end gate for the migration.

### The Python frontmatter-rules surface: retire it (`tasks/`)

⚠️ **Not named in the work item; decision is to retire it.**
`tasks/lint/frontmatter_rules.py` is *not* a lint task — it is a pure
constants/rules **library** (no filesystem access, no CLI entry point, absent
from `tasks/lint/__init__.py`). It defines `ID_QUOTED_RE` (`:111`),
`SCHEMA_VERSION_RE` (`:114`), `TYPED_REF_RE` (`:119`), the base-field/
provenance/linkage constant banks, and `templates-schema.tsv`'s column contract.
It runs only transitively, via the three tests that import it, under
`test:unit:tasks` (`mise.toml:269-272`) and `test:integration:conformance`
(`:325-328`).

Exactly three dependants, all under `tests/`:

- `tests/unit/tasks/test_frontmatter_rules.py` — tests only the module. **Delete
  wholesale.**
- `tests/unit/tasks/test_template_frontmatter.py` — the load-bearing dependant.
  **Delete or re-home** (see coverage-loss below).
- `tests/integration/conformance/test_conformance.py` — dual dependency: it
  drives the *Rust* CLI (survives) but also reads `fr.BASE_FIELDS`/
  `PROVENANCE_FIELDS`/`OPTIONAL_EXTRAS` (`:200,287,289,292,451`). **Amend** —
  remove the `fr` import (`:23`), re-source those three constants (all mirror
  `cli/corpus/src/frontmatter_validation/schema.rs:186,199,228`).

⚠️ **Coverage loss — the retirement's real cost.** `test_template_frontmatter.py`
is the *only* automated validation of `templates/*.md`
(`test_the_real_templates_tree_passes`, `:585`). The Rust validator cannot
substitute, for two independent reasons: `DocTypeKey::Templates` is a virtual
type (`doc_type.rs:155-157`, no `config_path_key`) so the corpus walker never
reaches `templates/`; and even via `--file`, template placeholders
(`date: "YYYY-…"`, `parent: ""`, `blocks: []`) trip `BadTimestamp` and
`EmptyPlaceholder` — the Rust validator checks populated instances, not
skeletons. Seven template-shape checks have no Rust equivalent: placeholder-slot
grammar, the `blocked_by` inverse-guidance line, closed-set linkage keys per TSV
row, per-type extras in the skeleton, status-comment vocabulary, the TSV
field-count self-check, and the work-item Schema-Reference↔TSV cross-check.
**Template validation must be re-homed. Decision (2026-08-30): as a new Rust
template-shape check** — the TSV it needs is already Rust-owned at
`cli/corpus/src/frontmatter_validation/templates-schema.tsv`, keeping one
language and one schema source. This is a distinct plan deliverable, not a side
effect of the delete; the seven checks and the attachment point are grounded in
the follow-up section.

No over-reach risk: the adjacent `tasks/lint/skill_permissions.py` validates a
different surface (SKILL.md tool-permission rules) and must not be touched.

### The producers: five write paths, one renderer (`cli/work-cli/`, `cli/config-adapters/`)

No call site makes a per-scalar quoting decision; all delegate to
`document::render`. Reserving "producer" for these Rust call sites (per the 0221
review's terminology finding), the complete caller set is:

| Call site | Existing? | Preserves body | Notes |
|---|---|---|---|
| `cli/work-cli/src/create.rs:237` | `None` (fresh) | n/a | `work create`; also reached by `sync_author.rs:147` |
| `cli/work-cli/src/sync_author.rs:90` | `Some` | yes | `link-external-id` write-back; byte-identity canary |
| `cli/work-cli/src/update.rs:343` | `Some` | yes | `work update`; also the `--push` path |
| `cli/config-adapters/src/document.rs:27` | `Some` | yes | wrapper converting `config::Node` → `Yaml` |
| `cli/config-adapters/src/store.rs:242` | `Some` | yes | config write; calls the wrapper |

Config writes converge on the identical renderer via a 1:1 `Node`→`Yaml` mapping
(`config-adapters/src/document.rs:56-80`), so the same emitter change governs
config with no field knowledge — as ADR-0065 requires. The `config::Node` tree is
structurally identical to `document::Yaml` (same five scalar variants, ordered
mapping).

⚠️ **`render` is not byte-preserving for untouched fields today** — it drops
comments, normalises frontmatter CRLF→LF, and re-emits block sequences as flow
(pinned by `cli_update.rs:343-393`). AC #4 ("byte-identical serialisation of an
untouched field") is achievable *after* migration because the corpus becomes
canonical (flow arrays, quoted strings, LF), but comment-dropping means a field
carrying an inline comment cannot round-trip byte-identically. The plan should
scope AC #4 to canonical, comment-free frontmatter.

✅ **Not a sixth write path (confirmed).** The visualiser server's one
mutation endpoint — `PATCH /api/docs/{path}/frontmatter`
(`cli/visualiser/server/src/api/docs.rs:144`) — routes through
`corpus_adapters::patch_status` (`file_driver.rs:443`), the byte-level
line-preserving `status:` rewrite (`cli/corpus-adapters/src/patcher.rs:48`). It
reads raw bytes, splices only the one `status:` line, and never calls
`document::render`, `serde_saphyr`, or `Serialize for Yaml`. It mutates status
only (`deny_unknown_fields`, work-items only), preserves the rest byte-for-byte,
and **preserves the existing quote style** of the value token (bare→bare,
double→double, single→single). So the emitter change does not reach it, and after
the migration quotes status corpus-wide the patcher preserves that quoting on
subsequent writes with no further work. The earlier "goes through
`document::render`" signal was wrong; the `api_docs_patch.rs:105` assertion
passes because the fixture seeds bare `status:` and the patcher preserves it.

Producer tests pinning current (pre-standard) output — all flip bare→quoted:
`cli_create.rs:62-73` (id quoted, everything else bare), `:113-116` (linkage +
tag items bare — the exact producer↔validator conflict), `cli_create_push.rs:252`
(`external_id` bare), `cli_update.rs:105-152` (`tags: [a, b, c]` flow),
`cli_update_push.rs:147` (status bare). The byte-identity canary is
`cli/work-cli/tests/cli_link_external_id.rs:50-98`, whose fixture currently
expects `external_id` unquoted — it must flip and the fixture regenerate.

### The migration framework: a mechanical `0008` (`cli/migrate/`)

Migrations are a compile-time registry, not on-disk scripts
(`cli/migrate/src/registry.rs:68-77`). A canonical-quoting migration is
**mechanical** (no user decisions), modelled on `m0006` (frontmatter-rewriting,
idempotent) and `m0007` (whole-corpus enumeration + validate). Registration is a
two-line edit: `pub mod m0008;` in `migrations/mod.rs`, and a
`MigrationEntry::Mechanical(Box::new(Migration0008))` pushed onto the `vec!`.

The validator-in-process hook already exists:
`MigrationContext::validate_frontmatter(&[PathBuf])` (`cli/migrate/src/ports.rs:211-218`,
impl `migrate-adapters/src/context.rs:221-262`) runs the Rust validator with
`structure + references`; `m0007` calls it over rewritten files
(`m0007/mod.rs:189-196`) and whole-corpus (`:332-335`). ⚠️ It covers `meta/`
only — config is rejected `INVALID-TYPE` — so the migration must add an explicit
`.accelerator/config.md` pass (as `m0001` rewrites both corpus and config) and
verify config conformance by byte-identity via the renderer, not the validator.
`m0007/quote.rs:12-66` already holds byte-level double-quote helpers to reuse.

Three test-maintenance tripwires for adding `0008`:

- `cli/migrate/tests/fixtures/public-api.txt` — each `MigrationNNNN` struct is
  public API; add the `Migration0008` entry.
- The per-test "all other migrations applied" isolation lists in every
  `cli/migrate-cli/tests/migration_000N.rs` — add `0008` or the new migration
  runs inside other migrations' fixtures.
- `cli/migrate-cli/tests/full_registry_e2e.rs` — the one place the full chain
  runs together.

The dirty-tree refusal (`cli/migrate/src/preflight.rs`, scopes `meta/`,
`.claude/accelerator`, `.accelerator/`) and `ACCELERATOR_MIGRATE_FORCE=1` bypass
are framework-provided; the new migration inherits them.

### Producer skills and the coverage test (`skills/`, `tasks/`)

All 22 in-scope producer skills' SKILL.md paths are confirmed present, spanning
`skills/work/`, `skills/decisions/`, `skills/planning/`, `skills/research/`
(note `conduct-spike` is under `research/`, not `planning/`), `skills/notes/`,
and `skills/design/`. Two write mechanisms coexist: **CLI-delegated** (e.g.
`create-work-item` → `accelerator work create`, atomic ID-assignment + write) and
**direct Write tool** (e.g. `create-adr`, `create-note`, which hand-populate
frontmatter and write the file). None invoke `corpus frontmatter validate` today.

The static coverage test (AC #6) has a precedent to copy:
`tests/unit/tasks/test_skill_frontmatter_population.py`. Its shape —
`IN_SCOPE_PRODUCERS` closed allowlist (`:69-85`), schema rows from
`tests/unit/tasks/data/skills-schema.tsv`, live-tree `rglob("SKILL.md")`
discovery (`:305-315`), and a discovery-gate that flags any producer surfaced but
not allowlisted (`:318-324`) — is exactly the "every named producer SKILL.md
contains the validate step" test. ⚠️ Its current producer set does **not** equal
the 22 named skills: six are absent (`stress-test-work-item`, `sync-work-items`,
`conduct-spike`, `review-adr`, `stress-test-plan`, `implement-plan`) and it adds
two out-of-scope ones (`describe-pr`, `review-pr`). The new test needs its own
producer list, not a reuse of that allowlist.

The single-file invocation the step will add is supported:
`accelerator corpus frontmatter validate --file <path>`
(`cli/corpus-cli/src/cli.rs:127-137`). `skills/design/inventory-design/SKILL.md:55`
(`accelerator design validate-source`) is the structural template for adding a
validate bash rule to a skill's `allowed-tools` frontmatter.

### Templates and config to regenerate

Broad, mechanical regeneration (general goldens, not field-specific pins):

- `templates/*.md` — 13 template goldens with bare `author`/`status`/`title`/
  `last_updated_by` and flow `tags`. E.g. `templates/work-item.md` has id/date/
  last_updated quoted, author/status/last_updated_by bare, `schema_version: 1`
  bare. These are *also* validated by `test_template_frontmatter.py`, so
  regenerating them and generifying that test move together.
- `.accelerator/config.md` — committed repo config with bare flow strings
  (`kanban_columns: [draft, ready, in-progress, done]`, `integration: linear`);
  rewritten by the migration's config pass.
- In-source "valid document" fixtures reused across ~20+ tests must stay
  *accepted* after the standard tightens, so their bare fields need quoting:
  `frontmatter_goldens.rs:44-58`, `frontmatter.rs:170-173`/`:263-266`,
  `frontmatter_validation/mod.rs:502-514` (`minimal_valid_work_item`),
  `test_conformance.py:172-210` (`_emit_valid`).

## Code References

- `cli/document/src/render.rs:36-43` — `emit()`; the sole `serde_saphyr::to_string` call, no options.
- `cli/document/src/value.rs:157-189` — `Serialize for Yaml`; `Scalar::String` arm at `:173-175` is the fix point; `FlowSeq` at `:177`.
- `cli/document/src/value.rs:22-29` — five-variant `Scalar` (`String`/`Bool`/`Int`/`Float`/`Null`).
- `cli/Cargo.toml:88` — `serde-saphyr = "=0.0.29"`, confined to `cli/document`.
- `cli/work/src/tags.rs:47-62` — second, independent tag-item quoting rule; tests `:152-229`.
- `cli/corpus/src/frontmatter_validation/mod.rs:221-241` — `check_id_quoting` / `check_schema_version`.
- `cli/corpus/src/frontmatter_validation/mod.rs:243-259` — `check_timestamps` (quote-agnostic today).
- `cli/corpus/src/frontmatter_validation/mod.rs:356-387` — `check_linkage_shape` (`quoted:false` branch folds).
- `cli/corpus/src/frontmatter_validation/violation.rs:73-93` — the 17-code `Violation` taxonomy.
- `cli/corpus-cli/tests/frontmatter_goldens.rs:309-322` — `this_repositorys_own_corpus_is_clean` self-check gate.
- `tasks/lint/frontmatter_rules.py:109-119` — the Python validator's `ID_QUOTED_RE`/`SCHEMA_VERSION_RE`/`TYPED_REF_RE`.
- `cli/work-cli/src/create.rs:237`, `sync_author.rs:90`, `update.rs:343` — the three work-cli write paths.
- `cli/config-adapters/src/store.rs:242`, `document.rs:27` — config write through the shared renderer.
- `cli/work-cli/tests/cli_link_external_id.rs:50-98` — byte-identity canary (external_id currently bare).
- `cli/migrate/src/registry.rs:68-77` — the compile-time migration registry.
- `cli/migrate/src/ports.rs:211-218` + `cli/migrate-adapters/src/context.rs:221-262` — the `validate_frontmatter` in-process hook.
- `cli/migrate/src/migrations/m0006.rs`, `m0007/quote.rs:12-66` — mechanical-migration + quote-helper precedents.
- `cli/migrate-cli/tests/full_registry_e2e.rs`, per-test isolation lists — migration test tripwires.
- `tests/unit/tasks/test_skill_frontmatter_population.py:69-85`, `:305-324` — the AC #6 coverage-test template.
- `cli/corpus/tests/fixtures/public-api.txt`, `cli/document/tests/fixtures/public-api.txt` — the two API-snapshot guards.

## Architecture Insights

- **One emitter, one arm, symmetric validator.** The renderer's confinement to
  `cli/document` and the recursion of sequence elements through the `String` arm
  make the emission change a genuinely single-point edit. ADR-0065 states the
  same predicate drives emit and validate ("a bare value passes only if it is an
  integer/boolean/null literal or a flow collection whose elements recurse"), so
  the Rust validator's general rule mirrors the emitter exactly.
- **The type-driven rule is field-agnostic by design.** Because it keys on YAML
  type, not field name, it applies unchanged to config's untyped `Node` tree —
  which is why config conforms via the renderer without the validator ever
  seeing it (config is `INVALID-TYPE` to the corpus validator; its validation is
  work item 0227).
- **`DoubleQuoted<T>` solves churn structurally.** The PR #76 churn was
  `prefer_block_scalars` refolding long titles to `>-` and minimal quoting
  collapsing arrays. Per-value `DoubleQuoted` disables folding for the value, so a
  deterministic style removes the churn source rather than reverting it by hand.
- **Two hidden standards, two languages.** The Rust validator and the Python
  lint independently encode the same field-specific quoting rules. A standard
  that claims to be "canonical" must reconcile both, or it re-creates the
  original defect (two enforcers disagreeing) in a new form.
- **Enforcement is producer-run, not CI-swept.** By decision, each producer skill
  validates the one document it wrote; the accepted proxy for prose-driven skills
  is AC #6 (static presence of the invocation) + AC #7 (deterministic
  non-zero-exit + stderr signal), not a runtime assertion.

## Historical Context

- `meta/decisions/ADR-0065-canonical-frontmatter-quoting-standard.md` — accepted;
  the type-driven rule (quote all strings/timestamps/floats; bare only
  int/bool/null), `schema_version` bare by the rule not as an exception, and the
  explicit override (not supersession) of ADR-0033's identity-value clause and
  ADR-0034's linkage clause. Enforcement wiring is owned by 0221.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — accepted; types
  `author`/`status`/`tags` bare (the clause ADR-0065 generalises). Its
  `schema_version` integer contract is why bare `1` stays.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — accepted; the
  `"plan:0042"` single-quoted-string rule (subsumed by the general rule).
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — the framework
  a migration `0008` plugs into.
- `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`
  — omit-when-empty emission, adjacent to the emitter change.
- `meta/reviews/work/0221-canonical-quoting-standard-for-all-frontmatter-review-1.md`
  — 3-pass review, final APPROVE (2026-08-30); binds the plan to a closed
  producer set, a deterministic validator signal, the config-conforms-but-
  not-validated seam to 0227, byte-identical anti-churn, and a red-state
  regression test (AC #9).
- `meta/work/0220-untracked-remote-discovery-never-runs-on-linear.md` — the
  originating linkage defect (three `BAD-LINKAGE-SHAPE` violations).
- `meta/work/0227-accelerator-config-validate-command.md` — the consumer that
  reuses this standard's predicate over config.
- `meta/prs/76-description.md` — the 37-file minimal-quoting churn that motivated
  a deterministic style.
- `meta/notes/2026-03-24-yaml-block-sequence-array-parsing.md`,
  `meta/notes/2026-07-17-render-line-ending-normalisation.md` — renderer/parse
  behaviour directly relevant to the array-collapse and CRLF→LF nuances.

## Related Research

- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md` —
  the `document`/`corpus` crates (shared renderer + parsing conventions).
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md` —
  the serde-saphyr config reader and order-preserving `Node` round-trip.
- `meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`
  — the corpus CLI / frontmatter validator surface.
- `meta/research/codebase/2026-08-06-0172-migration-engine-implementation-research.md`
  — the migration engine groundwork.
- `meta/research/codebase/2026-06-15-0105-corpus-validator-provenance-linkage-blind-spots.md`
  — prior validator work on linkage.

## Open Questions

Two of the original six are resolved (see Follow-up Research): the Python-lint
scope (**retire**, re-homing template validation) and the visualiser sixth-path
question (**not a renderer producer**). The rest remain.

- ❓ **`Scalar::Float` treatment.** ADR-0065 quotes floats, but the value tree's
  `Float` variant emits bare and quoting it changes round-trip type to string.
  Confirm no float-typed frontmatter or config value exists, then decide the
  emitter/validator behaviour for `Float` symmetrically.
- ❓ **`schema_version` under the general rule.** Confirm `check_schema_version`
  stays a standalone "must be bare `1`" value constraint, since the general
  quoting rule alone would permit both `1` and `"1"`.
- ❓ **AC #4 byte-identity scope.** `render` drops inline comments and normalises
  CRLF→LF even for untouched fields. Scope AC #4 to canonical, comment-free
  frontmatter, or the criterion is unmeetable for a field carrying a comment.
- ❓ **Producer-skill slice split.** The review flags producer-skill wiring (~22
  SKILL.md edits + coverage test) as a separately-shippable child story if it
  threatens to stall the core emitter/validator/migration fix. Decide whether the
  plan keeps it in one story or pre-splits it.
Resolved after the follow-up: template validation re-homes as a **new Rust
template-shape check** (see the second follow-up section for the grounding).

## Follow-up Research 2026-08-30T14:26:56+00:00

Two directed follow-ups after the initial pass: a decision to retire the Python
frontmatter-rules surface, and a definitive investigation of the suspected sixth
write path.

### Retire the Python frontmatter-rules surface — resolved

Decision taken: retire `tasks/lint/frontmatter_rules.py`. The investigation found
it is a pure constants/rules **library**, not a registered lint task — imported
by exactly three test files and run transitively under `test:unit:tasks` and
`test:integration:conformance`. The clean-retirement map: delete
`frontmatter_rules.py` and `test_frontmatter_rules.py`; amend
`test_conformance.py` to drop the `fr` import and re-source three constants from
the Rust schema; and **re-home** the template-shape validation currently in
`test_template_frontmatter.py`, which is the only validator of `templates/*.md`
and has no Rust equivalent. The full map and the seven uniquely-lost checks are
in "The Python frontmatter-rules surface: retire it" above. The plan gains a
distinct deliverable: a template-shape validator to replace what the delete
removes.

### The visualiser is not a sixth renderer producer — corrected

The suspected sixth write path does not exist as a renderer producer. The
visualiser server's sole mutation endpoint writes status via the byte-level
`patch_status` (`cli/corpus-adapters/src/patcher.rs`), never `document::render`,
and preserves the value's existing quote style. The emitter change does not reach
it; the migration's status-quoting is preserved by the patcher on later writes
with no further work. My initial framing endorsed the hypothesis as likely; the
code shows it is a byte-level patch, unaffected. Detail in the "Not a sixth write
path (confirmed)" note under "The producers" above. Net effect: the write-path
inventory stays at five renderer producers (three work-cli call sites + two
config), with the byte-level status patcher a separate, unaffected mutation
surface.

### Rust template-shape check — grounding

Decision: re-home template validation as a new Rust template-shape check. It is a
**distinct concern** from `validate_file`, not an extension of it — the corpus
validator would reject every template (`check_empty_placeholders` on `parent: ""`
/`blocks: []`, `check_timestamps` on `date: "YYYY-…"`), whereas the template
check *requires* that placeholder grammar. It must be a separate code path over
the shared `SchemaRow` data.

- **What it validates.** The 13 `templates/*.md` (one per SCHEMA/TSV row; note
  `rca.md`→`issue-research`, `validation.md`→`plan-validation`). Shared structural
  conventions — quoted `id:`, bare `schema_version: 1`, a `status:` line whose
  trailing comment reproduces the TSV `status_vocab` verbatim, each declared
  linkage slot as `key: "" # typed-linkage ref: …` / `key: [] # typed-linkage
  list: …`, the `# inverse of blocks …` line after `blocked_by:`, and the
  `revision`/`repository` bundle present iff `code_state_anchored`. Placeholder
  *tokens* diverge (`"NNNN"`, `"ADR-NNNN"`, `"{filename-stem}"`) and the check
  tolerates them — it never checks `id`/`date` value content, only quoting.
- **The TSV seam is Rust-owned already.** `schema.rs:274` embeds
  `templates-schema.tsv` via `include_str!`; `SchemaRow` (`schema.rs:4-12`) and the
  cross-cutting constants (`BASE_FIELDS`, `PROVENANCE_FIELDS`,
  `LINKAGE_SOURCE_TYPES`, `OPTIONAL_EXTRAS`, `OBSOLETE_LEGACY_KEYS`,
  `schema.rs:186-239`) are the data the Python module duplicated. No Rust code
  reads the template `.md` files for validation today.
- **Where it attaches.** Pure per-row shape logic in a new
  `cli/corpus/src/frontmatter_validation/template_shape.rs` (row + frontmatter
  string → violations, mirroring `mod.rs`/`shape.rs` purity); the filesystem
  driver (read each template, TSV field-count self-check, work-item
  Schema-Reference↔TSV cross-check) in `cli/corpus-adapters/src/frontmatter_validation.rs`
  via the `RealFs` `FileReader` port. Surface it as a **new
  `FrontmatterAction::ValidateTemplates` variant** (`cli/corpus-cli/src/cli.rs`),
  not a new `--checks` value — the `Validate` pipeline is artifact-oriented and
  structurally rejects templates.
- **Templates are located at a fixed path.** `<project_root>/templates/<name>.md`
  (use the `composed.project_root` already computed in `run_frontmatter`,
  `main.rs:117`); *not* the configurable `paths.templates` override, which is the
  unrelated config-ejection feature.
- **The real-tree test needs no new mise task.** Copy
  `this_repositorys_own_corpus_is_clean` (`cli/corpus-cli/tests/frontmatter_goldens.rs:308-322`)
  into a `the_real_templates_tree_is_clean` sibling that shells out to
  `frontmatter validate-templates` rooted at `CARGO_MANIFEST_DIR/../..` — it runs
  under the existing `test:unit:cli` lane. Add pure `corpus`-crate unit tests for
  the per-row logic on the same lane.
- **The seven checks to port** (input → output): (1) placeholder-slot grammar per
  declared linkage key; (2) the `blocked_by` inverse-guidance line; (3)
  closed-set linkage keys per TSV row (rejecting any linkage key not a declared
  slot or extra, e.g. `superseded_by`); (4) per-type extras present in the
  skeleton; (5) status-comment vocabulary reproduced from `status_vocab`; (6) the
  TSV 7-field-count self-check (a runtime check over the on-disk file, distinct
  from the existing `#[cfg(test)]` mirror); (7) the work-item Schema-Reference
  table ↔ TSV first-column cross-check.
- ⚠️ **Deletion gate.** `frontmatter_rules.py` has two live consumers —
  `test_template_frontmatter.py` *and* `test_conformance.py:23`. It can be deleted
  only once both have moved: template validation to the Rust check, and
  `test_conformance.py`'s three constants re-sourced from the Rust schema. The
  lost CI coverage is one pytest module on `test:unit:tasks` (`mise.toml:269-272`).
