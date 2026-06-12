---
type: codebase-research
id: "2026-06-11-0096-templates-view-auto-discovery"
title: "Research: Templates View Auto-Discovers Available Templates (0096)"
date: "2026-06-11T13:38:46+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0096"
parent: "work-item:0096"
relates_to: ["work-item:0042", "work-item:0089", "work-item:0029", "work-item:0037"]
topic: "Templates view auto-discovery from the templates/ directory"
tags: [research, codebase, visualiser, templates, write-visualiser-config, config-generation, template-resolver]
revision: "cc4054111f79f67229cdfa6fa9539b278b620f00"
repository: "visualisation-system"
last_updated: "2026-06-11T13:38:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Templates View Auto-Discovers Available Templates (0096)

**Date**: 2026-06-11T13:38:46+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: cc4054111f79f67229cdfa6fa9539b278b620f00
**Branch**: HEAD (detached — jj workspace `visualisation-system`)
**Repository**: visualisation-system

## Research Question

For the story at `meta/work/0096-templates-view-auto-discovers-templates.md`: how does the
templates view (`/library/templates`) currently get its list of templates, and what
exactly needs to change so that the list is auto-discovered from the `templates/`
directory at config-generation time instead of being driven by a hardcoded roster?
Specifically — where is the hardcoded roster, is the rest of the chain (jq → config.json →
Rust server → API → React view) genuinely name-agnostic, what tests pin the current
roster, and are the story's Technical Notes accurate?

## Summary

The story's central thesis is **confirmed and accurate**: the templates view's list is
driven entirely by a hardcoded eight-name roster in the launcher script
`write-visualiser-config.sh`, while **13** `*.md` templates exist on disk. Everything
downstream of `config.json` — the Rust server (`TemplateResolver`, the `/api/templates`
handler, the watcher) and the React view (`LibraryTemplatesIndex`) — is **fully
name-agnostic** and requires **zero production-code change**. The fix is to replace the
eight fixed `template_tier` calls with a loop over the already-sourced
`config_enumerate_templates` helper, and to restructure the single monolithic `jq -n` so
the `templates` object is built dynamically and spliced in via one `--argjson`.

The story's helper references, glyph caveat (`rca` is the one currently-hidden template
lacking a glyph), three-tier shape, and the `config.rs:433` / `test-launch-server.sh:87-92`
test locations are all **verified correct** against the workspace checkout.

**One material gap was found.** The story's Technical Notes claim *"only two test locations
(`config.rs:433`, `test-launch-server.sh:87-92`) need updating."* There is a **third**
roster-pinning test the story does not mention: the launcher contract test at
`server/tests/config_contract.rs:72,77`, which asserts `cfg.templates.len() == 8` and
iterates the same eight hardcoded names asserting each `plugin_default` path. It runs
`write-visualiser-config.sh` for real and **will break** when discovery surfaces 13
templates. This must be added to the plan's "tests to update" list. Two further surfaces
(`frontend/e2e/start-server.mjs:76` and a `config.valid.json` fixture name mismatch) are
discussed under Open Questions as things to be aware of, though neither strictly blocks the
change.

## Detailed Findings

### Area 1 — The hardcoded roster and config assembly (`write-visualiser-config.sh`)

This is the single source of the view's list and the only production file that must change.
All line numbers verified against the workspace checkout
(`skills/visualisation/visualise/scripts/write-visualiser-config.sh`).

**The roster (lines 121-128)** — not an array; each template is resolved individually into
its own scalar shell variable via `template_tier <literal-name>`:

```bash
121  ADR="$(template_tier adr)"
122  PLAN="$(template_tier plan)"
123  RES="$(template_tier codebase-research)"
124  VAL="$(template_tier validation)"
125  PRD="$(template_tier pr-description)"
126  WI="$(template_tier work-item)"
127  DGAP="$(template_tier design-gap)"
128  DINV="$(template_tier design-inventory)"
```

The eight roster names are exactly `adr`, `plan`, `codebase-research`, `validation`,
`pr-description`, `work-item`, `design-gap`, `design-inventory`. The five on-disk templates
never wired in are `plan-review`, `pr-review`, `work-item-review`, `rca`, and `note`.

**The `template_tier` helper (lines 88-119)** performs the three-tier path resolution:
- Root paths set just above (lines 85-86): `TEMPLATES_USER_ROOT="$(abs_path templates)"`
  (project user-override dir) and `TEMPLATES_PLUGIN_ROOT="$PLUGIN_ROOT/templates"`
  (plugin-default dir, the canonical roster source 0096 will scan).
