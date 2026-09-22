---
type: "codebase-research"
id: "2026-09-20-0282-tunable-depth-and-breadth"
title: "Research: Tunable Depth and Breadth (0282)"
date: "2026-09-20T21:47:34+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0282"
parent: "work-item:0282"
relates_to: ["codebase-research:2026-09-10-0228-layered-configuration-key-model", "codebase-research:2026-09-08-0277-single-round-web-research-engine"]
topic: "Tunable Depth and Breadth"
tags: ["research", "codebase", "config", "catalogue", "research-topic", "skills"]
revision: "079a6fd672ee1f65e3734d317b3c321b2944f047"
repository: "accelerator"
last_updated: "2026-09-20T21:47:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Tunable Depth and Breadth (0282)

**Date**: 2026-09-20 21:47 UTC
**Author**: Toby Clemson
**Git Commit**: 079a6fd672ee1f65e3734d317b3c321b2944f047
**Branch**: HEAD (jj working copy)
**Repository**: accelerator

## Research Question

Where do the requirements of work item 0282 — register `research.breadth`
(default 8) and `research.depth` (default 1) as config knobs, thread them through
the `outline`/`conduct` research verbs with `flag > personal > team > default`
resolution and clamp-and-warn validation, keep `depth` dormant until 0283 —
meet existing machinery, and where do they demand net-new code or prose?

## Summary

**0282 is almost entirely extend-not-invent on the config side, and net-new
prose on the SKILL.md side — with one deliberate architectural divergence from
its own cited precedent that an implementer must go in knowing.**

Three findings shape the work:

- **Registering the keys mirrors `REVIEW_KEYS` exactly, but touches five call
  sites, not the "catalogue plus a test" the story implies.** There is no master
  group list; `default_for` and `dump::assemble` each enumerate groups by hand,
  and `public-api.txt` pins the public surface alphabetically. Adding
  `RESEARCH_KEYS` means: the declaration, the `default_for` scan array, a
  `dump::assemble` loop, the alphabetical `public-api.txt` insertion, and the
  key-count test (whose name encodes the count — `sixty_three`→`sixty_five`,
  `six_groups`→`seven_groups`).
- **The validation model 0282 specifies (clamp-and-warn in SKILL.md prose)
  diverges from the `review.*` precedent it cites (reject-and-default in typed
  Rust).** `review.rs` validates in Rust and hands the SKILL a guaranteed-valid
  number; 0282 asks the SKILL prose to validate. This is defensible (the knobs are
  prompt-resolved, so validation must sit where resolution sits), but it is a
  divergence, not a mirror — and "clamp" differs from review's "reject and
  substitute default" too.
- **The effort-scaling rubric the story references does not exist in the shipped
  SKILL.md.** The `1 / 2–4 / 10+` bands live only in the 0121 epic and its review
  docs. `research-topic/SKILL.md` carries a single sizing sentence bounded by "the
  breadth ceiling of 8"; there is no rubric block to edit — only that sentence and
  the two global bounding statements.

The config precedence the story assumes (`personal > team > default`) is
**confirmed** in `service.rs`. One subtlety matters for the SKILL idiom:
`config get --default N` deliberately **never consults the catalogue** — its
`--default` is the fallback when both config levels are absent. So
`config get research.breadth --default 8` is itself the `personal > team > 8`
resolution, and the `--breadth` flag sits above it in prose. This work item's own
review (review-1, verdict COMMENT, all findings resolved) already pinned the
ambiguous edges: non-integer clamps regardless of magnitude, and the clamp
warning must name the invalid value.

## Detailed Findings

### Config registration — mirror `REVIEW_KEYS`, wire five sites by hand

`REVIEW_KEYS` is the exact template. It is a `pub const REVIEW_KEYS: &[(&str,
Default)]` slice of `(key, default)` tuples (`catalogue.rs:150-176`); each entry
has only a dotted key name and a `Default`, with no description field. `Default`
is a two-variant enum, `Scalar(&'static str)` / `Seq(&'static [&'static str])`
(`catalogue.rs:10-13`).

