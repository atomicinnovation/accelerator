---
type: "plan"
id: "2026-09-20-0282-tunable-depth-and-breadth"
title: "Tunable Depth and Breadth Implementation Plan"
date: "2026-09-20T22:45:46+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0282"
parent: "work-item:0282"
derived_from: ["codebase-research:2026-09-20-0282-tunable-depth-and-breadth"]
tags: ["research", "skills", "config"]
revision: "c48d829a46e054520b2fdb153ad9765f256ff873"
repository: "accelerator"
last_updated: "2026-09-22T08:53:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tunable Depth and Breadth Implementation Plan

## Overview

Register `research.breadth` (default `8`) and `research.depth` (default `1`) as
catalogue config keys, make `config get` resolve unset keys from the catalogue,
then thread the knobs into the `outline` and `conduct` verbs of
`research-topic/SKILL.md` with `flag > personal > team > catalogue` resolution and
clamp-and-warn validation. `breadth` goes live as the `outline` focus-area
ceiling; `depth` ships dormant — resolved, threaded, and documented, with a notice
when it exceeds `1` — until the recursion engine consumes it.

The key enabling change is to `config get`. Today `config get <key>` resolves
`personal > team` and, on a miss, prints the caller-supplied default or empty —
**it never consults the catalogue** (`get.rs:1-2`), a catalogue-blindness work
item 0167 chose deliberately. This plan deliberately reverses that for the
cross-level case: `config get <key>` resolves `personal > team > built-in
default`, and the caller override moves to a `--default` flag. The rationale is
single-source-of-truth — otherwise every knob duplicates its default literal in
the SKILL, a duplication that multiplies as knobs accrue. `config get` gains
catalogue-aware resolution like `config path`'s `Absent` branch, though the two
stay deliberately different on grammar, unknown-key handling, and single-level
reads (Phase 2 documents the differences). **`--fail-safe` is unchanged** — its
only job is to keep the skill loadable on a read failure; it does not manufacture
a default. With the catalogue as the single source of truth, the SKILL reads a
bare `config get research.breadth --fail-safe` and carries no default literal of
its own; a read failure yields empty, which the SKILL surfaces as an error.

The work is extend-not-invent on the Rust side (mirror `REVIEW_KEYS`, mirror the
`config path` resolution shape, wire the five hand-enumerated group sites) and
net-new prose on the SKILL side (the verbs are prompt-resolved, so resolution and
validation live in the SKILL, not in compiled Rust). Verification of the prose
contracts is eval-level and manual by decision; no eval suite is authored in this
slice.

## Current State Analysis

The config catalogue is stringly-typed and hand-enumerated. `REVIEW_KEYS` is the
exact template for a new numeric-tunable group: a `&[(&str, Default)]` slice of
`(key, default)` tuples with numeric defaults stored as string scalars
(`catalogue.rs:150-176`). There is **no master group list** — `default_for`'s
scan array (`catalogue.rs:245`) and `dump::assemble`'s per-group loops
(`dump.rs:55-75`) each enumerate groups by hand, and `public-api.txt` pins the
public surface — so a new group is wired into each site independently.

The `research-topic` verbs carry the values as hardcoded prose. The global
bounding statement reads "**breadth is 8** … and **depth is 1**"
(`SKILL.md:51-52`); `outline` repeats "the breadth ceiling of 8"
(`SKILL.md:145-147`); `conduct` spawns one researcher per outstanding focus area
with no depth token beyond the global statement (`SKILL.md:163-206`). The verbs
have no arg parser — dispatch is prose (`SKILL.md:47-49`), and no verb reads a
flag today, so `--breadth`/`--depth` are net-new prose idioms.

Key constraints discovered:

- **`config get` never consults the catalogue today** (`get.rs:14-30`, and its own
  module doc). It returns `personal > team > <positional default> > empty`. The
  positional default is `Option<String>` with no `#[arg(long)]`
  (`launch/inbound/cli.rs:89-93`), so the caller passes it as a bare positional
  (`config get jira.site ""`). This plan changes both: the catalogue becomes the
  miss fallback, and the override becomes a `--default` flag.
- **`config path` is the resolution template.** `config path <key>` folds the
  catalogue via `effective` and lets an explicit default win over it
  (`launch/inbound/cli.rs:124-131`, `paths.rs:158`). The `get` change makes `get`
  behave like `path` for any dotted key.
- **The `!` preprocessor runs at SKILL load, before the invocation is read.** It
  cannot see a `--breadth` flag. So the preprocessor injects the config-resolved
  value; the verb prose then applies the invocation flag over it and validates the
  result.
- **`conduct` is already breadth-free** (`SKILL.md:155-206` contains no
  breadth/ceiling reference), so "breadth not re-checked at conduct" needs no
  code change — only a guard against introducing one.
- **`config get *` is already whitelisted** (`SKILL.md:12-16` allows
  `Bash(accelerator config *)`), so the new reads need no allowlist change.

### Key Discoveries:

- Numeric defaults are string scalars: `review.max_lenses` is
  `Default::Scalar("8")` (`catalogue.rs:153`); `to_value` wraps it as
  `Value::Scalar(Scalar::String("8"))` (`catalogue.rs:16-28`). The consumer
  parses the string. `breadth: 8` is the exact twin.
- The key-count test name encodes the numbers:
  `the_catalogue_holds_sixty_three_keys_across_six_groups` asserts `63` across
  six groups (`catalogue.rs:270-280`). Adding two keys in one group makes it
  `sixty_five` / `seven_groups`, asserting `65`.
- `public-api.txt` is alphabetical: `RESEARCH` sorts between `PATH_KEYS`
  (line 12) and `REVIEW_KEYS` (line 13). The catalogue group constants are not
  re-exported at the crate root, so there is a single insertion, not two.
