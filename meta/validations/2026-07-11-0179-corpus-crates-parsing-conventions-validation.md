---
type: plan-validation
id: "2026-07-11-0179-corpus-crates-parsing-conventions-validation"
title: "Validation Report: corpus and corpus-adapters Crates for Parsing and Conventions"
date: "2026-07-13T20:38:13+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-07-11-0179-corpus-crates-parsing-conventions"
target: "plan:2026-07-11-0179-corpus-crates-parsing-conventions"
tags: [rust, corpus, document, vcs, crates, frontmatter, serde-saphyr, doc-type, typed-linkage, parity]
last_updated: "2026-07-13T21:43:26+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: corpus and corpus-adapters Crates for Parsing and Conventions

All five phases are implemented and every automated gate is green. The first pass
found three plan requirements that had been ticked without being implemented —
one of them a live behavioural regression — plus several smaller gaps. All were
closed during validation; the fixes are described below and the remaining
divergences are documented deviations rather than open work.

**Result: pass.**

### Implementation Status

- ✓ **Phase 1: `document` crate + `config-adapters` retrofit** — fully implemented
- ✓ **Phase 2: `corpus` domain crate** — fully implemented
- ✓ **Phase 3: `corpus-adapters` — parse and conventions** — fully implemented
  (YAML-tag rule, `bash-parity` feature, and the parity-corpus gaps closed in
  validation)
- ✓ **Phase 4: `vcs` + `vcs-adapters`** — fully implemented (bare-repo fixture added)
- ✓ **Phase 5: artifact-metadata derivation** — fully implemented (the `date +%z`
  offset coupling is now recorded as a deviation in the plan)

### Automated Verification Results

Counts are post-fix. The `bash-parity` feature now gates the suites that shell
out, so there are two meaningful numbers:

✓ Full read-only CI mirror: `mise run check` (exit 0)
✓ CI path — `mise run test:unit:cli` (`--all-features`): **311 passed, 0 skipped**
✓ Bare-machine path — `cargo nextest run --workspace`: **295 passed, 0 skipped**
  (the differential suites compile out; nothing silently skips)
✓ Dependency bans: `mise run deny:check` (advisories/bans/licenses/sources ok)
✓ Import rules: `mise run pup:check` (exit 0)
✓ Enforcement regressions: `mise run test:integration:deny`, `mise run test:integration:pup`
✓ `bash skills/work/scripts/test-work-item-pattern.sh` — all passed
✓ `bash scripts/test-linkage-parser.sh` — all passed
✓ `bash scripts/test-metadata-helpers.sh` — all passed
✓ `bash skills/config/migrate/scripts/test-migrate-0007.sh` — all passed
✓ `bash scripts/test-config.sh` — all passed

Starting point for comparison: 294 tests, with the differential suites ungated.

### Code Review Findings

#### Matches Plan

- **Workspace + enforcement.** `cli/Cargo.toml:4` lists members in exactly the
  planned order. The serde-saphyr wrapper ban is re-homed to `document`
  (`cli/deny.toml:68`), and the deny fixture rename is complete *including* the
  load-bearing `Cargo.lock` `[[package]] name` — the detail the plan flagged as
  easy to miss.
- **`document` crate.** `fence_offsets` carries the 1 MiB `MAX_SCAN`, is
  CRLF-tolerant, and accepts a closing fence with no trailing newline. `split`
  slices the body verbatim. `render` genuinely **re-parses** the existing
  frontmatter (`render.rs:32`), so a fence-valid-but-invalid-YAML file fails closed.
- **Wildcard-free mapping.** `config::Scalar` dropped `#[non_exhaustive]`, and every
  `Yaml ↔ Node ↔ FrontmatterValue` arm is explicit — no `_` wildcard in any mapping
  function.
- **`corpus` domain purity.** Kernel-only, no `regex`, no serde. `DocTypeKey` has
  exactly 14 variants with a collision-free wire round-trip. `canonical_digit_width`
  correctly keeps the `0`/"admit-any" default rather than the server twin's `4`.
