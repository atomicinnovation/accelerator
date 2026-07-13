---
type: pr-description
id: "21"
title: "[0179] Add corpus crates and supporting crates"
date: "2026-07-13T21:54:19+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "0179"
parent: "work-item:0179"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/21"
pr_number: 21
tags: [rust, corpus, document, vcs, crates, frontmatter, serde-saphyr, doc-type, typed-linkage, parity]
revision: "5339bfff226dc2b46dec31c7c2a5287638ed9540"
repository: "accelerator"
last_updated: "2026-07-13T21:54:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0179] Add corpus crates and supporting crates

## Summary

Builds the parsing- and convention-layer crates for the meta corpus as a
consolidating rewrite onto the hexagonal pattern 0178 established: five new
crates in the `cli/` workspace — `document`, `corpus`, `corpus-adapters`, `vcs`,
`vcs-adapters` — plus a retrofit of the shipped `config-adapters` onto the new
shared `document` crate, so YAML is parsed in exactly one place.

This is the **library layer only**; no CLI surface ships here. It unblocks 0180
(atomic-store), 0170 (`accelerator-work`), 0173 (`accelerator-corpus`), and 0168
(folding the visualiser into `cli/`).

Porting the conventions off bash also surfaced four real bugs in the scripts that
were meant to be the parity *oracle* — including a latent awk bug that had been
corrupting every ADR path reference inside a linkage value into malformed YAML.
Those are fixed here too.

## Changes

### New crates

- **`document`** — the markdown-with-frontmatter *protocol* crate. Fence
  splitting in both forms the two consumers need (a byte-offset form with a 1 MiB
  scan cap for in-place edits, and an owned-halves form for round-trip render),
  serde-saphyr parsing, and rendering that preserves the body byte-for-byte.
  serde-saphyr is now confined to this crate.
- **`corpus`** — the domain crate (`kernel`-only; no serde, YAML, regex, or
  filesystem in its closure). A serde-free frontmatter value model, `DocTypeKey`
  and the doc-type inference matcher, the typed-linkage (ADR-0034) parser and
  single-document resolver, the slug conventions, the work-item-ID runtime
  predicate, a `Clock` port, and the artifact-metadata contract. The pure
  convention algorithms take infra-sourced data (a compiled scanner, a doc-type
  table, a parsed value) by injection.
- **`corpus-adapters`** — the outbound infra and imperative shell: the
  `document` → domain translation, the regex-backed `IdScanner`, the
  config-sourced doc-type table, the per-document assembler, the frontmatter
  write-convention (`patch_status`, preserving quote style, inline comments, and
  CRLF), and artifact-metadata derivation.
- **`vcs` / `vcs-adapters`** — a dedicated domain+adapters pair for the
  cross-cutting repo probe (repo-root, VCS-kind, revision, repo-name). Repo-root
  uses an ancestor marker-walk; revision uses a subprocess probe with a scrubbed
  environment, disabled colour, and a time cap — every failure mode resolves to
  `None` and is warn-logged rather than silently reading as a revision-less repo.

### `config-adapters` retrofit

- Obtains all frontmatter split/parse/render through `document`; its own
  `frontmatter.rs` is deleted and it no longer names serde-saphyr. Every
  `Yaml ↔ Node ↔ FrontmatterValue` mapping arm is explicit — no `_` wildcard —
  so a new scalar variant cannot be silently absorbed.

### Consolidation

- The dir→type fact is single-sourced on `DocTypeKey`: the runtime table, the
  0007 migration snapshot, and `0007-frontmatter-rewrite.awk` are now pinned
  against the crate by a `doc_type_single_source` suite. The four doc-type
  vocabularies (config key, linkage name, wire token, human label) become
  `const fn`s on `DocTypeKey` — they stay four (they are three external contracts
  plus a display string), but they can no longer drift.
- The two `{number:0Nd}` width parsers collapse into one; the title-casers that
  share semantics collapse into `corpus::slug` helpers, exposed `pub` so 0168 can
  retire the server-side copies.

### Bash-side fixes found while porting

The port read the bash surfaces closely enough to find four bugs, all with a
green test suite over them (each lived in an arm no fixture exercised). Recorded
in `meta/notes/2026-07-13-bash-corpus-script-inconsistencies.md`:

- **`normalize_paths` corrupted ADR path references** — awk's `RSTART`/`RLENGTH`
  are globals, and `path_to_typed` runs `match()` itself, so the splice read the
  inner match's offsets: `["meta/decisions/ADR-0026-old.md"]` became
  `["adr:ADR-0026"ecisions/ADR-0026-old.md"]`. Pre-existing, and malformed YAML.
- **Linkage was not config-aware** — `lp_type_from_path` carried a hardcoded,
  hand-ordered `case` over path globs. A repo that re-pathed its corpus got
  correct validation and migration but silently stopped resolving links. It now
  sources the shared `doc-type-inference.sh` matcher (longest-dir-wins), which
  also makes the ordering hack unnecessary.
- **Design-inventory references all collapsed to one dangling id** — the awk had
  no nested-manifest arm, so every reference to `<dir>/<slug>/inventory.md`
  resolved to `design-inventory:inventory`, an id no document has.
