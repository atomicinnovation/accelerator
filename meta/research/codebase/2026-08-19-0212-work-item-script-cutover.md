---
type: "codebase-research"
id: "2026-08-19-0212-work-item-script-cutover"
title: "Research: Work-Item Script Cutover (0212)"
date: "2026-08-19T10:55:44+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0212"
parent: "work-item:0212"
relates_to: ["codebase-research:2026-08-17-0210-provider-client-crates-over-the-tracker-port", "codebase-research:2026-08-17-0211-integration-binaries-and-bash-cluster-retirement", "codebase-research:2026-08-12-0194-tracker-crate-and-remote-sync-engine"]
topic: "Work-Item Script Cutover (0212)"
tags: ["research", "codebase", "rust", "cutover", "work-items", "fixtures", "cli", "tracker"]
revision: "113d370d3f44e3bc51f75d656709d68e36073215"
repository: "accelerator"
last_updated: "2026-08-19T10:55:44+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research scoping the extra work to make 0212 achievable in one plan"
schema_version: 1
---

# Research: Work-Item Script Cutover (0212)

**Date**: 2026-08-19T10:55:44+00:00
**Author**: Toby Clemson
**Git Commit**: 113d370d3f44e3bc51f75d656709d68e36073215
**Branch**: build-system workspace (detached HEAD)
**Repository**: accelerator

## Research Question

What does work item 0212 (Work-Item Script Cutover) actually require against
the live codebase — the eighteen scripts to delete, the three work skills to
repoint, the eleven parity tests to convert, the fixture corpus to relocate,
the dirty-guard reachability, the three port-less bridge capabilities, the
`E_DISPATCH` consolidation, and the build-system floor — and where do the work
item's stated premises diverge from what the code and the governing documents
now hold?

## Summary

The mechanical deletions and the fixture relocation are well-scoped and mostly
match the code. **Four findings materially change the shape of a plan**, three
of them blocking:

1. 🔴 **The three port-less capability fates are `*open*`, not decided —
   0212's own precondition is unmet.** 0171's `## Decisions` records all three
   verbatim as `*open*`, and every one needs a *new* `RemoteTracker` port
   operation (none has a Rust equivalent today). Per 0171's protocol, choosing
   the additive-port fate for any of them means filing a new work item parented
   to 0136, blocking 0212. A documented override moved the items to `ready`
   with the questions still open, deferring them to "the first planning
   session" — so planning must settle them before committing to a shape.
2. ⚠️ **`accelerator work` has no `normalise` and no `list` subcommand, and no
   composable sync primitives.** `sync-work-items` and `list-work-items` call
   bash capabilities (`normalise --stdin`, `sync-label`, `sync-baseline` reads,
   `sync-classify`, `fetch-remote search`) that `work sync` subsumes only as an
   all-in-one command, not as swappable pieces. Repointing `list-work-items`
   in particular is not a clean swap — its sync-status rendering has no CLI port.
3. ⚠️ **The `E_DISPATCH` "item 4" premise is largely stale.**
   `cli/tracker/src/errors.rs` does not exist; none of the four named comments
   still references a deleted bash artefact (already scrubbed); and the numeric
   taxonomy owner is `cli/work-cli/src/exit_codes.rs`, not the `tracker` crate.
   Extra bash coupling lives in `cli/tracker-support/`.
4. ⚠️ **The eleven-test list is partly wrong.**
   `cli/work-adapters/tests/project_remote_parity.rs` does not exist —
   projection parity lives in `remote-projection`, `jira-client`, and
   `jira/linear`'s `projection_corpus.rs`. Two extra script-coupled files the
   work item omits (`sync_baseline_shellout_parity.rs`,
   `cli/tracker-support/tests/mapper_differential.rs`) must also be handled.

✅ The one ⚠️ assumption the work item flagged as size-bounding — the dirty
guard's CLI reachability — **resolves in the work item's favour**: the guard is
live-wired into `accelerator work sync` and fail-safe. The corpus baseline
number is confirmed: **68 files** under `test-fixtures/`, recorded in
`cli/work-adapters/tests/fixtures/bash-parity-baseline.txt`.

