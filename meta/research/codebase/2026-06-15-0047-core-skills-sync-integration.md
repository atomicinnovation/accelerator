---
type: codebase-research
id: "2026-06-15-0047-core-skills-sync-integration"
title: "Research: Core Skills Sync Integration (story 0047)"
date: "2026-06-15T21:25:19+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0047"
parent: "work-item:0047"
relates_to: ["codebase-research:2026-06-14-0048-linear-integration-apis", "codebase-research:2026-05-08-0046-work-management-system-configuration"]
topic: "Core Skills Sync Integration"
tags: [research, codebase, work-management, integrations, sync, list-work-items, create-work-item]
revision: "7e4e9358c62ccb9863763c6aa42e0ffce439bac0"
repository: "ticket-management"
last_updated: "2026-06-15T21:25:19+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Core Skills Sync Integration (story 0047)

**Date**: 2026-06-15T21:25:19+00:00
**Author**: Toby Clemson
**Git Commit**: 7e4e9358c62ccb9863763c6aa42e0ffce439bac0
**Branch**: HEAD (workspace: ticket-management)
**Repository**: ticket-management

## Research Question

For story 0047 (Core Skills Sync Integration): how do `/list-work-items` and
`/create-work-item` work today, and what exactly must change to (a) render a
colour-coded synced/unsynced label per item in `/list-work-items`, and (b) offer
an interactive push-to-remote in `/create-work-item` after drafting but before
the local file is written? What existing scripts, conventions, and precedents
support these changes, and what tensions or gaps will the implementing plan have
to resolve?

## Summary

The story is well-scoped against the codebase, but the research surfaced **three
concrete tensions the plan must resolve up front**:

1. **Schema-key collision (the biggest one).** `/create-work-item` writes the
   own-identity key as **`id`** (a quoted string) and already reserves
   **`external_id`** for cross-system pointers. Story 0047 and the Linear
   precedent both write **`work_item_id`**. New files do not get a `work_item_id`
   line at all. This matters because the writeback primitive
   `config_set_frontmatter_field` **replaces** an existing frontmatter line and
   returns exit 5 ("field not found") if the key is absent — it cannot *insert*.
   So the push-on-accept flow cannot reuse `linear-create-flow.sh`'s writeback
   verbatim; the plan must decide which key holds the remote-allocated value and
   how the line gets created.

2. **Write-ordering inversion vs. the Linear precedent.**
   `linear-create-flow.sh` is **file-first**: it requires the local file to
   already exist on disk, reads title/body/work_item_id out of it, creates the
   remote issue, then writes the key back in place. Story 0047 wants the
   **opposite**: do *not* write the local file until the push succeeds, the user
   declines, or fallback-to-local is confirmed. The reusable part is the
   *create-then-validate-identifier* sequence and the loud non-idempotent
   failure stance — not the file lifecycle.

3. **No colour anywhere.** There is zero ANSI/colour output in the entire
   codebase (`scripts/`, `hooks/`, every skill). `/list-work-items` renders a
   markdown table and a `canonical-tree-fence` block, both plain text. Adding
   colour is a brand-new pattern that must satisfy the bash 3.2 floor and the
   bashisms linter, and the only terminal-awareness precedent is a TTY-detection
   idiom in the migrate skill.

Everything else the story claims checks out: the `work.integration` gate exists
and validates; `work-item-read-field.sh` transparently bridges `id`↔`work_item_id`;
the per-item-status-slot seam that 0051 depends on is a clean extension point;
and the synced/unsynced classification is fully derivable locally with no remote
read.

## Detailed Findings

### `/list-work-items` — current behaviour and the two seams

File: `skills/work/list-work-items/SKILL.md`

**ID is derived from the filename, not frontmatter.** Step 2 globs `*.md`, gates
each file with `wip_is_work_item_file` (frontmatter present + non-empty `id`/
`work_item_id`), then derives the displayed ID via
`wip_extract_id_from_filename` (`SKILL.md:159-166`). The filename prefix is
explicitly *authoritative* over the frontmatter `id`/`work_item_id` value
(restated as a Quality Guideline at `SKILL.md:310-314`). So the frontmatter ID
value is read only as a *gate*, never displayed — story 0047's requirement to
read the `work_item_id`/`id` *value* via `work-item-read-field.sh` is genuinely
net-new to this scan.