**Numeric defaults are stored as string scalars, never integers — confirmed.**
`review.max_lenses: 8` is the literal `Default::Scalar("8")` (`catalogue.rs:153`),
which `to_value` (`catalogue.rs:16-28`) turns into
`Value::Scalar(Scalar::String("8"))`. The `Scalar` enum has `Int`/`Float`
variants, but the catalogue never constructs them; the consumer parses the string.
So the new group is
`RESEARCH_KEYS = &[("research.breadth", Default::Scalar("8")),
("research.depth", Default::Scalar("1"))]`.

There is **no single master list of groups** — every consumer enumerates them
independently, so a new group must be wired into each by hand:

| Site | File:line | Change |
|---|---|---|
| Declaration | `catalogue.rs:150` (next to `REVIEW_KEYS`) | Add `RESEARCH_KEYS` slice |
| Default resolution | `catalogue.rs:245` | Add `RESEARCH_KEYS` to the scan array |
| Dump output | `dump.rs:45-77` | Add a `for (key,_) in RESEARCH_KEYS` loop |
| Public-API snapshot | `cli/config/tests/fixtures/public-api.txt:12-13` | Insert `RESEARCH_KEYS` (alphabetical) |
| Key-count test | `catalogue.rs:263-280` | `+ RESEARCH_KEYS.len()`, `63`→`65`, rename |

Two of these are sharper than the work item states. `default_for` scans a
hardcoded array — `for group in [PATH_KEYS, WORK_KEYS, REVIEW_KEYS,
VISUALISER_KEYS]` (`catalogue.rs:245`) — and `RESEARCH_KEYS` must be added there
or `default_for("research.breadth")` returns `None`. The key-count test is
`the_catalogue_holds_sixty_three_keys_across_six_groups` (`catalogue.rs:270-280`):
it sums six groups, asserts `63`, and its **name encodes both numbers**, so the
edit is `+ RESEARCH_KEYS.len()`, `63`→`65`, `sixty_three`→`sixty_five`,
`six_groups`→`seven_groups`, plus adding `RESEARCH_KEYS` to the test module's
`use` import (`catalogue.rs:263-266`). ⚠️ `public-api.txt` is alphabetical:
`RESEARCH` sorts *before* `REVIEW`, so the new line goes between `PATH_KEYS`
(line 12) and `REVIEW_KEYS` (line 13).

**No `research.*` config key exists today** — not `research.breadth`,
`research.depth`, nor the `research.contact_email` that 0280 will add. The 23
"research" hits in `catalogue.rs` are all path keys (`paths.research_codebase` →
`meta/research/codebase`), doc types, templates, and agent names — a distinct
namespace from the proposed behavioural knobs.

⚠️ **Workspace mirrors.** Every `cli/...` file above also exists under four
parallel `workspaces/<name>/cli/...` trees (jj workspaces). The canonical tree is
top-level `cli/`; confirm whether the `workspaces/*` copies are live checkouts or
throwaway before assuming they need edits.

### Golden and parity fixtures the new keys will move

- **`dump.golden`** — `cli/launcher/tests/fixtures/dump/dump.golden`, the
  byte-exact expected stdout of `config dump`, asserted by
  `dump_matches_the_committed_golden` (`config_read.rs:802-805`). The two new rows
  land here once `dump::assemble` emits them.
- **`parity.rs`** — `cli/config-adapters/tests/parity.rs` asserts resolved config
  values match declared fixtures (it calls `default_for("review.core_lenses")` at
  line 112). It is the config-catalogue parity file; three other same-named
  `parity.rs` files under `cli/` are unrelated (remote-projection, corpus-adapters,
  visualiser).