- **Tier 1 — config-override** (lines 90-97): reads `templates.<name>` from project config
  via `config-read-value.sh`; emits `null` when absent.
- **config-override provenance** (lines 98-111): scans `config.local.md` then `config.md`
  frontmatter to record which file declared the override.
- **Tier 2 / Tier 3 — user-override & plugin-default** (lines 115-116): the candidate paths
  `"$TEMPLATES_USER_ROOT/$name.md"` and `"$TEMPLATES_PLUGIN_ROOT/$name.md"`, emitted
  unconditionally (the Rust server decides which tier is actually `present`).
- **Emits** (lines 113-118) a compact JSON object with exactly four keys:
  `{config_override, user_override, plugin_default, config_override_source}`. This shape
  must be preserved **exactly** — the Rust `TemplateTiers` struct is
  `#[serde(deny_unknown_fields)]` (see Area 2), so any extra/missing key fails
  deserialisation.

**The monolithic `jq -n` (lines 256-315)** — each roster variable is passed as a
**statically named** `--argjson`, then hand-mapped to a display key in a `templates` object
literal:

```bash
273  --argjson adr "$ADR" --argjson plan "$PLAN" --argjson research_t "$RES" \
274  --argjson validation "$VAL" --argjson pr_description "$PRD" \
275  --argjson work_item_template "$WI" \
276  --argjson design_gap "$DGAP" \
277  --argjson design_inventory "$DINV" \
...
303    templates: {
304      adr: $adr, plan: $plan, "codebase-research": $research_t,
305      validation: $validation, "pr-description": $pr_description,
306      "work-item": $work_item_template,
307      "design-gap": $design_gap,
308      "design-inventory": $design_inventory
309    },
```

Note the triple indirection — shell var (`RES`), jq arg (`$research_t`), output key
(`"codebase-research"`) are all distinct, and hyphenated keys are quoted. **Crucially, the
on-disk basenames already equal the output display keys** for all eight (`codebase-research`,
`pr-description`, `work-item`, `design-gap`, `design-inventory`, `adr`, `plan`,
`validation`), so discovery keyed on basename reproduces all current keys exactly plus the
five new ones — no display-key remapping is lost.

**The required restructure** — a variable-length set cannot use statically-named args. Build
the whole `templates` object into one shell variable (accumulate `name → tier-JSON` pairs)
and splice via a single `--argjson templates "$TEMPLATES_JSON"`, replacing lines 303-309
with `templates: $templates,`. The analyser suggested folding per-name results with, e.g.,
`jq -Rn 'reduce inputs as $l ({}; ($l|split("\t")) as [$k,$v] | .[$k] = ($v|fromjson))'`.
This jq splice is the only structurally non-trivial part of the change — matching the
story's "S" sizing.

### Area 2 — The directory-scan helper already exists and is already sourced

`scripts/config-common.sh:139-149` (plugin root) defines `config_enumerate_templates`:

```bash
139  config_enumerate_templates() {
140    local plugin_root="$1"
141    local templates_dir="$plugin_root/templates"
142    if [ ! -d "$templates_dir" ]; then
143      return 0
144    fi
145    for f in "$templates_dir"/*.md; do
146      [ -f "$f" ] || continue
147      basename "$f" .md
148    done
149  }
```

- Takes the **plugin root** and appends `/templates` itself (line 141) — so it scans the
  same directory as `TEMPLATES_PLUGIN_ROOT`.
- Uses the repo's bash 3.2-safe idiom: a plain `for f in "$dir"/*.md` loop with a per-item
  `[ -f "$f" ] || continue` guard and an early `return 0` for a missing dir — **no
  `nullglob`/`globstar`**. The `-f` guard is what makes the zero-match case safe (the
  literal `*.md` glob string fails `-f` and is skipped). This directly satisfies the
  story's K∈{0,1,3,13} acceptance criterion: 0 files → no output; the loop is one-to-one
  with files.
- Outputs one basename-without-`.md` per line.

`write-visualiser-config.sh:6` **already** sources `config-common.sh`, so no new `source` is
needed. Two sibling scripts are exact precedents for the iteration idiom:
- `scripts/config-list-template.sh:21`: `for KEY in $(config_enumerate_templates "$PLUGIN_ROOT"); do`
- `scripts/config-eject-template.sh:121`: same pattern driving the `--all` branch.

Discovery therefore reduces to: `for KEY in $(config_enumerate_templates "$PLUGIN_ROOT"); do
... template_tier "$KEY" ...` — minimal new code.

### Area 3 — The Rust server is fully name-agnostic (zero production change)

