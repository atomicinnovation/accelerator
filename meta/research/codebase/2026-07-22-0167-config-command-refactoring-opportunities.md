---
type: codebase-research
id: "2026-07-22-0167-config-command-refactoring-opportunities"
title: "Research: Duplication, layering, and shared-crate placement in the 0167 config-command migration"
date: "2026-07-22T16:24:52+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0167"
parent: "work-item:0167"
relates_to:
  - "codebase-research:2026-07-19-0167-config-command-and-invocation-contract-migration"
topic: "Duplication, adapter-vs-core layering, and shared-crate placement in the implemented 0167 config-command migration"
tags: [research, codebase, rust, cli, config, launcher, store, adapters, hexagon, refactoring]
revision: "407b626cb01aa9f77270bc1680f049ccde84629c"
repository: "build-system"
last_updated: "2026-07-22T16:24:52+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Duplication, layering, and shared-crate placement in the 0167 config-command migration

**Date**: 2026-07-22 16:24 UTC
**Author**: Toby Clemson
**Git Commit**: 407b626cb01aa9f77270bc1680f049ccde84629c
**Branch**: (detached / workspace `build-system`)
**Repository**: build-system

## Research Question

We recently implemented the plan
`meta/plans/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
(changes since `tmmrknzvmlrztsstropxzxnkrwokrvpm`). Analyse the changes to determine:

1. Areas of **duplication** that should be refactored out.
2. Logic placed in **adapters** that would be better in the application **core**, and
   vice-versa.
3. Logic that should be moved into **shared crates** (e.g. the `document` crate,
   the `kernel` crate).

## Summary

The migration built the `config` command family as a clean three-layer hexagon
(`inbound/` → `core/` → `render/`) and consolidated `atomic_write` into a new
`store` crate. The **big-ticket architectural decisions are sound and the plan's
own reviewers signed them off**: the `store` extraction with a generalised
`permitted_root`, the private `to_config_error`/`to_store_error` translators
(forced by the pup domain rules + orphan rule), the fail-safe contract, and the
buffer-then-emit-on-success mechanism are all implemented once and reused
correctly.

The refactoring opportunities are almost entirely **within the launcher's
`config_command`**, and they cluster around one root cause: **the domain `config`
crate exposes `get` (value only) and `catalogue::default_for` (default only), but
no "effective value" operation and no source attribution.** Because that seam is
missing, every subcommand re-wires the same resolve → default → render tail, and
the *scalar* subcommands (`get`, `path`, `agent`, `work`) resolve **inline in
`inbound/cli.rs`** instead of delegating to `core/` the way the *block*
subcommands do. That asymmetry is the source of most duplication and the one
**genuine correctness bug** found (agent empty-value handling has drifted between
`config agents` and `config agent <name>`).

Priority ranking of what to refactor:

| #  | Finding                                                                                                           | Kind                          | Severity   |
|----|-------------------------------------------------------------------------------------------------------------------|-------------------------------|------------|
| 1  | Agent-default logic triplicated **and drifted** → observable behaviour divergence                                 | Duplication + correctness     | **High**   |
| 2  | No domain "effective value" op → resolve+default+render tail copied ~10× across core & inbound                    | Duplication + layering        | **High**   |
| 3  | Scalar subcommands resolve inline in `inbound/cli.rs`; block subcommands delegate to `core/`                      | Layering (inbound→core)       | **High**   |
| 4  | Domain defaults/validators hard-coded as literals in the launcher, duplicating the catalogue                      | Duplication + layering        | Medium     |
| 5  | Identifier validator `^[a-z0-9][a-z0-9-]*$` copy-pasted verbatim (2×)                                             | Duplication                   | Medium     |
| 6  | Output prose assembled in `core/` (review, summary, dump); computation (LCS diff) in `render/`                    | Layering (core↔render)        | Medium     |
| 7  | Source attribution + `Level`→filename mapping re-derived in launcher (domain gap)                                 | Layering (launcher→domain)    | Medium     |
| 8  | `extract_body`/`is_fence` re-roll `document::split`; `render_node`/`render_scalar` re-roll `config::render_value` | Duplication (document/config) | Low–Medium |
| 9  | Per-write umask re-read in config-adapters (corpus reads once); umask-mode formula duplicated                     | Duplication + latent race     | Low–Medium |
| 10 | `kernel::Error::Refusal` doc encodes a launcher-side exit-code fact                                               | Shared-crate hygiene          | Low        |
| 11 | `.tmp-` literal in `cache.rs` drifts from `store::TEMP_PREFIX`; duplicated render tests                           | Duplication (latent)          | Low        |

## Detailed Findings

### Concern 1 — Duplication that should be refactored out

#### 1.1 Agent-default logic is triplicated and has drifted (correctness bug) — **High**

The prefixed-default computation `format!("{}{name}", catalogue::AGENT_PREFIX)`
exists in three places, two of which bypass the domain:

- Domain: `cli/config/src/catalogue.rs:227-233` (`default_for` already returns the
  prefixed agent default).
- Launcher block path: `cli/launcher/src/config_command/core/agents.rs:57-59`
  (`default_agent`).
- Launcher scalar path: `cli/launcher/src/config_command/inbound/cli.rs:756`
  (`resolve_agent`).

They have **drifted on the resolved-but-empty case** (verified directly):

- `config agents` — `core/agents.rs:35-44`: a resolved value that renders empty
  falls back to `default_agent(name)`.
- `config agent <name>` — `inbound/cli.rs:754-757`: a resolved value that renders
  empty is kept **empty**; only `Resolved::Absent` falls back.

So with `agents.reviewer: ""` set explicitly, `config agents` renders
`accelerator:reviewer` while `config agent reviewer` renders nothing — the same
key reported two different ways. **Fix**: route both through
`catalogue::default_for("agents.{name}")` (or a domain "effective value" helper,
see 1.2) so there is one definition and one empty-handling rule.

#### 1.2 The resolve → default → render tail is copied ~10× (no domain "effective value") — **High**

The identical shape `Key::parse` → `config.get(key, None)` →
`Resolved::Found(v) => render_value(v)` / `Resolved::Absent => default` recurs at:

`core/dump.rs:76-86`, `core/init.rs:42-53`, `core/paths.rs:87-98`,
`core/review.rs:507-517`, `core/template.rs:99-111`, `core/summary.rs:89-95`,
and in the inbound adapter at `inbound/cli.rs:617-621` (`resolve_get`),
`:640-643` (`resolve_path`), `:753-757` (`resolve_agent`), `:771-774`
(`resolve_work`).

The catalogue-fallback variant ("resolve, else `catalogue::default_for`, else
empty") is copy-pasted at `core/init.rs:48-52`, `core/paths.rs:92-97`,
`core/dump.rs:105-109` and `:136-140`, and re-implemented again at
`inbound/cli.rs:733-734` (`path_fallback`).

Root cause: the domain `ConfigService::get` (`cli/config/src/service.rs:319-335`)
returns only the value, and `catalogue::default_for`
(`cli/config/src/catalogue.rs:220-235`) returns only the default; **nothing
combines them.** A single domain operation — e.g.
`ConfigAccess::effective(&key) -> Resolved`/`String` that applies precedence then
the catalogue default — would collapse all ~10 sites to one call and remove the
temptation to re-resolve inline.

#### 1.3 Identifier validator `^[a-z0-9][a-z0-9-]*$` copy-pasted verbatim — **Medium**

Byte-identical bodies differing only in the error string:

- `core/context.rs:82-96` (`validate_skill_name`)
- `core/template.rs:113-127` (`validate_name`)

Extract one `validate_identifier(kind, name)` (a `config_command` core helper, or
better a domain helper — see 3.7). Note the port docs already *assume* a validated
identifier (`cli/config/src/service.rs:63,72`), which argues for homing it in the
domain.

#### 1.4 `## <Name> Unavailable` notices use two inconsistent conventions — **Low–Medium**