- **`public-api.txt`** — one per crate; only `cli/config/tests/fixtures/public-api.txt`
  is load-bearing here (it enumerates each `pub const` group).

### Numeric-tunable consumption — the `review.rs` precedent, and where 0282 diverges

`review.rs` validates entirely in typed Rust and is invoked by (not defined in) a
SKILL.md. The shape is **reject-and-substitute-default**, not range-clamp:

- `is_non_negative` (`review.rs:532-534`) — the shared predicate: non-empty and
  all-ASCII-digits (so any sign, whitespace, or decimal point fails).
- `non_negative` (`review.rs:536-552`) and `positive` (`review.rs:554-570`) —
  read the raw string via `resolve`, accept if valid, else **push a warning and
  return the default**. `positive` adds a `!= "0"` guard.
- `int` (`review.rs:528-530`) — a bare `parse().unwrap_or(0)`, used only for the
  `min > max` comparison, which reverts *both* values to their defaults
  (`review.rs:84-101`).

The warning wording is the house style to mirror:
`"review.{key} must be a positive integer, got '{value}' — using default
({default})"` — it single-quotes the invalid value, names the default, and joins
with an em-dash. The renderer prefixes each with `Warning: ` to stderr
(`render/review.rs:86-90`).

⚠️ **The division of labour is the load-bearing contrast.** For review, **Rust
owns validation** (parse, reject, warn, default) and the **SKILL owns only
behavioural application** — `review-pr/SKILL.md:24` injects the pre-validated
block, and lines 233-237 cap at `max_lenses` / floor at `min_lenses` in prose.
0282 instead puts *validation itself* in SKILL.md prose. That is a genuine
divergence from the cited precedent, driven by the knobs being prompt-resolved
rather than consumed by a compiled subcommand. Two consequences an implementer
should weigh: (1) "clamp to 1" is not review's behaviour (review substitutes the
key's default and does not clamp); (2) prose validation is an eval-level contract,
not a unit test, so it cannot reuse `review.rs`'s tested helpers. If the team
wanted the tested-Rust path, the alternative would be an `accelerator config
research <verb>` subcommand echoing `config review` — larger than 0282's scope but
the consistent-with-review option.

### Config precedence and the `config get --default` idiom

**Precedence is `personal > team > catalogue default` — confirmed**
(`service.rs:388-413`, `448-459`). `effective` reads personal first, then team,
then folds the catalogue default via `default_for`, tagging the source
`Personal` / `Team` / `Catalogue`.

The idiom the SKILL will use has one crucial property: `config get <key>
--default <n>` (`core/get.rs:14-30`) calls raw `config.get` (which applies
`personal > team`) and, on absence from **both** levels, returns the `--default`
string — it **never consults the catalogue**. So:

```text
config get research.breadth --default 8
  → personal value, else team value, else "8"
```

That is precisely `personal > team > hardcoded-8`, with the `8` written at the
call site rather than read from the catalogue. The `--breadth` invocation flag
then sits above this in SKILL prose, giving the full `flag > personal > team > 8`
chain 0282 specifies. The catalogue's `research.breadth: 8` entry is therefore
what surfaces in `config dump` and `default_for`; the SKILL's resolved default
comes from the `--default 8` literal. ⚠️ Keep the two `8`s in sync by hand — they
are independent (this mirrors the line-width duplication the repo already
tolerates).

`config dump` attributes each key's source by *presence*, not value
(`dump.rs:91-101`): `effective().source()` maps to the labels
`local (.accelerator/config.local.md)` / `team (.accelerator/config.md)` /
`default` (`render/dump.rs:39-45`). A key set to its default string still
attributes to the level that set it. Config lives in exactly two in-repo files —
`.accelerator/config.md` (team) and `.accelerator/config.local.md` (personal),
parsed as YAML frontmatter (`store.rs`); **there is no home-directory level**.

### Flag-over-config precedent (`work create --project`)

