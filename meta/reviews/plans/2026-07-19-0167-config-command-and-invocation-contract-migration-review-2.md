---
type: plan-review
id: "2026-07-19-0167-config-command-and-invocation-contract-migration-review-2"
title: "Plan Review: Built-in config Command and Invocation-Contract Migration"
date: "2026-07-20T09:40:54+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
target: "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
relates_to:
  - "plan-review:2026-07-19-0167-config-command-and-invocation-contract-migration-review-1"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, test-coverage, safety, compatibility, security, performance, code-quality]
review_number: 2
review_pass: 3
tags: [rust, config, cli, skills, invocation-contract, allowed-tools, hooks, store, migration]
last_updated: "2026-07-20T13:32:01+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Built-in config Command and Invocation-Contract Migration

**Verdict:** REVISE

This is the third review pass, and the pattern the second pass identified has
repeated: the revisions resolved almost every prior finding, and the mechanisms
added to resolve them introduced a new generation of defects. The plan's
verified-measurement discipline is now excellent — the compatibility lens
reproduced every headline count in the measurements table exactly, and the
correctness lens confirmed the plan's analysis of the hardest bash behaviours
(the eject/diff/reset exit-2 asymmetry, the `get`/`path` precedence split, the
`0006:298` subshell trap) in fine detail. The problems are concentrated in three
bands. First, several **premises asserted as settled are factually false** —
most seriously Phase 0's source-checkout gate, whose load-bearing factor is
satisfied in every marketplace install (verified directly against the live
plugin cache), and the migration set, which has seven members, not six. Second,
the plan now **specifies several things two incompatible ways** in different
sections — the `config_command` pup rule, `compose()`'s role, `--fail-safe`'s
scope, and `--format=hook`'s output states. Third, the newly-widened permission
grant **creates capability that did not previously exist**: `config set` has no
bash equivalent, and reaching it from ~25 read-only skills makes both a
`token_cmd` write and an arbitrary out-of-project delete prompt-free.

### Cross-Cutting Themes

