---
type: plan
id: "2026-08-13-0194-tracker-crate-and-remote-sync-engine"
title: "Tracker Crate and Remote Sync Engine Implementation Plan"
date: "2026-08-12T23:16:02+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0194"
parent: "work-item:0194"
derived_from: ["codebase-research:2026-08-12-0194-tracker-crate-and-remote-sync-engine"]
relates_to: ["plan:2026-08-11-0204-remote-tracker-port", "plan-review:2026-08-13-0194-tracker-crate-and-remote-sync-engine-review-1"]
tags: [rust, sync, tracker, work-items, bash-parity]
revision: "5ab2154f4ebbbbe2f6fde5c392e8aa714c6eb3fd"
repository: "accelerator"
last_updated: "2026-08-13T12:05:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tracker Crate and Remote Sync Engine Implementation Plan

## Overview

Build the remote sync state machine over the `RemoteTracker` port 0204 froze,
the `accelerator work sync` command that drives it, and `--push` on 0170's
`create`/`update` — all tested against a fake tracker, all shipping beside the
live bash path rather than replacing it. 0171 performs the cutover.

## Current State Analysis

Nine bash scripts under `skills/work/scripts/` implement the sync engine today,
orchestrated by `skills/work/sync-work-items/SKILL.md`. The Rust side has the
port (`cli/tracker/`), a substantially-built `work`/`work-adapters` pair from
0170, and two already-ported pieces with no caller — `work/src/normalise.rs` and
`work-adapters/src/project_remote.rs`.

**What the bash engine actually does**, verified against source rather than the
split-time summary:

| Script | Shape | Rust destination |
|---|---|---|
| `work-item-sync-classify.sh` | two default-changed side computations → 2×2 verdict | `work::sync::classify` |
| `work-item-sync-decide.sh` | 7 states × 3 modes, `dirty` sub-splitting one state, + token resolver | `work::sync::decide` |
| `work-item-sync-label.sh` | 5-state glyph table; rejects 2 states | `work::sync::label` |
| `work-item-sync-baseline.sh` | one JSON doc, read-modify-write, degrade-to-empty | `work_adapters::sync::baseline` |
| `work-item-sync-apply.sh` | push/pull/finalise, side-effect-first, fault seam | `work_adapters::sync::apply` |
| `work-item-push-decide.sh` | (code × attempt × write-failed) → action | `work::sync::push_decide` |
| `work-item-project-remote.sh` | per-tracker projection | already `work_adapters::project_remote` |
| `work-item-normalise.sh` | ASCII trim + `IGNORE_KEYS` filter | already `work::normalise` |
| `work-item-bridge-codes.sh` | `E_DISPATCH_*` 70/71/72/73 | `work_cli::exit_codes` |

Four properties are easy to lose in a rewrite and are pinned by tests below. A
missing baseline hash means **changed** and short-circuits the work entirely
(`work-item-sync-classify.sh:161-172`) — that is what makes first-sync-on-dirty
resolve to `conflict`. Bash's presence tests are *emptiness* tests (`jq -r '…
// empty'` then `[ -n "$…" ]`, `:156-176`), so a persisted empty-string hash is
**absent**, not present — and an empty `remote_hash` is a routine on-disk value,
since a failed post-push read writes exactly that. The updated-equality
short-circuit is gated on `remote_hash` presence, so an entry carrying
`remote_updated_at` but no `remote_hash` is always remote-changed. And `pull`
derives **both** hashes from post-overwrite state (`:172-176`); deriving either
from pre-pull content self-corrupts into a phantom `locally-modified`.

Two safeguards live above the scripts, in `sync-work-items/SKILL.md`, and are
easy to lose precisely because no script holds them. The **aggregate
pull-overwrite gate** (`:172-181`) refuses before any pull write when a run
would overwrite more than the shared threshold of 25 local files, and fails
safe with zero writes when not interactive. And the SKILL reports
`indeterminate` items under `needs-retry` and `remote-absent` items under
`remote-absent` (`:168-170`) — states this plan's report contract must therefore
be able to express.

### Key Discoveries

- **`work` cannot import `tracker` as it stands.** `cli/pup.ron:92-107` permits
  only `std`/`kernel::Error`/`corpus`/`crate`; the first `use tracker::…` exits
  101. Resolved by widening — see Implementation Approach.
- **The default run has no suite-exclusion mechanism yet, but nextest supplies
  one.** `tasks/test/cli.py:11-16` is `--workspace --exclude
  accelerator-visualiser --all-features` with no `-E` filter, so
  `corpus-adapters`' `zero_spawn` binary runs in the default pass too; its
  dedicated task exists to run it in isolation for its own CI job.
  `--all-features` rules out a Cargo feature as the gate. There is no
  `nextest.toml` anywhere in the tree — but the pinned nextest (0.9.138,
  `mise.toml:30`) does support a profile-level `default-filter`, verified by
  feeding it `default-filter = 'not binary(contract)'`: the key parses and the
  only complaint is that no binary matched. A filter is therefore available and
  is the mechanism this plan uses — see Implementation Approach.

- **`corpus::Clock` has no epoch-seconds operation.** It is
  `now_utc_iso() -> String` plus `filename_timestamp(format) -> String`
  (`cli/corpus/src/metadata.rs:15-18`), and no `cli/` domain crate exposes an
  epoch anywhere. The baseline's `timestamp` is a JSON integer compared against
  file mtimes, so the run-start epoch needs a seam that does not exist yet — see
  Implementation Approach.

- **The decision table's `dirty` input needs wiring, not just porting.**
  `work/src/file_dirty.rs` is written and tested; nothing calls it. Its
  `VcsMode::Indeterminate → dirty` mapping is a deliberate fail-safe, for the
  reason `sync-work-items/SKILL.md:132-134` states: the recovery model is VCS
  revert, which cannot recover uncommitted working-copy changes. Its
  `status_text` contract also differs by mode — `jj diff --name-only` is a
  whole-tree list, `git status --porcelain -- <path>` is per-path — so "one probe
  per run or one per item" is a real choice, not an implementation detail.
- **`work-item-push-decide.sh` is already nine-tenths covered.**
  `test-work-item-create-remote.sh:267-289` drives every `--code` × `--attempt`
  row plus `--write-failed`. Only the non-integer-argument and usage arms are
  uncovered.
- **`work-item-sync-label.sh` renders five states, not seven.** Its `--label`
  arm (`:54-67`) rejects `remote-absent` and `indeterminate` with exit 1, and
  every output is `printf` without `\n`.
- **`work/src/normalise.rs` has no parity test** against
  `work-item-normalise.golden`. AC 19 is new work, not a re-point.
- **`work-item-normalise.sh` composes its two halves unconditionally.**
  `:113-116` is `fm=$(…); body=$(…); printf '%s\n%s\n' "$fm" "$body"` — command
  substitution strips each side's trailing newlines and `printf` adds exactly one
  back to **each**, whether or not that side is empty. Plain concatenation
  therefore diverges by a byte for an empty or all-blank body.

- **A file that does not open with a frontmatter fence makes the normaliser
  fail, not emit a leading newline.** `config_extract_frontmatter`
  (`scripts/config-common.sh:76-85`) has `NR == 1 && !/^---[[:space:]]*$/ { exit }`,
  and `exit` runs the `END` block where `if (!closed) exit 1`. Under `set -euo
  pipefail` the assignment at `:114` inherits that status and the script aborts.
  Verified: `bash work-item-normalise.sh` over a body-only file produces no output
  and exit 1. The same holds for an unclosed frontmatter and for an empty file.

- **`document`'s fence recogniser is stricter than the awk it would stand in
  for.** `cli/document/src/fence.rs:41` is `if &raw[..first_line_end] != b"---"`
  and `:70` is `== b"---"` — exact bytes modulo a trailing `\r` — whereas both
  awk functions match `/^---[[:space:]]*$/`. A work item whose fence carries a
  trailing space is frontmatter-plus-body to bash and all-body to `document`.
  `document` also stops scanning at `MAX_SCAN` (1 MiB, `:11`) and returns
  `Unterminated`; awk has no cap. So `document` cannot be the digest path's
  splitter — see Implementation Approach.
- **`work-adapters` depends on neither `corpus` nor `store`.** Baseline
  persistence adds a `corpus` edge (the `AtomicWrite` port, not the adapter).
- **`sha2` is already a workspace dependency** (`cli/Cargo.toml:63`, used by
  `launcher`), so hashing needs no new third-party crate.
- **`work` is pinned by cargo-public-api, `work-adapters` is exempt**
  (`_PINNED_CRATES` at `tasks/public_api.py:15-26`, `_EXEMPT_MEMBERS` at `:38-43`,
  where `vcs-test-support` sits at `:62-65`), so every public
  classify/decide/plan type churns `cli/work/tests/fixtures/public-api.txt` and
  adapter-side types churn nothing. Widening `work`'s import rule also puts
  `tracker`'s types into that pinned surface, coupling the two pins: `tracker`'s
  next additive change reddens `work`'s snapshot with no first-party edit in
  `work`. Accepted, and named in Phase 2's manual verification so the diff reads
  as the pin working rather than as noise.
- **`accelerator-work` hand-rolls its exit codes** (`main.rs:57-77`, `:159-167`)
  rather than using the shared `report(&kernel::Error)` helper. 0, 1, 2 and 3
  are taken; 4 and 5 are free.
- **`parse_key_value` errors exit 2, not 1.** `cli/work-cli/src/cli.rs:77-82`
  returns `Err(String)` to clap, which surfaces it as a usage error — the same
  code `conflicts_with` produces. `--set`/`--append`/`--remove` already behave
  this way, so `--resolve` must too.

## Desired End State

`accelerator work sync [--push-only|--pull-only] [--preview] [--per-item-reads]
[--max-pulls <n>] [--resolve <id>=<remote|local|skip>]…` and `accelerator work
create|update --push` exist, compile against the frozen port, resolve a tracker
from `work.integration`, and are exercised end to end against a fake. Every bash
script named in the work item is still present, still invoked by its existing
callers, and its suite still passes. `mise run` exits 0.

Verify with `mise run` (the bare default task), plus `mise run
test:integration:tracker-contract` for the filtered suite and `mise run
test:integration:pup` for the widened rule.

## What We're NOT Doing

- Not removing any bash script, and not repointing any SKILL, script or
  user-facing entry point at the binary. `_EXPECTED_WORK_SUITES` stays at 5.
- Not writing real Jira or Linear clients. The provider-selection seam is built
  and exercised against fakes; the registry ships empty, so `accelerator work
  sync` in production fails with a named error until 0171 fills it.
- Not wiring `work_adapters::project_remote` to the port. Each provider client
  owns reproducing its own recipe (0171); this story consumes the
  already-projected `RemoteIssue`.
- Not adding a detector for the default-bodied-trait-method gap 0204 handed
  over. See Implementation Approach.
- Not building the conversational conflict flow. A test harness closes the
  two-invocation loop instead.
- Not adding item selection to `sync` (`--only <id>` or positional ids). Every run
  classifies the whole corpus, so investigating one item costs a full pass and a
  `fetch_all`. Worth doing, but it is new surface rather than parity, and the work
  item's command specification does not carry it — recorded here so the absence is a
  decision rather than an oversight.
- Not asking the pull-overwrite question. This story ships the fail-closed refusal
  (exit 5, zero writes); the interactive half the SKILL owns today follows the rest of
  the conversational flow into 0171.
- Not lifting the fixture tables behind `test-work-item-sync-apply.sh`,
  `test-work-item-fetch-remote.sh`, `test-work-item-create-remote.sh` or
  `test-work-item-update-remote.sh`, which AC 16 names alongside the sections this story
  does lift. Those four exercise the bash **bridge** scripts — the shell-out plumbing to
  a live provider — whose Rust counterparts are 0171's client adapters, not this story's
  code; their behavioural content lands in 0171 as the contract harness run against each
  real client. The baseline section is likewise replaced by typed unit tests plus the
  generated corpus rather than a lifted table, because its cases assert file-level
  atomicity that a table cannot express. Recorded here because AC 16 read literally
  covers all of them, so this is a criterion the work item should narrow rather than an
  omission to discover at validation.
- Not providing a way to push an **existing** unsynced item. `unsynced` collapses to
  `noop` in all three directions (`work-item-sync-decide.sh:112-115`), `update --push`
  refuses a target with no `external_id`, and `create --push` only pushes what it is
  creating — so an item that exists locally and was never pushed has no route through
  the binary. The bash SKILL covers this with Step 4's unsynced-push offer and its
  untracked-remote pull; both stay bash-only here. Recorded because it is a real hole
  in the surface rather than a deferred nicety, and 0171 needs to decide who owns it —
  relaxing `update --push` to create when `external_id` is absent is the obvious
  candidate.

## Implementation Approach

Five phases, each independently mergeable and each green under `mise run` on its
own. They map onto the work item's own A/B/C: phases 1–3 are A, phase 4 is B,
phase 5 is C. Phase A is split because it spans pup config, a new crate, two
domain modules, two adapter modules and three fixtures — three smaller slices
review better and land sooner.

The Rust signatures in this plan were **compile-checked** against a scratch crate at
the workspace's own edition (2021) and MSRV (1.90.0), exercising `run`'s real sequence:
load the baseline, plan, apply per item through a reborrowed `ItemApplier`, blank
unreconciled hashes, finalise, then read the baseline again. That is what caught `run`'s
lifetime coupling. Sketches here are what compiled, not what reads well.

Test-driven throughout: every behavioural unit gets a failing test first. The
lifted bash tables are the oracle, so for the ported units the red step is
writing the table and watching it fail against an unimplemented function. Each
phase's Changes required is ordered accordingly — the table, golden or test comes
before the module it drives. Every lifted expectation is derived from **executing
bash or reading a bash assertion**, never from reading the new Rust; a row whose
expectation was taken from the implementation characterises nothing and passes
forever, which no row-count assertion can detect.

Twelve decisions, taken here so implementation meets none of them mid-flight.

**The `work` → `tracker` edge widens the pup rule.** `work_domain_imports_only_permitted`
gains `^tracker(::|$)`. The precedent is exact — `work` was already widened once
for `^corpus(::|$)`, and `migrate` carries two allowances. `tracker` is a
zero-dependency, zero-logic port crate, so the edge cannot drag a transitive
graph into the domain, which is the property the rule protects. The alternative
— translating at the `work-adapters` boundary — needs a domain-shaped timestamp
type that duplicates `RemoteTimestamp` and its `proves_unchanged_since` rule,
the single thing in this story most dangerous to have two copies of. The cost
this accepts: `tracker` becomes a second blessed domain dependency, and the next
port crate will cite it.

**A baseline's empty `remote_updated_at` reads back as `RemoteTimestamp::NotRead`.**
`NotReported` and `NotRead` both persist as `""`, so the variant does not survive
the round trip. `NotRead` asserts nothing about the remote; `NotReported` would
assert a property of the tracker record that an empty string cannot support, and
would be false whenever the emptiness came from a failed post-push read. `NotRead`
is also already the sync-engine-owned variant — no port operation returns it —
and `last-sync.json` is the sync engine's own store. Classification is unaffected
either way (`proves_unchanged_since` is false for both); only a report differs.

