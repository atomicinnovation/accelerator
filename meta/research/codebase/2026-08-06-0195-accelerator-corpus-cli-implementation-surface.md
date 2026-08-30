---
type: "codebase-research"
id: "2026-08-06-0195-accelerator-corpus-cli-implementation-surface"
title: "Research: Implementation surface for work-item 0195 (accelerator-corpus CLI)"
date: "2026-08-06T08:16:04+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0195"
parent: "work-item:0195"
topic: "Implementation surface for accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI"
tags: ["research", "codebase", "rust", "cli", "corpus", "adr", "frontmatter", "linkage", "sub-binary"]
revision: "451c185a684756747b850c5530088878fc02949a"
repository: "accelerator"
last_updated: "2026-08-06T08:16:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Implementation surface for work-item 0195 (accelerator-corpus CLI)

**Date**: 2026-08-06T08:16:04+00:00
**Author**: Toby Clemson
**Git Commit**: 451c185a684756747b850c5530088878fc02949a (working copy, empty, sits directly on `main` @ 6e7a84e079f9518754043b9187d972f24e83f413)
**Branch**: main
**Repository**: accelerator

## Research Question

What is the full implementation surface for [work-item 0195](../work/0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md) — building `accelerator-corpus`, a thin CLI over the shared `corpus`/`corpus-adapters` crates covering ADR numbering/status, artifact-metadata derivation, corpus-frontmatter validation, and typed-linkage extraction — and rewriting every caller of the four bash scripts it replaces?

## Summary

0195 replaces five bash entry points (`adr-next-number.sh`, `adr-read-status.sh`, `artifact-derive-metadata.sh`, `validate-corpus-frontmatter.sh`, `linkage-parser.sh`) with one new sub-binary, `accelerator-corpus`, dispatched as `accelerator corpus <noun> <verb>`. The research below establishes four things:

1. **The bash behaviour to reproduce byte-for-byte** (AC1 requires characterization tests against this as golden baseline) — fully characterized for all five scripts below, including several non-obvious edge cases (e.g. `adr-next-number.sh`'s `%04d` is a *minimum* width so a 5-digit overflow prints untruncated; `adr-read-status.sh` silently accepts an empty status value as success; `validate-corpus-frontmatter.sh`'s typed-linkage shape check only recognises double-quoted values, so a well-formed single-quoted reference is unconditionally `BAD-LINKAGE-SHAPE`; `linkage-parser.sh` hardcodes a fixed 4-digit bare-number scan regardless of the corpus's configured `work.id_pattern`).
2. **What's already built in Rust and directly reusable** — the `corpus`/`corpus-adapters` crates (work-item 0179, done) already ship a near-complete `linkage extract` domain implementation (`corpus::linkage::parse_document`, differentially parity-tested against `linkage-parser.sh`), a near-complete `metadata derive` implementation (`corpus_adapters::metadata::derive_at`/`render`, differentially parity-tested against `artifact-derive-metadata.sh`), the work-item-id scan-regex compiler (`corpus_adapters::work_item_pattern::compile_scan_regex`) and its `RegexScanner` port adapter, doc-type inference, slug derivation, and a generic (untyped) frontmatter parser.
3. **What's genuinely new work** — reading *named* frontmatter fields (artifact-type discriminator, `schema_version`, the typed-linkage keys) out of the generic value tree; ADR-number allocation and ADR-status reading (nothing exists for either); the `config`-crate wiring to source `work.id_pattern`/doc-type paths into `corpus`/`corpus-adapters`'s injected types (today only the visualiser server does this); and the `accelerator-corpus` binary crate itself.
4. **The registration mechanics and precedent** — `accelerator-vcs` (`cli/vcs-cli/`) is a directly analogous, already-shipped sub-binary (`vcs`/`vcs-adapters` domain/adapter split, dispatched the same way) to model the new crate on. The thirteen-point sub-binary checklist in `tasks/README.md` applies in full, with one naming collision to resolve up front: `cli/corpus/` is already the **domain** crate, so the new binary crate needs its own directory (e.g. `cli/corpus-cli/`, mirroring `vcs`→`vcs-cli`) plus an explicit `_SUBBINARY_MANIFESTS` entry in `tasks/manifest.py`.

0195 itself is `status: ready`, already reviewed once (verdict APPROVE after a re-review pass resolved nearly every finding), and its stated dependencies (0166, 0179, 0167, 0187) are all done — nothing blocks starting implementation.

## Detailed Findings

### The five source bash scripts — behavioural contract to reproduce

#### `adr-next-number.sh` (`skills/decisions/scripts/adr-next-number.sh`)

Scans the configured decisions directory for the highest `ADR-NNNN*` file and prints the next `--count N` (default 1) sequential numbers, zero-padded to a *minimum* of 4 digits.

- Invocation: `adr-next-number.sh [--count N]` — any other flag/arg shape is a usage error (exit 1). `--count` must match `^[1-9][0-9]*$` (exit 1 otherwise, with the invalid value echoed).
- Resolves the repo root by walking up from `$PWD` for `.jj`/`.git` (`find_repo_root` in `scripts/vcs-common.sh:8-18`), falling back to `$PWD` if none found.
- Reads the decisions directory via `accelerator config path decisions` (through `$ACCELERATOR_BIN` or `$PLUGIN_ROOT/bin/accelerator`).
- If the decisions directory doesn't exist: warns on stderr, prints `0001..000N` on stdout, **exits 0** (a success path, not an error).
- Otherwise: globs `ADR-[0-9][0-9][0-9][0-9]*`, extracts each match's full leading digit run (not clamped to 4 digits — `ADR-10000-x.md` yields `10000`), tracks the max via base-10 arithmetic (`10#$NUM`, avoiding octal misparse of leading zeros), and prints `HIGHEST+1 .. HIGHEST+COUNT` with `%04d` (minimum width — verified by the test suite's `ADR-9999-overflow.md` → `10000` case).
- No stdout on any failure path; failures write a `Usage:`/`Error:` line to stderr.

