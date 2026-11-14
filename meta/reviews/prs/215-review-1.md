---
type: "pr-review"
id: "215-review-1"
title: "[0371] Add a Shortcut issue-tracker connector"
date: "2026-11-14T22:55:22+00:00"
author: "Tom Scott"
producer: "review-pr"
status: "complete"
target: "pr:215"
reviewer: "Tom Scott"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "security", "safety", "usability", "documentation"]
review_number: 1
pr_number: 215
tags: ["shortcut", "integrations"]
last_updated: "2026-11-14T22:55:22+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---

## Code Review: #215 - [0371] Add a Shortcut issue-tracker connector

**Verdict:** COMMENT

The connector follows the Jira and Linear adapter shape. The transport's retry
rules are sound: every write picks a resend policy, and a timed-out create or
comment is never sent twice. The weak points sit at the edges of that design.
The create skill tells the model to retry in the one case the resend policy
exists to prevent. `create` writes work-item frontmatter with its own
unlocked, line-based editor. The tests never pin the exit codes that tell a
skill whether a retry is safe. No lens found a critical defect, so the verdict
is COMMENT, but the first three themes below should be fixed before merge.

### Cross-Cutting Themes

- **The skills cannot tell a failed write from a write that may have gone
  through** (flagged by: usability, safety, documentation, test-coverage).
  `create-shortcut-story` says every failure without JSON output is safe to
  retry. Exit 141 means the create may have gone through, and that path prints
  no JSON. Write failures carry no `E_*` name, although every write skill says
  to show the `E_*` line. No test asserts exit 140, 141 or 142. Together these
  mean a timeout on create can lead the model to create a duplicate story.
- **`create` edits work-item files outside the work domain** (flagged by:
  architecture, code-quality, safety, correctness). `work_item.rs` reads and
  rewrites frontmatter line by line and writes with `std::fs::write`.
  `work-cli`'s `link_external_id` already does this with the file lock, a YAML
  parser and an atomic write. The line reader misreads inline comments and
  single quotes, and `create FILE --title X` skips the already-linked check
  and then overwrites `external_id`.
- **The search deadline restarts for each team** (flagged by: architecture,
  correctness). `search_team` calls `transport.deadline()` on every call, so a
  multi-team search can run for N times the configured budget.
- **Domain rules are written more than once, and the copies already differ**
  (flagged by: code-quality, architecture). Team parsing appears three times
  with three rules for a missing name. The read-only-token check, the key
  spelling and the accepted-kinds list each have more than one home.
- **The read skills grant every Shortcut subcommand** (flagged by: security).
  `show-shortcut-story` and `search-shortcut-stories` render untrusted story
  text and grant `Bash(accelerator shortcut *)`, which covers the write
  subcommands and `--body-file`. The Linear and Jira read skills grant their
  binaries in the same way, so the fix belongs in all three.

### Tradeoff Analysis

- **Consistency with Jira and Linear vs a narrower tool grant**: the security
  lens wants read skills to grant only `show` and `search`. The Shortcut
  skills copy the Linear grant on purpose. I recommend narrowing the Shortcut
  grants now and recording a work item for Jira and Linear.
- **Shipping now vs removing duplication**: architecture and code-quality want
  the frontmatter linker and the team parser shared. Work item 0377 already
  covers cross-tracker duplication. The linker sits inside one tracker and
  corrupts data when it fails, so I recommend fixing it in this PR. The team
  parser can wait for 0377.

### Strengths

- ✅ The transport records `may_have_applied` across every attempt, and each
  write chooses `WhileRetryable` or `OnlyWhenThrottled` at its call site.
- ✅ The API base is pinned to the https Shortcut host. Redirects are off,
  `next_page_url` must stay on the same origin and under `/api/v4/`, path
  values go in as URL segments, and the loopback seam cannot compile into a
  release build.
- ✅ The token is validated without being echoed, marked sensitive in the
  header, redacted in `Debug`, and goes through the shared consent ladder.
- ✅ `ShortcutKey` with `StoryId` and `EpicId` makes "only a story can be
  transitioned" a type rule.
- ✅ Shortcut joins sync through the existing `RemoteTracker` port and one
  registry arm. New pup rules and the provider-isolation test keep it apart
  from the work domain and the other clients.
- ✅ Refusals made before sending are tested with zero-hit assertions on the
  mock server. The scenario inventory, exit-code parity and stdout goldens pin
  the CLI contract.
- ✅ The show and search skills tell the model to treat story text as data.
  The Jira and Linear show skills lack that guard.
- ✅ The docs match the code on flags, outcome keywords, config keys and the
  140/141/142 meanings.

### General Findings