## Detailed Findings

### The eighteen files to delete

All eighteen exist under `skills/work/scripts/`, matching the work item exactly:
thirteen production scripts (`work-item-bridge-codes`, `-create-remote`,
`-fetch-remote`, `-file-dirty`, `-normalise`, `-project-remote`, `-push-decide`,
`-sync-apply`, `-sync-baseline`, `-sync-classify`, `-sync-decide`, `-sync-label`,
`-update-remote`) and five suites (`test-work-item-create-remote`,
`-fetch-remote`, `-scripts`, `-sync-apply`, `-update-remote`).

**Consumer sweep beyond `skills/work/`** — the load-bearing external references:
- `tasks/lint/scripts.py:34` — `work-item-bridge-codes.sh` in `SHELL_LIBRARIES`
  (work item says line 18; **actual is line 34** — line drift).
- `skills/integrations/jira/scripts/jira-resolve-fields.sh:140` and
  `skills/integrations/linear/scripts/linear-create-flow.sh:304` — **comments
  only**, confirming the one-directional coupling the ordering argument rests on.
- `docs-site/src/content/docs/reference/skills/work/{list,sync,create}-*.md` —
  the generated reference **mirror** carries 38 references (23 + 12 + 3) that
  regenerate from the SKILL.md sources. The AC's "excluding `meta/`" grep sweep
  will hit these; they clear when the sources are repointed and docs regenerate.
- `hooks/`, `templates/`, `agents/` — **zero** hits.

`skills/work/create-work-item/` also holds an `evals/` subtree; its
`benchmark.json` references already-migrated basenames (`work-item-next-number.sh`,
`work-item-read-field.sh`) that are not among the thirteen — historical eval
data, not production wiring.

### Dirty-work-item overwrite guard — reachable and fail-safe ✅

This resolves the work item's flagged ⚠️ size-bounding assumption. The guard is
live-wired into `accelerator work sync`; a dirty local item can never be planned
as `Pull`:

```text
work-cli/src/sync.rs:305  VcsWorkingCopyStatus::probed_from(&root)  (port wiring)
  → work-adapters/src/sync/run.rs:141-147  gather(..., ports.status, ...)
    → work-adapters/src/sync/fetch.rs:247   status.is_dirty(&item.path) → PlanInput.dirty (:98)
      → work/src/sync/decide.rs:87-103      Action::Pull only when Dirtiness::Clean
```

`decide.rs` documents `Dirtiness::Unknown` as "decide as `Dirty` everywhere"
(`decide.rs:14-15`), so a failed probe blocks the overwrite — fail-safe.
`working_copy_status.rs` caches one whole-tree diff from
`vcs-adapters/src/library/dirty_paths.rs` (gix `status` for git; a snapshot-vs-
parent-tree diff that never persists for jj).

⚠️ **Gap**: the normalisation *logic* (`cli/work/src/normalise.rs`) exists and is
used internally by the sync digest (`work-adapters/src/sync/digest.rs:78-92`),
but there is **no `Normalise` CLI variant and no `--stdin` mode**. The
`work-item-project-remote.sh | work-item-normalise.sh --stdin` pipe at
`sync-work-items/SKILL.md:312/315` has no `accelerator work` equivalent.

### The `accelerator work` CLI surface — nine subcommands, key gaps

Crate `cli/work-cli/`; `enum Command` at `cli/work-cli/src/cli.rs:19-90`; frozen
golden `cli/work-cli/tests/fixtures/cli_surface.golden`. Subcommands: `resolve`,
`template-hints`, `show`, `diff`, `create` (`--push`), `update` (`--push`),
`canonicalise-id`, `next-number`, `sync` (`--preview`, `--push-only`,
`--pull-only`, `--resolve`, `--max-pulls/--max-pushes`).

Repointing state differs per skill:
- **`create-work-item`** — already largely on the CLI (`resolve`, `show`,
  `next-number`, `create`); only remote push (`SKILL.md:502/532/542`) still
  uses bash → `work create --push`.