- **Premises asserted as settled that the tree contradicts** (flagged by:
  architecture, security, correctness, compatibility, performance) — Phase 0's
  `cli/Cargo.toml` source-checkout probe (present in every marketplace install —
  `marketplace.json` declares a git-clone source, and the file is 2.1K in the
  live cache); "the six migrations" (there are seven, and `0007` reads config
  transitively through `doc-type-table.sh`); `template` as a scalar subcommand
  (it emits a fenced multi-line file body); the `Bash(cmd *)` space-separated
  rule shape (zero precedent among the tree's 84 Bash rules); the operator-aware
  matcher (this repo's own research at
  `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md:83-88`
  concluded a literal prefix/glob match); and `config-read-path.sh` avoiding
  VCS-detection overhead (it sources `vcs-common.sh` at `:19-23` and `exec`s into
  a script that calls `find_repo_root` twice). Each was recorded as resolved
  research rather than an open question, so each is excluded from the probe that
  would have caught it.

- **The same specification given two incompatible ways** (flagged by:
  architecture, code-quality, correctness, test-coverage) — the `config_command`
  pup rule allowlists `config_adapters` and then forbids it two paragraphs later;
  `compose()` is called from the dispatch arm while `main.rs` is claimed to be
  the only module naming `config_adapters`; Phase 2's fail-safe criterion
  requires `summary --fail-safe` to emit a notice block while divergence 5
  requires it to emit nothing; Phase 2's `--format=hook` criteria assert exit 0
  on unavailable without the flag while Phase 6 says the flag is required for
  exactly that. In each case the failure-open reading is the one an implementer
  is as likely to pick.

- **`--fail-safe` mandated universally, defined narrowly** (flagged by:
  architecture, correctness, test-coverage) — `check-skill-permissions.sh` is
  specified to assert the flag on *every* `bin/accelerator config` invocation in
  a `!` block, but divergence 5 defines degraded behaviour for only seven
  subcommands. ~26 `!` sites across 20 files invoke `dump`, `paths`, `work` and
  `templates list`, whose degraded shape is unspecified — and `work` and
  `paths --doc-types` are simultaneously two of the three deliberate *fail-closed*
  exceptions. A build gate will force the flag onto call sites where its meaning
  is undecided, and `list-work-items/SKILL.md:30` already documents empty output
  as "no integration configured".

- **The bootstrap remains outside every guarantee the plan makes** (flagged by:
  safety, compatibility, performance) — this was pass 1's top critical and pass
  2 noted it as partially addressed; it is still unmodelled. `--fail-safe` is a
  *launcher* flag parsed after `exec` at `bin/accelerator:194`, while the
  bootstrap has eleven distinct `fail()` paths above it — including resolving
  curl/wget unconditionally at `:47-62` **before** the cache-hit check at `:178`,
  so a host with a warm verified cache but no downloader cannot run at all. After
  Phase 6 this is also the blocking, once-per-session path, with a 300s fetch
  timeout in front of session start and a 30s lock wait that is ten times
  shorter than the fetch it waits on.

- **New gates that cannot fail** (flagged by: test-coverage, safety,
  performance) — the deletion-ledger replay is satisfied by a gate deleted in the
  same commit; the parity gate is the only new gate with no known-positive floor,
  so a missed binding leaves the suite silently exercising bash; `check-inventory`'s
  floor for member 4 is derived from the extractor it is meant to check; the
  `--explain` criterion is satisfied by any single byte on stderr — the exact
  tautology divergence 5 diagnoses two pages earlier; and the migration
  conversion's stub-fails-always criterion can only ever prove the first of eight
  sites was converted.

- **The permission widening creates capability, not just reach** (flagged by:
  security, safety) — `config set` has no bash counterpart today; `skills/config/configure`
  uses the Edit tool. Making it reachable under `config *` from ~25 read-only
  skills — several of which ingest PR diffs, Jira/Linear issue bodies and
  arbitrary repository files — yields two prompt-free primitives: writing
  `*.token_cmd`, which `jira-auth.sh:118` later executes via `bash -c`, and
  `config set templates.plan <abs-path>` followed by `templates reset --confirm`,
  which reaches an unguarded `rm` at `config-reset-template.sh:92`.

### Tradeoff Analysis

- **Parity vs. capability containment**: the plan's default is "reproduce the
  bash exactly", which is right for output but wrong for reach. The out-of-project
  `templates.<key>` value was safe when only a human could author it; it is not
  safe once `config set` is model-reachable in the same grant. Recommendation:
  keep byte-parity for rendering, but treat *reach* as a separate axis — split
  the permission rule into a read-only set for the ~25 read-only skills and
  confine `set`, `init`, `templates eject|reset` to `config/configure` and
  `config/init`. `check-skill-permissions.sh` is already being written and can
  enforce the split mechanically.

- **Uniform fail-safe vs. fail-closed validation**: the build gate wants one
  simple rule (the flag is always present); the three fail-closed subcommands
  want validation refusals to stay loud. Recommendation: make `--fail-safe`
  degrade *read/IO* failures only, leaving validation refusals fail-closed
  regardless — that keeps the universal gate honest and preserves the
  `work.integration` guard, but it must be stated, because the plan currently
  implies the opposite for `work`.

- **Mechanical rewrite vs. latency**: the performance lens shows the plan's own
  cost model makes the 25% budget arithmetically unsatisfiable while lever 1 is
  deferred (the re-verification term alone is 187–391ms against an 85–128ms
  allowance), and that lever 2 (batching) dominates lever 1 on precisely the two
  skills the budget is measured against. Recommendation: pull lever 1 into Phase
  0 or move the measurement to the end of Phase 4 and make it a Phase 5 *entry*
  precondition — the current placement discovers a predictable regression after
  the irreversible step.

- **Self-relative budget vs. standing guard**: a ratio to a bash baseline is the
  right instrument for the cutover and worthless afterwards, since Phase 7
  deletes the comparator. Recommendation: keep the ratio as the gate but commit
  the absolute aggregate and the launcher byte size as artefacts, and add a
  size regression check — the plan's own model makes size a per-call latency term.

### Findings

#### Critical

- 🔴 **Architecture / Security**: The Phase 0 override gate's load-bearing factor
  is satisfied in every install
  **Location**: Phase 0 §1: Local-build override, constraint 2
  `.claude-plugin/marketplace.json` declares `"source": "url"` against the repo's
  git URL at a tag, so a marketplace install is a full clone. Verified directly:
  `~/.claude/plugins/cache/.../1.24.0-pre.13/cli/Cargo.toml` exists (2.1K). The
  three-factor gate therefore collapses to two environment variables plus a file
  under the gitignored `cli/target/`, which anything holding the Bash tool can
  create. After Phase 5 this path is pre-authorised in 35 SKILL.md files and after
  Phase 6 runs unattended at every SessionStart. The Phase 0 criterion asserting
  refusal "in a marketplace install" would pass against a synthetic fixture while
  being wrong in the field.

- 🔴 **Security**: `config set` is a net-new pre-authorised RCE primitive via
  `*.token_cmd`
  **Location**: Phase 3 §1; Phase 5 §1
  `jira-auth.sh:118` and `linear-auth.sh:124` execute `token_cmd` with
  `bash -c "$cmd"`. `config set` has no bash equivalent today. Under
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)` in ~25 read-only skills,
  `accelerator config set linear.token_cmd '<payload>'` becomes prompt-free, and
  the next integration operation executes it. The payload needs no `/`, so this
  holds under either outcome of the Q1 wildcard probe.

- 🔴 **Security / Safety**: `config set` plus the deliberately-permitted
  out-of-project template value yields arbitrary file deletion
  **Location**: Phase 3 §2
  The plan keeps the bash's out-of-project `templates.<key>` allowance, justified
  as "a user who configured that path asked for it" — a justification that assumed
  hand-authoring. `config-reset-template.sh:92` is an unguarded `rm` on the
  resolved path under `--confirm`. Validating the template *name* constrains
  neither the value nor the delete.

- 🔴 **Safety / Compatibility / Performance**: The bootstrap sits outside every
  fail-safe guarantee, and Phase 6 puts it on the blocking session path
  **Location**: Phase 5 §5; Phase 6 §2; Migration Notes
  `--fail-safe` is parsed by the launcher, after `exec` at `bin/accelerator:194`.
  Eleven `fail()` paths precede it, including downloader resolution at `:47-62`
  which runs *before* the cache-hit check at `:178` — so a warm, verified cache
  plus no curl/wget is a total outage for all 46 skills. Recurring from pass 1.

- 🔴 **Safety / Code Quality**: `atomic_write`'s signature cannot express its own
  mode policy, and the credential case is the one not asserted
  **Location**: Phase 1 §1
  The committed signature carries no mode parameter, yet four policies are
  required (0600 clamp, umask-derived, preserve, corpus default). The criteria —
  "existing mode survives; a newly created `config.local.md` is 0600; a
  pre-existing 0600 file is still 0600" — are all satisfied by pure preservation.
  The case the prose says matters, a pre-existing **0644** `config.local.md`,
  has no criterion, so tokens land world-readable.

- 🔴 **Correctness**: There are seven migrations, not six, and `0007` is
  structurally barred from the prescribed fix
  **Location**: Key Discoveries; Phase 2 §1; Phase 5 §4b
  Verified: `skills/config/migrate/migrations/` holds seven files. `0007:19`
  sources `doc-type-table.sh`, which spawns `config-read-doc-type-paths.sh`. That
  script does not assert legacy layout today, so divergence 1's uniform gating
  newly refuses it — but the call site is in a shared helper, and
  `check-call-site-migration.sh` fails the build if `--allow-legacy-layout`
  appears outside `migrations/`. The recorded read counts are also wrong (×5/×5,
  not ×3/×4).

- 🔴 **Compatibility**: Eight failure-suppressing reads outside the migrations
  receive none of the structural conversion
  **Location**: Phase 5 §4b
  The suppression grep was run over the migrations only. The same shapes appear at
  `write-visualiser-config.sh:64,65,91,185,217,263,267` and
  `launch-server.sh:110`. Worst: the tickets→work pre-flight guard at `:64-66`
  branches on `[ -n "$TICKETS_OVERRIDE" ]`, so an unread config makes the guard
  *not fire* and the visualiser launches with the empty kanban it exists to prevent.

- 🔴 **Safety**: No stated criterion can prove more than one of the eight
  migration sites was converted
  **Location**: Phase 5 §4b and Success Criteria
  A stub that fails on every invocation dies at the first converted read, so sites
  2..N are never reached; the Nth-invocation criterion asserts only that *some*
  site fired. A site left as `|| true` still turns a read failure into "nothing to
  migrate", the migration exits 0, `run-migrations.sh:655` records it applied, and
  there is no `--unapply`.

- 🔴 **Compatibility**: Grep A's "exactly 0" is unachievable under basename
  matching
  **Location**: Phase 5 §5; Phase 5 Success Criteria
  Basename matching now sees ~27 references in files that survive: `CHANGELOG.md`
  (15 hits, an immutable record), `config-defaults.sh` (8 comment hits),
  `config-common.sh:315,394`, `config-read-browser-executor.sh:6`, and
  `visualise/server/src/config.rs:823`. None is on the exclude list. The companion
  criterion — "no retained file references a removal-set member" — is falsified
  outright by `CHANGELOG.md`.

- 🔴 **Compatibility**: The `allowed-tools` rule shape has zero precedent and is
  excluded from the probe as "settled"
  **Location**: Phase 5 §1
  All 84 `Bash(...)` rules in the tree are either bare command names or a path
  glob with `*` appended **directly**, no space, no colon. Claude Code's docs show
  `Bash(npm run test:*)`. The plan rewrites 35 blocks to the unprecedented
  space-separated form and treats the far less consequential `*`-spans-`/`
  question as the blocking one. If it does not match, all 46 skills throw a prompt
  per config call — and `check-skill-permissions.sh`, written to the same
  assumption, would agree the rule covers it.

- 🔴 **Architecture / Correctness / Test Coverage**: `--fail-safe` is mandated on
  every `!` invocation but undefined for most, and collides with fail-closed
  **Location**: Phase 5 §5; divergence 5; Phase 2 Success Criteria
  ~26 `!` sites invoke `dump`, `paths`, `work`, `templates list`, whose degraded
  shape is unspecified. `work` and `paths --doc-types` are simultaneously
  deliberate fail-closed exceptions, and the fail-closed criteria are stated only
  *without* the flag. Under scalar-suppression a bad `work.integration` degrades
  to empty + exit 0, which `list-work-items/SKILL.md:30` documents as "no
  integration configured".

- 🔴 **Correctness**: `template` is classified as scalar but emits a fenced
  multi-line file body
  **Location**: Phase 2 §4; Phase 2 Success Criteria
  `config-read-template.sh:36-60` emits the entire resolved template wrapped in
  markdown fences (or verbatim if already fenced, `:41-43`), invoked from 19 `!`
  sites. The scalar criterion ("exactly the value plus one `\n`") is unsatisfiable,
  and because goldens are specified only for the Block list, the second-highest-volume
  prompt-injected renderer ships with no byte-exact coverage.

- 🔴 **Architecture / Code Quality**: The `config_command` pup rule is specified
  twice with contradictory allowlists
  **Location**: Phase 2 §2
  One paragraph permits `config_adapters`; two paragraphs later it is explicitly
  excluded, with the reason the seam exists. Both are in the same block. The
  permissive reading lets a handler call `FileConfigStore::at(...)` while the check
  stays green. The rule must also key on `^accelerator::config_command($|::)` to
  match the file's existing convention.

- 🔴 **Architecture / Code Quality**: `compose()` cannot deliver the two new
  ports, and composition moves into the dispatch module
  **Location**: Phase 2 §2
  `compose.rs:19-26` returns only `ConfigService<FileConfigStore, FileConfigStore>`,
  consuming the store — so it cannot hand back the `ReadProjectContent` /
  `ScaffoldProject` handles the arm is meant to pass on. And calling it from
  `launch/mod.rs` contradicts "main.rs is the only launcher module that names
  `config_adapters`", inverting the injection shape `dispatch` uses for every
  other collaborator.

- 🔴 **Code Quality**: `From<WriteError> for ConfigError` has no legal home — the
  same defect the plan solves one crate over
  **Location**: Phase 1 §2
  Inside `config` it needs `config` to import `store`, denied by
  `config_domain_imports_only_permitted` (pup.ron:42-56); inside `config-adapters`
  both types are foreign (E0117); inside `store` it contradicts the declared
  dependency set. The plan reasons this through correctly for corpus and lands on
  `to_store_error`, then does not apply it here.

- 🔴 **Test Coverage / Safety**: The deletion-ledger replay is satisfied by a gate
  deleted in the same commit
  **Location**: Phase 7 Success Criteria
  For most removal-set entries the covering gate is the repointed `test-config.sh`,
  which Phase 7 deletes. The replay asserts the gate was green *at or before* the
  deleting commit, which is true. Nothing requires the ~337 assertions to be ported
  first — the behaviour inventory is scoped to the four *non-repointable* members
  only. It is also the one new gate with no known-positive proof.

- 🔴 **Test Coverage**: The parity gate is the only new gate without a
  known-positive floor
  **Location**: Phase 4 §2 and Success Criteria
  "`test-config.sh` passes with every binding repointed" is satisfiable with an
  arbitrary subset repointed — a missed binding leaves the suite silently
  exercising the bash script and passing. The rebind list is hand-enumerated across
  seven scattered ranges plus two bare-exec sites, which is exactly the shape that
  loses one.

- 🔴 **Test Coverage**: A family of consumer-census invariants degrades to vacuous
  passes at the cutover
  **Location**: Phase 2 §6 (inventory member 1)
  Member 1 omits `test-config.sh:2543-2577`, `:3287-3316`, `:4056-4083`,
  `:4085-4114`, `:4116-4143`, `:4145-4164` — guards against hardcoded inline
  defaults at call sites, unregistered catalogue keys, and missing `allowed-tools`
  coverage. Most grep for the *old* script name, so after Phase 5 they match nothing
  and pass rather than fail. They will not surface by running the suite.

- 🔴 **Performance**: The plan's own cost model makes the 25% budget
  arithmetically unsatisfiable
  **Location**: Performance Considerations; Phase 7 Manual Verification
  17 calls × 20-30ms bash = 340-510ms, so 25% grants 85-128ms. The deferred
  re-verification term alone is 11-23ms × 17 = 187-391ms, before bash startup, the
  ~10 forks per warm bootstrap, or the launcher's own work. The measurement is
  placed in Phase 5 with no stated ordering against the 247-site rewrite, so a
  predictable, already-quantified regression is discovered after the irreversible
  step.

- 🔴 **Safety**: The release pipeline's partial-publish window defeats the
  standing gate
  **Location**: Phase 5 Success Criteria
  `tasks/release.py:69-76` commits and pushes the version bump and tag *before*
  `upload_and_verify_release`. A pre-publish gate can only inspect locally staged
  artefacts. If upload fails after the push, `plugin.json` on `main` names a
  version `bin/accelerator` has nothing to fetch for — the exact "released
  half-migrated contract" the plan says must never ship.

#### Major

- 🟡 **Architecture / Correctness**: `--allow-legacy-layout`'s source-fallback
  half has no implementation site
  **Location**: Phase 2 §1 and §2
  Only `compose(cwd, LegacyPolicy)` is specified, and the justification is
  guard-scoped. The fallback lives in `FileConfigStore::level_path`
  (`store.rs:51-56`), which hard-codes the two current paths and is not listed as
  changing. An implementer following the wording wires the guard only, and
  migrations resolve absent and proceed on defaults.

- 🟡 **Safety**: The legacy-fallback proof covers only two of the keys the
  migrations read
  **Location**: Phase 5 Success Criteria
  The fixture carries non-default `paths.tickets` and `work.id_pattern`, read only
  by `0001` and `0002`. For `0004`/`0005`/`0006` the criterion degrades to "runs
  green", which the plan itself declares insufficient. `config-read-path.sh:31-54`
  returns the *catalogue default* when `$2` is omitted, so a failed fallback yields
  `meta/plans` — non-empty — and every empty-check guard the plan relies on never
  fires.

- 🟡 **Safety**: `--allow-legacy-layout` is global across the `config` tree, so
  `config set` can orphan a whole legacy configuration
  **Location**: Phase 2 §1
  The flag is accepted by mutating subcommands. `config set --allow-legacy-layout`
  reads from `.claude/accelerator.md` and writes to `.accelerator/config.local.md`,
  which permanently disengages the fallback condition. `.claude/accelerator.local.md`
  is gitignored, so unlike the team file it is not VCS-recoverable.

- 🟡 **Correctness**: The prescribed 2-vs-1 return code is unobservable at the
  caller shape the plan names
  **Location**: Phase 5 §4b
  `0006:298` is `if ! rel="$(resolve_corpus_path "$key")"` — inside the `then`
  branch `$?` is the `!` pipeline's status, not the function's 1 or 2. The callee
  contract is specified without the required caller reshape.

- 🟡 **Correctness**: The fail-safe criterion contradicts divergence 5 for
  `summary` and leaves `instructions` unspecified
  **Location**: Phase 2 Success Criteria
  "Each injection command with `--fail-safe` … stdout exactly `## Agent Names
  Unavailable`" includes `summary`, which divergence 5 says emits nothing. Three
  headers are named for four notice-rendering commands.

- 🟡 **Correctness**: `--format=hook`'s three states are a property of the flag in
  Phase 2 and of flag-plus-`--fail-safe` in Phase 6
  **Location**: Phase 2 Success Criteria vs Phase 6 §1-2
  Implementing Phase 2 verbatim either fails against correct behaviour or is
  "repaired" by making `--format=hook` imply fail-safe — reintroducing exactly the
  coupling Phase 6 forbids.

- 🟡 **Architecture**: `permitted_root` has no production supplier on the corpus
  side
  **Location**: Phase 1 §2
  `FileCorpusStore` is constructed only in tests. The stated construction-site
  ripple is test-only, and those tests will pass the same `TempDir` they write
  into, making containment trivially satisfied. `append_record` also
  `create_dir_all`s and acquires a lock directory before the guard runs.

- 🟡 **Safety**: The containment guard's two stated holes are not reachable from
  the committed design
  **Location**: Phase 1 §1-2
  "Refuse a permitted root that is itself a symlink resolving outside the project
  root" — but `atomic_write` receives no project root, and corpus has none defined.
  And `corpus-adapters/src/store.rs:59,111-113` already `create_dir_all`s outside
  `atomic_write`, so the chain is built through the symlink before the check.

- 🟡 **Architecture**: Moving config temps into `.accelerator/` leaves existing
  repos with no ignore rule
  **Location**: Phase 1 §2; Phase 3 §3
  The writer in real repos is `accelerator_ensure_inner_gitignore`
  (`accelerator-scaffold.sh:19-27`), which appears on no phase's change list and
  writes only when the file is absent. An already-initialised repo never re-runs
  `init`, so from Phase 5 every config write stages an un-ignored temp under jj
  auto-snapshot.

- 🟡 **Architecture / Code Quality**: The two driven ports are low-cohesion, and
  `ScaffoldProject` also deletes
  **Location**: Phase 2 §2
  `ReadProjectContent` bundles five unrelated subcommand needs behind one name —
  an omnibus read trait split from the write side rather than by responsibility.
  `ScaffoldProject` covers `init`'s tree, `eject`'s write **and** `reset`'s delete;
  a reader looking for the destructive operation will not look there.

- 🟡 **Architecture**: Domain rules live in the inbound module
  **Location**: Phase 2 §2
  The ports are declared in `config` but consumed by nobody there, so template
  resolution precedence, `init`'s scaffold contract, skill-context resolution and
  `summary`'s enumeration rule all land in `config_command` — making the most
  branching subcommands testable only through the spawned binary.

- 🟡 **Code Quality**: The largest body of new logic has no named home
  **Location**: Phase 2 §2 and §4
  Six render modules are named; nothing is allocated for the view assembly they
  render — the review aggregate, `dump`'s attribution, the agents table, the 13
  doc-type resolutions, `summary`'s enumeration. By the plan's own account that
  includes "a ~500-line `review` port". The path of least resistance puts it in the
  render modules, reintroducing the impurity `Rendered` exists to remove.

- 🟡 **Code Quality**: The new ports return `ConfigError`, whose `Io` Display says
  "config file"
  **Location**: Phase 2 §2
  `error.rs:84-86` hard-codes `"I/O error on config file '{path}'"`. Under the new
  ports that renders lens directories, skill files, template overrides and `init`'s
  14 directories. Divergence 10's mitigation is explicitly that "the error must name
  the offending file, so a wide failure is diagnosable in one read".

- 🟡 **Code Quality**: `FileCorpusStore::new()` gaining a root breaks its `Default`
  impl
  **Location**: Phase 1 §2
  `store.rs:36-40` delegates to `new()` and cannot survive a required parameter.
  The plan's framing — "recorded here so the ripple is not mistaken for scope
  creep" — is what makes the omission worth flagging.

- 🟡 **Code Quality**: `subcommand_required` alone does not produce
  `DisplayHelpOnMissingArgumentOrSubcommand`
  **Location**: Phase 2 §3
  In clap 4 that kind comes from `arg_required_else_help`; `subcommand_required`
  yields `MissingSubcommand`. Under the plan's interception a bare
  `accelerator config` would exit 1, contradicting its own criterion. Replacing
  `error.exit()` also drops clap's stdout/stderr choice.

- 🟡 **Code Quality**: The pup rule combines `allowed_only` and `denied` with no
  precedent, and `^crate` already admits what `denied` forbids
  **Location**: Phase 2 §2
  All five existing rules use `denied: None`. Precedence between the two clauses is
  unestablished in this tree, so the deny half may be silently vacuous — while
  Phase 7's dependency check is described as relying on it.

- 🟡 **Compatibility**: Grep A excludes `cli/` while its floor requires two
  references inside `cli/`
  **Location**: Phase 5 §5 vs Success Criteria
  `catalogue.rs:301,325` and `corpus-adapters/tests/common/mod.rs:66` are both
  under `cli/`. The two criteria are mutually unsatisfiable, and `cli/` need not be
  excluded — Phase 7 §3 already repoints all four Rust dependants.

- 🟡 **Compatibility**: The operator-awareness claim contradicts this repo's own
  matcher research, and a check omission is derived from it
  **Location**: Phase 5 §1
  `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md:83-88,118-126`
  concluded a literal prefix/glob match stripping only recognised wrappers. That is
  incompatible with per-segment splitting. The same research records an unresolved
  report that `allowed-tools` may be enforced only on the first matching Bash call
  per session — directly relevant to skills making 17 config calls at load.

- 🟡 **Compatibility**: Phase 2 changes 0164's shipped launcher-wide exit-code
  contract, contradicting the phase-independence claim
  **Location**: Implementation Approach vs Phase 2 §3
  The plan states phases 1-4 leave user-visible behaviour unchanged, then states
  the opposite about Phase 2 itself. A release cut between phases 2 and 5 ships a
  changed usage-error contract with no call site yet exercising it.

- 🟡 **Test Coverage**: Nothing asserts the divergence note's named tests exist,
  and Phase 7 may delete three of them
  **Location**: Phase 2 §6; Phase 7 §3
  Divergences 10-12 name `parity.rs:256/278/296`; Phase 7 says "delete the
  differential suite or repoint it". The stated escape — the `bash-parity` feature
  — does not exist in `config-adapters/Cargo.toml`, and CI runs `--all-features`.

- 🟡 **Test Coverage**: `check-inventory`'s floor is self-derived for the member
  the work item calls the likeliest silent loss
  **Location**: Phase 2 §6
  Member 4's floor is committed from the extractor's own output. A member-4 list
  missing three scripts yields a smaller extraction that a correspondingly smaller
  inventory satisfies.

- 🟡 **Test Coverage**: The two-hop suite-audit generator misses three-hop suites,
  and its floor only defends one-hop→two-hop
  **Location**: Phase 2 §7
  `jira-common.sh` and `linear-common.sh` are sourced libraries pulled in by ~20
  `*-flow.sh` scripts, each with its own suite — three hops. The floor's transitive
  example (`test-write-visualiser-config.sh`) is a two-hop case, so a two-hop
  generator passes by construction.

- 🟡 **Test Coverage**: The per-phase Rust criterion is weaker than CI
  **Location**: Phases 1-3 Success Criteria
  `cargo test --workspace` omits `--all-features`, which is what enables
  `bash-parity` — gating three of the suites this plan must repair.

- 🟡 **Test Coverage**: Golden coverage is baseline-only for `agents`, `dump` and
  `applies_to` mode filtering
  **Location**: Phase 2 §5
  The argument that produced the custom-lenses fixture is not generalised. `dump`'s
  em-dash token masking, `agents`' fallback/warn/hyphen rules, and
  `config-read-review.sh`'s `applies_to` filtering (ten assertions at
  `test-config.sh:2010-2236`) are pinned only by loose `grep -q` checks in a
  deleted suite. The proposed custom-lenses fixture carries no `applies_to`, so its
  three mode goldens would be identical.

- 🟡 **Test Coverage**: The `--explain` criterion is the exact tautology the plan
  diagnoses two pages earlier
  **Location**: Phase 2 Success Criteria
  "Writes its diagnostic to stderr and leaves stdout byte-identical" is satisfied by
  any single byte, or by the diagnostic the default already emits.

- 🟡 **Test Coverage**: The SKILL.md censuses are rewritten in Phase 5 and deleted
  in Phase 7 with no stated successor
  **Location**: Phase 5 Success Criteria; Phase 7 §2
  They assert injection presence in exactly 42 skills, ordering, last-line
  placement, and skill-name/frontmatter agreement. Inventory member 1 says they map
  to Rust tests; Phase 5 says they are rewritten in place. The dispositions are
  incompatible, and the `context` collapse makes the ordering invariant newly
  fragile.

- 🟡 **Security**: The risk acceptance for the broad grant rests on a false
  trust-boundary claim
  **Location**: Phase 5 §1
  "The content that could steer such a call arrives only through reviewed commits"
  is untrue of `review-pr`, `respond-to-pr`, `show-jira-issue`,
  `search-jira-issues`, the Linear equivalents, `extract-work-items` and
  `research-issue` — all of which carry the rule and ingest attacker-controllable
  text. The "10 declare bare `- Bash`" argument does not transfer: those are the
  integration write skills, not the ~25 read-only ones the widening affects.

- 🟡 **Security**: The tracked-marker rejection refutes the option actually adopted
  **Location**: Phase 0 §1
  The objection — a marker "would have to be committed … permanently satisfying the
  second factor for every clone" — applies verbatim to `cli/Cargo.toml`. The
  obvious variant, an **untracked, gitignored** marker anchored to
  `CLAUDE_PLUGIN_ROOT`, is never considered and is strictly stronger.

- 🟡 **Security**: Factor 3's containment check is under-specified where Phase 1's
  equivalent is explicit
  **Location**: Phase 0 §1 and Success Criteria
  Phase 1 spells out canonicalisation ordering, component-wise comparison and
  symlinked-ancestor refusal; Phase 0 states none of it. macOS ships no
  `readlink -f` and the repo enforces a bash 3.2 floor, so the likely implementation
  is a string-prefix test that a symlinked intermediate directory defeats.

- 🟡 **Security**: `ACCELERATOR_CACHE_DIR` stays ungated while the new variable
  gets three factors
  **Location**: Phase 0 §2
  That variable relocates both the verify shim and the launcher to a caller-chosen
  directory. The temp-then-`mv` fix closes the torn-file race but not the
  stage→exec window, in which the shim can be replaced by a stub that exits 0.

- 🟡 **Security**: No guard against writing credential keys to the tracked team
  config
  **Location**: Phase 3 §1
  Nothing refuses `config set jira.token <secret> --level team`. `.accelerator/config.md`
  is committed; the whole point of the split is that credentials live in
  `config.local.md`.

- 🟡 **Performance**: The latency measurement is unperformable as specified
  **Location**: Phase 5 Success Criteria vs Phase 7 measurement spec
  With overrides unset it fetches the launcher for the version in `plugin.json`.
  The stated entry gate only requires artefacts at "the currently published
  version", which may predate `config` — in which case `config path` falls through
  to `external_subcommand` and fails.

- 🟡 **Performance**: A purely self-relative budget leaves no standing guard once
  the comparator is deleted, and the aggregate framing buys nothing it claims
  **Location**: Performance Considerations
  With no cross-invocation reuse, cost is linear in call count, so the aggregate
  ratio equals the per-call ratio. The aggregate would add information only as an
  *absolute* bound, which the plan declines to state.

- 🟡 **Performance**: SessionStart latency is not accounted, and the hook gains an
  unbounded cold-path network dependency
  **Location**: Phase 6 Success Criteria
  Today the hook is ~3 spawns with no network. After Phase 6 the first session
  after every version bump serialises a 10-25MB fetch, bounded at `--max-time 300`,
  in front of session start. `--fail-safe` cannot cover it.

- 🟡 **Performance**: The 30s lock wait is ten times shorter than the 300s fetch it
  waits on
  **Location**: Phase 0 Success Criteria
  A skill load fires up to 17 bootstrap invocations; on a cold cache one fetches
  while the rest spin, and any fetch over 30s makes every waiter `fail`. Phase 0's
  concurrency criterion is explicitly scoped to a **warm** cache.

- 🟡 **Performance**: Lever 2 dominates lever 1 on the two skills the budget is
  measured against
  **Location**: Performance Considerations
  14 and 13 consecutive `config path` calls respectively; collapsing 14 spawns to 1
  removes ~13 full bootstrap cycles, whereas lever 1 removes only the verify term
  from each. Lever 2's real cost is that it changes injected prompt text, not that
  it is unimplemented — `config paths --all` lands in Phase 2.

- 🟡 **Architecture**: A Phase 7 criterion asserts a crate boundary the design does
  not create
  **Location**: Phase 7 Success Criteria
  `config_command` is a module inside `launcher`, which depends on `reqwest` and
  `rustls`. No `cargo tree` check can express "the config subcommand's crate graph
  contains no HTTP dependency"; the next criterion half-concedes this.

#### Minor

- 🔵 **Correctness**: `init` does not use the path catalogue today — switching to
  `catalogue::default_for` is a source-of-truth change, not a reproduction
  **Location**: Phase 3 §3
  `init.sh:36` passes explicit non-empty defaults from its own `DIR_DEFAULTS`, which
  `config-read-path.sh:31-33` makes win over the catalogue. The values coincide
  today; `config-defaults.sh:14-16` records the vocabularies as deliberately
  un-unified.

- 🔵 **Correctness**: The doc-type blank-value coercion and its stderr note are not
  in the behaviour list
  **Location**: Phase 2 §4
  `config-read-doc-type-paths.sh:87-91` coerces a blank value to the registry
  default and always emits 13 rows. `0007:814-819` reasons from exactly that
  invariant for its fail-closed net.

- 🔵 **Correctness**: The `context` separator rule is defined over blocks, but
  degradation adds notice-shaped members with unspecified ordering
  **Location**: Phase 2 §4
  These bytes are byte-exact-goldened and injected into 42 prompts, so an
  unspecified ordering means the golden pins whatever was implemented first.

- 🔵 **Compatibility**: The existing `DisplayHelp` interception renders top-level
  help for every subcommand `--help`
  **Location**: Phase 2 Success Criteria
  `main.rs:102-108` rebuilds `Cli::command()` unconditionally, discarding clap's
  matched-subcommand help. The help-snapshot criterion cannot pass without an
  unplanned change to shared help dispatch.

- 🔵 **Compatibility**: Divergence 13's premise is derived from script source,
  never captured from the preprocessor
  **Location**: Divergence 13; Phase 2 §4
  In all 42 files the two `!` calls occupy separate lines, so whether the rendered
  output already carries a blank line depends on undocumented preprocessor
  behaviour. If it does, divergence 13 is actually parity.

- 🔵 **Compatibility**: The SessionStart envelope moves from jq pretty-printed to
  compact without being recorded
  **Location**: Phase 6 §1
  `config-detect.sh:18` omits `jq -c`. Any of the seven repointed `CONFIG_DETECT`
  assertions matching on layout fails for a formatting reason.

- 🔵 **Compatibility**: The v2.1.144 floor is prose-only and triplicated
  **Location**: Phase 5 §1
  Declared in `docs/releases-and-compatibility.md:36`, `CLAUDE.md:121` and
  `ADR-0051:117`, with nothing machine-readable and no coherence check — so a probe
  result that moves the floor has nowhere to land.

- 🔵 **Test Coverage**: Wiring `test-init.sh` into CI needs three edits, and the two
  offered options are not equivalent
  **Location**: Phase 4 §4
  A new invoke task needs a `mise.toml` entry and an addition to the
  `test:integration` aggregate. The claim that `_EXPECTED_CONFIG_SUITES` is
  unaffected holds only under the dedicated-task option. Phase 7 also never removes
  `_EXPECTED_INIT_SUITES`.

- 🔵 **Test Coverage**: `jj show` does not print file contents, and the audit corpus
  count is wrong
  **Location**: Phase 2 §6-7
  The content-at-revision command is `jj file show -r <rev> <path>`. And
  `integration.py` has **eight** `run_shell_suites` call sites, not nine — the ninth
  exists only after Phase 4, so `test-init.sh` appears in no pinned audit row.

- 🔵 **Test Coverage**: Repointing `doc_type_table()` at the binary has no stated
  resolution mechanism
  **Location**: Phase 7 §3
  `CARGO_BIN_EXE_accelerator` is only injected for `launcher` integration tests, so
  `corpus-adapters` cannot use it; a path-based resolve is build-order dependent.

- 🔵 **Test Coverage**: Dropping the drift test's runtime cross-check removes a
  declared-vs-runtime assertion with no replacement
  **Location**: Phase 7 §1
  `catalogue.rs:321-332`'s `M` lines deliberately overwrite the `K` lines, so the
  assertion is what the renderer *emits* against the declared default — catching a
  hardcoded `min_lenses`. The declarative comparison does not.

- 🔵 **Security**: The 0600 clamp is asserted only where it changes nothing
  **Location**: Phase 1 Success Criteria
  See the critical above. Relatedly, `jira-auth.sh:189-192` refuses a *symlinked*
  `config.local.md`; nothing gives the Rust reader the same refusal.

- 🔵 **Security**: The override's only detection control is invisible on both paths
  where it matters
  **Location**: Phase 0 §1
  The stderr warning is unseen at SessionStart (stderr not presented) and at the 247
  `!` sites (whose `--fail-safe` contract routes diagnostics to stderr precisely so
  they do not reach the prompt).

- 🔵 **Security**: `dump` redacts credential keys but `get` does not
  **Location**: Phase 2 §4
  At parity with today, but this is the point at which the credential-key set
  becomes a first-class concept and the inconsistency becomes cheap to close.

- 🔵 **Architecture**: The permission checker forbids a rule form strictly narrower
  than one it accepts
  **Location**: Phase 5 §3 and §5
  It rejects ancestor globs while §3 accepts bare `- Bash` in 10 files, which grants
  strictly more — leaving a legal escape worse than what was rejected.

- 🔵 **Architecture**: A per-subcommand semantic is placed in the cross-cutting
  kernel taxonomy
  **Location**: Phase 2 §3
  `kernel` documents itself as cross-cutting, and exit 2's meaning is stated to hold
  for built-ins only — while 0168/0169/0173 all add clap-based sub-binaries whose
  default is exit 2.

- 🔵 **Code Quality**: The code sketches would not pass `cli:check`
  **Location**: Phase 1 §1; Phase 2 §3
  `WriteError` carries no derives, no `#[non_exhaustive]`, no Display — and since
  `thiserror` is not in the declared dependency set, Display must be hand-written as
  `corpus::StoreError` does. `atomic_write` has no `# Errors` section
  (`clippy::missing_errors_doc`), and `Refusal` no `#[error]` attribute.