- 🟡 **Test Coverage**: The PR description overstates the sync tests. Eight of
  the nine Shortcut sync cases run in Preview mode, and none applies a
  successful push or pull.
- 🔵 **Safety**: `ShortcutCache::write` never compares workspace slugs, so
  running `init discover` with a token for another workspace repoints the
  committed catalogue for the whole team (`cache.rs:134`).
- 🔵 **Safety**: `ShortcutKey::parse` ignores case, so a leftover Jira key
  `SC-12` or Linear key `EPIC-3` parses as a Shortcut entity on the sync path
  (`shortcut_key.rs:88`).
- 🔵 **Correctness**: `fetch_all` can report ids absent while the merged
  completeness is not Complete, and it reports a repeated id twice
  (`client.rs:452`).
- 🔵 **Correctness**: `init list-teams` offers a uuid as the key of a colliding
  team, but `Catalogue::team_for_key` cannot resolve a uuid (`team_key.rs:88`).
- 🔵 **Correctness**: `search --limit N` keeps the N lowest-numbered stories,
  not the newest (`main.rs:407`).
- 🔵 **Correctness**: whole-workspace mode is inferred from `base.is_none()`,
  so an explicitly named team without a base team is skipped as Transient
  (`client.rs:301`).
- 🔵 **Correctness**: state-filter validation is case-sensitive, while
  `resolve_story_state` ignores case (`client.rs:617`).
- 🔵 **Security**: the comment, update and attach skills lack the
  body-provenance rule that the Jira write skills and `create-shortcut-story`
  carry. The comment skill also puts free Markdown inside a double-quoted
  shell argument.
- 🔵 **Usability**: write flows report a bad reference as
  `E_SHOW_BAD_REFERENCE`, and owners are shown by display name but accepted
  only by mention name.
- 🔵 **Usability**: write commands parse the reference after resolving the
  token, so a typo reports `E_NO_TOKEN` first.
- 🔵 **Documentation**: `skills/work/sync-work-items/SKILL.md` still describes
  itself as "Jira or Linear" in the frontmatter that drives auto-triggering.
  The landing page, the workflow page and `work-items.md` say the same.
- 🔵 **Documentation**: the transition and update skills tell the user to
  "pick one" for `E_STATE_AMBIGUOUS`, which cannot resolve it.
- 🔵 **Code Quality**: `client.rs` is 1,090 lines, and `ShortcutClient` holds
  the search and keyed-fetch logic as well as the raw transport.
- 🔵 **Architecture**: the string-matched integration dispatch gains
  `"shortcut"` in eight or more places. Work item 0376 covers this.

### Additional Findings

- 🟡 `skills/integrations/shortcut/show-shortcut-story/SKILL.md:16` — Read
  skill pre-approves Shortcut write subcommands (security)
- 🟡 `skills/integrations/shortcut/search-shortcut-stories/SKILL.md:15` —
  Search skill's broad Bash grant also covers write subcommands (security)
- 🟡 `cli/shortcut-cli/src/write_flows.rs:48` — Write failures have neither an
  outcome keyword nor an `E_*` name (usability)
- 🔵 `cli/shortcut-cli/tests/flow_create.rs:83` — Failed-create test asserts
  only non-success (test-coverage)
- 🔵 `cli/shortcut-cli/src/write_flows.rs:83` — `--title`/`--body-file` create
  mode and its refusals are untested (test-coverage)
- 🔵 `cli/work-adapters/tests/sync_run_real_client.rs:1716` — No Shortcut
  capped-keyed-read safety test (test-coverage)
- 🔵 `cli/shortcut-cli/src/main.rs:414` — Search cap-hit and exit 79 untested
  (test-coverage)
- 🔵 `cli/shortcut-client/src/client.rs:339` — Create-state whoami fallback
  untested (test-coverage)
- 🔵 `cli/shortcut-client/tests/writes.rs:96` — Comment and link failure tests
  do not check the error class (test-coverage)
- 🔵 `cli/shortcut-client/src/writes.rs:142` — Ambiguous-owner refusal
  untested (test-coverage)
- 🔵 `cli/shortcut-cli/src/main.rs:173` — Composition root composes raw wire
  calls (architecture)
- 🔵 `cli/shortcut-client/src/client.rs:166` — `ShortcutClient` exposes a
  generic HTTP read and write (architecture)
- 🔵 `cli/shortcut-client/src/client.rs:259` — Error explanation makes a
  hidden whoami request (architecture)
- 🔵 `cli/shortcut-client/src/workspace.rs:7` — `workspace` and `discovery`
  depend on each other (architecture)
- 🔵 `cli/shortcut-cli/src/main.rs:359` — Failed comment read in `show` is
  reported as a discovery failure (code-quality)