The repo's flag-over-config idiom is one line: `let project =
args.project.clone().or_else(|| scheme.key.clone())` (`create.rs:637`), with a
terminal `unwrap_or("")`. The `--project` flag (an `Option`) wins; else the
config-derived `scheme.key`; the catalogue default is folded in earlier inside
`effective_nonempty`, so only two layers are visible at the call site.

Translated to SKILL prose above a `config get`: check the flag first; if unset,
fall through to `config get <key> --default <n>` (which is itself the config-then-
literal fallback). The ordering to preserve is **explicit flag → `config get`
→ literal default**. This is Rust; 0282's version is prose, but the precedence
shape is identical.

### `research-topic/SKILL.md` — the primary edit target

The verbs are pure prose with no arg parser. Dispatch is a prose instruction
(`SKILL.md:47-49`); the `SLUG` positional is handed straight to `accelerator
corpus resolve` (`SKILL.md:54-64`). The grammar today is strictly *verb + one
positional* — **there is no existing example of reading a flag** like `--breadth`
from the invocation text, so that idiom is net-new prose.

The hardcoded values sit in three places:

- **Global bounding statement** (`SKILL.md:51-52`): "**breadth is 8** (at most
  eight focus areas per round) and **depth is 1** (one researcher per focus area,
  no recursion)."
- **`outline`** (`SKILL.md:123` heading, `145-147` rule): "Scale each round's
  focus areas to the subject under the breadth ceiling of 8 — never write a ninth
  focus area in a single round."
- **`conduct`** (`SKILL.md:155` heading, `168-184` spawn loop): one researcher per
  outstanding focus area; there is no `conduct`-local `depth: 1` token beyond the
  global statement and the heading.

⚠️ **The effort-scaling rubric with a `10+` band is not in this file.** The only
sizing guidance is the single sentence at 145-147; the word "effort-scaled"
appears only in the heading and description. The `1 / 2–4 / 10+` bands exist only
in `meta/work/0121-topic-research-skillset.md` and its review. So the work item's
"the effort-scaling rubric may reduce beneath N but never raise above it" (AC:
Breadth ceiling) describes model judgement bounded by the 145-147 sentence, not a
rubric block to edit.

**`breadth` is enforced at `outline` only — confirmed.** The entire `conduct`
section (`SKILL.md:155-206`) contains zero references to breadth/ceiling/eight; it
operates on "each still-outstanding focus area across all rounds"
(`SKILL.md:163-166`) with no cap re-applied. This matches AC "Breadth not
re-checked at conduct" without a code change — `conduct` stays breadth-free and
gains only `depth` threading.

Config-read plumbing is ready: `allowed-tools` already whitelists `Bash(accelerator
config *)` (`SKILL.md:12-16`), and the file uses the `!`preprocessor`` idiom
heavily (`config context`, `agents`, `path`, `template`, `agent`, `instructions`,
each with `--fail-safe`). It **never calls `config get` today**, so the new reads
are net-new but need no allowlist change:

```text
!`accelerator config get research.breadth --default 8 --fail-safe`
!`accelerator config get research.depth --default 1 --fail-safe`
```

Prose patterns to mirror for the two signals: the **quoted-example notice** at
`SKILL.md:239-241` (a reopen that "prints" a literal string) is the template for
the depth>1 notice; the **refuse-and-name** style at `SKILL.md:84-87` / `107-110`
is the template for the clamp warning that "names the invalid value"; the
**outcome-report** line at `SKILL.md:142-143` models a short observable status
line.

### `configure/SKILL.md` — docs are the only change

`configure help` emits its entire reference as one fenced block (outer ``` opens
at `SKILL.md:111`, closes at `976`); nested examples use escaped fences. Each
config namespace is a `###` heading with a consistent template: an intro sentence,
a `Key | Default | Description` table, an escaped YAML example, and trailing
notes. The `review` section (`SKILL.md:165-226`) is the representative shape.

