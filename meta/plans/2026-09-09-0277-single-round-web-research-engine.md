---
type: "plan"
id: "2026-09-09-0277-single-round-web-research-engine"
title: "Single-Round Web Research Engine Implementation Plan"
date: "2026-09-09T21:11:19+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0277"
parent: "work-item:0277"
derived_from: ["codebase-research:2026-09-08-0277-single-round-web-research-engine"]
relates_to: ["adr:ADR-0067", "adr:ADR-0068"]
tags: ["research", "skills", "deep-research", "corpus", "topic-research"]
revision: "d6b4b2954cbad1b4e631cfd55395028eca73c3f6"
repository: "accelerator"
last_updated: "2026-09-10T01:27:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Single-Round Web Research Engine Implementation Plan

## Overview

Deliver the walking-skeleton engine for topic research: a `research-topic` skill
that dispatches `brief → outline → conduct → synthesise` over web sources and
writes a contract-conforming *set* under `meta/research/topics/<slug>/`
(`manifest.md`, `brief.md`, `outline.md`, `findings/*`, `synthesis.md`). The
engine is built on the reusable-infrastructure seam — a generic `researcher`
agent specialised at spawn by an injected `(web profile, question)` pair,
mirroring the `reviewer` agent — plus the corpus-side machinery the artifacts
depend on: a `(type, kind)`-discriminated frontmatter schema (ADR-0067), a
general `corpus resolve` subcommand (ADR-0068), the `paths.research_topics`
config key, and five web-only templates.

This is the engine half of epic Slice 1. Its sibling, 0278 (visualiser doc-type
registration + nested-manifest indexer), makes the artifacts reader-observable
and registers `topic-research` as a `DocTypeKey`. The two **co-land**: the engine
writes conforming artifacts on disk and passes all local checks in isolation, but
`corpus resolve --type topic-research` and whole-corpus discovery only function
once 0278 adds the doc-type variant. The `manifest.md` contract this engine
writes is the seam 0278's indexer keys on; the hand-authored full-set fixture
(Testing Strategy) is committed here as the shared artifact both work items
reference, so a shape mismatch surfaces as a failing test rather than only at
simultaneous merge.

## Current State Analysis

Every seam the engine needs has a concrete precedent already in the tree; the
genuinely new code is one schema dimension, one config key, one CLI subcommand,
five templates, one agent, two profile/format skills, and the skill body.

- **Corpus frontmatter validation** keys on `type` alone. `SchemaRow`
  (`cli/corpus/src/frontmatter_validation/schema.rs:4-12`) has no `kind` field;
  `row_for` (`schema.rs:177-179`) is a pure string match; `SCHEMA` is
  `[SchemaRow; 13]`; the TSV (`templates-schema.tsv`) has 7 columns and 13 data
  rows kept 1:1 with `SCHEMA` by `every_row_matches_templates_schema_tsv`
  (`schema.rs:274-322`). No code reads a `kind`/`research_kind` discriminator.
- **`--file` validation is `DocTypeKey`-independent.** `target_files`
  (`cli/corpus-cli/src/frontmatter.rs:26-47`) never scope-filters files named by
  `--file`; `validate_file` (`mod.rs:187-211`) resolves the row from the
  document's own `type:`. So the engine writes and validates each document with
  only `SCHEMA` rows present.
- **`DocTypeKey` couplings sit behind 0278.** Adding `topic-research` to
  `DOC_TYPES` (`cli/config/src/catalogue.rs:66-80`) without a
  `DocTypeKey::TopicResearch` variant hard-fails `doc_type_single_source`
  (`cli/corpus-adapters/tests/common/mod.rs:112-118`). `nested_manifest_filename`
  and the type→path mapping both live on `DocTypeKey`
  (`cli/corpus/src/doc_type.rs`), so type-driven `topic-research` resolution
  needs 0278.
- **`corpus-cli` is clap-derive with four subcommands** (`Adr`, `Metadata`,
  `Linkage`, `Frontmatter`; `cli/corpus-cli/src/cli.rs:17-40`) and **no**
  per-outcome exit-code taxonomy — its `main` distinguishes only
  success/failure/refusal (`main.rs:129-156`). `work resolve`
  (`cli/work/src/resolve.rs`, `cli/work-cli/src/resolve.rs`) is the mirror source
  for classification, candidate handling, and the canonicalise-then-under-root
  containment check.
- **Template resolution is a name-keyed 3-tier walk** (`config-adapters`
  `ReadTemplate`, `store.rs:368-410`) orchestrated by `template.rs::resolve`
  (`cli/launcher/src/config_command/core/template.rs:22-31`); it has no
  `(type, kind)` awareness.
- **The generic-agent pattern is path-passing (ADR-0005).** `agents/reviewer.md`
  is content-free (`tools: Read, Grep, Glob, LS`); `review-plan`
  (`skills/planning/review-plan/SKILL.md:234-274`) spawns it with a lens path +
  output-format path injected into the prompt, resolving `subagent_type` via
  `!accelerator config agent reviewer`. `agents/web-search-researcher.md` is the
  only `WebFetch`/`WebSearch` grant.

### Key Discoveries

- **`cargo-public-api` runs inside `mise run check`** (`mise.toml:667-669`), not
  only the test lane. Any phase that changes a pinned crate's public surface
  (`corpus`, `config`) must regenerate the snapshot (`mise run public-api:update`)
  in that same phase or `check` goes red. `corpus-cli` and `work-cli` are exempt
  composition roots (`tasks/public_api.py:67,71`) — a subcommand confined to
  `corpus-cli` touches no snapshot.
- **`validate_templates` runs in the test lane, not `check`**
  (`cli/corpus-adapters/tests/template_shape_tree.rs:15-28`). A TSV row naming a
  template file requires that file to exist *and* its filename added to the
  `## Schema Reference` table of one of work items 0065/0066/0067
  (`cross_check`, `template_shape.rs:593-608`); only the filename cell is
  test-load-bearing.
- **The catalogue has one count assertion**
  (`the_catalogue_holds_fifty_five_keys…`, `catalogue.rs:256-266`, currently
  `55`) plus two byte-exact goldens (`paths.golden`, `dump.golden`). A new
  `PATH_KEY` and a new `AGENT_KEY` each move the count and `dump.golden`.
- **The nested-manifest resolver branch is testable in isolation.**
  `nested_manifest_filename()` already returns `Some("inventory.md")` for
  `DesignInventories` (`doc_type.rs:160-165`), so the resolver's set-root walk is
  exercised against `design-inventory` and its flat-dated candidate path against
  `codebase-research`/`plan`, none needing 0278. `corpus::slug::derive`
  (`slug.rs:10-38`) already dispatches filename shape per type.
- **A subagent cannot be granted safely-scoped shell.** Agent `tools:` is a hard
  ceiling admitting only bare `Bash`; a skill's `allowed-tools` auto-approves
  rather than sandboxes, and the `!` preprocessor (the only scoped CLI a no-Bash
  subagent gets, via a preloaded skill) runs at spawn and cannot see post-spawn
  output. So the `researcher` runs no CLI; `conduct` owns metadata derivation and
  the post-write validation, mirroring how `review-plan` — not the `reviewer` —
  owns a review's post-processing.

## Desired End State

Running `mise run` (the bare default task) exits 0 with the engine present: the
`research-topic` skill, the `researcher` agent, the web profile and finding
outputter skills, five templates, the `(type, kind)` schema, the
`paths.research_topics` key, and the general `corpus resolve` subcommand all land
with format, lint, type-check, and the full test suite green.

