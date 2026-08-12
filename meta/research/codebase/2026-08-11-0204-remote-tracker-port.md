---
type: codebase-research
id: "2026-08-11-0204-remote-tracker-port"
title: "Research: Implementation ground for the RemoteTracker port crate"
date: "2026-08-11T11:37:26+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0204"
parent: "work-item:0204"
topic: "What an implementation plan for 0204 (the tracker port crate) must know"
tags: [research, codebase, rust, tracker, sync, port, cargo-pup, jira, linear]
revision: "669484767129367b025152ee975d53a6e096246f"
repository: "accelerator"
last_updated: "2026-08-11T11:37:26+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Implementation ground for the RemoteTracker port crate

**Date**: 2026-08-11 11:37 UTC
**Author**: Toby Clemson
**Git Commit**: `669484767129367b025152ee975d53a6e096246f`
**Branch**: jj workspace `build-system` (commit not pushed; references are local paths, not permalinks)
**Repository**: accelerator

## Research Question

What does an implementation plan for `meta/work/0204-remote-tracker-port.md` need
to know about the codebase it lands in — the crate conventions it must match, the
bash semantics it freezes a contract against, and the enforcement surfaces it must
register with?

## Summary

The mechanical half of 0204 is smaller and better-supported than the item assumes.
A new dependency-free domain crate needs exactly three registrations — one line in
`cli/Cargo.toml` `[workspace].members`, a regenerated `Cargo.lock`, and a
`cli/pup.ron` rule — and everything else (rustfmt, clippy, cargo-deny, nextest)
picks it up from workspace membership with no per-crate wiring. Twelve of the
thirteen points on the sub-binary registration checklist do not apply.

The contract half has four problems that should be settled **before** the signature
is frozen, because the item's own protocol makes post-acceptance change a new work
item rather than an edit.

**1 → `cargo public-api` does not exist in this repository.** Not a `mise.toml`
pin, not a task, not a CI job, not a lockfile entry. Acceptance criterion 1 depends
on tooling that would be the repo's *third* Rust toolchain. §Blockers gives three
substitutes, one of which matches an existing house idiom and costs nothing.

**2 → `work-item-bridge-codes.sh` defines four dispatch codes, not two.** The
parity fixture and its criterion are specified against "the two dispatch codes";
the file has 70/71/72/73. The two-class `TrackerError` is still *right* — 72 and 73
resolve above the port — but a fixture enumerating two codes does not fail when the
script gains a fifth, which is the property the criterion asks for.

**3 → `fetch_all`'s return type cannot express "the fetch was incomplete".** The
bash bulk path returns a three-way partition (`found`/`absent`/`indeterminate`) and
guards an invariant: *absent is only ever drawn from a provably complete fetch*.
`Vec<(ExternalId, RemoteIssue)>` collapses indeterminate into absent, and Linear's
bulk fetch truncates at 250 issues against ~180 synced items here. This is the one
finding that can make the port produce a wrong answer.

**4 → `RemoteIssue.body` is not obtainable from either provider's current bulk
query.** Linear's bulk GraphQL selection set has no `description` field at all, and
the Rust projection that would produce the body already exists — in `work-adapters`,
on the wrong side of the boundary 0204 exists to draw.

None of these blocks starting. All four are cheap now and expensive after acceptance.

## Detailed Findings

### The crate skeleton: what a dependency-free domain crate looks like here

The workspace (`cli/Cargo.toml`) has 24 members and a clean hexagonal split: bare-named
domain crate (`vcs`, `corpus`, `config`, `work`), a `<name>-adapters` sibling, and a
`<name>-cli` composition root owning the `accelerator-<name>` binary.

**Package name is the bare directory name.** `cli/vcs/Cargo.toml:2` is `name = "vcs"`;
`accelerator-vcs` is the *binary* crate at `cli/vcs-cli/`. The reason is recorded at
`tasks/README.md:454-459` — cargo-pup rules match on whole crate names, so a domain
crate renamed `accelerator-tracker` would silently stop matching its rule. So:
`cli/tracker/Cargo.toml` with `name = "tracker"`.

Manifest shape, copied verbatim from `cli/vcs/Cargo.toml`:

```toml
[package]
name = "tracker"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true
```

with **no `[dependencies]` table at all**. Note this would be the workspace's first
zero-dependency member — every current member has at least `kernel`. Inheriting
`version` is load-bearing: `tasks/build.py:102-129` only catches a *mismatch*, so a
hardcoded literal passes today and breaks at the next workspace bump
(`tasks/README.md:348-351`).

`src/lib.rs` follows one of two shapes. For five items, the `vcs` shape fits — types
declared directly in `lib.rs`, no `pub use` re-export block, a `//!` doc comment
naming what the crate is *and what it deliberately excludes*
(`cli/vcs/src/lib.rs:1-12`). No crate-level attributes: a grep for `^#![` across every
domain crate's `lib.rs` returns zero hits. Lint policy lives entirely in
`[workspace.lints]`.