**The frontmatter scan extracts exactly six fields.** A single-pass `awk`
emits all frontmatter lines (`SKILL.md:134-149`); the model then parses
`title, kind, status, priority, tags, parent` (`SKILL.md:181-190`) — matching
the story exactly. The ID-shape read must be added here.

**Two render surfaces, both plain text** (`SKILL.md:220-262`):
- Default table — header `| ID | Title | Kind | Status | Priority |`
  (`SKILL.md:226`), with an all-`—`-column-suppression rule
  (`SKILL.md:230-233`). A sync-status label is a new column here.
- Hierarchy/tree — per-item template
  `NNNN — title (kind: <kind>, status: <status>)` inside a
  `<!-- canonical-tree-fence -->` block (`SKILL.md:246-251`). **This fence is
  almost certainly asserted verbatim by a test**, so its exact content is
  load-bearing; a label appended in tree mode changes it.

**Existing label-like precedent**: the only inline annotations today are
`(parent NNNN not found)` and `(cycle)` in tree mode (`SKILL.md:258-261`). There
is no colour and no status→label lookup anywhere.

**Config reads** (`!`-preprocessor, top of file): context/skill-context/agents
(`:14-16`), `config-read-path.sh work` (`:24`), `config-read-work.sh id_pattern`
(`:25`), `config-read-work.sh default_project_code` (`:26`),
`config-read-template.sh work-item` (`:34`), and trailing skill-instructions
(`:319`). **There is no `work.integration` read today** — it must be added
alongside these.