- **`meta/prs` was unresolvable**, and its id was derived from the filename stem
  (`12-description`) rather than the PR number (`12`) — so promoting it to a
  legal linkage target would have pointed at nothing. The stem and the identity
  are now separate concepts, and `pr-description` is a resolvable target type.

### Enforcement

- The serde-saphyr cargo-deny wrapper ban re-homes from `config-adapters` to
  `document` (with its regression test); cargo-pup gains kernel-only import rules
  for the `corpus` and `vcs` domains.
- A new `bash-parity` cargo feature gates the suites that shell out to bash, awk,
  jj, and git, so `cargo test` stays runnable on a bare machine. CI enables it via
  `--all-features`, where an absent tool hard-fails rather than skipping: **311
  tests with the feature on, 295 with it off, nothing skipped either way.**

## Context

- Work item: `meta/work/0179-corpus-crates-parsing-conventions.md` (child of
  0166 — Shared config, corpus, and store crates)
- Plan: `meta/plans/2026-07-11-0179-corpus-crates-parsing-conventions.md`
- Research: `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`
- Validation: `meta/validations/2026-07-11-0179-corpus-crates-parsing-conventions-validation.md`
  (result: pass)
- Bash inconsistencies note: `meta/notes/2026-07-13-bash-corpus-script-inconsistencies.md`
- Conventions: ADR-0034 (typed-linkage), ADR-0045 (bash/Rust duplication)

## Testing

- [x] `mise run check` — exit 0 (full read-only CI mirror, all four toolchains)
- [x] `mise run test:unit:cli` — **311 passed, 0 skipped** (CI path,
      `--all-features`, so the differential suites run)
- [x] `mise run deny:check` and `mise run pup:check` — exit 0 (domain purity and
      the serde-saphyr wrapper ban both enforced, not just asserted)
- [x] Bash oracle suites all green — `test-work-item-pattern.sh`,
      `test-linkage-parser.sh`, `test-metadata-helpers.sh`, `test-migrate-0007.sh`
- [x] Adversarial frontmatter fixtures parse to a clean malformed/error result
      under a bounded-time guard, with no `catch_unwind` sandbox — serde-saphyr
      being pure Rust retires the libyml panic guard the visualiser needed
- [x] Parity against the bash oracle over a fixture corpus spanning the 14
      `DocTypeKey` variants and all three identity schemes. The doc-type parity
      test injects a table with a prefix-pair and a tie (the repo's own config has
      neither), and a vacuity guard pins the oracle's exact output so the diff
      cannot pass by both sides resolving nothing
- [ ] **Manual**: review the four beyond-plan bash commits against the 0007
      migration's behaviour on a real corpus before this ships
- [ ] **Manual**: confirm the config read path failing closed on YAML tags is
      intended (no config in the repo uses tags, so nothing changes today)

## Notes for Reviewers

**Where to focus.** Two things widened beyond a straight port and deserve a
second opinion:

1. **The parity oracle was changed during the port.** Four commits (`8abc3609`,
   `35d78cd5`, `869a81ed`, `3cf0b568`) modify the bash side that the plan
   positioned as the *oracle*. They fix real bugs and the bash suites stay green,
   but changing the thing you are being measured against is worth conscious
   acknowledgement rather than a nod. The reasoning for each is in the
   inconsistencies note.
2. **The YAML-tag rule is a structural boundary, not a serde one.** The plan
   assumed a `YamlVisitor`-level guard; that is impossible — serde-saphyr resolves
   a tag against its schema *before* the serde boundary, so the tag is gone by the
   time any `Visitor` method runs. It instead scans serde-saphyr's re-exported
   parser event stream (`cli/document/src/tags.rs`) and rejects the first tagged
   node. This keeps `document` the sole serde-saphyr wrapper and stays bounded on
   an alias bomb (aliases are `Alias` events, not expansions).

**Accepted deviations** (documented in the plan and validation report, not open
work):

- `SystemClock::try_new` shells out to `date +%z` for the host offset. Phase 5
  justified the `time` dependency as "no shell-out to `date`" — true of the
  *rendering*, not of acquiring the offset. It sidesteps `time`'s multithread
  refusal; the alternative is a self-re-exec, which trades a POSIX utility for a
  hidden subcommand.
- The assembler takes `raw: &[u8]`, not a path — the file read lands with the CLI
  surface in 0173, keeping the assembler pure.
- The controlled-`TZ` assertion is realised as an injected offset rather than
  mutating process env from a multithreaded test.

**Carried forward to 0168** (not blocking):

- The fold must add a conformance test binding the server-side twins
  (`config::label_from_key`, `api::library::humanise_status`,
  `indexer::number_width_from_id_pattern`) to the canonical `corpus` copies as it
  retires them. Nothing binds them today, so they can silently diverge in the
  window.
- The SPA/API JSON `Serialize` boundary is still deferred: the order-preserving
  `Vec<(String, _)>` model and the big-int-as-`String` policy diverge from the
  shipped `BTreeMap`/numeric shape, so 0168 must either preserve the old shape or
  accept the change deliberately.
- `config-adapters::discover_root` remains a second marker-walk distinct from
  `vcs::MarkerWalkRoot` (it also stops at `.accelerator` and falls back to
  `start`). 0168 should fold it onto a parameterised walk or record it as a
  permanent fork.

The visualiser server is deliberately **untouched** — 0179 extracts, 0168 folds.
