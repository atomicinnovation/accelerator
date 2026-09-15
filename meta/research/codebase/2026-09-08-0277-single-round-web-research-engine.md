---
type: "codebase-research"
id: "2026-09-08-0277-single-round-web-research-engine"
title: "Research: Single-Round Web Research Engine (0277) implementation seams"
date: "2026-09-08T22:01:02+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0277"
parent: "work-item:0277"
relates_to: ["work-item:0121"]
topic: "Single-Round Web Research Engine implementation seams"
tags: ["research", "codebase", "research-topic", "skills", "corpus", "config", "templates"]
revision: "2350de33d5625096f33e1cd498fe0eb779bd8aff"
repository: "accelerator"
last_updated: "2026-09-09T07:51:08+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Resolved open questions Q1 (validator depth: (type, kind) lookup, rename research_kind -> kind), Q2 (DOC_TYPES ships with 0278; only paths.research_topics in 0277), Q3 (slug resolution via a general 'corpus resolve --type <type> <slug>' subcommand; set handle renamed slug), Q4 (topic-research registered as a typed-linkage target in 0277), and Q5 (source-profile packaging: path-passing per spawn, profiles under skills/research/profiles/); updated findings, resolver section, changes-required table, and open questions to match"
schema_version: 1
---

# Research: Single-Round Web Research Engine (0277) implementation seams

**Date**: 2026-09-08T22:01:02+00:00
**Author**: Toby Clemson
**Git Commit**: 2350de33d5625096f33e1cd498fe0eb779bd8aff
**Branch**: working copy `xvnxupuuvlkm` (jj-colocated, unpushed)
**Repository**: accelerator

## Research Question

For story 0277 "Single-Round Web Research Engine", locate and map the existing
codebase seams the engine must reuse or mirror — the `configure` subcommand
dispatch, the generic `reviewer` agent, `work resolve` handle resolution, the
`paths.*` config catalogue, the 3-tier template override, and
`corpus frontmatter validate` — so an implementation plan can copy each pattern
precisely and the 0277/0278 boundary is unambiguous.

## Summary

0277 is a **new skill built almost entirely by copying four established
patterns**; the genuinely new code is a general slug resolver, one config key, one
corpus schema row, five templates, one agent, and the skill body. Every seam has
a concrete precedent in the tree.

Five findings shape the plan:

- **Dispatch is model-side prose, no router.** `research-topic` is one
  `SKILL.md` with a per-verb branch, modelled on `configure` (heading-encoded
  routing) or `comment-jira-issue` (a shared parse step then per-verb branches).
  The four-verb, positional-handle shape matches `comment-jira-issue` most
  closely.
- ⚠️ **Slug resolution — a general `corpus resolve` subcommand.** Both dispatch
  exemplars pass positionals straight to a CLI that resolves them, so there is no
  in-skill precedent. The epic's "mirror `work resolve`" steer becomes a
  **general `accelerator corpus resolve --type <type> <slug>`
  subcommand** (mirroring `cli/work/src/resolve.rs`), not resolution prose in the
  SKILL.md body.
- ✅ **Decision — the corpus validator gains a `(type, kind)` lookup.** Today
  `row_for` keys on `type:` alone, so one `topic-research` row could not enforce
  the six shapes' differing required-extras and per-shape statuses, and one type
  admits only one schema row. Resolved (2026-09-08) by extending the schema to a
  `(type, kind)` composite with a type-level fallback and renaming the
  discriminator `research_kind` → `kind` (mirroring work items). Each `kind`
  carries its own required fields and status vocabulary; kind-agnostic types
  resolve unchanged through `(type, "")`. The mechanism is general — it later
  admits per-work-item-kind templates. ⚠️ ADR candidate.
- ✅ **`--file` validation needs only a `SCHEMA` row — no `DocTypeKey` variant.**
  This cleanly splits 0277 (write + `corpus frontmatter validate --file`) from
  0278 (whole-corpus discovery + indexing + `DocTypeKey` + `server/src/docs.rs`).
- **The generic-agent pattern is path-passing (ADR-0005), not content
  injection.** The `researcher` mirrors `agents/reviewer.md`: a compact task
  prompt with `(source_profile, question)` framing; the agent reads its profile
  in its own isolated context. Tool grant (`WebFetch`) copies
  `agents/web-search-researcher.md`.

## Detailed Findings

### Subcommand dispatch — the skill skeleton

**Model-side dispatch, single `SKILL.md`, no router script.** The `configure`
skill (`skills/config/configure/SKILL.md`) encodes routing in each verb's H3
heading — `### `view` (or no argument with existing config)`,
`### `create` (…)`, `### `help``. `comment-jira-issue`
(`skills/integrations/jira/comment-jira-issue/SKILL.md`) is the closest four-verb
analog: a `## Step 1: Parse the subcommand and flags` step names the first
positional as the verb and gives each verb a bolded sub-block of its positionals
and flags, then later steps branch `For `add`/`edit`/`delete`…` vs `For
`list`…`.

Both set `disable-model-invocation: true` and scope `allowed-tools` narrowly
(`configure`: `Bash(accelerator config *)`, `Read`, `Write`, `Edit`).

| Skill | Idiom | Fits 0277 when |
|---|---|---|
| `configure` | One `### <verb>` section, routing in the heading | Each verb has a distinct body |
| `comment-jira-issue` | Shared `## Step N` pipeline, per-verb branches inside | Verbs share a preview/confirm/execute flow |

The `research-topic` verbs (`brief`, `outline`, `conduct`, `synthesise`) have
distinct bodies but a shared handle-resolution + manifest-update preamble, so a
hybrid is natural: a shared parse/resolve step, then a `### <verb>` section each.
`argument-hint` follows the `comment-jira-issue` expanded form, e.g.
`"brief SUBJECT | outline HANDLE | conduct HANDLE | synthesise HANDLE"`.

