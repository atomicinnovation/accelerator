---
date: "2026-05-30T01:30:00+00:00"
type: plan
skill: create-plan
work_item_id: "0065"
status: ready
---

# Update All Artifact Templates to Unified Schema Implementation Plan

## Overview

Bring every artifact template under `templates/` and every non-review producer
that touches them into line with **ADR-0033** (unified base frontmatter) and
**ADR-0034** (typed linkage vocabulary), so that artifacts produced by the
Accelerator plugin are born unified rather than waiting on the corpus
migration (0070). The work is decomposed into one foundation phase
(test scaffolding), one independent infrastructure phase (metadata helpers),
eight independent template-family phases, and a closing discovery phase. Every
phase follows test-driven development: tests are added first and start RED;
implementation moves them to GREEN.

## Current State Analysis

The codebase has nine in-scope templates (eight existing with idiosyncratic
frontmatter, one — `templates/validation.md` — body-only with no frontmatter
block at all), three metadata helper scripts that emit a non-ISO timestamp
shape and a `GIT_COMMIT` key (ADR-0033 mandates ISO `+00:00` and `REVISION`),
and ten consuming SKILL.md files, nine of which do not instruct the model to
populate the new mandatory base fields (`producer`, `schema_version`,
`last_updated`, `last_updated_by`). Of the ten consuming skills, two are
"hybrid" producers (`describe-pr` and `validate-plan`) that emit inline
frontmatter on top of the template — their inline emission overrides the
template's block. `describe-pr` is in this story's scope; `validate-plan`
belongs to 0066.

The corpus migration (0070) handles existing `meta/` files; this story
explicitly does not touch `meta/`.

## Desired End State

After this plan is complete:

- Each of the nine in-scope templates emits the unified base fields and (where
  applicable) the provenance bundle, per the Schema Reference table in
  `meta/work/0065-update-artifact-templates-to-unified-schema.md`.
- The three metadata helper scripts emit ISO `+00:00` timestamps, a
  `REVISION` key (not `GIT_COMMIT`), and no `GIT_BRANCH`.
- Each consuming SKILL.md instructs the model to populate every mandatory base
  field, plus (for code-state-anchored producers) the provenance bundle.
- `describe-pr`'s inline frontmatter is rewritten to align with the unified
  template, so the template's fields actually reach disk.
- `extract-adrs/SKILL.md` reads `templates/adr.md` explicitly via
  `config-read-template.sh adr`, removing the prose indirection through
  `create-adr/SKILL.md`.
- A reproducible producer-discovery pass is recorded in the plan/work-item;
  re-running its grep produces the same set, which excludes exactly the four
  review/validation skills owned by 0066 and confirms no other inline
  producer needs work.
- Three new test scripts under `scripts/` encode the schema contract and pass:
  `test-template-frontmatter.sh`, `test-skill-frontmatter-population.sh`,
  `test-metadata-helpers.sh`. They are wired into a new mise task
  `test:unit:templates`.

Verification: `mise run test:unit:templates` passes, and the recorded
discovery-pass grep produces the recorded output verbatim.

### Key Discoveries:

- Authoritative schema is ADR-0033 (`meta/decisions/ADR-0033-unified-base-frontmatter-schema.md:109-219`); linkage vocabulary is ADR-0034 (`meta/decisions/ADR-0034-typed-linkage-vocabulary.md:44-108`).
- Template loading is centralised through `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh <name>` (`scripts/config-read-template.sh:53-60`); template edits propagate to consumers without code changes — only SKILL.md prose changes are needed to instruct substitution of new dynamic fields.
- Three independent metadata helpers exist with overlapping output schemas: `scripts/artifact-derive-metadata.sh:5-25`, `skills/design/inventory-design/scripts/inventory-metadata.sh:10-34`, `skills/design/analyse-design-gaps/scripts/gap-metadata.sh:10-34`. All three need the same edits.
- `describe-pr/SKILL.md:99-110` emits inline frontmatter that overrides the loaded `templates/pr-description.md`. Without prose edits there, the unified template's fields never reach disk.
- `extract-adrs/SKILL.md:175-178` reads the ADR template by prose reference to the create-adr skill; an explicit `config-read-template.sh adr` invocation tightens this.
- Per-type status vocabularies are unchanged by this story (vocabulary unification is explicitly out of scope per 0057). Implementer reproduces each template's existing valid-status set verbatim in the status comment.

## What We're NOT Doing

- Editing existing `meta/` artifacts to bring them into line with the new
  schema — corpus migration is 0070's exclusive territory. This includes
  ADR-0033's own frontmatter, which still uses `adr_id:`; that gets
  rewritten by 0070. Two narrow exceptions are accepted within this story
  (both explicitly scoped):
  - **Phase 1's template-shape test driver READS the Schema Reference
    table in `meta/work/0065-...md`** as a cross-check input. This is a
    read-only operation; the file is not modified.
  - **Phase 11 APPENDS a "Discovery Pass Record" section** to the same
    work-item file. This is an additive write to a `meta/` file that is
    itself the source for 0065's requirements, not a corpus-migration
    rewrite of frontmatter.
  These carve-outs do not extend to any other `meta/` file.
- Creating the three review templates (`plan-review.md`, `work-item-review.md`,
  `pr-review.md`) — owned by 0066.
- Rewiring `validate-plan/SKILL.md` to read `templates/validation.md` for its
  frontmatter — owned by 0066. This story only adds the frontmatter block to
  the template.
- Creating `templates/note.md` — owned by 0067.
- Unifying value vocabularies (status, verdict, etc.) — explicitly out of
  scope per 0057.
- Touching the three review/validation skills' inline frontmatter emissions
  (`review-plan`, `review-work-item`, `review-pr`, `validate-plan`) — owned by
  0066. Any *other* inline producer surfaced by Phase 11's discovery pass
  IS in scope here.

## Implementation Approach

Test-driven development at three layers:

1. **Template-shape tests** — `scripts/test-template-frontmatter.sh` parses
   each in-scope template's frontmatter block and asserts the unified
   contract. This test exists before any template is edited and starts RED.
2. **SKILL-prose tests** — `scripts/test-skill-frontmatter-population.sh`
   greps each consuming SKILL.md for instructions naming the new mandatory
   fields. Starts RED for nine of ten skills; goes GREEN as each
   per-template phase edits the corresponding SKILL.md.
3. **Helper-output tests** — `scripts/test-metadata-helpers.sh` runs each
   of the three helpers in a controlled directory and asserts the new output
   shape (ISO timestamp, `REVISION` key, no `GIT_BRANCH`). RED first.

Phases 3–10 are file-independent (no two phases edit the same file) but
not runtime-independent. Ordering constraints:

- **Phase 1 must land first** — it provides the failing tests every later
  phase satisfies.
- **Phase 2 must land before Phases 4, 5, 7, 8, 9, 10** — those phases'
  consuming SKILL.md prose substitutes the values printed under Phase 2's
  new helper labels (`Current Revision:`, `Repository Name:`,
  `Current Date/Time (UTC):`) into template fields. Producing them out of
  order yields SKILL.md prose that references stale labels and produces
  broken artifacts at runtime. The tightened SKILL-prose test (Phase 1 §2)
  catches this only if the SKILL prose names the new labels; the safest
  course is to land Phase 2 before any code-state-anchored producer phase.
- **Phase 3 (work-item) is independent** of Phases 4–10 but its §4
  consumer-fallback edits touch shared scripts (`work-item-common.sh`,
  `work-item-read-field.sh`) — schedule it without overlapping
  list/refine/update-work-item work in other branches.
- **Phase 11 must land last** — it records the post-Phase-10 producer set
  and depends on Phase 4's extract-adrs edit to surface that skill.

Within those constraints, Phases 4, 5, 7, 8, 9, 10 can be parallelised by
different engineers once Phase 2 has landed.

Every per-template phase follows the same micro-cycle:

- Read the relevant template-shape tests and the SKILL-prose test rows;
  confirm they fail for the target template/skill.
- Edit the template to emit the unified base fields, the provenance bundle
  (where applicable), and per-type extras. Preserve the existing per-type
  status vocabulary verbatim in the status-comment requirement.
- Edit the consuming SKILL.md(s) to instruct the model to substitute
  values into the new fields. Use the metadata-helper output (Phase 2)
  where the field is `revision`/`repository`/`date`; use explicit `date -u`
  prose for `last_updated`; substitute `producer` and `schema_version`
  literally; populate `last_updated_by` from the `author` resolution path
  already documented in `create-work-item/SKILL.md:578-580`.
- Re-run `mise run test:unit:templates` and confirm GREEN for the rows
  touched.

### Prerequisites

