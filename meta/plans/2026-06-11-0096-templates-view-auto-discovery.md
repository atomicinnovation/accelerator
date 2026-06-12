---
type: plan
id: "2026-06-11-0096-templates-view-auto-discovery"
title: "Templates View Auto-Discovers Available Templates Implementation Plan"
date: "2026-06-11T15:24:40+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0096"
parent: "work-item:0096"
derived_from: ["codebase-research:2026-06-11-0096-templates-view-auto-discovery"]
relates_to: ["work-item:0042", "work-item:0089", "work-item:0029", "work-item:0037"]
tags: [visualiser, templates, write-visualiser-config, config-generation]
revision: "5db2e80bebd0a75326aa12b8849020591402bfae"
repository: "visualisation-system"
last_updated: "2026-06-12T09:25:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Templates View Auto-Discovers Available Templates Implementation Plan

## Overview

The templates view (`/library/templates`) currently lists templates from a
hardcoded eight-name roster baked into the launcher's config-generation step
(`write-visualiser-config.sh`), while **thirteen** `*.md` templates exist on
disk. Five (`plan-review`, `pr-review`, `work-item-review`, `rca`, `note`) never
appear, and the list drifts further each time a template is added.

This plan replaces that roster with build-time discovery: the config-generation
step scans the plugin-default `templates/` directory and wires every discovered
`*.md` through the existing three-tier path resolution. The entire downstream
chain (`config.json` → Rust `TemplateResolver` → `GET /api/templates` → React
view) is already name-agnostic and needs **zero production change**. The only
production edit is to `write-visualiser-config.sh`.

## Current State Analysis

The view fetches its list at runtime from `GET /api/templates`. The server does
not scan `templates/`; it enumerates the keys of a `templates` map supplied in
the generated `config.json`. That map is built by `write-visualiser-config.sh`:

- **The roster** (`write-visualiser-config.sh:121-128`) — eight individual
  `template_tier <literal-name>` calls into eight scalar shell variables
  (`ADR`, `PLAN`, `RES`, `VAL`, `PRD`, `WI`, `DGAP`, `DINV`).
- **The `template_tier` helper** (`write-visualiser-config.sh:88-119`) resolves
  one name through the three tiers (config-override → user-override →
  plugin-default) and emits a compact JSON object with exactly four keys
  (`config_override`, `user_override`, `plugin_default`, `config_override_source`).
- **The monolithic `jq -n`** (`write-visualiser-config.sh:256-315`) passes each
  roster variable as a **statically named** `--argjson` (lines 273-277) and
  hand-maps them into a literal `templates` object (lines 303-309).

Everything downstream is data-driven and name-agnostic:

- `server/src/config.rs:29` — `pub templates: HashMap<String, TemplateTiers>`
  (arbitrary-keyed map). `TemplateTiers` (`config.rs:247-260`) is
  `#[serde(deny_unknown_fields)]` — so the per-template four-key shape must be
  preserved exactly, but the *set* of names is unconstrained.
- `server/src/templates.rs:116-235` — `TemplateResolver::build` iterates the
  config map; `list` sorts alphabetically.
- `server/src/api/templates.rs` — verbatim passthrough.
- `frontend/src/routes/library/LibraryTemplatesIndex.tsx:60-128` — a single
  `.map` over `data.templates`, no allow-list or per-name `switch`; `TierPills`
  always renders all three tier slots from `tier.present`/`tier.active`.

The discovery primitive **already exists and already works**:
`config_enumerate_templates` (`scripts/config-common.sh:139-149`) globs `*.md`
under `<plugin-root>/templates` with the repo's bash 3.2-safe idiom, and is
already sourced by the launcher (`write-visualiser-config.sh:6`). Sibling scripts
`config-list-template.sh:21` and `config-eject-template.sh:121` already iterate it
via `for KEY in $(config_enumerate_templates "$PLUGIN_ROOT"); do`. It already has
dedicated test coverage in `scripts/test-config.sh:5013-5052` that asserts a
count of **13** plus substring presence of each name (including `rca`,
`plan-review`, `pr-review`, `work-item-review`, `note`), plus the empty-dir
(K=0), mixed, and no-`.md` cases. So the scan that finds 13 is proven today — the
launcher just ignores it.

### Test blast radius — a correction to the work item and research