Verification of the end state:

- `accelerator corpus resolve --type design-inventory <slug>` resolves a slug,
  a set directory, and a sub-document path to the same set root; `--type
  codebase-research <slug>` handles the ambiguous dated-slug case with candidate
  output. `--type topic-research` reports an unknown type until 0278 (expected).
- A hand-authored set under `meta/research/topics/<slug>/` — one `manifest.md`,
  `brief.md`, `outline.md`, a `findings/01-*.md`, and `synthesis.md` — each
  passes `accelerator corpus frontmatter validate --file <path>`, with per-kind
  required fields and per-kind status vocab enforced.
- `accelerator config path research_topics` prints `meta/research/topics`;
  `accelerator config template topic-research --kind brief` resolves the brief
  template, falling back to a general `topic-research` template when no
  kind-specific file exists; `accelerator config agent researcher` resolves to
  `accelerator:researcher`.
- A `relates_to: ["topic-research:<slug>"]` reference in another document passes
  the frontmatter shape check.

## What We're NOT Doing

- **0278**: the `DocTypeKey::TopicResearch` variant, the `DOC_TYPES` entry, the
  `server/src/docs.rs`/`frontend` registration, glyph/colour/VR baselines, the
  nested-manifest indexer wiring, and whole-corpus `topic-research` discovery or
  dangling-reference integrity.
- **0279**: gap-detection re-invocation, multi-round `outline` appending,
  anti-changelog wholesale synthesis rewrite, and `finalise`. `conduct` here
  researches every focus area of a single round.
- **0280**: academic source profiles (OpenAlex/arXiv), `research.contact_email`,
  rate-limit-aware fan-out, mixed-profile rounds.
- **0281**: the `ask`/`report` consume verbs and the `report` kind.
- **0282/0283**: config-tunable `breadth`/`depth`, override flags, and the
  `depth > 1` recursion engine. `breadth: 8` and `depth: 1` are hardcoded in the
  skill prompts; `depth: 1` is inert.
- **0284**: the set-level detail page.
- Any `report`-kind schema row or template; any citation agent (inline tier
  tagging in the immutable findings is the source of truth).

## Implementation Approach

Seven phases, each independently mergeable and green in sequence — every phase
depends only on earlier phases, never later ones. Each phase follows
red-green-refactor: a failing test states the behaviour, the minimum change makes
it pass, then the surrounding code is tidied. The Rust phases (1–5) land the
corpus/config machinery; the markdown phases (6–7) land the agent, profiles, and
skill. Phases 1–4 are logically self-contained but not conflict-free in parallel:
several mutate the same shared registries — the catalogue count assertion and its
renamed test (2, 5, 6), `templates-schema.tsv`/`SCHEMA` (1, 3, 5), the pinned
`public-api.txt` snapshot (1, 3, 4, 5), and `dump.golden` (2, 5, 6) — so land them
in ascending phase order rather than concurrently to avoid rebasing those lines.
Phase 5 depends on Phase 1 and Phase 7 depends on 2/4/5/6.

The Rust snippets below are illustrative; rustfmt wraps them to the enforced
80-column width on implementation, so their exact line breaks are not normative.

Phase-to-criterion map against the story's acceptance criteria:

| Phase | Story acceptance criteria addressed |
|---|---|
| 1. `(type, kind)` mechanism | Per-kind validation + template resolution (AC 10, 12) |
| 2. `paths.research_topics` | Path key default + bare-slug resolution (AC 7) |
| 3. Linkage source type | `topic-research:<slug>` shape check (AC 13) |
| 4. `corpus resolve` | General resolver, tested vs registered type (AC 5, 6) |
| 5. Templates + kind rows | Per-kind templates + frontmatter validation (AC 10, 12) |
| 6. `researcher` + web profile + outputter | Single generic agent, web tiers (AC 9, 11) |
| 7. `research-topic` skill | The four verbs, breadth ceiling, tiers (AC 1–4, 8) |

---

## Phase 1: `(type, kind)` schema and template resolution

### Overview

Realise ADR-0067: extend the corpus frontmatter schema and the template resolver
to key on `(type, kind)` with a type-level fallback, renaming the (as-yet
unimplemented) discriminator `research_kind` → `kind`. Behaviour-preserving — the
13 existing types become `(type, "")` rows and resolve exactly as before.

### Changes Required

#### 1. `SchemaRow` gains a `kind` dimension

**File**: `cli/corpus/src/frontmatter_validation/schema.rs`
**Changes**: Add a `kind` field; extend `row_for` to a composite lookup with a
type-level fallback; the 13 existing rows carry `kind: ""`.

```rust
pub struct SchemaRow {
    pub linkage_type: &'static str,
    pub kind: &'static str,
    pub code_state_anchored: bool,
    pub extras: &'static [&'static str],
    pub status_vocab: &'static [&'static str],
    pub forbidden_own_id_keys: &'static [&'static str],
    pub typed_linkage_keys: &'static [&'static str],
}

pub fn row_for(linkage_type: &str, kind: &str) -> Option<&'static SchemaRow> {
    SCHEMA
        .iter()
        .find(|row| row.linkage_type == linkage_type && row.kind == kind)
        .or_else(|| {
            SCHEMA
                .iter()
                .find(|row| row.linkage_type == linkage_type && row.kind.is_empty())
        })
}
```

Every existing `SchemaRow` literal gains `kind: ""`. `SCHEMA` stays length 13 in
this phase; `thirteen_rows_are_present` is unchanged.

`row_for` is a pinned public item; the `row_for(type)` → `row_for(type, kind)`
change is breaking, so extend every caller this phase (the compiler enumerates the
full set — the `migrate` m0007 production and test sites under `backfill.rs`/
`rewrite.rs`, and the corpus in-crate schema tests). `validate_file` and the
in-crate `dangling_refs` (`cli/corpus/src/frontmatter_validation/mod.rs`) must each
read the document's real `kind` from the frontmatter they already parse — a
hardcoded `""` at `dangling_refs` would leave a multi-kind type like topic-research
(which has no `(type, "")` default row) unresolvable, silently skipping its
outbound-reference integrity check. The `migrate` and schema-test callers touch only
the 13 legacy types, which do have a `(type, "")` row, so they pass `""`.

#### 2. `validate_file` reads and forwards the `kind`

**File**: `cli/corpus/src/frontmatter_validation/mod.rs`
**Changes**: Read the document's `kind` (empty when absent) and pass it to
`row_for`.

```rust
let declared = declared_type(&entries).unwrap_or_default();
let kind = raw_value(&entries, "kind")
    .map(strip_surrounding_quote)
    .unwrap_or_default();
let Some(row) = schema::row_for(declared, kind) else {
    return vec![unresolved_row_violation(declared, kind)];
};
```

`unresolved_row_violation` distinguishes the two failures `row_for`'s single
`None` conflates: an unknown type (no `SCHEMA` row shares the `linkage_type`)
stays `Violation::InvalidType`, while a known type carrying an unmatched `kind`
with no `(type, "")` default yields a new `Violation::UnknownKind { type, kind }`.
Without the split a typo'd `kind` on a valid type misreports the type as invalid,
and any future `(type, "")`-defaulted multi-kind type would let a typo'd kind fall
through to the wrong required-field/status rules silently. The new variant moves
the public-API snapshot (regenerated in section 6). Add `Violation::UnknownKind`
to the adapter's `structural_gate_failed` set
(`corpus-adapters/src/frontmatter_validation.rs`) so a kind-unresolvable document is
gated and reported `Skipped`, exactly as `InvalidType` is, rather than run through
reference checking against a row that cannot resolve.

