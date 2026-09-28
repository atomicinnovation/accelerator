---
type: "plan-validation"
id: "2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys-validation"
title: "Validation Report: Unify the Trust Barrier for Consent Config Keys Implementation Plan"
date: "2026-09-27T23:29:55+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys"
tags: ["security", "config", "consent", "credentials", "session-start"]
last_updated: "2026-09-27T23:29:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Unify the Trust Barrier for Consent Config Keys

Validated at `prtrorvlqyry` (working copy empty). The code faithfully
implements all seven phases, but the full local CI mirror fails. Phase 7's
docs link to `configure#consent-keys`, and that anchor does not exist on the
generated page. Phase 7's `docs:check` and `mise run` criteria are ticked
but do not hold at this revision.

### Implementation Status

✓ Phase 1: Consent policy, fail-closed tracking, `jira.allowed_sites` - Fully implemented
✓ Phase 2: Hardened command runner - Fully implemented
✓ Phase 3: Command-valued keys on the policy - Fully implemented
✓ Phase 4: Path-valued `design.browser_path` - Fully implemented
✓ Phase 5: One daemon per browser - Fully implemented
✓ Phase 6: Consent warnings at session start - Fully implemented
✓ Phase 7: Retirement and documentation - Fully implemented (broken anchor fixed during validation)

### Automated Verification Results

After the anchor fix, a rerun of `mise run` exited 0 in 408 s: 3990 Rust
tests passed, and the docs link check reported all internal links valid. The
consent keys section and its command runner now sit outside the `help` block,
as `## Consent Keys` in `skills/config/configure/SKILL.md`. The `help` action
now points to that section. The results of the first run follow.

✗ `mise run`: exit 1. `docs:build` fails because `starlight-links-validator`
  reports 5 invalid `#consent-keys` hashes. Their sources are
  `configuration.md:47`, `collaboration.md:71`, `visualiser.md:130`,
  `research.md:203` and the generated `reference/skills/config/configure.md:31`
  (from `skills/config/configure/SKILL.md:29`). The failure cancelled the
  in-flight test tasks, so they were rerun on their own:
✓ `mise run test:unit:cli`: 3990 passed, 1 skipped
✓ `test:integration:{conformance,dev,entrypoint,hooks,skill-invocation,visualiser}`:
  27, 17, 66, 29, 139 and 3 passed
✓ Completed within the full run: every format, lint and type task, plus
  `test:unit:{tasks,frontend,frontend-licenses,vcs,design-automation}`,
  `test:integration:{tasks,deny}`, `deny:check` and `notices:check`
✓ `mise run` left the working copy unchanged, so no formatter drift
✓ Phase 3 check, override names only in the catalogue: the only non-catalogue
  matches sit in `#[cfg(test)]` modules
✓ Phase 3 and Phase 7 check, no doc calls a team-level `_cmd` key ignored: no
  matches
✓ Phase 6 check, `file_tracking` confined to its permitted crates: no matches
✓ Phase 7 check, consent keys read only through the policy: the non-test
  matches are exactly the six sites accepted by name
✓ Phase 7 check, retired identifiers gone: no matches in `cli`, `skills` or
  `docs-site`
✓ `InProcessProbe::is_tracked` is `pub(crate)`
  (`cli/vcs-adapters/src/library.rs:442`)

The anchor failure has one root cause. In `skills/config/configure/SKILL.md`,
the whole `help` configuration reference (lines 112–1213) sits inside a
single fenced code block. `### consent keys` (line 1006) is therefore code,
not a heading, and produces no anchor. Commit `kxkxuqpkvotz` (Phase 7) added
all five links. None of the earlier 0226 commits contains them.

### Code Review Findings

#### Matches Plan:

- The catalogue declares a `Trust` attribute per key.
  `ConsentKey`/`CommandKey`/`ExecutablePathKey::declared` and `resolve_checked`
  read the team level eagerly, then the environment, then the personal level
  lazily (`cli/config/src/consent.rs:1081-1137`).
