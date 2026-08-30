---
type: "codebase-research"
id: "2026-08-08-0197-accelerator-collaboration-pr-helper-cli"
title: "Research: Implementation surface for work-item 0197 (accelerator-collaboration PR Helper CLI)"
date: "2026-08-08T15:50:29+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
parent: "work-item:0197"
topic: "Implementation surface for the accelerator-collaboration PR Helper CLI"
tags: ["research", "codebase", "cli", "collaboration", "vcs", "config", "github", "octocrab", "rust"]
revision: "2b3b86f028048fc96ca382113794db546a0ae8a6"
repository: "accelerator"
last_updated: "2026-08-08T15:50:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Implementation surface for work-item 0197 (accelerator-collaboration PR Helper CLI)

**Date**: 2026-08-08T15:50:29+00:00
**Author**: Toby Clemson
**Git Commit**: 2b3b86f028048fc96ca382113794db546a0ae8a6
**Branch**: (jj workspace, no active bookmark on this change)
**Repository**: accelerator

## Research Question

What does the codebase already provide, and what is genuinely missing, to implement
work-item 0197 (`accelerator-collaboration`: migrate `pr-base-repo.sh` and
`pr-update-body.sh` into a Rust sub-binary calling the GitHub REST API in-process via
`octocrab`)? This research is intended to educate an implementation plan, covering: the
sub-binary structural precedent to mirror, the config-catalogue pattern for
`github.token`/`github.token_cmd`, the `vcs`/`vcs-adapters` gap for origin-remote-URL
parsing, the full sub-binary registration checklist, the exact behaviour of the two bash
scripts being replaced (for characterization testing), and the HTTP-client/mocking
pattern to use for `octocrab`.

## Summary

The codebase has a clean, proven three-crate structural pattern (`vcs` / `vcs-adapters`
/ `vcs-cli`, `work` / `work-adapters` / `work-cli`) that `collaboration` /
`collaboration-adapters` / `collaboration-cli` should mirror exactly, and a fully
data-driven sub-binary dispatch mechanism that requires zero launcher code changes —
only registry/manifest entries. However, three assumptions embedded in the work item's
own Technical Notes turned out to be **wrong or imprecise** on direct inspection of the
code, and should reshape the plan:

1. **The `jira`/`linear` `token`/`token_cmd` precedence-and-shared-config-ban logic that
   0197 says already exists in `cli/config/src/catalogue.rs` / `cli/config-adapters/src/store.rs`
   does not exist in Rust at all.** `EXTRA_KEYS` in `catalogue.rs` only declares the key
   *names* — no default, no validation, no precedence, no ban. The actual precedence
   chain (env → env `_cmd` → local config token → local config `token_cmd` → shared
   config token, with shared-config `token_cmd` rejected-and-warned) is implemented
   entirely in bash, independently, in `jira-auth.sh` and `linear-auth.sh`. This item
   must write this resolver logic fresh in Rust — there is nothing to "reuse", only a
   *behavioural contract* to mirror (see Detailed Findings § 2).

2. **`vcs`'s `RepoFacts` has no remote-derived data of any kind, and neither
   `vcs-adapters` backend reads a git remote today** — confirmed by direct read, not
   just by the work item's own claim. This is a green-field addition: a new narrow
   port (e.g. `OriginRemote`) in the `vcs` domain crate, satisfied by both the
   `subprocess` adapter (shell `git remote get-url origin`, reusing the existing
   `run_capped`/`scrub_environment` machinery) and the `library` adapter (`gix`
   already reads git config via `config_snapshot()` for `user.name` — the same
   mechanism reads `remote.origin.url`). See Detailed Findings § 3.

3. **The bash-script branch enumeration in the work item's Technical Notes has two
   small inaccuracies against the actual source**, worth resolving before writing
   characterization tests: (a) `pr-base-repo.sh`'s "REST request failure" and "missing
   origin remote" are enumerated as two separate branches, but in the current bash
   they are *one* branch — any `gh pr view` failure hits the same `exit 1`, with the
   "no default remote" case only distinguished by an optional extra stderr line
   gated on a `grep` match against gh's own error text (this split is architecturally
   correct for the *target* Rust design, since local owner/repo resolution moves to a
   separate pre-network-call `vcs` step, but the bash source itself only has one
   branch here); (b) the work item states `test-pr-base-repo-scripts.sh` has 15 test
   cases — it actually has 19 (16 numbered branch/behaviour tests + 3 regression
   guards). See Detailed Findings § 5.