- **`match_end` is the full-match end**, with the direct assertion the plan asked for.
- **Fail-closed store pins** all exist, including the over-cap → `MalformedFrontmatter`
  case.
- **The visualiser server is untouched**, as the plan required.
- **Design-inventory id bug fixed** as described, with the two-inventory probe that a
  basename-derived id would collapse.

#### Fixes Applied During Validation

**1. The YAML-tag fail-closed rule (was: unimplemented; a live regression).**

Phase 3 §2 required *any* explicit YAML tag in frontmatter → `Malformed`. No tag
handling existed anywhere in `cli/`. Tagged nodes silently parsed as their untagged
base value (`key: !custom value` → `Parsed("value")`) — a regression against the
visualiser oracle, and internally inconsistent (`!!int` failed closed while `!!str`
and `!custom` did not).

*Why the plan's approach was impossible.* The plan assumed a `YamlVisitor`-level
guard. serde-saphyr resolves a tag against its schema **before** the serde boundary
and hands the visitor the resolved base value, so the tag is already gone by the time
any `Visitor` method runs. There is also no tag-rejection option on
`serde_saphyr::Options`.

*Where it went.* serde-saphyr re-exports its parser (`pub use granit_parser`), whose
event stream carries an explicit `Option<Tag>` on every `Scalar`, `SequenceStart`,
and `MappingStart`. `cli/document/src/tags.rs` scans that stream and rejects the first
tagged node. This keeps `document` the sole serde-saphyr wrapper (no new dependency;
the ban regression still passes) and is the *structural* boundary the plan asked for:
a tag inside a quoted scalar is string content carrying no tag, and a tag on a nested
value is still a tagged event. Aliases are `Alias` events rather than expansions, so
the scan stays bounded on an alias-bomb input.

`DocumentError` gains `Tagged(String)`; `corpus-adapters` maps it to `Malformed` and
`config-adapters` to `MalformedFrontmatter` through their existing `Err` arms, so the
config read path fails closed on tags too. No document in the repo uses a tag
(verified), so nothing changes in practice. 12 tests added.

**2. The `bash-parity` cargo feature (was: did not exist).**

Declared on `corpus-adapters` and `vcs-adapters`; CI enables it via `--all-features`
in `tasks/test/cli.py`. `parity.rs`, `doc_type_single_source.rs`, and `detection.rs`
are gated whole-file; in `metadata.rs` only the live-helper test is gated so the
deterministic fake-port assertions keep running bare. `vcs-adapters`' marker-walk
no-facts case moved into a crate unit test — it needs no VCS binary, so gating it away
would have lost bare-machine coverage for nothing. The plan's contract now holds in
both directions: 311 tests with the feature on, 295 with it off, nothing skipped
either way.

**3. The prefix-dir and exact-length-tie parity cases (were: missing entirely).**

`doc_type_inference_matches_the_bash_matcher` (`tests/parity.rs`) drives the live
`doc-type-inference.sh` — which was previously never invoked from `cli/` at all — over
an **injected** table, since the repo's own config has no prefix-pair among its
`PATH_KEYS`. The table nests `meta/design/inventories` under `meta/design` and ties two
review types on the same directory, so longest-dir-wins and first-entry-tie are
exercised on **both** sides. A vacuity guard pins the oracle's exact output, so the
diff cannot pass by both sides resolving nothing.

**4. Smaller gaps.**

- Non-numeric review suffix → `None` now pinned, along with internal-`-review-`
  preservation (`corpus/src/slug.rs`).
- Bare-repo fixture added (`vcs-adapters/tests/detection.rs`): `git init --bare`,
  asserting the layout has no `.git` marker and `facts → None`.
- `a_body_without_a_trailing_newline_is_preserved` re-ported as a direct value
  assertion (`document/src/fence.rs`) — the round-trip test it had been folded into
  compares two `split` calls to each other and would still pass if the body were
  dropped.
