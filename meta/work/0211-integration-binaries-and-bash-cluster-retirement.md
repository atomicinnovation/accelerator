---
type: work-item
id: "0211"
title: "Integration Binaries and Bash Cluster Retirement"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: done
kind: story
priority: medium
parent: "work-item:0171"
blocked_by: ["work-item:0210", "work-item:0212"]
blocks: ["work-item:0174"]
relates_to: ["work-item:0165", "work-item:0203", "codebase-research:2026-08-17-0211-integration-binaries-and-bash-cluster-retirement"]
tags: [rust, jira, linear, integrations, cli, cutover]
last_updated: "2026-08-17T13:33:15+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-741
---

# 0211: Integration Binaries and Bash Cluster Retirement

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Ship `accelerator-jira` and `accelerator-linear` as thin inbound CLI adapters
over 0210's client crates, repoint the jira and linear skill bodies at them, then
delete both bash script clusters, their suites and their Python mock servers.
Register the two dispatch tokens end to end, including the release manifest.

The provider surface is **larger than the eight flows this item was sized
against** — see Assumptions, where the size-bounding assumption is now recorded
as refuted rather than unconfirmed.

## Context

Last of the three sequenced children of 0171, ordered **after** the work-item
cutover (0212). The clusters this child deletes carry the shared test assets four
of 0212's suites consume — `test-helpers/` holding the Python mock servers, and
`test-fixtures/` holding their scenario fixtures — so those suites must be gone
before the clusters can be retired whole.

The coupling is one-directional **in the sense that matters**: verified by grep,
the clusters invoke no `work-item-*.sh`, and the two references to
`work-item-sync-label.sh` in `linear-create-flow.sh:304` and
`jira-resolve-fields.sh:140` are comments about shared normalisation, not calls.
Landing this child first would break `test-work-item-{create,update,fetch}-remote.sh`
and `-sync-apply.sh` along with the `_EXPECTED_WORK_SUITES` floor.

The **reverse** direction is live production code, not merely test assets, and
strengthens the same ordering. Nine invocations run from `skills/work/scripts/`
into both clusters:

```
work-item-create-remote.sh:199   → linear-create-flow.sh
work-item-fetch-remote.sh:120    → jira-search-flow.sh
work-item-fetch-remote.sh:161    → linear-search-flow.sh
work-item-fetch-remote.sh:258    → jira-search-flow.sh
work-item-fetch-remote.sh:265    → jira-show-flow.sh
work-item-fetch-remote.sh:279    → linear-search-flow.sh
work-item-fetch-remote.sh:285    → linear-show-flow.sh
work-item-update-remote.sh:143   → jira-update-flow.sh
work-item-update-remote.sh:154   → linear-update-flow.sh
```

0212 deletes all nine callers. Until it does, four Linear flows and three Jira
flows serve a *machine* contract as well as a skill contract.

### Cluster inventory, corrected

The counts this item previously carried double-counted the libraries. The real
figures, measured at revision `5e8e8677`:

| Provider | Non-test `.sh` | Executable | Sourced-only libraries | Data assets | Lines |
|---|---|---|---|---|---|
| Jira | 22 | 17 | 5 | 3 (`.jq`/`.awk`) | 5,082 + 455 |
| Linear | 12 | 10 | 2 | — | 2,912 |

So "22 production scripts **plus** five libraries" was wrong: it is 22 files
*including* them, of which 17 are executable. Likewise Linear is 12 including
`linear-common` and `linear-auth`, of which 10 are executable. The
flow-coverage criterion's denominator is corrected accordingly.

Total deletion set (measured at implementation) is **263 files / 21,422 lines** —
production `.sh` 34/7,994, data assets 5/746, suites 33/9,204, `test-fixtures/`
188/3,079, `test-helpers/` 3/399 — not the ~17,650 lines first estimated.

## Requirements

### Binaries and flow surface