Everything else lines up cleanly: the sub-binary registration checklist (13 items,
1/2/3/4/7/8 must land together) is well-documented and has a fresh worked example
(0195/corpus) to mirror; `reqwest` is already a pinned workspace dependency (used by
the launcher's `Fetcher`) but `octocrab` is not yet present anywhere, and will need
`default-features = false` + an explicit rustls feature selection to avoid violating
`cli/deny.toml`'s `native-tls`/`openssl` ban; and the established HTTP-mocking pattern
in this codebase is a hand-rolled `std`-only loopback `TcpListener` mock server
(`cli/launcher/tests/common/mod.rs`), not an external crate like `wiremock`/`mockito`.

## Detailed Findings

### 1. Sub-binary structural precedent (`vcs`/`vcs-adapters`/`vcs-cli`, `work`/`work-adapters`/`work-cli`)

**Crate layout.** Both are three-crate stacks registered in `cli/Cargo.toml`'s
`[workspace].members`:

- **Domain crate** — bare package name (`vcs`, `work`), pure logic, only depends on
  `kernel` (plus, for `work`, `corpus`). Defines port traits as `pub trait X { fn
  method(&self, ...) -> ...; }` and free composition functions that take `&dyn Trait`
  arguments (`cli/vcs/src/lib.rs:51-120`). Unit tests use hand-written fixed/fake
  trait implementations inline in the same module — no mocking framework anywhere in
  the workspace.
- **Adapters crate** (`vcs-adapters`, `work-adapters`) — depends on the domain crate
  plus real I/O dependencies (`gix`, `jj-lib`, `tracing`, etc. for `vcs-adapters`).
  `vcs-adapters` ships two alternative backends behind separate modules —
  `subprocess` (spawns real `git`/`jj`, scrubbed environment) and `library`
  (in-process via `gix`/`jj-lib`) — both satisfying the same port traits
  (`cli/vcs-adapters/src/lib.rs:1-26`). `work-adapters` reuses `vcs`/`vcs-adapters`
  directly as a dependency (cross-domain adapter reuse is an accepted pattern, not a
  layering violation).
- **Thin `-cli` crate** (`vcs-cli` → package `accelerator-vcs`, `work-cli` → package
  `accelerator-work`) — the **crate directory name and the Cargo package/binary name
  differ**: directory `vcs-cli/`, package `accelerator-vcs`, `[[bin]] name =
  "accelerator-vcs"`. `cli.rs` is the clap inbound adapter (`#[derive(Parser)]`
  `Cli`/`#[derive(Subcommand)]` `Command`, `disable_version_flag = true` since the
  top-level launcher owns `version`). `main.rs` has one `run_<subcommand>()` function
  per subcommand that composes adapters, calls the domain crate's pure function, and
  translates the outcome to stdout/stderr/exit code, dispatched from a single `match`
  in `fn main() -> ExitCode`.

**Error/exit-code convention.** `kernel::Error` is the shared error type across both
`-cli` crates. `vcs-cli` centralises the mapping (`Refusal → ExitCode::from(2)`,
everything else → `ExitCode::FAILURE`); `work-cli` maps per-subcommand outcome enums to
specific exit codes with dedicated `E_*` stderr prefixes. Successful, machine-consumable
output goes to stdout; diagnostics/errors go to stderr.

**Config threading (the model to follow, since `vcs-cli` uses no config at all).**
`work-cli` depends on `config`/`config-adapters` and every config-using `run_*`
function follows the same three-step composition inline in `main.rs`: `compose(&start,
LegacyPolicy::Reject)?` → borrow `&composed.service` as `&dyn ConfigAccess` → pass into
domain functions, mirroring the `vcs` ports pattern. Shared config-key helpers (parsing
a dotted key, resolving overrides) live in a `-cli`-local module,
`cli/work-cli/src/config.rs` — not the domain crate, not the adapters crate. A
`collaboration-cli/src/config.rs` reading `github.*` keys would follow this exact
shape.

**Central dispatch mechanism.** This is fully data-driven, not a static per-token match
arm in the launcher — a new domain requires **zero launcher Rust code changes**:

- `cli/launcher/src/launch/mod.rs`'s `dispatch()` only special-cases `Version`/`Config`;
  everything else is captured generically as `Command::External(raw)` and its first
  token is looked up.
- The signed release `manifest.json` (schema at
  `cli/launcher/src/launch/outbound/resolve/manifest.rs:26-32`) is keyed generically by
  binary token — `BTreeMap<String, BinaryEntry>` — so the launcher dispatches whatever
  token the manifest names, no allowlist.
- The manifest is produced at release time from `DISPATCHED_SUBBINARIES` in
  `tasks/shared/paths.py:29-34` (currently `("visualiser", "vcs", "work", "corpus")`)
  and `_SUBBINARY_MANIFESTS` in `tasks/manifest.py:53-60` (only needed when the crate
  path deviates from the `cli/<token>/Cargo.toml` default — which it will for
  `collaboration`, since the binary crate lives at `cli/collaboration-cli/`, mirroring
  `vcs`).
- The **actual "central dispatch manifest"** in the human-process sense the work item
  refers to is the thirteen-point registration checklist in `tasks/README.md` — see § 4
  below.

**Ports-and-adapters pattern for a mockable GitHub port.** The concrete template:
define a narrow trait in the `collaboration` domain crate (returning `Option<T>` for
"infallible" queries, `kernel::Error` where genuinely fallible, exactly as `vcs`'s
`RepoRoot`/`VcsProbe`/`UserIdentityProbe` do); implement it in `collaboration-adapters`
as a real HTTP-calling struct (the equivalent of `subprocess.rs`, the *only* module
allowed to hold `reqwest`/`octocrab` types); fake it by hand in the domain crate's own
unit tests (`FixedRoot`/`FixedProbe`-style structs, no mocking library). The existing
reqwest-based **mock-seam precedent** to imitate for the adapter's own tests is
`cli/launcher/src/launch/outbound/resolve/fetcher.rs`'s `Fetcher` — `Fetcher::new()` for
production (https-only), `Fetcher::with_backoff(Duration)` for tests (permits `http`,
points at a local mock server). Adjacent precedent for parsing a raw JSON REST payload
into domain data: `cli/work-adapters/src/project_remote.rs` — pure `serde_json::Value`
projection keyed by an `Integration` enum, not yet wired to a live subcommand.

**A pre-existing signal that `collaboration` is expected.** `WORK_INTEGRATION_VALUES` in
`cli/config/src/catalogue.rs:107` already lists `"github-issues"` as an accepted (if
currently unimplemented) `work.integration` value — independent confirmation that a
GitHub-backed integration is anticipated by the config system.

### 2. Config catalogue — `github.token`/`github.token_cmd` (and a discrepancy with the work item)

**What exists today.** `EXTRA_KEYS` in `cli/config/src/catalogue.rs:121-131` is a bare
list of key-name strings with **no attached default, validation, precedence, or ban
logic** — the doc comment above it (`catalogue.rs:116-120`) states these are "read
ad-hoc by their own consumers." `linear.token`/`linear.token_cmd` is the exact
"bare pair, no extra fields" shape the work item cites as the template for
`github.*` (`jira.*` additionally carries `site`/`email`). The bash mirror,
`EXTRA_KEYS` in `scripts/config-defaults.sh:208-218`, is a hand-synced duplicate list —
**not** covered by the Rust/bash drift test (`catalogue.rs:410-476`'s `EXTRACT`
heredoc iterates the other five key groups, but explicitly skips `EXTRA_KEYS`), so a
`github.*` addition must be added to both lists by hand with no automated check that
they match.

`dump.rs` (`cli/launcher/src/config_command/core/dump.rs:73-75,156-176`) hides any
`EXTRA_KEYS` value whose leaf segment is `"token"` or `"token_cmd"` — this is
leaf-name-based, so `github.token`/`github.token_cmd` will automatically render as
`*(set — hidden)*` in `config dump` with no extra code once added to `EXTRA_KEYS`.

**The discrepancy.** Work-item 0197's Technical Notes (lines 272-275) state the
precedence/ban logic is "already implemented for `jira.token`/`jira.token_cmd` and
`linear.token`/`linear.token_cmd` in `cli/config/src/catalogue.rs:124-127` and
`cli/config-adapters/src/store.rs`." Direct inspection of both files shows this is not
accurate: `catalogue.rs:124-127` only declares the four key names (no logic at all),
and `cli/config-adapters/src/store.rs` (`FileConfigStore`, all 1181 lines read) is a
generic two-level (`Team`/`Personal`) file store with zero `token_cmd`-aware code —
`jira.token` appears in its tests only as a plausible credential-shaped key for
generic file-mode/symlink-escape assertions, not for any precedence logic.

**Where the real logic lives (bash only).** The four-step precedence chain and the
shared-config `token_cmd` ban are implemented independently, in near-identical shape,
in `skills/integrations/jira/scripts/jira-auth.sh` (`jira_resolve_credentials`,
lines 132-250) and `skills/integrations/linear/scripts/linear-auth.sh`
(`linear_resolve_credentials`, lines 167-258):

1. `ACCELERATOR_JIRA_TOKEN`/`ACCELERATOR_LINEAR_TOKEN` env var — source `"env"`.
2. `ACCELERATOR_JIRA_TOKEN_CMD`/`ACCELERATOR_LINEAR_TOKEN_CMD` env var, executed via
   `bash -c` — source `"env_cmd"`.
3. `config.local.md` — gated on file mode ≤ 0600 (else `E_LOCAL_PERMS_INSECURE`, with
   an `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` + VCS-tracked marker override) — `token`
   first (source `"local"`), else `token_cmd` executed via `bash -c` (source
   `"local_cmd"`).
4. `config.md` (shared/team) — **only when `config.local.md` is absent** — `token`
   only (source `"shared"`); `token_cmd` is read here *only* to detect-and-warn
   (`E_TOKEN_CMD_FROM_SHARED_CONFIG` on stderr) and is never executed. This read goes
   through the CLI itself (`accelerator config get jira.token ""`), while steps 1-3
   read `config.local.md` directly via an awk-based helper.
5. Failure: `E_NO_TOKEN` if nothing resolved.

There is no shared helper function for the ban — it is duplicated inline in both
scripts, same shape each time. `linear-auth.sh` additionally validates the resolved
token for shell-hostile characters (quote/backslash/control-char/newline) since it gets
embedded in a quoted `curl --config -` directive — jira's resolver has no equivalent
guard, so whether `github.token` needs one is a judgement call depending on how the
token reaches `octocrab` (this is an HTTP client library, not shelled through `curl`,
so this specific guard is likely unnecessary — but worth confirming).

**Implication for the plan.** Since 0197 does not shell out to `gh`/`curl` at all
(it calls `octocrab` in-process), there is no existing Rust code to extend for
`github.token`/`github.token_cmd` resolution — a **new Rust module implementing this
four-way precedence chain** (config value → `token_cmd` output → `GH_TOKEN` env →
`GITHUB_TOKEN` env, per the work item's stated order, which already differs slightly
from jira/linear's env-before-config order) needs to be written, mirroring the
*behavioural contract* of `jira_resolve_credentials`/`linear_resolve_credentials`
(including the shared-config `token_cmd` ban) rather than reusing existing Rust code.
Existing bash test suites for this behaviour: `test-jira-auth.sh` (Test 6 is the ban
test; Test 5a covers "shared token blocked when local file exists but has no token
entry") and `test-linear-auth.sh` (same shape) — useful as a checklist of cases a Rust
equivalent's own tests should cover, even though nothing there is directly portable.

### 3. `vcs`/`vcs-adapters` — origin-remote-URL parsing (green-field addition)

**Confirmed: no remote-reading capability exists anywhere in either crate today.**
`RepoFacts` (`cli/vcs/src/lib.rs:42-48`) has exactly four fields (`root`, `name`,
`kind`, `revision`) — no remote field. `name` is derived purely from
`repository_root.file_name()` (`lib.rs:110`), confirmed by the doc comment
(`lib.rs:38-41`: "the *repository* the working copy belongs to... stamps artifacts
with the repository's name," i.e. basename semantics, not remote-derived) and by an
existing test (`the_name_is_the_final_component_of_the_root`,
`cli/vcs/src/lib.rs:180-193`). A repo-wide grep for `remote`/`origin` inside `vcs`/
`vcs-adapters` turns up only two non-functional hits: a literal test string
`"git remote -v"` in the VCS guard's allowlist test (`guard.rs:233`), and a static
help-text line in `vcs-cli/src/detect.rs:53`.

**The port-design convention to follow** (established by `CheckoutProbe`/`ModeProbe`,
which were deliberately *narrowed* from a fuller query surface — comment at
`cli/vcs/src/classify.rs:46-50`): define a small, single-purpose trait (e.g.
`OriginRemote { fn origin_url(&self, root: &Path) -> Result<Option<String>,
kernel::Error>; }`) in its own file in the `vcs` domain crate, alongside
`checkout.rs`/`mode.rs`. Parsing the URL into an owner/repo pair (the four supported
forms in the work item's Requirements) is pure logic and belongs in the domain crate,
separate from the port itself — mirroring how `classify()`/`determine()` are pure
functions composed over injected probes.

**Adapter implementation, both backends already have the raw capability to extend:**

- **`subprocess` adapter** (`cli/vcs-adapters/src/subprocess.rs`) — `CommandProbe`
  already runs real `git`/`jj` binaries through a capped, polled wait (`run_capped`,
  lines 243-302) with a scrubbed environment (`scrub_environment`, lines 220-235,
  denying `GIT_DIR`/`JJ_CONFIG` etc.). A `git remote get-url origin` invocation would
  reuse this exact machinery. Every existing failure mode in this adapter is
  `warn!`-logged and folded to `None`/fallback text rather than propagated as `Err` —
  worth deciding deliberately whether the new port breaks this convention (the work
  item wants a hard, user-facing "no `origin` remote configured" error, which argues
  for `Result<Option<String>, kernel::Error>` rather than silent `None`-on-failure, to
  distinguish "cleanly absent" from "probe itself failed").
- **`library` adapter** (`cli/vcs-adapters/src/library.rs`) — already reads git config
  via `gix`'s `repository.config_snapshot()` for `git_user_name` (lines 488-505,
  reading `user.name`); the identical mechanism reads `remote.origin.url`. `gix` is
  already a pinned dependency of `vcs-adapters` (`~0.85.0`, `default-features =
  false`).

**Testing convention to mirror.** Domain-level: a hand-written `StubProbe` in the same
module (pattern used for `CheckoutProbe`/`ModeProbe` — a `Default`-derived struct of
`Option<Result<T, String>>` fields, converted to `kernel::Error` via a local helper).
Adapter-level, if real-repo coverage is wanted: new fixture shapes added to
`vcs-test-support`'s `Matrix` (`cli/vcs-test-support/src/fixtures.rs`) plus a
`bash-parity`-gated integration test exercising both `subprocess` and `library`
adapters for parity, following `cli/vcs-adapters/tests/queries.rs`'s existing
oracle-table pattern. Given 0197's own AC only requires **unit-level** coverage of the
four URL forms (no bash-script branch to characterize), the full `Matrix`/`bash-parity`
machinery is likely more than this item needs — a lighter, purpose-built set of fixture
repos (each with a real `origin` remote set to one of the four URL forms) is probably
sufficient, but the existing infrastructure is there if fuller coverage is wanted.

### 4. Sub-binary registration checklist (`tasks/README.md`)

The full thirteen-point checklist lives at `tasks/README.md` (the "## Registering a
dispatched sub-binary" section, work item cites lines 304-441). Each item is tagged
**[PR]** (a CI gate catches a miss), **[release]** (only the release job catches it),
or **[author]** (nothing catches it — must get right by hand):

1. Add the token to `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py`); update the
   registry pin, upload count, and staged fixture/manifest in
   `tests/integration/tasks/test_github.py`. **[PR]**
2. Add a `_SUBBINARY_MANIFESTS` entry (`tasks/manifest.py`) — needed for
   `collaboration` since the binary crate deviates from `cli/<token>/` (mirrors
   `"vcs": CLI_DIR / "vcs-cli/Cargo.toml"`). **[release]**
3. Crate `Cargo.toml`: `[[bin]] name = "accelerator-collaboration"`, mandatory
   `package.description`, inherited `version`/`edition`/`rust-version`/`license`/
   `publish`, and a lints table (`workspace = true` or crate-local). **[release]**
4. Register in `cli/Cargo.toml` `[workspace].members`, commit the regenerated
   `Cargo.lock` (`lint:cli:check` runs `--locked`). **[release]**
5. Add `bin/collaboration-*` to `.gitignore`. **[author]** — nothing catches a miss.
6. Optionally add an entry to `cli/launcher/tests/fixtures/manifest.example.json` (not
   required — the golden contract is key-agnostic). **[author]**
7. Skill binding: a skill invoking `accelerator collaboration` via the `!`
   preprocessor with a tightly-scoped `Bash(...)` allowed-tools rule (subcommand
   segment exactly `collaboration`, no wildcards, no bare-launcher rule). If satisfied
   by a *new* config/instruction-injecting skill, bump
   `EXPECTED_INJECTION_SKILLS`. **[PR]**
8. Add `accelerator-collaboration` to `_CLI_RELEASE_BINARIES`
   (`tasks/build.py`) — rides the existing `cli_cross_compile` staging path for
   free. **[release]**
9. Update the three `attest-build-provenance` blocks in `.github/workflows/main.yml`
   only if a new artefact type is introduced that no existing glob covers — not
   needed for a plain new binary (`dist/release/accelerator-*` already matches).
   **[PR]**
10. `BUILTIN_SUBCOMMANDS` (`tasks/shared/dispatch_coherence.py`) — no action for a
    plain dispatch token; only relevant if the launcher's own built-in set changes.
    **[PR]**
11. User-facing docs: docs-site page, root README Concepts entry,
    `ACCELERATOR_COLLABORATION_BIN` override row, Starlight sidebar config. **[author]**
12. `DEBUG_ARCHIVE_DIRS` (`tasks/shared/paths.py`) only if shipping a symbolication
    archive; triggers item 9's obligation. **[author]**
13. Extend `cli/deny.toml` only if `mise run deny:check` actually flags something
    once `octocrab` is added. **[PR]**

**Items 1, 2, 3, 4, 7, and 8 must land in the same change** — confirmed verbatim in
`tasks/README.md`: "The release path resolves them together, and only the 1↔7 pair is
caught before the release job — by the dispatch guard... which runs from
`tasks/manifest.py` on every release *and* as `lint:dispatch-coherence:check`." Items
2/3/4/8 fail silently until the release job itself runs.

**CI floor constant.** `tasks/test/integration.py:74` currently reads
`_EXPECTED_GITHUB_SUITES = 3` (not yet decremented — this item's job, per its own
Requirements). `_EXPECTED_DECISIONS_SUITES = 0` (line 73) shows the pattern already
applied by a sibling item, confirming each domain has an independent, named,
hand-ratcheted floor constant enforced via `_require_suite_floor`. The work item's own
Dependencies section correctly flags a **concurrency risk**: 0195, 0196, and 0197 each
decrement their own independent constant in this same shared file around the same
time — not a logical conflict, but ordinary merge contention.

**Best practical precedent to mirror structurally**: the 0195 (corpus) plan
(`meta/plans/2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`,
status `done`), whose Phase 1 walks through this exact checklist end-to-end with
concrete file/line targets — including a corrected derived-uploads-count formula, since
`tasks/README.md`'s own worked example (`assert len(uploads) == 22`) is stale relative
to the now-generalised checklist.

**cargo-deny guidance for `octocrab`.** No generic "why we added this dependency"
section exists in `cli/deny.toml` — entries are purely reactive. The concrete first
step is: add `octocrab` to `cli/Cargo.toml`, run `mise run deny:check`
(equivalently `cli:check`), and react to whatever it reports (see § 6 below for the
specific risk this will surface).

### 5. PR-helper bash scripts — exact behaviour and discrepancies against the work item

**Files** (all under `skills/github/`):
`scripts/pr-base-repo.sh`, `scripts/test-pr-base-repo-scripts.sh`,
`scripts/test-pr-base-repo-real-gh.sh`, `scripts/test-helpers.sh`,
`describe-pr/scripts/pr-update-body.sh`, `describe-pr/scripts/test-pr-update-body-scripts.sh`,
plus the three call sites `review-pr/SKILL.md`, `respond-to-pr/SKILL.md`,
`describe-pr/SKILL.md`, and `tasks/test/integration.py:74,377-380` for the
`_EXPECTED_GITHUB_SUITES` floor and its `github` integration task wiring. No other
skill or script anywhere in the repository references either script (confirmed by a
whole-repo grep — only `meta/` planning documents and the scripts/SKILL.md files
themselves mention them).

**`pr-base-repo.sh` branches** (exit codes: 0 success, 1 resolution failed, 2 usage
error):

- Usage error (wrong arg count) → exit 2.
- Missing `jq` preflight → exit 2 (no Rust analog, correctly dropped from the
  migration count).
- `gh pr view "$pr_number" --json url` failure — **a single bash branch** covering
  every failure mode (network error, auth failure, 404, no default remote, etc.):
  gh's captured stderr is replayed verbatim, then a generic
  `could not resolve base repo` message, then **only if** the captured stderr
  contains the literal substring `"no default remote repository"` an extra
  remediation line is appended — this is a `grep`-based string match on gh's own
  error text, not a structurally distinct branch or exit code → exit 1.
- Malformed/non-JSON gh output (`jq -e .` pre-validation fails) → exit 1, raw
  payload echoed.
- Empty/null `.url` field → exit 1, raw payload echoed.
- `url` present but doesn't match the expected PR-URL regex
  (`^https://[^/]+/([A-Za-z0-9][A-Za-z0-9-]*)/([A-Za-z0-9._-]+)/pull/[0-9]+$`,
  deliberately restrictive — owner cannot start with `-`, repo can start with `.`/`_`
  e.g. `.github`) → exit 1.
- Success → owner/repo printed to stdout, exit 0.

**Discrepancy (a):** the work item's Technical Notes enumerate resolver branches (2)
"REST request failure" and (3) "missing `origin` git remote" as two separate branches.
In the current bash, these are one branch (see above) — the split only becomes real in
the *target* Rust design, where local owner/repo resolution moves to an explicit
pre-network-call `vcs`-level step (so a missing `origin` remote becomes a genuinely
distinct, earlier failure). This split is the right target design, not a bug in the
work item — but characterization tests should be written against the *target*
behaviour, not against a literal (nonexistent) bash branch boundary, and this should be
stated explicitly in the plan so it isn't mistaken for a bash-parity gap.

**`pr-update-body.sh` branches** (exit codes: 0 success, 1 encode/resolution failed,
2 usage error, 4 PATCH failed):

- Usage error (wrong arg count) → exit 2.
- Missing/unreadable `--body-file` → exit 2.
- Missing `jq` preflight → exit 2 (dropped from migration count).
- Base-repo resolution failure — invokes `pr-base-repo.sh` as a subprocess, captures
  its exit code explicitly, prepends one contextual line, then **propagates the
  resolver's own exit code verbatim** (its stderr, including the resolver's own
  conditional remediation hint, has already been emitted by the resolver itself).
- `jq` JSON-encode failure on the body file → exit 1 (dropped from migration count).
- `gh api --method PATCH repos/$base_repo/pulls/$pr_number --input <payload>` failure
  → exit 4 (the one exit code unique to this script).
- Success → gh's own stdout passes through unredirected, exit 0.

This matches the work item's 5-branch enumeration exactly (usage, missing file,
resolver-failure-propagation, PATCH failure, success), with both jq-only branches
correctly dropped.

**Discrepancy (b):** the work item states `test-pr-base-repo-scripts.sh` has "15
cases." Direct count: 16 numbered/lettered branch-behaviour test blocks (including
URL-charset hardening sub-cases 4b/4c/4d/4e, added later per work-item 0071) plus 3
regression guards (tests 22-24, including a legacy `--json baseRepository` field
guard) = **19 total**. `test-pr-update-body-scripts.sh`'s stated count of 23 is
correct (21 numbered + 2 regression guards). This doesn't change the underlying
7+5-branch characterization target, but the actual bash test surface for the resolver
is larger than stated — worth knowing so a repointed suite isn't accidentally scoped
to fewer cases than currently exist.

**Test stubbing mechanism** (for reference, being replaced per the AC):
`skills/github/scripts/test-helpers.sh`'s `install_fake_gh`/`setup_gh_stub` writes a
fake `gh` into a `PATH`-prepended temp `bin/`, dispatching on `pr view` vs `api`
subcommands, logging argv/stdin, and returning caller-configured
stdout/stderr/exit-code via `GH_*_OUT`/`GH_*_ERR`/`GH_*_RC` env vars; a second fake,
`install_fake_jq`, simulates an encode failure for one specific test case.
`test-pr-base-repo-real-gh.sh` deliberately runs against the real, installed `gh` as a
smoke test (skipped if `gh` is absent from `PATH`) and is a stated candidate for
retirement rather than porting, since it exists only to probe `gh`'s `--json` field
allowlist — moot once no `gh --json` call remains.

### 6. HTTP client / `octocrab` precedent

**`octocrab` is not a dependency anywhere in the workspace** (`Cargo.toml`/
`Cargo.lock` grep confirms this) — genuinely new. **`reqwest` is already a pinned
workspace dependency** (`cli/Cargo.toml:30-34`, exact-pinned `=0.12.28`,
`default-features = false`, features `["blocking", "rustls-tls-webpki-roots-no-provider",
"hickory-dns"]`, paired with a pinned `rustls = "=0.23.41"`), currently used only by
`cli/launcher/src/launch/outbound/resolve/fetcher.rs`'s `Fetcher` (wraps
`reqwest::blocking::Client`, https-only + redirect-host allowlist in production, a
`with_backoff()` constructor relaxing to `http` for pointing at a local mock server in
tests).

