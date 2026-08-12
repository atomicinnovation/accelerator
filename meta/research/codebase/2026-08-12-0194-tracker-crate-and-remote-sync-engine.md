---
type: codebase-research
id: "2026-08-12-0194-tracker-crate-and-remote-sync-engine"
title: "Research: Educating a plan for 0194 — Tracker Crate and Remote Sync Engine"
date: "2026-08-12T21:29:42+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0194"
parent: "work-item:0194"
relates_to: ["codebase-research:2026-08-11-0204-remote-tracker-port", "codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
topic: "Educating a plan for 0194 — Tracker Crate and Remote Sync Engine"
tags: [research, codebase, rust, sync, tracker, work-items, bash-parity]
revision: "211759d5ec960752fcf8bd6dc26504fd5021906e"
repository: "accelerator"
last_updated: "2026-08-12T21:29:42+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Educating a plan for 0194 — Tracker Crate and Remote Sync Engine

**Date**: 2026-08-12T21:29:42+00:00
**Author**: Toby Clemson
**Git Commit**: `211759d5ec960752fcf8bd6dc26504fd5021906e`
**Branch**: no bookmark on the working copy (change `rvkxonnvlwro`); the item's own bookmark `0194-tracker-crate-and-remote-sync-engine` sits at `3fe80be8`
**Repository**: accelerator

## Research Question

What does the codebase actually contain today that a plan for
`meta/work/0194-tracker-crate-and-remote-sync-engine.md` must build on, preserve,
or correct — the bash sync engine's real behaviour, its real test coverage, the
`tracker` port as shipped, the `work`/`work-adapters`/`accelerator-work` surface,
and the workspace conventions (pup, deny, public-api, nextest, fakes) the new code
must satisfy?

## Summary

The story is buildable as written, with one structural decision still open and
**six factual corrections** the plan must absorb before it is written. Nothing
found blocks the work; several findings shrink it.

The corrections, ordered by how much they change the plan:

1. **`work-item-push-decide.sh` is already covered.** The item's dedicated
   characterization-test requirement (Requirements bullet, AC 13) is largely
   discharged: `test-work-item-create-remote.sh:267-289` drives nine rows of the
   push-decision table under an explicit banner. Only two paths are genuinely
   uncovered — the non-integer argument error (`work-item-push-decide.sh:74-77`)
   and the usage arm. The characterization test shrinks from "the whole script"
   to "two error rows".
2. **There are no fixture *tables* in `test-work-item-scripts.sh` to lift.**
   Every case is an inline `assert_eq` with literal arguments. Only two
   constructs are table-shaped (a 3×4 `for` loop over noop states at
   `:432-436`, and a pairwise label-distinctness loop at `:66-84`). The
   "lift the fixture tables into Rust test data" work is transcription, case by
   case, not a mechanical port of a delimited table. Budget for it accordingly.
3. **`work-item-sync-label.sh` renders five states, not seven.** Its `--label`
   arm (`work-item-sync-label.sh:54-67`) rejects `remote-absent` and
   `indeterminate` with exit 1. The new `work-item-sync-label.golden` must
   encode those two as **exit-1 rows**, and AC 20's "covering each of the seven
   classified states" needs that reading written into it.
4. **`work-item-sync-label.sh` has exactly one live consumer, not three.**
   `linear-create-flow.sh:304-306` and `jira-resolve-fields.sh:139-142`
   hand-copy the `sed` normalisation in a comment-annotated duplicate; they never
   invoke the script. The coupling is a convention across three copies of one
   `sed` expression — which makes it *more* fragile than the item assumes, not
   less: a Rust trimming change that diverges from those two `sed`s is silent.
5. **`work-item-sync-baseline.sh` has no `finalise` subcommand.** Its verbs are
   `path`/`get`/`set`/`set-timestamp`/`remove`; `finalise` is a sibling
   sub-action on `work-item-sync-apply.sh:179-194` that delegates to
   `set-timestamp`. The preview criterion (AC 7) is right about the behaviour and
   wrong about where it lives.
6. **`work-adapters/src/project_remote.rs` does not depend on `work`.** Its only
   import is `serde_json::Value` (`project_remote.rs:13`). The item's argument
   that a 0171 client reusing it "would acquire the whole lifecycle domain" is
   false as stated — the *crate* depends on `work`, the *module* does not. The
   substantive half of the argument survives (each client owns its own recipe),
   but the plan should not repeat the dependency claim.

The **open boundary question the item flags is real and now precisely costed**.
`work` importing `tracker` fires `work_domain_imports_only_permitted`
(`cli/pup.ron:92-107`) at the first `use tracker::…`, exit 101. Widening it costs
one permit-list line plus **two mandatory probe edits** — `"tracker"` into
`_ALL_EXTRA_CRATES` (`tests/integration/pup/test_import_rule.py:725`) and into
`work`'s extras tuple in `_DOMAIN_RULES` (`:712`) — which auto-generates
cross-rejection cases proving corpus/vcs/migrate still reject `tracker`. The
alternative (translate at the `work-adapters` boundary) costs nothing in pup
terms: `work-adapters` has no whole-crate rule, only a module-scoped one on
`work_adapters::filesystem`. My recommendation is in Architecture Insights.

Two further things the plan gets for free, and one it does not:

- **Free**: `work` is already pinned by cargo-public-api
  (`tasks/public_api.py:15-26`), so every public classify/decide type churns
  `cli/work/tests/fixtures/public-api.txt`; `work-adapters` is exempt, so
  adapter-side types churn nothing. Deliberate placement can halve the snapshot
  noise.
- **Free**: the excluded-suite pattern already exists and is *not* `#[ignore]`
  and *not* nextest test-groups — it is a separate test **binary** selected by
  `-E 'binary(name)'` from a dedicated invoke task
  (`tasks/test/integration.py:159-178`). There is no `nextest.toml` in the repo.
- **Not free**: `cargo nextest` runs `--workspace --all-features`
  (`tasks/test/cli.py:11-16`), so a Cargo feature cannot gate the contract suite
  out of the default run. The separate-binary route is the only one that works.

## Detailed Findings

### The classifier — `work-item-sync-classify.sh`

Seven keywords, emitted in this order of reachability: `unsynced` (:129),
`indeterminate` (:135), `remote-absent` (:139), then the 2×2 verdict at
:188-196 — `synced` / `locally-modified` / `remotely-modified` / `conflict`.

The algorithm is two independent side computations, each defaulting to
**changed**, then a 2×2 table:

```
local_changed  = 1                                    # default
if base_local_hash present:
    if mtime <= timestamp: local_changed = 0          # advisory, inclusive <=
    elif sha256(normalise(file)) == base_local_hash: local_changed = 0

remote_changed = 1                                    # default
if base_remote_hash present:
    if base_remote_updated non-empty AND remote_updated == base_remote_updated:
        remote_changed = 0                            # trusted short-circuit
    elif remote_body_file exists:
        if sha256(normalise --stdin < body)) == base_remote_hash: remote_changed = 0
```

Three non-obvious properties the Rust port must reproduce:

- **A missing baseline hash means changed, and short-circuits the work.** With
  `base_local_hash` empty, neither `stat` nor the hash runs at all
  (`:161-172`). This is what makes first-sync-on-dirty resolve to `conflict`
  rather than `synced` — both sides default changed.
- **The `[ -n "$base_remote_updated" ]` guard at :177** exists because both
  sides default to `""`. Without it, `"" = ""` declares the remote unchanged on
  zero evidence. `RemoteTimestamp::proves_unchanged_since`
  (`cli/tracker/src/lib.rs:88-102`) already carries exactly this rule — call it,
  never `==`.
- **The updated-equality short-circuit is gated on `remote_hash` presence.** A
  baseline entry carrying `remote_updated_at` but no `remote_hash` is always
  remote-changed. That asymmetry is easy to lose in a rewrite.

`_wisc_mtime` (`:66-80`) keeps its `||` fallback **outside** the command
substitution because GNU `stat -f %m` prints a `File:` block to stdout and exits
non-zero. A missing/non-numeric mtime becomes the sentinel `9999999999`, forcing
the hash path. The Rust port reads mtime directly and needs no sentinel, but
the *semantics* — unreadable mtime forces a hash rather than skipping — must
survive.

### The decision table — `work-item-sync-decide.sh`

Complete, from `:112-149`. Six outcomes: `push`, `pull`, `skip-conflict`,
`skip-dirty`, `prompt`, `noop`.

| State | bidirectional | push-only | pull-only |
|---|---|---|---|
| `synced` | noop | noop | noop |
| `unsynced` | noop | noop | noop |
| `remote-absent` | noop | noop | noop |
| `indeterminate` | noop | noop | noop |
| `locally-modified` | push | push | **noop** |
| `remotely-modified` (clean) | pull | **noop** | pull |
| `remotely-modified` (dirty) | prompt | **noop** | skip-dirty |
| `conflict` | prompt | **skip-conflict** | **skip-conflict** |

`--dirty` affects only the two `remotely-modified` rows; elsewhere the harness
omits it entirely via `${3:+--dirty "$3"}` (`:422`), so the lifted Rust table
needs `dirty: Option<bool>`, not `bool`. `--dirty` is **not validated** — the
only test is `[ "$dirty" = "1" ]`, so `true`/`yes`/`0`/empty all read as clean.

The token resolver (`:151-165`) case-folds **then** trims (`tr` then `sed`), so
interior whitespace survives and `remote foo` does not match. `remote` →
`accept-remote`, `local` → `push-local`, and **everything else including empty**
→ `skip`. There is no error path.

### The baseline — `work-item-sync-baseline.sh`

Document shape (`:10-17`, written at `:117-119`):

```json
{"timestamp": 1750000000,
 "items": {"0194": {"remote_updated_at": "…", "remote_hash": "…", "local_hash": "…"}}}
```

Keyed by the **local id**, not the external id. `timestamp` is an integer epoch
(`--argjson`); all three item fields are JSON strings (`--arg`), so a failed
post-push read persists `""`, not `null`. Written via `atomic_write`
(`scripts/atomic-common.sh:16-32`): same-dir `mktemp`, `cat >`, `mv`, no fsync.

Degrade-to-empty (`:79-86`) is `[ -f "$f" ] && jq -e . "$f"` — a missing file and
a present-but-unparseable file both yield `{"timestamp":0,"items":{}}`. Every
mutating verb is a **read-modify-write of the whole document** with no locking:
concurrent writers lose updates. The Rust port inherits that single-writer
assumption unless it deliberately changes it.

`timestamp` is the **run-start** epoch, captured before any item is read
(`sync-work-items/SKILL.md:86-88`) and written only on clean completion
(`:322-328`). Using run-start means a file edited *during* the run has
`mtime > timestamp` and is re-hashed next time rather than wrongly skipped —
which is exactly what AC 9's "strictly earlier than the mtime of any file the
run mutated" is protecting.

### Apply and the resumability seam — `work-item-sync-apply.sh`

Verbs: `push`, `pull`, `finalise` (`:196-220`). The fault seam (`:64-71`)
requires **both** `ACCELERATOR_TEST_MODE=1` and
`WORK_SYNC_FAIL_AFTER=side-effect`, and calls `exit 99` — it kills the process,
it does not return. It sits strictly between side effect and baseline write in
both `push` (`:118`) and `pull` (`:170`).

`push` ordering (`:110-133`): remote **update** call → fault hook → post-push
`show` → projection + normalise → `local_hash` from the file on disk →
`baseline set` **last**. Two details worth pinning:

- A failed remote call **returns the dispatch code verbatim and leaves the
  baseline entry unset** (`:114-116`). The item's `E_DISPATCH_TERMINAL`-clears-
  the-entry rule (AC 12) is therefore a *deliberate change* from bash for the
  `update --push` path, not a port of it — the plan should say so.
- A failed post-push `show` leaves both remote fields `""` and **still writes the
  baseline entry** (`:121-129`). That is where `RemoteTimestamp::NotRead` is
  written, and where the lossy `""` round trip the item flags actually bites.

`pull` (`:136-177`) writes via `atomic_write` then derives **both** hashes from
post-overwrite state — `local_hash` from the just-written file, `remote_hash`
from the projection actually written. Deriving either from pre-pull content
self-corrupts into a phantom `locally-modified`.

`--preview` **is not a flag on any script**. It is enforced entirely by the
SKILL (`sync-work-items/SKILL.md:76-79, 320-333`): dry-run bridges, report pulls
instead of writing, no `set`, no `finalise`. Passing `--preview` to `apply`
hits the `*)` arm and exits 2. The Rust `--preview` is therefore new surface,
not a port — which makes AC 7's three-observable check the only specification
of it that exists.

### Projection, normalisation, and hashing

```
remote_hash = sha256( normalise --stdin < project_remote(body, show_json) )
local_hash  = sha256( normalise(work_item_file) )
```

`work-item-project-remote.sh:66-87` — jira `body` is
`summary\n` + `jq -cS '.fields.description // null'`; linear `body` is
`title\n` + description verbatim. `-S` is load-bearing for jira: ADF key ordering
must not flip equality. Absent description projects as the literal four bytes
`null` (jira) or an empty line (linear).

`work-item-normalise.sh` forces `LANG=C LC_ALL=C` at `:31` — process-wide, so
`[[:space:]]` is ASCII and the digest is machine-independent. `cli/work/src/
normalise.rs:23` already mirrors this with `is_ascii_whitespace`, and the test
at `:110-114` pins that a trailing U+00A0 survives. A Unicode `str::trim()`
would silently reclassify every synced item.

File mode output is **always** `fm + "\n" + body + "\n"` (`:114-116`), so empty
frontmatter yields a leading blank line. `IGNORE_KEYS` is exactly six keys
(`normalise.sh:38`, mirrored at `cli/work/src/normalise.rs:14-21`), matched only
at column 0 — nested keys never match, and the unconditional trim destroys their
indentation (pinned by `normalise.rs:117-130`).

`cli/work-adapters/src/project_remote.rs` is a complete port of the two recipes,
using `serde_json` without `preserve_order` so BTreeMap ordering reproduces
`jq -cS` with no explicit sort (`:42-48`). It has no caller. Wiring it is
in-scope only in the sense that the *fake* must produce equivalent bodies; the
port itself returns already-projected `RemoteIssue`.

### The `tracker` port as shipped

`cli/tracker/src/lib.rs`, package name **`tracker`** (bare, not
`accelerator-tracker` — pup matches whole crate names). Zero dependencies, zero
`[features]`, enforced by `tests/structure.rs:54-65`.

```rust
pub trait RemoteTracker {                                       // :249
    fn create(&self, title: &str, body: &str, kind: &str) -> Result<ExternalId, TrackerError>;
    fn update(&self, id: &ExternalId, title: &str, body: &str) -> Result<(), TrackerError>;
    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError>;
    fn fetch_all(&self, ids: &[ExternalId]) -> Result<FetchOutcome, TrackerError>;
}
```

Synchronous, `&self`, dyn-compatible, no default bodies. `RemoteTimestamp` is the
three-variant enum `Reported(String) | NotReported | NotRead` (`:50-75`) with
**no `PartialOrd`** — comparison is `proves_unchanged_since` only.
`FetchOutcome` is a bare three-`Vec` struct with **no constructor and no
methods** (`:226-236`); totality is an unenforced obligation, explicitly handed
to this story.

`tracker/tests/port.rs` holds a **private** `FixedTracker` with three named
`const fn` constructors — `holding` / `truncating` / `losing` (`:20-50`) — and
the free function `partitions_totally(&FetchOutcome, &[ExternalId])`
(`:238-262`), which asserts no id appears twice and coverage is exactly the
distinct requested set. That function is dependency-free and lifts verbatim.

One inherited gap the validation records
(`meta/validations/2026-08-11-0204-remote-tracker-port-validation.md:117-121`):
a fifth trait method with a default body would be caught by neither
`public-api:check` nor `port.rs`. Worth one contract-test case here if judged
worth the cost.

### `work` / `work-adapters` / `accelerator-work` as they stand

`work` has **ten** modules — the item's list of ten misses `file_dirty`
(`cli/work/src/file_dirty.rs:8-20`, `VcsMode` + `is_dirty`), which is written,
tested, and **wired into nothing**. It is exactly the `--dirty` input the
decision table needs, so the plan should wire it rather than write it.

`work-adapters` has four modules: `author`, `diff_shellout`, `filesystem`,
`project_remote`. Crate deps are `work`, `kernel`, `vcs`, `vcs-adapters`,
`tempfile`, `serde_json` — **no `store`, no `corpus`**. Persistence reaches
`store` only via `work-cli`'s dependency on `corpus-adapters`. Siting `baseline`
in `work-adapters` therefore adds a `corpus`/`store` edge that does not exist
today.

`accelerator-work` (`cli/work-cli/`) is clap-derive, one `Subcommand` enum
(`cli.rs:19-75`), one `run_*` fn per arm each repeating a `compose(&start,
LegacyPolicy::Reject)` preamble (`main.rs:192-221`). Ports are passed as `&dyn`
arguments or concrete ZSTs at the call site — no container, no registry. A
`Box<dyn RemoteTracker>` selected from `work.integration` fits that shape
directly.

Four touch points for `--push`:

1. `CreateArgs` (`cli.rs:113-159`) / `UpdateArgs` (`cli.rs:85-108`).
2. The mirror struct + `create_args_from_cli` (`main.rs:172-190`); `update` has
   no mirror, so one edit fewer.
3. `create::try_run` after the single atomic write (`create.rs:300-302`) and
   `update::try_run` after its write (`update.rs:177-181`). **`run_update`
   composes no config today** (`main.rs:223-231`) — `update --push` requires
   adding the whole `compose(...)` preamble.
4. `cli/work-cli/tests/fixtures/cli_surface.golden` — every subcommand's
   `--help` is frozen (`cli_surface.rs:32-56`); adding flags is a deliberate
   golden update.

Exit codes: `accelerator-work` is the **one** sub-binary without the shared
`report(&kernel::Error)` helper — each arm hardcodes its own code to match the
bash original (`main.rs:57-77`, `:159-167`). A new sync exit code for
"resolution needed" (AC 5) belongs in that same hand-rolled style, and 2 is
already spoken for as the refusal/ambiguity code across the workspace.

### Test infrastructure the plan must use

| Need | Established pattern | Where |
|---|---|---|
| Suite excluded from default run | Separate `tests/*.rs` binary + invoke task with `-E 'binary(name)'` | `tasks/test/integration.py:159-178` |
| Shared fake across crates | A sibling **crate** (`<domain>-test-support`), dev-dep only | `cli/vcs-test-support/src/lib.rs:1-11` |
| Golden comparison | Parse both sides, mask volatile fields, ship `regenerate.sh` + README | `cli/vcs-cli/tests/detect_goldens.rs:24-31` |
| Call-order assertion | `SpyReporter` with `RefCell<Vec<String>>` of `verb:payload` strings, `assert_eq!` on the slice | `cli/migrate/tests/lifecycle.rs:105-179` |
| Injected clock | `corpus::Clock` port; fake redefined per test file | `cli/corpus/src/metadata.rs:13-18`, `corpus-adapters/tests/metadata.rs:20-38` |
| Atomic write | `store::atomic_write` behind `corpus::AtomicWrite` | `cli/store/src/lib.rs:92`, `cli/corpus/src/store.rs:59-64` |
| Table over lifted data | JSON file + one test iterating, **with a row-count assertion** | `cli/vcs-cli/tests/guard_decision_table.rs:143-182` |

Two constraints that rule options out. `--all-features` in the default run
(`tasks/test/cli.py:11-16`) means a Cargo feature cannot exclude a suite —
`bash-parity` gates whole files but is *on* in CI. And `vcs-test-support`'s
manifest comment (`Cargo.toml:12-23`) records why a `test-support` feature was
rejected: it would force the crate under test to compile with the fixture
feature via a normal edge. The shared `RemoteTracker` fake this story owes 0204
should follow that precedent as a `tracker-test-support` crate, not a feature.

Bash-side floor: `_EXPECTED_WORK_SUITES = 5` (`tasks/test/integration.py:47`) is
an **at-least** check asserted by `tests/unit/tasks/test_integration.py:63-98`.
Since this story removes nothing, the floor is untouched — but a *new* bash
suite would need the constant raised in the same change. The comment at `:43-46`
already names six suites for a floor of five and is stale; worth fixing in
passing.

### Architecture enforcement

`work_domain_imports_only_permitted` (`cli/pup.ron:92-107`) permits
`^(std|core|alloc)(::|$)`, `^kernel::Error(::|$)`, `^corpus(::|$)`,
`^crate(::|$)`. `use tracker::…` in `cli/work/src/**` matches none; pup emits a
`severity: Error` finding naming the rule and exits 101.

Two mechanics that will bite mid-implementation if unknown:

- **Grouped imports break `allowed_only`.** cargo-pup resolves
  `use a::{b, c}` to an empty module name, which any permit list rejects
  (`cli/pup.ron:151-153`, pinned by `test_import_rule.py:602-609`). Every module
  under such a rule writes one single-item `use` per import.
- **`denied` beats `allowed_only`** on overlap — the message differs
  (`is denied` vs `is not allowed`), which is how the probes tell them apart.

`cli/deny.toml:100-123` denies `native-tls`, `openssl`, `openssl-sys` outright.
That is not this story's problem (no HTTP here) but it constrains 0171 and
therefore the port's shape. `sources` denies all git dependencies
(`:125-128`).

cargo-public-api pins `work` and exempts `work-adapters`
(`tasks/public_api.py:15-26`, `:38-43`). Snapshot at
`cli/work/tests/fixtures/public-api.txt`, rendered with
`--include function-parameter-names`, updated only via
`mise run public-api:update` after reading the diff (`tasks/README.md:555-558`).

Task composition: `cli:check` is rustfmt + one workspace clippy + four Python
guards — it does **not** include pup, deny, or public-api. Those are siblings
under the aggregate `check` (`mise.toml:598-600`). `test:integration:pup` runs
only in the `check-architecture` CI job, not the test roll-up. So a pup
regression will not show up in `mise run cli:check`.

### Consumers that must keep working

`sync-work-items/SKILL.md` invokes eighteen distinct script calls in run order
(fully enumerated by the consumer analysis). Load-bearing details for the Rust
command's contract:

- **Bulk-then-show is already the rule**, owned by the SKILL, not by any script.
  One `search --keys` call, then `show` only for keys whose `updated` differs
  from the baseline (`:96-98`, `:119-127`). A failed bulk call marks **every**
  key indeterminate and writes nothing (`:101-104`). AC 18's call-count
  criterion is asserting existing behaviour, not new.
- Two `AskUserQuestion` blast-radius gates at threshold **25**, both failing
  safe to abort with zero writes (`:172-181`, `:288-299`). Neither is in this
  story's scope (they are the SKILL's), but the binary must not make them
  impossible — a batch-mode command that writes before reporting would.
