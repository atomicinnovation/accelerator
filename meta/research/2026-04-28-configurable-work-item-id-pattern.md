---
date: 2026-04-28T09:40:31+01:00
researcher: Toby Clemson
git_commit: 6947ac9f1b3d2429623df1d008cc38578bbde52f
branch: (no bookmark — change qkn)
repository: accelerator
topic: "Configurable work-item ID prefix pattern"
tags: [ research, work-items, configuration, naming, migration, ticket-management ]
status: complete
last_updated: 2026-04-28
last_updated_by: Toby Clemson
---

# Research: Configurable work-item ID prefix pattern

**Date**: 2026-04-28 09:40:31 BST
**Researcher**: Toby Clemson
**Git Commit**: 6947ac9f1b3d2429623df1d008cc38578bbde52f
**Branch**: (no bookmark — change qkn)
**Repository**: accelerator

## Research Question

The accelerator plugin's work-management skills currently assume work-item
filenames carry a `NNNN` numeric prefix (e.g. `0042-add-user-login.md`). Teams
using external trackers like Jira or Linear typically issue IDs of the form
`XXX-NNNN` where `XXX` is a project code (e.g. `ENG-1234`). Extend the
work-management skills so users can configure an alternative prefix pattern,
with a syntax that distinguishes static segments from dynamic ones, and a
smarter next-number picker that handles non-trivial patterns. Sync work-item IDs
may originate from multiple upstream projects, so the project code itself is
*not* part of the pattern; the user supplies a default and can override per
creation.

## Summary

The numeric `NNNN-` prefix is hard-coded across two artefact families (work
items and ADRs), but the user's request is scoped to **work items only** — ADRs
always live in the repo and have no need for project-coded IDs.

The change has three pieces:

1. **A pattern DSL using Python-style named tokens** (`{project}-{number:04d}`)
   configured under a new `work.id_pattern` key in `.claude/accelerator.md`. The
   pattern describes the **ID prefix only**; the trailing `-<slug>.md` is
   invariant.
2. **A `work.default_project_code` config field plus a per-creation `--project`
   override** for the `{project}` token, supporting work items synced from
   multiple upstream projects.
3. **A migration extension** that renames the existing `NNNN-*.md` corpus to the
   new pattern with the configured default project code, and a new "skipped"
   track in the migration framework so users can opt out without the migration
   re-prompting on every run.

The next-number algorithm is generalised to: compile the pattern into a
`(scan_regex, format_string)` pair; scan the work directory; filter to filenames
matching `scan_regex` *with the chosen project value substituted*; take max of
the captured `{number}` group; output the next allocated value(s) in the
configured format. This means counters are **per-project** when `{project}` is
in the pattern.

The implementation stays bash-only, consistent with the existing
`scripts/config-*.sh` family, on the understanding that helper scripts will be
re-implemented in a Rust CLI in a future change.

## Detailed Findings

### Current state of the `NNNN` convention

Two parallel implementations of the same algorithm currently exist:

- `skills/work/scripts/work-item-next-number.sh:43-72` — scans
  `[0-9][0-9][0-9][0-9]-*` glob via `find`-style shell glob, parses leading
  digits with `grep -oE '^[0-9]+'`, tracks max via `10#$NUM` (forces base-10),
  emits `printf "%04d\n"`. Has a hard-fail overflow guard at 9999 (lines 60-69).
- `skills/decisions/scripts/adr-next-number.sh:53-61` — same shape with `ADR-`
  literal prefix; no overflow guard; no missing-directory warning. **Out of
  scope for this change.**

Filename validation is not centralised — pattern recognition is scattered
across:

- 7 shell-glob sites in work-item skills: `work-item-next-number.sh:47`,
  `list-work-items/SKILL.md:109,127`, `create-work-item/SKILL.md:81,85`,
  `update-work-item/SKILL.md:47`, `review-work-item/SKILL.md:41`,
  `extract-work-items/SKILL.md:352`.
- A discriminator regex `^[0-9]+$` at `create-work-item/SKILL.md:81`
  distinguishing ticket-number arguments from path arguments.
- Parent-field normalisation prose in `list-work-items/SKILL.md:174-177` and
  `update-work-item/SKILL.md:135-137` (strip quotes/leading zeros, zero-pad to 4
  digits before comparison).
- The H1 format `# NNNN: <title>` at `create-work-item/SKILL.md:512-514`.
- The frontmatter field `work_item_id: NNNN` (filename prefix is authoritative
  when it disagrees per `list-work-items/SKILL.md:147-149`).

### The configuration extension path