**Deny-list risk.** `cli/deny.toml:99-106` bans `native-tls`, `openssl`,
`openssl-sys` outright (keeps the musl-static build intact). `octocrab`'s default
feature set pulls in reqwest with `native-tls` unless explicitly configured with
`default-features = false` and a rustls variant selected — this **will need to mirror
the workspace's existing reqwest feature flags exactly**, or `deny:check` fails. This
is the most concrete, actionable risk surfaced by this research: get the `octocrab`
dependency declaration's feature flags right on the first attempt (matching the
existing workspace `reqwest` entry) rather than discovering the ban via a failing
`deny:check`.

**No Jira/Linear Rust port exists to compare against.** Work-item 0171 (Jira/Linear
integrations) is still `status: draft`, blocked on 0187/0194; no `cli/tracker`,
`cli/jira-client`, or `cli/linear-client` crates exist. Today's Jira/Linear
integrations are bash + Python, with hand-rolled Python `http.server`-based mock
servers (`skills/integrations/{jira,linear}/scripts/test-helpers/mock-*-server.py`) —
not a Rust pattern to copy structurally, though useful for behavioural-parity
reference if ever needed. `cli/config/src/catalogue.rs`'s existing `github-issues`
value in `WORK_INTEGRATION_VALUES` (§ 1 above) suggests 0197's `collaboration` crate
may eventually be expected to back a `RemoteTracker`-shaped port from 0194, but 0197
itself is independent of that not-yet-built crate.