Seven hand-written notice literals across two shapes: block renderers return an
owned `Rendered` **with** trailing `\n` (`render/agents.rs:32-34`,
`render/dump.rs:42-44`, `render/paths.rs:36-38`, `render/review.rs:84-86`,
`render/template.rs:59-61`); context/instructions use bare `&str` consts **without**
newline (`render/context.rs:11-12`, `render/instructions.rs:4`), with the newline
added later by `emit_sections` (`inbound/cli.rs:353`). The strings are each defined
once and referenced by name (no handler copy-paste), so this is a consistency
nit, not true duplication — but a single `unavailable(header)` helper would
normalise the newline convention.

#### 1.5 `Level` walk and `Level`→filename mapping re-coded — **Medium** (see also 3.7)

The `[Level::Team, Level::Personal]` iteration is re-coded at `core/agents.rs:65`,
`core/context.rs:27`, `core/summary.rs:136`, `inbound/cli.rs:688-692`, and in
Personal-then-Team order at `core/dump.rs:92-98`. The mapping
`Team → ".accelerator/config.md"` / `Personal → ".accelerator/config.local.md"` is
triplicated at `inbound/cli.rs:715-720` (`level_file`), `core/summary.rs:49,51`
(inline), and `render/dump.rs:35-36` (`source`). These filenames are already the
domain's concern (`FileConfigStore::level_path`,
`cli/config-adapters/src/store.rs`); a domain-exposed `Level::filename()` /
attribution API removes the launcher copies.

