---
type: plan
id: "2026-06-06-0067-create-note-skill"
title: "Create create-note Skill Implementation Plan"
date: "2026-06-06T10:39:57+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0067"
parent: "work-item:0067"
derived_from: ["codebase-research:2026-06-06-0067-create-note-skill"]
tags: [skills, notes, templates, frontmatter, typed-linkage]
revision: "f3db668eceb7fcba192290da762578e235859884"
repository: "ticket-management"
last_updated: "2026-06-06T12:35:19+00:00"
last_updated_by: Toby Clemson
schema_version: 1
relates_to: ["work-item:0067", "codebase-research:2026-06-06-0067-create-note-skill", "adr:ADR-0033", "adr:ADR-0034", "adr:ADR-0040"]
---

# Create create-note Skill Implementation Plan

## Overview

Ship two new artifacts — the `templates/note.md` template and the
`skills/notes/create-note/SKILL.md` skill that consumes it — so that
short-form notes become a first-class artifact type with a creator skill
and unified-schema frontmatter. Notes are date-slug-named files under
`meta/notes/` carrying the unified base fields plus `topic` and the
`revision`/`repository` provenance bundle. The skill mirrors
`create-work-item` structurally but is materially simpler: no sequential
allocator, no approval-gated number reservation.

Nothing in the implementation is novel — every shape and convention
already exists as precedent. The real work is assembling those precedents
correctly and wiring each new file into the three data-driven test/manifest
surfaces that gate it.

## Current State Analysis

- **`templates/note.md` does not exist.** All twelve shipped templates
  (`templates/*.md`) share a strict frontmatter spine. The closest
  exemplar is `templates/codebase-research.md` — the only shipped template
  combining a `topic` extra *and* the `revision`/`repository` provenance
  bundle, exactly the note extra-field set.
- **`skills/notes/` category does not exist.** Current categories:
  `config, decisions, design, github, integrations, planning, research,
  review, vcs, visualisation, work`.
- **The `notes` path key is already wired.** `scripts/config-defaults.sh:37,57`
  (`paths.notes` → `meta/notes`); covered by `scripts/test-config.sh:3013-3023`.
  No path config work required.
- **14 existing notes** in `meta/notes/`, all `YYYY-MM-DD-<slug>.md`, all
  free-form (13 with no frontmatter). Their migration is story 0070's
  scope, not this story's.
- **Three data-driven gates already enforce the conventions this story
  must satisfy** (discovered during planning research; not flagged in the
  work item or research doc):
  1. `scripts/test-template-frontmatter.sh` validates each row of
     `scripts/templates-schema.tsv`, **and** cross-checks that the TSV
     template set exactly equals the union of `## Schema Reference` tables
     in the work-item files listed in its `WORK_ITEM_MDS` array (currently
     `meta/work/0065-*.md` and `meta/work/0066-*.md`).
  2. `scripts/test-skill-frontmatter-population.sh` validates each row of
     `scripts/skills-schema.tsv`, **and** runs a discovery pass that greps
     `skills/**/SKILL.md` for template-consumer / emitter patterns
     (including `config-read-template\.sh`) and fails if any surfaced
     SKILL.md is not allow-listed in `IN_SCOPE_PRODUCERS` /
     `NON_EMITTER_TEMPLATE_CONSUMERS`.
  3. `.claude-plugin/plugin.json` carries an explicit `skills` array of
     category directories; `skills/notes/` is absent, so the skill would
     not be discovered (`marketplace.json` has no skill refs and is
     unaffected).

### Key Discoveries:

- Template loading is **file-existence based**, not registry-based:
  `config_resolve_template` (`scripts/config-common.sh:189-228`) Tier 3
  checks `<plugin_root>/templates/note.md` directly. `config-read-template.sh
  note` works the moment the file exists — **no `TEMPLATE_KEYS` registration
  needed**. Six shipped templates (`rca`, `design-gap`, `design-inventory`,
  `plan-review`, `pr-review`, `work-item-review`) are deliberately absent
  from `TEMPLATE_KEYS`; `note` follows that precedent. (Registering it
  would also break the order-locked assertion at
  `scripts/test-config.sh:2461`.)
- The template test's `SOURCE_TYPE_RE`
  (`scripts/test-template-frontmatter.sh:~75`) does **not** include `note`.
  So the note template's `parent` / `relates_to` comments must use a
  curated source-type token — `work-item:NNNN` — not own-type `note:`.
  This settles research Open Question #3.