### Live-context injection — the `!` preprocessor recipe

**The injection sequence is uniform across skills.** `configure` itself carries
**no** `!` lines (it reaches config only through `Bash` calls in verb sections),
so the model to copy is `research-codebase` / `create-work-item` /
`comment-jira-issue`. The recipe, run once per invocation because the whole
`SKILL.md` expands to a single prompt:

```markdown
!`accelerator config context --skill research-topic --fail-safe`
!`accelerator config agents --fail-safe`

**Research topics directory**: !`accelerator config path research_topics --fail-safe`

!`accelerator config template <name> --fail-safe`
...
!`accelerator config instructions research-topic --fail-safe`
```

Every command takes `--fail-safe` so a failure degrades rather than aborting the
skill. `config context`/`config agents` sit near the top; `config path` resolves
directories; `config template` injects a document template; `config instructions`
is the final line. `${CLAUDE_PLUGIN_ROOT}` is used only when a skill wants a
subagent to *Read* another skill file rather than splice it (the review
orchestrators do this for lens + output-format paths).

Per-skill `instructions.md` / `context.md` are **user-provided**, live at
`.accelerator/skills/<skill-name>/`, and are not shipped in the plugin. The
directory name must equal the skill's `name`. No plugin work is needed to
"support" them beyond calling `config context`/`config instructions` with the
skill name.

### Generic `researcher` agent — mirror `reviewer`, grant WebFetch