#### 3. TSV gains a `kind` column

**File**: `cli/corpus/src/frontmatter_validation/templates-schema.tsv`
**Changes**: Insert a `kind` column after `type`. Existing rows carry `-`
(the empty-value sentinel). Header and one row for illustration:

```text
template	type	kind	code_state_anchored	extras	status_vocab	forbidden_own_id_key	typed_linkage_keys
codebase-research.md	codebase-research	-	yes	topic	complete	-	parent relates_to
```

#### 4. TSV field-count and parity self-checks move to 8 columns

**File**: `cli/corpus/src/frontmatter_validation/template_shape.rs`
**Changes**: `SCHEMA_TAB_FIELDS` `7`→`8`; the `HEADER` const gains the `kind`
column; `parse_schema_tsv` binds the new `kind` field; `TemplateRow` gains `kind`.
`kind` is inserted as the second column (after `type`), so every positional field
index after it in `parse_schema_tsv` shifts by one — `code_state_anchored`,
`extras`, `status_vocab`, `forbidden_own_id_keys`, and `typed_linkage_keys` — each
must be re-pointed, not only the new binding added. `validate_template` gains a
`kind` presence/match check — the template's declared `kind:` must equal `row.kind`
— mirroring the existing `type`/`doc_type` check, so the new `TemplateRow.kind` is
enforced rather than dead data and a template's declared kind cannot drift from its
schema row.

**File**: `cli/corpus/src/frontmatter_validation/schema.rs`
**Changes**: `every_row_matches_templates_schema_tsv` binds the 8th column and
keys `row_for(type_name, kind)`.

```rust
let [_template, type_name, kind, anchored, extras, status_vocab, forbidden, linkkeys] =
    columns.as_slice()
else {
    return Err(format!("unexpected column count: {line}").into());
};
let kind = if *kind == "-" { "" } else { kind };
let row = row_for(type_name, kind)
    .ok_or_else(|| format!("no SCHEMA row for ({type_name}, {kind})"))?;
```

#### 5. Template resolver gains an optional `kind` with a type fallback

**File**: `cli/launcher/src/config_command/core/template.rs`
**Changes**: `resolve` accepts an optional `kind`; when present it attempts
`<name>-<kind>` first, falling back to `<name>`. The store's tier-walk
(`store.rs`) is untouched.

```rust
pub fn resolve(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    name: &str,
    kind: Option<&str>,
) -> Result<Option<ResolvedTemplate>, ConfigError> {
    if let Some(kind) = kind.filter(|value| !value.is_empty()) {
        let composite = format!("{name}-{kind}");
        if let Some(resolved) = resolve_one(config, templates, &composite)? {
            return Ok(Some(resolved));
        }
    }
    resolve_one(config, templates, name)
}
```

The `config template` CLI surface gains an optional `--kind` argument threaded to
`resolve`. The existing `resolve` callers are launcher-internal — roughly five
sites in `config_command/inbound/cli.rs` plus the internal call in
`core/template.rs` — each threaded `None` to keep single-attempt behaviour
(`launcher` is a public-API-exempt composition root, so this is an internal
change).

#### 6. Public-API snapshot

**File**: `cli/corpus/tests/fixtures/public-api.txt`
**Changes**: Regenerate — the new `SchemaRow::kind` field and the changed
`row_for` signature appear in the snapshot. Run `mise run public-api:update`.

### Success Criteria

#### Automated Verification

- [x] Composite lookup and fallback unit tests pass: `cargo test -p corpus row_for`
- [x] TSV/SCHEMA parity holds: `cargo test -p corpus every_row_matches_templates_schema_tsv`
- [x] Template resolver kind-fallback test passes: `cargo test -p launcher template::resolve`
- [x] Public-API snapshot regenerated and clean: `mise run public-api:update && mise run public-api:check`
- [x] Full corpus + launcher tests pass: `mise run test`
- [x] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [x] `accelerator corpus frontmatter validate --file <existing codebase-research doc>` still passes unchanged (a document with no `kind` resolves via `(type, "")`).
- [x] `accelerator config template codebase-research` (no `--kind`) resolves the same file as before.

---

## Phase 2: `paths.research_topics` config key

### Overview

Register the path key so the skill locates the set directory, defaulting to
`meta/research/topics`. A bare `PATH_KEY` — no `DOC_TYPES` entry (that couples to
the 0278 `DocTypeKey` variant).

### Changes Required

#### 1. Catalogue entry

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add to `PATH_KEYS`, after `paths.research_issues`.

```rust
    (
        "paths.research_topics",
        Default::Scalar("meta/research/topics"),
    ),
```

#### 2. Catalogue count test

**File**: `cli/config/src/catalogue.rs`
**Changes**: Bump the assertion `55`→`56` and rename the test function to match.
`resolve_with_fallback` and `config path` gate on `default_for`, which now
recognises the key with no further change.

#### 3. Byte-exact goldens

**File**: `cli/launcher/tests/fixtures/baseline/paths.golden`
**Changes**: Add `- research_topics: meta/research/topics` in `PATH_KEYS` order
(after `research_issues`).

**File**: `cli/launcher/tests/fixtures/dump/dump.golden`
**Changes**: Add `` | `paths.research_topics` | `meta/research/topics` | default | ``
in `PATH_KEYS` order.

### Success Criteria

#### Automated Verification

- [x] Catalogue count test passes: `cargo test -p config the_catalogue_holds`
- [x] Golden tests pass: `cargo test -p launcher config_read`
- [x] Full test suite passes: `mise run test`
- [x] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [x] `accelerator config path research_topics` prints `meta/research/topics` with no config override present.

---

## Phase 3: `topic-research` typed-linkage source type

### Overview

Register `topic-research` as a typed-linkage source type so other documents may
reference a set via `relates_to: ["topic-research:<slug>"]` (or `parent:`, etc.).
These arrays are decoupled from `DOC_TYPES`/`DocTypeKey`, so the frontmatter shape
check passes in isolation; whole-corpus dangling-reference integrity co-lands with
the 0278 indexer.

### Changes Required

#### 1. Frontmatter linkage vocabulary

**File**: `cli/corpus/src/frontmatter_validation/schema.rs`
**Changes**: Add `"topic-research"` to `LINKAGE_SOURCE_TYPES` (`14`→`15`). This is
what `is_well_formed` (`shape.rs`) checks for frontmatter typed-linkage values.

#### 2. Template-slot vocabulary

**File**: `cli/corpus/src/frontmatter_validation/template_shape.rs`
**Changes**: Add `"topic-research"` to `SOURCE_TYPES` (`14`→`15`) so template
example refs using a `topic-research:` prefix validate.

#### 3. Linkage extraction classification

**File**: `cli/corpus/src/linkage.rs`
**Changes**: Add the `topic-research` `(source_type, key, target_type)` triples to
`TYPE_PAIRS` (`59-76`) so extracted references to/from topic-research documents
classify as `Resolved` rather than `Ambiguous`. Enumerate the intended triples
explicitly (e.g. `(topic-research, parent, …)`, `(topic-research, relates_to, …)`,
and the inbound `(…, relates_to, topic-research)` edges other documents use to
point at a set), matching how the existing multi-type entries are spelled out — an
omitted triple leaves a valid reference classified `Ambiguous`, the outcome this
change exists to prevent. `TYPE_PAIRS` is a pinned public const whose length is
encoded in its type, so it also moves the snapshot (section 4).