**HTTP-mocking pattern established in this codebase (the direct precedent for 0197's
AC "HTTP-level stubbing replacing the PATH-`gh`-stub harness").** No external mocking
crate (`wiremock`/`mockito`) exists anywhere in the workspace. The pattern is a
hand-rolled, `std`-only HTTP/1.1 mock server:
`cli/launcher/tests/common/mod.rs` — `MockServer::start()` binds an ephemeral loopback
`TcpListener`, spawns a background thread; `route(path, Route)` registers per-path
responses (`Route::Ok`, `Status`, `Redirect`, `FlakyThenOk { fail_times, body }` for
retry-logic tests, `Stall(duration)` for timeout tests); `hits(path)` asserts call
counts; a minimal raw-socket parser/responder handles requests, no framework
dependency. Consumed in `cli/launcher/tests/resolution.rs` via a `Harness` struct.
The direct transferable pattern for 0197: point `octocrab::Octocrab::builder()`'s base
URI at `mock_server.base_url()`, register routes/fixtures per test, assert on
`hits()`/captured bodies — no live GitHub API calls, no `PATH`-based `gh` stub needed.

## Code References

- `cli/vcs/src/lib.rs:42-120` — `RepoFacts`, port traits (`RepoRoot`, `VcsProbe`,
  `UserIdentityProbe`), `facts()`/`user_name()` composition functions.