The plugin's userspace config system (`.claude/accelerator.md` +
`.claude/accelerator.local.md`, both with YAML frontmatter and free-form
markdown body) is well-paved for adding a new section. Existing sections include
`agents`, `review`, `paths`, `templates`, plus per-skill overrides at
`.claude/accelerator/skills/<skill-name>/`. Reader scripts live under
`scripts/config-*.sh`, all bash + awk, no `yq`/Python.

For a new section the typical change-set is:

1. Pick section + key names (max 2 levels of nesting).
2. Either consume via `config-read-value.sh "<section>.<key>" "<default>"` for a
   single scalar, or add a dedicated `config-read-<section>.sh` modelled on
   `config-read-review.sh` for richer cases.
3. Wire into consuming SKILL.md preprocessor `!`backtick`` lines.
4. Document under `skills/config/configure/SKILL.md`.
5. Add tests to `scripts/test-config.sh`.

There is no central registry of supported keys; each section maintains its own
canonical list. ADR-0017 (configuration extension points) explicitly
contemplates additions of this shape.

### The migration framework

`skills/config/migrate/` provides numbered migrations under
`migrations/[NNNN]-name.sh` (the only one to date is
`0001-rename-tickets-to-work.sh`). The runner at `scripts/run-migrations.sh`
reads `meta/.migrations-applied` (newline-delimited list of applied IDs),
atomically appends on success, preserves unknown IDs (forward-compat), and
pre-flights a clean working tree check (`ACCELERATOR_MIGRATE_FORCE=1` to
bypass).

Key gap for this work: there is currently **no concept of a skipped migration**.
A user who wants to opt out has only two choices — apply it or face the prompt
forever. The framework needs extending to track skips separately.

### Industry precedent for pattern languages

Surveyed across Jira, Linear, GitHub Issues, Shortcut, ClickUp, Notion, Trello,
Asana, Height, Pivotal Tracker, Azure DevOps:

- **`KEY-NUMBER` is the dominant convention.** Jira (`ABC-123`), Linear (
  `ENG-123`), Shortcut (`sc-1234`), Height (`T-123`).
- **Bare integer is the runner-up.** GitHub `#42`, Pivotal `#123456789`, Azure
  DevOps plain integer.
- **Few tools let users configure the *format*** — most expose only a prefix
  string + start index (ClickUp, Notion). Jira hard-codes `KEY-NUMBER` and only
  lets admins constrain the key validator regex.

Surveyed pattern-language paradigms:

| Approach                | Examples                                              | Strength                                             | Weakness                                                         |
|-------------------------|-------------------------------------------------------|------------------------------------------------------|------------------------------------------------------------------|
| Prefix-only string      | ClickUp, Notion                                       | Trivial config                                       | One-dimensional                                                  |
| Token DSL (named)       | Hugo permalinks, Jekyll, Python format spec, Mustache | Readable, extensible, can carry format spec (`:04d`) | Tokens proliferate over time                                     |
| Printf positional       | ffmpeg `img-%03d.bmp`                                 | Familiar, terse                                      | Single namespace, opaque                                         |
| Repeated-char literal   | MADR `nnnn-title.md`                                  | Self-documenting (width = char count)                | Conflicts when literal letters appear; no format-spec equivalent |
| Regex with named groups | Jira project-key validator                            | Excellent for parsing                                | Hostile for generation                                           |

Python-style format spec inside named tokens is the most ergonomic precedent:
human-readable, carries width control, extensible to date/type/slug placement if
ever needed.

### Pitfalls to design against