**Provider-selection failures reuse the dispatch taxonomy.** `work.integration`
unset, empty, or naming a tracker outside `{linear, jira, trello,
github-issues}` is `E_DISPATCH_UNRECOGNISED` (73, fail closed); naming a
recognised tracker with no client wired is `E_DISPATCH_NOT_AVAILABLE` (72).
`cli/tracker/tests/fixtures/dispatch-codes.txt` already records both as
resolving "above the port, at the composition root selecting `Box<dyn
RemoteTracker>` from `work.integration`" — this is that composition root, so the
codes land where the fixture already says they do.

**`sync` is plan-then-apply, and planning is pure.** The run computes a complete
`SyncPlan` (one `PlannedAction` per item) from pre-fetched remote state, then
executes it — `--preview` runs the plan stage and stops. Fetching stays *outside*
planning: a thin `work_adapters::sync::fetch` shell performs `fetch_all`/`show`
and hands the gathered facts to a pure `work::sync::plan`, so the two-tier read
rule, the `fetch_all`-error rule and resolution application are unit-testable
without a tracker fake and sit inside the crate the pup rule and the public-api
pin both cover. This is the siting decision the work item's boundary criterion
left open for the orchestration layer, taken the same way it took classify and
decide. The cost accepted: every planning type churns `work`'s snapshot.

Preview still issues `fetch_all`/`show`, which AC 8 permits (it forbids only
`create` and `update`) — and `--preview`'s help says so, because "preview"
otherwise reads as "offline". AC 9's plan-fidelity is **not** structural: preview
and the real run are separate invocations, each re-deriving from freshly stat'ed
mtimes, a freshly read baseline and a fresh `fetch_all`. Sharing one derivation is
a design property that makes divergence unlikely; the AC is discharged by
asserting the two invocations' stdout as a set comparison, excluding `failed`
lines.

**The contract suite is excluded by nextest filter, not by crate.** A
profile-level `default-filter = 'not binary(contract)'` in a new
`cli/.config/nextest.toml` keeps every `tests/contract.rs` binary out of the
default run, and a `[profile.contract]` selecting `binary(contract)` is what the
dedicated task runs. Verified against the pinned 0.9.138: the key parses.

Three reasons this beats `--exclude tracker-test-support`. It composes to 0171 —
whose real clients live in adapter crates whose *unit* tests must keep running, so
a crate exclusion could not gate their contract binaries without silencing
everything else in those crates. It leaves `tracker-test-support`'s own lib tests
running in the default pass, which a crate exclusion would silence — the one crate
whose correctness every downstream assertion depends on. And it covers the
residual a crate exclusion concedes: a hand-run `cargo nextest run` at the `cli/`
root honours the config too.

Belt and braces, because a filter is still one line of config standing between a
plain test run and live remote mutations from 0171 onward: `contract::run_all`
refuses unless `ACCELERATOR_TRACKER_CONTRACT=1` is set, so the harness fails
closed as well as being filtered out.

**The run-start epoch gets a sync-owned port, not a widened `corpus::Clock`.**
`corpus::Clock` has no epoch operation, and adding one would touch a port
`corpus-adapters`, `corpus-cli` and `migrate-adapters` all implement, churn
`corpus`'s pinned snapshot, and break every existing fake. `work::sync::RunClock`
(`fn run_start_epoch(&self) -> Result<u64, kernel::Error>`) is declared beside the
code that needs it and implemented over `SystemTime` in `work-adapters`. A
derivation failure yields **no advance** — the baseline timestamp is left
untouched, forcing a full re-hash — rather than any fallback value, because the
persisted timestamp is the sole gate on the hash-free local short-circuit and a
too-large one silently disables local change detection corpus-wide.

**A run bounds how many writes it may make, in both directions.** The bash SKILL's
aggregate pull-overwrite gate (25 files, fail-closed when non-interactive) survives
as a non-interactive refusal, and gains a mirror: if the plan's `Pull` count exceeds
`--max-pulls` or its `Push` count exceeds `--max-pushes` (both default 25), the run
reports the counts and exits 5 having written nothing. This is not the confirm-prompt
UX the project rejects — it is the same quantitative limit, expressed as an order
rather than a question.

⚠️ Bounding only pulls would leave the more dangerous direction open. The plan's own
named mis-classification causes work both ways: recipe drift makes every `local_hash`
stale, which classifies `locally-modified`, which decides `Push`, which is a
whole-content `tracker.update` replacing every remote issue's title and body — and
`--push-only` would be entirely unbounded. A bad pull destroys uncommitted local
edits, which VCS revert cannot recover; a bad push destroys remote content that
exists nowhere locally at all, which neither revert nor a re-run recovers.

The refusal is evaluated in **both** run modes. A preview that reported 40 pulls and
exited 0, followed by a real run that wrote nothing and exited 5, would break
preview's whole purpose and violate AC 9's fidelity property for exactly the plan
with the largest blast radius. Preview writes nothing anyway, so the refusal costs it
nothing and tells the operator what they need before they commit.

The message states the count, the current limit, `--max-pulls <n>` / `--max-pushes
<n>` to raise it, and `--preview` to inspect the plan first — the same what/why/fix
shape as the selection errors, since this is the one case where the command flatly
declines an order. `0` means refuse every write in that direction; there is no
unlimited sentinel, and raising the bound is an explicit number.

**Local dirtiness is a three-valued input with a fail-safe unknown.** `decide`
takes `Dirtiness::{Clean, Dirty, Unknown}`, not `Option<bool>`: absence and
cleanliness must not be the same value, because `file_dirty`'s existing
`VcsMode::Indeterminate → dirty` mapping exists precisely so a failed probe does
not authorise an overwrite. `Unknown` is treated as dirty everywhere. A
`WorkingCopyStatus` port injected into the fetch shell supplies it — probed once
per run for the jj whole-tree shape, per path for git — and `work::file_dirty`
does the interpreting, unchanged.

**Token and id handling is ASCII-only.** `resolve_conflict_token` uses
`to_ascii_lowercase()` and `trim_matches(char::is_ascii_whitespace)`, matching
bash's `tr '[:upper:]' '[:lower:]'` plus `[[:space:]]` sed in the C locale.
Rust's Unicode-aware `to_lowercase()`/`trim()` would resolve `"\u{00A0}remote"`
to accept-remote where bash leaves it unrecognised and skips — turning the
deliberately safe default into a local overwrite for whitespace no human can see.
`work/src/normalise.rs:5-10` already documents this precedent for the same
reason. `classify_external_id` strips bash's *combined* `[[:space:]"']` class from
both ends (`work-item-sync-label.sh:45`), not quotes-then-whitespace.

**An empty-string persisted hash reads back as absent.** The `Entry` →
`BaselineEntry` conversion maps `""` to `None`, mirroring bash's `// empty` plus
`[ -n ]`. Getting this wrong changes which branch runs: an empty `local_hash` read
as present would enter the mtime pre-filter and short-circuit to *unchanged* where
bash counts it changed, losing first-sync semantics, and an empty `remote_hash`
read as present would force a remote body hash bash never computes, breaking the
digest-call assertions the lifted table rests on.

**The report names every classified item, at fixed arity.** One tab-separated
line per item — `<id>\t<action>\t<state>` — for **every** classified item, not
only those whose decision is not `Noop`. Emitting only actioned items would make a
total `fetch_all` failure (which marks every id `Indeterminate`, which decides
`Noop`) produce empty stdout and exit 0: byte-identical to a corpus already in
sync, on a run that read nothing. The bash SKILL reports those items under
`needs-retry` and `remote-absent` and the port contract must not lose that. Fixed
arity means a consumer splits and reads field 3 unconditionally, matching
`migrate-cli --list`'s shape rather than branching on whether a third field exists.

**Failures carry their retryable/terminal class.** A per-item apply failure emits
`<id>\tfailed\t<state>\t<retryable|terminal>`. Collapsing it to `failed` with the
detail on unstructured stderr would discard the one distinction the whole 70/71
taxonomy exists for — `EXIT_CODES.md` says 71 is *not* safe to auto-retry — at the
exact point a caller decides whether to re-invoke. The `TrackerError.detail`
string still goes to stderr; the class goes on the wire.

**A cleared baseline entry classifies `conflict`, and that is the contract.**
`update --push`'s terminal path clears the entry, leaving both hashes absent so
both sides default to changed. `Indeterminate` is reachable **only** from
`RemotePresence::Indeterminate` — a failed read — so no amount of local
bookkeeping can produce it. Under a bidirectional run `conflict` writes neither
side, which is the safety property that matters, and it needs no third on-disk
state store. Residual, stated rather than hidden: `conflict` is *resolvable*, so a
later `--resolve <id>=remote` will pull the remote over the local for an update
that may never have applied. The alternative — persisting an uncertainty flag the
classifier honours — was rejected because it widens the baseline schema the live
bash engine also reads, during the window both engines are live.

**The work item needs amending to match.** AC 14 and the Requirements bullet at
`0194-…md:236-246` both state that the cleared entry makes the next sync classify the
item `indeterminate`. That is unreachable, for the reason above. A validator checking
AC 14 as written would mark a correct implementation failed, so the criterion should be
corrected to `conflict` rather than leaving the divergence to be re-litigated at
acceptance.

**The default-bodied-trait-method gap is accepted, not detected.** 0204 handed
over that a `RemoteTracker` implementor which quietly stops implementing an
operation once the trait gains a default-bodied method is invisible to both of
0204's guards. Rust offers no compile-time assertion that a trait method is
required, and the practical alternatives (a test calling all four through a
trait object) provably do not catch it — `tracker/tests/port.rs:144-167` already
does exactly that. The mitigation is the contract harness's per-operation
behavioural assertions: an implementor inheriting a no-op or panicking default
fails them. Recorded in `tracker-test-support`'s module docs so the next reader
does not re-derive it.

---

## Phase 1: Boundaries and shared test apparatus

### Overview

Everything the sync code needs to exist before it can be written: the widened
crate boundary, the shared fake and contract harness, the test-exclusion
mechanism, the Rust owner of the dispatch taxonomy, and the two characterization
gaps that must close against untouched bash. No sync logic.

### Changes Required

#### 1. Widen the `work` domain import rule

**File**: `cli/pup.ron`
**Changes**: add `tracker` to `work_domain_imports_only_permitted`'s permit
list, and extend the rule's own comment to name the second allowance.

```ron
Module((
    name: "work_domain_imports_only_permitted",
    matches: Module("^work($|::)"),
    rules: [
        RestrictImports(
            allowed_only: Some([
                "^(std|core|alloc)(::|$)",
                "^kernel::Error(::|$)",
                "^corpus(::|$)",
                "^tracker(::|$)",
                "^crate(::|$)",
            ]),
            denied: None,
            severity: Error,
        ),
    ],
)),
```

**File**: `tests/integration/pup/test_import_rule.py`
**Changes**: both edits are mandatory — the extras tuple drives the compliant
control, and `_ALL_EXTRA_CRATES` drives the auto-generated cross-rejection cases
proving `corpus`, `vcs` and `migrate` still reject `tracker`.

```python
_DOMAIN_RULES = [
    ("corpus", "corpus_domain_imports_only_permitted", ()),
    ("vcs", "vcs_domain_imports_only_permitted", ()),
    ("work", "work_domain_imports_only_permitted", ("corpus", "tracker")),
    (
        "migrate",
        "migrate_domain_imports_only_permitted",
        ("corpus", "document"),
    ),
]

_ALL_EXTRA_CRATES = ("corpus", "document", "tracker")
```

Red first: run `mise run test:integration:pup` after the probe-table edit and
before the `pup.ron` edit, and watch `work`'s compliant control fail on a
`tracker` import the shipped rule does not admit.

#### 2. The shared fake and contract harness

**File**: `cli/tracker-test-support/Cargo.toml` (new)
**Changes**: a library crate depending only on `tracker`. No dev-dependency edge
back to any consumer, for the reason `cli/vcs-test-support/Cargo.toml:12-23`
records.

```toml
[package]
name = "tracker-test-support"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
tracker = { path = "../tracker" }
```

**File**: `cli/tracker-test-support/src/lib.rs` (new)
**Changes**: the recording fake. Interior mutability through `RefCell` so it
satisfies the port's `&self` signatures while counting calls — the counts are
what ACs 7, 11 and 18 assert on.

```rust
pub mod contract;

use std::cell::RefCell;

use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Create { title: String, body: String, kind: String },
    Update { id: ExternalId, title: String, body: String },
    Show { id: ExternalId },
    FetchAll { ids: Vec<ExternalId> },
}

#[derive(Default)]
pub struct RecordingTracker {
    issues: RefCell<Vec<(ExternalId, RemoteIssue)>>,
    unprovable: Vec<ExternalId>,
    absent: Vec<ExternalId>,
    failures: Vec<(ExternalId, TrackerError)>,
    next_id: RefCell<u32>,
    calls: RefCell<Vec<Call>>,
}
```

Recording the pushed title and body on `Call::Create`/`Call::Update` is what lets
a test assert the whole-content contract rather than only the call count.

Builders mirroring `tracker/tests/port.rs:20-50`'s three named constructors —
`holding`, `truncating`, `losing` — plus `calls()` returning a snapshot and four
failure seams phase 5 needs and cannot express without them:

- `failing_update(id, error)` — the retryable/terminal split on `update --push`.
- `failing_create(error)` — the same split on `create --push`.
- `creating_then_failing(error)` — records the created issue **and** returns the
  error. This is AC 13's state: a terminal failure where the remote issue was in
  fact created. Without it that criterion is unbuildable.
- `failing_show(id, error)` — the failed post-push read that writes
  `RemoteTimestamp::NotRead`, the one variant no port operation returns.

**File**: `cli/tracker-test-support/src/contract.rs` (new)
**Changes**: the harness, parameterised over implementations. `partitions_totally`
lifts verbatim from `tracker/tests/port.rs:238-262` — it is dependency-free.

```rust
pub trait ContractSubject {
    fn tracker(&self) -> &dyn RemoteTracker;

    /// An id this implementation will never account for in a `fetch_all`.
    fn unaccountable_id(&self) -> ExternalId;

    /// An id whose `show` this implementation will fail.
    fn unreadable_id(&self) -> ExternalId;
}

pub fn partitions_totally(outcome: &FetchOutcome, requested: &[ExternalId]);

pub fn create_then_show_round_trips(subject: &dyn ContractSubject);
pub fn update_replaces_whole_content(subject: &dyn ContractSubject);
pub fn fetch_all_partitions_totally(
    subject: &dyn ContractSubject,
    ids: &[ExternalId],
);
pub fn unaccounted_id_is_indeterminate_not_absent(subject: &dyn ContractSubject);
pub fn a_failing_read_is_retryable(subject: &dyn ContractSubject);

pub fn run_all(subject: &dyn ContractSubject, ids: &[ExternalId]);
```

The last two are the obligations the port documents but cannot enforce, and which
no test in `tracker` can hold. `unaccounted_id_is_indeterminate_not_absent`
asserts the unseen id lands in `indeterminate`, never `absent`;
`a_failing_read_is_retryable` asserts `show`'s error is never `Terminal`.

Both properties are conditions the *implementation* must be induced into, not
behaviours it exhibits unprompted — which is why the harness takes a
`ContractSubject` rather than a bare `&dyn RemoteTracker`. Truncation and a lost
write are configurations of the fake (`truncating`, `losing`); a real Jira or
Linear client cannot be asked to truncate a bulk fetch, so with a bare trait
object these two assertions would be unrunnable against the very implementations
they exist to hold — and 0171 would inherit a harness that cannot discharge its
own criterion. `unaccountable_id` and `unreadable_id` are how a real client
supplies the same conditions: a definitely-nonexistent id, and one whose read it
can make fail.

`run_all` is the **conformance** set — the properties every implementation must
satisfy. It therefore excludes the shape-specific cases: asserting
`create_then_show_round_trips` against a deliberately `losing` tracker would
require it to fail, which is not a conformance property.

`run_all` also refuses unless `ACCELERATOR_TRACKER_CONTRACT=1` is set. The nextest
filter keeps it out of the default run; this makes it fail closed too, so that from
0171 onward a hand-run invocation or an IDE test-runner button cannot mutate a live
tracker workspace on the strength of one line of config.

`run_all` returns the **count of properties it executed**, and
`tests/contract.rs` asserts that count is non-zero. A gate that merely skips fails
*open with respect to signal*: drop or misspell the env var in the task, or run the
profile by hand, and every contract binary exits 0 having asserted nothing — which
from 0171 onward is the sole mechanism holding real clients to `FetchOutcome`
totality and the retryable-read obligation. A silently vacuous pass is worse than a
red one. The manual check confirms the skip path; the count assertion confirms the
run path.

**File**: `cli/tracker-test-support/src/lib.rs` — a `ContractSubject` impl for
`RecordingTracker`, returning an id it was never seeded with as `unaccountable_id`
and its `failing_show` id as `unreadable_id`.

**File**: `cli/tracker-test-support/src/lib.rs` — `#[cfg(test)]` tests for the fake
itself: `truncating` omits the ids it claims to, `losing` drops the write, `calls()`
records each operation once and in order, and each failure seam returns the class it was
built with. These run in the default pass (the filter excludes binaries named
`contract`, not the crate), and they are load-bearing rather than tidy: the argument for
filtering by binary name over excluding the crate is precisely that this crate's own
tests keep running, and Phase 1's manual check asserts they do — both vacuous if none
exist.

**File**: `cli/tracker-test-support/tests/contract.rs` (new)
**Changes**: runs `contract::run_all` against `RecordingTracker` in its
conformance shape, then the two shape-specific cases against `truncating` and
`failing_show` subjects respectively.

**File**: `cli/tracker-test-support/src/contract.rs` — module docs recording the
inherited default-bodied-trait-method gap and why the per-operation behavioural
assertions are its only mitigation, so the next reader does not re-derive it.

#### 3. Register the crate

Per `tasks/README.md#registering-a-library-crate`, five obligations:

- `cli/Cargo.toml` — add `"tracker-test-support"` to `[workspace].members`,
  immediately after `"tracker"` so it sits with its family the way
  `vcs-test-support` does, adding the trailing comma `"tracker"` currently lacks.
  Then sync the lockfile with `cargo metadata --manifest-path cli/Cargo.toml
  --format-version 1`. Clippy runs `--locked`.
- Inherited manifest fields and `[lints] workspace = true` — above.
- **File**: `cli/pup.ron` — a rule permitting `std`, `tracker` and `crate`:

  ```ron
  Module((
      name: "tracker_test_support_imports_only_permitted",
      matches: Module("^tracker_test_support($|::)"),
      rules: [
          RestrictImports(
              allowed_only: Some([
                  "^(std|core|alloc)(::|$)",
                  "^tracker(::|$)",
                  "^crate(::|$)",
              ]),
              denied: None,
              severity: Error,
          ),
      ],
  )),
  ```

- **File**: `tests/integration/pup/test_import_rule.py` — a probe pair: a
  violation case importing something outside the permit list, and a compliant
  control that **imports `tracker`**. The control must actually import something
  permitted: a rule whose matcher silently resolves nothing still passes an
  import-free control, which is the failure mode the checklist warns about.

  The hyphen-to-underscore matcher is **established convention, not new ground**:
  `work_adapters::filesystem` (`cli/pup.ron:133`), `vcs_adapters::library` (`:160`)
  and `migrate_adapters` (`:217`) all match hyphenated packages by their underscore
  module names, and all three are enforced today. `^tracker_test_support($|::)` is
  the same mechanism with one more hyphen.

  Adding `"tracker"` to `_ALL_EXTRA_CRATES` also materialises a synthetic
  sibling crate named `tracker` inside **every** domain probe workspace, where the
  shipped `tracker_domain_imports_only_permitted` rule (`^tracker($|::)`) now
  matches it too. Benign — the generated lib has no imports — but check it rather
  than discover it.
- **File**: `tasks/public_api.py` — add to `_EXEMPT_MEMBERS`. The coverage guard
  in `tests/unit/tasks/test_rust.py` fails until it is classified.
  `vcs-test-support` is already exempt (`tasks/public_api.py:62-65`), so lift its
  reason string into a shared constant rather than writing a second one, following
  the file's own `_ADAPTER`/`_COMPOSITION_ROOT` idiom:

  ```python
  _TEST_SUPPORT = (
      "test support: consumed only by other crates' test targets, where a"
      " widened surface is caught by the tests that use it failing to compile"
  )

  _EXEMPT_MEMBERS = {
      ...
      "tracker-test-support": _TEST_SUPPORT,
      "vcs-test-support": _TEST_SUPPORT,
  }
  ```

Then `mise run deny:check`.

#### 4. Filter the contract suite out of the default run

**File**: `cli/.config/nextest.toml` (new)
**Changes**: the filter, and the profile that selects what it excludes.

```toml
[profile.default]
default-filter = 'not binary(=contract)'

[profile.contract]
default-filter = 'binary(=contract)'
```

⚠️ `binary(=contract)` is the **exact-match** form. Bare `binary(contract)` is a
substring predicate in nextest's filterset DSL, which errs safe in the default
profile but would silently pull a future `contract_helpers` or `contract_smoke`
binary into the contract profile — and the pinned assertion below would still pass.

Filtering by **binary name** rather than by crate is what makes this inheritable:
every crate's `tests/contract.rs` is excluded by the same line, so 0171's client
crates get the behaviour for free while their unit tests keep running. It also
leaves `tracker-test-support`'s own lib tests in the default pass — a crate
exclusion would have silenced the one crate every downstream assertion depends on.

**File**: `tasks/test/integration.py`
**Changes**: a task selecting the contract profile.

```python
@task
def tracker_contract(context: Context) -> None:
    """Run the RemoteTracker contract harness (filtered out of test:unit:cli)."""
    context.run(
        f"cargo nextest run --profile contract {_MANIFEST}",
        env={"ACCELERATOR_TRACKER_CONTRACT": "1"},
        pty=True,
    )
```

It reuses `tasks/test/cli.py`'s `_MANIFEST` rather than re-spelling the
invocation, which is what carries `--exclude accelerator-visualiser`. Without that
exclusion, `--all-features` sets `CARGO_FEATURE_EMBED_DIST` and
`cli/visualiser/server/build.rs` asserts `cli/visualiser/frontend/dist/index.html`
exists — the profile filter suppresses the visualiser's *tests*, not its *build*. The
task would fail on any checkout without a built frontend, taking the
`test:integration` roll-up and the `mise run` done-gate with it, with a build-script
panic that reads as unrelated to tracker work. Re-spelling it would also let the two
invocations drift on member selection and feature flags.

`tasks/test/cli.py` is otherwise unchanged — no `--exclude tracker-test-support`, and
`_MANIFEST` keeps its existing shape.

**File**: `mise.toml`
**Changes**: a `test:integration:tracker-contract` task carrying the same
`depends` edges every workspace-wide Rust task needs — `deps:install:python` (it runs
through `invoke`), `deps:install:rust-components` and `build:frontend:stub` — added to
the `test:integration` roll-up's `depends` list. It drives a fake and makes no network
call, so it belongs in the roll-up — unlike `pup` and `zero-spawn`, which have their
own CI jobs.

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: ⚠️ this is the file that guards mise task registration, **not**
`tests/unit/tasks/test_integration.py`, which holds only the shell-suite count
floors. Two assertions there fail on any unregistered `test:integration:*` task:
`test_every_integration_task_declares_its_launcher_need` (`:146`) requires an
entry in `_LAUNCHER_DEPENDENTS` or `_NO_LAUNCHER_NEEDED`, and
`test_every_integration_task_is_in_the_rollup_or_excluded_with_a_reason` (`:180`)
requires roll-up membership or a `_NOT_IN_INTEGRATION_ROLLUP` reason. Add to
`_NO_LAUNCHER_NEEDED` (`:57`):

```python
"test:integration:tracker-contract": "cargo nextest over a fake tracker",
```

Roll-up membership satisfies the second guard.

**File**: `tests/unit/tasks/test_nextest_filter.py` (new)
**Changes**: assert the filter is still in place — that `cli/.config/nextest.toml`'s
default profile carries `not binary(=contract)` in the exact-match spelling, and that
`tasks/test/cli.py`'s command passes neither `--profile` nor
`--ignore-default-filter`, either of which would bypass it.

Its own file rather than `test_rust.py`, following the precedent
`tests/unit/tasks/test_registration_docs.py` states in its own docstring: guards over
non-Python artefacts live in dedicated files beside `test_mise.py` and
`test_workflows.py`. `test_rust.py`'s scope is the `tasks/shared/rust.py` helpers and
the task leaves, so a guard over a Rust-tooling config file is invisible there to both
the config's maintainer and the task's.

The property AC 20 rests on is otherwise guarded by nothing but a manual eyeball, and
from 0171 onward its silent removal means live API calls in the default test run.

**File**: `tasks/README.md`
**Changes**: a short subsection beside the existing zero-spawn and pup enforcement
notes, recording the convention this phase establishes repo-wide: any crate's
`tests/contract.rs` is excluded from the default run by
`cli/.config/nextest.toml`, the `contract` profile is how to run it, and
`ACCELERATOR_TRACKER_CONTRACT` gates it. Cross-referenced from the library-crate
checklist.

Without this, the next author to name a test binary `contract.rs` in some other
crate gets it silently dropped from `mise run` with no signal, and the location of a
config file that gates the default test run is undiscoverable from the documentation
`CLAUDE.md` points developers at. `cli/.config/` is also the first `.config/`
directory in the tree — every other `cli/` tool config is flat (`rustfmt.toml`,
`clippy.toml`, `deny.toml`, `pup.ron`) — so its existence needs recording somewhere
shared rather than only in a module doc comment.

#### 5. The Rust owner of the exit-code taxonomy

**File**: `cli/work-cli/src/exit_codes.rs` (new)
**Changes**: one definition for the whole binary, at the composition root where
every code becomes an exit status. Naming the low codes here too — rather than
only 70-73 — is what stops 4 and 5 joining the hand-rolled literals already
scattered through `main.rs:57-77`.

```rust
pub const CLEAN: u8 = 0;
pub const ERROR: u8 = 1;
pub const USAGE: u8 = 2;
pub const RESOLVE_NOT_FOUND: u8 = 3;
pub const UNRESOLVED: u8 = 4;
pub const REFUSED_BULK_OVERWRITE: u8 = 5;

pub const RETRYABLE: u8 = 70;
pub const TERMINAL: u8 = 71;
pub const NOT_AVAILABLE: u8 = 72;
pub const UNRECOGNISED: u8 = 73;

#[must_use]
pub const fn for_tracker_error(error: &TrackerError) -> u8 {
    match error {
        TrackerError::Retryable { .. } => RETRYABLE,
        TrackerError::Terminal { .. } => TERMINAL,
    }
}
```

**File**: `cli/work-cli/tests/exit_codes_parity.rs` (new)
**Changes**: parses `skills/work/scripts/work-item-bridge-codes.sh` for its four
`readonly E_DISPATCH_*=N` lines and asserts each against the Rust constant,
including a count assertion so a fifth code added to bash fails here.

Reading bash from `work-cli` rather than reusing
`cli/tracker/tests/fixtures/dispatch-codes.txt` keeps that fixture as the port's
own membership pin and avoids a test crossing a crate boundary for fixture data.

#### 6. Close the `push-decide` characterization gap

**File**: `skills/work/scripts/test-work-item-create-remote.sh`
**Changes**: extend the existing "Decision seam" section at `:267-289` with the
two uncovered arms. Extending rather than adding a suite keeps
`_EXPECTED_WORK_SUITES` at 5 and `tests/unit/tasks/test_integration.py`
untouched.

```bash
assert_exit_code "non-integer --code → usage error (2)" 2 \
  bash "$DECIDE" --code abc --attempt 1
assert_exit_code "non-integer --attempt → usage error (2)" 2 \
  bash "$DECIDE" --code 0 --attempt x
assert_exit_code "unrecognised flag → usage (2)" 2 \
  bash "$DECIDE" --bogus
assert_eq "unknown dispatcher code → loud-terminal (conservative)" \
  "loud-terminal" "$(bash "$DECIDE" --code 99 --attempt 1)"
assert_eq "--write-failed is consulted only under code 0 (retryable)" \
  "retry" "$(bash "$DECIDE" --code 70 --attempt 1 --write-failed)"
assert_eq "--write-failed is consulted only under code 0 (terminal)" \
  "loud-terminal" "$(bash "$DECIDE" --code 71 --attempt 1 --write-failed)"
```

The last two rows are the ones that matter beyond coverage arithmetic. Bash
consults `write_failed` only *inside* the `code == 0` branch, so a Rust port that
tested `write_failed` first would return `loud-terminal` for
`--code 70 --attempt 1 --write-failed` and still pass every previously-covered
row — a plausible reordering, invisible without these two.

This runs against the unmodified script and must pass before phase 5 touches
anything it protects.

#### 7. Close the normalise parity gap

**File**: `cli/work/tests/normalise_parity.rs` (new)
**Changes**: reads each `skills/work/scripts/test-fixtures/work-item-normalise/
case-*/` directory and asserts `work::normalise` reproduces `expected`. Follows
the `cli/work-adapters/tests/diff_shellout_parity.rs:14-24` root-resolution
idiom. Not feature-gated — it reads committed files and shells nothing.

```rust
fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}
```

Cases: `case-file-mode` through `filter_frontmatter_keys`, `case-stdin-mode` and
`case-nbsp-not-trimmed` through `trim_lines`. Assert the case count so a fixture
added to the golden index without a Rust arm fails here.

### Success Criteria

#### Automated Verification

- [ ] Widened rule and its probes pass: `mise run test:integration:pup`
- [ ] Rust workspace builds, formats and lints: `mise run cli:check`
- [ ] Default test run passes and runs no `contract` binary: `mise run test:unit:cli`
- [ ] Contract harness passes against the fake: `mise run test:integration:tracker-contract`
- [ ] The bash suites still pass unchanged: `mise run test:integration:work`
- [ ] Shell lint and format pass: `mise run scripts:check`
- [ ] Public-api coverage guard and snapshots pass: `mise run public-api:check`
- [ ] Dependency policy passes: `mise run deny:check`
- [ ] Build-system tests pass, including both mise-registration guards and the
      new filter assertion: `mise run test:unit:tasks`
- [ ] Whole tree green: `mise run`

#### Manual Verification

- [ ] `mise run test:unit:cli` runs `tracker-test-support`'s lib tests but no
      `contract` binary
- [ ] `mise run test:integration:tracker-contract` with
      `ACCELERATOR_TRACKER_CONTRACT` unset skips rather than running
- [ ] The compliant control fails when `tracker` is removed from the permit list —
      proving the matcher resolves the hyphenated package rather than silently
      matching nothing
- [ ] `_EXPECTED_WORK_SUITES` is still 5 and no suite file was added

---

## Phase 2: The pure state machine in `work`

### Overview

Classify, decide and label as pure functions over hashes and enums, asserted
against the bash fixture tables lifted into committed Rust test data. No
filesystem, no subprocess, no tracker calls.

### Changes Required

#### 1. The classified state

**File**: `cli/work/src/sync/mod.rs` (new), `cli/work/src/sync/state.rs` (new)
**Changes**: the seven-keyword vocabulary, with the `Display`/`parse` pair the
label table and the report contract both need.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Synced,
    Unsynced,
    LocallyModified,
    RemotelyModified,
    Conflict,
    RemoteAbsent,
    Indeterminate,
}
```

`Display` renders the bash keyword exactly (`locally-modified`, not
`LocallyModified`); `from_keyword` is its inverse, returning `Option`.

#### 2. The classifier

**File**: `cli/work/src/sync/classify.rs` (new)
**Changes**: one function over one input struct, plus the lazy-digest port that
preserves the mtime short-circuit.

```rust
pub trait ItemDigests {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error>;
    fn local(&self) -> Result<String, kernel::Error>;
    fn remote_body(&self) -> Result<Option<String>, kernel::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePresence {
    Present,
    Absent,
    Indeterminate,
}

pub struct BaselineEntry<'a> {
    pub remote_updated_at: &'a RemoteTimestamp,
    pub remote_hash: Option<&'a str>,
    pub local_hash: Option<&'a str>,
}