- **`list-work-items`** — uses `template-hints`, `canonicalise-id`; its
  **sync-status rendering** (`sync-label`, `sync-baseline` read, `sync-classify`,
  `fetch-remote search/show`) has ❌ **no CLI port**.
- **`sync-work-items`** — still bash-heavy; `work sync` is the umbrella
  replacement for the whole `sync-*`/`fetch-remote`/`project-remote` cluster,
  but as one command with flags, not composable primitives. `work diff` and
  `work next-number` are already used.

Invocation is always `${CLAUDE_PLUGIN_ROOT}/bin/accelerator work …` via the
launcher; dev override is `ACCELERATOR_WORK_BIN` (used unverified before fetch),
which the skills never reference directly.

### The eleven parity tests — list needs correcting

Nine of the eleven named tests exist and reference `skills/work/scripts`. Two
corrections:
- ❌ `cli/work-adapters/tests/project_remote_parity.rs` **does not exist**.
  Projection parity lives in `cli/remote-projection/tests/parity.rs:26`,
  `cli/jira-client/tests/projection_corpus.rs:25`,
  `cli/linear-client/tests/projection_corpus.rs:18` — three crates reaching the
  `work-item-project-remote` fixtures.
- Two script-coupled files the work item omits: `cli/work-adapters/tests/
  sync_baseline_shellout_parity.rs` (shells out to `work-item-normalise.sh` and
  `work-item-project-remote.sh` at `:77/:110`) and
  `cli/tracker-support/tests/mapper_differential.rs:58-59` (shells out to
  `work-item-create-remote.sh` / `-update-remote.sh`). 0171's D10 records the
  differential tests are "deleted by 0212 with the assets they drive".

Also in the tree-wide set: `cli/work-adapters/tests/bash_parity_baseline.rs`
(reads `fixtures/bash-parity-baseline.txt`) and `cli/remote-projection/tests/
corpus_hashes.rs:18`.

### Fixture corpus — 68 files, ten orphans

`skills/work/scripts/test-fixtures/` = **68 files** (14 loose goldens + four
case-directory trees, 54 files). Confirmed against 0210's committed baseline
number. Both target dirs already exist. The bash suites use inline heredoc data
and do **not** read the case-dir trees, so those corpora are already Rust-only.

Relocation, corrected against actual consumers:

| Corpus | Consumers | Proposed home |
|---|---|---|
| `sync-classify.json`, `sync-decide.golden`, `sync-label.golden`, `push-decide.golden`, `normalise/case-*` | `cli/work/tests/*` | `cli/work/tests/fixtures/` |
| `sync-baseline/case-*` | `work-adapters` (2 tests) + `remote-projection/corpus_hashes.rs` | `cli/work-adapters/tests/fixtures/` (majority; remote-projection reaches across) |
| `project-remote/case-*` | `remote-projection`, `jira-client`, `linear-client` | ⚠️ **`cli/remote-projection/tests/fixtures/`, not `work-adapters`** (work item's map is wrong — no work-adapters consumer) |
| `section-diff/case-*` | `work-adapters` + `work-cli` | `cli/work-adapters/tests/fixtures/` |

**Ten orphan goldens** with no runtime reader (bash tests them inline; Rust
carries its own inline oracles): `canonicalise-id`, `template-field-hints`,
`file-dirty`, `next-number`, `read-field`, `resolve-id`, `update-tags`, plus
three loose provenance-header goldens `normalise.golden`, `section-diff.golden`,
`project-remote.golden`. These are the delete-with-reason set.

Relocation convention: switch each reader from the reach-into-repo form
(`CARGO_MANIFEST_DIR/../..` + `skills/work/scripts/test-fixtures`) to the
in-crate form (`CARGO_MANIFEST_DIR/tests/fixtures`). The
`bash-parity-baseline.txt` guard (`bash_parity_baseline.rs:87`) reds the build
when the corpus set changes, so it must be updated in lockstep.

### `E_DISPATCH` taxonomy — split three ways; item-4 premise stale

`work-item-bridge-codes.sh:34-38` defines five `readonly` codes (70–74). Sourced
by four scripts (`update-remote`, `create-remote`, `push-decide`, `fetch-remote`).
The taxonomy is currently split **three** ways:
- bash constants — `work-item-bridge-codes.sh`;
- Rust integers — `cli/work-cli/src/exit_codes.rs:12-16` (the actual numeric
  owner, **not** the `tracker` crate);
- Rust class enum — `TrackerError` (`cli/tracker/src/lib.rs:144-179`, two
  classes: `Retryable`/`Terminal`).

Cross-pins: `cli/work-cli/tests/exit_codes_parity.rs:21` (bash↔integers,
textual) and `cli/tracker/tests/errors.rs:35` (fixture↔`TrackerError` classes,
via `dispatch-codes.txt`). Extra coupling the work item misses:
`cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt:16` references
the bash file.

⚠️ **Item-4 comments are already scrubbed.** `cli/tracker/src/errors.rs` does
**not exist** (module doc lives in `tests/errors.rs`). None of the four named
comments (`RemoteIssue.body` `lib.rs:111-131`, the errors module doc, `show`'s
`# Errors` `lib.rs:308-316`, `RemoteTimestamp::Reported` `lib.rs:52-61`) still
names a deleted bash artefact — they describe the concepts provider-neutrally.
Nothing to remove there; the live bash coupling is in `work-cli` and the
`tracker`/`tracker-support` fixtures.

### Build-system floor — narrower edit than "remove outright"

- `tasks/test/integration.py:51` — `_EXPECTED_WORK_SUITES = 5`; the `work` task
  (`:397-401`) calls the **shared** `_require_suite_floor` (`:81-107`) with
  `required=()`. That function backs six subtree floors (config 15,
  integrations 32, hooks 1, decisions 0, github 0) and must **not** be removed —
  only the `work` constant and the `work` task's call come out (or the constant
  drops to the new discovered count / 0). Suites are discovered by globbing
  executable `test-*.sh` under `skills/work` (`tasks/test/helpers.py:74-107`).
- `tasks/lint/scripts.py:34` — deleting `work-item-bridge-codes.sh` from disk
  trips the **stale-entry guard** (`:107-112`) unless line 34 is removed in the
  same change. `tests/unit/tasks/test_exec_bits.py:260` asserts it non-executable.
- No `tasks/` reference names the five deleted suites individually — discovery
  is glob-based.

### Three port-less bridge capabilities — all need new port ops 🔴

None has a Rust equivalent; each requires a **new `RemoteTracker` operation**:

| Capability | Bash | Rust today | 0171 fate |
|---|---|---|---|
| Unkeyed discovery `search` | `fetch-remote.sh:259-263/280-283`; used `sync SKILL.md:273-289` | `fetch_all(ids)` is key-scoped, "total over requested ids" (`tracker/src/lib.rs:338-341`) — cannot express it | `*open*` |
| Create `--dry-run` field preview | `create-remote.sh:116-144` (unresolvable project → 70 pre-gate) | `create` is create-only; no resolve/dry-run method anywhere | `*open*` |
| Update `--dry-run` payload validation | `update-remote.sh:143-155` (`--print-payload` into live flow) | `sync --preview` early-returns before the apply loop (`run.rs:405-420`), makes **no** port call | `*open*` |

The update case is the sharpest: 0194's `--preview` is verified to make no port
mutation call (`sync_run.rs:287-308`), exactly as the work item claims, so it
does not discharge the bridge's live payload validation.

## Code References