The work item's Technical Notes name `config.rs:433` and `test-launch-server.sh:87-92`
as tests that "will break"; the research adds `config_contract.rs` as a third.
Having read all three against the live checkout, the actual impact is **smaller**
than either document states:

| Test | Reads what? | Breaks? |
|---|---|---|
| `server/tests/config_contract.rs:72` | Runs the **real** `write-visualiser-config.sh`; asserts `templates.len()==8` + 8 names | **Yes — the only test that breaks.** Emits 13 after the change. |
| `server/src/config.rs:460` (`parses_valid_config`) | A **static fixture** (`tests/fixtures/config.valid.json`), *not* the script | **No** — fixture is independent of the script. The *fixture* is the stale side: it pins `doc_paths.len()==12` and omits `review_work`, whereas the launcher emits 13 — already asserted by `config_contract.rs:46` (with `review_work` in its key loop at `:54`). |
| `scripts/test-launch-server.sh:87-92` | Runs the launcher; asserts `user_override` paths for 6 names | **No** — all 6 names are still in the discovered 13, so the assertions still pass. |

So one test breaks, not two or three. The `config.rs` fixture test and
`test-launch-server.sh` are not "the source of the view's list" and do not break
— but per the decision below we still proactively update `config.rs` (+ its
fixture) for realism.

## Desired End State

`write-visualiser-config.sh` derives the template set by scanning the
plugin-default `templates/` directory; the `config.json` `templates` object
contains exactly one entry per `*.md` file, each resolved through the same three
tiers as today. With the current on-disk set, the view lists all **thirteen**
templates including the five previously hidden. Adding or removing a `*.md` file
is reflected on the next config generation with no edit to any hardcoded list.

**Verification:** `config_contract.rs` (which runs the real script) asserts the
emitted template set equals the `*.md` set in `templates/`; the launcher's own
unit test asserts the same and exercises the tier wiring and the
config-override-only exclusion; the discovery primitive's count/add/remove
behaviour is locked at the helper level. All format/lint/test gates pass.

### Key Discoveries

- `config_enumerate_templates` (`config-common.sh:139-149`) is the drop-in
  scanner — already sourced, already tested to 13 keys.
- The per-template shape is contractually pinned by `TemplateTiers`
  `deny_unknown_fields` (`config.rs:247-260`); reusing `template_tier` unchanged
  preserves it exactly.
- The on-disk basenames already equal the current display keys for all eight
  wired templates, so keying discovery on basename reproduces every current key
  plus the five new ones — no display-key remap is lost.
- The server sorts templates alphabetically (`templates.rs` `list`), so the
  `config.json` key order produced by the dynamic build is irrelevant.
- `config.valid.json` uses the key `research` (not `codebase-research`) and is
  read **only** by `parses_valid_config` (`config.rs:452`); nothing asserts
  `.get("research")`, so renaming it is safe.

## What We're NOT Doing

- **No frontend change.** The React view is already name-agnostic.
- **No Rust production change.** The server is already name-agnostic.
- **No live/runtime hot-reload.** Discovery is build-time (config-generation);
  a new/removed file is reflected on next launch, per the work item.
- **No `rca` glyph entry.** `rca` renders the blank-glyph fallback until a
  `STEM_TO_GLYPH` stem is added — a non-blocking 0037 follow-on; out of scope.
- **No change to the config-CLI roster** (`config-defaults.sh` `TEMPLATE_KEYS`,
  a separate 6-entry list) — that is 0029 territory.
- **No `doc_paths` fixture fix.** The *fixture* (`config.valid.json` /
  `config.rs:456`) pins `doc_paths.len()==12` and omits `review_work`, while the
  launcher emits 13 — already asserted by the contract test (`config_contract.rs:46`,
  with `review_work` in its key loop). Only the fixture is stale;
  `config_contract.rs` needs no `doc_paths` change. Pre-existing and unrelated to
  0096; left untouched.
- **No `test-launch-server.sh` change.** Its six template assertions still pass.

## Implementation Approach

Two independently mergeable phases that touch disjoint test surfaces. Each is
green at its end and can land in either order — Phase 1 has no production change,
and Phase 2's script rewrite consumes no Phase 1 artifact (the discovery
primitive it relies on already exists and is already tested). Phase 1 is
presented first only for narrative clarity, not because of a build/test
dependency. Within each phase, tests are written/changed first (red), then the
implementation makes them green — but each phase as a *merge unit* is green. (One
exception, flagged in Phase 2 §2: the config-override-only exclusion case is a
characterisation/lock test that is green under both the old and new script.)