#### 1.6 Cross-crate duplication that is deliberate (do NOT refactor)

For completeness, the reviewers and the pup rules explicitly justify leaving these:

- `to_config_error` (`config-adapters/src/store.rs:703-720`) vs `to_store_error`
  (`corpus-adapters/src/store.rs:87-100`) are **not** mergeable: they translate
  into different taxonomies (`to_store_error` is a near-identity re-tag;
  `to_config_error` is a lossy projection since `ConfigError` lacks
  `NotWritable`/`CrossFilesystem`). The orphan rule + pup domain rules
  (`cli/pup.ron:42-72`) forbid a shared `From` impl anywhere. Keep as-is.
- The three structurally-identical value trees `document::Yaml` /
  `config::Node` / `corpus::FrontmatterValue` (`cli/document/src/value.rs:3-5`
  documents this) are the accepted cost of confining serde-saphyr to `document`.
- The allowlisted temp-rename exceptions `cache.rs` (0600 publish + paired
  signature) and `lock.rs` (directory rename-as-claim) are correctly **not**
  folded into `store`.

### Concern 2 — Logic in adapters/inbound that belongs in core (and vice-versa)

#### 2.1 Scalar subcommands resolve in the inbound adapter; block subcommands delegate to core — **High**

This is the structural asymmetry underlying much of Concern 1. The block
subcommands (`agents`, `dump`, `paths`, `review`, `template`, `context`,
`instructions`) assemble their view in `core/`. The scalar subcommands do the
same *kind* of work **inline in the driving adapter** `inbound/cli.rs`:

- `resolve_get` (`:610-626`), `resolve_path` (`:628-649`), `resolve_agent`
  (`:752-759`), `resolve_work` (`:766-785`) — key-parse + precedence + default.
- `path_fallback` (`:724-738`), `work_fallback` (`:789-799`) — catalogue defaults,
  duplicating `core/paths.rs:resolve_or_default` and `core/init.rs:resolve_path`.
- `legacy_alias_warning` (`:653-674`), `unknown_path_key_warning` (`:740-750`) —
  hard-code migration-0004 domain knowledge in the adapter.
- `explain_lines` (`:678-713`) — re-derives resolution provenance/precedence.
- `bad_integration` (`:805-814`) + the enum check at `:775-777` — `work.integration`
  validation in the adapter.

None of this is a clap concern. It is view-assembly that structurally matches the
`core/` modules. **Fix**: give the scalar subcommands `core/` view-assemblers
(`core/scalar.rs` or per-subcommand) and reduce `inbound/cli.rs` to arg-mapping +
fail-safe dispatch — the shape the block subcommands already follow. clap
containment is otherwise clean (no clap types reach `core/`; the mapping lives in
`launch/mod.rs:32-162` and `launch/inbound/cli.rs`).

#### 2.2 Output prose assembled inside `core/` that belongs in `render/` — **Medium**

The clean split (core → data view, render → bytes) is inverted in three places:

- `core/review.rs`: `verdict_lines`/`revise_verdict` (`:437-505`) emit literal
  output lines (e.g. `:456-462`), and `core_lenses_note` (`:153-183`) builds the
  `"Note: built-in work-item lens(es) …"` block; `render/review.rs:49-52,79` just
  pushes them verbatim.
- `core/summary.rs:46-85`: assembles the entire human-facing summary body;
  `render/summary.rs` only does the JSON envelope — the opposite of every other
  subcommand.