**`allowed-tools` scope** (`SKILL.md:7-9`) is limited to
`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` and
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)`. Calling
`work-item-read-field.sh` (under `skills/work/scripts/`) is already permitted;
`config-read-work.sh` is already permitted. A new colour helper outside these two
prefixes would require widening the allowlist.

### `/create-work-item` — the draft-then-write lifecycle

File: `skills/work/create-work-item/SKILL.md`

**The "drafted but not written" window is large and explicit.** Steps 0–4 are
entirely conversational — no disk write, and crucially **no ID allocation**
during the Step 4 review loop (`SKILL.md:346`, `:378-379`). The push offer slots
in here: after Step 4 approval (`:367-379`) and before the Step 5 write.

**The single write happens at `SKILL.md:489`** (new-item path, Step 5 item 4).
Step 5 first allocates the ID via `work-item-next-number.sh` (`:406-411`),
resolves the path (`:423`), guards against an existing path (`:427-432`), then
substitutes frontmatter and writes once. The enrich-existing path has its own
gated write at `:562`.

**ID allocation is local-only and key `id`.** `work-item-next-number.sh` scans
the work dir for the highest existing number and increments — it reflects only
local files, so a remote push does not affect it. The allocated value is written
under **`id:`** as a quoted string (`SKILL.md:448-450`), and substituted into the
body H1 (`:487-488`). `work-item-resolve-id.sh` is read/enrich-side only; it does
not allocate.

**The new-item path has no scripted y/n gate** — it relies on the Step 4 draft
approval plus the guideline "Never write a file without explicit user approval"
(`SKILL.md:576`). The *scripted* fail-safe gate exists only on the
enrich-existing path (`:524-551`): exactly `y`/`Y` proceeds, exactly `n`/`N`
declines, anything else is treated as `n`. **This is the template to copy** for
the push-offer prompt.

**Frontmatter the new file gets** (`SKILL.md:445-486`): `type, id, title, date,
author, producer, status: draft, last_updated, last_updated_by, schema_version`,
plus omit-by-default linkage keys. Notably **`external_id`** already exists as an
omit-by-default "cross-system pointer (e.g. a Jira/Linear key)" (`:484-486`) — a
candidate home for the remote-allocated key (see Architecture Insights). New
files never write `work_item_id`; that key is read-only legacy (`:595-604`).

**Config reads**: context/skill-context/agents (`:13-15`),
`config-read-path.sh work` (`:23`), `config-read-template.sh work-item` (`:31`),
trailing skill-instructions (`:675`). **No `work.*` read in SKILL.md today** —
`work.*` values are read only inside scripts. A `work.integration` read must be
added.

### The `work.integration` gate — confirmed present and validating

- `scripts/config-read-work.sh` — call `config-read-work.sh integration` with the
  bare subkey (it prepends `work.`). Validation at `:46-58` only fires for a
  *non-empty* value, hard-failing via `log_die` if it is not one of the allowed
  values. **Not-configured prints an empty line and exits 0** — so both skills
  must gate on empty-string output, not on exit code.
- `scripts/config-defaults.sh:91-96` — `WORK_INTEGRATION_VALUES=(jira linear
  trello github-issues)`; empty is additionally permitted (the unset default,
  `:82`). `paths.integrations` default is `.accelerator/state/integrations`
  (`:59`).

### `work-item-read-field.sh` — the id↔work_item_id bridge

File: `skills/work/scripts/work-item-read-field.sh`

CLI: `work-item-read-field.sh <field-name> <file>`. Prints the value
(quote-stripped) to stdout, exit 0; exit 1 on any failure (bad args, missing
file, malformed frontmatter, or field genuinely absent). The bridge
(`:81-100`): asking for `id` falls back to `work_item_id` if `id` is absent, and
vice-versa — **but only for those two keys, and only when the requested key is
absent** (first-match-wins; if a file carries *both*, the requested key wins and
the alias is never consulted). For `/list-work-items`, reading `work_item_id`
returns whichever key the file actually carries; a non-zero exit means neither is
present.

### The Linear create-then-writeback precedent

File: `skills/integrations/linear/scripts/linear-create-flow.sh`

Flow: dependency preflight → arg parse → file-readable guard (`E_CREATE_NO_FILE`,
100) → extract frontmatter → read `work_item_id`+`title` → **already-synced
guard** → title guard → catalogue/team-id resolve → build payload (title +
trimmed Markdown body) → (`--print-payload` dry-run returns here) → GraphQL
`issueCreate` → **validate returned identifier** → **writeback** → print bare
identifier.

- **Already-synced rejection** (`:134-142`): `LINEAR_IDENTIFIER_RE` =
  `^[A-Z][A-Z0-9]*-[0-9]+$` (`:47`). A numeric `work_item_id` proceeds; a
  remote-format value (even quoted) fires `E_CREATE_ALREADY_SYNCED` (102).
- **Identifier validation before any write** (`:197-205`): empty/non-conforming →
  `E_CREATE_BAD_IDENTIFIER` (106), file left untouched. Injection guard.
- **Writeback** (`:207-213`):
  `config_set_frontmatter_field "$file" work_item_id "$identifier"`; on failure,
  `E_CREATE_WRITEBACK_FAILED` (107) with a loud "do NOT blindly re-run — it would
  create a duplicate" message. This is the house stance: a non-idempotent op that
  half-succeeds does **not** auto-retry; it hands recovery to the user.
- Exit-code band 100–109 is reserved to this script (`EXIT_CODES.md`); transport
  codes 11–36 from `linear-graphql.sh` propagate verbatim.
- **Critical for 0047**: this script presupposes an existing file and only mutates
  it in place. It has *no* code path that creates a file. The new flow must defer
  file creation until after the validated identifier is in hand.

### The writeback primitive — `config_set_frontmatter_field`

File: `scripts/config-common.sh:122-209`

Signature `config_set_frontmatter_field <file> <key> <value>`. Replaces a
**top-level, already-present, exactly-once** frontmatter field via env-passed awk
(injection-safe), re-verifies integrity, then `atomic_write`s. **Returns 1 if the
field is absent (awk exit 5) — it cannot insert a new key.** It adds no YAML
quoting (caller's responsibility). This is the constraint behind tension #1: to
land a remote key on a freshly-drafted file, either the frontmatter must already
contain the target key (so it can be replaced) or the value must be substituted
into the frontmatter block *before* the single write (the more natural fit, since
the file isn't written until after the push anyway).

### Interactive-prompt and retry/fallback precedents

- **Fail-safe y/n write gate** (gold standard):
  `skills/work/create-work-item/SKILL.md:524-551` — copy this for the push offer.
- **Confirm-before-network-call, three-way branch**:
  `skills/integrations/linear/create-linear-issue/SKILL.md:67-75`
  (confirm / revise / abort), with a preview rendered first.
- **Offer-to-push / decline-exit / guided-vs-express modes**:
  `skills/github/respond-to-pr/SKILL.md:61-63, 260-264, 454-457`.
- **Retry semantics**: there is **no** existing N-retry-in-memory loop. The house
  patterns are (a) loud-fail + idempotency guard + manual recovery
  (`linear-create-flow.sh:207-213`), and (b) build-state-then-commit-ledger-last
  (`skills/config/migrate/scripts/run-migrations.sh:293-294`). Story 0047's
  retry-then-fallback state machine is genuinely new; the closest "hold partial
  decisions across attempts" precedent is the migrate interactive resume-state
  file (`skills/config/migrate/scripts/interactive-lib.sh:300-306`).
- **Only terminal-awareness precedent** (no colour): TTY-detect fd routing at
  `skills/config/migrate/scripts/interactive-lib.sh:318-323`.

### The 0051 seam (what this story must leave behind)

`meta/work/0051-sync-work-items-skill.md` (`blocked_by: 0046, 0047`) places
`last-sync.json` under the configured integrations path
(`.accelerator/state/integrations/<system>/last-sync.json`, `:262-264`) with
schema `{ timestamp, items: { <work_item_id>: { remote_updated_at, local_hash }}}`
(`:265-267`). It derives the three baseline-dependent states (locally modified /
remotely modified / conflict) from the `(local-changed, remote-changed)` pair and
**explicitly reuses "the per-item status slot established by 0047"** (`:94-100`,
`:287-292`). So 0047's hard contract is: render the status as a
`status → {label, colour}` **lookup**, not a hardcoded binary, so 0051 adds three
keys without touching the rendering call site. 0051 also reuses 0047's
classification rule and `/create-work-item` confirmation UX style.

## Code References

- `skills/work/list-work-items/SKILL.md:134-149` — frontmatter scan (awk)
- `skills/work/list-work-items/SKILL.md:159-166` — ID derived from filename
- `skills/work/list-work-items/SKILL.md:181-190` — six fields parsed today
- `skills/work/list-work-items/SKILL.md:226` — default table header (add column)
- `skills/work/list-work-items/SKILL.md:246-251` — canonical-tree-fence template
- `skills/work/list-work-items/SKILL.md:7-9` — allowed-tools scope
- `skills/work/create-work-item/SKILL.md:367-379` — Step 4 draft-approval point
- `skills/work/create-work-item/SKILL.md:406-411` — local ID allocation
- `skills/work/create-work-item/SKILL.md:445-486` — new-file frontmatter (`id`, `external_id`)
- `skills/work/create-work-item/SKILL.md:489` — the single new-item write
- `skills/work/create-work-item/SKILL.md:524-551` — fail-safe y/n gate template
- `skills/work/create-work-item/SKILL.md:595-604` — `id` vs legacy `work_item_id`
- `scripts/config-read-work.sh:46-58` — `work.integration` validation
- `scripts/config-defaults.sh:91-96` — allowed integration values
- `scripts/config-defaults.sh:59` — `paths.integrations` default
- `scripts/config-common.sh:122-209` — `config_set_frontmatter_field` (replace-only)
- `skills/work/scripts/work-item-read-field.sh:81-100` — id↔work_item_id bridge
- `skills/integrations/linear/scripts/linear-create-flow.sh:47` — identifier regex
- `skills/integrations/linear/scripts/linear-create-flow.sh:197-213` — validate + writeback + loud fail
- `skills/config/migrate/scripts/interactive-lib.sh:318-323` — TTY-detect (only terminal-aware precedent)

## Architecture Insights

- **`work_item_id` vs `id` vs `external_id` is the central design decision for
  `/create-work-item`.** Three plausible plans: (a) write the remote key as
  `work_item_id` (matches the story text and the classification rule the labels
  depend on, but diverges from the new-file `id`-only convention and means the
  key must be substituted into the frontmatter pre-write, since
  `config_set_frontmatter_field` can't insert); (b) write it as `external_id`
  (already exists as an omit-by-default cross-system pointer, but then
  `/list-work-items`' synced/unsynced classifier and `work-item-read-field.sh`
  would need to read `external_id`, not `work_item_id`); (c) write both. The
  classification rule (`^[0-9]+$` → unsynced) and `work-item-read-field.sh`'s
  bridge both point at `work_item_id`/`id`, so option (a) is the path of least
  resistance for the *label* side — but the plan must reconcile it with the
  `id`-as-own-identity contract. **This is the first thing the plan should pin
  down.**
- **Defer-the-write is the clean ordering.** Because the file isn't created until
  after the push resolves, the remote key can simply be substituted into the
  in-memory frontmatter block before the single Write call — sidestepping
  `config_set_frontmatter_field`'s replace-only limitation entirely. The Linear
  writeback primitive is the precedent for *what* to write and *how loudly to
  fail*, not for the file lifecycle.
- **The status slot must be a data-driven lookup, not branching prose.** The
  verifiable form (per the 0047 review) is: "adding a `status → {text, colour}`
  entry yields a rendered label with no call-site edit." Design the rendering as
  a single lookup table consulted once per item.
- **Colour is net-new and constrained.** No ANSI exists anywhere; the bash 3.2
  floor and bashisms linter apply. The review suggests phrasing the requirement
  as "distinct, non-empty ANSI colour codes" so it's verifiable independent of
  palette. Consider whether colour is emitted by a script or described in the
  SKILL.md for the model to apply, and whether TTY-detection (the migrate idiom)
  is warranted.
- **Gate on empty-string, not exit code.** `config-read-work.sh integration`
  exits 0 with an empty line when unconfigured. Both skills must branch on the
  empty value.

## Historical Context

- `meta/work/0046-work-management-system-configuration.md` (**done**) — defines
  the `work.integration` single-string key, values `jira|linear|trello|
  github-issues`; declares the key only, provides no create path.
- `meta/work/0048-linear-integration.md` (**done**) — the concrete create
  capability 0047's push-on-accept invokes; originates the `work_item_id`
  writeback (no Jira skill writes the key back). Intentionally not `blocked_by`
  0047.
- `meta/work/0051-sync-work-items-skill.md` — owns `last-sync.json` and the three
  baseline-dependent states; consumes 0047's status slot and classification rule.
- `meta/reviews/work/0047-core-skills-sync-integration-review-1.md` — **APPROVE**;
  drove the scope decision to synced/unsynced-only, the presumed-synced framing,
  and the acyclic forward-data-dependency on 0051. Suggests verifiable phrasings
  for the status-slot AC and the colour requirement.
- `meta/work/0064-canonicalise-work-item-id-and-author-fields.md` — relevant to
  the `work_item_id`/`id` key canonicalisation tension.
- Jira integration plans/validations (2026-04-29 → 05-03) — sibling create-path
  prior art.

## Related Research

- `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md`
- `meta/research/codebase/2026-05-08-0046-work-management-system-configuration.md`
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`
- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md`

## Open Questions

1. **Which frontmatter key holds the remote-allocated value** — `work_item_id`,
   `external_id`, or both? The label classifier and `work-item-read-field.sh`
   favour `work_item_id`/`id`; the new-file convention favours `id` +
   `external_id`. (Plan-blocking — resolve first.)
2. **Colour mechanism** — emitted by a script vs. described in SKILL.md for the
   model; whether TTY-detection is applied; the exact distinct, non-empty ANSI
   palette for synced vs unsynced.
3. **Retry count** before fallback in `/create-work-item` (AC is
   count-independent; deferred to plan).
4. **Does the local numeric ID allocator need to run before or after the push?**
   `work-item-next-number.sh` is local-only and unaffected by the push, so either
   ordering works — but on push-accept, is a local number still allocated for the
   filename/slug while `work_item_id` holds the remote key? (The filename ID and
   the remote key are independent; the plan should state the intended
   relationship.)
5. **Tree-mode label and the `canonical-tree-fence`** — appending a label changes
   a likely test-asserted block. Confirm the test and update fixtures.
6. Minor: 0047's Technical Notes mis-cite `last-sync.json`'s location as the
   legacy `meta/integrations/` path; 0051 (the owner) correctly uses
   `.accelerator/state/integrations`. 0047 does not read the file, so behaviour is
   unaffected — but the plan should not echo the legacy path.
