---
type: plan
id: "2026-08-11-0196-design-cli-migration"
title: "accelerator-design: CLI Migration and Shell-Free Executor Implementation Plan"
date: "2026-08-11T21:49:36+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0196"
parent: "work-item:0196"
derived_from: ["codebase-research:2026-08-11-0196-design-cli-implementation-surface"]
relates_to: ["plan:2026-08-11-0196-design-vendored-runtime-distribution"]
supersedes: ["plan:2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli"]
tags: [rust, design, cli, sub-binary, executor, playwright]
revision: "8117629cd5dc64027b0174a21ddb33c72ef0468d"
repository: "accelerator"
last_updated: "2026-08-12T11:19:28+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# accelerator-design: CLI Migration and Shell-Free Executor Implementation Plan

## Overview

Migrate the design inventory and gap tooling into an `accelerator-design` dispatched
sub-binary, and reproduce the Playwright launcher in Rust so the delegation chain is
CLI → Node with no shell in between. The runtime still comes from where it comes from
today: `ensure-playwright.sh` survives, the lockhash namespace survives, and a system
Node.js is still required.

This is one of two plans against work-item:0196. The other —
`plan:2026-08-11-0196-design-vendored-runtime-distribution` — vendors the runtime so
that prerequisite goes away. **This plan is deliberately independent of it**: nothing
here touches the launcher's fetch-verify-cache mechanism, the release pipeline, or the
manifest schema, so it can be implemented, reviewed and merged on its own.

### Why this is a separate plan

The two halves were planned as one eight-phase document and reviewed three times. Every
pass closed the previous pass's findings and introduced new criticals in the fix
material — 7 after pass 1, 8 after pass 2 — and **every one of those landed in the
tree-artifact, release-pipeline and runtime-swap phases**. The migration phases
accumulated majors and minors but no criticals in the last two passes.

Splitting on that line lets the settled work proceed while the unsettled work gets the
empirical answers it needs. See
`meta/reviews/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1.md`
for the full record.

### Phase numbering is inherited

Phases keep the numbers they carried in the superseded plan — 1, 2, 3, 6 here; 4, 5, 7
in the sibling plan — so the gaps are expected. Renumbering would have meant rewriting
every one of the dozens of internal `Phase N §M` and `Step 4b` cross-references, and a
single missed reference is precisely the defect class three review passes kept finding
in this material. The numbering is a little untidy; the cross-references are correct.

## Current State Analysis

Six of the nine shell scripts backing the two design skills are in scope here, plus the
Playwright launcher:

| Script | Lines | Disposition |
|---|---|---|
| `inventory-design/scripts/validate-source.sh` | 306 | → `design validate-source` |
| `inventory-design/scripts/resolve-auth.sh` | 68 | → `design resolve-auth` |
| `inventory-design/scripts/scrub-secrets.sh` | 47 | → `design scrub-secrets` |
| `inventory-design/scripts/notify-downgrade.sh` | 46 | → `design notify-downgrade` |
| `analyse-design-gaps/scripts/audit-cue-phrases.sh` | 105 | → `design audit-cue-phrases` |
| `inventory-design/scripts/playwright/run.sh` | 203 | → `design executor` |
| `inventory-design/scripts/inventory-metadata.sh` | 37 | deleted (Phase 3) → `corpus metadata derive` |
| `analyse-design-gaps/scripts/gap-metadata.sh` | 37 | deleted (Phase 3) → `corpus metadata derive` |
| `inventory-design/scripts/regenerate-notify-downgrade-fixtures.sh` | 18 | deleted (Phase 2 §6) with its fixtures |

`scripts/test-metadata-helpers.sh` — a top-level `scripts/` bash suite, not one of the
nine above — is also deleted (Phase 3), since it is the bash suite driving
`inventory-metadata.sh` and `gap-metadata.sh` and has no target to test once they are
gone. It runs in both `test:unit:templates` and the config integration lane, and its
deletion moves the `scripts/` floor; see Phase 3 §2.

Two survive this plan and are the sibling plan's to remove:
`ensure-playwright.sh` (367 lines) and, with it, the lockhash namespace and the system
Node prerequisite.

The ten `lib/*.js` modules and `run.js` are retained: they are the Playwright automation
itself and must run in Node.

**`scripts/test-design.sh` runs in CI, and carries more than the suites it drives.** An
earlier draft of the superseded plan recorded the opposite, on the grounds that the file
is referenced by no mise task, no invoke task and no workflow. That is true by name and
irrelevant by mechanism: `run_shell_suites` globs `**/test-*.sh` under the subtree and
filters on the exec bit (`tasks/test/helpers.py:96-102`), `test:integration:config` calls
it for `scripts/`, and `mise run test:integration` runs in CI
(`.github/workflows/main.yml:91`). `test-design.sh` is `0755`, so it and the four bash
suites it drives run on every build.

The consequence is a sequencing constraint: of the file's 553 lines, roughly 200 are
*inline* assertions over surfaces that **survive** this migration, so its per-script
blocks cannot simply be dropped by the phases that delete those scripts. This plan
re-homes what it can and cuts its own ranges; the file itself outlives this plan, because
the `ensure-playwright.sh` delegation and the `scripts/*` `allowed-tools` assertion both
belong to the sibling.

By contrast the retained `node --test` suites genuinely have no runner —
`scripts/playwright/package.json` declares no `scripts` block, and no glob reaches them
because discovery is `test-*.sh`. Phase 6 §6 adds one.

**Registration forces skill rewiring into the same change.** The checklist at
`tasks/README.md:322-474` requires points 1, 2, 3, 4, 7 and 8 to land together, and point
7 is the skill binding — a SKILL.md invoking `accelerator design` through the `!`
preprocessor with a `Bash(...)` rule whose token segment is exactly `design`.

## Desired End State

`accelerator design <subcommand>` serves both design skills for everything except the
runtime's provenance. Five ported subcommands plus `executor` are Rust; the shell layer
between the CLI and Node is gone; `run.sh`, the five ported scripts, their bash suites
and the two metadata scripts are deleted; and the `browser-executor` resolver skill
retires with the path it existed to resolve.

A Playwright-driven inventory still requires a system Node ≥20 and a bootstrapped
lockhash namespace, exactly as today. Removing that is the sibling plan's job.

Verified by: `mise run` exits 0; `accelerator design --help` lists six subcommands;
`skills/design/` contains only `ensure-playwright.sh` and the retained JavaScript; both
design skills run end to end in a live session.

### Acceptance criteria in scope

Fully: **AC5** (registration checklist), **AC15** (`corpus metadata derive
--filename-timestamp-format`).

Partially, with the remainder in the sibling plan: **AC1** (six of seven subcommands —
`notices` needs the artifacts), **AC2** (the envelopes the ported subcommands and the
executor emit), **AC3** (call sites for everything except the bootstrap step), **AC4**
(the migrated scripts and their floors, except `ensure-playwright.sh`).

Out of scope entirely: AC6, AC7, AC8, AC9, AC10, AC11, AC12, AC13, AC14, AC16.

### Key Discoveries

- `FilenameTimestampFormat::CompactTime` (`cli/corpus/src/metadata.rs:5-11`) already
  renders `{Y}-{m}-{d}-{H}{M}{S}` with label `Timestamp For Filename`, digit-for-digit
  `inventory-metadata.sh:11`'s format. It is unreachable only because
  `corpus-cli/src/main.rs:77,82` hardcodes `DateTimeUnderscored`.
- `format_filename_timestamp` (`cli/corpus-adapters/src/metadata.rs:69`) is a pure
  function with a test at `:301-312` pinning a fixed instant to `"2026-07-13-090507"` —
  so AC15's byte-for-byte claim is already covered, and the genuinely new risk is the
  argument→variant mapping.
- `lib/identity.js` and `lib/lock.js` have no production callers, and
  `identity.test.js:70-95` cross-validates against a `launcher-helpers.sh` that no longer
  exists, passing silently via `catch { return; }`.
- `test-run.sh:44-63` sources `run.sh` and asserts `start_time_of` under `LANG=C` equals
  it under `de_DE.UTF-8`. That guard covers the exact bug ADR-0058 names and must survive
  the port.
- `run.sh` exits 0 for daemon-side errors — `client.js:41,47` always resolves and error
  envelopes land on stdout — while launcher-level failures go to stderr with non-zero.
  `SKILL.md:142-143` discriminates on exactly that asymmetry.
- `run.sh`'s own envelopes are 3-key (`error`, `message`, `category`), unlike everything
  `errors.js:10` produces.
- `cargo-pup`'s `allowed_only` rules reject grouped imports, so a `design` domain crate
  needs one single-item `use` per import (`cli/pup.ron:132-138`).
- `cli/corpus/src/work_item_id.rs:13-17` injects an `IdScanner` so the domain crate never
  depends on `regex` — the precedent the cue-phrase matcher follows.
- `tasks/lint/cli.py:7` and `tasks/test/cli.py:13` both pass `--all-features`, the latter
  deliberately to enable `bash-parity`. Any non-default cargo feature added to a `cli/`
  crate is therefore **on** during `mise run cli:check` and `mise run test:unit:cli`.
  Nothing in this plan adds one; it is recorded because the sibling plan tried to.

## What We're NOT Doing

- Vendoring the Playwright runtime, extending the launcher's fetch-verify-cache
  mechanism, touching the manifest schema, or changing the release pipeline. All of that
  is `plan:2026-08-11-0196-design-vendored-runtime-distribution`.
- Removing the system Node ≥20 prerequisite, `ensure-playwright.sh`, the lockhash
  namespace, or the disk and node-version floors.
- Replacing the downgrade vocabulary. This plan ports today's six reasons as-is, because
  `ensure-playwright.sh` still emits five of them.
- Restructuring the inventory report format, or moving the model-authored report body
  into the binary. No script produces it today.
- Byte-comparing screenshots. Assertions cover count, dimensions and non-emptiness.
- Keeping `scripts/test-design.sh` indefinitely — but it is not deleted here either,
  because two of its blocks assert surfaces the sibling plan owns.
- Deleting `regenerate-notify-downgrade-fixtures.sh`'s replacement affordance: fixture
  regeneration becomes a test affordance on the Rust goldens.

## Implementation Approach

Four phases. Phase 1 unblocks Phase 3; Phase 2 unblocks Phase 6; Phase 6's removal
residue lands in the sweep.

```
Phase 1 ──> Phase 3 ──┐
Phase 2 ──> Phase 6 ──┴──> Removal sweep
```

Each phase leaves the tree green and no call site pointing at a missing script.

Decisions taken during planning, so no phase carries an open question:

- **Layout**: unchanged in this plan. `lib/*.js` keeps requiring `playwright` via
  `playwright-loader.js`; retargeting at `playwright-core` belongs with the vendored
  bundle.
- **Exit codes**: `scrub-secrets` and `audit-cue-phrases` split usage error onto exit 2,
  matching `validate-source` and `notify-downgrade` and the `kernel::Error::Refusal`
  mapping every other sub-binary uses.
- **Downgrade vocabulary**: ported as-is, six reasons, existing messages and goldens.
  Replacement is the sibling plan's, because `ensure-playwright.sh` still emits
  `node-missing`, `node-too-old`, `disk-floor-not-met`, `cache-unwritable` and
  `bootstrap-failed` until it is deleted.
- **Crate shape**: `cli/design/` (domain), `cli/design-adapters/` (filesystem, process,
  clock), `cli/design-cli/` (the `accelerator-design` binary), matching the
  corpus/vcs/work precedent.
- **Daemon identity**: one writer. The launcher observes the start time at fork and hands
  it to the daemon at spawn; `state.js` publishes the whole record in its existing single
  atomic write. See Phase 6 §1.
- **Open forks**: three decisions, resolved rather than left implicit. Two are
  empirical checks run *before* their phase is scheduled, each with its branch's
  edit set stated — whether `${CLAUDE_PLUGIN_ROOT}` expands in a subagent Bash call
  (Phase 6 §7), and whether `resolve_optional` extraction is proportionate (deferred
  with the sibling plan, which owns `design.browser_path`). The third is decided
  outright: the promoted start-time probe (Phase 6 §1) lands in a new `process-probe`
  crate depending only on `libc`, not in `design-adapters` depending on
  `visualiser/server`'s heavier dependency graph.

---

## Phase 1: `corpus metadata derive --filename-timestamp-format`

### Overview

Expose the existing `CompactTime` variant through the corpus CLI. Nothing in
`corpus` or `corpus-adapters` changes — the variant, its renderer and its label
already exist and already match the shell script's output.

### Changes Required

#### 1. CLI-local value enum

**File**: `cli/corpus-cli/src/cli.rs`
**Changes**: Add a `ValueEnum` mirror of `FilenameTimestampFormat` and a `From`
impl. The domain crate cannot derive `ValueEnum` — `cli/pup.ron:40-56` restricts
its imports to std, `kernel::Error` and `crate`.

```rust
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FilenameTimestampFormatArg {
    DateTimeUnderscored,
    CompactTime,
}
```

Add `--filename-timestamp-format` to the `metadata derive` args, defaulting to
`date-time-underscored` so existing callers are unaffected.

#### 2. Unhardcode the format

**File**: `cli/corpus-cli/src/metadata.rs`, `cli/corpus-cli/src/main.rs`
**Changes**: Thread the argument through in place of the two hardcoded
`DateTimeUnderscored` constants at `main.rs:72-86`.

#### 3. Golden harness

**File**: `cli/corpus-cli/tests/metadata_goldens.rs`
**Changes**: `:31` hardcodes `["metadata","derive"]`. Parameterise on the arg
list and add a golden for the `compact-time` variant. The binary-level golden
pins *shape* only — `derive_at` builds its own `SystemClock`
(`corpus-adapters/src/metadata.rs:235`) so there is no clock seam through the
compiled binary.

#### 4. Where AC15's byte-for-byte claim already holds

**File**: `cli/corpus-adapters/src/metadata.rs`
**Changes**: None to the renderer, and **no `FakeClock` test** — a fake would be
the wrong instrument. `Clock::filename_timestamp` *is* the seam a `FakeClock`
replaces, so a fake returns a canned string and the renderer never executes; such
a test asserts plumbing and would pass through a changed format string.

The property AC15 wants is already pinned: `format_filename_timestamp`
(`cli/corpus-adapters/src/metadata.rs:69`) is a pure function, and the test at
`:301-312` fixes an instant to `"2026-07-13-090507"` — digit-for-digit
`inventory-metadata.sh:11`'s `date '+%Y-%m-%d-%H%M%S'`, both rendered host-local so
no timezone divergence arises. AC15 points at that test.

What is genuinely new, and therefore what the new test covers, is the
argument→variant mapping: a direct assertion on the
`From<FilenameTimestampFormatArg>` impl, so selecting `compact-time` cannot
silently resolve to `DateTimeUnderscored`. That mutation is exactly what a
shape-only golden would miss.

### Success Criteria

#### Automated Verification

- [x] Failing test first for each change above — including the
      `From<FilenameTimestampFormatArg>` mapping assertion, which is the only
      genuinely new behaviour in this phase
- [x] `mise run cli:check` exits 0
- [x] `cargo nextest run -p accelerator-corpus` passes, including the new
      `compact-time` golden and a new assertion on the
      `From<FilenameTimestampFormatArg>` mapping — this lives in the binary
      crate, since that is where the `ValueEnum` mirror is defined and
      `corpus-adapters` cannot depend on it
- [x] `cargo nextest run -p corpus-adapters` passes, with AC15's byte-for-byte
      claim pointed at the existing `format_filename_timestamp` test
      (`:301-312`)
- [x] `mise run check` exits 0

#### Manual Verification

- [x] `accelerator corpus metadata derive --filename-timestamp-format compact-time`
      emits four lines whose `Timestamp For Filename` matches
      `inventory-metadata.sh`'s shape
- [x] Omitting the flag reproduces today's output exactly

---
---

## Phase 2: The `design` sub-binary and its five non-Playwright subcommands

### Overview

Create the three design crates, port five scripts, and register the sub-binary.
Registration points 1, 2, 3, 4, 7 and 8 must land together, and point 7 is a
skill binding — so both SKILL.md files are rewired in this phase for the
subcommands it introduces.

