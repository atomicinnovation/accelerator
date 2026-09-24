---
type: "work-item"
id: "0226"
title: "Unify the Trust Barrier for Consent Config Keys"
date: "2026-08-20T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0136"
blocks: ["work-item:0227"]
relates_to: ["work-item:0196", "work-item:0272", "work-item:0183", "work-item:0172", "work-item:0080"]
tags: ["security", "config", "design", "tracker"]
last_updated: "2026-09-24T13:45:35+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Addressed review 1 pass 3: reclassified as a story, delivery phases moved to Technical Notes, severity matrix per refusal reason, bash-boundary environment criteria, runner kill/cap/tmpdir criteria, daemon fall-back restart, precise config-root and file-mode boundaries."
schema_version: 1
external_id: "PP-756"
---

# 0226: Unify the Trust Barrier for Consent Config Keys

**Kind**: Story
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
catalogue, so team or tracked config files can never set a consent key, a
path-valued consent key can never launch a binary inside the repository, a
command-valued key never runs with the repository as its working directory, and
a future consent key inherits the barrier without writing one. The work also
moves every command-valued key onto one shared runner, makes a failed VCS
tracking check refuse rather than admit, has the `SessionStart` hook warn about
refused team-level values and a tracked `config.local.md`, and restarts the
Playwright daemon when its vetted browser changes.

## Context

🔒 A repo-tracked config value that names an executable the tooling later runs is
an untrusted-input-to-code-execution path: cloning a hostile repository is
enough to have the tooling execute an attacker-chosen binary. Two kinds of check
close it. *Provenance checks* stop the repository *setting* the key: a
team-level value is refused, and so is a value from a VCS-tracked
`config.local.md`. *Value checks* stop the repository *supplying the binary* a
personally set key points at (a README saying "set
`browser_path: ./tools/chromium`", or a relative path resolving into a hostile
checkout): the path rules of Requirement 4 and the shared runner of
Requirement 5. "The repository" throughout means the three roots of
Requirement 3.

Provenance checks are not specific to executables. `jira.site` is deliberately
team-settable, and a host outside `*.atlassian.net` is admitted only when it
appears in `jira.allowed_sites`: the repository proposes, the user consents.
That consent is only meaningful while `jira.allowed_sites` is personal-only, or
a repository could approve its own exfiltration host.

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

The design executor — the per-invocation design CLI process that vets
`design.browser_path` and then spawns or reuses the Playwright daemon — has
further gaps a generalised policy must not copy: a relative value is
canonicalised against the executor's cwd but launched raw, and the Playwright
daemon receives the path only at spawn and is reused while its state directory
(keyed on a single root) survives, so a
later executor's vetting never reaches a running daemon; the executor treats
only the current jj workspace or git worktree root as inside, so a binary
inside the main checkout passes; and a present-but-empty personal value masks the team value,
suppressing the team-level warning. The `token_cmd` keys run via `bash -c` with
cwd set to the config root, so `token_cmd: ./scripts/token.sh` executes a
repository-controlled script — parsing shell to find paths is unsound, so the
fix is to move the cwd.

All three `Provenance` implementations treat a failed VCS tracking query as
"untracked", so the tracked-file refusal disappears whenever the VCS
misbehaves.

The `SessionStart` config summary emits its only existing warning (unknown skill
customisation names) to stderr at exit 0, which reaches nobody (0183). 0172
moved the migrate advisory onto the hook's `systemMessage`, the channel the user
sees. A consent-key warning needs both channels: `systemMessage` because only
the user can move a key into `config.local.md`, and `additionalContext` so
Claude can explain a refusal when it surfaces mid-session.

## Requirements

1. Add a single trust attribute to the config catalogue with three values:
   consent, path-valued consent and command-valued consent. "Executable key"
   is shorthand for the latter two; each adds value checks on top of the
   provenance checks every consent key carries.