### Error type: hand-written, because `thiserror` is unreachable

`thiserror` is a workspace dependency (`cli/Cargo.toml:22`) but only `kernel` and the
visualiser server use it. **Every** domain and adapter error type hand-writes `Display`
and `std::error::Error`. A dependency-free crate has no choice anyway.

The exact precedent for a crate with no `kernel` dependency is
`cli/document/src/error.rs:1-42`: hand-written `Display`, empty
`impl std::error::Error for X {}`, and **no** `From<X> for kernel::Error` (that
mapping is what every *kernel-carrying* crate adds, e.g.
`cli/corpus/src/store.rs:52-56`). `tracker` omits it.

The house recipe, from `cli/corpus/src/store.rs:1-56`:

- `use std::fmt::Display;` and `use std::fmt::Formatter;` on separate lines
- parameter named `formatter`, returning `std::fmt::Result`
- `impl std::error::Error for X {}` with an empty body, no `source()`
- **one test per `Display` arm** pinning the exact message string
  (`cli/corpus/src/store.rs:81-148`)

That last one matters for AC 9 ("no `#[cfg(test)]` module in `tracker/src/`"): the
house style puts `Display` tests in an inline `#[cfg(test)] mod tests`, and 0204
forbids exactly that. The tests must go in `tracker/tests/` instead, which works —
`Display` is public surface. Worth stating in the plan so the deviation is deliberate.

On `#[non_exhaustive]`: 0204 deliberately omits it, and there is precedent both ways —
`corpus::StoreError` and `config::ConfigError` carry it, `document::DocumentError` does
not. 0204's choice matches `document`.

### The import discipline that will bite

Every pup-governed crate must write **one single item per `use` line, fully
`crate::`-qualified**, in production code. This is not a rustfmt setting
(`cli/rustfmt.toml` sets only `max_width` and `edition`) — it exists because cargo-pup
resolves a grouped `use a::{b, c}` to an empty module name, which no `allowed_only`
regex matches, so the rule rejects it. Recorded at `cli/pup.ron:131-134`, pinned by
`tests/integration/pup/test_import_rule.py:593-600`.

Consequences:

- `use super::Foo;` and `use self::foo::Bar;` also fail — they do not match
  `^crate(::|$)`, because `allowed_only` matches the *literal* use-path text.
- Glob imports fail for the same reason.
- `#[cfg(test)]` modules are exempt in practice: `tasks/pup.py:17` runs bare
  `cargo +nightly pup` with no `--tests`, so test targets are never analysed. That
  is why `cli/work/src/next_number.rs:146-147` can use grouped imports.

A grep for `^use .*(\{|\*)` across `cli/config`, `cli/corpus`, `cli/vcs`, `cli/work`
and `cli/migrate` returns zero matches — the discipline holds today.

### The pup rule, verbatim

`cli/pup.ron` carries 11 lints for 24 members. **There is no coverage guard** — a new
crate ships with zero architectural enforcement until its rule is written, and nothing
in `tasks/` or `tests/` notices the omission.

The shape to copy, `cli/pup.ron:57-72`:

```
        // The whole corpus crate is domain (no adapter modules live in it).
        Module((
            name: "corpus_domain_imports_only_permitted",
            matches: Module("^corpus($|::)"),
            rules: [
                RestrictImports(
                    allowed_only: Some([
                        "^(std|core|alloc)(::|$)",
                        "^kernel::Error(::|$)",
                        "^crate(::|$)",
                    ]),
                    denied: None,
                    severity: Error,
                ),
            ],
        )),
```

Two notes on the syntax: `matches:` is the **resolved module path** (crate root =
package name with `-` → `_`), while `allowed_only` entries are matched against the
**literal `use`-path text** — which is why `^crate(::|$)` works at all
(`cli/pup.ron:1-8`). The whole-crate anchor is `"^<crate>($|::)"`; the import anchor
is `"^<path>(::|$)"`. The alternation order differs between the two positions; copy
each verbatim.

**Recommend dropping the `^kernel::Error(::|$)` line.** 0204 keeps it as "headroom,
not a used edge", but with an empty `[dependencies]` table `use kernel::Error;` cannot
compile, so the allowance is inert and misdescribes the crate. One-line deviation from
the item's requirement; flag it in the plan rather than silently doing it.

**Add a probe pair while you are there.** `tests/integration/pup/test_import_rule.py`
drives the *shipped* `cli/pup.ron` against synthetic workspaces whose crates are
literally named `config`/`vcs-adapters`/`accelerator`, so a typo in or deletion of the
shipped rule fails the test. Only `config` has that coverage; `corpus`, `vcs`, `work`
and `migrate` have only `test_real_cli_pup_ron_loads`
(`tests/integration/pup/test_import_rule.py:208-213`), which proves parsing and nothing
more. Copying the `config` probe pair (a violation case plus a positive control) for
`tracker` is the TDD-consistent move — **and it converts AC 8's manual "a scratch edit
importing a `work` type is shown to fail `mise run pup:check`" into an automated
test.** That is strictly better than a demonstration nobody can re-run.