pub struct Subject<'a> {
    pub external_id: Option<&'a ExternalId>,
    pub presence: RemotePresence,
    pub remote_updated: &'a RemoteTimestamp,
    pub baseline_timestamp: u64,
}

pub fn classify(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    digests: &dyn ItemDigests,
) -> Result<SyncState, kernel::Error>;
```

`ItemDigests` is a port rather than pre-computed values so that a caller pays for a
syscall only when the classifier asks. That preserves two behaviours: the mtime
pre-filter short-circuits **without hashing**
(`work-item-sync-classify.sh:165-167`), and an absent baseline hash skips the
`stat` *and* the hash entirely (`:162`). It also makes the short-circuits directly
testable — the lifted table's fake records whether each was called, which the bash
tests can only assert indirectly.

⚠️ `mtime` belongs **behind** the port, not on `Subject`. A `Subject.mtime` field
would require the caller to have stat'ed the file before `classify` runs, which is
precisely the syscall the `:162` short-circuit skips — the signature would defeat
half the property the port exists to preserve.

Every method is fallible, and `classify` returns `Result`. An infallible port
has nowhere to put a read failure but a panic or a fabricated hash, and a
fabricated `""` silently classifies the item locally-changed — which pushes stale
local content over a good remote, with no log line to diagnose it from. `Ok(None)`
from `remote_body` means "no body was fetched"; a read that failed is `Err`, and
the two must not collapse.

`external_id: Option<&ExternalId>` rather than `Option<&str>`: `tracker`'s vocabulary
is importable now, so the domain speaks the port's language.

⚠️ `ExternalId` does **not** validate. `cli/tracker/src/lib.rs:22-26` is
`pub const fn new(value: String) -> Self` with no checks, and its doc says outright
"The port neither parses nor validates it" — the rejection obligation sits on client
implementations for ids they *return*. So `Some(ExternalId(""))` is representable, and
an earlier revision of this plan wrongly claimed construction rules it out. The
emptiness collapse is therefore an explicit transformation at the one boundary that
reads frontmatter: `fetch::LocalItem` construction maps an `external_id` that is empty
after the `[[:space:]"']` strip `classify_external_id` performs to `None`. Bash's
branch 1 is `[ -z "$external_id" ]` (`work-item-sync-classify.sh:128`), so without the
collapse an item carrying `external_id: ""` — which `create --push`'s failure path
produces — would classify `remote-absent` or `indeterminate` instead of `unsynced`, and
a `show` would be issued for an empty id. Pinned by a classify row and a
`LocalItem`-construction test.

Branch order, reproducing `:127-196` exactly:

1. `external_id` absent → `Unsynced`. Nothing else is consulted. (Bash tests
   absent-or-empty because a shell string cannot express the distinction;
   a shell string cannot express the distinction, and `LocalItem` construction
   collapses an empty or whitespace-only `external_id` to `None` — see below.)
2. `presence` `Indeterminate` → `Indeterminate`; `Absent` → `RemoteAbsent`.
3. Local side, defaulting to changed. Unchanged only if `baseline.local_hash` is
   present **and** either `mtime <= baseline_timestamp` (inclusive; an absent
   mtime is never `<=`, forcing the hash) or `digests.local()` matches it.
4. Remote side, defaulting to changed. Unchanged only if `baseline.remote_hash`
   is present **and** either
   `subject.remote_updated.proves_unchanged_since(baseline.remote_updated_at)`
   or `digests.remote_body()` matches `baseline.remote_hash`.
5. 2×2 verdict.

⚠️ "Present" in steps 3 and 4 means **non-empty**. The `Entry` → `BaselineEntry`
conversion in `work-adapters` maps a persisted `""` to `None`, so the distinction
is already made by the time `classify` sees it — but the lifted table carries rows
for both empty-hash shapes, so a future conversion change reddens here rather than
silently changing which branch runs.

⚠️ Step 4 calls `proves_unchanged_since`, never `==`. `RemoteTimestamp` derives
`PartialEq`, so `==` compiles and reports two identical unknowns as equal —
which classifies an item whose baseline was never written as already synced. The
lifted table carries an `(NotReported, NotReported)` row that fails under `==`.

`mtime: Option<u64>` replaces the bash `9999999999` sentinel
(`work-item-sync-classify.sh:66-80`). The semantics survive — an unreadable
mtime forces the hash path — but the sentinel does not; a Rust port carrying it
would wrongly short-circuit for any baseline timestamp above it.

#### 3. The decision table

**File**: `cli/work/src/sync/decide.rs` (new)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Bidirectional,
    PushOnly,
    PullOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dirtiness {
    Clean,
    Dirty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Push,
    Pull,
    SkipConflict,
    SkipDirty,
    Prompt,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    AcceptRemote,
    PushLocal,
    Skip,
}

#[must_use]
pub fn decide(
    direction: SyncDirection,
    state: SyncState,
    dirty: Dirtiness,
) -> Action;

/// `None` when the token is not one this resolver recognises.
#[must_use]
pub fn resolve_conflict_token(raw: &str) -> Option<Resolution>;
```

The table is **7 states × 3 directions**, with `dirty` sub-splitting
`remotely-modified` only — four states collapse to `noop` regardless
(`work-item-sync-decide.sh:111-148`). The bash script's only test is
`[ "$dirty" = "1" ]`, so `true`/`yes`/`0`/absent all read as clean.

`Dirtiness` rather than `Option<bool>`: absence and cleanliness must not be the
same value. `work::file_dirty` already maps `VcsMode::Indeterminate` to dirty
deliberately, because the recovery model is VCS revert and that cannot recover
uncommitted working-copy changes (`sync-work-items/SKILL.md:132-134`). `Unknown`
therefore decides as `Dirty` everywhere, and a failed status probe cannot authorise
an overwrite by defaulting quietly to clean.

`resolve_conflict_token` folds case then trims **in ASCII only** —
`to_ascii_lowercase()` and `trim_matches(char::is_ascii_whitespace)` — matching
bash's `tr '[:upper:]' '[:lower:]'` plus `[[:space:]]` sed in the C locale.
Interior whitespace survives, so `remote foo` is unrecognised.

Unicode-aware `to_lowercase()`/`trim()` would resolve `"\u{00A0}remote"` to
`AcceptRemote` where bash leaves it unrecognised and skips — converting the
deliberately safe default into a local overwrite for whitespace no human can see.
`work/src/normalise.rs:5-10` documents the same precedent. The lifted table carries
a leading-U+00A0 row that must resolve to unrecognised.

Returning `Option` rather than folding straight to `Skip` keeps the behaviour
(the CLI maps `None` to `Skip`, so the safe default is unchanged) while letting
`work-cli` warn that a token was not understood. `--resolve 0194=remotee`
otherwise produces stdout, stderr and an exit code identical to passing no order
at all, and a caller looping until exit 0 re-invokes forever with the same typo.

`Action` gets its own keyword `Display` and `from_keyword` pair, exactly as
`SyncState` does — `push`, `pull`, `skip-conflict`, `skip-dirty`, `noop`, and
`Prompt` rendering as **`unresolved`**.

That last one is the report's only deliberate divergence from the bash keyword
(`prompt`), and it is the one carrying the exit-code semantics. Owning it here means
one place holds the wire vocabulary; hand-rolling the mapping in `work-cli` would give
one concept three spellings across two live implementations, with nothing failing when
they drift.

#### 4. The label table

**File**: `cli/work/src/sync/label.rs` (new)

```rust
/// The five states that carry a rendered label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderableState {
    Synced,
    Unsynced,
    LocallyModified,
    RemotelyModified,
    Conflict,
}

impl TryFrom<SyncState> for RenderableState { /* … */ }

#[must_use]
pub fn classify_external_id(raw: &str) -> SyncPresence;

