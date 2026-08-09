---
type: plan
id: "2026-08-08-0197-accelerator-collaboration-pr-helper-cli"
title: "accelerator-collaboration: PR Helper CLI Implementation Plan"
date: "2026-08-08T16:10:01+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0197"
parent: "work-item:0197"
derived_from: ["codebase-research:2026-08-08-0197-accelerator-collaboration-pr-helper-cli"]
tags: [rust, collaboration, cli, github, gh, octocrab]
revision: "32ea3631c3796388be454a7eceeecf9c0d9c26be"
repository: accelerator
last_updated: "2026-08-08T21:49:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# accelerator-collaboration: PR Helper CLI Implementation Plan

## Overview

Migrate `skills/github/scripts/pr-base-repo.sh` and
`skills/github/describe-pr/scripts/pr-update-body.sh` into a new
`accelerator-collaboration` sub-binary. The new binary calls the GitHub REST
API in-process via `octocrab` (not by shelling out to `gh`), authenticates via
a new `github.token`/`github.token_cmd` config pairing (with `GH_TOKEN`/
`GITHUB_TOKEN` env fallbacks), and resolves the PR's base (upstream)
repository owner/repo in two steps: parsing the `origin` git remote's URL
via a new capability added to `vcs`/`vcs-adapters` to get a candidate
owner/repo, then a `collaboration`-level repository-metadata lookup that
follows GitHub's `parent` field when the candidate is a fork — replicating
`gh`'s own default fork-to-parent resolution rather than assuming `origin`
is always already the upstream repository (see Key Discoveries). `review-pr`,
`respond-to-pr`, and `describe-pr` are repointed to call it, the two bash
scripts and their three test suites are removed, and the
`_EXPECTED_GITHUB_SUITES` CI floor drops from 3 to 0.

## Current State Analysis

Today, three skills shell out to two untestable bash scripts that both wrap
`gh`:

- `skills/github/scripts/pr-base-repo.sh` resolves a PR's base (upstream)
  owner/repo by running `gh pr view <pr> --json url` and regex-extracting the
  owner/repo from the URL. It exits 0 (success, prints `owner/repo` to
  stdout), 1 (resolution failed — network/auth/404/malformed-JSON/empty-url/
  bad-url-shape, all folded into one bash branch), or 2 (usage error).
- `skills/github/describe-pr/scripts/pr-update-body.sh` shells out to
  `pr-base-repo.sh` internally to resolve the base repo, then PATCHes the PR
  body via `gh api --method PATCH repos/<repo>/pulls/<pr> --input <file>`. It
  exits 0, 1 (encode failed, or the resolver's own exit code propagated
  verbatim), 2 (usage error), or 4 (PATCH failed).
- `review-pr/SKILL.md`, `respond-to-pr/SKILL.md`, and `describe-pr/SKILL.md`
  all invoke `pr-base-repo.sh` directly and capture its `owner/repo` stdout;
  `describe-pr/SKILL.md` additionally invokes `pr-update-body.sh` and
  documents its four exit codes explicitly in its own error-handling prose.

The codebase has a proven three-crate structural pattern for a dispatched
sub-binary (domain crate → adapters crate → thin `-cli` crate) that `vcs` and
`work` already instantiate (`cli/vcs/`, `cli/vcs-adapters/`, `cli/vcs-cli/`;
`cli/work/`, `cli/work-adapters/`, `cli/work-cli/`), and a fully data-driven
launcher dispatch mechanism that needs zero launcher code changes for a new
domain — only registry/manifest entries
(`cli/vcs/src/lib.rs:42-120`, `cli/vcs-cli/src/main.rs`,
`cli/work-cli/src/config.rs`).

Three capabilities this migration needs do not exist yet, confirmed by direct
inspection (not just by the work item's own claim):

- **`github.token`/`github.token_cmd` resolution logic.** `EXTRA_KEYS` in
  `cli/config/src/catalogue.rs:121-131` only declares key *names* — no
  default, validation, precedence, or ban logic exists for any `EXTRA_KEYS`
  entry in Rust. The real four-step precedence chain (env → env `_cmd` →
  local `token`/`token_cmd` → shared `token` only, with a shared-config
  `token_cmd` ban) is implemented independently in bash in
  `skills/integrations/jira/scripts/jira-auth.sh` and
  `skills/integrations/linear/scripts/linear-auth.sh`. There is nothing to
  reuse in Rust — only a behavioural contract to mirror, and 0197's own
  precedence order (config-first, env-last) deliberately differs from
  jira/linear's (env-first).
- **Origin-remote-URL-to-owner/repo parsing.** `vcs`'s `RepoFacts` has no
  remote-derived field at all — `name` is the local directory basename
  (`cli/vcs/src/lib.rs:42-120`) — and neither `vcs-adapters` backend reads a
  git remote today. This is a green-field addition.
- **The `collaboration`/`collaboration-adapters`/`collaboration-cli` crates
  themselves**, and the `octocrab` dependency (not present anywhere in the
  workspace).

### Key Discoveries:

- **`octocrab` requires an async runtime; nothing in `cli/` uses one today.**
  The workspace's existing `reqwest` usage
  (`cli/launcher/src/launch/outbound/resolve/fetcher.rs`) is the
  `blocking` feature only. `octocrab` has no blocking API — every call is
  `async fn`. This is confirmed via `octocrab`'s own docs (all examples use
  `#[tokio::main]`). `tokio` is a new workspace dependency purely to drive
  `collaboration-adapters`' async calls to completion; it is confined to
  `collaboration-cli`'s `main()`, which stays `fn main() -> ExitCode` like
  its siblings by wrapping the async work in a single
  `tokio::runtime::Builder::new_current_thread().enable_all().build()` +
  `.block_on(...)` call.
- **`octocrab`'s TLS feature must be pinned to avoid the `native-tls` ban.**
  `cli/deny.toml:99-106` denies `native-tls`/`openssl`/`openssl-sys`
  outright. `octocrab` defaults to `opentls` unless `default-features =
  false` and an explicit `rustls-*` feature (e.g. `rustls-ring`, matching the
  existing `reqwest`/`rustls` pin's `ring` provider,
  `cli/Cargo.toml:30-35`) is selected.
- **No simple `base_uri()` setter exists on `Octocrab::builder()`** for
  pointing at a mock server. The documented pattern for a custom base URI is
  `OctocrabBuilder::new_empty().with_service(client).with_layer(&BaseUriLayer::new(uri))`
  — this is the seam `collaboration-adapters`' own tests use to point at the
  hand-rolled mock server (mirroring
  `cli/launcher/tests/common/mod.rs`'s `MockServer`), rather than
  `Fetcher::with_backoff`'s simpler pattern (`reqwest` alone has no
  equivalent builder restriction).
- **A single `GET /repos/{owner}/{repo}/pulls/{pull_number}` call cannot
  resolve the base repo on its own.** `GET /repos/{owner}/{repo}/pulls/
  {pull_number}` only ever returns PRs scoped to the exact `{owner}/{repo}`
  supplied in the request — the response cannot reveal a *different* repo
  than what was queried, regardless of which field is read (`url`, or even
  `base.repo.full_name`); parsing owner/repo back out of it is circular.
  `gh pr view <pr> --json url`'s actual cross-fork-safety comes from `gh`'s
  own pre-call resolution (it targets the parent repo by default when the
  local checkout is a fork), not from anything in the response. The design
  below therefore resolves the fork-parent relationship itself, before
  making any PR-number-scoped call: `GET /repos/{owner}/{repo}` against the
  `origin`-parsed candidate (checked for a `parent`), then
  `GET /repos/{base_owner}/{base_repo}/pulls/{pull_number}` purely to
  confirm the PR exists at the resolved base (a 404 there means the PR is
  not at the resolved base — e.g. opened directly against the fork rather
  than upstream — and is surfaced as a failure rather than a silently
  wrong repo).
- **`pr-base-repo.sh`'s "REST failure" and "missing origin remote" are one
  bash branch, not two** — any `gh pr view` failure hits the same `exit 1`,
  distinguished only by an optional stderr line gated on a substring match
  against gh's own error text. The *target* Rust design correctly splits
  these into two branches (missing origin remote is now a separate,
  earlier, `vcs`-level failure, resolved before any network call) — this is
  legitimate re-architecture, not a bash-parity gap, and the plan's
  characterization tests are written against this target behaviour.
- **The `base-repo` subcommand's stdout contract (`owner/repo\n`) is a real
  external contract**, not just an implementation detail: `review-pr`
  writes it to `repo-info.txt` and later reads it back to build the Reviews
  API URL (`skills/github/review-pr/SKILL.md:125`); `respond-to-pr` calls it
  directly for the same purpose (`skills/github/respond-to-pr/SKILL.md:67`).
  This format must be preserved exactly.
- **The sub-binary registration checklist's items 1, 2, 3, 4, 7, and 8 must
  land in the same change** (`tasks/README.md:431-434`) — the release path
  resolves them together, and the dispatch-coherence guard (item 7) needs a
  real skill call site naming the token, so the skill rewiring cannot be
  deferred to a later phase than the crate registration.