- `skills/work/scripts/` — all 18 target files; `test-fixtures/` (68 files)
- `cli/work-cli/src/cli.rs:19-90` — the nine-subcommand surface; `cli_surface.golden`
- `cli/work-cli/src/exit_codes.rs:12-24` — numeric `E_DISPATCH` owner + enum mapping
- `cli/work/src/sync/decide.rs:87-103` — dirty-gated pull decision (the guard)
- `cli/work-adapters/src/sync/run.rs:405-420` — preview early-return (no port call)
- `cli/work/src/normalise.rs` — normalise logic (library-only, no CLI/`--stdin`)
- `cli/tracker/src/lib.rs:144-179` — `TrackerError` two-class taxonomy
- `cli/tracker/tests/errors.rs:35` + `fixtures/dispatch-codes.txt` — class fixture pin
- `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt` — the 68-file/case/assertion baseline guard
- `cli/tracker-support/tests/mapper_differential.rs:58-59` — omitted script-coupled test
- `tasks/test/integration.py:51,81-107,397-401` — the work suite floor
- `tasks/lint/scripts.py:34` — `SHELL_LIBRARIES` entry + stale-entry guard

## Architecture Insights

- **`work sync` is an umbrella, not a toolbox.** The bash cluster was a set of
  composable primitives (`sync-classify`, `sync-decide`, `sync-baseline`,
  `fetch-remote`, `project-remote`, `file-dirty`, `normalise`); the Rust engine
  fuses them into one `sync` command whose stages are internal. Repointing is
  therefore a *behaviour* migration for `sync-work-items`/`list-work-items`, not
  a call-site swap — anywhere a skill used a primitive in isolation (list's
  status rendering, the normalise pipe), there is no CLI seam to target.