- The conflict prompt is a **typed token, not `y/N`** (`:209-213`), with one
  re-ask then `skip`. The `--resolve` grammar this story adds is the machine
  half of that same vocabulary.

`list-work-items/SKILL.md` short-circuits **before** the classifier: no
`external_id` → presence-only `unsynced`; `external_id` with an empty baseline
entry → presence-only `synced` (`:316-319`). Sync has no such carve-out, which
is precisely why first-sync-on-dirty reaches `conflict` only from sync. Do not
"fix" the asymmetry.

`skills/work/scripts/EXIT_CODES.md` documents the 70/71/72/73 taxonomy as
derived from `work-item-bridge-codes.sh` (source of truth). 72 and 73 resolve
**above** the port — at the composition root selecting the client — which is
already recorded in `cli/tracker/tests/fixtures/dispatch-codes.txt`. That
fixture is the natural anchor for this story's Rust-side taxonomy assertion.

## Code References

- `skills/work/scripts/work-item-sync-classify.sh:161-196` — the two-side
  default-changed computation and the 2×2 verdict
- `skills/work/scripts/work-item-sync-classify.sh:177` — the
  `[ -n "$base_remote_updated" ]` guard; now `proves_unchanged_since`
- `skills/work/scripts/work-item-sync-decide.sh:112-149` — the full decision
  table; `:151-165` the token resolver