- 🔵 `cli/shortcut-client/src/client.rs:719` — Key spelling rebuilt with
  `format!` (code-quality)
- 🔵 `cli/shortcut-client/src/workspace.rs:24` — Read-only-token rule written
  twice (code-quality)
- 🔵 `cli/shortcut-client/src/filter.rs:16` — Kind table restated in error
  messages (code-quality)
- 🔵 `cli/shortcut-client/src/client.rs:597` — `validate_filters` nests too
  deep (code-quality)
- 🔵 `cli/shortcut-client/src/client.rs:443` — Keyed-read failures discarded
  without a log (code-quality)
- 🔵 `cli/shortcut-client/src/story_state.rs:69` — Workflow and state ids are
  bare `i64` (code-quality)
- 🔵 `cli/shortcut-cli/src/main.rs:1` — Stale module doc; read flows left in
  `main` (code-quality)
- 🔵 `cli/shortcut-client/src/update.rs:28` — `UpdateFields::is_empty` used
  only by a test (code-quality)
- 🔵 `cli/shortcut-cli/src/work_item.rs:93` — `external_id` writeback is not
  atomic (safety)
- 🔵 `skills/integrations/shortcut/comment-shortcut-story/SKILL.md:11` —
  Comment skill offers `--body-file` but cannot read the file (usability)
- 🔵 `cli/shortcut-cli/src/cli.rs:119` — `update --description` has no file
  form (usability)
- 🔵 `cli/shortcut-cli/src/cli.rs:173` — `attach --file` is advertised but
  always fails (usability)
- 🔵 `cli/shortcut-cli/src/main.rs:113` — `init list-teams` ignores `--quiet`
  (usability)
- 🔵 `cli/shortcut-client/src/writes.rs:71` — `E_NOT_A_STORY` wording says
  epics support update while refusing one (usability)
- 🔵 `skills/integrations/shortcut/search-shortcut-stories/SKILL.md:62` —
  `cap-hit` branch does not say to render the partial results (usability)
- 🔵 `skills/integrations/shortcut/init-shortcut/SKILL.md:56` — Says
  `/init-shortcut` instead of `/accelerator:init-shortcut` (usability)
- 🔵 `docs-site/src/content/docs/shortcut.md:41` — Docs promise an `E_*` line
  that write failures never print (documentation)
- 🔵 `skills/integrations/shortcut/search-shortcut-stories/SKILL.md:70` —
  Missing-team-key message described inaccurately (documentation)
- 🔵 `skills/integrations/shortcut/init-shortcut/SKILL.md:78` — Team-key
  definition omits the display-name and uuid fallbacks (documentation)
- 🔵 `cli/shortcut-client/tests/evidence_hygiene.rs:16` — Secret scan covers
  only `fixtures/v4` and the `sct_` prefix (security)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `skills/integrations/shortcut/create-shortcut-story/SKILL.md:85-86` — Create skill calls a may-have-applied failure safe to retry
**Severity**: major | **Confidence**: high | **Lens**: usability, safety, documentation

🟡 **Usability / Safety / Documentation**

This paragraph says that when stdout carries no JSON, nothing was created and a retry is safe. But `create` maps `TrackerError::Terminal` to exit 141 (`WRITE_MAY_HAVE_APPLIED`). That happens after a timeout or a 5xx once the POST was sent, and that path prints no JSON either. `shortcut.md` documents 141 as "may have applied".

**Impact**: after a timed-out create, the model tells the user a retry is safe. The retry creates a duplicate story or epic, which is the case `Resend::OnlyWhenThrottled` exists to prevent.

**Suggestion**: split the no-JSON case by failure class. On 141, tell the user the story may exist. Have them check with `/accelerator:search-shortcut-stories --text "<title>"`, and set `external_id` by hand instead of retrying. Only a 140 or a pre-send `E_*` refusal is safe to retry. `comment-shortcut-story` already has a check-before-retry step to copy.

---

### `cli/shortcut-cli/src/write_flows.rs:84-102` — `create FILE --title X` skips the already-linked check, then overwrites `external_id`
**Severity**: major | **Confidence**: high | **Lens**: safety

🟡 **Safety**

`CreateArgs` lets a caller pass a positional `file` and `--title` together, since clap declares no conflict. When `--title` is set, `create_inputs` returns early and never reads the file, so the `already_synced` refusal never runs. After the create succeeds, `create` still sees `args.file` and calls `work_item::link`, which replaces the existing `external_id`.

**Impact**: `accelerator shortcut create meta/work/0042.md --title X` on a linked work item creates a second story. It then relinks the local item away from its original story, and the sync baseline for the original story is orphaned without a warning.