#### `adr-read-status.sh` (`skills/decisions/scripts/adr-read-status.sh`)

Self-contained (no sourced libraries, no VCS/config dependency). Reads `status:` out of an ADR file's frontmatter.

- Invocation: `adr-read-status.sh <adr-file-path>` — no args or missing/non-regular file → exit 1 with a usage/`File not found` message.
- Line-by-line scan: the first bare `---` line opens frontmatter, the second closes it and **breaks the loop immediately** (content after the closing fence, including a body `status:` line, is never seen — confirmed by the test suite's line-after-fence case).
- Inside the fence, every line matching `^status:` overwrites the captured value (last match inside the fence wins); the value is unwrapped of one layer of surrounding `"`/`'` quotes and trailing whitespace is stripped.
- Success requires **both** a closed fence and at least one `status:` line inside it; an **empty** status value (`status: ` with nothing after) is still success — the value's presence, not its content, is validated. The status vocabulary (`proposed|accepted|rejected|superseded|deprecated`) appears only in the failure message text, never enforced.
- Failure (no fence, unclosed fence, or no `status:` key found) always emits the same two-line stderr message and exits 1.

#### `scripts/artifact-derive-metadata.sh`

No positional args, flags, or env vars read; no author resolution (that's agent-level prose in the calling skills, not in this script or anywhere in the Rust crates either).

- Prints, in fixed order: `Current Date/Time (UTC): <ISO-8601 UTC>` (always), `Current Revision: <rev>` (only if non-empty), `Repository Name: <name>` (only if non-empty), `Timestamp For Filename: <YYYY-MM-DD_HH-MM-SS, local time>` (always).
- VCS detection: **jj is checked first** (`command -v jj && jj root`) and wins even in a colocated repo; only falls through to git (`git rev-parse --is-inside-work-tree`) if the jj probe fails. Neither present → all VCS fields empty, no error.
- jj branch resolves a **secondary-workspace** indirection: if `$REPO_ROOT/.jj/repo` is a file (marker of `jj workspace add`), it re-derives the primary workspace's root via that file's target, swallowing any failure with `|| true`.
- git branch has a real hazard: `git rev-parse HEAD` on an **unborn/zero-commit repo** fails and (under `set -e`) aborts the whole script non-zero — no graceful fallback exists for this specific case (the test harness works around it with `git commit --allow-empty`).
- Output-contract note directly from 0195's own Technical Notes: this script's output is invoked by *many* skills and must be preserved as-is — no field additions/removals/reorderings.

#### `scripts/validate-corpus-frontmatter.sh`

Two modes: `<dir>` (whole-corpus, structural + referential-integrity checks) vs one-or-more `<file>...` (file-list, structural only, no doc-type filtering, `DANGLING-REF` never fires). All findings go to stderr, one per violation, format `<file>: <CODE> — <message>` (em dash, not hyphen — byte-significant). Exit 0 (clean), 1 (violations, or a schema/allowlist-resolution failure), 2 (usage error).

- Validates against a 13-row schema table (`scripts/templates-schema.tsv`) keyed by `type:` — `work-item`, `plan`, `plan-validation`, `pr-description`, `adr`, `codebase-research`, `issue-research`, `design-inventory`, `design-gap`, `plan-review`, `work-item-review`, `pr-review`, `note` — each row defining: code-state-anchored (whether `revision`/`repository` provenance is required vs forbidden), required "extras," the status vocabulary, a forbidden own-id key (e.g. `work_item_id` on `work-item`), and which keys carry typed-linkage values.
- 16 distinct violation codes, the most subtle being: `UNQUOTED-ID` and `EMPTY-PLACEHOLDER` both have a gap where a genuinely empty `id:` value (bare `id:` with nothing after) triggers neither; `BAD-LINKAGE-SHAPE`'s quote handling is **double-quote only** — a well-formed single-quoted reference (`'work-item:0042'`) is unconditionally flagged, an asymmetry versus the rest of the script's `fm_inner` helper which accepts both quote styles; an **unclosed** frontmatter fence has no dedicated error code — the extractor's awk state machine just treats the rest of the file as frontmatter, which can produce spurious matches from body prose.
- Referential integrity (`DANGLING-REF`) is whole-corpus-mode only, built from a single `type:id` index over all in-scope fenced files; `pr:` refs are the sole prefix exempted from this check (still subject to shape-checking).

#### `scripts/linkage-parser.sh`

Extracts typed-linkage references from prose inside five `## `-level sections (`References`, `Dependencies`, `Historical Context`, `Related Research`, `Source References`) — distinct from frontmatter parsing. Companion data: `scripts/linkage-type-pairs.tsv` (16 `(source_type, key, target_type)` rows), mirrored exactly in the Rust port's `TYPE_PAIRS`.

- Candidate tokens per qualifying line, in priority order: `meta/...\.md` paths (matched against a regex built from the configured doc-type directories), `ADR-[0-9]{3,4}` ids, `pr:[0-9]+` refs, and — **only inside `## Dependencies`, only on a line carrying a recognised relationship label** (`blocks?|blocked[ -]by|related|depends? on|sibling|parent:`) — bare 4-digit numbers.
- **Key finding, flagged as a genuine behavioural gap to preserve, not fix**: the bare-number scan is **hardcoded to exactly 4 digits** (`grep -oE '[0-9]{4}'`) — it does not read `work.id_pattern`/`work.default_project_code` at all (unlike a path-derived work-item id, which uses an unbounded leading-digit-run). If the corpus's `work.id_pattern` were customised to a project-prefixed pattern, this bash script — and therefore any Rust characterization test built against it — would not track that customisation for prose-derived bare-number refs.
- Key inference (`lp_infer_key`) checks explicit keyword hints first (sibling → `relates_to`, supersedes(s) → `supersedes`, blocked[- ]by → `blocked_by`, blocks? → `blocks`, a line-leading `Source:` label → `parent`/`derived_from`/`source` depending on the resolved target type), all boundary-anchored so hyphen/underscore compounds (`code-block`) don't false-match, then falls back to a per-section default key.
- Band classification (`resolved` vs `ambiguous`, per ADR-0038's two-band model) requires an explicit keyword hit *and* an unambiguous target type inferable from the `(source_type, key)` pair table; anything else is `ambiguous`.
- Output: one TSV record per token, `source_type\tkey\ttarget_ref\tbody:<section-slug>#<seq>\tband`, `<seq>` a document-wide (not per-section) monotonic counter.
- Fails closed at **module load time** (before any CLI arg parsing) if the doc-type table can't be resolved — relevant to characterization-test harness design, since even sourcing the script can abort the process.

### Call sites — every executable invocation to rewrite (AC2)

No CI workflow and no `tasks/` Python task shells out to these scripts directly. All real call sites are in `SKILL.md` files (`!`-preprocessor invocations) plus test harnesses.

- **`adr-next-number.sh` / `adr-read-status.sh`**: `skills/decisions/create-adr/SKILL.md` (both scripts, allowed-tools wildcard `Bash(${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/*)`), `skills/decisions/extract-adrs/SKILL.md` (next-number only), `skills/decisions/review-adr/SKILL.md` (read-status only).
- **`artifact-derive-metadata.sh`**: allowed-tools grant `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)` present in `create-adr`, `extract-adrs`, `research/conduct-spike`, `research/research-issue`, `research/research-codebase`, `notes/create-note` — plus **prose-only invocations with no matching grant** (a pre-existing gap, not introduced by 0195) in `work/create-work-item`, `work/review-work-item`, `work/extract-work-items`, `github/describe-pr`, `github/review-pr`, `planning/create-plan`, `planning/review-plan`, `planning/validate-plan`. Worth flagging during implementation since AC2's "matching `allowed-tools`" requirement gives a natural point to close this gap for the new `accelerator corpus metadata derive` invocation.
- **`validate-corpus-frontmatter.sh`** and **`linkage-parser.sh`**: no `SKILL.md` invokes either directly (they're corpus-integrity/CI-time tools) — but both are invoked from `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh` (a **non-skill** shell caller, in scope per AC2's "skills, CI workflows, `tasks/` build tasks, and other scripts alike").

### Existing test coverage — golden-baseline suites for AC1

| Script | Test file | Case count |
|---|---|---|
| `adr-next-number.sh` + `adr-read-status.sh` | `skills/decisions/scripts/test-adr-scripts.sh` | 21 (10 + 11) |
| `artifact-derive-metadata.sh` | `scripts/test-metadata-helpers.sh` (shared across 3 sibling metadata helpers) | 12 attributable (2 scenarios × 6 assertions) |
| `validate-corpus-frontmatter.sh` | `scripts/test-validate-corpus-frontmatter.sh` | 49 |
| `linkage-parser.sh` | `scripts/test-linkage-parser.sh` (sources the script directly) | 33 |

`validate-corpus-frontmatter.sh`'s suite is additionally **required by name** in CI (`tasks/test/integration.py`'s `_REQUIRED_CONFIG_SUITES`), not just counted toward a floor — a fail-closed gate. All four use the shared `scripts/test-helpers.sh` assertion library (`assert_eq`, `assert_exit_code`, `assert_matches_regex`, etc.), which is also the natural model for the Rust characterization tests' assertion shape.

Rust-side differential parity tests **already exist** for two of the five and are directly reusable as a template: `cli/corpus-adapters/tests/metadata.rs` (vs `artifact-derive-metadata.sh`) and `cli/corpus-adapters/tests/parity.rs` (vs `linkage-parser.sh`), both gated behind the `bash-parity` cargo feature and hard-failing (not skipping) when the bash oracle is absent.

### What's already built in `corpus`/`corpus-adapters` (work-item 0179)

`corpus` (`cli/corpus/src/`) is a kernel-only domain crate (no serde/regex/filesystem deps — only `kernel`); `corpus-adapters` is the infra shell (regex, `document`-crate-backed YAML, `vcs`/`vcs-adapters`, filesystem/locking). Neither depends on the `config` crate — both take config-shaped values by injection.

**Directly reusable, no new domain logic needed:**
- `corpus::linkage::parse_document` (`cli/corpus/src/linkage.rs`) — full port of `linkage-parser.sh`'s section-scan/keyword-inference/band-classification pipeline, differentially parity-tested over an 8-fixture corpus.
- `corpus_adapters::metadata::derive_at` + `render` (`cli/corpus-adapters/src/metadata.rs:196-227`) — composes a `SystemClock` + `vcs_adapters::facts` into the same labelled block `artifact-derive-metadata.sh` prints, parity-tested.
- `corpus_adapters::work_item_pattern::compile_scan_regex` (`cli/corpus-adapters/src/work_item_pattern.rs:125-229`) + `corpus_adapters::scanner::RegexScanner` (`cli/corpus-adapters/src/scanner.rs:10-40`) — the `work.id_pattern` DSL → ERE compiler and its `IdScanner` port adapter, parity-tested against `_wip_compile` in `skills/work/scripts/work-item-common.sh`. **This resolves 0195's own Dependencies note** ("the compiled work-item-id scan regex ... has no runtime dependency on work-item:0170 ... `corpus-adapters` already ships the compiler") — confirmed present, not merely test-injected as an earlier (now stale) research document for 0179 claimed.
- `corpus::doc_type` / `corpus_adapters::doc_type` — path→doc-type inference and table-building, the same mechanism `validate-corpus-frontmatter.sh`'s `doc-type-table.sh`/`doc-type-inference.sh` implement in bash.
- `corpus::slug` — per-doc-type slug derivation, needed if `metadata derive` or `frontmatter validate` need filename-stem id extraction.
- `corpus_adapters::document::parse` — generic frontmatter fence extraction into an untyped `FrontmatterState::{Parsed(Mapping)|Absent|Malformed}`, fail-closed on tagged YAML nodes.

**Gaps — genuinely new work for 0195:**
1. **Named frontmatter field reading.** `corpus`'s value model is an untyped, order-preserving `Mapping` (`.get()`/`.push()`/`.entries()`) with no typed struct for the artifact-type discriminator, `schema_version`, or the typed-linkage keys (`parent`, `blocks`, `blocked_by`, `relates_to`, `derived_from`, `source`) — a caller must navigate the generic tree itself. This is deliberate per the 0179 research document (production consumers treat frontmatter as opaque JSON, never typed-deserialize), so `frontmatter validate` should follow the same navigation style rather than introducing typed deserialization.
2. **ADR numbering and status.** No allocator or status reader exists anywhere in `cli/` — a grep for `next_number`/`read_status`/`superseded_by`-as-lifecycle across the whole workspace found nothing beyond `linkage.rs`'s unrelated `supersedes` *keyword* handling. `adr next-number`/`adr read-status` are the smallest, most self-contained noun group (no config/VCS dependency for `read-status`; a `config path decisions` + directory-glob dependency for `next-number`) and a good first PR per 0195's own phasing note.
3. **`config`-crate wiring.** Neither `corpus` nor `corpus-adapters` reads `work.id_pattern`/`work.default_project_code`/doc-type paths from the `config` crate — today the *only* in-repo caller that constructs these injected types from real config is the visualiser server (`cli/visualiser/server/src/config.rs`, `indexer.rs`). `accelerator-corpus` must add this wiring itself; there's no existing CLI-facing precedent to copy verbatim, though the visualiser server's construction code is a close model.
4. **VCS port-surface decision, explicitly deferred to this work item.** The 0179 research document flags an open question it deliberately left to "0173's command surface" (0195's predecessor): whether the artifact-metadata port should expose the full working-copy revision (parity with the existing bash helpers, what `corpus_adapters::metadata` currently does) or a short, file-scoped id (parity with the separate 0007-migration helper pattern). Since `metadata derive`'s output contract must be preserved as-is (0195's own Technical Notes), the existing full-revision behaviour is the one to keep — but this is worth confirming explicitly at implementation time since the deferral was never formally closed.

### Sub-binary precedent: `accelerator-vcs` (`cli/vcs-cli/`)

The closest already-shipped analogue — same domain/adapter split shape (`vcs` + `vcs-adapters` mirrors `corpus` + `corpus-adapters`), same dispatch mechanism, same hexagonal port-injection pattern (`main.rs` composes a real adapter, e.g. `InProcessProbe`, and passes it into a generic `run<P: SomePort>` function also exercised in unit tests against a hand-written stub).

- `cli/vcs-cli/Cargo.toml` — package name `accelerator-vcs` (**directory name `vcs-cli` differs from the package name** — the pattern to replicate for `corpus-cli`/`accelerator-corpus`), `[[bin]] name = "accelerator-vcs"`, depends on `vcs`, `vcs-adapters`, `kernel`, `clap`, `serde_json`.
- `cli/vcs-cli/src/main.rs` — thin dispatcher: parses a clap `Cli`, matches each `Command` variant to a `run_*` function composing a real adapter and calling into the domain crate, maps `Result<(), kernel::Error>` to `ExitCode`.
- Multi-level (noun + verb) subcommand precedent already exists in the launcher itself: `accelerator config templates list/show/eject/diff/reset` (`cli/launcher/src/launch/inbound/cli.rs:252-312`) — directly the same `Noun { #[command(subcommand)] action: Verb }` clap-derive shape needed for `corpus adr next-number`, `corpus metadata derive`, etc.
- Characterization-test template: `cli/vcs-cli/tests/detect_goldens.rs` — spawns the compiled binary via `env!("CARGO_BIN_EXE_accelerator-vcs")` and asserts against fixture goldens; for `accelerator-corpus`, the goldens are the *captured bash output* per AC1's explicit definition.
- Dispatch mechanics: an unrecognised top-level token (`corpus`) falls into the launcher's `#[command(external_subcommand)]` catch-all, is resolved to a fetched/cached binary named `accelerator-corpus-<version>-<sha256>`, and is `exec`'d with the **remaining** args (`["adr", "next-number"]`) — the new binary's own clap tree must parse everything past the first token itself. A local-dev override (`ACCELERATOR_CORPUS_BIN`) works the same way as `ACCELERATOR_VCS_BIN` today.

### Sub-binary registration checklist (`tasks/README.md#registering-a-dispatched-sub-binary`) — applied to `corpus`

The thirteen-point checklist, with the `corpus`-specific answer to each:

1. **[PR]** Add `"corpus"` to `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py`); update the registry pin, `uploads` count, and `_setup_release` fixture/manifest in `tests/integration/tasks/test_github.py`.
2. **[release]** Add `"corpus": CLI_DIR / "corpus-cli/Cargo.toml"` to `_SUBBINARY_MANIFESTS` (`tasks/manifest.py`) — **required**, not optional, because `cli/corpus/` is already the domain crate; the default `cli/<token>/` resolution would collide with it. This is the one point where `corpus` deviates from the "no action needed" default path most tokens get.
3. **[release]** New crate's `Cargo.toml`: `[[bin]] name = "accelerator-corpus"`, mandatory `package.description`, workspace-inherited version/edition/rust-version/license/publish, `[lints] workspace = true` (or a justified local override).
4. **[release]** Register the new crate directory in `cli/Cargo.toml`'s `[workspace].members` (currently: `launcher, kernel, verify, document, config, config-adapters, corpus, corpus-adapters, vcs, vcs-adapters, vcs-cli, vcs-test-support, store, visualiser/server`); commit the regenerated `cli/Cargo.lock` (`lint:cli:check` runs `--locked`).
5. **[author]** `bin/corpus-*` needs no new `.gitignore` entry (`bin/<token>-*` is already the generic pattern the launcher's cache uses).
6. **[author]** No action needed on `cli/launcher/tests/fixtures/manifest.example.json` (key-agnostic consumers).
7. **[PR]** Skill binding: rewrite each call site's `allowed-tools` to a rule whose subcommand segment is exactly `corpus` (not wildcarded) — this is exactly 0195's own AC2, so satisfying the checklist point and the acceptance criterion are the same work.
8. **[release]** Add `"accelerator-corpus"` (the `[[bin]]` name, not the bare token) to `_CLI_RELEASE_BINARIES` (`tasks/build.py`) — rides the existing `cli_cross_compile` loop with no new task/`mise.toml` leaf needed.
9. **[PR]** No `attest-build-provenance` change needed — `dist/release/accelerator-*` already covers it.
10. **[PR]** No `BUILTIN_SUBCOMMANDS` change — `corpus` is a new dispatch token, not a built-in.
11. **[author]** User-facing docs: a `docs-site/` page, a README Concepts-list entry, an `ACCELERATOR_CORPUS_BIN` override row (worked example: `docs-site/src/content/docs/visualiser.md`) — needed since skill authors and maintainers are named beneficiaries in 0195's own Context.
12. **[author]** No `DEBUG_ARCHIVE_DIRS` entry unless `accelerator-corpus` ships a symbolication archive (unlikely for a thin utility CLI).
13. **[PR]** Extend `cli/deny.toml` only if the new crate's dependency graph needs a licence/advisory exception.

Points 1, 2, 3, 4, 7, and 8 must land **in the same change** — the release path and the dispatch-coherence guard (`lint:dispatch-coherence:check`, part of `build-system:check`) resolve them together. Token syntax must match `^[a-z][a-z0-9-]*$`; `verify`/`launcher` are reserved and would collide on staged-asset naming — not a concern for `corpus`.

### ADR guidance shaping the implementation

**ADR-0045 (Skills vs CLI division of labour, accepted)**: skills own only probabilistic work; all deterministic logic moves to a compiled CLI, invoked by skills via the `!` preprocessor. Explicitly names the duplication this migration removes: "the same corpus parsing, schema, and path conventions implemented once for the skills' shell library and again for the visualiser server." Hands off internal CLI structure to ADR-0053.

**ADR-0053 (Thin CLI over a hexagonal ports-and-adapters core, accepted)**: the CLI is the *inbound adapter* — argument parsing and presentation only, no business logic in the command layer, delegating immediately into the domain core's ports. Since `corpus` (core) and `corpus-adapters` (outbound adapters) already exist as separate crates, `accelerator-corpus` should depend on `corpus` for the port/domain types its commands compose against, and touch `corpus-adapters` only at the composition root (`main.rs`) to wire concrete adapters — command logic itself should be written against port traits, not `corpus-adapters` concrete types directly. Explicitly flags the trade-off worth checking during implementation: this indirection "is overhead for trivial commands carrying no real domain logic" — some of the five subcommands (e.g. `adr read-status`, which is pure text scanning) may not warrant a full port/adapter split and could reasonably be a thin pass-through instead.

### 0195's own review history

Already reviewed once (`meta/reviews/work/0195-...-review-1.md`), verdict **APPROVE** after a two-pass process: Pass 1 (verdict REVISE, 12 findings across clarity/completeness/dependency/scope/testability) was resolved almost entirely by direct edits in the same session, and Pass 2 caught and resolved 11 further issues its own edits introduced. Three items are deliberately left open by reviewer decision (not blocking): the four subcommand groups' asymmetric risk profile is accepted as intentional; ADR-0053 has no inline gloss in 0195's References; and no explicit re-open trigger is stated if a noun group's call-site count turns out larger than expected. The Dependencies section's claim about the scan-regex compiler already being available (see "What's already built," gap #… above — actually confirmed, not a gap) was directly source-verified during the re-review pass and is accurate as written.

## Code References

- [`skills/decisions/scripts/adr-next-number.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/skills/decisions/scripts/adr-next-number.sh) — ADR next-number allocator
- [`skills/decisions/scripts/adr-read-status.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/skills/decisions/scripts/adr-read-status.sh) — ADR status reader
- [`scripts/artifact-derive-metadata.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/artifact-derive-metadata.sh) — artifact-metadata derivation
- [`scripts/validate-corpus-frontmatter.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/validate-corpus-frontmatter.sh) — corpus-frontmatter validator
- [`scripts/linkage-parser.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/linkage-parser.sh) — body-prose typed-linkage extractor
- [`scripts/linkage-type-pairs.tsv`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/linkage-type-pairs.tsv) — `(source_type, key, target_type)` table
- [`scripts/templates-schema.tsv`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/templates-schema.tsv) — per-doc-type frontmatter validation schema
- [`skills/decisions/scripts/test-adr-scripts.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/skills/decisions/scripts/test-adr-scripts.sh) — golden baseline for `adr next-number`/`adr read-status`
- [`scripts/test-metadata-helpers.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/test-metadata-helpers.sh) — golden baseline for `metadata derive`
- [`scripts/test-validate-corpus-frontmatter.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/test-validate-corpus-frontmatter.sh) — golden baseline for `frontmatter validate`
- [`scripts/test-linkage-parser.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/scripts/test-linkage-parser.sh) — golden baseline for `linkage extract`
- [`cli/corpus/src/linkage.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/corpus/src/linkage.rs) — `parse_document`, `TYPE_PAIRS`, keyword/band classification (reusable)
- [`cli/corpus-adapters/src/metadata.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/corpus-adapters/src/metadata.rs) — `SystemClock`, `derive_at`, `render` (reusable)
- [`cli/corpus-adapters/src/work_item_pattern.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/corpus-adapters/src/work_item_pattern.rs) — `compile_scan_regex` (reusable)
- [`cli/corpus-adapters/src/scanner.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/corpus-adapters/src/scanner.rs) — `RegexScanner` (reusable)
- [`cli/corpus-adapters/src/document.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/corpus-adapters/src/document.rs) — generic frontmatter parse (reusable primitive; no named-field reader yet)
- [`cli/corpus-adapters/tests/metadata.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/corpus-adapters/tests/metadata.rs) — existing bash-parity test template
- [`cli/corpus-adapters/tests/parity.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/corpus-adapters/tests/parity.rs) — existing bash-parity test template
- [`cli/vcs-cli/Cargo.toml`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/vcs-cli/Cargo.toml) — sub-binary crate precedent
- [`cli/vcs-cli/src/main.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/vcs-cli/src/main.rs) — thin-dispatcher precedent
- [`cli/launcher/src/launch/inbound/cli.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/launcher/src/launch/inbound/cli.rs) — `config templates` noun/verb clap-derive precedent; external-subcommand dispatch
- [`cli/launcher/src/launch/core.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/launcher/src/launch/core.rs) — `ExternalCommand::from_raw`, override-var derivation
- [`cli/launcher/src/launch/outbound/resolve/mod.rs`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/launcher/src/launch/outbound/resolve/mod.rs) — sub-binary asset resolution/naming
- [`cli/Cargo.toml`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/cli/Cargo.toml) — workspace members list
- [`tasks/shared/paths.py`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/tasks/shared/paths.py) — `DISPATCHED_SUBBINARIES`, `DEBUG_ARCHIVE_DIRS`
- [`tasks/manifest.py`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/tasks/manifest.py) — `_SUBBINARY_MANIFESTS`
- [`tasks/build.py`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/tasks/build.py) — `_CLI_RELEASE_BINARIES`, cross-compile staging
- [`tasks/README.md`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/tasks/README.md) — the thirteen-point sub-binary registration checklist
- [`skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`](https://github.com/atomicinnovation/accelerator/blob/6e7a84e079f9518754043b9187d972f24e83f413/skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh) — non-skill caller of both `validate-corpus-frontmatter.sh` and `linkage-parser.sh`, in scope for AC2

## Architecture Insights

- **Hexagonal enforcement is graduated by packaging granularity** (ADR-0053): while `corpus`/`corpus-adapters` are already separate crates, the compiler's crate-boundary check already enforces the inward-dependency rule for this hexagon — `cargo-pup`'s nightly-lane enforcement matters most for *intra-crate* module boundaries, which is less of a concern here since the split already exists.
- **The value model is deliberately untyped.** `corpus`'s `FrontmatterValue`/`Mapping` mirrors the `config` crate's serde-free `Node` type — every production consumer navigates it as opaque JSON rather than deserializing into typed structs, a pattern established by 0178/0179 that `frontmatter validate`/`metadata derive` should follow rather than break.
- **Bash-parity testing is the established idiom for this exact kind of migration.** Two of 0195's five subcommands (`metadata derive`, `linkage extract`) already have working differential-parity test suites in `corpus-adapters/tests/`, gated behind a `bash-parity` feature that hard-fails (not skips) when the oracle script is missing — the same pattern, extended to the other three subcommands, satisfies AC1 directly.
- **The `accelerator-vcs`/`vcs-cli` precedent resolves the one packaging wrinkle for `corpus`**: because `cli/corpus/` is already a domain crate name, the binary crate cannot default to `cli/corpus/` (that's the checklist's normal no-`_SUBBINARY_MANIFESTS`-entry path) — it needs a sibling directory and an explicit manifest entry, exactly mirroring how `vcs` (domain) and `vcs-cli` (binary, package `accelerator-vcs`) already coexist.

## Historical Context

- [`meta/decisions/ADR-0034-typed-linkage-vocabulary.md`](../decisions/ADR-0034-typed-linkage-vocabulary.md) — defines the typed-linkage vocabulary (`parent`, `blocks`, `blocked_by`, `relates_to`, `derived_from`, `source`, `supersedes`, `target`) that `linkage extract` and `frontmatter validate` both operate over.
- [`meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`](../decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md) — defines the two-band (`resolved`/`ambiguous`) confidence model `linkage-parser.sh`/`corpus::linkage` implement.
- [`meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md`](../decisions/ADR-0045-skills-vs-cli-division-of-labour.md) — the epic-level rationale for this migration (see Detailed Findings above).
- [`meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`](../decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md) — the internal CLI structure this new binary must follow.
- [`meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`](2026-07-11-0179-corpus-crates-parsing-conventions.md) — the 0179 design research; explicitly names 0173 (0195's predecessor) as the consumer of these crates and flags the VCS-port-surface question left open for this work item. Note: its narrative on the work-item-id scan-regex compiler being test-only-injected is now stale — the compiler shipped in `corpus-adapters` (confirmed by source and by 0195's own review re-review pass).
- [`meta/reviews/work/0173-remaining-subdomains-corpus-design-collaboration-review-1.md`](../reviews/work/0173-remaining-subdomains-corpus-design-collaboration-review-1.md) — the scope-lens finding that triggered the 0173→0195/0196/0197 split.
- [`meta/reviews/work/0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli-review-1.md`](../reviews/work/0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli-review-1.md) — 0195's own review (verdict APPROVE); see Detailed Findings above for the resolved/open finding breakdown.
- [`meta/prs/52-description.md`](../prs/52-description.md) — the PR that executed the 0173 split into 0195/0196/0197.

## Related Research

- [`meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`](2026-06-28-0136-rust-cli-migration-scope-and-architecture.md) — epic-level scope/architecture for the whole Rust CLI migration (work-item 0136).
- [`meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`](2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md) — the original shell-script/test-suite surface survey the epic was scoped from.
- [`meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`](2026-07-11-0179-corpus-crates-parsing-conventions.md) — see Historical Context above.
- [`meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`](2026-08-02-0187-generalise-sub-binary-registration-surface.md) — the research behind the thirteen-point checklist this work item must satisfy.

## Open Questions

- **VCS port surface for `metadata derive`**: full working-copy revision (current `corpus_adapters::metadata` behaviour, matches the bash helpers) vs a short, file-scoped id (matches the separate 0007-migration bash pattern) — the 0179 research document deferred this explicitly to 0195's command surface and it was never formally closed. Since the output contract must be preserved as-is, the existing full-revision behaviour is presumably correct, but this is worth a one-line confirmation before implementation rather than an assumption.
- **Crate directory naming for the binary**: this research assumes `cli/corpus-cli/` (mirroring `vcs`→`vcs-cli`) as the natural name, but that's an inference from precedent, not a decision recorded anywhere — worth confirming before scaffolding (it drives the `_SUBBINARY_MANIFESTS` entry and the workspace-members edit).
- **Whether to preserve or correct `linkage-parser.sh`'s hardcoded 4-digit bare-number scan**: AC1 requires byte-for-byte characterization parity with current bash behaviour, which argues for preserving the gap (a customised `work.id_pattern` wouldn't affect prose-derived bare-number refs) rather than "fixing" it opportunistically during the port — but this is worth a conscious decision rather than an accidental carry-over, since `corpus::linkage`'s existing Rust port may already have made a different choice (not checked in this research pass — recommend diffing `corpus::linkage`'s bare-number extraction against `lp_extract_tokens`'s `[0-9]{4}` behaviour specifically before relying on the existing Rust module for this subcommand).
- **`frontmatter validate`'s exact API shape** against the untyped `Mapping`/`FrontmatterValue` tree — no precedent CLI-facing named-field reader exists yet to model on; this is new design work, not just porting.