Implementer environment requires: `bash` >= 4, `mise` (toolchain pin in
`mise.toml`), `invoke` (Python task runner), `git` >= 2.28 (for
`init.defaultBranch`), `awk` (GNU or BSD — `-F$'\t'` works on both),
`grep` >= GNU 2.0 OR `rg` (the test driver uses `grep -E` with POSIX
character classes, not PCRE escapes), `mktemp -d`, `date` (BSD or GNU —
both accept the literal `+%Y-%m-%dT%H:%M:%S+00:00` format string), and
`gh` for the describe-pr integration test. Optional but recommended:
`jj` >= 0.20 (required for the jj-branch coverage matrix in Phase 1 §3;
absent → that branch is `skip_test`'d).

All required tools other than `jj` are already pinned in `mise.toml`; no
toolchain edits needed.

### Template inline-comment convention

The rewritten template frontmatter blocks (Phases 3–10) carry inline
comments at uneven density across the plan's YAML examples (Phase 3
annotates every field; Phases 5/7/8/9/10 are sparser). To prevent that
drift reaching disk, apply this policy uniformly when writing the
templates:

- **Comment required** on: `type:` (cite ADR-0033 as the discriminator
  source), `status:` (enumerate the per-type vocabulary verbatim — this
  is also the AC7 contract enforced by the template-shape test),
  `schema_version:` (cite ADR-0033 §Schema versioning), every foreign
  reference (`work_item_id`, etc. — cite ADR-0033 §Identity-value shape
  contract), and every typed-linkage key (`parent`, `supersedes`,
  `target`, etc. — cite ADR-0034).
- **Comment recommended** on: any per-type extra whose vocabulary or
  shape is non-obvious (e.g. `kind` on work-item).
- **Comment omitted** on: self-evident base fields (`id`, `title`,
  `date`, `author`, `producer`, `tags`, `last_updated`,
  `last_updated_by`) — the field name is sufficient.

The plan's YAML examples are specifications, not literal copy-paste:
implementers apply the convention above when materialising the templates.

### Canonical persistence-step prose snippet

Every per-template phase (3, 4, 5, 7, 8, 9, 10) must add a persistence
step to the consuming SKILL.md that satisfies the tightened SKILL-prose
test (Phase 1 §2). To prevent the prose-style drift the documentation
review flagged, use this canonical snippet verbatim, substituting the
parameters in `{braces}`:

> ### Step N: Populate frontmatter
>
> Before writing the artifact file, capture metadata and substitute the
> unified base fields into the template's frontmatter block:
>
> 1. Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
>    to obtain `Current Date/Time (UTC):`, `Current Revision:`, and
>    `Repository Name:`.
> 2. **Substitute** every field below with the indicated value:
>    - `type:` ← `{type-literal}` (e.g. `work-item`, `plan`, `adr`)
>    - `id:` ← `{id-source}` (e.g. filename stem, work-item number,
>      ADR-NNNN identifier; always quoted as a YAML string)
>    - `title:` ← `{title-source}` (the artifact's H1 or computed title)
>    - `date:` ← the `Current Date/Time (UTC):` value from step 1
>    - `author:` ← the author value resolved per
>      `create-work-item/SKILL.md:578-580` (config → VCS user → prompt)
>    - `producer:` ← `{producer-name}` (this skill's name, literally)
>    - `status:` ← `{initial-status}` (per the template's status
>      vocabulary comment)
>    - `last_updated:` ← the same `Current Date/Time (UTC):` value
>    - `last_updated_by:` ← the same value resolved for `author`
>    - `schema_version:` ← `1` (bare integer, not quoted)
>    {if provenance bundle applies, include:}
>    - `revision:` ← the `Current Revision:` value from step 1
>    - `repository:` ← the `Repository Name:` value from step 1
> 3. Write the file with the substituted frontmatter block.

The snippet satisfies SKILL-prose assertion context 2 (imperative verb
`Substitute` near each field name, inside a section heading matching
`Populate frontmatter`) and disambiguates `last_updated` from
`last_updated_by` by listing them on separate lines with a colon suffix.

Each per-phase section below references this snippet and lists the
phase-specific values for the `{braces}` parameters. Do not rewrite
the snippet shape per phase.

---

## Phase 1: Test Scaffolding

### Overview

Add three failing test scripts that encode the contracts every later phase
will satisfy. Wire them into a new mise task `test:unit:templates` so they
run as a unit. After this phase: all three scripts run, all three fail (this
is the RED baseline).

### Changes Required:

#### 1. Template-shape test driver

**File**: `scripts/test-template-frontmatter.sh` (new)
**Changes**: New executable bash script that, for each entry in a hardcoded
table of nine in-scope templates, parses the YAML frontmatter block (delimited
by `---` lines at the head of the file) and asserts:

- Presence of every unified base field: `type`, `id`, `title`, `date`,
  `author`, `producer`, `status`, `tags`, `last_updated`,
  `last_updated_by`, `schema_version`.
- `type:` value equals the table-row expectation (e.g. `work-item`,
  `plan-validation`, `issue-research`).
- `schema_version:` is the bare integer `1` (matched literally, not
  `"1"`).
- `id:` value is a quoted YAML string (regex `^id: ".*"$`).
- No `work_item_id:`, `adr_id:`, or any other `<type>_id:` key is used for
  the artifact's OWN identity (i.e. the value of the `type:` field
  immediately preceding such a key in the same frontmatter block).
- For the five code-state-anchored templates (`plan.md`,
  `codebase-research.md`, `rca.md`, `design-inventory.md`,
  `pr-description.md`): `revision:` and `repository:` present; `git_commit:`
  and `branch:` absent.
- For each row, the per-type extras pinned in the table are present (e.g.
  `kind`, `priority`, `external_id` on work-item; `decision_makers` on
  adr; `pr_url`, `pr_number`, `merge_commit` on pr-description;
  `current_inventory`, `target_inventory` on design-gap; `result` on
  validation; `reviewer` on plan; `topic` on the two research types;
  `source`, `source_kind`, `source_location`, `crawler`, `sequence`,
  `screenshots_incomplete` on design-inventory).
- A status comment is present on the same line as `status:` and its
  vocabulary substring matches the per-row `expected_status_vocabulary`
  (see TEMPLATES table below) exactly. The assertion is `grep -F` on the
  pinned vocabulary string against the `status:` line, NOT a free-form
  regex; this enforces AC7 (vocabulary verbatim) rather than the weaker
  "any non-empty comment" check.

Source `scripts/test-helpers.sh` for `assert_*` helpers and PASS/FAIL/SKIP
counters. Print `test_summary` at the end and exit non-zero on any failure.

**Single source of truth for the schema table**: extract the per-template
contract into `scripts/templates-schema.tsv` (new file), read by both
`test-template-frontmatter.sh` and `test-skill-frontmatter-population.sh`
via `awk -F$'\t'`. This eliminates the previous three-way drift surface
(work-item Schema Reference table ↔ template-shape test array ↔
SKILL-prose test table). The work-item Schema Reference table is the
human-readable authority; the TSV is a machine-readable mirror that the
tests consume. A small assertion in `test-template-frontmatter.sh` parses
the work-item Schema Reference table (lines starting with `|` between the
Schema Reference header and the row count line) and compares row count
and key fields against the TSV; mismatch fails the test.

The TSV has six tab-separated fields per row: template filename, expected
type, code-state-anchored (`yes`|`no`), per-type extras (space-separated),
expected status vocabulary (verbatim string for `grep -F`), forbidden own-id
key (the single legacy own-identity key that must NOT appear in the template
post-rename; literal `-` sentinel for templates with no such legacy key).

The `-` sentinel (rather than an empty trailing column) is load-bearing: it
keeps every row at exactly six non-empty tab-separated fields, so an editor
applying `trim_trailing_whitespace` cannot silently turn a six-field row
into a five-field row. The test driver's self-check (below) asserts
`NF == 6` on every row and fails fast if any row drifts.

Each row's status vocabulary is sourced from the per-template authority:

| Template            | Status vocabulary source                                                        |
|---------------------|---------------------------------------------------------------------------------|
| work-item.md        | existing `templates/work-item.md` `status:` comment (pre-edit)                  |
| plan.md             | `skills/planning/create-plan/SKILL.md` transitions section                      |
| validation.md       | `skills/planning/validate-plan/SKILL.md` inline-frontmatter status              |
| pr-description.md   | `skills/github/describe-pr/SKILL.md` (default status used by skill)             |
| adr.md              | existing `templates/adr.md` `status:` comment (pre-edit)                        |
| codebase-research.md| `skills/research/research-codebase/SKILL.md` persistence step                   |
| rca.md              | `skills/research/research-issue/SKILL.md` persistence step                      |
| design-inventory.md | existing `templates/design-inventory.md` `status:` value (pre-edit)             |
| design-gap.md       | `skills/design/analyse-design-gaps/SKILL.md` (default status)                   |

**`scripts/templates-schema.tsv`** (new file, tab-separated, columns:
template • type • code-state-anchored • extras (space-sep) • status_vocab
(verbatim) • forbidden_own_id_key):

```
work-item.md	work-item	no	kind priority external_id	draft | ready | in-progress | review | done | blocked | abandoned	work_item_id
plan.md	plan	yes	reviewer	draft | ready | in-progress | done	-
validation.md	plan-validation	no	result	complete	-
pr-description.md	pr-description	yes	pr_url pr_number merge_commit	complete	pr_title
adr.md	adr	no	decision_makers	proposed | accepted | superseded | deprecated	adr_id
codebase-research.md	codebase-research	yes	topic	complete	-
rca.md	issue-research	yes	topic	complete	-
design-inventory.md	design-inventory	yes	source source_kind source_location crawler sequence screenshots_incomplete	draft	-
design-gap.md	design-gap	no	current_inventory target_inventory	draft	-
```

Also commit a `.editorconfig` rule (or extend the existing one) pinning
`indent_style = tab` and `trim_trailing_whitespace = false` for `*.tsv`
so editor defaults cannot corrupt the file silently.

**`scripts/test-template-frontmatter.sh`** loads the TSV and applies the
contract:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"
cd "$ROOT"

echo "=== Template frontmatter shape ==="

SCHEMA_TSV="$SCRIPT_DIR/templates-schema.tsv"

BASE_FIELDS=(type id title date author producer status tags last_updated last_updated_by schema_version)
PROVENANCE_FIELDS=(revision repository)
FORBIDDEN_PROVENANCE_FIELDS=(git_commit branch)

# Self-check first: every TSV row must have exactly six tab-separated fields.
# Catches accidental whitespace corruption (tab→space, trailing trim, missing column).
awk -F'\t' 'NF != 6 { print "ERROR: " FILENAME ":" NR " has " NF " fields, expected 6"; exit 1 }' "$SCHEMA_TSV"

# Iterate TSV rows; for each row:
#   - Read frontmatter block (everything between the first two `---` lines,
#     normalising line endings to LF with `tr -d '\r'` first to handle CRLF)
#   - For each BASE_FIELDS entry: assert a line `^<field>:` exists in the block
#   - Assert `type: <expected>` literal
#   - Assert `schema_version: 1` (anchored: regex `^schema_version: 1([[:space:]]+#.*)?$` —
#     uses POSIX `[[:space:]]` not PCRE `\s` because the test driver uses `grep -E`)
#   - Assert `id:` value matches `"..."` (quoted, regex `^id: ".*"$`)
#   - If forbidden_own_id_key != `-`, assert `^<key>:` is absent in the block
#     (handles legitimate foreign references on other templates correctly:
#     plan.md keeps `work_item_id:` as a foreign reference; its row's
#     forbidden_own_id_key is the `-` sentinel and the assertion is skipped)
#   - If code-state-anchored: assert PROVENANCE_FIELDS present, FORBIDDEN_PROVENANCE_FIELDS absent
#   - For each extra in the row: assert `^<extra>:` exists
#   - Extract the `status:` line and assert `grep -F` against the row's
#     status_vocab string returns a match (vocabulary verbatim, AC7)

# Cross-check: parse the work-item Schema Reference markdown table and
# assert its row count and template-filename column match the TSV exactly.
# Drift between the two surfaces fails the test.

test_summary
```

**`scripts/test-skill-frontmatter-population.sh`** reads the same TSV for
the producer→template mapping (column 1) and consults a separate
`scripts/skills-schema.tsv` (also new, tab-separated, columns: skill_path
• producer_name • fields_to_assert) for the per-skill assertion rows. The
two TSVs together replace the prior three-table drift surface.

#### 2. SKILL-prose population test driver

**File**: `scripts/test-skill-frontmatter-population.sh` (new)
**Changes**: New executable bash script that, for each consuming SKILL.md,
asserts the prose actually instructs the model to populate each mandatory
new field — not merely that the field name appears somewhere in the file.

The test table:

```
skills/work/create-work-item/SKILL.md       | producer schema_version last_updated last_updated_by
skills/work/extract-work-items/SKILL.md     | producer schema_version last_updated last_updated_by
skills/planning/create-plan/SKILL.md        | producer schema_version last_updated last_updated_by revision repository
skills/github/describe-pr/SKILL.md          | producer schema_version last_updated last_updated_by revision repository
skills/decisions/create-adr/SKILL.md        | producer schema_version last_updated last_updated_by
skills/decisions/extract-adrs/SKILL.md      | producer schema_version last_updated last_updated_by
skills/research/research-codebase/SKILL.md  | producer schema_version last_updated last_updated_by revision repository
skills/research/research-issue/SKILL.md     | producer schema_version last_updated last_updated_by revision repository
skills/design/inventory-design/SKILL.md     | producer schema_version last_updated last_updated_by revision repository
skills/design/analyse-design-gaps/SKILL.md  | producer schema_version last_updated last_updated_by
```

`validate-plan/SKILL.md` is excluded from this test — its rewiring is 0066's.

**Assertion contract** (per field, per skill): the field name must appear
in at least ONE of the following instruction contexts within the SKILL.md
(checked in order; first match wins):

1. **Fenced-block context** — the field name appears as a YAML key
   (`^<field>:`) inside a triple-backtick fenced code block in the SKILL.md
   that is NOT a `!`config-read-template.sh ...`` template-inclusion line.
   This is the strong form: the skill is actively naming the field as part
   of an emission/example block.
2. **Imperative-instruction context** — the field name appears in a line
   that also contains one of the verbs `[Ss]ubstitute|[Pp]opulate|[Ss]et|[Ww]rite|[Ee]mit`,
   AND that line is within a section whose heading matches
   `(?i)(persistence|metadata|frontmatter|populate|capture metadata|step \d)`.

The `last_updated` and `last_updated_by` checks must be distinguishable.
Prefer the POSIX-safe YAML-key anchor form `^last_updated:` and
`^last_updated_by:` (each followed by space or `[[:space:]]`) rather than
the PCRE `\b` word-boundary, since the test driver uses `grep -E` which
does not portably accept `\b` across BSD/GNU. With anchors, a line that
only mentions `last_updated_by:` cannot satisfy the `last_updated:` row.

A SKILL.md that merely embeds the template via `!`config-read-template.sh ...``
without writing instruction prose for the field does NOT pass; the
fenced-block context check explicitly excludes lines beginning with
`` !`config-read-template.sh `` (the directive itself, which is statically
present in the SKILL.md source — the rendered template content is a runtime
expansion that does not exist in the file at grep-time, so no exclusion of
"rendered template content" is needed or possible).

This trades a small amount of prose-style brittleness for the ability to
fail when the SKILL.md regresses to template-only inclusion with no
substitution prose. The canonical persistence-step snippet (added in
Group 4 / Implementation Approach) is designed to satisfy assertion
context 2 across every skill.

#### 3. Helper-output test driver

**File**: `scripts/test-metadata-helpers.sh` (new)
**Changes**: New executable bash script that runs each of the three metadata
helpers in a hermetically-isolated temp git repo and asserts the output.

**Isolation pattern** (per-invocation function — NOT a global trap, so each
of the six coverage-matrix invocations gets a fresh tmpdir and per-call
cleanup with no cross-iteration state leakage):

```bash
# Run a helper script inside a clean VCS-isolated subshell.
# Args: $1 = vcs ("git" or "jj"); $2 = absolute path to the helper.
# Output: the helper's stdout on success; non-zero exit on failure.
run_helper_in_clean_repo() {
  local vcs="$1" helper="$2"
  local tmpdir
  tmpdir=$(mktemp -d)
  (
    # Subshell: cleanup and env changes are scoped to this invocation only.
    trap "rm -rf '$tmpdir'" EXIT
    unset GIT_DIR GIT_WORK_TREE JJ_CONFIG
    # MUST export so child processes (git/jj) see the new HOME, otherwise
    # bare assignment stays in the parent shell and the helper still reads
    # ~/.gitconfig from the developer's real home.
    export HOME="$tmpdir"
    export XDG_CONFIG_HOME="$tmpdir/.config"

    mkdir -p "$tmpdir/repo"
    cd "$tmpdir/repo"

    case "$vcs" in
      git)
        git -c init.defaultBranch=main -c commit.gpgsign=false init -q .
        git -c user.email=test@example.com -c user.name=Test \
            -c commit.gpgsign=false \
            commit --allow-empty -q -m init
        ;;
      jj)
        # jj init form is version-sensitive; pin the colocated git-backend
        # form (jj >= 0.20). Coverage matrix below skips this branch when
        # `command -v jj` fails or the pinned form is not accepted.
        jj git init --colocate .
        ;;
      *)
        echo "run_helper_in_clean_repo: unknown vcs '$vcs'" >&2
        return 2
        ;;
    esac

    bash "$helper"
  )
}
```

This avoids the three portability traps the naive
`git init && git commit --allow-empty -m init` would hit: missing
`user.email`/`user.name` on fresh CI runners, the `master`-vs-`main`
default-branch variance, and `~/.gitconfig` / `GIT_*` env vars leaking
in from the host shell.

**Coverage matrix**: the helper test must run each of the three helpers
under both VCS branches:

- **git branch** (always exercised): the pattern above forces git by
  building a git-only temp repo. The host's jj install, if any, finds no
  jj workspace at the temp path and the helper falls through to git.
- **jj branch** (skipped when jj absent): if `command -v jj` succeeds,
  additionally run each helper inside a `jj init` workspace; otherwise
  call `skip_test "$name (jj not installed)"` per `test-helpers.sh`. The
  jj setup mirrors the git pattern (isolated `HOME`, no host config).

The helper output contract is a fixed set of human-readable labelled lines.
**Pinned labels** (every helper emits these literal prefixes):

- `Current Date/Time (UTC): <iso>` — UTC ISO timestamp
- `Current Revision: <hash>` — VCS-neutral commit identifier
- `Repository Name: <name>` — repo basename
- `Timestamp For Filename: <stamp>` — per-helper filename stamp shape

Assertions:

- Contains a line matching `^Current Revision:[[:space:]]+[^[:space:]]+$`
  (label-anchored, non-empty value; POSIX character class, not PCRE `\s`,
  because the assertions use `grep -E`).
- Contains a line matching
  `^Current Date/Time \(UTC\):[[:space:]]+[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\+00:00$`
  (label-anchored to prevent false matches against other lines that happen
  to end with an ISO substring).
- Does NOT contain any `GIT_BRANCH=` or `Current Branch Name:` line
  (label-form negative assertion).
- Does NOT contain a non-ISO `%Z`-shaped timestamp (regex
  `[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2} [A-Z]+`).

#### 4. Mise task wiring

To match the convention every other `test:unit:*` task uses
(`invoke <namespace>.<task>` delegation into a Python invoke task), add
the orchestration to the invoke task tree rather than putting a bash
heredoc directly in `mise.toml`.

**File**: `tasks/test/unit.py` (new function `templates`)
**Changes**: Add a `templates` task that shells out to the three test
drivers and accumulates exit codes so a failure in one driver does not
mask failures in the others. Mirrors the existing `unit.visualiser`
pattern that runs cargo twice.

```python
@task
def templates(ctx):
    """Run template / SKILL / metadata-helper schema tests."""
    drivers = [
        "scripts/test-template-frontmatter.sh",
        "scripts/test-skill-frontmatter-population.sh",
        "scripts/test-metadata-helpers.sh",
    ]
    failures = []
    for d in drivers:
        result = ctx.run(f"bash {d}", warn=True, pty=False)
        if result.exited != 0:
            failures.append(d)
    if failures:
        raise Exit(f"Template schema tests failed: {', '.join(failures)}", code=1)
```

**File**: `mise.toml`
**Changes**: Append a new task that delegates to the invoke task:

```toml
[tasks."test:unit:templates"]
description = "Run template / SKILL / metadata-helper schema tests"
run = "invoke test.unit.templates"
```

Also append the task name to the `depends` list of `tasks."test:unit"`
(`mise.toml:95-97`) so the umbrella unit-test target picks it up. Preserve
the existing single-line array shape:

```toml
[tasks."test:unit"]
description = "Run all unit tests in parallel"
depends = ["test:unit:visualiser", "test:unit:frontend", "test:unit:tasks", "test:unit:templates"]
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh; [ $? -ne 0 ]` — exits non-zero (RED baseline, as expected before Phases 3–10).
- [x] `bash scripts/test-skill-frontmatter-population.sh; [ $? -ne 0 ]` — exits non-zero (RED baseline).
- [x] `bash scripts/test-metadata-helpers.sh; [ $? -ne 0 ]` — exits non-zero (RED baseline).
- [x] `mise run test:unit:templates` is a recognised task (`mise tasks ls | grep test:unit:templates`).
- [x] `bash scripts/test-format.sh` still passes (no regressions in existing checks).

#### Manual Verification:

- [ ] Failure messages are specific enough to point an implementer at the field/template/skill that needs editing.
- [ ] The hardcoded TEMPLATES table in `test-template-frontmatter.sh` matches the Schema Reference table in the work item.

---

## Phase 2: Metadata Helpers — Emit Unified Provenance

### Overview

Update the three metadata helpers to emit ISO `+00:00` UTC timestamps,
replace the `Current Git Commit Hash:` line with `Current Revision:`, and
drop the `Current Branch Name:` line. Output labels are pinned (see Phase 1
§3 for the contract). Independent of all template/skill phases. Phase 1's
helper-output test goes GREEN.

**Prerequisite for downstream phases**: this phase MUST land before
Phases 4, 5, 7, 8, 9, 10 — those phases' consuming SKILL.md prose
substitutes the values printed under the new labels into template fields,
so producing them out of order yields broken artifacts at runtime.

### Changes Required:

#### 1. Shared helper

**File**: `scripts/artifact-derive-metadata.sh`
**Changes**: Replace the localised timestamp with ISO UTC; rename the
commit variable from `GIT_COMMIT` to `REVISION`; relabel the printed line
from `Current Git Commit Hash:` to `Current Revision:`; drop the
`Current Branch Name:` line entirely.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Collect metadata
DATETIME_UTC=$(date -u +%Y-%m-%dT%H:%M:%S+00:00)
FILENAME_TS=$(date '+%Y-%m-%d_%H-%M-%S')

if command -v jj >/dev/null 2>&1 && jj root >/dev/null 2>&1; then
  REPO_ROOT=$(jj root)
  REPO_NAME=$(basename "$REPO_ROOT")
  # Inside the outer jj-root guard, jj log MUST succeed; suppress neither
  # stdout nor stderr. Letting the script exit non-zero is the right
  # signal that something is wrong with the jj workspace — better than
  # silently writing an artifact with empty provenance.
  REVISION=$(jj log -r @ --no-graph --template 'commit_id')
elif command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  REPO_ROOT=$(git rev-parse --show-toplevel)
  REPO_NAME=$(basename "$REPO_ROOT")
  REVISION=$(git rev-parse HEAD)
else
  REPO_ROOT=""
  REPO_NAME=""
  REVISION=""
fi

echo "Current Date/Time (UTC): $DATETIME_UTC"
[ -n "$REVISION" ] && echo "Current Revision: $REVISION"
[ -n "$REPO_NAME" ] && echo "Repository Name: $REPO_NAME"
echo "Timestamp For Filename: $FILENAME_TS"
```

**On the triplication smell**: the same edits are applied independently to
three near-duplicate helpers (this one plus the two skill-local variants in
§§2-3 below). The shared logic — VCS detection, ISO/filename timestamp
construction, label emission — differs only in the filename-timestamp
format string. Consolidating to a single helper (e.g.
`scripts/artifact-derive-metadata.sh --filename-format=...`) is recorded
as a follow-up cleanup story; it is OUT of scope for 0065 because the
helpers themselves are not part of the templates-and-schema surface, and
collapsing them would touch every consumer SKILL.md beyond what the schema
work strictly requires. Track this under: "Consolidate metadata helpers"
(to be filed as a follow-up work item once 0065 lands).

The jj-first branching mirrors what `inventory-metadata.sh` already does; the
shared helper gains it for consistency.

#### 2. Inventory-design helper

**File**: `skills/design/inventory-design/scripts/inventory-metadata.sh`
**Changes**: Same edits — ISO timestamp, `REVISION` key, drop branch.
Filename-timestamp format (`%Y-%m-%d-%H%M%S`) preserved unchanged.

#### 3. Design-gap helper

**File**: `skills/design/analyse-design-gaps/scripts/gap-metadata.sh`
**Changes**: Same edits — ISO timestamp, `REVISION` key, drop branch.
`FILENAME_DATE` format preserved unchanged.

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-metadata-helpers.sh` passes (GREEN).
- [x] `rg -n "GIT_COMMIT|GIT_BRANCH" scripts/artifact-derive-metadata.sh skills/design/inventory-design/scripts/inventory-metadata.sh skills/design/analyse-design-gaps/scripts/gap-metadata.sh` returns no matches.
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running each helper inside this repo produces a sane-looking line set with the new keys.
- [ ] The helpers still work when `jj` is unavailable but `git` is (fallback branch).

---

## Phase 3: Work-Item Template & Producer Skills

### Overview

Update `templates/work-item.md` and the two producer skills
(`create-work-item`, `extract-work-items`) so newly-created work items emit
the unified base fields and use `id:` for own identity. Independent of all
other template phases.

### Changes Required:

#### 1. Template

**File**: `templates/work-item.md`
**Changes**: Rewrite the frontmatter block:

```yaml
---
type: work-item                              # ADR-0033 artifact-type discriminator
id: "NNNN"                                   # from work-item-next-number.sh; always a quoted string
title: "Title as Short Noun Phrase"          # human-readable title; kept in sync with body H1
date: "YYYY-MM-DDTHH:MM:SS+00:00"            # date -u +%Y-%m-%dT%H:%M:%S+00:00
author: Author Name                          # human creator
producer: create-work-item                   # producer skill (ADR-0033 — replaces ADR-0028's `skill`)
status: draft                                # draft | ready | in-progress | review | done | blocked | abandoned
kind: story                                  # story | epic | task | bug | spike
priority: medium                             # high | medium | low
parent: ""                                   # typed-linkage key per ADR-0034: "work-item:NNNN" or empty
external_id: ""                              # cross-system pointer per epic 0045 conventions
tags: []                                     # YAML array, e.g. [backend, performance]
last_updated: "YYYY-MM-DDTHH:MM:SS+00:00"    # refreshed only by skills that touch the artifact
last_updated_by: Author Name
schema_version: 1                            # per ADR-0033 §Schema versioning
---
```

The existing body section below the frontmatter is preserved untouched. The
own-identity key is now `id`, not `work_item_id`.

#### 2. create-work-item

**File**: `skills/work/create-work-item/SKILL.md`
**Changes**: In the persistence step (the section that writes the work-item
file), **apply the canonical persistence-step snippet** (see Implementation
Approach §Canonical persistence-step prose snippet) with these parameters:

- `{type-literal}` = `work-item`
- `{id-source}` = the four-digit number produced by
  `work-item-next-number.sh`, as a quoted YAML string
- `{title-source}` = the body H1 title
- `{producer-name}` = `create-work-item`
- `{initial-status}` = `draft`
- provenance bundle = no (work-item is not code-state-anchored)

Additional work-item-specific edits to the SKILL.md:

- The H1 sync rule at `skills/work/create-work-item/SKILL.md:566-569` is
  updated to use `id` (not `work_item_id`) as the own-identity key.
- Search the skill for every remaining occurrence of `work_item_id:` (own
  identity, not foreign reference) and rename to `id:`. References to
  foreign work-items elsewhere remain `work_item_id`.

#### 3. extract-work-items

**File**: `skills/work/extract-work-items/SKILL.md`
**Changes**: Apply the canonical persistence-step snippet in the
extraction-emit step with `{producer-name}` = `extract-work-items` (not
`create-work-item`, per the producer-distinction rule in ADR-0033 — a
producer is the skill that actually wrote the file). All other snippet
parameters identical to create-work-item.

#### 4. Work-item consumer scripts — read-path fallback

The work-item own-identity rename (`work_item_id:` → `id:`) is a producer-side
change. Five consumers read `work_item_id:` directly from work-item
frontmatter and would fail on artifacts produced by the new template. The
fix is a transitional `id:`-first / `work_item_id:`-fallback read at the two
shared script entry points; the four consuming SKILL.md files inherit the
fallback automatically and only need prose updates where they describe the
field key.

**File**: `skills/work/scripts/work-item-common.sh` (`wip_is_work_item_file`,
line 447)
**Changes**: Extend the awk pattern from
`^[[:space:]]*work_item_id[[:space:]]*:` to
`^[[:space:]]*(id|work_item_id)[[:space:]]*:` and adjust the
value-extraction `sub` similarly. The predicate now returns success when
either own-identity key is present with a non-empty value. Add a one-line
comment naming the transition window (until 0070 lands).

**File**: `skills/work/scripts/work-item-read-field.sh`
**Changes**: When invoked with field name `work_item_id` against a file
whose frontmatter contains `id:` but not `work_item_id:`, transparently
return the `id:` value. Symmetrically, when invoked with `id` against a
legacy file, return the `work_item_id:` value. Add a one-line comment
naming the transition window.

**File**: `skills/work/list-work-items/SKILL.md`
**Changes**: Update prose at lines 118-119, 137-138, 165, 307-311 to read
"the `id` field (or `work_item_id` on legacy files)" wherever it currently
says "the `work_item_id` field". The script-level fallback above makes the
behaviour identical; the prose change is documentation-only.

**File**: `skills/work/update-work-item/SKILL.md`
**Changes**: Update prose at 135-138, 263, 278, 284 — same prose shape as
list-work-items. The hard-block on editing the own-identity field stays;
only the key name varies. The error message should accept either key name
in the user's input.

**File**: `skills/work/refine-work-item/SKILL.md`
**Changes**: Update prose at 183, 195, 401 — same prose shape. Note that
`parent` (at line 195) is a foreign reference per ADR-0034 and is unchanged
by this rename.

**File**: `skills/work/create-work-item/SKILL.md`
**Changes**: The enrich-existing self-check at lines 99 and 454 continues to
function via the `work-item-read-field.sh` fallback (no call-site change).
Update prose at 115, 433-435, 450, 462, 483, 499, 525, 536-540, 546, 566-569
to refer to `id` as the new canonical key, with `work_item_id` named as the
legacy alias accepted on read.

This fallback layer is intentionally temporary; 0070's corpus migration
rewrites every legacy `work_item_id:` to `id:` on disk, after which the
fallback can be removed in a follow-up cleanup story.

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `work-item.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS rows for `create-work-item` and `extract-work-items`.
- [x] `bash scripts/test-format.sh` passes.
- [x] `rg -n "^work_item_id:" templates/work-item.md` returns no matches (own-identity rename complete).
- [x] `bash skills/work/scripts/work-item-read-field.sh work_item_id <fixture-with-id-only>` returns the `id` value (fallback works in legacy→new direction).
- [x] `bash skills/work/scripts/work-item-read-field.sh id <fixture-with-work_item_id-only>` returns the `work_item_id` value (fallback works in new→legacy direction).
- [x] `bash -c "source skills/work/scripts/work-item-common.sh && wip_is_work_item_file <fixture-with-id-only>"` exits 0.

#### Manual Verification:

- [ ] Running `create-work-item` against a fresh prompt produces a file with `type: work-item`, `id: "NNNN"` (quoted), `schema_version: 1`, and non-empty `producer`, `last_updated`, `last_updated_by`.
- [ ] Running `extract-work-items` on a sample notes file produces frontmatter with the unified shape.
- [ ] Running `list-work-items` after creating a new work item lists that work item alongside legacy `work_item_id:` files.
- [ ] Running `update-work-item` against a newly-created (post-rename) work item resolves it correctly and hard-blocks edits to the own-identity field.

---

## Phase 4: ADR Template & Producer Skills

### Overview

Update `templates/adr.md` and the two producer skills (`create-adr`,
`extract-adrs`) to emit unified frontmatter. Rename own-identity from
`adr_id` to `id`; add the explicit `config-read-template.sh adr` call to
`extract-adrs` to replace the prose indirection. Independent of all other
template phases.

### Changes Required:

#### 1. Template

**File**: `templates/adr.md`
**Changes**: Rewrite the frontmatter block:

```yaml
---
type: adr                                    # ADR-0033 artifact-type discriminator
id: "ADR-NNNN"                               # always a quoted string per ADR-0033
title: "Title as Short Noun Phrase"          # ADR title (without ADR-NNNN prefix)
date: "YYYY-MM-DDTHH:MM:SS+00:00"
author: Author Name
producer: create-adr                         # or extract-adrs when produced by that skill
status: proposed                             # proposed | accepted | superseded | deprecated
decision_makers: []                          # per ADR-0033 per-type extras
supersedes: []                               # typed-linkage list per ADR-0034: ["adr:ADR-NNNN", ...] or []
tags: [tag1, tag2]
last_updated: "YYYY-MM-DDTHH:MM:SS+00:00"
last_updated_by: Author Name
schema_version: 1
---
```

Notes:

- The own-identity key is now `id`, not `adr_id`.
- `supersedes:` shape shifts to a YAML list of quoted typed-linkage refs
  per ADR-0034 §Reference value shape (e.g. `["adr:ADR-0027"]`); the
  current unquoted single value form is dropped.

#### 2. create-adr

**File**: `skills/decisions/create-adr/SKILL.md`
**Changes**: In the persistence step, **apply the canonical
persistence-step snippet** with these parameters:

- `{type-literal}` = `adr`
- `{id-source}` = `ADR-NNNN` (quoted YAML string, from
  `adr-next-number.sh` or similar resolution)
- `{title-source}` = the ADR title (without ADR-NNNN prefix)
- `{producer-name}` = `create-adr`
- `{initial-status}` = `proposed`
- provenance bundle = no

The own-identity field is now `id`, not `adr_id`. Update any prose checks
against `adr_id` (e.g. the body-H1 sync description) to `id`.

`adr-read-status.sh` reads frontmatter directly — confirm it reads `status:`
(unchanged) and does NOT depend on `adr_id:`. If it parses `adr_id:` for any
reason, update it to read `id:`. (Per current grep, it only reads `status`;
no script change expected.)

#### 3. extract-adrs

**File**: `skills/decisions/extract-adrs/SKILL.md`
**Changes**:

- Replace the prose-indirection at lines 175–178 (`Use the template exactly
  as defined in the create-adr skill...`) with an explicit dynamic template
  read: `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh adr``.
- Apply the canonical persistence-step snippet with the same parameters as
  create-adr but `{producer-name}` = `extract-adrs`.

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `adr.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS rows for `create-adr` and `extract-adrs`.
- [x] `rg -n "^adr_id:" templates/adr.md` returns no matches.
- [x] `rg -n "config-read-template\.sh adr" skills/decisions/extract-adrs/SKILL.md` returns at least one match.
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `create-adr` produces an ADR with `id: "ADR-NNNN"` (quoted), `decision_makers:` present, `supersedes:` as a list (empty if not superseding).
- [ ] Running `extract-adrs` against a research doc with embedded decisions produces ADRs with the unified shape and `producer: extract-adrs`.

---

## Phase 5: Plan Template & create-plan Skill

### Overview

Update `templates/plan.md` (largest single delta) and `create-plan/SKILL.md`
(currently the SKILL.md with zero metadata-capture instructions). Plan
own-identity is the filename stem per the design decision recorded in this
plan's preamble.

### Changes Required:

#### 1. Template

**File**: `templates/plan.md`
**Changes**: Rewrite the frontmatter block:

```yaml
---
type: plan                                   # ADR-0033 artifact-type discriminator
id: "{filename-stem}"                        # full filename without .md, e.g. "2026-05-30-0065-update-artifact-templates-to-unified-schema"
title: "{Feature/Task Name} Implementation Plan"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: create-plan
status: draft                                # draft | ready | in-progress | done
work_item_id: ""                             # foreign reference per ADR-0033 §Identity-value shape contract; empty string when plan has no linked work-item
parent: ""                                   # typed-linkage key per ADR-0034 (optional)
reviewer: ""                                 # per-type extra; present-but-empty until reviewed
tags: []
revision: "{commit hash from artifact-derive-metadata.sh}"  # provenance bundle (ADR-0033)
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---
```

The existing body (Overview, Current State Analysis, etc.) is preserved.
The status comment is the existing `# draft | ready | in-progress | done`
vocabulary, derived from `create-plan/SKILL.md` (preserved verbatim — no
new values added).

#### 2. create-plan skill

**File**: `skills/planning/create-plan/SKILL.md`
**Changes**: Add a new step (after the existing Step 4 "Detailed Plan
Writing") titled "Step 5: Populate frontmatter" (canonical step name from
the snippet — supersedes the prior draft's bespoke "Step 4b" naming). The
step body is the **canonical persistence-step snippet** with these
parameters:

- `{type-literal}` = `plan`
- `{id-source}` = the filename stem (same value computed in step 4.1 for
  the file path, minus the `.md` suffix)
- `{title-source}` = the H1 title `{Feature/Task Name} Implementation Plan`
- `{producer-name}` = `create-plan`
- `{initial-status}` = `draft`
- provenance bundle = yes (plan is code-state-anchored)

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `plan.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `create-plan`.
- [x] `rg -n "^skill:" templates/plan.md` returns no matches (renamed to `producer:`).
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `create-plan` produces a plan file with non-empty `revision`, `repository`, `id` (filename stem), and `producer: create-plan`.
- [ ] The plan body still renders as before.

---

## Phase 6: Validation Template — Add Frontmatter Block

### Overview

`templates/validation.md` is currently body-only. Add the unified frontmatter
block. Do NOT rewire `validate-plan/SKILL.md` — that's 0066's. Independent of
all other phases.

**Important caveat**: between 0065 and 0066 landing, `validate-plan`'s
existing inline frontmatter emission (at SKILL.md lines 134-145) is the
whole file write — it overrides the template's new block, so plan-validation
artifacts produced in the gap window ship the legacy shape. This is the
single "born unified" exception within 0065's scope and is recorded in
Migration Notes.

### Changes Required:

#### 1. Template

**File**: `templates/validation.md`
**Changes**: Prepend a YAML frontmatter block to the existing body content
(the existing `## Validation Report: [Plan Name]` section is preserved):

```yaml
---
type: plan-validation                        # ADR-0033 artifact-type discriminator
id: "{filename-stem}"                        # e.g. "2026-05-18-0042-templates-view-redesign-validation"
title: "Validation Report: {Plan Name}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: validate-plan
status: complete                             # complete (per current corpus vocabulary)
result: ""                                   # pass | partial | fail (filled by validate-plan; rewiring in 0066)
target: ""                                   # typed-linkage key per ADR-0034: "plan:..." (filled by validate-plan in 0066)
tags: []
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---
```

The acceptance criterion explicitly excludes this template from the
consuming-skill population check — `validate-plan` is rewired in 0066, and
that's where field population from the skill is verified.

#### 2. No skill changes here

The skill `validate-plan/SKILL.md:117` will start reading the new
frontmatter as part of its template inclusion (`config-read-template.sh
validation`), but its current inline-frontmatter emission at lines 136–141
remains until 0066 rewires it. This phase intentionally leaves `validate-plan`
alone.

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `validation.md`.
- [x] `validate-plan` is NOT in `scripts/test-skill-frontmatter-population.sh`'s table (verified by row count or comment in the script).
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] `templates/validation.md` opens with `---` and a unified frontmatter block, followed by the existing body.
- [ ] 0066's prerequisites are now satisfied (the template carries a frontmatter block to read).

---

## Phase 7: PR-Description Template & describe-pr Skill

### Overview

Update `templates/pr-description.md` (rename `skill:` → `producer:`, add base
fields and provenance bundle, migrate `pr_title` into base `title:`, add
`pr_url` / `merge_commit`) and rewrite `describe-pr/SKILL.md:99–110`'s
inline frontmatter so it stops overriding the template. Independent of all
other template phases.

### Changes Required:

#### 1. Template

**File**: `templates/pr-description.md`
**Changes**: Rewrite the frontmatter block:

```yaml
---
type: pr-description
id: "{pr_number}"                            # quoted string of the PR number
title: "{PR Title}"                          # migrated from former pr_title field
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: describe-pr
status: complete                             # complete (only vocabulary value in current corpus)
work_item_id: ""                             # foreign reference (optional)
pr_url: ""                                   # filled by describe-pr from `gh pr view`
pr_number: {pr_number}                       # bare integer
merge_commit: ""                             # present-but-empty until merged
tags: []
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---
```

#### 2. describe-pr skill

**File**: `skills/github/describe-pr/SKILL.md`
**Changes**:

- Replace the inline frontmatter block at lines 99–110 with a reference to
  the unified template via `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh pr-description``.
- Apply the **canonical persistence-step snippet** as the Populate
  frontmatter step, with these parameters:
  - `{type-literal}` = `pr-description`
  - `{id-source}` = the PR number as a quoted YAML string (e.g. `"42"`)
  - `{title-source}` = the PR title from `gh pr view <number> --json title`
  - `{producer-name}` = `describe-pr`
  - `{initial-status}` = `complete`
  - provenance bundle = yes
- Beyond the snippet, describe-pr also captures PR-specific extras via
  `gh pr view <number> --json url,number,title` (for `pr_url:`,
  `pr_number:`, `title:`) and the PR's merge status (for `merge_commit:`,
  empty if not yet merged).
- Update the on-re-run regeneration prose at lines 113–117 to regenerate the
  full unified frontmatter (not just `date`). Specify the re-run semantics
  explicitly: `last_updated:` is refreshed to the new ISO timestamp;
  `last_updated_by:` is rewritten to the current author (per the standard
  resolution path); `date:`, `author:`, `id:`, `pr_number:`, `pr_url:` are
  preserved from the existing on-disk file (these are creation-time
  immutable). `merge_commit:` is filled if the PR is now merged.
- The frontmatter-strip-before-posting logic at lines 121+ is unchanged
  (still strips the whole `---...---` block before posting to GitHub).

**Cross-artifact coordination with 0066**: `review-pr/SKILL.md:458` still
writes `pr_title:` inline as part of the 0066-owned review-pr inline
frontmatter. Renaming `pr_title:` → `title:` in this story therefore
creates a cross-artifact divergence: post-0065 PR descriptions emit
`title:` while post-0065 PR reviews continue to emit `pr_title:` until
0066 lands. The divergence is acceptable because the two artifact types
are read by disjoint consumers (the visualiser, plus humans skimming the
files), but it must be recorded in Migration Notes and 0066 must
explicitly rename `pr_title:` → `title:` on the review-pr template it
creates. Coordinate the release ordering so 0066 ships close behind 0065.

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `pr-description.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `describe-pr`.
- [x] `rg -n "^skill:\s*describe-pr" templates/pr-description.md skills/github/describe-pr/SKILL.md` returns no matches (rename complete).
- [x] `rg -n "^pr_title:" templates/pr-description.md` returns no matches (migrated to `title`).
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `describe-pr <pr-number>` on a real PR produces a file with `id: "<pr_number>"`, `pr_url:` populated, `revision:`/`repository:` populated, and the PR description body unchanged.
- [ ] The frontmatter is stripped before posting to GitHub (existing behaviour preserved).

---

## Phase 8: Research Templates & Skills

### Overview

Update both research templates (`codebase-research.md`, `rca.md`) — they
have the same shape today and the same target shape — and both consuming
skills (`research-codebase`, `research-issue`). Independent of all other
template phases.

### Changes Required:

#### 1. codebase-research template

**File**: `templates/codebase-research.md`
**Changes**: Rewrite the frontmatter block:

```yaml
---
type: codebase-research
id: "{filename-stem}"
title: "Research: {User's Question/Topic}"
date: "{ISO timestamp from artifact-derive-metadata.sh}"
author: "{author from VCS}"
producer: research-codebase
status: complete                             # complete (current vocabulary)
work_item_id: ""                             # foreign reference (optional)
topic: "{User's Question/Topic}"
tags: [research, codebase, relevant-component-names]
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{Researcher name}"
schema_version: 1
---
```

#### 2. rca template

**File**: `templates/rca.md`
**Changes**: Same shape, with `type: issue-research` and `producer:
research-issue`. The body remains unchanged.

#### 3. research-codebase skill

**File**: `skills/research/research-codebase/SKILL.md`
**Changes**: In Step 5 ("Gather metadata for the research document"),
**apply the canonical persistence-step snippet** with these parameters:

- `{type-literal}` = `codebase-research`
- `{id-source}` = the filename stem
- `{title-source}` = `Research: {User's Question/Topic}`
- `{producer-name}` = `research-codebase`
- `{initial-status}` = `complete`
- provenance bundle = yes

Apply at file-write time (not only in the follow-up branch at L152). The
existing `revision`/`repository` substitution flows from Phase 2's helper
output (Phase 2 is a prerequisite of this phase).

#### 4. research-issue skill

**File**: `skills/research/research-issue/SKILL.md`
**Changes**: Apply the canonical persistence-step snippet with the same
parameters as research-codebase but `{type-literal}` = `issue-research`
and `{producer-name}` = `research-issue`.

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS rows for `codebase-research.md` and `rca.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS rows for `research-codebase` and `research-issue`.
- [x] `rg -n "^git_commit:|^branch:" templates/codebase-research.md templates/rca.md` returns no matches.
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `research-codebase` on a sample question produces a research file with `revision:` (the new helper output) and `producer: research-codebase`.
- [ ] Running `research-issue` produces an investigation file with `producer: research-issue`.

---

## Phase 9: Design-Inventory Template & Skill

### Overview

Update `templates/design-inventory.md` (already the closest to the target —
mostly add the missing base fields and rename `git_commit` → `revision`) and
`inventory-design/SKILL.md`. Independent of all other template phases.

### Changes Required:

#### 1. Template

**File**: `templates/design-inventory.md`
**Changes**: Rewrite the frontmatter block, preserving all existing domain
fields (`source`, `source_kind`, `source_location`, `crawler`, `sequence`,
`screenshots_incomplete`):

```yaml
---
type: design-inventory
id: "{filename-stem}"
title: "Design Inventory: {source-id}"
date: "{ISO timestamp}"
author: "{author name}"
producer: inventory-design
status: draft                                # draft (current vocabulary)
source: "{source-id}"
source_kind: "{code-repo | prototype | running-app}"
source_location: "{path or URL}"
crawler: "{code | runtime | hybrid}"
sequence: 1
screenshots_incomplete: false
tags: [design, inventory, "{source-id}"]
revision: "{commit hash — omit if not a code repo}"
repository: "{repo name — omit if not a code repo}"
last_updated: "{ISO timestamp}"
last_updated_by: "{author name}"
schema_version: 1
---
```

#### 2. inventory-design skill

**File**: `skills/design/inventory-design/SKILL.md`
**Changes**: At the metadata-substitution step (around line 215 per the
research doc), **apply the canonical persistence-step snippet** with these
parameters:

- `{type-literal}` = `design-inventory`
- `{id-source}` = the filename stem
- `{title-source}` = `Design Inventory: {source-id}`
- `{producer-name}` = `inventory-design`
- `{initial-status}` = `draft`
- provenance bundle = yes (with the caveat that `revision`/`repository`
  may be omitted when the source is not a code repo — per the template's
  comment)

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `design-inventory.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `inventory-design`.
- [x] `rg -n "^git_commit:|^branch:" templates/design-inventory.md` returns no matches.
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `inventory-design` on a sample source produces an inventory file with the new base fields and existing domain fields preserved.

---

## Phase 10: Design-Gap Template & Skill

### Overview

Update `templates/design-gap.md` and `analyse-design-gaps/SKILL.md`. Note:
design-gap is NOT code-state-anchored per the work item's Schema Reference
table — no provenance bundle. Independent of all other template phases.

### Changes Required:

#### 1. Template

**File**: `templates/design-gap.md`
**Changes**: Rewrite the frontmatter block, preserving `current_inventory`
and `target_inventory` as type-specific keys per ADR-0034 §Design-gap
inventory keys:

```yaml
---
type: design-gap
id: "{filename-stem}"
title: "Design Gap Analysis: {current-source} → {target-source}"
date: "{ISO timestamp}"
author: "{author name}"
producer: analyse-design-gaps
status: draft                                # draft (current vocabulary)
current_inventory: "{path to current inventory.md}"
target_inventory: "{path to target inventory.md}"
tags: [design, gap-analysis]
last_updated: "{ISO timestamp}"
last_updated_by: "{author name}"
schema_version: 1
---
```

#### 2. analyse-design-gaps skill

**File**: `skills/design/analyse-design-gaps/SKILL.md`
**Changes**: At the metadata-substitution step (around line 138 per the
research doc), **apply the canonical persistence-step snippet** with these
parameters:

- `{type-literal}` = `design-gap`
- `{id-source}` = the filename stem
- `{title-source}` = `Design Gap Analysis: {current-source} → {target-source}`
- `{producer-name}` = `analyse-design-gaps`
- `{initial-status}` = `draft`
- provenance bundle = no (design-gap is NOT code-state-anchored per the
  Schema Reference table)

The skill currently calls `gap-metadata.sh` for `revision`/`repository`
output — those keys are dropped from the design-gap template, so the
helper's revision/repo output is ignored at template-substitution time
even though the helper still produces them for code-repo contexts. (No
helper change required beyond Phase 2.)

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-template-frontmatter.sh` PASS row for `design-gap.md`.
- [x] `bash scripts/test-skill-frontmatter-population.sh` PASS row for `analyse-design-gaps`.
- [x] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] Running `analyse-design-gaps` on a pair of inventories produces a gap file with the new base fields and `current_inventory` / `target_inventory` preserved.

---

## Phase 11: Inline-Producer Discovery Pass

### Overview

Final compliance step required by the work item's sixth acceptance criterion:
record a reproducible grep enumerating every frontmatter-emitting producer,
and confirm the discovered set minus the four review/validation skills
matches the skills already updated by Phases 3–10. If the pass surfaces any
non-review inline producer not covered, bring it into the unified schema as
part of this phase.

### Changes Required:

#### 1. Record discovery commands and outputs

**File**: `meta/work/0065-update-artifact-templates-to-unified-schema.md`
(append a "Discovery Pass Record" section at end, immediately before
References — this is part of fulfilling AC6).

**Timing**: this phase runs AFTER Phases 3-10 land, so all in-scope
producer skills already mention `producer:` and the unified field names.
The recorded greps therefore enumerate the post-Phase-10 state, not the
pre-edit state. (Running the same greps before Phase 4 would miss
`extract-adrs`, which currently uses prose indirection rather than a
`config-read-template.sh` call.)

**Changes**: Append:

```markdown
## Discovery Pass Record

Commands executed (run from repo root, after Phases 3-10 have landed):

```
# Pass A — template-using and unified-schema-emitting producers
rg -n "config-read-template\.sh|^[[:space:]]*producer:|^[[:space:]]*schema_version:" skills --glob '**/SKILL.md'

# Pass B — legacy inline-frontmatter emitters that have NOT been moved to templates yet
rg -n "verdict:|review_pass:|review_target:|^[[:space:]]*target:|^[[:space:]]*result:|pr_number:" skills --glob '**/SKILL.md'
```

Pass A surfaces every skill that either reads a template via the canonical
loader or directly emits the unified base fields. Pass B surfaces every
remaining inline emitter that has not yet moved its frontmatter into a
template — specifically the four 0066-owned skills (their `verdict:`,
`review_pass:`, `target:`, `result:` literals), plus describe-pr's
`pr_number:` for cross-reference.

Producer split:

- **Template-based emitters (updated by 0065)**: create-work-item,
  extract-work-items, create-plan, create-adr, extract-adrs,
  research-codebase, research-issue, inventory-design,
  analyse-design-gaps.
- **Hybrid emitter brought into compliance by 0065**: describe-pr.
- **Inline-only emitters owned by 0066 (excluded from this story)**:
  review-plan, review-work-item, review-pr, validate-plan.
- **Non-emitter template consumers (no action on frontmatter; read-path
  fallback handled in Phase 3 §4)**: refine-work-item, update-work-item,
  list-work-items.

Other non-review inline producers found: NONE.

### Consumer-side sweep

A second sweep checks read-path consumers of the renamed/removed keys so
the compatibility surface is recorded:

```
rg -n "work_item_id|adr_id|pr_title|^skill:|supersedes:|GIT_COMMIT|Current Git Commit Hash" skills scripts --glob '**/SKILL.md' --glob '**/*.sh'
```

Expected hits and resolution per hit:

- `work_item_id` in `work-item-common.sh` and `work-item-read-field.sh`
  → handled by Phase 3 §4 read-path fallback.
- `work_item_id` in the four work-item consuming SKILL.md files →
  prose-updated by Phase 3 §4.
- `adr_id` in visualiser `indexer.rs` and `wiki-links.ts` → protected by
  filename-prefix fallback; no change required.
- `pr_title` in `review-pr/SKILL.md` → owned by 0066; coordinated via
  Migration Notes.
- Any other hit → must be either (a) renamed in this story or (b) recorded
  here with explicit handoff to a named follow-up.
```

#### 2. Verify completeness via test

Add a final assertion to `scripts/test-skill-frontmatter-population.sh`:
a discovery check that runs Pass A and Pass B (above), unions the matched
SKILL.md set, and asserts every file in the union appears in one of three
explicitly-named allowlists hardcoded in the test:

- `IN_SCOPE_PRODUCERS` — the ten skills in the SKILL-prose test table.
- `OWNED_BY_0066` — `review-plan`, `review-work-item`, `review-pr`,
  `validate-plan`.
- `NON_EMITTER_TEMPLATE_CONSUMERS` — `refine-work-item`, `update-work-item`,
  `list-work-items` (they call `config-read-template.sh` but do not write
  frontmatter; they appear in Pass A and must be allowlisted).

If a new SKILL.md ever appears that is in none of the three allowlists,
this test fails until the new skill is categorised. The grep patterns
themselves are defined as a single bash array at the top of the test
script and referenced from both the test and (by symbol name) the
work-item Discovery Pass Record, so the two cannot drift.

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-skill-frontmatter-population.sh` passes including the new discovery assertion.
- [ ] The two `rg` commands in the work-item's Discovery Pass Record section produce output equivalent to the recorded split (re-runnable).
- [ ] `mise run test:unit:templates` passes (all three sub-drivers GREEN).
- [ ] `bash scripts/test-format.sh` passes.

#### Manual Verification:

- [ ] The recorded discovery commands and the resulting producer split are accurate when re-run by a different engineer.

---

## Testing Strategy

### Unit Tests:

- **Template-shape contract** (`scripts/test-template-frontmatter.sh`): one assertion row per template × required field; designed to fail loudly with a pointer to the field/template needing edit.
- **SKILL-prose population** (`scripts/test-skill-frontmatter-population.sh`): one assertion row per consuming skill × required field name.
- **Metadata-helper output** (`scripts/test-metadata-helpers.sh`): runs each of the three helpers in a controlled temp git repo and inspects the output shape.

### Integration Tests:

- **End-to-end production of one artifact per template**: for each updated skill in Phases 3, 4, 5, 7, 8, 9, 10, run the skill once against a representative input and parse the resulting file's frontmatter. Confirm `producer`, `schema_version`, `last_updated`, `last_updated_by`, and (for code-state-anchored types) `revision`/`repository` are populated with non-empty, non-tokenised (`{{…}}` or `<…>`) values, `schema_version == 1`, and the two timestamps parse as ISO UTC. These tests are predominantly manual because the producer skills are LLM-driven.

### Accepted limitation: AC8 verification is manual-only

The work item's acceptance criterion 8 (artifacts born populated with non-tokenised values) cannot be enforced by an automated test in this story because the producer skills are LLM-driven — running a skill requires a model invocation, not a deterministic script call. The Phase 1 §2 SKILL-prose test enforces the *substitution-instruction prose* contract (i.e. the SKILL.md tells the model to substitute every required field) and is sufficient to catch the failure mode where a SKILL.md regresses to template-only inclusion without substitution prose. The end-to-end "no unsubstituted token reaches disk" check remains in the Manual Testing Steps below. This trade-off is accepted; a follow-up story could automate it via fixture-based skill harnesses if the manual surface becomes load-bearing in practice.

### Manual Testing Steps:

1. Run `create-work-item` with a short prompt, confirm output file has the new base block (`id:` quoted, `schema_version: 1`).
2. Run `create-plan` referencing this plan's work item; confirm `revision` and `repository` are populated and `id` matches the filename stem.
3. Run `create-adr` for a tiny invented decision; confirm `id: "ADR-NNNN"` (quoted), `decision_makers: []`, `supersedes: []`.
4. Run `research-codebase` on a small question; confirm `revision:` is the current commit and `producer: research-codebase`.
5. Run `describe-pr <pr-number>` on a real PR; confirm new fields populated AND the body posted to GitHub still excludes the frontmatter.
6. Visually confirm `templates/validation.md` now opens with a frontmatter block.
7. Re-run the discovery grep from Phase 11 in a fresh shell and confirm the producer split matches the recorded one.

## Performance Considerations

None of substance. Templates and SKILL.md files are small; the test scripts
run in <1s each. The metadata helpers are invoked once per artifact write,
unchanged in cost.

## Migration Notes

- Existing `meta/` artifacts are NOT touched by this story. The corpus
  migration (0070) handles them. After this story, newly-produced artifacts
  diverge from the existing corpus in shape — that's expected and is what
  0070 will reconcile.
- 0066 depends on this story for the `templates/validation.md` frontmatter
  block (Phase 6). 0066's rewiring of `validate-plan` will read this
  template's new frontmatter block.
- **Plan-validation is the single exception to "born unified"** in the
  window between 0065 and 0066. Phase 6 adds the unified frontmatter block
  to `templates/validation.md` but `validate-plan/SKILL.md:134-145`
  continues to emit its own inline frontmatter (legacy shape: `skill:`,
  `target:`, `result:`, `status:`, `date:`, `type:` — no `id`, no
  `producer`, no `schema_version`) until 0066 rewires the skill. New
  validation reports written during the 0065→0066 gap therefore ship the
  legacy shape; the template's new frontmatter block is dead code at that
  point and reaches disk only once 0066 lands. Order 0066 close behind
  0065 so the gap window stays short.
- **Work-item consumer fallback is transitional**: Phase 3 §4 adds an
  `id:`-first / `work_item_id:`-fallback read at `wip_is_work_item_file`
  and `work-item-read-field.sh` so the four consuming skills
  (`list-work-items`, `update-work-item`, `refine-work-item`,
  `create-work-item`'s enrich-existing self-check) work against both shapes
  during the 0065→0070 window. The fallback is intentionally temporary and
  removed by a follow-up cleanup story once 0070 normalises the corpus.
- **`pr_title:` → `title:` cross-artifact coordination**: Phase 7 migrates
  `pr_title:` into the unified base `title:` field on the PR-description
  template, but `review-pr/SKILL.md:458` (owned by 0066) still emits
  `pr_title:` in its inline frontmatter. Post-0065 PR descriptions will
  carry `title:` while post-0065 PR reviews still carry `pr_title:` until
  0066 lands and renames the field in its review-pr template. The
  divergence is acceptable because the two artifact types have disjoint
  consumer sets, but it must remain time-bounded — 0066 should ship close
  behind 0065.
- **Consumer-side breakage surface for other renames** (`adr_id:` → `id:`,
  `skill:` → `producer:`, `git_commit` → `revision`, `supersedes:` shape
  change): the ADR own-id rename is shielded by visualiser filename
  fallback (`indexer.rs:1098`, `wiki-links.ts:103-115`); the helper output
  label change is internal (no consumer parses the literal); the
  `supersedes:` shape change has no known consumers today but creates a
  mixed-shape corpus until 0070; the `skill:` → `producer:` rename is
  covered by the consumer-side sweep in Phase 11. See Phase 11 for the
  recorded enumeration and per-hit resolution.

## References

- Original work item: `meta/work/0065-update-artifact-templates-to-unified-schema.md`
- Related research: `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`
- Authoritative schema ADR: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Authoritative linkage ADR: `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Blocked-by predecessors (all done): 0060, 0061, 0063, 0064
- Blocks: 0066 (validation.md frontmatter handoff), 0070 (corpus migration)
- Template-loader: `scripts/config-read-template.sh:53-60`
- Existing metadata helpers: `scripts/artifact-derive-metadata.sh:5-25`, `skills/design/inventory-design/scripts/inventory-metadata.sh:10-34`, `skills/design/analyse-design-gaps/scripts/gap-metadata.sh:10-34`
- Test-harness helpers: `scripts/test-helpers.sh` (sourced by all three new test drivers)
- Hybrid producer needing inline-block rewrite: `skills/github/describe-pr/SKILL.md:99-110`
- Prose-indirection to remove in extract-adrs: `skills/decisions/extract-adrs/SKILL.md:175-178`