**Suggestion**: mark `file` as conflicting with `title` and `body_file` in clap. Alternatively, run the `already_synced` check whenever a file will be linked. Add a flow test that pins the refusal and zero POST hits.

---

### `cli/shortcut-cli/src/work_item.rs:71-96` — Work-item linking repeats the work domain's linker without its lock, parser or atomic write
**Severity**: major | **Confidence**: high | **Lens**: architecture, code-quality, safety, correctness

🟡 **Architecture / Code Quality**

`link` rewrites `external_id` line by line and writes the file with `std::fs::write`. `work-cli/src/sync_author.rs::link_external_id` already does this job. It takes the file's lock, rewrites the YAML mapping through the `document` crate and writes atomically. Neither the Jira CLI nor the Linear CLI writes work-item frontmatter, so this is the first provider CLI to do so, and it skips the domain's guarantees.

🔵 **Safety**: `std::fs::write` truncates the file before it writes. An interruption just after a live create leaves the work item empty and unlinked.

🔵 **Correctness**: the reader above (`field` and `unquote`, lines 29–42) misreads valid frontmatter. With the template's own inline comments, `external_id: ""   # remote tracker identifier` reads as non-empty, so create refuses with `E_CREATE_ALREADY_SYNCED`. `kind: "story"  # story | epic` reads as an unknown kind, and `external_id: ''` counts as linked.

**Impact**: a `create` that runs during `/sync-work-items` can lose an update or tear the file. Template-style work items are refused or get the wrong kind.

**Suggestion**: move the YAML-aware read and link out of `work-cli` into a crate both composition roots can use, and call it here. If pup blocks that today, record it beside 0377 instead of keeping a second copy.

---

### `cli/shortcut-cli/src/exit_codes.rs:188-198` — No test asserts the retry-contract exit codes 140, 141 and 142
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

`for_tracker_write` maps Retryable to 140, Terminal to 141 and Rejected to 142. These codes tell a skill whether a retry could duplicate a write. No unit test covers the mapping, and no flow test asserts `Some(140)`, `Some(141)` or `Some(142)`. The failed-create and read-only-update flow tests assert only `!status.success()`, and `exit_codes_parity` checks only that the constants exist.

**Impact**: if someone swapped Terminal and Retryable here, a timed-out create would report "nothing applied" and every test would still pass.

**Suggestion**: add a table-driven test beside `each_status_class_has_its_own_code` for `for_tracker_write` and `for_write`. Tighten `a_failed_create_leaves_the_work_item_unchanged` to `Some(141)` and the read-only update test to its exact code.

---

### `cli/shortcut-cli/src/write_flows.rs:140-153` — No test reaches the `writeback-failed` path
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

When Shortcut creates the entity but `work_item::link` fails, `create` prints `writeback-failed` with the key, exits 107, and tells the user not to create again. `WritebackFailed` appears only in `keywords.rs` and the keyword pin, and no test asserts exit 107.

**Impact**: this branch is the only guard against a duplicate when the remote create succeeded and the local link failed. A regression that exits 0 with `created`, or exits before printing the key, would lead the skill to create the entity again.

**Suggestion**: add a flow test that makes the work-item file read-only after `create_inputs` reads it. Assert exit 107, `outcome == "writeback-failed"`, the key on stdout, and one POST hit.

---

### `cli/work-adapters/tests/sync_run_real_client.rs:1910-1942` — Shortcut sync tests cover planning only, not pull or push
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

Every new Shortcut sync test except the 403 case runs `RunMode::Preview` and asserts only the planned state. `every_pull_entry_point_writes_the_description_without_a_title_line` still iterates only Jira and Linear. Shortcut projects its body as `name\ndescription`, so that test guards exactly the Shortcut projection. No test checks the PATCH body a successful push sends to `/stories/N` or `/epics/N`.

**Impact**: a regression in stripping the title line on pull, or in the push body, would write wrong content into `meta/work/` or into Shortcut, and no test would fail. The PR description says these tests cover push and pull, which overstates them.

**Suggestion**: add a `Tracker::Shortcut` arm to the every-pull-entry-point test. Add one Apply-mode push test that asserts the sent PATCH body for a story and for an epic.

---

### `cli/shortcut-client/src/client.rs:545` — Each team in a search gets its own deadline
**Severity**: major | **Confidence**: high | **Lens**: architecture, correctness

🟡 **Architecture / Correctness**

`search_team` calls `self.transport.deadline()` on every call, and `search_stories` calls it once per team. `Deadline` is documented to bound a whole operation, and `fetch_all` does share one deadline between stories and epics. The team read, `show_issue`, `owner_uuid`, `scopes` and `create_state` also start their own deadlines.

