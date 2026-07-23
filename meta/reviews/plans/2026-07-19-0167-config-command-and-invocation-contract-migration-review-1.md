---
type: plan-review
id: "2026-07-19-0167-config-command-and-invocation-contract-migration-review-1"
title: "Plan Review: Built-in config Command and Invocation-Contract Migration"
date: "2026-07-19T22:01:01+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
target: "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
reviewer: Toby Clemson
verdict: REVISE
lenses: [architecture, correctness, test-coverage, safety, compatibility, security, performance, code-quality]
review_number: 1
review_pass: 4
tags: [rust, config, cli, skills, invocation-contract, allowed-tools, hooks, store, migration]
last_updated: "2026-07-20T01:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Built-in config Command and Invocation-Contract Migration

**Verdict:** REVISE

This is an exceptionally rigorous plan on the axes it has scoped: known-positive
floors for its verification greps, corpora fixed inside the check scripts rather
than chosen at verification time, same-commit permission replay rather than a
final-tree check, five divergences recorded as decisions, and a phase order that
keeps bash authoritative until the one irreversible step. It also corrects its own
work item in three places where the tree disagrees. The findings are not about
sloppiness — they are about two systematic blind spots. First, the plan reasons
carefully about failure *inside* `accelerator config` while the `bin/accelerator`
bootstrap beneath it is unconditionally fail-closed, network-coupled, and about to
become a single point of failure for 46 skills and every session start; every
fail-open guarantee in the plan engages only after that bootstrap has already
succeeded. Second, the plan's consumer model is SKILL.md-shaped, and the removal
set has a substantial second consumer population — the visualiser launcher,
`work-common.sh`, `doc-type-table.sh`, and the shipped migrations 0001–0006 — that
both verification greps are structurally incapable of seeing.

### Cross-Cutting Themes

- **The bootstrap is the unmodelled layer** (flagged by: architecture,
  compatibility, safety, performance, correctness) — every one of the 261 migrated
  call sites, plus SessionStart, now runs through `bin/accelerator`, which exits 1
  on unset `CLAUDE_PLUGIN_ROOT`, missing curl/wget, an unwritable or `noexec` cache
  dir, a lock timeout, a failed fetch, or a failed signature check. A non-zero exit
  from a `!` site discards the whole prompt. The plan's fail-open contract
  (`## Agent Names Unavailable`, exit 0) is unreachable in exactly these cases. The
  same layer is also where the latency lives (a dozen process spawns and a
  whole-binary minisign hash per invocation), where the concurrency hazard lives
  (non-atomic `cp` of the verify shim, outside the lock), and where the "we can't
  test Phase 5 until a signed release exists" problem lives.

- **Verification corpora too narrow to be complete** (flagged by: compatibility,
  test-coverage, safety) — Grep A/B are scoped `--include=SKILL.md skills/` plus
  `hooks/`, so they return 0 and 1 while ten-plus production shell consumers and
  the entire migration chain still call the deleted scripts by path. Similarly,
  `check-inventory.sh` extracts from sources Phase 7 deletes, so it passes
  vacuously at the final state; and the "no script deleted before its coverage went
  green" criterion is a claim about history with no artefact that could falsify it.

- **The gate protecting the irreversible step sits after it** (flagged by:
  architecture, safety, compatibility) — Migration Notes says 0165's artefact
  verification "is a gate on" Phase 5, but the criterion is a Phase 7 manual
  checkbox, and the plan simultaneously states every phase may sit in `main`
  indefinitely. The one state the plan says must never ship is reachable through
  the plan's own phase-independence rule.

- **`allowed-tools` breadth is under-analysed in both directions** (flagged by:
  security, safety, compatibility) — `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)`
  grants the launcher's whole dispatch surface including the
  `external_subcommand` fetch-and-exec catch-all and the net-new `config set`;
  meanwhile adding the key to `skills/config/configure` (which has none today)
  converts an unrestricted skill into one allowed only that single Bash pattern,
  stripping its Write/Edit access. Q1 also asks only whether `*` spans `/`, not
  whether a space-separated argument wildcard is honoured at all — a rule shape
  with no precedent anywhere in the tree.

- **Named seams that do not exist** (flagged by: architecture, code-quality,
  test-coverage) — the "injected-filesystem port that moves with `stage`/`persist`"
  is net-new infrastructure, not a relocation; the port for the filesystem access
  that `templates`, `init`, `skill-context` and `summary` need is unspecified; and
  the pup rule "matching the `version_core` shape" would deny `config_command` the
  very imports Phase 2 depends on.

- **Two `StoreError` types** (flagged by: architecture, code-quality, correctness) —
  `corpus::StoreError` already exists and, under the whole-crate pup rule, can never
  adopt `store`'s. The permitted-root refusal has nowhere to land in the corpus
  taxonomy and would collapse into `Io`.

- **Exit 2 still carries two meanings** (flagged by: correctness, code-quality,
  compatibility) — divergence 4 routes clap usage errors to 1 "so exit 2 keeps
  exactly one meaning", but Phase 3's own table gives it two opposite triggers, and
  `kernel::Error::Refused` becomes a launcher-wide contract change made for
  config-scoped reasons.

### Tradeoff Analysis

- **Fail-safe splice behaviour vs fail-closed distribution**: the plan is right that
  injection commands must fail open, and right that the bootstrap must fail closed
  on signature verification. These collide at the `!` call site. Recommendation:
  keep the bootstrap fail-closed for *verification*, but give it an injection-safe
  mode for a declared subcommand set that emits the `## … Unavailable` block and
  exits 0 when the binary is simply unavailable (cold cache, offline, unsupported
  platform) — distinguishing "cannot obtain" from "obtained but untrustworthy".

- **Byte-exact parity vs hardening**: security wants the skill-name and
  template-name arguments validated as identifiers (they become model-chosen after
  Phase 5, and `templates reset` deletes the resolved path); correctness and
  compatibility want "reproduce exactly". Recommendation: harden, and record it as a
  sixth divergence — the codebase already validates this way in
  `config-read-doc-type-paths.sh:101-107`, so it is inconsistency rather than
  contract.

- **Uniform legacy-layout gating vs migration reachability**: divergence 1 argues
  uniform gating is "strictly safer". It is safer for correctness and strictly more
  breaking for un-migrated repos — the shipped migrations call `config-read-path.sh`
  precisely because it does *not* assert. Recommendation: keep uniform gating but
  exempt the path-resolution commands the migration engine depends on, and record
  the exemption.

- **Mechanical 1:1 call-site rewrite vs batching**: mechanical is safer and
  reviewable; it also preserves 13–17 serial spawns per skill load at the moment the
  per-spawn cost multiplies. Recommendation: keep the rewrite mechanical for 245
  sites, carve out the consecutive `config path` runs in `visualise` and
  `config/init` as an explicit non-mechanical subset.

### Findings

#### Critical

- 🔴 **Architecture / Compatibility / Safety**: Fail-safe splice contract is
  defeated by a fail-closed bootstrap the plan never models
  **Location**: Migration Notes; Phase 2 Success Criteria (fail-open); Phase 5 Overview
  Every migrated call site now runs through `bin/accelerator`, which exits 1 on
  unset `CLAUDE_PLUGIN_ROOT`, missing curl/wget, an unwritable or `noexec` cache
  dir, a missing verify shim, a lock timeout, or any fetch/verify failure — and a
  non-zero exit from a `!` site discards the whole prompt. Today's bash scripts are
  local files that cannot fail this way. The migration moves the dominant failure
  mode from "one config block renders as Unavailable" to "the skill produces
  nothing at all", for causes entirely outside the config domain, and no phase has a
  criterion for it.

- 🔴 **Compatibility**: The removal set has production shell consumers outside
  SKILL.md that no grep, inventory or phase covers
  **Location**: Phase 5 §5 (Grep A/B corpus); Phase 7 §1 (The removal set)
  `skills/visualisation/visualise/scripts/{launch,status,stop}-server.sh` and
  `write-visualiser-config.sh`, `scripts/work-common.sh:19`,
  `skills/work/scripts/work-item-resolve-id.sh`, `scripts/doc-type-table.sh:15`,
  `skills/integrations/jira/scripts/jira-init-flow.sh:169`, and the shipped
  migrations `0001:31`, `0002:19,21`, `0004:383,459`, `0005:17`, `0006:60,335,356,372`
  all invoke removal-set scripts by path. Both verification greps filter to
  `--include=SKILL.md`, so Grep A returns exactly 0 while the visualiser launcher
  and the entire migration chain break at runtime.

- 🔴 **Security**: The new `allowed-tools` rule grants the entire launcher dispatch
  surface, not just `config`
  **Location**: Phase 5 §3: Rewrite `allowed-tools`
  `bin/accelerator` is a dispatcher whose clap tree has an
  `#[command(external_subcommand)]` catch-all, so any unrecognised first token
  routes to fetch-verify-cache-and-exec of a remote binary; the launcher also
  honours `ACCELERATOR_<SUB>_BIN` to exec an arbitrary unverified path. The outgoing
  glob enumerated 22 fixed-purpose read-mostly scripts; the incoming one is a single
  default-open entry point whose covered capability set grows automatically as 0168,
  0169 and 0173 ship sub-binaries.

- 🔴 **Security**: `config set` reaches config values that are *executed*, under a
  blanket rule and defaulting to the credential file
  **Location**: Phase 3 §1: `config set` (with Phase 5 §3)
  `jira.token_cmd`/`linear.token_cmd` are run to obtain credentials and are honoured
  **only** from `config.local.md` — exactly the file `--level Personal` (the
  default) writes; `visualiser.binary` is resolved and exec'd. No arbitrary-key write
  primitive exists in the bash removal set, so this is new capability, not
  re-homing, and one `config set jira.token_cmd '<command>'` becomes arbitrary
  command execution on the next Jira invocation.

- 🔴 **Safety**: Adding an `allowed-tools` key to `skills/config/configure` strips
  its Write/Edit access
  **Location**: Phase 5 §2 and §3
  The file currently has no `allowed-tools` key and therefore runs unrestricted.
  `allowed-tools` is an allowlist, not an additive grant, so adding
  `- Bash(…/bin/accelerator *)` restricts the skill to that one pattern and removes
  the Read/Write/Edit access it uses to create and edit `.accelerator/config.md` and
  `config.local.md`. The plan's own manual check only exercises the read path, so
  this would pass while configuration authoring is broken.

- 🔴 **Correctness / Test Coverage**: Phase 6 deletes `hooks/config-detect.sh` while
  `test-config.sh` still executes it at seven sites
  **Location**: Phase 6 §2: Registration; Phase 4 §2: Rebind; Phase 7 §2
  `scripts/test-config.sh:19` binds `CONFIG_DETECT` and exercises it at `:764`,
  `:769`, `:783`, `:798`, `:5088`, `:5098-5099`, two of which assert the
  stdout/stderr split. The suite survives to Phase 7 and runs under
  `set -euo pipefail`, so Phase 6's own "`mise run` exits 0" is unreachable and the
  phase is not independently mergeable. The script is on neither the removal set nor
  any inventory member, so those seven assertions escape every gate and are simply
  deleted.

- 🔴 **Correctness**: The fail-open `## … Unavailable` output is an unrecorded
  divergence that contradicts the same phase's parity list
  **Location**: Phase 2 §4 and Phase 2 Success Criteria (fail-open)
  Phase 2 requires `agents` to *always* emit its 9 bullets and `context` to emit
  nothing when bodies trim empty — both verified against the bash. The same phase
  then requires `## Agent Names Unavailable\n` against the unreadable-config fixture.
  No removal-set script ever emits an `Unavailable` header. These strings are
  injected verbatim into 247 skill prompts, and the divergence note — the artefact
  that makes deliberate deviations reviewable — would omit the most user-visible one.

- 🔴 **Performance**: Bootstrap warm-path overhead is per-invocation and Phase 5
  multiplies it ~250×
  **Location**: Performance Considerations; Phase 5
  Every `bin/accelerator` call re-runs two `uname` forks, a `sed` over
  `plugin.json`, `probe_dir` (mkdir + write + chmod + **exec** + rm), an
  unconditional `cp`+`chmod` of the verify shim, and a full minisign verification
  that hashes the entire launcher binary (which links `reqwest` + `rustls`) — roughly
  a dozen spawns where bash paid two. `visualise/SKILL.md` fires 17 `!` config
  invocations and `config/init/SKILL.md` 13, serially, per load.

- 🔴 **Performance**: The latency criterion measures a single invocation, manually
  **Location**: Phase 7 Manual Verification: "Performance"
  Users experience a skill load, not one invocation. The criterion is also ambiguous
  about whether it measures `bin/accelerator` or the launcher binary directly — if
  the latter, it omits the dominant cost entirely. A 15–20ms per-call regression
  passes while adding a quarter-second to two skill loads, and nothing in `mise run`
  would catch a later regression.

#### Major

- 🟡 **Architecture / Safety / Compatibility**: The 0165 artefact gate protecting the
  irreversible flip is written into Phase 7, after the flip
  **Location**: Migration Notes; Phase 5; Phase 7 Manual Verification
  Since every phase is stated to be independently mergeable, Phase 5 and 6 can both
  merge and ship before the gate is evaluated. There is also no development path:
  `bin/accelerator` gates on a minisign signature developers and CI do not hold, so
  between Phase 5 and the next signed release no migrated call site can be executed
  at all — stranding `test-configure-round-trip.sh`, the manual `/configure` and
  `/commit` checks, and Phase 6's live-hook capture. Source installs, forks and
  prerelease versions (the tree is at `1.24.0-pre.14`) have no release to fetch.

- 🟡 **Architecture / Code Quality / Correctness**: Two `StoreError` taxonomies, made
  irreconcilable by the existing pup rules
  **Location**: Phase 1 §1 and §2
  `corpus::StoreError` already exists as the return type of the `AtomicWrite` and
  `RecordStore` ports, and `corpus_domain_imports_only_permitted` allows corpus to
  name only std, `kernel::Error` and `crate` — so corpus can never adopt `store`'s
  type. `corpus::StoreError` has no `UnsafePath` variant, so the permitted-root
  refusal, whose stated justification is "so corpus gets the same guarantee",
  collapses into `Io` on the corpus side.