- Implement `accelerator-jira` and `accelerator-linear` as thin inbound CLI
  adapters over 0210's client crates, covering **the whole executable surface of
  each cluster, not merely the eight named flows**. Per Assumptions, that is 17
  Jira executables spanning **21 dispatch modes** and 10 Linear executables
  spanning **6 dispatch modes** (most linear executables are flag-and-positional
  only), not the ~25/~15 first estimated.
  Where a script becomes a flag or a projection of another subcommand rather
  than a peer subcommand, record that mapping — it is the flow-coverage
  criterion's evidence.
- Decide and record the fate of the four entrypoints that are user-facing today
  but sit outside the eight flows. Each is a decision, not a mechanical port:
  - `jira-emit-key.sh` — a projection of `create` (`create --emit key`), not a
    verb, but it carries a distinct post-create non-retryable semantic (exit 16).
  - `jira-resolve-fields.sh` — makes **no provider API call**. It parses
    work-item frontmatter and shells to `accelerator config`, so it arguably
    belongs in `accelerator work` rather than `accelerator jira`.
  - `jira-auth-cli.sh` and `linear-auth-cli.sh` — both print the credential in
    cleartext on stdout. Reproducing that as a subcommand re-opens a security
    decision; the `--debug` masking discipline already in both scripts shows the
    original authors knew.
  - `jira-jql-cli.sh` is an **orphan** — executable, arg-parsing, invoked only by
    its own test. Porting it would create user-facing surface that does not
    exist today. The default is to drop it.
- Treat `init` as the multi-operation surface it is: `jira-init-flow.sh` takes
  `verify`, `discover`, `prompt-default`, `refresh-fields`, `list-projects` and
  `list-fields` plus a bare interactive full-flow that blocks on `read -r`
  (`jira-init-flow.sh:191`); `linear-init-flow.sh` takes `verify`, `list-teams`
  and `discover` plus a bare mode. State the TTY policy for the bare mode — the
  existing `--non-interactive` flag only half-covers it. The TTY-refusal
  obligation is **Jira-only**: `linear-init-flow.sh`'s bare mode does not block
  on `read`; it prints the team list and returns a "re-run with `--team-id`"
  instruction, so only `jira init` needs the no-TTY refusal.

### Exit-code contract