- 🔵 **Code Quality**: `Rendered` erases stream interleaving, so the ordering
  guarantee is not unit-assertable
  **Location**: Testing Strategy
  Two independent collections with no relative sequencing; ordering is a property of
  the handler, not the value.

- 🔵 **Code Quality**: Fail-safe notice rendering escapes the render modules
  **Location**: Divergence 5; Phase 2 §2
  The `## … Unavailable` blocks are byte-exact user-visible output produced on paths
  where the renderer was never reached, so they land in the handler — putting
  byte-exact output in two places per family.

- 🔵 **Code Quality**: The `Refusal` doc comment encodes a launcher-side fact in the
  kernel crate
  **Location**: Phase 2 §3
  "Maps to exit 2" lives in `main.rs`'s match arm; stating it in a 15-line
  authoritative taxonomy file is the staleness class this project bans.

- 🔵 **Safety**: `strip = true` was priced before the launcher was on the critical
  path
  **Location**: Phase 0 §3
  After Phase 7 there is no bash equivalent to diff against, so a field panic yields
  an unsymbolised backtrace with no comparator.

- 🔵 **Performance**: The 475KB shim is re-copied on every invocation, and Phase 0
  makes that path more expensive
  **Location**: Phase 0 §2
  Copy-to-temp-then-`mv` adds a third fork without removing the copy. Content-addressing
  the staged shim (the repo already commits
  `bin/accelerator-verify.vendored.sha256`) removes the write *and* makes staging
  atomic by construction.