2. Provide one shared policy in the `cli/config` crate that every consumer of a
   consent key resolves through. Its provenance checks, which apply only to
   values read from config files:
   - always inspect the team level, refusing a team-level value even when a
     present-but-empty or whitespace-only personal value would otherwise mask
     it, and return any winning personal value alongside the refusal
     (precedence: environment, then personal, then team);
   - refuse a value from `config.local.md` when that file is VCS-tracked, or
     when a VCS is detected but the tracking query fails; with no VCS detected,
     the file is untracked;
   - leave the 0600 and no-symlink invariants to the config file store
     (`FileConfigStore`), which enforces them on every personal read.
3. Define "the repository" as the union of the canonicalised config root (the
   directory containing the nearest `.accelerator/`, `.git` or `.jj` entry,
   searched upward from the working directory), the current workspace root and
   the main repository root; with no VCS, the config root alone.
4. For path-valued keys, the policy's value checks require an absolute path,
   refuse a value that canonicalises inside the repository (including via a
   symlink, or a nonexistent file whose parent lies inside), and hand the
   canonical path to the consumer that launches the binary.
5. Provide one shared runner, owned by the policy, through which every
   command-valued key runs whatever its source: `bash -c` with its cwd set to
   a fresh, empty temporary directory created per run and removed afterwards;
   an environment cleared to `PATH`, `HOME` and `TERM`, plus `GH_HOST` and
   `GH_CONFIG_DIR` for `github.token_cmd` only, each passed only when the parent
   sets it; null stdin; a 30s timeout that kills every process the command
   started; and a 65,536-byte cap on stdout and stderr combined.
6. Apply the value checks — Requirement 4 and the shared runner in full — to
   `ACCELERATOR_*` environment overrides of consent keys; provenance checks do
   not apply to them.
7. Keep refusal severity with each consumer. A *usable value* is a winning
   personal or environment value that passes every check, or the consumer's
   own fallback (the bundled browser; the default `*.atlassian.net` admission;
   another configured credential, such as a plaintext token). A refusal is
   fatal exactly when no usable value remains; otherwise the consumer warns and
   continues. A consumer reports a refusal — fatal or not — on the invoking
   command's stderr, carrying the refusal code and the key name; the
   `SessionStart` warnings of Requirement 9 are what reach the user
   unprompted. A team-level value is always reported, even when another
   credential wins.
8. Emit one refusal code per reason, each carrying the key name:
   `E_CONSENT_KEY_TEAM_LEVEL`, `E_CONSENT_KEY_TRACKED`,
   `E_CONSENT_KEY_TRACKING_UNKNOWN`, `E_EXECUTABLE_PATH_RELATIVE`,
   `E_EXECUTABLE_PATH_INSIDE_REPOSITORY`, `E_COMMAND_TIMED_OUT` and
   `E_COMMAND_OUTPUT_EXCEEDED`. Retire the `E_TOKEN_CMD_FROM_SHARED_CONFIG`
   code, the `AllowlistFromSharedConfig` error variant, and the bespoke
   `github.token_cmd` and `design.browser_path` refusal messages.
9. `accelerator config summary --format=hook` emits, in both the hook's
   `systemMessage` and its `additionalContext`, a warning for each team-level
   consent key naming the personal route (`.accelerator/config.local.md`), and
   a warning whenever `config.local.md` exists and is VCS-tracked or its
   tracking status cannot be determined, whatever keys it sets.
10. Record, in the Playwright daemon, the canonical browser path it launched
    with, or none for the bundled browser; restart the daemon when an
    executor's vetted value differs — including a fall-back to the bundled
    browser because the path is unset or refused — and reuse it when the value
    matches.
11. Migrate `design.browser_path`, `jira.token_cmd`, `linear.token_cmd`,
    `github.token_cmd` and `jira.allowed_sites` onto the policy, deleting the
    bespoke barriers, the three duplicated `Provenance` implementations,
    `ACCELERATOR_ALLOW_INSECURE_LOCAL` and its `.accelerator/allow-insecure-local`
    marker, and rewording every shipped doc and skill that names a retired
    refusal code.
12. Document `visualiser.editor` as exempt: a browser-side link, never spawned.

