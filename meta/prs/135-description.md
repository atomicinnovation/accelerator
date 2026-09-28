---
type: "pr-description"
id: "135"
title: "[0226] Put consent config keys behind one trust barrier"
date: "2026-09-28T08:57:26+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0226"
parent: "work-item:0226"
relates_to: ["work-item:0280", "work-item:0298"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/135"
pr_number: 135
tags: ["security", "config", "consent", "credentials", "session-start"]
revision: "adb072c4970484ff177d62ef3e32458a0f3bc183"
repository: "accelerator"
last_updated: "2026-09-28T08:57:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0226] Put consent config keys behind one trust barrier

## Summary

Six config keys hold a value only the user may supply: the five executable
keys `design.browser_path`, `jira.token_cmd`, `linear.token_cmd`,
`github.token_cmd` and `openalex.api_key_cmd`, and `jira.allowed_sites`, which
decides which hosts receive the Jira token. Each had its own hand-written,
mutually inconsistent trust barrier. This PR replaces them with one **consent
policy**, which a key opts into by declaring a `Trust` attribute in the config
catalogue. Under it, a team-level or VCS-tracked value can never set a consent
key, a path key can never launch a binary inside the repository, and a command
key never runs with the repository as its working directory or environment.

> Stacked on #134 (`0280-academic-source-profiles`). Review only the commits
> above it.

## Changes

### Consent policy (`cli/config/src/consent.rs`, `catalogue.rs`)

- The catalogue declares `Trust::{Open, Consent, PathConsent,
  CommandConsent { admitted_environment }}` per key. `ConsentKey`,
  `CommandKey` and `ExecutablePathKey` read the team level eagerly, then the
  environment, then the personal level lazily.
- Every refusal now uses one code family: `E_CONSENT_KEY_TEAM_LEVEL`,
  `E_CONSENT_KEY_TRACKED` and `E_CONSENT_KEY_TRACKING_UNKNOWN`. These replace
  `E_TOKEN_CMD_FROM_SHARED_CONFIG`, `E_TOKEN_CMD_FROM_TRACKED_FILE` and
  `E_ALLOWED_SITES_FROM_SHARED_CONFIG`.
- A refusal falls through to the next credential source as a `warning:`, and
  is fatal only when nothing usable remains. Severity is decided in exactly
  one place, `or_fallback`.
- A `notice:` line names the environment variable that decided a consent key.
  It never prints the command.

### Fail-closed tracking (`consent-adapters`, `vcs-adapters`, `vcs-cli`)

- 🔒 An undeterminable tracking status for `config.local.md` now gives
  `Unknown` and refuses the key, where it used to count as untracked. A failed
  canonicalisation or an unusable `.jj` or `.git` marker also gives `Unknown`.
- The new `consent-adapters` crate holds the adapters that need VCS, so
  `gix` and `jj-lib` stay out of `config-adapters` and its dependents, the
  launcher and the visualiser server among them. `pup.ron` denies
  `std::process` in the crate.
- `repository_roots` resolves the three roots every value check is measured
  against: the config root, the current workspace and the main repository.

### Hardened command runner (`config-adapters/src/command_runner.rs`)

- 🔒 Every credential command, `github.token_cmd` included, runs through
  `BashCommandRunner`. It runs in a fresh temporary directory outside the
  repository roots, and it refuses the command when no such directory exists.
- The environment is cleared down to `PATH`, `HOME`, `TERM`, the XDG and D-Bus
  variables, and each key's `admitted_environment` (`GH_HOST` and
  `GH_CONFIG_DIR` for GitHub).
- `PATH` entries that are relative, empty, missing or inside the repository
  are dropped, both before and after canonicalisation.
- The command gets null stdin and its own process group. stdout and stderr
  share one 64 KiB output cap, and exceeding it gives
  `E_COMMAND_OUTPUT_EXCEEDED`.
- A timed-out command gives `E_COMMAND_TIMED_OUT`. On exit the runner sends
  `SIGTERM` to the process group and escalates to `SIGKILL` after a bounded
  grace period. Interrupts are forwarded the same way, then re-raised.

