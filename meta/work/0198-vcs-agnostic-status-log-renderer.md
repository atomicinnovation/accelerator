---
type: "work-item"
id: "0198"
title: "Replace vcs status/log with a VCS-agnostic library-backed renderer"
date: "2026-08-06T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "story"
priority: "low"
parent: "work-item:0136"
blocked_by: ["work-item:0169", "work-item:0188"]
relates_to: ["work-item:0125", "work-item:0185", "work-item:0199", "work-item:0200", "work-item:0201"]
derived_from: ["plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
tags: ["rust", "vcs", "cli", "performance"]
last_updated: "2026-08-30T22:29:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-728"
---

# 0198: Replace vcs status/log with a VCS-agnostic library-backed renderer

**Kind**: Story
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer running `/commit` in either a git or a jj repo, I want `vcs
status` and `vcs log` to give the skill its changed-file list and recent-commit
context without shelling out to `jj`/`git`, so that commit authoring keeps
working with no runtime dependency on those binaries being installed on `PATH`.

`accelerator-vcs`'s `status` and `log` subcommands currently shell out to the
real `jj`/`git` binaries (`run_vcs_text` in `cli/vcs-adapters/src/subprocess.rs`,
wired through `cli/vcs-cli/src/status.rs`/`log.rs`) — the last two `vcs`
subcommands still spawning an external process. This item replaces them with a
single VCS-agnostic renderer: one status/log output format, computed in-process
from `gix` (git) and `jj-lib` (jj) and rendered in that single format for both
backends. Reproducing each tool's native CLI text is an explicit non-goal — we
own the format.

## Context

0169 chose subprocess for these two deliberately. Its Key Discoveries recorded
why: "`vcs status`/`vcs log` cannot be produced from the six taxonomy queries —
none of them render `jj status`/`git diff --stat`-shaped text, and
reimplementing that formatting against `gix`/`jj-lib` would be a
disproportionate undertaking with no byte-parity guarantee." The
`vcs_adapters::subprocess` module — shelling `jj`/`git` under a scrubbed
environment — was already an established, first-class adapter pattern, so
`status` and `log` followed it rather than inventing a new one.

Since 0198 was drafted, 0185 (done) retired the subprocess `CommandProbe`
entirely — moving detect/classify onto the library-backed adapter and deleting
it. That leaves `status` and `log` as the only remaining users of
`vcs_adapters::subprocess`: `run_vcs_text` and the two public entry points are
now the sole reason the module exists.

Today's implementation (`run_vcs_text` in `subprocess.rs`) runs exactly four
commands — `jj status`, `jj log --limit 5`, `git diff --cached --stat`, `git
log --oneline -5` — under a scrubbed environment and a 10-second cap, falling
back to a literal `(... unavailable)` string on any failure (never itself
fails).

These outputs have exactly one in-repo consumer: the `/commit` skill
(`skills/vcs/commit/SKILL.md`) injects `vcs status --fail-safe` and `vcs log
--fail-safe` into its prompt as orientation for authoring commits — the
changed-file list drives grouping into atomic commits, and recent commit
subjects set the message style. No hook reads them, nothing machine-parses
them, and the skill runs in one repo at a time, so it never compares git and jj
output. That narrow, human-readable use is what makes owning a single format
safe: the fields a lowest-common-denominator format would drop (jj change-ids,
parent/bookmark lines, the log graph and working-copy marker, per-commit
author/date) are jj-specific orientation the commit skill does not use — with
one exception, conflict state, which the `/commit` skill surfaces to the
developer authoring the commit and which both backends can express.

The subprocess choice is revisited here with a different resolution than 0169
imagined. Research (2026-08-30) confirmed neither backend exposes its native
rendering as a reusable library API: jj's template engine, graph-log renderer,
and status-summary formatting all live in the `jj-cli` binary crate, not
`jj-lib`. Rather than reimplement jj's graph engine or keep shelling out, this
item drops native-CLI parity as a goal and defines a single VCS-agnostic output
computed from the data both `gix` and `jj-lib` do expose (working-copy diff,
revsets, commit-graph walks).

**Motivation**: no runtime dependency on `jj`/`git` being installed and on
`PATH` for these two subcommands (mirroring 0125's argument for `detect`/`guard`);
the several-fold latency difference 0125 measured (~3.6-4.7 ms cold
in-process against ~23.8 ms per subprocess round-trip); with 0185 done,
`status`/`log` are the last two `vcs` subcommands still spawning a process; and
a single in-process renderer over both backends is simpler to build and test
than two native-shaped reproductions. The uniformity is an implementation and
test convenience, not a user-facing benefit — the sole consumer runs in one
repo at a time and never compares the two.

## Requirements

- Capture the VCS-agnostic status/log output format in a dedicated ADR —
  fields, ordering, change-type markers, conflict indicator, per-subcommand
  fallback text, and log depth — authored as the first deliverable of this
  item, before the backend adapters are built. The format changes the text the
  `/commit` skill injects, so it is recorded as a cross-cutting decision, not
  left implicit in the code.
- Define the VCS-agnostic **status** output: a backend-neutral header (branch
  or bookmark), a working-copy change summary (counts and a per-file change
  list with change-type markers), and an explicit **conflict / unmerged-path
  indicator** — specified once and rendered in the same format from git and jj
  data. The summary carries only these; no parent commit and no ahead/behind.
- The conflict indicator is not optional: it is the one native signal the sole
  consumer benefits from, so it must survive the move to an agnostic format.
  Both backends can express it (jj conflicts, git unmerged paths).
- Define the VCS-agnostic **log** output: a flat list of the five most recent
  commits, each a short id plus subject — no ASCII DAG graph, no author or date.
- Compute the data in-process and feed one renderer: `gix` for git
  (working-copy status/diff plus a revwalk for recent commits); `jj-lib` for jj
  (`working_copy`/`diff` for the change set, `revset`/`graph`/`dag_walk` for
  recent commits).
- Rewrite the goldens (`cli/vcs-cli/tests/status_log_goldens.rs` and
  `cli/vcs-test-support/fixtures/vcs-status-log/`) to the ADR format. Byte-parity
  with native `jj`/`git` output is an explicit non-goal.
- Preserve today's contract: `status`/`log` never fail — an unconditional
  in-subcommand fallback to the ADR-defined per-subcommand text on any adapter
  failure (matching the shell's original `2>/dev/null || echo` behaviour) — and
  stay diagnosable via `ACCELERATOR_LOG` on that fallback path. This is a
  distinct failure domain from the dispatch-layer `--fail-safe` flag the
  `/commit` skill passes: the internal fallback handles adapter failures, while
  `--fail-safe` handles fetch/dispatch failures (per 0169).
- Delete `vcs_adapters::subprocess` once both backends are library-backed —
  since 0185 removed `CommandProbe`, `status`/`log`/`run_vcs_text` are all that
  remain in it.

## Acceptance Criteria

- [ ] The VCS-agnostic status/log format is captured in a dedicated ADR —
      fields, ordering, change-type markers, conflict indicator, per-subcommand
      fallback text, and log depth (five) — and that ADR is authored as the
      first deliverable of this item, before the backend adapters are built.
- [ ] Given `jj` and `git` shadowed at every absolute path a library could reach
      (`/usr/bin`, `/usr/local/bin`, `/opt/homebrew/bin` and the jj equivalents,
      per 0188's strong form — a `PATH`-only stub cannot observe an absolute-path
      spawn from `gix`/`jj-lib`), when `vcs status` and `vcs log` run over the
      `vcs-status-log` fixture matrix (the shapes inherited from 0169: clean and
      dirty git, ahead/behind, detached HEAD, clean and dirty jj, colocated, jj
      secondary workspace, no repository), then each matches its ADR-defined
      golden and no subprocess is launched — the zero-spawn assertion running on
      and extending 0188's privileged Linux CI job (`check-zero-spawn`), which
      absolute-path shadowing requires (it cannot run on SIP-protected macOS).
- [ ] Given the same logical repository state — one untracked file and one
      modified tracked file (plus one staged change on the git side, which jj has
      no equivalent for), over three prior commits, built identically in a git
      repo and a jj repo — when `vcs status` (and `vcs log`) runs, then both
      backends produce output with the same field labels, ordering, and line
      structure defined by the ADR, with volatile values (ids, timestamps,
      branch/bookmark names) normalised by a named mask set
      (`cli/vcs-test-support/fixtures/masks.toml`); no mask may be added to
      rescue a failing comparison, and an unmasked control must show the masks
      cover only volatile values.
- [ ] A dirty-repo status golden — known untracked, modified, and (git-only)
      staged files — pins the exact change-type markers and counts, with AC1's
      ADR deciding whether staging collapses into "modified" in the agnostic
      format; and a log golden asserts exactly five entries of short id plus
      subject, with a negative assertion that no author, date, or ASCII-graph
      characters appear.
- [ ] Given a working copy with a conflict/unmerged path — built in each backend
      by a merge with conflicting edits to one tracked file — when `vcs status`
      runs, then the output contains the ADR-defined conflict marker together
      with the unmerged path name, for both git and jj.
- [ ] The goldens in `status_log_goldens.rs` are updated to the ADR format;
      byte-parity with native `jj`/`git` output is explicitly not required.
- [ ] The never-fail contract holds under fault injection: with an adapter
      forced to fail by a test-only failing adapter (never file permissions),
      `vcs status` and `vcs log` each yield the exact per-subcommand fallback
      text the ADR defines, and an `ACCELERATOR_LOG` run on that path emits a
      diagnostic line containing the failed adapter's name token (`gix` or
      `jj-lib`).
- [ ] `vcs_adapters::subprocess` is deleted, both backends now library-backed.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

- Does `jj-lib`'s pre-1.0 API instability (no stable-API promise, symbol churn
  every release, downstream consumers pinning per jj version) impose an
  acceptable maintenance cost for the jj data path? Resolve this alongside AC1's
  ADR, before the jj adapter is built. The Acceptance Criteria assume full
  migration; if jj-lib is judged unacceptable, this item is deliberately
  re-scoped — the jj adapter and the `vcs_adapters::subprocess` deletion move to
  a follow-up item and this item ships git-only — rather than passing in a
  degraded form.

## Dependencies

- Blocked by: work-item:0169 (done) — built `accelerator-vcs` and the
  subprocess `status`/`log` implementation this item replaces.
- Blocked by: work-item:0188 (done) — landed the `gix`/`jj-lib` dependency
  trees, the library-backed `vcs-adapters` module this item extends, and the
  cargo-pup library-reads rule this item must widen to cover the status/log
  code.
- External crates: `gix` (git data) and `jj-lib` (jj data), coupling this item
  to the shared dependency-policy artefacts 0188 established — the
  `cli/deny.toml` MPL licence exception, the multi-way version pin (jj-lib, gix,
  the Rust toolchain, the jj CLI pin, prost, pollster), the single-`gix`-version
  invariant, and `gix` pinned `default-features = false`. The working-copy
  status/diff surface this item needs may require a `gix` feature the current
  selection omits; enabling one re-opens the shared deny.toml and
  licence-closure review, and any feature enabled must preserve
  `default-features = false` and must not re-admit the subprocess-spawning
  `gix-credentials` subsystem, or it breaks this item's own zero-subprocess
  premise (0188's invariant, enforced by the cargo-pup `std::process` deny this
  item widens).
- Relates to: work-item:0125 — the performance/no-PATH-dependency argument
  this item borrows; work-item:0185 (done) — the sibling half of the
  `vcs_adapters::subprocess` retirement, which removed `CommandProbe` and left
  `status`/`log` as the module's only users, a completed precondition for the
  full-module-deletion criterion (without it this item could migrate
  `status`/`log` but delete only `run_vcs_text`, not the whole module);
  work-item:0199, work-item:0200,
  work-item:0201 — adjacent `vcs`-subdomain and subprocess-to-in-process work
  sharing this item's pattern and no-PATH-dependency motivation.
- Parent: epic 0136.

## Assumptions

- The common-denominator fields chosen for the agnostic format are expressible
  from both `gix` and `jj-lib` — e.g. jj's bookmark maps onto git's branch for
  the status header, both expose a working-copy change set and a conflict/
  unmerged signal, and both expose recent-commit id plus subject.

## Technical Notes

- Data sources: git via `gix` (working-copy status/diff for the change set, a
  revwalk over `gix::Repository` for recent commits); jj via `jj-lib`
  (`working_copy`/`local_working_copy` plus `diff` for the change set,
  `revset`/`graph`/`dag_walk` for recent commits). Neither backend's native CLI
  rendering is reusable — jj's lives in `jj-cli`, not `jj-lib` — so a shared
  renderer over a backend-neutral data struct is the shape: one function turns
  the struct into text; the two adapters populate it.
- Scope boundary: `status`/`log` stay orientation-only. If a future consumer
  needs jj-native richness (operation log, change evolution, change-ids), it
  should get a separate structured subcommand rather than re-enriching these
  two. The item is deliberately whole: the git and jj adapters are the only
  decomposition seam, both feed one ADR-defined renderer, and the
  `vcs_adapters::subprocess` deletion gates on the second — so splitting would
  leave a half-migrated state with no standalone value.
- Prior art: `gulbanana/gg` links `jj-lib` directly and renders its own commit
  graph from library data — confirming the data is reachable and the rendering
  must be ours.
- `jj-lib` is pre-1.0 with no stable-API guarantee; downstream GUIs pin to and
  track each jj release. Weigh this standing maintenance cost for the jj data
  path.

## Drafting Notes

- Reframed the item from "migrate while preserving native output" to "replace
  with a VCS-agnostic renderer" per the chosen direction (2026-08-30);
  byte-parity with `jj`/`git` CLI output is now an explicit non-goal. Changed
  kind task→story and proposed a new title to match.
- Verified the sole in-repo consumer is the `/commit` skill, using the text as
  human-readable orientation (changed-file list plus recent subjects), not
  machine-parsed and not read by hooks. That finding is what justifies agnostic
  as non-degrading; it also made conflict state a hard requirement, since it is
  the one native signal that consumer benefits from.
- Corrected an earlier over-claim of my own: cross-VCS uniformity is an
  implementation and test simplification here, not user-facing value — the
  consumer runs in one repo at a time and never compares backends.
- Chose the flat-log, no-graph shape from the selected option; the
  reimplement-native (ASCII-DAG) alternative was set aside as higher-cost, since
  jj's graph engine is `jj-cli`-layer, not in `jj-lib`.
- Removed the `CommandProbe` carve-out and reference-list entry: 0185 (done)
  deleted it (verified zero occurrences in `cli/`). Corrected fixture/masks
  paths from `hooks/test-fixtures/` to `cli/vcs-test-support/fixtures/`
  (relocated when 0174 folded the shell fixtures into the `cli/` workspace).
- Kept priority low and status draft (no request to change); kept 0169 in
  `relates_to` and reconciled the body's "Blocked by" wording to match.
- Acted on review 1 (2026-08-30): routed the output format to a dedicated ADR
  sequenced as the first deliverable (AC1) and resolved the format Open
  Questions — log lines carry short id plus subject only (no author/date), the
  status summary stays minimal (header, change list, conflict; no parent or
  ahead/behind), depth stays five. Added a PATH-stripped behavioural criterion,
  pinned the never-fail fault-injection mechanism and a mask-based parity
  comparison, and gave the conflict criterion a fixture recipe. Named `gix`,
  `jj-lib`, and 0188 (with its deny.toml and version-pin coupling) in
  Dependencies, and aligned the "rendered identically" wording to the
  shape/format sense.
- Acted on the re-review pass (2026-08-30): hardened the no-subprocess criterion
  to 0188's absolute-path shadowing (a `PATH`-only stub cannot see a library's
  absolute-path spawn), enumerated the `vcs-status-log` fixture matrix and the
  parallel-state recipe, and added ordinary-content goldens (change markers and
  counts; a five-entry log with a negative author/date/graph assertion). Added
  `blocked_by` frontmatter to mirror the prose blockers, distinguished the
  adapter-failure and dispatch `--fail-safe` domains, tied any new `gix` feature
  to the zero-subprocess invariant, gave the jj-lib Open Question a resolution
  path and fallback, and softened "order-of-magnitude" to "several-fold".