#### 4. Public-API snapshot

**File**: `cli/corpus/tests/fixtures/public-api.txt`
**Changes**: Regenerate — both `LINKAGE_SOURCE_TYPES: [&str; 15]` and the
`TYPE_PAIRS` length pin change. Run `mise run public-api:update`.

### Success Criteria

#### Automated Verification

- [x] A `topic-research:<slug>` reference passes the shape check: `cargo test -p corpus shape`
- [x] Public-API snapshot clean: `mise run public-api:update && mise run public-api:check`
- [x] Full test suite passes: `mise run test`
- [x] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [x] A document carrying `relates_to: ["topic-research:some-subject"]` passes `accelerator corpus frontmatter validate --file <path>` (shape check clean; whole-corpus dangling-ref integrity co-lands with 0278).

---

## Phase 4: General `corpus resolve --type <type> <slug>` subcommand

### Overview

Realise ADR-0068: a general subcommand on the `corpus` binary that resolves a slug
to a document's root for any registered doc type — a file for flat types, the set
directory for nested-manifest types — mirroring `work resolve`. Delivered and
tested against already-registered types; type-driven `topic-research` resolution
is verified at the 0278 co-land.

### Changes Required

#### 1. Domain algorithm

**File**: `cli/corpus/src/resolve.rs` (new), wired into `cli/corpus/src/lib.rs`
**Changes**: Mirror the *shape* of `cli/work/src/resolve.rs` — a syntactic
`InputClass` classifier (`Path` / `Slug` / `Invalid`, matching the work mirror; a
sub-document path is a `Path` the adapter narrows, not a domain variant, so the
domain stays filesystem-free and unit-testable), a `DirectoryLister` port, and a
`resolve` cascade producing `Single` / `Ambiguous(Vec<TaggedCandidate>)` /
`NotFound`. Only the shape is mirrored: the ambiguity rule is corpus-specific —
match a `-<slug>.md` suffix across `YYYY-MM-DD-…` filenames — not
`resolve_bare_number`'s number-prefix candidate sources (project-code prepend,
zero-padding), which have no meaning for slugs.

Recovering a slug for a nested-manifest set comes from the *set directory name*,
not the manifest filename: `corpus::slug::derive` requires a `.md` filename and
returns `None` for the constant `inventory.md`, so add a dedicated directory-name
slug rule rather than reusing `slug::derive` (whose
`strip_prefix_date_and_optional_id` would mis-strip a 6-digit `HHMMSS` run as a
work-item id). Define one rule against *both* naming conventions — strip an optional
leading `YYYY-MM-DD` date and keep the full remainder, else take the whole directory
name — so it yields `HHMMSS-<id>` for a design-inventory set and the bare slug for a
topic-research set. Pin it with a unit test on the exact `YYYY-MM-DD-HHMMSS-<id>`
shape and a bare-slug shape before wiring the goldens, so the primitive is general
at delivery rather than retrofitted at the 0278 co-land.

#### 2. Adapter and containment

**File**: `cli/corpus-cli/src/resolve.rs` (new), `mod resolve;` in `main.rs`
**Changes**: Mirror `resolve_path_class` (`cli/work-cli/src/resolve.rs:33-49`) —
join, canonicalize the candidate, and enforce `starts_with(root)` against an
**equally-canonicalised root** (mirror `canonical_work_dir`,
`cli/work-cli/src/resolve.rs:73-85`: a canonicalised candidate carries the macOS
`/private` prefix a raw `project_root.join(dir)` lacks, so an uncanonicalised root
mis-classifies in-root paths as outside-root). Then branch on type: a file for flat
types; for nested-manifest types the set root is the first path segment **under the
configured type directory** — walk up from the canonicalised target until its
parent is the type root — not the immediate parent, because sub-documents live in
subdirectories (`findings/`, `screenshots/`) whose immediate parent is not the set
root. Resolve the type's configured directory via `config::compose` +
`config::paths::resolve_with_fallback(&service, <config_path_key>, None)` joined
against `project_root` (the `resolve_decisions_dir` precedent,
`cli/corpus-cli/src/config.rs:45-59`). Map `--type` to `DocTypeKey` via
`from_linkage_type_name`; an unregistered type yields a refusal (exit code below).

#### 3. CLI registration and exit codes

**File**: `cli/corpus-cli/src/cli.rs`, `cli/corpus-cli/src/main.rs`
**Changes**: Add a `Resolve { doc_type: String, slug: String }` variant
(`--type <type>` plus the slug positional) and a `run_resolve` handler that
returns `ExitCode` directly (bypassing the shared `Outcome`→`report` path) to
mirror the `work resolve` taxonomy: resolved→0, invalid→1, ambiguous→2,
not-found→3, outside-root→6. `resolve` adds one outcome `work resolve` has no
equivalent for — an unregistered/unknown `--type` (the `topic-research`-until-0278
case) — which takes a **distinct exit code 4**, never the shared `report`
refusal→2: within this binary exit 2 already means "refusal" for the four
`Outcome`-based subcommands and here means "ambiguous", so routing unknown-type
through the refusal path would make it indistinguishable from an ambiguous match
the skill preamble must branch on. Add the `exit_codes` surface corpus-cli lacks as
the single authority for the whole binary — naming every code it can emit
(`resolved=0`, `invalid=1`, `ambiguous=2`, `not_found=3`, `unknown_type=4`,
`outside_root=6`, and the pre-existing `report` refusal) — and route both the shared
`report` path and `resolve` through it, so the per-subcommand meaning of exit 2
(refusal vs. ambiguous) is documented in one place rather than folklore, mirroring
`work-cli`'s single `exit_codes` module.

#### 4. Tests

**File**: `cli/corpus/src/resolve.rs` unit tests; `cli/corpus-cli/tests/resolve_goldens.rs` (new)
**Changes**: Unit-test classification and ambiguity candidates in the domain.
Black-box goldens (mirroring `frontmatter_goldens.rs`, using `canonical_root` for
the macOS `/private` symlink) exercise every exit-code outcome, not only the happy
paths: a nested-manifest set-root walk against `design-inventory` (slug, set
directory, and a sub-document path nested one level below the manifest — e.g.
`screenshots/…` — all resolving to the set root, exit 0); the ambiguous flat-dated
candidate path against `codebase-research` (exit 2); an invalid input (exit 1); a
not-found slug (exit 3); an unregistered `--type` (exit 4); and an outside-root
refusal (exit 6). Add an in-root path exercised through a symlinked temp dir so the
root-canonicalisation is regression-covered.

#### 5. Public-API snapshot

**File**: `cli/corpus/tests/fixtures/public-api.txt`
**Changes**: Regenerate — the new `corpus::resolve` module adds pinned public
items. Run `mise run public-api:update`. (`corpus-cli` is exempt.)

### Success Criteria

#### Automated Verification