### Changes Required

#### 1. Domain crate

**File**: `cli/design/` (new)
**Changes**: Pure logic with no I/O, organised by domain concept rather than one
module per deleted script. The scripts' file boundaries are an artefact of shell,
and the corpus crate this follows names domain nouns (`doc_type`, `record`, `slug`,
`typed_ref`, `linkage`, `cluster`) rather than activities.

`design` as a bounded context means **inspecting a running design surface, auditing
the documents produced from it, and knowing whether the runtime to do either is
available here** — three sub-domains, each a module directory rather than a prose
grouping, so a new module has an obvious home or an obvious rejection:

*Source acquisition* — what may be inspected, and with what credentials:

- `host.rs` — a `Host` value type owning canonicalisation (lowercase,
  bracket/zone-id/port/trailing-dot stripping, userinfo rejection) and its
  rejections.
- `host_reach.rs` — a `HostReach` classification: loopback, private, link-local,
  reserved, unspecified, or public (six variants — see below for why five is not
  enough).
- `source_location.rs` — parsing into a `Url { scheme, Host }` or a
  `RepositoryPath`, including `about:blank`, which `validate-source.sh:198`
  classifies as its own scheme and then accepts by falling through every rejection.
  A reimplementation of "the decision tree" would plausibly turn that accept into a
  rejection, so it is named here and pinned by a characterization test. Path
  locations are classified here; existence is checked in adapters.
- `access_policy.rs` — the verdict from a `SourceLocation` plus an `Allowances
  { internal, insecure_scheme }` value type. The two flags only ever travel together
  and are only meaningful as a pair, so passing them as bare booleans would make
  every call site read `evaluate(&location, true, false)`.
- `credentials.rs` — the header/form/none precedence and the
  partial-configuration refusal naming the missing variables.

*Document auditing* — what the produced documents must not contain:

- `leaked_credentials.rs` — literal-substring detection over a set of named
  values, reporting the *name* and never the value. `ACCELERATOR_BROWSER_AUTH_HEADER`
  holds a full `Name: value` pair (`auth-header.js:14-17` splits on the first
  colon), so the scan splits it and matches the value component too — otherwise an
  artefact rendering just the bearer token, the likely leakage shape, matches
  nothing.
- `cue_phrase_audit.rs` — H2 sectioning and the audit verdict, over a
  `CuePhraseMatcher` **port**. The patterns are EREs —
  `scripts/extract-work-items-cue-phrases.txt` documents itself as "one ERE
  alternative per non-comment line" and contains `users? need` and
  `[Ii]mplement [A-Z]` — and the domain crate's pup rule permits only std,
  `kernel::Error` and `crate`, exactly as `cli/corpus/Cargo.toml` depends on nothing
  but `kernel`. So the compiled regex is injected, mirroring how corpus injects its
  `IdScanner` (`corpus/src/work_item_id.rs:13-17`) for the same reason. The `const`
  pattern slice stays in the domain as the *source* the adapter compiles; without the
  port the module could not compile against its own lint rule, and the alternative —
  hand-rolling an ERE subset matcher in the domain — is complexity with no test
  story.

*Runtime capability* (`src/runtime/`) — whether a browser runtime is available, and
what to tell the user when it is not: `executor/` (Phase 6 §1) and `downgrade.rs`.
The sibling plan adds `platform.rs` (its Phase 7 §4) to this sub-domain. An earlier draft called `downgrade.rs` homeless and
then added two more modules that belonged to neither stated sub-domain; naming the
third is what makes the layout predict where things go.

`downgrade.rs` **ports today's six-reason vocabulary as-is**, with the existing messages and
goldens. The replacement happens in the **sibling plan's Phase 7 §6**, and it must not
happen earlier:
`ensure-playwright.sh` survives until then and emits `node-missing`,
`node-too-old`, `disk-floor-not-met`, `cache-unwritable` and `bootstrap-failed`
(`:131,139,155,280,293,308,339,352`), which `SKILL.md:132` passes verbatim to
`notify-downgrade --reason`. Since every phase is independently mergeable, shipping
the new vocabulary here would make every real downgrade exit 2 with "unknown
--reason" — failing the graceful-degradation path on exactly the machines that need
it.

`credentials.rs` likewise ports `resolve-auth.sh` behaviour-for-behaviour, and this
is the place to record why despite a real defect: `makeAuthHeaderHandler` is
imported at `daemon.js:11` and never called, and its second required input
`ACCELERATOR_BROWSER_LOCATION_ORIGIN` is set nowhere in the repository, so the
header-auth path is doubly dead — while `SKILL.md:89-95,196` documents its origin
allowlist as security-critical. Users are therefore told to put real bearer tokens
into the environment of a browser-driving daemon for a feature that never applies
them, and an authenticated crawl silently produces an unauthenticated inventory.
Neither wiring it up (new feature work) nor retiring it (removing a documented
capability) belongs in a behaviour-preserving migration, so this plan preserves the
status quo and the defect becomes a **follow-up work item**, named in the Removal
sweep §4.

**Reachability classification is not transcribed.** `classify_internal`'s regexes
have real gaps that a verbatim port would preserve: they match `::ffff:127.0.0.1`
but not `::ffff:169.254.169.254` or `::ffff:10.0.0.1`, miss IPv6 unique-local
`fc00::/7`, carrier-grade NAT `100.64.0.0/10`, `0.0.0.0/8` beyond the exact string
`0.0.0.0`, and `0:0:0:0:0:0:0:1`; and the octal rejection `^0[0-9]+\.` inspects only
the *first* octet, so mixed encodings slip through. Since `validate-source` is the
SSRF boundary for a tool that then drives a headless browser at the supplied
location, each unlisted encoding is a route to a link-local metadata endpoint. So
`host_reach.rs` parses the canonical host with `std::net::IpAddr` and classifies via
`is_loopback` / `is_private` / `is_link_local` / `is_unspecified` / `is_multicast`.
The multicast predicate has no variant of its own — a multicast address folds into
`Reserved` (it is not internet-routable to a normal destination and carries the same
`--allow-internal` recoverability) — and `Unspecified` (`0.0.0.0`, `::`) is
**unconditionally rejected, with no recovering flag**: it names no host at all, so
there is nothing for `--allow-internal` to recover into.

"Looks numeric but fails strict parsing" is a concrete predicate, not a vibe: every
dot-separated label of the host is entirely digits (optionally `0x`-prefixed or
zero-padded), or the host contains a `:` — in either case the host must parse as an
`IpAddr` via `std::net::IpAddr::from_str` or be rejected outright, never falling
through to hostname treatment. This is what closes `1.2.3.4.5`, `0x7f.1`,
`10.0.0.1.example.com` (whose first label alone would look numeric under a
per-label check, which is why the rule is stated over the whole host, not per
label) and a trailing dot, alongside the encodings already named.

`Ipv4Addr::is_global` is unstable, so the reserved set is
enumerated by hand rather than gestured at: `0.0.0.0/8`, `100.64.0.0/10`,
`192.0.0.0/24`, `198.18.0.0/15`, `240.0.0.0/4`, IPv6 `fc00::/7`, and the transition
encodings that embed an IPv4 address — IPv4-mapped `::ffff:0:0/96`, 6to4 `2002::/16`,
Teredo `2001::/32` and NAT64 `64:ff9b::/96` — all unwrapped and re-classified on their
embedded address, so `2002:a9fe:a9fe::` cannot reach the metadata endpoint through an
encoding the regexes never contemplated. The matrix is a table-driven unit test.
Two of the transition forms need an explicit unwrap step rather than a direct
`is_*` call: `Ipv6Addr::to_ipv4()` is what turns `::ffff:a.b.c.d` and the
IPv4-compatible `::a.b.c.d` form alike into an `Ipv4Addr` for re-classification, so
both are covered by one unwrap rather than only the `::ffff:` form. The host string
is also normalised before parsing — percent-decoded and stripped of control
characters — so an encoding Chromium resolves but `IpAddr::from_str` rejects
outright does not silently fall through to being treated as an opaque hostname.

**`HostReach` carries six variants, not five, and the loopback carve-out is named
explicitly.** The reserved ranges above (`100.64.0.0/10`, `192.0.0.0/24`,
`198.18.0.0/15`, `240.0.0.0/4`, `fc00::/7` and the unwrapped transition encodings)
have nowhere to live in a `{ loopback, private, link_local, unspecified, public }`
enum, and that vocabulary is user-facing text (the `"host X is a $classification
address"` stderr line) — so the sixth variant is `Reserved`, and `AccessPolicy`
recovers it under `--allow-internal`, the same flag that recovers `Private` and
`LinkLocal`. And `validate-source.sh:79-84` accepts `localhost` and `127.0.0.1` on
**http with no flags at all**, applied *before* internal classification
(`:277-279`) — the skill's primary documented invocation
(`http://localhost:3000`). That carve-out is named as its own rule rather than left
to fall out of the reachability model: `HostReach::Loopback` (any address for which
`IpAddr::is_loopback` holds — a deliberate widening from the shell's two literal
strings to also cover `::1` and the rest of `127.0.0.0/8`) is always allowed
regardless of `--allow-internal` or `--allow-insecure-scheme`, on the grounds that a
loopback destination is the local machine talking to itself and carries none of the
internal-network or plaintext-interception risk the two flags guard against.
`Private`, `LinkLocal` and `Reserved` still require `--allow-internal`; a public
non-https host still requires `--allow-insecure-scheme`. A dedicated test pins the
widened set — `::1` accepted with no flags, matching the literal-string carve-out's
intent.

Three limits are recorded in the module docs as taken positions rather than left
implicit. The check is **pre-resolution only** — a public hostname resolving to
`169.254.169.254` still passes, and nothing re-checks after DNS. It covers **only
the initial location**: `validate-source` inspects the one argument the skill is
invoked with, while the daemon's `navigate` command takes an arbitrary `url` per
request and calls `page.goto(req.url)` with no classification at all
(`lib/daemon.js:165-167`), and `links` hands the agent a crawlable set whose
`same_origin` flag drives route following. So describing `validate-source` as "the SSRF
boundary" would be wrong — it is the front door, and the navigation surface is
unconstrained. And it has **no containment on the path branch**: `validate-source
/Users/me/.ssh` exits 0 today (`RepositoryPath` has no repo-root check), and the
port preserves that — stated as a taken position (any path is a valid source
location; the tool does not confirm it is inside a repository) rather than left to
be inferred from the absence of a check.

Plumbing the `AccessPolicy` verdict through the executor so `navigate` URLs are
classified by this same code is the right fix and the domain code will exist for it,
but it changes the behaviour of every crawl and belongs in its own change rather than
inside a migration. It is raised as a follow-up (Removal sweep §4) with the module docs
naming the gap, so nobody reads the hardened front door as covering the whole
surface.

One single-item `use` per import; `cli/pup.ron` gains a
`design_domain_imports_only_permitted` rule copied from the corpus precedent at
`cli/pup.ron:57-72`.

#### 2. Adapters crate

**File**: `cli/design-adapters/` (new)
**Changes**: Directory-existence checks for path locations, file reading for
`scrub-secrets` and `audit-cue-phrases`, environment reads for `resolve-auth`, and the
compiled regex behind `CuePhraseMatcher`.

The `cli/pup.ron` rule mirrors the **vcs-adapters** shape, and its scoping matters
from the start: that rule is `denied: ["^std::process"]` scoped to
`vcs_adapters::library` precisely because "the sibling subprocess module spawns by
design" (`pup.ron:140-163`). `design-adapters` will spawn — Phase 6 adds `process.rs`
for `/proc` and `ps` reads, daemon spawn and signalling — so the module split is
declared now: `design_adapters::filesystem` and `design_adapters::environment` carry
the no-spawn rule, `design_adapters::process` spawns by design. Landing the rule
crate-wide and weakening it two phases later would read to a future maintainer as a
rule nobody meant.

#### 3. Binary crate

**File**: `cli/design-cli/` (new)
**Changes**: Package `accelerator-design`, `[[bin]] name = "accelerator-design"`,
a mandatory `package.description`, inherited workspace version/edition/
rust-version/license/publish, and `[lints] workspace = true`.

Subcommands:

```
accelerator design validate-source <location> [--allow-internal] [--allow-insecure-scheme]
accelerator design resolve-auth
accelerator design scrub-secrets <file>
accelerator design notify-downgrade --reason <enum> [--from <mode>] [--to <mode>]
accelerator design audit-cue-phrases <file>
```

Exit codes: 0 accept, 1 domain rejection, 2 usage. This splits
`scrub-secrets.sh` and `audit-cue-phrases.sh`'s conflated `1` — a deliberate
behaviour change, so the repointed tests assert the new mapping explicitly.

Those three observable codes are what the plan intends, but they must not be reached
by inverting `kernel::Error`. Every other sub-binary maps `Refusal → 2` and
everything else → 1 (`corpus-cli:132`, `vcs-cli:77`, `migrate-cli:297`,
`launcher:206`, `collaboration-cli:232`), with `Refusal` documented at
`cli/kernel/src/lib.rs:16-19` as "a subcommand-scoped, caller-actionable refusal".
Making a *usage* error the `Refusal` while a *domain rejection* — the most
caller-actionable outcome this binary has — becomes a `Failed` sharing exit 1 with
genuine internal failures would leave a caller unable to tell "the tool worked and
refused your input" from "the tool broke".

So the rejection is modelled as a **domain verdict, not an error**: the domain returns
a verdict, the command layer renders it and returns exit 1 explicitly, and
`kernel::Error` keeps its documented meaning with `Refusal` carrying usage errors to
exit 2. The three classes map:

| Outcome | Mechanism | Exit |
|---|---|---|
| Accepted | `Verdict::Accepted` | 0 |
| Domain rejection | `Verdict::Rejected(reason)` | 1 |
| Usage error | `kernel::Error::Refusal` | 2 |

**The rule, not just the examples.** A usage error is a malformed *invocation* — an
unknown flag, a missing or excess argument, an argument the tool cannot interpret at
all. Anything the tool successfully evaluated and then rejected is a verdict. That
rule is what settles the cases an example list leaves open: `scrub-secrets` on a
nonexistent file is exit 2 (the argument cannot be interpreted as a file to scan),
while `validate-source` on a path that exists but is not a directory is exit 1 (it was
evaluated and rejected, matching `validate-source.sh:223-226`), and `notices` for an
artifact that is not materialised is exit 1 rather than 2.

**The carrier is named, because the corpus one cannot express exit 1.**
`cli/corpus-cli/src/outcome.rs` is two `String` fields and `corpus-cli/src/main.rs:145-149`
maps every `Ok(outcome)` to `ExitCode::SUCCESS`; no sub-binary in the workspace has a
successful-outcome path that exits non-zero. So `design-cli` returns
`Verdict<Reason> { Accepted { stdout }, Rejected { reason, stderr } }` and its `main`
matches on it, giving one render-and-exit function rather than the four bespoke
mappings a per-subcommand verdict enum would need. This is a new shape, not reuse of
an existing pattern, and the plan says so rather than describing it as
"corpus-cli-style".

**Canonical data stays in the domain crate as `const`s**, not `include_str!`'d into
the binary. Two path-relative files are involved:
`notify-downgrade-messages.json` (script-dir-relative at `notify-downgrade.sh:5`)
and `scripts/extract-work-items-cue-phrases.txt` (a four-level `..` climb at
`audit-cue-phrases.sh:32`). Baking them into the binary would leave the downgrade
enum and its message table in different crates coupled only by runtime string
equality — a new variant would compile cleanly and fail at lookup, and the compiler
could not prove the table exhaustive. It would also orphan the cue-phrase file's own
header claim to be canonical for both `extract-work-items` and this subcommand.

The workspace already has the pattern
(`cli/corpus/src/frontmatter_validation/schema.rs:277`): the canonical data is a
`const` in the domain crate — a `match reason { … }` returning `&'static str`, and a
`const` slice of cue-phrase patterns — and `include_str!` appears only inside a
`#[cfg(test)]` drift test asserting the on-disk file still agrees. Exhaustiveness
becomes a compile error and the shared-file contract becomes an executable
assertion.