### What registration actually costs

The thirteen-point checklist at `tasks/README.md:322-474` is about *dispatched
sub-binaries*. For a plain library crate:

| Point | Applies |
|---|---|
| 4 — `[workspace].members` + regenerated `Cargo.lock` | **Yes, mandatory** |
| 3 — manifest inheritance (not the `[[bin]]`/`description` half) | **Yes** |
| 13 — `cli/deny.toml` | Only if new third-party deps; none here |
| 1, 2, 5–12 | **No** |

Plus the obligation outside the checklist (`tasks/README.md:458-459`): a domain crate
owes a `cli/pup.ron` rule.

`lint:cli:check` runs `--locked` (`tasks/lint/cli.py:15`), so an unregenerated lockfile
reddens `cli:check` as an apparent clippy failure — a confusing failure mode worth
knowing.

Everything else is automatic. `tasks/test/cli.py:6-14` runs
`cargo nextest run --workspace --exclude accelerator-visualiser --all-features`, so a
new crate's `tests/` directory is picked up by membership alone — **AC 10's "built and
run by the workspace's `cargo nextest run` invocation rather than being excluded from
it" is satisfied for free.** There are no Rust coverage floors; coverage is
report-only, explicitly no `--fail-under` (`tasks/test/cli.py:22-24`). The suite-count
floors (`_EXPECTED_CONFIG_SUITES = 15`, etc.) count shell `test-*.sh` files and are
untouched by a Rust crate.

Gate membership, since the item's AC 10 names three tasks: `cli:check`, `deny:check`
and `pup:check` are **all three** in the aggregate `check` (`mise.toml:577`) and in the
bare `default` (`mise.toml:581`). `pup:check` sits outside the `cli:` roll-up but not
outside `check` — the root `CLAUDE.md` phrasing "Rust enforcement beyond `cli:check`"
means the former. `pup:check` needs a rustup-managed nightly
(`PUP_NIGHTLY = "nightly-2026-01-22"`, `tasks/shared/rust.py:6-7`) and builds cargo-pup
from source on first run.

One live hazard: `tasks/lint/store_duplication.py` scans `cli/**/src/*.rs` for
`fs::rename(`, `NamedTempFile` and `.persist(` outside `cli/store/`. Irrelevant to a
crate with no I/O, but it rides `cli:check` and is easy to trip later.

### Clippy pressure on a frozen signature

`[workspace.lints.rust] warnings = "deny"` plus clippy `pedantic` and `nursery`
(`cli/Cargo.toml:133-147`). Two consequences for a verbatim-frozen API:

- **`missing_errors_doc`** — every fallible public method needs a `/// # Errors`
  section. All four `RemoteTracker` methods and nothing else. The house style is
  visible at `cli/vcs/src/classify.rs:46-76`.
- **`missing_const_for_fn`** (nursery, so warn → deny) will likely demand
  `const fn as_str` and possibly `const fn new`. Adding `const` neither widens nor
  narrows the public API, so it is compatible with the freeze — but the plan should
  say so explicitly, or a reviewer comparing against the verbatim block will read it
  as drift.

Also worth pre-empting: `fn as_str(&self) -> &str` returns **zero hits** across the
workspace today. Every existing `as_str` is `const fn as_str(self) -> &'static str` on
a fieldless enum. The `new(String)` + `as_str(&self)` pair 0204 freezes is a deliberate
departure from house style (which prefers validating constructors like `Key::parse` and
domain-named accessors like `segments()`). Fine — it is frozen — but name it as
deliberate so it survives review.

Conversely, the freeze has nothing to strip: the house style already forgoes `Deref`,
`AsRef`, `FromStr` and infallible `From` on value types. Grepping for those across
`cli/**` returns two hits, both test helpers.

### The bash contract the port freezes against

`work-item-bridge-codes.sh` is at `skills/work/scripts/`, not `scripts/`. The whole
file is 35 lines. Verbatim, `skills/work/scripts/work-item-bridge-codes.sh:8-18`:

```
#   70  E_DISPATCH_RETRYABLE      failure provably BEFORE any remote mutation
#                                 (arg/validation/auth/connect) — safe to retry.
#                                 For a READ bridge there is nothing to mutate,
#                                 so 70 simply means "read failed / degrade".
#   71  E_DISPATCH_TERMINAL       failure AT/AFTER a mutation (request sent,
#                                 response lost or invalid) — NOT safe to
#                                 auto-retry. Read bridges never emit this (a read
#                                 mutates nothing).
#   72  E_DISPATCH_NOT_AVAILABLE  tracker recognised but the operation is not
#                                 built yet (trello / github-issues).
#   73  E_DISPATCH_UNRECOGNISED   <sys> not in {linear,jira,trello,github-issues}
#                                 or empty — fail closed.
```