**Impact**: a sync over `additional_teams` or `all_teams` can run for N times the configured budget. A slow or throttling API stalls the run past its limit, and a cut-off run reports `Transient` late.

**Suggestion**: create the deadline once in `search_stories` and pass `&Deadline` into `search_team` and the lookup helpers. Call `transport.deadline()` only at the outermost entry points, as Linear's client does.

---

### `cli/shortcut-client/src/client.rs:643-648` — `ReadFailure::Unsent` covers four unrelated failures and maps them all to CONNECT
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`ReadFailure::Unsent` stands for a real send failure, a 2xx that is not JSON, a projection parse error, and a story without a workflow (`writes.rs:104`). `exit_codes::for_read` maps every `Unsent` to `CONNECT` (21).

**Impact**: when Shortcut answers but leaves out a field, the user and the skills see a connect failure, and the user starts checking proxies and certificates.

**Suggestion**: split the variant by cause, for example `Unsent`, `Status` and `UnusableBody`, and map `UnusableBody` to `BAD_RESPONSE`. Fold the delivery-to-`ReadFailure` match that `read` and `show_issue` both repeat into one helper.

---

### `cli/shortcut-client/src/client.rs:339-358` — whoami failures are dropped, then reported as a different cause
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`create_state` calls `whoami(...).ok()` and drops the error. When whoami fails with a 401, a 5xx or a timeout, create reports that the team "has no unstarted state in its default workflow; run init discover". `scopes()` also drops a whoami failure with `unwrap_or_default()` and caches the empty result, so every later 403 in the process gets the generic explanation instead of the read-only-token one.

**Impact**: the message sends the user to rerun discovery when the real cause is the network or the token, and nothing records that cause.

**Suggestion**: propagate the whoami error, for example as `TrackerError::Retryable` with its detail, or at least log it and name it in the Unconfigured message. Do not cache a failed lookup in `scopes()`.

---

### `cli/shortcut-client/src/client.rs:902-919` — Team parsing is written three times, and the copies differ
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`team_key.rs` calls itself the one home of the team-naming rule. But three places turn a team entity into a team. `enumerate_visible_entities` here gives a missing `name` the value `""`. `list_teams` in `shortcut-cli/src/main.rs:173` drops a team with no name. `discovery::team` rejects it as Unusable.

**Impact**: the same workspace can show different teams and keys in `init list-teams`, in sync discovery and in `init discover`.

**Suggestion**: add one `TeamIdentity::from_entity(&Value)` in `team_key.rs` with a single strictness rule, and call it from all three places.

## Per-Lens Results

Each lens's comments are shown with their location, severity, confidence and
the main point. The inline comments above carry the full bodies of the
comments posted to GitHub.

### Architecture

**Summary**: The connector follows the provider-adapter shape: `shortcut-client` implements `RemoteTracker`, `shortcut-cli` is the composition root, and one registry arm wires it into sync, with pup and provider-isolation guards. Two structural problems stand out: `create` links work items through its own unlocked, line-based writer, and the client restarts the deadline for each sub-call. The duplication and string dispatch are acknowledged by 0376 and 0377.

**Strengths**: the provider boundary is enforced by new pup rules and the provider-isolation test; Shortcut joins sync only through the port and one registry arm; each write chooses its idempotency explicitly; catalogue writes use the shared atomic write and lock; remote hashing stays in `remote-projection`; the contract suite gains a `create_kind` hook; the tradeoffs are recorded as work items.

**Comments**:
- major, high — `cli/shortcut-cli/src/work_item.rs:71-96` — work-item linking repeats the work domain's linker and skips its lock and atomic write.
- major, medium — `cli/shortcut-client/src/client.rs:545` — the deadline restarts for each team and each read.
- minor, high — `cli/shortcut-cli/src/main.rs:173-187` — the composition root composes raw wire calls and repeats the team mapping.
- minor, medium — `cli/shortcut-client/src/client.rs:166-229` — `ShortcutClient` exposes a generic HTTP read and write plus its transport and paths.
- minor, medium — `cli/shortcut-client/src/client.rs:259-279` — error explanation makes a hidden whoami request with its own deadline.
- suggestion, high — `cli/shortcut-client/src/workspace.rs:7` — `workspace` and `discovery` depend on each other.

**General Findings**:
- suggestion — the string-matched integration dispatch gains `"shortcut"` in eight or more places; do 0376 before a fourth tracker.

### Correctness

**Summary**: The write paths are sound: retry classification covers every attempt, resend policies match idempotency, and pagination guards against loops and foreign origins. The problems sit at the edges: the keyed read does not fully keep the `FetchOutcome` contract, the search deadline is per team, whole-workspace mode is inferred from scope shape, team keys and catalogue lookup disagree for colliding teams, and the frontmatter reader misreads some valid YAML.