- Tracking fails closed. A failed canonicalisation or an unusable `.jj` or
  `.git` marker gives `Unknown`, and there is a tripwire
  (`cli/vcs-adapters/src/tracking.rs:91-183`). `std::process` is denied in
  `consent-adapters` (`cli/pup.ron:403`).
- `BashCommandRunner` does what the plan asks:
  - it clears the environment and passes through only `HOME` and the admitted
    list;
  - it filters `PATH` both raw and canonicalised;
  - it runs with null stdin, in its own process group, with a 64 KiB combined
    output cap;
  - on shutdown it sends `SIGTERM` to the group, waits a bounded grace period,
    then sends `SIGKILL`;
  - it re-raises interrupts after cleanup.
- The token ladder falls through from a refused team command to a personal
  token, and a test pins that behaviour (`cli/config/tests/credentials.rs:368`).
  Severity is decided only by `or_fallback`.
- The path checks follow the planned six steps, including following symlinks
  for at most 40 hops. A relative path warns with `E_EXECUTABLE_PATH_RELATIVE`
  and falls back to the bundled browser.
- The daemon state slot is `bundled` or `custom-<16 hex>`, and the symlink
  guards are in `cli/design-adapters/src/paths.rs:53-96`.
- `vcs tracking` and `kernel::TrackingAnswer` exist. The launcher depends on
  neither `vcs`, `vcs-adapters` nor `consent-adapters`. The capture is bounded:
  its own process group, a 4 KiB cap, and a group kill before the reap.
- The retirement is complete. `ACCELERATOR_ALLOW_INSECURE_LOCAL` and the old
  refusal codes appear only in `CHANGELOG.md`.

#### Deviations from Plan:

The Implementation Notes record the deviations below except where marked
unrecorded.

- ⚠️ Unrecorded: the session-start summary still reports the
  insecure-personal-file warning twice. `cli/launcher/src/main.rs:298` still
  calls `composed.report_ignored_personal_file()`, so `config summary
  --format=hook` writes `E_LOCAL_PERMS_INSECURE` to stderr and also puts it in
  the hook fields. The plan said the audit finding would replace the direct
  print "so the summary reports it once". No test asserts stderr for this
  case.
- Unrecorded: `GH_TOKEN` and `GITHUB_TOKEN` are not separate rungs.
  `environment_candidate` takes the first non-blank override
  (`cli/config/src/consent.rs:1411-1420`), so a malformed `GH_TOKEN` hides a
  valid `GITHUB_TOKEN` and the result is `E_TOKEN_MALFORMED`.
- Unrecorded: work-cli passes the config root where the cwd belongs
  (`cli/work-cli/src/tracker_registry.rs:146`, `credential_ports(&self.root,
  &self.root)`). Jira and Linear pass the cwd instead
  (`cli/jira-cli/src/context.rs:143`). Neither choice covers both
  repositories, so the runner's roots can miss the other repository's VCS
  roots.
- Unrecorded: the Phase 6 audit-error test (`core/summary.rs:330-349`)
  exercises only `consent_warnings`. It does not assert the complete summary
  the plan asked for.
- The plan contradicts itself on Phase 4's verification. Its Implementation
  Notes say the `mise run` criterion "stays unticked", but line 2399 is
  ticked.

#### Potential Issues:

- 🔒 The runner's `/tmp` fallback fails open
  (`cli/config-adapters/src/command_runner.rs:123-131`). When the preferred
  base lies inside the repository roots, or cannot be created, the runner
  falls back to `/tmp` without re-checking it. A checkout at `/`, `/tmp` or
  `/private/tmp` therefore runs the credential helper inside the repository.
- Legacy-layout users lose every personal consent key.
  `PERSONAL_CONFIG_RELATIVE` is hard-coded to `.accelerator/config.local.md`
  (`cli/config-adapters/src/credentials.rs:18`). Under the legacy fallback,
  though, the store reads `.claude/accelerator.local.md`
  (`cli/config-adapters/src/store.rs:168-173`). Tracking the absent path
  answers `Unknown`, so each such key is refused with
  `E_CONSENT_KEY_TRACKING_UNKNOWN`. This fails closed, but it is a silent
  regression for those users.