- Define and document the exit-code contract for the two binaries. Enumerate the
  failure classes — at minimum auth-invalid, target-not-found, payload-rejected,
  transport-failure and usage-error — and **anchor each integer externally rather
  than choosing it**: it equals the value the retiring bash flow returned for the
  same condition, captured pre-deletion and committed, or `tracker`'s existing
  `E_DISPATCH_*` value where that is genuinely the same taxonomy. Name the
  document of record (the CLI's own exit-code documentation). Choosing the
  integers freely and then writing them down would make the mapping
  unfalsifiable, and the repointed skill bodies branch on these values.
- ⚠️ **`70`–`74` are reserved for the dispatch meaning** at any surface
  `accelerator-work` consumes (corrected from `70`–`73`: 0212 added
  `E_DISPATCH_UNCONFIGURED` = 74). The dispatch band defines
  `E_DISPATCH_RETRYABLE/TERMINAL/NOT_AVAILABLE/UNRECOGNISED/UNCONFIGURED` on
  exactly the integers both search flows use for their own errors (Jira `70-73`
  is `E_SEARCH_BAD_PAGE_TOKEN/BAD_LIMIT/NO_SITE_CACHE/BAD_FLAG`; Linear `70-73`
  is `E_SEARCH_BAD_FLAG/BAD_LIMIT/NO_CATALOGUE/BAD_STATE`). An unmapped
  search-flow code reaching the caller reads as a dispatch verdict, so the search
  codes are remapped off the whole `70`–`74` band (to `75`–`78`) and a test
  proves no binary emits `70`–`74`.
- Reproduce, or deliberately diverge from and record, three cross-provider
  collisions the two tables already carry:

  | Code | Jira | Linear | Nature |
  |---|---|---|---|
  | `82` | `E_SHOW_BAD_FLAG` (usage) | `E_SHOW_NOT_FOUND` | opposite classes |
  | `81` | `E_SHOW_BAD_COMMENTS_LIMIT` | `E_SHOW_BAD_FLAG` | shifted by one |
  | `34` | HTTP 400, **retryable** | 400 *and* 200-with-`errors[]`, **terminal** | opposite retry semantics |

  Rate limiting has no shared number either — Jira `19`, Linear `35`, each
  reserved-but-unused in the other.
- Account for the **asymmetry in what the skills actually observe**. Jira skills
  branch on roughly forty-five distinct integers; the Linear skills cite exactly
  one (`107`, at `create-linear-issue/SKILL.md:92`) and are otherwise symbolic
  `E_*` names. So the byte-anchoring obligation binds hard on Jira; for Linear
  the **symbolic names must survive in stderr**, which is the equivalent
  contract.
- Pick one of the repository's two working enforcement models and say which.
  Only these hold a table honest: Linear's derived-doc grep
  (`test-linear-paths.sh:71-103` asserts each `readonly E_*=NN` appears at the
  same value in `EXIT_CODES.md`), or `accelerator-work`'s
  `src/exit_codes.rs` plus the textual parity test at
  `cli/work-cli/tests/exit_codes_parity.rs`. ⚠️ Jira's table is held by neither
  and is **already wrong** — `EXIT_CODES.md:12` says usage errors exit `2` while
  `jira-request.sh:207` exits `1`. Capture the code's behaviour, not the doc's
  claim.
- Preserve or deliberately unify the credential-resolution divergence: Jira
  flattens every `jira_resolve_credentials` failure to `22`
  (`jira-request.sh:215-222`, discarding 24/25/27/28/29) while Linear preserves
  `25`, `27` and `29` (`linear-graphql.sh:481-489`). A single client cannot
  reproduce both without an explicit per-provider rule.

### Goldens and fixtures

- Capture, before deleting each bash flow, a golden of its **stdout** as well as
  its request and response. The strictest stdout contracts, which a golden must
  preserve exactly, are `show-linear-issue/SKILL.md:54-62` (six explicit
  `.data.issue.*` paths), `create-jira-issue/SKILL.md:65-66` (a **tab-separated
  four-field positional line**), the two bare-identifier flows, and the three
  skills that gate on *empty* stdout as a failure signal
  (`create-jira-issue:183`, `update-jira-issue:82`, `comment-jira-issue:98`).
- **No `!`-preprocessor clause is needed.** All 32 `!` blocks across the sixteen
  cluster `SKILL.md` files are `bin/accelerator config context|instructions
  … --fail-safe`; no cluster script runs at prompt-assembly time and no tracker
  output is ever interpolated into prompt text. Every cluster invocation is an
  execution-time instruction inside a fenced block.
- Record the **provenance** of every captured fixture in 0171's `## Decisions`:
  per flow, whether it was captured against the credentialed target or the
  retiring Python mock server, and for each mock-served entry the blocker that
  prevented a real capture.
- Confirm the shared test assets have no surviving consumer before deleting them.
  Re-run the sweep at this child's boundary and record the grep and its output.
  At revision `5e8e8677` the only consumers outside the clusters are the four
  0212 suites plus two Python config files (below). ⚠️ Note that
  `skills/integrations/jira/scripts/test-fixtures/api-responses/` — all ten files
  — already has **zero consumers** and is dead weight today.

### Registration

- Register both dispatch tokens per
  `tasks/README.md#registering-a-dispatched-sub-binary`. Corrected: the two
  `*-cli` composition-root crates carry **no** `cli/pup.ron` rule and **no**
  public-API snapshot — they are classified as composition roots in
  `tasks/public_api.py`'s `_EXEMPT_MEMBERS`; the client crates (`jira-client`,
  `linear-client`) carry the pup rules from 0210. Add both binaries to the
  per-platform release upload set and the minisign-signed `manifest.json` 0165
  owns. If 0210's copyleft check fired, 0203's attribution artefact is a
  release-path dependency.
- ⚠️ **Token registration and skill repointing must land in the same commit.**
  `tasks/shared/dispatch_coherence.py` binds both directions: a registered token
  with no consuming skill fails at `:196-202`, and a skill invoking an
  unregistered token fails at `:210-216`. `tasks/README.md:537` independently
  requires checklist points 1, 2, 3, 4, 7 and 8 to land together.