**The taxonomy is four codes in one namespace.** `work-item-push-decide.sh:89-108` is
the only consumer that branches on all four.

The two-class `TrackerError` remains the right model: 72 and 73 are dispatch-routing
outcomes, and in the Rust design both resolve *above* the port — at the composition
root that selects `Box<dyn RemoteTracker>` from `work.integration`. If the config names
`trello`, no client exists and there is no `RemoteTracker` to call. That is a clean
partition and it strengthens the design.

What needs fixing is the fixture and its criterion. As written, a fixture enumerating
two codes cannot fail when the script gains a fifth — which is precisely what AC 5 asks
of it. **The fixture should enumerate all four, assert that exactly two map onto
`TrackerError`'s classes, and record why the other two are out of scope.** Then the
1:1 mapping is a real assertion and a new code breaks the build.

The semantics also need a sharper doc comment than 0204's paraphrase. The stated rule
is asymmetric: retryable is *provable* absence of transmission; everything unproven is
terminal. Every mapping function in the bridges implements it with a catch-all
`*) return "$E_DISPATCH_TERMINAL"` — see `work-item-create-remote.sh:99-111` and
`work-item-update-remote.sh:51-72`. 0204's "no remote mutation provably occurred" is a
fair reading of the retryable class but drops the conservative default, which is the
part a client implementer will get wrong.

One asymmetry worth recording for 0171: **Linear code 34 (bad-request, including a
200-response GraphQL error body) is retryable on `create` but terminal on `update`.**
Deliberate, per the comment at `work-item-update-remote.sh:59-65`. The same wire
condition maps to two classes depending on the operation. `TrackerError` is
operation-agnostic, so the port is unaffected — but it is evidence the classification
genuinely belongs behind the port, and it is a trap for a client author assuming a
single status→class table.

Also worth noting for the hazard rationale: create's danger is *duplication* (the
remote create is non-idempotent, `work-item-push-decide.sh:32-33`), while update's is
*response uncertainty* (a whole-item update is idempotent, so the hazard is not
knowing, `work-item-update-remote.sh:26-28`).

### `RemoteTimestamp`: two formats and an empty string

The values are stored verbatim, never parsed. `work-item-project-remote.sh:16-18`
states it: *"the tracker's remote `updated` field, raw … Compared lexically against a
baseline written from the same tracker, so the raw string is correct."*

Real values differ by provider:

- **Linear** — `"2026-06-21T00:06:10.647Z"` (UTC `Z`, milliseconds)
- **Jira** — `"2026-07-09T08:00:00.000+0000"` (numeric offset, **no colon**)

A `chrono`/`time` round-trip would rewrite `+0000` → `+00:00`. The opaque-newtype
decision is well founded.

`work-item-sync-classify.sh:177` compares with raw `[ "$a" = "$b" ]` string equality —
it is a **cache key, not a clock**. An unequal stamp means "go hash the body", not
"remote is newer". The absence of `PartialOrd`/`Ord` is correct and load-bearing.

Two gotchas for the type:

- **`""` is a legal stored value.** `work-item-project-remote.sh:68,79` default to
  empty via `// ""`, and `work-item-sync-apply.sh:121` initialises `remote_updated=""`
  and leaves it empty when the post-push `show` fails. A `NonEmptyString`-style
  constructor would reject data already on disk. `new(value: String)` with no
  validation is right — say so in the doc comment so nobody "improves" it.
- **The fixture source is committed and live.**
  `.accelerator/state/integrations/linear/last-sync.json` is a real bash-written
  baseline with ~180 entries, tracked (`.gitignore:13` ignores only
  `config.local.md`). It is entirely Linear, so it yields no `+0000` value.
  **Commit both formats** — take the Linear string from that file and the Jira string
  from `skills/integrations/jira/scripts/test-fixtures/scenarios/apply-push-204-show.json:17`.
  A single-format fixture leaves the format the item is most worried about untested.

Baseline schema for context (`work-item-sync-baseline.sh:10-17`): one file per
integration at `.accelerator/state/integrations/<sys>/last-sync.json`, keyed by **local
`id`** (not `external_id`), holding `{remote_updated_at, remote_hash, local_hash}` per
item plus a top-level epoch `timestamp`. There is no Rust port of the baseline store or
the classifier — those are 0194's.

### The two `fetch_all` defects

**Defect A — the return type cannot say "incomplete".**

The bash bulk path returns a three-way partition
(`work-item-fetch-remote.sh:21-25`):

```
#       { "found":         { "<key>": { "updated": "<iso|null>" }, … },
#         "absent":        [ "<key>", … ],   # gone from a COMPLETE fetch
#         "indeterminate": [ "<key>", … ] }  # fetch incomplete — never absent
```