**ADR-0005 fixes the pattern: path-passing, not content injection.** The generic
`agents/reviewer.md` (`tools: Read, Grep, Glob, LS`; body: "You are a specialist
reviewer. Your task instructions provide a review lens…") is spawned by
orchestrators that pass a compact ~30–37-line task prompt with *paths* to a lens
skill and an output-format spec; the agent reads those in its own isolated
context. Three concerns stay separate: lens/profile skill (what to evaluate),
output-format reference (how to format), orchestrator task prompt (what context).

For the `researcher` the two axes are `(source_profile, question)`:

| Reviewer concept | Researcher analog | Location |
|---|---|---|
| Generic agent def | `agents/researcher.md` (new) | mirror `agents/reviewer.md` |
| Lens skill (what) | Source-profile skill (web) | new, under `skills/research/…` |
| Output-format spec | Finding output contract | new |
| Orchestrator | `conduct` verb of `research-topic` | new skill body |
| `subagent_type` resolution | `accelerator config agent researcher` | mirror `config agent reviewer` |

⚠️ **Tool grant.** `agents/reviewer.md` has no web tools. The only agent granting
`WebFetch`/`WebSearch` is `agents/web-search-researcher.md`
(`tools: WebSearch, WebFetch, TodoWrite, Read, Grep, Glob, LS`). The new
`researcher` needs `WebFetch` (the epic specifies "WebFetch of arbitrary URLs"),
so copy that agent's tool line, not the reviewer's.

Resolved (2026-09-09): the source profile is injected **per spawn by
path-passing** (the ADR-0005 reviewer pattern), *not* the static `skills:`
preload — the profile varies per spawn (academic + mixed rounds in 0280), which a
fixed preload cannot express. `conduct` passes the profile skill's path plus the
finding output-format path; the researcher Reads both in its own context. The
subagent **skill-preload** mechanism (`skills:` frontmatter key, min Claude Code
v2.1.144; `agents/documents-locator.md` preloads `accelerator:paths`) stays
available only for a *fixed* helper skill a researcher always needs, not the
varying profile — and such a preloaded skill must not set
`disable-model-invocation: true` (`skills/config/paths/SKILL.md` documents this).

Only `conduct` spawns researchers; `synthesise` runs inline over the immutable
findings and spawns nothing (epic 0121 requirement 1).

### Slug resolution — a general `corpus resolve` subcommand mirroring `work resolve`

✅ **Decision: a general `accelerator corpus resolve --type <type> <slug>`
subcommand on the `corpus` binary.** `comment-jira-issue` and `configure` both
hand positionals opaquely to the CLI, so there is no in-skill precedent; the epic
steers to "mirror `work resolve`". The resolver is general across doc types
(`--type` selects the type), resolving a slug to a file for flat types or the set
directory for nested-manifest types, and the skill calls it with
`--type topic-research`. ⚠️ Type-driven `topic-research` resolution needs the
doc-type registration (0278), so 0277 ships the generic resolver (tested against
an already-registered type) and topic-research resolution is verified at co-land.

The mirror target:

| Layer | File | Key items |
|---|---|---|
| Domain classifier | `cli/work/src/resolve.rs` | `classify_input` → `InputClass::{Path, FullId, BareNumber, Invalid}` (l.15); `resolve` (l.347); `resolve_bare_number` (l.258); `TaggedCandidate` (l.29); `ResolveOutcome` (l.35) |
| Adapter/binary | `cli/work-cli/src/resolve.rs` | `run` (l.54); `resolve_with` (l.92, infallible core); `canonical_work_dir` (l.73); `resolve_path_class` (l.33, canonicalises and enforces the resolved path stays under the root) |
| Config glue | `cli/work-cli/src/config.rs` | `resolve_scheme` (l.24); `resolve_work_dir` (l.55) |
| CLI registration | `cli/work-cli/src/main.rs` | `Command::Resolve { input }` (l.462) → `run_resolve` (l.41) |
| Golden surface | `cli/work-cli/tests/cli_resolve.rs`, `tests/fixtures/cli_surface.golden` | |

The three input forms the resolver accepts for `--type topic-research`, all
normalising to the set root `meta/research/topics/<slug>/`: a bare slug (resolved
against `paths.research_topics`), the set directory, and any document within the
set. This maps onto `InputClass::{Path, FullId, BareNumber}` with the twist that
the "id" is a slug and a sub-document path must walk *up* to the set root (a
finding at `<slug>/findings/01-x.md` resolves to `<slug>/`).
`resolve_path_class`'s "stays under the root" enforcement is the safety check to
copy. ⚠️ For flat dated types (`YYYY-MM-DD-…-<slug>.md`) the slug alone is
ambiguous — resolve with candidate handling as `work resolve` does.

### `paths.research_topics` config key

**One line in the catalogue, defaulting to `meta/research/topics`.** Config keys
and defaults are declared in `cli/config/src/catalogue.rs`:

- `PATH_KEYS: &[(&str, Default)]` (l.31–64) — the ordered `paths.*` list.
  Existing siblings: `("paths.research_codebase", …)`,
  `("paths.research_issues", Default::Scalar("meta/research/issues"))` (l.60–63).
  Add `("paths.research_topics", Default::Scalar("meta/research/topics"))`.
- `DOC_TYPES` (l.66) maps doc-type name → path-key suffix (e.g.
  `("issue-research", "research_issues")`). A `topic-research` → `research_topics`
  entry belongs here — ⚠️ but see the boundary note: this couples to indexing.
- `default_for(key)` (l.230) resolves any key to its default; `TEMPLATE_KEYS`
  (l.82) is the template-name registry.

Resolution flows through `cli/config/src/paths.rs::resolve_with_fallback(config,
path_key, level)` (l.88), which prefixes `paths.` and falls back to the catalogue
default, with `is_unsafe` (l.118) and `normalise` (l.131) guards. The
`accelerator config path <key>` surface the skill calls lives at
`cli/launcher/src/config_command/core/paths.rs::resolve` (l.62).

⚠️ `.accelerator/config.md` has **no `paths:` block** — it overrides only
`visualiser.kanban_columns` and `work.integration`. So `paths.research_topics`
resolves purely from the catalogue default unless a project overrides it; no
config-file edit is required for the default to work.

### Template resolution — 3-tier override, five new templates

**The tier walk already exists; 0277 adds plugin-default files.** The precedence
is implemented in `cli/config-adapters/src/store.rs` `impl ReadTemplate for
FileConfigStore` (from l.367):

```text
tier 1  config path            (templates.<name> config value)
tier 2  user override          absolutise(templates_dir).join("<name>.md")   → UserOverride
tier 3  plugin default         plugin_root.join("templates").join("<name>.md") → PluginDefault
```

Orchestrated by `cli/launcher/src/config_command/core/template.rs::resolve`
(l.22), which reads `templates.<name>`, gets `templates_dir` (from
`paths.templates`, default `.accelerator/templates`), then calls
`resolve_template`. `TemplateSource` and its labels ("config path" / "user
override" / "plugin default") live in `cli/config/src/service.rs`.

The 13 current plugin-default templates sit in `templates/` (`work-item.md`,
`note.md`, `codebase-research.md`, `plan.md`, `adr.md`, `rca.md`,
`design-inventory.md`, …). 0277 adds the web-only shapes — one each for
`manifest`, `brief`, `outline`, `finding`, `synthesis` (five files). Each must
carry the base frontmatter contract (`type`, `id`, `title`, `date`, `author`,
`producer`, `status`, `tags`, `revision`?, `repository`?, `last_updated`,
`last_updated_by`, `schema_version: 1`) plus the shape's extra keys, with
`{placeholder}` tokens and `[bracketed prose]` bodies. ⚠️ `validate_templates`
(see below) reads `templates/<row.template>` and emits `MissingTemplateFile` if a
schema row names a template file that is absent, so the template files and the
schema row must land together.

**Naming (resolved).** Templates are addressed by `(type, kind)`: files
`templates/topic-research-<kind>.md` (`topic-research-manifest`, `-brief`,
`-outline`, `-finding`, `-synthesis`), resolved as `topic-research-<kind>` with a
fallback to a general `topic-research` template. Each stem is also its
`templates.<stem>` override key (`TEMPLATE_KEYS`, catalogue.rs:82) and its
`accelerator config template <stem>` injection argument — the `rca` ↔ `rca.md`
precedent proves stems are free-form. ⚠️ Each new TSV row's `template` filename
must also appear in the `## Schema Reference` tables of work items `0065`, `0066`,
and `0067`, or `cross_check` (template_shape.rs:593) fails.

### `corpus frontmatter validate` — the flat-schema constraint

**The validator is a string-keyed table, one `SchemaRow` per `type`.**
`cli/corpus/src/frontmatter_validation/schema.rs`:

```rust
pub struct SchemaRow {
    pub linkage_type: &'static str,        // the ONLY lookup key
    pub code_state_anchored: bool,         // true ⇒ require revision+repository
    pub extras: &'static [&'static str],   // flat required-extras list
    pub status_vocab: &'static [&'static str],
    pub forbidden_own_id_keys: &'static [&'static str],
    pub typed_linkage_keys: &'static [&'static str],
}
pub const SCHEMA: [SchemaRow; 13] = [ /* … */ ];
```

`row_for(linkage_type)` (l.177) matches the first row whose `linkage_type`
equals the document's declared `type:`. `validate_file` (mod.rs l.188) reads
`declared_type`, looks up the row, and short-circuits with a single
`Violation::InvalidType` when there is **no** matching row — an unrecognised type
*fails*, it does not skip (tests `an_unrecognised_type_short_circuits_every_
other_check`, `an_absent_type_is_invalid_type_and_nothing_else`).

The existing research rows are minimal:

```rust
SchemaRow { linkage_type: "codebase-research", code_state_anchored: true,
    extras: &["topic"], status_vocab: &["complete"],
    forbidden_own_id_keys: &[], typed_linkage_keys: &["parent", "relates_to"] }
SchemaRow { linkage_type: "issue-research", code_state_anchored: true,
    extras: &["topic"], status_vocab: &["complete"],
    forbidden_own_id_keys: &[], typed_linkage_keys: &["parent", "relates_to"] }
```

`check_required_extras` (mod.rs l.354) iterates `row.extras` only — no
discriminator logic, and it does **not** reject unknown extras (a shape may carry
extra keys freely). `check_status` (mod.rs l.290) checks `status` against the
single flat `status_vocab`. Nothing anywhere reads `kind` as a discriminator
today; its value is
subject only to canonical-quoting and empty-placeholder checks, never an
allow-list.

**Why one flat row is not enough, and the resolution.** A single `topic-research`
row can enforce only the *common* extras and the *union* status vocab — it would
accept `status: draft` on a finding, which must always be `complete`, because it
cannot tell a finding from a brief. Nor can the five per-shape templates each get
their own row: `row_for` is first-match and `every_row_matches_templates_schema_tsv`
(schema.rs:277) validates every TSV line against `row_for(type)`, so two rows of
the same type with different fields fail the test. Distinct types per shape are
ruled out — the epic mandates a single umbrella `topic-research` type for the
visualiser. So the flat row would leave four of five templates and all per-shape
instance rules unguarded.

✅ **Resolved (2026-09-08) — extend the validator to a `(type, kind)` lookup:**

- Rename the discriminator `research_kind` → `kind` (mirrors the work-item `kind`).
- `SchemaRow` and the TSV gain a `kind` column; `row_for(type, kind)` selects the
  kind-specific row, falling back to the type-default `(type, "")` row when none
  exists.
- `validate_file` reads the document's `kind` and resolves the composite key; the
  existing 13 types become `(type, "")` rows and resolve unchanged.
- `every_row_matches_templates_schema_tsv`, the TSV field-count self-check
  (`SCHEMA_TAB_FIELDS`, template_shape.rs:98), and the `cargo-public-api` snapshot
  move to the composite key.
- General mechanism: it later admits per-work-item-kind rows (`work-item` +
  `story`/`epic`/…). ⚠️ ADR candidate.

### The 0277 / 0278 boundary — what each schema change gates

✅ **`corpus frontmatter validate --file <path>` runs purely off `SCHEMA` /
`row_for`** — no `DocTypeKey` variant on the call path
(`corpus-cli/src/frontmatter.rs::run_validate` → `validate_targets` →
`validate_path` → `validate_file` → `row_for`). So the engine (0277) can write
each set document and validate it by explicit `--file` with only a `SCHEMA` row
present. This is exactly the AC's per-document validation loop.

⚠️ **Discovery goes through `DocTypeKey`.** Whole-corpus mode and `--dir` mode
filter files by `DocTypeKey` config paths / path inference
(`cli/corpus/src/doc_type.rs`, `linkage.rs::type_from_path`). Indexing for the
visualiser also keys on `DocTypeKey` + `config_path_key` + `server/src/docs.rs`.
Those belong to 0278 (the epic explicitly moved "server-side `config_path_key`
wiring" to 0278).

Changes required, by owner:

| Change | File(s) | Owner |
|---|---|---|
| `SchemaRow` + `row_for` gain a `kind` dimension; `(type, kind)` lookup + `(type, "")` fallback | `schema.rs` (`SchemaRow` l.4, `row_for` l.177) | 0277 |
| Five `(topic-research, <kind>)` rows; array `13`→`18`; length test renamed | `schema.rs` (l.16, l.262) | 0277 |
| TSV gains a `kind` column; five matching rows; field-count self-check `7`→`8` | `templates-schema.tsv`, `template_shape.rs` (`SCHEMA_TAB_FIELDS` l.98) | 0277 |
| Existing 13 types become `(type, "")` rows — kind-agnostic, resolve unchanged | `schema.rs`, TSV | 0277 |
| Five templates `topic-research-<kind>.md` + `templates.<stem>` keys | `templates/*.md`, `TEMPLATE_KEYS` (catalogue.rs l.82); `validate_templates` | 0277 |
| Template resolver: `<type>-<kind>` → `<type>` fallback | `config_command/core/template.rs`, `store.rs` `ReadTemplate` | 0277 |
| Schema Reference cross-check: list new template filenames | work items `0065`/`0066`/`0067` | 0277 |
| public-API snapshot (`SCHEMA` len, `SchemaRow` shape) | `cli/corpus/tests/fixtures/public-api.txt` | 0277 |
| `paths.research_topics` PATH_KEY — forces 3 test-fixture edits: catalogue count `55`→`56`, `paths.golden`, `dump.golden` | `catalogue.rs` (l.31–64, l.264); `config_read.rs` goldens | 0277 |
| `corpus resolve --type <type> <slug>` general subcommand (mirrors `work-cli/resolve.rs`); topic-research `--type` support co-lands with 0278 | `cli/corpus-cli` (new `resolve.rs`), `config::paths::resolve_with_fallback` | 0277 |
| `LINKAGE_SOURCE_TYPES` + `SOURCE_TYPES` — register `topic-research` as a linkage target (shape check in 0277; dangling-check co-lands with 0278) | `schema.rs` (l.206), `template_shape.rs` (l.51), public-api snapshot | 0277 |
| `DocTypeKey` variant + `all()`/`config_path_key`/`label`/`wire_str` + tests | `cli/corpus/src/doc_type.rs`, public-api snapshot | 0278 |
| `catalogue.rs` `DOC_TYPES` entry — coupled to the `DocTypeKey` variant (`doc_type_single_source.rs` hard-fails without it) | `catalogue.rs` (l.66) | 0278 |
| `server/src/docs.rs`, `frontend/src/api/types.ts`, indexer wiring | server + frontend | 0278 |

✅ **`DOC_TYPES` placement — RESOLVED: it belongs to 0278.** The three catalogue
surfaces (`PATH_KEYS`, `DOC_TYPES`, `TEMPLATE_KEYS`) are independent, and
`DOC_TYPES` is cross-checked against `DocTypeKey` (`doc_type_single_source.rs`): a
`topic-research` `DOC_TYPES` entry with no `DocTypeKey::TopicResearch` variant
hard-fails (`from_linkage_type_name` returns `None`), dragging in the 0278 enum
work and its ripple (`parity.rs`, `api_types.rs`, `api_smoke.rs`). The 0277 engine
does not need it: `--file` validation is never scope-filtered and checks the
file's `type:` against `SCHEMA`, independent of `DOC_TYPES`. So only
`paths.research_topics` lands in 0277, forcing exactly three test-fixture edits —
the catalogue count test `55`→`56`, `paths.golden`, and `dump.golden` — while
every doc-type/`DocTypeKey`/visualiser test stays green. `format`/`lint`/`types`
are unaffected (pure data edits on already-public slices); the breakage is
test-only. **0277's `mise run check` passes in isolation.**

## Code References

- `skills/config/configure/SKILL.md` — model-side verb dispatch; per-`### verb` sections; `templates` sub-dispatch
- `skills/integrations/jira/comment-jira-issue/SKILL.md` — four-verb parse step + per-verb branches; `disable-model-invocation`; `!` context/instructions lines
- `skills/research/research-codebase/SKILL.md:15-26` — `config context`/`config agents`/`config path` injection recipe
- `skills/work/create-work-item/SKILL.md:15-41,454-491` — richest write skill; template injection; CLI-mediated atomic write; `corpus frontmatter validate`
- `skills/design/inventory-design/SKILL.md:245-367` — temp-dir-and-rename atomic set write (`.tmp/` → final, lister skips dot-dirs)
- `agents/reviewer.md` — generic agent specialised at spawn time (`tools: Read, Grep, Glob, LS`)
- `agents/web-search-researcher.md:4` — the only `WebFetch`/`WebSearch` grant to copy
- `agents/documents-locator.md:10-11` — subagent `skills:` preload of `accelerator:paths`
- `skills/planning/review-plan/SKILL.md:235-274` — orchestrator spawn: inject lens path + output-format path; `subagent_type: accelerator config agent reviewer`
- `cli/work/src/resolve.rs:15,258,347` — `classify_input`, `resolve_bare_number`, `resolve`
- `cli/work-cli/src/resolve.rs:33,92` — `resolve_path_class` (under-root enforcement), `resolve_with`
- `cli/config/src/catalogue.rs:31-64,66,82,230` — `PATH_KEYS`, `DOC_TYPES`, `TEMPLATE_KEYS`, `default_for`
- `cli/config/src/paths.rs:88,118,131` — `resolve_with_fallback`, `is_unsafe`, `normalise`
- `cli/config-adapters/src/store.rs:367-432` — 3-tier `ReadTemplate`; `plugin_root_from_env` (l.203)
- `cli/launcher/src/config_command/core/template.rs:22` — `resolve` (3-tier orchestration)
- `cli/corpus/src/frontmatter_validation/schema.rs:4-12,16,84-99,177,206,262` — `SchemaRow`, `SCHEMA`, research rows, `row_for`, `LINKAGE_SOURCE_TYPES`, length test
- `cli/corpus/src/frontmatter_validation/mod.rs:188,290,354` — `validate_file`, `check_status`, `check_required_extras`
- `cli/corpus/src/frontmatter_validation/templates-schema.tsv:7-8` — research-row TSV lines (lock-stepped to `SCHEMA`)
- `cli/corpus/src/doc_type.rs:9-24,28,48-116` — `DocTypeKey` enum, `all()`, `config_path_key`, `linkage_type_name`, `label`, `wire_str`
- `cli/corpus-cli/src/frontmatter.rs:32-43,76` — target-file selection (`--file` un-filtered vs `--dir`/whole-corpus `DocTypeKey`-gated), `run_validate`
- `templates/` — 13 plugin-default templates; five new topic-research shapes land here

## Architecture Insights

- **Filesystem-as-bus, path-passing agents.** Phases communicate through `meta/`,
  not conversation; subagents get compact prompts and read detail in isolated
  context (ADR-0005). The `researcher` inherits both: `conduct` writes findings to
  disk and spawns `(source_profile, question)` researchers that read their profile
  themselves.
- **Registries are string-keyed and length-pinned.** `SCHEMA` (`[…; 13]`),
  `DocTypeKey::all()` (`[…; 14]`), `LINKAGE_SOURCE_TYPES`, and a
  `cargo-public-api` snapshot all hard-code array lengths and are cross-checked by
  tests and a TSV mirror. Any type/row addition is a multi-file, length-bumping
  change with a snapshot regen — mechanical but easy to under-scope.
- **The `type`/`kind` split is deliberate and lossy at the validator.**
  Collapsing six shapes into one visualiser doc type (one glyph, one library
  entry) is an epic-level choice; the price is that `corpus frontmatter validate`
  sees one type and cannot police per-shape frontmatter. Templates carry that
  weight instead.
- **Config defaults live in Rust, not the config file.** The catalogue is the
  source of truth; `.accelerator/config.md` holds only overrides. A new key works
  from its catalogue default with zero config-file change.
- **Two dispatch idioms, no framework.** Multi-verb skills are prose contracts the
  model executes; there is no shared router. This keeps `research-topic` a
  documentation task for the verb bodies and a Rust task only for resolution.

## Historical Context

- `meta/decisions/ADR-0005-single-generic-reviewer-agent-with-runtime-lens-injection.md`
  — establishes path-passing over content injection for the generic agent; the
  direct precedent the `researcher` mirrors. Its three-file separation (lens
  skill / output-format / orchestrator prompt) maps onto (source-profile skill /
  finding contract / `conduct` prompt).
- `meta/work/0121-topic-research-skillset.md` — parent epic. Carries the binding
  six-document artifact contract (`§ Artifact contract`), the `research_status`
  five-state lifecycle, the depth/breadth semantics, and the Technical Notes that
  name every precedent (`configure`, `reviewer`, `work resolve`, nested-manifest
  indexer). The single authoritative design source — no separate research or plan
  exists yet for this epic.
- `meta/reviews/work/0277-single-round-web-research-engine-review-1.md` — the one
  completed review of 0277 (verdict APPROVE), currently staged.
- Sibling children (all present, 0278–0284): `0278` visualiser doc-type +
  indexer (co-land), `0279` iterative accretion + finalise, `0280` academic
  sources, `0281` corpus consumption (ask/report), `0282` tunable depth/breadth,
  `0283` recursive finding deepening, `0284` set detail page. No prior research,
  plans, or ADRs exist for the topic-research engine itself.

## Related Research

- None directly on topic/web research. `meta/research/codebase/2026-02-22-skills-agents-commands-refactoring.md`
  and `meta/research/codebase/2026-03-15-review-lens-optimal-structure.md`
  underpin ADR-0005's generic-agent pattern and are the nearest prior art for the
  `researcher` design.

## Open Questions

- ✅ **Validator depth — RESOLVED (2026-09-08): Option B, generalised.** Extend
  the corpus schema to a `(type, kind)` lookup with a type-level fallback and
  rename the discriminator `research_kind` → `kind`. Per-kind required fields and
  status are enforced; each of the five templates gets its own row. Recorded in
  0277's Technical Notes and 0121's contract notes; candidate for an ADR.
- ✅ **`DOC_TYPES` / catalogue placement — RESOLVED (2026-09-08).** Only
  `paths.research_topics` (a PATH_KEY) lands in 0277, forcing three test-fixture
  edits (catalogue count `55`→`56`, `paths.golden`, `dump.golden`) and nothing in
  the doc-type/`DocTypeKey`/visualiser tests. The `DOC_TYPES` entry is coupled to
  the `DocTypeKey` variant (`doc_type_single_source.rs` hard-fails without it), so
  it ships with 0278. `--file` validation is `DOC_TYPES`-independent, so the
  engine needs none of it. 0277 stays green in isolation.
- ✅ **Resolver home — RESOLVED (2026-09-09).** A general
  `accelerator corpus resolve --type <type> <slug>` subcommand on the `corpus`
  binary (mirroring `work-cli/resolve.rs`), resolving any doc type by slug. "set
  handle" is renamed "slug" (the codebase's cross-type identifier); the flag is
  `--type`, not `--root`. ⚠️ Type-driven `topic-research` resolution couples to
  the 0278 doc-type registration, so 0277 ships the generic resolver and
  topic-research resolution is verified at co-land.
- ✅ **`topic-research` as a linkage target — RESOLVED (2026-09-09): add in 0277.**
  Register `topic-research` in `LINKAGE_SOURCE_TYPES` (`schema.rs`) and
  `SOURCE_TYPES` (`template_shape.rs`) + the public-api snapshot, so other docs may
  `relates_to: ["topic-research:<slug>"]`. These arrays are decoupled from
  `DOC_TYPES`/`DocTypeKey`, so the shape check lands cleanly in 0277; full
  dangling-reference integrity (whole-corpus mode) co-lands with 0278.
- ✅ **Source-profile packaging — RESOLVED (2026-09-09): path-passing.** `conduct`
  passes the chosen profile's skill path + the finding output-format path per
  spawn; the researcher Reads them in its own context (ADR-0005 reviewer pattern).
  A static `skills:` preload is unsuitable because the profile varies per spawn
  (web now; academic + mixed rounds in 0280). Profiles live at
  `skills/research/profiles/<profile>/SKILL.md`, mirroring `skills/review/lenses/`,
  with a finding output-format reference alongside.

## Follow-up Research 2026-09-08T22:16:16+00:00 — Reference deep-research systems

**Scope.** Web research on the plugins and systems the epic cites, to educate
how the four verb prompts and the `researcher` brief are written. The epic
already made its design choices against these systems; this section extracts the
*concrete, copyable* prompt/workflow mechanics and marks where 0277 deliberately
diverges. All sources are linked at the end.

### The five reference systems at a glance

| System | Kind | Loop shape | Immutability model | Citation model |
|---|---|---|---|---|
| Sagan | Claude Code plugin (3 skills) | brief(+topics) → deep-research(1 round) → synthesize | Per-topic files skipped if present; brief append-only; synthesis full-rewrite | `[T1]/[T2]/[T3]` source-**type** tags, inline + `## Sources` |
| Anthropic multi-agent | Prod system (essay) | lead plans → parallel subagents → citation pass | n/a (report, not corpus) | Dedicated `CitationAgent` final pass |
| dzhng/deep-research | CLI tool | feedback(up-front) → recursive breadth×depth | Accumulated `learnings`/`urls` threaded | URLs collected, final report |
| 199-biotech skill | Claude skill | 8-phase Scope→…→Package | Append-only `sources/evidence/claims.jsonl` | Composite domain trust score |
| Weizhena skill | Claude skill (3 cmds) | /research → /research-deep → /research-report | JSON intermediates between phases | Per-item fields |

**The closest model is Sagan** — a Claude Code plugin solving the identical
problem through on-disk, filesystem-communicating skills. It is the primary
teacher for prompt phrasing; Anthropic supplies the orchestration theory; dzhng
and the skill repos supply parameterisation and source-tiering vocabulary.

### Sagan — the copyable mechanics (and the one structural fork)

**File layout maps almost 1:1 onto 0277, with one addition.** Sagan's set is
`brief.md` (sole top-level file, doubling as the append-only round log via
`## Round N` sections), `research/{slug}.md` (immutable per-topic dossiers), and
`synthesis/general.md` (rewritten whole each round). 0277 adds a dedicated
`manifest.md` aggregate root Sagan lacks — because the 0278 indexer keys on it —
and splits Sagan's brief-embedded round log into a separate `outline.md`.

**Gap detection is a pure file-existence check** — worth copying near-verbatim
into 0279 (not 0277, which is single-round):

> For each `## Round N` section in order: list the topic files it expects (one
> per `File:` line). Check which already exist. **The first round where any topic
> file is missing is the round to execute.**

Plus a staleness clause (synthesis older than the latest topic file → rewrite)
and a **post-dispatch verify gate** ("verify every expected topic file exists…
Don't synthesise an incomplete corpus silently"). This is the mechanism behind
the epic's "finding-existence is ground truth; `conduct` reconciles the checkbox".

**The anti-changelog rule lives in two prompts and is structurally enforced.**
Sagan puts it in both the subagent prompt and the synthesis-writer prompt, and
backs it by full-rewrite-never-append on the synthesis:

> Write as standalone research. Do not reference the research process itself —
> no "in this round", "previously we found"… The reader should not be able to
> tell from the prose what round produced this file.

0277's `synthesise` writes `synthesis.md` inline from the findings; adopt this
wording verbatim, and note the epic's matching carve-out — `outline.md` is
*exempt* (it is the working log), exactly as Sagan exempts the brief's round
sections.

**The subagent brief template is a ready-made skeleton for the `researcher`.**
Sagan's shape: `"you are 1 of N"` framing → context slots pasted **verbatim from
the brief** (goal, constraints, source types) → a single `OUTPUT PATH` → an
external-first directive ("Do NOT rely on training data alone — it is stale,
generic, and uncitable") → an explicit output contract (word/section count,
citation density, a `## Sources` section) → hard boundaries ("Do not write
anywhere else… that's the orchestrator's job") → a return contract (a <200-word
summary; the real artefact is the file). This is the concrete filling of
Anthropic's four-part brief (below) and the template to adapt for
`(source_profile, question)`.

⚠️ **The structural fork: Sagan has no separate `outline` verb.** It folds topic
enumeration into `create-brief` and folds propose-next-round into
`deep-research` behind a gate. 0277/0121 **deliberately reject this** — the
epic's central design divergence is a user-editable `outline` verb split from
`conduct` (the `create-plan` → `implement-plan` idiom). So copy Sagan's prompts
but *not* its verb boundaries; the natural re-partition is: `brief` writes the
header (goal/scope/`source_profiles`), `outline` writes the `## Round 1`
checklist.

⚠️ **Do not conflate Sagan's `[T1]/[T2]/[T3]` with 0277's tiers.** Sagan's tags
are a source-**type** taxonomy (paper / doc / social-post). 0277's
`tier-1/2/3` is a **reputation** ranking by venue standing, orthogonal to type.
A peer-reviewed paper and an official standard are both `tier-1` in 0277 but
different Sagan types. The finding template must tag reputation, not category.

### Anthropic multi-agent — the orchestration theory for `outline` and `conduct`

**The effort-scaling rubric carries a per-agent tool-call budget, not just a
count** (verbatim), and belongs in the `outline` prompt:

> Simple fact-finding requires just 1 agent with 3-10 tool calls, direct
> comparisons might need 2-4 subagents with 10-15 calls each, and complex
> research might use more than 10 subagents with clearly divided
> responsibilities.

⚠️ At 0277's hardcoded `breadth: 8` the "10+" band is unreachable (the epic's
intended ceiling behaviour) — but the *per-researcher* call budget (3–10 /
10–15) is a second dial the epic doesn't yet mention and worth encoding in the
outline rubric.

**The four-part subagent brief is the anti-duplication contract for `conduct`:**

> Each subagent needs an objective, an output format, guidance on the tools and
> sources to use, and clear task boundaries. Without detailed task descriptions,
> agents duplicate work, leave gaps, or fail to find necessary information.

0277's `(source_profile, question)` supplies **objective** (question) +
**tools/sources** (profile); `conduct` must additionally inject **output
format** and **task boundaries** so parallel researchers don't run "the exact
same searches". Additional guard-rails to bake into the researcher prompt:
"start wide, then narrow" (short broad queries first), and a stop condition
("stop when you have sufficient results") — both quoted failure modes.

⚠️ **Citation as a dedicated final pass is Anthropic's model but the epic defers
it.** Anthropic runs a separate `CitationAgent` over aggregated findings. 0121
explicitly chooses inline reputation-tagging in immutable findings with **no
citation agent**, keeping the dedicated final pass as a "deferred backstop, safe
to add retroactively because the immutable tagged findings preserve
attribution". So 0277 does *not* build a citation pass; it relies on the web
profile tagging each source inline.

**Cost:** "multi-agent systems use about 15× more tokens than chats." The
effort rubric, the stop condition, and reserving fan-out for genuinely broad
work are the levers — reinforcing why `breadth: 8` is a conservative default.

### dzhng and the skill repos — parameterisation and source-tiering

**dzhng's two question generators map onto the brief/depth split.**
`generateFeedback` (asked once, up front, to a human, ~3 questions) is 0277's
interactive `brief` scoping. `followUpQuestions` (machine-generated inside the
recursion) is the intra-finding recursion 0277 defers to `depth > 1` (0282/0283)
and is inert here. dzhng's defaults: `breadth` 4 (queries per level), `depth` 2,
with `breadth = ceil(breadth/2)` per descent — the halving 0282 will adopt.

**199-biotech is the richest source-tiering and immutable-finding model.** It
uses named domain tiers (`HIGH_AUTHORITY` academic/gov/standards,
`MODERATE_AUTHORITY` tech-news/business, `LOW_AUTHORITY` free-host blogs) and
append-only `sources.jsonl` / `evidence.jsonl` / `claims.jsonl` ledgers with a
"3+ sources per major claim, no unsupported claim ships" gate.

⚠️ **0277's reputation model is deliberately simpler.** The epic mandates a
closed three-value set (`tier-1` authoritative-primary, `tier-2`
reputable-secondary, `tier-3` unvetted), reputation-only, reflecting venue
standing not claim correctness — *not* 199-biotech's composite weighted score
(domain × recency × expertise × bias). 199-biotech's **named domain lists** are
still useful as a starting concordance for what lands in each tier, but the
scoring machinery is out of scope. imbad0202 adds one worth-borrowing nuance for
the finding contract: distinguish *provably wrong* (block) from *unverifiable*
(annotate, don't block) rather than a single pass/fail.

**Weizhena and Awesome-Deep-Research validate the single-round plan/execute
shape.** Weizhena exposes the same three-verb split (outline → deep → report)
with JSON intermediates "for auditability over speed". The landscape survey's
takeaway for a single-round web engine is "search more, think less" — broad
initial coverage in one pass over deep iterative loops — which is exactly 0277's
single-round posture with the heavy planning in `outline`.

### Design implications, mapped to 0277's verbs

| Verb | Copy from the references | Source |
|---|---|---|
| `brief` | ~3 up-front clarifying questions (interactive scoping); write goal/scope/`source_profiles` header | dzhng `generateFeedback`, Sagan brief interview |
| `outline` | Effort-scaling rubric verbatim (count + per-researcher call budget); define finding fields up front; `breadth` = focus areas, capped | Anthropic rubric, Weizhena field schema |
| `conduct` | Four-part brief per researcher; partition questions to avoid overlap; single-message parallel dispatch; batches of 3–5 | Anthropic delegation, Sagan single-message fan-out |
| `researcher` | Sagan's "1 of N" brief skeleton; external-first directive; "start wide then narrow"; stop condition; inline tier-tagged `## Sources` | Sagan subagent prompt, Anthropic heuristics |
| `synthesise` | Full-rewrite-never-append; anti-changelog wording in the prompt; carry citations forward verbatim | Sagan synthesis + extraction prompts |

### Where 0277 deliberately diverges from all of them

- **Separate, user-editable `outline`** — none of the references surface a
  user-editable plan step; 0277's plan/execute split is the intentional
  divergence (matches `create-plan` → `implement-plan`).
- **YAML frontmatter, not Sagan's bold-markdown headers** — an epic assumption,
  and what makes `corpus frontmatter validate` applicable.
- **Reputation-only `tier-1/2/3` by venue standing** — simpler than 199-biotech's
  composite trust score, orthogonal to Sagan's source-type tags.
- **Single broad round now; recursion deferred** — dzhng's `depth` recursion is
  0282/0283, inert at `depth: 1`.
- **Inline tagging, no citation agent** — Anthropic's `CitationAgent` is the
  deferred backstop, not built in 0277.
- **Dedicated `manifest.md` aggregate root** — Sagan uses the brief as the root;
  0277 needs a stable manifest for the 0278 indexer.

### Sources

- Sagan plugin — [create-brief/SKILL.md](https://github.com/robertbagge/claude-sagan-plugin/blob/main/skills/create-brief/SKILL.md), [deep-research/SKILL.md](https://github.com/robertbagge/claude-sagan-plugin/blob/main/skills/deep-research/SKILL.md), [synthesize/SKILL.md](https://github.com/robertbagge/claude-sagan-plugin/blob/main/skills/synthesize/SKILL.md), example set [brief.md](https://github.com/robertbagge/claude-sagan-plugin/blob/main/meta/natural-language-processing/brief.md) / [synthesis/general.md](https://github.com/robertbagge/claude-sagan-plugin/blob/main/meta/natural-language-processing/synthesis/general.md) / [research/tokenization.md](https://github.com/robertbagge/claude-sagan-plugin/blob/main/meta/natural-language-processing/research/tokenization.md)
- Anthropic — [Building a multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)
- [dzhng/deep-research](https://github.com/dzhng/deep-research) — [src/deep-research.ts](https://github.com/dzhng/deep-research/blob/main/src/deep-research.ts), [src/feedback.ts](https://github.com/dzhng/deep-research/blob/main/src/feedback.ts)
- [199-biotechnologies/claude-deep-research-skill](https://github.com/199-biotechnologies/claude-deep-research-skill) — [scripts/source_evaluator.py](https://github.com/199-biotechnologies/claude-deep-research-skill/blob/main/scripts/source_evaluator.py)
- [imbad0202/academic-research-skills](https://github.com/imbad0202/academic-research-skills)
- [lingzhi227/agent-research-skills](https://github.com/lingzhi227/agent-research-skills)
- [Weizhena/Deep-Research-skills](https://github.com/Weizhena/Deep-Research-skills)
- [DavidZWZ/Awesome-Deep-Research](https://github.com/DavidZWZ/Awesome-Deep-Research)