- ⚠️ **Only six of the sixteen cluster skills can witness a token.** The ten
  write skills (`create-*`, `update-*`, `comment-*`, `transition-*`, `attach-*`)
  declare bare `Bash`, which disqualifies the whole skill as a witness
  (`dispatch_coherence.py:57-65`). The six read/init skills use path-scoped
  rules and are the only candidates. Narrowing the write skills is a decision:
  the bare grant currently covers `wc -c` at `attach-jira-issue/SKILL.md:70` and
  a `source` of `config-common.sh` at `create-jira-issue/SKILL.md:113`, both of
  which need a replacement if it goes.
- The release **upload set and manifest need no new list** — both are derived
  from `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py:29-37`) by
  `tasks/signing.py:50-57`, `tasks/manifest.py:81-108` and
  `tasks/github.py:219-248`. Registration is a registry edit. Both tokens clear
  every constraint: they match `^[a-z][a-z0-9-]*$`, are not reserved
  (`{verify, launcher}`) and shadow no built-in (`{version, config, help}`).
- Name the new crates `jira`/`linear`, never `tracker-adapters` —
  `cli/tracker/tests/structure.rs:68-77` asserts the workspace contains no such
  crate, with the comment "provider clients live in their own crates".

### Repointing and deletion

- Repoint the **bodies** of every jira and linear `SKILL.md` at
  `accelerator jira …` / `accelerator linear …`, not merely their frontmatter,
  and remove the `jq`/`curl` `allowed-tools` entries.
- Delete both clusters whole: all production scripts, all seven library entries,
  the bash integration suites, and `mock-jira-server.py` /
  `mock-linear-server.py`.
- Enumerate every skill whose frontmatter declares `jq` or `curl` in
  `allowed-tools` after the change, and record the enumeration. ⚠️ **The expected
  result is the empty set**, not the previously-stated "skills still backed by
  repo-root `scripts/*.sh`". Measured at revision `5e8e8677`, exactly six skills
  declare either token and all six are the jira and linear read/init skills; no
  skill anywhere carries a `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` grant, though
  thirty such scripts still exist. Every occurrence is the bare parenthesised
  form `Bash(jq)` / `Bash(curl)` — there is no `Bash(jq:*)` variant.
- Retire `_EXPECTED_INTEGRATIONS_SUITES = 32` (`tasks/test/integration.py:57`)
  outright rather than decrementing it, and remove the seven jira and linear
  library entries from `SHELL_LIBRARIES` (`tasks/lint/scripts.py:18`).
  ⚠️ **`linear-graphql.sh` is now classified: a production script, so the
  expected removal count is seven.** It is `0755`, has zero `source`rs
  repo-wide, has eight out-of-process call sites (`bash "$DIR/linear-graphql.sh"`
  from every Linear flow), lacks any `BASH_SOURCE`/`$0` guard around its
  top-level dispatch, and is absent from `SHELL_LIBRARIES`. The two senses of
  "library" must still not be conflated in the record.

### Build-system guards this child must edit

Three tripwires fire on deletion and none was previously named here. Each is a
hard failure, not a degradation.

- **The stale-entry guard.** `tasks/lint/scripts.py:102-112` compares every
  `SHELL_LIBRARIES` member against `shell_sources()` and emits one offender per
  orphan, exiting 1 — seven offenders if the frozenset is not edited. Pinned
  twice more by `tests/unit/tasks/test_exec_bits.py:279-282` (set equality
  against `_RECONCILED_LIBRARIES`, which duplicates all seven members at
  `:261-267`) and `:284-287`.
- **The mock-server existence assertion.**
  `tests/unit/tasks/test_python_coverage.py:102-113` asserts both mock servers
  are present *in the walked repo tree*, so deleting them fails `mise run
  test:unit` regardless of what is done to `pyproject.toml`. And
  `test_ruff_extend_exclude_is_exactly_justified` (`:85-88`) uses set **equality**
  against `pyproject.toml:77-81`. This is a coordinated three-way edit: delete
  the files, strip the two `extend-exclude` entries, remove `MOCK_JIRA`/
  `MOCK_LINEAR` and all six usages.