The invariant is explicit: **absent is only ever drawn from a provably complete
fetch.** Downstream, `work-item-sync-classify.sh:127-147` routes `absent` →
`remote-absent` and `indeterminate` → `indeterminate` — two different user-visible
states driving two different actions.

`fetch_all(&self) -> Result<Vec<(ExternalId, RemoteIssue)>, TrackerError>` gives the
caller only present-vs-not-present. Computing `absent = requested − returned` is
exactly the unsound inference the bash side is built to avoid. And it is not
hypothetical: Linear's key path fetches up to 250 issues team-wide and marks *every*
missing key indeterminate when truncated (`work-item-fetch-remote.sh:155-181`,
`_WIFR_LINEAR_LIMIT=250`) — against roughly 180 synced items in this repo today.

There are two honest resolutions, and the plan must pick one:

1. **Contract it away.** Write into `fetch_all`'s doc comment that a truncated or
   otherwise incomplete bulk retrieval is `Err(TrackerError::Retryable)`, never a
   partial `Ok`. Soundness preserved, four operations preserved, signature unchanged.
   Costs the degrade-per-key behaviour bash has today.
2. **Change the return type before freezing.** Carry the partition. More faithful,
   but it is a signature change — and after acceptance the item's own protocol makes
   it a new work item rather than an edit.

Option 1 is cheaper and keeps the freeze intact; option 2 is more faithful to the
behaviour being replaced. Either way this must be decided now.

Secondary: **`fetch_all` takes no arguments**, while the bash bulk mode is key-scoped
(`search --keys`, 50-key JQL chunks for Jira, `--all-projects`, a 20-page cap). An
unscoped `fetch_all` changes both cost and blast radius for the Jira client.

**Defect B — `RemoteIssue.body` is not obtainable from either bulk query.**

Verified directly. Linear's bulk selection set (`linear-search-flow.sh:157-165`) is:

```graphql
nodes { id identifier title updatedAt state { name } assignee { name } }
```

No `description`. Jira's key path requests `--fields updated,summary,description`, so
Jira could populate a body — Linear cannot without adding `description` to the query,
which interacts with Linear's complexity cap (exit code 36, classified retryable). That
is a client-side fix behind the port, so the port is not *wrong*, but it is a
constraint 0171 inherits and the plan should name it.

Compounding this: the key-scoped path indexes only `{updated}` for both providers, so
`RemoteIssue` as a bulk return type is a genuine widening of what the bash bulk path
retrieves.

### The stranded projection

`RemoteIssue.body` is specified as "the already-projected domain body … the output of
the projection recipe `work-item-project-remote.sh` defines", with each 0171 client
owning its provider's recipe.

That recipe **already exists in Rust** — `cli/work-adapters/src/project_remote.rs`,
whose own doc comment reads:

```rust
//! Lives in `work-adapters`, not the `work` domain crate: this is JSON
//! field extraction, adapter-shaped I/O-adjacent logic, not a domain
//! decision, and typing it against `serde_json::Value` would need a
//! dependency `work`'s own import-restriction rule does not permit.
//!
//! Not called by any of the five user-facing commands — its only consumer
//! is `sync-work-items`' bidirectional diff/apply flow, which today shells
//! out to the bash script directly rather than calling this projection.
```

Two problems follow. First, `work-adapters` depends on `work`, so a 0171 client
reusing it would transitively pull in the whole lifecycle domain — the exact coupling
`tracker` exists to prevent. Second, it projects the **`show`** payload shape
(`/fields/...`, `/data/issue/...`); the bulk payloads are shaped differently
(`.issues[].fields`, `.data.issues.nodes[]`), so it is not reusable for `fetch_all`
even setting the dependency question aside.

So the projection is currently stranded: written, unused in production, and on the
wrong side of the boundary 0204 draws. 0204 need not move it — but the plan should
record the tension and hand it to 0171 explicitly, or 0171 will either duplicate it
three ways or quietly take the dependency.

For reference, the recipe's per-provider asymmetry (`work-item-project-remote.sh:65-93`):
Jira emits `summary\n` + `jq -cS` key-sorted compact ADF; Linear emits `title\n` +
Markdown verbatim with **no** canonicalisation. Absent descriptions differ too — Jira
projects the literal `null`, Linear an empty line. Both then pass through
`work-item-normalise.sh --stdin`, which trims per-line whitespace and strips trailing
blank lines under `LANG=C LC_ALL=C`. The Rust normaliser exists at
`cli/work/src/normalise.rs`.

### Test patterns to model on

**Fakes.** The closest analogue is `collaboration`, which defines forge ports
(`RepositoryLookup`, `PullRequestExistence`, `PullRequestBodyUpdate`) with `github` as
the adapter — the same split a tracker port takes. House style
(`cli/collaboration/src/base_repo.rs:96-198`): newtype tuple structs carrying the
canned outcome (`struct FixedRepositoryLookup(Result<RepositoryDetails, ForgeApiError>)`),
a named constructor per interesting state (`no_parent()`, `pr_exists()`), and a local
helper collapsing the call under test. Naming is `Fixed*` / `Stub*` / `Fake*` /
`Spy*` / `InMemory*` / `Panic*`.

