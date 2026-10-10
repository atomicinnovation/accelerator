---
type: "work-item"
id: "0299"
title: "Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069"
date: "2026-10-05T10:02:05+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "task"
priority: "medium"
relates_to: ["work-item:0226", "work-item:0219", "adr:ADR-0069", "adr:ADR-0054"]
external_id: "PP-881"
tags: ["cli", "architecture", "dependencies", "refactor"]
last_updated: "2026-10-10T17:32:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0299: Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Make every crate dependency in the `cli/` workspace satisfy ADR-0069's
dependency rules, and add a lint over normal and build dependencies that
keeps them satisfied. The work merges `consent-adapters` into
`config-adapters` and moves the launcher's tracking
check in-process, removing the `vcs tracking` subcommand. It makes `config`'s
catalogue describe the `<tracker>.pull` and `<tracker>.push` blocks so the
launcher can drop `tracker-support`. `migrate` parses frontmatter through a
new `corpus` port and gains a port of its own for canonical rendering. Each
adapter → adapter dependency across contexts that rule 4 forbids is replaced
by a port injected at a composition root, by moving context-owned logic or
types into the upstream domain crate, or by moving a context-free primitive
into a technical library. The remaining domain → domain
dependencies are each justified or removed, and every binary is re-measured
for the `gix`, `jj-lib` and `uluru` closure.

## Context

ADR-0069 gives every workspace crate a role (kernel, domain, adapter,
technical library, composition root, test support, bootstrap verifier) and
every context a kind (platform, shared, product). It sets eight rules for
which dependencies are allowed. Today's workspace breaks several of them, and
nothing at the crate-graph level detects it: `cargo-pup` checks `use` paths
and `cargo-deny` checks third-party crates.

Plan 0226 created `consent-adapters` to keep `gix` and `jj-lib` out of
`config-adapters`' dependents. It also made the launcher answer its SessionStart
tracking question by running `vcs tracking --path` as a captured child process.
That needs a process-capture subsystem, and it adds a `TrackingCheck::Unchecked`
state for a cold first session in which the `vcs` binary cannot be fetched in
time. ADR-0069 rule 7 rules out a crate boundary kept for packaging. Rule 6
allows the launcher to use platform contexts such as `vcs`.

The violations, by rule:

| Dependency | Rule | Used for |
|---|---|---|
| `corpus-adapters` → `vcs-adapters` | 4 | `facts` for metadata derivation |
| `work-adapters` → `vcs-adapters` | 4 | `InProcessProbe`, `facts` for working-copy status and author |
| `research-adapters` → `corpus-adapters` | 4 | frontmatter validation, `parse`, `FileCorpusStore` |
| `jira-client` → `corpus-adapters` | 4 | `acquire`, `LockOptions` file lock |
| `linear-client` → `corpus-adapters` | 4 | `acquire`, `LockOptions` file lock |
| `migrate-adapters` → `config-adapters` | 4 | `FileConfigStore`, `Composed`, `LegacyPolicy` |
| `migrate-adapters` → `vcs-adapters` | 4 | VCS facts |
| `migrate-adapters` → `corpus-adapters` | 4 | corpus store, walking, frontmatter validation |
| `migrate-adapters` → `work-adapters` | 2, 4 | sync baseline paths, store and digests |
| `consent-adapters` (whole crate) | 4, 7 | packaging boundary; `vcs-adapters` for tracking and repository roots |
| `migrate` → `document` | 1 | frontmatter parse and canonical render in `m0001`, `m0008` |
| `work` → `config` | 3 | nothing: the edge is dead |
| `launcher` → `tracker-support` | 6 | `<tracker>.pull`/`.push` validation in the `config` dump |

## Requirements

1. Each workspace crate declares its ADR-0069 role in its manifest's
   `[package.metadata.accelerator]` table (format in Technical Notes). The
   launcher declares a `launcher` role of its own, because rule 6 limits it more
   than other composition roots. Rule 5 still applies to it both ways: the
   launcher depends on no composition root, and no composition root depends on
   the launcher. Domain and adapter crates also declare their context there; no
   other role declares one. Each context's kind is declared on its domain crate,
   or, for a context with no domain crate (`jira`, `linear`), on each of its
   adapter crates. Each shared context's domain crate also declares its closed
   set of downstream contexts.