- [x] Domain classification/candidate tests pass: `cargo test -p corpus resolve`
- [x] Resolver goldens pass: `cargo test -p accelerator-corpus --test resolve_goldens`
- [x] Public-API snapshot clean: `mise run public-api:update && mise run public-api:check`
- [x] Full test suite passes: `mise run test`
- [x] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [x] `accelerator corpus resolve --type design-inventory <slug>`, the set directory, and a sub-document path all print the same set root.
- [x] `accelerator corpus resolve --type codebase-research <ambiguous-slug>` lists tagged candidates and exits 2.
- [x] `accelerator corpus resolve --type topic-research <slug>` reports an unknown type and exits 4 (expected until 0278).

---

## Phase 5: Topic-research templates and `(topic-research, <kind>)` schema rows

### Overview

Add the five web-only shapes as `(topic-research, <kind>)` schema rows and
matching plugin-default templates, so every document the loop writes validates by
its `(type, kind)` pair. Depends on Phase 1. Templates, TSV rows, SCHEMA rows, and
the Schema-Reference edits co-land so the full test lane stays green.

### Changes Required

#### 1. Five SCHEMA rows

**File**: `cli/corpus/src/frontmatter_validation/schema.rs`
**Changes**: Add five rows (`SCHEMA` `13`→`18`); bump `thirteen_rows_are_present`
to `18` and rename it. None is `code_state_anchored` (external subjects carry no
`revision`/`repository`).

| kind | extras | status_vocab |
|---|---|---|
| `manifest` | `slug`, `research_status`, `round_count`, `finding_count`, `primary` | `complete` |
| `brief` | `source_profiles` | `draft`, `complete` |
| `outline` | (none) | `complete` |
| `finding` | `round`, `question`, `source_profile` | `complete` |
| `synthesis` | `rounds_covered` | `complete` |

All five carry `typed_linkage_keys: ["parent", "relates_to"]`.

```rust
    SchemaRow {
        linkage_type: "topic-research",
        kind: "finding",
        code_state_anchored: false,
        extras: &["round", "question", "source_profile"],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
```

#### 2. Five TSV rows

**File**: `cli/corpus/src/frontmatter_validation/templates-schema.tsv`
**Changes**: Add five rows (data rows `13`→`18`), lock-stepped to `SCHEMA`.

```text
topic-research-finding.md	topic-research	finding	no	round question source_profile	complete	-	parent relates_to
```

#### 3. Five template files

**File**: `templates/topic-research-{manifest,brief,outline,finding,synthesis}.md`
**Changes**: Author each on the existing template format (frontmatter field order,
`{placeholder}` tokens in quoted values, `[bracketed prose]` bodies,
`schema_version: 1` bare, `id` quoted, status vocab as an inline comment, the
typed-linkage slot grammar). None carries the `revision`/`repository` provenance
pair. The finding template:

```text
---
type: "topic-research"                       # artifact-type discriminator
id: "{filename-stem}"                         # filename without .md
title: "Finding: {Focus Area Question}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "complete"                            # finding base status: complete only
kind: "finding"                               # (type, kind) discriminator
round: {round number}
question: "{focus area question}"
source_profile: "web"
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""
relates_to: []
tags: ["research", "topic-research", "finding"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# Finding: [Focus Area Question]

## Question
[The focus area's question, restated]

## Findings
[Standalone research prose — no round narration]

## Sources
- [Title](url) — tier-1 — {source domain/venue}
```

The manifest template carries `slug`, `research_status`, `round_count`,
`finding_count`, `primary`; the brief carries `source_profiles: ["web"]`; the
outline body is `## Focus Areas` with a `## Round N` checklist; the synthesis
carries `rounds_covered` and `Overview`/`Findings`/`Open Threads`/`Sources`. All
five carry the established per-field inline comments (type discriminator, id
derivation, per-kind status vocab, `(type, kind)` discriminator, typed-linkage slot
grammar) and present the typed-linkage slots as `parent: ""`/`relates_to: []` with
the standard omit-when-empty comment, exactly as `codebase-research.md`/
`design-inventory.md` do — those literal empties are valid in *templates*
(`check_empty_placeholders` runs in `validate_file` on written documents, not on
template shape validation). The omit-when-empty rule bites at write time, so the
finding outputter (Phase 6) instructs the agent to drop any empty linkage key from
the finding it writes; a written finding never carries `parent: ""`/`relates_to: []`.

#### 4. `TEMPLATE_KEYS` entries

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add five `templates.topic-research-<kind>` entries so users can
override each; bump the catalogue count assertion accordingly and rename the test.

**File**: `cli/launcher/tests/fixtures/dump/dump.golden`
**Changes**: Add the five `` | `templates.topic-research-<kind>` | *(not set)* |
default | `` rows in `TEMPLATE_KEYS` order — `dump.golden` enumerates every
template key by name, so `config_read` goes red without them.

#### 5. Schema-Reference cross-check

**File**: `meta/work/0065-update-artifact-templates-to-unified-schema.md`
**Changes**: Add five `` | `topic-research-<kind>.md` | … | `` rows to the
`## Schema Reference` table so `cross_check` sees the TSV set and the referenced
set match. Only the filename cell is test-load-bearing.

#### 6. Public-API snapshot

**File**: `cli/corpus/tests/fixtures/public-api.txt`
**Changes**: Regenerate — `SCHEMA: [SchemaRow; 18]` changes the pinned length.

#### 7. Committed full-set fixture

**File**: `cli/corpus-cli/tests/fixtures/topic-research-set/` (new) plus a case in
`frontmatter_goldens.rs`
**Changes**: Commit a hand-authored set — `manifest.md`, `brief.md`, `outline.md`,
`findings/01-*.md`, `synthesis.md` — and a golden that validates each via explicit
`accelerator corpus frontmatter validate --file <path>` (not the whole-corpus walk,
which skips `meta/research/topics/` until 0278 registers the doc type). This locks
the artifact contract 0278's indexer keys on, independent of live web and the
co-land. Add negative cases the positive fixture cannot cover: a `manifest` missing
`primary` yields `MissingExtra { extra: "primary" }`, and a `type: topic-research`
document with a missing or bogus `kind` yields `UnknownKind` (not `InvalidType`, and
not a silent pass against a wrong row) — the committed test for the Phase 1
`UnknownKind` split, only reachable now that the topic-research rows exist.

### Success Criteria

#### Automated Verification

- [x] TSV/SCHEMA parity and length pin pass: `cargo test -p corpus`
- [x] Per-kind validation is enforced by committed tests (not manual): a `kind: finding` document with `status: draft` is rejected and one with `status: complete` passes; a brief missing `source_profiles` yields a `MissingExtra`; a `manifest` missing `primary` yields `MissingExtra`; a topic-research document with a missing/bogus `kind` yields `UnknownKind`; and `row_for("topic-research", "finding")` resolves a row distinct from any `(topic-research, "")` fallback: `cargo test -p corpus`
- [x] The committed full-set fixture validates clean via explicit `--file`: `cargo test -p accelerator-corpus --test frontmatter_goldens`
- [x] The shipped templates tree is clean: `cargo test -p accelerator-corpus-adapters --test template_shape_tree`
- [x] Catalogue count and config-read goldens pass: `cargo test -p config the_catalogue_holds && cargo test -p launcher config_read`
- [x] Public-API snapshot clean: `mise run public-api:update && mise run public-api:check`
- [x] Full test suite passes: `mise run test`
- [x] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [x] A hand-authored finding with `status: draft` fails validation (finding vocab is `complete` only); with `status: complete` it passes.
- [x] A brief missing `source_profiles` fails with a `MissingExtra` violation.
- [x] `accelerator config template topic-research --kind synthesis` resolves `topic-research-synthesis.md`; `--kind report` (no such file) falls back to a general `topic-research` template if present, else reports not-found.