- **The dual-use exemplar.** `_DUAL_USE_SCRIPTS`
  (`tests/unit/tasks/test_exec_bits.py:274-275`) has exactly one member,
  `jira-fields.sh` — sourced by `jira-init-flow.sh:32` and path-invoked from four
  flows. `test_dual_use_scripts_are_entrypoints` `os.access`es it and fails on a
  deleted file; an empty tuple makes the test vacuous. `tasks/README.md:90-94`
  documents the whole dual-use classification through this script. Either find a
  surviving exemplar or retire both the test and the prose deliberately.

Removing the integrations floor pulls in four further edits: the `integrations`
task (`tasks/test/integration.py:405-410`), the `_GUARDED` entry at
`tests/unit/tasks/test_integration.py:69` (an `AttributeError` otherwise),
`mise.toml:350-353` and `:369`, and `tests/unit/tasks/test_mise.py:51`, whose
partition assertion is exact.

### Explicitly out of scope — confirmed as no-ops

Recorded so the plan does not spend effort on them:

- **The sixteen generated docs-site pages.**
  `docs-site/src/content/docs/reference/skills/` is gitignored (`.gitignore:26`)
  and untracked; `mise run docs:generate` rebuilds it from `SKILL.md` and
  `docs:generate:check` depends on that task. Nothing to commit.
- **`EXPECTED_INJECTION_SKILLS = 42`** (`tasks/lint/skill_permissions.py:48`) is
  unchanged. All sixteen cluster skills inject both `config context --skill` and
  `config instructions`, but the skills survive and only their bodies are
  repointed.
- **`.claude-plugin/plugin.json:16-17`** keeps both skill directories registered;
  only their `scripts/` subdirectories go.

## Acceptance Criteria

- [ ] For each migrated flow, a mock-backed test asserts the outgoing request
      (method, path or GraphQL document, and body) and the parsed response
      against a fixture captured from the bash flow before deletion. Where a flow
      issues no single provider request — `init verify` if it only validates
      credentials and writes config — the criterion is replaced by a named
      observable outcome; for `attach`, the fixture file and expected transport
      shape (multipart or link-based) are named. Every fixture's provenance is
      recorded in 0171's `## Decisions` as credentialed-target or mock-served,
      each mock-served entry naming the blocker.
- [ ] Every subcommand's stdout is asserted byte-for-byte against a golden
      captured from the corresponding bash flow. Where the shape deliberately
      changed, that branch commits a golden of the **new** stdout plus a recorded
      diff against the bash golden, and a CLI-level test asserts the specific
      tokens the repointed `SKILL.md` body instructs the model to read.
- [ ] Every enumerated failure class maps to an integer **equal to the value the
      retiring bash flow returned for the same condition** — captured
      pre-deletion and committed as a fixture, so the comparison survives the
      scripts' removal — or to `tracker`'s existing `E_DISPATCH_*` value.
      Asserted table-driven at the CLI level, in `accelerator-jira` and
      `accelerator-linear`, the layer the repointed skills invoke. The document
      of record states the same mapping. Where a bash code was captured and
      deliberately not reproduced, the divergence is recorded with the test that
      would fail if it regressed.
- [ ] Neither binary emits `70`, `71`, `72` or `73` for any provider-level
      condition, asserted by a test over the whole subcommand set. The three
      recorded cross-provider collisions (`81`, `82`, `34`) each resolve to a
      stated per-provider behaviour with a test.
- [ ] Every Jira and Linear executable maps either to a named subcommand of its
      binary, to a recorded flag or projection of one, or to a recorded
      internal-helper or dropped classification — with the mapping committed and
      its count reconciling to the **corrected** pre-deletion file list (17 Jira
      executables + 5 libraries + 3 data assets; 10 Linear executables + 2
      libraries). A classification of "internal helper" must name the subcommand
      whose implementation subsumes it, so the criterion cannot be satisfied by
      fiat.
