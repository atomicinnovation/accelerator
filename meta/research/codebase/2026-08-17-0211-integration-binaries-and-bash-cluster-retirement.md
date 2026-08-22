---
type: codebase-research
id: "2026-08-17-0211-integration-binaries-and-bash-cluster-retirement"
title: "Research: Integration Binaries and Bash Cluster Retirement (0211)"
date: "2026-08-17T13:16:26+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0211"
parent: "work-item:0211"
relates_to:
  - "codebase-research:2026-08-02-0187-generalise-sub-binary-registration-surface"
  - "codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"
  - "codebase-research:2026-08-17-0210-provider-client-crates-over-the-tracker-port"
topic: "Implementation ground for shipping accelerator-jira and accelerator-linear and retiring both bash script clusters"
tags: [research, codebase, jira, linear, integrations, cli, cutover, exit-codes, registration]
revision: "990669317762ae2f6f7283437cbd8dd85d2f1fa8"
repository: "accelerator"
last_updated: "2026-08-19T01:04:51+00:00"
last_updated_by: Toby Clemson
last_updated_note: "Added follow-up research for the completion of work item 0210 (provider client crates)"
schema_version: 1
---

# Research: Integration Binaries and Bash Cluster Retirement (0211)

**Date**: 2026-08-17 13:16 UTC
**Author**: Toby Clemson
**Git Commit**: `5e8e86777e45334c864b21053d5e6abe7a2c7a89`
**Branch**: no bookmark (change `pmluwtrlktmo`)
**Repository**: accelerator