2. A lint over `cargo metadata`, part of `mise run check`, rejects every
   normal or build dependency that ADR-0069 rules 1, 2, 4, 5 and 6 forbid,
   and ignores dev-dependencies. Rule 3 is enforced by review, not the lint.
   The lint also enforces three constraints ADR-0069 leaves open: `kernel`
   depends on no workspace crate; a technical library depends only on
   `kernel` and other technical libraries; and only the launcher may depend
   on the bootstrap verifier. It also runs two checks no ADR rule names: the
   declaration check (every crate's declarations are present, recognised and
   consistent) and the test-support check (no other crate has a normal or
   build dependency on a test-support crate). Each finding names the crate,
   the dependency where there is one, and the rule number or check name. The
   lint exempts test support and the bootstrap verifier as sources of
   dependencies. Both registration checklists in `tasks/README.md` gain the
   declaration step.
3. The shipped workspace passes the lint with no exception list.
4. The `vcs` domain publishes the tracking questions (file tracking, and the
   roots of the enclosing repositories) as a port, and `vcs-adapters` implements
   it. The contents of the `consent-adapters` crate move into `config-adapters`,
   which depends on `vcs` and not on `vcs-adapters`. Each composition root that
   needs tracking (7 roots with 8 call sites, listed in Technical Notes) injects
   the implementation, and the `consent-adapters` crate is removed.
5. The launcher injects the in-process tracking implementation. The captured
   `vcs tracking --path` child, the `vcs tracking` subcommand,
   `kernel::tracking`, `TrackingCheck` with its `Unchecked` variant,
   `ConfigFileTracking::check`, and
   `AuditFinding::PersonalFileUnchecked` with its "was not checked" note are
   removed.
6. The launcher no longer depends on `tracker-support`: the `config` crate's
   catalogue describes the `<tracker>.pull` and `<tracker>.push` blocks, and
   `tracker-support` derives its parsers from that description. The launcher
   no longer depends on `store`, because `TEMP_PREFIX` moves to `kernel`.
7. Each adapter → adapter dependency across contexts that rule 4 forbids is
   replaced in one of three ways: by a port of the upstream context injected
   at a composition root; by moving context-owned logic or types into the
   upstream domain crate, as the Landing table does for frontmatter
   validation, the `document` types and `canonicalise_id`; or by moving a
   context-free primitive into a technical library.
8. `migrate` parses frontmatter through the `corpus` frontmatter-parsing port,
   and reaches canonical rendering through a port on `MigrationContext`,
   implemented in `migrate-adapters` over `document`. This settles the existing
   `migrate` → `corpus` dependency under rule 3: migrations rewrite corpus
   documents, so `migrate`'s model is expressed in `corpus`'s terms. The dead
   `work` → `config` dependency and its `pup.ron` permission are removed.
9. The remaining domain → domain dependencies (`work` → `corpus`, `work` →
   `tracker`, `collaboration` → `vcs`) are each either justified under
   ADR-0069 rule 3 or removed.
10. The launcher's binary size, warm-dispatch latency and SessionStart config
    summary latency (cold and warm, in a git and a jj repository) are measured
    before any launcher change and again after, by the method in Technical
    Notes. An after/before ratio of 10 or more in binary size or in any
    summary latency median, or of 1.4 or more in the warm-dispatch median,
    stops implementation for a decision by the work item's author. Every
    sub-binary, the launcher and the visualiser are checked by symbol count for
    whether they link `gix`, `jj-lib` or `uluru` (which carries an MPL-2.0
    notice obligation), and the `deny.toml` symbol-count table is rewritten to
    match. The visualiser linking any of the three stops implementation for a
    decision by the work item's author.
11. The licence record matches the re-measured binaries: `deny.toml`'s
    account of where the MPL-2.0 notice obligation is live names exactly the
    binaries that link `uluru`, including the launcher if it does, and the
    third-party notices artefact still discharges it.

## Acceptance Criteria