For a stateful fake, `InMemoryManifestStore` (`cli/migrate/tests/preflight.rs:52-82`)
is the model — `RefCell` fields, a `seeded(...)` constructor.

**Pinning an API shape.** The workspace has no signature-probe test today; the nearest
idiom is the closed-set guard at `cli/vcs/src/classify.rs:584-599`:

```rust
    #[test]
    fn the_classification_enum_has_exactly_seven_variants() {
        // A closed-set guard: fails to compile if a variant is added or
        // removed without this match arm list moving with it.
        let assert_exhaustive =
            |classification: Classification| match classification {
                Classification::Main
                | Classification::JjSecondary { .. }
                | ...
                | Classification::None => {}
            };
        assert_exhaustive(Classification::Main);
    }
```

That is exactly AC 4's wildcard-free match, and it is one of the few places a comment
survives the repo's low comment tolerance — because it explains why the body looks like
a no-op. Also see the golden-with-instructions pattern at
`cli/work-cli/tests/cli_surface.rs:45-55`, whose failure message says *"update {path}
deliberately if this change is intended"*.

**Integration test conventions.** File named for the topic (`store.rs`, `parity.rs`),
not `test_*`. A `//!` header stating what the suite proves. `type TestError = Box<dyn
std::error::Error>;` with tests returning `Result` and using `?`. `#![allow(clippy::
expect_used, clippy::unwrap_used)]` where unwrapping is unavoidable. Each integration
test binary is its own crate, which is why shared `mod common;` files open with
`#![allow(dead_code)]` (`cli/corpus-adapters/tests/common/mod.rs:1-6`).

**Fixtures.** Always `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/…")`
— never cwd-relative, 27 call sites. Reaching outside the crate uses the same env var
plus `..`, e.g. `cli/vcs-cli/tests/detect_goldens.rs:24-31` reads
`../../hooks/test-fixtures/`. **That is the mechanism the parity fixture needs**: to
fail when `work-item-bridge-codes.sh` changes, the test must read the script at
`../../skills/work/scripts/work-item-bridge-codes.sh` and compare against the committed
fixture. Line-splitting and string matching need only `std`, so this stays inside the
zero-dependency constraint.

**Dev-dependencies are an open question.** AC 8 says the manifest "declares no
dependencies at all". Strictly, `[dev-dependencies]` is a separate table. Nothing in
the planned tests needs one (`tempfile` is unnecessary — there is no I/O), so the
cleanest reading is that both tables stay empty. State it explicitly; otherwise a
reviewer and an implementer will read AC 8 differently.

**No parameterised-test framework.** `rstest`, `test-case` and `proptest` are absent.
Table-driven tests are plain `#[test]` fns with an in-fn array and a `for` loop, with
the case name interpolated into every assertion message
(`cli/document/tests/adversarial.rs:56-70`). Where assertions differ per case, the
workspace prefers one named `#[test]` each.

**`Box<dyn Trait>`.** AC 3 requires it, and there is precedent — but the dominant style
is `&dyn Trait` at function boundaries, with `Box<dyn T>` reserved for a struct owning a
port bundle (`ConfigStack`, `cli/launcher/src/config_command/core/mod.rs:87-158`, whose
`Box` fields are private and exposed as `&dyn T` accessors) or a heterogeneous
collection (`MigrationEntry`, `cli/migrate/src/registry.rs:38-78`). A test constructing
`Box<dyn RemoteTracker>` is fine and matches
`cli/launcher/tests/crypto_provider.rs:99-122`.

One design note from `collaboration`: forge-API failures are modelled as
`Ok(Outcome::Failed(_))`, not `Err`, because they are not failures of the function's own
logic (`cli/collaboration/src/base_repo.rs:41-47`). `RemoteTracker` takes the opposite
route — `Err(TrackerError)` — which is defensible for a port whose whole job is to
carry the failure class, but it is a divergence from the nearest sibling and worth a
sentence in the plan.

## Blockers

### `cargo public-api` is absent — AC 1 as written cannot be met

Searched `mise.toml`, `mise.lock`, `tasks/`, `.github/workflows/`, `cli/` and every
`*.toml`: no `cargo-public-api`, `cargo-semver-checks`, `rustdoc-json`, `insta` or any
equivalent. Every hit for "public API" is prose in `meta/` — including two earlier
review documents (`meta/reviews/work/0194-…-review-1.md:154`,
`meta/reviews/work/0170-…-review-1.md`) that proposed the same criterion. **The
requirement has been written three times and the tooling has never been installed.**

