---
type: plan-validation
id: "2026-07-11-0179-corpus-crates-parsing-conventions-validation"
title: "Validation Report: corpus and corpus-adapters Crates for Parsing and Conventions"
date: "2026-07-13T20:38:13+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: partial
parent: "plan:2026-07-11-0179-corpus-crates-parsing-conventions"
target: "plan:2026-07-11-0179-corpus-crates-parsing-conventions"
tags: [rust, corpus, document, vcs, crates, frontmatter, serde-saphyr, doc-type, typed-linkage, parity]
last_updated: "2026-07-13T20:38:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: corpus and corpus-adapters Crates for Parsing and Conventions

Validated at revision `dbce52f7` on bookmark `0179-corpus-crates`, working copy
clean. All five crates exist, every automated gate is green, and the structural
core of the plan landed faithfully. Three plan requirements were **not**
implemented, one of which is a genuine behavioural regression against the oracle
the plan set out to preserve.

### Implementation Status

- ✓ **Phase 1: `document` crate + `config-adapters` retrofit** — fully implemented
- ✓ **Phase 2: `corpus` domain crate** — fully implemented
- ⚠️ **Phase 3: `corpus-adapters` — parse and conventions** — partially implemented
  (YAML-tag rule was absent — **fixed during validation**; `bash-parity` feature
  absent; parity-corpus gaps remain)
- ⚠️ **Phase 4: `vcs` + `vcs-adapters`** — implemented; one specified fixture missing
- ⚠️ **Phase 5: artifact-metadata derivation** — implemented; offset resolution
  diverges from the stated design rationale

### Automated Verification Results

(Re-run after the YAML-tag fix; counts below are post-fix.)

✓ Full read-only CI mirror: `mise run check` (exit 0)
✓ Rust unit + integration suites: `mise run test:unit:cli` — **306 passed, 0 skipped**
  (294 as found, +12 from the tag guard)
✓ Dependency bans: `mise run deny:check` (advisories/bans/licenses/sources ok)
✓ Import rules: `mise run pup:check` (exit 0)
✓ `bash skills/work/scripts/test-work-item-pattern.sh` — all passed
✓ `bash scripts/test-linkage-parser.sh` — all passed
✓ `bash scripts/test-metadata-helpers.sh` — all passed
✓ `bash skills/config/migrate/scripts/test-migrate-0007.sh` — all passed

Note: the test count is identical (294) under default features and
`--all-features`, which is itself the evidence that the planned `bash-parity`
feature gate does not exist.

### Code Review Findings

#### Matches Plan

- **Workspace + enforcement.** `cli/Cargo.toml:4` lists members in exactly the
  planned order. The serde-saphyr wrapper ban is re-homed to `document`
  (`cli/deny.toml:68`), and the deny fixture rename is complete *including* the
  load-bearing `Cargo.lock` `[[package]] name`
  (`tests/integration/deny/fixtures/serde-saphyr-clean/Cargo.lock:6`) — the detail
  the plan flagged as easy to miss.
- **`document` crate.** `fence_offsets` carries the 1 MiB `MAX_SCAN`
  (`cli/document/src/fence.rs:11,45`), is CRLF-tolerant, and accepts a closing
  fence with no trailing newline (`fence.rs:57-63`, pinned at `fence.rs:166-173`).
  `split` slices the body verbatim — no `trim_start_matches('\n')` anywhere.
  `render` genuinely **re-parses** the existing frontmatter (`render.rs:32`), so a
  fence-valid-but-invalid-YAML file fails closed
  (`cli/document/tests/document.rs:88-93`).
- **Wildcard-free mapping.** `config::Scalar` dropped `#[non_exhaustive]`
  (`cli/config/src/node.rs:12-13`), and every `Yaml ↔ Node ↔ FrontmatterValue` arm
  is explicit — no `_` wildcard in any mapping function
  (`cli/config-adapters/src/document.rs:31-80`,
  `cli/corpus-adapters/src/document.rs:69-96`).
- **`corpus` domain purity.** Kernel-only, no `regex`, no serde. `DocTypeKey` has
  exactly 14 variants with a collision-free wire round-trip over all of them
  (`cli/corpus/src/doc_type.rs:239-260`). `canonical_digit_width` correctly keeps
  the `0`/"admit-any" default rather than the server twin's `4`
  (`work_item_id.rs:64-82`, pinned at `:162-175`).
- **The `or_else` slug fallback** the plan called out is real and tested:
  `0042-legacy.md` under a `{project}-{number:04d}` pattern still yields `legacy`
  (`cli/corpus/src/slug.rs:277-300`).
- **`match_end` is the full-match end**, with the direct assertion the plan asked
  for (`cli/corpus-adapters/src/scanner.rs:58-66`).
- **Fail-closed store pins** all exist: `a_write_against_a_malformed_file_fails_closed`,
  the fence-valid-but-invalid-YAML store test, and the over-cap → `MalformedFrontmatter`
  test (`cli/config-adapters/src/store.rs:281,300-318,320-333`).