- [ ] Given a fixture workspace with one forbidden dependency per check, when
  the lint runs, then it exits non-zero and names the crate, the dependency
  and the rule or check for each. The fixtures are:
  - a domain crate depending on a technical library (rule 1)
  - a domain crate depending on an adapter (rule 1)
  - a domain crate, and an adapter, of one context depending on a crate of
    a different, product context (rule 2)
  - two platform contexts whose crates depend on each other in a cycle
    (rule 2)
  - an adapter depending on another context's adapter (rule 4), declared
    once as a normal and once as a build dependency
  - a composition root depending on another composition root, a
    composition root depending on the launcher, and the launcher depending
    on a composition root (rule 5)
  - an adapter depending on a composition root (rule 4)
  - the launcher depending on a shared context's domain crate, on its
    adapter, and on a product context (rule 6)
  - a domain crate depending on a composition root and on the launcher
    (rule 1)
  - `kernel` depending on a domain crate, and a technical library depending
    on a domain crate and on an adapter (the `kernel` and technical-library
    constraints)
  - a non-launcher crate depending on the bootstrap verifier (the verifier
    constraint)
  - a non-test-support crate's normal, and build, dependency on a
    test-support crate (the test-support check)
- [ ] Given a fixture adapter depending on an adapter of its own context, a
  fixture test-support crate and a fixture bootstrap verifier each with an
  otherwise forbidden dependency, a forbidden edge declared only as a
  dev-dependency, the launcher depending on the bootstrap verifier, and a
  technical library depending on `kernel` and on another technical library,
  the launcher depending on a platform context's domain crate and adapter
  and on a technical library, an adapter depending on an upstream platform
  context's domain crate, and a non-launcher composition root depending on
  adapters of several contexts, when the lint runs, then none is reported.
- [ ] Given a fixture shared context with one declared and one undeclared
  downstream, when the lint runs, then the declared downstream's domain crate
  and adapter may depend on the shared context's domain crate and adapter
  respectively, and the undeclared downstream's may not.
- [ ] Given a crate with no role, a crate with an unrecognised role, a
  domain or adapter crate with no context, a context with no declared kind,
  an unrecognised kind, a context whose crates declare conflicting kinds,
  a context with no domain crate where only some adapters declare its kind,
  a kind declared on an adapter of a context that has a domain crate, a
  shared context's domain crate with no declared downstreams, downstreams
  declared on a crate that is not a shared context's domain crate, a
  downstream naming an unknown context, or a crate whose role carries no
  context but which declares one, when the lint runs, then the declaration
  check exits non-zero and names the crate.
- [ ] Given the shipped workspace, when `mise run check` runs, then the lint
  passes and no exception list exists. Given a forbidden normal dependency
  added to a workspace crate, when `mise run check` runs, then it exits
  non-zero with the lint's finding.
- [ ] Given `cargo metadata` for the workspace, then `consent-adapters` is not
  a member. The only adapter → adapter edges across contexts are declared
  downstreams' adapters depending on `tracker-support`. `work` does not
  depend on `config`, and `pup.ron` grants `work` no permission on `config`.
  The launcher's normal dependencies are limited to `kernel`, the crates of
  the `config`, `vcs` and `corpus` contexts, technical libraries other than
  `store`, and the bootstrap verifier.
- [ ] Given `cargo metadata` for the shipped workspace, then every member's
  declared role, context, kind and downstreams match the Expected
  declarations table in Technical Notes exactly, and every member appears
  in it.
- [ ] Given both registration checklists in `tasks/README.md`, then each
  includes the step declaring the crate's role, and its context and kind
  where the role carries one.
- [ ] Given a `config.local.md` that is tracked, untracked, or outside any
  repository, in a git and in a jj repository, with and without the `vcs`
  sub-binary in the launcher cache, when the SessionStart config summary
  renders, then a tracked file is warned about in every case, an untracked
  file or one outside any repository produces no tracking warning, and no
  "was not checked" note is emitted in any of them.
- [ ] Given the behaviours the refactor reroutes, then each is recorded as a
  committed characterisation test before any refactor begins, and those
  tests pass unchanged on the finished change. The tests run the built
  binaries against fixture repositories and config, in a git and in a jj
  repository, and compare stdout, stderr and exit status, so they call no
  API the refactor reshapes. They cover:
  - consent-key reads (the six user-only keys 0226 guards, such as
    `jira.token_cmd`) of each non-launcher root that injects
    tracking, and the launcher's repository-root resolution with a nested
    repository
  - the `accelerator config` dump of tracker blocks and `jira-cli`/
    `linear-cli` block parsing
  - metadata derivation (`accelerator corpus metadata derive`), including
    author and working-copy status
  - frontmatter validation findings over a fixture corpus
  - the `work-cli` commands that read the author
    (`work-adapters/src/author.rs`) and working-copy status
    (`work-adapters/src/sync/working_copy_status.rs`)
  - for each rerouted symbol in the violations table, at least one
    `research-cli`, `work-cli`, `jira-cli` or `linear-cli` command that
    exercises it, including corpus-store reads and writes and the file lock
  - every migration that reaches a refactored port, including sync-baseline
    refresh