#[must_use]
pub fn label(state: RenderableState) -> &'static str;
```

`classify_external_id` strips bash's *combined* `[[:space:]"']` class from both
ends and returns `SyncPresence::{Synced, Unsynced}` — a two-variant type rather
than the seven-variant `SyncState`, since those are the only answers it can give
and a caller should not have to handle five impossible cases.

The strip is one combined character class (`sed "s/^[[:space:]\"']*//;
s/[[:space:]\"']*$//"`, `work-item-sync-label.sh:45`), not quotes-then-whitespace.
A "strip quotes, then trim" port is a *different function* — it leaves
`  'PROJ-1'  ` classified differently — and would pass a golden that only covers
double quotes, bare whitespace and empty. The golden below carries mixed-class rows
for exactly this reason.

`label` is infallible over `RenderableState`, with the fallible narrowing in
`TryFrom`. Reaching for `kernel::Error` to say "this state has no glyph" would make
an error path out of a total function over a five-variant subset, and produce a
message no caller can act on differently — the shape that decays into an
`unwrap()` at the call site. The script's `--label` arm rejects `remote-absent` and
`indeterminate` with exit 1, and `work-cli` reproduces that by mapping the
`TryFrom` failure to exit 1.

Every label is `printf` without `\n` (`:57-61`). The Rust values carry no
trailing newline, and the golden encodes none.

#### 5. Lift the fixture tables

**File**: `skills/work/scripts/test-fixtures/work-item-sync-classify.json` (new)
**File**: `skills/work/scripts/test-fixtures/work-item-sync-decide.golden` (new)
**Changes**: the bash cases transcribed case by case. There are no delimited
tables in `test-work-item-scripts.sh` to port mechanically — every case is an
inline `assert_eq` with literal arguments, bar the 3×4 noop loop at `:432-436`.
Budget for transcription.

These live under `skills/work/scripts/test-fixtures/` and are looped over by
**both** implementations — not under `cli/work/tests/fixtures/`, read only by Rust.
The whole parity strategy is "both implementations held to one oracle while the
bash path stays live"; two independent transcriptions of the same table can drift,
and an edit to a bash assertion would leave the JSON stale and green. The label
golden already does it this way, and these two tables carry the state machine
itself. `jq` is already a suite dependency, so the bash side reads them directly.

Only classify is JSON. Its rows are nested (a stamp variant, a baseline object,
Rust-only annotations), which a pipe row cannot carry. `decide` is scalar-in,
scalar-out — state × direction × dirtiness → action — so it takes the established
`|`-delimited golden shape with a `#` header, exactly as
`work-item-sync-label.golden` and `work-item-file-dirty.golden` do. Two encodings
for one kind of artefact would also mean two loop shapes in the bash suite.

The same applies to the push-decide rows Phase 1 adds: they stay inline bash
assertions there (extending the existing section keeps `_EXPECTED_WORK_SUITES` at
5), and Phase 5's Rust port is asserted against
`skills/work/scripts/test-fixtures/work-item-push-decide.golden`, transcribed from
those assertions in the same pipe-delimited shape.

Classify rows are expressed in terms of **content**, not digests, so both
implementations can realise them. Each row gives the local file body and the remote
body verbatim; the harness computes whichever baseline hashes the row declares
present by running its own side's recipe over that content, and expresses mtime as
an offset from the baseline timestamp rather than an absolute epoch:

```json
{
  "name": "mtime pre-filter short-circuits without hashing",
  "applies_to": ["rust", "bash"],
  "external_id": "ENG-7",
  "presence": "present",
  "remote_updated": {"kind": "reported", "value": "2026-06-01T10:00:00.000+0000"},
  "local_content": "---\nstatus: ready\n---\n\nBody text\n",
  "remote_body": "Body text\n",
  "mtime_offset": -100,
  "baseline": {
    "remote_updated_at": {"kind": "reported", "value": "2026-06-01T10:00:00.000+0000"},
    "remote_hash": "from-content",
    "local_hash": "stale"
  },
  "rust_only": {
    "expect_mtime_called": true,
    "expect_local_digest_called": false
  },
  "expect": "synced"
}
```

`remote_hash`/`local_hash` take one of three symbolic values — `from-content` (the
harness hashes that side's own content, `remote_body` or `local_content` respectively,
with its own recipe), `stale` (any value that cannot match), or `absent` (the field is
empty, i.e. `None`) — never a literal digest.

The bash side realises `mtime_offset` by reading the fixture file's **real** mtime and
passing `--timestamp <mtime - offset>`, never by `touch`ing the file to an absolute
epoch — BSD `-t` and GNU `-d @…` disagree on spelling, which is the portability problem
the relative offset exists to avoid. `test-work-item-scripts.sh:387-392` already derives
a baseline timestamp from a real mtime this way.

⚠️ Fabricated digest strings and absolute mtimes are **not executable by bash**.
`work-item-sync-classify.sh` derives the local hash by running
`work-item-normalise.sh <file> | hash_sha256_stdin` over real content (`:162-183`);
there is no input that makes a file's normalised sha256 equal an arbitrary string
like `lh-xyz`, and `--dirty`-style flags do not exist for the digests. An absolute
`mtime` would also need materialising through `touch` with mutually incompatible
BSD `-t` and GNU `-d @…` spellings. Symbolic hashes plus a relative offset are what
make one table drive both sides — which is the entire reason it sits under
`skills/work/scripts/test-fixtures/` rather than in `cli/work/tests/fixtures/`.

`applies_to` marks which implementations must run each row, and `rust_only` holds
the call-observability assertions bash has no way to make. Three row families are `["rust"]`, each because bash cannot express the input rather
than because it disagrees on the output: `NotReported` versus `NotRead` (both
`--remote-updated ""` in bash), `Dirtiness::Unknown` (bash's only test is
`[ "$dirty" = "1" ]`, so unknown reads as clean), and the **absent-mtime** row — in bash
an unreadable mtime means the file is gone, so `_wisc_mtime` returns its
`9999999999` sentinel and the hash path then runs `work-item-normalise.sh` over a
missing file, which exits 1 and aborts classify under `pipefail`.

⚠️ Each side asserts the count of rows **it ran**, and both counts are pinned against
**committed floor constants** — not merely against the number of rows marked for that
side. Deriving the expectation from `applies_to` itself is circular: narrowing a row
from `["rust", "bash"]` to `["rust"]` would reduce expected and actual together, so the
bash suite goes green having stopped exercising it — a one-word fixture edit no
assertion can see, reintroducing the two-transcriptions drift the shared table exists to
prevent. The floors follow the `_EXPECTED_WORK_SUITES` idiom, so de-scoping a row takes
a deliberate second edit, and a `["rust"]`-only row must state why bash cannot express
it.

Rows to transcribe, from `test-work-item-scripts.sh:318-414`: the four verdict
rows, the trusted updated-equality short-circuit with no body, the ticked-updated
body-matches row, whitespace-equivalent-local, the mtime short-circuit, the
no-`external_id` row, `remote-absent`, `indeterminate`, and first-sync-on-dirty.

Add eight rows the bash suite lacks and the port needs:

- `(NotReported, NotReported)` proving nothing (→ `conflict`, not `synced`) — the
  row that fails under `==`.
- `NotRead` on either side proving nothing.
- A baseline carrying `remote_updated_at` but no `remote_hash` (always
  remote-changed).
- An absent mtime forcing the hash.
- `mtime == baseline_timestamp` exactly — the inclusive boundary. At one-second
  epoch granularity a file written during the run has this shape, so getting it
  wrong masks the very item a run just wrote.
- A persisted `remote_hash: ""` and, separately, `local_hash: ""` — both read as
  absent, both sides changed, **no digest call**. This is the row that pins the
  empty-string-is-absent conversion.
- An absent baseline hash asserting `rust_only.expect_mtime_called: false`, pinning
  the `:162` stat short-circuit the lazy port exists to preserve.

Decide rows are the 7 states × 3 directions grid — with `dirty` sub-splitting
`remotely-modified` across all three `Dirtiness` values, `Unknown` deciding as
`Dirty` — plus the token cases from `:417-464`. Note `:417-459` stops two
assertions short: the empty and `frobnicate` rows run to `:464`, and they are the
two that prove an unrecognised token never becomes a destructive write. Add the
leading-U+00A0 row.

The bash harness passes `--dirty` on five rows, not two (`:444-448`: three with
`0`, two with `1`). The semantic claim still holds — the script's only test is
`[ "$dirty" = "1" ]` — but transcribe from the assertions, not from that summary.

**File**: `cli/work/tests/sync_classify.rs` (new)
**File**: `cli/work/tests/sync_decide.rs` (new)
**Changes**: one test per file iterating its table, **with a row-count
assertion** — the `cli/vcs-cli/tests/guard_decision_table.rs:143-182` shape. A
table silently emptied by a parse change otherwise passes.

Parsing JSON in `work`'s own tests needs `serde_json` as a dev-dependency. That
is a dev edge only; `work`'s pup rule constrains `src/`, and the crate's shipped
dependency set is unchanged.

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes**: the bash side of the same two tables — a `jq`-driven loop over each,
with a row-count assertion, replacing the inline `assert_eq` cases.

⚠️ Read the table by **redirect**, never a pipeline: `while … done < <(jq …)` or a
`for` over a `jq`-built list. `jq … | while read` runs the loop body in a subshell,
so every `FAIL=$((FAIL + 1))` is discarded and the section reports green regardless
of what failed (`test-work-item-scripts.sh:89-98` tracks results in shell
variables). `mapfile`/`readarray` is bash 4 and banned by the 3.2 floor.

#### 6. The label golden, held by both implementations

**File**: `skills/work/scripts/test-fixtures/work-item-sync-label.golden` (new)
**Changes**: flat `|`-delimited, two sections, matching the
`work-item-file-dirty.golden` shape rather than a case-directory tree — the
script is scalar-in, scalar-out.

```text
# work-item-sync-label.sh golden file.
#
# Format, per section:
#   [CLASSIFY]  <raw-external-id>|<expected-status>
#   [LABEL]     <status>|<exit>|<expected-output>
#   [DEFAULT]   <raw-external-id>|<expected-output>
#
# (empty) is the empty-value sentinel, as in work-item-file-dirty.golden and
# work-item-canonicalise-id.golden. A trailing empty field would be invisible in
# review and lost to any trailing-whitespace trim.
#
# Outputs carry NO trailing newline (printf without \n,
# work-item-sync-label.sh:57-61).
#
# CLASSIFY strips one combined [[:space:]"'] class from both ends (:45) — not
# quotes and then whitespace. The mixed rows below are what distinguish the two.
#
# remote-absent and indeterminate are exit-1 rows: --label renders five states
# and rejects those two (work-item-sync-label.sh:54-67). Those rows pin the exit
# status only, not the message. They are classified states the caller handles,
# never labels /list-work-items renders.
#
# DEFAULT is the composed no-flag arm (:104-106) — the arm
# list-work-items/SKILL.md:289 actually invokes.

[CLASSIFY]
PROJ-0042|synced
BLA-123|synced
atomic-innovation/accelerator#42|synced
"PROJ-0042"|synced
'PROJ-0042'|synced
   "PROJ-0042"   |synced
|unsynced
""|unsynced
   |unsynced
"  "|unsynced
'"|unsynced

[LABEL]
synced|0|🟢 synced
unsynced|0|⚪ unsynced
locally-modified|0|🔵 locally modified
remotely-modified|0|🟣 remotely modified
conflict|0|🔴 conflict
remote-absent|1|(empty)
indeterminate|1|(empty)
bogus|1|(empty)

[DEFAULT]
PROJ-0042|🟢 synced
|⚪ unsynced
```

**File**: `skills/work/scripts/test-work-item-scripts.sh`
**Changes**: replace the inline label assertions at `:36-109` and `:308-315`
with a loop over the golden (by redirect, per the subshell warning above). The
section's existing pairwise-distinctness and no-ANSI checks stay — they are
properties of the set, not rows.

The golden must be a strict **superset** of what it replaces. The inline
assertions currently cover two arms the two original sections omit: the default
composed mode (`:104-106`) and `--label bogus` → exit 1 (`:109`). Dropping them
would reduce the regression net on a script that survives both this story and
0171's cutover, in the name of adding a golden — hence `[DEFAULT]` and the `bogus`
row.

**File**: `cli/work/tests/sync_label_parity.rs` (new)
**Changes**: the same golden, asserted against `work::sync::label`,
`classify_external_id` and the `RenderableState` narrowing, with a row-count
assertion per section. `[DEFAULT]` has no typed Rust counterpart (the composition
is the script's own), so that section is count-asserted only — recorded in the test
as a deliberate asymmetry rather than an oversight.

#### 7. The pure planner

**File**: `cli/work/src/sync/plan.rs` (new)

```rust
pub struct RemoteFacts<'a> {
    pub presence: RemotePresence,
    pub remote_updated: &'a RemoteTimestamp,
}

pub struct PlannedAction {
    pub id: String,
    pub state: SyncState,
    pub action: Action,
}

pub struct SyncPlan {
    pub actions: Vec<PlannedAction>,
}

pub struct PlanInput<'a> {
    pub id: String,
    pub external_id: Option<&'a ExternalId>,
    pub facts: RemoteFacts<'a>,
    pub dirty: Dirtiness,
    pub baseline: BaselineEntry<'a>,
    pub baseline_timestamp: u64,
    pub digests: &'a dyn ItemDigests,
}

impl SyncPlan {
    #[must_use]
    pub fn pull_count(&self) -> usize;
    #[must_use]
    pub fn push_count(&self) -> usize;
}

/// Which ids need a body read before the plan can be completed.
#[must_use]
pub fn needs_body_read(items: &[PlanInput<'_>]) -> Vec<ExternalId>;

pub fn plan(
    items: &[PlanInput<'_>],
    direction: SyncDirection,
    resolutions: &BTreeMap<String, Resolution>,
) -> Result<SyncPlan, kernel::Error>;
```

One input type, not three. An earlier revision named `SubjectRef`, `PlanInput` and a
`BaselineLookup` port without defining any of them, which would have left `work` — a
public-api-pinned crate — carrying three shapes for "the item being reconciled" and two
for "what the baseline knows", chosen at the keyboard. `PlanInput` carries everything
`classify` needs, including the item's `&dyn ItemDigests`, so `plan` reuses `Subject`
and `BaselineEntry` rather than paralleling them and needs no lookup port.

`pull_count`/`push_count` live on `SyncPlan` so the write bounds are a pure property of
the plan, checkable before `run` touches anything.

The purity claim is precisely: **no remote I/O**. Local reads still happen, lazily,
through the injected `ItemDigests` port when `classify` asks — which is the whole point
of the port. "Fetching stays outside planning" refers to `fetch_all`/`show`.

Planning is pure and lives in `work`, over facts a caller has already gathered.
`needs_body_read` is the two-tier read rule expressed as a pure function — the ids
whose stamp fails `proves_unchanged_since` against their baseline entry — so the
adapter's fetch shell asks it what to `show` rather than owning the rule.
Resolutions apply after `decide`: a `Prompt` with a matching order becomes `Pull`
(`AcceptRemote`) or `Push` (`PushLocal`); `Skip` and no order leave it `Prompt`.

`proves_unchanged_since` here is a deliberate divergence from bash, which gates
the `show` on raw string inequality (`sync-work-items/SKILL.md:116-122`). For an
issue the tracker reports with no stamp against a baseline holding `""`, bash sees
two equal empty strings and fetches no body; the Rust path fetches and can conclude
`synced`. The Rust behaviour is better — a null stamp proves nothing either way —
but it means AC 18's call counts must be stated in terms of the new rule (a
`NotRead`/`NotReported` baseline costs one `show`), not transcribed from bash.

**File**: `cli/work/tests/sync_plan.rs` (new)
**Changes**: the branch table, all at unit level with no fake tracker needed — a
stamp that proves unchanged yields no `needs_body_read` entry; `absent` →
`RemoteAbsent`; `indeterminate` → `Indeterminate`; each `Resolution` applied to a
`Prompt`; a stale resolution ignored; `Noop` suppression.

#### 8. The run clock port

**File**: `cli/work/src/sync/clock.rs` (new)

```rust
pub trait RunClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error>;
}
```

Declared here, implemented over `SystemTime` in `work-adapters`, for the reason the
decisions record: `corpus::Clock` has no epoch operation, and adding one would touch
a port three other crates implement, churn `corpus`'s pinned snapshot and break every
existing fake `Clock`.

Fallible, and a failure must leave the baseline timestamp **untouched** rather
than substituting any fallback. The persisted timestamp is the sole gate on the
hash-free local short-circuit, so a value derived too large marks every local file
unchanged and turns every remote-side change into an unconditional pull across the
whole corpus. No advance means a full re-hash, which is slow and correct.

### Success Criteria

#### Automated Verification

- [ ] Lifted tables pass against the Rust implementations: `mise run test:unit:cli`
- [ ] The same tables pass against the bash scripts: `mise run test:integration:work`
- [ ] Both label implementations pass the shared golden: `mise run test:unit:cli`
      and `mise run test:integration:work`
- [ ] Planner branch table passes: `mise run test:unit:cli`
- [ ] The architecture rule admits the new `tracker` imports:
      `mise run test:integration:pup`
- [ ] The `work` public-api snapshot is updated deliberately:
      `mise run public-api:check` after `mise run public-api:update`
- [ ] Shell lint and format pass: `mise run scripts:check`
- [ ] Format, lint and types: `mise run cli:check`
- [ ] Whole tree green: `mise run`

#### Manual Verification

- [ ] The `public-api.txt` diff contains only the intended `work::sync` surface,
      plus the `tracker` types the widened rule now exposes there — the coupling
      the Key discoveries note records, not noise
- [ ] Each classify row's provenance is traceable to a bash assertion or is one
      of the eight deliberately-added rows, and no expectation was derived from
      reading the Rust
- [ ] The bash label loop reads by redirect, not through a pipe — deliberately
      break one row and confirm the section reports the failure

---

## Phase 3: Persistence and apply in `work-adapters`

### Overview

The baseline document, the digest computation, and the per-item apply sequence
with its resumability contract and fault seam. Adds `corpus`, `tracker` and
`sha2` edges to `work-adapters`.

### Changes Required

#### 1. New dependencies

**File**: `cli/work-adapters/Cargo.toml`
**Changes**:

```toml
[dependencies]
work = { path = "../work" }
kernel = { path = "../kernel" }
tracker = { path = "../tracker" }
corpus = { path = "../corpus" }
vcs = { path = "../vcs" }
vcs-adapters = { path = "../vcs-adapters" }
sha2 = { workspace = true }
tempfile = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tracker-test-support = { path = "../tracker-test-support" }
```

`corpus` rather than `store`: baseline writes go through the `corpus::AtomicWrite`
port, and the concrete `FileCorpusStore` is injected by `work-cli`. No domain
crate gains a `store` edge, and neither does this adapter.

#### 2. Digest computation

**File**: `cli/work-adapters/src/sync/digest.rs` (new)
**Changes**: the two recipes the baseline stores, matching
`work-item-sync-apply.sh:126-130`.

```rust
pub fn local(file_content: &str) -> Result<String, kernel::Error>;

#[must_use]
pub fn remote_body(projected_body: &str) -> String;
```

`remote_body` is `sha256(work::normalise::trim_lines(projected_body))`. Both render
lowercase hex to match `hash-common.sh`'s `sha256sum`/`shasum` output.

`local` reproduces `work-item-normalise.sh:113-116` **byte for byte**:

```rust
let fm = filter_frontmatter_keys(frontmatter);
let body = trim_lines(body);
let hashed = format!(
    "{}\n{}\n",
    fm.trim_end_matches('\n'),
    body.trim_end_matches('\n'),
);
```

⚠️ Both separators are emitted **unconditionally**, whether or not that side is
empty. Bash is `fm=$(…); body=$(…); printf '%s\n%s\n' "$fm" "$body"` — command
substitution strips each side's trailing newlines and `printf` adds exactly one
back to each. Plain concatenation agrees only while the body is non-empty: for an
empty or all-blank body bash emits a trailing `\n\n` where concatenation emits `\n`.
A single byte reclassifies every affected synced item at cutover, which is exactly
what AC 24 exists to prevent — so the corpus below carries the empty-body cases
rather than only the populated ones.

The unconditional separators are a quirk, not a defect to correct: the digest is
opaque and only ever compared against digests from the same recipe, so the trailing
blank line for an empty body makes the hash arbitrary rather than wrong. Conditional
separators would be the defect — they would change every affected item's
`local_hash` and mass-reclassify at cutover.

⚠️ **A file that does not open with a fence is an error, not an empty
frontmatter.** `local` returns `Err` when the content has no opening
`^---[[:space:]]*$` line and when the frontmatter is unclosed, because bash aborts
non-zero in both cases (see Key discoveries) and produces no hash at all. Returning
`Ok` over a body-only file would hash something bash refuses — the Rust side
silently accepting input the oracle rejects, which is the wrong direction for a
parity port to fail in.

⚠️ **The splitter is the digest path's own, not `document`'s.** It matches
`^---[[:space:]]*$` on both delimiters and has no scan cap, reproducing
`config_extract_frontmatter`/`config_extract_body` exactly.
`document::fence::split` compares the exact bytes `b"---"` and stops at 1 MiB, so a
fence carrying a trailing space would make the whole file body, the `IGNORE_KEYS`
filter would never run, and a routine `last_updated` restamp would become a content
change — a silent `locally-modified` reclassification from whitespace no reviewer
sees and no Markdown formatter policies. `work-adapters` therefore gains **no**
`document` edge; the recogniser lives beside the recipe it serves, and the corpus
carries padded-fence cases so the two implementations are pinned against each other
rather than assumed equal.

The recipe does carry one genuine latent flaw, **inherited and out of scope
here**: `fm` and `body` are both multi-line and are joined by a single `\n`, so the
boundary is not recoverable from the hashed string. A frontmatter key moved verbatim
to the top of the body produces an identical hash, leaving the item `synced` while
local and remote have diverged. Fixing it needs a delimiter that cannot occur in
either side, which changes every hash — so it needs a recipe-version field in the
baseline and a deliberate one-time re-hash, and it cannot land in a story whose
contract is byte-agreement with the hashes bash already wrote.

`RemoteIssue.body` is the **un-normalised** projection. Normalising before
hashing is this story's obligation.

A `LazyItemDigests` struct implementing `work::sync::ItemDigests`, memoising mtime
and each hash in a `RefCell<Option<_>>`, stat'ing and reading the file only on first
call, and propagating I/O failures as `Err` rather than as an empty hash.

#### 3. The baseline document

**File**: `cli/work-adapters/src/sync/baseline.rs` (new)

```rust
pub struct Baseline {
    timestamp: u64,
    items: BTreeMap<String, Entry>,
}

pub struct Entry {
    pub remote_updated_at: RemoteTimestamp,
    pub remote_hash: String,
    pub local_hash: String,
}

pub enum Degradation {
    None,
    Unparseable { detail: String },
    EntriesDiscarded { ids: Vec<String> },
}

impl Baseline {
    #[must_use]
    pub fn read(content: Option<&str>) -> (Self, Degradation);

    #[must_use]
    pub fn render(&self) -> String;

    pub fn set(&mut self, id: &str, entry: Entry);
    pub fn remove(&mut self, id: &str);
    pub fn set_timestamp(&mut self, epoch: u64);

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Entry>;

    #[must_use]
    pub const fn timestamp(&self) -> u64;
}

pub fn path(
    integrations_dir: &Path,
    integration: &str,
) -> PathBuf;
```

`read(None)` and `read(Some(unparseable))` both yield `{timestamp: 0, items:
{}}` — the degrade-to-empty contract at `work-item-sync-baseline.sh:79-86`, which
is what lets a VCS-conflict-markered file degrade to a full re-hash rather than
crashing.

**Tolerance is per entry, not per document.** Bash reads each field individually
(`jq -r '.local_hash // empty'`), so a malformed or unknown-schema *entry* degrades
only that item. A strict whole-document deserialise would discard every entry over
one bad field — turning one item's problem into a corpus-wide reclassification, and
diverging from the engine still live beside it. So: unknown fields are ignored, a
missing or wrongly-typed field reads as empty, and only the offending entry is
discarded (reported via `Degradation::EntriesDiscarded`). Whole-document
degrade-to-empty is reserved for content that is not JSON at all.

**Degradation is returned, not swallowed — but nothing is moved.** Because a missing
baseline hash means changed, an empty baseline makes every item look modified — so a
conflict-markered `last-sync.json` turns one silent parse failure into a corpus-wide
re-sync with nothing pointing at the cause. `work-cli` prints a named warning on
stderr naming the path and the reason. Behaviour parity is preserved; the silence is
not.

An earlier revision renamed the unparseable file aside to
`last-sync.json.unparseable`. Dropped, for four reasons that only surface once it is
written down. It is a filesystem mutation the live bash engine does not perform
(`work-item-sync-baseline.sh:79-86` degrades in place), so the two engines would
disagree about whether *reading* a broken baseline writes. The motivating case is a
conflict-markered file, so the rename strips conflict markers from a tracked path — to
jj or git that reads as a resolution the user never made. It would fire before the
`--max-pulls`/`--max-pushes` refusal, so a run exiting 5 "having written nothing" would
in fact have moved the baseline, and the retry would then see a *missing* one — a
normal first-sync state carrying no warning at all, the safeguard bypassing itself. And
a fixed filename clobbers the previous aside on a second occurrence. The loud warning
is the whole remedy; the file stays where the user and their VCS left it.

**A missing or non-integer `timestamp` reads as 0, with every entry preserved.** Bash
is tolerant twice over — the SKILL reads `jq -r '.timestamp // 0'` and
`work-item-sync-classify.sh:153` coerces a non-numeric value to 0 — so a
`"timestamp": "1700000000"` costs only the pre-filter. A strict `u64` deserialise would
trip whole-document degrade-to-empty on a document that is otherwise valid JSON and
discard every entry, mass-reclassifying the corpus where bash keeps it intact: the
same asymmetry the per-entry rule above exists to prevent.

**The wire format matches `jq -c`.** Bash writes compact single-line JSON with
`timestamp` before `items`, and `list-work-items/SKILL.md:340` reads `.timestamp`
straight off the file. `render()` therefore emits compact JSON, `timestamp` first,
with a trailing newline. Item keys are `BTreeMap`-ordered rather than
insertion-ordered — a deliberate, stated divergence: it is stable across both
engines and across runs, which insertion order is not, and no consumer depends on
order. Anything else (a pretty-printer, a missing trailing newline) churns the whole
document on the first Rust write.

The stamp mapping, both directions:

```rust
fn stamp_to_json(stamp: &RemoteTimestamp) -> &str {
    stamp.reported().unwrap_or("")
}

fn stamp_from_json(raw: &str) -> RemoteTimestamp {
    if raw.is_empty() {
        RemoteTimestamp::NotRead
    } else {
        RemoteTimestamp::Reported(raw.to_owned())
    }
}
```

Lossy by design and decided deliberately: `NotReported` and `NotRead` both
persist as `""`, and a read-back resolves to `NotRead` because it asserts nothing
about the tracker. Classification is unaffected — `proves_unchanged_since` is
false for both — but a sync report is not. Pin it with a round-trip test
asserting `NotReported` in, `NotRead` out, so the loss is visible rather than
discovered.

`timestamp` serialises as a JSON integer (bash uses `--argjson`); all three
entry fields serialise as JSON strings (`--arg`), so a failed post-push read
persists `""`, never `null`.

⚠️ And an `""` read back is **absent**: the `Entry` → `work::sync::BaselineEntry`
conversion maps it to `None`, mirroring bash's `// empty` plus `[ -n ]`. Pinned by
the two empty-hash rows in Phase 2's classify table.

`path` resolves `<integrations>/<work.integration>/last-sync.json`, matching
`_wisb_path` (`:54-75`).

**File**: `cli/work-adapters/src/sync/baseline_store.rs` (new)
**Changes**: the persistence wrapper, read-modify-write through injected ports on
both sides.

```rust
pub struct BaselineStore<'a> {
    path: PathBuf,
    reader: &'a dyn FileReader,
    writer: &'a dyn AtomicWrite,
}

impl<'a> BaselineStore<'a> {
    pub fn new(
        path: PathBuf,
        reader: &'a dyn FileReader,
        writer: &'a dyn AtomicWrite,
    ) -> Self;

    /// Read the document, reporting any degradation rather than hiding it.
    pub fn load(&self) -> Result<(Baseline, Degradation), StoreError>;

    pub fn set(&mut self, id: &str, entry: Entry) -> Result<(), StoreError>;
    pub fn remove(&mut self, id: &str) -> Result<(), StoreError>;

    /// Blank the named items' `local_hash`, then advance the timestamp.
    /// Skips the advance if any blank fails.
    pub fn finalise_run(
        &mut self,
        blank: &[&str],
        run_start_epoch: u64,
    ) -> Result<(), StoreError>;
}
```

`finalise_run` is **one** operation, not two, because the blank-then-advance ordering is
load-bearing and a two-call API can be called in the wrong order or half-called. It also
owns the rule that a failed blank suppresses the advance. `ItemApplier::finalise`
delegates to it.

The **read** side must be injected too, not reached through `std::fs`. With a
write-only seam, a spy `AtomicWrite` sees the writes while reads come from disk, so
successive per-item `set` calls each start from the pre-run document and any test
asserting accumulated state silently observes last-write-wins instead. It also makes
the crash-and-resume test unwritable, since the second run must read back what the
first actually wrote.

**Each mutation re-reads immediately before rendering**, matching bash's per-`set`
semantics (`work-item-sync-baseline.sh` re-reads inside every `set`/`remove`/
`set-timestamp`). Holding one in-memory document for the whole run would widen the
lost-update window from a single write to an entire run — and this story deliberately
leaves the bash engine live and adds two further writers (`update --push`'s baseline
set and its terminal clear), so an overlapping `/sync-work-items` run would have its
entries dropped. A dropped entry means both sides count as changed, so the item
becomes a spurious `conflict` demanding human resolution, or loses the `remote_hash`
that proved it synced. Single-writer remains the assumption for *concurrent
mutation of the same entry*; per-`set` re-reads are what keep the window as narrow
as bash's.

#### 4. Apply

**File**: `cli/work-adapters/src/sync/apply.rs` (new)

```rust
pub struct ItemApplier<'ctx, 'store> {
    tracker: &'ctx dyn RemoteTracker,
    writer: &'ctx dyn AtomicWrite,
    baseline: &'store mut BaselineStore<'ctx>,
}

impl<'ctx, 'store> ItemApplier<'ctx, 'store> {
    pub fn new(
        tracker: &'ctx dyn RemoteTracker,
        writer: &'ctx dyn AtomicWrite,
        baseline: &'store mut BaselineStore<'ctx>,
    ) -> Self;

    pub fn push(&mut self, item: &PushRequest<'_>) -> Result<(), ApplyError>;
    pub fn pull(&mut self, item: &PullRequest<'_>) -> Result<(), ApplyError>;
    pub fn finalise(&mut self, run_start_epoch: u64) -> Result<(), ApplyError>;
}
```

⚠️ Two **independent** lifetimes. `&'a mut BaselineStore<'a>` — one lifetime for both
— is invariant in `'a`, so constructing the applier would borrow the store for the
whole of its own lifetime: `run` could never read the baseline again, which it must
do for `finalise` and which the resumability tests must do afterwards. It is the
classic `&'a mut T<'a>` trap and it fails at the first call site, where the reflex
fixes (clone, `RefCell`, `Option::take`) all erode the design.

`ApplyError` carries the work-item id and the operation attempted on every variant,
with `From<TrackerError>` and `From<StoreError>` conversions, so a failure names what it
was doing to which item rather than only what went wrong. It also exposes
`fn class(&self) -> Option<FailureClass>` returning `Retryable`/`Terminal` for
tracker-originated variants and `None` for store-originated ones — `for_tracker_error`
takes a `&TrackerError`, so without this accessor the report's fourth field has no
declared path from the error the run actually collected.

`push`, reproducing `work-item-sync-apply.sh:110-133` in order: `tracker.update`
→ fault point → post-push `tracker.show` → project, normalise, hash → local hash
from the file on disk → **baseline set last**.

- A failed `update` returns the error and leaves the baseline entry **unset**
  (`:112-116`), so the next run reclassifies.
- A failed post-push `show` leaves both remote fields empty and **still writes
  the baseline entry** (`:121-129`). That is where `RemoteTimestamp::NotRead` is
  written — the one variant no port operation returns.

`pull` (`:167-176`): atomic write of the reconstructed content → fault point →
baseline set from **post-overwrite** state. ⚠️ `local_hash` comes from the
just-written file and `remote_hash` from the projection actually written.
Deriving either from pre-pull content self-corrupts the baseline into a phantom
`locally-modified` on the next run.

`finalise` sets the global timestamp to the run-**start** epoch and is a sibling
of the per-item path, never folded into it. It lives here rather than on the
baseline module because that is where the bash sibling sits
(`work-item-sync-apply.sh:179-194`) — `work-item-sync-baseline.sh` has no
`finalise` verb, only `set-timestamp`, which `finalise` delegates to.

**The crash is induced through the collaborators, not through a seam on the
applier.** The bash fault point (`ACCELERATOR_TEST_MODE=1` +
`WORK_SYNC_FAIL_AFTER=side-effect`) becomes: a spy `AtomicWrite` configured to fail
the *baseline* write, with the tracker call and the local write allowed to succeed.
That reproduces "side effect landed, bookkeeping did not" exactly, and the post-crash
state is fully assertable — which `exit 99` forecloses.

Deliberately **no** `FaultPoint` field and no `ApplyError::Interrupted` variant.
A public field whose only purpose is a test crash puts a branch in production that
exists only for tests, adds an error variant production can never produce but every
`match` must handle, and makes a flag argument every real call site must pass. The
bash seam it replaces is doubly gated by env vars; a plain public field has no gate
at all, and a non-`None` value reaching production would mutate every remote and then
abandon the baseline write — indistinguishable from a legitimate interruption.
`work-adapters` is public-api-exempt, so nothing else would catch the surface growing
either.

#### 5. Resumability and ordering tests

**File**: `cli/work-adapters/tests/sync_apply.rs` (new)
**Changes**: an in-memory store implementing **both** `FileReader` and
`AtomicWrite` — recording `verb:path` strings in a `RefCell<Vec<String>>` (the
`cli/migrate/tests/lifecycle.rs:105-179` shape, so call **order** is asserted on a
slice rather than inferred) **and retaining the bytes written**, with a
`fail_next_write_to(path)` switch.

A write-only string recorder cannot support the resumability test: the second run
must read back what the first actually wrote. Recording order and retaining content
have to be the same double, or the highest-value test in the phase quietly weakens
into an ordering check.

- For every classified state that produces a side effect, assert the baseline
  write occurs strictly after it.
- With the baseline write failed after a successful side effect, assert a re-run
  reaches the same terminal state as an uninterrupted run.
- Assert a failed `update` leaves the baseline entry absent.
- Assert a failed post-push `show` still writes the entry, with `NotRead`.
- Assert `pull`'s hashes come from post-overwrite state, by pulling content that
  differs from the pre-pull file and checking the stored `local_hash` matches the
  new content.

#### 6. Capture the bash-generated baseline corpus

**File**: `skills/work/scripts/test-fixtures/work-item-sync-baseline/regenerate.sh` (new)
**File**: `skills/work/scripts/test-fixtures/work-item-sync-baseline/case-*/` (new)
**Changes**: `work-item-project-remote.sh` is the only thing that can produce
`remote_hash` values by the original recipe, and its removal is coming. Capture now.

Named for its generating script and `case-`-prefixed, matching
`work-item-project-remote/`, `work-item-section-diff/` and `work-item-normalise/`.
`regenerate.sh`, not `generate.sh`, matching `hooks/test-fixtures/vcs-detect/` and
`cli/migrate-cli/tests/fixtures/`. It is invoked as `bash regenerate.sh` and is
**exempt from the executable-bit invariant in both directions** — `tasks/lint/scripts.py`
skips any source whose path carries a `test-fixtures` segment (`_FIXTURE_SEGMENT`),
and `tasks/README.md` records fixtures as a third category outside the rule. ShellCheck,
shfmt and the bashisms linter still apply. Its sibling
`hooks/test-fixtures/vcs-detect/regenerate.sh` sits under the same exemption. Its provenance, pre-conditions and freeze
condition go in its **header comment block** following the `vcs-detect` shape — no
`README.md`, because no `test-fixtures/` tree in the repo has one and it would
create a second place fixture provenance lives. The header states the durable
condition — that the corpus must be regenerated while `work-item-project-remote.sh`
still exists and is frozen thereafter — with no work-item number, per the standing
rule against stale-prone references in comments.

Each case directory holds `remote.json` (the tracker `show` payload), `local.md`
(the work item), and `expected.json` (the baseline entry the bash path produces,
including which integration produced it). Every fixture file carries an extension.

Cases — the populated set, plus the empty and adversarial shapes the recipes
actually diverge on:

- jira with an ADF description including reordered keys
- jira with an absent description (projects as the literal four bytes `null`)
- linear with Markdown
- linear with an empty description (projects as an empty line)
- a local file carrying every `IGNORE_KEYS` field
- **a local file whose body is empty** and one whose body is all-blank — the two
  shapes where bash's unconditional `printf` separators diverge from concatenation
- **a padded opening fence (`--- `) and a padded closing fence** — the shapes where
  `document`'s exact-byte comparison would diverge from awk's
  `^---[[:space:]]*$`, pinning the digest path's own recogniser
- **error-parity cases, asserted as errors rather than hashes**: a body-only file
  with no opening fence, an unclosed frontmatter, and an empty file. Bash exits 1
  and emits nothing for all three, so these carry no `expected.json` hash — the
  case records the expected **failure**, and `regenerate.sh` records the exit
  status rather than output. Without this distinction the generator has nothing to
  write for them and the `bash-parity` test nothing to assert.
- **an ADF description carrying a fractional number, an exponent and a large
  integer** — jq 1.7 preserves an unmodified numeric literal; `serde_json`
  round-trips through `f64`, so `33.333333333333336`, an exponent, or an integer
  beyond 2^53 can render differently
- **an ADF description carrying a control character** — jq escapes U+007F as
  ``; `serde_json` emits it raw

Without the last two, the corpus cannot detect a jq-versus-`serde_json` rendering
divergence, which changes every jira `remote_hash`.

**File**: `cli/work-adapters/tests/sync_baseline_corpus.rs` (new)
**Changes**: for each case, feed `remote.json` through
`work_adapters::project_remote` and `digest::remote_body`, feed `local.md`
through `digest::local`, and assert both match `expected.json`. Assert the case
count.

**File**: `cli/work-adapters/tests/sync_baseline_shellout_parity.rs` (new)
**Changes**: feature-gated on `bash-parity` (which `--all-features` turns on in CI,
per the `diff_shellout_parity.rs` precedent): for each case, shell the live
`work-item-project-remote.sh` + `hash-common.sh` and assert **both** the Rust recipe
and the committed `expected.json` match what bash actually outputs.

Without this, the corpus's bash provenance rests on a manual step. Nothing
otherwise distinguishes a corpus written by bash from one accidentally regenerated
from the Rust recipe — in which case the oracle agrees with itself and the whole test
is vacuous. After the generator is removed the mistake is unrecoverable, so the guard
has to exist while the generator does.

**File**: `cli/work-adapters/tests/project_remote_parity.rs` (new)
**Changes**: the existing `test-fixtures/work-item-project-remote/case-*` fixtures
asserted against `work_adapters::project_remote`, which today no Rust test covers.
Record that the Rust projection omits the trailing newline bash's `printf '%s\n%s\n'`
emits, and that this is benign **only** because `trim_lines` absorbs it downstream —
an invariant worth stating, since it is the one place the two projections differ.

Together these are the classification-stability oracle AC 24 rests on. Note what
they prove and what they do not: digest equality is necessary but not sufficient. A
wrong branch order in `classify`, a `==` where `proves_unchanged_since` belongs, or a
baseline read-back bug leaves every digest identical and still mass-reclassifies —
which is why Phase 4 adds the end-to-end half.

### Success Criteria

#### Automated Verification

- [ ] Baseline round-trip, per-entry tolerance, degrade-to-empty and stamp mapping
      pass: `mise run test:unit:cli`
- [ ] The empty-string-hash-is-absent conversion passes: `mise run test:unit:cli`
- [ ] Apply ordering, the induced-crash re-run and post-overwrite hashing pass:
      `mise run test:unit:cli`
- [ ] The bash-generated corpus matches the Rust recipes, including the empty and
      numeric/control-character cases: `mise run test:unit:cli`
- [ ] The corpus and the Rust recipes both match live bash output:
      `mise run test:unit:cli` (the `bash-parity` gate is on under `--all-features`)
- [ ] Architecture rules still hold with the new adapter edges: `mise run test:integration:pup`
- [ ] Dependency policy admits `sha2` in `work-adapters`: `mise run deny:check`
- [ ] Shell lint and format pass, including the new regenerator:
      `mise run scripts:check`
- [ ] Format, lint and types: `mise run cli:check`
- [ ] Whole tree green: `mise run`

#### Manual Verification

- [ ] `regenerate.sh` reproduces the committed corpus byte for byte on a clean
      checkout
- [ ] The regenerator's header states the freeze condition in durable terms — that
      it must run while `work-item-project-remote.sh` exists — with no work-item
      number
- [ ] A deliberate one-byte change to `digest::local`'s separator handling reddens
      the empty-body corpus case

---

## Phase 4: `accelerator work sync`

### Overview

The command: plan-then-apply, `--preview`, the machine-parseable conflict report
with `--resolve`, provider selection from `work.integration`, and exit code 4.
Closes the two-invocation loop from a test harness.

### Changes Required

#### 1. The fetch shell

**File**: `cli/work-adapters/src/sync/fetch.rs` (new)

```rust
pub struct LocalItem {
    pub id: String,
    pub path: PathBuf,
    pub external_id: Option<ExternalId>,
}

pub enum RetrievalStrategy {
    Bulk,
    PerItem,
}

pub struct GatheredRemote {
    pub presence: RemotePresence,
    pub remote_updated: RemoteTimestamp,
    pub body: Option<String>,
}

pub struct GatheredFacts {
    pub per_id: BTreeMap<String, (GatheredRemote, Dirtiness)>,
    pub read_failure: Option<TrackerError>,
}

impl GatheredFacts {
    /// Borrowed planner inputs, paired with each item's digest port.
    pub fn plan_inputs<'a>(
        &'a self,
        items: &'a [LocalItem],
        digests: &'a [LazyItemDigests],
        baseline: &'a Baseline,
    ) -> Vec<PlanInput<'a>>;
}

pub fn gather(
    items: &[LocalItem],
    baseline: &Baseline,
    tracker: &dyn RemoteTracker,
    status: &dyn WorkingCopyStatus,
    strategy: RetrievalStrategy,
) -> GatheredFacts;
```

`GatheredFacts` holds **owned** values. `RemoteFacts<'a>`/`PlanInput<'a>` are borrowed
views, so a `GatheredFacts` carrying them directly would have to borrow from itself;
`plan_inputs` builds the borrowed views on demand from the owned store, the caller's
`LocalItem` slice and a parallel `LazyItemDigests` slice. That keeps the laziness the
digest port exists for — an eager `Vec<String>` of hashes would defeat both
short-circuits.

This is the imperative shell only — it performs the reads and hands the facts to
`work::sync::plan`, which owns every rule. Bulk mode is bulk-**then**-`show`, not
bulk-instead-of, matching `sync-work-items/SKILL.md:96-127`: one `fetch_all` over
every present `external_id`, then `show` for exactly the ids
`work::sync::needs_body_read` names. Per-item strategy calls `show` for every id and
never calls `fetch_all`.

`fetch_all` returning `Err` marks **every** id `Indeterminate` and writes nothing
(`SKILL.md:101-104`) — and carries the `TrackerError` into `GatheredFacts` so the run
can report it. `fetch_all` fails only on pre-flight problems (unresolvable
credentials, an unembeddable id) and its `detail` is the sole diagnostic; discarding
it turns a misconfigured token into a whole-corpus "nothing to do".

Dirtiness comes from an injected `WorkingCopyStatus` port feeding
`work::file_dirty::is_dirty` — probed **once per run** under jj (`jj diff
--name-only` is a whole-tree list) and **per path** under git (`git status
--porcelain -- <path>`), which is why the strategy is a property of the port rather
than of the caller. A probe failure yields `Dirtiness::Unknown`, which decides as
dirty.

The classifier itself never fetches, and neither does the planner. Their inputs
are pre-gathered, which is what AC 18's "the classifier records no calls" asserts —
and what makes the planner's rules unit-testable without a fake.

#### 2. The run

**File**: `cli/work-adapters/src/sync/run.rs` (new)

```rust
pub enum RunError {
    Refused {
        pulls: usize,
        pushes: usize,
        max_pulls: usize,
        max_pushes: usize,
    },
    Read(TrackerError),
    Internal(kernel::Error),
}

pub enum RunMode {
    Preview,
    Apply,
}

pub struct SyncPorts<'a> {
    pub tracker: &'a dyn RemoteTracker,
    pub status: &'a dyn WorkingCopyStatus,
    pub writer: &'a dyn AtomicWrite,
    pub clock: &'a dyn RunClock,
}

pub struct SyncRequest<'a> {
    pub items: &'a [LocalItem],
    pub direction: SyncDirection,
    pub strategy: RetrievalStrategy,
    pub resolutions: &'a BTreeMap<String, Resolution>,
    pub max_pulls: usize,
    pub max_pushes: usize,
    pub mode: RunMode,
}

pub enum ItemOutcome {
    Applied,
    NotApplied,
    Failed(ApplyError),
}

pub struct ReportedItem {
    pub planned: PlannedAction,
    pub outcome: ItemOutcome,
}

pub struct RunReport {
    pub reported: Vec<ReportedItem>,
    pub read_failure: Option<TrackerError>,
    pub baseline_degradation: Degradation,
    pub finalised: bool,
}

impl RunReport {
    /// Items this run left for a human: `Prompt`, `SkipConflict`, `SkipDirty`,
    /// `RemoteAbsent` and `Indeterminate`. Derived, never stored.
    pub fn awaiting_human(&self) -> impl Iterator<Item = &ReportedItem>;
}

pub fn run<'a>(
    ports: &SyncPorts<'a>,
    baseline: &mut BaselineStore<'a>,
    request: &SyncRequest<'_>,
) -> Result<RunReport, RunError>;
```

⚠️ The lifetime is **named and shared** between `ports` and the store's own parameter.
Verified by compiling this design: with both elided (`&SyncPorts<'_>` and
`&mut BaselineStore<'_>`) rustc rejects the `ItemApplier::new` call with *"argument
requires that `'1` must outlive `'2`… mutable references are invariant over their type
parameter"*. `ItemApplier<'ctx, 'store>` requires the tracker, the writer and the
store's inner writer to share `'ctx`, and `&mut BaselineStore<'ctx>` is invariant in
`'ctx`, so the two cannot be independent. This is the third variant of the same
invariance trap in this design and the only one the two-lifetime `ItemApplier` fix did
not already cover — the coupling is real, not incidental, and `composed_context()`
satisfies it naturally by building the ports and the store from the same borrows.

⚠️ The store is a **separate `&mut` argument**, not a field. Holding it as
`&'a mut BaselineStore<'a>` inside a context struct is the `&'a mut T<'a>`
invariance trap Phase 3 §4 rejects — and worse here, `'a` would unify with every
other borrow in the struct, so it would be inferred as the composition-root lifetime
and the store would be unusable for `finalise` and unreadable by the resumability
tests. Passing it separately scopes the borrow to the call, which is also what lets
`ItemApplier::new` reborrow it per item.

Splitting ports from request has a second benefit: only the store needs `&mut`, so
neither struct does, and a test varying one parameter no longer constructs a
ten-field clump. `mode` moves onto the request rather than being a stray argument, so
the next parameter has an obvious home.

⚠️ One record per item, not two parallel collections. An earlier revision carried
`reported: Vec<PlannedAction>` beside `failures: Vec<(String, ApplyError)>`, which forced
the renderer to join a `String` id against `PlannedAction.id` to recover `<state>` for a
`failed` line — with nothing in the types preventing a miss, and a miss silently dropping
either the state or the safety-critical class. `ItemOutcome` on the item makes a whole
line renderable from one value, and `ApplyError::class()` supplies field 4 directly.

Renamed `RunReport` from `RunOutcome` for the same reason the method was renamed:
`work-cli` already uses `RunOutcome` for three per-command result enums
(`resolve::RunOutcome`, `show::RunOutcome`, `diff::RunOutcome`), and this is a whole-run
report rather than one command's outcome.

`awaiting_human` is derived rather than stored: the exit-code decision reads it while
the report reads `reported`, and two fields that must agree can disagree — a filter
applied to one and not the other yields a run that reports conflicts and exits 0.

⚠️ Named `awaiting_human`, not `unresolved`, because `unresolved` is already the wire
keyword for `Action::Prompt` **alone**. Exit code 4 covers a wider set, so one word for
both would have a reader assume the report keyword and the method describe the same
items — and getting that backwards makes a run with one `skip-dirty` item exit 0.

`run` captures the run-start epoch from the injected `RunClock` **before reading any
item**, gathers facts and computes the plan. It then refuses — **in both modes** —
when the plan's `Pull` count exceeds `max_pulls` or its `Push` count exceeds
`max_pushes`, returning `RunError::Refused` with both counts and both limits, having
written nothing. Under `RunMode::Apply` it otherwise executes the plan, blanks the
`local_hash` of every item whose local side classified changed but was left
unreconciled, and calls `finalise(run_start_epoch)`. Under `RunMode::Preview` it
returns after planning: no local write, no `create`/`update` call, no per-item `set`,
no blanking, **no** `finalise`. A preview that advanced the global timestamp would
poison the next real run's mtime pre-filter.

**Apply is an exhaustive match, not "every non-`Noop` action".** Four of the six
`Action` variants are non-`Noop` and non-executable:

| Action | Applied | Reported as | Counts as unclean |
|---|---|---|---|
| `Push` / `Pull` | yes | `push` / `pull` | on failure |
| `Prompt` | no | `unresolved` | yes |
| `SkipConflict` / `SkipDirty` | no | its keyword | yes |
| `Noop` | no | its state | only if `indeterminate` |

**The finalise gate is per-item, not per-run.** Before advancing the timestamp, the
run **blanks the `local_hash`** of every unreconciled item **whose local side
classified changed** — the `Conflict`-derived states, reported as `unresolved` or
`skip-conflict` — and then finalises. An item with no `local_hash` cannot enter the
mtime pre-filter at all, so its local side is re-hashed on the next run regardless of
the timestamp. If any blank write fails, `finalise` is **skipped** and the timestamp
is left where it was.

⚠️ The scope is "local side classified changed", not "unreconciled". `skip-dirty` and
the bidirectional dirty-pull `Prompt` are reachable **only** from
`remotely-modified`, which by construction means the local side classified *unchanged*
(`work-item-sync-classify.sh:161-172`) — so their stored `local_hash` is accurate, not
stale, and the pre-filter is right to trust it. Blanking it would make the next run
default the local side to changed, turning `remotely-modified` into `conflict`, whose
bidirectional decision is `Prompt`, which blanks again — the item sticks at
`unresolved` **forever, even after the user commits and the file is clean**, and the
pull that should have happened once dirtiness cleared never does. Because
`Dirtiness::Unknown` decides as dirty for every item, one failed VCS status probe
would do that to the whole corpus at once. And a manufactured `conflict` is
resolvable, so `--resolve <id>=local` on it pushes never-modified local content over
a genuinely newer remote — the worst case the write bounds exist to prevent, reached
from a transient probe failure.

⚠️ A failed blank write must suppress the advance. Blanking goes through the same
read-modify-write path as every other mutation and can fail per item. Advancing anyway
would leave that item's *old* `local_hash` in place with the timestamp now past its
mtime — reinstating in full the hazard blanking exists to prevent, on the one item the
run knew was at risk. No advance is slow and correct, exactly as the `RunClock` failure
rule already argues.

⚠️ This is load-bearing, and both obvious readings are wrong. The mtime pre-filter
treats `mtime <= baseline_timestamp` as locally-unchanged **without hashing**
whenever a `local_hash` is present. Finalising unconditionally would advance the
timestamp past a skipped item's file mtime while its entry still holds the *old*
`local_hash`, so the next run short-circuits its local side to unchanged, a genuine
`conflict` degrades to `remotely-modified`, and the table pulls straight over the
user's local edits — no conflict, no prompt, no report. AC 10 does not catch it: it
checks the timestamp only against files the run *mutated*, and a skipped file was
not mutated.

⚠️ Withholding `finalise` whenever *any* item is unreconciled is also wrong,
because `remote-absent` and `indeterminate` are **sticky**: nothing a later sync does
clears them while the local file keeps its stale `external_id`. One deleted remote
issue would then freeze the global timestamp permanently and disable the pre-filter
corpus-wide — every run re-hashing every file, defeating the lazy `ItemDigests` port,
its short-circuit fixture rows and the Performance section that exist to preserve it.

Blanking the affected `local_hash` targets the actual hazard (a stale hash the
pre-filter would trust) rather than the run's overall tidiness, so sticky states cost
nothing and `unsynced` items — which have no reconcilable local side — do not block
the advance either.

#### 3. Provider selection

**File**: `cli/work-cli/src/tracker_registry.rs` (new)

```rust
pub enum SelectionError {
    Unset,
    Unrecognised { name: String },
    NotAvailable { name: String },
}

pub trait TrackerRegistry {
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError>;
}

pub struct ConfiguredTrackers;
```

`Box`, not `&dyn`. A borrowed trait object requires the registry to already own a
constructed client for **every** provider it can resolve — so resolving `linear`
would construct and authenticate a Jira client too, or need interior mutability plus
an arena retrofitted later. The port's own docs describe the composition as holding
`Box<dyn RemoteTracker>` (`cli/tracker/src/lib.rs:238-248`). This compiles today only
because every arm errors, so the shape would be discovered under 0171 — the seam
changing signature exactly when it is meant to be stable, taking its fake-backed
tests with it.

`ConfiguredTrackers` maps `linear` and `jira` to `SelectionError::NotAvailable`
(72) — 0171 replaces those arms with real clients — and everything else to
`Unrecognised` (73) or, when the key is absent or empty, `Unset` (also 73). Never a
silent default.

`Unset` is a separate variant from `Unrecognised` even though both exit 73: they are
different mistakes with different fixes, and until 0171 lands these three errors are
the *entire* observable behaviour of this command. The messages are therefore part of
the deliverable and are asserted in the command tests, not left to the implementer:

- `Unset` — names `work.integration`, lists the recognised set, and points at
  `/accelerator:configure`. The bash path has an exemplary what/why/fix block for
  this case (`sync-work-items/SKILL.md` Step 0); a bare code would be a regression.
- `Unrecognised` — echoes the offending value and lists the recognised set.
- `NotAvailable` — says the tracker is recognised but its client is **not built
  yet**, so it reads as unfinished rather than broken.

Tests inject a registry of boxed `RecordingTracker`s under both `jira` and `linear`,
which is what AC 23 asks for: the seam, not the clients.

#### 4. The command surface

**File**: `cli/work-cli/src/cli.rs`
**Changes**: a `Sync` variant.

```rust
/// Reconcile local work items with the configured remote tracker.
Sync(Box<SyncArgs>),

#[derive(Args)]
pub struct SyncArgs {
    /// Push local changes only; never write a local file.
    #[arg(long, conflicts_with = "pull_only")]
    pub push_only: bool,
    /// Pull remote changes only; never write to the remote.
    #[arg(long)]
    pub pull_only: bool,
    /// Report the actions a run would take without performing any of
    /// them. Remote reads still occur; no create, update or local write
    /// does.
    #[arg(long)]
    pub preview: bool,
    /// An `<id>=<remote|local|skip>` resolution for a reported conflict;
    /// repeatable. An unrecognised token skips.
    #[arg(long = "resolve", value_parser = parse_key_value)]
    pub resolutions: Vec<(String, String)>,
    /// Read each item with its own request instead of one bulk retrieval.
    #[arg(long)]
    pub per_item_reads: bool,
    /// Refuse the run if it would overwrite more than this many local
    /// files from the remote. 0 refuses every pull.
    #[arg(long, default_value_t = 25)]
    pub max_pulls: usize,
    /// Refuse the run if it would replace more than this many remote
    /// issues. 0 refuses every push.
    #[arg(long, default_value_t = 25)]
    pub max_pushes: usize,
}
```

`conflicts_with` gives clap the mutual exclusion `_wisd_mode` enforces with exit
2 (`work-item-sync-decide.sh:68-71`). Clap's own conflict exit code is 2, which
matches.

Three help-text choices, each fixing a way the obvious wording misleads.
`--preview` states that remote reads still happen — "preview" otherwise reads as
"offline", and planning deliberately issues `fetch_all`/`show` (which is why it
counts against rate limits). `--per-item-reads` names what is per-item; bare
`--per-item` sits beside three flags that all describe outcomes and does not say it
selects a read strategy. `--resolve` documents `skip` and the unrecognised-token
behaviour, both of which the resolver accepts and the original help hid.

**File**: `cli/work-cli/src/sync.rs` (new)
**File**: `cli/work-cli/src/main.rs`
**Changes**: extract the composition preamble **first**, then add the `run_sync`
arm. `main.rs` already repeats the same `current_dir()` +
`compose(&start, LegacyPolicy::Reject)` + error-print block in five `run_*`
functions; this phase would make it six and Phase 5 seven, at which point the next
change to composition or its error reporting is a seven-site edit with an easy miss.

One `fn composed_context() -> Result<Context, ExitCode>` returning the start dir,
config service, `FileCorpusStore` (for `AtomicWrite` and `FileReader`) and the
`SystemRunClock`, so every `run_*` arm opens with a single `?` and both new call
sites are one line each.

#### 5. The report contract and exit codes

**File**: `cli/work-cli/src/sync.rs`
**Changes**: stdout is machine-parseable and carries nothing else; the human
summary goes to stderr, where it can change freely.

One tab-separated line per **reported item**, at four fields always:

```text
<id>\t<action>\t<state>\t<detail>
```

`<action>` is the decision keyword — `push`, `pull`, `skip-conflict`, `skip-dirty`,
`unresolved`, `failed` — `<state>` the classification keyword, and `<detail>` either
`retryable`/`terminal` on a `failed` line or `-` everywhere else. Identical in
**both** modes, so a preview line and a real-run line for the same item are
byte-identical and AC 9 is a set comparison.

```text
0194\tunresolved\tconflict\t-
0195\tpush\tlocally-modified\t-
0196\tnoop\tindeterminate\t-
0197\tfailed\tlocally-modified\tretryable
```

Lines are ordered by **ascending work-item id**, and that ordering is part of the
contract.

⚠️ Without a stated order the golden is byte-compared against a sequence derived
from corpus enumeration, which is filesystem- and platform-dependent — a flake across
the darwin and linux CI lanes, or an accidental order that 0171's consumer starts
depending on. Ascending id also matches the baseline's own `BTreeMap` ordering, so a
diff between the two reads straight across.

**Every non-`synced` item gets a line**, including `noop\tindeterminate` and
`noop\tremote-absent`. Reporting only actioned items makes a total `fetch_all`
failure — which marks every id `Indeterminate`, which decides `Noop` — produce empty
stdout and exit 0: byte-identical to a corpus already fully in sync, on a run that
read nothing. The bash SKILL reports those items under `needs-retry` and
`remote-absent` (`sync-work-items/SKILL.md:168-170`).

`noop\tsynced` items are **not** listed individually; a single trailing line gives
their count, at the same arity and with the marker in field 1:

```text
#\tsummary\tsynced\t142
```

A healthy several-hundred-item corpus would otherwise print several hundred
uninteresting lines on every run, and the eventual consumer reads stdout into a model
conversation where length is a real cost. The outage property is untouched — an
all-`indeterminate` run emits N lines and a zero count, a clean run emits none and a
full count, so the two remain trivially distinguishable.

Genuinely fixed arity, with `-` as the no-detail placeholder. A fourth field on
`failed` lines only would force a consumer to branch on field count before reading
the safety-critical class — the shape the third field was made unconditional to
avoid.

The retryable/terminal split is the safety-critical distinction in this domain —
`EXIT_CODES.md` says 71 is *not* safe to auto-retry — and this plan ports it into
`for_tracker_error` only to discard it if `failed` carries no class. The
`TrackerError.detail` prose still goes to stderr; the class goes on the wire.

A baseline degradation prints a named warning on **stderr**, leaving stdout to the
contract above. Phase 5 adds the outstanding-marker warning on the same stream, once
`pending_push` exists to enumerate.

Exit codes, named in `exit_codes.rs` and mapped by one
`fn exit_code(result: &Result<RunReport, RunError>) -> u8` the tests assert against,
rather than chosen per match arm:

| Code | Meaning | Rank |
|---|---|---|
| 0 | clean; every item `synced`, pushed or pulled | 0 |
| 4 | items awaiting human action: `unresolved`, `skip-conflict`, `skip-dirty`, `remote-absent` or `indeterminate` | 1 |
| 70 | a read failed, or every per-item failure was retryable | 2 |
| 71 | any per-item failure was terminal | 3 |
| 1 | internal error | terminal |
| 2 | usage: mutually exclusive flags, malformed or duplicate `--resolve` | terminal |
| 5 | refused: would exceed `--max-pulls` or `--max-pushes`; zero writes | terminal |
| 72 | tracker recognised, no client wired | terminal |
| 73 | `work.integration` unset or unknown | terminal |

The ranked codes form **one total order — 71 > 4 > 70 > 0** — and `exit_code` returns
the highest rank present. Terminal codes are decided before any item is applied and
cannot co-occur with a ranked one.

Three corrections over the obvious table, each of which was wrong in a way a caller
would feel:

**4 covers every item awaiting a human, not just conflicts.** `skip-dirty` is neither
a conflict nor a failure, so a table scoping 4 to the `Prompt` subset leaves it
mapping nowhere while row 0 excludes it — `exit_code` unimplementable. `unresolved()`
widens to match, so "clean" and the report's notion of unclean are one predicate.

**4 beats 70, and 71 beats 4.** A run with one conflict and one retryable failure must
not exit 70: a SKILL branching on status would read "safe to retry" and re-invoke
indefinitely while the conflict, which only a human can clear, is never surfaced —
defeating the very requirement that the exit code distinguish "nothing to resolve" from
"resolution needed". A conflict needs a human before a retry helps. 71 outranks 4
because a terminal failure means a remote mutation of unknown outcome, which a human
must inspect *before* resolving anything else.

⚠️ The exit code is a lossy summary of a multi-item run, and the report is
authoritative. `after_help` and `EXIT_CODES.md` both say so: a caller must check for
`unresolved` lines regardless of status, because a 71 run may also carry conflicts.

**A `fetch_all` pre-flight failure is 70, not 1.** `EXIT_CODES.md` already defines a
read-bridge failure as 70, "safe to retry", and the taxonomy is reused everywhere
else in this command; mapping the most common transient failure to generic 1 would
strand a caller that can otherwise tell "the tracker blinked, run me again" from a
usage error. Its `detail` still goes to stderr. 1 is reserved for genuine internal
errors.

5, 72 and 73 are **terminal and mutually exclusive** with the others rather than
ranked among them: each is decided before any item is applied, so no per-item outcome
can coexist with them. Ranking them invites siting the refusal check where the ranking
would be meaningful — after applying — which destroys its zero-write guarantee.

4 rather than 3: `accelerator work resolve` already returns 3 for
`E_RESOLVE_NOT_FOUND` (`main.rs:71-74`).

`--resolve` handling:

- Parse `<id>=<token>`. A missing `=` is **exit 2**, not 1: `parse_key_value`
  returns `Err(String)` to clap, which surfaces it as a usage error — the same code
  `conflicts_with` produces, and the code `--set`/`--append`/`--remove` already give
  for the same malformed pair. Exit 1 for this case is unreachable as declared.
- Two orders for the same id is **exit 2** — contradictory orders for one item are a
  caller bug with a destructive consequence either way, so this is not last-wins,
  and it belongs with the other bad-input codes rather than on its own.
- The token resolves through `work::sync::decide::resolve_conflict_token`. `None`
  becomes `Skip`, so the safe default is never a destructive write — **and a warning
  on stderr names the token and the accepted set**. Without it, `--resolve
  0194=remotee` produces stdout, stderr and an exit code identical to passing no
  order at all, so a caller looping until exit 0 re-invokes forever with the same
  typo and a human concludes resolution is broken.
- An order naming an id the run did not classify as needing resolution is
  **ignored, with a warning on stderr**, and does not change the exit code — a
  SKILL may legitimately carry a stale order after the conflict resolved itself,
  and applying it blindly would write an item the table says `noop`.

**File**: `cli/work-cli/src/cli.rs`
**Changes**: the exit-code table goes in `sync`'s clap `after_help`, following
`migrate-cli/src/cli.rs`'s Environment-section precedent. It is then frozen by
`cli_surface.golden` and, more to the point, discoverable by the person writing the
consumer — who reads `--help` first and would otherwise find nothing about the one
code they must not misread (4 is "resolution needed", not an error).

**File**: `skills/work/scripts/EXIT_CODES.md`
**Changes**: ⚠️ this file's preamble scopes it to `skills/work/scripts/` helpers whose
codes are `readonly E_*=NN` constants in the owning script, with the document
*derived*. A Rust binary satisfies neither clause. So the same change amends the
preamble to state that the file now documents both the bash helpers and the
`accelerator work` binary, naming `cli/work-cli/src/exit_codes.rs` as the second
source of truth. Documentation of the binary's surface either way, not a repoint — no
script reference changes, so AC 25 stays green.

#### 6. Freeze the surface

**File**: `cli/work-cli/tests/cli_surface.rs`
**Changes**: add `"sync"` to `SUBCOMMANDS`.

**File**: `cli/work-cli/tests/fixtures/cli_surface.golden`
**Changes**: regenerate after reading the diff. The whole surface is frozen
(`cli_surface.rs:32-56`), so this is a deliberate update.

#### 7. Command-level tests

**File**: `cli/work-adapters/tests/sync_fetch.rs` (new)

- `fetch_all` returning `Err` marks every id `Indeterminate`, writes nothing, and
  carries the `TrackerError` through to the outcome. This is the riskiest error path
  in the story — a bulk read failure must never be mistaken for absence, or the
  engine pushes and pulls on the strength of a network blip — and it belongs at unit
  level, where a break costs seconds to localise rather than minutes.
- A `found` stamp that proves unchanged costs no `show`; one that does not costs
  exactly one.
- `absent` → `RemoteAbsent`; `indeterminate` → `Indeterminate`.
- A `WorkingCopyStatus` probe failure yields `Dirtiness::Unknown`, which decides as
  dirty.

**File**: `cli/work-cli/tests/fixtures/sync-report.golden` (new)
**File**: `cli/work-cli/tests/cli_sync.rs` (new)

- **The report shape**: over a corpus spanning every classified state, stdout is
  byte-compared against the committed golden — including the third field on every
  line, the fourth on `failed`, the `noop\tindeterminate` lines, and that stdout
  carries nothing else. This is a cross-process contract whose consumer cannot be
  refactored in step with the binary; the CLI surface is frozen by a golden and the
  public APIs by cargo-public-api, so the report should not be the one surface
  pinned by nothing.
- **Non-interactivity**: run a bidirectional sync over a conflicting corpus with
  **stdin closed**, and assert it neither blocks nor reads stdin, exiting **exactly
  4** — not merely "does not fail", which is ambiguous against an intended non-zero
  exit.
- **Exit codes**: exactly 4 for one unresolved conflict; exactly 5 with zero writes
  when the plan exceeds `--max-pulls`; exactly 70 when every per-item failure is
  retryable and 71 when any is terminal; exactly 70 on a `fetch_all` pre-flight
  failure with its `detail` on stderr, and exactly 1 only for a genuine internal
  error; exactly 4 for a run whose only anomaly is a `remote-absent` or
  `indeterminate` item; and 71 over 4 when a run carries both a terminal failure and
  a conflict.
- **The write bounds**, including the boundary comparison: 26 pulls under the default
  refuses; **exactly 25 proceeds**; 25 under `--max-pulls 24` refuses; `--max-pulls 0`
  refuses a single pull. The mirror set for `--max-pushes`. A refusal writes no local
  file, touches no baseline entry, makes no remote call, and names the count, the
  limit and the raising flag. `--preview` over an over-threshold plan reports the
  refusal rather than exiting 0.

  ⚠️ Without the exactly-at-the-limit cases, `>` and `>=` are indistinguishable, and
  an accidental `>=` turns a routine 25-pull run — bash's own threshold — into a hard
  exit 5.
- **Baseline degradation, on all three paths**: a conflict-markered document warns on
  stderr naming the path, leaves the file **untouched**, and succeeds; a document with
  one malformed entry retains every other entry and names exactly that id in
  `EntriesDiscarded`; a document whose `timestamp` is a string retains every entry and
  reads the timestamp as 0.
- **Warnings**: an unrecognised `--resolve` token warns on stderr naming the token
  and the accepted set, and still skips; a stale order warns and does not change the
  exit code; a duplicate order for one id exits 2; a malformed order exits 2.
- **Selection messages**: `work.integration` unset exits 73 and its stderr names the
  key, the recognised set and `/accelerator:configure`; an unknown value exits 73 and
  echoes the value; `trello` exits 72 and says the client is not built yet.
- **The two-invocation loop**: over a two-conflict corpus, parse the report from
  the first run's stdout, construct one `--resolve` order per reported id, and
  assert a second invocation applies exactly those orders. No SKILL, no stdin.
- **Preview, three observables**: over a set spanning every classified state,
  assert the baseline file is byte-identical before and after, every work item
  file's content **and mtime** are unchanged, and the fake records zero `create`
  and zero `update` calls.
- **Preview fidelity**: a real run immediately after the preview emits the same
  stdout lines.
- **Call counts**: bulk mode records exactly one `fetch_all` and at most one
  `show` per item whose stamp moved — zero when nothing changed; `--per-item-reads`
  records exactly N `show` calls and no `fetch_all`.
- **Timestamp**: a run that fails partway leaves the global timestamp at its
  pre-run value; a run that reports a conflict **does** finalise, having blanked that
  item's `local_hash`, **and a following run still detects its local modification**; a
  run containing a sticky `remote-absent` or `indeterminate` item also finalises, so
  the timestamp is never frozen; a `skip-dirty` item's baseline entry is
  **byte-identical** after the run and still classifies `remotely-modified` next run
  (and `pull` once the file is clean); a corpus-wide `Dirtiness::Unknown` run mutates
  no `local_hash`; a run whose blank write fails leaves the timestamp untouched; and a
  clean run persists the injected clock's run-start epoch, and it is strictly earlier than the mtime of every file the run
  mutated. Derive the injected epoch from the fixture files' own mtimes rather than
  from an arbitrarily early value — with a clock far below every real mtime the
  ordering assertion passes unconditionally and proves the fixture, not the property.
  The equal-second boundary (`mtime == baseline_timestamp`, which the inclusive
  pre-filter treats as unchanged) is pinned by its own row in Phase 2's classify
  table.
- **Baseline degradation**: a missing baseline and a VCS-conflict-markered one
  both read as empty and succeed.
- **Directional forbidden-write cells**, end to end rather than only at the
  decide unit: `--push-only` over a `conflict` and a `remotely-modified` item
  writes no local file; `--pull-only` over a `locally-modified` item makes no
  remote call.
- **The dirty-pull cells**, end to end: a `remotely-modified` item whose local file
  is dirty is byte-identical after a bidirectional run (routed to `unresolved`) and
  after a `--pull-only` run (routed to `skip-dirty`); the same holds when the status
  probe fails and dirtiness is `Unknown`. This is the cell that protects uncommitted
  work, which VCS revert cannot recover.
- **Provider selection**: a fake registry under `jira` and under `linear`
  resolves; unset exits 73; `trello` exits 72.
- **Classification stability, end to end**: load Phase 3's bash-generated corpus into
  a real `last-sync.json`, point a `RecordingTracker` at the matching `remote.json`
  records, run bidirectionally, and assert **every item classifies `synced`** with
  zero `create` and zero `update` calls. This is what AC 24 actually asks for; the
  Phase 3 corpus test proves digest equality, which is necessary but not sufficient —
  a wrong branch order, a `==` where `proves_unchanged_since` belongs, or a read-back
  bug leaves every digest identical and still mass-reclassifies a real user's corpus.

### Success Criteria

#### Automated Verification

- [ ] Command tests including the closed-stdin and two-invocation cases pass: `mise run test:unit:cli`
- [ ] The report golden matches byte for byte: `mise run test:unit:cli`
- [ ] Fetch-shell unit tests including the `fetch_all`-error branch pass: `mise run test:unit:cli`
- [ ] The end-to-end classification-stability case passes: `mise run test:unit:cli`
- [ ] The CLI surface golden matches after a deliberate update, including
      `after_help`: `mise run test:unit:cli`
- [ ] The bash suites still pass unchanged: `mise run test:integration:work`
- [ ] Every SKILL `!`-site still resolves: `mise run test:integration:skill-invocation`
- [ ] Format, lint and types: `mise run cli:check`
- [ ] Whole tree green: `mise run`

#### Manual Verification

- [ ] `accelerator work sync --help` reads as orders-not-questions; no flag
      implies a prompt; `--preview` says remote reads still occur; the exit-code
      table is present in `after_help`
- [ ] `accelerator work sync` in this repo (`work.integration` set) exits 72 with
      a message saying the client is not built yet — not a panic, not a silent
      success, and not something that reads as a broken tool
- [ ] With `work.integration` unset, the message names the key, the recognised set
      and `/accelerator:configure`
- [ ] `/sync-work-items` still runs the bash path end to end and touches no
      binary

---

## Phase 5: `--push` wiring onto `create` and `update`

### Overview

`--push` on 0170's two write commands, the pending-push marker that keeps
`create --push` retries idempotent, and the retryable-versus-terminal split on
`update --push`.

### Changes Required

#### 1. The pending-push marker

**File**: `cli/work-adapters/src/sync/pending_push.rs` (new)

```rust
pub enum PendingPush {
    /// A create was attempted; its outcome is unknown.
    Attempted { request: RequestFingerprint },
    /// A create succeeded; the local write had not happened yet.
    Created {
        request: RequestFingerprint,
        external_id: ExternalId,
    },
}

pub struct RequestFingerprint {
    pub title: String,
    pub digest: String,
    pub attempted_at: u64,
    pub failure: Option<String>,
}

pub fn path(integrations_dir: &Path, integration: &str, slug: &str) -> PathBuf;

pub fn read(content: Option<&str>)
    -> Result<Option<PendingPush>, MarkerError>;

#[must_use]
pub fn render(marker: &PendingPush) -> String;
```

Keyed by `work::create`'s own `slugify(title)` rather than by the allocated id. The
id is not stable across runs — a failed `create --push` still writes the file
(AC 12), consuming that id, so a re-run would allocate the next one and an id-keyed
marker would never match. The slug is the identity of the work item being created,
and it is available before allocation.

An enum rather than `Option<String>`: the two states have radically different
consequences, and the flow branches on them.

`digest` is `sha256` over the three request fields **length-prefixed**, not
concatenated: undelimited `title + body + kind` is not injective, so `("ab", "c", k)`
and `("a", "bc", k)` collide. That is the same non-injectivity this plan documents as
a latent flaw in `digest::local` — but there it is forced by bash parity, whereas this
digest is private to the marker and framing costs nothing. And here the whole job of
the value is proving two requests are the *same* before adopting a remote id.

`attempted_at` and `failure` exist so a stale marker can be triaged: the only
question that matters before retrying a non-idempotent `create` is whether a remote
issue actually exists, and a marker holding just a title answers nothing.

The marker is written through `corpus::AtomicWrite`, so a crash cannot leave a torn
one.

The flow, satisfying ACs 12 and 13 together:

**File**: `cli/work/src/sync/push_precondition.rs` (new)
**Changes**: the decision, as a pure function beside `push_decide`.

```rust
pub enum MarkerState<'a> {
    Absent,
    Unreadable,
    Present(&'a PendingPush),
}

pub enum RefusalReason {
    MarkerUnreadable,
    PriorAttemptUnknownOutcome,
    FingerprintMismatch,
    AlreadyWritten,
}

pub enum PushPrecondition {
    Proceed,
    ReuseId(ExternalId),
    Refuse(RefusalReason),
}

#[must_use]
pub fn push_precondition(
    marker: &MarkerState<'_>,
    request_digest: &str,
    corpus_carries: &dyn Fn(&ExternalId) -> bool,
) -> PushPrecondition;
```

⚠️ A five-branch table whose wrong branch binds two local work items to one remote issue
does not belong in `create.rs`. The plan's own argument for siting `push_decide` in the
domain — "the same kind of pure table … rather than in the binary crate where it is
hardest to test" — applies with more force here, because this table's failure mode is
remote data loss that neither revert nor a re-run recovers. `create.rs` executes the
returned decision; the six marker tests drive the function directly, without the binary.
`PendingPush` and `RequestFingerprint` move into `work::sync` with it, leaving
`path`/`read`/`render` (the JSON and the filesystem) in `work-adapters`.

The flow `create --push` then executes:

1. Compute `slug` from the title. Read `pending-push/<slug>.json`.
   - `Created` **whose `digest` matches this request**, and no work item on disk
     carries that `external_id` → the previous run's `create` succeeded and died before
     the local write. Reuse that id, skip the remote call, continue at step 4. Stdout is
     line 1 the written path, line 2 `write-once\t<id>` — the item was written, just
     with a reused id.
   - `Created` whose `digest` matches **and** a work item on disk already carries that
     `external_id` → **refuse**, naming both that item's path and the marker path.
   - `Created` whose `digest` does **not** match → refuse with a named error naming
     the marker path. **Never** adopt the id.
   - `Attempted` → the previous run's `create` failed terminally or was interrupted;
     a remote issue may exist. Report, naming the path, the recorded timestamp and
     the failure detail, and exit non-zero. **No second `create` call.**
   - Unparseable → treat as `Attempted` with an unknown outcome. Report and refuse.
2. Write `Attempted` with the fingerprint.
3. Call `tracker.create(title, body, kind)`.
   - `Ok(id)` → rewrite as `Created` carrying `id`.
   - `Err(Retryable)` → provably no mutation, so **delete** the marker, write the
     file without `external_id`, report the `retry` (attempt 1) or `local-save`
     row, exit 0.
   - `Err(Terminal)` → **retain** the marker as `Attempted` with the failure
     detail, write the file without `external_id`, report the `loud-terminal` row,
     exit 71.
4. Allocate the id, write exactly one file with `external_id` already
   substituted, then delete the marker.

⚠️ Two distinct hazards, and the fingerprint closes only the first.

Step 1's fingerprint check stops the reuse branch binding **two local work items to
one remote issue** across *different* requests. Two genuinely different items sharing
a title share a slug — `Fix flaky test`, `Update docs` — so without it, a `Created`
marker left by run A is silently adopted by run B, and the next `sync` pushes both
items over that one issue through the whole-content `update` contract.

Step 1's **corpus check** closes the same hazard for the *same* request. A crash
between step 4's local write and its marker delete leaves a `Created` marker whose
fingerprint still matches, so the fingerprint guard passes — and without the check the
re-run would allocate a *fresh* work-item number and write a second file carrying that
same `external_id`. Both are remote data loss that neither VCS revert nor a re-run
recovers.

⚠️ The corpus check **refuses**; it does not quietly succeed. An earlier revision had it
delete the marker and exit 0 "having changed nothing", which is wrong for the case the
fingerprint cannot distinguish: two deliberate creates with identical title, body and
kind — a templated or recurring item — are *indistinguishable requests*, so a genuine
second create would find the first item, exit 0, and create nothing, reporting success
for an item that does not exist. Refusing and naming both paths hands the operator the
one decision only they can make: accept the existing item, or clear the marker to create
a same-titled sibling. Fail-closed costs a manual step; failing open loses a requested
item silently.

⚠️ `read` returns `Result`, and unparseable content fails **closed**. Conflating "no
marker" with "torn marker" means a crash mid-write — exactly what the marker exists
to survive — reads as "no previous attempt" and issues a second `create` against a
non-idempotent operation, defeating AC 13 in its own scenario.

⚠️ A retained marker blocks that title until an operator clears it. The error names
the marker path, the recorded `attempted_at`, the recorded failure detail and, where
known, the `external_id` — everything needed to decide whether a remote issue exists
before retrying.

**File**: `cli/work-adapters/src/sync/pending_push.rs`
**Changes**: `pub fn outstanding(integrations_dir: &Path, integration: &str) ->
Result<Vec<(PathBuf, PendingPush)>, MarkerError>` — enumerate `pending-push/*.json`.

**File**: `cli/work-cli/src/sync.rs`
**Changes**: call `outstanding` after the run and print one named warning per marker on
**stderr** — path, `attempted_at`, failure detail, `external_id` where known. Stdout
stays the fixed-arity report with nothing else in it, so no marker line class and no
report-golden change is implied.

⚠️ This is the *only* way an operator learns a marker is blocking a title, because no
SKILL invokes the binary in this story. An earlier revision asserted the behaviour in
Phase 5's prose while the change site sat in Phase 4 — before `pending_push` exists to
enumerate — so it belonged to neither phase and no test covered it.

**File**: `.gitignore`
**Changes**: ignore the pending-push marker directory. Markers are per-checkout,
transient local state, and nothing currently ignores `meta/integrations/` — so under
jj's auto-snapshot a marker is committed the moment it is written, wedging every
collaborator's checkout on a title only one of them was pushing. This is an explicit
file change, not an assertion: Phase 5's manual verification confirms a written
marker does not appear in `jj status`.

The outcome rows reported are `work-item-push-decide.sh`'s vocabulary
(`write-once` / `retry` / `local-save` / `loud-terminal`), ported to
`work::sync::push_decide` — beside `decide`, since it is the same kind of pure table
over (code × attempt × write-failed) and belongs in the domain rather than in the
binary crate where it is hardest to test — and asserted against
`skills/work/scripts/test-fixtures/work-item-push-decide.golden`, transcribed from the
characterization rows phase 1 added. The binary is non-interactive, so it never
performs the `retry` row — it reports it and exits, and the SKILL decides.

It takes the dispatcher code as a bare `u8`, not a named constant.
`work_domain_imports_only_permitted` forbids `work` from importing `work_cli`, where
`exit_codes.rs` lives, so a domain-sited table cannot reference `RETRYABLE`/`TERMINAL`
by name. A `u8` parameter is also what keeps the golden's unknown-code row (`99` →
`loud-terminal`, `work-item-push-decide.sh:104-107`) expressible at all — an enum
over the four known codes could not represent it. `work-cli` maps its constants to
the `u8` at the call site.