- `corpus::slug::title_case_segment` and `strip_humanise_prefix` made `pub`, so the
  0168 retirement of the two server-side copies can actually import them.
- `PrDescriptions`' three genuinely-different names (config key `prs`, wire
  `pr-descriptions`, linkage `pr-description`) now pinned explicitly.
- The unused `kernel` dependency dropped from `corpus` and `vcs`, with a comment
  recording that the pup rule still permits `kernel::Error` when a fallible
  convention needs it.
- Two stale comments corrected (`cli/Cargo.toml`, `test_serde_saphyr_ban.py`).

**5. The harness now distinguishes files it spawns from files it reads.**

`require_script` asserts the executable bit (it spawns those directly, so a cleared
bit would surface as an opaque spawn failure); `require_file` asserts presence only.
Adding the exec-bit check immediately caught that four of the harness's targets are
*not* executable by design — `doc-type-inference.sh` and `config-defaults.sh` are
sourced libraries, and the `.awk`/`.tsv` targets are read — which is the repo's
exec-bit invariant working as intended. Those call sites now use `require_file`.

#### Accepted Deviations

- **Scope beyond the five phases (four commits).** `8abc3609`, `35d78cd5`, `869a81ed`,
  `3cf0b568` modify the bash side — `linkage-parser.sh`, `doc-type-table.sh`,
  `linkage-type-pairs.tsv`, the 0007 migration and its awk. They fix real bugs found
  while porting (a `normalize_paths` `RSTART`/`RLENGTH` splice that corrupted every ADR
  path reference into malformed YAML; a missing `meta/prs` arm; a hardcoded doc-type
  table), documented in `meta/notes/2026-07-13-bash-corpus-script-inconsistencies.md`.
  The plan positioned bash as the parity *oracle* and the oracle was changed during the
  port — worth conscious acknowledgement, though the bash suites remain green and the
  changes are improvements.
- **`SystemClock::try_new` shells out to `date +%z`** for the host offset. Phase 5 §1
  justified the `time` dependency as "no shell-out to `date`" — true of the *rendering*,
  not of acquiring the offset. It satisfies §2's "short-lived single-threaded subprocess"
  literally and sidesteps `time`'s multithread refusal; `date` is POSIX and present
  across the supported matrix. Now recorded as a deviation in the plan rather than
  reworked — a self-re-exec would trade a POSIX utility for a hidden subcommand.
- **The controlled-`TZ` assertion** is realised as an injected offset
  (`SystemClock::with_offset(+05:30)`), which is deterministic and non-vacuous even on a
  `TZ=UTC` CI host, rather than mutating process environment from a multithreaded test.
- **The assembler takes `raw: &[u8]`, not a path.** "Reads a file → parses → invokes the
  conventions" became "caller supplies bytes", keeping the assembler pure. The file read
  lands with the CLI surface in 0173.

### Manual Testing Required

1. Bash-oracle drift:
   - [ ] Review the four beyond-plan bash commits against the 0007 migration's behaviour
         on a real corpus before it ships
2. Tag policy widening:
   - [ ] Confirm the config read path failing closed on tags is intended (no config in
         the repo uses tags, so nothing changes today)

### Recommendations

Carried forward to 0168, not blocking:

- The 0168 fold must add a conformance test binding the server-side twins
  (`config::label_from_key`, `api::library::humanise_status`,
  `indexer::number_width_from_id_pattern`) to the canonical `corpus` copies as it
  retires them. Nothing binds them today, so they can silently diverge in the window.
- The SPA/API JSON `Serialize` boundary is still deferred: the order-preserving
  `Vec<(String, _)>` model and the big-int-as-`String` policy diverge from the shipped
  `BTreeMap`/numeric shape, so 0168 must either preserve the old shape or accept the
  change deliberately.
- `config-adapters::discover_root` remains a second marker-walk distinct from
  `vcs::MarkerWalkRoot` (it also stops at `.accelerator` and falls back to `start`).
  0168 should fold it onto a parameterised walk or record it as a permanent fork.