- `cli/vcs/src/classify.rs:46-76,203-252` — `CheckoutProbe` (narrowed port pattern),
  `StubProbe` domain-test-fake pattern.
- `cli/vcs-adapters/src/lib.rs:1-26` — subprocess vs library backend split doc/wiring.
- `cli/vcs-adapters/src/subprocess.rs:30-42,64-128,220-302` — `MarkerWalkRoot`,
  `CommandProbe`, `scrub_environment`, `run_capped`.
- `cli/vcs-adapters/src/library.rs:488-505,772-800` — `gix`-based `git_user_name`
  (the mechanism to extend for `remote.origin.url`), `discover_git`/`open_git`.
- `cli/vcs-cli/src/cli.rs:8-13`, `cli/vcs-cli/src/main.rs:19-25,71-101` — clap Cli/
  Command shape, error/exit-code mapping, `current_dir()` helper.
- `cli/work-cli/src/main.rs:20-21,36-84,102,298-312`, `cli/work-cli/src/config.rs:12-71`
  — config composition pattern (`compose`, `ConfigAccess`, plugin-root-relative store),
  per-subcommand outcome-to-exit-code mapping.
- `cli/config/src/catalogue.rs:98-131,257-267,410-476` — `EXTRA_KEYS`,
  `WORK_INTEGRATION_VALUES`, key-count test, Rust/bash drift test (excludes
  `EXTRA_KEYS`).