The structurally non-trivial edit is the jq assembly: a variable-length set
cannot use statically-named `--argjson` args, so the `templates` object is built
into one shell variable and spliced via a single `--argjson templates`.

---

## Phase 1: Characterise discovery & refresh the deserialization fixture

### Overview

Pure test-layer changes, green immediately, no production code touched. Lock the
discovery primitive's count/add/remove behaviour at the helper level (covering
the work item's K∈{0,1,3,13} and add/remove acceptance criteria where they are
most directly testable), and refresh the Rust deserialization fixture to a
faithful 13-template set.

### Changes Required

#### 1. Helper-level count/add/remove coverage

**File**: `scripts/test-config.sh`
**Changes**: Extend the existing `config_enumerate_templates` block (after the
current cases at ~`5052`) with a K=3 case and explicit add/remove sensitivity.
K=0, the count of 13, mixed, and no-`.md` are already covered at `5024-5052`.
While editing the block, tighten the existing per-name checks from the substring
`assert_contains "research"` (which `codebase-research` only incidentally
satisfies) to exact-line membership (e.g. `grep -qx`), so a future rename can't
slip through on a substring match.

```bash
echo "Test: counts files one-to-one (K=3, no dedup / off-by-one)"
K3_ROOT=$(mktemp -d "$TMPDIR_BASE/k3-plugin-XXXXXX")
mkdir -p "$K3_ROOT/templates"
echo a >"$K3_ROOT/templates/alpha.md"
echo b >"$K3_ROOT/templates/beta.md"
echo c >"$K3_ROOT/templates/gamma.md"
assert_eq "K=3 yields 3 keys" "3" \
  "$(config_enumerate_templates "$K3_ROOT" | wc -l | tr -d ' ')"

echo "Test: adding / removing a file is reflected one-to-one"
echo d >"$K3_ROOT/templates/delta.md"
assert_eq "after add → 4 keys" "4" \
  "$(config_enumerate_templates "$K3_ROOT" | wc -l | tr -d ' ')"
rm "$K3_ROOT/templates/alpha.md"
assert_eq "after remove → 3 keys" "3" \
  "$(config_enumerate_templates "$K3_ROOT" | wc -l | tr -d ' ')"
```

#### 2. Refresh the deserialization fixture to 13 templates

**File**: `skills/visualisation/visualise/server/tests/fixtures/config.valid.json`
**Changes**: Bump the `templates` object from 8 to the faithful on-disk 13:
rename the `research` entry to `codebase-research` (resolving the pre-existing
name mismatch), and add five entries — `plan-review`, `pr-review`,
`work-item-review`, `rca`, `note`. Add the fourth tier key
`config_override_source` to **all 13** entries (including the existing eight), so
the fixture mirrors the four-key shape `template_tier` actually emits rather than
leaning on `#[serde(default)]` to paper over a missing key. Leave `doc_paths`
unchanged (out of scope).

This fixture's role is to exercise the **deserializer shape**, not to mirror the
live set — `config_contract.rs` (which runs the real script) is the authoritative
generator/contract test. Bumping it to 13 keeps the sample representative without
making it a second source of truth.

```jsonc
// e.g. add (and likewise plan-review, pr-review, work-item-review, note);
// also add "config_override_source": null to the existing eight entries:
"rca": {
  "config_override": null,
  "user_override": "/abs/path/to/project/.accelerator/templates/rca.md",
  "plugin_default": "/abs/path/to/plugin/templates/rca.md",
  "config_override_source": null
}
```

#### 3. Bump and enrich the fixture assertion

**File**: `skills/visualisation/visualise/server/src/config.rs`
**Changes**: In `parses_valid_config` (~`450-466`), change
`assert_eq!(c.templates.len(), 8)` (currently `config.rs:460`) → `13`, and add
spot-checks that the renamed and newly-added names are present and tier-shaped —
including a `config_override_source` assertion so the fourth key is exercised at
the deserialization layer. Leave the `doc_paths.len() == 12` assertion
(`config.rs:456`) as-is.

