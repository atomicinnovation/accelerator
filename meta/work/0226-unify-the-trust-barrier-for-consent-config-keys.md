---
type: "work-item"
id: "0226"
title: "Unify the Trust Barrier for Consent Config Keys"
date: "2026-08-20T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0136"
relates_to: ["work-item:0196", "work-item:0227"]
tags: ["security", "config", "design", "tracker"]
last_updated: "2026-09-24T12:32:55+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Folded in stress-test decisions: widened from executable keys to consent keys (adding jira.allowed_sites), one shared command runner, a fail-closed tracking check, a three-root repository definition, design daemon restart, and unified refusal codes."
schema_version: 1
external_id: "PP-756"
---

# 0226: Unify the Trust Barrier for Consent Config Keys

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Five config keys hold a value only the user may supply — a *consent key* — and
each is guarded by its own hand-written, mutually inconsistent trust barrier.
Four of them name something the tooling executes (`design.browser_path`,
`jira.token_cmd`, `linear.token_cmd`, `github.token_cmd`); the fifth,
`jira.allowed_sites`, decides which hosts receive the Jira token. Replace the
barriers with a single policy that a key opts into by declaration in the config
catalogue, so a repository can never set a consent key, never name or supply a
binary the tooling runs, and a future consent key inherits the barrier without
writing one.

## Context

🔒 A repo-tracked config value that names an executable the tooling later runs is
an untrusted-input-to-code-execution path: cloning a hostile repository is
enough to have the tooling execute an attacker-chosen binary. Two distinct
barriers close it. The personal-only barrier stops the repository *setting* the
key; the repo-inside barrier stops the repository *supplying the binary* a
personally-set key points at (a README saying "set
`browser_path: ./tools/chromium`", or a relative path resolving into a hostile
checkout).

The personal-only barrier is not specific to executables. `jira.site` is
deliberately team-settable, and a host outside `*.atlassian.net` is admitted
only when it appears in `jira.allowed_sites`: the repository proposes, the user
consents. That consent is only meaningful while `jira.allowed_sites` is
personal-only, or a repository could approve its own exfiltration host.

This item was originally framed around `visualiser.editor`. Investigation showed
that key is never spawned: the frontend builds a `vscode://`/`jetbrains://` or
custom-template deep link rendered as an `<a href>`, guarded by a scheme
deny-list. It is out of scope for a spawned-binary barrier.

The consent keys today:

| Key | Team-level value | Tracked `config.local.md` | Value checks |
|---|---|---|---|
| `design.browser_path` | ignored, warning | not checked | repo-inside refused, warning |
| `jira.token_cmd`, `linear.token_cmd` | hard error only when `config.local.md` is absent; else silently ignored | hard error | none; cwd is the config root |
| `github.token_cmd` | hard refusal | not checked | none; inherited cwd and env, no timeout or cap |
| `jira.allowed_sites` | hard error | hard error | n/a |

The `design.browser_path` repo-inside check has further gaps a generalised
policy must not copy: a relative value is canonicalised against the executor's
cwd but launched raw, and the Playwright daemon that receives it at spawn is
reused per repository root, so a later executor's vetting never reaches a
running daemon; "the repository" is the current jj workspace or git worktree
root, so a binary inside the main checkout passes; and a present-but-empty
personal value masks the team value, suppressing the team-level warning. The
`token_cmd` keys run via `bash -c` with cwd set to the config root, so
`token_cmd: ./scripts/token.sh` executes a repository-controlled script —
parsing shell to find paths is unsound, so the fix is to move the cwd.

All three `Provenance` implementations treat a failed VCS tracking query as
"untracked", so the tracked-file barrier disappears whenever the VCS
misbehaves.

The `SessionStart` config summary emits its only existing warning (unknown skill
customisation names) to stderr, which 0183 established reaches nobody; warnings
must travel in the hook's `additionalContext`.

## Requirements

1. Add a trust attribute to the config catalogue that declares a key as a
   consent key, with executable kinds — path-valued and command-valued — that
   extend it.
2. Provide one shared policy in the `cli/config` crate that every consumer of a
   consent key resolves through, which:
   - always inspects the team level, refusing a team-level value even when a
     present-but-empty or whitespace-only personal value would otherwise mask
     it, and returning any winning personal value alongside the refusal;
   - refuses a value from `config.local.md` when that file is VCS-tracked, or
     when a VCS is detected but the tracking query fails; with no VCS detected,
     the file is untracked;
   - leaves the 0600 and no-symlink invariants to the store, which enforces
     them on every personal read.
3. Define "the repository" as the union of the canonicalised config root, the
   current workspace root and the main repository root; with no VCS, the config
   root alone.
4. For path-valued keys, the policy additionally requires an absolute path,
   refuses a value that canonicalises inside the repository (including via a
   symlink, or a nonexistent file whose parent lies inside), and hands the
   canonical path to the launcher.
5. Provide one shared runner, owned by the policy, for every command-valued key:
   `bash -c` with a cwd that cannot lie inside the repository (not `$HOME`,
   which may itself be a repository root), a scrubbed environment, null stdin,
   a timeout and an output cap.