Out of scope: `visualiser.editor` and other browser-built links; `$VISUAL`/
`$EDITOR` fallbacks; provenance checks on `ACCELERATOR_*` environment
overrides; a team-level plaintext `github.token`; binaries in an outer
repository enclosing a nested checkout; a personal `token_cmd` that names an
absolute path to a script inside the repository (the user wrote the command,
and parsing shell to find paths is unsound); removing the unconsumed
`visualiser.binary` key.

## Acceptance Criteria

Provenance:

- [ ] Given a team `config.md` sets any consent key and no personal value
      exists, when a consumer resolves it, then the value is not used and the
      refusal carries `E_CONSENT_KEY_TEAM_LEVEL`, the key name, and
      `.accelerator/config.local.md` as the route.
- [ ] Given a team `config.md` sets any consent key and the personal config
      sets the same key to an empty or whitespace-only value, when a consumer
      resolves it, then the team-level refusal is still reported.
- [ ] Given a team `config.md` sets any consent key and the personal config
      sets a usable value, when a consumer resolves it, then the personal value
      is used, the team-level refusal is reported, and the consumer does not
      fail.
- [ ] Given `config.local.md` is VCS-tracked, when any consent key resolves,
      then the value read from `config.local.md` is refused with
      `E_CONSENT_KEY_TRACKED` — including for `design.browser_path`,
      `github.token_cmd` and `jira.allowed_sites`.
- [ ] Given a VCS is detected but the tracking query for `config.local.md`
      fails, when any consent key resolves, then the value read from
      `config.local.md` is refused with `E_CONSENT_KEY_TRACKING_UNKNOWN`.
- [ ] Given `config.local.md` is VCS-tracked and a valid `ACCELERATOR_*`
      override of the same consent key, when the key resolves, then the
      override is used.
- [ ] Given no VCS is detected, when any consent key resolves, then
      `config.local.md` is treated as untracked.
- [ ] Given `config.local.md` is a symlink or has any mode granting group or
      other permissions (e.g. 0640, 0604), when any consent key resolves, then
      its value is never used (modes 0600 and 0400 are accepted) — including
      when
      `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` is set and the
      `.accelerator/allow-insecure-local` marker is present.
- [ ] Given `jira.site` names a host outside `*.atlassian.net`, when the Jira
      client resolves it, then it is admitted only if the host is listed in a
      usable `jira.allowed_sites` — personal and untracked, or an
      `ACCELERATOR_*` override.

Path-valued keys and the Playwright daemon:

- [ ] Given a personal or `ACCELERATOR_*` environment `design.browser_path`
      that is relative, whatever it would resolve to, when the design tooling
      launches, then the value is refused with `E_EXECUTABLE_PATH_RELATIVE`, a
      warning, and the bundled browser is used.
- [ ] Given a personal or `ACCELERATOR_*` environment `design.browser_path`
      that is absolute and canonicalises inside the config root, the current
      workspace root or the main repository root (directly, via a symlink, or
      as a nonexistent file whose parent is inside), when the design tooling
      launches, then the value is refused with
      `E_EXECUTABLE_PATH_INSIDE_REPOSITORY`, a warning, and the bundled browser
      is used.
- [ ] Given a team-level `design.browser_path` and no personal or environment
      value, when the design tooling launches, then it warns with
      `E_CONSENT_KEY_TEAM_LEVEL` and uses the bundled browser.
- [ ] Given no VCS is detected, when a `design.browser_path` canonicalises
      inside the config root, then it is refused.
- [ ] Given a personal `design.browser_path` that is an absolute symlink
      outside the repository pointing to a browser outside the repository,
      when the browser launches, then the launched executable is the symlink's
      canonical target.
- [ ] Given a running Playwright daemon launched with a different browser path
      (including none) than the one now vetted, when the design tooling runs,
      then a new daemon process replaces it, recording the vetted path.
- [ ] Given a running Playwright daemon launched with a custom browser path,
      when the design tooling runs with `design.browser_path` unset or refused,
      then a new daemon process replaces it, recording none and using the
      bundled browser.
- [ ] Given a running Playwright daemon launched with the canonical path now
      vetted, when the design tooling runs, then the same daemon process is
      reused.

Command-valued keys and the shared runner:

- [ ] Given a personal or `ACCELERATOR_*` environment `token_cmd` for any of
      `jira`, `linear` or `github`, when the command runs, then it runs through
      the shared runner and its working directory is an empty temporary
      directory outside the repository and other than `$HOME`, including when
      `$HOME` is itself a VCS root.
- [ ] Given a `token_cmd` that prints its working directory, when it runs
      twice, then the two directories differ and neither exists after its run
      ends — including a run ending in `E_COMMAND_TIMED_OUT` or
      `E_COMMAND_OUTPUT_EXCEEDED`.
- [ ] Given a `token_cmd` of `sleep 60 & wait`, when it runs, then the runner
      returns `E_COMMAND_TIMED_OUT` within 35s and no process the command
      started survives.
- [ ] Given a `token_cmd` whose stdout and stderr together total 65,536 bytes,
      when it runs, then its output is accepted; given 65,537 bytes, then it is
      refused with `E_COMMAND_OUTPUT_EXCEEDED`.
- [ ] Given a parent environment setting `PATH`, `HOME`, `TERM`, `GH_HOST`,
      `GH_TOKEN` and an unrelated variable, when a `jira.token_cmd` or
      `linear.token_cmd` runs, then the environment handed to `bash` contains
      exactly `PATH`, `HOME` and `TERM`, and the command's stdin reads
      end-of-file.
- [ ] Given a parent environment setting `PATH`, `HOME`, `TERM`, `GH_HOST`,
      `GH_CONFIG_DIR`, `GH_TOKEN` and an unrelated variable, when a
      `github.token_cmd` runs, then the environment handed to `bash` contains
      exactly `PATH`, `HOME`, `TERM`, `GH_HOST` and `GH_CONFIG_DIR`, with the
      parent's values.
- [ ] Given a parent environment without `TERM`, when any `token_cmd` runs,
      then the environment handed to `bash` has no `TERM`.
- [ ] Given `gh` is installed and authenticated through its config under
      `$HOME` (a manual check; the preceding criterion is its repeatable
      counterpart), and a personal `github.token_cmd` of `gh auth token`, when
      it runs through the shared runner, then it yields a token.

Severity:

- [ ] Given each refusal reason a consumer can meet, when a consumer refuses
      for it, then it follows this matrix, reporting the code and key name on
      stderr in both cases (`design.browser_path` always has the bundled
      browser, so it always warns):

      | Refusal code | Usable value remains | No usable value |
      |---|---|---|
      | `E_CONSENT_KEY_TEAM_LEVEL` | warns, continues | exits non-zero |
      | `E_CONSENT_KEY_TRACKED` | warns, continues | exits non-zero |
      | `E_CONSENT_KEY_TRACKING_UNKNOWN` | warns, continues | exits non-zero |
      | `E_EXECUTABLE_PATH_RELATIVE` | warns, continues | exits non-zero |
      | `E_EXECUTABLE_PATH_INSIDE_REPOSITORY` | warns, continues | exits non-zero |
      | `E_COMMAND_TIMED_OUT` | warns, continues | exits non-zero |
      | `E_COMMAND_OUTPUT_EXCEEDED` | warns, continues | exits non-zero |

- [ ] Given a team-level `jira.token_cmd`, `linear.token_cmd` or
      `github.token_cmd` and no usable value, when the tracker command runs,
      then it exits non-zero with `E_CONSENT_KEY_TEAM_LEVEL`.
- [ ] Given a `jira.allowed_sites` set only at team level and a `jira.site`
      outside `*.atlassian.net`, when the Jira client resolves the site, then
      it fails with an error carrying `E_CONSENT_KEY_TEAM_LEVEL`.
- [ ] Given a team-level `jira.token_cmd`, `linear.token_cmd` or
      `github.token_cmd` and another configured credential for that tracker,
      when the tracker command runs, then it succeeds using that credential
      and warns with `E_CONSENT_KEY_TEAM_LEVEL`.
- [ ] Given a `jira.allowed_sites` set only at team level and a `jira.site`
      inside `*.atlassian.net`, when the Jira client resolves the site, then it
      succeeds and warns with `E_CONSENT_KEY_TEAM_LEVEL`.

Session start:

- [ ] Given a team `config.md` sets any consent key, when a session starts,
      then both the `SessionStart` hook's `systemMessage` and its
      `additionalContext` contain a warning naming the key and the personal
      route.
- [ ] Given `config.local.md` is VCS-tracked, or its tracking query fails,
      when a session starts, then both `systemMessage` and `additionalContext`
      name `.accelerator/config.local.md` and carry `E_CONSENT_KEY_TRACKED` or
      `E_CONSENT_KEY_TRACKING_UNKNOWN` respectively.
- [ ] Given no team-level consent key and an untracked `config.local.md`, when
      a session starts, then neither field contains a consent-key warning.
- [ ] Given consent-key warnings alongside the existing unknown-skill-name
      warning, when a session starts, then the hook exits 0 and prints exactly
      one JSON object carrying both `systemMessage` and `additionalContext`.

Unification and retirement:

- [ ] Given the five consent keys each refused for a provenance reason
      (team-level, tracked, tracking unknown), and each executable key refused
      for a value reason that applies to its kind, when the refusals are
      compared, then every refusal for a given reason carries the same code,
      differing only in the key name.
- [ ] Given the migration is complete, when `cli/`, `skills/` and
      `docs-site/` are searched, then none of `E_TOKEN_CMD_FROM_SHARED_CONFIG`,
      `AllowlistFromSharedConfig`, `ACCELERATOR_ALLOW_INSECURE_LOCAL` or
      `allow-insecure-local` remains.
- [ ] Given the migration is complete, when non-test code is searched, then no
      crate other than `cli/config` and its adapters queries the VCS tracking
      status of `config.local.md`.
- [ ] Given the configure skill and its docs-site mirror, when read, then they
      state that `visualiser.editor` is not a consent key because it is a
      browser-side link that is never spawned.
- [ ] Given test-only catalogue entries declared as (a) a consent key, (b) a
      path-valued key and (c) a command-valued key, resolved through the
      shared policy with no key-specific barrier code, then (a), (b) and (c)
      are each refused at team level and from a tracked or tracking-unknown
      `config.local.md`; (b) is additionally refused when relative or inside
      any repository root and yields its canonical path when accepted; and (c)
      additionally runs through the shared runner.

## Open Questions

- Is `ACCELERATOR_ALLOW_INSECURE_LOCAL` still reachable?
  `require_secure_personal_file` appears to refuse an insecure personal file
  before the override is consulted. Requirement 11 deletes the variable and its
  marker either way; settle reachability with a test first, and if it is
  reachable, record the deletion as an accepted behavioural change under
  Assumptions.

## Dependencies

- Parent: 0136 — the Rust CLI migration epic under which the barriers were
  ported.
- Blocks: 0227 — `accelerator config validate` resolves consent keys through
  this item's shared policy rather than writing its own checks; it needs only
  the catalogue attribute and the policy's provenance checks, so it can start
  once delivery phase 1 lands.
- Relates to: 0196 (done) — established the `design.browser_path` barrier and
  the Playwright daemon handoff this item replaces.
- Relates to: 0272 (done) — relocated the insecure-local marker that
  Requirement 11 deletes, together with 0272's marker-lookup code.
- Relates to: 0183 (abandoned, absorbed by 0172) — diagnosed the stderr dead
  channel.
- Relates to: 0172 (done) — established `systemMessage` as the `SessionStart`
  user channel Requirement 9 uses.
- Relates to: 0080 (done) — built the open-in-editor link that makes
  `visualiser.editor` exempt.
- External: the GitHub CLI (`gh`) — the manual `gh auth token` check needs it
  installed and authenticated, and the `GH_HOST`/`GH_CONFIG_DIR` admission is
  tied to how `gh` resolves its config.

## Assumptions

- The process environment is the user's own trust boundary for *provenance*: an
  `ACCELERATOR_*` value is never refused as team-level or tracked. Because a
  repository can still set such variables through `mise.toml` `[env]` or
  `.envrc`, env values remain subject to every value check.
- An existing `.accelerator/allow-insecure-local` marker becomes inert once
  Requirement 11 lands; no migration removes it, and the configure docs say it
  can be deleted.