- **The visualiser server is untouched**, as the plan required — it does not appear
  in the branch diff.
- **Design-inventory id bug fixed** as described: the rewrite awk now takes the
  parent directory (`0007-frontmatter-rewrite.awk:103-109`) and
  `test-migrate-0007.sh:532-542` gained the two-inventory probe that a
  basename-derived id would collapse.

#### Deviations from Plan

- **Scope beyond the five phases (four extra commits).** `8abc3609`, `35d78cd5`,
  `869a81ed`, `3cf0b568` modify the bash side — `scripts/linkage-parser.sh` (+120
  lines), `scripts/doc-type-table.sh`, `scripts/linkage-type-pairs.tsv`, the 0007
  migration and its awk. These fix real, separately-discovered bugs (a
  `normalize_paths` `RSTART`/`RLENGTH` splice that corrupted every ADR path
  reference into malformed YAML; a missing `meta/prs` arm; a hardcoded doc-type
  table). They are documented in
  `meta/notes/2026-07-13-bash-corpus-script-inconsistencies.md`. The work is sound
  and the bash suites stay green, but the plan positioned bash as the **parity
  oracle**, and the oracle was changed during the port. Worth a conscious
  acknowledgement rather than silence.
- **`SystemClock::try_new` shells out to `date +%z`** (`corpus-adapters/src/metadata.rs:107-110`).
  The plan's Phase 5 §1 justified the `time` dependency precisely so the binary
  "stays self-contained (no shell-out to `date`)". The recorded deviation note
  reframes this as "the subprocess the plan already mandates", but the mandated
  subprocess was a single-threaded re-exec resolving the offset via `time`, not a
  call to `date`. Behaviourally sound and thread-safe; the self-contained-binary
  property is not achieved.
- **The controlled-`TZ` assertion** is realised as an injected offset
  (`SystemClock::with_offset(+05:30)`, `tests/metadata.rs:192-212`) rather than a
  `TZ`-driven resolution. Deterministic and non-vacuous, but the `TZ` → host-offset
  path is never exercised end-to-end.
- **The assembler takes `raw: &[u8]`, not a path** (`assemble.rs:33`) — there is no
  `std::fs` use in the crate. "Reads a file → parses → invokes the conventions"
  became "caller supplies bytes". Defensible (it keeps the assembler pure), but the
  file-read step the plan described is absent.
- **`SystemClock`'s field is `offset`, not `local_offset`**; `render` and
  `with_offset` are unplanned additions. Cosmetic.
- Two stale comments: `cli/Cargo.toml:44-45` still points serde-saphyr review at
  "the config-adapters adapter boundary", and
  `tests/integration/deny/test_serde_saphyr_ban.py:4` still documents
  `wrappers = ["config-adapters"]`. Both are now `document`.

#### Potential Issues

1. ~~**(Significant) The YAML-tag fail-closed rule is unimplemented, and tagged
   nodes now silently parse.**~~ — **FIXED during validation** (see "Fix applied"
   below). As found, Phase 3 §2's rule was absent: no tag handling existed
   anywhere in `cli/`, and tagged nodes silently parsed as their untagged base
   value (`key: !custom value` → `Parsed("value")`), a regression against the
   visualiser oracle and internally inconsistent (`!!int` failed closed while
   `!!str` and `!custom` did not).

2. **The `bash-parity` cargo feature does not exist.** No `[features]` table in any
   of the five new crates; zero matches for `bash-parity` outside `meta/` prose; and
   `tasks/test/cli.py:18-22` passes no feature flag. The parity/single-source/detection
   suites are therefore **ungated**: they do run in CI (good), but the plan's stated
   contract — that `cargo test` stays runnable on a bare machine without
   bash/awk/jj/git — is not met. On such a machine those suites hard-fail.

3. **Missing parity-corpus cases.** The plan required (Phase 3 §7) a fixture path
   nested under a configured doc-type directory that is *itself a prefix of
   another*, plus an exact-length-tie case, so longest-dir-wins is exercised on
   **both** sides. Neither exists — longest-match is covered only by pure-Rust unit
   tables (`corpus/src/doc_type.rs:308-322`), never diffed against bash. Also,
   `doc-type-inference.sh` is never invoked directly from `cli/` (only transitively,
   since `linkage-parser.sh` now sources it), and the pinned edges
   (`2026-04-17-100-day-plan.md`, `2026-05-31-0040.md`, `ADR-0001.md`) live as
   declared-value unit tests in `corpus`, not in the differential suite.

4. **Non-numeric review suffix → `None`** is implemented (`corpus/src/slug.rs:151-153`)
   but pinned by no test, despite being one of the three review-suffix edges the
   plan enumerated.

5. **No bare-repo fixture** in `vcs-adapters/tests/detection.rs`. The plan's Phase 4
   fixture list asked for one; the marker-less case is a plain empty temp dir
   (`a_tree_with_no_marker_has_no_facts`). A true bare repo (top-level
   `HEAD`/`objects`/`refs`) is untested.