6. Apply the value checks (Requirements 4 and 5) to `ACCELERATOR_*` environment
   overrides of consent keys; provenance checks do not apply to them.
7. Keep refusal severity per key: each consumer decides whether a refusal falls
   back with a warning or fails with an error, and a refusal is fatal only when
   it leaves no usable value.
8. Emit unified refusal codes carrying the key name, retiring
   `E_TOKEN_CMD_FROM_SHARED_CONFIG`, `AllowlistFromSharedConfig` and the
   bespoke `github.token_cmd` and `design.browser_path` wordings.
9. `accelerator config summary --format=hook` places in `additionalContext` a
   warning for each team-level consent key naming the personal route, and a
   warning when `config.local.md` is VCS-tracked or its tracking status cannot
   be determined.
10. Record, in the design daemon, the canonical browser path it launched with,
    and restart the daemon when an executor's vetted path differs.
11. Migrate `design.browser_path`, `jira.token_cmd`, `linear.token_cmd`,
    `github.token_cmd` and `jira.allowed_sites` onto the policy, deleting the
    bespoke barriers, the three duplicated `Provenance` implementations and
    `ACCELERATOR_ALLOW_INSECURE_LOCAL`, and rewording every shipped doc and
    skill that names a retired refusal code.
12. Document `visualiser.editor` as exempt: a browser-side link, never spawned.

Out of scope: `visualiser.editor` and other browser-built links; `$VISUAL`/
`$EDITOR` fallbacks; provenance checks on `ACCELERATOR_*` environment
overrides; a team-level plaintext `github.token`; binaries in an outer
repository enclosing a nested checkout; removing the unconsumed
`visualiser.binary` key.

## Acceptance Criteria

- [ ] Given a team `config.md` sets any consent key and no personal value
      exists, when a consumer resolves it, then the value is not used and the
      refusal carries the key name and names `.accelerator/config.local.md` as
      the route.
- [ ] Given a team `config.md` sets any consent key and the personal config
      sets the same key to an empty or whitespace-only value, when a consumer
      resolves it, then the team-level refusal is still reported.
- [ ] Given a team `config.md` sets any consent key and the personal config
      sets a usable value, when a consumer resolves it, then the personal value
      is used, the team-level refusal is reported, and the consumer does not
      fail.
- [ ] Given a team `config.md` sets any consent key, when a session starts,
      then the `SessionStart` hook's `additionalContext` contains a warning
      naming the key and the personal route.
- [ ] Given `config.local.md` is VCS-tracked, when any consent key resolves,
      then it is refused — including `design.browser_path`,
      `github.token_cmd` and `jira.allowed_sites` — and when a session starts,
      `additionalContext` warns that the file is tracked.
- [ ] Given a VCS is detected but the tracking query for `config.local.md`
      fails, when any consent key resolves, then it is refused with a reason
      stating the tracking check could not run, and `SessionStart`
      `additionalContext` warns likewise.
- [ ] Given no VCS is detected, when any consent key resolves, then
      `config.local.md` is treated as untracked.
- [ ] Given `config.local.md` is a symlink or not mode 0600, when any consent
      key resolves, then its value is never used.
- [ ] Given `jira.site` names a host outside `*.atlassian.net`, when the Jira
      client resolves it, then it is admitted only if the host is listed in a
      personal, untracked `jira.allowed_sites`.
- [ ] Given a personal or `ACCELERATOR_*` environment `design.browser_path`
      that is relative, or that canonicalises inside the config root, the
      current workspace root or the main repository root (directly, via a
      symlink, or as a nonexistent file whose parent is inside), when the
      design tooling launches, then the value is refused with a warning and the
      bundled browser is used.
- [ ] Given no VCS is detected, when a `design.browser_path` canonicalises
      inside the config root, then it is refused.
- [ ] Given an accepted `design.browser_path`, when the browser launches, then
      the launched executable is the canonical path that was vetted.
- [ ] Given a running design daemon launched with a different browser path
      (including none) than the one now vetted, when the design tooling runs,
      then the daemon is restarted with the vetted path.
- [ ] Given a personal or `ACCELERATOR_*` environment `token_cmd` for any of
      `jira`, `linear` or `github`, when the command runs, then it runs through
      the shared runner and its working directory lies outside every
      repository root, including when `$HOME` is itself a repository root.
- [ ] Given a personal `github.token_cmd` of `gh auth token`, when it runs
      through the shared runner, then it still yields a token.
- [ ] Given the migration is complete, when the shipped docs and skills are
      searched, then no retired refusal code (e.g.
      `E_TOKEN_CMD_FROM_SHARED_CONFIG`) remains.
- [ ] Given a new catalogue key declared as a consent key (or an executable
      key), when a consumer resolves it with no key-specific barrier code, then
      every applicable criterion above applies to it.

## Dependencies

- Relates to: 0196 — established the barrier on `design.browser_path`.
- Relates to: 0227 — `accelerator config validate` can reuse the policy.

## Assumptions