- `skills/work/scripts/work-item-sync-baseline.sh:79-86` — degrade-to-empty;
  `:110-136` the mutating verbs
- `skills/work/scripts/work-item-sync-apply.sh:64-71` — the fault seam;
  `:110-133` push ordering; `:168-176` pull ordering; `:179-194` `finalise`
- `skills/work/scripts/work-item-project-remote.sh:66-87` — both recipes
- `skills/work/scripts/work-item-normalise.sh:31` — the `LANG=C` forcing
- `skills/work/scripts/work-item-push-decide.sh:74-106` — the outcome table
- `skills/work/scripts/work-item-bridge-codes.sh` — the `E_DISPATCH_*` taxonomy
- `skills/work/scripts/test-work-item-create-remote.sh:267-289` — the existing
  push-decide coverage
- `skills/work/scripts/test-work-item-scripts.sh:432-436` — the one true matrix
- `cli/tracker/src/lib.rs:88-102` — `proves_unchanged_since`; `:226-236`
  `FetchOutcome`; `:249-342` the trait
- `cli/tracker/tests/port.rs:20-50` — the private fake; `:238-262`
  `partitions_totally`
- `cli/work/src/normalise.rs:14-95` — the ported normaliser
- `cli/work/src/file_dirty.rs:8-20` — the unwired dirty check
- `cli/work-adapters/src/project_remote.rs:53-80` — the ported projection
- `cli/work-cli/src/main.rs:192-231` — the composition preamble and the
  config-less `run_update`