**`create --push`'s stdout and exit contract**, which the rest of this phase depends
on and which must not collide with `create`'s existing one. `create`'s stdout **is**
the written path today, and `create-work-item/SKILL.md` reads it as such. So:

```text
<path>
<row>\t<external_id-or-empty>
```

Line 1 is unchanged, so every existing consumer keeps working; line 2 carries the
outcome. Exit codes: `write-once` → 0, `retry` and `local-save` → 0 (the file was
written; the push was not), `loud-terminal` → 71.

The retryable case exits **0** here while `update --push`'s exits **70**, and
that asymmetry is deliberate: `create --push` has written a usable local file and
succeeded at its primary job, whereas `update --push` has done nothing at all. Both
`--help` texts state it, because the two halves of one feature otherwise look
inconsistent — and a caller distinguishing "pushed" from "saved unsynced" reads line
2, not the exit code.

#### 2. `create --push`

**File**: `cli/work-cli/src/cli.rs`
**Changes**: `#[arg(long)] pub push: bool` on `CreateArgs`.

**File**: `cli/work-cli/src/main.rs`
**Changes**: carry `push` through `create_args_from_cli` (`:172-190`), and
resolve the tracker from the registry when it is set.

**File**: `cli/work-cli/src/create.rs`
**Changes**: the push happens **before** the single atomic write at `:300-302`,
so a successful push yields exactly one file already carrying its remote
identity — never a write-then-amend.