- [ ] The shared-asset sweep is recorded and its residual set is empty: grepping
      the repository for the four cluster `test-helpers`/`test-fixtures` paths
      plus `mock-jira-server` and `mock-linear-server`, under the **declared
      exclusion list** — `meta/`, `CHANGELOG.md` (immutable release record),
      `skills/work/create-work-item/evals/benchmark.json` (frozen eval
      transcript) and `docs-site/src/content/docs/reference/skills/` (gitignored
      generated mirror) — returns nothing outside the deleted clusters. The grep
      command, its exclusions and its output are the recorded result.
- [ ] `ls skills/integrations/jira/scripts/*.sh` and
      `ls skills/integrations/linear/scripts/*.sh` each match nothing (or both
      directories are absent); `mock-jira-server.py` and
      `mock-linear-server.py` do not exist; and every jira and linear
      `SKILL.md` body invokes `accelerator jira …` or `accelerator linear …`.
- [ ] The set of `skills/` frontmatter entries declaring `jq` or `curl` in
      `allowed-tools` after this change is **empty**. The enumerated result is
      recorded in 0171's `## Decisions`. (This child lands last, so it is the one
      that can assert the whole-repository equality; 0212 asserted only that no
      work skill declares them — a condition already true before 0212 began.)
- [ ] Both dispatch tokens are registered per the sub-binary checklist **in the
      same commit as the skill repointing**, both binaries appear in the
      per-platform upload set and the signed `manifest.json`, and the two `*-cli`
      composition-root crates are classified in `tasks/public_api.py` (no pup
      rule, no public-API snapshot; the client crates carry the pup rules). At
      least one witness skill
      per token declares a `Bash(...)` rule whose subcommand segment is exactly
      that token, in a skill carrying no bare `Bash`.
- [ ] `mise run build-system:check` is green, which exercises
      `lint:dispatch-coherence:check` in both directions.
- [ ] `_EXPECTED_INTEGRATIONS_SUITES` is removed from
      `tasks/test/integration.py` along with the `integrations` task, its
      `mise.toml` leaf, its `_GUARDED` entry and its `test_mise.py` partition
      member; the **seven** jira and linear entries are gone from
      `SHELL_LIBRARIES` and from `_RECONCILED_LIBRARIES`;
      `linear-graphql.sh`'s production-script classification is recorded in
      0171's `## Decisions`; and `lint:scripts:exec-bits:check` is green.
- [ ] The three build-system tripwires are discharged: no stale
      `SHELL_LIBRARIES` entry remains; `pyproject.toml`'s `extend-exclude` and
      `test_python_coverage.py`'s `RUFF_JUSTIFIED_EXCLUDES` agree and neither
      names a deleted file; and `_DUAL_USE_SCRIPTS` either names a surviving
      dual-use script or has been retired together with its test and the
      `tasks/README.md:90-94` prose.
- [ ] No Python remains in the `cli/` test lane: the mock servers are gone and
      neither client crate's dev-dependencies nor `tasks/` reference them.
- [ ] `mise run` exits 0 end-to-end at this child's merge boundary, not only
      after the whole of 0171.

## Dependencies

- Blocked by 0210: these binaries are thin adapters over its client crates, and
  the per-flow fixtures must be captured while the bash flows still exist.
- Blocked by 0212. The clusters carry `test-helpers/` and `test-fixtures/`, which
  four of 0212's suites consume. 0212 also removes the nine live production call
  sites from `skills/work/scripts/` into both clusters listed in Context.
- **Conditionally requires the credentialed tracker target.** Response shapes
  should be captured from the real Jira project and Linear team wherever
  reachable. Where the target is unreachable for a flow, the mock is acceptable
  and the blocker is recorded. The target has no work item; 0171 names the owner.
- **Blocks 0174**, as the last of the three: 0174 cannot remove the
  `SHELL_LIBRARIES` frozenset or the exec-bit guard while this child's seven
  orphaned entries and the live `_EXPECTED_INTEGRATIONS_SUITES` floor still
  exist.