- `core/dump.rs:148-157` (`work_row`): builds `"{value} (invalid: must be …)"` —
  render-layer formatting in a core assembler. Plus presentation transforms
  `display: name.replace('-', " ")` at `core/agents.rs:48` and the note string at
  `core/paths.rs:62-66`.

#### 2.3 Algorithmic logic inside `render/` that belongs in core/domain — **Medium**

- `render/template.rs:163-211`: a full LCS unified-diff (`unified_diff`,
  `lcs_lengths`, `push_line`) plus content-equality in `diff_report` (`:114`) —
  computation, not formatting, living in the render layer.
- `render/review.rs:10-11`: hard-codes
  `DEFAULT_CORE = "architecture, code-quality, test-coverage, correctness"`,
  duplicating the domain default at `cli/config/src/catalogue.rs:132-139`.

#### 2.4 `ConfigError::Io` hard-codes "config file" but the new ports span non-config paths — **Medium** (plan-acknowledged, open)

`config/src/error.rs:84-86` renders `"I/O error on config file '{path}'"`, yet the
`ReadSkillContent`/`ReadLensCatalogue`/`ScaffoldProject`/`TemplateOverride` ports
now surface lens dirs, skill files, template overrides and `init`'s 14 directories
through it. The plan (§Phase 2 §2) called for kind-accurate variants
(`ProjectIo`/`ScaffoldFailed`) or a file-kind-neutral reword; the plan reviewers
flagged this Major and it is carried into implementation. This directly undercuts
divergence 10's mitigation ("the error must name the offending file"). Worth
closing since the 28 shell consumers run fail-loud with no `--fail-safe`.

### Concern 3 — Logic that should move into shared crates

#### 3.1 Domain "effective value" operation → `config` crate — **High**

The single highest-leverage move. Adding resolution+catalogue-default (and
optionally source attribution) to `ConfigAccess`/`ConfigService`
(`cli/config/src/service.rs`) collapses findings 1.1, 1.2, and 2.1's fallback
copies. The catalogue already owns the defaults (`catalogue::default_for`); only
the *combination* is missing.

#### 3.2 Domain defaults are hard-coded as literals in the launcher — **Medium**

`core/review.rs` reads defaults as string literals that the catalogue already
declares in `REVIEW_KEYS` (`catalogue.rs:126-152`): `min_lenses`/`max_lenses`
`"3"/"4"`/`"8"` (`review.rs:69-72`) — note the `"3"` even disagrees with catalogue
`review.min_lenses = 4` — plus severity `"critical"`, `plan_revise_major_count`
`"3"`, `work_item_revise_major_count "2"`, `max_inline_comments "10"`,
`dedup_proximity "3"` (`review.rs:198,205,466,473,583,632`). Also
`core/summary.rs:93` (`paths.tmp = ".accelerator/tmp"`, domain `catalogue.rs:46`)
and `core/template.rs:86` (`paths.templates`, domain `catalogue.rs:43`). Read these
via `catalogue::default_for` instead of re-typing them.

#### 3.3 `work.integration` validator → `config`/catalogue — **Medium**

`WORK_INTEGRATION_VALUES` is domain data (`catalogue.rs:106-107`) but has no
validator; the membership check is duplicated at `inbound/cli.rs:776-777` (refusal)
and `core/dump.rs:148-149` (annotation). Add a domain `is_valid_work_integration`
(or a generic enum-key validator) beside the data.

#### 3.4 `document` crate: `extract_body`/`is_fence` re-roll `document::split` — **Low–Medium**

`config-adapters/src/store.rs:663-690` hand-rolls fence-based body extraction that
`document::split` (`cli/document/src/fence.rs:90-109`) already computes, and the
store already depends on `document` (calls `parse`/`render` at `store.rs:163,184`).
**But** the divergences are deliberate bash-parity (comment at `store.rs:660-662`):
`extract_body` accepts `"--- "` as a fence, returns empty (not error) on an
unterminated fence, and re-joins with `\n`. Consolidatable only if that bash-mirror
contract is later relaxed (0174 territory). Record, don't rush.

#### 3.5 `config`-domain: `render_node`/`render_scalar` re-roll `config::render_value` — **Low–Medium**

`config-adapters/src/store.rs:618-644` (used only for lens-field extraction)
duplicates the scalar/sequence stringification of `config::render_value`
(`cli/config/src/render.rs:10-29`) — the scalar arm is byte-identical. Both are in
the config subdomain, so this is consolidatable now (project the `Node` to a
`Value` and call `render_value`) without crossing a pup boundary.