> **Update 2026-08-19**: work item **0210 is now complete** (merged in PR #70).
> The findings below describe the pre-0210 codebase at revision `5e8e8677`;
> where 0210's completion changes them, see
> [Follow-up Research (2026-08-19)](#follow-up-research-2026-08-19--0210-complete)
> at the end. The headline reverses in 0211's favour: the client crates now
> implement **every** provider flow — comment, transition, attach, discovery and
> search included — so `accelerator-jira`/`accelerator-linear` are genuinely
> thin.

## Research Question

What does the codebase actually look like for work item 0211 — shipping
`accelerator-jira` and `accelerator-linear` as thin inbound CLI adapters over
0210's client crates, repointing sixteen `SKILL.md` bodies, and deleting both
bash script clusters, their suites and their Python mock servers?

## Summary

**0211's ⚠️ size-bounding assumption is refuted for both providers.** The eight
enumerated flows are not the whole user-facing surface. Jira has **eleven**
`SKILL.md`-reachable entrypoints and seventeen executables spanning roughly
twenty-five distinct verbs; Linear has **ten** executables. The child is larger
than its own sizing allows for.

Four other findings change what a plan must contain.

- **`linear-graphql.sh` is settled**: a production script, not a library. The
  `SHELL_LIBRARIES` removal count stays at **seven**.
- **The `jq`/`curl` survivor set is empty**, not "the skills 0174 owns". Only six
  skills declare either token and all six are jira/linear. The acceptance
  criterion describes a set that does not exist.
- **Three build-system tripwires fire on deletion** and none is named in 0211:
  an orphaned-`SHELL_LIBRARIES` stale-entry guard, a mock-server *existence*
  assertion in the Python coverage test, and the loss of the repo's only
  dual-use-script exemplar.
- **Token registration and skill repointing must land in one commit** — the
  dispatch-coherence guard binds both directions.

Two things are cheaper than 0211 assumes. The release pipeline is fully
implemented and its upload set is *derived* from one constant, so registration
is a registry edit rather than pipeline work. And the sixteen generated
docs-site pages are gitignored, so they need no attention at all.

## Detailed Findings

### The flow surface — the ⚠️ assumption is refuted

0211 assumes "the eight enumerated flows plus ADF↔markdown, JQL and GraphQL
construction are the whole user-facing surface of both clusters". They are not.

| Provider | Non-test `.sh` | Executable | Libraries | SKILL-reachable | Distinct verbs | Lines |
|---|---|---|---|---|---|---|
| Jira | 22 | 17 | 5 | 11 | ~25 | 5,082 |
| Linear | 12 | 10 | 2 | 9 | ~15 | 2,912 |

Jira adds three `SKILL.md`-invoked entrypoints beyond the eight flows:

- `jira-auth-cli.sh` — `init-jira/SKILL.md:83`. Prints `site=`/`email=`/`token=`
  on stdout. 46 lines.
- `jira-resolve-fields.sh` — `create-jira-issue/SKILL.md:61`. Emits a
  **tab-separated four-field line** and owns exit codes 108/109. 187 lines.
- `jira-emit-key.sh` — `create-jira-issue/SKILL.md:105`. Wraps
  `jira-create-flow.sh` and prints the bare issue key. 55 lines.

Linear adds one: `linear-auth-cli.sh` at `init-linear/SKILL.md:38`, printing
`token=<value>`.

⚠️ **`init` is not one operation.** `jira-init-flow.sh` takes six subcommands
(`verify`, `discover`, `prompt-default`, `refresh-fields`, `list-projects`,
`list-fields`) plus a bare interactive full-flow that blocks on `read -r`
(`jira-init-flow.sh:191`). `linear-init-flow.sh` takes three (`verify`,
`list-teams`, `discover`) plus a bare mode. The acceptance criterion's "sixteen
subcommands" undercounts by roughly nine.

**0211's inventory counts are wrong in the same way for both providers.** The
Context reads "22 production scripts plus the `jira-common`, `jira-auth`,
`jira-jql`, `jira-body-input` and `jira-custom-fields` libraries", implying 27
files. The real figure is 22 `.sh` files *including* those five. Linear's "12
production scripts plus `linear-common` and `linear-auth`" implies 14; the real
figure is 12 including both. The flow-coverage criterion uses "the 22 Jira and
12 Linear production scripts" as a fixed denominator, so the reconciliation it
demands will not close as written.

Three data assets sit outside the `.sh` count and map onto no subcommand:
`jira-adf-render.jq` (87), `jira-md-tokenise.awk` (177) and
`jira-md-assemble.jq` (191). The `\x1f`/`\x1e` binary record stream between the
awk tokeniser and the jq assembler is a bash-pipeline artefact that disappears
entirely in Rust.

Total deletion set: **~17,650 lines** — 5,082 (Jira `.sh`) + 455 (`.jq`/`.awk`)
+ 2,912 (Linear `.sh`) + 9,204 (33 suites), plus 191 fixture and helper files.

### `linear-graphql.sh` — the open classification, settled

**It is a production script**, so the `SHELL_LIBRARIES` removal count stays at
**seven**. The evidence is unanimous:

- Executable `0755`, confirmed by `stat` on the working tree.
- Absent from `SHELL_LIBRARIES` (`tasks/lint/scripts.py:40-41` lists only
  `linear-common.sh` and `linear-auth.sh`).
- Zero `source` references repo-wide; **eight** out-of-process call sites
  (`bash "$DIR/linear-graphql.sh"`) across every Linear flow.
- No `BASH_SOURCE`/`$0` guard around its top-level dispatch
  (`linear-graphql.sh:438-461`, `:529-535`), so sourcing it would execute the
  request pipeline against the caller's argv.

It carries a full CLI (`--query`, `--variables`, `--paginate`, `--debug`) but no
`SKILL.md` names it. It is an internal transport with a user-invocable surface —
reachable only because every Linear read skill grants
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/*)`.

### The `jq`/`curl` survivor set is empty

⚠️ 0211's acceptance criterion expects the post-change declarer set to be
"exactly the set belonging to skills still backed by repo-root `scripts/*.sh`,
all of which 0174 owns". **That set is empty.** Verified by two greps at this
revision:

```bash
grep -rn "Bash(jq\|Bash(curl" skills/          # 12 lines, 6 files
grep -rln 'Bash(${CLAUDE_PLUGIN_ROOT}/scripts/' skills/   # 0 files
```

All six declarers are the read/init skills of the two clusters — `init-jira`,
`show-jira-issue`, `search-jira-issues`, `init-linear`, `show-linear-issue`,
`search-linear-issues`. No work skill declares either token; no skill anywhere
is backed by a repo-root `scripts/*.sh` grant, though thirty such scripts still
exist. Every occurrence is the bare parenthesised form `Bash(jq)` / `Bash(curl)`
— there is no `Bash(jq:*)` variant to catch.

Two consequences. 0211's criterion should assert the set is **empty**, which is
strictly stronger and actually checkable. And 0212's criterion ("no work
`SKILL.md` declares `jq` or `curl`") is **already true** and requires no work.

One latent gap worth recording separately:
`skills/work/list-work-items/SKILL.md:340` and
`skills/work/sync-work-items/SKILL.md:108` both run `jq -r` inline while
declaring no `Bash(jq)` rule. They are members of the "needs jq" set that the
frontmatter does not admit — pre-existing, not caused by this change.

### Skill bodies: what the repointing actually touches

Sixteen `SKILL.md` files, eight per provider, in two permission regimes split
exactly on read-versus-write.

| Regime | Skills | `allowed-tools` | Declares `jq`/`curl` |
|---|---|---|---|
| Read/init | `init-*`, `show-*`, `search-*` (6) | path-scoped script glob + `config *` | yes |
| Write | `create-*`, `update-*`, `comment-*`, `transition-*`, `attach-*` (10) | bare `Bash` + `Read` + `Write` | no |

**No cluster script runs at `!`-preprocessor time.** All 32 `!` blocks across the
sixteen files are `bin/accelerator config context|instructions … --fail-safe`.
Every cluster invocation is an execution-time instruction inside a fenced block.
0211's requirement to capture stdout "including any `!`-preprocessor
interpolation" is therefore vacuous — no tracker output is ever interpolated
into prompt text.

⚠️ **The ten write skills declare bare `Bash`, which disqualifies them as
dispatch-coherence witnesses.** Only the six read/init skills can bind a token
(see below). The bare grant is load-bearing today:
`attach-jira-issue/SKILL.md:70` shells `wc -c`, and
`create-jira-issue/SKILL.md:113` sources `scripts/config-common.sh`. Narrowing
them is a decision the plan must take deliberately.

Stdout contracts vary sharply in strictness, and the strictest are the ones a
byte-for-byte golden must preserve:

- `show-linear-issue/SKILL.md:54-62` names six explicit `.data.issue.*` paths.
- `create-jira-issue/SKILL.md:65-66` parses a **tab-separated four-field
  positional line** from `jira-resolve-fields.sh`.
- `jira-emit-key.sh` and `linear-create-flow.sh` treat the whole of stdout as a
  bare identifier.
- Three skills gate on *empty* stdout as a failure signal
  (`create-jira-issue:183`, `update-jira-issue:82`, `comment-jira-issue:98`).

### The exit-code landscape — four taxonomies, three collisions

Both clusters already carry a document of record:
`skills/integrations/jira/scripts/EXIT_CODES.md` (154 lines) and
`skills/integrations/linear/scripts/EXIT_CODES.md`. The anchoring requirement is
satisfiable, but the tables collide in ways that constrain the new binaries.

⚠️ **`70`–`73` means two incompatible things.** `work-item-bridge-codes.sh:30-33`
defines `E_DISPATCH_RETRYABLE/TERMINAL/NOT_AVAILABLE/UNRECOGNISED` on exactly the
integers both search flows use for their own errors — Jira `70-73` is
`E_SEARCH_BAD_PAGE_TOKEN/BAD_LIMIT/NO_SITE_CACHE/BAD_FLAG`, Linear `70-73` is
`E_SEARCH_BAD_FLAG/BAD_LIMIT/NO_CATALOGUE/BAD_STATE`. An unmapped search-flow
code reaching `accelerator-work` reads as a dispatch verdict. This is the
single hardest constraint on the new binaries.

Two further collisions between the providers:

| Code | Jira meaning | Linear meaning | Divergence |
|---|---|---|---|
| `82` | `E_SHOW_BAD_FLAG` (usage) | `E_SHOW_NOT_FOUND` (not-found) | opposite classes |
| `81` | `E_SHOW_BAD_COMMENTS_LIMIT` | `E_SHOW_BAD_FLAG` | shifted by one |
| `34` | HTTP 400, **retryable** on create/update | HTTP 400 *and* 200-with-`errors[]`, **terminal** on update | opposite retry semantics |

Linear's `EXIT_CODES.md:20-23` already warns that readers "must **not** assume
per-number parity outside the transport band". Rate limiting has no shared
number at all — Jira `19`, Linear `35`, each reserved-but-unused in the other.

**The skill-visible contract is overwhelmingly a Jira phenomenon.** Jira skills
branch on roughly forty-five distinct integers; the Linear skills cite exactly
one (`107`, `E_CREATE_WRITEBACK_FAILED`, at `create-linear-issue/SKILL.md:92`)
and are otherwise symbolic `E_*` names. So the byte-anchoring obligation binds
hard on Jira and barely at all on Linear — but Linear's *symbolic names* must
survive in stderr, which no criterion currently states.

Enforcement is asymmetric, and this matters for which model to copy:

- **Linear's doc is machine-pinned.** `test-linear-paths.sh:71-103` greps every
  flow for `readonly E_*=NN` and asserts the same value appears in
  `EXIT_CODES.md`.
- **Jira's is prose-only** and already wrong: the doc says usage errors exit `2`
  (`EXIT_CODES.md:12`) while `jira-request.sh:207` exits `1`. Jira uses bare
  numeric literals — 79 raw `exit NN`/`return NN` across nine files.

`tracker` holds **no integer enum**. `TrackerError` is a two-variant class
(`cli/tracker/src/lib.rs:144-179`); the integers live in
`cli/work-cli/src/exit_codes.rs:5-24` and are pinned to bash by
`cli/work-cli/tests/exit_codes_parity.rs`, which parses both sources textually
because the binary is bin-only. ⚠️ `72`/`73` resolve **above** the port
(`cli/tracker/tests/fixtures/dispatch-codes.txt:3-6`) — a provider binary must
not own them.

Credential resolution diverges between the providers at the same conceptual
point: Jira flattens every `jira_resolve_credentials` failure to `22`
(`jira-request.sh:215-222`, discarding 24/25/27/28/29), while Linear preserves
`25`, `27` and `29` (`linear-graphql.sh:481-489`). A single client cannot
reproduce both without an explicit per-provider rule.

### Registration: two thirteen-point checklists, plus library-crate surface

`tasks/README.md:399-563` carries the checklist; `tasks/README.md:565-622`
carries the smaller library-crate one. Both apply — the binary crates take the
first, each new client crate takes the second.

The registry is Python, not Rust. There is **no dispatch table to edit**: the
launcher captures unknown subcommands via clap's `external_subcommand`
(`cli/launcher/src/launch/inbound/cli.rs:26-28`) and derives the asset name as
`format!("accelerator-{name}-{platform}")`
(`cli/launcher/src/launch/outbound/resolve/mod.rs:147`).

The central edit is `tasks/shared/paths.py:29-37`, seven tokens today. Both
`jira` and `linear` clear every constraint: they match `^[a-z][a-z0-9-]*$`, are
not in `RESERVED_TOKENS` (`{verify, launcher}`) and do not shadow
`BUILTIN_SUBCOMMANDS` (`{version, config, help}`). They derive
`ACCELERATOR_JIRA_BIN` and `ACCELERATOR_LINEAR_BIN` for free.

⚠️ **Token registration and skill repointing must land in one commit.**
`tasks/shared/dispatch_coherence.py` checks both directions: a registered token
with no consuming skill fails at `:196-202`, and a skill invoking an
unregistered token fails at `:210-216`. `tasks/README.md:537` says points 1, 2,
3, 4, 7 and 8 must land together. 0211 does not state this.

The binding rule is narrow. A witness skill must invoke
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token> …` — via `!` preprocessor *or* a
fenced block in a numbered step — and carry a `Bash(...)` rule whose subcommand
segment is exactly the token. A bare `Bash`, a bare-launcher rule, or a
wildcarded token segment anywhere in that skill's frontmatter disqualifies the
whole skill. Only the six read/init cluster skills qualify.

Concrete touch list, derived from the `design` token as the most recent
worked example:

| Point | File | Today |
|---|---|---|
| 1 | `tasks/shared/paths.py:29-37` | `DISPATCHED_SUBBINARIES` |
| 1 | `tests/integration/tasks/test_github.py:36-51`, `:533-541` | descriptions map + exact-tuple pin |
| 2 | `tasks/manifest.py:55-65` | `_SUBBINARY_MANIFESTS` (needed — crate is not at `cli/<token>/`) |
| 3 | new `cli/jira-cli/Cargo.toml` | `[[bin]] accelerator-jira`, mandatory `description` |
| 4 | `cli/Cargo.toml:4-35` + `Cargo.lock` | 30 members today; resync via `cargo metadata`, never `generate-lockfile` |
| 5 | `.gitignore:53` | `bin/jira-*`, `bin/linear-*` — **[author], nothing catches it** |
| 7 | six read/init `SKILL.md` files | the binding |
| 8 | `tasks/build.py:36-45` | `_CLI_RELEASE_BINARIES` — prefixed name, not bare token |
| 11 | `docs-site/`, `astro.config.mjs:102`, `README.md:64-66` | **[author]** |
| 13 | `cli/deny.toml` | only if `deny:check` reddens |

Library-crate surface per new crate: `cli/pup.ron` rule, a probe pair in
`tests/integration/pup/test_import_rule.py`, and a classification in
`tasks/public_api.py` (`_PINNED_CRATES` or `_EXEMPT_MEMBERS`). The last is the
one the build tells you about — `tests/unit/tasks/test_rust.py:159-217` fails
until every workspace member is classified. A provider client is an adapter and
belongs in `_EXEMPT_MEMBERS`. Note that no `*-cli` crate has a `pup.ron` rule;
the rules attach to the domain and adapters crates beside them.

⚠️ `cli/tracker/tests/structure.rs:68-77` asserts the workspace contains no
`tracker-adapters`, with the comment "provider clients live in their own
crates". Name them `jira`/`linear`.

### The release pipeline is done, and the upload set is derived

0165 is fully implemented. There is no flat `checksums.json` anywhere in
`tasks/`, no separate upload list and no manifest list to extend — signing
(`tasks/signing.py:50-57`), manifest emission (`tasks/manifest.py:81-108`) and
upload (`tasks/github.py:219-248`) all compute the cross product of
`DISPATCHED_SUBBINARIES × TARGETS`. Adding two tokens is a registry edit.

`tasks/shared/targets.py:3-8` holds four targets cross-compiled from a single
macOS runner via `cargo zigbuild` — there is no per-platform CI matrix. The
launcher verifies the manifest signature over raw bytes *before* parsing
(`resolve/mod.rs:130`), pins `schema_version` at 1, and requires exact version
equality as an anti-rollback measure.

Failure modes if registration is partial: `release:sign` fails first, at
`tasks/signing.py:73-77`, before the manifest is emitted and before commit/tag/
push — so it fails clean. The genuinely uncaught items are the `.gitignore`
entries (point 5) and the user docs (point 11).

One stale artefact found in passing: `docs-site/src/content/docs/guides/faq.md`
lines 18 and 41 still tell users the download is verified against
`bin/checksums.json`, a mechanism that no longer exists.

### Three build-system tripwires, none named in 0211

**1. The orphaned-`SHELL_LIBRARIES` guard is a hard failure.**
`tasks/lint/scripts.py:102-112` compares every list member against
`shell_sources()` and emits `stale library-list entry (not enumerated)` per
orphan, exiting 1. Deleting the clusters without editing the frozenset produces
seven offenders. The mode loop iterates `sources`, never `SHELL_LIBRARIES`, so
this pre-check exists precisely to close that hole. It is pinned twice more:
`tests/unit/tasks/test_exec_bits.py:279-282` (set equality against
`_RECONCILED_LIBRARIES`, which duplicates all seven members at `:261-267`) and
`:284-287` (every member enumerated).

**2. The mock-server coverage test asserts the files exist.** This is the
most dangerous item in the sweep, and 0211 does not mention it.

```python
# tests/unit/tasks/test_python_coverage.py:102-113
assert MOCK_JIRA in py
assert MOCK_LINEAR in py
```

`test_in_scope_set` asserts both mock servers are present **in the walked repo
tree**. Deleting them fails `mise run test:unit` regardless of what is done to
`pyproject.toml`. And `test_ruff_extend_exclude_is_exactly_justified` (`:85-88`)
uses set *equality* against `pyproject.toml:77-81`, so the two must move
together. A coordinated three-way edit: delete the files, strip the two
`extend-exclude` entries, and remove `MOCK_JIRA`/`MOCK_LINEAR` plus all six
usages.

**3. `_DUAL_USE_SCRIPTS` loses its only member.**

```python
# tests/unit/tasks/test_exec_bits.py:274-275
_DUAL_USE_SCRIPTS = ("skills/integrations/jira/scripts/jira-fields.sh",)
```

`jira-fields.sh` is the repository's **only** pinned dual-use exemplar — sourced
by `jira-init-flow.sh:32` and path-invoked from four flows. `test_dual_use_
scripts_are_entrypoints` (`:289-298`) `os.access`es it and fails on a deleted
file; an empty tuple makes the test vacuous. `tasks/README.md:90-94` documents
the dual-use classification entirely through this script, so the prose needs a
surviving exemplar too.

### The integrations floor has zero headroom

`_EXPECTED_INTEGRATIONS_SUITES = 32` (`tasks/test/integration.py:57`) counts
exactly the discoverable set: 21 Jira + 12 Linear `test-*.sh` = 33, minus
`test-jira-scripts.sh`, excluded by name in
`tasks/test/helpers.py:6-10`. Discovery is a glob filtered by the exec bit
(`helpers.py:97-103`), and `run_shell_suites` returns `[]` for a missing subtree
rather than erroring — so deletion trips the floor, not the walk.

Removing the floor pulls in four more edits: the `integrations` task
(`tasks/test/integration.py:405-410`), the `_GUARDED` entry at
`tests/unit/tasks/test_integration.py:69` (an `AttributeError` otherwise),
`mise.toml:350-353` and `:369`, and `tests/unit/tasks/test_mise.py:51`, whose
partition assertion is exact.

### What is *not* affected

Three things I expected to be in scope and are not.

- **The sixteen docs-site reference pages.**
  `docs-site/src/content/docs/reference/skills/` is **gitignored**
  (`.gitignore:26`) and untracked. `mise run docs:generate` rebuilds them from
  `SKILL.md` at build time and `docs:generate:check` depends on it. They pick up
  the repointed bodies automatically; nothing to commit, no staleness risk.
- **`EXPECTED_INJECTION_SKILLS = 42`** (`tasks/lint/skill_permissions.py:48`).
  All sixteen cluster skills inject both `config context --skill` and
  `config instructions`, but 0211 keeps the skills and repoints only their
  bodies, so the census is unchanged. It would drop to 26 only if the skills
  themselves were deleted.
- **`.claude-plugin/plugin.json:16-17`.** The two skill directories stay
  registered; only their `scripts/` subdirectories go.

### The reverse sweep, and a coupling 0211 understates

The cluster→work direction is clean. Grepping
`skills/integrations/*/scripts/*.sh` for `work-item-*.sh` invocations returns
exactly two hits, both comments:

- `linear-create-flow.sh:304` — `# (Same normalisation as the Jira guard and work-item-sync-label.sh.)`
- `jira-resolve-fields.sh:140` — `# guard and work-item-sync-label.sh).`

The work→cluster direction is **live production code**, and 0211's Context
frames the coupling only through test assets. Nine invocations:

```
skills/work/scripts/work-item-create-remote.sh:199   → linear-create-flow.sh
skills/work/scripts/work-item-fetch-remote.sh:120    → jira-search-flow.sh
skills/work/scripts/work-item-fetch-remote.sh:161    → linear-search-flow.sh
skills/work/scripts/work-item-fetch-remote.sh:258    → jira-search-flow.sh
skills/work/scripts/work-item-fetch-remote.sh:265    → jira-show-flow.sh
skills/work/scripts/work-item-fetch-remote.sh:279    → linear-search-flow.sh
skills/work/scripts/work-item-fetch-remote.sh:285    → linear-show-flow.sh
skills/work/scripts/work-item-update-remote.sh:143   → jira-update-flow.sh
skills/work/scripts/work-item-update-remote.sh:154   → linear-update-flow.sh
```

0212 deletes all nine callers, so the ordering holds and nothing is at risk —
but it strengthens rather than weakens the 0212-before-0211 constraint, and it
means four Linear flows and three Jira flows currently serve a *machine*
contract as well as a skill contract.

Beyond the two Python config files already covered, the shared-asset sweep found
**no consumer outside the three expected clusters**. `.github/` is clean;
`hooks/`, `templates/`, `agents/`, `scripts/`, `cli/` and `docs-site/` are clean.
The deletion set is 191 files: 150 under Jira (`test-helpers/` 2,
`adf-samples/` 43, `api-responses/` 10, `scenarios/` 95) and 41 under Linear.

⚠️ `skills/integrations/jira/scripts/test-fixtures/api-responses/` — all ten
files — has **zero consumers today**. It is already dead weight.

### `wiremock` conflicts with a thrice-restated convention

0210 mandates `wiremock-rs`. It appears nowhere in the workspace — not in any
`Cargo.toml`, not in `cli/Cargo.lock`, not in any test. The established pattern
is a purpose-built, std-only `MockServer` (~145 lines) in each consumer's
`tests/common/mod.rs`, and the decision against `wiremock` is stated explicitly
in two of the three copies:

> Ported from `cli/launcher/tests/common/mod.rs`'s `MockServer` structure …
> rather than shared as a crate, mirroring that file's own precedent for
> HTTP-level test stubbing in this workspace (no `wiremock`/`mockito`).
> — `cli/github/tests/common/mod.rs:5-10`

This is 0210's call, not 0211's, but 0211 inherits whichever fixture harness
results. The `cli/github/` copy is the richest and would need request-body
capture and a per-route response queue to serve eight subcommands with
pagination.

### Rust patterns to build on

**Thin CLI over a client crate.** `cli/collaboration-cli/` + `cli/github/` is
the structural match — a binary over a client crate implementing domain ports.
`cli/work-cli/` is the match for many subcommands plus a rich exit-code
taxonomy. The dominant `main.rs` shape threads `Result<Outcome, kernel::Error>`
and prints only in `main` (`cli/corpus-cli/src/main.rs:127-154`), with
`Outcome { stdout, stderr }` and `kernel::Error::Refusal → ExitCode::from(2)`
shared across every sub-binary.

**Stdout goldens.** No `insta`, no `assert_cmd`. Two committed-file patterns:

- `cli/work-cli/tests/cli_surface.rs` — freezes every subcommand's `--help` into
  one golden with `=== <bin> <sub> --help ===` section headers. This is the
  direct template for sixteen (really twenty-five) subcommands.
- `cli/launcher/tests/config_read.rs:150-152` — byte-exact `output.stdout`
  against a golden read as `Vec<u8>`, "never through `from_utf8_lossy`".
- `cli/design/tests/downgrade_goldens.rs:8-63` — the only self-regenerating
  variant (`UPDATE_DOWNGRADE_GOLDENS=1`), paired with an anti-orphan test.

**HTTP.** `reqwest` + `rustls` are already workspace dependencies with
`native-tls` excluded and `hickory-dns` pinned for musl
(`cli/Cargo.toml:57-66`). Because `RemoteTracker` is synchronous, the blocking
`reqwest` pattern at `cli/launcher/src/launch/outbound/resolve/fetcher.rs:53-101`
fits better than `collaboration-cli`'s `block_on` bridge. Timeout precedent for
small JSON bodies is 10s connect / 30s read / 30s write
(`cli/github/src/octocrab_client.rs:8-10`).

**Auth.** `cli/collaboration-cli/src/auth.rs:27-91` already reproduces
`jira-auth.sh`/`linear-auth.sh`'s precedence order, including the shared-config
`token_cmd` ban, and its own comments say so. Copy it wholesale.

**Test seam.** `ACCELERATOR_COLLABORATION_GITHUB_API_URL`
(`cli/collaboration-cli/src/main.rs:41-48`) is the base-URL override pattern;
`ACCELERATOR_JIRA_API_URL` / `ACCELERATOR_LINEAR_API_URL` on the same model.
Note `with_base_uri` is deliberately **not** `#[cfg(test)]`-gated, because
`tests/` links the crate as an external dependency.

### 0194's artefacts are present — the ⚠️ confirmation discharged

0171 instructs confirming 0194 against artefacts rather than its status field.
All three exist at this revision:

- `cli/tracker-test-support/src/contract.rs` (9.5K), with `ContractSubject` and
  six properties gated on `ACCELERATOR_TRACKER_CONTRACT=1`.
- `accelerator work sync` — `cli/work-cli/src/cli.rs:88`, `:191`.
- The baseline corpus at `skills/work/scripts/test-fixtures/` — 18 entries.

`jira-client`/`linear-client` do not exist; 0210 has not started.
`cli/work-cli/tests/cli_sync.rs:75` currently asserts `jira` exits `72`
(recognised, unbuilt) — that test changes when the clients land.

## Code References

- `skills/integrations/jira/scripts/` — 22 `.sh` + 3 assets, 5,537 lines
- `skills/integrations/linear/scripts/` — 12 `.sh`, 2,912 lines
- `skills/integrations/jira/scripts/EXIT_CODES.md:8-154` — Jira table, prose-only
- `skills/integrations/linear/scripts/EXIT_CODES.md:34-120` — Linear table, pinned
- `skills/integrations/linear/scripts/test-linear-paths.sh:71-103` — the doc pin
- `skills/work/scripts/work-item-bridge-codes.sh:28-34` — `E_DISPATCH_*`
- `cli/tracker/src/lib.rs:144-179` — `TrackerError`, two classes, not `non_exhaustive`
- `cli/tracker/tests/fixtures/dispatch-codes.txt:3-13` — 72/73 above the port
- `cli/work-cli/src/exit_codes.rs:5-24` — the taxonomy-in-one-module precedent
- `cli/work-cli/tests/exit_codes_parity.rs:19-79` — textual cross-source parity
- `cli/work-cli/tests/cli_surface.rs` — the whole-CLI-surface golden
- `cli/tracker-test-support/src/contract.rs:25-34` — `ContractSubject`
- `cli/tracker/tests/structure.rs:68-77` — no `tracker-adapters`
- `tasks/shared/paths.py:29-37` — `DISPATCHED_SUBBINARIES`
- `tasks/shared/dispatch_coherence.py:154-217` — both-directions binding
- `tasks/README.md:399-563` — the thirteen-point checklist
- `tasks/README.md:565-622` — the library-crate checklist
- `tasks/lint/scripts.py:18-43` — `SHELL_LIBRARIES`, seven cluster members
- `tasks/lint/scripts.py:102-112` — the stale-entry guard
- `tasks/test/integration.py:53-57`, `:405-410` — the integrations floor and task
- `tests/unit/tasks/test_exec_bits.py:244-298` — set-equality pin + dual-use
- `tests/unit/tasks/test_python_coverage.py:33-41`, `:102-113` — the tripwire
- `pyproject.toml:77-81` — the mock-server ruff excludes

## Architecture Insights

**The registry is Python; the launcher is generic.** No Rust source changes for
a new token. Every constraint — validity, reservation, built-in shadowing,
skill binding — is enforced from `tasks/`, and the manifest is the sole runtime
registry. This makes registration cheap but concentrates the whole surface in
one checklist whose points must land together.

**Guards are equality assertions, not floors, wherever the author could manage
it.** `SHELL_LIBRARIES` set-equality, `RUFF_JUSTIFIED_EXCLUDES` set-equality,
the `DISPATCHED_SUBBINARIES` tuple pin, the exact `mise.toml` partition. The
consistent idiom is an anti-vacuity anchor: a hardcoded count beside a loop so
the loop cannot pass on an empty set. 0211's `jq`/`curl` equality criterion is
in the same spirit, and should be tightened to "empty" now that the survivor set
is known.

**Deletion safety comes from mechanical derivation at a pinned revision.** This
is the lesson of the 0167 config-cluster retirement, and the 0211 review's own
Correction is the counter-example: a coupling "named but not verified" inverted
the entire child ordering three passes late. Any 0211 plan should open with the
recorded sweep — command, output, revision — not with the criteria.

**Two senses of "library" are structurally distinct and the repo enforces
both.** A sourced-only file on disk (exec bit `0644`) and a `SHELL_LIBRARIES`
frozenset member are pinned to each other by
`tests/unit/tasks/test_exec_bits.py`, with `jira-fields.sh` as the deliberate
counter-example proving the two can diverge. 0211 is right to insist the
distinction be recorded.

## Historical Context

- `meta/reviews/work/0211-…-review-1.md` — APPROVE after four passes. ⚠️ Its
  **Correction** section post-dates the Acceptance and resolves both
  self-contradictions 0171's drafting notes still list as live: the mock-server
  deferral (resolved by the reordering — deletion is now unconditional) and the
  `jq`/`curl` survivor set (0211 now owns the whole-repository equality, 0212
  asserts only the work-skill half). 0171's notes are stale on both.
- The same Correction records that the pass-1/2 premise — the two
  `work-item-sync-label.sh` references being live callers — was false, which is
  what inverted the ordering to 0210 → 0212 → 0211.
- Two findings were accepted unresolved and carry into planning: the non-port
  provider surface (five of eight flows) owned by neither 0210 nor 0211, and
  0210 carrying no criterion for HTTP-status or GraphQL error classification.
- `meta/inventories/0167-{removal-set,suite-audit,divergences}.md` — the
  artefact set the previous bash-cluster retirement produced, and the direct
  template for 0211's recorded mapping, sweep and enumeration. The divergences
  file's governing rule transfers verbatim: *"A divergence nothing can detect is
  indistinguishable from a defect, so every row names a real, passing test."*
  Its divergence #4 — "usage errors exit 1 (not clap's 2), so exit 2 is reserved
  for a subcommand refusal", pinned by a named black-box CLI test — is exactly
  the external anchor 0211's exit-code criterion needs.
- `meta/validations/2026-07-19-0167-…-validation.md` — verdict **partial**, and
  the generalisable lesson is that a whole-cluster retirement lands partial
  legitimately, provided un-runnable criteria (live-session, release-pipeline)
  are separated from un-built ones. It also records that the mixed
  bash/`accelerator` state is safe on `main` until the release that carries the
  flip — which answers the review's major #10 about bundling registration into a
  cutover child.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`,
  `ADR-0045-skills-vs-cli-division-of-labour.md`,
  `ADR-0046-zero-setup-static-binary-distribution.md`,
  `ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`.
- `meta/notes/2026-07-13-bash-corpus-script-inconsistencies.md` — porting bash to
  Rust surfaces latent bash bugs. Two are already visible here: Jira's
  doc-versus-code usage code, and the three-way duplicated `@me` resolution with
  three different error codes (`jira-create-flow.sh:87` → 106,
  `jira-update-flow.sh:95` → 115, `jira-search-flow.sh:72` → 72).

## Related Research

- `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md`
- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`
- `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`
- `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md`

## Open Questions

Carried from 0171 and unanswered at this revision:

- Where the credentialed target's secrets live — CI or local-only.
- The fate of the three port-less bridge capabilities (0212's, but it decides
  whether `search` means two things).
- `EXIT_CODES.md` siting.

Raised by this research:

- **Does 0211 grow to cover the extra entrypoints, or does the surface shrink?**
  `jira-emit-key.sh` is a projection of `create` (`create --emit key`), and
  `jira-resolve-fields.sh` makes no API call at all — it parses work-item
  frontmatter and shells to `accelerator config`, so it arguably belongs in
  `accelerator work`. Both `*-auth-cli.sh` scripts print credentials in
  cleartext on stdout; reproducing that as a subcommand is a security decision
  to re-open, not a mechanical port.
- **Which exit-code enforcement model?** Linear's derived-doc grep, or
  work-cli's `exit_codes.rs` + textual parity test. Only these two mechanisms in
  the repo actually hold a table honest, and Jira's current table is held by
  neither.
- **Do the ten bare-`Bash` write skills get narrowed?** They cannot witness a
  token, and the bare grant currently covers `wc -c` and a `source` of
  `config-common.sh` that the migration must otherwise replace.
- **`wiremock` versus the in-repo `MockServer`** — 0210's decision, but 0211
  inherits the fixture harness.
- **Where does `jira-jql-cli.sh` go?** It is an orphan: executable, arg-parsing,
  25 lines, invoked only by `test-jira-jql.sh`. Porting it would create
  user-facing surface that does not exist today.

## Follow-up Research 2026-08-19 — 0210 complete

**Revision**: `990669317762ae2f6f7283437cbd8dd85d2f1fa8`
**Author**: Toby Clemson

Work item 0210 merged (PR #70, "Mark work item 0210 done"). It shipped
`jira-client`, `linear-client`, the shared `tracker-support` policy crate and the
`http-test-support` dev harness, wired both clients into `accelerator-work`'s
composition root, and committed the three oracle transcriptions. Every finding
above that assumed "0210 has not started" is discharged here. **The headline
reverses in 0211's favour: the client crates implement every provider flow, so
`accelerator-jira`/`accelerator-linear` are genuinely thin.**

### The port-less flows are built — the largest open question is closed

The original research's sharpest uncertainty — "Does 0211 grow to cover the extra
entrypoints, or does the surface shrink?" — is answered by neither. **0210
absorbed the whole provider surface into the client crates.** Every port-less
flow the bash clusters expose is an inherent method on `JiraClient`/`LinearClient`
returning a dedicated `SurfaceError`:

| Flow | Jira method(s) | Linear method(s) |
|---|---|---|
| comment | `add_comment`/`edit_comment`/`delete_comment`/`list_comments` (`comment.rs`) | `add_comment` (`comment.rs`) |
| transition | `list_transitions`/`resolve_transition`/`transition` (`transition.rs`) | `resolve_state`/`transition` (`transition.rs`) |
| attach | `attach` (`attach.rs`) | `attach_link`/`attach_file` (`attach.rs`, three-step upload) |
| init/discovery | `discover_site`/`discover_projects`/`discover_fields` (`discovery.rs`) | `discover_viewer`/`list_teams`/`discover_team` (`discovery.rs`) |
| search | JQL composer (`jql.rs`) | `IssueFilter` composer (`filter.rs`) |

The lib docs make the intent explicit: these "sit beyond the four `RemoteTracker`
port methods … and have no port of their own yet"
(`cli/jira-client/src/surface.rs:1-8`). Endpoint paths and GraphQL documents are
hard-coded inside the crates (Jira REST constants at
`cli/jira-client/src/client.rs:47-48`; Linear `const &str` documents at
`cli/linear-client/src/client.rs:44-65`), and the pup rules forbid any of it
leaking out (`cli/pup.ron:194-262`). **0211's binaries need no request
construction** — they parse args, assemble a credential context, call these
methods, and render JSON or errors.

### The public surface 0211 consumes, and the three error layers it renders

The thin CLIs link against `JiraClient`/`LinearClient` (`from_config`
constructors at `client.rs:83` / `:101`) plus free functions
(`document_to_markdown`, `markdown_to_document`, `resolve_credentials`,
`classify`). ⚠️ **Neither crate carries a public-API snapshot** — both are
`_EXEMPT_MEMBERS`/`_ADAPTER` (`tasks/public_api.py:52-59`), so the surface is
unpinned and each CLI binds whatever its crate exposes.

Each binary must render **three** error layers to stderr plus an exit code, not
one:

- **`classify → TrackerError`** for the four port operations (`classify.rs`) —
  the retryable/terminal verdict the sync bridge reads.
- **`SurfaceError`** for the port-less flows — richly `E_*`-coded
  (`E_TRANSITION_NOT_FOUND`, `E_TRANSITION_AMBIGUOUS`, `E_ATTACH_BAD_FILENAME`,
  `E_COMMENT_BAD_PAGE_SIZE`, …) at `surface.rs`.
- **`ClientError`** for config/site/token/transport faults (`error.rs`).

Mapping these `E_*` variants to the numeric exit codes the SKILL bodies branch on
is the substance of the still-open "binaries' exit-code contract" decision below.

### ⚠️ The test seam the original research proposed does not exist

The original "Test seam" finding proposed `ACCELERATOR_JIRA_API_URL` /
`ACCELERATOR_LINEAR_API_URL` on the `collaboration-cli` model. **Those env vars
exist nowhere in the client crates.** The seam is constructor-based: Linear's
`Transport::new` takes the endpoint explicitly
(`cli/linear-client/src/transport.rs:91-97`), Jira derives it from
`credentials.base` (i.e. `jira.site`). `with_base_uri` is deliberately absent —
the transport doc forbids post-construction setters because `reqwest`'s timeout
is fixed at build time. **0211's CLI layer must add the env→constructor plumbing
itself**, reading `ACCELERATOR_*_API_URL` in the binary and feeding it into
`Transport::new`, mirroring `cli/collaboration-cli/src/main.rs:42-46`.

### The `wiremock` question is settled: no library, `http-test-support`

0171 decision **D3**: no mock library. `wiremock`, `mockito` and `httpmock`
appear nowhere in the workspace — none can hang a connect phase or drop a
connection mid-body, so the hand-rolled stall responder would survive regardless
and the dependency would replace nothing. The two former hand-rolled servers
(`launcher`/`github` `tests/common/mod.rs`, both now deleted) are unioned into
**`cli/http-test-support`** — a zero-dependency, std-only crate providing
request-body capture, a per-route response queue (`Route::Sequence`) and the
decisive `Route::Stall(Duration)` (`cli/http-test-support/src/lib.rs:56-58`). It
is a dev-dependency of both clients and of six crates in all. **0211 inherits
this harness**, closing the original's last open question on the point.

### The contract lane: offline-enforcing, evidence-file assured

0210 split the lane in two. `contract_offline.rs` runs in the **default** profile
over the `http-test-support` mock and is the *enforcing* gate; `contract.rs`
(named exactly so `nextest`'s `default-filter = 'not binary(=contract)'` excludes
it, `cli/.config/nextest.toml:5`) makes the live calls, and its output is a
committed evidence file. `ContractSubject` is implemented four times — live plus
offline per crate. The committed evidence
(`cli/{jira,linear}-client/tests/evidence/contract-run.txt`, live-tenant runs
dated **2026-08-18**) records **jira 4 / linear 5** conformance records — Linear
carries the extra `unaccounted_id_is_indeterminate_not_absent` because its team
scope is a structural indeterminate path, which a live Jira tenant cannot produce
for a benign key. `evidence_hygiene.rs` runs in the default profile and refuses
any evidence file carrying payloads or secrets. **No CI job exists** for the
contract lane — the acceptance route chosen was the committed evidence file, not
a required workflow. ⚠️ The evidence `README.md` still says "not committed yet",
stale relative to the committed data.

### The three oracle transcriptions landed — and one original expectation was wrong

All three exist and are consumed by tests:

| Oracle | Path | Shape | Consumer |
|---|---|---|---|
| Exit-code tables | `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt` | 74 rows, 5 cols (code/provider/operation/class/source) | `jira`+`linear-client/tests/classify.rs` |
| ADF inventory | `cli/jira-client/tests/fixtures/adf-node-types.txt` | 41 node + 16 mark rows | `cli/jira-client/tests/adf_inventory.rs` |
| Parity baseline | `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt` | 11 test rows + case-set rows | `bash_parity_baseline.rs` |

Two corrections to the original research:

- ⚠️ **The exit-code tables live under `tracker-support`, not `tracker`.** The
  original "Code References" cited `cli/tracker/tests/fixtures/…`; that path does
  not exist. And the classification is now *live*: `classify_bash_code` in both
  clients consumes the fixture (`cli/jira-client/src/classify.rs:73-74`,
  `cli/linear-client/src/classify.rs:160-161`), reproducing the create/update
  divergence — code `34` retryable on create, terminal on update.
- ⚠️ **The parity baseline records no `68` count.** The original research
  (via 0210's brief) expected the file to carry the pre-change file count under
  `skills/work/scripts/test-fixtures/` "as a committed number". The landed
  fixture deliberately does the opposite — it pins case-name **sets**, not a
  total, with an in-file note that "storing the numbers too is how the numeric
  copy goes stale". 0211/0212 verify against the sets; there is no committed 68.

### Exit-code landscape: classification pinned, the binaries' contract still open

The original section holds for what 0211 still owns — the *binaries'* exit-code
contract and its document of record remain **open** (0171 Decisions). But 0210
pinned the classification half beneath the port:

- **HTTP status → exit code** is fixed in each `classify.rs` `bash_code`: Jira
  `400→34, 401→11, 403→12, 404→13, 410→14, 429→19, non-JSON→16, transport→21`;
  Linear `auth→11, complexity→36, ratelimited→35, bad→34, non-JSON→16,
  transport→21`.
- **`classify` is per-operation**, and a read never yields `Terminal` — the port
  invariant the original flagged.
- **D11**: `Unconfigured` is exit **74**, and `70`/`71`
  (`E_DISPATCH_RETRYABLE`/`TERMINAL`) derive *exclusively* from `TrackerError`.
  This resolves the original's hardest constraint (the `70-73` collision) at the
  composition-root boundary.

Still 0211's to decide: the numeric codes for the `SurfaceError` variants (the
port-less flows), `EXIT_CODES.md` siting, and the binaries' document of record.

### The composition root is wired

`cli/work-cli/src/tracker_registry.rs` now resolves real clients: the `jira` arm
calls `JiraClient::from_config` and the `linear` arm
`LinearClient::from_config(&integrations_root)` (`:169-179`); `resolve`'s
signature is unchanged, with config injected at construction via
`ConfiguredTrackers::new`. `trello`/`github-issues` return `NotAvailable`
(exit 72); `SelectionError` maps `Unset`/`Unrecognised`→73, `NotAvailable`→72,
`Unconfigured`→74.

Consequently the test the original research flagged has flipped: **`cli_sync.rs`
no longer asserts `jira` exits `72`.** A wired-but-unconfigured `jira` now exits
**74** (`a_wired_tracker_without_credentials_exits_74`, `:78-84`), and only
`trello` still exits `72`. `cli_update_push.rs` matches (jira→74);
`cli_create_push.rs` has `--push` fall back to a local save. `linear.team_id` was
added to both the Rust catalogue (`cli/config/src/catalogue.rs:126`) and the bash
mirror (`scripts/config-defaults.sh:217`, Rust-only, no bash consumer). The
`for_tracker_error` duplication is resolved — `#[allow(dead_code)]` is gone and
`cli/work-cli/src/create.rs:462` delegates.

### Registration: the library surface is done; the tokens are still 0211's

0210 registered the two clients as **library adapter crates** and nothing more.
The workspace lists **35 members** (the original counted 30);
`jira-client`/`linear-client` carry `denied`-only pup rules with probe pairs
(`cli/pup.ron:194-262`, `tests/integration/pup/test_import_rule.py:1148-1276`)
and `_ADAPTER` classification. **No new licence exception was added** — the
rustls/HTTP tree rides the existing permissive allow, and 0171 D3 records no
copyleft is introduced, so **0203 does *not* become a release-path dependency of
0211** (evidence: `cli/licence-audit/new-trees.txt`,
`tests/integration/deny/test_licence_closure.py`). Two hickory DNS advisory
ignores (`RUSTSEC-2026-0118/0119`, review-by 2026-11-03) cover the reqwest stack.

What remains for 0211 is unchanged from the original "Registration" section:
**`DISPATCHED_SUBBINARIES` still lists 7 tokens** (`tasks/shared/paths.py:29-37`)
— `jira`/`linear` are absent — and the binary crates `jira-cli`/`linear-cli` are
not workspace members. The thirteen-point checklist applies to the two binary
crates and two tokens; the *library-crate* surface (pup rule, probe pair,
`public_api` classification) is already discharged for the client crates, and the
two new `*-cli` crates carry no pup rule of their own — the rules attach to the
client crates beside them, already present.

### Open questions: what 0210 closed, what carries forward

Closed by 0210: port-less-flow ownership (in the crates), `wiremock` vs
hand-rolled (`http-test-support`), `gouqi` (read, not vendored — D1), ADF
hand-built vs composed (hand-built — D5), `TrackerRegistry` config acquisition
(construction-time), the Linear team-id key (`linear.team_id`), and the
copyleft/0203 question (no copyleft).

Still open, and now unambiguously 0211's: the **binaries' exit-code contract**
and its document of record (only classification is pinned); **`EXIT_CODES.md`
siting**; the **`*-auth-cli.sh` cleartext-credentials** decision (reproducing
token-on-stdout as a subcommand is a security call, not a mechanical port); the
**`jira-jql-cli.sh` orphan**; and the shape of **`jira-resolve-fields.sh`** — its
field resolution now lives in the crate (`AccountResolver`/`FieldResolver` at
`cli/jira-client/src/client.rs:108-113`), but its frontmatter-parsing,
tab-separated contract is a work-domain concern the CLIs must either reproduce or
relocate to `accelerator work`.