- 🔵 **Performance**: Moving `install_crypto_provider` is sub-millisecond and does
  not address the size term it sits beside
  **Location**: Performance Considerations
  `tls.rs:9-12` builds a provider of static references into a `OnceLock` — microseconds.
  Moving the call does not unlink rustls/ring/reqwest.

- 🔵 **Performance**: The measurement does not fix the working directory
  **Location**: Phase 7 Manual Verification
  `find_repo_root` forks `dirname` per ancestor level and is called twice per bash
  invocation, while `discover_root` is depth-invariant. Across 17 calls the swing is
  easily enough to move the ratio by 2×.

#### Suggestions

- 🔵 **Performance**: The `config-read-path.sh` "avoids VCS-detection overhead"
  claim is wrong
  **Location**: Performance Considerations
  `:19-23` sources `vcs-common.sh` explicitly for `find_repo_root`, and `:75` `exec`s
  `config-read-value.sh`, which sources `config-common.sh` and walks twice.
  `config-read-all-paths.sh:5-7` states this directly. This cuts in the migration's
  favour, but the baseline should be measured, not recalled.

- 🔵 **Code Quality**: `--format` carries two unrelated value spaces
  **Location**: Phase 2 §4; Phase 6 §1
  `paths --format <block|tsv>` selects a rendering; `summary --format=hook` wraps in
  a transport envelope. Consider `summary --hook` as a boolean.