- `cli/pup.ron:92-107` — the rule the `work` → `tracker` edge trips
- `tests/integration/pup/test_import_rule.py:709-725` — `_DOMAIN_RULES` and
  `_ALL_EXTRA_CRATES`, both of which a widening must edit
- `tasks/test/cli.py:11-38` — `--all-features`, so features cannot gate suites
- `tasks/test/integration.py:47` — `_EXPECTED_WORK_SUITES`; `:159-178` the
  separate-binary exclusion pattern
- `tasks/public_api.py:15-43` — `work` pinned, `work-adapters` exempt
- `skills/work/sync-work-items/SKILL.md:96-127` — bulk-then-show;
  `:320-333` the preview suppression

## Architecture Insights

**On the `work` → `tracker` boundary — recommend widening the pup rule.** Three
reasons. The precedent already exists and is exactly parallel: `work` was
widened once for `^corpus(::|$)` to admit `WorkItemIdScheme`/`IdScanner`, and
`migrate` carries two such allowances. `tracker` is a zero-dependency, zero-logic
port crate — importing it cannot drag a transitive graph into the domain, which
is the property the rule exists to protect. And the alternative pushes the
classifier's inputs through a translation layer that would need its own
domain-shaped timestamp type, duplicating `RemoteTimestamp` and its
`proves_unchanged_since` rule — the single most dangerous thing in this story
to have two copies of. The cost is one permit line and two probe-table edits,
both mechanical. The counter-argument worth stating in the plan: it makes
`tracker` a second "blessed" domain dependency, and the next port crate will
cite it.