- **The upload-count assertion in `tests/integration/tasks/test_github.py`
  is already a derived expression**
  (`len(DISPATCHED_SUBBINARIES) * len(_PLATFORMS) * 2`, line 341-343), not
  the stale hardcoded `22` `tasks/README.md`'s own worked example warns
  about — a prior sibling already converted it. Two things there still need
  a literal edit: `_SUBBINARY_DESCRIPTIONS` (line 35) needs a
  `"collaboration"` entry, and
  `test_the_dispatched_registry_holds_visualiser_vcs_work_and_corpus`
  (line 516-521) asserts the literal tuple
  `("visualiser", "vcs", "work", "corpus")` and needs `"collaboration"`
  appended (and, ideally, renaming to keep matching its own assertion).

## Desired End State

`accelerator collaboration base-repo <pr-number>` and
`accelerator collaboration update-body <pr-number> --body-file <path>` exist
as dispatched sub-binary subcommands, calling the GitHub REST API in-process
via `octocrab`. All three call sites in `skills/github/` use them instead of
the bash scripts. The two bash scripts and their three test suites are
deleted; `_EXPECTED_GITHUB_SUITES` is 0. `mise run` is green.

**Verification:**

```bash
mise run cli:check
mise run test:unit:cli
mise run build-system:check
mise run test:integration:github   # exits 0 with zero discovered suites
mise run check
```

Manually: run `accelerator collaboration base-repo <a-real-pr-number>` and
`accelerator collaboration update-body <pr> --body-file <a-file>` against a
real repository with `github.token` configured, and confirm the three
skills' `!`-preprocessor sites resolve without error.

## What We're NOT Doing

- Renaming the `skills/github/**` directory to `skills/collaboration/**` —
  that is work-item:0150's separate, still-in-progress initiative. This
  binary is named `collaboration`; the skill directory stays `github/`.
- Adding a `RemoteTracker`-shaped port for work-item:0194 — `github-issues`
  already being a listed `work.integration` value is a future signal, not
  something this item builds toward.
- Building a general-purpose GitHub API client — only the two operations
  the two bash scripts perform (PR-base-repo resolution, PR-body PATCH).
- Content-validating the resolved token for shell-hostile characters (the
  guard `linear-auth.sh` has, for its `curl --config -` embedding) — this
  binary is an in-process HTTP client library, not a shelled `curl`
  invocation, so token content never crosses a shell boundary.
- Porting `test-pr-base-repo-real-gh.sh` (the real-`gh` smoke test guarding
  work-item:0071's regression) — it exists only to probe `gh`'s `--json`
  field allowlist, which is moot once no `gh --json` call remains.
- Retrofitting `vcs`'s existing `bash-parity`/`Matrix` fixture machinery for
  the new `OriginRemote` port — the AC only requires unit-level coverage of
  the four URL forms; a lighter, purpose-built fixture set is used instead
  (see Phase 2).
- Changing `work-item:0169`/`work-item:0188`'s own in-flight `vcs`/
  `vcs-adapters` work — this item only adds one small, narrowly-scoped
  port alongside the existing ones.

## Implementation Approach

Build bottom-up, proving each layer's logic with unit tests before the layer
above depends on it, and land each phase as an independently mergeable,
`mise run`-green change:

1. The config catalogue addition (no resolution logic, just names) plus the
   personal-config-file permission enforcement (access-control logic, but
   self-contained in `config`/`config-adapters` with no dependency on
   anything else this item adds) first, since nothing else depends on
   either but they unblock nothing being blocked later.
2. The `vcs` origin-remote capability next, in isolation — it is usable and
   testable with zero knowledge of GitHub or `collaboration`.
3. The `collaboration` domain crate's pure composition logic, proven against
   hand-written fakes for both the new `vcs` port and the new GitHub ports —
   this is where the 10 domain-level "characterization, restated as
   target Rust behaviour" branches (of 13 total; the remaining 3 are
   CLI-layer, covered in Phase 6) get their first, cheapest test coverage.
4. The `collaboration-adapters` crate, wiring `octocrab` for real, proven
   against a hand-rolled mock HTTP server.
5. The `github.*` credential resolver, as its own focused unit.
6. The `collaboration-cli` sub-binary itself — the checklist's landing-
   together items, plus the full skill call-site rewrite (both must land
   together per the dispatch-coherence guard).
7. Deletion of the now-dead bash scripts and their suites, with the CI floor
   decremented to 0.
8. User docs.

## Phase 1: Config Catalogue & Personal-File Permission Enforcement

### Overview

Two independent, self-contained additions to `config`/`config-adapters`,
landed together since neither depends on anything else this item adds:

1. Add the two `github.*` key names to the `EXTRA_KEYS` catalogue (Rust and
   its bash mirror), matching the existing `linear.*` bare-pair shape. This
   carries no resolution logic — `EXTRA_KEYS` is a name-only registry
   (`cli/config/src/catalogue.rs:116-120`) — but it makes the keys visible
   to `config dump`/`config get` and is a precondition for Phase 5's
   resolver.
2. Enforce that a personal-level config file (`config.local.md`) is only
   ever read when its permissions are 0600 or stricter, and is never a
   symlink — encapsulated entirely inside `FileConfigStore`
   (`cli/config-adapters/src/store.rs`), so every current and future
   `Level::Personal` read benefits, not just `github.token`/
   `github.token_cmd`. This is the Rust-native equivalent of the
   permission check `jira-auth.sh`/`linear-auth.sh` perform
   (`skills/integrations/jira/scripts/jira-auth.sh:184-219`) before
   Phase 5's resolver — the first Rust code path to actually resolve and
   use a live secret end-to-end — needs it to exist. Unlike the bash
   precedent, there is no bypass gate (no `ACCELERATOR_ALLOW_INSECURE_LOCAL`
   equivalent): `WriteConfigLevel::write` already unconditionally clamps a
   personal-level write to mode 0600 (`store.rs:227`, confirmed by the
   existing test `a_personal_write_clamps_a_preexisting_wider_mode`,
   `store.rs:1004-1027`), so any file created via `accelerator config set
   --local` or the `configure` skill already starts compliant — the
   read-time check can only ever trip on external tampering (a manual
   `chmod`, extracting from a backup/tarball, a stray umask on an
   unrelated tool), and the correct remedy is always "fix the
   permissions," never "bypass the check."

### Changes Required:

#### 1. Rust catalogue

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add `"github.token"` and `"github.token_cmd"` to `EXTRA_KEYS`.

```rust
pub const EXTRA_KEYS: &[&str] = &[
    "jira.site",
    "jira.email",
    "jira.token",
    "jira.token_cmd",
    "linear.token",
    "linear.token_cmd",
    "github.token",
    "github.token_cmd",
    "visualiser.editor",
    "visualiser.editor_project",
    "visualiser.binary",
];
```

Add a test in `cli/config/src/catalogue.rs`'s `mod tests` (following the
existing `EXTRA_KEYS`-adjacent tests' style) asserting `EXTRA_KEYS` contains
both new keys.

#### 2. Bash mirror

**File**: `scripts/config-defaults.sh`
**Changes**: Add the same two entries to the bash `EXTRA_KEYS` array
(lines 208-214), immediately after the `linear.*` pair, matching the
existing ordering.

#### 3. Personal-config-file permission enforcement

**Breaking-change notice**: this enforcement applies to **every**
`Level::Personal` read in the workspace, not just `github.token`/
`github.token_cmd` — deliberately, per the design decision behind this
phase (a personal config file is treated as sensitive-by-convention as a
whole, matching how SSH keys/`.netrc`/`.pgpass` are handled, rather than
gating enforcement per-key). This is a genuine behaviour change for any
existing `config.local.md` that predates this feature and has
looser-than-0600 permissions or is a symlink (plausible via dotfile
managers like chezmoi/stow, a tarball/backup restore that resets modes to
umask defaults, or a hand-created file that never went through
`accelerator config set --local`) — such a file, and every command that
reads any personal-level value from it (not just GitHub-related commands),
starts failing on upgrade with no bypass. This is intentional, not an
oversight: see Migration Notes for the documentation/remediation this
requires, and Phase 8 item 4 for the corresponding docs-site update.

**File**: `cli/config/src/error.rs`
**Changes**: A new `ConfigError` variant carrying the offending path and
observed mode, e.g. `ConfigError::InsecurePersonalPermissions { path:
PathBuf, mode: u32 }`, surfaced through `ConfigAccess::get`/`effective`
exactly like any other read failure, with a message naming the required
fix (`chmod 600 <path>`). Classify it in `ConfigError::is_refusal()`'s
exhaustive match — this is a caller-fixable local-environment problem
(exit code 2 at the CLI boundary), the same category as `Invalid`/
`PluginRootUnavailable`, not an internal failure.

**File**: `cli/config-adapters/src/store.rs`
**Changes**: `FileConfigStore` gains a private helper (e.g.
`fn require_secure_personal_file(&self, path: &Path) -> Result<(),
ConfigError>`) performing the check once: `fs::symlink_metadata` the
resolved path (reusing the existing, currently-private `level_path`); if
the path is a symlink, or its mode has any group/other bits set (stricter
than 0600 is fine; looser is not), return the new `ConfigError`. A missing
personal file is unaffected — no file means no personal-level values,
exactly as today, not an error.