- 🔵 **Code Quality**: `install_crypto_provider()`'s relocation has no phase, file
  or criterion
  **Location**: Performance Considerations
  It appears only in prose, but it is a real `main.rs` control-flow edit that changes
  where a provider-install failure is reported.

- 🔵 **Code Quality**: `store`'s `kernel` dependency is likely unused and its
  framing is inconsistent
  **Location**: Phase 1 §1 and §4
  Both consumers translate into their own taxonomy first, so
  `From<WriteError> for kernel::Error` has no caller. §1 calls `store` a domain
  crate; §4 calls it infrastructure.

- 🔵 **Architecture**: Phase 6 still frames `--format=hook` as an exception to a
  decision Phase 2 replaced
  **Location**: Phase 6 §1
  Under Phase 2's revised rule it is an ordinary instance, not an exception.

### Strengths

- ✅ Every headline measurement in the Verified measurements table reproduces
  exactly against the tree: 247 `!` sites across 46 files, 35 `config-*`
  `allowed-tools` files, 297 Grep B hits, 31 non-SKILL.md referencers grouped
  precisely as listed, and the claim that all 42 context-calling files also call
  skill-context.
- ✅ The plan's analysis of the hardest bash behaviours is correct in fine detail
  and independently verified: the eject-vs-diff/reset exit-2 asymmetry and its
  `config_resolve_template` cause; the `get`/`path` default-precedence split
  including that `path` treats omitted and empty `$2` identically while `get` does
  not; the `config-read-review.sh:270` double slash and its non-propagation to the
  three warnings; and the `0006:298` subshell trap in exact mechanical detail.
- ✅ Moving Grep A to basename matching is the right correction and does catch the
  three reference classes a path-shaped pattern structurally cannot see.
- ✅ The `WriteError`/`corpus::StoreError` split is derived from a real constraint
  (`pup.ron:57-72` genuinely forbids corpus naming `store`), and the private
  `to_store_error` free function correctly anticipates the orphan rule rather than
  discovering it at compile time.
- ✅ The `permitted_root` refusal ordering — canonicalise the nearest *existing*
  ancestor, verify containment, then `create_dir_all` — correctly handles the
  first-write case, and refusing a symlinked root closes the git-ships-symlinks case
  a naive reading would miss.
- ✅ Parsing the skill name in the handler rather than via a clap `value_parser` is
  subtle, correctly motivated (a parser rejects outside the fail-safe boundary), and
  comes with a criterion that would fail if done the other way.
- ✅ Identifier validation for skill and template names closes a genuine traversal
  in `config-read-skill-context.sh:23`, and follows the codebase's own precedent at
  `config-read-doc-type-paths.sh:101-107` rather than setting new policy.
- ✅ Fail-safe as an explicit opt-in with the 28 shell consumers left loud is the
  correct posture for credential resolution, and directly prevents `jira-auth.sh`
  treating a degradation notice as a token.
- ✅ `Rendered { stdout, warnings }` is the right seam, and the three behaviours
  justifying it are real and cited to their bash lines.
- ✅ Correctly identifies the most easily-missed cost in the design — that
  `bin/accelerator:178`'s cache-hit test performs a *full* minisign verification, so
  the bootstrap is a per-call rather than first-use cost — and refuses the
  external-subcommand shape on that basis.
- ✅ Phase 0 fixes a real pre-existing shim-staging race before the call-site count
  moves from zero to 247, rather than discovering it under load.
- ✅ `paths --doc-types` buffering all 13 lines improves on the bash's
  partial-prefix-then-fail contract for a consumer whose values scope in-place
  mutation.
- ✅ The `config-defaults.sh` retention reasoning is correct and well-evidenced, as
  is the finding that the catalogue drift test is *not* saved by it.
- ✅ Divergences 10-12 name real, existing tests at the cited lines rather than
  asserting coverage that does not exist.
- ✅ Pre-committing the latency budget before measurement correctly avoids the
  satisfied-by-construction tautology the plan calls out elsewhere, and recording
  the levers in advance means a bad number arrives with a decision attached.

### Recommended Changes

1. **Replace Phase 0's source-checkout probe** (addresses: gate premise false,
   tracked-marker rejection, factor 3 under-specified). Adopt an **untracked,
   gitignored** marker anchored to `CLAUDE_PLUGIN_ROOT` — jj auto-snapshot cannot
   track it, no clone or install carries it, and it is not CWD-selectable. Specify
   factor 3's canonicalisation mechanism explicitly (bash 3.2, no `readlink -f`),
   and test the refusal against a pristine clone at a release tag rather than a
   synthetic fixture.

2. **Split the permission grant by capability** (addresses: `token_cmd` RCE,
   template-delete, false trust-boundary claim, checker inconsistency). A read-only
   rule for the ~25 read-only skills; `set`, `init`, `templates eject|reset`
   confined to `config/configure` and `config/init`. Independently, refuse
   `config set` on `*.token`/`*.token_cmd` and on `--level team` for those keys, and
   bound `templates reset`'s delete by the same `permitted_root` the writes now use.
   Correct the recorded rationale to name the untrusted-input skills.

3. **Fold the rule *shape* into the Q1 probe** (addresses: zero precedent,
   operator-awareness contradiction). Test `Bash(<path> config *)` against the
   colon and no-space spellings, and test `;`/`&&`/`$(…)` against this repo's own
   prefix/glob finding. Until it resolves, have `check-skill-permissions.sh` reject
   any `!` invocation containing a metacharacter — the 247 sites are clean today, so
   it costs nothing.

4. **Correct the migration set to seven and resolve 0007's structural bar**
   (addresses: seven-not-six, read counts). Decide how `doc-type-table.sh` conveys
   the policy — a parameter to `load_doc_type_table`, or a named exception in
   `check-call-site-migration.sh` — and extend every migration criterion to 0001-0007.