A new `### research` section belongs inside the help fence, naturally after
`### review` (before `### paths` at 388) or after `### paths` (before `### work` at
439). ⚠️ Do **not** fold the knobs into `### paths` — `paths` already documents
`research_codebase`/`research_issues` path keys, a distinct namespace from the
behavioural `research.breadth`/`research.depth`.

The precedence note has two established idioms to mirror: the **bold `A > B > C`
chain** (visualiser, `SKILL.md:652-653`, `674-676`, `681-682`) and the **arrow
chain** (templates, `SKILL.md:907-909`); local-over-team is stated once at
`SKILL.md:27-29`. No section documents a CLI-flag tier feeding into config, so the
`flag > personal > team > default` phrasing is net-new. For depth's dormancy,
mirror the inline **`**Deprecated**`-in-Description-cell** annotation (work table,
`SKILL.md:448`) or the "not currently supported" framing (`SKILL.md:969-975`).

This SKILL has **no `get`/`set` verb** — config is edited via Read/Write/Edit
directly — so registering the knobs is purely a documentation edit to the help
block, with no key-enumeration code path to update here.

## Code References

- `cli/config/src/catalogue.rs:150-176` — `REVIEW_KEYS` template to mirror.
- `cli/config/src/catalogue.rs:10-28` — `Default` enum and `to_value` (string
  scalars).
- `cli/config/src/catalogue.rs:244-259` — `default_for`; scan array at line 245.
- `cli/config/src/catalogue.rs:263-280` — key-count test and its `use` import.
- `cli/launcher/src/config_command/core/dump.rs:45-77` — `assemble` per-group
  loops.
- `cli/config/tests/fixtures/public-api.txt:12-13` — alphabetical insert point.
- `cli/launcher/tests/fixtures/dump/dump.golden` — `config dump` golden.
- `cli/config-adapters/tests/parity.rs` — declared-value parity for the catalogue.
- `cli/launcher/src/config_command/core/review.rs:528-570` — validate helpers
  (`is_non_negative`, `positive`, `non_negative`, `int`).
- `cli/launcher/src/config_command/core/review.rs:84-101` — `min > max`
  reconciliation.
- `cli/launcher/src/config_command/render/review.rs:86-90` — `Warning:` prefix.
- `cli/config/src/service.rs:388-459` — `effective`/precedence/`default_resolution`.
- `cli/launcher/src/config_command/core/get.rs:14-30` — `config get --default`
  (skips catalogue).
- `cli/launcher/src/config_command/core/dump.rs:91-101` +
  `render/dump.rs:39-45` — source attribution and labels.
- `cli/work-cli/src/create.rs:637` — `--project` flag-over-config idiom.
- `skills/research/research-topic/SKILL.md:51-52,123,145-147,155,168-184` — the
  hardcoded breadth/depth and edit sites.
- `skills/research/research-topic/SKILL.md:239-241,142-143,84-87,107-110` —
  notice/warn/refuse prose patterns.
- `skills/config/configure/SKILL.md:111-976` — help reference block;
  `165-226` review section; `652-653`/`907-909` precedence idioms; `448` dormancy
  annotation.

## Architecture Insights

- **Config is stringly-typed by design.** Adding keys is a catalogue entry plus a
  consuming read; no deserialise or type change (confirmed here and in the 0228
  research). The cost of 0282 is the by-hand group enumeration and the SKILL prose,
  not schema plumbing.
- **Validation home is the real design decision.** The repo has one tested
  precedent (`review.rs`, Rust) and 0282 chooses the other (prose). This is the
  `ADR-0045` skills-vs-CLI seam: prompt-resolved knobs pull validation into the
  prompt, at the cost of eval-level rather than unit-level verification. The
  work item accepts this explicitly (Assumptions, Technical Notes).