```rust
assert_eq!(c.templates.len(), 13);
assert!(c.templates.contains_key("codebase-research"));
let rca = c.templates.get("rca").expect("rca tier");
assert!(rca.config_override.is_none());
assert!(rca.config_override_source.is_none());
assert!(rca.plugin_default.ends_with("rca.md"));
```

### Success Criteria

#### Automated Verification

- [ ] Config-helper shell tests pass: `mise run test:integration:config`
- [ ] Rust unit tests pass (incl. `parses_valid_config`):
      `mise run test:unit:visualiser`
      (fallback: `cd skills/visualisation/visualise/server && cargo test --lib`)
- [ ] Shell format + lint clean: `mise run scripts:check`
- [ ] Rust format + lint clean: `mise run server:check`
- [ ] The breaking contract test is **untouched and still green** in this phase
      (script still emits 8): `cd skills/visualisation/visualise/server && cargo test --test config_contract`

#### Manual Verification

- [ ] `config.valid.json` lists exactly the 13 on-disk template names (diff
      against `for f in templates/*.md; do basename "$f" .md; done`).
- [ ] This phase introduces no production-code change (review the diff:
      only `test-config.sh`, `config.valid.json`, `config.rs`).

---

## Phase 2: Auto-discover templates in the launcher

### Overview

The feature. Replace the hardcoded roster with a discovery loop and a dynamic jq
splice in `write-visualiser-config.sh`, and land the coupled tests with it: the
discovery-aware contract test and new launcher-unit coverage. These tests are
written first; most fail (red) against the unchanged script — the contract test,
the set==directory check, and the `rca`/`note` tier-path and
`config_override_source` cases — and the script change turns them green. (The
config-override-only exclusion case is the one exception: it is green under both
scripts and serves as a lock test, per the NOTE in §2.) The script and
`config_contract.rs` **must** land together — changing either alone leaves the
contract test red.

### Changes Required

#### 1. Rewrite the contract test to be discovery-aware (write first → red)

**File**: `skills/visualisation/visualise/server/tests/config_contract.rs`
**Changes**: Replace the `len()==8` assertion and the eight-name loop
(lines 71-91) with an assertion that the emitted set equals the `*.md` set in the
plugin `templates/` directory (derived in-test, so it never needs editing when
templates change). The plugin `templates/` dir is `CARGO_MANIFEST_DIR/../../../../templates`.

```rust
// Derive the expected set the same way the launcher does: scan the
// plugin-default templates/ dir. Keeps the contract drift-proof — adding or
// removing a template never requires editing this test.
let templates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../../../templates");
let mut expected: Vec<String> = std::fs::read_dir(&templates_dir)
    .expect("read plugin templates dir")
    .filter_map(|e| {
        let p = e.ok()?.path();
        // Mirror config_enumerate_templates' `[ -f ]` guard so the in-test
        // derivation matches the launcher exactly (skip dirs/symlinks).
        (p.is_file() && p.extension()? == "md").then(|| {
            p.file_stem().unwrap().to_string_lossy().into_owned()
        })
    })
    .collect();
expected.sort();

let mut actual: Vec<String> = cfg.templates.keys().cloned().collect();
actual.sort();

assert!(!expected.is_empty(), "expected at least one template on disk");
assert_eq!(
    actual, expected,
    "config.json templates must equal the *.md set in templates/"
);
for name in &expected {
    let tiers = cfg.templates.get(name).unwrap();
    assert!(
        tiers.plugin_default.to_string_lossy().ends_with(&format!("{name}.md")),
        "plugin_default for {name} should end with {name}.md, got {}",
        tiers.plugin_default.display()
    );
}
```

#### 2. Add launcher-unit coverage (write first → red)

**File**: `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`
**Changes**: Source `config-common.sh` for `config_enumerate_templates`
(currently only `test-helpers.sh` is sourced — add
`source "$PLUGIN_ROOT/scripts/config-common.sh"` near line 6). Append three cases
before `test_summary`:

```bash
# ─── templates: discovered set matches the templates/ directory ──────────────
echo "Test: templates object lists every *.md in the plugin templates/ dir"
PROJ_TD="$TMPDIR_BASE/t-templates-discovered"
make_project "$PROJ_TD"
OUT_TD="$TMPDIR_BASE/out-td.json"
run_config "$PROJ_TD" >"$OUT_TD"
EXPECTED_KEYS="$(config_enumerate_templates "$PLUGIN_ROOT" | sort | tr '\n' ' ')"
ACTUAL_KEYS="$(jq -r '.templates | keys[]' "$OUT_TD" | sort | tr '\n' ' ')"
assert_eq "templates keys match templates/ dir" "$EXPECTED_KEYS" "$ACTUAL_KEYS"

# Tier wiring flows through template_tier for a previously-hidden template.
# NOTE: user_override is the *unconditional candidate* path template_tier emits
# (make_project never creates .accelerator/templates/); the server decides
# present/absent. We pin the candidate path the launcher wires, not presence.
assert_json_eq "rca plugin_default points at plugin templates/" \
  ".templates.rca.plugin_default" "$PLUGIN_ROOT/templates/rca.md" "$OUT_TD"
assert_json_eq "note user_override points at project .accelerator/templates" \
  ".templates.note.user_override" "$PROJ_TD/.accelerator/templates/note.md" "$OUT_TD"

# Fourth tier key: config_override_source. Null with no config override — this
# guards the key the jq restructure is most likely to drop (it feeds the view's
# Tier 1 description and has no other automated check).
assert_json_eq "rca config_override_source null with no override" \
  ".templates.rca.config_override_source" "null" "$OUT_TD"

# …and populated when a config.md declares the override (exercises the 4th key
# in its non-null form, plus the provenance scan in template_tier).
echo "Test: config_override_source records the declaring config file"
PROJ_CS="$TMPDIR_BASE/t-templates-override-source"
make_project "$PROJ_CS"
mkdir -p "$PROJ_CS/custom"
echo "# custom rca" >"$PROJ_CS/custom/rca.md"
printf -- '---\ntemplates:\n  rca: custom/rca.md\n---\n' >"$PROJ_CS/.accelerator/config.md"
OUT_CS="$TMPDIR_BASE/out-cs.json"
run_config "$PROJ_CS" >"$OUT_CS"
# Pin both co-dependent halves of the populated-override shape: the path itself
# and its provenance. (Asserting only the source would miss a dropped path.)
assert_json_eq "rca config_override reflects the declared path" \
  ".templates.rca.config_override" "custom/rca.md" "$OUT_CS"
assert_json_eq "rca config_override_source names config.md" \
  ".templates.rca.config_override_source" ".accelerator/config.md" "$OUT_CS"

# ─── config-override-only template is NOT surfaced (templates/ is canonical) ──
# NOTE: a characterisation/lock test — GREEN under both the old 8-name roster and
# the new discovery (zzz-fake is in neither). It pins that .accelerator/templates/
# is not a discovery source, guarding a future change that scanned the override dir.
echo "Test: a template present only in .accelerator/templates is not surfaced"
PROJ_OO="$TMPDIR_BASE/t-templates-override-only"
make_project "$PROJ_OO"
mkdir -p "$PROJ_OO/.accelerator/templates"
echo "# fake" >"$PROJ_OO/.accelerator/templates/zzz-fake.md"
OUT_OO="$TMPDIR_BASE/out-oo.json"
run_config "$PROJ_OO" >"$OUT_OO"
assert_json_eq "override-only template absent from set" \
  '.templates | has("zzz-fake")' "false" "$OUT_OO"
```