5. **Re-run the suppression grep over all 28 consumers and make the conversion
   provable per site** (addresses: eight unconverted sites, unobservable criteria,
   caller reshape). Commit the full site table; specify the caller shape
   (`rc=0; x=$(f) || rc=$?`) alongside the callee contract; and make the criterion
   per-site — a stub failing on exactly the k-th invocation, for every k. Add
   `--unapply <id>` to `run-migrations.sh` so a discovered half-application has a
   recovery path.

6. **Define the fail-safe matrix exhaustively over the subcommand set** (addresses:
   universal mandate vs narrow definition, fail-closed collision, `summary`
   contradiction, `--format=hook` contradiction). State that `--fail-safe` degrades
   read/IO failures only and leaves validation refusals fail-closed; give every
   `!`-reachable subcommand a degraded shape; restate the Phase 2 criteria to
   enumerate the four notice headers, `summary`'s suppression, and all three hook
   states as `--format=hook --fail-safe`.

7. **Reclassify `template` as a Block subcommand** (addresses: scalar
   misclassification) with goldens for all three resolution tiers plus the
   already-fenced and warn-and-fallback branches.

8. **Resolve the four Rust shapes that will not compile or will not enforce**
   (addresses: pup contradiction, `compose()`, `From<WriteError>`, `Default` impl,
   clap kind). Write the pup RON block out literally without `config_adapters`;
   specify `compose()`'s new return type and keep composition in `main.rs` behind an
   injected lazy composer; replace `From<WriteError>` with a private
   `to_config_error`; add the `Default` removal; set `arg_required_else_help` on the
   `config` subcommand.

9. **Widen `atomic_write`'s signature to carry its policy** (addresses: mode policy
   inexpressible, 0644 case untested, containment holes unreachable). Pass the mode
   policy and the trusted project root explicitly; add the criterion that a
   pre-existing 0644 `config.local.md` is 0600 after a personal write; and remove the
   adapter-level `create_dir_all` calls so directory creation happens inside the
   guard.

10. **Give the gates that cannot fail a way to fail** (addresses: ledger replay,
    parity floor, `check-inventory` member 4, `--explain`, suite audit, census
    invariants). Add a *gate exists at the final state* ledger column; require the
    repointed suite to pass with every removal-set script stubbed to exit non-zero;
    derive member 4's population as removal-set minus audit-covered rather than from
    the extractor; assert `--explain`'s provenance content on a discriminating
    fixture; make the audit generation transitive-closure with a three-hop suite in
    its floor; and extend inventory member 1 to every consumer-census region with a
    per-region disposition.

11. **Move the latency measurement to the end of Phase 4 and make it a Phase 5
    entry precondition** (addresses: budget unsatisfiable, measurement unperformable,
    no standing guard, CWD unfixed). Pull lever 1 into Phase 0 or accept that the
    budget fails; require a signed release *carrying `config`* for the measurement;
    fix the working directory in the committed script; and commit the absolute
    aggregate and launcher byte size, with a size regression check that survives
    Phase 7.

12. **Reconcile Grep A's corpus with its own floor and with the retained files**
    (addresses: exactly-0 unachievable, `cli/` exclusion contradiction). Drop `cli/`
    from the exclude list; split the pattern into functional-reference and mention
    forms and gate only on the former; add `CHANGELOG.md` and `docs/` to the exclude
    list, or rewrite the retained comments in the same commit.

13. **Model the bootstrap layer explicitly** (addresses: outside fail-safe, session
    path, lock/fetch mismatch, partial publish). Move downloader resolution into the
    cache-miss branch; give the bootstrap an injection-safe mode or state plainly in
    Migration Notes that bootstrap failure is a hard outage and add fault-injected
    criteria; raise the lock ceiling above the fetch timeout and test cold-cache
    concurrency; and reorder `tasks/release.py` so artefacts are verified before the
    version bump is pushed.

14. **Assign the unassigned** (addresses: legacy-fallback site, view-assembly home,
    scaffold gitignore, `install_crypto_provider`). Name `config-adapters/src/store.rs`
    as the fallback's implementation site; name a module for view assembly so
    `render/*` stays pure; add `scripts/accelerator-scaffold.sh` to Phase 1 so
    existing repos gain the temp-name rule; and move the crypto-provider relocation
    into Phase 2 §3 with a criterion.

---

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally ambitious and unusually self-aware — it
correctly identifies the hexagonal seams, justifies the `store` extraction and the
`WriteError`/`StoreError` naming split against real orphan-rule and pup
constraints, and records most quality-attribute tradeoffs explicitly. However, the
composition-root story for `config_command` does not hold together: two mutually
exclusive pup allowlists in adjacent paragraphs, `compose()` routed through a
dispatch module while `main.rs` is claimed to be the only module naming
`config_adapters`, and a `compose()` widening that cannot deliver the two driven
ports it depends on. Phase 0's override rests on a premise verified false, and
`--allow-legacy-layout`'s source-fallback half has no plumbing route.

**Strengths**: `WriteError` naming derived from a real constraint and the
`to_store_error` free function anticipating the orphan rule; splitting filesystem
access into two responsibility-scoped ports rather than one omnibus trait, with
sound reasoning for not adding generic parameters to `ConfigService`; `Rendered`
as a functional-core/imperative-shell improvement; `config` as a built-in with the
permission prefix stopping at `config`; the `stage`/`persist` seam preserved rather
than invented, with a proportionate refusal to build a syscall recorder; and the
refusal ordering correctly handling the first-write case.