- **Parity tests pin bash↔Rust by construction.** The `class_of` derivation in
  `tracker/tests/errors.rs` and the textual read in `exit_codes_parity.rs` mean
  a rename propagates into assertions rather than silently passing — good, but it
  means the conversions must preserve the recorded fixture cases and goldens
  byte-for-byte (0210's `bash-parity-baseline.txt` is the frozen oracle).
- **The fixture home should follow the projection's crate identity**, not the
  work item's map: `project-remote` belongs with `remote-projection`, whose name
  the projection carries, not `work-adapters` (which does not consume it).

## Historical Context

- `meta/work/0171-jira-and-linear-integrations.md` — parent. `## Decisions`
  records the three capability fates as `*open*`; `## Open Questions` says five
  things "must be settled before pickup" but Drafting Notes (2026-08-17)
  overrode the five children to `ready` with the questions open, carrying them
  "into planning". EXIT_CODES.md siting and the contract-run route are also `*open*`.
- `meta/work/0210-…-tracker-port.md` (done) — commits `bash-parity-baseline.txt`
  (68-file count, per-test fixture-case ids + pre-conversion assertion counts),
  `bridge-exit-code-tables.txt`, `adf-node-types.txt`. Offline corpus half
  (byte-identity for 3 project-remote records, hash-after-normalise for 4
  sync-baseline records) is verifiable only while the bash corpus is on disk;
  online half ran against live tenants 2026-08-18.
- `meta/work/0211-…-bash-cluster-retirement.md` (ready; blocked by 0210+0212) —
  owns the whole-repo `jq`/`curl` empty-set equality assertion; 0212 owns only
  "no work skill declares them". Nine invocations run work→cluster (0212 deletes
  all nine callers); clusters never call work scripts — the one-directional
  coupling that fixes the order.
- `meta/reviews/work/0212-work-item-script-cutover-review-1.md` — prior review.

## Related Research

- `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- `meta/research/codebase/2026-08-06-0170-work-item-lifecycle-subdomain.md`

## Follow-up Research 2026-08-19 — Extra work to make 0212 achievable in one plan

Second research wave (six agents) scoping the net-new build so everything folds
into a single plan rather than deferring the port work to separate items. The
governing constraint and seven workstreams follow.

### Governance constraint (decide first)

0171's protocol says a capability needing a new port op is an **additive port
item parented to 0136**, and "re-site above the port" is not actually available
because all three need new surface. Folding the port additions into 0212 means
one of: (a) accept 0212 grows to include new port *behaviour* (contradicting
0171's "additive item" rule); or (b) file the additive port item and make this
single plan span both. Either is a planning decision — but the plan cannot be
"repointing only". 0204's frozen-port protocol is the thing being overridden.

### Workstream 1 — `search` port op (untracked discovery) — smaller than feared

Both clients already hold unkeyed filter composers with full flag-parity to the
bash flows; the machinery is not net-new, only the port surface and wiring.

- **New types, in the `tracker` crate itself** (🔒 pup limits `tracker` to
  `std/core/alloc/crate`): a provider-agnostic `SearchScope` input and a
  `Discovery { found: Vec<(ExternalId, RemoteTimestamp)>, complete: bool }`
  return (`FetchOutcome` can't be reused — it partitions over requested ids).
- **Jira** (`jira-client/src/jql.rs::compose` exists, test-wired only): adapt
  `fetch_chunk` (`client.rs:186-244`) to drive `compose` instead of `key_clause`
  and to **return a truncation flag instead of collapsing a cap-hit to `Err`**
  (it currently destroys the completeness signal).
- **Linear** (`filter.rs::compose` + `fetch_page` already run the team search in
  production): let `fetch_page` take a caller-populated `Search`; return the
  `(index, truncated)` the loop at `client.rs:376-394` already computes.

### Workstream 2 — `preview_create` port op — cheap, one AC nuance

- The bash `--dry-run` is **local only** (reads `fields.json`/`site.json`); it
  catches a config-*unset* project, not a config-set-but-nonexistent one.
- ⚠️ The AC demands surfacing "an unresolvable Jira **project key**". Meeting it
  literally needs the remote existence check via
  `jira-client/src/discovery.rs::discover_projects` (a `SurfaceError →
  TrackerError` shim, none exists today) — marginally more than bash did.
- **Linear** is trivial (`CreatePreview { project: None, issue_type: None }`).
- Recommend a new port method `preview_create(kind) -> CreatePreview`, not a
  flag on `create`.

### Workstream 3 — `validate_update` port op — cheapest

Both clients compose the payload inline then send; a validate-only mode is an
early-return before `transport.send` / `self.call` (`jira client.rs:366`,
`linear client.rs:318`), local only. Recommend a new method `validate_update(id,
title, body)`, extracting the small compose block into a shared helper.

### Workstream 4 — sync-engine orchestration — the real size, and the 0213 boundary

The Rust engine has **none** of `sync-work-items`' remote-discovery
orchestration. Building it is where 0212 stops being "repointing":

- **Gap A — untracked remote pull** (`fetch.rs:141-255`, `run.rs`): a new
  create-from-remote `Action` (the enum is `Push|Pull|Skip*|Noop` today,
  `decide.rs:24-28`), a remote-only field on `GatheredFacts` (`fetch.rs:53-60`,
  none exists), untracked-set computation (`search` results minus local
  `external_id`s), id allocation + `atomic_write` + baseline set, and the
  blast-radius gate. Consumes Workstream 1.
- **Gap B — unsynced push (create)**: the engine *drops* keyless items
  (`run.rs:219-225`) and never calls `create`; needs an `apply.rs` create path +
  the offer loop + `external_id` writeback.
- **Gap C — preview validation loop**: replace the `NotApplied`-mapping early
  return (`run.rs:183-197`) with a per-`Push` loop calling `validate_update`.
- **This subsumes the normalise pipe**: folding untracked-pull baseline
  recording into `work sync` (reusing `digest::remote_body`) deletes the
  `work-item-normalise.sh --stdin` pipe wholesale — so **no `work normalise`
  subcommand is needed** (a bare one would just re-expose the drift risk
  `digest.rs` exists to remove).
- ⚠️ **Boundary**: the interactive pull-overwrite prompt (Gap D) and the
  interactive conflict-resolution loop (Gap E) are **0213's** territory, which
  shares this SKILL.md. The plan must build the non-interactive engine seams
  0213 then hangs prompts on — not the prompts themselves — or the two children
  collide.

### Workstream 5 — `list-work-items` rendering — build one command or drop

Label vocabulary, classifier, baseline reads, and bulk fetch are **already
ported** (`work/src/sync/label.rs`, `decide.rs`, `digest.rs`,
`work-adapters/.../baseline_store.rs`, `fetch.rs`). Options:
- **Build**: a single `work list` command (scan + filter + render) reusing those
  modules — collapses all four script surfaces into one command.
- **Cheapest partial**: expose the label alone (`label.rs:60-68`), keep
  presence-only synced/unsynced, drop the four baseline-dependent states.
- **Drop**: `list-work-items` is declared a read-only filesystem-discovery skill;
  the status column is an explicit conditional add-on and removing it is coherent.

### Workstream 6 — EXIT_CODES.md → fold and delete the directory

`cli/work-cli/src/exit_codes.rs` is already "the whole binary's exit-code
taxonomy in one place" and a superset of the .md (adds 0–5). The .md's only
non-`meta/` consumer is `work-item-create-remote.sh:34`, itself deleted.
Recommend **option (b)**: move the human table into `exit_codes.rs`'s module doc
(or a docs-site page), delete `EXIT_CODES.md` and the now-empty
`skills/work/scripts/`. Regardless of branch, `exit_codes_parity.rs` must be
repointed off the deleted `work-item-bridge-codes.sh` oracle — pin it to the new
constants/doc as cheap insurance.

### Workstream 7 — contract-run route + corpus seeding

- The harness (`mise run test:integration:tracker-contract`,
  `tasks/test/integration.py:164-170`) is **manual, developer-run, no CI job**;
  an unconfigured run *fails* (three gates: nextest `contract` profile,
  `ACCELERATOR_TRACKER_CONTRACT=1`, credential resolution — none skip).
- **CI-gating adds**: repository secrets, a dedicated secret-holding job (not a
  leg of the OS-parallel `test:integration` roll-up), and network/rate-limit
  exposure as a non-defect failure mode. A manual run adds nothing to CI.
- ⚠️ **New machinery**: the corpus criterion needs a **write/seed step** creating
  one remote issue per relocated corpus record — the contract harness only reads
  two unmatched ids, never creates. Reuses the production token vars
  (`ACCELERATOR_{JIRA,LINEAR}_TOKEN`) + a scratch project/team; no new var names.

### Enforcement ripple per port method (applies to WS 1–3)

- Six `impl RemoteTracker` sites (2 real + `RecordingTracker` +
  `work-adapters/tests/sync_apply.rs::Fake` + `tracker/tests/port.rs::FixedTracker`
  + the trait def) — no default bodies, so all must implement each new method.
- Regenerate **`cli/tracker/tests/fixtures/public-api.txt`** (only `tracker` is
  pinned; clients/adapters exempt).
- Add a contract `*_property` + gate wiring in `tracker-test-support/src/
  contract.rs` (and bump `run_all`'s hard-coded count) per method.
- A `work list` subcommand trips only the `cli_surface.golden` freeze
  (`cli/work-cli/tests/cli_surface.rs:12-22`), not the thirteen-point checklist.

### Sizing

This is no longer a story — it is closer to an epic-sized effort: three port ops
(× six-impl ripple), a substantial sync-engine extension (untracked pull +
create offer + preview validation), a `work list` command or a documented drop,
a docs fold, and a credentialed seed harness. The plan should decompose into
mergeable phases and draw the 0213 boundary explicitly (build seams, not
prompts).

## Open Questions

- 🔴 **Governance**: absorb the three port ops into 0212 (overriding 0171's
  "additive port item" rule) or file the additive item and span both in one
  plan? The plan's shape depends on this.
- ❓ `list-work-items`: build a `work list` command, expose the label only
  (presence-only), or drop status rendering? All three are coherent.
- ❓ `preview_create`: match bash (local, config-unset only) or meet the AC's
  "unresolvable project key" literally (remote `discover_projects` check)?
- ❓ Contract-run route: CI-gated (secrets + dedicated job) or manual pre-merge?
  `*open*` in 0171; gates the corpus seed criterion.
- ❓ EXIT_CODES.md: confirmed recommendation is fold-and-delete (option b) — needs
  sign-off since 0171 records it `*open*`.