Relatedly, `downgrade.rs` does **not** port `notify-downgrade.sh`'s runtime
bidi-override and printable-ASCII filters. Those guard text the shell reads from a
file at runtime; once the reason is a clap enum and the messages are compiled-in
`const`s, the only input they can see is the binary's own data, making the branch
unreachable by construction and testable only by mutating the shipped table. The
invariant is kept as a single test asserting every message in the table is printable
ASCII and free of bidi overrides.

#### 4. Registration

**Files**: `tasks/shared/paths.py`, `tasks/manifest.py`, `tasks/build.py`,
`cli/Cargo.toml`, `cli/Cargo.lock`, `.gitignore`,
`tests/integration/tasks/test_github.py`, `cli/deny.toml`

- `DISPATCHED_SUBBINARIES` gains `"design"` (point 1), with the registry pin,
  upload count and `_setup_release` fixture in `test_github.py` updated, plus
  the `_SUBBINARY_DESCRIPTIONS` entry at `:35-46` that KeyErrors without it.
- `_SUBBINARY_MANIFESTS` gains `"design": CLI_DIR / "design-cli/Cargo.toml"`
  (point 2) — required because `cli/design/` is the domain crate.
- `[workspace].members` gains all three crates; `Cargo.lock` regenerated and
  committed (point 4).
- `.gitignore` gains `bin/design-*` (point 5).
- `_CLI_RELEASE_BINARIES` gains `accelerator-design` (point 8).
- `cli/deny.toml` extended only if `mise run deny:check` reddens (point 13).

#### 5. Skill rewiring

**Files**: `skills/design/inventory-design/SKILL.md`,
`skills/design/analyse-design-gaps/SKILL.md`
**Changes**: Every call site for the five ported scripts becomes
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator design <subcommand>`. `allowed-tools`
gains `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator design *)`. The existing
`scripts/*` rules stay for the phase, since `ensure-playwright.sh` and the
metadata scripts still live there.

The token segment must be exactly `design`, and any over-broad rule in the same
frontmatter disqualifies the whole file as a witness for
`lint:dispatch-coherence:check`.

**`analyse-design-gaps/SKILL.md`'s retry logic must branch on the exit code it now
receives.** Step 5's failure handling ("the script names the offending H2 sections.
Revise only those sections... After three consecutive failures...") is written for
`audit-cue-phrases.sh`'s single conflated non-zero exit, and is not itself rewired
by the call-site substitution above. Since this subcommand's usage error is now exit
2 rather than exit 1, Step 5 is rewritten so exit 2 is reported immediately as a
usage error — there are no offending H2 sections to revise against a file the tool
could not even read — and only exit 1 keeps the existing revise-and-retry flow with
its three-attempt budget.

#### 6. Script removal

**Files**: the five ported scripts, their three bash suites, and the by-name pins
**Changes**: Delete `validate-source.sh`, `resolve-auth.sh`, `scrub-secrets.sh`,
`notify-downgrade.sh`, `audit-cue-phrases.sh`, plus `test-validate-source.sh` and
`test-notify-downgrade.sh`.

`scripts/test-design.sh` exercises these scripts through **inline** assertions, not
only through delegated suites, and it runs in CI — so this phase must cut
`:169-281` (`validate-source`, including its delegation), `:282-315`
(`resolve-auth`), `:316-338` (`scrub-secrets`), `:368-425` and `:428-430`
(`audit-cue-phrases`) and `:547-551` (the `notify-downgrade` delegation). It must
also **rewrite** `:359-364`, which asserts that `analyse-design-gaps/SKILL.md`
invokes `audit-cue-phrases.sh` and that the script exists and is executable — three
assertions that break on the call-site rewrite even though they sit in a block
otherwise re-homed unchanged. They become assertions about the new subcommand. Leaving
them makes `test:integration:config` red on merge, so this phase is not
independently mergeable without the edit. Every assertion in those ranges must map
to a named Rust test in the migration checklist below, or be recorded as a
deliberate drop with a reason. The file itself, and the blocks unrelated to these
scripts, survive until the Removal sweep §1 — and the file itself outlives this plan
entirely, because two of its blocks assert surfaces the sibling plan owns.

Two by-name pins break and need lockstep edits:

- `tests/unit/tasks/test_exec_bits.py:275-278` — `_DUAL_USE_SCRIPTS` pins
  `validate-source.sh` by literal path.
- `scripts/test-skill-frontmatter-conformance.sh:102-103` — the `EMITTERS` array
  names both design SKILL.md files. This is a `_REQUIRED_CONFIG_SUITES` by-name
  gate (`tasks/test/integration.py:63`), so it runs unconditionally.

**Two more deletions belong to this phase and had no owning phase before.**
`regenerate-notify-downgrade-fixtures.sh` and the fixtures it regenerates are
deleted here, alongside `notify-downgrade.sh` — the "What We're NOT Doing" section
names the replacement affordance (fixture regeneration as a test affordance on the
Rust goldens) but that affordance is `cli/design/`'s, not a carried-forward shell
script, so the script itself has nothing left to call once `notify-downgrade.sh` is
gone. And `notify-downgrade-messages.json` moves rather than survives in place: it
is script-dir-relative under `skills/design/inventory-design/scripts/`, which this
phase empties, but §3's drift test still needs an on-disk copy to `include_str!`
against the domain crate's `const` table. So the file is relocated to
`cli/design/tests/fixtures/notify-downgrade-messages.json` as part of this phase,
not left where it is — leaving it in place would contradict the Removal sweep's
"only `ensure-playwright.sh` and the retained JavaScript remain under
`skills/design/**/scripts/`" criterion, since the JSON is neither.

`EXPECTED_INJECTION_SKILLS = 42` (`tasks/lint/skill_permissions.py:48`) is an
equality, not a floor — but it moves only if a *skill* is added or removed, and
neither happens here.

#### 7. Documentation

**Files**: `docs-site/src/content/docs/design.md` (new),
`docs-site/astro.config.mjs`, `README.md`
**Changes**: Registration point 11 — a page for the sub-binary, the sidebar
entry, the Concepts list, and an `ACCELERATOR_DESIGN_BIN` override row.

### Success Criteria

#### Automated Verification

- [x] Characterization tests written first, derived from the deleted bash suites
      through an enumerated migration checklist: every assertion in
      `test-design.sh:169-338` and `:368-430`, plus `test-validate-source.sh` and
      `test-notify-downgrade.sh`, maps to a named Rust test or is recorded as a
      deliberate drop with a reason. The checklist lands as a **committed artefact**
      in the phase's PR, not a scratch note, since it is the evidence nothing was
      lost; and each deliberate-drop row must name the replacement property or state
      explicitly that none survives, because "recorded with a reason" is otherwise
      self-certifying. `test-validate-source.sh` is the sharpest case: its assertions
      are over shell *functions* (`canonicalise_host`, `classify_internal`) with label
      vocabulary the `HostReach` model restructures, so many rows land in that bucket
      by construction — the `RFC1918` stderr wording (`test-design.sh:205-208`) is
      called out as a label that must survive the restructuring. AC1's "at least one
      success and one failure path per subcommand" is the floor, not the plan — `validate-source` alone
      pins the RFC1918 boundary at both edges *and* both just-outside cases
      (differentiated by stderr content), IPv6 zone-id/mapped/wildcard/bracketed
      forms, decimal/hex/octal IPv4 encodings, the `user:pass@127.0.0.1@evil.com`
      userinfo class, `about:blank` acceptance, and unknown-flag exit 2
- [x] Every migration-checklist row for `validate-source` and `scrub-secrets` — the
      SSRF boundary and the credential-scrubbing front door — is **shown to fail
      when its property is broken**, not merely present: the same mutation-style
      proof the Removal sweep requires of its structural re-homes, applied here to
      the ~200 lines of behavioural assertion this phase deletes rather than
      re-homes
- [x] A table-driven `host_reach` test covers every **newly**-rejected encoding, since
      the migration checklist by construction only demands tests for behaviour the
      shell already had: `::ffff:169.254.169.254`, `::ffff:10.0.0.1`, `fd00::1`,
      `100.64.0.1`, `0.1.2.3`, `127.0.0.01`, `2002:a9fe:a9fe::`, `2001:0:...` (Teredo,
      unwrapped by XORing its embedded 32 bits with `0xFFFFFFFF` per RFC 4380, not
      taken literally) and `64:ff9b::a9fe:a9fe`, plus a numeric-looking host that
      fails strict parsing being rejected rather than treated as a hostname.
      `0:0:0:0:0:0:0:1` is the fully-expanded form of `::1` and is **not** in this
      list — it is `Loopback`, always accepted, and is covered instead by the
      widened-loopback-set test the `HostReach` section names
- [x] A dedicated `leaked_credentials` test covers the value-half split, the one
      genuinely new behaviour among the ported subcommands with no shell equivalent
      to derive a migration-checklist row from — the same treatment `host_reach`
      gets above and for the same reason: a `Name: token` header value (e.g.
      `ACCELERATOR_BROWSER_AUTH_HEADER=Authorization: Bearer abc123`) matches an
      artefact containing only `abc123`; the report names the variable and never the
      value; the header name alone does not false-positive
- [x] The downgrade goldens are exhaustive by construction — the test iterates the
      reason enum, so a variant without a golden fails — replacing
      `test-notify-downgrade.sh`'s message-key/fixture set-equality check
- [x] `mise run cli:check` exits 0 (including the new pup rules)
- [x] `cargo nextest run -p accelerator-design -p design -p design-adapters`
      passes
- [x] `mise run lint:dispatch-coherence:check` exits 0
- [x] `mise run test:unit:tasks` passes with the updated registry pins
- [x] `mise run test:integration:config` passes with the updated `EMITTERS` array
- [x] `mise run deny:check` exits 0
- [x] `mise run docs:check` exits 0
- [x] `mise run` exits 0 end to end

#### Manual Verification

- [x] `accelerator design validate-source https://example.com` exits 0;
      `http://example.com` exits 1; `--allow-insecure-scheme` flips it to 0
- [x] `accelerator design validate-source 0x7f000001` exits 1 with the numeric
      IPv4 message, and no flag bypasses it
- [x] `accelerator design scrub-secrets /nonexistent` exits **2**, not 1
- [x] Each ported downgrade reason reproduces its existing golden fixture byte for
      byte — except `executor-ping-failed`, whose remediation text names `run.sh` and
      is rewritten in Phase 6 §5, the phase that deletes it
- [ ] Both design skills run end to end in a live session

---
---

## Phase 3: Retire the two metadata scripts

### Overview

Route both skills' frontmatter provenance through `corpus metadata derive` and
delete the scripts. Depends on Phase 1.

### Changes Required

#### 1. Skill call sites

**Files**: `skills/design/inventory-design/SKILL.md`,
`skills/design/analyse-design-gaps/SKILL.md`
**Changes**: Replace the `inventory-metadata.sh` / `gap-metadata.sh` invocations
with
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus metadata derive --filename-timestamp-format compact-time`.
`allowed-tools` gains
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus metadata derive *)` — the exact
subcommand invoked, not the whole `corpus *` surface. Nine other skills already
declare the exact subcommand they call rather than the sub-binary's full surface,
and there is no reason for these two to grant more.

#### 2. Deletion

**Files**: `inventory-metadata.sh`, `gap-metadata.sh`, `scripts/test-metadata-helpers.sh`
**Changes**: Deleted — but **not before** a characterization golden records what they
emit. Phase 1 pins the timestamp component byte-for-byte against a pure renderer, and
that is the only line covered: the claim that corpus "already emits the same four lines
with the same labels in the same order" is verified nowhere automatically, since the
binary-level golden pins shape only and the label/order equivalence sits under manual
verification. So the four-line output for a fixed clock and repo fixture is recorded as
a golden first, and `corpus metadata derive --filename-timestamp-format compact-time`
is asserted to reproduce its labels and ordering, with the five divergences below as
the only permitted differences. AC15 is then read as byte-for-byte on the timestamp and
label/order-equivalent on the rest, which is what is actually true.

**Both scripts are driven by a bash suite, and it moves the `scripts/` floor.**
`scripts/test-metadata-helpers.sh:22-23` names both scripts in its `HELPERS` array and
asserts their output contract in hermetic git and jj temp repos. It runs in two lanes —
as the `test:unit:templates` driver (`tasks/test/unit.py:41`) and glob-discovered by
`test:integration:config` — so this phase deletes the suite too, drops the driver line
from `tasks/test/unit.py:41`, and adds `mise run test:unit:templates` (with the driver
removed) to the automated criteria below. The property the suite pins already lives in
`corpus-adapters/tests/metadata.rs:100-147` and `corpus-cli/tests/metadata_goldens.rs`,
so nothing needs porting — the suite is a pure deletion, not a re-home.

Deleting it moves `_EXPECTED_CONFIG_SUITES` from today's 16-discovered-against-a-floor-
of-15 to **15 against 15** as this plan's own exit state — one suite and one phase
earlier than the Removal sweep previously assumed. The floor itself does not move here
(15 ≥ 15 still holds); it is the *sibling* plan's later deletion of `test-design.sh`
that will need to move the floor, from 15 to 14, since that deletion happens after this
plan's. See the Removal sweep's corrected arithmetic.

### Divergences to accept explicitly

Five places where the bash scripts and corpus genuinely differ, all improvements
or immaterial to frontmatter provenance:

- `jj log -r @` snapshots the working copy; `jj_revision` never does.
- The bash branches gate on `command -v jj` / `git`; corpus needs neither.
- `git rev-parse HEAD` on an unborn HEAD aborts the whole bash script under
  `set -e`; corpus omits one line and exits 0.
- Outside a repository the bash script exits 1 (the trailing
  `[ -n "$REPO_NAME" ] && echo` is its last command); corpus exits 0.
- Neither reads its two timestamps atomically.

The last two are the only observable changes for a skill, and both are strictly
better: a skill invoked outside a repository no longer sees a non-zero exit for
what is not an error.

### Success Criteria

#### Automated Verification

- [x] Failing test first: the four-line characterization golden lands before the
      scripts are deleted
- [x] `mise run test:integration:config` passes (skill frontmatter conformance),
      with `scripts/` discovering 15 suites against a floor of 15
- [x] `mise run test:unit:templates` passes with the `test-metadata-helpers.sh`
      driver removed
- [x] `mise run lint:dispatch-coherence:check` exits 0
- [x] No metadata script remains in `analyse-design-gaps/scripts/` (`audit-cue-phrases.sh`
      in the same directory is Phase 2's to remove, not this phase's)
- [x] `mise run` exits 0

#### Manual Verification

- [x] Both skills produce frontmatter with a correctly shaped filename timestamp,
      revision, and repository name
- [x] Running the inventory skill outside a repository does not fail

---
---

## Phase 6: `run.sh` → Rust, behaviour-preserving

### Overview

Reproduce the Playwright launcher in Rust so the chain is CLI → Node with no
shell in between (ADR-0058). This phase changes *nothing* about where the
runtime comes from: it still resolves the existing lockhash namespace and still
requires system Node. That keeps it independent of Phases 4 and 5 and makes it a
pure, characterization-testable port.

### Changes Required

#### 1. The executor subcommand

**Files**: `cli/design/src/executor/` (new), `cli/design-adapters/src/process.rs`,
`cli/design-cli/src/executor.rs`
**Changes**: `accelerator design executor <command> [json-args]`, forwarding args to
`run.js`.

**The logic goes in the domain crate, behind ports.** This is the most
regression-prone code in the plan — ADR-0058 warns these 203 lines "encode
hard-won fixes … each regresses silently if the port misses it" — and ADR-0053 puts
no business logic in the command layer. Placing the reuse decision, the tolerance
comparison, the state interpretation, the lock policy, the poll deadline and the
envelope/exit-code taxonomy in `design-cli` would put the riskiest code where it is
hardest to exercise, exempt from the domain-purity pup rule. It would also make AC2
unsatisfiable, since AC2 requires the volatile inputs to be "supplied through
injected ports so the output is deterministic by construction rather than by
normalisation".