**Strengths**: `may_have_applied` is gathered across the retry loop; each write's resend policy matches its idempotency; `paging::collect` refuses a repeated page and `admits_next_page` keeps the token on Shortcut's origin; an unrequested story makes a chunk Transient; the whoami fallback state is checked against the team's workflow.

**Comments**:
- minor, high — `cli/shortcut-client/src/client.rs:545` — each team gets its own deadline.
- minor, medium — `cli/shortcut-client/src/client.rs:301` — whole-workspace mode is inferred from `base.is_none()`.
- minor, medium — `cli/shortcut-client/src/client.rs:452` — `fetch_all` can report ids absent when completeness is not Complete, and reports duplicates twice.
- minor, medium — `cli/shortcut-client/src/team_key.rs:88-94` — a colliding team's uuid is offered as its key but cannot be looked up.
- minor, medium — `cli/shortcut-cli/src/work_item.rs:29-42` — the frontmatter reader misreads inline comments and single quotes.
- minor, medium — `cli/shortcut-cli/src/main.rs:407-412` — `search --limit N` returns the N oldest stories.
- suggestion, low — `cli/shortcut-client/src/client.rs:617-626` — state-filter validation is case-sensitive while state resolution is not.

**General Findings**: none.

### Code Quality

**Summary**: The design uses rich domain types, small write modules and injected seams. The main risks are in error handling and duplicated domain knowledge: `ReadFailure::Unsent` covers unrelated failures, two whoami calls drop their errors, and team parsing, key spelling and frontmatter linking each have several copies that already differ.

**Strengths**: `ShortcutKey` with `StoryId`/`EpicId` and `into_story()`; the named `Resend` policy at each call site; a small, pure `classify.rs`; short write modules sharing `UpdateFields`; injected `Sleeper`, `Jitter` and `Filesystem`; `const fn` exit-code matches without catch-alls.

**Comments**:
- major, high — `cli/shortcut-client/src/client.rs:643-648` — `ReadFailure::Unsent` covers four unrelated failures.
- major, high — `cli/shortcut-client/src/client.rs:339-358` — whoami failures are dropped and reported as a different cause.
- major, medium — `cli/shortcut-cli/src/work_item.rs:71-96` — the hand-rolled frontmatter writer repeats `work-cli`'s locked linker.
- major, high — `cli/shortcut-client/src/client.rs:902-919` — team parsing is copied three times, and the copies differ.
- minor, high — `cli/shortcut-cli/src/main.rs:359-368` — a failed comment read in `show` is reported as a discovery failure.
- minor, high — `cli/shortcut-client/src/client.rs:719-722` — key spelling is rebuilt with `format!`.
- minor, high — `cli/shortcut-client/src/workspace.rs:24-29` — the read-only-token rule is written twice.
- minor, medium — `cli/shortcut-client/src/filter.rs:16-17` — the kind table is restated in error messages.
- minor, high — `cli/shortcut-cli/src/main.rs:307-315` — every reference refusal is labelled `E_SHOW_BAD_REFERENCE`.
- suggestion, medium — `cli/shortcut-cli/src/write_flows.rs:77-81` — a local `NewEntity` struct shadows the client's `NewEntity` enum.
- minor, medium — `cli/shortcut-client/src/client.rs:597-615` — `validate_filters` nests too deep.
- minor, medium — `cli/shortcut-client/src/client.rs:443` — keyed-read failures are discarded without a log.
- suggestion, medium — `cli/shortcut-client/src/story_state.rs:69-73` — workflow and state ids are bare `i64`.
- suggestion, high — `cli/shortcut-cli/src/main.rs:1-2` — stale module doc; read flows left in `main`.
- suggestion, high — `cli/shortcut-client/src/update.rs:28-31` — `UpdateFields::is_empty` is used only by a test.

**General Findings**:
- minor — `ShortcutClient` holds too many responsibilities; extract `StorySearch` and `KeyedFetch`.
- suggestion — the `shortcut.team_key` read lives in `auth` and the key literal is declared twice.

### Test Coverage

**Summary**: The suite is large and well organised: recorded v4 fixtures, per-method resend tests, subprocess flow tests and pins for exit codes, keywords, surface, inventory and goldens. The gaps sit at the riskiest edges: the 140/141/142 mapping is never asserted, the writeback-failed path is untested, and Shortcut sync has preview-only tests and is left out of the pull-body and capped-keyed-read safety tests.