- **`config get --default` is the prose stand-in for `effective_nonempty`.** It
  deliberately skips the catalogue so the SKILL literal is the single source of the
  default. This keeps the flag/config/default arithmetic entirely in prose, above
  a thin CLI read — consistent with `ADR-0053` (thin CLI over a hexagonal core).
- **`breadth` and `depth` are asymmetric by construction.** `breadth` is a live
  outline-time ceiling; `depth` is registered, resolved, threaded, and documented
  but inert until 0283, emitting only a notice when > 1. Shipping `depth` dormant-
  but-honest de-risks 0283 (review-1 confirmed this bundling as a conscious,
  defensible call).
- **The dormant window has no version gate.** As the 0228 research notes, the repo
  has no semver feature-gating; "no effect until 0283" is enforced by 0283 shipping
  the recursion engine, and by `conduct`'s notice in the interim, not a runtime
  check.

## Historical Context

- `meta/reviews/work/0282-tunable-depth-and-breadth-review-1.md` — this work
  item's own review (verdict COMMENT, two passes, **all findings resolved, ready
  for implementation**). It pinned the edges an implementer relies on: a
  non-integer clamps to 1 *regardless of magnitude* (only integers ≥1 pass); the
  clamp warning must name the invalid value and state the clamp; the `--depth 2`
  flag-override is observable only via the depth notice firing. One open optional
  item: the shared-catalogue coupling with 0280 (below).
- `meta/research/codebase/2026-09-10-0228-layered-configuration-key-model.md` —
  the direct precedent for adding config keys and the shared-fixture touch list
  (key-count test, `dump.golden`, `public-api.txt`). Establishes that config is a
  dotted-key tree, `EXTRA_KEYS` vs default-carrying groups differ, and precedence
  is `personal > team > catalogue`.
- `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md` — the
  team/personal last-writer-wins model. ⚠️ **No unset sentinel**: personal config
  can only override a team value with a concrete value, never clear it back to the
  default. For these knobs that means a user cannot "unset" a team `research.depth`
  from personal config — only override it.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — the seam that
  makes prose-resolved knobs a legitimate choice against the Rust `review.*`
  precedent.
- `meta/work/0121-topic-research-skillset.md` — the parent epic; Slice 5 is the
  origin of the breadth/depth knob half, and the *only* home of the `1 / 2–4 /
  10+` effort-scaling rubric (which is not in the shipped SKILL.md).
- `meta/work/0277-single-round-web-research-engine.md` +
  `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md` —
  Slice 1, which authored the `outline` breadth ceiling and the hardcoded
  `breadth: 8`/`depth: 1` prose that 0282 replaces (transitively upstream via the
  done 0279).

## Related Research

- `meta/research/codebase/2026-09-10-0228-layered-configuration-key-model.md` —
  config-key registration and precedence.
- `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md` —
  the research engine 0282 wires into.
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md` —
  config crate internals.
- `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
  — the `config` command surface.

## Open Questions

- ❓ **Are the `workspaces/*/cli/...` mirror trees live or throwaway?** If live,
  every catalogue/fixture edit doubles across five trees. Confirm before
  implementation (locator flagged this; not resolvable read-only from the tree
  shape alone).
- ❓ **Does the plan want prose-only validation, or an `accelerator config
  research` subcommand?** 0282 specifies prose (eval-level contract). A Rust
  subcommand echoing `config review` would be consistent with the `review.rs`
  precedent and unit-testable, but exceeds 0282's stated scope. The work item has
  chosen prose; this is recorded so the divergence from precedent is a conscious
  plan-time decision, not an oversight.
- ❓ **Landing order with 0280.** 0280 also registers a `research.*` key
  (`research.contact_email`) touching the same key-count test, `dump.golden`, and
  `public-api.txt`. Whichever of 0282/0280 lands second reconciles those files for
  the other's key (review-1 open item; a merge cost, not a blocker).
</content>
</invoke>