---

## Phase 6: Generic `researcher` agent, web profile, and finding outputter

### Overview

Add the reusable research infrastructure: one generic `researcher` agent granted
web fetch and write, a web source profile injected per spawn, and a finding
outputter — mirroring the `reviewer` / review-lens / output-format triad. An
**outputter** generalises the review output-format along one axis: it owns the
result *format*, the *sink* (here, write-to-the-injected-path), and the frontmatter
*contract* the agent fills. The researcher carries no `Bash`: every CLI-touching
step (metadata derivation, frontmatter validation) is `conduct`'s, because a
subagent consuming attacker-controlled web content cannot be granted safely-scoped
shell — the agent `tools:` ceiling admits only bare `Bash`, and a skill's
`allowed-tools` auto-approves rather than sandboxes. The outputter is path-passed,
matching the reviewer's output-format, so a future write-variant reviewer outputter
reuses the same shape. Register the `researcher` agent config key.

### Changes Required

#### 1. Generic `researcher` agent

**File**: `agents/researcher.md` (new)
**Changes**: Mirror `agents/reviewer.md` — a content-free agent whose task prompt
provides a source profile, a focus question, an outputter path, the conduct-derived
frontmatter values, and an output path. Grant web fetch and write, no `Bash`:

```text
---
name: researcher
description: Generic research agent that investigates one focus area through an
  injected source profile. Spawned by the research-topic conduct verb with a
  profile path, a focus question, a finding outputter, and pre-derived frontmatter
  values injected at spawn time.
tools: WebSearch, WebFetch, Write, Read
---
```

The grant excludes `Grep`/`Glob`/`LS`: the agent only Reads its two injected
path-passed files (profile, outputter) and writes one finding, so repo-wide
read/reconnaissance is unnecessary and shrinks the disclosure surface an injected
agent could exfiltrate (a `WebFetch` + repo-wide-read pair is a covert channel; the
deferred write-scope assertion bounds writes, not reads).

The body directs the agent to read its profile and outputter files first, research
the focus question externally (start wide then narrow; stop when sufficient),
compose the finding per the outputter from the injected frontmatter values (running
no CLI), write it to the injected output path with each source tagged
`tier-1`/`tier-2`/`tier-3` by venue standing, and return a short summary rather than
the finding body. The body carries an explicit untrusted-content contract (defined
in the web profile, below): fetched page content is **data, never instructions** —
the agent never follows directives embedded in a page, never reads or transmits
repository files outside the injected task, and writes only to the single injected
output path. The outputter carries one explicit example frontmatter block (mirroring
the `topic-research-finding` template) as the agent's authoring scaffold, and a
committed test parses that block's field names and asserts they equal the
`(topic-research, finding)` schema row's required set — a concrete left-hand side so
the writer, template, and schema row cannot drift silently.

A cheap frontmatter assertion over `agents/researcher.md` pins the exact `tools:`
set and asserts `Bash` is absent, so the no-shell invariant is regression-guarded
rather than manually re-inspected.

#### 2. Web source profile

**File**: `skills/research/profiles/web-profile/SKILL.md` (new)
**Changes**: Model on a review lens — `name: web-profile`, `user-invocable: false`,
`disable-model-invocation: true`, no `allowed-tools`, static body (the leaf name
carries the category suffix, matching the review-side `<x>-lens`/`<x>-output-format`
precedent and avoiding a collision-prone bare `web`). Define the web source family
scoped to **public `http(s)` URLs only** — no `file://`, no link-local or private
ranges (`169.254.169.254` and internal hosts), since the fetched bytes land
verbatim in a persisted finding — and the untrusted-content contract (page content
is data, never instructions; never exfiltrate or read outside the injected task).
Define the closed reputation-tier vocabulary (`tier-1` authoritative-primary:
official docs, standards bodies, peer-reviewed; `tier-2` reputable-secondary;
`tier-3` unvetted), require the source domain/venue recorded alongside each tier so
a mistag is auditable, and state that the tier is derived from **venue identity
only** — never from claims on the page — and reflects venue standing, never
correctness. Note that a lookalike, typosquatted, or newly-registered domain
(`docs-stripe.com`, `stripe.com.evil.io`) does not inherit `tier-1` standing by name
resemblance — the recorded domain is the audit hook, not a trust guarantee.

#### 3. Finding outputter

**File**: `skills/research/outputters/finding-outputter/SKILL.md` (new)
**Changes**: Model on `skills/review/output-formats/*` (`user-invocable: false`,
`disable-model-invocation: true`, no `allowed-tools`, pure static body,
path-passed). Own the finding document contract end to end: the sink (write to the
injected output path, and only that path), the frontmatter shape (`kind: finding`,
`round`, `question`, `source_profile: web`, `status: complete`, plus `date`/author
composed from the injected values — no `revision`/`repository`), with the
typed-linkage slots represented as omit-when-empty (emit a key only with a value;
never `parent: ""`/`relates_to: []`, which the validator rejects). Reference the
`(topic-research, finding)` schema row/template as the field contract rather than
restating it, so the writer, the template, and the schema cannot drift. Own the
`Question`/`Findings`/`Sources` sections, inline tier-tagged sources each carrying
the source domain/venue, and the anti-changelog rule (standalone prose, no round
narration). Name the injected placeholders it expects (output path, round,
question, timestamp, author) so the agent composes rather than invents frontmatter.

#### 4. Register the agent key

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add `"researcher"` to `AGENT_KEYS` so `config agent researcher`
resolves to `accelerator:researcher`; bump the catalogue count assertion and
rename the test.

**File**: `cli/launcher/tests/fixtures/dump/dump.golden`,
`cli/launcher/tests/fixtures/baseline/agents.golden`,
`cli/launcher/tests/fixtures/agents/agents.golden`
**Changes**: Add the `` | `agents.researcher` | `accelerator:researcher` | default | ``
row to `dump.golden`, and a `researcher` line to both `agents.golden` fixtures — the
`config_read` suite asserts all three byte-exact, so a new agent key touches every
one.

### Success Criteria

#### Automated Verification

- [x] Catalogue count test passes: `cargo test -p config the_catalogue_holds`
- [x] Dump golden passes: `cargo test -p launcher config_read`
- [x] The `researcher` agent's `tools:` are pinned and `Bash` is absent, and the finding outputter's declared frontmatter fields equal the `(topic-research, finding)` schema row's required set.
- [x] Full test suite passes: `mise run test`
- [x] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [x] `accelerator config agent researcher` prints `accelerator:researcher`.
- [x] The profile and outputter skills are read-only (`user-invocable: false`); they do not appear as user-invocable in `/` skill listings.

---

## Phase 7: The `research-topic` skill

### Overview

The engine body: a single `SKILL.md` dispatching `brief`, `outline`, `conduct`,
`synthesise` on the `configure`/`comment-jira-issue` hybrid pattern — a shared
parse-and-resolve preamble, then a `### <verb>` section each. `conduct` spawns
`researcher` agents; `synthesise` runs inline. Depends on Phases 2, 4, 5, 6. The
non-`brief` verbs' resolution and the full loop are exercised at the 0278 co-land
(and against live web).

### Changes Required

#### 1. Skill body and dispatch