**Findings**: 4 critical (pup rule contradiction; composition inverted into
dispatch; `compose()` cannot deliver the ports; source-checkout premise false),
5 major (legacy-fallback plumbing; `--fail-safe` undefined for block subcommands;
`permitted_root` has no corpus supplier; temps in `.accelerator/` with no scaffold
change; domain rules in the inbound module; crate-boundary criterion unimplementable),
3 minor (checker forbids a form narrower than one it accepts; per-subcommand
semantics in the kernel taxonomy; `allowed_only` + `denied` unprecedented and
possibly inert), 1 suggestion (Phase 6's stale exception framing).

### Correctness

**Summary**: The plan's analysis of the trickiest bash behaviours is unusually
accurate — the eject/diff/reset asymmetry, the `get`/`path` split, the double
slash and its non-propagation, `config_find_files`'s fallback, and the
`0006:298` `set -e` trap all verified correct in fine detail. Three defects would
produce wrong behaviour: `template` classified as scalar when it emits a fenced
file body; a seventh migration reading config through a repointed shared helper,
excluded from every criterion and structurally barred from carrying the flag; and
`--fail-safe` mandated on every `!` invocation without defining its interaction
with the three fail-closed paths. Several newly added mechanisms are specified two
incompatible ways.

**Strengths**: The eject/diff/reset asymmetry analysed in more detail than the work
item, with the third fixture a real requirement; the `get`/`path` table verified
including the non-obvious empty-`$2` asymmetry; the `:270` double-slash analysis
exact; the `0006:59-60`→`:298` structural analysis precisely right; correctly
identifying `config_find_files`'s easily-missed second half; the buffered-output
divergence well-founded.

**Findings**: 3 critical (`template` as scalar; seven migrations and 0007's
structural bar; `--fail-safe` vs fail-closed), 4 major (legacy-fallback has no
implementation site; fail-safe criterion contradicts divergence 5 for `summary`;
`--format=hook` specified two ways; the 2-vs-1 return code unobservable at the
named caller), 3 minor (`init`'s catalogue switch; doc-type blank coercion missing;
`context` notice ordering unspecified).

### Test Coverage

**Summary**: Unusually rigorous — the plan identifies the parity gate's own
weakness (258 loose checks against 337 assertions, verified accurate), commits
goldens for the custom-lens state, and attaches a named failing test to most
divergences. But its central coverage claim rests on a gate it deletes: the
repointed `test-config.sh` is the covering gate for most of the removal set, the
ledger replay is satisfied by a gate green *at* the deleting commit, and nothing
requires the 337 assertions to be ported before they vanish. Several new
verification mechanisms are written so the failure mode they exist to catch would
not fail them.

**Strengths**: The parity gate's limits characterised accurately and verified; the
custom-lens golden gap correctly diagnosed and closed; the command-substitution
trailing-newline hazard real, with the pre-reuse `cmp` preventing the likeliest
silent repair; the `get`/`path` asymmetry correctly identified as untestable by the
existing suite; divergences 10-12 naming real tests at the cited lines; the
fail-safe pairing asserted in both directions with the concatenation-vs-blank-line
distinction called out explicitly.

**Findings**: 4 critical (ledger replay satisfied by a deleted gate; parity gate
has no known-positive floor; mandatory `--fail-safe` collides with fail-closed;
consumer-census invariants degrade to vacuous passes), 8 major (divergence tests
unasserted and three at risk of deletion; `check-inventory` floor self-derived for
member 4; two-hop audit misses three-hop suites; per-phase Rust criterion weaker
than CI; golden coverage baseline-only for `agents`/`dump`/`applies_to`;
`--explain` tautology; censuses rewritten then deleted with no successor), 4 minor
(`test-init.sh` wiring needs three edits; `jj show` wrong command and eight-not-nine
call sites; `doc_type_table()` repoint has no resolution mechanism; drift
cross-check dropped without replacement).

### Safety

**Summary**: Unusually safety-literate for its size — it identifies the migration
half-application hazard down to the subshell trap, clamps `config.local.md` to
0600, buffers `paths --doc-types`, and demands known-positive floors of most new
gates. Residual risk is concentrated in three places: the bootstrap as an unguarded
single point of failure with no degradation path outside the launcher;
`atomic_write`'s signature unable to express the per-level mode policy, with the
criterion that would catch a world-readable credential file absent; and several
newly-added protective mechanisms carrying happy-path-only criteria. Recovery for
the two genuinely irreversible outcomes is not provided.

**Strengths**: Phase 0 fixing a real pre-existing race before the call-site count
moves; fail-safe as opt-in with per-consumer reasoning; `paths --doc-types`
buffering; two credential-specific protections called out precisely with
consequences traced; the migration hazard identified in full including the subshell
trap; known-positive floors demanded of most new gates with explicit refusal of
tautological criteria; phases 0-4 genuinely additive with the flip isolated.

**Findings**: 4 critical (bootstrap on the critical path with no degradation;
partial-publish window defeats the standing gate; `atomic_write` cannot express its
mode policy and the 0644 case is untested; per-site migration conversion
unprovable), 6 major (legacy-fallback proof covers only two keys; global
`--allow-legacy-layout` lets `config set` orphan a legacy configuration; ledger
replay has no known-positive; temps in `.accelerator/` with no ignore rule and an
unpinned prefix; containment guard's holes unreachable from the design;
out-of-project template value plus unguarded `rm`), 1 minor (`strip = true` priced
before the launcher was on the critical path).

### Compatibility

**Summary**: The headline measurements all verify exactly, and the move to
basename matching is the right correction. But three consumer-contract defects
survive: Grep A's "exactly 0" is unachievable against a population of retained-file
references; eight failure-suppressing reads outside the migrations get none of the
structural conversion; and the rule shape declared "settled" has zero precedent
among the tree's 84 Bash rules. Separately, `--fail-safe` is asserted on all 247
sites as if it guaranteed splice safety, while the whole bootstrap — eleven exit-1
paths — sits outside that boundary.

**Strengths**: Every headline measurement reproduces exactly; the 42-file
context/skill-context pairing verified, making the collapse safe to call
mechanical; basename matching catching references a path pattern cannot see; the
`get`/`path` table right and genuinely load-bearing at the visualiser's config
writer; the `--allow-legacy-layout` two-halves analysis correct; divergences 10-12
naming real tests; the `config-defaults.sh` retention correct and well-evidenced.

**Findings**: 3 critical (Grep A unachievable; eight unconverted suppression sites;
rule shape without precedent excluded from the probe), 4 major (`cli/` excluded
while the floor requires it; operator-awareness contradicts this repo's research;
`--fail-safe` cannot cover eleven bootstrap paths; Phase 2 changes 0164's shipped
contract), 4 minor (`DisplayHelp` interception renders top-level help; divergence
13's premise never captured from the preprocessor; jq pretty-to-compact unrecorded;
the version floor prose-only and triplicated).

### Security

**Summary**: Genuine security maturity — fail-safe as opt-in, identifier
validation closing a real traversal, a correctly-ordered symlink refusal, explicit
0600 clamping, and a permission rule scoped to `config` rather than the binary.
However, the `ACCELERATOR_LAUNCHER_BIN` gate rests on a factually false premise
(verified directly against the live plugin cache), and the deliberate acceptance of
a broad `config *` grant is justified by a trust-boundary claim that does not hold
for the skills carrying it — while newly exposing `config set`, an unguarded write
into a file whose `token_cmd` values are executed via `bash -c`.

**Strengths**: Fail-safe opt-in with consumers left loud; identifier validation
closing `config-read-skill-context.sh:23`; handler-side parsing correctly
motivated; refusal ordering closing the git-ships-symlinks case; non-obvious and
correct file-mode reasoning; the permission prefix stopping at `config`; the
ancestor-glob check evaluated against the binary path rather than the literal;
`--allow-legacy-layout` preserving 0178's decision; `paths --doc-types` buffering.

**Findings**: 3 critical (source-checkout probe satisfied in every install;
`config set` as a `token_cmd` RCE primitive; `config set` plus out-of-project
template value yielding arbitrary deletion), 5 major (false trust-boundary
rationale; tracked-marker rejection self-defeating; factor 3 under-specified;
`ACCELERATOR_CACHE_DIR` ungated; no guard on credential keys at team level),
3 minor (0600 clamp asserted only where it changes nothing; the override's warning
invisible on both paths that matter; `get` unredacted where `dump` is).

### Performance

**Summary**: Unusually strong performance instincts — correctly identifying that
the cache-hit test performs a full minisign verification on every invocation,
refusing the external-subcommand shape, pre-committing the budget, and moving the
measurement out of the phase that deletes its comparator. However, the plan's own
cost model predicts the 25% budget will fail: the deferred re-verification term
alone is 1.5-3× the entire allowance, yet the measurement is placed in Phase 5 with
no stated ordering against the rewrite. The budget is also self-relative to a
baseline Phase 7 deletes, leaving no standing guard, and Phase 6 moves the
bootstrap onto SessionStart without bounding its cold-path network cost.

**Strengths**: Identifying the per-call verification as the dominant term; `config`
as a built-in; pre-committing the budget; recording quantified levers in advance;
the ~34 redundant parses honestly assessed and correctly subsumed by lever 2;
Phase 0's size-and-cost pairing exactly the right measurement; `ACCELERATOR_BIN`
bound to the compiled binary so the suite does not pay the bootstrap; the
`context` collapse as a genuine call-count reduction.

**Findings**: 1 critical (budget arithmetically unsatisfiable and measured after
the irreversible step), 6 major (measurement unperformable without a release
carrying `config`; self-relative budget leaves no standing guard and the aggregate
framing buys nothing; SessionStart unaccounted with an unbounded cold path;
lock/fetch timeout mismatch untested on cold cache; lever ordering inverted for the
measured skills; working directory unfixed), 2 minor (shim re-copied and Phase 0
makes it worse; `install_crypto_provider` sub-millisecond and size-irrelevant),
1 suggestion (the `config-read-path.sh` VCS claim is wrong).

### Code Quality

**Summary**: Rigorous about behaviour parity, divergence recording and gate
falsifiability, with sound render-seam and store-consolidation instincts. But
several newly-proposed Rust shapes do not survive contact with this workspace's own
enforcement: a `From` impl with no legal home (the symmetric defect the plan
correctly diagnoses one crate over), a pup rule that allowlists and forbids
`config_adapters` two paragraphs apart, and an `atomic_write` signature that cannot
express two of its own requirements. Beyond the compile-level defects, the two
ports are low-cohesion, the largest body of new logic has no named home, and
`Rendered`'s claimed testability benefit is overstated.

**Strengths**: The `WriteError` naming correctly reasoned from the actual pup rule;
`to_store_error` as the correct orphan-rule resolution, stated rather than
discovered; `Rendered` removing `eprintln!` with three real justifying behaviours;
the six render modules proportionate at 80 columns; the `get`/`path` table pinned
with the consumers depending on each direction and a note that the parity gate
cannot catch either; the `stage`/`persist` seam with observable-property framing;
handler-side name parsing, subtle and correctly motivated with a criterion that
would fail if done otherwise.

**Findings**: 2 critical (`From<WriteError> for ConfigError` has no legal home;
the pup rule contradicts itself), 8 major (`atomic_write` signature; lazy-compose
contradiction; omnibus port and scaffold-that-deletes; `ConfigError::Io` says
"config file"; view-assembly homeless; `Default` impl break; clap kind wrong for
bare `config`; `allowed_only`+`denied` unprecedented), 4 minor (sketches would fail
`cli:check`; `Rendered` erases interleaving; fail-safe notices escape the render
modules; `Refusal` doc comment encodes a launcher fact), 3 suggestions (`--format`
two value spaces; crypto-provider relocation unassigned; `store`'s `kernel` dep
likely unused).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-07-20

**Verdict:** REVISE

All eight lenses re-ran fresh against the fully-revised plan (grown to ~3040
lines). The revision resolved the overwhelming majority of review-2 pass-1's
findings — the fail-safe matrix, the seven-migration/`doc-type-table` allowlist,
the atomic_write signature and 0644→0600 criterion, the eight suppression sites,
the Grep A functional/mention split, the deletion-ledger final-state column, the
per-k migration stub, the retained parity divergence tests, and the release
reorder were all independently confirmed sound. But the pattern this plan has hit
on every prior pass repeated: **the revision introduced a new crop of defects,
almost all of them in the text just added, and ~14 of the ~24 findings are
regressions from the pass-1 revision itself** rather than pre-existing gaps. Two
are critical. Their character is milder than pass-1's — mostly mechanical
contradictions between a freshly-added statement and an older one, or a new gate
whose spec is internally inconsistent — but one (the shim security regression) is
a genuine new hole opened by a performance optimisation the revision added.

### Previously Identified Issues (pass 1) — resolution

**Resolved** (independently confirmed by the re-running lenses):

- 🔴 Phase 0 override premise false — **Resolved** (untracked-marker gate; but see
  new marker `.gitignore` gap).
- 🔴 `config set` token_cmd RCE / template-delete — **Resolved as recorded
  declines** with corrected rationale (but Phase 5 §1 still carries the old
  rationale — see new issues).
- 🔴 Bootstrap outside fail-safe — **Resolved** as an accepted-outage decision
  pushed to 0164/0165.
- 🔴 `atomic_write` mode policy inexpressible / 0644 untested — **Resolved**
  (`NewFileMode`, mode-on-temp, explicit 0644→0600 criterion).
- 🔴 Per-site migration conversion unprovable — **Resolved** (per-k stub +
  `--unapply`).
- 🔴 Release partial-publish window — **Resolved in intent** (reorder), but only
  as a criterion, not a Changes-Required task — see new issues.
- 🔴 Seven-migration / 0007 bar — **Resolved** (allowlist `doc-type-table.sh`;
  counts corrected, verified ×5/×5/×1).
- 🔴 Eight non-migration suppression sites — **Resolved** (enumerated, converted).
- 🔴 Grep A unachievable / `cli/` exclusion — **Resolved** (functional/mention
  split, `cli/` back in corpus).
- 🔴 Rule-shape / operator-awareness settled-claims — **Resolved** (moved into the
  probe; metacharacter rejection added).
- 🔴 `template` scalar misclassification — **Resolved** (moved to block + goldens).
- 🔴 pup rule self-contradiction — **Resolved** (config_adapters removed), though
  the allowlist namespace spelling is now wrong — see new issues.
- 🔴 `From<WriteError>` orphan rule — **Resolved** (`to_config_error`).
- 🔴 Deletion-ledger same-commit / parity-gate floor / check-inventory member-4 /
  `--explain` tautology / census member-1 / suite audit — **Resolved** (final-state
  column + known-positive; stub-fails floor; set-equality; provenance content;
  census expanded; transitive-closure — though its floor names non-existent
  suites, see new issues).
- 🔴 `--fail-safe` mandate vs narrow definition / fail-closed collision —
  **Resolved** (matrix with read/IO-vs-validation split), though `work`'s row is
  now wrong — see new issues.
- 🟡 Latency budget unsatisfiable / measurement placement — **Partially**: moved
  to a Phase 5 entry precondition, but the gate now measures the un-batched shape
  the promoted primary lever cannot touch — see new issues.

### New / Still-Open Issues

**Critical**

- 🔴 **Content-addressed shim skip-if-exists defeats signature verification**
  (security). Phase 0 §2's optimisation (skip staging when the content-addressed
  shim path exists) removes the per-call re-copy that currently overwrites any
  planted stub before it verifies the launcher; combined with the still-ungated
  `ACCELERATOR_CACHE_DIR`, an env-var injection plus a writable temp dir yields a
  full verification bypass. **Introduced by the pass-1 revision.** Fix: verify the
  staged shim against the committed `bin/accelerator-verify.vendored.sha256`
  before trust (or keep re-copying), and gate `ACCELERATOR_CACHE_DIR` in this
  story rather than deferring.