These cover AC #1/#2/#7 (set == directory) and AC #5 (config-override-only
exclusion), and exercise the full four-key tier shape (incl.
`config_override_source`, null and populated) the launcher emits. Add/remove
(AC #3/#4) is covered by the helper tests in Phase 1 plus the "set == directory"
assertion here.

AC #6 (tier-presence *indicators*) needs no new test here: the `present`/`active`
logic that drives the pills lives in the server's `TemplateResolver` and is
already covered by the name-agnostic resolver unit tests in
`server/src/templates.rs` — `only_plugin_default_present_picks_plugin_default_active`
(`:395`), `all_three_tiers_present_picks_config_override_as_active` (`:377`),
`user_override_wins_when_config_override_absent` (`:418`), and
`list_sorts_names_alphabetically` (`:436`). Discovery adds no tier-presence code,
so those tests apply unchanged to every discovered name; these launcher-unit
cases only confirm the tier *paths* (the resolver's inputs) are wired.

#### 3. Rewrite the config assembly to discover templates (make tests green)

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
**Changes**:

(a) **Remove** the eight scalar roster assignments (lines 121-128).

(b) **Add** a discovery builder after the `template_tier` helper. It iterates
`config_enumerate_templates` (mirroring the `config-list-template.sh:21` idiom),
emits one `<name>\t<compact tier JSON>` line per template, and folds them into a
single object via `jq -Rn`. `template_tier` uses `jq -nc`, so each tier JSON is
single-line and `split("\t")` yields exactly `[name, json]`. K=0 → `{}`.

```bash
# ── Templates (auto-discovered) ──────────────────────────────────────────────
# Derive the template set by scanning the plugin-default templates/ directory
# rather than a hardcoded roster (config_enumerate_templates is sourced via
# config-common.sh; bash 3.2-safe glob, one basename per *.md). Each name is
# resolved through the same three tiers as before (template_tier) and folded
# into one JSON object. A variable-length set cannot use statically-named
# --argjson args, so the object is built here and spliced via one --argjson.
#
# Two invariants the tab-delimited fold relies on (both hold by construction):
#   - template_tier emits single-line compact JSON (`jq -nc`), so split("\t")
#     yields exactly [name, tier-json] per line — keep that `-c` flag.
#   - template basenames are filename-safe (no tab / newline).
# Fail-fast: capture the tier into its own assignment so `set -e` aborts on a
# failed resolution. A failed `$(template_tier …)` used directly as a *printf
# argument* would be masked (printf still succeeds), losing the abort the current
# `ADR="$(template_tier adr)"` form gives; the subshell's set -e + pipefail then
# surface the failure to the `TEMPLATES_JSON="$(…)"` capture below. Keep `tier`
# declared via `local name tier` *separately* from its assignment — collapsing to
# `local tier="$(…)"` would re-mask the failure (the `local` builtin's own
# success status overrides the substitution's).
build_templates_json() {
  local name tier
  for name in $(config_enumerate_templates "$PLUGIN_ROOT"); do
    tier="$(template_tier "$name")"
    printf '%s\t%s\n' "$name" "$tier"
  done | jq -Rn '
    reduce inputs as $line ({};
      ($line | split("\t")) as [$k, $v] | .[$k] = ($v | fromjson))'
}
TEMPLATES_JSON="$(build_templates_json)"
```

(c) In the `jq -n` invocation (lines 256-315): **remove** the eight
`--argjson adr … --argjson design_inventory` flags (lines 273-277); **add**
`--argjson templates "$TEMPLATES_JSON"`; and **replace** the literal `templates`
object (lines 303-309) with `templates: $templates,`.

```bash
# replaces lines 273-277:
  --argjson templates "$TEMPLATES_JSON" \
# replaces lines 303-309:
    templates: $templates,
```

### Success Criteria

#### Automated Verification

- [ ] Contract + launcher integration tests pass: `mise run test:integration:visualiser`
      (fallback: `cd skills/visualisation/visualise/server && cargo test --test config_contract`)
- [ ] Launcher unit harness passes:
      `bash skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`
- [ ] Launcher binary-acquisition harness still passes (unchanged template
      assertions): `mise run test:integration:binary-acquisition`
      (covers `test-launch-server.sh`)
- [ ] Rust unit + config-helper tests still pass: `mise run test:unit:visualiser`
      and `mise run test:integration:config`
- [ ] Shell format + lint clean (shfmt + ShellCheck + bash-3.2 bashisms guard):
      `mise run scripts:check`
- [ ] Rust format + lint clean: `mise run server:check`
- [ ] Full read-only CI gate passes: `mise run check`
- [ ] Full test suite passes: `mise run test`
- [ ] Generated config lists 13 templates with no static roster left in the
      script: `bash skills/visualisation/visualise/scripts/write-visualiser-config.sh --plugin-version 0.0.0 --project-root "$(mktemp -d)" --tmp-dir /tmp/x --log-file /tmp/x.log --owner-pid 0 | jq '.templates | keys | length'`
      returns `13`; `grep -nE 'template_tier (adr|plan|validation)' skills/visualisation/visualise/scripts/write-visualiser-config.sh` returns nothing.

#### Manual Verification

- [ ] Launch the visualiser and open `/library/templates`; all **13** templates
      appear, including `plan-review`, `pr-review`, `work-item-review`, `rca`,
      `note`.
- [ ] Tier-presence pills are correct: a plugin-default-only template lights
      only plugin-default; a template ejected to `.accelerator/templates/` lights
      user-override too.
- [ ] `rca` renders the blank-glyph fallback (expected; 0037 follow-on) and the
      row is otherwise fully present and clickable.
- [ ] Add a throwaway `templates/zzz-temp.md`, relaunch → it appears; remove it,
      relaunch → it disappears.

---

## Testing Strategy

### Unit Tests

- **Discovery primitive** (`test-config.sh`): K∈{0,1,3,13}, mixed-extension,
  no-`.md`, and add/remove sensitivity on `config_enumerate_templates`. The
  one-to-one count mapping (AC #2) and add/remove (AC #3/#4) live here, where a
  temp `templates/` dir makes K directly controllable.
- **Deserialization** (`config.rs` `parses_valid_config`): the fixture
  exercises a 13-entry arbitrary-keyed map through `TemplateTiers`.

### Integration Tests

- **`config_contract.rs`**: runs the real `write-visualiser-config.sh` and
  asserts the emitted set equals the on-disk `*.md` set — the end-to-end proof
  that the script defers to discovery (AC #1/#2/#7), and drift-proof.
- **`test-write-visualiser-config.sh`**: set == directory, the four-key tier
  shape incl. `config_override_source` (null + populated), config-override-only
  exclusion (AC #5).

### Pre-existing coverage relied upon (no new tests)

- **AC #6 (tier-presence indicators)**: the `present`/`active` logic is covered
  by the name-agnostic `TemplateResolver` unit tests in `server/src/templates.rs`
  (see Phase 2 §2). Discovery adds no tier-presence code, so they apply unchanged
  to every discovered name.

### Manual Testing Steps

1. Launch the visualiser; confirm 13 templates at `/library/templates`.
2. Verify tier pills against a known plugin-default-only and an ejected template.
3. Add/remove a `templates/*.md` and relaunch to confirm reflection.

## Performance Considerations

Negligible. Discovery runs once at config-generation: one directory glob plus N
small `jq` invocations (the existing `template_tier` already runs per template).
N goes from 8 to 13.

## Migration Notes

None. No schema change, no data migration. `config.json`'s `templates` object
simply carries more (and correctly-named) keys; the server's `HashMap` and the
view's `.map` absorb it transparently. `deny_unknown_fields` on `TemplateTiers`
guarantees a malformed dynamic build fails loudly at boot rather than silently
dropping templates.

An empty `templates/` (K=0) is a valid degenerate case: the jq fold returns
`{}`, the server deserialises an empty `HashMap`, and the view renders empty with
no boot error — intended graceful degradation, not a fault. This cannot occur for
the plugin's own (always-populated) `templates/`, and the launcher's
`PLUGIN_ROOT` is fixed to the real tree, so it is not exercised end-to-end through
the script; the K=0 *scan* is covered at the helper level (Phase 1) and the
empty-fold behaviour is verified by reading.

## References

- Work item: `meta/work/0096-templates-view-auto-discovers-templates.md`
- Research: `meta/research/codebase/2026-06-11-0096-templates-view-auto-discovery.md`
- Source of truth (to change): `skills/visualisation/visualise/scripts/write-visualiser-config.sh:121-128, 273-277, 303-309`
- Discovery helper: `scripts/config-common.sh:139-149` (`config_enumerate_templates`)
- Iteration precedents: `scripts/config-list-template.sh:21`, `scripts/config-eject-template.sh:121`
- Name-agnostic downstream: `server/src/config.rs:29`, `server/src/templates.rs:116-235`, `frontend/src/routes/library/LibraryTemplatesIndex.tsx:60-128`
- Tests touched: `server/tests/config_contract.rs:71-91` (templates assertion at `:72`), `server/src/config.rs:450-466` (`parses_valid_config`; `templates.len()` at `:460`, `doc_paths.len()` at `:456`), `server/tests/fixtures/config.valid.json`, `scripts/test-config.sh:5013-5052`, `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`
- Tier-presence coverage relied upon: `server/src/templates.rs` resolver tests (`:377`, `:395`, `:418`, `:436`)
- ADR-0017 (three-tier template resolution — the contract to preserve)