So `cli/design/src/executor/` holds pure functions over injected ports:

- `Clock` — now, and the poll deadline arithmetic.
- `ProcessProbe` — liveness, and the start time for a pid, as
  `ObservedStartTime::{Known(u64), Unavailable}` so the container case where `/proc`
  is unreadable is a value the domain matches on rather than an adapter-side panic.
- `StateStore` — returns a domain value, not raw JSON: `RecordedState::{None,
  PidUnparseable, Daemon(RecordedDaemon)}`, where `RecordedDaemon { pid, start_time:
  RecordedStartTime }` and `RecordedStartTime::{Probe(u64), Wallclock(u64),
  WriterUnavailable, AbsentOrUnparseable}`. `None` is the case `run.sh:106` checks
  for directly (`server-info.json` and `server.pid` both absent); `PidUnparseable`
  is a present-but-empty pid field (`run.sh:107`'s `tr -cd '0-9'` yielding nothing)
  — the two states a single `NoPidRecorded` label would otherwise conflate.
  `RecordedDaemon` reads pid and start time from `server-info.json` alone: that file
  is the one value published by one atomic rename, whereas `server.pid` is a second,
  independently-renamed file (`state.js:63-66`) that a reader can observe between the
  two writes — `server.pid`'s continued presence is a compatibility artefact this
  port does not read. `WriterUnavailable` is the write-side counterpart of
  `ObservedStartTime::Unavailable`, for when the launcher's own probe cannot read a
  start time to hand to the daemon (§3). The design crate cannot parse JSON (its pup
  rule permits only std, `kernel::Error` and `crate`), so the parse happens in the
  adapter — but §3's decisions turn on parse *outcomes*, and a port whose return type
  is unstated cannot carry them.
- `Lock` — acquire, and release.
- `Spawner` — start the daemon. Signalling is a separate `ProcessControl` port;
  bundling creation with signalling made one port do two unrelated things.
- `RunClient` — run the client and never return control to the caller on success:
  `fn run(self, cmd: Command) -> kernel::Error` (equivalently `-> Result<Infallible,
  _>`), because the adapter's terminal implementation is `CommandExt::exec`, which
  replaces the process image and cannot come back. Typing the port as diverging
  makes the constraint visible at the call site: no domain logic — lock release,
  envelope rendering, exit-code mapping — may be sequenced *after* this call, because
  it would run in a test's fake (which returns) and never in production (where
  `exec` does not). This is the seam the previous draft omitted, and it covers the
  behaviour most visible to consumers: `SKILL.md:142-143` discriminates on the
  launcher-versus-daemon exit asymmetry, and without a port that asymmetry can only
  be tested end to end.
- `PathResolution` — the plugin root, the state-dir namespace root and the bootstrap
  log path. `run.sh` derives all three from `BASH_SOURCE` (`:5-6`), so it always
  finds `run.js`, `package-lock.json` and the plugin root; a dispatched sub-binary
  executes from the launcher's cache directory and cannot. The only mechanism this
  workspace has is `ACCELERATOR_PLUGIN_ROOT` (`config-adapters/src/store.rs:204`),
  so the port reads that and refuses with a named error when it is unset — the
  failure mode an `ACCELERATOR_DESIGN_BIN` override would otherwise hit silently,
  with no diagnosable message. Both path-bearing envelopes
  (`playwright-not-installed`'s `$NS_ROOT`, `daemon-start-timeout`'s
  `$BOOTSTRAP_LOG`) resolve through this port, so AC2's "supplied through injected
  ports" requirement and the byte-identical-JSON criterion are satisfiable by
  construction rather than by convention.

`run.sh` itself has no free-space check — `disk-floor-not-met` is emitted by
`ensure-playwright.sh`, which survives this plan — so there is no `FreeSpace` port
here; a port with no caller in this phase would be dead code until the sibling plan
wires it up.

`design-adapters` implements them (`/proc` and `ps` parsing, filesystem, spawn), and
`design-cli` is argument parsing plus composition — the same shape
`work`/`work-adapters`/`work-cli` already uses. The reuse verdict then becomes a pure
function of (recorded pid, recorded start time, observed liveness, observed start
time), which is what makes the six characterization behaviours below deterministic
rather than dependent on real processes and real elapsed time.

It must be **total** over those inputs, including the cases `run.sh` never named:

The input is modelled as `(RecordedState, ObservedDaemon)` where
`ObservedDaemon::{Live(ObservedStartTime), Absent}`, so the match is total by
construction rather than by enumeration:

| Recorded | Observed | Verdict |
|---|---|---|
| `Daemon(Probe(r))` | `Live(Known(o))`, `\|r-o\| ≤ 1` | reuse |
| `Daemon(Probe(r))` | `Live(Known(o))`, otherwise | stale → recover |
| `Daemon(Probe(_))` | `Live(Unavailable)` | reuse on liveness alone |
| `Daemon(Wallclock(_))` | `Live(_)` | reuse on liveness alone |
| `Daemon(WriterUnavailable)` | `Live(_)` | reuse on liveness alone |
| `Daemon(AbsentOrUnparseable)` | `Live(_)` | stale → recover |
| any | `Absent` | stale → recover |
| `None` | any | stale → recover |
| `PidUnparseable` | any | stale → recover |

**Recovery signals on no row.** Every `stale → recover` row removes the state files
and respawns *without* signalling anything — including the mismatch row, where a
recorded start time contradicts a live pid's observed one. That might look like the
one case where signalling is safe, since the identity is "validated and
contradicted," but the contradiction proves only that the live process is *not* the
recorded daemon; it says nothing about what that process actually is, and
`run.sh` never signals during recovery for any row (`:120`, `:156` delete state
files unconditionally; the only `kill -TERM` anywhere in the script is `:191`,
aimed at the launcher's own just-spawned child during the daemon-start timeout — a
separate mechanism, `ProcessControl`, entirely disjoint from this table). So
signalling here would be new behaviour the port introduces, and it would send
SIGTERM to whatever process now owns a possibly-recycled pid — an editor or a
build, on a developer machine. A test asserts no signal is delivered on every
recover row.

The three liveness-only rows exist because `/proc` being unreadable, or the writer
having recorded a wallclock or writer-unavailable value, defeats the PID-recycle
guard identically: none of the three carries a validated start time to compare
against, so all three reuse on liveness alone rather than mismatching. `run.sh:55`
answers the `Unavailable` case the same way — which is what stops the daemon
respawning on every command and losing page state in precisely the containers AC6
and AC11 exercise. A record with no `start_time_source` key at all is read as
`Wallclock`, not `Probe`: today's `state.js:40-61` falls back to
`Math.floor(Date.now()/1000)` on *any* failure — not only a non-linux/darwin
platform, but an unreadable `/proc` or a failing `getconf` under load — so a
pre-upgrade record's provenance is genuinely unknown, and reading it as a kernel
probe value would hold it to the ±1s tolerance on the strength of a guess. Reading
it as `Wallclock` accepts the same conservative consequence as the other
provenance-uncertain rows: a legacy daemon reuses on liveness alone, with no
PID-recycle guard, until it is next respawned. "Accepted once" is replaced by these
rules, because a fresh launcher process per invocation has nowhere to record that
the once was spent.

**Reserved internal tokens are rejected, not merely unnamed.** §3 claims
`design executor daemon` "is not reachable" because "the Rust surface does not expose
it", but verbatim forwarding *is* exposure: `run.js:18` dispatches on
`args[0] === 'daemon'` and `:20` takes the state dir from
`ACCELERATOR_PLAYWRIGHT_STATE_DIR`, which the executor sets on every path. A single
stray `accelerator design executor daemon` would start a second foreground daemon
that binds a fresh port, overwrites `server-info.json` and `server.pid` for the live
one, orphans it mid-crawl, and never returns — the failure `run.sh:191`'s
kill-on-timeout exists to prevent. So argument validation rejects `daemon` and any
other internal `run.js` subcommand by an explicit allowlist of forwardable commands,
with a test.

Behaviour ported from `run.sh`:

- **Start-time identity** (`run.sh:13-42`). Linux: read `/proc/<pid>/stat`, take
  everything after the last `)` (greedy, so a `comm` containing parens is safe),
  field 20, plus `/proc/stat`'s `btime`, divided by `CLK_TCK` with **truncating
  integer division** — `run.sh:25`'s shell arithmetic and `state.js:49`'s
  `Math.floor` both truncate, and floating-point or `(btime * hz + ticks) / hz`
  differs by up to a second, consuming the entire ±1s budget the tolerance exists to
  provide for whole-second-boundary drift. Fixture-pinned against
  `lib/__fixtures__/proc-stat-linux.txt` → `1700145620`, plus a second fixture whose
  tick count does not divide evenly.

  Darwin: **not** `ps -p <pid> -o lstart=`. That emits a wall-clock string with no
  offset, which both `run.sh:38` and `state.js:57` resolve in local time — so
  converting it needs the UTC offset *at that instant*, and the workspace's `time`
  crate is featured `["parsing"]` only (`cli/Cargo.toml:25`), with `local-offset`
  being a separate feature whose `current_local_offset()` returns
  `IndeterminateOffset` in a multi-threaded Unix process. Worse, during the repeated
  hour of a DST fall-back the string maps to two distinct instants, and the port
  would have to resolve that ambiguity identically to V8's parser or be wrong by up
  to 3600s against a ±1s tolerance.

  So Rust reads `sysctl` `KERN_PROC_PID`'s `p_starttime`, which is epoch-based. This
  is not a second implementation to write: `cli/visualiser/server/src/server.rs:527`
  already implements `/proc` on Linux and `sysctl` `KERN_PROC_PID` on Darwin, with
  the macOS ABI constants pinned and a test suite covering both.

  **The destination is a new crate, not `design-adapters` depending on the
  visualiser.** `visualiser/server` is an application crate carrying `axum`, `tokio`
  (`rt-multi-thread`), `tower-http`, `notify` and `rust-embed` as non-optional
  dependencies — pulling that whole graph into `design-adapters` to reuse a ~70-line
  function would work directly against the hot-path concern this phase already
  measures (the sub-binary resolve-and-verify budget) and against binary size on
  every release cross-compile. So the function is extracted into a new
  `process-probe` crate depending on `libc` alone, `visualiser/server` is repointed
  at it (removing its own copy), and `design-adapters::process::probe` wraps it via
  `ProcessProbe`. This is a third open fork alongside the two already named at the
  end of "Implementation Approach," with the crate-creation edit landing in
  Registration (Phase 2 §4) so the `[workspace].members`/`Cargo.lock` diff is
  accounted for. Nothing new joins `cli/Cargo.toml`'s `deny.toml` or pup surface,
  since `libc` is already a workspace dependency.

  This phase does not otherwise duplicate concepts: `visualiser/server`'s
  `orchestration/process.rs` implements `identity_matches`/`terminate` — the same
  "daemon identity via pid and start time" shape this phase re-derives as
  `ProcessProbe` and the reuse-verdict table — and was compared against rather than
  built past. The two differ enough to justify separate models rather than a shared
  one: `identity_matches` treats an unreadable expected start time as an
  unconditional mismatch, where this phase's `Wallclock`/`WriterUnavailable` rows
  deliberately reuse on liveness alone for provenance-uncertain records, and
  `terminate`'s SIGTERM-then-SIGKILL escalation has no equivalent here because
  recovery never signals (§1, above). Only the epoch-read primitive is shared; the
  identity semantics built on top of it are not, and are not meant to be. **`state.js`
  stops computing the start time at all.** Node has no `sysctl` binding and `sysctl(8)`
  prints `kern.proc.pid.<n>` as an opaque struct dump with no parseable epoch field, so a
  JS half would have no mechanism on Darwin — and the fallback it would take is
  `Math.floor(Date.now()/1000)`, the weakest path, for every macOS daemon on the primary
  development platform.

  **One observer, one writer, one atomic write, handed over a pipe.** The launcher
  spawns the daemon and learns its pid the moment `spawn()` returns — but that is not
  early enough to pass the pid or its start time *through the environment*: on Unix
  the child's `envp` is fixed at `execve`, and `std::process::Command` builds that
  `envp` before `fork`, so a value only knowable once the child exists cannot travel
  through it. Only the token, generated pre-spawn, could have used that route; the
  pid and start time cannot.

  So the launcher opens a pipe before spawning. In `pre_exec` (running in the forked
  child, before `exec`), it `dup2`s the pipe's read end onto a fixed fd number and
  clears `O_CLOEXEC` on that duplicate so it survives into the exec'd `node`
  process; the pipe's write end is opened `O_CLOEXEC` from the start, so the
  child's inherited copy of it closes automatically, as a side effect of the kernel
  performing `exec`, leaving the daemon holding only the read side. The fixed fd
  number is passed to the child via a new environment variable,
  `ACCELERATOR_PLAYWRIGHT_IDENTITY_FD`, alongside `ACCELERATOR_PLAYWRIGHT_STATE_DIR`
  — Node's `fs.createReadStream` (or a raw `fs.readSync` loop) can read an arbitrary
  inherited fd number given to it this way, no Node-specific IPC channel required.
  The wire format is four newline-delimited fields in a fixed order — `pid` and
  `start_time` as decimal integers, `start_time_source` as a single tag byte
  (`p`/`w`/`u` for probe/wallclock/writer-unavailable), `token` as a fixed-length hex
  string — pinned by a fixture-backed round-trip test, the same rigor the plan
  already applies to every other cross-language boundary it touches.

  Once `spawn()` returns the child's pid, the launcher observes the start time with
  the same probe it will later use to check it, writes the four fields down the
  pipe's write end, and closes its own copy of that write end immediately after the
  write. With the child's inherited copy already closed by `O_CLOEXEC` at `exec` and
  the launcher's own copy now closed too, no writable copy of the pipe remains
  anywhere, so the daemon's read reaches EOF deterministically rather than blocking
  — a test asserts this directly.

  **A launcher crash between spawn and the write is a daemon-side failure path, not
  an unhandled gap.** If the launcher is killed after `spawn()` returns but before it
  writes, the child's inherited write-end copy is still closed at its own `exec`
  (that closure doesn't depend on the launcher surviving), so the daemon's read
  still reaches EOF — immediately, with no data. The daemon treats a short or empty
  read identically to any other malformed input: it logs the failure and exits
  **before creating any Playwright/Chromium process and before opening its
  listening socket**, rather than proceeding with partial or default values. This is
  what stops an already-`setsid`-detached daemon from being left running,
  unsupervised and un-recorded, if the launcher dies mid-handoff — a characterization
  test pins the crash case alongside the normal one.

  The daemon's first action — before `state.js`'s `atomicWrite` and before it opens
  its listening socket — is to read the fd named by
  `ACCELERATOR_PLAYWRIGHT_IDENTITY_FD` to end of input and parse the four fields;
  only once they parse successfully does it publish the identity record and become
  ready. Readiness is ordered explicitly by this: `server-info.json` cannot appear
  before the daemon holds the values that go into it.

  This matters more than it looks. An earlier draft had the launcher *write* the record
  itself "once the daemon reports ready", which opens a window: readiness **is**
  `server-info.json` appearing, so a launcher killed between the daemon binding and the
  record landing leaves a live daemon whose record has no start time — which §3's own
  rule (`AbsentOrUnparseable` → recover) turns into deleting the state and spawning a
  second daemon while the first still holds a port, a browser and the crawl's page state.
  It also gave one file two whole-file-rename writers, which is a lost-update contract,
  and left the token unenforceable until after the daemon was already accepting
  connections. Handing the values to the daemon over the pipe before it publishes
  anything removes all three: the record is never partial, `server-info.json` keeps
  exactly one writer, and the daemon can require the token from its first accepted
  connection.

  ADR-0058 names this contract as the port's principal silent-regression risk. The
  inversion's real benefit is that the *probe* has one owner in one language; that is
  preserved, without moving a field out of the atomic unit it belonged to.
- **±1s tolerance** (`run.sh:52-59`) for a present, numeric expected value.
- **Double-checked reuse short-circuit** around the lock (`run.sh:101-121` and
  the re-check at `:139-160`) — but see §3: the pre-lock check becomes read-only.
- **Lock**: one implementation, not two. The `flock`-or-`mkdir` dichotomy exists
  because the `flock(1)` **binary** is absent on macOS — a constraint that vanishes
  in Rust, where `flock(2)`/`fcntl` is available on every supported target, and
  ADR-0058 already records that nothing external depends on the mkdir form. So
  `ACCELERATOR_LOCK_FORCE_MKDIR` and the mkdir backend are dropped unless an NFS
  requirement is found, in which case the plan records it; the `owner.<nonce>`
  sentinel protocol is not used either way.

  **The lock is released at launcher exit**, by a `Drop` guard, on every path. This
  resolves a contradiction in the previous draft, which claimed both "held for the
  daemon's lifetime where `run.sh` inherits the flock FD into the child" *and* "a
  `Drop` guard releases it on every path" — the negation of each other. The two
  backends genuinely differ today: `run.sh:126` leaks FD 9 into the daemon
  (confirmed by `test-run.sh:160-163`, which records that it "holds the launcher.lock
  FD across its shutdown, so the next launcher invocation is blocked until then"),
  while `run.sh:152,202` explicitly `rmdir`s the mkdir lock before `exec`. One guard
  cannot reproduce both, and Rust opens files `O_CLOEXEC` by default, so an
  unremarked port would silently drop the inheritance anyway. Releasing at exit is
  the more defensible of the two: holding it for the daemon's lifetime means a
  stale-start-time recovery while the daemon still lives falsely reports
  `another-launcher-running`. **This is a deliberate behaviour change**, so it moves
  to §3 and the contention test asserts the new semantics.

  On the `RunClient` path specifically, `exec` never returns, so no destructor runs
  — the lock is released by an explicit step immediately before that call, mirroring
  `run.sh:152,202`'s explicit `rmdir` before its own `exec`, and a criterion asserts
  the lock is observably free after a successful command. The `Drop` guard is the
  backstop for every other path: usage errors, refusals, and any exit that returns
  from `main` normally.
- **State dir** at `<repo>/<config path tmp>/inventory-design-playwright`, mode
  0700, with the `accelerator config path tmp` shell-out replaced by an
  in-process `config` call.
- **Lockhash namespace** at `${CACHE_ROOT}/$(sha256(package-lock.json) | cut -c1-8)`
  (`run.sh:86-92`), read via `PathResolution` rather than recomputed independently.
  `ensure-playwright.sh:50-60` — which survives this plan and is the only thing that
  populates the namespace — computes the digest the same way, and the port removes
  `sha256sum`/`shasum` from this path, so Rust must reproduce the exact byte sequence:
  lowercase hex, first 8 characters, hashing the same `package-lock.json`, with no
  trailing-newline or path-resolution difference from the shell's. A fixture-pinned
  criterion asserts the Rust digest equals `sha256_of` `ensure-playwright.sh`'s
  function for the shipped lockfile, cross-checked against the shell function while
  it still exists, with the shipped file's digest recorded as a golden. An
  off-by-anything here makes the executor look in a namespace the surviving
  bootstrap never fills, so every invocation returns `playwright-not-installed` at
  exit 3 on a correctly-bootstrapped machine.
- **Daemon spawn** and the 30s start poll with kill-on-timeout (`run.sh:163-194`).
  Three properties come from shell primitives the port removes, and each must be
  reproduced explicitly rather than inherited. `nohup … &` plus `disown` makes the
  daemon SIGHUP-immune and reparented — so the spawn calls `setsid` in `pre_exec` on
  the single spawned child, or a Ctrl-C in an interactive session kills the daemon
  mid-crawl. **Not a double fork**: `run.sh:169-173` spawns the daemon as a direct
  child, so `$!` is the daemon's pid, which `:191` uses for kill-on-timeout and which
  §1's identity handoff needs to observe a start time for. `setsid` before `exec`
  preserves that pid; a double fork does not, because the launcher's direct child
  forks the daemon and exits, and the launcher never learns the grandchild's pid. If
  `setsid` itself fails, the spawn reports an error rather than falling back to a
  double fork.
  `>>"$BOOTSTRAP_LOG" 2>&1` redirects the daemon's stdio away from the caller's — so
  the spawn redirects to the bootstrap log rather than inheriting. And the final
  `exec node run.js "$@"` means the client's exit status, stdout, stderr and
  signal-death *are* the launcher's, with no forwarding logic — so the client path
  either uses `CommandExt::exec` or propagates exit status and signal death (128+n)
  explicitly. `SKILL.md:142-143`'s error discrimination and the launcher-vs-daemon
  exit-code asymmetry both depend on that, and all three regress silently, which
  ADR-0058 names as the port's principal risk.
- **Environment handed to the child**: `ACCELERATOR_PLAYWRIGHT_STATE_DIR`,
  `NODE_PATH`, `ACCELERATOR_PLAYWRIGHT_NS_ROOT`, and the new
  `ACCELERATOR_PLAYWRIGHT_IDENTITY_FD` (§1) naming the inherited pipe's read end.
- **Four behaviours an earlier draft's inventory missed**, each observable and each
  the kind of thing ADR-0058 warns regresses silently. `run.sh:71-75` emits a
  `no-repo` 3-key envelope on stderr and exits **2** when `find_repo_root` finds
  nothing — a distinct usage outcome the state-dir bullet does not cover.
  `run.sh:159` removes `server-stopped.json` before every spawn, so a stale stop
  reason from a previous daemon's idle or wall-clock shutdown is not left for a later
  reader (`test-run.sh:224` reads it) — without it a failed start looks like a
  completed shutdown to anyone diagnosing a daemon that never came up. `run.sh:163-165`
  truncates the bootstrap log and `chmod 0600`s it before the daemon writes, which is
  what keeps the timeout envelope's "Check $BOOTSTRAP_LOG" pointing at *this* attempt's
  output, under a `umask 077` the Rust binary does not inherit. And that `umask 077`
  (`run.sh:3`) is inherited by the *daemon* too, not only the launcher's own writes —
  so every file the daemon creates without an explicit mode, notably
  `page.screenshot()` output (`daemon.js:221-233`), lands `0600` today; the spawn calls
  `umask(0o077)` in `pre_exec` on the child as well as for the launcher's own writes,
  with a criterion on a daemon-written screenshot's mode. The inventory is derived by
  a line-by-line pass over `run.sh`, the way Phase 2 §6 derives its cut list from
  `test-design.sh`'s ranges, rather than from memory.

The port removes runtime dependencies on `jq`, `flock`, `sha256sum`/`shasum`,
`nohup`, `sed`/`awk`/`tr`/`date`, and the bash 3.2 floor on this path.

**The adapter-level behaviours need a harness that is not Playwright.** `setsid`,
stdio redirection, log truncation and mode, exit-status and signal propagation, and
`server-stopped.json` removal are all properties of *spawning a child process*, not
of the Playwright automation the child happens to run — so they are exercised in
`design-adapters`' integration tests against a **stub** child binary rather than the
real `run.js`: a small fixture executable, built once for the test suite, that
prints a marker to stdout and stderr, exits with a code the test chooses, can be
told to die on a chosen signal instead, and can be told to never write
`server-info.json` at all, and — this is the property ADR-0058 names as the port's
principal risk, so the stub covers it rather than leaving it implicit — can be told
to read the fd named by `ACCELERATOR_PLAYWRIGHT_IDENTITY_FD` and echo what it reads
before deciding whether to signal ready. Against that stub: `setsid` is asserted by
checking the child's process group differs from the test's; redirection by asserting
the launcher's own stdout/stderr are untouched while the bootstrap log receives the
child's; log mode and truncation by writing a sentinel into the log first and
asserting it is gone and the mode is `0600` after the next spawn; exit-status and
signal-death propagation by asserting the launcher's own exit code matches the
stub's chosen one, including the 128+n convention for the signal case; timeout kill
and contention by combining the never-ready stub with the injected `Clock`/`Lock`
already described above; and the identity handoff itself by asserting the stub's
echoed values match what the launcher wrote, that the launcher's read-side close
of its own write-end fd lets the stub's read terminate rather than block, and that
a stub told to never write `server-info.json` still received the four values before
the test times out waiting for it — the no-partial-record property (§1) proven
without a real daemon. None of this requires Playwright, a browser, or a network
namespace, so it runs in the default CI lane rather than the opt-in one.

#### 2. Behaviours preserved deliberately

Four asymmetries the port keeps, each because a consumer depends on it:

- **Daemon-side errors exit 0** with the envelope on **stdout**; launcher-level
  failures go to **stderr** with a non-zero exit. `SKILL.md:142-143` discriminates
  on exactly that asymmetry, so collapsing it breaks the skill.
- **Launcher envelopes stay 3-key** (`error`, `message`, `category`) — no
  `protocol`, no `retryable`, unlike everything `errors.js:10` produces. AC2's
  byte-identical assertion pins this.
- **Exit codes 0, 1, 2, 3** keep their current meanings, including exit 3
  (`playwright-not-installed`) — the runtime still has to be bootstrapped in this plan,
  so the condition it names still arises. The sibling plan redefines it into an
  `artifact-unavailable` downgrade and moves `PROTOCOL.md:555-566`'s exit-code table
  with the vocabulary.

**The executor gets its own outcome type, not a stretched `Verdict<Reason>`.** Phase 2
§3's carrier has two variants (`Accepted`→0, `Rejected`→1) plus
`kernel::Error::Refusal`→2 — enough for a subcommand with one accept/reject axis, but
the executor has four *kinds* of outcome: a daemon-side envelope on stdout at exit 0,
a launcher-side envelope on stderr at an exit the envelope's `category` does not
determine (`another-launcher-running` and `daemon-start-timeout` are both
`category: "usage"` yet exit **1**, because both are outcomes the tool evaluated and
rejected, not malformed invocations — the exit-2-means-usage rule the ported
subcommands follow does not hold here), `no-repo` at exit 2, `playwright-not-installed`
at exit 3, and the client pass-through's arbitrary exit status and signal death via
`RunClient`, which diverges rather than returning at all. `ExecutorOutcome` is
therefore a separate type from `Verdict<Reason>`, with a table in `design-cli`
pinning each named envelope to its exit code so the `category`/exit-code divergence is
recorded rather than inferred.

#### 3. Behaviours corrected deliberately

Four, each recorded so the change reads as taken rather than accidental:

- **An absent or unparseable `start_time` is now a mismatch, not a match.**
  `run.sh:108,142` extract `.start_time` via `jq`, swallow failure with `|| true`,
  and `start_time_matches` returns 0 for an empty expected value (`:54`) — so any
  live PID is accepted. Native JSON parsing removes one *cause* of an empty value (a
  missing `jq`) but not the branch: a `server-info.json` written without a
  `start_time` key — by an older daemon, by a truncated write, or by any future
  `state.js` change — still bypasses the PID-recycle guard entirely, which is
  precisely the state an interrupted or partially-migrated daemon leaves behind. So
  absent-or-unparseable is treated as stale state and triggers recovery. The ±1s
  tolerance applies only to a present, numeric value.
- **The lock releases at launcher exit on every path**, by a `Drop` guard — see §1.
  This both fixes the mkdir leak `test-run.sh:178-182` calls "a pre-existing run.sh
  quirk" and deliberately drops the flock FD inheritance into the daemon, unifying
  two backends that behave differently today.
- **The pre-lock reuse check is read-only.** `run.sh:106-121`'s first check runs
  *before* the lock and ends with an unconditional `rm -f "$INFO" "$PID_FILE"` when
  the daemon looks dead. But `state.js:63-66` writes those two files as separate
  atomic renames, so a launcher reading between them — or racing a daemon that has
  just written both — judges the state stale and deletes a **live** daemon's state
  files outside any lock. Two launchers starting concurrently can therefore orphan a
  healthy daemon and spawn a second, losing page state mid-crawl: the exact symptom
  the start-time contract exists to prevent. So the pre-lock check short-circuits on
  a hit or falls through, and the stale-state deletion happens only under the lock,
  where `run.sh:156` already does it.
- **Internal `run.js` subcommands are rejected by argument validation.**
  `run.sh daemon` is an unfiltered internal subcommand that starts a second
  foreground daemon which never returns. Not exposing it on the Rust surface is not
  enough, because §1 forwards arguments to `run.js` and `run.js:18` dispatches on
  `args[0]` — so the executor validates against an explicit allowlist of forwardable
  commands. A command added to `daemon.js` later is unreachable until the allowlist
  moves with it, which the `PROTOCOL.md` ↔ `daemon.js` sync assertion re-homed into
  the node suites (Removal sweep §1) is extended to cover.

Three more, in the retained JS.

**`state.js` stops computing the start time.** `:40-61` currently probes it and
`:60` falls back to `Math.floor(Date.now()/1000)` on any failure — reached on any
platform that is neither `linux` nor `darwin`, and on Linux whenever `/proc` is
unreadable or `execSync('getconf CLK_TCK')` fails, which is common in minimal,
distroless and `hidepid`-hardened containers. §1's pipe handoff makes the fallback
unnecessary: the launcher observes the start time itself, with the probe it will
later use to check it, and sends it to the daemon over the pipe before the daemon
publishes anything. `state.js` writes exactly what it reads off the pipe, plus the
port and readiness facts it owns; the `getconf` shell-out goes with the probe, and
the two implementations can no longer disagree because there is only one.

`start_time_source` survives as a field, but it is the *launcher's* probe result
relayed over the pipe, not something `state.js` computes or the launcher writes to
disk directly: `RecordedStartTime` gains `WriterUnavailable` for the case where the
launcher's own probe returns `ObservedStartTime::Unavailable` on the writing side.
The launcher sends that variant down the pipe rather than substituting a wallclock
value, and the verdict table (§1) reuses on liveness alone for it — the same
treatment as `Wallclock`, and for the same reason: neither carries a PID-recycle
guard, so treating it as `AbsentOrUnparseable` would recover, and therefore
respawn, on every subsequent invocation. A `start_time_source` value that is
present but neither `"probe"` nor `"wallclock"` — a corrupted record, or one
written by a daemon version the reading launcher doesn't recognise — maps to
`AbsentOrUnparseable`, consistent with the empty-pid case: the reader cannot
validate what it cannot interpret, so it treats the record as stale rather than
guessing at a provenance it was not told.

**The daemon gains a request token.** It serves JSON commands on `127.0.0.1` with an
OS-assigned port and no authentication (`daemon.js:286-338`), and loopback binding is
not a uid boundary — any local process, including anything the model runs, can drive a
browser that may hold the user's authenticated session, screenshot it, and `evaluate`
in its context. This lands here rather than as a follow-up because the writer, the
reader, the `StateStore` port and the client path are all already being rewritten in
this phase: the launcher generates a random token pre-spawn (a CSPRNG value of at
least 128 bits) and sends it down the same pipe as the other three identity values, so
`state.js` — the sole writer — records it in the already-`0700` `server-info.json`
alongside them, and `daemon.js` requires it via a constant-time comparison from its
first accepted connection. Deferring would mean migrating that file's format and both
its readers twice.

The threat this closes has two halves, and the token is ineffective against one of
them. The token file is `0700`, so a same-uid process — including the model — can
read it directly regardless of the check; against that attacker the token adds
nothing. What it does close is, first, a browser-origin attack: CSRF and DNS
rebinding from the pages the crawl itself visits, which could otherwise reach the
daemon's loopback port directly. Second — and this is worth stating explicitly
rather than folding into "any local process," which the narrowing above correctly
rejects — a loopback TCP socket is not uid-scoped on Unix, so a **different** local
user on a shared or multi-user host cannot read the `0700`/`0600`-protected token
file but could otherwise connect to the daemon's port directly and drive the
crawl's browser session; the token is the only control in this design that closes
that cross-uid case. That is also why the transport must stay header-only —
`client.js` sends the token as a header, never a query parameter, and the daemon
rejects any request carrying an `Origin` header at all, since a legitimate client
has no reason to send one. Three implementation details follow directly from this
threat model and are pinned as criteria, not left to the adapter's discretion: the
executor's forwarding must not let a caller-supplied `command`/`protocol` pair
override the validated ones (`{...args, protocol, command}`, not
`{command, protocol, ...args}`, and reject any payload that itself carries a
`command` or `protocol` key); `client.js` must validate `info.url` as loopback
before connecting — parsing the URL and checking the hostname against `127.0.0.1`
or `::1` via Node's `net.isIP`, rejecting anything that is not a literal loopback
address rather than a string comparison — so a `server-info.json` rewritten by
anything able to write to the state dir cannot redirect the token-bearing request
to an arbitrary host; and the daemon rejects a token presented as a query
parameter, honouring only the header. The token, any resolved auth-header value and
any `ACCELERATOR_BROWSER_*` credential must never be written to the bootstrap log —
the daemon's stdio is redirected there, and a future diagnostic added to `daemon.js`
must not become a second place these values can leak.

**`makeAuthHeaderHandler` stays in place** — imported at `daemon.js:11` and never
called, with `ACCELERATOR_BROWSER_LOCATION_ORIGIN` set nowhere. Wiring it up is new
feature work and deleting it removes a capability `SKILL.md:89-95,196` documents as
security-critical, so neither belongs in a behaviour-preserving port and the defect is
raised as a follow-up (Removal sweep §4). But the *documentation* is corrected in the phases
that rewrite it: both SKILL.md files and `resolve-auth`'s help text state that the
header-auth path is currently inert and that `ACCELERATOR_BROWSER_AUTH_HEADER` should
not be given a live credential until the follow-up lands. Leaving a known-false
security claim in a file this plan is editing anyway is not defensible.

#### 4. The locale regression guard

**File**: `cli/design-adapters/tests/start_time.rs`
**Changes**: `test-run.sh:44-63` sources `run.sh` and asserts `start_time_of`
under `LANG=C` equals it under `de_DE.UTF-8`. That guard covers the exact bug
ADR-0058 names and must survive in some form. The Rust equivalent asserts the
computed start-time is identical with `LANG`/`LC_ALL` set to each of `C`,
`de_DE.UTF-8` and unset. It does **not** additionally assert agreement with
`lib/state.js`: §1 removes the JS probe entirely, so there is one implementation
and nothing to agree with — the equality assertion the shell guard needed is
replaced by single ownership, which is the stronger property.

It must also cover an axis the shell version never had. Reading a kernel epoch value
(§1) is what removes the locale hazard, and the guard should also prove `TZ`-
independence rather than assume it: assert the computed value is identical across
`TZ` ∈ {unset, `UTC`, a half-hour-offset zone such as `Asia/Kolkata`}. A
`ps`-parsing implementation would fail those cases; a `p_starttime` one cannot. This
proves TZ-independence only, not immunity from a DST fall-back — a live process's
start time is a single fixed instant, so varying `TZ` around a *test run* cannot
exercise the ambiguous repeated hour the way it would for a *stored, TZ-dependent*
timestamp; that boundary case is the reason `ps -p <pid> -o lstart=` was rejected in
the first place (§1), not something this guard additionally proves. The Linux
branch additionally pins the `CLK_TCK` source inside a static musl binary, where
`getconf` may be unavailable.

#### 5. Dead JS removal

**Files**: `lib/identity.js`, `lib/lock.js` and their tests, `lib/daemon.js`,
`PROTOCOL.md`, `skills/design/inventory-design/SKILL.md`
**Changes**: `identity.js` and `lock.js` have no production callers — they appear
only in their own tests, and `identity.test.js:70-95` and `:99-118`
cross-validate against a `launcher-helpers.sh` that no longer exists, passing
silently via `catch { return; }`. Both modules and both suites are deleted.

Three further defects get an explicit disposition rather than a mention, so they are
neither silently skipped nor fixed ad hoc mid-phase:

- `daemon.js:341-347` opens `/dev/null` and does nothing with the fd — **deleted**.
- `PROTOCOL.md:317` and `:603` are stale against `path-guard.js:74` and
  `daemon.js:341` — **corrected**.
- `PROTOCOL.md:18-19`'s Transport section ("Loopback only... External callers cannot
  reach it") states the false claim the new token exists specifically to address —
  **corrected**, and a new Authentication subsection documents the header name, its
  generation, and that same-uid processes (unlike external callers) are not what it
  defends against — a different local user on a shared host, and browser-origin
  CSRF/DNS-rebinding from the pages the crawl visits, are. `PROTOCOL.md` is the
  document this repository's own header states exists "so agent bodies and future
  callers can stay in sync without reading the source," so a client built against it
  alone must not omit a now-mandatory header.
- `makeAuthHeaderHandler` is imported at `daemon.js:11` and never called —
  **left in place**, with the dead-path defect raised as the follow-up work item
  Phase 2 §1 names. Wiring it up is new feature work and deleting it removes a
  capability `SKILL.md:89-95,196` documents as security-critical; neither belongs in
  a behaviour-preserving port. The unused *import* is not removed either, because
  removing it would make the dead path harder to find later.

`SKILL.md` is also in scope here, and carries **two** call sites missing from earlier
drafts' file lists, not one. Step 5 (`:139`) invokes `scripts/playwright/run.sh ping`,
which this phase deletes; the step is repointed at `accelerator design executor ping`.
Step 12 "Cleanup" (`:299`) invokes `.../playwright/run.sh daemon-stop` as the
belt-and-braces shutdown "even if an agent exits abnormally" — the only deterministic
stop path, and the one this phase would otherwise leave pointing at a deleted file with
no CI gate catching it (`call_site_migration.py` greps only `scripts/config-`, and this
is a fenced block in a numbered step, not a `!`-preprocessor site). Left unrepointed,
every inventory run leaves a headless Chromium holding the crawled page until the
10-minute idle timeout. Step 12 and its `:302` prose are repointed at
`accelerator design executor daemon-stop` alongside Step 5. Migration Notes claims call
sites are rewired in the phase that deletes what they call, and until now that was
true of neither.

**A dangling-call-site guard closes the class, not just this instance.** The
re-homed node-suite assertions (Removal sweep §1) gain one more: no SKILL.md or agent
body may name a path under `skills/design/**/scripts/` that does not exist on disk, so
a call site left pointing at a deleted script fails CI at merge time instead of
leaking a browser at run time.

`executor-ping-failed`'s message is also rewritten **here**. It currently reads "Run
`run.sh ping` manually to diagnose", naming a file this phase deletes — so without the
rewrite the plugin would ship a diagnostic whose remediation cannot be followed, and
Phase 2's byte-for-byte pin on that message actively prevents fixing it early. That pin
is therefore relaxed to the messages whose text genuinely survives unchanged. It is
rewritten to name `accelerator design executor ping`, which is the diagnosis step that
now applies. `daemon.js`'s `chromium-not-found` message has the same defect — "Run
ensure-playwright.sh to reinstall" — but that script survives this plan, so its text
stays correct here and the sibling plan rewrites it alongside the path change it needs.

#### 6. Node suite runner

**Files**: `skills/design/inventory-design/scripts/playwright/package.json`,
`skills/design/inventory-design/scripts/playwright/test-run.js`,
`skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`,
`skills/design/inventory-design/scripts/playwright/daemon-runtime.test.js` (new),
`tasks/test/unit.py`, `mise.toml`
**Changes**: The retained suites have no runner anywhere. Add a
`test:unit:design-automation` task running `node --test` over them, wired into the
aggregate unit-test task, so AC1 and AC2 have CI-observable meaning.

**Scope and floors**, split by what each lane can honestly guarantee. `test-run.js` lives
at the `playwright/` root, so a `lib/`-only glob would silently exclude the suite that
exercises the daemon end to end — but it also cannot run without a bootstrapped runtime,
so it belongs to the opt-in lane rather than the unit one. Both floors are asserted in
the established style (`_EXPECTED_CONFIG_SUITES` and friends), with the deletions
recorded in the comment and the discovered list passed to `node --test` explicitly.
`playwright-loader.test.js` survives this plan; the sibling plan deletes it with the
loader and adjusts the floor then. The precise scope of each lane is stated below,
after the namespace-gating fix that determines it.

**A file-count floor is not sufficient here, and neither is a skip count.** Two retained
suites gate themselves on a bootstrapped runtime:

- `test-run.js:93-94` computes
  `playwrightInstalled = existsSync(ACCELERATOR_PLAYWRIGHT_CACHE || ~/.cache/accelerator/playwright)`
  and carries `skip: !playwrightInstalled` on **14** tests.
- `daemon.test.js:71-72` — and only this one test, `'ping returns ok: true without
  launching browser'`; the file's other four forking tests (protocol-mismatch,
  daemon-status, daemon-stop, idle timer) don't touch Playwright at all and run
  today with no gate — resolves the real namespace via `resolvePlaywrightNsRoot()`
  and returns `null` if it can't find an installed `playwright` package, which the
  test turns into a bare early `return` **inside the test body**.

That second shape is the important one: `node --test` reports an early `return` as
**passed**, not skipped — the identical pattern this plan condemns in
`identity.test.js:70-95` (`catch { return; }`) — and a naive whole-file grep for it
would itself false-positive on legitimate helper-function returns already in these
suites (`test-run.js:31`'s `if (existsSync(filePath)) return;` inside the
`waitForFile` polling helper, well outside any test body). So this needs two
separate fixes, not one grep: **extraction** for the one gated test, and a
**narrower guard** for the general pattern.

**Extraction, named explicitly.** The `ping` test is moved out of
`daemon.test.js` — which otherwise stays entirely in the unit lane, unmodified —
into a new file at the `playwright/` root (not under `lib/`, for the same reason
`test-run.js` lives there and not in a `lib/`-only glob): **`daemon-runtime.test.js`**.
This is also where the `test-run.sh` `links` block lands (below): both are
runtime-dependent daemon tests, and both belong outside the unit lane's `lib/*.test.js`
glob for the same reason — `node --test` has no built-in mechanism to run only
some of a file's `test(...)` cases per lane, so a runtime-dependent assertion has
to live in a file the unit lane never discovers, not merely be labelled opt-in
inside one it does. The extracted `ping` test's runtime-absent case becomes an
explicit failure (`assert.fail('Playwright is not installed for this lockhash
namespace')`), not a bare `return`, matching the opt-in lane's own
fail-rather-than-skip philosophy below rather than reproducing the pattern one
file over — and the ported `links` assertions carry **no** internal
Playwright-availability guard of their own (`test-run.sh:124,190`'s
`if [[ -n "$LINKS_OUT" ]] ... else SKIP` wrapper is not ported), since the opt-in
task's own preflight already guarantees a runtime before any test in this file
runs; a literal port of the shell's own guard would reintroduce the exact bare-skip
shape this section exists to remove, one file over. `daemon.test.js` itself
now contains no namespace-gated test at all, so the unit lane's floor and
zero-skip assertion are unconditionally satisfiable — not merely satisfiable in
the common case.

- **`test:unit:design-automation`** covers the runtime-free `lib/*.test.js` suites, with a
  discovered-file floor of **8** (today's ten, less `identity.test.js` and `lock.test.js`
  deleted in §5 — `daemon-runtime.test.js` is not under `lib/`, so it does not
  change this count), and **zero skipped tests**. The executed-count floor is derived,
  not left to be discovered at implementation time: today's eight surviving files
  execute **51** `test(...)` cases (`11+3+6+5+6+9+3+8`, one count per file); extracting
  `ping` out of `daemon.test.js` (§6, below) removes one, and the **five** assertions
  re-homed from `test-design.sh` add at least five more — one each for the
  `evaluate-payload-rejected` and `mcp__playwright__` absence checks, one for the
  `PROTOCOL.md` sync, one for `BLOCKING_OPS`, and one for the `ownerPid` guard (all
  four Removal-sweep rows landing here, not the three the shorter enumerations
  elsewhere in this section might suggest — the `evaluate-payload-rejected`/
  `mcp__playwright__` row is one table row but two separate shell assertions and
  therefore two tests) — landing the floor at **55 or more**, stated as a floor
  rather than an exact count because the `PROTOCOL.md` sync assertion may reasonably
  become several parameterised cases rather than one. It joins the aggregate unit
  task, so it runs on every build on both runners.
- **`test:integration:design-automation`** (new, opt-in, *not* in the default
  `mise run`) covers `test-run.js` and `daemon-runtime.test.js` — the latter now
  hosting both the extracted `ping` test and the ported `links` block (below) —
  explicitly discovered by name, not by a glob, so this lane's own file set is
  stated rather than implied and never overlaps the unit lane's `lib/*.test.js`
  glob. These need a real Playwright install, which no CI lane creates and which
  this plan does not change — so a zero-skip assertion in the unit lane could
  never pass, and asserting it would either fail every build or force the
  runtime-dependent tests to run without a browser. Instead the task **fails
  rather than skips** when no runtime is present, following the `docker info`
  preflight precedent in `tasks/test/e2e.py`, so an absent runtime is a visible
  refusal rather than a silent pass.
- `test-run.js:93-94`'s 14 `skip: !playwrightInstalled` tests are converted to a
  hard failure on the same condition, for the same reason.
- **The general guard is scoped to test bodies, not whole files.** Rather than a
  whole-file grep — which cannot distinguish a test body from a helper function,
  and would either miss the pattern inside a test or false-positive on a helper
  like `waitForFile`'s — the guard extracts each `test(...)`/`it(...)` callback's
  source range by brace-depth from its opening `{` to its matching closing `}`,
  and greps for a bare `return;`/`return null;` or `catch { return; }` **only**
  within that range, excluding any line that calls the test context's `skip()`.
  This is narrow enough to pass over `test-run.js:31` and `daemon.test.js`'s own
  helpers untouched, while still catching the shape this plan condemns wherever it
  appears inside an actual test.
- **Executed-count verification uses `node --test`'s own reporter, not a
  file-count proxy.** The unit-lane task runs `node --test --test-reporter=tap` and
  parses the summary line for pass/fail/skip counts, asserting fail = 0, skip = 0,
  and pass at or above the recorded executed-count floor — this is what makes
  "zero skipped tests" a check against the runner's own authoritative accounting
  rather than against file or line counts, which is the level at which the
  brace-scoped source guard above cannot see (a test can still call assertions and
  return normally without ever reaching a bare-return shape, and TAP's pass/fail
  vocabulary is what confirms that, not a text pattern).

The sibling plan moves these suites into its container lane, which does provision a
runtime, and can then assert zero skips across the whole set.

**This is also the home for four assertions re-homed from `test-design.sh`
(Removal sweep §1), all four landing as new `test(...)` cases inside the existing
`daemon.test.js` — no new file, so the unit-lane floor of 8 does not move.** Only
two of the four carry a self-matching risk at all, and the risk is specific to
their shape, not their destination: `test-design.sh:124-127`'s
`evaluate-payload-rejected`/`mcp__playwright__` absence check and `:537-538`'s
`ownerPid`/`--owner-pid`/`OWNER_POLL_MS` guard are both **repo-wide sweeps that a
literal forbidden string must not appear anywhere in the scanned tree** — so a
`lib/*.test.js` file whose own assertion text contains that string, scanning a
tree that includes itself, matches itself and inverts permanently. The other two
— `:491-507`'s `PROTOCOL.md` ↔ `daemon.js` command/env-var sync and `:511-514`'s
`links` membership in `BLOCKING_OPS` — read two named files and assert specific
content is *present*, not that a string is absent from a tree; they carry no
self-matching risk regardless of where they land. For the two sweep-shaped checks,
the forbidden strings are built from concatenated fragments in the test's own
source (e.g. `'evaluate-payload' + '-rejected'`, `'owner' + 'Pid'`) rather than
written as literals, so the pattern the test searches for never appears verbatim
in the file doing the searching — the same guarantee `test-design.sh`'s own
`grep -r "evaluate-payload-rejected" "$EXECUTOR_SRC_DIR/lib" "$EXECUTOR_SRC_DIR/run.js"`
relies on today only because the literal happens to live in a shell file outside
the directories it scans, which stops holding the moment the check moves inside
`lib/`.

**Fragmenting the needle is not sufficient on its own — every string literal in the
hosting file must avoid the phrase, not only the needle variable, or the same file
reintroduces the match one line away from the code that guards against it.** There
are **three** at-risk tests, not two: `evaluate-payload-rejected` and
`mcp__playwright__` absence are two separate assertions in the shell source
(`test-design.sh:124-127`), and both port to separate `test(...)` cases, alongside
`ownerPid`. A direct, natural-looking port of any of their titles
(`test('evaluate-payload-rejected not in executor source', ...)`,
`test('no mcp__playwright__ references in executor source', ...)`) would place that
exact phrase in `daemon.test.js`'s own source as a plain string — regardless of how
carefully the needle variable inside the test body is constructed — and the sweep
would find it there. So all three titles are worded to describe the property
without repeating the forbidden phrase verbatim: `'the payload-rejection deny-list
marker is absent from executor source'`, `'no MCP-prefixed browser-tool reference
survives in executor source'`, and `'no owner-PID watcher identifier survives under
the playwright tree'` — the last mirroring `test-design.sh:537`'s own title, "no
watcher identifier references," which already avoids naming `ownerPid` directly.
The rule is not scoped to titles and comments alone: **any string literal in the
test's own source** — including an `assert`/`assert.equal`/`assert.match` failure
message, which an implementer as naturally writes as a title (`'mcp__playwright__
reference found in executor source'` is exactly the shape a third assertion
argument takes) — must either avoid the phrase or be built from the same
concatenated fragments as the needle itself. There is no comment in any of the
three tests that names a forbidden phrase either.

#### 7. Executor-path retirement

**Files**: `skills/config/browser-executor/`, `scripts/config-read-browser-executor.sh`,
`agents/browser-locator.md`, `agents/browser-analyser.md`,
`tasks/lint/call_site_migration.py`, `tests/unit/tasks/test_call_site_migration.py`
**Changes**: The `browser-executor` skill, `config-read-browser-executor.sh`, and
the `{browser-executor-script}` convention exist solely to resolve `run.sh`'s
absolute path. With `run.sh` gone they retire: both agents call
`accelerator design executor` directly at their ~40 call sites.

**One precondition must be checked before committing to that.** No agent in
`agents/` references `${CLAUDE_PLUGIN_ROOT}` or invokes `accelerator` today — the
established pattern is preload-a-skill-that-injects-resolved-values (`paths` →
`{work}`, `browser-executor` → `{browser-executor-script}`), and both agents carry an
explicit preload guard with a verbatim user-facing failure message that retires with
it. If `${CLAUDE_PLUGIN_ROOT}` is not expanded inside a subagent's Bash tool
environment, all ~40 rewritten call sites resolve to `/bin/accelerator` and every
browser agent breaks, with no guard left to produce a diagnosable message. So confirm
the expansion with a one-line manual check **before this phase is scheduled**, since
the two branches have different edit sets and discovering that mid-implementation
means rework:

| | Retire branch | Keep-a-preload-skill branch |
|---|---|---|
| `EXPECTED_INJECTION_SKILLS` | decremented | unchanged |
| `call_site_migration.py` allowlist | entry removed, fixture replaced | entry retargeted at the new script |
| `releases-and-compatibility.md:41-44` | rationale restated | unchanged |
| Manual criterion | agents work without `{browser-executor-script}` | agents work via the slimmed preload |

The keep branch retires only the `run.sh`-specific resolution and leaves a minimal
skill injecting the launcher path.

Two documentation surfaces go with it, and neither is covered by the Removal sweep
(scoped to "references to a deleted design script"):
`docs-site/src/content/docs/reference/agents.md`, and
`docs-site/src/content/docs/releases-and-compatibility.md:41-44`, which cites
`browser-executor` as one of the two mechanisms justifying the documented **minimum
Claude Code v2.1.144**. That rationale is restated against the mechanisms that
remain rather than left pointing at a retired skill.

`call_site_migration.py:8,33` allowlists that script as a retained shell call
site — the entry goes with it, and `test_call_site_migration.py:35` uses it as a
fixture, so that fixture is replaced.

Removing the `browser-executor` skill moves `EXPECTED_INJECTION_SKILLS`
(`tasks/lint/skill_permissions.py:48`) — it is an equality, not a floor.

#### 8. Deletion

**Files**: `run.sh`, `test-run.sh`
**Changes**: Deleted **after** the `links` block's assertions land in
`daemon-runtime.test.js` (§6) — `test-run.sh` is that contract's only copy today,
so deleting it first would have a window with none. Also deleted along
with `scripts/test-design.sh:542-546` (the `test-run.sh` delegation) and `:442-485`
(the `browser-executor` skill assertions this phase retires). `:518-531`'s
`browser-locator` links contract keeps its substance but loses its
`{browser-executor-script}` clause. Since `test-design.sh` runs in CI, omitting
these edits leaves `test:integration:config` red on merge.

### Success Criteria

#### Automated Verification

- [ ] Characterization tests written first for cold start, warm reuse, stale-PID
      recovery, PID-recycle rejection, lock contention and daemon-start timeout —
      derived from **`run.sh`'s source**, not from `test-run.sh`, which covers none
      of them: it contains structural and shellcheck checks, the `start_time_of`
      locale comparison, a ping/daemon-stop/links block and a
      survives-shell-exit smoke test, and everything from line 65 onward self-SKIPs
      without a real Playwright install, which CI does not have. Each is made
      deterministic by an injected port rather than by real time or real processes:
      PID-recycle via a `ProcessProbe` returning a fabricated start-time for a live
      pid, daemon-start timeout via an injected `Clock` and a `Spawner` that never
      signals ready, contention via two `Lock` holders — no sleeps

**`test-run.sh` is brought into scope, on its own terms, rather than dismissed
wholesale.** The paragraph above is about what `run.sh` port characterization is
*derived from* — it correctly says that's `run.sh`'s source, not `test-run.sh`. But
`test-run.sh`'s content still has to go somewhere, row by row, the same way
`test-design.sh`'s does:

- The `start_time_of` locale comparison (`:44-63`) is already ported — §4, above.
- The structural/shellcheck checks (`:12-24`) and the `evaluate-payload-rejected`
  deny-list absence (`:25-31`) assert properties of retained JS source files that
  `test-design.sh`'s equivalent blocks also assert; they are dropped as duplicates
  of the Removal-sweep re-homes, not silently lost.
- The `ping`/`daemon-stop` content-shape assertions (`:85-111` — `ok:true`, `node`,
  `chromium` fields; `server-stopped.json`'s reason) duplicate what
  `daemon.test.js:70-90`'s own `ping` test and `:136-158`'s `daemon-stop` test
  already assert directly against the daemon, bypassing the shell launcher — kept
  as the surviving assertion, since a Rust-side characterization of the same
  daemon-side content would be the third copy of one fact.
- The survives-launcher-shell-exit smoke test (`:195-224`) is explicitly weaker
  than its own comment states: "the actual regression guard... lives in
  `test-design.sh` as source-level grep assertions" (the `ownerPid` guard,
  re-homed into the node suites by the Removal sweep) — plus the new adapter-level
  harness's `setsid` assertion (§1) proves the mechanism the smoke test could only
  observe by timing. Dropped in favour of both.
- **The `links` block (`:113-193`, ~16 assertions) is the one genuine gap, and it
  is not dropped.** It forms a data-exposure/privacy contract over the **retained**
  `daemon.js` `links` implementation — no raw `href`, no fully-resolved URL, no
  echoed query string or fragment, correct `same_origin`/opaque-origin semantics,
  and the `about:blank` empty-array case — and `test-run.js`, the retained suite's
  own replacement runner, has no equivalent. Extracting real anchors from a real
  page needs a real Chromium, so — unlike the setsid/redirection/log-mode
  properties, which get a lightweight stub in §1 precisely because they don't need
  one — there is no honest way to make this suite runtime-free. It is ported into
  `daemon-runtime.test.js` (§6, the same file that hosts the extracted `ping`
  test — **not** `daemon.test.js`, which the unit lane discovers and requires to
  run with zero skips; `node --test` has no built-in way to run only some of a
  file's `test(...)` cases per lane, so a runtime-dependent assertion has to live
  in a file the unit lane's glob never reaches, not merely be *labelled*
  opt-in inside a file the unit lane still discovers), using the existing
  `links.html` fixture (`__fixtures__/links.html`) and the same `fork`+`send()`
  pattern `daemon.test.js`'s own `ping` test already uses, and runs in the
  **opt-in** `test:integration:design-automation` lane (§6) — which already fails
  rather than skips when no Playwright runtime is present, so this suite's privacy
  contract gets the same non-silent guarantee every other opt-in assertion does,
  rather than the wholesale self-SKIP `test-run.sh` gives it today.
- [ ] The reuse verdict is unit-tested as a pure function over (recorded pid,
      recorded start time, observed liveness, observed start time), including the
      absent-`start_time` case now treated as a mismatch
- [ ] The start-time probe agrees across `C`, `de_DE.UTF-8` and an unset locale,
      **and** across `TZ` ∈ {unset, `UTC`, a half-hour offset, a DST fall-back
      boundary}
- [ ] `lib/state.js` contains no start-time probe, so there is no cross-language
      agreement left to assert — the value it publishes is the one the launcher handed it
- [ ] `proc-stat-linux.txt` yields `1700145620`, and a fixture whose tick count does
      not divide evenly by `CLK_TCK` yields the truncated value
- [ ] `accelerator design executor daemon` is rejected by argument validation
- [ ] The daemon survives a SIGHUP to the launcher's process group, and the client
      path propagates a non-zero exit status and a signal death unchanged
- [ ] A `server-info.json` with `start_time_source: wallclock` is reused on liveness
      alone, unconditionally and on every invocation, never held to the ±1s tolerance —
      with the accepted consequence recorded that such a record gives no PID-recycle
      guard
- [ ] Concurrent executor invocations produce exactly one daemon and the loser
      reports `another-launcher-running`, asserted for the single lock backend
- [ ] The reuse verdict table is exhaustive: a test per row over `RecordedState`,
      including `None` and `PidUnparseable` recovering with no pid to signal,
      `Daemon(Probe(_))`+`Unavailable`, `Daemon(Wallclock(_))` and
      `Daemon(WriterUnavailable)` all reusing on liveness alone, and
      `Daemon(AbsentOrUnparseable)` recovering — and that no row signals
- [ ] A `server-info.json` with no `start_time_source` key is read as `Wallclock`,
      not `Probe`
- [ ] The launcher observes the start time and sends it to the daemon over the
      inherited pipe before the daemon publishes anything; `state.js` computes none,
      publishes the whole record — read from `server-info.json` alone — in one
      atomic write, and no `getconf` shell-out remains on that path
- [ ] A launcher killed between daemon readiness and its next invocation leaves a record
      that still carries a start time — there is no window in which a live daemon has a
      partial identity record
- [ ] A fixture-backed round-trip test pins the identity pipe's wire format (four
      newline-delimited fields, fixed field order and encoding); the daemon's read
      reaches EOF deterministically once both the child's `O_CLOEXEC`-closed inherited
      write-end copy and the launcher's own closed copy are gone