Installing it is not small: `cargo public-api` needs a nightly rustdoc-json toolchain,
so it would be the repo's third Rust toolchain. `tasks/README.md:257-291` records that
mise cannot pin two Rust toolchains, which is why cargo-pup uses a rustup-managed
`deps:install:*` lane. A third would follow that pattern, plus a `tasks/` module, a
`mise.toml` leaf, a `mise lock` regeneration, and a wiring decision for
`check`/`default`/CI.

Three substitutes, cheapest first:

1. **A self-reading pinned-surface test.** A test in `tracker/tests/` reads
   `../src/lib.rs` via `CARGO_MANIFEST_DIR`, extracts every `pub` declaration, and
   asserts the set equals a pinned list. Zero new tooling, catches additions,
   removals and renames, and matches the repo's established pin idiom (the registry
   pins in `tests/unit/tasks/shared/test_paths.py`, the 138-row assertion in
   `cli/vcs-cli/tests/guard_decision_table.rs:143-182`, the fixture-key set guard in
   `cli/vcs-test-support/tests/matrix.rs:20-39`). It is textual rather than semantic
   — it will not catch a changed derive — so pair it with the exhaustive-consumer
   test AC 2 already requires, which catches signature changes by failing to compile.
   **Recommended.**
2. **Descope AC 1** to the exhaustive-consumer test alone, accepting that additions
   are caught by review rather than mechanically. Weakest, but honest.
3. **Install `cargo public-api` as its own work item**, and have 0204 depend on it.
   Correct but disproportionate to a five-item crate, and it reintroduces the
   blocking-on-a-fragment problem the 0194 split existed to remove.

Whichever is chosen, AC 1's wording must change — it names a specific tool that does
not exist here.

## Code References

- `cli/Cargo.toml:4` — the `[workspace].members` line a new crate joins
- `cli/Cargo.toml:6-11` — `[workspace.package]` keys to inherit
- `cli/Cargo.toml:133-147` — `warnings = "deny"` + pedantic/nursery lint policy
- `cli/vcs/Cargo.toml:1-18` — the domain-crate manifest to copy
- `cli/vcs/src/lib.rs:1-12` — the `lib.rs` shape for a small domain crate
- `cli/vcs/src/classify.rs:46-76` — `/// # Errors` doc style on a port trait
- `cli/vcs/src/classify.rs:584-599` — the closed-set exhaustive-match guard
- `cli/document/src/error.rs:1-42` — hand-written error type with no `kernel` dep
- `cli/corpus/src/store.rs:1-56` — the full error recipe including per-arm tests
- `cli/collaboration/src/base_repo.rs:96-198` — the fake/double house style
- `cli/collaboration/src/base_repo.rs:41-47` — `Ok(Failed(_))` vs `Err` for remote failures
- `cli/work-adapters/src/project_remote.rs:1-80` — the existing Rust projection
- `cli/work/src/normalise.rs` — the existing Rust normaliser
- `cli/work-cli/tests/cli_surface.rs:45-55` — golden-with-instructions pattern
- `cli/vcs-cli/tests/detect_goldens.rs:24-31` — reading a fixture outside the crate
- `cli/pup.ron:1-8` — resolved-path vs literal-use-path semantics
- `cli/pup.ron:57-72` — the whole-crate `RestrictImports` rule to copy
- `cli/pup.ron:131-134` — why grouped imports are rejected
- `cli/deny.toml:67-98` — the `uluru` MPL-2.0 exception (context only)
- `tasks/README.md:322-474` — the thirteen-point registration checklist
- `tasks/README.md:454-459` — why a domain crate keeps its bare package name
- `tasks/test/cli.py:6-35` — the workspace-wide nextest invocation
- `tasks/lint/cli.py:15` — `--locked` clippy
- `tasks/pup.py:16-17` / `tasks/shared/rust.py:6-7` — the cargo-pup nightly lane
- `mise.toml:517-528`, `:575-581` — `cli:check`/`deny:check`/`pup:check` and gate membership
- `tests/integration/pup/test_import_rule.py:208-224`, `:593-600` — pup probe harness
- `skills/work/scripts/work-item-bridge-codes.sh:8-18` — the four dispatch codes
- `skills/work/scripts/work-item-create-remote.sh:99-111` — Jira create classification
- `skills/work/scripts/work-item-update-remote.sh:51-72` — update classification, both providers
- `skills/work/scripts/work-item-push-decide.sh:89-108` — the only four-way branch
- `skills/work/scripts/work-item-fetch-remote.sh:21-25` — the three-way partition
- `skills/work/scripts/work-item-fetch-remote.sh:155-181` — Linear's 250-item truncation
- `skills/work/scripts/work-item-project-remote.sh:65-93` — the projection recipe
- `skills/work/scripts/work-item-sync-baseline.sh:10-17` — `last-sync.json` schema
- `skills/work/scripts/work-item-sync-classify.sh:160-196` — the classification 2×2
- `skills/integrations/linear/scripts/linear-search-flow.sh:157-165` — bulk query, no `description`
- `skills/integrations/jira/scripts/jira-resolve-fields.sh:71-79` — the `kind` mapping
- `.accelerator/state/integrations/linear/last-sync.json` — committed real baseline
- `skills/integrations/jira/scripts/test-fixtures/scenarios/apply-push-204-show.json:17` — a real Jira `+0000` stamp