### Path-valued `design.browser_path` (`design`, `design-cli`, `design-adapters`)

- A relative value is refused with `E_EXECUTABLE_PATH_RELATIVE`.
- A value whose canonical target is inside the repository roots is refused
  with `E_EXECUTABLE_PATH_INSIDE_REPOSITORY`. So is a value that cannot be
  shown to lie outside them. Symlinks are followed for at most 40 hops.
- The crawler launches the canonical target it vetted, and falls back to the
  bundled browser, with a warning, when the value is refused.
- The Playwright daemon's state now lives in one slot per browser, `bundled`
  or `custom-<digest>`, beneath `inventory-design-playwright/`. A daemon runs
  only the browser it was spawned for, and a symlinked state directory or
  slot fails the launch.

### Consent warnings at session start (`launcher`, `kernel`, `vcs-cli`)

- The `SessionStart` summary names every consent key set in
  `.accelerator/config.md`, or in a `config.local.md` that is tracked or whose
  tracking is unknown.
- The launcher gets the tracking answer from a new `accelerator vcs tracking`
  sub-command through a bounded capture: its own process group, a 4 KiB cap
  and a group kill before the reap. That keeps the launcher off `vcs`,
  `vcs-adapters` and `consent-adapters`, using `kernel::TrackingAnswer` as the
  shared type.
- An insecure `config.local.md` (symlinked, or looser than `0600`) is ignored
  with one `E_LOCAL_PERMS_INSECURE` warning, where it used to fail every
  command. Writers still refuse. The hook summary carries the warning in its
  envelope only, so it is not repeated on stderr.

### Retirement and docs

- `ACCELERATOR_ALLOW_INSECURE_LOCAL` and the `.accelerator/allow-insecure-local`
  marker are removed.
- `ACCELERATOR_JIRA_ALLOWED_SITES` is added.
- `/accelerator:configure` gains a `## Consent Keys` reference listing the
  six keys, their shared rule and every refusal code with its remedy. The
  configuration, research, collaboration and visualiser docs link to
  it.
- The new `lint:config-test-support:check` confines the
  `config/test-support` feature to the dev-dependencies of `config` and
  `config-adapters`. It is wired into `cli:check` and `lint:check`.
- `CHANGELOG.md` records every behaviour change under Breaking, Added,
  Changed, Removed and Fixed.

## Breaking changes

⚠️ Credential helpers see a different world. The changelog's Breaking section
is authoritative. The changes most likely to break someone:

- **Environment**: a helper needing `SSH_AUTH_SOCK`, `OP_SESSION_*` or STS
  variables no longer sees them. The remedy is to export the resolved token
  as `ACCELERATOR_JIRA_TOKEN`, `ACCELERATOR_LINEAR_TOKEN`,
  `ACCELERATOR_OPENALEX_API_KEY` or `GH_TOKEN`, or to use `env VAR=… cmd`.
- **Working directory and `PATH`**: the working directory is no longer the
  project root, and repo-local or relative `PATH` entries are gone.
- **Interactive helpers**: a helper that prompts on `/dev/tty` now fails or
  times out, because the command has no controlling terminal.
- **Signals**: a helper that traps only `INT` must now trap `TERM`.
- **Oversized output**: output over the cap is refused rather than
  truncated.
- **Refusal codes**: the codes are renamed, and linear-cli exit codes are
  split into 25, 27 and 29, where all three used to be 24. Scripts that match
  stderr need updating.
- **OpenAlex**: a team `openalex.api_key_cmd` beside a key-less personal file
  now fails the fetch instead of going keyless.
- **GitHub precedence**: a personal `github.token_cmd` now wins over a team
  `github.token`, matching the trackers.

## Context

- Work item: `meta/work/0226-unify-the-trust-barrier-for-consent-config-keys.md`
  (PP-756). It replaces the narrower
  `0226-audit-repo-settable-config-keys-for-executable-path-injection.md`,
  which is deleted.
- Research: `meta/research/codebase/2026-09-24-0226-unify-the-trust-barrier-for-consent-config-keys.md`
- Plan (7 phases, reviewed over six passes):
  `meta/plans/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys.md`