Both of `FileConfigStore`'s `Level::Personal`-reading paths call this
helper before touching file content — not just `ReadConfigLevel::read`
(around `store.rs:187-206`, the structured key/value path), but also
`ReadContent::config_body` (around `store.rs:236-241`, which reads
`config.local.md`'s markdown body directly and is already used today by
`cli/launcher/src/config_command/core/context.rs`'s `project_body` and
`core/summary.rs`'s `has_project_context` to inject personal project
context into skill prompts). Missing this second call site would leave a
real gap: personal-config *body* content — documented as freeform prose,
not just structured keys — would bypass the permission check entirely
even after this phase lands. `Level::Team` reads on both paths are
untouched (team config is checked-in and shared; enforcement is
specifically about a file that may hold locally-resolved secrets on a
single machine).

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:cli` (new `EXTRA_KEYS` test;
      new `FileConfigStore` tests covering **both** `ReadConfigLevel::read`
      and `ReadContent::config_body`: a `Level::Personal` read at exactly
      0600 succeeds, a read at a stricter-than-0600 mode (e.g. 0400) also
      succeeds, a read at a looser mode fails with the new `ConfigError`
      variant, a symlinked personal file fails the same way, a missing
      personal file is unaffected, and `Level::Team` reads are unaffected
      regardless of mode)
- [x] Component check passes: `mise run cli:check`
- [x] Config drift/dump behaviour unaffected:
      `mise run test:integration:config`

#### Manual Verification:

- [x] `accelerator config dump` shows `github.token`/`github.token_cmd` as
      `*(set — hidden)*` once a value is configured, and unset otherwise
      (the existing leaf-name-based hiding in
      `cli/launcher/src/config_command/core/dump.rs`'s `extra_row` applies
      automatically — no code change needed there, only verify it).
- [x] Setting a value via `accelerator config set --local` produces a
      `config.local.md` at mode 0600 (already true today — verify, not
      implement), and any `accelerator config`/dispatched sub-binary
      command that reads it succeeds.
- [x] Manually widening `config.local.md`'s permissions (e.g. `chmod 640`)
      and confirming any command that reads personal-level config fails
      clearly, naming the required `chmod 600` fix; restoring 0600
      resolves it.

---

## Phase 2: `vcs`/`vcs-adapters` — Origin-Remote-URL Parsing

### Overview

Add a new, narrow `OriginRemote` port to the `vcs` domain crate and a pure
URL-to-owner/repo parser, then satisfy the port in both `vcs-adapters`
backends. This is usable and independently testable without any knowledge
of `collaboration` or GitHub specifics — it is the local-repository-facts
capability the resolver subcommand needs before making any network call.

### Changes Required:

#### 1. Domain port and parser

**File**: `cli/vcs/src/origin_remote.rs` (new)
**Changes**: A new module alongside `checkout.rs`/`mode.rs`, following the
`CheckoutProbe` narrowed-port pattern (`cli/vcs/src/classify.rs:46-76`).

```rust
/// Reads a repository's configured `origin` remote URL.
pub trait OriginRemote {
    /// The `origin` remote's URL, `Ok(None)` when no `origin` remote is
    /// configured, or `Err` when the probe itself could not answer —
    /// callers must be able to distinguish "cleanly absent" from
    /// "probe malfunctioned", unlike `RepoRoot`/`VcsProbe`'s infallible
    /// fold-to-`None` convention.
    ///
    /// # Errors
    ///
    /// When the repository cannot be opened or its remote configuration
    /// cannot be read.
    fn origin_url(
        &self,
        root: &Path,
    ) -> Result<Option<String>, kernel::Error>;
}

/// The owner and repository name parsed from a GitHub remote URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerRepo {
    pub owner: String,
    pub repo: String,
}

/// Parses a GitHub remote URL into its owner/repo pair.
///
/// Supports the four forms GitHub itself documents:
/// `https://github.com/{owner}/{repo}.git`,
/// `https://github.com/{owner}/{repo}`,
/// `git@github.com:{owner}/{repo}.git`,
/// `ssh://git@github.com/{owner}/{repo}.git`.
///
/// `None` for any URL not matching one of these four shapes.
#[must_use]
pub fn parse_github_remote_url(url: &str) -> Option<OwnerRepo> { .. }

/// The local repository's owner/repo, resolved from its `origin` remote.
///
/// `Err` when no `origin` remote is configured (the caller-facing "no
/// default remote repository" case), when the probe itself fails, or when
/// the configured URL does not match a supported GitHub remote shape.
///
/// # Errors
///
/// See above.
pub fn resolve_origin_owner_repo(
    root: &Path,
    remote: &dyn OriginRemote,
) -> Result<OwnerRepo, kernel::Error> { .. }
```

`kernel::Error::Refusal` is used for the user-facing "no `origin` remote
configured" and "unsupported remote URL shape" cases (they map to exit code
2 at the CLI boundary, matching `vcs-cli`'s existing Refusal convention);
probe malfunctions use `kernel::Error::Failed`.

#### 2. Subprocess adapter

**File**: `cli/vcs-adapters/src/subprocess.rs`
**Changes**: A new `CommandProbe`-adjacent impl of `OriginRemote`, reusing
`run_capped`/`scrub_environment` (lines 220-302) to run
`git remote get-url origin`. Unlike the existing `VcsProbe`/`RepoRoot`
impls, failures here propagate as `Err` rather than folding to `None` —
deliberately breaking from this adapter's usual convention, per the port's
own contract above. A missing remote (`git remote get-url` exits non-zero
with "No such remote") is distinguished from a probe failure by exit code/
stderr shape.

#### 3. Library (`gix`) adapter

**File**: `cli/vcs-adapters/src/library.rs`
**Changes**: A new `OriginRemote` impl using the same
`repository.config_snapshot()` mechanism `git_user_name` already uses
(lines 488-505), reading `remote.origin.url` instead of `user.name`.
`git_user_name` itself folds a `gix::discover` failure to `None`
(warn-logged) — this is **not** the convention to copy here: like the
subprocess adapter above, a `gix::discover`/config-read failure in this
impl must propagate as `Err`, not fold to `Ok(None)`, per `OriginRemote`'s
own contract (`Ok(None)` means cleanly absent, `Err` means the probe
itself malfunctioned). Implementing this by direct analogy to
`git_user_name` without this correction would silently present a broken
repository (permissions, corruption) as "no origin remote configured" —
a misleading error pointing the user at the wrong fix — and would also
diverge from the subprocess adapter's behaviour for the same underlying
failure, defeating the point of the two backends being interchangeable.

### Success Criteria:

#### Automated Verification:

- [x] Domain-level unit tests pass (TDD: write first) —
      `parse_github_remote_url` covers all four supported forms plus
      rejection of an unsupported shape; `resolve_origin_owner_repo`
      covers "no origin configured" (`Ok(None)` from the port → `Err`),
      "probe failure" (`Err` from the port → propagated), and success, via
      a hand-written `StubProbe` (mirroring `classify.rs`'s pattern):
      `cargo test -p vcs`
- [x] Adapter-level tests pass for both backends against real fixture
      repos with an `origin` remote set to each of the four URL forms, and
      one with no `origin` remote configured:
      `cargo test -p vcs-adapters`
- [x] For **both** backends, a test against an inaccessible/broken
      repository fixture (e.g. a directory with no `.git` at all, or one
      with unreadable permissions) confirms `origin_url` returns `Err`,
      not `Ok(None)` — this is the case the library (`gix`) adapter's
      "don't copy `git_user_name`'s fold-to-`None`" note above exists to
      guard, and needs its own explicit test rather than relying on the
      "no origin remote configured" fixture (a valid repo genuinely
      missing the remote) to exercise it, since the two are different
      code paths
- [x] Component check passes: `mise run cli:check`

#### Manual Verification:

- [x] In a real checkout with an `origin` remote, a small ad-hoc call
      through both adapters resolves the same owner/repo pair.

---

## Phase 3: `collaboration` Domain Crate

### Overview

The pure logic crate: three GitHub ports (mockable, no HTTP types), and the
composition functions the two subcommands will call. This is where the
characterization-tests-restated-as-target-Rust-behaviour checklist (13
branches: 8 resolver + 5 body-update) gets its first coverage, entirely
against hand-written fakes — no real HTTP, no real git, following the
house style (`cli/vcs/src/lib.rs:122-273`, `cli/vcs/src/classify.rs:188-600`)
of no mocking framework anywhere in this workspace.

### Changes Required:

#### 1. Crate scaffold

**File**: `cli/collaboration/Cargo.toml` (new)
**Changes**: Bare package name `collaboration`, depends only on `kernel`
and `vcs` (for `OriginRemote`/`resolve_origin_owner_repo`/`OwnerRepo`),
matching `vcs`'s and `work`'s domain-crate dependency shape.

#### 2. Ports

**File**: `cli/collaboration/src/lib.rs` (new)
**Changes**:

```rust
/// A repository's metadata, as returned by the GitHub REST API's
/// repository-get endpoint — specifically whether it has a `parent`
/// (i.e. is a fork), and if so, the parent's owner/repo.
pub trait RepositoryLookup {
    /// # Errors
    ///
    /// [`GitHubApiError`] describing why the lookup failed.
    fn repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<RepositoryDetails, GitHubApiError>;
}

