---
type: "work-item"
id: "0136"
title: "Migrate Shell Scripts into a Rust CLI"
date: "2026-06-22T23:41:03+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "in-progress"
kind: "epic"
priority: "medium"
source: "note:2026-06-22-ideas-backlog"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture", "codebase-research:2026-06-23-0136-shell-scripts-rust-cli-migration-surface"]
tags: ["rust", "cli", "migration", "epic"]
last_updated: "2026-08-31T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-157"
---

# 0136: Migrate Shell Scripts into a Rust CLI

**Kind**: Epic
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate the shell-script library that backs the skills into a Rust CLI,
consolidating logic into a typed, testable, cross-platform binary.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). A large bash library (config
reading, VCS detection, frontmatter parsing, migrations) currently backs the
skills and is constrained by a bash 3.2 floor.

## Requirements

- Identify the shell scripts and shared library functions in scope for
  migration. *(Done — see the surface research and the scope/architecture
  research below.)*
- Define the Rust CLI surface that replaces them, preserving the
  `${CLAUDE_PLUGIN_ROOT}` invocation contract used by skills. *(Done — the
  `accelerator` launcher + `accelerator-<sub>` binaries, crate split, and
  subcommand surface are defined in the architecture research.)*
- Plan an incremental migration that keeps skills working throughout. *(Done —
  the 12-phase decomposition below, sequenced along the dependency spine.)*

## Decomposition

This epic is decomposed into the following child work items, ordered along the
dependency spine; each keeps the plugin functional at its step. Statuses vary
and are tracked on the items themselves. Grandchildren are listed indented
beneath the parent that owns them and are covered transitively by that parent's
completion. This list was reconciled against the real parent-linked child set on
2026-08-31 (see Drafting Notes), and extended on 2026-09-05 with the
post-migration remnants recovered from an audit of work items numbered above
0136 (see the closing group).

**Foundations (Phases 0–2):**
- 0162 — Rust Toolchain Guard Rails in mise + CI
- 0163 — Scaffold the cli/ Hexagonal Workspace with a version Subcommand
- 0164 — Launcher and Git-Style Dispatch
- 0165 — Multi-Binary Static Distribution and Release Pipeline with minisign
- 0186 — Remove the Exec Probe from the Bootstrap Warm Path *(unblocked 0169)*
- 0187 — Generalise the Sub-Binary Registration Surface *(unblocked
  0169/0170/0171/0172/0173)*

**Shared core + the contract cutover (Phases 3–4):**
- 0166 — Shared config, corpus, and store Crates
  - 0178 — config and config-adapters Crates with Native YAML Reader
  - 0179 — corpus and corpus-adapters Crates for Parsing and Conventions
  - 0180 — Atomic-Store Primitives in corpus-adapters
- 0167 — Built-in config Command and Invocation-Contract Migration