#### 3. `update --push`

**File**: `cli/work-cli/src/cli.rs`
**Changes**: `#[arg(long)] pub push: bool` on `UpdateArgs`.

**File**: `cli/work-cli/src/main.rs`
**Changes**: ⚠️ `run_update` composes no config today (`:223-231`). `--push`
needs `work.integration`, the integrations path and a tracker, so the whole
`compose(&start, LegacyPolicy::Reject)` preamble is added — the largest single
edit in this phase, and the reason it is bigger than "add a flag".

**File**: `cli/work-cli/src/update.rs`
**Changes**: ordering is render → push → write local → baseline set. Pushing
before the local write is what makes "the local file is left untouched" on
failure true by construction rather than by rollback. A crash between the push
and the local write leaves the file stale and the baseline untouched, so the next
sync classifies `remotely-modified` and pulls — safe.

- Refuse with a named error when the target carries no non-empty `external_id`:
  `--push` replaces an existing remote issue, and `create --push` is the path for
  an unsynced one.
- `Err(Retryable)` → local file untouched, baseline entry untouched, exit 70.
- `Err(Terminal)` → local file untouched, baseline entry **cleared**, exit 71. A
  subsequent `sync` then classifies the item **`conflict`** and, bidirectionally,
  writes neither side.

`conflict`, not `indeterminate`. `Indeterminate` is reachable **only** from
`RemotePresence::Indeterminate` — a failed or truncated remote read — so no amount of
local bookkeeping can produce it: a cleared entry leaves both hashes absent, both
sides default to changed, and the 2×2 verdict is `conflict`. The safety property the
work item wanted is intact (neither side is written), and this needs no third on-disk
state store. Residual, stated rather than hidden: `conflict` is *resolvable*, so a
later `--resolve <id>=remote` will pull the remote over the local for an update that
may never have applied. The alternative — persisting an uncertainty flag the
classifier honours — was rejected because it widens the baseline schema the live bash
engine also reads, during the window both engines are live.
- `Ok(())` → write the local file, then post-push `show`, then the baseline entry
  last, through the same `ItemApplier::push` code path phase 3 built.