- **Filesystem case-sensitivity.** macOS (default APFS) and Windows (NTFS) treat
  `Proj-0001-foo.md` and `proj-0001-foo.md` as the same file; Linux ext4 sees
  two. Recommendation: constrain project codes to `[A-Z][A-Z0-9]*` (Jira's rule)
  at validation time.
- **Counter merge collisions.** Two branches both create `PROJ-0042`
  independently; lexical clash on merge. Per-project counters reduce surface
  area but don't eliminate this — fundamental to any sequential-counter scheme.
  Log4brains' workaround is date prefixes; not adopted here because it changes
  the artefact identity model significantly.
- **Round-trip ambiguity.** A pattern like `{project}{number}` (no separator)
  can't be parsed back into prefix + number when the prefix ends in digits.
  Validation must enforce that adjacent dynamic tokens are separated by at least
  one literal character.
- **Filesystem-hostile chars.** `/`, `\`, `:`, `*`, `?`, `<`, `>`, `|`, `"` must
  be rejected in literal segments and in the resolved `{project}` value.

## Design Proposal

### Pattern DSL

Adopt Python-style named tokens with format-spec syntax. Parsed grammar (
informal):

```
pattern    := segment+
segment    := literal | token
literal    := <any non-{} character, escape with {{ and }}>
token      := "{" name (":" spec)? "}"
name       := "project" | "number"
spec       := <Python-style format spec, e.g. 04d>
```

For v1 the only supported tokens are `{project}` and `{number}`. New tokens (
e.g. `{type}`, `{date}`) can be added incrementally without breaking existing
patterns.

Examples:

| Pattern                     | Generated ID   | Filename                         |
|-----------------------------|----------------|----------------------------------|
| `{number:04d}` (default)    | `0042`         | `0042-add-user-login.md`         |
| `{project}-{number:04d}`    | `PROJ-0042`    | `PROJ-0042-add-user-login.md`    |
| `{project}-{number:05d}`    | `PROJ-00042`   | `PROJ-00042-add-user-login.md`   |
| `WI-{project}-{number:04d}` | `WI-PROJ-0042` | `WI-PROJ-0042-add-user-login.md` |

The pattern describes the **ID prefix only**. The full filename is always
`<id>-<slug>.md`. The leading `-` separator before the slug is invariant.

### Configuration schema

New top-level YAML section in `.claude/accelerator.md`:

```yaml
---
work:
  id_pattern: "{project}-{number:04d}"
  default_project_code: "PROJ"
---
```

Both keys are optional:

- **`work.id_pattern`** — defaults to `{number:04d}`, exactly recovering the
  current behaviour.
- **`work.default_project_code`** — used when the pattern contains a `{project}`
  token and the caller does not supply `--project`. If the pattern has no
  `{project}` token, this field is ignored (warn at config-read time so users
  notice typos).

Local override in `.claude/accelerator.local.md` works as for any other key —
last-writer-wins, so a developer can override the default project code
personally.

### Validation

Performed at config-read time and again at compile time inside
`work-item-next-number.sh`:

1. Pattern must contain at least one `{number...}` token (otherwise no counter
   to allocate against).
2. Pattern must not contain filesystem-hostile chars (`/`, `\`, `:`, `*`, `?`,
   `<`, `>`, `|`, `"`) outside of token format specs.
3. Adjacent dynamic tokens must be separated by at least one literal character (
   round-trip safety).
4. Format spec on `{number}` must compile to a valid printf width (`0` flag plus
   integer), otherwise reject.
5. Project-code value (from config or `--project`) must match
   `[A-Za-z][A-Za-z0-9]*` after expansion. Mixed case is allowed; users are
   responsible for not introducing case-only collisions on case-insensitive
   filesystems.
6. The whole pattern must round-trip: `format(N)` then `parse` must recover `N`.
   Verified with a self-test on init and at config-read time using `N=1` and
   `N=9999`.

### Pattern compilation

The next-number algorithm becomes:

```
compile_pattern(pattern, project_value) -> (scan_regex, format_string):
  for each segment:
    if literal:
      append re.escape(literal) to scan_regex
      append literal verbatim to format_string
    elif token == {project}:
      append re.escape(project_value) to scan_regex   # project filters the scan
      append project_value verbatim to format_string/cle
    elif token == {number:04d}:
      append "([0-9]{4,})" to scan_regex
      append "%04d" to format_string
  scan_regex = "^" + scan_regex + "-"     # trailing - separator before slug
  return (scan_regex, format_string)
```

Substituting the project value into the scan regex is what scopes the counter
per project: when scanning `meta/work/`, only filenames whose prefix matches the
chosen `{project}` value contribute to the max. `PROJ-0001-…` and `OTHER-0001-…`
both exist and don't collide.

**Width tolerance during scan.** The scan regex captures `{number}` as
`[0-9]+` (one or more digits, regardless of the configured width). Generation
uses the configured width verbatim. This means width-only pattern changes (e.g.
`{number:04d}` → `{number:05d}`) do *not* reset the counter — old 4-digit files
still contribute to the max, new IDs are produced 5-digit. Mixing widths is the
user's deliberate choice (see "Pattern changes after first use" below); the
next-number logic Just Works.

For counters narrower than the literal width of the produced output (overflow),
keep the existing 9999-style guard but at the configured width — `printf "%04d"`
of `10000` produces `10000`, breaking the scan glob on the next run. Detect at
output time, fail explicitly.

### Pattern changes after first use

Per the resolved questions below, `work.id_pattern` may be changed at any time
and **no migration of existing work items is performed**. Concretely:

- Width-only changes (`{number:04d}` → `{number:05d}`): old files remain visible
  to the scanner because of the width-tolerance behaviour above; new IDs use the
  new width. Sort stability across the corpus may be lost.
- Structural changes (adding/removing `{project}`, changing literals): old files
  no longer match the scan regex once a project value is substituted in; new IDs
  start fresh from `0001` under the new pattern. Old files coexist on disk with
  their original IDs and are still discoverable via the bare-glob fallback in
  `list-work-items` (which lists everything matching `*.md` and skips files
  whose name doesn't parse as an ID, regardless of pattern — see "Lookup
  semantics" below).
- No warnings or migration prompts are emitted on pattern change. The user is
  treated as having made a deliberate decision.

### CLI surface

`work-item-next-number.sh` gains a `--project` flag:

```
work-item-next-number.sh [--count N] [--project CODE]
```

- If pattern has no `{project}` token: `--project` is rejected (clear error).
- If pattern has `{project}` and `--project` is omitted: fall back to
  `work.default_project_code`. If neither is set, fail with a clear error
  pointing at the config.
- `CODE` is validated against `[A-Z][A-Z0-9]*` before substitution.

`create-work-item` is updated to accept the project code as an argument or
prompt for one when needed. Existing invocations that pass just a number or path
remain valid (the default project code is used).

### Lookup semantics

Multiple sites currently look up a work item by its bare number (e.g.
`update-work-item 0042`). With multi-project corpora, "0042" alone is ambiguous.
Resolution rules:

1. **Full ID** (`PROJ-0042`) — always unambiguous; resolves directly. Supported
   regardless of whether the current pattern would generate that ID — i.e.
   legacy IDs still resolve after a pattern change.
2. **Bare number** (`0042`) — resolves via `work.default_project_code` if set;
   otherwise scans `*-0042-*.md` and `0042-*.md` (legacy), warns and lists
   candidates if multiple match.
3. **Path** (`meta/work/PROJ-0042-add-user-login.md`) — passes through
   unchanged.

The `^[0-9]+$` discriminator at `create-work-item/SKILL.md:81` is replaced with
three checks: (a) is the input a known full ID matching the configured pattern
*or any legacy filename in the directory*? (b) is it a path? (c) is it a bare
number? Order of precedence: full-ID-or-legacy, then path, then bare number.

`list-work-items` discovery glob is broadened to `*.md` (with header/frontmatter
validation discarding non-work-item files) so files that don't match the
*current* pattern — e.g. legacy `0042-foo.md` files after a structural pattern
change — remain listable.

### Migration

A new migration `0002-rename-work-items-with-project-prefix.sh` is added under
`skills/config/migrate/migrations/`. Behaviour:

1. Reads `work.id_pattern` and `work.default_project_code` from config.
2. If pattern has no `{project}` token: no-op, marks migration as applied.
3. Otherwise: globs `meta/work/[0-9][0-9][0-9][0-9]-*.md` (legacy filenames) and
   renames each to the new pattern with the default project code substituted.
   Updates `work_item_id` in frontmatter to the **full ID** (e.g.
   `work_item_id: "PROJ-0042"` rather than `"0042"`).
4. **Updates cross-references across the whole `meta/` tree**, not just
   `meta/work/`. Scope:
  - **Frontmatter ID-bearing fields** in any `meta/**/*.md` file:
    `work_item_id`, `parent`, `related`, `blocks`, `blocked_by`, `supersedes`,
    `superseded_by` (and any other field whose values match a known legacy
    work-item ID). Both string (`"0042"`) and bare (`0042`) forms are normalised
    to quoted full IDs.
  - **Markdown links** that reference the old filename: `[*](*0042-*.md)` and
    `[*](meta/work/0042-*.md)` patterns are rewritten to point to the new
    filename.
  - **Prose ID references** of the strict forms `#0042`, `# 0042` (heading), and
    `meta/work/0042-...md` paths in code fences. Bare 4-digit numbers in prose
    without an obvious work-item context are *not* rewritten — too lossy.
5. Atomic per file — rename + frontmatter rewrite happens as a transaction;
   partial failure leaves the file untouched. Cross-reference rewrites in other
   files are batched after all renames complete and use the final old→new
   mapping.
6. Frontmatter `work_item_id` semantics flip globally: it is now the **full ID
   **. `list-work-items`, `update-work-item`, etc. treat the full ID as
   canonical; the filename prefix remains the authoritative source of truth for
   lookup but the field is consistent with it.

#### Skip-tracking extension to the migration framework

The framework gains a sibling state file `meta/.migrations-skipped` with the
same line-delimited format as `meta/.migrations-applied`. Changes to
`scripts/run-migrations.sh`:

1. **Pending computation** becomes
   `pending = (id NOT IN applied) AND (id NOT IN skipped)`.
2. **A new `--skip <id>` flag** appends `id` to `meta/.migrations-skipped` and
   exits.
3. **A new `--unskip <id>` flag** removes `id` from `meta/.migrations-skipped` (
   allowing the user to change their mind).
4. **The skill prose** (`skills/config/migrate/SKILL.md`) is updated to describe
   the skipped track, `--skip`/`--unskip`, and to surface pending counts as
   `applied: A; skipped: S; available: V` instead of just applied vs. available.
5. **Idempotency holds**: re-running with the same state is a no-op, regardless
   of whether migrations are applied or skipped.

The split into two files (rather than augmenting `.migrations-applied` with a
status column) preserves the existing format as audit-grade and keeps each file
as a simple set. Unknown-ID preservation behaviour applies equally to both
files.

### What does *not* change

- ADR scripts, globs, and skill prose stay exactly as they are. ADR-NNNN remains
  the canonical ADR ID format.
- Plans (date-prefixed) are unaffected.
- The internal migrations (`skills/config/migrate/migrations/[NNNN]-*.sh`) are
  *plugin*-internal numbering and stay literal — they are not work items.
- The free-form markdown body of `.claude/accelerator.md` is unchanged.
- Per-skill `context.md` / `instructions.md` overrides are unaffected.

## Code References

### Files to extend

- `scripts/config-read-value.sh` — already supports the new `work.id_pattern`
  and `work.default_project_code` reads with no changes (it's generic).
- `skills/work/scripts/work-item-next-number.sh:43-72` — generalise scan/format
  around the compiled pattern; add `--project` flag.
- `skills/work/scripts/work-item-pattern.sh` (new) — compile a pattern + project
  value into `(scan_regex, format_string)`. Exports a sourcable bash function (
  or stand-alone script) for reuse across work-item scripts.
- `skills/work/scripts/work-item-resolve-id.sh` (new) — given an input string
  and the configured pattern, classify as full-ID / bare-number / path and
  return the canonical resolved path. Used by `update-work-item`,
  `review-work-item`, `create-work-item` (enrich path).
- `skills/work/create-work-item/SKILL.md:81` — replace the `^[0-9]+$`
  discriminator and the `{work_dir}/NNNN-*.md` glob with calls to
  `work-item-resolve-id.sh`. Update the H1 format and the `work_item_id`
  frontmatter doc (line 512-514) to be pattern-aware.
- `skills/work/list-work-items/SKILL.md:109,127,147-149,174-177` — replace
  literal `[0-9][0-9][0-9][0-9]-*.md` glob with the compiled scan regex; replace
  parent normalisation rule with a pattern-aware canonicaliser.
- `skills/work/update-work-item/SKILL.md:47,135-137` — same.
- `skills/work/review-work-item/SKILL.md:41` — same.
- `skills/work/extract-work-items/SKILL.md:352,357-364` — collision-check glob
  and the `--count N` allocator. The allocator now passes `--project` through
  unchanged.
- `skills/config/migrate/scripts/run-migrations.sh` — add skipped-state file
  handling and `--skip` / `--unskip` flags.
- `skills/config/migrate/SKILL.md` — document skip semantics.
-
`skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh` (
new).
- `skills/config/configure/SKILL.md` — document `work.id_pattern` and
  `work.default_project_code` under a new "Work Items" subsection (mirroring the
  existing `agents`/`review`/`paths`/`templates` subsections).
- `scripts/test-config.sh` — add tests for the new keys.
- `skills/work/scripts/test-work-item-pattern.sh` (new) — unit tests for the
  pattern compiler covering: default `{number:04d}`, `{project}-{number:04d}`,
  validation rejections, round-trip property.

### Files explicitly *not* changed

- `skills/decisions/scripts/adr-next-number.sh` — ADRs out of scope.
- `skills/decisions/create-adr/SKILL.md`,
  `skills/decisions/extract-adrs/SKILL.md` — same.
- All ADR globs and prose.

## Phased Implementation

The work breaks into six phases. Phases 1-4 form a serial chain; Phase 5 is
independent and can ship at any time; Phase 6 depends on Phases 1-3 *and* Phase
5.

Each phase below is a candidate scope for one plan document. The wording is
suggestive, not prescriptive — the actual plans will refine the slicing.

### Phase 1 — Pattern compiler and config schema

**Scope.** Introduce the pattern DSL infrastructure with no behavioural change
for existing users.

**Deliverables.**

- New script `skills/work/scripts/work-item-pattern.sh`. Exposes a function that
  takes `(pattern, project_value)` and emits `(scan_regex, format_string)` on
  stdout, plus a `--validate` mode for config-time checks.
- New config keys `work.id_pattern` (default `{number:04d}`) and
  `work.default_project_code` (default empty), readable via the existing
  `config-read-value.sh`.
- Validation enforced at config-read time: tokens supported are `{project}` and
  `{number[:spec]}`; rules 1-6 from the Validation subsection above.
- Documentation under a new "Work Items" subsection in
  `skills/config/configure/SKILL.md`.
- Unit tests in `skills/work/scripts/test-work-item-pattern.sh` covering:
  default pattern, `{project}-{number:04d}`, width variants, validation
  rejections (no-`{number}`, hostile chars, adjacent dynamic tokens, bad format
  spec, bad project value), round-trip property at `N=1` and `N=9999`.
- Config tests added to `scripts/test-config.sh`.

**Exit criteria.** Compiler is callable, validates correctly, has full test
coverage. Default-pattern behaviour is bit-identical to current. No SKILL.md
prose has been changed yet.

**Depends on.** Nothing.

### Phase 2 — Next-number generaliser and ID resolver

**Scope.** Make the work-item allocation and lookup primitives pattern-aware.
Still no user-visible change on default config.

**Deliverables.**

- `skills/work/scripts/work-item-next-number.sh` rewritten to use the Phase 1
  compiler. Adds `--project CODE` flag; rejects when pattern has no `{project}`
  token, falls back to `work.default_project_code` when present, errors clearly
  when project token requires a value and none is configured.
- New `skills/work/scripts/work-item-resolve-id.sh`. Classifies input as
  full-ID / legacy-NNNN / path / bare-number; returns canonical absolute path or
  a structured error.
- Width-tolerance scan behaviour as described in "Pattern changes after first
  use".
- Tests for both scripts: corpus of `0001-foo.md`, `0002-bar.md`,
  `PROJ-0001-baz.md` simulated under various patterns; verify counters scope per
  project, legacy IDs still resolve, ambiguous bare numbers warn.

**Exit criteria.** A user with `work.id_pattern: "{project}-{number:04d}"` and
`work.default_project_code: "PROJ"` can run `work-item-next-number.sh` and get
`PROJ-0001`. Default-pattern users see no change. No SKILL.md changes yet.

**Depends on.** Phase 1.

### Phase 3 — Skill wiring

**Scope.** Update the work-item skill prose and globs to use the new primitives.
This is the phase where the feature becomes user-visible for non-batch flows.

**Deliverables.**

- Updates to `create-work-item/SKILL.md`, `update-work-item/SKILL.md`,
  `review-work-item/SKILL.md`, `list-work-items/SKILL.md`. Each replaces
  hard-coded `[0-9][0-9][0-9][0-9]-*.md` globs and the `^[0-9]+$` discriminator
  with calls to `work-item-resolve-id.sh` (lookup) and the compiled scan regex (
  listing).
- `list-work-items` discovery glob broadened to `*.md` with frontmatter
  validation, so legacy files remain visible after structural pattern changes.
- H1 and `work_item_id` frontmatter doc updated to use the full ID form.
- Parent-field normalisation prose updated to be pattern-aware (canonicalise to
  full ID, strip quotes, accept short and long forms).
- Skill-specific tests / evals updated where they assert filename shapes.

**Exit criteria.** All non-extract work-item skills work end-to-end on default
pattern (no behavioural change) and on `{project}-{number:04d}` pattern with
default project code. `extract-work-items` is *not* yet updated and may be
marked as known-limited in this phase.

**Depends on.** Phase 2.

### Phase 4 — `extract-work-items` interactive amendment

**Scope.** The multi-upstream-project UX — suggest-amend-confirm flow.

**Deliverables.**

- `extract-work-items/SKILL.md` updated with the new flow: parse source →
  suggest IDs using default project code → present table → user amends →
  allocate per-project → write.
- Per-project allocation calls (one
  `work-item-next-number.sh --project X --count N` per distinct project in the
  amended set, in original presentation order).
- Failure semantics preserved: partial allocation aborts the entire batch.
- Validation feedback: invalid project codes in amendments re-prompt rather than
  abort silently.
- Eval/test fixtures covering: single-default-project batch, mixed-project
  batch, all-overridden batch, validation failure mid-amendment.

**Exit criteria.** `extract-work-items` works for both default and configured
patterns; multi-project batches are handled cleanly via amendment.

**Depends on.** Phase 2 (allocator), Phase 3 (resolver patterns).

### Phase 5 — Migration framework: skip-tracking

**Scope.** Strictly additive extension to the migration framework. Independent
of all work-item changes. Could ship at any time.

**Deliverables.**

- `skills/config/migrate/scripts/run-migrations.sh` extended to read sibling
  state file `meta/.migrations-skipped` (newline-delimited migration IDs).
  Pending = NOT in applied AND NOT in skipped.
- New flags `--skip <id>` and `--unskip <id>` for explicit user control. Same
  atomic-append + unknown-ID-preservation semantics as `.migrations-applied`.
- `skills/config/migrate/SKILL.md` prose updated: skip semantics, flag
  descriptions, audit-trail explanation, summary line format (
  `applied: A; skipped: S; available: V`).
- Tests in the existing migrate test suite: skip a known migration → it stays
  pending-but-suppressed; unskip → reappears as pending; skip unknown ID →
  preserved on rewrite, warned about; interaction with
  `ACCELERATOR_MIGRATE_FORCE`.

**Exit criteria.** Skip/unskip works end-to-end. Existing migration `0001`
continues to apply or be skipped correctly. No format change to
`meta/.migrations-applied`.

**Depends on.** Nothing — can be developed in parallel with Phases 1-4.

### Phase 6 — Migration 0002: rename existing work items

**Scope.** Provide a migration that renames the legacy `NNNN-*.md` corpus to the
configured pattern and rewrites cross-references repo-wide.

**Deliverables.**

- New
  `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh`.
- Behaviour as detailed in the "Migration" subsection above: no-op when pattern
  lacks `{project}`; otherwise rename + frontmatter rewrite + repo-wide
  cross-reference rewrite (frontmatter ID-bearing fields, markdown links,
  strict-form prose references).
- Atomic per-file rename + frontmatter update; cross-reference rewrites batched
  after all renames using the final old→new mapping.
- Migration tests covering: pattern-lacks-project (no-op), single-project
  rename, frontmatter `parent`/`related`/`blocks`/`blocked_by`/`supersedes`/
  `superseded_by` rewrites, markdown-link rewrites in `meta/plans/` and
  `meta/research/`, strict-form prose references, and the case where some files
  are non-work-item but happen to live in `meta/work/` (skipped via frontmatter
  validation).
- A test corpus checked into the test fixtures: a small `meta/` tree containing
  work items with parent links, plans referencing work items, and research
  documents with both inline and link references.
- Documentation in the migration script header explaining what is and is not
  rewritten (bare digits in prose explicitly excluded).

**Exit criteria.** Running `/accelerator:configure migrate` after setting
`work.id_pattern: "{project}-{number:04d}"` and
`work.default_project_code: "PROJ"` correctly renames all legacy work items,
rewrites all cross-references in `meta/`, and leaves the
`meta/.migrations-applied` audit trail in good order. Users who do not want this
migration can `--skip 0002-rename-work-items-with-project-prefix` (Phase 5
dependency).

**Depends on.** Phase 3 (the new resolver/listing primitives must be live so
post-migration state is consistent), Phase 5 (skip-tracking lets users opt out).

### Cross-cutting

Across all phases:

- **Bash-only** as confirmed; helper scripts factored when they exceed ~30 lines
  or have non-trivial test surface, so the future Rust port has clean
  transliteration targets.
- **No dependency on additional CLI tools** beyond what's already in the
  plugin (bash, awk, sed, grep, find).
- **README and CHANGELOG** updates only after Phase 4 (when the feature is
  end-to-end usable). Phase 5 may merit its own CHANGELOG line.
- **Version bump** strategy: minor bump after Phase 4 (user-visible feature
  complete); patch on Phase 5 if shipped standalone; minor again after Phase 6 (
  migration tool). The plan author may prefer bundling.

## Architecture Insights

- **The pattern is a "prefix shape" abstraction, not a "filename shape"
  abstraction.** The slug-suffix invariant (`-<slug>.md`) is preserved, which
  keeps the parsing logic in `list-work-items` (extract NNNN from filename
  prefix) structurally intact — only the regex compiles differently.
- **Per-project counter scoping falls out naturally** from substituting the
  project value into the scan regex. There is no need for a separate "counters"
  map or per-project allocator — the filesystem itself is the source of truth.
- **The change touches one artefact family (work items) but the scaffolding is
  reusable.** `work-item-pattern.sh` and `work-item-resolve-id.sh` could be
  lifted to a generic location later if ADRs or another family adopts
  configurable patterns. For now, keeping them under `skills/work/scripts/`
  reflects their actual scope.
- **The skipped-migrations track is a strictly additive extension to the
  existing migration framework.** No breaking change to
  `meta/.migrations-applied` format or semantics. Same atomic-append pattern,
  same forward-compat (unknown-ID preservation).
- **Bash-only is sustainable for this size of pattern compiler** but is
  approaching the edge. A pattern compiler with format-spec parsing, regex
  generation, and round-trip validation is on the order of 100-200 lines of
  bash + awk. The user has confirmed a future Rust CLI will replace it; the bash
  implementation should be straightforward enough that the Rust port is a
  transliteration rather than a rewrite.

## Historical Context

- `meta/research/2026-04-08-ticket-management-skills.md:891-897` (Resolved
  Question 1) — the original ticket-management research explicitly flagged this
  work as a future enhancement: *"A future enhancement will add configurable
  filename patterns — e.g., `{project-code}-{ticket-number}-{description}.md` —
  to support project/team code prefixes (`XXX-NNNN-description.md`) needed for
  eventual sync with external trackers (Jira, Linear, Trello). This
  configurability is **out of scope** for the initial implementation."* This
  research delivers exactly that follow-up.
- `meta/research/2026-04-08-ticket-management-skills.md:916-920` (Resolved
  Question 5) — confirms `meta/work/` is the local source of truth; sync with
  external trackers is a future enhancement that depends on this work.
- `meta/plans/2026-04-08-ticket-management-phase-1-foundation.md:128` — Phase 1
  plan called out "No configurable filename patterns (e.g.
  `XXX-NNNN-description.md`) — future" as an explicit non-goal.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — the framework
  under which `work.*` is added.
- `meta/decisions/ADR-0022-work-item-terminology.md` — establishes "work item"
  as the canonical term and notes the format-check requirement around
  hyphen-vs-space distinction (line 113), relevant to validating user-supplied
  project codes.
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — the
  framework being extended for skip-tracking.
- `meta/research/2026-03-18-adr-support-strategy.md` — establishes the parallel
  `ADR-NNNN` convention. This research deliberately leaves it alone.

## Related Research

- `meta/research/2026-04-08-ticket-management-skills.md` — original
  ticket-management research; the parent of this work.
- `meta/research/2026-03-22-skill-customisation-and-override-patterns.md` —
  config-system foundations.
- `meta/research/2026-03-27-skill-customisation-implementation-status.md` —
  status of the userspace config rollout.

### `extract-work-items` interactive amendment

`extract-work-items` produces multiple work items from a single source (e.g. a
brainstorm document or research write-up). With a configurable pattern, each
extracted item needs a project code. Behaviour:

1. After parsing source items but before allocation, the skill **suggests** an
   ID for each using `work.default_project_code` (or omits the project segment
   if no default and the pattern requires one).
2. Suggestions are presented as a table to the user — slug, suggested project
   code, and the projected ID (e.g. `PROJ-0042-add-user-login`). Numbers are not
   yet allocated; they are *projected* from the current
   `work-item-next-number.sh --project PROJ --count N` output for display
   purposes only.
3. **The user can amend** any project code per item before confirming. This is
   the path for multi-upstream-project batches: the user sets
   `default_project_code: "PROJ"` and overrides individual rows to `OTHER`,
   `THIRD`, etc.
4. On confirmation, the skill calls `work-item-next-number.sh` once per distinct
   project code in the amended set, in original presentation order. Per-project
   ordering preserves the existing semantic at
   `extract-work-items/SKILL.md:366-369`.
5. If any project code in the amended set fails validation (Validation rule 5
   above), the skill reports the offending row(s) and re-prompts.
6. Failure semantics on partial allocation match the existing behaviour at
   `extract-work-items/SKILL.md:376-381` — the entire batch is aborted; no
   numbers are committed.

This sidesteps the "single `--project` per batch" limitation flagged in the
original Phase 1 plan and gives the user explicit control without forcing them
to invoke the allocator manually for each upstream project.

## Resolved Questions

1. **Multi-project allocation in batch** — RESOLVED: `extract-work-items`
   suggests IDs using the default project code, presents them for amendment, and
   allocates per distinct project code on confirmation. See "extract-work-items
   interactive amendment" above.
2. **Project-code character set** — RESOLVED: uppercase **and** lowercase
   alphanumeric (`[A-Za-z][A-Za-z0-9]*`). Users are responsible for avoiding
   case-only collisions on case-insensitive filesystems.
3. **`work_item_id` frontmatter** — RESOLVED: stores the full ID (
   `work_item_id: "PROJ-0042"`). The migration rewrites all existing values.
4. **Cross-reference rewrite scope** — RESOLVED: rewrite across all of `meta/`.
   Scope detailed under "Migration" above (frontmatter fields, markdown links,
   strict-form prose references; bare digits in prose left untouched).
5. **Pattern changes after first use** — RESOLVED: silently allowed; existing
   work items are not migrated. Width-only changes preserve the counter via the
   scan-regex width tolerance; structural changes leave legacy files coexisting
   under their original IDs. See "Pattern changes after first use" above.

## Open Questions

None at this time. The design is complete and ready to be turned into an
implementation plan.