**Preview is new surface, not a port.** No script takes `--preview`; the SKILL
implements it by routing to dry-run bridges. That means AC 7's three observables
(byte-identical baseline, unchanged file content *and* mtime, zero fake
`create`/`update` calls) are the only specification that exists, and AC 8's
plan-fidelity check is what stops the two implementations diverging. Both are
well-chosen; the plan should treat them as the definition rather than as
verification of something specified elsewhere.

**The parity strategy is asymmetric and that is correct.** Because nothing is
removed, every duplication this story creates survives it. The lifted tables
hold both implementations to one oracle. But note what the oracle *is*: for
classify/decide it is transcribed assertions, for normalise/project it is
committed case directories, and for label it is a fixture that does not exist
yet. Only the second kind is machine-checkable against both sides today. Writing
the label golden in the case-directory style would be over-engineering (the
script is scalar-in/scalar-out) — the flat `|`-delimited form with `[CLASSIFY]`
and `[LABEL]` sections is right, with the two rejected states as exit-1 rows and
an explicit note that outputs carry **no trailing newline** (`printf` without
`\n`, `work-item-sync-label.sh:57-61`).

**Baseline siting pulls a new edge.** `work-adapters` depends on neither `store`
nor `corpus` today. Putting baseline persistence there is still right per the
item's reasoning (no domain crate may import `store`), but the plan should name
the new dependency explicitly and check it against `cli/deny.toml` — a
`serde_json`-shaped JSON document store is fine, but `work-adapters` currently
has no JSON *document* facility, only `project_remote`'s parsing. `corpus`
offers JSONL, not JSON. This is hand-rolled.