**File**: `skills/research/research-topic/SKILL.md` (new)
**Changes**: Frontmatter with `argument-hint: "brief SUBJECT | outline SLUG |
conduct SLUG | synthesise SLUG"` and `allowed-tools` as a YAML block sequence
(one `- entry` per line, not an inline comma-separated scalar) scoping `Bash`
narrowly to the specific subcommands the skill runs — `accelerator config` reads,
`accelerator corpus resolve`, `accelerator corpus metadata derive`, and
`accelerator corpus frontmatter validate` — matching the
review-plan/create-plan/implement-plan convention of listing only the scoped
`Bash(accelerator ...)` entries, not a broad `corpus *` wildcard. `Task`, `Read`,
`Write`, `Edit` are ambient and left off the list (no comparable agent-spawning skill
enumerates them; `allowed-tools` auto-approves rather than allowlists).
Injection bookends:
`!accelerator config context --skill research-topic --fail-safe`,
`!accelerator config agents --fail-safe` (with a hardcoded `researcher` default so
`{researcher agent}` resolves), `**Research topics directory**: !accelerator
config path research_topics --fail-safe`, the per-kind template injections
(`!accelerator config template topic-research --kind <kind> --fail-safe`), and the
trailing `!accelerator config instructions research-topic --fail-safe`.

A shared preamble resolves the slug for every verb but `brief` via `accelerator
corpus resolve --type topic-research <slug>`, accepting the tolerant forms (slug,
set directory, sub-document path). Each non-`brief` verb then asserts its
precondition before acting — its expected prior `research_status` (or the presence of
its input: `outline.md` for `conduct`, at least one finding for `synthesise`) — and
refuses with a clear message otherwise, so an out-of-order invocation cannot jump the
`briefed → outlined → researching → synthesised` state machine (e.g. `conduct` before
`outline` writing `researching` over an empty outline).

#### 2. `brief` — interactive scoping and atomic set creation