- A binary inside an outer repository that contains the current nested
  checkout or submodule is not refused; walking every ancestor for VCS markers
  is not worth the cost.
- Moving `github.token_cmd` onto the shared runner (scrubbed environment,
  timeout, output cap, null stdin) is an accepted behavioural change.
- A team-level plaintext `github.token` still wins over a `github.token_cmd`;
  it is not executed, so it stays outside this policy. A team-level
  `github.token_cmd` alongside it is still reported as a warning.
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
  refuses a symlinked or group/other-readable personal file on every read;
  `ACCELERATOR_ALLOW_INSECURE_LOCAL` is consulted at `credentials.rs:422-432`.
- Repository roots: `FileConfigStore::discover_root` (`store.rs:113-125`) gives
  the config root — nearest `.accelerator/`, `.git` or `.jj`, not canonicalised;
  `vcs_adapters::facts(&cwd).root` gives the workspace root;
  `repository_root()` (`cli/vcs-adapters/src/library.rs:600-619`) gives the main
  repository.
- Shared runner: derive from `CommandPolicy` in
  `cli/tracker-support/src/credentials.rs:124-133,450-459`, which already clears
  the environment to `PATH`, `HOME`, `TERM`, nulls stdin, and caps at 30s and
  64KiB.
- Playwright daemon: the browser path reaches it only at spawn via
  `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` (`cli/design-cli/src/executor.rs:442-445`
  → `skills/design/inventory-design/scripts/playwright/lib/daemon.js:209-210`);
  the daemon's state dir is per repository root (`executor.rs:375-376`) and
  `DaemonSpawner::spawn` sets no cwd (`cli/design-adapters/src/process.rs:144-156`).
- Summary pipeline: `cli/launcher/src/config_command/core/summary.rs:38-79` →
  `render/summary.rs` `hook_envelope`; existing warnings go to stderr via
  `render::emit` (`render/mod.rs:43-48`). `assemble` already receives
  `ConfigAccess` and `ReadConfigLevel`, so team-level detection needs no new
  plumbing. The hook runs `--fail-safe` (`hooks/hooks.json:18`). The
  `systemMessage` precedent is `kernel::hooks::session_start`, used by
  `cli/migrate-cli/src/discoverability.rs`; one hook invocation may print only
  one JSON object, so both fields share an envelope.
- Retired codes in docs: `skills/config/configure/SKILL.md:764-801,877-881` and
  its mirror under `docs-site/src/content/docs/reference/skills/config/`.
- Delivery phases, each a separate rollback point, and each deleting the
  bespoke barrier of the keys it migrates:
  1. catalogue attribute, shared policy with the full refusal-code set,
     repository definition, fail-closed provenance and the
     `jira.allowed_sites` migration — closing the fail-open tracking gap for
     `jira.allowed_sites` and unblocking 0227;
  2. shared runner and the three `token_cmd` migrations — the two accepted
     breaking changes, revertable without touching phase 1, and the point at
     which the `token_cmd` fail-open gap closes;
  3. `design.browser_path` migration and the Playwright daemon restart;
  4. `SessionStart` warnings, and retirement of the shared leftovers — the
     `Provenance` duplicates, the insecure-local override and the doc
     rewording — only once phases 1–3 have landed.

## Drafting Notes

- Dropped the `visualiser.editor` premise on evidence that it is never spawned,
  and retitled from an audit to the unification the audit implies.
- Read "generalise" as a catalogue-declared attribute plus one shared policy,
  not a reusable helper each consumer calls ad hoc.
- Chose to refuse relative paths outright rather than resolve them against a
  chosen base.
- Widened from executable keys to consent keys after a stress test found
  `jira.allowed_sites` already carries the same provenance check; retitled
  accordingly.
- Kept severity with each consumer rather than on the catalogue descriptor,
  since whether a fallback exists is consumer knowledge.
- Kept as one item after review flagged its breadth, reclassified from task
  to story to reflect its size: it is one policy cutover, specified and
  reviewed as one contract, and delivered in the phases listed under
  Technical Notes.

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items)
- Related: 0196, 0227, 0183, 0172, 0080, 0272