**Subdomain migrations (Phases 5–10):**
- 0168 — Fold the Visualiser into the cli/ Workspace
- 0188 — Library-Backed VCS Adapter over gix and jj-lib *(precedes 0169)*
- 0169 — VCS Subdomain and Hooks Migration
- 0198 — Migrate vcs status/log off subprocess onto library-backed adapters
  *(revisits 0169's deliberate subprocess choice for `status`/`log`)*
- 0200 — Decide whether git log/diff belong in vcs guard's blocked subcommand
  set *(spike; reconsiders the blocklist 0169 carried over verbatim)*
- 0204 — RemoteTracker Port *(split from 0194 on 2026-08-10; the frozen
  port signature, which precedes both 0171's client adapters and 0194)*
- 0194 — Tracker Crate and Remote Sync Engine *(split from 0170 on
  2026-08-05; also wires `--push` onto 0170's `create`/`update` commands,
  and precedes 0171's cutover half)*
- 0170 — Work-Item Lifecycle Subdomain
- 0171 — Jira and Linear Integrations
  - 0210 — Provider Client Crates over the RemoteTracker Port
  - 0211 — Integration Binaries and Bash Cluster Retirement
  - 0212 — Work-Item Script Cutover
  - 0213 — Conversational Conflict Resolution Flow for Sync
- 0172 — Migration Engine Subdomain
- 0202 — Reconcile Migration-Engine ADRs Against the Rust Port *(ADR-0038's
  shape drifted when 0172 ported the engine to native Rust)*
- 0173 — Remaining Subdomains: corpus, design, collaboration *(abandoned
  2026-08-05 — split into 0195/0196/0197, see Drafting Notes)*
- 0195 — accelerator-corpus: ADR, Metadata, Frontmatter Validation, and
  Linkage CLI
- 0196 — accelerator-design: Design Inventory and Gap Tooling CLI
  - 0206 — Classify Navigation URLs, Not Only the Initial Location
  - 0207 — Detect Encoded Credentials in Scrubbed Artefacts
  - 0209 — Wire Up or Retire the Header-Auth Path
  - 0214 — Settle the Vendored-Runtime Tree-Artifact Mechanisms
- 0197 — accelerator-collaboration: PR Helper CLI

**Cleanup (Phase 11):**
- 0185 — Converge corpus-adapters on the Library-Backed VCS Adapter
- 0199 — Retire `scripts/vcs-common.sh`'s Residual Shell Callers and
  `hooks/launcher-link-refresh.sh`
- 0174 — Empty scripts/ and Retire Shell Tooling and CI Guards
- 0221 — Canonical Quoting Standard for All Frontmatter
- 0203 — Ship an MPL-2.0 Attribution Artefact with the Release Uploads *(filed
  2026-08-10 from 0185's licensing finding; joins the release upload set 0165
  owns, and the notice obligation predates 0185 for four of the five affected
  binaries)*

**Launcher hardening and warm-path performance:**
- 0189 — At-Most-Once Guarantee for the Launcher's Cache-Root Probe
- 0190 — acquire_lock Misclassifies an Unusable Lock Directory and Can Spin
  Unbounded on Reclaim
- 0191 — Batch the Bootstrap's Two Shim Hashes into One sha256 Invocation
- 0205 — Close the Warm-Dispatch Latency Measurement Method *(spike)*
- 0215 — Remove the Cache-Hit sha256 from Warm Dispatch *(filed 2026-08-17 from
  0189's measurement; 6.05 ms of a 35.53 ms dispatch; must preserve the
  name/version binding the signature does not carry)*
- 0216 — Close the sha2 Hardware-Intrinsics Gap *(spike: sha256 at ~550 MB/s
  against openssl's 1,708, and BLAKE2b outruns it 2.6x; may make 0215 moot)*
- 0217 — Measure Warm Dispatch on Linux *(0189 verified darwin-arm64 only;
  darwin-x64 and linux-arm64 have no CI lane at all)*
- 0218 — Bound Cache-Root Growth *(`cache::find` scans a never-evicted directory
  on every dispatch)*
- 0219 — Own the Recurring Absolute-Budget Check *(0189's primary gate is
  re-runnable in principle and re-run by nothing)*

**Post-migration remnants (reconciled 2026-09-05):** recovered from an audit of
work items numbered above 0136 that belonged to the migration or its shipped
crates but had been left parentless. Post-migration *evolution* of the workspace
(refactors, ergonomics, crate renames) is out of scope here and lives under the
successor epic 0276.

- 0158 — Modular Rust CLI Architecture and Hexagonal Workspace Layout *(the
  foundational architecture spike, ported from luminosity, that feeds this epic)*
- 0182 — bin/accelerator Requires CLAUDE_PLUGIN_ROOT in the Environment *(launcher
  bug)*
- 0184 — template_names Succeeds-With-Nothing on a Non-Installation Plugin Root
  *(bug in the shipped config-adapters crate)*
- 0201 — In-Process Section Diff *(removes work-adapters' shell-out to `diff -u`)*
- 0240 — Corpus Update Frontmatter Quote Roundtrip *(bug in the shipped corpus
  crate; pairs with 0221)*
- 0241 — Migrate False Dirty-Tree Detection *(bug in the shipped migrate crate)*
- 0245 — Invoke accelerator Directly in Skills *(completes the invocation-contract
  cutover)*
- 0264 — Remove Bash-Migration Negative-Assertion Tests *(retires migration
  scaffolding)*
- 0269 — Remove Bash References from the Jira and Linear Clients *(retires
  shell-to-Rust residue)*
- 0222 — Offline Populate Route for the Runtime Cache *(runtime-cache cluster)*
- 0223 — Bounding the Default Runtime Cache Root *(sibling of 0218)*
- 0225 — Advisory-Feed Monitoring for the Vendored Runtime Pins *(sibling of 0214)*
- 0226 — Audit Repo-Settable Config Keys for Executable-Path Injection *(hardens
  the shipped config crates)*

The target architecture (git-style `accelerator` launcher dispatching to
on-demand `accelerator-<sub>` static binaries, each a hexagonal crate; the
visualiser folded in as the first sub-binary) is fixed by
ADR-0045/0046/0047/0051/0052/0053/0054.

## Acceptance Criteria

- [ ] Shell-script responsibilities are migrated into a Rust CLI without
      regressing skill behaviour, with the migration sequenced so the plugin
      stays functional at each step.
- [ ] Every work item in the Decomposition list above — direct children and the
      grandchildren indented beneath their parents — reaches a terminal status
      (done, or abandoned where superseded by a split).

## Open Questions

*(All resolved — see the architecture research's decision log. The two original
questions are answered:)*

- Which scripts migrate first, and which remain as shell? — Leaf/shared crates
  and the launcher first, the migration engine last; a thin residual shell
  surface (launcher bootstrap, hook wrapper, Playwright executor) remains under
  the bash 3.2 floor (ADR-0048/0049).
- How is the CLI distributed? — Zero-setup, fully static musl/darwin binaries
  fetched, sha256+minisign-verified, and exec'd on demand, reusing the existing
  visualiser release pipeline (ADR-0046).

## Dependencies

- Blocked by: None.
- Blocks: None directly (the children carry the internal dependency spine).
- Children (direct, parented to this epic): 0162–0174, 0185–0191, 0194–0200,
  0202–0205, 0215–0219 and 0221.
- Grandchildren (covered transitively by their parent's completion): 0178–0180
  under 0166; 0210–0213 under 0171; 0206, 0207, 0209 and 0214 under 0196.

## Assumptions

- The Rust CLI is distributed similarly to the existing visualiser binary
  (release artefact + checksum verification), confirmed and extended with
  minisign by ADR-0046.

## Technical Notes

- Removes the bash 3.2 floor constraint for migrated functionality; a thin
  residual shell surface remains and stays under the floor (ADR-0048/0049).
- Must preserve bare-path invocation semantics expected by skill
  `allowed-tools`; the cache lives under `${CLAUDE_PLUGIN_ROOT}` to keep
  permission matches working (the contract cutover is 0167).

## Drafting Notes

- Treated as an epic per the user's instruction. Decomposed 2026-06-28 from the
  scope/architecture research into children 0162–0174, with all open questions
  resolved interactively; promoted from `draft` to `ready`.
- **Extended 2026-07-31 with 0185–0188**, when 0169 was split. A four-pass
  review measured that three editing passes had not reduced its major-finding
  count (19 → 15 → 14 → 14), with most later findings being defects introduced
  by the previous pass's fixes — the signature of one work item carrying more
  than a single document can hold consistently. The extracted concerns are each
  independently deliverable: 0186 (bootstrap exec-probe fix, ~108 ms per hook
  invocation, benefits every SessionStart hook today), 0187 (sub-binary
  registration surface, unblocks four sibling subdomain stories), 0188 (the
  `gix`/`jj-lib` dependency adoption and its licence-policy change), and 0185
  (converging the remaining `CommandProbe` consumer). 0169 retains the
  subdomain, the hooks migration and the skill repoint — the parts that cannot
  be separated, since the shell hooks cannot be deleted before their
  replacements exist.
- **Extended 2026-08-05 with 0195–0197, when 0173 was split.** A review-1 pass
  on 0173 found it bundled three functionally independent efforts —
  `accelerator-corpus`, `accelerator-design`, and `accelerator-collaboration` —
  sharing no relationship beyond the registration pattern, each with its own
  source scripts, target crate(s), skill domain, and test suite; the combined
  scope was also judged undersized for a single `story`. 0173 is now
  **abandoned**; 0195/0196/0197 each carry forward their slice of 0173's
  Requirements/Acceptance Criteria (with the review's AC and Dependencies gaps
  fixed) and are parented directly under this epic.
- **Reconciled the decomposition list on 2026-08-31.** A transitive-closure walk
  of parent-links (rather than this hand-maintained list) found the written
  decomposition had drifted from the real child set: nine direct children were
  missing — 0189, 0190, 0191 (launcher hardening), 0198, 0199, 0200 (VCS
  residuals), 0202 (migration-engine ADR reconciliation), 0205 (warm-dispatch
  measurement spike) and 0221 (canonical frontmatter quoting) — and every
  grandchild under 0171 (0210–0213) and 0196 (0206, 0207, 0209, 0214) was
  unlisted. The list, the Children line and the second Acceptance Criterion now
  derive from the real closure; the criterion references the Decomposition list
  itself rather than an ID range, which is the form that drifted.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
- Research:
  `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Research:
  `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- Spike:
  `meta/work/0158-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md`
- ADRs: ADR-0045, ADR-0046, ADR-0047, ADR-0048, ADR-0049, ADR-0051, ADR-0052,
  ADR-0053, ADR-0054