- 🔴 **Latency entry-gate measures the shape the primary lever cannot move**
  (performance). The gate runs at end-of-Phase-4 over the raw un-batched
  invocation set; batching (promoted to the primary lever) is a Phase 5 change, so
  only the deferred validity stamp can move the number — and the plan's own
  arithmetic says the budget fails without it, with the remedy left unscoped.
  **Introduced by the pass-1 revision** (measurement placement + lever reorder).
  Fix: commit the validity stamp into Phase 0, or measure the batched shape after
  Phase 5's carve-out and state which shape is authoritative.

**Major** (regressions from the pass-1 revision unless noted)

- 🟡 **FnOnce composer returning `ComposedConfig` forces `launch/mod.rs` to name
  `config_adapters`** (architecture + code-quality, 2 lenses) — contradicts the
  "only main.rs names config_adapters" invariant. Fix: composer returns
  `config`/`config_command`-owned types, not the adapter struct.
- 🟡 **pup allowlist uses `accelerator::` where the convention is literal
  `crate::`** (architecture + code-quality, 2 lenses) — as written the rule denies
  config_command's own intra-crate imports (or enforces nothing). Fix:
  `^crate::config_command(::|$)` in `allowed_only`.
- 🟡 **eject-write assigned to both `ScaffoldProject` and `TemplateOverride`**
  (architecture + code-quality, 2 lenses) — the port split contradicts itself.
  Fix: eject-write belongs to `TemplateOverride` only.
- 🟡 **Removing `append_record`'s pre-lock `create_dir_all` collides with the
  lock's mkdir precondition** (code-quality) — the mkdir-based lock needs the
  parent to exist before it acquires. Fix: scope hole-1's removal to `stage`'s
  `:59`, keep `append_record`'s pre-lock ensure.
- 🟡 **Fail-safe matrix renders a notice for `work`, breaking the integration
  presence-probe** (correctness) — `config work integration` is consumed like a
  scalar (empty = not configured); a `## Work Unavailable` notice reads as
  "configured". Fix: `work` degrades read/IO by suppression, not notice.
- 🟡 **Suite-audit three-hop floor names non-existent suites** (test-coverage) —
  `test-jira-create-flow.sh`/`test-linear-search-flow.sh` do not exist; the real
  suites are `test-jira-create.sh`/`test-linear-search.sh`. Fix: name the real
  three-hop-only suites.
- 🟡 **Catalogue-drift replacement test asserts `min_lenses == default_for`, false
  for work-item** (test-coverage) — work-item mode renders 3, catalogue declares
  4. Fix: assert per-mode expected values, not `default_for`, for `min_lenses`.
- 🟡 **Broad `config *` grant accepted on the 'reviewed commits' rationale Phase 3
  §1 debunks** (security) — Phase 5 §1 still carries the false premise the
  revision corrected elsewhere. Fix: restate Phase 5 §1 with the accurate
  residual-risk framing (or apply the split lever).
- 🟡 **Dev-launcher marker's `.gitignore` rule is prose-only** (security) — not in
  the change list, and the refusal criterion passes whether or not the rule
  exists; under jj a contributor-created marker becomes tracked and shippable. Fix:
  add the `.gitignore` edit to Phase 0 §1's change list + a test asserting the
  rule is present and the marker untracked.
- 🟡 **Release reorder/standing gate are criteria with no Changes-Required task,
  plus a dangling-release re-run wrinkle** (safety) — highest-blast-radius
  mechanism under-specified. Fix: add the `release.py` reorder as a task; specify
  create_release/upload interaction with an un-pushed tag.
- 🟡 **`mise run cli:test` is a non-existent task** (test-coverage + correctness) —
  the real task is `test:unit:cli`. Fix across all per-phase criteria.
- 🟡 **Stale lever cross-references** (performance) — Phase 7 says "lever 1 (the
  validity stamp)" and the tail says re-parse is "subsumed by lever 2", both
  inverted after the reorder (batching is now lever 1). Fix: renumber.
- 🟡 **Fail-loud-on-malformed-config reaches the credential path** (compatibility,
  pre-existing/refinement) — divergence 10's blast radius names work-item/doc-type/
  ADR/visualiser but not jira-auth/linear-auth. Fix: name the credential path and
  decide the auth-outage-on-typo tradeoff explicitly.
- 🟡 **Batching may be inapplicable — `config paths` can't reproduce the skills'
  custom labels** (performance, pre-existing/sharpened) — the path-heavy skills
  inject `**Plans directory**: <v>`, not `- key: value`. Fix: specify the batching
  mechanism or accept the block-form label change.

**Minor / suggestion** (selected): `context --skill` needs a fifth named notice
header (`## Skill-Specific Context Unavailable`); Phase 2 §1's read-count omits
`config-read-all-paths.sh ×1` (0004:459); `paths --doc-types [root]` positional
is "settled in Phase 2" but not designed/tested there; the `resolve_corpus_path`
reshape snippet uses `continue` where `walk_corpus` needs `return 0` and drops the
diagnostic; a few file:line citations off by 1-2 (`:129`, `:107`, `:17`); member-4
"named by a suite" is weaker than "behaviourally covered"; parity.rs retention
must strip the differential tails of `:278`/`:296`; team `config.md` reader still
follows a symlink; metacharacter reject-list omits `>`/`>>`/`<`/`&`/newline;
exit-code convergence across sub-binaries is a note not a gate; hooks.json
argument-splitting probe should default to the thin-shim fallback; the
corpus-construction ripple count is 8+1 not 9+1.

### Assessment

The plan is much stronger than review-2 pass 1 — the large revision closed real
holes across every lens. But it is not ready, and the reason is now structural
rather than about any single finding: this is a ~3000-line document whose
mechanisms are each specified in several places, and every editing pass (the
author's and this reviewer's alike) introduces a fresh set of
one-statement-contradicts-another defects. Two of the new findings are genuine
(the shim verification regression and the latency gate/lever inconsistency); the
rest are mechanical corrections to freshly-added text. A third editing pass that
fixes these will, on this plan's demonstrated history, introduce a fourth crop.
The recommendation is to (a) fix the two criticals and the clear mechanical
regressions, then (b) stop iterating prose and take the remaining
specification-level consistency to implementation, where a compiler and the test
gates catch contradictions that prose review keeps re-seeding — which is the
disposition the author already reached on review-1.

## Approval (Pass 3) — 2026-07-20

**Verdict:** APPROVE

The pass-2 recommendation was followed. Both criticals were fixed in the plan:
the content-addressed shim now hash-verifies against
`bin/accelerator-verify.vendored.sha256` before trust and `ACCELERATOR_CACHE_DIR`
is gated in this story (closing the verification bypass); and the latency
inconsistency was resolved by dropping the metadata validity stamp (which would
have traded warm-path tamper detection) and measuring the shipped *batched*
invocation shape as a pre-release gate, with batching for the two worst-case
skills promoted to committed Phase 5 work and its label question recorded as a
Phase 5 blocker. The clear mechanical regressions were also fixed: the composer
returns `config`/`config_command` trait objects (so `launch/mod.rs` does not name
`config_adapters`); the pup allowlist uses the literal `^crate::config_command`
form with a second violating fixture for the `config_adapters` seam; the
eject-write is de-duplicated onto `TemplateOverride`; the `create_dir_all` removal
is scoped to `stage` (keeping `append_record`'s pre-lock mkdir); `work` moved to
the fail-safe suppression row and the fifth notice header is named; the
suite-audit floor names real suites; the catalogue-drift test is scoped per-mode;
the Phase 5 §1 rationale is corrected; `cli:test` → `test:unit:cli`; and the
read-count, `resolve_corpus_path` snippet, parity-test retention, citations,
corpus construction count, and `paths [root]` positional are all corrected.

The verdict is moved to **APPROVE** by reviewer decision. The residual items —
the malformed-config credential blast-radius decision, team-config symlink
handling, the metacharacter reject-list additions, cross-sub-binary exit-code
convergence as an enforced gate, the hooks.json argument-splitting default, and
the member-4 "named by a suite" → "behaviourally covered" strengthening — are
**accepted for resolution during implementation** rather than in further prose
review, on the reasoning recorded in the pass-2 assessment: this plan's
demonstrated pattern is that each editing pass reseeds contradictions, and the
compiler and the test gates the plan installs will surface the remaining
specification-level consistency more reliably than a fourth review round. The
plan is marked `ready`.