**The two-invocation conflict flow needs an exit code that is not 2.** Across the
workspace, exit 2 means refusal (`report()` in five sub-binaries) or ambiguity
(`accelerator work resolve`). The "resolution needed" code AC 5 demands must be
distinct from 0, 1 and 2, and should be documented in
`skills/work/scripts/EXIT_CODES.md` alongside the taxonomy it will eventually
join — even though no SKILL consumes it until 0171.

**Phasing holds, with one shift.** A (state machine) → B (`sync` command) → C
(`--push` wiring) is sound. The shift: the `work-item-push-decide.sh`
characterization work the item puts in Phase A is nearly done already, so Phase A
shrinks; and `update --push` needs `run_update` to gain a composition preamble it
has never had, so Phase C is larger than "add a flag". Net, the phases are closer
in size than the item implies.

## Historical Context

- `meta/work/0204-remote-tracker-port.md` — accepted and implemented 2026-08-12;
  `RemoteTimestamp` became an enum on implementation review, a **breaking**
  change from the newtype the plan text still shows
  (`meta/plans/2026-08-11-0204-remote-tracker-port.md:1117-1149` is stale on this
  point). Neither 0171 nor 0194 had started, so nothing broke.
- `meta/plans/2026-08-11-0204-remote-tracker-port.md:2686-2849` — the handoff
  section, naming five obligations to this story: the pending-push marker, the
  shared reusable fake, the widened contract test including a `fetch_all`
  partition case, the empty-stamp trap, and the extra post-push `show`.