- [ ] Given a tracked and an untracked `config.local.md` in a git and in a jj
  repository, when each non-launcher composition root that injects tracking
  (`collaboration-cli`, `jira-cli`, `linear-cli`, `research-cli`, `work-cli`
  and both `design-cli` sites) reads consent keys, then its
  behaviour matches the pre-change behaviour.
- [ ] Given `accelerator vcs tracking --path x`, when it runs, then it fails
  as an unknown subcommand. `TrackingCheck`, `ConfigFileTracking::check`,
  `AuditFinding::PersonalFileUnchecked` and `kernel::tracking` no longer
  exist. The files listed under Removals in Technical Notes do not exist,
  `pup.ron` has no `consent_adapters_is_zero_spawn` rule,
  `tasks/lint/skill_cli_refs.py` has no `"tracking"` entry, and
  `tasks/README.md` has no `consent-adapters` placement rule or
  registration.
- [ ] Given valid, structurally invalid and unset `<tracker>.pull` and
  `<tracker>.push` blocks, when `accelerator config` dumps them, then the
  output and exit status match the pre-change behaviour.
- [ ] Given the same valid, structurally invalid and unset blocks, when
  `jira-cli` and `linear-cli` parse them through `tracker-support`'s derived
  parsers, then the parse results and error text match the pre-change
  behaviour.
- [ ] Given a field added in a test only to the catalogue's `<tracker>.pull`
  description, and separately to its `<tracker>.push` description, and set
  to a valid value, then the `accelerator config` dump
  prints it and `tracker-support`'s parser returns it. Set to an invalid
  value, both report it as they report other structurally invalid fields.
  No tracker `pull` or `push` field name appears as a string literal in
  `dump.rs`.
- [ ] Given a fixture corpus holding documents with no frontmatter, empty
  frontmatter, malformed frontmatter, and frontmatter that `m0001` and
  `m0008` rewrite, including a value whose canonical rendering differs from
  its source, when the two migrations run before and after the change, then
  their output is byte-identical.
- [ ] Given the launcher's binary size, warm-dispatch latency, and
  SessionStart config summary latency (cold and warm, in a git and a jj
  repository) measured before any launcher change and after, by the method
  in Technical Notes, with the after warm-dispatch figure taken from a
  signed prerelease of the finished change before merge, then the raw
  figures and their ratios are recorded in 0299's implementation plan.
  An after/before ratio of 10 or more in binary size or in any summary
  latency median, or of 1.4 or more in the warm-dispatch median, stops
  implementation for a decision by the work item's author, recorded in that
  plan.
- [ ] Given every sub-binary, the launcher and the visualiser re-measured by
  `deny.toml`'s documented method, then the symbol-count table is rewritten
  to match. If the visualiser links any of `gix`, `jj-lib` or `uluru`,
  implementation stops for a decision by the work item's author, recorded
  in 0299's implementation plan.
- [ ] Given every binary re-measured, then the binaries `deny.toml` names as
  carrying the MPL-2.0 notice obligation are exactly those the symbol-count
  table shows linking `uluru`, and `mise run notices:check` passes.
- [ ] Given the three remaining domain → domain dependencies, then each is
  either justified under rule 3 or removed, with the outcome recorded in
  0299's implementation plan. Each justification names the upstream types
  the downstream domain's model is expressed in.
- [ ] Given the finished change, when `mise run` runs, then it exits 0.

## Open Questions

- Whether planning splits this item with `/refine-work-item`.
- Rule 3: whether each of `work` → `corpus`, `work` → `tracker` and
  `collaboration` → `vcs` is justified or removed. A removal is refactoring
  this item does not yet describe.

## Dependencies