- [ ] A launcher that crashes after spawning but before writing to the pipe produces a
      daemon that reads immediate EOF with no data, logs the failure and exits before
      creating any Playwright/Chromium process or opening its listening socket — no
      unsupervised, un-recorded daemon survives a launcher crash mid-handoff
- [ ] A daemon request without the token is refused **from the daemon's first accepted
      connection**, and `client.js` supplies it
- [ ] A request carrying an `Origin` header is refused regardless of token validity,
      and a request presenting a valid token only as a query parameter (never a
      header) is refused
- [ ] `PROTOCOL.md` documents the token's header name and generation, and no longer
      claims the daemon is unreachable by "external callers" without also naming what
      the token defends against
- [ ] Invoking outside a repository emits the `no-repo` envelope on stderr and exits 2
- [ ] A stale `server-stopped.json` is removed before a spawn, so a failed start is
      not reported as a completed shutdown
- [ ] The bootstrap log is truncated and `0600` before the daemon writes, under a
      `umask 077`, and a daemon-written screenshot is `0600` under the same
      child-side `umask(0o077)`
- [ ] The bootstrap log's contents, after a normal run and after a run with an auth
      header configured, never contain the token value or the resolved credential
      value — the non-leakage property §3 states, checked rather than assumed
- [ ] A warm executor invocation is measured against today's `run.sh` path with
      work-item:0186's interleaved-sample method, before `run.sh` is deleted (§8);
      the hard gate is the ratio (port ≤ shell), and the delta against the
      20–45ms-faster estimate is recorded as an observation, not a pass/fail
      threshold — the per-invocation cost of the path a crawl takes 100–200 times, which no
      criterion measured before