/// Confirms a pull request exists at a given repository — used only to
/// validate the resolved base repo actually holds the PR, never to derive
/// owner/repo from its response (see Key Discoveries for why deriving
/// owner/repo from this endpoint's response is circular).
pub trait PullRequestExistence {
    /// # Errors
    ///
    /// [`GitHubApiError`] describing why the check failed, including a
    /// REST 404 when the PR is not at this repository.
    fn confirm_exists(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<(), GitHubApiError>;
}

/// Updates a pull request's body.
pub trait PullRequestBodyUpdate {
    /// # Errors
    ///
    /// [`GitHubApiError`] describing why the update failed.
    fn update_body(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        body: &str,
    ) -> Result<(), GitHubApiError>;
}

/// The subset of a GET repository response this domain needs: the raw
/// parent owner/name, still unvalidated — a present-but-incomplete parent
/// (one field set, the other missing) is this crate's job to reject as
/// [`BaseRepoFailure::MalformedParent`], not the adapter's.
pub struct RepositoryDetails {
    pub parent_owner: Option<String>,
    pub parent_repo: Option<String>,
}

/// Why a GitHub REST call did not succeed, carrying enough for the AC's
/// "non-zero exit code, with the REST error's status code and message on
/// stderr" requirement.
///
/// `Malformed` is a distinct variant from `Transport` — not merged into
/// it — specifically so `BaseRepoFailure::MalformedRepositoryResponse`
/// (branch 4) is constructible: a response that deserializes into the
/// wrong shape is a materially different failure from a connection
/// dropping or timing out, and collapsing the two would make that branch
/// dead code no test could ever exercise.
pub enum GitHubApiError {
    Transport(String),
    Malformed(String),
    Status { code: u16, message: String },
}
```

#### 3. Composition functions

**File**: `cli/collaboration/src/base_repo.rs` (new)
**Changes**: The resolver subcommand's pure logic — all 8 characterization
branches restated as target behaviour. `BaseRepoFailure` deliberately
excludes a "resolved" arm (unlike an earlier draft's `BaseRepoOutcome`,
which — since `update_body.rs` wrapped the whole enum — could represent
the nonsensical state of a "failure" holding a successful resolution);
`BaseRepoOutcome` composes `Resolved` and `Failed(BaseRepoFailure)`
instead, and `update_body.rs` embeds `BaseRepoFailure` directly:

```rust
pub enum BaseRepoOutcome {
    Resolved(vcs::OwnerRepo),
    Failed(BaseRepoFailure),
}

pub enum BaseRepoFailure {
    RepositoryLookupFailed(GitHubApiError),   // branch 3 (Transport/Status)
    MalformedRepositoryResponse(String),       // branch 4 (from GitHubApiError::Malformed)
    MalformedParent,                           // branch 5
    PullRequestLookupFailed(GitHubApiError),   // branch 6
}

/// # Errors
///
/// [`kernel::Error`] when the local repository's own `origin` remote
/// cannot be resolved (branch 2 — a `vcs`-level failure, resolved before
/// any network call).
pub fn resolve_base_repository(
    root: &Path,
    origin_remote: &dyn vcs::OriginRemote,
    repository_lookup: &dyn RepositoryLookup,
    pull_request_existence: &dyn PullRequestExistence,
    pull_number: u64,
) -> Result<BaseRepoOutcome, kernel::Error> { .. }
```

Resolution order: parse `origin` via `origin_remote` (branch 2 on
failure); call `repository_lookup.repository(...)` against the parsed
candidate — on `Err(GitHubApiError::Malformed(message))`, branch 4
(`MalformedRepositoryResponse(message)`); on any other `Err`, branch 3
(`RepositoryLookupFailed`, carrying the `Transport`/`Status` error as-is).
This is why `GitHubApiError` has a distinct `Malformed` variant rather
than folding deserialization failures into `Transport` (see its doc
comment) — without that distinction, branch 4 could never be reached from
branch 3's catch-all. If `parent_owner`/`parent_repo` are both present,
use them as the base — if exactly one is present,
`BaseRepoFailure::MalformedParent` (branch 5); otherwise the candidate is
its own base; call `pull_request_existence.confirm_exists(...)` against
the resolved base (branch 6 on failure, including a REST 404 when the PR
is not at the resolved base); success is `Resolved` with either the
candidate (branch 7, no parent) or the parent (branch 8, fork resolved).

(Branch 1, invalid CLI invocation, is covered at the `collaboration-cli`
layer in Phase 6 — clap handles it structurally.)

**File**: `cli/collaboration/src/update_body.rs` (new)
**Changes**: The body-update subcommand's pure logic — the 5
characterization branches:

```rust
pub enum UpdateBodyOutcome {
    Updated,
    BaseRepoResolutionFailed(BaseRepoFailure), // branch 3, propagated in-process
    PatchFailed(GitHubApiError),               // branch 4
}

pub fn update_pull_request_body(
    root: &Path,
    origin_remote: &dyn vcs::OriginRemote,
    repository_lookup: &dyn RepositoryLookup,
    pull_request_existence: &dyn PullRequestExistence,
    body_update: &dyn PullRequestBodyUpdate,
    pull_number: u64,
    body: &str,
) -> Result<UpdateBodyOutcome, kernel::Error> { .. }
```

`update_pull_request_body` calls `resolve_base_repository` internally;
its `Failed(failure)` arm becomes this function's own branch-3
`BaseRepoResolutionFailed(failure)` directly (no wrapping-of-a-wrapper,
since `BaseRepoFailure` already excludes the impossible "failed but
resolved" state — this single branch-3 case covers any of the resolver's
own branches 2-6, matching the source script's subprocess exit-code
propagation from calling `pr-base-repo.sh`), and `resolve_base_repository`'s
own outer `kernel::Error` (its branch 2, missing origin remote) simply
propagates via `?`, unchanged.

(Branch 1, usage error, and branch 2, missing/unreadable `--body-file`, are
CLI-layer concerns — reading the file is I/O, done in `collaboration-cli`
before this function is called, per the ports-and-adapters convention of
keeping the domain crate free of concrete I/O.)

### Success Criteria:

#### Automated Verification:

- [x] TDD: write each characterization test first (red), then the minimum
      code to pass it (green). **10 of the 13 characterization branches are
      domain-level** and covered here against hand-written fakes
      (`FixedOriginRemote`, `FixedRepositoryLookup`,
      `FixedPullRequestExistence`, `FixedBodyUpdate`, mirroring
      `vcs/src/lib.rs`'s `FixedRoot`/`FixedProbe` pattern):
      `cargo test -p collaboration`. The remaining 3 (resolver branch 1;
      body-update branches 1 and 2) are CLI-layer concerns per this
      phase's own text above — they are **not** covered by this command;
      see Phase 6's Success Criteria for where each of the 3 is actually
      tested. (The prior draft of this bullet claimed "all 13" here,
      which contradicted the branch-1/branch-2 CLI-layer notes already in
      this phase's Composition-functions text — corrected.)
- [x] Component check passes: `mise run cli:check`

#### Manual Verification:

- [x] None — this crate has no I/O and no CLI surface yet.

---

## Phase 4: `collaboration-adapters`

### Overview

The real `octocrab`-backed implementations of `RepositoryLookup`,
`PullRequestExistence`, and `PullRequestBodyUpdate`, plus the
`octocrab`/`tokio` dependency additions.
Proven against a hand-rolled mock HTTP server, mirroring
`cli/launcher/tests/common/mod.rs`'s `MockServer` — the established pattern
for HTTP-level test stubbing in this workspace (no `wiremock`/`mockito`).

### Changes Required:

#### 1. Workspace dependency additions

**File**: `cli/Cargo.toml`
**Changes**: Add to `[workspace.dependencies]`:

```toml
# Exact-pinned (matching this workspace's convention for behaviour-
# sensitive dependencies — clap, reqwest, rustls, serde-saphyr — rather
# than a floating range) since octocrab is a young, actively-evolving
# crate whose builder API (with_service/with_layer, the mock-server test
# seam) and Error enum shape (the error-mapping code below depends on
# octocrab::Error::GitHub's exact fields) this plan's own code and tests
# assume. 0.54.1 is the latest stable release as of this plan's writing;
# confirm it's still current before implementing and bump deliberately,
# not implicitly via a `cargo update`.
# rustls-ring mirrors the workspace's existing reqwest/rustls pin's TLS
# backend; default-features = false + rustls-ring avoids opentls, which
# would violate deny.toml's native-tls ban. `timeout` is added explicitly
# (default-features = false disables it, along with retry/follow-redirect,
# same as everything else octocrab enables by default) so
# set_connect_timeout/set_read_timeout/set_write_timeout are available —
# see Phase 4 item 3 for why `follow-redirect` is deliberately left out.
octocrab = { version = "=0.54.1", default-features = false, features = ["rustls-ring", "timeout"] }
# Minimal runtime: collaboration-cli is octocrab's only consumer of an
# async runtime in this workspace; rt for the current-thread executor, net
# for the HTTP client's sockets, time for backoff/timeouts.
tokio = { version = "1", default-features = false, features = ["rt", "net", "time"] }
```

Add `collaboration`, `collaboration-adapters` to `[workspace].members`
(`collaboration-cli` is added in Phase 6, alongside the checklist's other
landing-together items).

Run `mise run deny:check` immediately after adding `octocrab` and react to
whatever it reports (checklist item 13) — the research identified this as
the single most concrete risk, since `octocrab`'s dependency graph is
unverified against this workspace's exact ban list until checked.

**Accepted limitation**: `cli/deny.toml`'s `bans.multiple-versions` is set
to `"warn"`, not `"deny"` — if `octocrab` 0.54.1 pulls in a second
`reqwest`/`rustls` version line alongside the workspace's own exact pin,
`deny:check` prints a warning but still exits 0, so it will not fail CI
on a genuine duplication. Reading its warning output at implementation
time (not just checking the exit code) is required to actually catch
this; running `cargo tree -p collaboration-adapters -i reqwest` (or
equivalent) alongside `deny:check` is the more reliable confirmation.

#### 2. Adapter crate scaffold

**File**: `cli/collaboration-adapters/Cargo.toml` (new)
**Changes**: Depends on `collaboration`, `octocrab`, `tokio`, `tracing`,
matching `vcs-adapters`'/`work-adapters`' dependency shape (real I/O
confined here, never in the domain crate).

#### 3. The adapter

**File**: `cli/collaboration-adapters/src/octocrab_client.rs` (new)
**Changes**: A struct wrapping an `octocrab::Octocrab` instance, providing
the behaviour of `RepositoryLookup`, `PullRequestExistence`, and
`PullRequestBodyUpdate` as `async fn`s under an inherent `impl` (the trait
methods themselves stay synchronous at the domain-port boundary — see
Phase 6 for the `BlockingGitHubClient` shim that bridges each sync port
method to one of these `async fn`s via its own, non-nested `block_on`
call; the simplest option, consistent with keeping I/O concrete-typed
only in this adapter, is for this crate to expose `async fn`-returning
inherent methods that the shim awaits, rather than trying to implement
the domain crate's synchronous trait signatures here directly).

```rust
pub struct OctocrabClient {
    client: octocrab::Octocrab,
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

impl OctocrabClient {
    /// Production client, authenticated with `token`, talking to
    /// `https://api.github.com`. Built with `set_connect_timeout`/
    /// `set_read_timeout`/`set_write_timeout` (mirroring
    /// `cli/launcher`'s `Fetcher` connect/total-timeout precedent, sized
    /// down since these are small JSON request/response bodies, not
    /// multi-MB binary downloads) so a stalled or maliciously slow
    /// connection cannot hang the CLI indefinitely. `follow-redirect` is
    /// deliberately excluded from this crate's `octocrab` feature set
    /// (Phase 4 item 1) — the three REST paths this client calls are
    /// fixed and don't legitimately redirect, so an unexpected redirect
    /// is surfaced as a REST-shaped error rather than silently followed,
    /// which also forecloses a redirect ever carrying the `Authorization`
    /// header to an unintended host.
    ///
    /// Accepted trade-off: GitHub's REST API does redirect `GET
    /// /repos/{owner}/{repo}` for a repository that has since been
    /// renamed or transferred, and `gh` (which the bash scripts this
    /// binary replaces shelled out to) follows such redirects
    /// transparently. A local checkout whose `origin` points at a
    /// pre-rename URL will now hard-fail with a REST-shaped error
    /// (naming the renamed-repo redirect) rather than silently
    /// succeeding against the new location — a narrow behavioural
    /// regression versus the replaced scripts, judged an acceptable cost
    /// for closing the credential-redirect risk above. The remediation
    /// (update the `origin` remote URL) is the same fix a stale remote
    /// needs regardless.
    pub fn new(token: String) -> Result<Self, String> { .. }