## Architecture Insights

- **Enforcement is membership-derived, not registry-derived.** `cargo fmt --all`,
  `clippy --workspace`, `cargo deny`, `cargo pup` and `nextest --workspace` all take
  their scope from `[workspace].members`. The hand-maintained registries
  (`DISPATCHED_SUBBINARIES`, `_CLI_RELEASE_BINARIES`, `DEBUG_ARCHIVE_DIRS`) exist only
  for *dispatched binaries*. A library crate joins everything by being a member — with
  the single exception of cargo-pup, which is per-crate and unguarded.
- **cargo-pup's import-path matching shapes the source style.** The one-item-per-`use`,
  `crate::`-qualified discipline exists because the tool cannot resolve grouped
  imports. It looks like a formatting preference and is actually a tool constraint.
- **Ports are declared in the domain crate, implemented in the adapter, and selected
  at the `-cli` composition root**, passed as `&dyn Trait` — concrete adapters are
  zero-sized unit structs constructed inline. There is no DI container and, outside
  `ConfigStack` and `MigrationEntry`, almost no boxing.
- **The bash sync path is conservative by construction.** Unproven means terminal;
  absent is only ever inferred from a provably complete fetch. Both defaults exist to
  avoid a destructive wrong answer, and both are properties a type signature can
  silently discard.
- **Timestamps are cache keys, not clocks.** Nothing in the sync path parses or orders
  a remote timestamp; equality is the only operation. This is why the opaque newtype
  is right and why an "improvement" to a real time type would be a regression.

## Historical Context

- `meta/work/0194-tracker-crate-and-remote-sync-engine.md` — the parent this crate was
  split out of on 2026-08-10; owns the sync state machine, the `sync` command, the
  shared reusable fake and the pending-push marker.
- `meta/reviews/work/0194-…-review-2.md` — the review whose clarity, scope and
  dependency lenses independently recommended the split.
- `meta/reviews/work/0204-remote-tracker-port-review-1.md` — three passes; settled the
  synchronous/dyn-compatible trait, the crate-local error type, the opaque timestamp
  and the four-operation surface.
- `meta/work/0171-jira-and-linear-integrations.md` — the client adapters that
  `impl RemoteTracker`; unblocked by this item, and the inheritor of the projection
  and bulk-query constraints above.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md` — the
  ports-and-adapters shape.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` — establishes
  `external_id` as the remote identifier and sync classification as presence-based.
- **Closest precedent plans**:
  `meta/plans/2026-07-11-0179-corpus-crates-parsing-conventions.md` (five crates in one
  plan — the richest), `meta/plans/2026-07-07-0178-config-crates-native-yaml-reader.md`
  (domain + adapter pair), `meta/plans/2026-08-06-0170-work-item-lifecycle-subdomain.md`
  (the neighbouring subdomain).
- **Validations worth reading for deviation patterns**:
  `meta/validations/2026-07-11-0179-…-validation.md` and
  `meta/validations/2026-08-06-0170-…-validation.md`. Note there is **no** validation
  for 0178 — the earliest crate-adding plan with one is 0179.

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — the source this work item derives from
- `meta/research/codebase/2026-08-06-0170-work-item-lifecycle-subdomain.md` — the
  closest domain precursor
- `meta/research/codebase/2026-06-18-0051-sync-work-items-skill.md` — the shell sync
  engine's behaviour
- `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md` and
  `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` — the two
  provider API surfaces the port must remain satisfiable by
- `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md` —
  the existing CLI that talks to a remote service, closest in shape

## Open Questions

1. **What replaces `cargo public-api` in AC 1?** Recommended: the self-reading
   pinned-surface test. Needs a decision before the criteria can be written.
2. **Does `fetch_all` contract truncation as `Err`, or does the return type change
   before the freeze?** The only question here that can make the port give a wrong
   answer.
3. **Should `fetch_all` be key-scoped?** The bash bulk path is; an unscoped fetch
   changes cost and blast radius for the Jira client.
4. **Does "no dependencies at all" include `[dev-dependencies]`?** Nothing planned
   needs one; state the reading explicitly.
5. **Where does the projection live?** `work-adapters::project_remote` is written,
   unused, and unreachable from 0171's clients without dragging in `work`. 0204 need
   not move it, but somebody must own the answer.
6. **Does the pup rule keep the inert `kernel::Error` allowance?** Recommended: drop
   it; it misdescribes a crate that cannot compile such an import.