- `meta/validations/2026-08-11-0204-remote-tracker-port-validation.md:117-121` —
  the default-bodied-method gap, confirmed unguarded.
- `meta/reviews/work/0194-tracker-crate-and-remote-sync-engine-review-2.md` —
  verdict REVISE on thirteen findings; the item has since been revised against
  all of them, and the two splits (0171 cutover, 0204 port) came out of it.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  and `ADR-0045-skills-vs-cli-division-of-labour.md` — the two decisions the
  non-interactivity requirement follows from: judgment belongs to the skill, the
  binary takes orders.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` — the
  `external_id` convention the classifier's first branch tests.
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — Open Question 2 resolves the work↔integrations coupling the Assumptions
  section cites.

## Related Research

- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md` — the port's
  own research pass
- `meta/research/codebase/2026-08-06-0170-work-item-lifecycle-subdomain.md` — the
  local CRUD half this story extends
- `meta/research/codebase/2026-06-15-0047-core-skills-sync-integration.md` and
  `2026-06-18-0051-sync-work-items-skill.md` — the shell-era sync design
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
  — the full shell inventory

## Open Questions

- **Which `RemoteTimestamp` unknown does a baseline round trip resolve to?**
  Both `NotReported` and `NotRead` persist as `""`, and the on-disk value cannot
  say which. The item requires the choice be made deliberately and recorded.
  Classification is unaffected; a sync *report* is not. Nothing in the codebase
  decides this — it is a plan-time call.
- **Does the story raise `_EXPECTED_WORK_SUITES`?** It adds no bash suite as
  scoped, so no. But if the two uncovered `push-decide` error rows land as a
  *new* bash suite rather than an extension of
  `test-work-item-create-remote.sh:267-289`, the constant and
  `tests/unit/tasks/test_integration.py` both need editing in the same change.
  Extending the existing suite avoids that entirely and is the cheaper route.
- **Is the default-bodied-method gap worth a contract-test case?** 0204 handed it
  over explicitly. Detecting it needs something like a compile-time assertion
  that every method is required, which Rust does not offer directly — the
  practical option is a test that calls all four through a trait object on a
  fake that `unimplemented!()`s nothing, which is what `port.rs:144-167` already
  does and which would *not* catch it. Judged cost/benefit call for the plan.
- **Where does the Rust-side `E_DISPATCH_*` assertion live?**
  `cli/tracker/tests/fixtures/dispatch-codes.txt` already pins 70/71 against
  `TrackerError` and records 72/73 as above-the-port. The item wants a single
  Rust owner asserted against `work-item-bridge-codes.sh`. Whether that is a
  second fixture in `work-adapters` or a reuse of tracker's is unsettled;
  reusing tracker's risks a test crossing a crate boundary for fixture data.