- `parity.rs` has **no key-count or group enumeration** — its only catalogue
  touch is `default_for("review.core_lenses")` (`parity.rs:112`). The two new
  keys do not disturb it; the work item's "update parity.rs" is honoured by
  adding a resolution test (below).
- **The `config get` behaviour change has a bounded test surface**, all in
  `cli/launcher/tests/config_read.rs`. Two tests assert exactly the behaviour
  being inverted and must be rewritten to the catalogue-backed contract:
  `get_of_a_catalogue_backed_key_on_a_miss_does_not_inject_the_catalogue`
  (`:306`) and `get_with_an_explicit_empty_default_yields_empty_on_a_miss`
  (`:198`). The positional-default tests (`:187`, `:189`, `:200`, `:320`) switch
  to the `--default` flag. The uncatalogued-key miss
  (`get_of_an_unset_key_without_a_default_prints_empty`, `:177`) stays empty and
  is unchanged.
- The validation model **diverges from the `review.rs` precedent it cites**:
  `review.rs` validates in typed Rust with reject-and-substitute-default
  (`positive`, `review.rs:554-570`); 0282 clamps-to-1 in SKILL prose. The
  warning house style to mirror is
  `"… must be a positive integer, got '{value}' — using default (…)"`
  (`review.rs:565`), rendered with a `Warning:` prefix.
- The `1 / 2–4 / 10+` effort-scaling rubric the work item references is **not in
  the shipped SKILL.md** — only the single sizing sentence at `SKILL.md:145-147`.
  The "may reduce beneath N but never raise above it" AC describes model
  judgement bounded by that sentence, not a rubric block to edit.

Two open questions from the research are resolved:

- **`workspaces/*/cli/…` mirror trees are untracked** (`git ls-files
  workspaces/` is empty) — jj working copies, not files to edit. The canonical
  tree is top-level `cli/`.
- **0280 has not landed** (`meta/work/0280-*.md` is `draft`; no
  `research.contact_email` in the catalogue). 0282 lands first and carries no
  reconciliation burden; 0280 reconciles the shared fixtures when it lands.

## Desired End State

`accelerator config dump` shows `research.breadth: 8` and `research.depth: 1`
with source attribution. `config get research.breadth` (resolved across levels)
returns `personal > team > built-in default`, with a non-empty `--default`
overriding the built-in default; an unset key returns `8`. `--fail-safe` is
unchanged — a read failure suppresses to empty, which the SKILL surfaces as an
error rather than a fabricated default. `outline` sizes each round to at most the
resolved breadth (invocation flag over config), clamping an invalid value to 1
with a named warning; `conduct` reads the resolved depth, always spawns one
researcher per focus area, and prints a notice when depth exceeds 1. `configure
help` documents both knobs, their defaults, the `flag > personal > team >
default` order, and depth's dormancy caveat in plain language. The `config get`
help documents the built-in-default fallback and the `--default` override.

Verify the end state: `mise run check` is green; `mise run test:unit:cli`,
`mise run public-api:check`, and `mise run test:integration:skill-invocation`
pass; and the definition of done — the bare `mise run` (full local CI mirror) —
exits 0 end-to-end.

## What We're NOT Doing

- **No eval suite.** The flag-over-config, clamp-and-warn, and depth-notice
  contracts are shipped as SKILL prose written to be eval-able later; no
  `skills/research/research-topic/evals/` is authored here (decided with the
  author). research-topic has no eval infra today, and evals do not gate
  `mise run check`. The eval-able prose is the natural home for an eval harness
  scheduled with the recursion-engine work that activates depth.
- **No recursion.** `depth` stays inert — one researcher per focus area
  regardless of value. The recursion engine is a separate work item. Its
  dormant→live flip is a coordinated prose edit across three sites, recorded here
  for the hand-off: the depth line and notice in the knob-resolution block
  (Phase 4 §2), the depth read/gate in the `conduct` section (Phase 4 §3), and the
  dormancy caveat in `configure help` (Phase 5).