#### 3.6 `store` crate: read-half of the containment contract, and umask formula — **Low–Medium**

- `store` owns the write half (`atomic_write`) of the containment contract but the
  read half is re-derived per adapter: `config-adapters/src/store.rs:648-658`
  (`read_within`) is re-implemented inline in `corpus-adapters/src/store.rs:124-128`
  and `:145-151`. A `store::read_within` companion (its own doc at
  `store/src/lib.rs:100-102` already anticipates this) removes the copy.
- The umask-mode formula `0o666 & !store::current_umask()` is duplicated at
  `corpus-adapters/src/store.rs:44` and `config-adapters/src/store.rs:193`. A
  `store` helper (e.g. `NewFileMode::preserve_or_umask()`) removes it. **Behavioural
  note**: corpus reads the umask **once at construction** (documenting the race it
  avoids, `:21-23`) while config **re-reads per write** (`:193`) — the exact race
  the corpus doc warns against. Worth aligning on read-once regardless of the
  helper extraction.

#### 3.7 Identifier validation → `config` domain — **Medium**

Beyond de-duplicating 1.3, the natural home is the domain: `ReadContent`'s port
docs already promise a "validated identifier" (`service.rs:63,72`), and `Key::parse`
has its own (different) validation (`key.rs:19-27`). A domain
`validate_identifier` keeps the contract next to the ports that assume it.

#### 3.8 `kernel` crate: `Refusal` doc encodes a launcher-side fact — **Low**

`kernel::Error::Refusal` (`cli/kernel/src/lib.rs:16-18`) is the newly-added variant
(exit 2, wired at `main.rs:207-210`). Its doc comment states the exit-code mapping,
which actually lives in `main.rs`; the plan reviewers flagged this as the
staleness class the project bans, and separately noted the exit-2 convention needs
to converge across the 0168/0169/0173 sub-binaries. Keep the variant in `kernel`
(cross-cutting is correct) but strip the exit-code assertion from its doc. The
broader filesystem-error taxonomy (`UnsafePath`/`NotWritable`/`CrossFilesystem`/`Io`
repeated in `ConfigError`/`StoreError`/`WriteError`) is a *possible* kernel
centralisation but is deliberately kept per-domain by the pup rules — leave it.

#### 3.9 Latent drift: `cache.rs` `.tmp-` literal vs `store::TEMP_PREFIX` — **Low**

`cache.rs:94-95` re-derives the temp prefix as the literal `".tmp-"` rather than
reusing `store::TEMP_PREFIX` (`store/src/lib.rs:22`). `cache.rs` is correctly
outside `store` (its semantics diverge), but importing the one shared constant
removes a silent drift risk. Also harmless: `renders_scalar_kinds` /
`renders_a_sequence_in_bracketed_form` tests are duplicated verbatim between
`config/src/render.rs:38-59` and `config-adapters/src/render.rs:30-51`.

## Code References

- `cli/launcher/src/config_command/inbound/cli.rs` — fail-safe dispatch (`finish`
  `:428-447`, `Degrade` `:421-426`, `From<ConfigError>` `:412-419`, `section`
  `:325-339`); **inline scalar resolution** `:610-799` (the layering issue 2.1).
- `cli/launcher/src/config_command/core/agents.rs:35-59` — drifted agent default (1.1).
- `cli/launcher/src/config_command/core/{dump,init,paths,review,template,summary}.rs`
  — repeated resolve+default tail (1.2), prose-in-core (2.2).
- `cli/launcher/src/config_command/core/context.rs:82-96`,
  `core/template.rs:113-127` — duplicated identifier validator (1.3).
- `cli/launcher/src/config_command/render/mod.rs:18-41` — `Rendered` + `emit`
  buffer-then-emit (implemented once, correct).
- `cli/launcher/src/config_command/render/template.rs:163-211` — LCS diff in render (2.3).
- `cli/config/src/service.rs:319-335`, `catalogue.rs:220-235` — the missing
  "effective value" seam (3.1).