- `cli/config-adapters/src/store.rs` (full file, 1181 lines) — `FileConfigStore`,
  confirmed no `token_cmd`-specific logic.
- `skills/integrations/jira/scripts/jira-auth.sh:132-250` — `jira_resolve_credentials`
  (the precedence/ban logic to mirror behaviourally in Rust).
- `skills/integrations/linear/scripts/linear-auth.sh:167-258` —
  `linear_resolve_credentials`.
- `scripts/config-defaults.sh:199-218` — bash `EXTRA_KEYS` mirror.
- `tasks/README.md:304-441` — the thirteen-point sub-binary registration checklist.
- `tasks/shared/paths.py:29-34` — `DISPATCHED_SUBBINARIES`.
- `tasks/manifest.py:53-60` — `_SUBBINARY_MANIFESTS`.
- `tasks/test/integration.py:73-74,377-380` — `_EXPECTED_DECISIONS_SUITES` (already
  decremented, precedent), `_EXPECTED_GITHUB_SUITES` (this item's job).
- `cli/launcher/src/launch/mod.rs`, `cli/launcher/src/launch/core.rs:227-293`,
  `cli/launcher/src/launch/outbound/resolve/manifest.rs:26-32` — data-driven dispatch.
- `cli/launcher/src/launch/outbound/resolve/fetcher.rs:53-101` — `Fetcher`
  production/test-seam pattern.
- `cli/launcher/tests/common/mod.rs` — hand-rolled `MockServer` HTTP-stubbing pattern.
- `cli/deny.toml:39-42,52-64,67-81,99-106` — advisory-ignore format, license
  allow-list, exception pattern, banned-crate list (`native-tls`/`openssl`/
  `openssl-sys`).
- `cli/Cargo.toml:30-34` — pinned `reqwest`/`rustls` feature flags to mirror for
  `octocrab`.
- `skills/github/scripts/pr-base-repo.sh` (full file) — resolver subcommand source.
- `skills/github/describe-pr/scripts/pr-update-body.sh` (full file) — body-update
  subcommand source.
- `skills/github/scripts/test-pr-base-repo-scripts.sh` — 19 test blocks (not 15).
- `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` — 23 test blocks
  (matches work item).
- `skills/github/scripts/test-helpers.sh:17-179` — `install_fake_gh`/`setup_gh_stub`/
  `install_fake_jq` PATH-stubbing harness being replaced.
- `cli/work-adapters/src/project_remote.rs:9-80` — pure JSON-projection adapter
  precedent, `Integration` enum shape.

## Architecture Insights

- **Domain crates never name a concrete adapter or transport.** Every port trait in
  `vcs`/`work` is satisfied by `&dyn Trait`, with real I/O confined to the adapters
  crate. This is the load-bearing convention `collaboration` must follow for the
  GitHub REST port to be testable without live network calls.
- **The crate-directory name and the Cargo package/binary name are allowed to
  diverge** (`vcs-cli/` directory → `accelerator-vcs` package/binary) whenever the
  binary crate isn't co-located with a same-named domain crate — this is exactly
  `collaboration`'s situation and is why `_SUBBINARY_MANIFESTS` (checklist item 2)
  is required, not optional, for this item.
- **Sub-binary dispatch is entirely data-driven** — no launcher Rust code changes are
  ever needed to add a new domain; only registry constants, a Cargo workspace member,
  and (at release time) a signed manifest entry. The "landing together" constraint
  (items 1/2/3/4/7/8) exists because the release path resolves them as one unit, only
  partially guarded pre-release.
- **Bash-script "branches" are often a single `if`/exit-code covering several English-
  language failure modes**, distinguished only by which optional stderr line gets
  appended based on string-matching the underlying tool's own error text. A target-Rust
  branch enumeration that treats these as separate structural branches is doing
  legitimate re-architecture, not literal characterization — this distinction should be
  explicit in the plan so reviewers don't read it as a bash-parity gap.
- **No mocking framework is used anywhere in this Rust workspace** — either hand-written
  fake trait implementations (domain-level) or a hand-rolled loopback HTTP server
  (adapter/integration-level). `octocrab`'s tests should follow this house style rather
  than introducing `wiremock`/`mockito` as a new dependency.
- **Config `EXTRA_KEYS` is a "just the names" catalogue, not a policy engine.** All
  actual resolution/precedence policy for ad-hoc integration keys lives outside the
  catalogue, in per-integration code (today, bash resolvers; for `collaboration`, a new
  Rust resolver in `collaboration-cli` or `collaboration-adapters`).

## Historical Context

- `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md` (abandoned) —
  0197 was split out of this item on 2026-08-05 after review-1 flagged that bundling
  three independent sub-binaries risked partial-completion ambiguity.
- `meta/work/0071-describe-pr-base-repo-resolver-uses-unsupported-gh-field.md` and its
  linked review/research/plan documents — the origin of the URL-shape regex hardening
  and the legacy `--json baseRepository` regression guard now present in
  `test-pr-base-repo-scripts.sh` (tests 4b-4e, 24) but not reflected in the work item's
  "15 cases" figure.
- `meta/plans/2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`
  (status `done`) — the most concrete practical worked example of the sub-binary
  registration checklist, including a corrected derived-uploads-count formula
  superseding `tasks/README.md`'s own stale worked example.
- `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
  and `meta/plans/2026-08-02-0187-generalise-sub-binary-registration-surface.md` — the
  research/plan pair that produced the thirteen-point checklist itself.
- ADR-0053 (cited by 0197) — establishes the thin-CLI-over-hexagonal-ports-and-adapters
  core pattern that `vcs`/`work`/`collaboration` all instantiate.

## Related Research

None found specific to `collaboration`/GitHub-REST-port design; the closest prior art
is the 0195 corpus research/plan pair and the 0187 registration-surface research/plan
pair cited above.

## Open Questions

- **Env-var precedence order for `github.*` vs `jira.*`/`linear.*`.** The work item
  specifies config → `token_cmd` → `GH_TOKEN` → `GITHUB_TOKEN` (config keys ahead of
  *all* env vars). The existing jira/linear resolvers put env vars *first*, ahead of
  config. This is presumably deliberate (matching `gh`'s own documented precedence
  order rather than jira/linear's), but the plan should state explicitly that
  `github.*`'s resolver is a new precedence shape, not a copy of jira/linear's order,
  so it isn't "fixed" to match them during implementation.
- **Does the new `OriginRemote` vcs port raise `Err` on probe failure or fold to
  `None`?** Existing `vcs-adapters` convention (both backends) is to `warn!`-log and
  fold every failure to `None`/fallback text, never propagating `Err` to callers for
  the "infallible" port family. The work item wants a clear, user-facing "no `origin`
  remote configured" error distinct from a probe malfunction — this likely means the
  new port should join the `CheckoutProbe`/`ModeProbe` fallible family
  (`Result<Option<String>, kernel::Error>`) rather than the infallible family
  (`RepoRoot`/`VcsProbe`/`UserIdentityProbe`), but this is a design choice to make
  explicitly during planning, not something the existing code decides for you.
- **Whether `linear-auth.sh`'s token-content validation (rejecting quote/backslash/
  control-char/newline) has any Rust equivalent need.** Likely no, since `octocrab`
  is an HTTP client library (token goes into an `Authorization` header, not a shelled
  `curl` command), but worth a one-line confirmation in the plan rather than silent
  omission.
- **Exact shape of the `--body-file` reading path in Rust** — the bash script reads an
  arbitrary local file path and JSON-encodes its raw bytes via `jq -Rs`; the work item
  doesn't specify encoding/newline-handling nuances (e.g. trailing-newline behaviour)
  that a Rust `std::fs::read_to_string` + `serde_json::to_string` might handle
  differently from `jq -Rs`. Not investigated here since it's a narrow branch (2), but
  worth a byte-for-byte comparison during implementation if strict behavioural parity
  matters.