- **`server/src/config.rs:29`** — `pub templates: HashMap<String, TemplateTiers>`. An
  arbitrary-keyed map; any number of arbitrarily-named entries deserialise without code
  change. `TemplateTiers` (≈ lines 247-260) is `#[serde(deny_unknown_fields)]` with exactly
  `config_override`, `user_override`, `plugin_default`, `config_override_source` — this is
  why the launcher's per-template shape must match exactly, but the *set* of names is
  unconstrained.
- **`server/src/templates.rs`** — `TemplateResolver::build` (≈116-209) iterates
  `for (name, tiers) in templates` straight off the config map (no directory scan, no
  hardcoded list); `list` (≈211-235) iterates the internal `by_name` map and sorts
  alphabetically. The per-template shape is `TemplateSummary { name, tiers: Vec<TemplateTier>,
  active_tier }`.
- **`server/src/api/templates.rs`** — `templates_list` returns `resolver.list()` verbatim;
  no filtering, counting, or name matching.
- **`server/src/server.rs:91-99`** builds the resolver from `&cfg.templates`; the watcher
  (`server/src/watcher.rs:369-441`) derives its change-detection key set from
  `cfg_templates.keys()` — again all config-driven.

**Verdict: the server correctly serves any set of templates present in `config.json`,
regardless of count or names, with no code change.**

### Area 4 — The React view is fully name-agnostic with soft glyph fallback

Verified chain (`frontend/`):
- `src/api/fetch.ts:150-154` — `fetchTemplates()` GETs `/api/templates`, returns the body
  verbatim (no per-entry normalisation).
- `src/api/types.ts:175-179` — `TemplateSummary { name, tiers: TemplateTier[], activeTier }`;
  `TemplateTierSource` is the three-value union `config-override | user-override |
  plugin-default` (types.ts:154-157). No new metadata is needed (matches the story).
- `src/routes/library/LibraryTemplatesIndex.tsx` — `TemplatesIndexList` (≈92-128) does a
  single `.map` over `data.templates` with **no allow-list, filter, or per-name `switch`**.
  Every returned template yields a clickable row labelled `{name}.md`. `TierPills` (≈60-85)
  always renders all three tier slots in fixed `TIER_ORDER`
  (`plugin-default → user-override → config-override`), styling each by a `data-state` of
  `absent`/`present`/`active` derived from `tier.present`/`tier.active` — this directly
  satisfies the story's tier-presence acceptance criterion.
- **Glyph fallback** (LibraryTemplatesIndex.tsx:111-117): `glyphKeyForTemplate(t.name)`
  returns `DocTypeKey | null`; on `null` an empty `<span className={styles.rowGlyphFallback}/>`
  renders. The row still renders fully — a missing glyph never blocks "appearing".

**Glyph caveat — story claim verified precisely.** `STEM_TO_GLYPH`
(`src/routes/library/template-tier.ts:31-59`) plus the suffix-matching `glyphKeyForTemplate`
(67-80) resolve the five currently-hidden templates as:

| Hidden template | Glyph? | Resolves via |
|---|---|---|
| `plan-review` | ✅ | exact key `plan-review` → `plan-reviews` (line 51) |
| `pr-review` | ✅ | exact key `pr-review` → `pr-reviews` (line 53) |
| `work-item-review` | ✅ | exact key `work-item-review` → `work-item-reviews` (line 55) |
| `note` | ✅ | exact key `note` → `notes` (line 57) |
| `rca` | ❌ | no key; `["rca"]` has no matching suffix → `null` → blank fallback |

The story's claim that **only `rca`** lacks a glyph is **correct**. (Note `codebase-research`
has no literal key but resolves via the `research` suffix.) Adding an `rca` stem entry is a
non-blocking 0037 follow-on.

### Area 5 — Tests that pin the roster (the story under-counts these)

| # | Location | What it pins | In story's "tests to update"? |
|---|---|---|---|
| 1 | `server/src/config.rs:433` | `assert_eq!(c.templates.len(), 8)` (in `parses_valid_config`, against `tests/fixtures/config.valid.json`); also `.get("adr")` at ~434 | ✅ Yes |
| 2 | `scripts/test-launch-server.sh:87-92` | Six per-template `.templates.<name>.user_override` path assertions (`adr`, `plan`, `codebase-research`, `validation`, `pr-description`, `work-item`) | ✅ Yes |
| 3 | **`server/tests/config_contract.rs:72,77`** | `assert_eq!(cfg.templates.len(), 8)` **and** a loop over the eight hardcoded names asserting each `plugin_default` ends with `{name}.md` — **runs `write-visualiser-config.sh` for real** | ❌ **No — story omits this** |