6. **One `split` test was dropped, not ported.** `a_body_without_a_trailing_newline_is_preserved`
   asserted `split(...).body == "no newline"` directly. Its only trace is inside
   `render_preserves_the_body_byte_for_byte`, which compares two `split` calls to each
   other — an equality that would still hold if `split` dropped a newline-less body
   entirely. The plan required the relocated tests be ported, "not dropped".

7. **`corpus::slug::title_case_segment` and `strip_humanise_prefix` are crate-private.**
   The plan names `title_case_segment` the *canonical* title-caser that 0168 retires
   the two server-side copies onto — but it is not reachable from another crate. 0168
   must widen visibility first.

8. **`corpus` and `vcs` declare a `kernel` dependency that no module imports.** Harmless
   today (both crates' pup rules permit `kernel::Error`), but it is dead weight.

### Manual Testing Required

1. YAML-tag policy:
   - [x] Fail-closed on any tag — implemented and pinned (see "Fix applied")
   - [ ] Confirm the widening to the **config** read path is intended (a tagged
         config value now resolves as `MalformedFrontmatter`; no config in the repo
         uses tags, so nothing changes today)
2. Bare-machine ergonomics:
   - [ ] Confirm whether `cargo test` on a machine without bash/awk/jj/git is a
         requirement worth keeping, or whether the ungated suites are acceptable
3. Bash-oracle drift:
   - [ ] Review the four beyond-plan bash commits against the 0007 migration's
         behaviour on a real corpus before it ships

### Fix Applied: the YAML-tag guard

Potential issue 1 was fixed during validation rather than deferred, since it was
the only finding that changed runtime behaviour.

**Why a `YamlVisitor` arm was not possible.** The plan assumed the guard could be
a visitor-level structural check. It cannot: serde-saphyr resolves a tag against
its schema *before* the serde boundary and hands the visitor the resolved base
value, so the tag is already gone by the time any `Visitor` method runs. There is
also no tag-rejection option on `serde_saphyr::Options`.

**Where the guard actually went.** serde-saphyr re-exports its parser
(`pub use granit_parser`), whose event stream carries an explicit
`Option<Tag>` on every `Scalar`, `SequenceStart`, and `MappingStart`. The guard —
`cli/document/src/tags.rs`, `reject_tagged` — scans that event stream and rejects
the first tagged node. This keeps `document` the sole serde-saphyr wrapper (no new
dependency, `deny:check` and its ban regression still green) and is the *structural*
boundary the plan asked for: a tag inside a quoted scalar is string content carrying
no tag, and a tag on a nested value is still a tagged event. Aliases are `Alias`
events rather than expansions, so the scan stays bounded on an alias-bomb input.

**Behaviour.** `DocumentError` gains a `Tagged(String)` variant naming the offending
tag; `parse_frontmatter` rejects before deserialising. `corpus-adapters` maps it to
`FrontmatterState::Malformed` through the existing `Err(_)` arm, and `config-adapters`
to `MalformedFrontmatter` — so the config read path fails closed on tags too. That is
a slight widening beyond the plan's corpus-only wording, but it is the consistent
choice and no document in the repo uses a tag (verified). All seven cases from the
probe table above now behave as the plan specified, including the quoted-substring
case still parsing.

**Coverage.** 12 new tests (306 total, up from 294): 10 in `cli/document/src/tags.rs`
(local tag, standard tags, nested value, sequence item, tagged collection, tagged
root, quoted-substring, untagged, error names the tag, malformed input still reports
`InvalidYaml`) and 2 in `cli/corpus-adapters/src/document.rs`
(`a_tagged_node_is_malformed` across six shapes, `a_tag_inside_a_quoted_scalar_still_parses`).

Re-verified green: `mise run check`, `test:unit:cli` (306 passed),
`deny:check`, `pup:check`, `test:integration:deny`, `test:integration:pup`, and the
bash suites (`test-work-item-pattern.sh`, `test-linkage-parser.sh`,
`test-metadata-helpers.sh`, `test-migrate-0007.sh`, `test-config.sh`).

### Recommendations

**Before merge:**

1. **Either add the `bash-parity` feature or amend the plan.** The suites are ungated
   and the plan's bare-machine claim is false as written. Amending the plan is a
   legitimate resolution; leaving the prose asserting a feature that does not exist is
   not.

3. **Add the prefix-dir and exact-length-tie parity fixtures.** These were specified
   precisely so longest-dir-wins could not pass vacuously — and right now it passes
   vacuously on the bash side.

**Lower priority:**

4. Re-port `a_body_without_a_trailing_newline_is_preserved` as a direct value assertion.
5. Pin the non-numeric review-suffix → `None` case.
6. Add a genuine bare-repo fixture to `detection.rs`.
7. Make `title_case_segment` / `strip_humanise_prefix` `pub` so the 0168 retirement can
   actually import them.
8. Fix the two stale comments (`cli/Cargo.toml:44-45`,
   `tests/integration/deny/test_serde_saphyr_ban.py:4`).
9. Reconcile the `date +%z` shell-out with the plan's self-contained-binary rationale —
   either implement the re-exec the plan described or amend the rationale.