- [ ] Launcher envelopes are byte-identical 3-key JSON on stderr, and daemon
      envelopes reach stdout at exit 0
- [ ] `mise run test:unit:design-automation` passes over the runtime-free suites with
      **zero skipped tests** and an executed count at or above its floor, verified
      against `node --test --test-reporter=tap`'s own pass/fail/skip counts rather
      than a discovered-file floor, which a wholesale skip would not move and
      therefore would not catch
- [ ] No `lib/*.test.js` suite contains a bare early `return`, `return null;` or
      `catch { return; }` **inside a `test(...)`/`it(...)` callback body**, asserted
      by a brace-scoped grep that excludes helper functions and lines calling the
      test context's `skip()` — narrow enough to pass over `test-run.js:31`'s and
      `daemon.test.js`'s own helper returns while still catching the shape this plan
      condemns inside an actual test
- [ ] `daemon.test.js`'s three re-homed guards (`evaluate-payload-rejected` absence,
      `mcp__playwright__` absence, and `ownerPid`/`--owner-pid`/`OWNER_POLL_MS`
      absence — three separate `test(...)` cases) build their forbidden strings
      from concatenated fragments, not literals — **and** no title, comment, or
      assertion failure-message string in any of the three names the forbidden
      phrase verbatim — so the check does not self-match the file it lives in
      through any of those channels
- [ ] `test:integration:design-automation` **fails** rather than skips when no Playwright
      runtime is present
- [ ] The Rust lockhash digest equals `sha256_of` `ensure-playwright.sh`'s function for
      the shipped `package-lock.json`, pinned as a golden
- [ ] The Linux/Darwin start-time test suite currently at `cli/visualiser/server`
      moves to `process-probe`, not merely the implementation it tests — both
      `cargo nextest run -p process-probe` and `cargo nextest run -p
      accelerator-visualiser` (with its import repointed at the new crate) pass
- [ ] `PathResolution` refuses with a named error when `ACCELERATOR_PLUGIN_ROOT` is
      unset, and both path-bearing envelopes (`playwright-not-installed`,
      `daemon-start-timeout`) are byte-identical across two invocations from different
      working directories
- [ ] `RunClient`'s type makes it impossible to sequence domain logic after the call in
      the compiled binary (a `-> kernel::Error` or equivalent diverging signature); the
      lock is observably free immediately after a successful command
- [ ] The pid the launcher records for the identity handoff is the pid the daemon
      reports as its own — asserted with a fixture-backed check that `setsid` is used,
      not a double fork
- [ ] The forwarded-command validator rejects a payload that itself carries a `command`
      or `protocol` key, and `client.js` refuses to connect to a `server-info.json`
      whose `url` is not loopback
- [ ] `::1`, `::a.b.c.d` and `::ffff:a.b.c.d` all classify via `Ipv6Addr::to_ipv4()` to
      their embedded address; a percent-escaped or control-character-bearing host is
      rejected rather than treated as an opaque hostname
- [ ] `mise run cli:check` and `mise run scripts:check` exit 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A live inventory crawl completes with page state preserved across
      consecutive executor commands (the identity contract holding)
- [ ] Both browser agents work end to end without `{browser-executor-script}`
- [ ] Running two executor commands concurrently produces one daemon, and the
      loser reports `another-launcher-running`

---

---

## Removal sweep

### Overview

The residue this plan owns: the `test-design.sh` blocks whose scripts are gone, the
registration checklist's four drifted points, documentation for the migrated
subcommands, and the follow-up items this work surfaced. The file itself and the
`skills/design/` no-`.sh` assertion belong to the sibling plan, which deletes the last
two scripts.