**Strengths**: the resend policy is pinned per method and outcome; pre-send refusals use zero-hit assertions; `scenario_inventory` prevents orphaned fixtures; `exit_codes_parity` checks constants, the reserved band and uniqueness; a sentinel token checks no output leaks it; `flow_seam` runs in the default build; `fetch_all` partition tests cover the completeness rules.

**Comments**:
- major, high — `cli/shortcut-cli/src/exit_codes.rs:188-198` — 140/141/142 are never asserted.
- minor, high — `cli/shortcut-cli/tests/flow_create.rs:83` — the failed-create test asserts only non-success.
- major, high — `cli/shortcut-cli/src/write_flows.rs:140-153` — the writeback-failed path is untested.
- minor, high — `cli/shortcut-cli/src/write_flows.rs:83-122` — the `--title`/`--body-file` mode and its refusals are untested.
- minor, medium — `cli/shortcut-cli/src/work_item.rs:71-92` — the reader and linker have no unit tests; the append branch never runs.
- major, high — `cli/work-adapters/tests/sync_run_real_client.rs:1910-1942` — Shortcut sync is tested only in Preview.
- minor, medium — `cli/work-adapters/tests/sync_run_real_client.rs:1716-1724` — no Shortcut capped-keyed-read test.
- minor, high — `cli/shortcut-cli/src/main.rs:414-445` — search cap-hit, exit 79 and the transient warning are untested.
- minor, high — `cli/shortcut-client/src/client.rs:339-358` — the create-state whoami fallback is untested.
- minor, medium — `cli/shortcut-client/tests/writes.rs:96` — comment and link failure tests do not check the error class.
- suggestion, high — `cli/shortcut-client/src/writes.rs:142-144` — the ambiguous-owner refusal is untested.

**General Findings**:
- minor — the PR description's testing claims overstate the sync coverage.
- suggestion — team-key writeback has five outcomes, and the flow tests cover three.

### Security

**Summary**: The core controls are strong and match Jira and Linear: a pinned https host, loopback compiled out of release, no redirects, same-origin paging, segment-built paths, quote-checked search values, a sensitive auth header and the shared consent ladder. The gap is in the agent layer: model-invocable read skills render untrusted text while granting every subcommand, and three write skills lack the provenance guard.

**Strengths**: `api_base` accepts only the Shortcut https origin; redirects are off and paging stays on origin; values are added as path segments and slugs are restricted; `validate_token` never echoes the token; `token_cmd` is a consent key; search values with quotes are refused; `attach_link` requires https; show and search tell the model to treat story text as data.

**Comments**:
- major, medium — `skills/integrations/shortcut/show-shortcut-story/SKILL.md:16` — the read skill pre-approves write subcommands while rendering untrusted content.
- major, medium — `skills/integrations/shortcut/search-shortcut-stories/SKILL.md:15` — the search skill's grant covers write subcommands.
- minor, high — `skills/integrations/shortcut/comment-shortcut-story/SKILL.md:28` — no body-provenance guard.
- minor, high — `skills/integrations/shortcut/update-shortcut-story/SKILL.md:32` — no provenance guard for `--title`/`--description`.
- minor, medium — `skills/integrations/shortcut/comment-shortcut-story/SKILL.md:52` — comment text goes inline into a shell command.
- suggestion, low — `skills/integrations/shortcut/attach-shortcut-story/SKILL.md:30` — no provenance rule for the attach URL.
- suggestion, low — `cli/shortcut-client/tests/evidence_hygiene.rs:16` — the secret scan depends on the `sct_` prefix and covers only `fixtures/v4`.

**General Findings**:
- minor — write subcommands read any local path; with the broad read-skill grants this gives prompt injection an exfiltration path.

### Safety

**Summary**: The write path is built with care: resend policy per write, `may_have_applied` across every attempt, and Terminal on a timed-out non-idempotent write. Two gaps undercut it: the create skill calls any no-JSON failure safe to retry, and `create FILE --title` skips the already-linked check. Smaller risks: discovery from another workspace repoints the catalogue, keys parse case-insensitively, and the work-item writeback is not atomic.

**Strengths**: `may_have_applied` across every attempt, with an unparseable 2xx counted as applied; non-idempotent writes resend only on 429; only a first-attempt connect failure counts as unsent; create refuses a linked work item and has a `writeback-failed` outcome; the team-key writeback needs `--force`; incomplete discovery never writes the catalogue; every write skill previews and confirms; write intent refuses an ignored personal config.

**Comments**:
- major, high — `skills/integrations/shortcut/create-shortcut-story/SKILL.md:85-86` — every no-JSON failure is called safe to retry.
- major, high — `cli/shortcut-cli/src/write_flows.rs:84-102` — `create FILE --title` skips the already-linked check and overwrites `external_id`.
- minor, medium — `cli/shortcut-client/src/cache.rs:134-143` — discovery from another workspace repoints the committed catalogue.
- minor, medium — `cli/shortcut-client/src/shortcut_key.rs:88-104` — case-insensitive parsing accepts `SC-12` and `EPIC-3`.
- minor, medium — `cli/shortcut-cli/src/work_item.rs:93-95` — the `external_id` writeback is not atomic.