**Changes**: Interview the user to scope the subject (~3 clarifying questions).
Derive metadata (`accelerator corpus metadata derive`). Before building, **refuse
if the target `meta/research/topics/<slug>/` already exists** — the refusal names the
exact directory and points to the safe recovery (delete it or choose a different
slug; a committed set is recoverable via git), so a re-`brief` never silently
overwrites a prior set (the bare-slug directory name is not collision-free the way
`inventory-design`'s dated names are). Build the set under a dot-prefixed sibling
temp dir (`meta/research/topics/.<slug>.tmp/`), removing any stale `.<slug>.tmp/`
left by an aborted run first so it cannot contaminate the build, then rename it into
`meta/research/topics/<slug>/`, mirroring `inventory-design`, so the indexer's
dot-skipping lister never sees a half-written set; clean up the temp dir on a failed
rename. Write `manifest.md` (`research_status: briefed`, `primary: brief.md`,
counts 0) and `brief.md` (`source_profiles: ["web"]`, base `status: draft` during
scoping, `complete` once authored). Validate both with `corpus frontmatter validate
--file`.

#### 3. `outline` — effort-scaled focus areas under a breadth ceiling

**Changes**: Write `outline.md` with a `## Round 1` checklist of focus areas.
Carry the effort-scaling rubric (1 focus area for a simple subject, 2–4 for
comparisons, more for broad subjects) but state the hardcoded `breadth: 8` as an
**absolute cap that overrides the rubric** — the prompt instructs the model to emit
at most 8 focus areas regardless of the rubric, so the guidance can never instruct a
violation of AC 2/AC 8. The cap is prompt-advisory this slice (a skill cannot count
and truncate the outline an LLM writes); a hard post-write count check — `conduct`
asserting the outline holds ≤ `breadth` focus areas before spawning — is the natural
enforcement point when `breadth` becomes tunable in 0282. Write and validate
`outline.md`, then edit `manifest.md` to `research_status: outlined` as the final
step.

#### 4. `conduct` — one round, one researcher per focus area

**Changes**: Derive the finding metadata once (`accelerator corpus metadata
derive` — timestamp and author; topic-research is not code-state-anchored, so no
`revision`/`repository`). At the start of every run, reconcile against the set on disk — flip the outline
checkbox of any focus area whose finding already exists and validates, and repair a
stale `manifest.md` — so a re-run after a partial or crashed round repairs it rather
than duplicating findings or stalling. For each still-outstanding focus area (all of
them this single-round slice), allocate a path keyed on the focus area's identity,
`findings/<nn>-<slug>.md` (the `<nn>` index scans both `<nn>-*.md` and any
quarantine marker so an index is never reused), and **refuse to write a finding path
that already exists** so an existing immutable finding is never clobbered by a re-run
or an allocation collision. Spawn
`researcher` agents in parallel via the Task tool with `subagent_type:
"!accelerator config agent researcher --fail-safe"`, injecting the profile path
(`${CLAUDE_PLUGIN_ROOT}/skills/research/profiles/web-profile/SKILL.md`), the
outputter path
(`${CLAUDE_PLUGIN_ROOT}/skills/research/outputters/finding-outputter/SKILL.md`),
the focus question, the round number, the derived timestamp/author values, and the
output path. The researcher composes the finding per the outputter from those
injected values and writes it to the output path, returning a short summary rather
than the finding body — no CLI runs in the subagent. After all return, `conduct`
handles three outcomes per focus area. A researcher that wrote **no file** (a
`WebFetch` failure, agent error, or refusal) is reported and left outstanding — its
checkbox unflipped, excluded from `finding_count`, and not routed through the
quarantine rename (there is nothing to rename) — so one dead researcher degrades
gracefully rather than aborting the round. A finding that **fails validation**
(`accelerator corpus frontmatter validate --file`) is **quarantined, not deleted** —
renamed aside to a dot-prefixed, uniquely-suffixed marker (e.g.
`.<nn>-<slug>.md.invalid`, refusing to overwrite an existing marker so an earlier
quarantine is never clobbered) and reported to the user (the research prose is
expensive to reproduce and the defect is usually repairable), with its checkbox left
unflipped. The dot-prefix keeps a quarantined finding inside the indexer's
dot-skipping convention — a bare `.invalid` suffix would not be skipped — and the
committed fixture carries a quarantined-finding case so the 0278 indexer's skip
behaviour is locked by a test. `conduct` then flips only the checkboxes of findings
that validated, and edits `manifest.md` as its final step (after the findings are on
disk and validated) to `research_status: researching`, `round_count: 1`, and
`finding_count` set to the count of **retained, validated** findings — never the raw
focus-area count — so the manifest never overstates what is on disk. Each finding
carries `kind: finding`, `round: 1`, its focus area's `question`, and
`source_profile: web`. Hardcoded `depth: 1` — no intra-finding recursion.

#### 5. `synthesise` — inline dossier from the findings

**Changes**: Read the findings and write `synthesis.md` inline (spawning nothing),
carrying each finding's tiers (and recorded source domains) forward. Because
`synthesise` runs in the main skill context (which holds the broader `Bash`/`Task`
grants) and the findings contain verbatim web excerpts, read the finding bodies as
untrusted data — orientation only, never instructions to follow — extending the
researcher's untrusted-content contract across this second-order boundary, exactly as
`skills/vcs/commit/SKILL.md` wraps injected VCS context; `conduct`'s handling of the
researcher's returned summary carries the same framing. Anti-changelog discipline —
standalone prose, no round narration (`outline.md` remains the exempt working log). Write and validate `synthesis.md` first, then as the final step edit
`manifest.md` to `research_status: synthesised` and flip `primary` to
`synthesis.md`, so a failure before the flip leaves the prior consistent state
(`primary` never points at a file that failed to write).

#### 6. Per-skill context wiring

**Changes**: No plugin file is shipped for `instructions.md`/`context.md` — they
are user-provided under `.accelerator/skills/research-topic/`. The skill only
calls `config context`/`config instructions` with the skill name (bookends above).

#### 7. Set-mutation consistency and deferred hardening

**Changes**: Every mutating verb writes and validates its content *before* editing
`manifest.md`, edits the manifest as its final step, and **re-validates `manifest.md`
after each in-place edit** (`corpus frontmatter validate --file`) — the manifest is
the aggregate root the 0278 indexer keys on, so its in-place status/count/`primary`
edits get the same validate-after-write discipline as every content document, not
just the one validation at `brief` time. `finding_count` is always reconciled to the
findings actually present after any quarantine.

Two hardening items are deliberately **deferred** (recorded here, not built this
slice): a `conduct`-side write-scope assertion — snapshot the set before spawning
and reject a round if any path other than the assigned finding changed, bounding the
researcher's `Write` grant against an injected agent — and cross-verb transactional
atomicity beyond the write-then-flip ordering. The untrusted-content contract
(profile/outputter/synthesise) ships this slice; the write-scope assertion is its
follow-up, and its **accepted compensating control this slice is human commit review
of a git-tracked tree** (a stray write lands in the diff and is reverted with `git
checkout`). Enabling the loop against live web in an unattended/hosted context is
gated on that assertion landing, not merely noted as a follow-up.

### Success Criteria

#### Automated Verification

- [x] The skill frontmatter and `!` lines lint clean under the plugin's markdown/skill checks: `mise run check`
- [x] Full test suite passes: `mise run test` (only the load-sensitive `test:integration:dev` visualiser-server lifecycle tests flake on readiness timeouts — a different subset each run, unrelated to this slice; green on a quiet run)
- [x] The bare default task passes end-to-end: `mise run` (green apart from the same flaky `test:integration:dev` lane)

#### Manual Verification

- [ ] `brief` produces `manifest.md` + `brief.md` with `research_status: briefed`, `primary: brief.md`, and `brief.md` declaring `source_profiles: ["web"]`; both pass `corpus frontmatter validate`.
- [ ] `outline` (at the 0278 co-land, resolving the slug) writes a `## Round 1` checklist whose count is ≤ 8 and sets `research_status: outlined`; a hand-edit of `outline.md` is honoured by `conduct`.
- [ ] `conduct` writes one immutable finding per focus area, each tier-tagged and validate-clean, flips the checkboxes, and sets `research_status: researching`, `round_count: 1`, `finding_count` = focus-area count.
- [ ] `synthesise` writes `synthesis.md` from the findings with tiers carried forward, sets `research_status: synthesised`, and flips `primary` to `synthesis.md`.
- [ ] Exactly one generic `researcher` agent is spawned with the web profile injected; no web-specific agent definition exists.

---

## Testing Strategy

### Unit Tests

- `row_for` composite lookup and `(type, "")` fallback; `validate_file` reading
  `kind`; the template resolver's `<type>-<kind>` → `<type>` fallback (Phase 1).
- The resolver domain: input classification, ambiguous-slug candidate handling,
  and set-root recovery for nested-manifest types (Phase 4).
- Per-kind required extras and status vocab enforcement across the five shapes
  (Phase 5), exercised via `corpus frontmatter validate` fixtures.

### Integration Tests

- TSV/SCHEMA parity (`every_row_matches_templates_schema_tsv`), the length pins,
  and the shipped-templates tree (`template_shape_tree`) after the topic-research
  rows and templates land (Phases 1, 5).
- The config-read goldens (`paths.golden`, `dump.golden`) and catalogue count
  after the path key, template keys, and agent key land (Phases 2, 5, 6).
- Resolver black-box goldens covering the full exit-code taxonomy (0/1/2/3/4/6)
  against `design-inventory` and `codebase-research`, including a deeper
  sub-document, an in-root symlinked path, and the outside-root refusal (Phase 4).
- A hand-authored full set committed as a fixture — `manifest.md`, `brief.md`,
  `outline.md`, a `findings/01-*.md`, and `synthesis.md` — asserting every document
  validates clean (plus the negative per-kind cases), so the artifact contract
  0278's indexer keys on is locked independent of live web and the 0278 co-land
  (Phases 5, 7).
- The `researcher` agent's `tools:` frontmatter is pinned (no `Bash`) and the
  finding outputter's fields match the `(topic-research, finding)` schema row
  (Phase 6).

### Manual Testing Steps

The full `brief → outline → conduct → synthesise` loop depends on live external
web access via WebFetch and on the 0278 doc-type registration for non-`brief` slug
resolution, so end-to-end verification is manual and coupled to the co-land:

1. Co-land 0277 + 0278; run `brief` on a subject and confirm the set renames into
   `meta/research/topics/<slug>/` with `manifest.md` + `brief.md` validate-clean.
2. Run `outline`, hand-edit a focus area, run `conduct`, and confirm one finding
   per focus area with defensible reputation tiers and checkbox reconciliation.
3. Run `synthesise` and read `synthesis.md` against the brief — the Slice 3
   output-quality gate (0280) formally judges dossier quality; here confirm only
   structure, tier carry-through, and the `primary` flip.

## Performance Considerations

Multi-agent rounds are token-heavy (~15× a chat). The hardcoded `breadth: 8`
ceiling and the effort-scaling rubric in the `outline` prompt bound fan-out; the
researcher-writes-finding model keeps `conduct`'s context bounded (short summaries
back, not full finding bodies). `depth: 1` keeps each focus area to a single agent
with no recursion.

## Migration Notes

No data migration. The `(type, kind)` schema change is backward-compatible — the
13 existing types resolve through `(type, "")` unchanged. No `.accelerator/config.md`
edit is required; `paths.research_topics` and the `researcher` agent key resolve
from their catalogue defaults.

⚠️ `kind` is now overloaded: a validated *extra* for work items (story/epic) and
the schema *discriminator* for topic-research. Introducing any future
`(work-item, <kind>)` row — the direction ADR-0067 anticipates — is therefore a
behaviour change, not an addition: every existing `kind: story`/`epic` work item
would switch from resolving via `(work-item, "")` to the new kind-specific row and
be re-validated against its rules. Treat such a row as its own migration with its
own analysis, not a drop-in. Back this with a guard test that fails when a
`(type, kind)` row is introduced for any type whose `kind` also appears in that
type's `extras`, so the migration must be made explicit rather than slipping in as an
apparently additive registry change.

## References

- Work item: `meta/work/0277-single-round-web-research-engine.md`
- Parent epic: `meta/work/0121-topic-research-skillset.md`
- Research: `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md`
- ADR-0067 (kind-discriminated schema/templates): `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`
- ADR-0068 (general slug resolution): `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md`
- Generic-agent precedent: `agents/reviewer.md`, `skills/planning/review-plan/SKILL.md:234-274`
- Resolver mirror: `cli/work/src/resolve.rs`, `cli/work-cli/src/resolve.rs:33-49`
- Schema internals: `cli/corpus/src/frontmatter_validation/schema.rs:4-12,177-179`
- Atomic set write: `skills/design/inventory-design/SKILL.md:245-330`