- Blocked by: none (ADR-0069 is accepted)
- Blocks: none
- Requires before merge: a signed prerelease of the finished change, because
  `mise run measure:warm-dispatch` needs a published signed release for the
  tree's own version to take the after warm-dispatch figure. CI publishes
  prereleases only on push to `main`, so this one comes from the local
  `mise run prerelease` flow, run by a holder of
  `ACCELERATOR_RELEASE_SECRET_KEY`, which publishes it to the release host the
  launcher fetches from. The harness and the launcher check its minisign
  signature only, not SLSA build provenance (`tasks/measure.py` needs only a
  signed release published for the tree's version), so a locally signed
  prerelease suffices.
- May block on: a decision by the work item's author if a launcher
  after/before ratio reaches 10, or the visualiser links `gix`, `jj-lib` or
  `uluru`.
- Coordinates with: any in-flight work that adds a `cli/` crate or a
  cross-crate edge, or edits a crate this item restructures
  (`consent-adapters`, `config-adapters`, `corpus-adapters`, `kernel`,
  `research-adapters`, `jira-client`, `linear-client`, `migrate`,
  `migrate-adapters`, `work-adapters`, `tracker-support`, `config`, the
  launcher and the 7 tracking roots). Once the lint is in `mise run check`,
  new crates must declare roles and follow the updated registration
  checklists.
- Coordinates with: 0219, which changes the `measure:warm-dispatch` harness
  (two-version interleaved sessions, an entry-free warm-up path, runner
  identity). This item does not wait for it, but takes its before and after
  figures on the same harness revision; if 0219's interleaved mode has landed,
  one interleaved session pairing the current release with this item's signed
  prerelease replaces the separate before and after runs. The locally cut
  prerelease enters the version sequence 0219's lane pairs against.

## Assumptions

- Observable behaviour is preserved, except for removing the `vcs tracking`
  subcommand and the `TrackingCheck::Unchecked` state.
- The launcher is the only caller of `vcs tracking`; no skill, hook, agent or
  `bin/` script invokes it.
- `jira-client` and `linear-client` are adapters of the `jira` and `linear`
  product contexts, which have no domain crate. `tracker-support` is an
  adapter of the shared `tracker` context, which declares `work`, `jira` and
  `linear` as its downstreams, so the clients' dependencies on it are
  allowed under rule 4's shared-context exception.
- `remote-projection` and `process-probe` are technical libraries. `github`
  is an adapter of `collaboration`, and `design-adapters` of `design`.
- `verify` is the bootstrap verifier. It and the `*-test-support` crates are
  exempt from the lint as sources of dependencies.
- Rule 3 is a judgement the lint cannot make. It is enforced by review and
  recorded in 0299's implementation plan; the lint allows any domain →
  domain edge rule 2 allows.
- Rule 1's "one of its own ports" includes a port of an upstream domain the
  crate may depend on, so `migrate` parsing through `corpus`'s port
  satisfies it: `migrate` never reaches `document` directly.
- The launcher queries tracking through the same in-process implementation
  as every other composition root, with no launcher-specific deadline. A
  slow repository can delay SessionStart; the latency measurement surfaces
  this rather than a guard.
- The launcher and its sub-binaries move together: sub-binaries re-fetch on
  a manifest-hash change and a new plugin version drives a new launcher
  (ADR-0054), so no launcher calls `vcs tracking` against a `vcs` binary that
  has dropped it.

## Technical Notes

- **Sequencing:** take the before measurements and commit the characterisation
  tests first. Then declare roles, then build the lint against its fixtures
  without wiring it into `mise run check`, then the refactors. Inside the
  refactors, each port or primitive lands before its consumers switch to it: the
  `vcs` tracking port before the `consent-adapters` merge and the launcher's
  in-process tracking; the `vcs` repository-facts port before the
  `corpus-adapters`, `work-adapters` and `migrate-adapters` rewires; the
  `corpus` parsing port and the moves in the "Landing per symbol" table before
  `migrate` and `research-adapters` switch; the lock move to `store` before the
  `jira-client` and `linear-client` edges go; `TEMP_PREFIX` in `kernel` before
  the launcher drops `store`; and the catalogue block description before
  `tracker-support` derives its parsers from it. Wire the lint into `mise run
  check` last, once the workspace passes it. Then re-measure every binary,
  rewrite `deny.toml`'s symbol-count table and the notices, cut the signed
  prerelease, and take the after figures.
- **Measurement:** binary size is the stripped `--release` launcher on
  aarch64-apple-darwin. Warm-dispatch latency is `mise run
  measure:warm-dispatch`, against the current release before and against a
  signed prerelease of the finished change after. SessionStart config summary
  latency is the median of at least 20 runs after 3 discarded warm-ups, in a git
  and in a jj fixture repository. A cold run starts from an empty launcher cache
  and a repository not yet opened in the run; a warm run has a populated cache
  and repeats an invocation. Before and after figures are taken on the same
  host, whose model, OS version and load are recorded with them. The 10× gate is
  an after/before ratio of 10 or more, of binary size and of each latency
  median.
- **Tracking port:** something like `vcs::tracking::RepositoryTracking`, with
  `file_tracking` and `repository_roots`, implemented by an `InProcessTracking`
  in `vcs-adapters` over the existing `file_tracking` and `repository_roots`
  functions. `credential_ports` and `repository_roots`, moving from
  `consent-adapters` to `config-adapters`, take the port as a parameter. Two
  pieces of `config` vocabulary stay on the `config-adapters` side of the port:
  the translation from `FileTracking` to `Tracking`, and `beside_config_root`,
  which adds the config root to the repository roots the port reports. The 8
  call sites are `collaboration-cli`, `jira-cli`, `linear-cli`, `research-cli`,
  `work-cli`, `design-cli` (twice) and the launcher.
- **Repository facts:** `corpus-adapters`, `work-adapters` and
  `migrate-adapters` need `vcs_adapters::facts` and
  `vcs_adapters::library::InProcessProbe`, the in-process reader of a
  repository's root, idiom, revision, user name and working-copy status. A
  repository-facts port in `vcs`, separate from the tracking port and injected
  at each composition root, covers both for all three adapters.
  `corpus::metadata::RepoFactsProbe` already exists; its one implementation,
  `VcsBackedRepoFactsProbe` (`corpus-adapters/src/metadata.rs:214`), stays and
  calls the repository-facts port instead of `vcs_adapters::facts`.
- **Declaration schema:** roles are kernel, domain, adapter, technical
  library, composition root, launcher, test support and bootstrap verifier.
  Domain and adapter crates declare a context; no other role does. A
  context's kind sits on its domain crate, or on each adapter crate of a
  context with no domain crate. A shared context's domain crate also
  declares its downstreams. Values are kebab-case:

  ```toml
  [package.metadata.accelerator]
  role = "domain"
  context = "tracker"
  kind = "shared"
  downstreams = ["work", "jira", "linear"]
  ```
- **Expected declarations:** every workspace member after the change.

  | Context | Kind | Domain crate | Adapter crates |
  |---|---|---|---|
  | `config` | platform | `config` | `config-adapters` |
  | `vcs` | platform | `vcs` | `vcs-adapters` |
  | `corpus` | platform | `corpus` | `corpus-adapters` |
  | `tracker` | shared (`work`, `jira`, `linear`) | `tracker` | `tracker-support` |
  | `work` | product | `work` | `work-adapters` |
  | `collaboration` | product | `collaboration` | `github` |
  | `migrate` | product | `migrate` | `migrate-adapters` |
  | `design` | product | `design` | `design-adapters` |
  | `research` | product | `research` | `research-adapters` |
  | `jira` | product | none | `jira-client` |
  | `linear` | product | none | `linear-client` |

  | Role | Crates |
  |---|---|
  | kernel | `kernel` |
  | technical library | `document`, `store`, `remote-projection`, `process-probe` |
  | composition root | `corpus-cli`, `vcs-cli`, `work-cli`, `collaboration-cli`, `migrate-cli`, `jira-cli`, `linear-cli`, `design-cli`, `research-cli`, `visualiser/server` |
  | launcher | `launcher` |
  | test support | `cli-test-support`, `vcs-test-support`, `tracker-test-support`, `http-test-support`, `graphql-test-support` |
  | bootstrap verifier | `verify` |
- **Landing per `corpus-adapters` and `config-adapters` symbol:**

  | Symbol | Landing |
  |---|---|
  | `frontmatter_validation::{corpus_files, build_index, validate_path, validate_text, validate_targets, validate_templates, Checks, TargetOutcome}` | `corpus` domain, generic over `CorpusWalker`, `FileReader` and the parsing port |
  | `document::{parse, FrontmatterState, ParsedDocument}` | types to `corpus`; a new `corpus` frontmatter-parsing port, implemented in `corpus-adapters` over `document` |
  | `jsonl::{parse_record, compose_record}` | stay in `corpus-adapters` behind `corpus::RecordStore`, extended if it lacks a read |
  | `FileCorpusStore`, `fs::RealFs` | `research-adapters` and `migrate-adapters` take `RecordStore`, `AtomicWrite`, `FileReader`, `CorpusWalker`, injected |
  | `metadata::SystemClock` | `research-adapters` and `migrate-adapters` take `corpus::Clock`, injected |
  | `work_item_pattern::canonicalise_id` | `corpus` domain |
  | `lock::{acquire, LockOptions}` | `store` |
  | `config_adapters::{FileConfigStore, Composed, LegacyPolicy}` | `migrate-cli` builds `Composed` over `FileConfigStore` with `LegacyPolicy::Allow` (today in `migrate-adapters/src/context.rs:44`) and injects it through `config`'s port |

- **Migration frontmatter:** `migrate` parses through the `corpus`
  frontmatter-parsing port, so requirement 8's `MigrationContext` port covers
  canonical rendering only. Whether it is a method on `MigrationContext` or a
  trait it hands to each migration is a planning decision.
- **Catalogue blocks:** `config/src/catalogue.rs` cannot describe a
  structured block today; `ExtraKey` and `Default` are scalar or sequence
  only. Building that description, and deriving `tracker-support`'s
  `pull.rs` and `push.rs` parsers from it, is in scope and is the largest
  piece of this work.
- **File lock:** `acquire` and `LockOptions` contain no corpus knowledge.
  Moving them into `store` removes the `jira-client` and `linear-client`
  dependencies without adding a port.
- **Sync baselines in migrations:** `migrate-adapters`' use of
  `work_adapters::sync::{baseline, baseline_store, digest}` becomes a port in
  `migrate` for refreshing sync baselines, which `migrate-cli` wires to
  `work-adapters`. Only the composition root names `work`.
- **Removals:** `cli/launcher/src/launch/outbound/capture.rs`,
  `cli/launcher/src/launch/outbound/tracking.rs`,
  `cli/vcs-cli/src/tracking.rs`, `cli/vcs-cli/tests/tracking.rs`,
  `cli/kernel/src/tracking.rs`, the `cli/consent-adapters/` crate, and
  the `consent_adapters_is_zero_spawn` rule in `pup.ron`. `TrackingCheck`,
  `ConfigFileTracking::check`, `AuditFinding::PersonalFileUnchecked` and its
  context note in `cli/launcher/src/config_command/core/summary.rs` go, as do
  the `"tracking"` entry in `tasks/lint/skill_cli_refs.py` and the `tracking`
  branch of `cli/launcher/tests/fixtures/accelerator_fixture.rs`. The placement
  rule for `consent-adapters` in `tasks/README.md:761-764` goes too, along with
  that crate's licence-audit, `cargo-deny` and public-API registrations.
- **Licence notices:** `deny.toml`'s `uluru` exception records where the
  MPL-2.0 obligation is live, and the third-party notices artefact
  (`mise run notices:update`, checked by `notices:check`) discharges it.
- **Lint precedent:** `tasks/lint/config_test_support.py` already reads
  `cargo metadata` to limit which crates enable a feature.
- **Duplicated field lists:** `dump.rs:85-180` repeats the Jira and Linear
  `pull` field lists that `tracker-support` holds. The catalogue description
  removes the duplicate.

## Drafting Notes

- `launcher` → `store` already satisfies rule 6, because `store` is a
  technical library. Moving `TEMP_PREFIX` to `kernel` is an agreed
  simplification, not a violation fix.
- One work item covers every violation, at the author's request, and stays a
  task after review. The no-exception-list criterion means the lint lands
  with the last remediation, and the launcher's behaviour change is not
  separable from the refactors; the author accepts both. Planning may still
  split it with `/refine-work-item`.
- The 10× threshold reflects the author's tolerance: launcher latency is
  measured, and avoided only if it grows by an order of magnitude.

## References

- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
- `meta/plans/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys.md`
- Related: 0226