**General Findings**:
- minor — may-have-applied failures carry no stable `E_*` tag for skills to branch on.

### Usability

**Summary**: The CLI surface mirrors Linear's, so it is easy to learn, and most pre-send refusals say how to fix them. The main problems are at the failure edges: write failures (140/141/142) carry no keyword and no `E_*` name, and the create skill calls a may-have-applied failure safe to retry. Smaller friction: errors named after the wrong subcommand, owner names that differ between output and input, and a comment skill that cannot preview a body file.

**Strengths**: flags and subcommands match `accelerator linear`; pre-send refusals can be acted on; `init verify` detects a read-only token; references accept keys or URLs and refuse other workspaces; `writeback-failed` is clear; help text uses Shortcut's own concepts.

**Comments**:
- major, high — `skills/integrations/shortcut/create-shortcut-story/SKILL.md:85-86` — the create skill calls a may-have-applied failure safe to retry.
- major, medium — `cli/shortcut-cli/src/write_flows.rs:48-51` — write-failure classes have neither an outcome keyword nor an `E_*` name.
- minor, high — `skills/integrations/shortcut/comment-shortcut-story/SKILL.md:11-13` — `--body-file` is offered but the skill cannot read the file.
- minor, high — `cli/shortcut-cli/src/write_flows.rs:34-36` — write commands report a bad reference as `E_SHOW_BAD_REFERENCE`.
- minor, medium — `cli/shortcut-cli/src/main.rs:469-470` — output shows owner display names, but `--owner` accepts only mention names.
- minor, medium — `cli/shortcut-cli/src/cli.rs:119-122` — `update --description` has no file form, and several flags have no help text.
- suggestion, medium — `cli/shortcut-cli/src/cli.rs:173-175` — `attach --file` is advertised but always fails.
- suggestion, high — `cli/shortcut-cli/src/main.rs:113` — `init list-teams` ignores `--quiet`.
- suggestion, medium — `cli/shortcut-client/src/writes.rs:71-76` — the `E_NOT_A_STORY` wording conflicts with update's refusal.
- suggestion, medium — `skills/integrations/shortcut/search-shortcut-stories/SKILL.md:62-65` — the `cap-hit` branch does not say to render the partial results.
- suggestion, low — `skills/integrations/shortcut/init-shortcut/SKILL.md:56-57` — says `/init-shortcut` instead of `/accelerator:init-shortcut`.

**General Findings**:
- minor — write commands parse the reference after resolving the token.
- suggestion — owner refusals list every member, or none, in large workspaces.

### Documentation

**Summary**: The docs are broad and mostly accurate; flags, keywords, kind mapping, config keys, env overrides, consent keys and 140/141/142 all match the code. The problems are in failure guidance: the create skill calls exit 141 safe to retry, the docs promise an `E_*` line that write failures never print, and some remediation hints contradict the binary. Several pages outside the diff still say "Jira or Linear".

**Strengths**: every documented flag matches `cli_surface.golden`; every outcome keyword exists in `keywords.rs`; the 140/141/142 meanings match `exit_codes.rs`; `configure/SKILL.md` matches `catalogue.rs`; the deliberate limits are stated and enforced; the docs cover every place a user would look.

**Comments**:
- major, high — `skills/integrations/shortcut/create-shortcut-story/SKILL.md:85-86` — the create skill calls every failure safe to retry.
- minor, high — `docs-site/src/content/docs/shortcut.md:41-43` — write failures print no `E_*` line, despite the docs.
- minor, high — `skills/integrations/shortcut/transition-shortcut-story/SKILL.md:64-65` — "pick one" cannot resolve `E_STATE_AMBIGUOUS`.
- minor, high — `skills/integrations/shortcut/search-shortcut-stories/SKILL.md:70-71` — the missing-team-key message is described wrongly.
- suggestion, medium — `skills/integrations/shortcut/init-shortcut/SKILL.md:78-80` — the team-key definition omits its fallbacks.
- suggestion, medium — `skills/integrations/shortcut/comment-shortcut-story/SKILL.md:52` — the command template shows only `--body`.

**General Findings**:
- minor — several pages outside the diff, including the `sync-work-items` skill description, still say "Jira or Linear".
- suggestion — `update-shortcut-story` does not list `E_OWNER_AMBIGUOUS`.

---
*Review generated by /accelerator:review-pr*