- The process environment is the user's own trust boundary for *provenance*: an
  `ACCELERATOR_*` value is never refused as team-level or tracked. Because a
  repository can still set such variables through `mise.toml` `[env]` or
  `.envrc`, env values remain subject to the value checks (absolute, outside
  every repository root, canonical launch path, command cwd outside the
  repository).
- A binary inside an outer repository that contains the current nested
  checkout or submodule is not refused; walking every ancestor for VCS markers
  is not worth the cost.
- Moving the `github.token_cmd` onto the shared runner (scrubbed environment,
  timeout, output cap, null stdin) is an accepted behavioural change.
- A team-level plaintext `github.token` still wins before the `token_cmd`
  refusal; it is not executed, so it stays outside this policy.
- Moving the `token_cmd` cwd outside the repository is an accepted breaking
  change for anyone relying on a repo-relative script path; the refusal and
  documentation are sufficient notice. No shipped doc, skill or template shows
  such a path — every example is an `op read …` or `gh auth token` in
  `config.local.md`.

## Technical Notes

- Catalogue: `cli/config/src/catalogue.rs` holds flat `const` tables of key
  names with only an optional default; the executable keys and
  `jira.allowed_sites` live in `EXTRA_KEYS` (:131-148). The attribute needs a
  new table or a descriptor element type.
- Provenance: `Source::{Personal, Team, Catalogue, Unset}` in
  `cli/config/src/service.rs:50-55`; `effective_nonempty` (:428-440) is what
  collapses an empty personal value to `Unset`, and `env_beats_config`
  (`cli/config/src/precedence.rs:21-26`) trims a whitespace-only one.
- Existing barriers to replace: `cli/design-cli/src/config.rs:51-82` and
  `cli/design/src/runtime/browser_path.rs:34-90`;
  `cli/tracker-support/src/credentials.rs:243-409`;
  `cli/collaboration-cli/src/auth.rs:67-160`;
  `cli/jira-client/src/auth.rs:157-181` (`allowed_sites`).
- Duplicated `Provenance`: `cli/jira-cli/src/context.rs:86-110`,
  `cli/linear-cli/src/context.rs:67-91`,
  `cli/work-cli/src/tracker_registry.rs:80-112`. All fail open on a tracking
  query error (`unwrap_or(false)`; `tracing::warn!` in `work-cli`).
- Store-level invariants: `require_secure_personal_file`
  (`cli/config-adapters/src/store.rs:174-193`, applied at `:210`, `:260`)
  refuses a symlinked or group/other-readable personal file on every read, which
  likely makes `ACCELERATOR_ALLOW_INSECURE_LOCAL` (`credentials.rs:422-432`)
  unreachable — confirm with a test before deleting it.
- Repository roots: `FileConfigStore::discover_root` (`store.rs:113-125`) gives
  the config root — nearest `.accelerator/`, `.git` or `.jj`, not canonicalised;
  `vcs_adapters::facts(&cwd).root` gives the workspace root;
  `repository_root()` (`cli/vcs-adapters/src/library.rs:600-619`) gives the main
  repository.
- Shared runner: derive from `CommandPolicy` in
  `cli/tracker-support/src/credentials.rs:124-133,450-459`, which already clears
  the environment to `PATH`, `HOME`, `TERM`, nulls stdin, and caps at 30s and
  64KiB. `gh auth token` may need `GH_*` variables admitted.
- Design daemon: the browser path reaches it only at spawn via
  `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` (`cli/design-cli/src/executor.rs:442-445`
  → `skills/design/inventory-design/scripts/playwright/lib/daemon.js:209-210`);
  the daemon's state dir is per repository root (`executor.rs:375-376`) and
  `DaemonSpawner::spawn` sets no cwd (`cli/design-adapters/src/process.rs:144-156`).
- Summary pipeline: `cli/launcher/src/config_command/core/summary.rs:38-79` →
  `render/summary.rs` `hook_envelope`; existing warnings go to stderr via
  `render::emit` (`render/mod.rs:43-48`). `assemble` already receives
  `ConfigAccess` and `ReadConfigLevel`, so team-level detection needs no new
  plumbing. The hook runs `--fail-safe` (`hooks/hooks.json:18`).
- Retired codes in docs: `skills/config/configure/SKILL.md:764-801,877-881` and
  its mirror under `docs-site/src/content/docs/reference/skills/config/`.

## Drafting Notes

- Dropped the `visualiser.editor` premise on evidence that it is never spawned,
  and retitled from an audit to the unification the audit implies.
- Read "generalise" as a catalogue-declared attribute plus one shared policy,
  not a reusable helper each consumer calls ad hoc.
- Chose to refuse relative paths outright rather than resolve them against a
  chosen base.
- Widened from executable keys to consent keys after a stress test found
  `jira.allowed_sites` already carries the same provenance barrier; retitled
  accordingly.
- Kept severity with each consumer rather than on the catalogue descriptor,
  since whether a fallback exists is consumer knowledge.
- Kept this a single task despite its breadth; planning phases it along the
  dependency order policy → runner → consumers and summary.

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items)
- Related: 0196, 0227, 0183, 0080, 0272