- The runner can spin: a `poll` error other than `EINTR` returns without
  sleeping (`command_runner.rs:252-254`), so the loop busy-waits until the
  deadline. The final `leader.wait()` (`:405`) has no bound.
- The daemon slot can drift mid-crawl. `daemon-stop` resolves the hatch
  afresh, so if `design.browser_path` changes during a crawl it cold-starts a
  Chromium in the new slot just to stop it. The original daemon then lives
  until its idle timeout.
- The audit flags any tracked `config.local.md` with `E_CONSENT_KEY_TRACKED`,
  even one that sets no consent key (`consent.rs:1156-1177`).
- The jj edge case: after `jj workspace forget`, `jj_is_tracked` returns
  `Ok(false)` (`library/tracked.rs:103-106`). That reads as `Untracked`, not
  `Unknown`.
- Comment policy:
  - `cli/config/src/catalogue.rs:646-647` is stale: "the executor reads it
    ad-hoc from the personal level".
  - `cli/design-adapters/src/paths.rs:19,29-30` still describes `state_dir` as
    the repository-relative temporary directory.
  - Doc comments that only restate the name: `capture.rs:33,62`,
    `kernel/src/tracking.rs:32` and `executor.rs:391`.
- The configure skill's severity wording (`SKILL.md:1030-1032`) counts "the
  key's built-in fallback" as a usable value. For OpenAlex, a keyless call is
  not a fallback: a team `api_key_cmd` beside a key-less personal file is
  fatal.

### Manual Testing Required:

These are the plan's unticked manual criteria, and none has been run.

1. Jira and credentials:
  - [ ] A team `jira.allowed_sites` in a scratch git repo makes `accelerator jira search …` warn `E_CONSENT_KEY_TEAM_LEVEL` and still run
  - [ ] A personal `jira.token_cmd: op read op://…` yields a token through 1Password
  - [ ] On a Linux desktop, a personal `github.token_cmd: gh auth token` backed by the Secret Service keyring yields a token
  - [ ] A personal `github.token_cmd: gh auth token`, with `gh` authenticated through its config under `$HOME`, lets `accelerator collaboration …` reach GitHub
  - [ ] A team `jira.token_cmd` beside a personal `jira.token` warns and succeeds
2. Design browser:
  - [ ] A personal absolute Chrome outside the repository is used for the crawl
  - [ ] `design.browser_path: ./chromium` warns `E_EXECUTABLE_PATH_RELATIVE` and crawls with the bundled browser
  - [ ] Switching browsers starts a new daemon in the `custom-…` slot, and the bundled daemon exits within `ACCELERATOR_PLAYWRIGHT_IDLE_MS`
  - [ ] Two crawls with the same custom browser keep the same daemon pid
  - [ ] Unsetting the key within the idle window reuses the still-running bundled daemon
3. Session start and docs:
  - [ ] A repo whose `config.md` sets `github.token_cmd` gets a start-of-session message naming the key and the personal route
  - [ ] The SessionStart wall-time delta is measured on a second repository and is acceptable
  - [ ] `/accelerator:configure` explains the consent keys and the `visualiser.editor` exemption coherently
  - [ ] Every `*_cmd` paragraph matches the severity matrix

### Recommendations:

- Make the launcher's direct `report_ignored_personal_file()` call conditional
  on the output format, so the hook summary reports the warning once. Add the
  stderr assertion the plan implied.
- Re-check `/tmp` against the repository roots, or refuse to run the command
  when no directory outside the roots exists.
- Choose the personal-config path from the store's active layout instead of
  the constant, or record the legacy-layout refusal in Migration Notes and the
  CHANGELOG.
- Decide whether `GH_TOKEN` and `GITHUB_TOKEN` should be separate rungs, and
  record the decision in the Phase 3 notes.
- Remove or correct the stale comments and the doc comments that only restate
  names.