- 0165 owns the signed `manifest.json` and upload set both binaries join. It is
  **implemented and working** at this revision — the manifest is generated,
  minisign-signed, uploaded and re-verified before publish, and the launcher
  parses it; there is no flat `checksums.json` left in `tasks/`. Because the
  upload set is derived from `DISPATCHED_SUBBINARIES`, this child adds no
  pipeline code. A partial registration fails clean at `tasks/signing.py:73-77`,
  before the manifest is emitted and before commit/tag/push. The genuinely
  uncaught items are the two `.gitignore` entries (checklist point 5) and the
  user docs (point 11).
- 0203 becomes a release-path dependency if 0210's copyleft check fired. **The
  trigger is explicit**: whoever records a copyleft component adds
  `work-item:0203` to this child's `blocked_by` and `work-item:0211` to 0203's
  `blocks`.
- 0174 owns the fourteen repo-root `scripts/*.sh` `SHELL_LIBRARIES` entries.
- **External systems**: Jira REST and Linear GraphQL, for any fixture captured
  against the credentialed target. Reachability, rate limits and Linear's
  query-complexity cap bear on those captures.
- 0194's artefacts are **confirmed present** at this revision, discharging 0171's
  instruction to verify them rather than trust a status field:
  `cli/tracker-test-support/src/contract.rs` exists, `accelerator work sync` is
  wired at `cli/work-cli/src/cli.rs:88`, and the baseline corpus at
  `skills/work/scripts/test-fixtures/` holds 18 entries.
- Parent: 0171.

## Assumptions

- ⚠️ **REFUTED, 2026-08-17.** The assumption that "the eight enumerated flows
  plus ADF↔markdown, JQL and GraphQL construction are the whole user-facing
  surface of both clusters" does not hold. Measured surface:

  - **Jira**: eleven `SKILL.md`-reachable entrypoints, not eight — the eight
    flows plus `jira-auth-cli.sh` (`init-jira/SKILL.md:83`),
    `jira-resolve-fields.sh` (`create-jira-issue/SKILL.md:61`) and
    `jira-emit-key.sh` (`create-jira-issue/SKILL.md:105`). Seventeen executables
    in total, spanning **21 dispatch modes** (not the ~25 first estimated) once
    `comment`'s four subcommands, `init`'s six, `fields`' three and
    `resolve-fields`' modes are counted.
  - **Linear**: ten executables — the eight flows plus `linear-auth-cli.sh`
    (`init-linear/SKILL.md:38`) and `linear-graphql.sh` (internal, no `SKILL.md`
    caller, but reachable via the wildcard `allowed-tools` glob) — spanning only
    **6 dispatch modes** (nine of the ten executables are flag-and-positional
    only), not the ~15 first estimated.

  This child is correspondingly larger than its sizing allowed. Per 0171's
  drafting notes, a short flow list is the trigger for revisiting the declined
  **provider-seam split** — separating jira and linear into sibling children.
  That should be reconsidered before planning commits to a shape.
- The three `.jq`/`.awk` assets map onto no subcommand. The `\x1f`/`\x1e` binary
  record stream between `jira-md-tokenise.awk` and `jira-md-assemble.jq` is a
  bash-pipeline artefact with no public contract; it disappears in Rust. This was
  true of the **product** surface but **false of the test surface** until Phase 0:
  `cli/jira-client`'s ADF differential shelled to the two driver scripts (which
  pull in the `.jq`/`.awk`), so all three were load-bearing for a live Rust test.
  Phase 0 froze that differential to a committed oracle before the deletion.
- The committed goldens are a sufficient oracle once their bash generators are
  gone.
- Corrected: the client crates' read surface is **not** complete for search. The
  port `search` op returns a stamps-only `Discovery` (external ids + timestamps),
  which cannot render the State/Assignee/Status columns the search bodies show
  nor jira's `--page-token` cursor. Each client therefore gains an **additive
  read-side projection op** over a distinct search query (Decision 20), bound by
  the search subcommand — not the port `Discovery`.