- 🟡 **Architecture / Code Quality / Test Coverage**: The "injected filesystem port"
  the interruption invariant is tested through does not exist
  **Location**: Phase 1 §1 and Success Criteria; Testing Strategy
  `corpus-adapters` has `stage`/`persist` as private free functions over `std::fs`
  and `tempfile`, tested against real `TempDir`s with no injection seam. The Phase 1
  criterion ("recorded call sequence contains no `open`/`create` on the target,
  exactly one `rename`") requires net-new abstraction that contradicts the concrete
  `atomic_write` signature specified in the same section — so the crate's primary
  public surface is unsettled at the point Phase 1 claims to be mergeable.

- 🟡 **Code Quality**: No port is specified for the filesystem access most
  subcommands need
  **Location**: Phase 2 §2; Phase 3 §2-3
  The `config` crate exposes only `ReadConfigLevel`/`WriteConfigLevel`, reaching two
  files. `skill-context`, `skill-instructions`, all five `templates` subcommands,
  `init` (14 directories, `.gitkeep`s, three `.gitignore` files) and `summary`'s
  sentinel check all need more, and Phase 2 declares `config_command` "inbound-only",
  leaving no outbound home. The path of least resistance — raw `std::fs` in the
  inbound adapter — makes half the subcommands untestable except through the
  black-box harness.

- 🟡 **Architecture / Code Quality**: `config_command`'s composition root and
  internal decomposition are both unspecified
  **Location**: Phase 2 §1-2
  The launcher's existing discipline is that `main.rs` constructs every adapter and
  injects it into `dispatch`; the plan never says who constructs `FileConfigStore`.
  Separately, 4 scalar + 14 block subcommands (including a ~500-line `review` port
  and the `dump`/`summary`/`agents` rendering rules) are allocated to
  `{mod.rs,inbound/mod.rs,inbound/cli.rs}` plus two modules, which at 80 columns
  plausibly lands `inbound/cli.rs` in the low thousands of lines.

- 🟡 **Code Quality**: The `render_*` purity contract has no defined inputs and
  cannot express the stderr channel
  **Location**: Phase 2 §2; Testing Strategy
  The plan cites `version::core`'s `render(&VersionReport) -> String` but never
  defines the analogous view types, and asserts behaviours a `-> String` renderer
  cannot model — `agents` warns on stderr and skips, `path` warns and yields an
  empty line, `summary` buffers stdout so warnings always precede it. Renderers will
  reach for `eprintln!` inline, making them impure and pushing every warning
  assertion into the slow harness.

- 🟡 **Architecture / Code Quality / Correctness**: `config_command`'s pup rule, as
  described, would deny the imports Phase 2 needs
  **Location**: Phase 2 §2 (`cli/pup.ron` change)
  "A module rule matching the `version_core` shape" is a domain-core inward rule
  permitting only std, `kernel::Error` and `crate::version::core`. Applied literally
  to an inbound module that must import `config` and `config-adapters`, it fails the
  build.

- 🟡 **Architecture / Correctness / Compatibility**: Uniform legacy-layout gating
  breaks un-migrated repos and may block the migration path itself
  **Location**: Implementation Approach, divergence 1
  Only 7 of 22 scripts gate today, and the omission is load-bearing: the shipped
  migrations `0004`, `0005`, `0006` call `config-read-path.sh` and
  `config-read-all-paths.sh` on repos that by definition have not been migrated.
  Combined with splice semantics, a user on the legacy layout goes from partial
  degradation to total failure, with the fixing tool potentially caught in the same
  refusal. The plan also does not say whether `LegacyLayout` takes the fail-open or
  fail-closed path, and Phase 2/Phase 6 criteria demand opposite exit codes.

- 🟡 **Correctness / Code Quality / Compatibility**: Exit 2 still carries two
  meanings, and `Refused` is a launcher-wide contract change
  **Location**: Implementation Approach divergence 4; Phase 2 §3; Phase 3 §2
  Divergence 4 justifies routing clap errors to 1 "so exit 2 keeps exactly one
  meaning", but `eject` exits 2 when the override exists and `diff`/`reset` when it
  does not. Separately, replacing `error.exit()` and adding `Refused → 2` lands on
  the *global* parse point shared by `version` and the `external_subcommand`
  catch-all — so exec'd externals returning 2 for their own reasons become
  indistinguishable from "confirmation required", and 0164's shipped exit contract
  changes with no note.

- 🟡 **Security / Compatibility**: Q1 is under-scoped — it asks the wrong questions
  about the trailing `*`
  **Location**: Phase 5 §1: Resolve Q1 first
  The proposed rule uses a *space-separated argument* wildcard, a shape with no
  precedent anywhere in `skills/` (every existing rule is a path glob or an exact
  command name). Q1 asks only whether `*` spans `/`. It does not ask whether an
  argument-position wildcard is honoured at all, whether it matches a zero-argument
  invocation, or — the security question — whether the matcher operates on the raw
  command string, so that `… config get x; <anything>` satisfies the rule. The probe
  also targets only the version floor, not what users run.

- 🟡 **Security**: Model-controlled name arguments meet unvalidated path
  interpolation ported forward verbatim
  **Location**: Phase 2 §4; Phase 3 §2
  `config-read-skill-context.sh:23` interpolates `$SKILL_NAME` into a path with no
  validation, and `config_resolve_template` interpolates the template key into three
  candidates while Tier 1 accepts an absolute value outright. Today these arguments
  are literals in each SKILL.md; after Phase 5 the model chooses them under a
  blanket rule. `templates reset` *deletes* the resolved path, and `skill-context`
  fails silently on absence so probing is invisible.

- 🟡 **Safety / Security / Correctness**: File mode across the `atomic_write`
  consolidation is unspecified
  **Location**: Phase 1 §2; Phase 3 §1
  `rename` replaces the inode, so the result takes the temp file's mode. Moving from
  `fs::write` (umask, ~0644) to `NamedTempFile` (0600) silently flips
  `.accelerator/config.md` to 0600 for a committed, team-shared file — while
  `jira-auth.sh:21,28-30` fails closed with `E_LOCAL_PERMS_INSECURE` when
  `config.local.md` is looser than 0600, making that file's mode a load-bearing
  control that no criterion pins.

- 🟡 **Security / Correctness**: The symlink refusal has holes at the absent-parent
  and symlinked-root cases
  **Location**: Phase 1 §1
  The permitted root for config is `<project>/.accelerator`, absent on a fresh repo —
  so "treat an absent root as Ok" disables the guard on exactly the first-write
  path. Ordering against `stage`'s `create_dir_all(parent)` is undefined:
  canonicalising before creation fails, after means the chain was created through
  whatever symlink was being checked. And canonicalising the root as trusted means a
  root that is itself a symlink (git stores symlinks; a cloned repo can ship
  `.accelerator` as one) is followed while passing by construction.

- 🟡 **Safety**: `templates reset --confirm` deletes a config-resolved path that may
  be outside the repository and outside VCS
  **Location**: Phase 3 §2; Phase 1 §1
  `config-reset-template.sh:92` does a bare `rm "$RESOLVED_PATH"` where the path may
  come from an absolute `templates.<key>` config value; the script only *warns* when
  the target is outside the project root and still deletes under `--confirm`.
  `eject --force --all` can overwrite N such files. Phase 1 adds a `permitted_root`
  bound and Phase 3 routes neither destructive operation through it — the new safety
  primitive is scoped to the one command that is not destructive, and the project's
  revert-based recovery path does not exist for out-of-tree files.

- 🟡 **Safety**: `config set` re-serialises frontmatter — comments and formatting are
  lost, unrecoverably for personal config
  **Location**: Phase 3 §1
  `document::render` re-emits frontmatter from the parsed node tree and concatenates
  only the *body* verbatim, so YAML comments, blank lines and quoting styles are
  destroyed on the first write. The Phase 3 criterion pins body prose, which the
  concatenation satisfies while saying nothing about frontmatter. For
  `--level personal` the target is gitignored, so "VCS revert is the recovery path"
  does not apply.

- 🟡 **Safety**: SessionStart now runs the network bootstrap synchronously
  **Location**: Phase 6 §2: Registration
  `bin/accelerator` allows `--connect-timeout 30 --max-time 300` per artefact and up
  to 30s on the cache lock, all before the launcher execs. Today `config-detect.sh`
  returns immediately, offline. A non-critical context-injection hook becomes a
  blocking dependency on GitHub Releases at every session start, and the plan's
  three output states do not model the bootstrap stage that precedes them.

- 🟡 **Correctness**: The `--format=hook` contract has no state for bootstrap failure
  **Location**: Phase 6 §1-2
  `config-detect.sh` has no `set -e`, guards jq, and exits 0 on every path — it can
  never fail a SessionStart. The plan drops the `{"systemMessage": …}` branch as a jq
  artefact, but that branch was precisely the graceful-degradation path for "the tool
  this hook needs is unavailable", and its replacement can exit 1 with a bare
  `accelerator: …` line.

- 🟡 **Architecture / Safety**: The bootstrap stages its verify shim with a
  non-atomic `cp` on every invocation, outside the lock
  **Location**: Phase 5 (system impact of 261 call sites)
  `bin/accelerator:106-110` unconditionally copies and chmods the shim before the
  cache-hit check; the lock is taken only on the miss path. Phase 5 takes this from
  near-zero invocations to 247 call sites plus every session start, many concurrent
  within one skill load. A partially-written shim execs, fails verification, falls
  through to a fetch and then `exit 1` — load-dependent and invisible to
  single-invocation goldens.

- 🟡 **Test Coverage**: The repointed suite is a far weaker parity gate than its
  assertion count implies
  **Location**: Implementation Approach; Phase 4
  `test-config.sh` contains **258 `grep -q` substring checks**. The custom-lens
  assertions at `:1382` are `grep -q "| compliance |"` — a port emitting a different
  path, column spacing or row order passes. Roughly half the suite cannot detect
  rendering drift, which is precisely the defect class a bash-to-Rust output port
  introduces, yet a green repointed run is relied on as byte-level parity evidence.

- 🟡 **Test Coverage**: Two of the five recorded divergences have no assertion to
  update
  **Location**: Implementation Approach (divergences 2, 3); Phase 4 §3
  The custom-lens tests assert only name and source columns, never the path, and the
  three committed review goldens contain no custom lens at all — so nothing would
  fail if the double-slash fix were not made. Divergence 3 is reachable only from a
  subdirectory and appears only as manual step 6. A divergence nobody can detect is
  indistinguishable from a defect.

- 🟡 **Test Coverage**: The reused review goldens cover only the built-in-lens path
  **Location**: Phase 2 §5
  All three existing goldens contain only built-in rows. None exercises custom-lens
  enumeration, the `| custom |` source column, the name-conflict warning, invalid
  custom frontmatter, or a lens directory without `SKILL.md`. `review` is the most
  branching block subcommand and a fail-open injection command, and its custom-lens
  path — where behaviour is also changing — would have golden coverage of zero.

- 🟡 **Test Coverage**: At least four further suites exercise removal-set scripts and
  are unnamed
  **Location**: Phase 2 §7: Surviving-suite audit
  `skills/work/scripts/test-work-item-create-remote.sh:144`,
  `skills/work/scripts/test-work-item-scripts.sh:1047-1050`,
  `scripts/test-skill-frontmatter-population.sh:75,249` (which greps SKILL.md bodies
  for the invocation shape — the same class as `test-design.sh`), and
  `skills/integrations/jira/scripts/test-jira-paths.sh`. The audit's scope is also
  ambiguous: `run_shell_suites` is called from nine entry points over different
  subtrees.

- 🟡 **Test Coverage / Safety**: The two ordering guarantees that protect deletion
  are unverifiable or self-nulling
  **Location**: Phase 7 Success Criteria; `check-inventory.sh`
  "No script or suite was deleted before the assertions covering it either passed
  repointed or appeared in the inventory" is a claim about history with no
  falsifying artefact — while every neighbouring criterion has one and Phase 5
  solves the identical problem by per-commit replay. Separately,
  `check-inventory.sh` extracts from `test-init.sh` and the removal-set scripts,
  which Phase 7 deletes, so at the final state the extraction yields nothing and the
  check passes trivially — the same tautology the plan correctly identifies for
  `_EXPECTED_CONFIG_SUITES` but does not identify here.

- 🟡 **Test Coverage / Compatibility / Architecture**: The shims' binary resolution is
  unspecified, and `doc-type-paths` needs argument translation
  **Location**: Phase 4 §1-2
  `exec accelerator config …` resolves through PATH, so a stale global build could
  silently validate the wrong binary, and an unresolved name produces shim failures
  indistinguishable from the parity defects Phase 4 exists to surface. Separately,
  `test-config-read-doc-type-paths.sh`'s 8 call sites pass a **repo-root positional**
  (`"$RESOLVER" "$DEFREPO"`), which a forwarding shim hands to a CWD-based
  subcommand — so the shim must either `cd` (performing translation production sites
  will not perform) or the CLI must accept a root argument. Neither is stated.

- 🟡 **Test Coverage**: The `test-init.sh` characterisation step pre-commits to its
  own conclusion, and the depth floor misses the suite's real invariants
  **Location**: Phase 4 §4
  "The only way to learn whether the behaviour being ported is the behaviour the
  script has" is followed by "fix what the first run surfaces", pre-deciding that
  `init.sh` is right and the suite is wrong. The depth floor is over *`init.sh`'s*
  branches, but the suite's most valuable checks are cross-cutting invariants no
  branch owns — the `tree_hash` idempotency assertion (`:91-94`), gitignore
  non-duplication (`:101-102`), legacy `.claude/accelerator.local.md` preservation
  (`:104-122`), deep-subdirectory root discovery (`:124-131`), and the `paths.tmp`
  override (`:133-145`). All satisfy the floor vacuously.

- 🟡 **Correctness**: The exit-2 fixture model assumes a shared customised state the
  three commands do not share
  **Location**: Phase 3 §2
  `config-eject-template.sh:88` tests `[ -f "$TEMPLATES_DIR/<key>.md" ]` only, while
  `diff`/`reset` go through `config_resolve_template`, which also honours a
  `templates.<name>` config-path entry. A template customised that way is
  "customised" for diff/reset and "not customised" for eject — a third state two
  fixtures cannot exercise, so a port unifying the two resolution paths would change
  `eject`'s exit code and write target with no test failing.

- 🟡 **Correctness**: The `doc-type-escape` criterion does not match the bash's
  streaming output
  **Location**: Phase 2 Success Criteria (fail-closed)
  `config-read-doc-type-paths.sh:81-110` emits each line inside the loop and exits 1
  mid-iteration, so every doc type ordered before the offending one is already on
  stdout. "stdout empty" is unachievable against a realistic fixture, and achieving
  it by buffering is a silent divergence not on the list.

- 🟡 **Performance**: The mechanical 1:1 rewrite forecloses the highest-leverage
  optimisation
  **Location**: Phase 5 §2
  13–14 invocations in `visualisation/visualise/SKILL.md:16-29` and
  `config/init/SKILL.md:21-33` are consecutive `config-read-path.sh <key>` calls
  resolving different keys from the same two files. A batch form already exists on
  both sides (`config-read-all-paths.sh`, and the plan's own `config paths`), and
  `config-read-all-paths.sh:5-7` carries a comment lamenting exactly this. "Mechanically"
  forecloses collapsing 13 spawns into 1 precisely where the per-spawn cost is highest.

- 🟡 **Performance**: Cold-start cost is unbounded and unmeasured on a path all 46
  skills now depend on
  **Location**: Phase 7 Manual Verification
  The first invocation after a version bump or fresh clone downloads a
  multi-megabyte launcher plus signature inside a skill load, with the only criterion
  a binary "it bootstraps" check — no latency bound, no statement of what the user
  sees during the wait, and a version bump silently re-triggers it for everyone.

- 🟡 **Code Quality**: Five new grep-based shell scripts, three of them
  migration-moment artefacts with no stated lifecycle
  **Location**: Phase 1 §3; Phase 2 §6; Phase 5 §5; Testing Strategy
  Only `lint-store-duplication.sh` is registered into `mise run check`.
  `check-call-site-migration.sh` (Grep A = 0, Grep B = 1) and `check-inventory.sh`
  are true only of the migration moment; `check-skill-permissions.sh` encodes a
  permanent invariant but needs a bash-3.2 frontmatter reader and a glob matcher
  whose semantics are, per Phase 5 §1, still unknown.

#### Minor

- 🔵 **Architecture**: The SessionStart envelope is propagated to 0169/0172 as prose,
  inviting three independent reimplementations of one wire format — including the
  load-bearing "empty means emit nothing" rule. **Location**: Phase 6 §1, §3
- 🔵 **Architecture / Performance**: The latency reasoning stops one layer above the
  layer that runs — the criterion should name
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator config path <key>` explicitly and record
  bootstrap-only overhead separately. **Location**: Performance Considerations
- 🔵 **Correctness**: `main.rs` clap interception enumerates only `DisplayHelp` and
  `DisplayVersion`; `DisplayHelpOnMissingArgumentOrSubcommand` fires for a bare
  `accelerator config` and would exit 1 while `config --help` exits 0.
  **Location**: Phase 2 §3
- 🔵 **Correctness**: `config get`'s behaviour on an unset key is unspecified. The
  bash prints the default and exits 0; if not-found becomes non-zero, an unset
  optional key injects an error string where bash injected an empty line.
  **Location**: Phase 2 §4
- 🔵 **Correctness**: `ConfigService::set` matches `_ => Mapping::new()`, so a
  well-formed frontmatter whose root is a sequence or scalar is silently replaced.
  Only *malformed* frontmatter is fixture-covered. **Location**: Phase 3 §1
- 🔵 **Correctness**: Two stated parity behaviours are wrong — `summary` only
  enumerates an unrecognised skill dir when it has non-whitespace content, and the
  `config-read-review.sh:270` double slash does not reach the three stderr warnings
  (they interpolate `$lens_dir`, not `$skill_file`). **Location**: Phase 2 §4; Key
  Discoveries
- 🔵 **Correctness**: The binding counts do not reconcile — the measurements table
  says 21, Phase 4 says 18, and the cited `:12-21` range contains `CONFIG_DETECT`.
  A mechanical sweep would bind the hook to a config shim. **Location**: Phase 4 §2
- 🔵 **Safety**: Same-directory temps now land in `.accelerator/`, whose `.gitignore`
  contains only `config.local.md` — and under jj the working copy is auto-snapshotted,
  so an orphaned temp gets committed without anyone acting. **Location**: Phase 1 §2
- 🔵 **Safety**: The `permitted_root` bound may refuse legitimately-configured
  absolute paths — the catalogue explicitly supports them
  (`config_resolve_template:409-411`). **Location**: Phase 1 §1-2
- 🔵 **Safety**: `config set` is an unlocked read-modify-write; concurrent sets
  silently lose updates. `corpus-adapters/src/lock.rs` already ships a suitable
  primitive. **Location**: Phase 3 §1
- 🔵 **Security**: The bootstrap signature is over bytes only — the fetched filename
  is version-independent, so a legitimately signed older release is valid for any
  version. `Manifest::parse_and_validate` pins `expected_version`; the bootstrap does
  not. **Location**: Migration Notes; Phase 7
- 🔵 **Security**: `dump`'s redaction is a name-suffix match; reimplemented over the
  55-key catalogue, a future `github.pat` or `slack.webhook_url` leaks with no test
  failing. Make secrecy a declared catalogue property. **Location**: Phase 2 §4
- 🔵 **Security**: The injection commands render checked-in repository content
  straight into the prompt under a "apply this context" header — a prompt-injection
  channel the codebase already reasons about elsewhere
  (`create-jira-issue/SKILL.md:157`). **Location**: Phase 2 §4
- 🔵 **Security**: `accelerator_ensure_inner_gitignore` writes the `config.local.md`
  rule only when the file is absent, so a repo with a hand-edited
  `.accelerator/.gitignore` is never fixed — and the criterion can pass vacuously on
  a fixture where `init` already ran. **Location**: Phase 3 §3
- 🔵 **Compatibility**: 0106's proven invocation shape (unquoted, braced, no `bash`
  prefix) is not carried into the `accelerator` variant — and the 14 non-`!` sites
  are exactly the ones violating it today. **Location**: Phase 7 §5; Phase 5 §2
- 🔵 **Compatibility**: `hooks.json` gains its first argument-bearing command;
  whether the field is shell-interpreted is undocumented vendor behaviour with no
  precedent in the tree. Fold into the Q1 probe. **Location**: Phase 6 §2
- 🔵 **Performance**: No cross-invocation reuse — each process re-walks ancestors and
  re-parses both config files, ~26–34 reads and parses per skill load.
  **Location**: Phase 2 §2
- 🔵 **Performance**: `install_crypto_provider()` runs unconditionally before
  argument parsing, and `reqwest`+`rustls` linkage inflates the binary the bootstrap
  hashes on every call. Phase 7's criterion constrains the module graph, not the
  binary. **Location**: Phase 2 §3; Phase 7
- 🔵 **Code Quality**: `test-configure-round-trip.sh` is auto-discovered under
  `scripts/`, so it shifts the suite-count arithmetic the plan tracks in Phases 4
  and 7. **Location**: Testing Strategy
- 🔵 **Code Quality**: How `permitted_root` reaches the corpus write path is
  unspecified — the `AtomicWrite` port has no root, `FileCorpusStore` has no root
  field, and its constructor is a `const fn`. **Location**: Phase 1 §2

#### Suggestions

- 🔵 **Test Coverage**: Specify a golden regeneration switch (`UPDATE_GOLDENS=1`) and
  require regenerated goldens to be reviewed as a diff alongside the behaviour
  change — otherwise the pressure under a failing golden is to hand-edit the
  expected file. **Location**: Testing Strategy
- 🔵 **Test Coverage / Code Quality**: Assert the configure round trip per extracted
  command rather than against one concatenated whole-document golden, so a prose edit
  invalidates only the affected fragment and the failure names the command.
  **Location**: Testing Strategy
- 🔵 **Compatibility**: Consider one release where each removed script is a two-line
  deprecating shim, or document the removal set script-by-script in the release notes
  — userspace hooks and forks may call these paths. **Location**: Phase 7 §1-2

### Strengths

- ✅ Phase ordering is genuinely risk-ordered: 1–4 are additive with bash
  authoritative, so the irreversible surface is confined to two late phases rather
  than smeared across seven.
- ✅ Verification design is exemplary — known-positive floors (a mistyped pattern
  also returns zero), corpora fixed inside the check script rather than chosen at
  verification time, and same-commit permission replay rather than a final-tree
  check.
- ✅ The plan corrects its own work item where the tree disagrees: `doc-type-paths`
  and `work` reclassified as block, and the ADR-0021 exit-2 fixture matrix corrected
  to opposite customisation states.
- ✅ The `store` extraction is done properly — a shared infrastructure crate with a
  generalised `permitted_root` rather than config-specific logic in a shared place —
  and ships with a durable anti-duplication check carrying two justified exceptions.
- ✅ Divergences are enumerated, justified, written to a `meta/` decision record, and
  each paired with the assertion update in the same commit.
- ✅ The `hooks/test-vcs-detect.sh:620-634` index-sensitivity hazard is identified
  precisely and the mitigation is correct.
- ✅ Repoint-first-then-inventory correctly inverts the usual migration failure mode,
  giving the migration a real denominator instead of asking a human to re-derive 337
  assertions.
- ✅ Characterise-before-retire for `test-init.sh` (wire into CI, observe green at a
  recorded commit, then port) is the right instinct, and the retirement commit
  referencing the recorded green run makes it auditable.
- ✅ Phase 7's "no HTTP or fetch dependency in the config crate graph" is a real
  supply-chain control, not a comment.
- ✅ The `--format=hook` three-state contract preserves observable SessionStart
  behaviour, and the live-hook check correctly parses `additionalContext` out rather
  than comparing an object to a string.
- ✅ Assertion granularity is deliberately split — byte-exact stdout, substring
  stderr — so wording can evolve while the named entity stays pinned.
- ✅ Refusing to fold `cache.rs` (0600 publish + paired signature) and `lock.rs`
  (directory rename-as-claim) into the shared primitive is correct; both would be
  made less safe by a forced abstraction.

### Recommended Changes

1. **Add a bootstrap-failure posture and a Phase 0 for it** (addresses: fail-safe
   defeated by fail-closed bootstrap; SessionStart blocking; splice-safety criteria;
   cold start). Decide what a `!` site renders when `bin/accelerator` cannot run.
   Recommendation: an injection-safe mode for a declared subcommand set that emits
   the `## … Unavailable` block and exits 0 when the binary is *unavailable*, while
   keeping fail-closed for *untrustworthy*. Add criteria exercising offline,
   cold-cache, `noexec` cache dir and unsupported platform. Bound the SessionStart
   invocation so it cannot block on a fetch.

2. **Widen the removal-set consumer analysis beyond SKILL.md** (addresses:
   production shell consumers; audit scope; migrations under uniform legacy gating).
   Before Phase 5, scan the whole tree for removal-set paths, add a second consumer
   class to `0167-removal-set.md` with a per-consumer disposition, and change Grep A
   from a narrow `--include` to a tree-wide scan with a committed, auditable exclude
   list for `meta/` and `cli/`. Generate the suite-audit table mechanically over the
   union of all nine `run_shell_suites` subtrees.

3. **Narrow the `allowed-tools` rule and split `config set` out of it** (addresses:
   dispatch-surface grant; `token_cmd` execution; `configure` losing Write/Edit).
   Use `Bash(…/bin/accelerator config *)`, give `config set` its own rule declared
   only by `skills/config/configure`, refuse `*.token`/`*.token_cmd`/`visualiser.binary`
   writes from `config set`, and enumerate `configure`'s full current tool usage
   before adding its key — with an end-to-end write check in Phase 5's manual steps.

4. **Expand Q1 into a four-question probe** (addresses: Q1 under-scoping; the
   argument-wildcard shape; `hooks.json` argument expansion; metacharacters). Ask:
   does `*` span `/`; is a space-separated argument wildcard honoured; does the rule
   match a zero-argument invocation; is the matcher raw-string (admitting `;`, `&&`,
   `|`, `$(…)`). Run against both v2.1.144 and current, record both versions, and
   confirm the winning shape against one real skill before rewriting the other 34.
   Fold the `hooks.json` argument question into the same session.

5. **Move the 0165 artefact gate to a Phase 5 entry precondition and add a
   development path** (addresses: gate placement; no local-build path; source
   installs). Make it an automated Phase 5 criterion tied to all four supported
   triples, and either add a guarded development override to `bin/accelerator` or
   state explicitly which Phase 5/6 criteria are deferred until a signed prerelease
   exists.

6. **Settle the Rust structure Phase 2 leaves open** (addresses: composition root;
   missing filesystem port; `config_command` decomposition; `render_*` signature;
   pup rule). Name: who constructs `FileConfigStore` and how it is injected through
   `dispatch`; the additional driven port(s) for template/skill-context/init
   filesystem access; a `render/` submodule per output family; a `Rendered { stdout,
   warnings }` return shape; and the actual `config_command` pup allowlist.

7. **Resolve the two `StoreError` taxonomies and the filesystem-port claim**
   (addresses: duplicate error types; non-existent injection port; `permitted_root`
   plumbing; file modes). Either add `UnsafePath` to `corpus::StoreError` with an
   explicit variant mapping or rename the new type; restate the interruption
   criterion in terms of the observable invariant the existing `TempDir` tests
   already prove, or design the port explicitly; state where the corpus root comes
   from and what the constructor change is; and specify mode preservation with a
   criterion per file.

8. **Close the deletion-ordering and inventory-vacuity holes** (addresses:
   unverifiable ordering criterion; self-nulling `check-inventory.sh`; the seven
   `config-detect` assertions). Commit a deletion ledger (`deleted path → covering
   gate → commit where the gate went green`) with a per-commit replay, exactly as
   Phase 5 does for permissions. Pin `check-inventory.sh`'s extraction to a recorded
   pre-deletion revision. Add `hooks/config-detect.sh` as a fifth inventory member,
   or move `--format=hook` into Phase 2 so `CONFIG_DETECT` can be repointed in
   Phase 4 with the others.

9. **Strengthen the parity gate where greps cannot reach** (addresses: 258
   substring checks; divergences with no assertion; built-in-only review goldens).
   State explicitly that the repointed suite is a *behavioural* gate and the goldens
   carry the byte-exactness claim; add a `custom-lenses` fixture with goldens for all
   three review modes; add a `summary` test invoked from a deep subdirectory; and
   require each of the five divergences to name a specific test.

10. **Record the two unrecorded divergences and fix the exit-code story** (addresses:
    fail-open `Unavailable` strings; `doc-type-escape` streaming; exit 2's two
    meanings; launcher-wide contract change). Add the fail-open envelope as
    divergence 6 with exact strings; decide buffer-vs-stream for `doc-type-paths` and
    record it; and either split exit 2 into two codes or drop the "exactly one
    meaning" claim, renaming `kernel::Error::Refused` to match what it carries and
    noting that exec'd externals pass their own codes through.

11. **Make the latency criterion measure what users experience** (addresses:
    single-invocation p95; bootstrap overhead; foreclosed batching; shim resolution).
    Measure the full ordered `!` set of `visualise` and `config/init` through
    `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, as a scripted committed measurement with
    a recorded budget. Give the bootstrap a warm fast path (skip the shim `cp` when
    already staged; replace per-invocation full-binary verification with a validity
    stamp; make the shim staging atomic). Carve the consecutive `config path` runs
    out of the mechanical rewrite. State that Phase 4 shims exec the compiled binary
    by absolute path, not through PATH or the bootstrap.

12. **Harden the argument surface and the destructive template paths** (addresses:
    path traversal in skill/template names; `templates reset` deleting out-of-tree
    files; symlink refusal holes; init gitignore). Validate skill and template names
    as identifiers; bound `reset --confirm` and `eject --force` by the same
    permitted root Phase 1 introduces, converting today's warning into a refusal;
    specify canonicalise-nearest-existing-ancestor-then-create ordering and refuse a
    symlinked permitted root; and make `init` ensure the inner gitignore rule with
    the same `grep -qFx` guard the root rule uses.

---

## Corrections Established During Iteration

Verified against the tree while working through the recommended changes. Where
these contradict a finding above, **these supersede it**.

- **The non-SKILL.md consumer count is 28, not ~10.** The compatibility lens
  listed roughly ten files. A full scan finds 28 production `.sh` files invoking
  removal-set scripts: six work-item scripts, four visualiser, four Jira, two
  Linear, all six migrations, four shared `scripts/` helpers,
  `adr-next-number.sh` and the Playwright `run.sh`. **`jira-auth.sh` and
  `linear-auth.sh` resolve the integration credential through
  `config-read-value.sh`**, so the deletion breaks authentication outright.

- **The `allowed-tools` syntax concerns were largely unfounded.** Documentation
  research established that `Bash(cmd *)` is valid and that `Bash(cmd:*)` is an
  *equivalent* spelling, not a better one; that the trailing wildcard matches a
  zero-argument invocation (word boundary is space **or end-of-string**); and
  that the matcher is **operator-aware**, splitting on `&&`, `||`, `;`, `|`,
  `|&`, `&` and newlines with each subcommand matched independently. The security
  lens's metacharacter-chaining concern is therefore **closed** — the harness
  already prevents it, and no metacharacter screening is needed.

- **A new unknown replaced it.** Only `${CLAUDE_PROJECT_DIR}` substitution in
  `allowed-tools` is documented, and only from **v2.1.196** — above the plugin's
  declared **v2.1.144** floor. `${CLAUDE_PLUGIN_ROOT}` substitution is
  undocumented. The 35 existing rules evidently work, so it expands in practice,
  but the declared floor may already be wrong for the *current* plugin,
  independent of this story.

- **A blocking conflict with 0178 that no lens found.** `config_assert_no_legacy_layout`
  returns early under `ACCELERATOR_MIGRATION_MODE=1`, which
  `run-migrations.sh:632` sets for every migration — but **0178 deliberately
  dropped that bypass as a shipped acceptance criterion**, pinned by
  `config-adapters/tests/config_reader.rs`'s `run_with_migration_mode` helper and
  `a_legacy_layout_exits_non_zero_with_the_migrate_directive`. All six migrations
  read config, so repointing them makes `/accelerator:migrate` refuse on exactly
  the legacy-layout repos it exists to fix. Resolved by an explicit
  `--allow-legacy-layout` flag replacing the ambient env var.

- **The warm bootstrap is a per-call cost, not a first-use cost.** The cache-hit
  test at `bin/accelerator:178` *includes* a full minisign verification, so the
  launcher is hashed on every invocation. Measured on darwin-arm64: **≈3.75ms
  fixed + ≈0.75ms per megabyte**, i.e. 11-23ms per call for a 10-25MB launcher,
  before the rest of the warm path. The performance lens was right that this is
  the dominant term.

- **Two parity statements in the plan were wrong and are now corrected.**
  `config-summary.sh:113-140` enumerates an unrecognised skill dir only when it
  has non-whitespace content (the warning is unconditional, the enumeration is
  not); and the `config-read-review.sh:270` double slash does **not** reach the
  three stderr warnings at `:277,282,287`, which interpolate `$lens_dir`.

- **Quantified:** `test-config.sh` carries **258 `grep -q`** substring checks
  against 338 `assert_` calls, and the three committed review goldens contain
  **31 `| built-in |` rows and zero `| custom |`** — both confirming the
  test-coverage lens's concerns precisely.

## Disposition

All 13 recommended changes were worked through with the author. Accepted in full
or in part: 1 (partial — development override, atomic shim staging, release
profile; fail-open posture and warm-path stamp declined), 2 (option A — repoint
all 28 consumers), 3 (partial — rule narrowed to `config *`, `configure` tool
enumeration; credential-key refusal declined), 4-13 (4 reshaped by the
documentation research; 13b declined).

Declined with reasons recorded: bootstrap fail-open posture and SessionStart
bounding (a dead binary cripples the plugin regardless); warm-path validity stamp
(measure before optimising — which is what makes change 12 load-bearing);
`config set` credential-key refusal and the prompt-injection boundary (repo
content is team-reviewed before it lands); out-of-root template delete refusal
(the user configured that path deliberately).

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is unusually well-structured for its blast radius: seven
phases with bash authoritative until Phase 5, a genuine hexagonal shape for the new
`config` surface, a shared `store` crate with a generalised permitted-root parameter
plus a durable anti-duplication check, and five divergences recorded as decisions
rather than drift. The architectural weakness is not in the CLI's internal structure
but at its edges — the plan reasons carefully about failure inside
`accelerator config` while the bootstrap layer beneath it (`bin/accelerator`) is
unconditionally fail-closed, admits no unsigned local build, and becomes a new single
point of failure for all 46 config-consuming skills at Phase 5. Secondary concerns
are two competing `StoreError` taxonomies that the existing pup rules make
irreconcilable, an unspecified composition-root and domain/rendering boundary for
`config_command`, and a uniform legacy-layout gate that contradicts the plan's own
hot-path latency argument.

**Strengths**: risk-ordered phases; proper `store` extraction with allowlisted
exceptions; clean dependency direction (clap-free domain via `From`); exit-code
problem solved at the right layer; divergences as decision records; parity strategy
with a real denominator; the plan correcting its own work item.

**Findings**:

- 🔴 critical / high — **Bootstrap admits no locally built launcher** —
  *Phase 5; Migration Notes*. `bin/accelerator:147-190` gates on a minisign
  verification against an embedded release key. Between Phase 5 and the next signed
  release, no migrated call site can execute — stranding
  `test-configure-round-trip.sh`, the manual `/configure` and `/commit` checks, and
  Phase 6's live-hook capture. The only acknowledgement is a *manual* Phase 7
  checkbox framed as a release gate rather than a development enabler. Suggestion:
  add a guarded development override, or make a signed prerelease an explicit Phase 5
  entry precondition and state which criteria are deferred.

- 🔴 critical / high — **Fail-closed bootstrap under a fail-safe design** —
  *Phase 5; Implementation Approach*. The plan's splice-safe degradation is reached
  only after `bin/accelerator` succeeds; the bootstrap exits 1 on missing/unset
  `CLAUDE_PLUGIN_ROOT`, unsupported arch, absent curl/wget, unwritable or `noexec`
  cache dir, lock timeout, failed fetch, or failed verification
  (`:14-16, 61, 101, 141, 186`). Today's bash cluster has no such failure surface.
  Suggestion: decide and record the bootstrap-failure posture for `!` call sites, and
  add criteria for offline-cold-cache and `noexec`-cache-dir.

- 🟡 major / high — **Two `StoreError` taxonomies** — *Phase 1 §1-2*.
  `corpus::StoreError { NotWritable, LockTimeout, CrossFilesystem, Validation, Io }`
  already exists (`cli/corpus/src/store.rs:14-74`) as the port return type, and
  `corpus_domain_imports_only_permitted` (`cli/pup.ron:57-72`) means corpus can never
  adopt `store`'s. The permitted-root refusal has no corpus representation and
  collapses into `Io`. Suggestion: add `UnsafePath` to `corpus::StoreError` (already
  `#[non_exhaustive]`) with a documented `From`, or move the port taxonomy into
  `store` and adjust the pup rules.

- 🟡 major / high — **The injected filesystem port does not exist** — *Phase 1 §1;
  Testing Strategy*. `corpus-adapters` has `stage`/`persist` as free functions over
  `std::fs`, tested against real `TempDir`s (`:212-237`). The criterion requires
  net-new abstraction contradicting the concrete signature specified in the same
  section. Suggestion: design the port explicitly, or restate the criterion in terms
  of observable properties the existing seam already proves.

- 🟡 major / medium — **Composition root unspecified; pup rule self-contradictory** —
  *Phase 2 §1-2*. `main.rs` constructs every adapter and injects into `dispatch`
  (`main.rs:86-92`, `launch/mod.rs:22-39`), but the plan never says who constructs
  `FileConfigStore`; and a rule "matching the `version_core` shape" would deny
  `config`/`config_adapters` imports. Suggestion: state the injection through
  `dispatch` and write the actual inbound allowlist.

- 🟡 major / medium — **No rendering/policy boundary for the block subcommands** —
  *Phase 2 §2, §4*. `review`'s `min_lenses` defaults and lens assembly, `agents`'
  fallbacks and warn-and-skip, `dump`'s attribution and redaction are policy, not
  formatting — and `config-adapters/src/render.rs` is a third candidate home. Likely
  outcome: the domain accumulates in the launcher while the `config` crate stays
  anemic, so 0168's consumer must reach into the launcher or reimplement. Suggestion:
  state a placement rule and name which functions land where for `review` and `dump`.

- 🟡 major / medium — **Uniform legacy gating on the hot path** — *Implementation
  Approach divergence 1*. The bash deliberately omits the guard from
  `config-read-path.sh` (the 66-call-site hot path the plan's own Performance section
  cites), `config-read-agents.sh`, `config-read-context.sh` and
  `config-read-doc-type-paths.sh`. It also collides with "agents always emits its 9
  bullets" and would silence the SessionStart summary for the users who most need
  migration guidance. Suggestion: scope the divergence, or state the fail-open
  routing for `LegacyLayout` and reconcile the `agents` criterion.

- 🟡 major / medium — **Verify-shim `cp` races at 261 call sites** — *Phase 5*.
  `bin/accelerator:106-110` copies and chmods the shim on every invocation, outside
  the lock (taken only on the miss path, `:181`). A partially written shim execs,
  fails closed, refetches, and can `exit 1`. Load-dependent, so invisible to
  single-invocation goldens. Suggestion: stage to a unique temp name and `mv`, or
  skip when correctly staged; add a concurrent-invocation criterion.

- 🔵 minor / medium — **SessionStart envelope propagated as prose** — *Phase 6 §1,
  §3*. `vcs-detect` (0169) and `migrate-discoverability` (0172) will need the
  identical envelope including the load-bearing emptiness rule. Suggestion: factor a
  shared `session_start_envelope(Option<&str>) -> Option<String>` and record the type
  as the contract.

- 🔵 minor / medium — **Shim binary resolution unspecified** — *Phase 4 §1*. A bare
  `accelerator` cannot be the bootstrap path, so it must be the cargo output — giving
  the shell suites a build-order dependency on the `cli` workspace and coupling two
  otherwise-separable components. Suggestion: `exec "${ACCELERATOR_BIN:?}"` and state
  where the build is sequenced.

- 🔵 minor / medium — **Latency reasoning stops one layer above what runs** —
  *Performance Considerations; Phase 7*. The measured path should be
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator config path <key>`, with bootstrap-only
  overhead recorded separately.

### Correctness

**Summary**: The plan is unusually rigorous about parity — it caught the exit-2
overload the work item got wrong, it insists on known-positive proofs for its greps
and its duplicate-check, and its headline measurements (297 `scripts/config-` hits,
35 files with the `config-*` glob, 9 agent keys, 7-of-20 legacy-layout gating,
`atomic_write` at `config-adapters/src/store.rs:58-80`) all verify against the tree.
The correctness weaknesses are concentrated in three places: the exit-code scheme
still carries two contradictory meanings for exit 2 while the plan asserts it carries
one; the fail-open contract for injection commands invents output strings and error
states the bash never produces, contradicting the same phase's "reproduce exactly"
list; and Phase 6 deletes `hooks/config-detect.sh` while `scripts/test-config.sh` —
which survives until Phase 7 — executes it at seven sites, so Phase 6 cannot be green
on its own as the plan claims. Several boundary states (legacy layout under
`--format=hook`, bootstrap failure at SessionStart, absent permitted root on a first
write, clap's help-on-missing-subcommand kind) are unspecified rather than wrong, but
each one is a live path.

**Strengths**: the ADR-0021 exit-2 correction verified against the tree; `--all`
aggregation semantics stated correctly (`config-eject-template.sh:130-136`);
known-positive floors closing the "mistyped pattern returns zero" hole; the `.git`
boundary marker being genuinely load-bearing given `discover_root`'s unbounded walk;
the `hooks.json` index hazard correctly diagnosed; the `stage`/`persist` seam
keeping the interruption invariant testable; characterise-before-port for
`test-init.sh`.

**Findings**:

- 🔴 critical / high — **`CONFIG_DETECT` falls between Phase 6 and Phase 7** —
  *Phase 6 §2; Phase 7 §2*. `test-config.sh:19` binds it and executes it at `:764`,
  `:769`, `:783`, `:798`, `:5088`, `:5098-5099`. Phase 4's repoint range `:12-21`
  silently spans it while the stated count of 18 excludes it. The suite runs under
  `set -euo pipefail`. Suggestion: shim it to `config summary --format=hook`, or move
  its seven assertions into the inventory as a fifth member and delete them in the
  same commit.

- 🔴 critical / high — **Fail-open `Unavailable` strings are an unrecorded
  divergence** — *Phase 2 §4 and Success Criteria*. Verified:
  `config-read-agents.sh:117-124` emits the default block; `config-read-context.sh:26-33`
  emits nothing. No script emits an `Unavailable` header. These strings reach 247
  skill prompts. Suggestion: add as divergence 6 with exact strings and triggering
  fixtures, or reproduce the bash degradation shapes.

- 🟡 major / high — **Exit 2 still has two meanings** — *divergence 4; Phase 3 §2*.
  A caller cannot interpret exit 2 without knowing which subcommand it invoked — the
  exact ambiguity divergence 4 exists to eliminate. Suggestion: restate as
  "caller-actionable refusal, subcommand-scoped" in the divergence note and
  ADR-0021, or split into distinct codes and record the `configure` branch updates.

- 🟡 major / high — **The customised/not-customised fixture model is binary but the
  behaviour is not** — *Phase 3 §2*. `config-eject-template.sh:88` tests the
  templates dir only; `diff`/`reset` (`:34-44`, `:58-68`) go through
  `config_resolve_template`, which honours `CONFIG_TEMPLATE_SOURCE_CONFIG_PATH`.
  Suggestion: add a **config-path-customised** fixture and assert the asymmetry
  including `reset`'s source-specific `Note:`.

- 🟡 major / high — **`doc-type-escape` stdout-empty criterion contradicts the
  script** — *Phase 2 Success Criteria*. `config-read-doc-type-paths.sh:81-110` emits
  inside the loop and exits 1 mid-iteration. Suggestion: buffer and record it as a
  divergence, or assert the partial prefix with the escaping key placed mid-catalogue.

- 🟡 major / medium — **Legacy-layout policy precedence undefined** — *divergence 1
  vs Phase 6 §1*. Only 7 scripts gate (`config-read-value.sh:22`,
  `config-read-skill-context.sh:14`, `config-read-skill-instructions.sh:15`,
  `config-dump.sh:13`, `config-read-review.sh:13`, `config-read-template.sh:27`,
  `config-summary.sh:11`). Phase 2 requires uniform refusal *and* fail-open exit 0;
  Phase 6 requires exit 0 in all three states. Suggestion: state precedence and add a
  **legacy-layout** fixture asserting all three classes.

- 🟡 major / medium — **The hook can now fail** — *Phase 6 §2*. `config-detect.sh`
  has no `set -e`, guards jq, and exits 0 on every path. The bootstrap `fail()`s on
  six conditions (`bin/accelerator:14-17,100-102,178-190`), and the dropped
  `{"systemMessage": …}` branch was the graceful-degradation path. Suggestion: add a
  fourth state, keep a thin absorbing wrapper, or `|| true` in `hooks.json`.

- 🟡 major / medium — **Symlink refusal is bypassed on the first write** — *Phase 1
  §1*. `permitted_root` for config is `<project>/.accelerator`, absent on a fresh
  repo, and "absent root is Ok" disables the guard there. Ordering against `stage`'s
  `create_dir_all(parent)` (`corpus-adapters/src/store.rs:59`) is unspecified.
  Suggestion: canonicalise the nearest existing ancestor, verify containment, then
  create; add a unit test for "root absent, target reached through a symlinked
  ancestor".

- 🔵 minor / medium — **Third clap non-error kind unhandled** — *Phase 2 §3*.
  `DisplayHelpOnMissingArgumentOrSubcommand` fires for a bare `accelerator config`.
  Suggestion: enumerate all three kinds and snapshot exit code and stream for
  `config`, `config --help`, `--version`.

- 🔵 minor / medium — **`config get`'s not-found behaviour unspecified** — *Phase 2
  §4*. `config-read-value.sh:126-130` prints `$DEFAULT` and exits 0. Suggestion: add
  an explicit criterion and state whether `catalogue::default_for` is consulted.

- 🔵 minor / medium — **Non-mapping frontmatter root is silently replaced** —
  *Phase 3 §1*. `config/src/service.rs:116-119` matches `_ => Mapping::new()`, and
  `config-adapters/src/document.rs:33-37` parses every YAML variant successfully.
  Suggestion: add a **non-mapping-root** fixture and a refuse-and-preserve criterion.

- 🔵 minor / medium — **Consolidation details underspecified** — *Phase 1 §2*. The
  `corpus::StoreError` name collision needs a mapping the plan does not name, and
  switching to `NamedTempFile` changes `.accelerator/config.md`'s mode from
  umask-derived to 0600 without carrying over an existing file's mode. Suggestion:
  name the `From` impls and state the mode policy with a test.

- 🔵 minor / high — **Two stated parity behaviours are wrong** — *Phase 2 §4; Key
  Discoveries*. `config-summary.sh:107-140` warns unconditionally but enumerates only
  when `context.md`/`instructions.md` has non-whitespace content; and the
  `:270` double slash does not reach the warnings at `:277,282,287`, which
  interpolate `$lens_dir`. Fixing `:270` changes the table and `custom_lens_paths`
  (`:306`) only.

- 🔵 minor / high — **Binding counts do not reconcile** — *Phase 4 §2; measurements*.
  21 vs 18, with `:12-21` containing `CONFIG_DETECT` at `:19`. Suggestion: replace
  the range with an explicit line list and reconcile the breakdown.

### Test Coverage

**Summary**: The plan's testing strategy is unusually rigorous for a migration of
this blast radius — repoint-first-then-inventory is the right sequence, the
known-positive proofs close the classic "a broken check also returns zero" hole, and
the fixture design (`.git` boundary marker, `{pid}-{counter}` uniqueness, byte-exact
stdout with substring stderr) is sound. However, the plan overestimates the strength
of its central gate: `test-config.sh` contains 258 `grep -q` substring checks, so
"the parity gate for the bulk" will not detect the byte-level rendering drift the
Rust port is most likely to introduce. Three concrete coverage holes are verifiable
in the tree today — the seven `config-detect` assertions fall through every gate, two
of the five recorded divergences have no assertion to update, and at least four
suites outside the named three exercise removal-set scripts without appearing in the
plan.

**Strengths**: repoint-first inverts the usual failure mode; known-positive proofs;
the per-branch/per-exit-code depth floor pre-empting a rubber-stamp inventory;
correct and specific fixture design; deliberate byte-exact-stdout/substring-stderr
split; per-commit replay for same-commit re-homing; characterise-before-retire for
`test-init.sh`; the corrected `--all` aggregation and exit-2 fixture table.

**Findings**:

- 🔴 critical / high — **The seven `config-detect` assertions fall through every
  gate** — *Phase 4 §2; Phase 6; Phase 7 §2*. The shim shape targets `config
  <subcommand>` but the behaviour is `config summary --format=hook`, which does not
  exist until Phase 6; the script is not on the removal set (so inventory member 4
  misses it) and has a covering suite (so member 3 misses it); then `test-config.sh`
  is deleted. Suggestion: add it as a fifth inventory member with the depth floor
  applied to its three output states, or move `--format=hook` into Phase 2.

- 🟡 major / high — **258 `grep -q` substring checks** — *Implementation Approach;
  Phase 4*. The custom-lens assertions at `:1382` are
  `grep -q "| compliance |" && grep -q "| custom |"`; stderr assertions are
  regex-loose (`grep -q "Warning.*missing.*name"`, `:1442`). Roughly half the suite
  cannot detect rendering drift. Suggestion: state the limit, treat the repointed
  suite as behavioural only, and require every block subcommand to have byte-exact
  goldens covering non-trivial states, not only baseline.

- 🟡 major / high — **Divergences 2 and 3 have no assertion** — *divergences 2-3;
  Phase 4 §3*. The custom-lens tests (`:1371-1520`) never assert the path column, and
  the three committed goldens contain no custom lens. Divergence 3 is reachable only
  from a subdirectory. Suggestion: require each divergence to name a test; add a
  custom-lens review golden and a `summary` test with
  `.current_dir(fixture.join("src/deep"))`.

- 🟡 major / high — **The reused review goldens cover only built-in lenses** —
  *Phase 2 §5*. None exercises custom-lens enumeration, the `| custom |` column, the
  name-conflict warning (`:1451-1468`), invalid custom frontmatter, or a lens
  directory without `SKILL.md`. Suggestion: add a `custom-lenses` fixture with
  goldens for all three modes.

- 🟡 major / high — **At least four further suites break** — *Phase 2 §7*.
  `test-work-item-create-remote.sh:144` (bare-exec `config-read-work.sh`),
  `test-work-item-scripts.sh:1047-1050` (stubs `config-read-template.sh`),
  `test-skill-frontmatter-population.sh:75,249` (greps SKILL.md for the invocation
  shape — breaks at Phase 5, same class as `test-design.sh`), `test-jira-paths.sh`.
  `run_shell_suites` is called from nine entry points, so the audit's referent is
  ambiguous. Suggestion: fix the corpus as the union of all nine and generate the
  table mechanically.

- 🟡 major / high — **The no-premature-deletion criterion is aspirational** —
  *Phase 7 Success Criteria*. It is a claim about history with no falsifying
  artefact, while every neighbouring criterion has one and Phase 5 solves the
  identical problem by per-commit replay. It also guards the plan's single
  irreversible risk. Suggestion: commit a deletion ledger plus a per-commit replay
  script, and commit the output.

- 🟡 major / high — **The injected filesystem port is net-new, not relocated** —
  *Phase 1 §1 and Success Criteria*. `cli/corpus-adapters/tests/store.rs` has nine
  tests covering JSONL round-tripping, prefix anchoring and concurrent appends — no
  recorded-call-sequence test, no `Filesystem` trait. Suggestion: restate as new
  work, define the port and a recording fake, and confirm the signature impact on
  both callers.

- 🟡 major / medium — **The `doc-type-paths` shim must translate, not forward** —
  *Phase 4 §2*. The 8 call sites pass a repo-root positional
  (`def_out="$("$RESOLVER" "$DEFREPO")"`, `:37`). Suggestion: decide whether the
  subcommand takes an optional root positional or is CWD-only; if CWD-only, rewrite
  the call sites to `cd` and add a golden asserting CWD discovery directly.

- 🟡 major / medium — **Characterisation pre-commits to its conclusion; the depth
  floor is vacuous for the suite's invariants** — *Phase 4 §4*. "Fix what the first
  run surfaces" pre-decides that `init.sh` is right. And the floor is over `init.sh`'s
  branches, while the suite's most valuable checks — `tree_hash` idempotency
  (`:91-94`), gitignore non-duplication (`:101-102`), legacy rule preservation
  (`:104-122`), deep-subdirectory discovery (`:124-131`), `paths.tmp` override
  (`:133-145`) — belong to no branch. Suggestion: record a decision per failure
  (script wrong vs suite wrong) and extend the floor to "every assertion gets a row"
  (~25 assertions).

- 🔵 minor / medium — **Golden-heavy pyramid with no regeneration mechanism** —
  *Testing Strategy; Phase 2 §5*. 13 block subcommands × ~12 fixtures plus `--help`
  snapshots that churn on every doc-comment edit. Suggestion: specify
  `UPDATE_GOLDENS=1` and require regenerated goldens reviewed as a diff in the same
  commit.

- 🔵 suggestion / medium — **The configure round-trip golden is coupled to SKILL.md
  prose** — *Testing Strategy*. Any reordering or added block changes the
  concatenation. It also lands under `scripts/` and raises discovery. Suggestion:
  assert per-command with keyed golden fragments.

### Safety

**Summary**: The plan is unusually safety-aware for a migration of this size: it
consolidates atomic writes behind one primitive with a symlink-escape refusal and a
fault-injection seam, keeps bash authoritative through Phase 4, pins
fail-open/fail-closed behaviour per command class against named fixtures, and records
known-positive floors so its verification greps cannot pass vacuously. The residual
hazards cluster in three places: the irreversible Phase 5/6 contract flip whose
protective gate is written into Phase 7, a net-new write path (`config set`) that
re-serialises hand-authored frontmatter into a gitignored file, and two genuinely
destructive template operations (`reset --confirm`, `eject --force --all`) that
operate on config-controlled paths and are the only commands the plan's new bounded-write
primitive does not cover. Several of these break the project's "VCS revert is the
recovery path" assumption because the files involved are either gitignored or outside
the repository.

**Strengths**: irreversible surface confined to two late phases; per-class
fail-open/closed contracts with dedicated fixtures; structural write-path assertions
(no open/create, exactly one rename, no temp residue, byte-identical after refusal);
vacuity-proofed verification (recorded floors, fixed corpora, per-commit replay);
explicit removal-set file list; `config-summary.sh`'s golden captured before
deletion; correctly refusing to fold `cache.rs` and `lock.rs` into the primitive.

**Findings**:

- 🔴 critical / high — **The 0165 gate is written after the flip it protects** —
  *Phase 5; Migration Notes; Phase 7*. Phase-independence makes the "released
  half-migrated contract" state reachable through the plan's own rule, and the failure
  is total for any user on an unserved platform. Suggestion: make it a blocking
  Phase 5 Success Criterion covering all four triples in `bin/accelerator`'s `case`
  arms.

- 🔴 critical / high — **`allowed-tools` on `configure` strips Write/Edit** —
  *Phase 5 §2-3*. Current frontmatter is `name`, `description`, `argument-hint`,
  `disable-model-invocation` only, so the skill runs unrestricted; `allowed-tools` is
  an allowlist. The skill writes `.accelerator/config.md` and `config.local.md` at
  SKILL.md `:30-34`, `:98-100`. The plan's manual check exercises only the read path.
  Suggestion: enumerate its full tool usage and add an end-to-end write check.

- 🟡 major / high — **`config set` destroys frontmatter comments, unrecoverably for
  personal config** — *Phase 3 §1*. `document::render` re-emits from the parsed tree
  (`config-adapters/src/document.rs:26-29,56-70`) and concatenates only the body.
  `--level personal` writes the gitignored file. Suggestion: record as divergence 6
  with a pinned re-render shape, or add a criterion asserting a commented frontmatter
  round-trips with only the edited key changed.

- 🟡 major / high — **`templates reset --confirm` deletes an out-of-tree,
  out-of-VCS path** — *Phase 3 §2; Phase 1 §1*. `config-reset-template.sh:92` is a
  bare `rm "$RESOLVED_PATH"`; Tier 1 accepts an absolute `templates.<key>` value
  verbatim (`config-common.sh:409-414`); the out-of-project case only *warns*
  (`:82-84`). `eject --force --all` can overwrite N such files. Phase 1's
  `permitted_root` is scoped to the one non-destructive command. Suggestion: bound
  both with the same check, converting the warning into a refusal, with criteria
  against an out-of-project fixture.

- 🟡 major / high — **SessionStart becomes a synchronous network dependency** —
  *Phase 6 §2*. `--connect-timeout 30 --max-time 300` per artefact
  (`bin/accelerator:51-53`) plus up to 30s lock wait (`:139-144`), before the launcher
  execs. Suggestion: bound the invocation or add a bootstrap mode that emits nothing
  and exits 0 on a cold cache; add a criterion asserting bounded exit-0 with the
  cache absent and the network unreachable.

- 🟡 major / medium — **The fail-safe contract only engages past the bootstrap** —
  *Phase 5; Migration Notes*. `bin/accelerator:14-17,185-187` prints to stderr and
  exits 1; the stdout fail-safe block is never produced. Across 247 sites in 46
  files, the plan does not state what the skill body receives. Suggestion: add a
  criterion covering bootstrap failure at a call site, and consider having the
  bootstrap emit the fail-safe block for `config` invocations.

- 🟡 major / medium — **`check-inventory.sh` nulls itself at the final state** —
  *Phase 7 §2 and Success Criteria*. Its extraction sources are exactly what Phase 7
  deletes — the same tautology the plan identifies for `_EXPECTED_CONFIG_SUITES` but
  not here. Suggestion: pin the extraction to a recorded pre-deletion revision.

- 🔵 minor / high — **Temps now land in a tracked directory** — *Phase 1 §2*.
  `.accelerator/.gitignore` contains only `config.local.md`; the `*`/`!.gitkeep`
  pattern lives in `.accelerator/tmp/.gitignore` and no longer applies. Under jj the
  working copy is auto-snapshotted. Suggestion: add the temp pattern to
  `.accelerator/.gitignore` and the `init` scaffold.

- 🔵 minor / medium — **`persist` replaces the target's mode with 0600** — *Phase 1
  §1*. Today `fs::write` is umask-derived (~0644). A team-shared committed config
  becomes unreadable to other users and CI, invisibly to git. Suggestion: stat the
  target and re-apply its mode; add a criterion.

- 🔵 minor / medium — **The permitted-root bound may refuse legitimate absolute
  paths** — *Phase 1 §1-2*. The catalogue supports them
  (`config_resolve_template:409-411`, `config-eject-template.sh:70-72`). Suggestion:
  add a criterion and decide permitted-vs-refused explicitly.

- 🔵 minor / medium — **Non-atomic verify-shim `cp` on every invocation** —
  *Phase 5*. `bin/accelerator:107`, outside the lock, now at 247 call sites plus
  every session start. Suggestion: stage to a temp name and `mv`, or skip when
  already correct.

- 🔵 minor / medium — **`config set` is an unlocked read-modify-write** — *Phase 3
  §1*. `atomic_write` prevents torn reads, not lost updates; `corpus-adapters/src/lock.rs`
  already ships a suitable primitive. Suggestion: reuse it, or record
  last-writer-wins as a deliberate decision with a pinning criterion.

### Compatibility

**Summary**: The plan is unusually rigorous about the contract surface it has scoped
— byte-exact goldens, a two-grep denominator, same-commit permission re-homing, an
index-sensitive `hooks.json` hazard already spotted, and five divergences recorded
rather than smuggled. But its consumer model is SKILL.md-shaped, and the removal set
has a substantial second consumer population it never enumerates: production shell
scripts (the visualiser launcher, `work-common`, `doc-type-table`, `jira-init-flow`)
and the shipped migrations 0001–0006 all invoke removal-set scripts by path, and the
Grep A/B corpus is structurally incapable of seeing any of them. Separately, the
migration replaces a zero-dependency local bash call with a fail-closed,
network-and-release-coupled bootstrap on a splice path where any non-zero exit
discards the entire prompt — a compatibility posture change that no criterion covers.

**Strengths**: the `hooks/test-vcs-detect.sh:620-634` hazard identified precisely
with a correct mitigation; sound two-grep denominator with a corpus fixed inside the
script; per-commit replay for same-commit re-homing; five divergences each with a
reason and a same-commit assertion update; the exit-2 opposite-states correction; the
`--format=hook` three-state contract and the correct `additionalContext` parse in the
live-hook check.

**Findings**:

- 🔴 critical / high — **Removal set has production shell consumers no grep sees** —
  *Phase 5 §5; Phase 7 §1*. Enumerated:
  `skills/visualisation/visualise/scripts/launch-server.sh:16,110`,
  `status-server.sh:16`, `stop-server.sh:16`,
  `write-visualiser-config.sh:55,64,65,91,155,156,185,217,263,267`;
  `scripts/work-common.sh:19`; `skills/work/scripts/work-item-resolve-id.sh:38,45,46`;
  `scripts/doc-type-table.sh:15` (consumed by
  `scripts/validate-corpus-frontmatter.sh:12,58`);
  `skills/integrations/jira/scripts/jira-init-flow.sh:169`; migrations `0001:31`,
  `0002:19,21`, `0004:383,459`, `0005:17`, `0006:60,335,356,372`. None appears in the
  plan, work item or research. Suggestion: scan tree-wide for `*.sh`, add a second
  consumer class with per-consumer disposition, and widen Grep A to a tree-wide scan
  with a committed exclude list (auditable in a way a narrow include is not).

- 🔴 critical / high — **Fail-safe posture defeated by the fail-closed bootstrap** —
  *Migration Notes; Phase 2 Success Criteria; Phase 5*. Enumerated failure paths:
  `CLAUDE_PLUGIN_ROOT` unset/not a directory, no curl or wget (`:61`), no
  writable-and-exec-capable cache dir (`:100-102` — a `noexec` mount or read-only
  install), missing verify shim (`:66`), missing release public key (`:68`), any
  fetch or verification failure (`:186`). Suggestion: add a splice-safety criterion —
  e.g. `… config context || printf '## Project Context Unavailable\n'`, or a
  bootstrap flag rendering the unavailable notice and exiting 0 for a declared
  injection-safe set; test with `ACCELERATOR_RELEASE_BASE_URL` pointing at an
  unreachable host.

- 🟡 major / high — **Release-artefact gate placed in Phase 7; no story for
  non-release installs** — *Phase 7 Manual Verification; Migration Notes*.
  `bin/accelerator:40-43,149` reads the version from `plugin.json` and fetches
  `v${version}/accelerator-${platform}`, so every bump needs a matching four-platform
  signed release. Git clones, forks, and the current `1.24.0-pre.14` prerelease have
  no release to fetch. Suggestion: make it an automated Phase 5 criterion (extending
  the existing version-coherence pattern) and record the intended source-install
  behaviour.

- 🟡 major / high — **Q1 is under-scoped for the rule shape being proposed** —
  *Phase 5 §1*. Every existing `Bash(...)` rule in `skills/` is a path glob or an
  exact command name; a literal space before `*` appears nowhere. Q1 asks only about
  `/`-spanning, not whether an argument-position wildcard is honoured or whether it
  matches a zero-argument invocation. The probe targets only the version floor.
  Suggestion: three questions, both versions, confirmed against one real skill first.

- 🟡 major / medium — **Uniform legacy gating breaks un-migrated repos and may block
  the fix** — *divergence 1*. The 13 non-asserting scripts are not an oversight:
  migrations `0004:383,459`, `0005:17`, `0006:60,335,372` call `config-read-path.sh`
  and `config-read-all-paths.sh` on repos that by definition have not been migrated.
  Suggestion: exempt the path-resolution commands, record the exemption, and add a
  legacy-layout fixture with an end-to-end migration run.

- 🟡 major / high — **The exit-code change is launcher-wide, not config-scoped** —
  *Phase 2 §3*. `Cli::try_parse()` is the single parse point for `version`, top-level
  `--help`, and the `external_subcommand` catch-all (`launch/mod.rs:33-37`). 0164
  shipped with clap's exit 2 for usage errors; and an external tool exiting 2 for its
  own reasons becomes indistinguishable from "confirmation required". Suggestion:
  record as a launcher-wide contract change with criteria for
  `accelerator --bogus-flag` and `accelerator version --bogus-flag`, and state that
  exit 2's single meaning holds only for built-ins.

- 🔵 minor / medium — **0106's proven invocation shape is not carried forward** —
  *Phase 7 §5; Phase 5 §2*. 0106's plan `:96-100` records that only the unquoted,
  braced bare path matches. `configure`'s nine sites are
  `bash "$CLAUDE_PLUGIN_ROOT/scripts/config-…"` and `init/SKILL.md:45` is
  `bash "${CLAUDE_PLUGIN_ROOT}/…"`. Suggestion: state the shape verbatim in the
  variant and have `check-skill-permissions.sh` fail on a wrapper prefix or quoted
  path.

- 🔵 minor / medium — **First argument-bearing `hooks.json` command** — *Phase 6 §2*.
  All four current registrations are bare paths. Whether the field is
  shell-interpreted is undocumented. Suggestion: fold into the Q1 probe; if not, keep
  a thin `config-detect.sh` that execs with the arguments.

- 🔵 suggestion / medium — **Shims exec a bare `accelerator`** — *Phase 4 §1*. PATH
  resolution could silently validate a differently-versioned binary. Suggestion:
  `exec "${ACCELERATOR_BIN:?}"`.

- 🔵 suggestion / low — **No deprecation window** — *Phase 7 §1-2*. No references in
  `docs/`, `templates/` or `agents/`, so these are plugin internals — but userspace
  hooks and forks may call them. Suggestion: one release of deprecating shims, or
  script-by-script release notes.

### Security

**Summary**: The plan is unusually rigorous on process controls — same-commit
permission-coverage replay, a no-HTTP-in-config dependency assertion, a 0165 signed-artefact
ship gate — and it introduces a symlink-escape refusal that does not exist today. But
its central methodology ("reproduce exactly", "flag surface preserved exactly") ports
forward the bash cluster's unvalidated-argument weaknesses at precisely the moment it
widens who supplies those arguments: replacing 35 narrow `config-*` globs with
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)` grants the launcher's entire dispatch
surface — including the `external_subcommand` fetch-and-exec catch-all and the net-new
`config set` write path — to every skill that previously only needed to read a path.
The `config set` grant is the sharpest edge: `jira.token_cmd`, `linear.token_cmd` and
`visualiser.binary` are config values that get *executed*, and `--level` defaults to
Personal, which is exactly the file whose `token_cmd` the auth path is willing to run.

**Strengths**: the permitted-root refusal is a control that exists in neither adapter
today, with correct reasoning about canonicalising both sides; the no-HTTP crate-graph
assertion is a real supply-chain control; per-commit permission replay; the
`vcs/commit` fail-open gap correctly identified as "the one file that could silently
break"; the `config.local.md` gitignore criterion carried into the new write path;
`dump`'s masking carried forward as a byte-pinned behaviour; divergence 1 as genuine
defence in depth; the 0165 ship gate treating Phase 5 as irreversible.

**Findings**:

- 🔴 critical / high — **The rule grants the whole dispatch surface** — *Phase 5 §3*.
  `cli/launcher/src/launch/inbound/cli.rs:15-22` has an `#[command(external_subcommand)]`
  catch-all; `ExternalCommand::from_raw` validates only non-emptiness; the launcher
  honours `ACCELERATOR_<SUB>_BIN` to exec an arbitrary unverified path
  (`launch/outbound/mod.rs:12-21`). Suggestion: scope to
  `Bash(…/bin/accelerator config *)`, have `check-skill-permissions.sh` reject a rule
  stopping at `accelerator `, and make the launcher refuse external subcommand names
  outside a compiled-in allowlist.

- 🔴 critical / high — **`config set` reaches executed values, defaulting to the
  credential file** — *Phase 3 §1 with Phase 5 §3*. `jira-auth.sh:24-26` documents
  that `token_cmd` is honoured **only** from `config.local.md` — the `--level Personal`
  default; `launch-server.sh:110-120` execs `visualiser.binary`; `agents.*` selects
  the dispatched subagent. No equivalent primitive exists in the removal set.
  Suggestion: separate rule declared only by `skills/config/configure`, plus a domain
  refusal for `*.token`, `*.token_cmd` and `visualiser.binary` with a test.

- 🟡 major / high — **Model-controlled names meet unvalidated path interpolation** —
  *Phase 2 §4; Phase 3 §2*. `config-read-skill-context.sh:23`;
  `config_resolve_template` (`config-common.sh:399-439`, Tier 1 accepts absolute).
  `templates reset` deletes; `skill-context` fails silently so probing is invisible.
  `config-read-doc-type-paths.sh:101-107` already hardens this way, so it is
  inconsistency, not contract. Suggestion: validate as `^[a-z0-9][a-z0-9-]*$`, apply
  the existing safe-relpath check to Tier 1, reject absolutes, and make these Phase
  2/3 criteria.

- 🟡 major / high — **File mode unspecified across the consolidation** — *Phase 1 §2;
  Phase 3 §1*. `jira-auth.sh:21,28-30` fails closed with `E_LOCAL_PERMS_INSECURE (29)`
  above 0600, so the credential file's mode is load-bearing and the plan adds the
  first write path that touches it. Suggestion: explicit mode (0600 for
  `config.local.md`, 0644 for `config.md`) with criteria including "pre-existing 0600
  stays 0600".

- 🟡 major / medium — **Two holes in the escape refusal** — *Phase 1 §1*. Ordering
  against `stage`'s `create_dir_all(parent)` (`corpus-adapters/src/store.rs:56-61`)
  leaves the create-new case — the one the control most needs — either unchecked or
  checked after the chain exists; and canonicalising the root as trusted follows a
  root that is itself a symlink (a cloned repo can ship `.accelerator` as one).
  Suggestion: canonicalise the nearest existing ancestor before any `create_dir_all`,
  and `symlink_metadata` the permitted root itself, refusing when it resolves outside
  the discovered project root.

- 🟡 major / medium — **Q1 asks the wrong question about the trailing `*`** —
  *Phase 5 §1*. The security-relevant unknown is whether the matcher is string-prefix
  over the unparsed command, so `… config get x; <anything>` satisfies the rule.
  Suggestion: probe `;`, `&&`, `||`, `|`, `$(…)`, backticks and newlines; and have
  `check-skill-permissions.sh` refuse any `!`-invocation containing a shell
  metacharacter regardless of the outcome.

- 🔵 minor / medium — **Bootstrap signature not bound to version or asset name** —
  *Migration Notes; Phase 7*. The base URL is overridable and the filename is
  version-independent, so a legitimately signed older release is valid for any
  version — while `Manifest::parse_and_validate` pins `expected_version` for
  sub-binaries. Suggestion: bind version and asset name in the trusted comment and
  refuse a cached launcher whose embedded version mismatches `plugin.json`.

- 🔵 minor / medium — **Redaction is a name-suffix match over a hand-maintained
  list** — *Phase 2 §4*. In bash it works because `config-defaults.sh:150-160` is
  hand-maintained; over 55 catalogue keys a `github.pat` or `slack.webhook_url`
  leaks with no test failing. Suggestion: `secret: true` as a catalogue property,
  defaulting true for integration groups, with a test that fails on an unmarked,
  unredacted key.

- 🔵 minor / medium — **The repo-content-into-prompt trust boundary is unrecorded** —
  *Phase 2 §4*. `config-read-skill-context.sh:30-35` renders checked-in content under
  an "apply this context" header; the codebase reasons about this explicitly at
  `create-jira-issue/SKILL.md:157`. Suggestion: record the boundary and frame the
  content as untrusted data, pinned by the goldens.

- 🔵 minor / high — **The gitignore control is absent-file-only and vacuously
  testable** — *Phase 3 §3*. `accelerator_ensure_inner_gitignore`
  (`scripts/accelerator-scaffold.sh:19-27`) writes only when the file does not exist,
  while the root rule *is* guarded with `grep -qFx`. Suggestion: diverge — use the
  same guard, have `config set --level personal` ensure both rules, and assert on a
  fixture where `init` has not run.

### Performance

**Summary**: The plan correctly identifies skill-load latency as the user-visible
risk and correctly rules out an external subcommand, but its only latency criterion
measures a single `config path` invocation and lives under Manual Verification —
which is not what users experience. The real cost is aggregate: a single SKILL.md
fires up to 17 serial `!` invocations (13-14 of them `config path` on the same two
config files), and under Phase 5 each becomes a `bin/accelerator` bootstrap that
copies the verify shim, execs a probe binary, and re-verifies the whole launcher's
minisign signature before the Rust process even starts — roughly 12 process spawns
plus a full-binary hash where bash paid two `bash` forks. The plan's mechanical 1:1
call-site rewrite preserves the N-call shape while multiplying the per-call cost, and
no batching, caching, or aggregate regression guard is proposed.

**Strengths**: the built-in-not-external decision is right and explicitly reasoned;
same-directory `NamedTempFile::new_in` removes a `create_dir_all` per write and the
cross-mount degradation risk; the plan verified `discover_root` is a cheap ancestor
walk with no subprocess; the self-relative methodology avoids needing a reference
machine; Phase 6's SessionStart hook incidentally warms the binary cache before any
skill loads.

**Findings**:

- 🔴 critical / high — **Per-invocation bootstrap overhead, ×250** — *Performance
  Considerations; Phase 5*. Two `uname` forks, a `sed` over `plugin.json`,
  `probe_dir` (mkdir + write + chmod + **exec** + rm), an unconditional `cp` of the
  multi-megabyte per-triple verify shim plus `chmod +x`, then `verify_launcher`
  hashing the entire `reqwest`+`rustls`-linked binary. `visualise/SKILL.md` has 17 `!`
  config invocations; `config/init/SKILL.md` 13. Suggestion: warm fast path — skip the
  `cp` when already staged, skip `probe_dir` when the cache dir is known-good, and
  replace per-invocation full verification with a validity stamp (dev/inode/size/mtime
  sidecar), re-verifying only on mismatch.

- 🔴 critical / high — **The criterion measures a single manual invocation** —
  *Phase 7 Manual Verification*. It is also ambiguous about whether the measured
  command is the bootstrap or the binary; "with a warm binary cache" suggests the
  bootstrap may be out of frame. A 15-20ms per-call regression passes while adding a
  quarter-second to two skill loads, and nothing in `mise run` catches later
  regressions. Suggestion: measure the full ordered `!` set of the two worst-case
  SKILL.md files through `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, as a scripted
  committed measurement with a recorded budget.

- 🟡 major / high — **Mechanical rewrite forecloses batching** — *Phase 5 §2*. 13-14
  consecutive `config-read-path.sh <key>` calls in
  `visualisation/visualise/SKILL.md:16-29` and `config/init/SKILL.md:21-33` resolve
  different keys from the same two files; `config-read-all-paths.sh:5-7` already
  laments the per-key subprocess cost. Suggestion: carve these out as a
  non-mechanical subset — one `config paths`, or a multi-key `config path k1 k2 …`.

- 🟡 major / medium — **Test shims may route ~545 assertions through the bootstrap** —
  *Phase 4 §1*. `test-config.sh` is 6,289 lines; the config shell suites are already
  observed to flake under parallel CI load, and a slower suite widens that window.
  Suggestion: state that shims exec the compiled launcher directly, and record
  before/after suite runtime as a Phase 4 criterion.

- 🟡 major / medium — **Cold start is unbounded and unmeasured** — *Phase 7 Manual
  Verification*. A multi-megabyte fetch (`--connect-timeout 30 --max-time 300`) inside
  a skill load, on the critical path of all 46 skills, re-triggered by every version
  bump. Suggestion: record a cold-start figure; verify the SessionStart hook actually
  warms the cache before the first `!` invocation; consider tightening `--max-time`.

- 🔵 minor / high — **No cross-invocation reuse** — *Phase 2 §2*. Each process re-runs
  `discover_root` and re-reads/re-parses both files
  (`config-adapters/src/store.rs:33-45,84-100`) — ~26-34 reads and parses per skill
  load. Largely subsumed by batching; if batching is rejected, note it as accepted
  cost.

- 🔵 minor / high — **Unconditional TLS provider install and HTTP linkage** —
  *Phase 2 §3; Phase 7*. `main.rs:95` installs the rustls ring provider before
  argument parsing; `reqwest`+`rustls` inflate the binary the bootstrap hashes every
  call. Phase 7's criterion constrains the module graph, not the binary. Suggestion:
  move the install behind the external-resolution path and record binary size as a
  measurement.

### Code Quality

**Summary**: This is an unusually rigorous plan on the axes of behavioural fidelity
and verification design — known-positive floors, fixed grep corpora, recorded
divergences, and per-phase mergeability are all well above the norm. Its weakness is
the inverse: it specifies *what the binary must emit* in exhaustive detail while
leaving the *internal Rust decomposition* almost entirely to the implementer. The
error layering (a second `StoreError` colliding with `corpus::StoreError`, a
`kernel::Error::Refused` carrying three different exit-2 meanings), the missing
filesystem port for the ~10 subcommands that read beyond the two config files, and the
undefined module structure for 18+ subcommands in one `config_command` are the places
where an implementer must re-derive decisions the plan implies it has already made.

**Strengths**: exemplary verification design (known-positive floors, corpora fixed in
the script, same-commit replay); the plan correcting its work item and saying so;
genuinely low-risk phase ordering with Phase 4 as an explicit parity gate; divergences
tied to same-commit assertion updates; reuse of existing goldens rather than
regenerating possibly-drifted output; the characterisation instinct for `test-init.sh`.

**Findings**:

- 🟡 major / high — **`store::StoreError` collides with `corpus::StoreError`** —
  *Phase 1 §1-2*. `cli/corpus/src/store.rs:14-20` already defines an overlapping set
  with its own `Display` and `From<StoreError> for kernel::Error`; `cli/pup.ron:58-72`
  forces the duplication; the mapping and the missing `UnsafePath` variant are
  unaddressed. Suggestion: add `UnsafePath` (the enum is `#[non_exhaustive]`) with a
  variant-by-variant `From`, or name the new type `store::WriteError`.

- 🟡 major / high — **The "injected-filesystem port" does not exist** — *Phase 1 §1
  and Success Criteria; Testing Strategy*. The interruption invariant is tested by
  staging into a real `TempDir` and dropping the `NamedTempFile` (`:213-237`) — a
  good, cheap test with no injection. Building a syscall-recording abstraction behind
  a ~25-line primitive is a large amount of indirection for very little value.
  Suggestion: restate in terms of the observable invariant, or specify the trait shape
  if a port is genuinely intended.

- 🟡 major / high — **No port for the filesystem access most subcommands need** —
  *Phase 2 §2; Phase 3 §2-3*. `ReadConfigLevel`/`WriteConfigLevel`
  (`config/src/service.rs:29-45`) reach two files; `skill-context`,
  `skill-instructions`, all five `templates` subcommands, `init` and `summary`'s
  sentinel need more, and "inbound-only" leaves no outbound home. Suggestion: name
  the additional driven port(s) — e.g. `ReadProjectFile`/`ListTemplates` implemented
  by `FileConfigStore`.

- 🟡 major / high — **`config_command`'s decomposition is unspecified** — *Phase 2
  §2; Phase 3 §2-3*. 4 scalar + 14 block subcommands including a ~500-line `review`
  port and the `dump`/`summary`/`agents` rendering rules, allocated to three files
  plus two modules; at 80 columns `inbound/cli.rs` plausibly reaches the low
  thousands of lines. Suggestion: `render/{scalar,agents,review,dump,summary,paths}.rs`
  each exporting its pure `render_*`, with `inbound/cli.rs` holding only dispatch.

- 🟡 major / medium — **`Refused` carries three meanings while the plan claims one** —
  *Phase 2 §3; Phase 3 §2*. `eject`'s exit 2 is a genuine confirmation refusal;
  `diff`/`reset`'s is a nothing-to-do condition. The carrier is a new variant in the
  lowest crate in the workspace whose doc comment describes only one case.
  Suggestion: rename and re-document (`NeedsConfirmation`/`NotApplicable`, or a
  neutral `Signalled { code }`), or drop the "exactly one meaning" claim.

- 🟡 major / medium — **The `render_*` contract has no inputs and cannot express
  stderr** — *Phase 2 §2; Testing Strategy*. `version::core`'s pattern is
  `render(&VersionReport) -> String` (`version/inbound/cli.rs:7-15`), but the config
  view types are undefined, and `agents`/`path`/`summary` all warn on stderr with
  ordering guarantees a `-> String` cannot model. Suggestion:
  `fn render_summary(view: &SummaryView) -> Rendered` where
  `Rendered { stdout: String, warnings: Vec<String> }`, with the handler the only
  writer.

- 🟡 major / medium — **Five new grep-based shell scripts, three with no lifecycle** —
  *Phase 1 §3; Phase 2 §6; Phase 5 §5; Testing Strategy*. Only
  `lint-store-duplication.sh` is registered. `check-call-site-migration.sh` and
  `check-inventory.sh` are migration-moment truths that will read as inscrutable
  constants; `check-skill-permissions.sh` needs a bash-3.2 frontmatter reader and a
  matcher whose semantics are still unknown. Suggestion: add the first two to the
  Phase 7 deletion list and register the third in `tasks/lint/scripts.py` with the
  probe's answer as an inline constant.

- 🔵 minor / high — **`test-configure-round-trip.sh` shifts the suite count** —
  *Testing Strategy*. `tasks/test/integration.py:87` discovers every `test-*.sh` under
  `scripts/` against `_EXPECTED_CONFIG_SUITES` (`:16`, currently 21), so the plan's
  arithmetic in Phases 4 and 7 is off by one. Suggestion: record it with the other
  counter movements and assert per-command rather than on a concatenated golden.

- 🔵 minor / medium — **The `version_core`-shaped pup rule contradicts Phase 2** —
  *Phase 2 §2*. `cli/pup.ron:10-24` permits only std, `kernel::Error` and
  `crate::version::core` — a domain-core inward rule applied to an inbound module
  that must import `config` and `config-adapters`. Suggestion: write the intended
  rule out (deny `launch::outbound`; permit `config`, `config_adapters`,
  `kernel::Error`, std).

- 🔵 minor / medium — **`permitted_root` plumbing into corpus is unspecified** —
  *Phase 1 §2*. The `AtomicWrite` port is `write(&self, path, bytes)`
  (`corpus/src/store.rs:58`); `FileCorpusStore` holds only `LockOptions`
  (`corpus-adapters/src/store.rs:18-20`) and `with_lock_options` is a `const fn`;
  `append_record` and `remove_by_key` each need a root. Suggestion: state where the
  root comes from and note the constructor change so it is not mistaken for scope
  creep.

## Re-Review (Pass 2) — 2026-07-20

**Verdict:** REVISE

All eight lenses re-ran fresh against the revised plan (they were not given the
previous findings, so resolution below is assessed by comparison rather than by
their say-so). The revision resolved almost every finding from pass 1 — but four
new critical defects were introduced or newly exposed by the changes themselves,
three of which are silent-wrong-value failures rather than loud ones.

### Previously Identified Issues

**Resolved**

- 🔴 Removal set has production shell consumers no grep sees — **Resolved.** 28
  consumers enumerated, Grep A's corpus widened to tree-wide with an exclude list.
- 🔴 `allowed-tools` grants the whole dispatch surface — **Resolved.** Prefix
  narrowed to `config`, enforced by `check-skill-permissions.sh`.
- 🔴 Adding `allowed-tools` to `configure` strips Write/Edit — **Resolved.**
- 🔴 The seven `config-detect` assertions fall through every gate — **Resolved.**
  `--format=hook` moved into Phase 2 so `CONFIG_DETECT` repoints in Phase 4.
- 🔴 Unrecorded fail-open `## … Unavailable` divergence — **Resolved** as
  divergence 6.
- 🟡 0165 artefact gate placed after the flip — **Resolved** as a Phase 5 entry
  precondition (though the *mechanism* is now questioned — see new issues).
- 🟡 Two `StoreError` taxonomies — **Resolved** by the `WriteError` rename.
- 🟡 Composition root unspecified — **Resolved** (`ConfigAccess` via `dispatch`).
- 🟡 `render_*` cannot express stderr — **Resolved** by `Rendered`.
- 🟡 258 substring checks overstate the gate — **Resolved**, stated explicitly.
- 🟡 Divergences with no assertion — **Resolved**, each must name a test.
- 🟡 Built-in-only review goldens — **Resolved** by the custom-lenses fixture.
- 🟡 Deletion ordering unverifiable — **Resolved** by the ledger (though its
  authoring phase is missing — see new issues).
- 🟡 `check-inventory.sh` self-nulling — **Resolved** by pinning to a revision.
- 🟡 Shim binary resolution — **Resolved** (`${ACCELERATOR_BIN:?}`).
- 🟡 Exit 2's two meanings — **Resolved**, documented rather than denied.
- 🟡 `test-init.sh` depth floor vacuous — **Resolved** (per-assertion).
- 🔵 Q1 metacharacter chaining — **Resolved** by documentation research; the
  matcher is operator-aware. (Compatibility notes the conclusion should be
  narrowed to *separator*-aware — `$(…)` and backticks are not separators.)

**Partially resolved**

- 🟡 `doc-type-paths` shim translation — **Partially.** Still deferred to Phase 4,
  but three lenses now show the decision belongs in Phase 2, and correctness found
  a production consumer (`doc-type-table.sh:41`) passes the root positional, not
  just the test harness.
- 🟡 `config_command` pup rule — **Partially.** Written out explicitly, but
  architecture found the new rule permits `config_adapters`, contradicting the
  composition-root statement four paragraphs above it.

### New Issues Introduced

- 🔴 **`--allow-legacy-layout` restores only half of `ACCELERATOR_MIGRATION_MODE`**
  (architecture, correctness, compatibility — three lenses independently).
  Verified: `config_find_files` (`config-common.sh:41-47`) uses the env var to
  enable a *data-source fallback* reading `.claude/accelerator.md` when the
  `.accelerator/` pair is absent — its comment reads "Legacy fallback: config not
  yet moved by migration 0003". The flag as specified suppresses only the guard.
  Repointed migrations 0001/0002 would resolve *absent* rather than fail, and
  proceed with defaults: `0001` migrates the wrong tickets directory, `0002`
  takes the wrong `work.id_pattern` branch. Silent corruption on exactly the
  repos `/accelerator:migrate` exists to repair, and Phase 5's "runs green
  against a legacy-layout fixture" criterion passes because green is the wrong
  signal. **This defect was introduced by change 5 of the iteration.**

- 🔴 **`config get`'s caller-supplied default is unspecified** (test-coverage,
  compatibility). Verified: `config-read-value.sh:25,131` returns the *caller's*
  `$2`, never a catalogue value. Consumers depend on both directions —
  `write-visualiser-config.sh:64-65`, `jira-auth.sh:228` and `linear-auth.sh:241`
  pass `""` as a **presence probe** to distinguish configured from defaulted,
  while `write-visualiser-config.sh:185` passes a non-empty default. A catalogue
  fallback breaks every probe; a bare-empty fallback drops every explicit
  default. Both are silent wrong values. The parity gate cannot catch it: the two
  two-positional sites in `test-config.sh` (`:5975`, `:5977`) pass defaults that
  coincide with the catalogue values.

- 🔴 **Repointed migrations swallow failures and are then recorded as applied**
  (safety). `0006:335,356,372` use `2>/dev/null || true`, `0004:383` uses
  `|| echo ""`, `0001:31,33` use `|| true`, each followed by an empty-means-skip
  guard. Under the repointed contract these reads can now fail (bootstrap,
  legacy-layout refusal) in ways the bash could not — and every failure becomes
  "nothing to migrate". The migration exits 0, `run-migrations.sh:25` appends it
  to `migrations-applied`, and it never re-runs. Permanent half-migration with no
  recovery short of hand-editing the ledger.

- 🔴 **`ACCELERATOR_LAUNCHER_BIN` is an env-var bypass of the whole verification
  chain** (security, medium confidence). Two environment variables plus a stderr
  line disable fetch, cache and minisign verification on a path that is
  blanket-pre-authorised in 35 SKILL.md files and auto-invoked by a SessionStart
  hook. The codebase has a stronger precedent for exactly this shape:
  `jira-auth.sh:196-203` honours `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` only
  alongside a non-symlink, VCS-**tracked** marker file.

- 🟡 **`summary --format=hook` cannot satisfy both the opt-in rule and Phase 6's
  contract** (correctness). Divergence 5 says degradation requires `--fail-safe`;
  Phase 2 asserts every injection command exits non-zero without it; Phase 6
  requires exit 0 on unavailable and registers the command *without* the flag.
  Mutually unsatisfiable, and the registration takes the failing side — turning a
  hook that cannot fail today into one that fails every session on a legacy or
  unreadable config.

- 🟡 **Malformed-config fail-loud is an unrecorded divergence** (correctness,
  compatibility). `cli/config-adapters/tests/parity.rs:296-323` already records
  that the Rust errors where bash skips the malformed level and reads the other.
  With the 28 shell consumers deliberately given no `--fail-safe`, one broken
  `config.local.md` turns warn-and-default into hard failure across the work-item,
  doc-type, ADR and visualiser scripts. `parity.rs:256-293` records further
  block-sequence and value-encoding divergences, also unlisted.

- 🟡 **The `--fail-safe` checker covers five subcommands; the rewrite rule says
  "every injection site"** (correctness). Most of the 247 sites are `path`/`get`,
  which the checker does not cover — so for the bulk of the corpus a missed flag
  regresses to a discarded prompt with no build failure.

- 🟡 **`context --skill`'s join is specified two incompatible ways** (correctness).
  "Joined by exactly one blank line" versus "byte-identical to concatenating the
  two bash scripts' output" — concatenation yields *zero* blank lines. The
  empty-project-block case (the common one) is undefined and would emit a leading
  blank line.

- 🟡 **The deletion ledger has no authoring phase** (test-coverage). It appears
  only in Phase 7's criteria, so it would be reconstructed by reading the history
  it purports to certify — the unfalsifiable claim it replaced.

- 🟡 **The reused review goldens were never byte-exactly validated**
  (test-coverage). `test-config.sh:1947-1949` compares via command substitution,
  which strips trailing newlines from both operands. The plan's byte-exactness
  claim rests on files whose trailing-newline content is unconstrained.

- 🟡 **The 0165 gate cannot extend `validate_version_coherence`** (compatibility,
  safety). That check (`tasks/build.py:184-210`) compares local version strings
  and has no network dependency; and `tasks/release.py:87` bumps the version
  inside the release run, so `plugin.json` on `main` names the last *published*
  release. Asserting artefacts for it says nothing about the release carrying
  Phase 5.

- 🟡 **Latency measurement placed in the phase that deletes its comparator**
  (performance). Phase 7 deletes `config-read-path.sh`, which is the bash side of
  the comparison. The budget is also recorded at measurement time rather than
  pre-committed, so the measurement cannot fail — the same tautology the plan
  refuses twice elsewhere — and nothing forbids running it through Phase 0's
  override, which skips the verification term being measured.

- 🟡 **`From<store::WriteError> for corpus::StoreError` violates the orphan rule**
  (code-quality). Both types are foreign to `corpus-adapters`, and `corpus` cannot
  name `store`. The impl has no legal home as specified; it needs to be a private
  free function.

- 🟡 **`config_adapters::compose()` is bypassed** (code-quality). The existing
  tested wiring helper (`compose.rs:19-26`) already discovers the root, runs the
  legacy guard and builds the service — the plan duplicates that protocol in
  `main.rs` and never says how `--allow-legacy-layout` reaches the guard.

- 🟡 **The new driven port is the largest new abstraction and is one paragraph**
  (architecture, code-quality). Unnamed, operations unenumerated, error type
  unstated, injection unaddressed — while spanning read, enumerate, create-tree
  and delete.

- 🔵 Smaller: nothing confines `--allow-legacy-layout` to the migrations (no
  inverse assertion); `test-init.sh` gains no CI floor constant and the plan's
  claim that the config floor moves up is wrong (it is a different subtree);
  `check-inventory.sh` has no known-positive floor; the suite-audit generator is
  one-hop so transitive consumers are misclassified; `config init` is ported in
  Phase 3 before its contract is characterised in Phase 4; new team-level
  `config.md` would land 0600; `--explain` is indistinguishable from the default;
  `config set` remains an unlocked read-modify-write; `SHELL_LIBRARIES` still
  names the deleted `config-defaults.sh`.

### Assessment

The revision worked: 18 of 20 pass-1 findings are fully resolved and the two
partials are narrowed to specific decisions. The plan is materially stronger — the
28 shell consumers, the narrowed permission rule, the deletion ledger and the
aggregate latency criterion all close real holes.

But it is not ready. Four new criticals stand, and their shape is worse than
pass 1's: three of the four fail **silently with wrong values** rather than
loudly. The `--allow-legacy-layout` half-restoration and the migration
failure-swallowing compound each other — both corrupt migrations, both pass the
criteria written to catch them. The `config get` default gap sits underneath 28
consumers and two authentication paths.

The common cause is worth noting: each was introduced by a change made *during*
the iteration, and each survived because the criterion written alongside it
asserts the happy path. A third pass should target the newly added mechanisms
specifically rather than re-reviewing the whole plan.

## Re-Review (Pass 3) — 2026-07-20

**Verdict:** REVISE

After pass 2's twelve accepted changes, all eight lenses re-ran, scoped to the
mechanisms those changes added or reshaped. Pass 2's four criticals were all
resolved. Five new criticals surfaced — three introduced by pass-2 changes
themselves, one a design flaw in a pass-2 fix, one latent in the plan from the
start and missed by every prior pass.

### Previously Identified Issues (pass 2)

- 🔴 `--allow-legacy-layout` restored only half of `ACCELERATOR_MIGRATION_MODE`
  — **Resolved.** The flag now carries the legacy source fallback as well as the
  guard suppression; verified the fallback engages only when the current-layout
  pair is absent, so it is inert for migrations 0004-0006.
- 🔴 `config get`'s caller-supplied default unspecified — **Resolved.** The
  `get`/`path` asymmetry is now a table (`get` = caller default only, no
  catalogue; `path` = caller default, else catalogue, else empty+warning), with
  criteria for the presence-probe consumers.
- 🔴 Repointed migrations swallow failures then record themselves applied —
  **Resolved in principle** (converted to distinguish non-zero from
  exit-0-empty), but pass 3 found the conversion incomplete — see new issues.
- 🔴 `ACCELERATOR_LAUNCHER_BIN` env-var bypass — **Addressed but the fix was
  wrong** — see new issues.

### New Issues Introduced

- 🔴 **Phase 5 still claimed the `context --skill` collapse was "byte-identical
  concatenation"** while Phase 2 proved it inserts a blank line. Two sections
  gave the implementer opposite acceptance conditions for the same rewrite.
- 🔴 **`--fail-safe` + `--format=hook` contradicted.** Divergence 5 rendered a
  notice on stdout; Phase 6 required nothing on stdout, and registered the
  command without the flag. Introduced by adding `--fail-safe` to the
  registration (pass-2 change E).
- 🔴 **`0006:59-60` omitted from the migration conversion list** — the worst
  site, quoted as the motivating example yet not in the list. It sits inside
  `resolve_corpus_path()` called at `:298` as `if ! rel="$(…)"`, a command
  substitution where `set -e` is suspended, so `log_die`'s `exit` terminates only
  the subshell and the migration is still recorded applied.
- 🔴 **The `--allow-legacy-layout` / marker gate was CWD-discoverable.** Whoever
  can set the two env vars can also choose the CWD and synthesise a repo with a
  tracked marker; under jj auto-snapshot a created file is tracked by the time the
  probe runs. The two-factor gate collapsed to one. (Design flaw in a pass-2 fix.)
- 🔴 **Phase 7 deleted `config-defaults.sh` while retaining `config-common.sh`,
  which sources it unconditionally at `:9`.** Every surviving sourcer — six
  migrations, work-item, Jira/Linear, visualiser — would break at source time.
  Latent from the start; missed by every prior pass including two that examined
  the removal set.

### Assessment

The revision resolved pass 2's findings, but the criticals were not converging
(9 → 4 → 5) and were changing character — from "you will break 28 consumers" to
"this sentence contradicts the paragraph below it". Three of the five were
introduced by the iteration itself, each surviving because the criterion written
alongside it asserted the happy path. Root cause identified as method, not plan:
a 2200-line document whose mechanisms are specified in several places each, edited
under approval pressure. Author elected to fix the five criticals only and stop
iterating specification detail.

## Re-Review (Pass 4) — 2026-07-20

**Verdict:** REVISE

The five pass-3 criticals were fixed and re-reviewed narrowly (correctness,
safety, security, test-coverage). Four fixes held; one — the launcher gate — was
wrong again, on a checkable fact; and the review surfaced that the
`config-defaults.sh` defect was one instance of a class with three more members.

### Previously Identified Issues (pass 3)

- 🔴 `context --skill` byte-identity contradiction — **Resolved.** Phase 5 now
  defers to Phase 2's join rule; recorded as divergence 13.
- 🔴 `--fail-safe` + `--format=hook` contradiction — **Resolved.** `summary` and
  the scalar commands degrade by suppression (empty stdout), not by notice.
- 🔴 `0006:59-60` omission and the subshell hazard — **Resolved.** Site table
  completed by grep; conversion made structural (distinct return codes 2 = read
  failed / 1 = not configured, caller dies on 2).
- 🔴 `config-defaults.sh` deletion — **Resolved.** Removed from the removal set,
  retained for 0174, with the consequential edits.
- 🔴 Launcher marker gate CWD-discoverable — **Fix was wrong again** — see below.

### New / Still-Open Issues

- 🔴 **The source-checkout launcher gate does not distinguish a marketplace
  install.** The replacement gated on `${CLAUDE_PLUGIN_ROOT}/cli/Cargo.toml`
  existing — but verified against the installed cache
  (`~/.claude/plugins/cache/.../1.24.0-pre.13/`), the marketplace ships the whole
  repository tree including `cli/`. Condition 2 is satisfied in every install, so
  the gate collapses to one env var — the same failure mode as the marker design
  it replaced. **Second wrong launcher-gate design in two passes.** Recommendation
  recorded: cut the override from this plan; it is `bin/accelerator`'s design and
  belongs with 0164/0165.
- 🔴 **The `config-defaults.sh` defect is a class with four members**, only one
  of which the fix addressed. Confirmed by the mechanical audit (below):
  `config-common.sh:407,422` (retained library invoking two removal-set scripts),
  `cli/config/src/catalogue.rs:301,325` (drift test sourcing `config-dump.sh` and
  invoking `config-read-review.sh` — the plan's "unaffected" claim was false), and
  `cli/corpus-adapters/tests/common/mod.rs:66` (`doc_type_table()` shelling out to
  `config-read-doc-type-paths.sh`). Grep A is structurally blind to all of them
  because it matches paths, not basenames.
- 🟡 Scalar `--fail-safe` suppression, the migration return-code scheme's third
  outcome (unsafe-relpath skip), the stub-injection mechanism for the migration
  criteria, and the Phase 0 gate's per-condition criteria all need specification
  work — carried as an open list rather than closed in prose.

### Mechanical Removal-Set Audit

Rather than continue finding one instance of the class per pass, a mechanical
audit was run and committed as `meta/inventories/0167-removal-set-references.md`
(re-runnable via `0167-audit-removal-set.sh`). It enumerates every reference to
every removal-set member **by basename**, over the whole tree, in every file
type. Results: **95 distinct referencing files** — 47 SKILL.md, 31 shell, 10 test
suites, 4 Rust, 3 other. It found the fourth class member
(`corpus-adapters/tests/common/mod.rs`) that no hand pass had, and corrected two
plan counts (shell consumers 28 → 31; SKILL.md files 46 → 47).

The audit's four open items were applied to the plan: the `catalogue.rs`
disposition (move `REVIEW_*`/`AGENT_*` arrays into `config-defaults.sh`, drop the
runtime cross-check), the two additional pinned Rust dependants, deleting
`config_resolve_template` in Phase 7 with its callers, and Grep A matching by
basename with a floor naming the three references a path pattern misses — plus a
new criterion that no retained file references a removal-set member at the final
state.

### Assessment

The audit closed the defect class that had produced one critical per pass — the
highest-value action available, and one that a review pass structurally could not
achieve. Two items remain genuinely unresolved rather than merely unspecified:
the launcher override (two wrong designs; recommend removing from this plan) and
the specification-level open list (recommend handing to implementation).

The plan is materially stronger than review 1 pass 1 — the 31 consumers, the
narrowed permission rule, the deletion ledger, the basename audit and the
aggregate latency budget all close real holes — but it is large enough that
plan-time prose review has reached diminishing returns. The remaining verdict is
REVISE, with the path to APPROVE being: remove the override, apply the audit's
class-level gate (done), and take the specification-level items into
implementation as a tracked open list.