- Corrected two defects the third review pass found in the pass-2 edits
  (2026-08-30): the git-only fallback now reads as a deliberate re-scope into a
  follow-up item (jj adapter plus module deletion), not a degraded pass, so the
  "both backends" criteria are not silently contingent; and the parity and
  golden fixtures qualify the staged change as git-only (jj has no staging area),
  with AC1's ADR deciding whether staging collapses into "modified". Also named
  0185 as a completed precondition for the full-module deletion, pointed AC2's
  zero-spawn assertion at 0188's `check-zero-spawn` Linux CI job, and carried
  0169's mask-closure rule into the parity criterion.

## References

- `cli/vcs-adapters/src/subprocess.rs` — `status`, `log`, `run_vcs_text`
- `cli/vcs-cli/src/status.rs`, `cli/vcs-cli/src/log.rs`
- `cli/vcs-cli/tests/status_log_goldens.rs`
- `cli/vcs-test-support/fixtures/vcs-status-log/`,
  `cli/vcs-test-support/fixtures/masks.toml`
- `skills/vcs/commit/SKILL.md` — the sole consumer; injects `vcs status`/`vcs
  log` as commit-authoring orientation
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — Key Discoveries,
  "`vcs status`/`vcs log` cannot be produced from the six taxonomy queries"
- `meta/work/0125-converge-vcs-detection-on-probe-layer.md` — the
  no-PATH-dependency and cold-call performance evidence this item echoes
- `meta/work/0185-converge-corpus-adapters-on-library-backed-vcs.md` — the
  sibling half of the `vcs_adapters::subprocess` retirement that removed
  `CommandProbe`
- `meta/work/0188-library-backed-vcs-adapter.md` — the `gix`/`jj-lib`
  in-process precedent this item extends
- Jujutsu architecture (library/CLI crate split, templates as a CLI-crate
  concern): https://docs.jj-vcs.dev/latest/technical/architecture/
- `jj-lib` public API surface: https://docs.rs/jj-lib/latest/jj_lib/
- `gulbanana/gg` — a GUI that links `jj-lib` and renders its own graph:
  https://github.com/gulbanana/gg