#### 4. Freeze the surface again

**File**: `cli/work-cli/tests/fixtures/cli_surface.golden`
**Changes**: regenerate for the two new flags, after reading the diff.

#### 5. Tests

**File**: `cli/work-cli/tests/cli_create_push.rs` (new)

- Success writes exactly one file with `external_id` substituted, no marker
  survives, and stdout is line 1 the path plus line 2 `write-once\t<id>`.
- A failed create writes the file without `external_id`, emits the outcome row on
  line 2, and exits without prompting and with stdin closed.
- **AC 13**: a terminal failure where the fake **did** create the issue
  (`creating_then_failing`), then a re-run for the same work item, records **zero**
  further `create` calls.
- A retryable failure deletes the marker, so a re-run is free to call `create`
  again.
- A crash between a successful `create` and the local write leaves a `Created`
  marker; the re-run reuses its id and calls `create` zero times.
- **The reuse guard**, with the title held **constant**: same title (hence same slug
  and same marker path), differing body — and separately differing kind — is refused
  with a named error, `create` is called zero times, and the marker's id is **not**
  adopted.

  ⚠️ Constructing the mismatch by varying the *title* would put the two requests on
  different marker paths, so the assertion would hold even against an implementation
  whose digest covers only the title — leaving the actual hazard (an ordinary title
  collision) untested by the test written to cover it.