    /// Test client pointed at a local mock server via `BaseUriLayer`,
    /// unauthenticated or with a fixed token. Deliberately **not**
    /// `#[cfg(test)]`-gated: the mock-server tests that call this live in
    /// `tests/common/mod.rs`, a separate integration-test binary that
    /// links this crate as an ordinary external dependency and cannot see
    /// `#[cfg(test)]` items — mirroring `Fetcher::with_backoff`
    /// (`cli/launcher/src/launch/outbound/resolve/fetcher.rs`), the
    /// established precedent for this exact situation. Not exported from
    /// the crate's public docs-facing surface beyond that: it is `pub` for
    /// visibility to `tests/`, not for external consumers.
    pub fn with_base_uri(base_uri: http::Uri, token: Option<String>) -> Result<Self, String> { .. }

    /// Repository metadata, including its `parent` (if it is a fork).
    pub async fn repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<collaboration::RepositoryDetails, collaboration::GitHubApiError> { .. }

    /// Confirms a pull request exists at `owner/repo` — the response body
    /// is not parsed for any field beyond a successful status; see Key
    /// Discoveries for why deriving owner/repo from this response would be
    /// circular.
    pub async fn confirm_pull_request_exists(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<(), collaboration::GitHubApiError> { .. }

    pub async fn update_body(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        body: &str,
    ) -> Result<(), collaboration::GitHubApiError> { .. }
}
```

Errors from `octocrab::Error::GitHub { source, .. }` map to
`GitHubApiError::Status { code: source.status_code.as_u16(), message:
source.message }`. `octocrab::Error::Serde { .. }` and `::Json { .. }`
(octocrab 0.54.1's two deserialization-specific variants — confirmed
against its own `Error` enum, which also has separate `Hyper`/`Http`/
`Service`/`Uri`/`UriParse` transport-and-below variants) map to
`GitHubApiError::Malformed(error.to_string())` — this is what makes
`BaseRepoFailure::MalformedRepositoryResponse` (resolver branch 4)
constructible; every other `octocrab::Error` variant (network/transport
failures below the HTTP layer) maps to `GitHubApiError::Transport
(error.to_string())`.

#### 4. Mock server test support

**File**: `cli/collaboration-adapters/tests/common/mod.rs` (new)
**Changes**: A minimal `std`-only loopback `TcpListener` mock server, sized
to this crate's actual needs (a `GET`/`PATCH` JSON responder is enough —
this need not carry `Fetcher`'s `FlakyThenOk` variant, which exists for
the launcher's own retry logic; this client has no hand-rolled retry
logic to test, since `retry` is not in its `octocrab` feature set). A
`Stall` variant (accept the connection, never write a response) is worth
including if a timeout test is added — the risk here is lower than
`Fetcher`'s, since `set_read_timeout`/`set_write_timeout` are
library-provided behaviour rather than hand-rolled, but a single test
confirming the configured timeout actually fires (using a short
test-only override rather than the production 30s value) is cheap
insurance. Ported from `cli/launcher/tests/common/mod.rs`'s structure,
not imported as a shared crate (no existing shared test-support crate
covers this; introducing one is out of scope for this item).

Unlike `cli/launcher/tests/common/mod.rs`'s `MockServer` (which discards
request headers entirely), this mock server records the `Authorization`
header of each received request, exposed to the test after the request
completes — needed so an adapter test can assert the configured token
actually reached the outbound request (see Success Criteria), proving
credential resolution and HTTP dispatch compose correctly rather than
only being tested in isolation from each other.

### Success Criteria:

#### Automated Verification:

- [x] TDD: write adapter tests first for the branches genuinely HTTP-shaped
      (resolver branches 3 and 6 — repository-lookup and PR-existence-check
      request failure/non-2xx; branch 4 in body-update — PATCH failure; all
      three also verifying the AC's fail-fast status-code +
      message-on-stderr surfacing), then the adapter code:
      `cargo test -p collaboration-adapters`
- [x] `mise run deny:check` passes with `octocrab` in the graph, including
      the `timeout` feature (react to any reported issue before
      proceeding — checklist item 13)
- [x] A test confirms `OctocrabClient` does not follow a redirect response
      (e.g. the mock server responds `3xx` with a `Location` header; the
      client surfaces this as a `GitHubApiError`, not a followed request)
- [x] A test confirms the token passed to `OctocrabClient::with_base_uri`
      reaches the outbound request as an `Authorization` header, asserted
      against the mock server's recorded header — proving credential
      resolution and HTTP dispatch compose correctly, not just each in
      isolation (addresses the AC's "Authentication" criterion, which
      Phase 5's precedence tests alone do not cover end-to-end)
- [x] Component check passes: `mise run cli:check`

#### Manual Verification:

- [x] None — no CLI surface yet; covered end-to-end in Phase 6.

---

## Phase 5: `github.*` Credential Resolver

### Overview

A `collaboration-cli`-local module implementing the four-way precedence
chain the work item specifies (config `github.token` → `github.token_cmd`
output → `GH_TOKEN` env → `GITHUB_TOKEN` env), plus the shared-config
`token_cmd` ban already established for `jira`/`linear` in bash. This
precedence order (config-first) is a **deliberate departure** from
`jira-auth.sh`/`linear-auth.sh`'s env-first order — stated explicitly here
so it is not "corrected" to match them later. Unlike `jira-auth.sh`/
`linear-auth.sh`, this resolver carries no personal-config-file permission
check of its own — Phase 1 encapsulates that inside `FileConfigStore`, so
every `ConfigAccess::get` call this resolver makes against `Level::
Personal` already fails closed on an insecure `config.local.md` before
this module ever sees a value.

### Changes Required:

#### 1. Resolver module

**File**: `cli/collaboration-cli/src/auth.rs` (new)
**Changes**: Following `work-cli/src/config.rs`'s precedent of keeping
config-key-specific helpers CLI-local rather than in the domain/adapters
crate (this resolver is authentication plumbing, not `collaboration`'s core
PR-helper business logic).

```rust
pub enum TokenSource {
    Config,
    ConfigCmd,
    GhTokenEnv,
    GithubTokenEnv,
}

pub struct ResolvedToken {
    pub value: String,
    pub source: TokenSource,
}

/// Resolves the `github.token` credential.
///
/// Precedence: `github.token` config value, then `github.token_cmd`
/// output (executed via `bash -c`), then `GH_TOKEN`, then `GITHUB_TOKEN`.
/// A `token_cmd` configured in the shared/team config file (rather than
/// local overrides) is rejected with a clear error — mirroring
/// `jira`/`linear`'s shared-config `token_cmd` ban — rather than silently
/// executed.
///
/// # Errors
///
/// `kernel::Error::Refusal` when nothing resolves, or when a
/// shared-config `token_cmd` is present.
pub fn resolve_github_token(
    config: &dyn ConfigAccess,
) -> Result<ResolvedToken, kernel::Error> { .. }
```

The shared-vs-local distinction is read via `ConfigAccess`'s existing
`Source`/`Level` machinery (already used by `dump.rs`'s `source_of`,
`cli/launcher/src/config_command/core/dump.rs:92-101`), not by re-parsing
config files directly as the bash resolvers do.

### Success Criteria:

#### Automated Verification:

- [ ] TDD: one test per precedence step, plus the shared-config
      `token_cmd` ban, modelled on `test-jira-auth.sh` Test 6 (the ban) and
      Test 5a (shared token blocked when local file exists but has no
      token entry) as a checklist — not ported, since nothing there is
      directly reusable: `cargo test -p collaboration-cli`
- [ ] Component check passes: `mise run cli:check`

#### Manual Verification:

- [ ] Setting `github.token` in `config.local.md`, then `github.token_cmd`,
      then `GH_TOKEN`, then `GITHUB_TOKEN` individually and confirming each
      resolves in the stated precedence order.
- [ ] Confirming a `github.token_cmd` in the shared `config.md` produces
      the ban error rather than executing.

---

## Phase 6: `collaboration-cli` Sub-binary + Skill Call-site Migration

### Overview

The thin CLI crate itself, wired through the full registration checklist,
**and** the rewrite of all three skills' call sites in the same change —
the dispatch-coherence guard (checklist item 7) requires a real skill
invocation naming the `collaboration` token before it will pass, so this
cannot be split into "register the binary" then "point skills at it" across
two phases without an intermediate broken state.

### Changes Required:

#### 1. Crate scaffold and registration (checklist items 1-5, 8)

**File**: `cli/collaboration-cli/Cargo.toml` (new)
**Changes**: `[package] name = "accelerator-collaboration"` (per
`tasks/README.md`'s checklist: "The Cargo package is `accelerator-
<token>`" — a distinct declaration from `[[bin]] name`, not implied by it;
without it, `package.name` defaults to the crate directory name,
`collaboration-cli`, diverging from `vcs-cli`'s/`work-cli`'s own
manifests, which both set `[package] name` explicitly to their
`accelerator-<token>` form), `[[bin]] name = "accelerator-collaboration"`,
mandatory `package.description`, inherited `version`/`edition`/
`rust-version`/`license`/`publish`, `[lints] workspace = true`. Depends on
`collaboration`, `collaboration-adapters`, `config`, `config-adapters`,
`vcs`, `vcs-adapters`, `clap`, `tokio`.

**File**: `cli/Cargo.toml`
**Changes**: Add `"collaboration-cli"` to `[workspace].members`; regenerate
and commit `Cargo.lock`.

**File**: `tasks/shared/paths.py`
**Changes**: Add `"collaboration"` to `DISPATCHED_SUBBINARIES`.

**File**: `tasks/manifest.py`
**Changes**: Add `"collaboration": CLI_DIR / "collaboration-cli/Cargo.toml"`
to `_SUBBINARY_MANIFESTS` (needed since the binary crate is not at
`cli/collaboration/` — that path is the domain crate).

**File**: `tests/integration/tasks/test_github.py`
**Changes**: Add a `"collaboration"` entry to `_SUBBINARY_DESCRIPTIONS`
(line 35); update
`test_the_dispatched_registry_holds_visualiser_vcs_work_and_corpus` (line
516-521) to assert
`("visualiser", "vcs", "work", "corpus", "collaboration")`, renaming the
test to match. The upload-count assertion (line 341-343) is already a
derived expression and needs no edit.

**File**: `.gitignore`
**Changes**: Add `bin/collaboration-*` alongside the existing
`bin/vcs-*`/`bin/work-*`/`bin/corpus-*` lines.

**File**: `tasks/build.py`
**Changes**: Add `"accelerator-collaboration"` to `_CLI_RELEASE_BINARIES`.

#### 2. CLI surface

**File**: `cli/collaboration-cli/src/cli.rs` (new)
**Changes**: Following `vcs-cli/src/cli.rs`'s shape exactly
(`disable_version_flag = true`, no domain-name repetition in subcommand
names):

```rust
#[derive(Parser)]
#[command(name = "accelerator-collaboration", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Resolve a PR's base (upstream) owner/repo; prints "<owner>/<repo>".
    BaseRepo { pull_number: u64 },
    /// Update a PR's body from a file.
    UpdateBody {
        pull_number: u64,
        #[arg(long)]
        body_file: PathBuf,
    },
}
```

#### 3. `main.rs`

**File**: `cli/collaboration-cli/src/main.rs` (new)
**Changes**: Following `vcs-cli`/`work-cli`'s composition pattern —
`compose(&start, LegacyPolicy::Reject)?` for config, `MarkerWalkRoot`/
`InProcessProbe` (or the `subprocess` equivalent — matching whichever
`vcs`/`vcs-adapters` backend is already the default for CLI consumers) for
`OriginRemote`, `auth::resolve_github_token` for the credential.

The domain crate's ports (`RepositoryLookup`, `PullRequestExistence`,
`PullRequestBodyUpdate`) are synchronous, but `OctocrabClient`'s methods
are `async fn`s (Phase 4). A single top-level `block_on` wrapping the
*entire* call to `resolve_base_repository`/`update_pull_request_body`
does not work here: if the async block driven by that `block_on` then
invoked a synchronous port method that itself called `block_on` again to
reach `OctocrabClient`, that would be a nested runtime-enter on the same
thread, which Tokio panics on ("Cannot start a runtime from within a
runtime"). Instead, a small blocking shim built once in `main()` bridges
the boundary — each of its trait methods does its own, non-nested
`block_on` call, and `main()` itself calls the (fully synchronous)
composition functions directly, never entering a runtime itself:

```rust
struct BlockingGitHubClient {
    runtime: tokio::runtime::Runtime,
    client: collaboration_adapters::OctocrabClient,
}

impl collaboration::RepositoryLookup for BlockingGitHubClient {
    fn repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<collaboration::RepositoryDetails, collaboration::GitHubApiError> {
        self.runtime.block_on(self.client.repository(owner, repo))
    }
}

impl collaboration::PullRequestExistence for BlockingGitHubClient {
    fn confirm_exists(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<(), collaboration::GitHubApiError> {
        self.runtime.block_on(
            self.client.confirm_pull_request_exists(owner, repo, pull_number),
        )
    }
}

impl collaboration::PullRequestBodyUpdate for BlockingGitHubClient {
    fn update_body(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        body: &str,
    ) -> Result<(), collaboration::GitHubApiError> {
        self.runtime.block_on(
            self.client.update_body(owner, repo, pull_number, body),
        )
    }
}
```

`run_base_repo`/`run_update_body` each build a
`tokio::runtime::Builder::new_current_thread().enable_all().build()`
`Runtime`, construct `OctocrabClient::new(token)`, wrap both in a
`BlockingGitHubClient`, and pass `&blocking_client` wherever
`resolve_base_repository`/`update_pull_request_body` expect a
`&dyn RepositoryLookup`/`&dyn PullRequestExistence`/
`&dyn PullRequestBodyUpdate` — the domain crate's composition functions
are called exactly as ordinary synchronous functions, with no `block_on`
visible at the call site. This keeps `collaboration` itself free of any
`tokio` dependency (its `cargo test -p collaboration` unit tests need no
async runtime at all), confining async entirely to `collaboration-cli`
and `collaboration-adapters` as intended.

```rust
fn report(error: &kernel::Error) -> ExitCode {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("{message}");
    }
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::BaseRepo { pull_number } => run_base_repo(pull_number),
        Command::UpdateBody { pull_number, body_file } =>
            run_update_body(pull_number, &body_file),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}
```

`run_base_repo` prints `owner/repo` to stdout on the `BaseRepoOutcome::
Resolved` arm (preserving the exact stdout contract `review-pr`/
`respond-to-pr` depend on, regardless of whether resolution followed a
`parent` or not). `BaseRepoOutcome::Failed` is matched exhaustively, and
`GitHubApiError` itself renders per-variant rather than one shared format
(`Transport` has no `code` field, so `"{code}: {message}"` only fits
`Status`):
- `GitHubApiError::Status { code, message }` → `"{code}: {message}"`
- `GitHubApiError::Transport(message)` → `"{message}"` alone
- `GitHubApiError::Malformed(message)` → `"{message}"` alone (the same
  shape as `Transport`, since both are already-formatted human-readable
  strings by the time they reach here; only `Status` has a separate
  numeric field to interpolate)

`BaseRepoFailure::RepositoryLookupFailed`/`PullRequestLookupFailed` render
their inner `GitHubApiError` per the above; `MalformedRepositoryResponse
(message)` renders its carried message directly (no longer a bare fixed
string — it now carries the actual deserialization-failure detail, giving
an operator something concrete to act on); `MalformedParent` (which
genuinely carries no further detail — the response parsed fine, the
`parent` shape itself was inconsistent) keeps its own fixed, descriptive
stderr message. All arms map to `kernel::Error::Failed` (exit 1) via
`report`, satisfying the AC's fail-fast REST-error-surfacing requirement
for the `GitHubApiError` arms. `run_update_body` matches `UpdateBodyOutcome`
the same way, with `BaseRepoResolutionFailed(failure)` delegating to the
same `BaseRepoFailure` rendering, and `PatchFailed`'s `GitHubApiError`
rendered identically.

#### 4. Skill call-site rewiring

Every rewritten invocation must be the first non-blank line of its own
single-purpose fenced code block (or the `!`-preprocessor form) — this is
what `tasks/shared/skill_parsing.py`'s `fenced_block_commands()` (the
dispatch-coherence guard's parser) actually recognises as a binding for
the `collaboration` token. A command buried as a later line of a
multi-command block, or as inline single-backtick text outside a fenced
block, is invisible to the guard even though it works fine at runtime —
confirmed by reading the three files' current content below, two of
which need restructuring, not just text substitution, to satisfy this.

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: `allowed-tools` entry
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/*)` →
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator collaboration base-repo *)`.
The numbered step "Fetch additional metadata for the Reviews API"
currently holds the `gh api ... > head-sha.txt` and
`pr-base-repo.sh {number} > ...repo-info.txt` lines in one shared fenced
block, with the resolver call second — **split into two separate fenced
blocks** (one per command) so the rewritten collaboration invocation is
the first line of its own block:
```bash
gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha' > {tmp directory}/pr-review-{number}/head-sha.txt
```
```bash
${CLAUDE_PLUGIN_ROOT}/bin/accelerator collaboration base-repo {number} > {tmp directory}/pr-review-{number}/repo-info.txt
```
Error-handling prose ("If `pr-base-repo.sh` exits non-zero...") updated to
name the new subcommand and its exit-code convention (non-zero = failure,
stderr carries the reason; the `gh repo set-default` remediation hint no
longer applies — replaced by the new resolver's own "no `origin` remote
configured" message); the adjacent, unmodified "No default remote
repository" bullet (about `gh`'s own `gh repo set-default` mechanism) gets
a one-clause disambiguation so the two "no remote" failure modes aren't
conflated.

**File**: `skills/github/respond-to-pr/SKILL.md`
**Changes**: Same `allowed-tools` pattern. The numbered step "Get repo
info and current user" already places
`${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/pr-base-repo.sh {number}` as
the first line of its own fenced block (ahead of `gh api user --jq
'.login'`) — no restructuring needed here, only the text substitution:
→ `${CLAUDE_PLUGIN_ROOT}/bin/accelerator collaboration base-repo
{number}`.

**File**: `skills/github/describe-pr/SKILL.md`
**Changes**: `allowed-tools` entries for both
`skills/github/describe-pr/scripts/*` and `skills/github/scripts/*` are
replaced with a single entry,
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator collaboration update-body *)`
— **not** a `base-repo` entry too: this skill never calls `pr-base-repo.sh`
directly today (confirmed by inspection — only `pr-update-body.sh`, which
internally shells out to `pr-base-repo.sh` as a *subprocess*), and after
migration `update-body`'s base-repo resolution happens in-process inside
the same binary invocation, so there is no separate `base-repo` call for
Claude Code's permission system to see. The numbered step "Post the body
via the helper script" currently holds the invocation as inline
single-backtick text, not inside a fenced block at all — **convert it
into its own fenced block**:
```bash
${CLAUDE_PLUGIN_ROOT}/bin/accelerator collaboration update-body {number} --body-file {tmp directory}/pr-body-{number}.md
```
The exit-code table is rewritten: the four-way 1/2/4/generic table
(specific to the bash scripts' scheme, including the now-nonexistent
distinction between `pr-update-body.sh:`- and `pr-base-repo.sh:`-prefixed
stderr) is replaced with the new binary's two-code convention (2 =
usage/refusal — e.g. missing `--body-file`, no `origin` remote
configured; 1 = any other failure — base-repo resolution failure, body-
file read failure, REST failure), with stderr always carrying which
stage failed.

### Success Criteria:

#### Automated Verification:

- [ ] TDD: `collaboration-cli` argument-parsing/exit-code-mapping unit
      tests (mirroring whatever test shape `vcs-cli`/`work-cli` use for
      their own `main.rs` logic) pass: `cargo test -p collaboration-cli`
- [ ] The 3 CLI-layer characterization branches deferred from
      `collaboration` (resolver branch 1, body-update branches 1 and 2 —
      see Testing Strategy) are each covered by a named test, not folded
      silently into "argument parsing" — in particular, a dedicated test
      for body-update branch 2 (missing/unreadable `--body-file`), which
      no earlier draft of this plan named a test for anywhere
- [ ] An end-to-end `collaboration-cli` test against the Phase 4 mock
      server exercises the full compiled binary for at least one success
      case per subcommand and one representative failure per
      `BaseRepoOutcome`/`UpdateBodyOutcome` variant, asserting the exact
      stdout/stderr text — this is the only test that exercises `main.rs`'s
      error-rendering match arms and the `BlockingGitHubClient` shim
      itself; without it, both are covered only by manual verification
- [ ] `mise run lint:dispatch-coherence:check` passes (validates the
      1↔7 pairing — the token is dispatched and named by a compliant skill
      call site); manually confirm all three rewritten call sites
      (`review-pr`, `respond-to-pr`, `describe-pr`), not just one, are
      each independently guard-visible per the fenced-block-placement
      requirement above — the check itself only needs one compliant site
      per token to pass, so it would not catch the other two regressing
      to a non-visible form later
- [ ] `mise run cli:check` passes (includes the `--locked` `Cargo.lock`
      check)
- [ ] `mise run build-system:check` passes (includes
      `test_the_dispatched_registry_holds_...` and the manifest/paths
      registry tests)
- [ ] `mise run test:integration:skill-invocation` passes (every
      `!`-preprocessor site in the three rewritten SKILL.md files runs in
      production shape)
- [ ] Full local CI mirror: `mise run check`

#### Manual Verification:

- [ ] `accelerator collaboration base-repo <pr>` against a real repo with
      `github.token` configured prints the correct `owner/repo`.
- [ ] `accelerator collaboration update-body <pr> --body-file <path>`
      against a real repo updates the PR body.
- [ ] Run `/review-pr`, `/respond-to-pr`, and `/describe-pr` against a real
      PR end-to-end and confirm each completes without falling back to the
      old scripts (which no longer exist after Phase 7, but should already
      be unused after this phase).

---

## Phase 7: Remove the Legacy Bash Scripts and Suites

### Overview

With no remaining call site referencing the bash scripts (Phase 6 complete
and merged), delete them and their test suites, and decrement the CI floor.
Coordinated in lockstep with work-item:0174 per the work item's Dependencies
section.

The two bash suites (`test-pr-base-repo-scripts.sh`, 15 cases;
`test-pr-update-body-scripts.sh`, 23 cases) are **deleted, not repointed**
to HTTP-level stubbing — a deliberate deviation from the work item's
original "repointed suites" phrasing, not an oversight. Reworking a bash
suite to stand up and assert against a mock HTTP server is disproportionate
next to the Rust-native mock-server tests `collaboration-adapters` already
builds (Phase 4), and largely redundant with them: the 38 old cases
exercise the same branches the 17 new Rust characterization tests cover
(Phase 3), and a meaningful share of the 38 are bash/regex artifacts —
e.g. exhaustively probing malformed-URL shapes against a hand-rolled
regex — that collapse into a single Rust branch once the parser is a
typed function. Coverage is superseded by Phase 3's characterization
tests plus `mise run test:integration:skill-invocation` (Phase 6), not
replaced case-for-case.

### Changes Required:

#### 1. Script removal

**Files removed**: `skills/github/scripts/pr-base-repo.sh`,
`skills/github/describe-pr/scripts/pr-update-body.sh`,
`skills/github/scripts/test-pr-base-repo-scripts.sh`,
`skills/github/scripts/test-pr-base-repo-real-gh.sh`,
`skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`.

A repo-wide grep (`grep -rln 'test-helpers.sh\|install_fake_gh\|
install_fake_jq' skills/github/`) confirms `skills/github/scripts/
test-helpers.sh` is referenced only by the three suites already being
removed above (`test-pr-base-repo-scripts.sh`, `test-pr-base-repo-real-
gh.sh`, and — from the sibling directory —
`describe-pr/scripts/test-pr-update-body-scripts.sh`) and by its own
`README.md`. Nothing else in `skills/github/` depends on it, so
`test-helpers.sh` is removed too, and with every file in
`skills/github/scripts/` gone, **the directory itself — including
`skills/github/scripts/README.md`, which names `pr-base-repo.sh` in its
own prose and would otherwise go stale — is removed as well**. Re-run
the same grep at implementation time in case a sibling in-flight change
has added a new dependent in the interim.

#### 2. CI floor decrement

**File**: `tasks/test/integration.py`
**Changes**: `_EXPECTED_GITHUB_SUITES = 3` → `_EXPECTED_GITHUB_SUITES = 0`
(line 74), matching the `_EXPECTED_DECISIONS_SUITES = 0` precedent (line
73) — the `github` task and `_require_suite_floor` call stay in place with
a zero floor, not removed, mirroring how `decisions` was handled.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:integration:github` passes (exits 0, discovers 0
      shell suites, floor is 0)
- [ ] A repo-wide grep confirms no remaining reference to either removed
      script outside `meta/` planning documents:
      `grep -rn 'pr-base-repo.sh\|pr-update-body.sh' skills/ hooks/ scripts/`
      returns nothing
- [ ] `mise run check`

#### Manual Verification:

- [ ] None beyond Phase 6's — this phase only deletes now-dead code.

---

## Phase 8: User Documentation

### Overview

Checklist item 11 — document the new sub-binary for users. Author-only gate
(nothing in CI catches an omission), independently mergeable after Phase 6.

### Changes Required:

#### 1. Docs site page

**File**: `docs-site/src/content/docs/collaboration.md` (new, following
`docs-site/src/content/docs/visualiser.md`'s shape) — documents the two
subcommands, the `github.token`/`github.token_cmd` config pair and env-var
fallbacks, and the `ACCELERATOR_COLLABORATION_BIN` override.

**File**: `docs-site/astro.config.mjs`
**Changes**: Add the new page to the sidebar so Starlight's prev/next chain
includes it.

#### 2. Root README

**File**: `README.md`
**Changes**: Add an entry to the **Concepts** list under
`## Documentation`.

#### 3. Existing user guide's prerequisites

**File**: `docs-site/src/content/docs/guides/review-a-pr.mdx`
**Changes**: The Prerequisites section currently lists only `gh` CLI
auth (`gh auth login`) and a default remote (`gh repo set-default`) —
accurate for the other `gh` calls `review-pr`/`respond-to-pr`/
`describe-pr` still make (diffs, PR metadata, comments), which are out of
this item's scope, but no longer sufficient on its own: base-repo
resolution and body-update now authenticate independently via
`github.token`/`github.token_cmd`/`GH_TOKEN`/`GITHUB_TOKEN`, which `gh
auth login` does not populate. Add a third bullet naming this
requirement, e.g. "`github.token` (or `github.token_cmd`) configured, or
`GH_TOKEN`/`GITHUB_TOKEN` set — see [the collaboration
docs](../collaboration.md) — for the parts of these workflows that call
the GitHub API directly rather than through `gh`." The existing `gh
repo set-default` bullet is left as-is (still required for the
unmigrated `gh` calls); do not merge or replace it, since the two
mechanisms (a `gh`-specific default-repo setting and this binary's
`origin`-remote-based resolution) are different and conflating them
would misdirect a reader trying to fix the wrong one.

#### 4. Configuration docs — personal-file permission requirement

**File**: `docs-site/src/content/docs/configuration.md`
**Changes**: The "Config Files" table (line 16) already documents
`.accelerator/config.local.md` as "Personal (gitignored)" — add a short
note directly below the table stating the new requirement Phase 1
introduces: the personal file must be mode 0600 or stricter and not a
symlink, or it (and every value in it) is refused on read, with the
remediation (`chmod 600 <path>`). This is the general-audience home for
this requirement, not `collaboration.md` (Phase 8 item 1), since it
applies to personal config as a whole, not specifically to
`github.token`.

**File**: `docs-site/src/content/docs/guides/configuration-cookbook.md`
**Changes**: The existing "Credentials are personal" section (line 142)
already tells readers to put credentials in `config.local.md` — add one
sentence there cross-referencing the new permission requirement, since
this is exactly the section a reader configuring `github.token` for the
first time is likely to land on.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run docs:check` passes (not part of the aggregate `check`
      task — run manually per project convention)

#### Manual Verification:

- [ ] The new docs page renders correctly via `mise run docs:build` and a
      local preview.
- [ ] `review-a-pr.mdx`'s Prerequisites section lists both the `gh` auth
      requirement and the new `github.token`/env-var requirement, and a
      reader following only the old bullets would still hit a clear,
      actionable error (not a confusing one) if they skipped the new one.

---

## Testing Strategy

### Unit Tests:

- `vcs`: `parse_github_remote_url` (4 supported forms + rejection),
  `resolve_origin_owner_repo` (no-origin, probe-failure, success) against
  a hand-written `StubProbe`.
- `collaboration`: the 10 domain-level characterization-restated-as-
  target-behaviour branches (of 8 resolver + 5 body-update = 13 total;
  the remaining 3 — resolver branch 1, body-update branches 1 and 2 — are
  CLI-layer, see `collaboration-cli` below) against hand-written fakes,
  including both success paths (no `parent`, `parent` present) and the
  `MalformedParent` edge case.
- `collaboration-adapters`: the HTTP-shaped branches (repository-lookup
  failure, malformed-repository-response, PR-existence-check failure,
  PATCH failure) against the mock server, plus success-path response
  parsing for both the fork and non-fork repository-metadata shapes.
- `collaboration-cli`: credential-resolver precedence (4 steps + ban),
  argument parsing, exit-code mapping, **plus the 3 CLI-layer
  characterization branches deferred from `collaboration`** — resolver
  branch 1 (invalid CLI invocation, covered structurally by clap's own
  argument-parsing tests), body-update branch 1 (invalid CLI invocation,
  same), and body-update branch 2 (missing/unreadable `--body-file` —
  the one branch with no test anywhere in an earlier draft of this plan;
  a dedicated test now confirms `run_update_body` maps a nonexistent or
  unreadable `--body-file` path to exit code 2 with a clear stderr
  message, before `update_pull_request_body` is ever called). Also: an
  end-to-end test (against the Phase 4 mock server) exercising the full
  compiled binary for at least one success case per subcommand and one
  representative failure per `BaseRepoOutcome`/`UpdateBodyOutcome`
  variant, asserting the exact stdout (`owner/repo\n`) and stderr text —
  covering `main.rs`'s error-rendering match arms and the
  `BlockingGitHubClient` shim itself, neither of which `collaboration`'s
  own unit tests exercise (they test the pure composition functions
  against fakes, not the real async-bridging plumbing that only exists
  in this crate).

### Integration Tests:

- `mise run test:integration:skill-invocation` — the three rewritten
  SKILL.md `!`-sites in production shape.
- `mise run test:integration:github` — post-removal, floor-0 pass.
- `mise run lint:dispatch-coherence:check` — the 1↔7 registration pairing.

### Manual Testing Steps:

1. Configure `github.token` locally, run both subcommands against a real
   PR, confirm output/behaviour matches the removed bash scripts'.
2. Run `/review-pr`, `/respond-to-pr`, `/describe-pr` end-to-end against a
   real PR.
3. Unset `github.token`/`github.token_cmd`, set `GH_TOKEN`, confirm
   fallback works; repeat for `GITHUB_TOKEN` alone.
4. Configure a `github.token_cmd` in the shared `config.md` and confirm
   the ban error, not silent execution.
5. Point a checkout at a repo with no `origin` remote and confirm the
   clear "no origin remote configured" error from `base-repo`.
6. Point a checkout whose `origin` is a fork at a PR opened against the
   fork's upstream and confirm `base-repo` resolves to the upstream
   owner/repo, not the fork's.

## Performance Considerations

None — this is a low-frequency, human-invoked CLI (PR review/description
workflows), not a hot path. The `current_thread` tokio runtime is
sufficient; no concurrency is needed since each invocation makes at most
three sequential REST calls (repository-metadata lookup, then
PR-existence confirmation, then — for `update-body` only — the PATCH).

## Migration Notes

No data migration for the collaboration binary itself. The two bash
scripts' skill call sites are the only consumers (confirmed by a
whole-repo grep in the research); once Phase 6 lands, Phase 7's removal is
safe with no transition window needed beyond normal PR review/merge
sequencing. Coordinate Phase 7's floor decrement with work-item:0174 per
the work item's Dependencies section, and be aware work-item:0195/0196
decrement their own independent floor constants in the same shared
`tasks/test/integration.py` file around the same time — ordinary merge
contention, not a logical conflict.

**Phase 1's personal-config-file permission enforcement is a genuine
breaking change**, separate from the collaboration feature itself: any
existing `config.local.md` with looser-than-0600 permissions or a symlink
starts failing every personal-level config read (not just GitHub-related
ones) immediately on upgrade, with no bypass. This is treated as an
accepted, documented trade-off rather than something requiring a
warn-then-enforce rollout — a personal config file has always been
capable of holding credentials (`jira.token`, `linear.token`, and now
`github.token`), and `accelerator config set --local` has always written
it at 0600, so a non-compliant file only arises from something outside
normal tool usage. Remediation is a single `chmod 600 <path>` (or
recreating a symlinked file as a regular one), named directly in the new
error message. See Phase 8 item 4 for the corresponding release-notes-
facing documentation this requires.

## References

- Original work item: `meta/work/0197-accelerator-collaboration-pr-helper-cli.md`
- Related research: `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`
- Similar implementation: `cli/vcs/src/lib.rs:42-120`, `cli/vcs/src/classify.rs:46-76`,
  `cli/vcs-cli/src/main.rs`, `cli/work-cli/src/main.rs`, `cli/work-cli/src/config.rs`
- Sub-binary registration checklist: `tasks/README.md:304-441`
- Structural precedent (worked example): `meta/plans/2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`
- ADR-0053 (thin-CLI-over-hexagonal-ports-and-adapters pattern)