**Finding #3 is the key correction to the story.** `config_contract.rs` is the launcher's
own contract test (verified present, 2865 bytes, May 13). It executes the real launcher and
pins both the count (8) and the eight names — it **will break** when discovery emits 13.
Because it exercises the real script (not a fixture), it is arguably the *most* important
test to update and should ideally be turned into a discovery-aware assertion (e.g. assert
the emitted set equals `config_enumerate_templates` output, or at minimum bump to 13 + the
full name list including `plan-review`, `pr-review`, `work-item-review`, `rca`, `note`).

`scripts/test-write-visualiser-config.sh` was checked — it asserts only an editor-template
round-trip (≈ lines 330-340) and **pins nothing about the roster**, matching the story.

### Area 6 — Out-of-scope rosters (correctly excluded by the story)

- **`scripts/config-defaults.sh:66-72`** — `TEMPLATE_KEYS` (a **6**-entry, order-locked
  array: `plan`, `codebase-research`, `adr`, `validation`, `pr-description`, `work-item`),
  consumed by `config-dump.sh:185` and pinned hard by `scripts/test-config.sh:2460-2465`
  (length 6 + exact order). This is the config-CLI surface the story explicitly scopes to
  0029. **Leave untouched** — but note it is a *third distinct* roster (8 in the launcher, 6
  in the CLI, 13 on disk), reinforcing that drift is endemic and 0096 only fixes the view's
  surface.

## Code References

- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:121-128` — the eight-name roster (the thing to remove)
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:88-119` — `template_tier` three-tier helper (reuse as-is, fed by discovery)
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:85-86` — `TEMPLATES_USER_ROOT` / `TEMPLATES_PLUGIN_ROOT`
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:273-277` — static `--argjson` flags (to be collapsed to one)
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:303-309` — the `templates` object literal (to be replaced with `templates: $templates`)
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:6` — already sources `config-common.sh`
- `scripts/config-common.sh:139-149` — `config_enumerate_templates` (the drop-in scanner)
- `scripts/config-list-template.sh:21`, `scripts/config-eject-template.sh:121` — precedents for the `for KEY in $(config_enumerate_templates …)` idiom
- `skills/visualisation/visualise/server/src/config.rs:29` — `templates: HashMap<String, TemplateTiers>` (name-agnostic container)
- `skills/visualisation/visualise/server/src/config.rs:247-260` — `TemplateTiers` `deny_unknown_fields` (per-template shape contract)
- `skills/visualisation/visualise/server/src/config.rs:433` — `templates.len() == 8` assertion (**update**)
- `skills/visualisation/visualise/server/tests/config_contract.rs:72,77` — `len()==8` + 8-name loop (**update — story omits this**)
- `skills/visualisation/visualise/scripts/test-launch-server.sh:87-92` — 6 per-template path assertions (**update**)
- `skills/visualisation/visualise/server/src/templates.rs:116-235` — config-map-driven `build`/`list`
- `skills/visualisation/visualise/server/src/api/templates.rs:17-24` — `/api/templates` handler (verbatim passthrough)
- `skills/visualisation/visualise/frontend/src/api/fetch.ts:150-154` — `fetchTemplates`
- `skills/visualisation/visualise/frontend/src/api/types.ts:154-183` — `TemplateSummary` / tier types
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx:60-128` — name-agnostic list + `TierPills` + glyph fallback
- `skills/visualisation/visualise/frontend/src/routes/library/template-tier.ts:31-80` — `STEM_TO_GLYPH` + `glyphKeyForTemplate`
- `scripts/config-defaults.sh:66-72` — `TEMPLATE_KEYS` (out of scope — 0029)
- `templates/` (plugin root) — 13 `*.md` files confirmed on disk

## Architecture Insights

- **Single point of truth, by design downstream.** The whole chain past `config.json` is
  data-driven: a `HashMap` in Rust, a `.map` in React, an alphabetical sort in the resolver.
  The *only* place that imposes a fixed roster is the launcher's config-assembly. This is
  exactly why 0096 is an "S" — the leverage point is one script.
- **The launcher's static-arg jq pattern is the friction.** The config is assembled in one
  `jq -n` with statically-named `--argjson`s. A variable-length set is fundamentally
  incompatible with statically-named args, forcing the templates object to be built
  separately and spliced. This is the one genuinely fiddly edit.
- **Build-time discovery is the natural seam.** There is no compiled build for templates and
  the server reads content at runtime, so config-generation is the discovery point.
  Consequence (per the story): a new/removed template file is reflected on next launch, not
  live. The server *does* have a runtime directory watcher (`watcher.rs`) but it watches
  template *content* for the already-configured set, not the *membership* of `templates/` —
  so it is not a runtime-discovery path and the build-time decision stands.
- **Roster drift is endemic and multi-surface.** Three independent rosters disagree today: 8
  (launcher), 6 (`TEMPLATE_KEYS` CLI), 13 (disk). 0096 fixes only the launcher/view surface;
  the CLI drift remains for 0029. Worth stating plainly in the plan so the partial fix is
  intentional, not an oversight.
- **`deny_unknown_fields` is a useful guardrail.** It guarantees a malformed dynamic build
  fails loudly at server boot rather than silently dropping templates — the implementer
  should rely on it (and `cargo test`) to catch a botched jq splice.

## Historical Context

- `meta/decisions/ADR-0017-configuration-extension-points.md` — establishes the three-tier
  template resolution hierarchy (config-override → user-override → plugin-default) that 0096
  must preserve for every discovered template. **The contract to honour.**
- `meta/decisions/ADR-0021-template-management-subcommands.md` — decides the template-management
  CLI surface built on ADR-0017; defines the separate `TEMPLATE_KEYS` roster 0096 leaves
  untouched (0029 territory).
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — the two-tier config-file
  scheme (`config.md` / `config.local.md`) underlying the config-generation step.
- `meta/plans/2026-05-05-add-missing-templates-to-visualiser.md` — **most directly relevant
  prior art**: documents the manual `write-visualiser-config.sh` wiring and the on-disk vs
  wired drift — i.e. the exact manual step 0096 eliminates.
- `meta/work/0042-templates-view-redesign.md` + `meta/research/codebase/2026-05-18-0042-templates-view-redesign.md`
  + `meta/plans/2026-05-18-0042-templates-view-redesign.md` — designed the tier-presence
  index and detail view 0096 populates with discovered entries (0096 changes the source of
  the list, not the look).
- `meta/work/0037-glyph-component.md` + `meta/research/codebase/2026-05-12-0037-glyph-component.md`
  — the `DocTypeKey`-keyed glyph map; relevant to the `rca` glyph follow-on.
- `meta/work/0029-template-management-subcommand-surface.md` — the out-of-scope CLI roster.
- `meta/work/0089-templates-preview-whitespace-fix.md` — independent change to the same view
  surface; no ordering dependency with 0096.
- `meta/reviews/work/0096-templates-view-auto-discovers-templates-review-1.md` — prior review
  of the work item itself.

## Related Research

- `meta/research/codebase/2026-05-18-0042-templates-view-redesign.md` — the templates view's
  three-tier model and SSE/watcher plumbing.
- `meta/research/codebase/2026-03-29-template-management-subcommands.md` — the configure-skill
  template subcommands and override mechanics.
- `meta/research/codebase/2026-05-12-0037-glyph-component.md` — the glyph map design.

## Open Questions

- **`config_contract.rs` is the unlisted third test surface (action required).** The plan
  should add `server/tests/config_contract.rs:72,77` to the "tests to update" set, and
  preferably rewrite it to assert against `config_enumerate_templates` output rather than
  re-pinning a literal 13 (which would just reintroduce drift in test form). This is the one
  substantive correction to the story's Technical Notes.
- **e2e fixture does not exercise discovery.** `frontend/e2e/start-server.mjs:76` builds its
  own five-name templates fixture (`adr`, `plan`, `research`, `validation`, `pr-description`)
  for a mock server — it does **not** run `write-visualiser-config.sh`, so it won't *break*,
  but it also won't *verify* the discovery behaviour. Confirming the acceptance criteria
  (K∈{0,1,3,13}, add/remove reflected) likely belongs in a launcher-level test
  (`test-write-visualiser-config.sh`, which currently pins nothing about the roster and is
  the natural home) rather than the frontend e2e suite.
- **Fixture name mismatch to be aware of.** `server/tests/fixtures/config.valid.json` uses
  the key `research` (not `codebase-research`); `config_contract.rs` expects the launcher to
  emit `codebase-research`. These two Rust test surfaces already disagree on one name, so
  updating one does not automatically fix the other — touch them independently and
  deliberately.
- **Should `test-write-visualiser-config.sh` gain roster coverage?** It is the launcher's own
  unit test and currently asserts nothing about templates. Adding the K∈{0,1,3,13} count
  cases here (with a temp `templates/` dir) would directly cover the story's acceptance
  criteria at the right altitude — worth proposing in the plan.