- `artifact-derive-metadata.sh` emits `Current Date/Time (UTC)`,
  `Current Revision`, `Repository Name` — supplying `date`, `revision`,
  `repository`, and the date portion for the filename.
- `create-work-item` aborts if the target path already exists
  (`SKILL.md:427-432`) to catch a concurrent allocator race. `create-note`
  has no allocator and no such race, so instead of aborting it
  auto-disambiguates the slug on collision and writes (resolves research
  Open Question #4).

## Desired End State

A user can invoke `create-note`, be interactively prompted for the note's
topic, body, and optional tags (and optionally a related artifact), and
get a new `meta/notes/YYYY-MM-DD-<topic-slug>.md` file whose frontmatter
conforms to the unified schema: base fields + `topic` + provenance bundle,
with `relates_to`/`parent` populated only when a linkage was named and
confirmed, and with `source`/`derived_from` never emitted.

Verification: `mise run test:unit:templates` passes (covering both the
template-shape and skill-population schema tests); the skill is registered
and routes from the three test phrases; a live run produces a conformant
note.

### Decisions locked for this plan (no open questions remain)

- **Status vocabulary**: single `captured` (inline comment `# captured`),
  mirroring research's single-value `complete`.
- **Author/provenance style**: research-style — `author: "{author from VCS}"`
  + `revision`/`repository` provenance bundle + quoted `last_updated_by`.
- **Linkage comment tokens**: `work-item:NNNN` (curated set requirement).
- **Path-collision handling**: auto-disambiguate the slug on collision
  (append a short qualifier) and write, rather than aborting — a deliberate
  departure from `create-work-item`, whose abort guards a concurrent
  allocator race notes do not have.
- **No `TEMPLATE_KEYS` registration** for `note`.

## What We're NOT Doing

- **No `list-notes` / `show-note` skills** — explicitly deferred by the
  story (create-only is consistent with research/ADR/plan surfaces).
- **No notes allocator script** — no `*-next-number.sh`; filenames are
  date-slug.
- **No migration of the 14 existing free-form notes** — that is story 0070.
- **No new path config** — `paths.notes` is already wired and tested.
- **No `templates.note` entry in `TEMPLATE_KEYS`** (`config-defaults.sh`)
  and no `templates.note` row in the configure-skill docs — consistent
  with the six already-unregistered templates.
- **No edits to the 0065/0066 Schema Reference tables** — 0067 carries its
  own table (0065 explicitly excludes `note`).
- **No `source`/`derived_from`/`blocks`/`blocked_by` slots** in the note
  template — notes are the extraction-origin end of the graph and never
  own those keys.
- **No change to `SOURCE_TYPE_RE`** — `note` is not added to the curated
  linkage source-type set (no template in scope points *to* a note).

## Implementation Approach

Test-driven, two phases, each leaving `main` green and independently
mergeable in order. Within each phase the test-data / manifest edits land
first (turning the relevant schema test red against the not-yet-written
artifact), then the artifact is authored to turn it green.

Phase 1 (template) is fully self-contained: the template can ship and pass
its shape test with no consumer (exactly as `rca.md` does). Phase 2 (skill)
adds the consumer and its registration; it depends on Phase 1's template at
runtime, so it merges after Phase 1.

---

## Phase 1: `templates/note.md` and its shape-test wiring

### Overview

Introduce the note template and the schema-test machinery that pins its
shape, TDD-style: wire the expected shape into the data-driven test first
(red), then author the template (green).

### Changes Required:

#### 1. Template shape row

**File**: `scripts/templates-schema.tsv`
**Changes**: Append one tab-separated row (7 fields, matching the header
`template / type / code_state_anchored / extras / status_vocab /
forbidden_own_id_key / typed_linkage_keys`):

```
note.md	note	yes	topic	captured	-	parent relates_to
```

The row is **TAB-separated** into exactly 7 columns (the final column holds
the space-joined `parent relates_to`). A space-for-tab slip makes the row's
field count ≠ 7, which aborts the entire suite at the `awk -F'\t' 'NF != 7'`
field-count self-check before any per-row assertion runs — a confusing
failure during the red phase. After appending, sanity-check with
`awk -F'\t' 'END{print NF}' scripts/templates-schema.tsv` (expect `7`).

- `code_state_anchored = yes` → provenance bundle `revision`/`repository`
  required and asserted.
- `extras = topic` → `topic:` line required.
- `status_vocab = captured` → the `status:` line must contain `captured`
  verbatim.
- `forbidden_own_id_key = -` → no legacy own-id key to forbid.
- `typed_linkage_keys = parent relates_to` → only these two linkage slots,
  each validated for exact shape+comment grammar; the closed-set check then
  rejects any other linkage-vocabulary key appearing in the block.

#### 2. 0067 Schema Reference table (satisfies the cross-check)

**File**: `meta/work/0067-create-note-skill.md`
**Changes**: Append a `## Schema Reference` section (body-only; frontmatter
untouched) carrying a markdown table whose single template row matches the
awk extractor in `test-template-frontmatter.sh` (row shape
`| \`<file>.md\` | ... |`). The first cell's exact lexical form is
load-bearing: a lowercase, backtick-wrapped `note.md` with at least one
space after the leading pipe. Any later edit that drops the backticks,
uppercases a letter, or removes the leading space silently drops the row
from the cross-check union and fails it with a confusing "templates differ"
message rather than an obvious table error.

```markdown
## Schema Reference

The note template emits the unified base schema plus the `topic` per-type
extra and the `revision`/`repository` provenance bundle per ADR-0033, and
the `parent`/`relates_to` typed-linkage slots per ADR-0034, emitted
omit-when-empty per ADR-0040. Authoritative source: ADR-0033 and ADR-0034,
with omit-when-empty emission per ADR-0040; on any discrepancy the ADRs win
and this table should be re-synced. This table is mirror data validated by
`test-template-frontmatter.sh`, but its cross-check compares only the
template-filename cell — the `type` / `schema_version` / provenance / extras
columns are unverified by any test and must be checked by hand against the
template and the ADRs during manual verification.

| Template file | Artifact `type` | `schema_version` | Provenance bundle? | Per-type extras (beyond base) |
|---|---|---|---|---|
| `note.md` | `note` | 1 | yes | `topic` |
```

#### 3. Wire 0067 into the cross-check source set

**File**: `scripts/test-template-frontmatter.sh`
**Changes**: Add the 0067 work-item path to the `WORK_ITEM_MDS` array so
its Schema Reference table joins the authoritative union:

```bash
WORK_ITEM_MDS=(
  "meta/work/0065-update-artifact-templates-to-unified-schema.md"
  "meta/work/0066-update-review-skills-inline-frontmatter.md"
  "meta/work/0067-create-note-skill.md"
)
```

Also update the stale comment immediately above the array (currently
`scripts/test-template-frontmatter.sh:24-25`, "both 0065 and 0066 carry
Schema Reference tables; …") so it no longer contradicts the now-three-entry
array. **Prefer the generalised wording** — "each listed work item carries a
Schema Reference table; the union must match the TSV exactly" — rather than
re-enumerating "0065, 0066, and 0067", so the comment stays accurate as the
array grows and does not re-create the very staleness this edit fixes.

#### 4. The note template

**File**: `templates/note.md` (new)
**Changes**: Author the frontmatter spine following the
`codebase-research.md` exemplar, with note-specific values. Final shape:

```markdown
---
type: note                                   # artifact-type discriminator
id: "{filename-stem}"                         # filename without .md
title: "{Note title}"
date: "{ISO timestamp from artifact-derive-metadata.sh}"
author: "{author from VCS}"
producer: create-note
status: captured                             # captured
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                               # typed-linkage list: ["work-item:NNNN", ...] or []
topic: "{Note topic}"
tags: []
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# {Note title}

{The note's body — a short-form observation, insight, or strategy snippet.}
```

Notes on shape:
- **Author/provenance combination (deliberate, precedent-setting).** Notes
  are the first artifact type that is both author-authored *and*
  provenance-bearing. The template follows the research-template convention —
  VCS-derived quoted `author: "{author from VCS}"` (and matching
  `last_updated_by`) alongside the `revision`/`repository` bundle — because
  carrying provenance signals a code-state anchor, which by convention pairs
  with the VCS-derived author. Record this rationale so the next code-state-
  anchored author-authored type copies the pairing as a principle, not an
  accident.
- No `work_item_id` foreign-reference slot. This is a deliberate, ADR-grounded
  departure from the `codebase-research.md`/`rca.md` exemplars (which carry
  `work_item_id: ""`): ADR-0033 lists no foreign-reference extra for the note
  type, notes link via `parent`/`relates_to`, and the closed-set check governs
  only linkage-vocabulary keys, so the omission passes the shape test. The
  0067 Schema Reference / drafting notes should state the absence is
  intentional so a future reader comparing against the exemplars does not read
  it as an oversight.
- The two linkage comments use the `work-item:NNNN` token (curated-set
  requirement), single-ref grammar for `parent`, list grammar for
  `relates_to` — matching `check_linkage_slot`'s regexes exactly. Note this
  forces the `relates_to` example to show `work-item:NNNN` even though
  note→note is a plausibly-common loose linkage: the own-type `note:` token is
  rejected by `SOURCE_TYPE_RE`. The deeper reason note→note must use the
  path-form reference is that notes are date-slug-named and carry no stable
  `doc-type:id` (no `NNNN`), so per ADR-0034 the path form is the canonical
  channel for any note *target* regardless of `SOURCE_TYPE_RE` — adding `note`
  to the curated set would not give note→note a typed token. Accept the
  constraint (it is the same one driving the curated-token choice).
- **Linkage direction is intentional.** A note carrying `parent`/`relates_to`
  *to* a work-item is the inverse of ADR-0034's canonical work-item→note
  extraction edge (where the note is the `source`/`derived_from` target). This
  is permitted and intended: `parent` is a corpus-wide ownership key any type
  may carry, and `relates_to` is the loose catch-all; the canonical extraction
  edge (`source`/`derived_from`) is owned by the *extracting* artifact, not
  the note (which is exactly why the note never writes those keys — AC7).
- `schema_version: 1` is a bare integer and stays last.
- **Author the frontmatter by copying `codebase-research.md`'s block verbatim
  and editing values in place** rather than retyping it. Comment-column
  alignment is hand-maintained and not covered by `shfmt` (markdown
  frontmatter is outside its scope), so copy-and-edit preserves alignment
  mechanically; the only manual check is then that the *values* are correct.

### Success Criteria:

#### Automated Verification:

- [x] Template-shape test passes: `bash scripts/test-template-frontmatter.sh`
- [x] Full template/skill schema suite passes: `mise run test:unit:templates`
- [x] Template resolves through the loader:
      `bash scripts/config-read-template.sh note` prints the note template
      wrapped in `markdown` fences and exits 0
- [x] Shell format/lint clean for the edited test script:
      `mise run format:check` and `mise run lint:check`

#### Manual Verification:

- [x] `templates/note.md` frontmatter visually matches the
      `codebase-research.md` spine (field order, comment style, quoting).
- [x] The 0067 Schema Reference table reads accurately and does not
      contradict ADR-0033/0034.

---

## Phase 2: `skills/notes/create-note/SKILL.md`, its population-test wiring, and registration

### Overview

Author the skill, wire it into the skill-population schema test (red →
green), and register the new `skills/notes/` category so Claude Code
discovers it.

### Changes Required:

#### 1. Skill population row

**File**: `scripts/skills-schema.tsv`
**Changes**: Append one tab-separated row (4 fields, matching the header
`skill_path / producer_name / fields_to_assert / omit_when_empty`):

```
skills/notes/create-note/SKILL.md	create-note	producer schema_version last_updated last_updated_by revision repository	parent relates_to
```

- `fields_to_assert` includes the provenance fields (`revision repository`)
  because the note type is code-state-anchored — matching the
  `research-codebase` row.
- `omit_when_empty = parent relates_to` → the SKILL.md must carry a
  per-field fill/omit guidance bullet for each (the
  `in_populate_section_with_guidance` check).

#### 2. Allowlist the skill for the discovery pass

**File**: `scripts/test-skill-frontmatter-population.sh`
**Changes**: Add the skill path to `IN_SCOPE_PRODUCERS`:

```bash
IN_SCOPE_PRODUCERS=(
  ...
  skills/work/refine-work-item/SKILL.md
  skills/notes/create-note/SKILL.md
)
```

Append this entry **last**, so it stays row-aligned with the
`skills-schema.tsv` append (both are the new final row) — the two
hand-maintained lists are kept positionally diffable, so preserve that
correspondence.

(The discovery `comm -23` only flags discovered-but-not-allowlisted, so
adding *this allowlist line* before the SKILL.md exists is harmless. Note
the asymmetry: the `skills-schema.tsv` row loop instead fails hard if the
SKILL.md is absent, so the TSV row and the SKILL.md are an inseparable unit
— they must land together within Phase 2 and the phase must not be
subdivided across a green-`main` boundary.)

#### 3. Register the new skill category

**File**: `.claude-plugin/plugin.json`
**Changes**: Add `"./skills/notes/"` to the `skills` array (do not touch
`version` or any other field):

```json
  "skills": [
    ...
    "./skills/design/",
    "./skills/notes/",
    "./skills/config/"
  ]
```

Two deliberate choices to record:
- **Placement**: the `skills` array has no enforced ordering (it is neither
  alphabetical nor strictly pipeline-ordered), and discovery is by directory
  presence, not order — so inserting `notes` before `config` is functionally
  fine. The choice is deliberate (config stays last); if a future reader
  prefers grouping `notes` with its conceptual siblings (`research`/`work`),
  that is an equally valid placement.
- **Single-skill category**: `skills/notes/` initially hosts only
  `create-note`. This matches the plugin's per-artifact-family grouping
  convention (and the plugin already nests where a family subdivides, e.g.
  `skills/integrations/jira/`); the category is provisioned for the notes
  artifact family, leaving room for the deferred `list-notes`/`show-note`
  skills, so the single-skill state is an intentional waypoint, not an
  accident.

#### 4. The skill

**File**: `skills/notes/create-note/SKILL.md` (new)
**Changes**: Author the skill, modelled on `create-work-item` but
simplified. Required elements:

**Frontmatter** — `name`/`description`/`argument-hint`/`allowed-tools`.
The `description` must contain the routing keywords ("note" + "capture"/
"jot") per AC9. Lean the description on the note-distinctive verbs
("jot"/"note") and keep "short-form note in `meta/notes/`" prominent: the
bare word "capture" also appears in `create-work-item`'s description
("capturing a … work item"), so the note→work-item disambiguation should
rest on "note"/"jot" rather than "capture" alone. `allowed-tools` needs only
the config glob (no work-item scripts dir); the `artifact-*` glob's only
consumer is `artifact-derive-metadata.sh`, and its breadth is intentional
house-style (matching `create-work-item`), not an oversight:

```yaml
---
name: create-note
description: Interactively capture a short-form note. Use when jotting down
  an observation, insight, or strategy snippet as a short-form note in
  meta/notes/ — e.g. "make a note of this", "jot this down", "capture a note".
argument-hint: "[note topic]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)
---
```

**Body** — header config injections, then numbered steps:
- Top-of-file injections mirroring sibling skills:
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh``,
  `config-read-skill-context.sh create-note`,
  `config-read-agents.sh`, the notes-directory resolution
  `**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes``,
  and the template load
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh note``.
- **Parameter check** (mirror `create-work-item`'s Step 0, minus the
  resolver machinery): the skill declares `argument-hint: "[note topic]"`, so
  define both paths explicitly.
  - **Argument provided** — treat the argument as the note's topic: slug it,
    skip the topic prompt, and proceed straight to eliciting body + optional
    tags + optional linkage. This is the natural quick-capture path the
    `argument-hint` advertises; re-asking for a topic the user already typed
    would defeat the in-the-moment purpose.
  - **No argument** — print a short defined greeting inviting the topic,
    e.g. *"What would you like to note? Give me a short topic and the note
    itself; tags and a related work item/plan are optional."* Then wait.
- **Elicitation**: solicit the note's **topic** (unless supplied as the
  argument), **body content**, and **optional tags** (AC4), plus optionally a
  related work-item/plan for linkage (AC6), in a single compact prompt with
  tags and linkage clearly marked optional — aim for one round-trip. Keep it
  short: no multi-agent research flow. In the no-argument case, the greeting
  above **is** this single elicitation prompt (it already solicits topic +
  body + optional tags + optional linkage) — do not issue a second prompt
  before the user replies. (The precedent greeting/prompt scaffolding is
  `create-work-item/SKILL.md:135-145` and `research-codebase/SKILL.md`
  Initial Setup.)
- **Linkage handling**: when the user names a related artifact, record it
  under `relates_to` by default; record it under `parent` **only** when the
  user confirms that artifact *owns* the note. Never infer ownership. Use a
  neutral, jargon-free confirmation prompt that explains the distinction in
  user terms and defaults to `relates_to` on any non-affirmative or unclear
  answer — e.g. *"Does <artifact> own this note as its parent, or is it just
  related? [owns / related]"* (default: related). Do **not** use a leading
  *"Is this the parent? (y/n)"* form, which biases toward yes and presumes
  the user knows `parent` semantics. Never write `source` or `derived_from`
  (AC7).
- **Filename derivation**: `<notes_dir>/YYYY-MM-DD-<topic-slug>.md`, date
  from `artifact-derive-metadata.sh`, slug = meaningful kebab summary of the
  topic (not raw input — follow `create-work-item`'s slug rule for the
  normalisation/condensation; an empty or degenerate topic must not yield an
  empty slug). **Path-collision handling**: if the target path already
  exists (a same-day, same-slug note), auto-disambiguate using a **single
  deterministic strategy** — append an incrementing numeric suffix and
  **probe until the first free path**: try `<slug>-2.md`, then `<slug>-3.md`,
  … testing existence at each step, and write to the first index that does
  not already exist. Do **not** use a "distinguishing token" or any
  non-deterministic qualifier — that would read as the random suffix the
  deterministic-path contract (work-item AC7: "no random suffix") forbids,
  and could itself collide. This auto-disambiguation writes rather than
  aborts because same-day same-topic capture is an everyday, non-concurrent
  occurrence for notes, and a hard abort would discard the just-elicited
  body. (This is the one place create-note deliberately *departs* from
  `create-work-item`'s abort-on-collision guard, because that guard exists to
  catch a concurrent allocator race that notes do not have.)
- **Populate frontmatter** section. The population test
  (`test-skill-frontmatter-population.sh`) imposes exact structural
  conditions, so author this section to satisfy them — the precedent that
  passes them is `create-work-item`'s Step 5:
  - **Heading**: use a real `#`-prefixed heading matching the test's
    `POPULATE_HEADING_RE` (e.g. `### Populate frontmatter`). The
    `in_imperative_section` helper only treats a line as starting a section
    inside its `/^#/` branch — a bold lead-in (`**Populate…**`) is *not* a
    heading and will not arm the section, so the heading must be a genuine
    `#` heading.
  - **Imperative verb + colon-anchored fields**: within that section, render
    every `fields_to_assert` entry (`producer`, `schema_version`,
    `last_updated`, `last_updated_by`, `revision`, `repository`) as a
    colon-anchored reference — i.e. as a `` `field:` `` token (e.g.
    `` - `revision:` ← … ``) — alongside an imperative verb
    (Substitute / Populate / Set / Write / Emit) in the same `#`-headed
    section. Prose like "populate revision and repository" without the
    colon-anchored `field:` tokens will *not* satisfy `in_imperative_section`
    and the test will fail. Note `last_updated` needs its **own**
    `` `last_updated:` `` token: a `` `last_updated_by:` `` token does **not**
    satisfy the `last_updated` assertion (the matcher anchors on
    `last_updated:`, and `last_updated_by:` has `_by` before the colon). (As a
    belt-and-braces alternative, the same fields may instead appear as
    `^field:` lines inside a non-template fenced YAML block, which
    `in_fenced_block` also accepts.)
  - Also substitute the remaining fields the same way: `type` (`note`),
    `id` (filename stem, quoted), `title`, `date`, `author`, `status`
    (`captured`), `topic`, `tags`, `schema_version` (`1`).
  - **Per-field fill/omit bullets** for the omit-when-empty keys (required by
    the `in_populate_section_with_guidance` check). Each key must be its
    **own top-level bullet** (the check binds the keyword to each field's own
    bullet window, opened at the `` `parent:` ``/`` `relates_to:` `` bullet
    and closed at the next bullet/heading/EOF) and must carry a **standalone
    whole-word, lowercase** `fill` or `omit` on a line that does **not** begin
    a new list item (the check matches `fill`/`omit` only as whole words and
    **case-sensitively** — a buried substring like "backfill" is rejected, and
    capitalised `Fill`/`Omit` does **not** count, so use the lowercase forms
    verbatim; a merged bullet covering both keys fails for one of them):

  ```markdown
  - `parent:` ← the owning artifact as a typed-linkage ref
    (`"work-item:NNNN"`); fill only when the user confirms that artifact
    owns the note, otherwise omit the key.
  - `relates_to:` ← list of typed-linkage refs to related artifacts
    (`["work-item:NNNN", ...]`); fill when the user names a related
    artifact, otherwise omit the key.
  ```

  (Both keywords above are lowercase `fill`/`omit` verbatim, per the
  case-sensitive matcher — do not capitalise them.)

  Plus an explicit statement that `source`/`derived_from` are never written
  by this skill — phrased in the colon-less backtick form
  (`` `source` ``/`` `derived_from` ``), never the colon-anchored
  `` `source:` ``/`` `derived_from:` `` form, so it does not trip the AC7
  negative grep (which forbids that colon-backtick token anywhere in the
  file, including cautionary prose). Do **not** cite ADR numbers inline in the
  SKILL.md — no sibling producer skill (`create-work-item`,
  `research-codebase`) carries ADR numbers, and matching that convention keeps
  the skill consistent; the omit-when-empty authority (ADR-0040) is cited
  where ADRs belong, in the 0067 Schema Reference table prose.

  The test-mechanics explanations above (the `#`-heading requirement, the
  colon-anchored-token and whole-word-keyword rules, the binding windows) are
  **authoring guidance for this plan only** — they must **not** be transcribed
  into the SKILL.md. The shipped skill carries a clean `#`-headed Populate
  section with the field substitutions and the per-field bullets (as in the
  markdown sample above), matching sibling skills' lean style, without the
  parenthetical explanations of why each form is required.
- **Write step**: create the notes directory if absent, write the file,
  print a confirmation line in the precedent's format so the path is directly
  actionable — e.g. `Note created: <notes_dir>/YYYY-MM-DD-<slug>.md`
  (mirroring `create-work-item`'s `Work item created: {path}`). The
  confirmation **must print the literal final path actually written** — so on
  a disambiguated write it shows the qualified slug (`…-2.md`), not the
  topic-derived slug — and must signal that the slug was adjusted, ideally
  pointing at the pre-existing note so both files are actionable, e.g.
  `Note created: <final-path> (an earlier note on this topic exists at
  <first-path>; this one was written as <slug>-N.md)`, so the user is never
  surprised by a silently different filename. An accumulating `-2`, `-3`, …
  family for a repeatedly-used broad topic is expected and acceptable — do
  **not** add a cap or an extra prompt that would reintroduce friction into
  the quick-capture path.
- Close with the standard
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh create-note``
  injection.

### Success Criteria:

#### Automated Verification:

- [x] Skill-population test passes:
      `bash scripts/test-skill-frontmatter-population.sh` (row asserts +
      discovery pass green)
- [x] Full template/skill schema suite passes: `mise run test:unit:templates`
- [x] `plugin.json` is valid JSON and lists `./skills/notes/`:
      `jq -e '.skills | index("./skills/notes/")' .claude-plugin/plugin.json`
- [x] Routing keywords live in the **`description:` value** (the AC8 routing
      surface), not merely elsewhere in the frontmatter or body. The grep must
      isolate the `description:` line and its indented YAML continuation lines
      — scoping only to the frontmatter block is **not** sufficient, because
      `name: create-note` always contains the substring "note" and would make
      a whole-frontmatter `grep 'note'` pass unconditionally. Extract the
      description value, then grep:
      `awk '/^description:/{d=1;print;next} d&&/^[[:space:]]/{print;next} {d=0}' skills/notes/create-note/SKILL.md | grep -Eiq 'note'`
      and the same `awk … | grep -Eiq 'capture|jot'`. (These are presence
      floors backstopped by the AC10 manual dispatch test, which is the
      load-bearing routing verification.)
- [x] Collision-handling regression floor — the skill documents
      auto-disambiguation, not abort:
      `grep -Eiq 'disambiguat' skills/notes/create-note/SKILL.md`
      (a cheap standing guard that the deliberate departure from
      `create-work-item`'s abort-on-collision behaviour is not silently
      reverted; the full behaviour is exercised by the manual collision step).
- [x] AC7 regression net — the skill carries no `source:`/`derived_from:`
      population instruction:
      `! grep -Eq '\`(source|derived_from):\`' skills/notes/create-note/SKILL.md`.
      For this to stay clean, the prohibition statement must use the
      colon-less backtick form (`` `source` ``/`` `derived_from` ``), never
      the colon-anchored `` `source:` ``/`` `derived_from:` `` populate-field
      form — only populate-field references carry the trailing colon.
- [x] Shell format/lint clean: `mise run format:check`, `mise run lint:check`

#### Manual Verification:

- [ ] `create-note` appears in the available-skills listing alongside
      `create-work-item` (re-list skills after registration). *(Pending: needs
      the plugin reloaded/reinstalled — the new skill lives in this workspace,
      not the running harness's loaded plugin cache.)*
- [ ] Each of "capture a note", "jot this down", and "make a note of this"
      routes to `create-note` on a single dispatch against the full enabled
      skill set (AC10). *(Pending: routing dispatch requires the skill
      registered in the running harness — not testable until the plugin is
      reloaded.)*
- [x] A live run elicits topic + body + tags and writes
      `meta/notes/YYYY-MM-DD-<topic-slug>.md` with base fields, `topic`,
      and provenance populated; the body and `tags` reflect the supplied
      values (AC3, AC4, AC5). *(Verified by executing the skill's documented
      procedure against the real `config-read-path.sh`,
      `artifact-derive-metadata.sh`, and `note.md` template; the written note
      was conformant. Test artifact removed afterwards.)*
- [x] Naming a related work-item records it under `relates_to`; confirming
      ownership moves it to `parent`; unused linkage slots are omitted, not
      emitted empty; `source`/`derived_from` never appear (AC6, AC7).
      *(Verified via the same dry-run: declined-ownership note carried
      `relates_to`, confirmed-ownership note carried `parent`, neither emitted
      the unused slot nor `source`/`derived_from`.)*
- [x] The emitted slug is a meaningful condensed kebab summary of the topic,
      distinct from a naive passthrough of the raw input (AC5).
- [x] On a same-day same-slug collision the skill auto-disambiguates the slug
      and writes — it neither aborts nor overwrites; the pre-existing note is
      left intact and the new note lands at the disambiguated path. *(Verified:
      re-running the same topic probed to `…-behaviour-2.md` with the first
      note left intact.)*

---

## Testing Strategy

### Unit Tests (schema/shape):

- `scripts/test-template-frontmatter.sh` — note template row + cross-check
  union. Drives Phase 1 TDD (red before the template exists / before its
  shape matches; green after).
- `scripts/test-skill-frontmatter-population.sh` — create-note row +
  discovery allowlist. Drives Phase 2 TDD.
- Both run together via `mise run test:unit:templates` (also runs
  `test-metadata-helpers.sh`).
- The negative-fixture self-tests inside both scripts already prove the
  assertions can reject bad input; no new self-test fixtures are required
  for the note row (it reuses the existing pure-function checks).
- **AC7 skill-prose net** (added in Phase 2 Success Criteria): a grep-based
  negative check asserts the SKILL.md contains no colon-anchored
  `source:`/`derived_from:` populate reference. This complements the
  template-level enforcement (the closed-set check rejects those slots in the
  template) by also guarding the *skill* against a future edit that adds
  populate guidance for them — closing the gap where AC7's skill-prose half
  was otherwise manual-only.

### Integration / Loader:

- `bash scripts/config-read-template.sh note` — confirms three-tier
  resolution finds the new plugin-default template.
- `bash scripts/config-read-path.sh notes` — confirms the output directory
  (already covered by `test-config.sh`, re-checked as a sanity step).

### Manual Testing Steps:

The skill's remaining behavioural contracts — ownership-driven
`relates_to`-vs-`parent` placement (AC6), routing dispatch (AC10), slug
quality (AC5), and the collision auto-disambiguation — are inherently hard to
machine-check (live elicitation, LLM routing, judgemental slug quality) and
are covered by the steps below. These execute **once at merge** and are not
re-run by CI, so they are merge-time verification rather than a standing
regression net; treat a later edit to the elicitation/linkage/slug logic as
requiring a manual re-run.

1. Run `create-note`; supply a topic, a multi-line body, and two tags.
   Confirm the written file path, frontmatter, body, and `tags`.
2. Re-run and name a related work item; decline ownership → expect
   `relates_to`. Re-run, confirm ownership → expect `parent`. In both,
   confirm `source`/`derived_from` are absent.
3. Re-run with the same topic on the same day → expect the slug to be
   auto-disambiguated (a short qualifier appended) and a second note written,
   with the first note left intact (no abort, no overwrite).
4. Issue each of the three routing phrases in a fresh turn, evaluated against
   the full enabled skill set (which includes `create-work-item`); confirm
   single-dispatch routing to `create-note` with no collision against
   `create-work-item`.

## Performance Considerations

None. Both artifacts are static markdown; the skill performs only local
file reads/writes and config-script invocations.

## Migration Notes

None for this story. The 14 existing free-form notes are untouched; their
treatment is story 0070's decision. The new convention applies only to
notes created by `create-note` going forward.

## References

- Original work item: `meta/work/0067-create-note-skill.md`
- Related research: `meta/research/codebase/2026-06-06-0067-create-note-skill.md`
- Template exemplar: `templates/codebase-research.md:1-20`
- Skill precedent: `skills/work/create-work-item/SKILL.md` (frontmatter
  `:1-9`; field substitution `:443-487`; omit-by-default linkage `:458-484`;
  path-existence guard `:427-432`)
- Template loader: `scripts/config-common.sh:189-228`
  (`config_resolve_template`), `scripts/config-read-template.sh`
- Template shape test: `scripts/test-template-frontmatter.sh`,
  `scripts/templates-schema.tsv`
- Skill population test: `scripts/test-skill-frontmatter-population.sh`,
  `scripts/skills-schema.tsv`
- Skill registration: `.claude-plugin/plugin.json` (`skills` array)
- Path config: `scripts/config-defaults.sh:37,57`
- Metadata helper: `scripts/artifact-derive-metadata.sh`
- Decisions: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`,
  `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`,
  `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`