### Changes Required

#### 1. `test-design.sh` partial teardown and the floors

**Files**: `scripts/test-design.sh`,
`scripts/test-skill-frontmatter-conformance.sh`,
`skills/design/inventory-design/scripts/playwright/lib/*.test.js`,
`tasks/test/integration.py`
**Changes**: `test-design.sh` is not only a runner. Its blocks split three ways, and this
plan owns everything except the two rows marked *sibling*:

| Block | Lines | Disposition |
|---|---|---|
| `validate-source` behavioural + delegated suite | 169-281 | dies with the script (Phase 2) |
| `resolve-auth` behavioural | 282-315 | dies with the script (Phase 2) |
| `scrub-secrets` behavioural | 316-338 | dies with the script (Phase 2) |
| `audit-cue-phrases` behavioural | 368-425, 428-430 | dies with the script (Phase 2) |
| delegated `test-notify-downgrade.sh` | 547-551 | dies with the script (Phase 2) |
| `audit-cue-phrases.sh` call site, existence, exec bit | 359-364 | **rewrite in Phase 2** → asserts the new subcommand |
| `browser-executor` preloaded skill | 442-485 | dies with the skill (Phase 6) |
| delegated `test-run.sh` | 542-546 | dies with the launcher (Phase 6) |
| `init` path keys + `DIR_COUNT` marker | 12-29 | **re-home** → frontmatter conformance |
| `configure` paths table | 31-38 | **re-home** → frontmatter conformance |
| canonical `research_design_*` call-site guard | 40-44 | **re-home** → frontmatter conformance |
| docs list `design-inventories/`, `design-gaps/` | 46-57 | **re-home** → frontmatter conformance |
| browser agents exist; `tools:` is exactly `Bash` | 59-108 | **re-home** → frontmatter conformance |
| `browser-analyser` body forbids `fetch`/`eval`/… | 109-119 | **re-home** → frontmatter conformance |
| `.mcp.json` does not exist | 131-136 | **re-home** → frontmatter conformance |
| both skills' structure | 141-153, 156-168, 350-358, 365-367 | **re-home** → frontmatter conformance (`:140`'s `SKILL=` assignment stays behind — see note) |
| `analyse-design-gaps` skill-instructions hook | 426-427 | **re-home** → frontmatter conformance |
| both skills' `evals.json`/`benchmark.json` validity | 339-349, 431-441 | **re-home** → frontmatter conformance |
| `evaluate-payload-rejected`/`mcp__playwright__` absent from executor source | 121-129 | **re-home** → `daemon.test.js`, needle built from concatenated fragments (Phase 6 §6) |
| `PROTOCOL.md` ↔ `daemon.js` command and env-var sync | 491-510 | **re-home** → `daemon.test.js` (Phase 6 §6) |
| `links` is in `BLOCKING_OPS` | 511-517 | **re-home** → `daemon.test.js` (Phase 6 §6) |
| `ownerPid`/`--owner-pid`/`OWNER_POLL_MS` never return | 532-541 | **re-home** → `daemon.test.js`, needle built from concatenated fragments (Phase 6 §6) |
| `browser-locator` links contract | 518-531 | **re-home** minus the `{browser-executor-script}` clause Phase 6 retires |
| `inventory-design` `allowed-tools` `scripts/*` glob | 153-155 | **sibling** — the rule it asserts is dropped there |
| delegated `test-ensure-playwright.sh` | 486-490 | **sibling** — dies with the script |

The target is chosen by what each assertion is *about*. **Two adjacency traps in the
138-168 range are called out explicitly, not left to a mechanical line-range cut.**
`:140`'s `SKILL="$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md"` is defined once
and read by both the re-homed block above it and the sibling's surviving assertion at
`:154-155` — so the re-home takes `:141-153`, not `:138-153`, leaving the assignment in
`test-design.sh` for the assertion that still needs it. And `:153`'s
`# shellcheck disable=SC2016` comment belongs to the `:154-155` assertion immediately
below it, not to the block above — so it moves with the sibling's row, and the sibling
plan's edit set includes it. Skill, agent and docs structure
goes to `scripts/test-skill-frontmatter-conformance.sh`, already a
`_REQUIRED_CONFIG_SUITES` by-name gate (`tasks/test/integration.py:63`) and so run
unconditionally. Anything asserting a property of the retained JavaScript goes to the
`test:unit:design-automation` suites Phase 6 §6 introduces.

Three are worth naming because losing them would be silent. The
`evaluate-payload-rejected` and `mcp__playwright__` guards constrain `lib/` and `run.js`
— both **retained** — so they are unrelated to this migration and need only a new home
(Phase 6 §6 names it, and the self-matching risk this move introduces). The `ownerPid`
guard encodes a resolved incident (see
`meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md`) and carries the
same self-matching risk for the same reason. And the canonical
`research_design_*` guard greps the very SKILL.md and agent files this plan rewrites,
making it more valuable during this work than before it.

**`_EXPECTED_CONFIG_SUITES` does not move again in the Removal sweep.** Phase 3 already
moved it once, deleting `test-metadata-helpers.sh` and landing this plan's own exit
state at **15 discovered against a floor of 15** — one suite and one phase earlier than
an earlier draft assumed here. `test-design.sh` survives the Removal sweep (two of its
blocks are the sibling's), so nothing in this plan changes that count again. The
handoff to the sibling plan is therefore **14-against-15, not 15-against-15**: the
sibling deletes `test-design.sh` itself, landing discovered at 14, and — unlike the
previous draft's assumption that the floor would already be waiting at the right
number — the sibling plan must move the floor from 15 to 14 in the same change that
deletes the file, or `test:integration:config` reddens on its own merge.

The new `test:unit:design-automation` task gains a floor of its own (Phase 6 §6).

#### 2. Documentation

**Files**: the `docs-site/src/content/docs/` pages naming a migrated script, `README.md`,
`CHANGELOG.md`, `docs-site/src/content/docs/reference/agents.md`,
`docs-site/src/content/docs/releases-and-compatibility.md`
**Changes**: Every reference to a deleted design script is repointed at its subcommand.
`plugin.json`'s `Node >= 20` declaration **stays** — this plan does not remove that
prerequisite.

Three behaviour changes need an explicit note rather than a silent repoint.
`validate-source`'s reachability rewrite (Phase 2 §1) **narrows** the accepted set, and
the note must split it in two: addresses that are *internal reach* and recoverable with
`--allow-internal` (`::ffff:10.0.0.1`, `fc00::/7`, `100.64.0.0/10`, `fe80::/10`),
versus alternate numeric encodings that are rejected
**unconditionally with no flag** (decimal, hex and octal IPv4 forms, including a
non-first octal octet). Phase 2's manual verification pins the second class, so the
documented remediation must not promise a flag that does not apply. Second,
`scrub-secrets` and `audit-cue-phrases` now split usage error onto exit 2. Third,
`releases-and-compatibility.md:41-44` cites `browser-executor` as one of two mechanisms
justifying the documented minimum Claude Code **v2.1.144**; Phase 6 §7 restates that
rationale against the mechanisms that remain.

#### 3. Registration checklist drift

**File**: `tasks/README.md`
**Changes**: The checklist has drifted in four places, and
`tests/unit/tasks/test_registration_docs.py` enforces its shape, so fixing it is part of
the work:

- The `assert len(uploads) == 22` count no longer exists — it is derived.
- `_setup_release` now loops the registry rather than being single-token.
- "the visualiser is the worked example" is stale; all six tokens have entries.
- Point 7 no longer describes only `!`-preprocessor commands — fenced blocks in numbered
  steps count too (`dispatch_coherence.py:9-13,95`).

Add the undocumented per-token edit the research found: `_SUBBINARY_DESCRIPTIONS`
(`tests/integration/tasks/test_github.py:35-46`) KeyErrors without an entry.

#### 4. Follow-up work items

**Changes**: Three defects surfaced during this work are deliberately **not** fixed here,
because each is a behaviour change rather than a migration:

- **The header-auth path is dead.** `makeAuthHeaderHandler` is imported at
  `daemon.js:11` and never called, and `ACCELERATOR_BROWSER_LOCATION_ORIGIN` is set
  nowhere — while `SKILL.md:89-95,196` documents its origin allowlist as
  security-critical. Users are told to place real bearer tokens in a browser-driving
  daemon's environment for a feature that never applies them, and an authenticated crawl
  silently produces an unauthenticated inventory. The follow-up decides between wiring it
  up (with an origin-allowlist test) and retiring `resolve-auth`, the
  `ACCELERATOR_BROWSER_*` variables and their scrub rules together. The **documentation**
  is corrected here rather than deferred (Phase 6 §5).
- **Navigation URLs are unclassified.** `validate-source` hardens the initial location,
  but `daemon.js:165-167` calls `page.goto(req.url)` on whatever each request supplies,
  so an attacker-influenced page or a redirect can steer a crawl at an internal endpoint,
  and `links`' `same_origin` flag drives route following. The follow-up plumbs the
  `AccessPolicy` verdict — including `--allow-internal` — through the executor into
  `navigate` and the `links` decision, reusing the `host_reach` code this plan writes.
  Until then the module docs and the design page state that the check is advisory rather
  than a boundary.
- **Credential scanning is literal-substring only.** `leaked_credentials.rs` matches the
  named values and, per Phase 2 §1, the value half of `ACCELERATOR_BROWSER_AUTH_HEADER`
  — but the artefacts scanned are model-authored prose, where a credential is as likely
  to appear base64-encoded, percent-encoded, whitespace-normalised or truncated. The
  follow-up derives candidate encodings per named value plus a minimum-length prefix
  match, still reporting only the variable name.

### Success Criteria

#### Automated Verification

- [ ] Failing test first: every re-homed assertion from §1's table is present in its new
      suite **and shown to fail when its property is broken**, verified before its old
      home is cut
- [ ] `mise run test:integration:config` passes with `_EXPECTED_CONFIG_SUITES` still at
      15 (moved once already, by Phase 3) and `test-design.sh` still discovered (it
      survives this plan)
- [ ] `mise run test:unit:design-automation` passes at its floor with zero skipped tests
- [ ] `mise run test:unit:tasks` passes, including `test_registration_docs.py`
- [ ] `mise run lint:scripts:exec-bits:check` exits 0
- [ ] `mise run docs:check` exits 0
- [ ] Only `ensure-playwright.sh` and the retained JavaScript remain under
      `skills/design/**/scripts/`
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] The docs site builds and every design page's links resolve
- [ ] Both design skills run end to end in a live session, on a machine with a
      bootstrapped Playwright namespace
- [ ] A Playwright-driven inventory still works exactly as before this plan — same
      prerequisite, same bootstrap, same downgrade reasons

---

## Testing Strategy

### Unit Tests

- Domain logic in `cli/design/` with no I/O: host canonicalisation across the numeric
  encodings, the reachability classification matrix including the newly-rejected
  encodings, the auth precedence table, the printable-ASCII and bidi message invariants
  as a table test, and H2 sectioning against the cue-phrase patterns behind the
  `CuePhraseMatcher` port.
- The executor's reuse verdict in `cli/design/` as a total function over injected
  `Clock`, `ProcessProbe`, `StateStore`, `Lock`, `Spawner`, `ProcessControl` and
  `RunClient` ports — so cold start, warm reuse, stale-PID recovery, PID-recycle
  rejection, lock contention and daemon-start timeout need neither real processes nor
  real elapsed time.

### Integration Tests

- Repointed suites derived from the deleted bash suites, invoking the binary rather than
  the scripts, with the exit-code split asserted explicitly and the migration checklist
  landing as a committed artefact.
- The retained `lib/*.test.js` suites under `test:unit:design-automation`, with zero
  skips and an executed-count floor.
- `test-run.js` and `daemon-runtime.test.js` — the latter hosting the namespace-gated
  `ping` test extracted out of `daemon.test.js`, plus the `links` privacy-contract
  assertions ported from `test-run.sh` — all under an opt-in
  `test:integration:design-automation` task that requires a bootstrapped runtime and
  **fails rather than skips** without one (Phase 6 §6).

### Manual Testing Steps

1. Run both design skills end to end in a live session after Phase 2.
2. Run an inventory crawl of a multi-route app after Phase 6 and confirm page state
   survives consecutive executor commands.
3. Confirm both browser agents work without `{browser-executor-script}`.
4. Run two executor commands concurrently and confirm one daemon, with the loser
   reporting `another-launcher-running`.

## Performance Considerations

**The warm executor path.** A crawl makes 100–200 executor invocations, and this plan
changes what happens on each one: a shell launcher becomes a dispatched sub-binary, which
adds a sub-binary resolve plus the per-exec sha256 and minisign re-verify at
`resolve/mod.rs:191-199` — on top of the shim's own per-invocation verify, which
`run.sh`'s nested `bin/accelerator config path tmp` call pays too, so the two paths
share that baseline rather than one paying it and the other not. It also *removes*
`jq`, `flock`, `sha256sum` (the lockhash recompute — negligible at the shipped
lockfile's actual size, well under a millisecond), `nohup`, and
`sed`/`awk`/`tr`/`date`.

**The expected direction is faster, but the specific figure is a hypothesis to
confirm, not a derived target.** Today's `run.sh` pays a *nested full
`bin/accelerator` bootstrap* on every invocation (`run.sh:77`), measured at 29.92ms
median in work-item:0186 — a figure that already includes the launcher's own
shim-verify and dispatch overhead, which the new path pays too, on top of the
added sub-binary resolve and re-verify (roughly 5–10ms). So the two paths' shared
overhead does not net out cleanly, and the real saving is the aggregate of the
several shell-outs removed above, none of which is individually quantified here.
**20–45ms faster per call is therefore stated as an expectation to record against,
not a threshold the criterion must meet**: the hard gate is the ratio (port ≤
shell — no regression), and the measured delta against the 20–45ms estimate is
reported alongside it as an observation, so a smaller-but-real improvement passes
the gate and is simply noted as smaller than hoped, rather than failing a
criterion calibrated on an unreconciled estimate. Phase 6 measures the warm path
against today's `run.sh` path with work-item:0186's interleaved-sample method,
before `run.sh` is deleted (§8), and records the result as a committed artefact —
this is necessarily a one-time comparison, since the shell baseline disappears in
the same phase, not a repeatable CI gate.

**Nothing else in this plan is on a hot path.** The five ported subcommands are invoked
once or twice per skill run. No tree resolution, no manifest fetch, no launcher size
change — the `tar` and `flate2` additions that made launcher size a per-invocation
latency term belong to the sibling plan.

## Migration Notes

Skills and agents are rewired inside the phases that delete what they call, so no
intermediate state has a call site pointing at a missing script — including the three
places earlier drafts missed, all repointed in Phase 6 §5: `inventory-design/SKILL.md`'s
Step 5 `run.sh ping` call, its Step 12 `run.sh daemon-stop` cleanup call, and the
`executor-ping-failed` remediation text, which stops naming `run.sh` in the same phase
that deletes it. A grep-based CI guard (Removal sweep §1) now covers the class, so a
future dangling call site under `skills/design/**/scripts/` fails at merge rather than
at run time.

`SKILL.md` Step 4's `ensure-playwright.sh` bootstrap and its
`ACCELERATOR_DOWNGRADE_REASON=` stderr protocol are **untouched** here, along with the
residual `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/**/scripts/*)` `allowed-tools` rules
that keep them reachable. Both are the sibling plan's to remove.

Existing installs keep their `${ACCELERATOR_PLAYWRIGHT_CACHE}` namespace and keep using
it. Nothing about the runtime's location or provenance changes in this plan.

## References

- Work item: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- Sibling plan: `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`
- Superseded plan and its three-pass review:
  `meta/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli.md`,
  `meta/reviews/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1.md`
- Research: `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
- Prior research:
  `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- ADR-0053 (CLI as argument parsing and presentation only), ADR-0058 (shell-free
  CLI-to-Node delegation). ADR-0057, ADR-0059 and ADR-0060 govern the sibling plan.
- Sub-binary template:
  `meta/plans/2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`
- Registration checklist: `tasks/README.md:322-474`