- `cli/config/src/catalogue.rs:126-152` — review defaults re-typed in launcher (3.2).
- `cli/config/src/error.rs:84-86` — "config file" wording over non-config ports (2.4).
- `cli/store/src/lib.rs` — `atomic_write`, `ensure_contained`, `current_umask`,
  `NewFileMode`, `WriteBounds` (clean primitive).
- `cli/config-adapters/src/store.rs:663-690` (`extract_body`), `:618-644`
  (`render_node`), `:193` (per-write umask), `:703-720` (`to_config_error`).
- `cli/corpus-adapters/src/store.rs:44` (umask once), `:87-100` (`to_store_error`),
  `:124-151` (re-rolled `read_within`).
- `cli/kernel/src/lib.rs:16-18` — `Refusal` variant + doc (3.8).
- `cli/launcher/src/launch/outbound/resolve/cache.rs:94-95` — `.tmp-` drift (3.9).
- `cli/pup.ron:42-72` — domain-boundary rules that make several cross-crate
  "duplications" deliberate and unmergeable.

## Architecture Insights

- **The root cause is a single missing domain operation.** Findings 1.1, 1.2,
  2.1, 3.1, 3.2, 3.3 are all facets of the same gap: `config` exposes value
  resolution and catalogue defaults separately, so every consumer re-composes them
  — and once you are re-composing inline, doing it in the inbound adapter (scalars)
  vs core (blocks) becomes an arbitrary choice that drifted. Close the domain seam
  first; most launcher duplication and the correctness bug fall out with it.
- **The hexagon is real but only half-applied.** Block subcommands honour the
  core/render/inbound split the plan prescribed (Phase 2 §2 "The hexagon"); scalar
  subcommands short-circuit it. Symmetry between the two families is the cheap win.
- **The cross-crate boundaries are correct and load-bearing.** The `store`
  extraction, the twin private translators, and the three separate value trees are
  all forced by `pup.ron` + the orphan rule and were endorsed by the plan reviews.
  Do not "de-duplicate" across `store`/`config`/`corpus` error types or value
  trees — that is the boundary working as designed.
- **`core`↔`render` purity is worth restoring** (2.2/2.3): the `Rendered` seam
  exists precisely to keep render pure and core presentation-free; review/summary/
  dump currently blur it in both directions.

## Historical Context

- `meta/plans/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
  — the implemented plan. Phase 1 §1-2 (store crate + translators), Phase 2 §2
  ("The hexagon": inbound/core/render, `Rendered { stdout, warnings }`,
  `render_unavailable()`, driven ports, view-assembly-in-core), Phase 2 §3 (exit
  codes / `kernel::Error::Refusal`).
- `meta/reviews/plans/2026-07-19-0167-…-review-1.md` and `-review-2.md` — plan
  reviews (REVISE→APPROVE at pass 3). Several Concern-2/3 items here were
  **deliberately deferred to implementation** by the pass-3 approval ("the compiler
  and test gates will surface remaining consistency more reliably than a fourth
  review round"): domain-rules-in-inbound (2.1), view-assembly homing (2.2),
  `ConfigError::Io` wording (2.4), and cross-sub-binary exit-code convergence (3.8).
  So these findings are expected residue, not regressions.
- `meta/reviews/work/0167-…-review-1.md` — work-item review; established the
  `atomic_write`-relocation-to-shared-crate decision (3.x) and flagged the
  unnamed-crate/ownership gap now closed by the `store` crate.
- No `0166` review exists — 0166 decomposed into 0178/0179/0180, each with its own
  review under `meta/reviews/work/`.

## Related Research

- `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
  — the pre-implementation research that fed the plan.

## Open Questions

1. **Effective-value API shape**: should the domain expose
   `effective(&key) -> String` (value-or-default) or a richer
   `Resolution { value, level, from_catalogue }` that also serves `--explain` and
   `dump`'s source attribution? The latter closes 3.1 *and* 1.5/3.7's attribution
   copies in one move.
2. **Identifier validation home**: domain helper vs `config_command` core helper —
   the port docs argue for domain, but `Key::parse` already has its own validation;
   is one unified validator wanted, or two intentionally-different ones?
3. **`extract_body` bash-parity (3.4)**: is the bash-mirror contract still required
   post-cutover, or can it move to `document::split` once 0174 retires
   `config-common.sh`?
4. **Umask read-once vs per-write (3.6)**: align config-adapters onto corpus's
   read-at-construction to close the documented race, independent of the helper
   extraction?