- **The write-to-delete window**: a crash after the local write but before the marker
  delete leaves a `Created` marker whose fingerprint matches; the re-run finds the
  item already carrying that `external_id`, deletes the marker, calls `create` zero
  times, and writes **no** second file.
- **The delimited fingerprint**: two requests whose concatenated
  `title + body + kind` bytes coincide but whose fields differ produce different
  digests.
- **`push_precondition`'s five branches** as a unit test in `work`, driven directly:
  absent → `Proceed`; unreadable → `Refuse(MarkerUnreadable)`; `Attempted` →
  `Refuse(PriorAttemptUnknownOutcome)`; matching `Created` with the id absent from the
  corpus → `ReuseId`; matching `Created` with the id present → `Refuse(AlreadyWritten)`.
- **The already-written refusal**: a matching `Created` marker whose `external_id` is
  already carried by an item on disk refuses, names both paths, calls `create` zero
  times, and writes **no** second file.
- **The reuse branch's stdout**: line 1 the path, line 2 `write-once\t<id>`.
- **`sync`'s marker warning**: with one outstanding `Attempted` marker, `sync` names it
  on stderr and stdout remains exactly the fixed-arity report.
- **Fail-closed read**: a deliberately truncated marker is treated as an attempt of
  unknown outcome; the command refuses and calls `create` zero times.
- Exit codes: `write-once`, `retry` and `local-save` exit 0; `loud-terminal` exits
  71.

**File**: `cli/work-cli/tests/cli_update_push.rs` (new)

- A synced item's remote is replaced via the whole-content contract.
- `Retryable`: local file byte-identical, baseline entry intact, exit 70.
- `Terminal`: local file byte-identical, baseline entry **cleared**, exit 71,
  and a following bidirectional `sync` classifies the item **`conflict`** and writes
  neither side.
- An unsynced target is refused with a named error.

### Success Criteria

#### Automated Verification

- [ ] `create --push` and `update --push` tests including the idempotency case pass: `mise run test:unit:cli`
- [ ] The marker's reuse guard and fail-closed read pass: `mise run test:unit:cli`
- [ ] The push-decide port matches the shared table, both implementations:
      `mise run test:unit:cli` and `mise run test:integration:work`
- [ ] The CLI surface golden matches after a deliberate update: `mise run test:unit:cli`
- [ ] Every bash suite still passes and `_EXPECTED_WORK_SUITES` is unchanged: `mise run test:integration:work`
- [ ] Format, lint and types: `mise run cli:check`
- [ ] Whole tree green: `mise run`

#### Manual Verification

- [ ] A written marker does not appear in `jj status`
- [ ] Every script named in the work item is still present:
      `ls skills/work/scripts/work-item-*.sh`
- [ ] `/create-work-item` and `/sync-work-items` still drive bash only —
      `grep -rn "accelerator work sync\|work create --push" skills/` returns
      nothing
- [ ] `accelerator work create --push --help` and `update --push --help` state
      that failures are reported rather than prompted

---

## Testing Strategy

### Unit Tests

- **Classify**: the shared table plus eight rows bash lacks — `(NotReported,
  NotReported)` proving nothing, `NotRead` on either side, an entry with
  `remote_updated_at` and no `remote_hash`, an absent mtime, the
  `mtime == baseline_timestamp` boundary, and the two empty-string-hash shapes. Each
  row also asserts whether the mtime and each digest were called, which pins both
  short-circuits.
- **Decide**: the 7 states × 3 directions grid with `Dirtiness` three-valued
  (`Unknown` deciding as `Dirty`), plus the token cases — the interior-whitespace one
  (`remote foo`), the empty and `frobnicate` rows that prove the safe default, and
  the leading-U+00A0 row that catches a Unicode-aware trim.
- **Plan**: the branch table as a pure function — the `needs_body_read` gate, the
  partition mapping, each resolution override, stale-order rejection.
- **Label**: the shared golden, both implementations, including the two exit-1
  rows, the `bogus` row, the `[DEFAULT]` arm and the absent trailing newline.
- **Baseline**: round-trip, per-entry tolerance, whole-document degrade-to-empty from
  missing and from conflict-markered content with the degradation *reported*, the
  empty-string-hash-is-absent conversion, and the deliberate `NotReported` → `""` →
  `NotRead` loss.
- **Digest**: the bash-generated corpus including the empty-frontmatter, empty-body
  and numeric/control-character cases, plus the `bash-parity` shellout that proves the
  corpus itself came from bash.

### Integration Tests

- **Apply ordering** via a double that both records call order and retains bytes,
  asserting the baseline write's position in the call slice for every state.
- **Crash-and-resume** by failing the baseline write after a successful side effect,
  asserting the re-run's terminal state matches an uninterrupted run's.
- **The two-invocation conflict loop**, driven from the test harness: parse the
  report, build `--resolve` orders from it, re-invoke, assert.
- **The report contract** byte-compared against a committed golden, since its
  consumer sits across a process edge and cannot be refactored in step.
- **Classification stability end to end**: the bash-written corpus loaded as a real
  baseline, asserting every item classifies `synced` with zero writes.
- **The bulk-overwrite refusal**, asserting zero writes above the threshold.
- **The contract harness** against the fake in its conformance shape plus the two
  induced shapes, in its own filtered binary behind the opt-in env var.

### Manual Testing Steps

1. `mise run test:unit:cli` and confirm it runs `tracker-test-support`'s lib tests
   but no `contract` binary.
2. `mise run test:integration:tracker-contract` and confirm it runs the contract
   binaries alone; unset `ACCELERATOR_TRACKER_CONTRACT` and confirm it skips.
3. Run `/sync-work-items --preview` in this repo and confirm it still drives the
   bash scripts and touches no binary.
4. Run `accelerator work sync` directly and confirm exit 72 with a message saying
   the client is not built yet; unset `work.integration` and confirm exit 73 names
   the key, the recognised set and `/accelerator:configure`.
5. Regenerate the baseline corpus with `regenerate.sh` on a clean checkout and
   confirm it reproduces the committed fixtures byte for byte.
6. Break `digest::local`'s separator handling by one byte and confirm the
   empty-body corpus case reddens.

## Performance Considerations

Bulk-then-`show` bounds remote reads at one `fetch_all` plus one `show` per item
whose stamp moved, which is what makes a large corpus viable — a per-item mode
over N items is N round trips. `fetch_all` exposes a large corpus to per-tenant
rate limits, but only from 0171, since nothing here makes a live call.

The mtime pre-filter avoids reading and hashing unmodified files. Preserving it
is why the classifier takes a lazy `ItemDigests` port rather than pre-computed
hashes: an eager caller would pay for every file the filter exists to skip.

The baseline is read-modify-write over the whole document per item, inherited
from bash — re-read immediately before each render, matching bash's per-`set`
semantics rather than holding one in-memory document for the run. At the corpus sizes
in play (hundreds of items) the extra reads are not worth optimising away, and doing
so would widen the lost-update window from a single write to an entire run while both
implementations are live and two further writers (`update --push`'s set and its
terminal clear) share the file.

The bulk-overwrite refusal costs nothing: it reads the plan's `Pull` count, which
planning has already computed, before any write happens.

## Migration Notes

Nothing migrates. Both implementations run against one oracle until 0171's
cutover, and the user-facing path is unchanged throughout — no script removed,
no SKILL repointed, no entry point moved.

Obligations passing to 0171, each needing a matching acceptance criterion there:

- Running the contract harness against each real client — implementing
  `ContractSubject` for it, which means supplying a definitely-unknown id and a way to
  force a partial fetch. The nextest `default-filter` already excludes any
  `tests/contract.rs` by binary name, so no new exclusion mechanism is needed.
- Reproducing the jira and linear projection recipes exactly, gated by the baseline
  corpus this story commits — including its numeric and control-character cases,
  which are where jq and `serde_json` diverge.
- Reinstating the *interactive* half of the pull-overwrite gate if wanted: this story
  ships the fail-closed refusal, and a SKILL that asks the question and re-invokes
  with `--max-pulls` is the conversational counterpart.
- The cutover itself — removing the nine migrated scripts, repointing
  `sync-work-items`, `create-work-item`, `list-work-items` and `EXIT_CODES.md`,
  adding the conversational conflict flow, and decrementing
  `_EXPECTED_WORK_SUITES`. Note the report keyword `unresolved` is what a SKILL
  parses, while bash prints `prompt` and the enum variant is `Prompt`; the wire
  vocabulary is owned by `Action`'s `Display` impl, so read it there rather than
  inferring the mapping.

One thing 0171 must not lose: `list-work-items/SKILL.md:316-319` short-circuits
**before** the classifier — no `external_id` is presence-only `unsynced`, and an
`external_id` with an empty baseline entry is presence-only `synced`. Sync has no
such carve-out, which is exactly why first-sync-on-dirty reaches `conflict` only
from sync. The asymmetry is deliberate.

## References

- Work item: `meta/work/0194-tracker-crate-and-remote-sync-engine.md`
- Research: `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- Port: `meta/plans/2026-08-11-0204-remote-tracker-port.md`, `cli/tracker/src/lib.rs`
- Port validation, for the inherited default-bodied-method gap:
  `meta/validations/2026-08-11-0204-remote-tracker-port-validation.md:117-121`
- Registration checklist: `tasks/README.md#registering-a-library-crate`
- Decisions: ADR-0045 (skills versus CLI division of labour), ADR-0053 (thin CLI
  over a hexagonal core), ADR-0044 (remote identity in `external_id`)