- Corrected: the `init` caches are **read-compatible** with bash-era state and
  **fail closed** on an unrecognisable one (Decision 21). Bash-era `site.json`/
  `fields.json` carry no version envelope, so an absent marker is the implicit
  bash-era version and reads unchanged; a present-but-unrecognised marker or an
  unparseable shape fails closed in the client crate's cache-read path, before
  any live-tenant mutation.

## Open Questions

- **Does the provider-seam split now apply?** The refuted assumption is the
  condition 0171 named for revisiting it.
- **Which exit-code enforcement model** — Linear's derived-doc grep or
  `work-cli`'s `exit_codes.rs` plus textual parity test. Jira's current table is
  held by neither and already contradicts its code.
- **Do the ten bare-`Bash` write skills get narrowed?** They cannot witness a
  token, and the bare grant currently covers a `wc -c` call and a `source` of
  `config-common.sh`.
- **`wiremock` versus the in-repo `MockServer`.** 0210 mandates `wiremock-rs`,
  which appears nowhere in the workspace; the established pattern is a
  purpose-built std-only `MockServer` per consumer, with the decision against
  `wiremock`/`mockito` stated explicitly at `cli/github/tests/common/mod.rs:5-10`
  and `cli/collaboration-cli/tests/common/mod.rs:5-7`. 0210's call, but this
  child inherits the harness — and the `cli/github/` copy would need request-body
  capture and a per-route response queue to serve this surface.
- Carried from 0171 and still open: the credentialed target's secrets siting,
  the fate of the three port-less bridge capabilities, and `EXIT_CODES.md`
  siting.

## Drafting Notes

- Enriched 2026-08-17 from
  `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`,
  measured at revision `5e8e8677`. `producer` left as `review-work-item`: it
  records where the item came from, and this pass enriched it rather than created
  it.
- ⚠️ **0171's drafting notes are stale on two points.** They list "0211's
  mock-server deferral" and "0211's `jq`/`curl` survivor set" as live
  self-contradictions carried into planning. The 0211 review's **Correction**
  section post-dates its Acceptance and resolves both — the deferral branch was
  removed entirely when the ordering was reversed, and 0211 now owns the
  whole-repository equality assertion while 0212 asserts only the work-skill
  half. The `jq`/`curl` half is further sharpened here: the survivor set is
  empty.
- Three 0171 `## Decisions` entries marked *pending (0211)* are now answerable
  and should be recorded there: `linear-graphql.sh`'s classification (production
  script, count seven), the cross-skill `jq`/`curl` audit result (six declarers,
  all in-cluster, expected post-change set empty), and the reverse cross-cluster
  sweep (two comments only; the live coupling runs work → clusters and is 0212's
  to remove).
- Two findings the parent review accepted unresolved remain unaddressed by this
  pass, because neither is this child's to close: the non-port provider surface
  (five of eight flows) owned by neither 0210 nor 0211, and 0210 carrying no
  criterion for HTTP-status or GraphQL error classification.
- The 0167 config-cluster retirement is the closest precedent and produced the
  artefact set this child should mirror: `meta/inventories/0167-removal-set.md`,
  `-suite-audit.md` and `-divergences.md`. The divergences file's governing rule
  transfers verbatim — *"A divergence nothing can detect is indistinguishable
  from a defect, so every row names a real, passing test."* Its divergence #4,
  pinning "usage errors exit 1, not clap's 2" with a named black-box CLI test, is
  the external anchor shape this child's exit-code criterion needs. Its
  validation also records that a mixed bash/`accelerator` state is safe on `main`
  until the release that carries the flip, which bears on the review's unresolved
  objection to bundling registration into a cutover child.

## References

- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Blocked by: `meta/work/0210-provider-client-crates-over-the-tracker-port.md`,
  `meta/work/0212-work-item-script-cutover.md`
- Research: `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
- Review: `meta/reviews/work/0211-integration-binaries-and-bash-cluster-retirement-review-1.md`
- Precedent: `meta/inventories/0167-removal-set.md`,
  `meta/inventories/0167-suite-audit.md`,
  `meta/inventories/0167-divergences.md`,
  `meta/validations/2026-07-19-0167-config-command-and-invocation-contract-migration-validation.md`
- Related: 0165, 0174, 0203