- Validation (pass):
  `meta/validations/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys-validation.md`
- Adds work item 0298, which consolidates the `SessionStart` hooks behind
  `accelerator hooks session-start`. It is filed as follow-up work, not
  implemented here.

## Testing

- [ ] `mise run` at the branch tip exited 1 on one visualiser e2e test.
      `aside-row-resolved-colours.spec.ts:25` timed out in `page.goto`
      (`ERR_ABORTED`) on both attempts under the full parallel load.
      Everything else passed, 3992 Rust tests included.
- [x] Re-run of `mise run test:e2e:visualiser` alone: 355 passed and
      1 skipped, so the failure looks like a load flake. This branch changes
      the visualiser only by logging an ignored personal config.
- [x] Validation run: `mise run` exited 0 after the post-validation fixes
- [x] New test suites:
  - `config/tests/consent.rs` and `config/tests/credentials.rs`
  - `config-adapters/tests/{runner,personal_file,paths}.rs`
  - `vcs-adapters/tests/{file_tracking,roots}.rs` and
    `vcs-cli/tests/tracking.rs`
  - `design-cli/tests/{browser_path,executor_preflight}.rs`
  - `{jira,linear}-cli/tests/flow_consent.rs` and
    `work-cli/tests/{consent,personal_file}.rs`
  - `launcher/tests/config_personal_file.rs`
  - `collaboration-cli/tests/end_to_end.rs`
- [x] Edge cases pinned by tests:
  - a refused team command falls through to a personal token;
  - no temporary directory outside the repository refuses the command;
  - the hook summary reports the insecure-file warning exactly once;
  - symlinked daemon slots fail the launch.
- [ ] Attended manual checks, none of them run yet:
  - [ ] A team `jira.allowed_sites` warns `E_CONSENT_KEY_TEAM_LEVEL` and
        `jira search` still runs
  - [ ] A personal `jira.token_cmd: op read op://…` yields a token through
        1Password
  - [ ] A personal `github.token_cmd: gh auth token` works through both the
        Linux Secret Service keyring and `gh`'s config under `$HOME`
  - [ ] A team `jira.token_cmd` beside a personal `jira.token` warns and
        succeeds
  - [ ] Design: an absolute Chrome outside the repository is used, and
        `./chromium` warns and falls back to the bundled browser
  - [ ] Daemons: switching browsers starts a `custom-…` daemon, and the
        bundled one idles out; the same browser keeps its pid, and unsetting
        the key reuses the bundled daemon
  - [ ] Session start: a team `github.token_cmd` is named in the
        start-of-session message, and the wall-time delta is acceptable on a
        second repository
  - [ ] `/accelerator:configure` explains the consent keys and the
        `visualiser.editor` exemption coherently

## Notes for Reviewers

- **Where to start**: `cli/config/src/consent.rs` holds the policy,
  `cli/config-adapters/src/command_runner.rs` the runner, and
  `cli/vcs-adapters/src/tracking.rs` the fail-closed tracking. Most other code
  changes route a consumer through one of these three.
- **Diff size**: GitHub cannot render this diff (over 20,000 lines). About
  7,000 of those lines are `meta/` documents, the plan alone 3,221. Review
  code locally with
  `jj diff --from 0280-academic-source-profiles --to 0226-consent-key-trust-barrier`.
- **Known limitations** from validation, left as they are:
  - `GH_TOKEN` and `GITHUB_TOKEN` share one rung, so a malformed `GH_TOKEN`
    hides a valid `GITHUB_TOKEN`.
  - work-cli passes the config root as the working directory for the runner's
    roots, where Jira and Linear pass the actual working directory.
  - The runner's poll loop can busy-wait on a non-`EINTR` error until its
    deadline, and its final `wait` is unbounded.
  - After `jj workspace forget`, the tracking status reads as `Untracked`
    rather than `Unknown`.
  - A tracked `config.local.md` that sets no consent key is still flagged at
    session start.
- **Incidental change**: `.accelerator/state/integrations/linear/last-sync.json`
  changes only the 0226 (PP-756) entry, which was synced when the work item
  was refined.