- **No `accelerator config research` subcommand.** Validation stays in SKILL
  prose (the work item's chosen divergence from the `review.rs` Rust path); the
  `config get` change is a generic catalogue-backed read, not a research-specific
  typed consumer.
- **No numeric type in the config model.** Values remain string scalars parsed
  by the consumer, per the `review.*` model.
- **No `breadth` re-check at `conduct`.** A hand-edited `outline.md` exceeding the
  ceiling is honoured; `conduct` stays breadth-free. `breadth` bounds per-round
  commissioning only — peak `conduct` fan-out (one researcher per outstanding
  focus area across all rounds) is intentionally uncapped.
- **No upper bound on either knob.** `breadth` is the per-round cost guard; there
  is no hard cap.
- **No change to `--fail-safe`.** Its role stays "keep the skill loadable on a
  read failure" (suppress to empty, exit 0); it does not degrade to the built-in
  default. The built-in default comes from the normal `Absent` resolution, and an
  empty resolved knob is surfaced to the user as an error.
- **No migration of `config path` to a `--default` flag.** `get` gains the flag;
  `path` keeps its positional. The deliberate grammar difference is documented
  (Phase 2, Migration Notes); unifying them is a possible later follow-up.
- **Plain-language dormancy, not the AC's literal `0283` wording.** The work
  item's Docs/Depth ACs say "until 0283"; user-facing output renders this as "not
  yet available" to avoid leaking an internal id. The ACs are satisfied in intent;
  the literal id lives only in plan and work-item prose.
- **No `research.contact_email`** — that key belongs to a separate slice.

## Implementation Approach

Five phases with forward dependencies only. Phase 1 (research keys) and Phase 2
(`config get` from the catalogue) are independent of each other; both are
prerequisites for Phases 3–4, which read a bare, catalogue-resolving
`config get research.<knob>`. Each phase is green under `mise run check` on its
own.

- **Phase 1** registers the keys in Rust and reconciles the fixtures — pure
  catalogue plumbing, unit- and golden-tested.
- **Phase 2** makes `config get` resolve the built-in default and moves the caller
  default to a `--default` flag (deliberately reversing 0167's catalogue-blindness)
  — a generic config-CLI change, unit- and golden-tested, with the two `init-jira`
  callers and the `cli/migrate` port doc updated; `--fail-safe` is untouched.
- **Phase 3** makes `breadth` a live `outline` ceiling in SKILL prose, reading a
  bare `config get research.breadth`.
- **Phase 4** threads `depth` through `conduct` dormant, with the `>1` notice.
- **Phase 5** documents both knobs in `configure help`.

Test-driven where applicable: Phases 1–2 follow red-green-refactor against the
catalogue unit tests, the `config get` behaviour tests, and the dump golden.
Phases 3–5 are prompt-resolved prose; their automated coverage is
`test:integration:skill-invocation` (the new preprocessor sites execute cleanly)
and `lint:bare-invocation:check` (invocation form), with behaviour verified
manually.

## Phase 1: Register the research knobs

### Overview

Add a `RESEARCH_KEYS` group to the catalogue and wire it into every hand-enumerated
site, reconciling the shared fixtures. Establishes the keys in `config dump`,
`default_for`, and `config get`, satisfying the "Keys visible", "Default
resolution", and "Config precedence" acceptance criteria through existing config
machinery.

### Changes Required:

#### 1. Catalogue declaration and scan array

**File**: `cli/config/src/catalogue.rs`
**Changes**: Declare `RESEARCH_KEYS` next to `REVIEW_KEYS`; add it to
`default_for`'s scan array.

```rust
pub const RESEARCH_KEYS: &[(&str, Default)] = &[
    ("research.breadth", Default::Scalar("8")),
    ("research.depth", Default::Scalar("1")),
];
```

```rust
for group in [PATH_KEYS, WORK_KEYS, REVIEW_KEYS, RESEARCH_KEYS, VISUALISER_KEYS]
{
    if let Some((_, default)) = group.iter().find(|(name, _)| *name == key) {
        return Some(default.to_value());
    }
}
```

#### 2. Catalogue tests

**File**: `cli/config/src/catalogue.rs` (test module)
**Changes**: Import `RESEARCH_KEYS`; update the key-count test to `65` across
seven groups and rename it; add a declared-value test for the new keys (the
red-first test).

```rust
use super::{
    default_for, AGENT_KEYS, DOC_TYPES, EXTRA_KEYS, PATH_KEYS, RESEARCH_KEYS,
    REVIEW_KEYS, TEMPLATE_KEYS, VISUALISER_KEYS, WORK_KEYS,
};
```

```rust
#[test]
fn the_catalogue_holds_sixty_five_keys_across_seven_groups() {
    let count = PATH_KEYS.len()
        + TEMPLATE_KEYS.len()
        + WORK_KEYS.len()
        + REVIEW_KEYS.len()
        + RESEARCH_KEYS.len()
        + AGENT_KEYS.len()
        + VISUALISER_KEYS.len();
    assert_eq!(count, 65);
    assert_eq!(DOC_TYPES.len(), 14);
}

#[test]
fn default_for_the_research_knobs_are_typed_scalars() {
    assert_eq!(
        default_for("research.breadth"),
        Some(Value::Scalar(Scalar::String("8".to_owned())))
    );
    assert_eq!(
        default_for("research.depth"),
        Some(Value::Scalar(Scalar::String("1".to_owned())))
    );
}
```

#### 3. Dump assembly

**File**: `cli/launcher/src/config_command/core/dump.rs`
**Changes**: Emit the research rows immediately after the review rows, fixing
their position in the dump order (and thus the golden).

```rust
for (key, _) in catalogue::REVIEW_KEYS {
    rows.push(defaulted_row(config, key)?);
}
for (key, _) in catalogue::RESEARCH_KEYS {
    rows.push(defaulted_row(config, key)?);
}
for name in catalogue::AGENT_KEYS {
    rows.push(defaulted_row(config, &format!("agents.{name}"))?);
}
```

#### 4. Public-API snapshot

**File**: `cli/config/tests/fixtures/public-api.txt`
**Changes**: Insert the new group constant alphabetically, between `PATH_KEYS`
(line 12) and `REVIEW_KEYS` (line 13).

```text
pub const config::catalogue::PATH_KEYS: &[(&str, config::catalogue::Default)]
pub const config::catalogue::RESEARCH_KEYS: &[(&str, config::catalogue::Default)]
pub const config::catalogue::REVIEW_KEYS: &[(&str, config::catalogue::Default)]
```

#### 5. Dump golden

**File**: `cli/launcher/tests/fixtures/dump/dump.golden`
**Changes**: Insert the two rows after `review.work_item_revise_major_count`
(line 15), before `agents.reviewer` (line 16). The fixture config sets neither
knob, so both attribute to `default`.

```text
| `research.breadth` | `8` | default |
| `research.depth` | `1` | default |
```

#### 6. Parity resolution test

**File**: `cli/config-adapters/tests/parity.rs`
**Changes**: Add a test that materialises a fixture setting the knobs at team and
personal levels and asserts the **resolved** value, exercising `personal > team`
rather than re-asserting `default_for` (which the catalogue unit test already
covers). Note this reaches only `personal > team`: `parity.rs`'s `resolve()` does
not fold the catalogue, so the built-in-default leg is covered by the Phase 2
black-box precedence test, not here.

### Success Criteria:

#### Automated Verification:

- [x] Catalogue unit tests pass (key count, declared values): `mise run test:unit:cli`
- [x] Dump golden matches: `mise run test:unit:cli` (drives `dump_matches_the_committed_golden`, `cli/launcher/tests/config_read.rs:802`)
- [x] Parity resolution test passes: `mise run test:unit:cli`
- [x] Public-API snapshot matches: `mise run public-api:check`
- [x] Rust format and clippy clean: `mise run cli:check`
- [x] Full read-only lane green: `mise run check`

#### Manual Verification:

- [x] `accelerator config dump` in a configured repo lists `research.breadth`
      (`8`) and `research.depth` (`1`) with correct source attribution.

---

## Phase 2: Resolve `config get` from the catalogue

### Overview

Give `config get <key>` a catalogue fallback: on a miss at both config levels
(when resolving across levels, i.e. no `--level`), fall back to the catalogue
default rather than empty, and move the caller override from a positional argument
to a `--default` flag. The catalogue becomes the single source of truth for a
key's default, so the SKILL sites in Phases 3–4 carry no default literal.

⚠️ This **deliberately reverses** the catalogue-blindness that work item 0167
gave `config get` (its review flagged routing `get` through the catalogue as a
bug at the time). The reversal is intentional here: `get` becomes a general
catalogue-aware read like the `Absent` branch of `config path`. The motivation is
single-source-of-truth — every additional research (and future) knob would
otherwise duplicate its default literal in the SKILL, a duplication that
multiplies as knobs accrue rather than staying the one-off the research likened
to the tolerated line-width duplication.

`config get` gains catalogue-aware cross-level resolution like `config path`, but
the two remain deliberately different on three axes, documented rather than
unified:

| Axis | `config get` (after this phase) | `config path` |
|---|---|---|
| Override grammar | `--default` flag | positional |
| Unknown/uncatalogued key | accepted (resolves to `--default` or empty) | refused |
| `--level` read on a miss | that level only, no built-in default | folds the built-in default (its `resolve_with_fallback` ignores level) |

The `--level` row is a genuine divergence, not an alignment: `config path
--level team` on a miss still returns the built-in default, whereas `config get
--level team` returns empty. `get`'s rule — a single-level read is a precise
inspection of what that level set — is the deliberate choice here. The override
grammar differs (`get` takes a `--default` flag, `path` a positional) to keep the
common `config get <key>` case a clean bare read and avoid a second positional;
unifying `path` onto the flag is a possible later follow-up (see "What We're NOT
Doing").

`--fail-safe` is **not** changed. Its role is only to keep the skill loadable on
a read failure (exit 0 rather than abort); it keeps the shared `Degrade::Suppress`
behaviour. It does not manufacture the catalogue default — a genuine read failure
yields empty output (or an error notice), which the SKILL surfaces to the user as
an error rather than resolving to a fabricated default. The normal `Absent`
resolution supplies the default in the common (unset) case.

Update the two `init-jira` callers and the cross-crate `migrate` port doc, and
reconcile the affected tests.

### Changes Required:

#### 1. Grammar: `--default` flag

**File**: `cli/launcher/src/launch/inbound/cli.rs`
**Changes**: In `ConfigAction::Get`, change the positional `default` to a
`#[arg(long)]` flag, and update the doc comment to state the catalogue fallback,
the override, the single-level behaviour, and how to discover catalogued keys.

```rust
/// Print a configuration value. Without `--level` the value resolves
/// personal-over-team, then the built-in default. `--default` overrides the
/// built-in default on a miss. With `--level`, only that level is read and no
/// built-in default is applied. A key that carries no built-in default (those
/// shown with a value under the `default` source in `config dump`) and has no
/// `--default` prints an empty line.
Get {
    /// The dotted `section.key` to read (e.g. `agents.reviewer`).
    key: String,
    /// Override the built-in default when the key is unset at both levels.
    #[arg(long)]
    default: Option<String>,
    // ... level, explain, allow_legacy_layout, fail_safe unchanged
}
```

The clap → `config_cli::Action::Get` map (`launch/mod.rs:35-48`) already passes
the `default` field through by name, so it needs no change beyond the field now
arriving from a flag. Use the phrase **built-in default** consistently (not
"catalogue default", an internal term) across this doc, the SKILL knob-resolution
block, and the `configure help` section — and, in the same pass, reconcile the
neighbouring `ConfigAction::Path` ("plugin-standard default", `cli.rs:125`) and
`ConfigAction::Work` ("catalogue default", `cli.rs:171`) doc comments to the same
term, so the enum reads with one name for the concept.

#### 2. Resolution: catalogue fallback

**File**: `cli/launcher/src/config_command/core/get.rs`
**Changes**: When resolving across levels (`level` is `None`), on
`Resolved::Absent` resolve a **non-empty** `--default` if supplied, else the
catalogue default via `default_for(raw_key)`, else empty. Filter an empty
`--default` so it falls through to the catalogue (matching `paths.rs`, which does
`default.filter(|value| !value.is_empty())`), removing the one behavioural
divergence from `path`. When `level` is `Some`, read only that level and do
**not** apply the catalogue fallback (single-level reads stay a precise
inspection of what that level set). Update **both** places that currently say
"never the catalogue default" — the `//!` module summary (`get.rs:1-2`) and the
`resolve` function doc (`get.rs:8-9`) — so neither contradicts the new behaviour.

```rust
let value = match config.get(&key, level)? {
    Resolved::Found(value) => config::render_value(&value),
    Resolved::Absent => match default.filter(|value| !value.is_empty()) {
        Some(explicit) => explicit.to_owned(),
        None if level.is_none() => catalogue::default_for(raw_key)
            .map(|value| config::render_value(&value))
            .unwrap_or_default(),
        None => String::new(),
    },
};
```

#### 3. Fail-safe stays standard

**File**: `cli/launcher/src/config_command/inbound/cli.rs`
**Changes**: **None.** The `Action::Get` arm keeps routing through
`finish_scalar` → `Degrade::Suppress`, and the `--fail-safe` doc comment ("Never
exit non-zero: on a read failure, print nothing and exit 0") stays accurate. A
read failure therefore yields empty output on exit 0 — enough to keep the skill
loadable; the SKILL treats an empty resolved knob as an error to surface (Phase
3). `config get` and `config path` thus share identical fail-safe behaviour.

#### 4. Update the callers and the migrate port doc

**File**: `skills/integrations/jira/init-jira/SKILL.md` (lines 64, 71)
**Changes**: Drop the positional `""`. `jira.site`/`jira.email` have no built-in
default, so a bare `config get jira.site` still resolves to empty on a miss —
behaviour unchanged, syntax corrected.

```text
`accelerator config get jira.site`. If still empty,
```

**File**: `cli/migrate/src/ports.rs` (`config_value` port doc, lines 101-104)
**Changes**: The doc comment describes the port as matching `accelerator config
get --allow-legacy-layout <key> ""` — the positional-`""` form this phase removes,
and it pairs an empty default with catalogue fallback, which now matches the new
behaviour (empty `--default` falls through to the catalogue). Update the analogy
to the flag form and the corrected semantics.

#### 5. Behaviour tests

**File**: `cli/launcher/tests/config_read.rs`
**Changes**:

- Rewrite `get_of_a_catalogue_backed_key_on_a_miss_does_not_inject_the_catalogue`
  (`:306`) to assert the catalogue default **is** now returned — rename to
  `get_of_a_catalogue_backed_key_on_a_miss_returns_the_catalogue_default`, e.g.
  `config get paths.plans` returns `meta/plans`.
- Rewrite `get_with_an_explicit_empty_default_yields_empty_on_a_miss` (`:198`) as
  `get_prefers_a_non_empty_default_over_the_built_in` — replacing its stale
  `jira-auth.sh` presence-probe docstring: bare `config get review.min_lenses`
  prints `4` (the built-in default), a non-empty
  `config get review.min_lenses --default 9` prints `9`, and an empty
  `config get review.min_lenses --default ""` falls through to `4` (matching
  `config path`).
- Switch `:187` (`get_of_an_unset_key_prints_the_callers_default`) and the `""`
  positional at `:200` to the `--default` flag. **`:320`** lives in
  `path_with_an_empty_default_falls_through_to_the_catalogue`, whose `get` half
  asserts `config get paths.plans ""` yields empty — the exact divergence this
  phase removes. Do not merely swap the syntax: change that `get` assertion to
  `meta/plans` (empty `--default` now falls through to the built-in default) and
  drop the divergence framing, or fold the `get` half into the rewritten `:306`.
- Add a black-box precedence test using an already-catalogued key so Phase 2
  stays independent of Phase 1: `config get review.min_lenses` returns the
  personal value, then the team value, then `4` when unset — end-to-end CLI
  coverage of the "Config precedence" criterion (the parity test in Phase 1 §6
  covers only `personal > team`, since `parity.rs`'s `resolve()` does not fold the
  catalogue). The research-key precedence is then confirmed manually and once
  Phase 1 has landed.
- Add a single-level test: `config get review.min_lenses --level team` on a miss
  prints empty (no built-in default under `--level`), while
  `config get review.min_lenses --level team --default 5` prints `5` (a
  `--default` flag is still honoured under `--level`).
- Add an uncatalogued-empty-default test: `config get jira.site --default ""` on
  a miss still prints empty (the presence-probe contract the old `:198`
  docstring described).
- **Leave `a_scalar_with_fail_safe_suppresses_a_read_failure_and_exits_zero`
  (`:424`) unchanged** — fail-safe still suppresses to empty, so
  `config get agents.reviewer --fail-safe` on a read failure stays empty.
- Leave `get_of_an_unset_key_without_a_default_prints_empty` (`:177`, a key with
  no built-in default) unchanged.
- Extend the `get --help` assertion (`:1795`) to pin a substring from the new doc
  (e.g. `--default` or `built-in default`), so the changed help contract is
  guarded.

#### 6. Changelog

**File**: `CHANGELOG.md`
**Changes**: Add a `### Changed` entry under `[Unreleased]` recording the breaking
`config get` change — the override moves from a positional argument to a
`--default` flag, and an unset key resolved across levels now returns the built-in
default rather than empty. `config get` is user-invokable from a terminal, so the
grammar change is user-visible even though the only in-repo callers are updated in
this phase.

### Success Criteria:

#### Automated Verification:

- [x] `config get` behaviour tests pass (catalogue fallback, non-empty `--default` override, empty-`--default` fall-through, single-level no-fallback, `--level` + `--default` honoured, uncatalogued empty-default, precedence chain): `mise run test:unit:cli`
- [x] `a_scalar_with_fail_safe_suppresses_a_read_failure_and_exits_zero` (`:424`) still passes unchanged: `mise run test:unit:cli`
- [x] `config get --help` renders and its extended assertion passes: `mise run test:unit:cli`
- [x] Rust format and clippy clean: `mise run cli:check`
- [x] Full read-only lane green: `mise run check`

#### Manual Verification:

- [x] `accelerator config get research.breadth` in a repo with neither level set
      returns `8`; with `research.breadth` in `config.md` (team) and a different
      value in `config.local.md` (personal), returns the personal value; with
      only the team value, the team value.
- [x] `accelerator config get research.breadth --default 3` returns `3` when the
      key is unset at both levels; `--default ""` falls through to `8`.
- [x] `accelerator config get research.breadth --level team` on an unset team
      level returns an empty line (no built-in default under `--level`).
- [x] `accelerator config get jira.site` (no built-in default) returns an empty
      line on a miss.

---

## Phase 3: Breadth as a live outline ceiling

### Overview

Replace the hardcoded breadth prose in `research-topic/SKILL.md` with a resolved
value read from the catalogue-backed `config get`, introduce the shared
`flag > config > default` resolution-and-validation rule, and make `outline` read
`--breadth` and size its round under the resolved ceiling. Because `config get`
now returns the catalogue default, the SKILL carries no `8` literal. Satisfies the
breadth halves of "Flag override" and "Validation", plus "Breadth ceiling" and
"Breadth not re-checked at conduct".

### Changes Required:

#### 1. Argument hint

**File**: `skills/research/research-topic/SKILL.md` (line 11)
**Changes**: Advertise the `outline` flag.

```text
argument-hint: "brief SUBJECT | outline SLUG [--breadth N] | conduct SLUG | synthesise SLUG | finalise SLUG"
```

#### 2. Knob-resolution block

**File**: `skills/research/research-topic/SKILL.md`
**Changes**: Replace the hardcoded bounding statement (lines 51-52) with a block
that injects the catalogue-resolved breadth and states the shared
resolution-and-validation rule. The depth line stays a plain statement here,
upgraded in Phase 4.

```markdown
Two knobs bound the research. **breadth** is the ceiling on focus areas an
`outline` round may commission; **depth** is the recursion limit within a
finding. breadth's configured value, resolved `personal > team > built-in
default`, is below; depth stays a fixed 1 until Phase 4 wires it to config:

- breadth: !`accelerator config get research.breadth --fail-safe`
- depth: 1 (one researcher per focus area, no recursion)

A verb resolves its knob as **flag > resolved value above**: an `--<knob> N` flag
on the invocation wins over the configured value. Then apply these rules in order:

- **Empty value** — if the resolved value is empty (an unreadable config, or a
  knob explicitly set to an empty value), do not proceed or guess a default: tell
  the user the research configuration is missing or unreadable, and stop.
- **Valid** — an integer of 1 or more passes unchanged.
- **Out of range or malformed** — a zero, negative, or non-integer value
  (a non-integer clamps regardless of magnitude) clamps to 1, and the verb warns,
  substituting the knob name being validated:

  > Warning: research.<knob> must be a positive integer, got '{value}' — clamping to 1

  single-quoting the offending value. There is no upper bound; breadth is the
  per-round cost guard.
- **Misplaced flag** — a flag belonging to the other verb (`--depth` on
  `outline`, `--breadth` on `conduct`) is ignored with a one-line note; it never
  clamps the verb's own knob.
```

#### 3. Outline ceiling rule

**File**: `skills/research/research-topic/SKILL.md`
**Changes**: Replace the hardcoded ceiling sentence (lines 145-147) with the
resolved ceiling and the flag read.

```markdown
Resolve breadth per the knob-resolution rule above, reading any `--breadth N`
flag on the invocation. Scale each round's focus areas to the subject under the
resolved breadth ceiling — the effort-scaling judgement may size a round beneath
the ceiling but never above it, and never write more focus areas than the ceiling
in a single round. The ceiling is per round, so an accreting set may exceed it
across rounds.
```

### Success Criteria:

#### Automated Verification:

- [ ] The new `config get research.breadth` preprocessor site executes cleanly: `mise run test:integration:skill-invocation`
- [ ] Invocation form and frontmatter/markdown lint pass: `mise run check` (includes `lint:bare-invocation:check`)

#### Manual Verification:

- [ ] With no config set, `outline SLUG` sizes the round under a ceiling of 8 (the
      catalogue default), and `outline SLUG --breadth 3` sizes it to at most 3.
- [ ] A resolved breadth of `0`, `-2`, or `2.5` clamps to 1 with the
      `Warning: research.breadth …` message naming the value; an integer ≥ 1 passes
      unchanged.
- [ ] `outline` never writes more focus areas than the resolved ceiling in one
      round; the effort-scaling judgement may size beneath it.
- [ ] `outline SLUG --depth 2` (a misplaced flag) is ignored with a note and does
      not affect breadth.
- [ ] An `outline.md` hand-edited above the ceiling, then `conduct SLUG`,
      researches every outstanding focus area without clamping (conduct remains
      breadth-free — no breadth reference in the `conduct` section).

---

## Phase 4: Depth threaded through conduct, dormant

### Overview

Upgrade the depth line to a catalogue-resolved value and make `conduct` read
`--depth`, always spawn one researcher per focus area, and print a notice when the
resolved depth exceeds 1. Satisfies the depth halves of "Flag override" and
"Validation", and "Depth dormant with a signal". The `conduct` edits add depth
only — no breadth re-check, preserving Phase 3's "not re-checked at conduct"
property. The dormancy caveat is phrased in plain language — no internal work-item
id appears in user-facing output.

### Changes Required:

#### 1. Argument hint

**File**: `skills/research/research-topic/SKILL.md` (line 11)
**Changes**: Advertise the `conduct` flag alongside the `outline` flag from
Phase 3.

```text
argument-hint: "brief SUBJECT | outline SLUG [--breadth N] | conduct SLUG [--depth N] | synthesise SLUG | finalise SLUG"
```

#### 2. Resolved depth value

**File**: `skills/research/research-topic/SKILL.md`
**Changes**: In the knob-resolution block from Phase 3, broaden the intro line so
both values read as resolved (drop the "depth stays a fixed 1 until Phase 4"
clause), replace the plain depth line with the catalogue-resolved read, and
append the conduct-notice sentence to the rule.

```markdown
- breadth: !`accelerator config get research.breadth --fail-safe`
- depth: !`accelerator config get research.depth --fail-safe`
```

```markdown
Depth is dormant: `conduct` always spawns exactly one researcher per focus area
regardless of the resolved depth. When conduct's resolved depth exceeds 1, it
prints this notice before spawning:

> depth resolved to {value}, but recursive deepening is not yet available; conducting at depth 1 (one researcher per focus area)
```

#### 3. Conduct depth read and notice

**File**: `skills/research/research-topic/SKILL.md`
**Changes**: In the `conduct` section (after the reconcile step, before the spawn
loop at lines 168-184), read the resolved depth, validate it, and gate the
notice. Reference the single notice defined in the knob-resolution block rather
than restating it; add no breadth clause.

```markdown
Resolve depth per the knob-resolution rule above, reading any `--depth N` flag on
the invocation. Depth is dormant: spawn exactly one researcher per outstanding
focus area whatever the resolved value. When the resolved depth exceeds 1, print
the depth notice defined in the knob-resolution block before spawning.
```

### Success Criteria:

#### Automated Verification:

- [ ] The new `config get research.depth` preprocessor site executes cleanly: `mise run test:integration:skill-invocation`
- [ ] Invocation form and frontmatter/markdown lint pass: `mise run check`

#### Manual Verification:

- [ ] With no config set, `conduct SLUG --depth 2` fires the notice (proving the
      flag resolved to 2 over the catalogue default 1) and still spawns one
      researcher per focus area.
- [ ] A resolved depth of 1 spawns one researcher per focus area and emits no
      notice; a resolved depth greater than 1 spawns one per focus area (no
      recursion) and emits the notice.
- [ ] A resolved depth that is zero, negative, or non-integer clamps to 1 with the
      `Warning: research.depth …` message naming the value.
- [ ] The notice contains no internal work-item id — only plain "not yet
      available" language.
- [ ] The `conduct` edits introduce no breadth re-check.

---

## Phase 5: Document the knobs in configure help

### Overview

Add a `### research` section to the `configure help` reference block, documenting
both knobs, their defaults, the resolution order, and depth's dormancy caveat in
plain language. Place it adjacent to the `### review` section — the two document
behavioural tuning knobs — rather than beside the path namespaces. Pure
documentation; satisfies the "Docs" acceptance criterion.

### Changes Required:

#### 1. Research help section

**File**: `skills/config/configure/SKILL.md`
**Changes**: Insert a `### research` section inside the help fence, directly after
the `### review` family (before the per-skill-customisation content), following
the `### review` template shape (intro, table, escaped YAML example, notes). Do
not fold into `### paths` — that namespace documents path keys, not behavioural
knobs. Keep the Description cells short and padded to the longest so the table
stays column-aligned; put the depth dormancy detail in the prose beneath.

```markdown
### research

Tune how wide and deep `/accelerator:research-topic` goes. Config keys live under
the `research` namespace; both are positive integers. (These behavioural knobs are
distinct from the `paths.research_*` output-directory keys under `### paths`.)

| Key       | Default | Description                            |
|-----------|---------|----------------------------------------|
| `breadth` | `8`     | Focus-area ceiling per `outline` round |
| `depth`   | `1`     | Recursion limit within a finding       |

Resolution order for each knob is **flag > personal (`config.local.md`) > team
(`config.md`) > built-in default**: an `outline SLUG --breadth N` or `conduct
SLUG --depth N` flag on the invocation overrides config, which overrides the
built-in default. A resolved value that is not an integer of 1 or more is clamped
to 1 with a warning naming the invalid value. There is no upper bound — breadth is
the per-round cost guard.

`depth` has no behavioural effect yet — recursive deepening is not yet available.
A resolved depth above 1 prints a notice and still conducts one researcher per
focus area.

Example configuration:

\```yaml
---
research:
  breadth: 6
  depth: 1
---
\```

Note: YAML comments (`#`) are not supported by the config parser. Do not
add inline comments to config values.
```

### Success Criteria:

#### Automated Verification:

- [ ] Markdown lint on the help block passes: `mise run check`

#### Manual Verification:

- [ ] `/accelerator:configure help` renders the `### research` section adjacent to
      `### review`, with both knobs, their defaults (`8`, `1`), the
      `flag > personal > team > default` order, and a plain-language dormancy
      caveat with no internal work-item id.

---

## Testing Strategy

### Unit Tests (Phase 1):

- Catalogue key count (`65` across seven groups) and the declared-value
  assertion for `research.breadth`/`research.depth` (`catalogue.rs` test module).
- The parity resolution assertion against a team/personal fixture
  (`config-adapters/tests/parity.rs`), exercising `personal > team` (the built-in
  default leg is covered by the Phase 2 black-box chain test, since `parity.rs`'s
  `resolve()` does not fold the catalogue).
- Edge cases owned by existing machinery: `default_for` returns the typed scalar;
  `dump` attributes by presence.

### Behaviour and Golden Tests (Phases 1-2):

- `config get` catalogue fallback (`:306` rewritten), non-empty `--default`
  override and empty-`--default` fall-through (`:198` rewritten), single-level
  no-fallback, and the black-box `research.breadth` precedence chain
  (`cli/launcher/tests/config_read.rs`).
- The fail-safe suppression test (`:424`) stays green **unchanged** — fail-safe is
  not altered, so a catalogued key still suppresses to empty on a read failure.
- `dump_matches_the_committed_golden` (`cli/launcher/tests/config_read.rs:802`)
  against the two new golden rows.
- `public-api:check` against the inserted `RESEARCH_KEYS` line.

### Integration Tests (Phases 3-4):

- `test:integration:skill-invocation` extracts and runs every `!`accelerator
  config …`` site from `skills/**/SKILL.md`; the two new `config get` reads must
  execute cleanly under an empty environment.
- `lint:bare-invocation:check` (within `mise run check`) guards the bare
  `accelerator` invocation form of the new sites.

### Manual / Eval-level Testing Steps:

1. In a scratch repo, set `research.breadth`/`research.depth` at team and personal
   levels and confirm `config dump` and `config get` precedence, including the
   unset case resolving to the built-in default.
2. Confirm `config get research.breadth --default 3` overrides on a miss,
   `--default ""` falls through to `8`, `--level team` on a miss prints empty, and
   `config get jira.site` (no built-in default) prints empty.
3. Invoke `outline SLUG --breadth 3` and confirm the round is sized to ≤ 3.
4. Invoke `outline SLUG --breadth 0` (and `-1`, `2.5`) and confirm the clamp
   warning names the value and states the clamp.
5. Invoke `outline SLUG --depth 2` (misplaced flag) and confirm it is ignored with
   a note.
6. Hand-edit `outline.md` above the ceiling, run `conduct SLUG`, and confirm every
   focus area is researched without clamping.
7. Invoke `conduct SLUG --depth 2` and confirm the notice (no work-item id) fires
   and exactly one researcher per focus area is spawned; repeat at depth 1 and
   confirm no notice.
8. Read `configure help` and confirm the research section content and placement.

## Performance Considerations

⏱️ `breadth` is the per-round cost guard: it caps focus areas (and therefore
parallel researcher spawns and web fetches) an `outline` round may commission. It
does **not** bound `conduct`'s peak fan-out — `conduct` spawns one researcher per
outstanding focus area across all rounds, so peak concurrency is the accumulated
set (`breadth × rounds`, or an arbitrary hand-edited `outline.md`), uncapped by
design. In practice the Task harness's own parallel-subagent limit throttles the
spawn, so "uncapped" means bounded by the runtime, not unlimited simultaneous web
fetches. `depth > 1` would multiply spend, which is why it ships dormant — the
notice makes an over-ambitious depth visible without incurring its cost.

## Migration Notes

The research keys are additive with catalogue defaults; unset repos resolve to
`8`/`1`. There is no unset sentinel (ADR-0047) — personal config can override a
team value but not clear it to the default. No data migration.

⚠️ The `config get` change is a breaking change to the CLI surface, and it
deliberately reverses the catalogue-blindness work item 0167 gave `config get`:
the override moves from a positional argument to a `--default` flag, and an unset
catalogued key resolved across levels now returns the built-in default rather than
empty. `--fail-safe` is unchanged (a read failure still suppresses to empty). The
in-repo consumers are the two `init-jira` sites and the `cli/migrate`
`config_value` port doc (both updated in Phase 2); no external consumer exists.
`config get` and `config path` remain deliberately different on three axes — see
the Phase 2 table — override grammar (flag vs positional), unknown-key handling
(get accepts, path refuses), and single-level reads (`config get --level` applies
no built-in default, `config path --level` still folds it); the empty-`--default`
edge is the one aligned (both fall through to the built-in default).

⚠️ Landing-order coupling with the academic-sources slice (0280): that work also
registers a `research.*` key (`research.contact_email`) touching the key-count
test, `dump.golden`, and `public-api.txt`. 0282 lands first (0280 is `draft`), so
0280 reconciles these fixtures for its key when it lands. If 0280 lands first
after all, whichever lands second reconciles the count and fixtures for the
other's key.

## References

- Original work item: `meta/work/0282-tunable-depth-and-breadth.md`
- Related research: `meta/research/codebase/2026-09-20-0282-tunable-depth-and-breadth.md`
- Work item review: `meta/reviews/work/0282-tunable-depth-and-breadth-review-1.md`
- Plan review: `meta/reviews/plans/2026-09-20-0282-tunable-depth-and-breadth-review-1.md`
- Precedent — numeric tunable group: `cli/config/src/catalogue.rs:150-176` (`REVIEW_KEYS`)
- Precedent — catalogue-backed single-key read + empty-default filter: `cli/launcher/src/launch/inbound/cli.rs:124-131`, `cli/launcher/src/config_command/core/paths.rs:158` (`config path`)
- Decision being reversed — `config get` catalogue-blindness: work item 0167 (`config-command-and-invocation-contract-migration`) and its plan review
- Precedent — validated numeric consumption / warning house style: `cli/launcher/src/config_command/core/review.rs:554-570`
- Precedent — flag over config: `cli/work-cli/src/create.rs:637` (`--project` over `work.key`)
- `config get` today (to change): `cli/launcher/src/config_command/core/get.rs:14-30`; grammar at `cli/launcher/src/launch/inbound/cli.rs:89-108`
- `config get` behaviour tests (to update): `cli/launcher/tests/config_read.rs:187,198,200,306,320`; `:424` stays unchanged (fail-safe not altered)
- Callers to update: `skills/integrations/jira/init-jira/SKILL.md:64,71`; `cli/migrate/src/ports.rs:101-104` (port doc analogy)
- Edit targets: `skills/research/research-topic/SKILL.md:51-52,145-147,155-206`; `skills/config/configure/SKILL.md` (help block, `### review` neighbourhood)
- Blocks: `meta/work/0283-recursive-finding-deepening.md` (the recursion engine that consumes `depth`)
