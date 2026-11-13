---
type: "plan-review"
id: "2026-11-13-shortcut-integration-review-1"
title: "Plan Review: Shortcut Integration Implementation Plan"
date: "2026-11-13T18:29:45+00:00"
author: "Tom Scott"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-13-shortcut-integration"
reviewer: "Tom Scott"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "compatibility", "usability", "standards"]
review_number: 1
review_pass: 4
tags: ["integrations", "shortcut", "tracker"]
last_updated: "2026-11-13T22:14:43+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---

## Plan Review: Shortcut Integration Implementation Plan

**Verdict:** REVISE

The plan has the right shape. Every lens praised the `ShortcutKey` domain
type, the phase order and the cargo-pup boundaries, and every lens found that
each phase merges on its own. But one rule breaks the port contract, and three
designs are underspecified. The failure-classification table marks provably
rejected writes as `Terminal`. The catalogue bootstraps itself inside the
client constructor. Live evidence and test fixtures are linked in a way the
existing evidence guard forbids. And the team identity rests on a nullable
mention name. The plan also contains five factual slips that the reviewers
checked against the code.

### Cross-Cutting Themes

- **Write failures are misclassified** (flagged by: correctness, code-quality,
  architecture) — The table maps 401, 403, 400/422 and an exhausted 429 on a
  write to `Terminal`. The port (`cli/tracker/src/lib.rs:685-711`) and
  `jira-client/src/classify.rs:57-72` class a provable rejection as
  `Retryable`. They keep `Terminal` for outcomes that may have landed, and
  `Rejected` for refusals before send. Under the plan's table, a read-only
  token turns every push into `MayHaveLanded`.
- **The catalogue design is underspecified and diverges from Linear**
  (flagged by: architecture, correctness, usability, code-quality, security)
  — `from_config` makes network calls and writes a committed file. Work item
  0336 (lines 158-159) deliberately kept that out of non-init commands. The
  "incomplete" marker has no reader. Bootstrap races between processes. Only
  default workflows are catalogued, so stories in other workflows cannot be
  transitioned. Slugs and ids are trusted without validation.
- **Live evidence cannot be committed fixtures** (flagged by: security,
  test-coverage, compatibility) — `tracker_test_support::evidence` and
  `evidence_hygiene` accept only reduced PASS/FAIL records. The plan wants raw
  bodies to become mock fixtures. The evidence list also leaves out most of
  the v4 request shapes Phase 3 depends on.
- **Team identity uses a nullable mention name** (flagged by: usability,
  architecture, correctness) — `shortcut.team_key`, `resolve_scope` and
  `enumerate_visible_entities` key on `mention_name`, which v4 allows to be
  null. Search scopes by a mutable display name.
- **The API-URL override sits in the wrong crate** (flagged by: standards,
  architecture) — Linear reads the override and the `test-loopback` gate in
  `linear-cli/src/context.rs` and passes loopback into the client as a
  parameter.
- **The restated sub-binary checklist is wrong** (flagged by: standards,
  code-quality) — It lists twelve items against the README's fourteen. It
  drops `_LOOPBACK_SEAMED`, the `test_github.py` pin, the skill binding and
  the `shortcut-cli` declaration, and lists steps the README marks
  no-action.
- **Integration-name match sites are incomplete** (flagged by: code-quality,
  architecture, compatibility) — The inventory misses `work-cli/src/main.rs:531`,
  the sync skill's tracker lists and the exit-code docs. It adds an unneeded
  `migrate-cli` → `shortcut-client` edge. The catch-all arms mean a missed site
  compiles and fails silently.

### Tradeoff Analysis

- **Time to first success vs. a read-only constructor**: usability values
  sync working without `init`, while architecture and correctness want
  construction to stay local and cheap. Recommendation: keep `from_config`
  read-only and fail `Unconfigured` with an `init-shortcut` hint. Bootstrap
  only in `init discover`, plus a healing step modelled on Linear's
  `CatalogueHealing` for workflows that sync finds missing.
- **Shortcut fit vs. Linear consistency**: usability wants `team_key` to
  accept a name, mention name or uuid, while standards prefers mirroring
  Linear. Recommendation: store the uuid in the catalogue, and accept the
  mention name or display name in `shortcut.team_key` as aliases resolved to
  it.

### Findings

#### Critical

- 🔴 **Correctness / Code Quality / Architecture**: Write-error table marks
  provably unapplied writes as Terminal
  **Location**: Phase 2, Section 6: Failure classification
  The engine turns `Terminal` on create into `OutcomeUnknown`/`MayHaveLanded`
  (`work-adapters/src/remote_create.rs:52`). Jira classifies these statuses as
  `Retryable` on writes. The plan also reuses `RequestInvalid` for a server
  400/422, though Jira uses that name for a refusal before send.

#### Major

- 🟡 **Architecture / Usability / Code Quality**: Client constructor performs
  network I/O and writes a committed file
  **Location**: Catalogue; Phase 2, Section 7
  This diverges from `LinearClient::from_config` and from 0336's explicit
  choice. Every `resolve` can then hit the network and change the working
  tree.
- 🟡 **Correctness / Architecture / Usability**: Catalogue covers only default
  workflows and has no refresh path
  **Location**: Catalogue; Phase 6, Section 1; Phase 3, Section 4
  Stories in other workflows cannot be transitioned. `transition` needs a read
  of the story's `workflow_id` first. State-filter validation is ambiguous
  across teams.
- 🟡 **Correctness**: Incomplete-catalogue and concurrent-bootstrap
  transitions are undefined
  **Location**: Phase 2, Section 7
  Nothing reads the incomplete marker. A check-then-write bootstrap can let a
  partial catalogue replace a complete one.
- 🟡 **Usability / Architecture / Correctness**: Team identity rests on a
  nullable mention name
  **Location**: Phase 1 §2; Phase 2 §3, §7; Phase 3 §3
  A team without a mention name cannot be configured. A rename silently
  changes the search scope.
- 🟡 **Security / Test Coverage**: The live smoke test plans to commit raw
  payloads as evidence
  **Location**: Phase 2, Section 8; Phase 3, Section 6
  The evidence module accepts only reduced records. Raw captures carry the
  slug, member identity and possibly token text. Fixtures need a separate,
  sanitised, hygiene-checked home.
- 🟡 **Compatibility / Correctness**: The smoke test leaves most v4 request
  shapes unverified
  **Location**: Phase 2, Section 8
  Unchecked: `?fields=`, `GET /stories?ids=&filter=include_archived`,
  `team:` search, `next_page_url` on teams and search, `external_links_add`,
  comments, and the quote escape.
- 🟡 **Compatibility**: No rule for tolerating new fields or enum values from
  a beta API
  **Location**: Phase 2, Section 7; Implementation Approach
  Closed enums or strict structs in the response and catalogue models could
  break every teammate's client when Shortcut adds a value.
- 🟡 **Correctness**: "Unparseable key is Terminal" violates the read and
  validate contracts
  **Location**: Implementation Approach: Shortcut keys
  Reads may only return `Retryable`. Writes refused before sending are
  `Rejected`. `validate_update_reports_outcome_property` passes `PREVIEW-1`
  and expects `Valid`.
- 🟡 **Correctness**: Running every `_property` with `moved_issue: None`
  panics
  **Location**: Phase 3, Section 6
  `contract.rs:430-432` calls `.expect()` on it, so the default profile would
  fail.
- 🟡 **Correctness**: Case canonicalisation breaks fetch_all pairing
  **Location**: Shortcut keys; Phase 3, Section 3
  A requested `SC-12` reported as `sc-12` breaks `partitions_totally`, and a
  `show` returning `sc-12` looks like a key change.
- 🟡 **Test Coverage**: The live contract suite is never wired for Shortcut
  **Location**: Phase 3 Manual Verification; Phase 2, Section 8
  No phase adds live story and epic `ContractSubject`s that call
  `run_all`/`timed_conformance`.
- 🟡 **Test Coverage**: The API-URL override allowlist has no tests
  **Location**: Phase 2, Section 5
  Lookalike hosts, http, userinfo and loopback without the feature are all
  untested.
- 🟡 **Test Coverage**: No test pins POST retry behaviour on 5xx and 429
  **Location**: Phase 2, Section 4
  Retrying a POST on 5xx would pass every listed test and create duplicates.
- 🟡 **Test Coverage**: fetch_all chunking, archived and fallback paths are
  untested
  **Location**: Phase 3, Section 3
  Untested cases: the 100/101 boundary, mixed story and epic batches,
  archived-found and partial pages.
- 🟡 **Test Coverage**: Write-side refusal paths are specified but not tested
  **Location**: Phase 6, Section 1
  Untested: a cross-workflow state, a non-https attach, `--file`, epic keys on
  story-only flags, epic comments, and the write-back after a failed create.
- 🟡 **Standards / Architecture**: The API-URL override and loopback gate are
  placed in the client
  **Location**: Phase 2, Section 5
  Linear reads them in its composition root, and the client takes loopback as
  a constructor parameter.
- 🟡 **Standards / Code Quality**: The restated sub-binary checklist does not
  match the README
  **Location**: Phase 5, Section 1
  Twelve items against fourteen, with omissions and wrong actions.
- 🟡 **Code Quality / Architecture**: The inventory of integration-name match
  sites is incomplete
  **Location**: Phase 4, Section 2
  It misses `main.rs:531` (the finaliser) and others. The catch-all arms
  compile and fail silently.

#### Minor

- 🔵 **Compatibility**: The skill-structure test pins the old key regex
  **Location**: Phase 1, Section 4
  `test_sync_work_items_skill_structure.py:268` asserts the uppercase regex.
  The widened gate also changes Jira and Linear behaviour.
- 🔵 **Compatibility**: Phase 1 expects exit 74, but `NotAvailable` is 72
  **Location**: Phase 1 Manual Verification
- 🔵 **Compatibility**: Tracker lists in the sync skill and the exit-code docs
  are missing from the change lists
  **Location**: Phase 1, Section 3; Phase 4, Section 2
- 🔵 **Compatibility**: The new `Integration` variant changes a pinned
  public-API snapshot
  **Location**: Phase 3, Section 1
- 🔵 **Architecture**: Unneeded `migrate-cli` → `shortcut-client` dependency
  **Location**: Phase 4, Section 2
  `PlainDialect` lives in `tracker`, and no repository holds pre-0008
  Shortcut baselines.
- 🔵 **Architecture / Code Quality**: A conditional `Integration::ShortcutEpic`
  would leak entity type into a per-integration enum
  **Location**: Phase 3, Section 1
- 🔵 **Code Quality**: Key-variant and kind dispatch risk repetition across
  call sites
  **Location**: Shortcut keys; Phase 3 §3; Phase 6 §1
- 🔵 **Code Quality**: No observability provisions
  **Location**: Phase 2, Section 4; Phase 3, Section 3
- 🔵 **Code Quality**: Evidence-contingent branches leave the module shape
  undecided
  **Location**: Phase 3, Sections 1 and 3
- 🔵 **Correctness**: The absence proof relies on unverified `?ids=`
  semantics
  **Location**: Phase 2 §8; Phase 3 §3
- 🔵 **Correctness**: The PATCH resend policy is unspecified, and
  `external_links_add` appends
  **Location**: Phase 2 §4; Phase 6 §1
- 🔵 **Security**: The evidence denylist has no `sct_` shape
  **Location**: Phase 2, Section 8
- 🔵 **Security**: The committed catalogue is trusted without validation
  **Location**: Catalogue; Phase 2, Section 5
- 🔵 **Security**: Search escaping relies on an unverified quote-escape
  grammar
  **Location**: Phase 3, Section 4
- 🔵 **Security**: The read skills do not mark remote content as untrusted
  **Location**: Phase 5, Section 5
- 🔵 **Usability**: The `epic-<n>` trigger will fire on incidental mentions
  **Location**: Phase 5, Section 5
- 🔵 **Usability**: Two vocabularies for kind and type, and `-story` skills
  that handle epics
  **Location**: Kind table; Phase 5 §2; Phase 6
- 🔵 **Usability**: Flag and default drift from the Linear CLI
  **Location**: Phase 5 §2; Phase 6 §1
- 🔵 **Usability**: The 401 and missing-token messages are unspecified
  **Location**: Phase 2, Section 6
- 🔵 **Standards**: Keyword parity needs `init-shortcut` in `_NOT_KEYWORDS`
  **Location**: Phase 5, Section 5
- 🔵 **Standards**: The `plugin.json` insertion point is unspecified
  **Location**: Phase 5, Section 5
- 🔵 **Standards**: The exit-code bands omit the init band, and the fixture
  name diverges
  **Location**: Phase 5, Section 3
- 🔵 **Standards**: The write operations have no home in the client crate
  **Location**: Phase 6, Section 1
- 🔵 **Test Coverage**: The classifier section has no test list
  **Location**: Phase 2, Section 6
- 🔵 **Test Coverage**: The create workflow-state choice is untested
  **Location**: Phase 3, Section 3
- 🔵 **Test Coverage**: The state-filter refusal and team composition are
  untested
  **Location**: Phase 3, Section 4
- 🔵 **Test Coverage**: The Shortcut arms in `resolve_active_scope_key` and
  `body_rules` are untested
  **Location**: Phase 4, Section 2
- 🔵 **Test Coverage**: The widened regex is tested only on the accept path
  **Location**: Phase 1, Section 4
- 🔵 **Test Coverage / Standards**: The flow-test list omits the inventory and
  diagnostics guards
  **Location**: Phase 5, Section 4

#### Suggestions

- 🔵 **Usability**: Accept Shortcut URLs as CLI input
- 🔵 **Usability**: Mark recognised-but-unwired values in the configure docs
- 🔵 **Security / Compatibility**: Report token scopes from whoami, never by
  slicing the token
- 🔵 **Compatibility**: State that switching an existing corpus between
  trackers is out of scope
- 🔵 **Code Quality**: Default `create_kind` to `preview_kind`
- 🔵 **Architecture**: Record the string-match registry as debt in a
  follow-up work item
- 🔵 **Correctness**: Specify `Rejected` for an unknown kind, the
  `all_teams` exclusion and validation of the fallback state
- 🔵 **Test Coverage**: Test concurrent bootstrap and the `.gitignore`
  scaffold

### Strengths

- ✅ `ShortcutKey { Story, Epic }` is a rich domain type. The entity is fixed
  by the key after create, so later edits to the local kind cannot
  misdirect calls.
- ✅ The phase order (seams, client, composition root, binary, skills) lets
  each phase merge independently. Phase 1 reuses the `NotAvailable` path.
- ✅ The cargo-pup rules, deny lists and command-runner updates mirror the
  Linear and Jira isolation exactly.
- ✅ The transport refuses redirects and limits `next_page_url` to the
  configured origin. It never resends a POST on an ambiguous failure.
- ✅ Every v4 path sits in one `paths` module, which isolates the beta API.
- ✅ The `fetch_all` absence proof requires a complete read and counts
  archived entities as found.
- ✅ `ContractSubject::create_kind` is additive, and the plan re-runs the
  Jira and Linear contracts as a regression guard.
- ✅ Credentials reuse the shared consent ladder. Write skills are not
  model-invocable and confirm before writing.
- ✅ Error paths are designed to be actionable: unknown team, ambiguous
  state, read-only scope, and unsupported upload.
- ✅ The scope exclusions are explicit and tied to the work items that own
  them.

### Recommended Changes

1. **Rewrite the classification table per operation** (addresses: the write
   table; "unparseable key is Terminal"; the classifier test list).
   Mirror `jira-client`'s `classify(Outcome, Operation, detail)`:
   - provable rejections on writes (400, 401, 403, 404, 422, 429) become
     `Retryable`;
   - 5xx, timeout and non-JSON responses after send become `Terminal`;
   - refusals before send become `Rejected`;
   - reads return only `Retryable`.

   Define key-parse failures per operation, and have `validate_update`
   ignore the id. Add a table-driven test.
2. **Make the catalogue explicit** (addresses: constructor I/O; default
   workflows only; incomplete state; nullable mention name; untrusted
   catalogue).
   - `from_config` only reads the catalogue, and fails `Unconfigured` with an
     `init-shortcut` hint when it is absent.
   - `init discover` catalogues every workflow from `GET /workflows`.
   - Teams are keyed by uuid, with the mention name and display name as
     aliases. A null mention name falls back to the uuid.
   - A missing workflow seen during sync is healed through a finaliser that
     reports "commit it", as Linear's does.
   - Slugs and ids are validated on load.
   - The lock is re-checked before every write, and an incomplete catalogue
     never overwrites a complete one.
3. **Split the evidence from the fixtures** (addresses: raw payloads; the
   unverified v4 request shapes; the `sct_` denylist; quote escaping).
   - Committed evidence stays reduced.
   - Raw captures go to a gitignored directory. Fixtures are sanitised and
     committed by hand, with a hygiene test.
   - The evidence list gains every Phase 3 request shape, plus the quote and
     backslash behaviour.
   - `sct_` is added to `SECRET_SHAPES`.
4. **Fix the contract-suite wiring** (addresses: moved-issue panic; live suite
   never wired; `create_kind` and `preview_kind` agreement).
   - Exclude the moved-issue property explicitly.
   - Add live story and epic subjects in `tests/contract.rs`.
   - Default `create_kind` to `preview_kind`.
5. **Report found ids under the requested form** (addresses: case
   canonicalisation). Refuse leading zeros, and test both choices.
6. **Move the URL override to `shortcut-cli`** (addresses: override placement;
   allowlist tests). Keep `paths` pure, add `from_config_at`, and list the
   allowlist tests.
7. **Replace the checklist paraphrase with the README's fourteen points**
   (addresses: checklist mismatch). Mark each point as action or no-action.
   Add `init-shortcut` to `_NOT_KEYWORDS` and give the `plugin.json` position.
8. **Complete the match-site inventory** (addresses: match sites; tracker
   lists; exit codes; the `migrate-cli` edge; the public-API snapshot).
   - Add `main.rs:531`, the sync skill's tracker lists and the exit-code
     docs.
   - Drop the `migrate-cli` edge.
   - Add the `remote-projection` snapshot.
   - Fix exit 74 to 72.
   - Update the structure test pin.
   - Add a follow-up work item for a tracker registry.
9. **Add the missing tests** (addresses: the POST retry tests; fetch_all; the
   write refusals; the create state; the filters; the match-site arms; the
   negative regex cases; the flow-test inventory).
10. **Specify model tolerance** (addresses: beta schema drift). Unknown fields
    are ignored, open sets get an `Other(String)` variant, and a test parses a
    response with an extra field.
11. **Smaller usability and code fixes** (addresses: the minor findings).
    - Trigger `show` automatically only on `sc-` keys and Shortcut URLs.
    - Let `--type` accept local kinds.
    - Match Linear's `discover` write-back and `--quiet`.
    - Specify the 401 and no-token messages.
    - Give PATCH a resend policy, treating `external_links_add` like POST.
    - Give the write operations client modules.
    - Add tracing events.
    - Mark remote content as untrusted in the read skills.

## Per-Lens Results

### Architecture

**Summary**: The plan fits Shortcut into the hexagonal shape well. The main
risk is a client constructor that makes network calls and writes a committed
file, which diverges from Linear and from 0336 without saying so. Smaller
issues are the override's crate placement, a write classification at odds with
the port, and a deeper string-match registry.

**Strengths**: the `ShortcutKey` domain type; the `NotAvailable` reuse; the
cargo-pup mirroring; resilience in the transport; the isolated `paths`
module; the additive contract hook; scope exclusions tied to the owning work
items.

**Findings**:
- 🟡 major (high) — Phase 2 §7 — The client constructor performs network I/O
  and writes a committed file. The suggestion is a read-only `from_config`,
  with bootstrap in `init` or a healing port.
- 🟡 major (medium) — Catalogue; Phase 6 §1 — The catalogue covers only
  default workflows and has no refresh. The suggestion is to decide the
  catalogued workflows and the policy for missing ones.
- 🔵 minor (high) — Phase 2 §5 — The env override is read in the adapter. The
  suggestion is a pure `paths`, `from_config_at`, and the read in
  `shortcut-cli`.
- 🔵 minor (medium) — Phase 2 §6 — The write mapping departs from the port's
  provable-non-application rule.
- 🔵 minor (high) — Phase 4 §2 — There is an unneeded `migrate-cli` edge.
- 🔵 minor (medium) — Phase 3 §1 — `ShortcutEpic` would leak entity type into
  the enum. The suggestion is to normalise in the client.
- 🔵 minor (medium) — Phase 2 §7; Phase 3 §3 — Scope identity uses a nullable
  and mutable name. The suggestion is the uuid as identity.
- 🔵 suggestion (medium) — Phase 7 — Record the string-match registry as debt.

### Code Quality

**Summary**: The plan models the domain well. The biggest risk is the error
classification, which contradicts the port and `jira-client`. Other risks are
repeated dispatch, an incomplete match-site inventory and undefined catalogue
states.

**Strengths**: `ShortcutKey`; the isolated `paths`; module decomposition
mirroring the existing clients; the additive contract hook; named tests for
the subtle transport guarantees; explicit YAGNI exclusions.

**Findings**:
- 🟡 major (high) — Phase 2 §6 — The write classification contradicts the port
  and Jira. `RequestInvalid` is misnamed. Rows are missing for timeout,
  not-sent and non-JSON.
- 🟡 major (medium) — Phase 4 §2 — The match-site inventory is incomplete. The
  suggestion is to list every site, or parse into a closed enum.
- 🔵 minor (medium) — Shortcut keys; Phase 3 §3; Phase 6 §1 — Put the
  dispatch on the domain types: `collection()`, `NewEntity::for_kind`,
  `into_story()`.
- 🔵 minor (medium) — Phase 2 §7 — The bootstrap is a hidden side effect, and
  the incomplete state has no consumer.
- 🔵 minor (medium) — Phase 2 §4; Phase 3 §3 — There are no observability
  provisions.
- 🔵 minor (low) — Phase 3 §1, §3 — Evidence-contingent branches. The
  suggestion is a `KeyedRead` strategy seam and a post-Phase 2 checkpoint.
- 🔵 suggestion (medium) — Phase 3 §5-6 — `create_kind` should default to
  `preview_kind`.
- 🔵 suggestion (high) — Phase 5 §1 — Drop the restated checklist.

### Test Coverage

**Summary**: The plan is test-first throughout, and its strongest move is
running the contract suite for two subjects. The gaps are in the riskiest
custom logic: the URL allowlist, POST retries, fetch_all chunking, the write
refusals, the live-suite wiring and evidence hygiene.

**Strengths**: test-first ordering; two contract subjects; a test-driven
`create_kind` with a regression re-run; thorough `ShortcutKey` boundary tests;
security-relevant transport tests; mocks only at the HTTP boundary; the
`contract.rs` naming.

**Findings**:
- 🟡 major (high) — Phase 3 Manual; Phase 2 §8 — The live contract suite is
  never wired.
- 🟡 major (high) — Phase 2 §5 — The URL override allowlist has no tests.
- 🟡 major (high) — Phase 2 §4 — No test covers a POST getting 5xx, a POST
  retried after 429, the PATCH policy, Retry-After or the deadline.
- 🟡 major (medium) — Phase 3 §3 — The fetch_all chunking, archived and
  fallback paths are untested.
- 🟡 major (high) — Phase 6 §1 — The write-side refusal paths are untested.
- 🟡 major (medium) — Phase 2 §8; Phase 3 §6 — The evidence hygiene guard is
  missing, and pinning fixtures to the evidence clashes with reduced
  evidence.
- 🔵 minor (high) — Phase 2 §6 — The classifier has no test list.
- 🔵 minor (medium) — Phase 3 §3 — The create state choice is untested.
- 🔵 minor (medium) — Phase 3 §4 — The state refusal and team composition are
  untested.
- 🔵 minor (medium) — Phase 4 §2 — The scope-key and `body_rules` arms are
  untested.
- 🔵 minor (medium) — Phase 1 §4 — The regex has no negative eval cases.
- 🔵 minor (medium) — Phase 5 §4 — `scenario_inventory.rs` and
  `stderr_diagnostics.rs` are missing.
- 🔵 suggestion (low) — Phase 2 §7 — Test concurrent bootstrap, the
  incomplete re-run and the `.gitignore` scaffold.

### Correctness

**Summary**: The shape is sound, but several classification and key-handling
rules conflict with the port and the contract suite. The worst is the
`Terminal` write table, which strands a read-only token's pushes as "may have
landed".

**Strengths**: the entity is fixed by the key, not by the kind; absence only
after a complete read; POST resent only on 429; distinct prefixes avoid id
collisions; the Phase 1 fall-through paths are safe.

**Findings**:
- 🔴 critical (high) — Phase 2 §6 — The write table marks provably unapplied
  writes as `Terminal`.
- 🟡 major (high) — Shortcut keys — "Unparseable is Terminal" violates the
  read and validate contracts, and `PREVIEW-1` would fail.
- 🟡 major (high) — Phase 3 §6 — The moved-issue property panics with `None`.
- 🟡 major (medium) — Shortcut keys; Phase 3 §3 — Case and leading-zero
  canonicalisation breaks pairing.
- 🟡 major (medium) — Phase 2 §7 — The incomplete-catalogue and concurrent
  bootstrap transitions are undefined.
- 🟡 major (medium) — Phase 6 §1; Phase 3 §4 — Only default workflows are
  catalogued, and transition needs the story's workflow first.
- 🔵 minor (medium) — Phase 3 §3 — Teams with a null mention name cannot be
  keyed.
- 🔵 minor (medium) — Phase 2 §8 — The absence proof relies on unverified
  `?ids=` semantics. Treat an unrequested id in the response as incomplete.
- 🔵 minor (medium) — Phase 2 §4; Phase 6 §1 — The PATCH resend policy is
  unspecified.
- 🔵 suggestion (medium) — Kind table; Phase 3 §4 — Specify an unknown kind
  as `Rejected`, the `all_teams` exclusion and validation of the fallback
  state.

### Security

**Summary**: The plan follows the existing security model well. The main gap
is the plan to commit raw live payloads as evidence. The smaller gaps are the
denylist, catalogue trust, search escaping and untrusted remote content.

**Strengths**: the override allowlist and loopback gate; same-origin paging
with redirects off; the consent ladder; strict key parsing; non-invocable,
confirming write skills; https-only attach; the `.gitignore` scaffold.

**Findings**:
- 🟡 major (high) — Phase 2 §8 — The live test plans to commit raw response
  payloads.
- 🔵 minor (high) — Phase 2 §8 — There is no `sct_` shape in
  `SECRET_SHAPES`.
- 🔵 minor (medium) — Catalogue; Phase 2 §5 — The catalogue slug and ids are
  trusted. Validate them and build paths with `path_segments_mut`.
- 🔵 minor (medium) — Phase 3 §4 — Quote-escape support is unverified, so
  refuse `"` and `\` until it is confirmed.
- 🔵 minor (medium) — Phase 5 §5 — The read skills should mark remote content
  as untrusted data.
- 🔵 suggestion (medium) — Phase 5 §2 — Pin that `init verify` never echoes
  token material.

### Compatibility

**Summary**: The plan only adds things, and it protects Jira and Linear. The
main risk is external: unverified v4 shapes and no rule for schema tolerance.
Several pinned internal contracts are missing from the change lists.

**Strengths**: the isolated `paths`; the default `create_kind`; Phase 1
through `NotAvailable`; owned ids unchanged; live evidence before mocks; POST
and paging that follow the HTTP contract.

**Findings**:
- 🟡 major (medium) — Phase 2 §8; Phase 3 §3 — Most of the v4 request shapes
  are unverified.
- 🟡 major (medium) — Phase 2 §7 — There is no rule for tolerating new fields
  or enum values.
- 🔵 minor (high) — Phase 1 §4 — The structure test pins the old regex, and
  the change affects all trackers.
- 🔵 minor (high) — Phase 1 Manual — Exit 74 should be 72.
- 🔵 minor (high) — Phase 1 §3; Phase 4 §2 — The sync skill and exit-code
  docs list trackers.
- 🔵 minor (high) — Phase 3 §1 — The `remote-projection` public-API snapshot.
- 🔵 minor (low) — Migration Notes — Foreign `SC-`/`EPIC-` keys after a
  tracker switch.
- 🔵 suggestion (low) — Phase 5 §2 — Don't parse token scopes from the token.

### Usability

**Summary**: The plan mirrors Linear closely and designs actionable errors.
The friction is in Shortcut-specific details: a nullable `team_key`, default
workflows only, mixed vocabulary, and small drifts from Linear's flags.

**Strengths**: actionable errors; recognisable `sc-` keys; a mirrored CLI
surface; epic usage errors that guide the user; confirming write skills; read
scope reported early.

**Findings**:
- 🟡 major (high) — Phase 1 §2; Phase 2 §3, §7 — `team_key` holds a nullable
  mention name.
- 🟡 major (medium) — Catalogue; Phase 6 §1 — State resolution has no
  recovery path.
- 🔵 minor (medium) — Phase 5 §5 — The `epic-<n>` trigger will fire on
  incidental mentions.
- 🔵 minor (medium) — Kind table; Phase 5 §2 — Two vocabularies, and
  `-story` names that undersell epic support.
- 🔵 minor (medium) — Phase 5 §2; Phase 6 §1 — Flag and default drift from
  Linear.
- 🔵 minor (medium) — Phase 2 §7 — The bootstrap writes a tracked file without
  notice.
- 🔵 minor (low) — Phase 2 §6 — The 401 and no-token guidance is unspecified.
- 🔵 suggestion (low) — Phase 2 §2 — Accept Shortcut URLs.
- 🔵 suggestion (low) — Phase 1 §5 — The docs list `shortcut` before it
  works.

### Standards

**Summary**: The plan follows the precedent in naming, layout, config keys,
pup rules and skills. The divergences are the override's placement, a wrong
checklist paraphrase, and smaller registries.

**Strengths**: crate names and metadata; module names; config key shapes;
pup rule naming and its companion edits; skill naming; the regex aligned with
`is_tracker_key`; the `contract.rs` naming.

**Findings**:
- 🟡 major (high) — Phase 2 §5 — The override and loopback gate are in the
  client.
- 🟡 major (high) — Phase 5 §1 — The restated checklist doesn't match the
  README's fourteen points.
- 🔵 minor (medium) — Phase 5 §5 — Add `init-shortcut` to `_NOT_KEYWORDS`.
- 🔵 minor (high) — Phase 5 §5 — The `plugin.json` position (after `linear/`)
  and the configure text.
- 🔵 minor (medium) — Phase 5 §3 — The init band 60-69 and the
  `captured-exit-codes.txt` name.
- 🔵 minor (medium) — Phase 6 §1 — The write operations need client modules.
- 🔵 suggestion (low) — Phase 5 §4 — The full flow-test peer set.
- 🔵 suggestion (low) — Phase 5 §5; Phase 6 §2 — Say in the descriptions that
  epics are accepted.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-13

**Verdict:** REVISE

### Previously Identified Issues

Of the 70 earlier findings, 66 are resolved and 4 are partly resolved. None is
still present. The critical finding (write classification) is resolved.

- 🔴 **Correctness / Code Quality / Architecture**: Write-error table marks
  provably unapplied writes as Terminal — Resolved
- 🟡 **Architecture / Usability / Code Quality**: Client constructor performs
  network I/O — Resolved
- 🟡 **Correctness / Architecture / Usability**: Catalogue covers only default
  workflows — Partially resolved (the state filter lacks the team-to-workflow
  relation)
- 🟡 **Correctness**: Incomplete-catalogue and bootstrap transitions —
  Resolved
- 🟡 **Usability / Architecture / Correctness**: Team identity rests on a
  nullable mention name — Partially resolved (see the new team-resolution
  finding)
- 🟡 **Security / Test Coverage**: Raw payloads committed as evidence —
  Resolved
- 🟡 **Compatibility / Correctness**: v4 request shapes unverified — Resolved
- 🟡 **Compatibility**: No beta tolerance rule — Resolved
- 🟡 **Correctness**: "Unparseable key is Terminal" — Resolved
- 🟡 **Correctness**: Moved-issue property panics — Resolved
- 🟡 **Correctness**: Case canonicalisation breaks pairing — Resolved
- 🟡 **Test Coverage**: Live contract suite not wired — Resolved
- 🟡 **Test Coverage**: URL allowlist untested — Resolved
- 🟡 **Test Coverage**: POST retry untested — Resolved
- 🟡 **Test Coverage**: fetch_all paths untested — Resolved
- 🟡 **Test Coverage**: Write refusals untested — Resolved
- 🟡 **Standards / Architecture**: Override placed in the client — Resolved
- 🟡 **Standards / Code Quality**: Checklist mismatch — Resolved
- 🟡 **Code Quality / Architecture**: Match-site inventory incomplete —
  Resolved
- 🔵 **Test Coverage**: Scope-key and `body_rules` arms untested — Partially
  resolved (`body_rules` and `m0009` still name no test)
- 🔵 **Usability**: Flag drift from Linear — Partially resolved (`--owner` and
  `--estimate` are unexplained, and the value `--owner` takes is unspecified)
- 🔵 **Code Quality**: Evidence-contingent branches — Partially resolved
  (conditional text remains in Phase 3)
- Every other minor finding and suggestion — Resolved

### New Issues Introduced

#### Major

- 🟡 **Architecture / Compatibility / Correctness**: Team resolution breaks
  wider scopes — The port tells `resolve_scope` to leave broadened keys as
  configured (`cli/tracker/src/lib.rs:821-824`). `identifier_for` matches
  `VisibleEntity.key` exactly (`work-adapters/src/sync/scope.rs:112-129`).
  The plan rewrites every key to a uuid, keys entities by mention name, and
  accepts display-name aliases. So any sync with `additional_teams` or
  `all_teams` exits 74. The fix is to follow `LinearClient::resolve_scope`,
  turn aliases into one form, and make the uuid the `identifier`.
- 🟡 **Correctness**: Classifying only the last attempt misclassifies resends
  — The plan resends a POST on `NotSent` and a PATCH on `Transport`. Jira
  resends only on 429 or 5xx (`jira-client/src/transport.rs:347-351`). An
  unsent POST that is resent and fails again becomes `Terminal`. A timed-out
  PATCH that is then refused becomes `Retryable`, though the first attempt may
  have applied. The fix is to carry a "may have applied" flag across attempts.
- 🟡 **Correctness**: Search scopes by a display name that can be shared —
  `team:"<name>"` returns every team with that name. The fix is to filter each
  page by team uuid, and to record how `team:` matches.
- 🟡 **Compatibility**: Catalogue versioning breaks teams on mixed plugin
  versions — A newer `schema_version` locks out teammates on older plugins,
  and a rewrite drops fields the client does not know. Linear's catalogue
  keeps them (`linear-client/src/catalogue/document.rs:629-649`).
- 🟡 **Usability / Code Quality / Security**: A blanket load-time team-name
  check locks out the workspace — One team whose name holds `"` or `\` makes
  every command `Unconfigured`, and the suggested `init discover` cannot fix
  it. The check also disagrees with the Phase 3 filter rule. The fix is to
  check only in-scope names when they go into a query, including the live name
  `search` reads.
- 🟡 **Test Coverage**: No test pins each write module's resend policy —
  `comment` and `attach` could be declared resendable without any test failing.

#### Minor

- 🔵 **Security**: The secret check finds tokens by prefix only. The live test
  should check its output files for the exact token string.
- 🔵 **Compatibility**: The new `GET /teams/{uuid}` has no evidence entry.
- 🔵 **Architecture**: Phase 4 can merge before `init-shortcut` exists, so
  Phase 5 must land first.
- 🔵 **Architecture / Code Quality**: Two builders for the same PATCH. Put
  `UpdateFields` in Phase 3.
- 🔵 **Code Quality**: The attach resend policy is fixed in one table and
  conditional in another.
- 🔵 **Code Quality**: The classification prose says "every 4xx", but the
  table lists some codes and omits 410.
- 🔵 **Code Quality**: `into_story` names CLI subcommands, so it should return
  a domain error instead.
- 🔵 **Code Quality**: The `type` filter reuses the create-side kind table.
  Add `story_type_for_kind`, and refuse `epic` in the filter.
- 🔵 **Correctness**: The catalogue lacks the team-to-workflow relation the
  state filter needs.
- 🔵 **Correctness**: Filter refusals have no stated class. They must be
  `ScopeError` or `Unconfigured`, never `classify(Refused, Read)`.
- 🔵 **Correctness**: `enumerate_visible_entities` must fail on a truncated
  `GET /teams`.
- 🔵 **Correctness**: A 2xx create with no usable id must be `Terminal`.
- 🔵 **Standards**: Use the shared variant names `RequestInvalid`,
  `Status(u16)`, `WhileRetryable` and `OnlyWhenThrottled`.
- 🔵 **Test Coverage**: The older `schema_version` branch is untested.
- 🔵 **Test Coverage**: The `init discover` overwrite and `--force` paths, and
  foreign-workspace and epic URLs, are untested.
- 🔵 **Test Coverage**: The null-mention fallback and the shared deadline
  across chunks are untested.
- 🔵 **Usability**: Following the rename warning can break a display-name
  `team_key`. Recommend the mention name as the stable form.
- 🔵 **Usability**: The state-filter refusal lacks the `init discover` hint.

#### Suggestions

- 🔵 **Test Coverage**: Drive the offline contract through `run_all`, or count
  the properties it runs.
- 🔵 **Test Coverage**: Assert that `tests/captures/` is gitignored.
- 🔵 **Code Quality**: Make the checkpoint's output replace each conditional in
  Phase 3.
- 🔵 **Usability**: Say how to recover from a truncated discovery.
- 🔵 **Standards**: State `name = "shortcut-cli"` in the Cargo.toml.

### Assessment

The revision fixed every finding from pass 1 in substance. The critical
classification error is gone, and the catalogue, the evidence and the contract
wiring are now sound. But the edits introduced six new major findings, which
is over the threshold of three, so the verdict stays REVISE. Four of the six
sit in one area: how a team is named, resolved and searched. They share one
fix. The client should resolve every team alias to the uuid in one place,
leave wider scopes to the engine as the port requires, filter search results
by uuid, and check names only when they enter a query. The other two concern
retry classification across attempts, and teams on mixed plugin versions.
After one more focused revision, the plan should be ready to implement.

## Re-Review (Pass 3) — 2026-11-13

**Verdict:** COMMENT

### Previously Identified Issues

All six pass-2 major findings are resolved, apart from one test gap that
remains, described below.

- 🟡 **Architecture / Compatibility / Correctness**: Team resolution breaks
  wider scopes — Resolved
- 🟡 **Correctness**: Classifying only the last attempt — Resolved
- 🟡 **Correctness**: Search scopes by a shared display name — Resolved
- 🟡 **Compatibility**: Catalogue versioning on mixed plugin versions —
  Resolved
- 🟡 **Usability / Code Quality / Security**: Load-time team-name check —
  Resolved for configured teams. It is partly resolved under `all_teams`,
  where one unquotable name still fails discovery.
- 🟡 **Test Coverage**: Per-module resend tests — Partially resolved. The
  `create` gap is carried forward below.
- Every pass-2 minor finding and suggestion — Resolved. Security and standards
  report no open item from earlier passes.

### New Issues Introduced

#### Major

- 🟡 **Test Coverage**: No test pins that `create` declares
  `OnlyWhenThrottled` — The only `create` resend test uses a timeout, which no
  policy resends. A `create` that resent on a 5xx would create duplicate
  stories or epics, and every listed test would still pass. The fix is to count
  requests when `POST /stories` and `POST /epics` return 500.
- 🟡 **Compatibility**: Multi-value pull filters have no lowering rule —
  Linear and Jira treat several values for one field as OR
  (`linear-client/src/filter.rs:402-408`, `jira-client/src/jql.rs:278-298`).
  Shortcut search ANDs repeated operators. So `label: [a, b]` would narrow
  discovery silently. The fix is to record the operator grammar in Phase 2,
  and to lower to OR, or to one search per value with the results unioned.

#### Minor

- 🔵 **Architecture / Code Quality / Correctness**: The team-key rule and its
  ambiguity rule have no single home. The live enumeration can emit duplicate
  keys, and the engine's `find` then picks one at random. A display name can
  also equal another team's mention name. The fix is a `TeamKey` type and one
  uniqueness check over all keys, configured and enumerated.
- 🔵 **Architecture / Code Quality / Correctness**: `auth::team_key` is used in
  Phase 4 but never defined, and it shares a name with the catalogue resolver.
  The absent-key case is unspecified.
- 🔵 **Architecture / Correctness**: Broadened-scope state validation needs
  catalogue lookups. The plan does not say what happens to a team that is
  visible live but missing from the catalogue.
- 🔵 **Correctness**: `NonJsonBody` must mean a 2xx only. A non-JSON 403 from a
  proxy must classify by its status.
- 🔵 **Correctness**: Under `all_teams`, one refused team name fails
  discovery for the whole workspace.
- 🔵 **Code Quality**: `classify` takes a bare boolean, and its table holds a
  body-decoding row. The fix is to return one `Delivery` value from the
  transport.
- 🔵 **Code Quality**: `update --state` and `transition` both need the same
  workflow-scoped state lookup.
- 🔵 **Compatibility**: The Phase 6 request shapes (`GET /members`,
  `owner_ids`, `estimate`, a story's `workflow_id`) are missing from the
  evidence list.
- 🔵 **Compatibility**: An older plugin's `init discover` could overwrite a
  newer-schema catalogue.
- 🔵 **Test Coverage**: No test passes `may_have_applied` through a port call.
- 🔵 **Test Coverage**: No test covers a failing `GET /teams/{uuid}` in
  `search`.
- 🔵 **Usability**: `init discover` is said to ask before overwriting, but
  Linear's binary never prompts. The skill confirms with the user, then
  re-runs with `--force`.
- 🔵 **Usability**: "Retry" is the wrong advice for a page-cap cut.
- 🔵 **Usability**: The "add a mention name" and "rename the team" hints omit
  `init discover` and the `team_key` update.
- 🔵 **Usability**: `init-shortcut` does not say to commit and share the
  catalogue.
- 🔵 **Standards**: "BadApiUrl code" names an error variant. Linear exits 2
  (`USAGE`) with an `E_BAD_API_URL:` prefix.

### Assessment

Each pass found fewer and smaller problems. Pass 1 found 1 critical and 17
major findings, pass 2 found 6 majors, and pass 3 finds 2. Security reports no
findings, and standards reports one minor. Both open majors are narrow and have
a clear fix: one missing test, and one lowering rule plus its evidence item.
None of the minor findings changes the plan's structure. Most say which
function owns a rule, or tighten an error message. The plan is acceptable to
implement once the two major findings are addressed.

## Re-Review (Pass 4) — 2026-11-13

**Verdict:** APPROVE

The reviewer set the verdict to APPROVE after the two pass-4 minor findings
were fixed in the plan.

This pass re-ran the compatibility and test-coverage lenses, which raised the
two pass-3 major findings.

### Previously Identified Issues

- 🟡 **Test Coverage**: No test pins that `create` declares
  `OnlyWhenThrottled` — Resolved
- 🟡 **Compatibility**: Multi-value pull filters have no lowering rule —
  Resolved
- 🔵 **Compatibility**: Phase 6 request shapes missing from the evidence —
  Resolved
- 🔵 **Compatibility**: An older plugin's `init discover` could downgrade the
  catalogue — Resolved
- 🔵 **Test Coverage**: `may_have_applied` not tested through a port call —
  Resolved
- 🔵 **Test Coverage**: A failing `GET /teams/{uuid}` untested — Resolved
- 🔵 **Test Coverage**: The other pass-3 fixes (`NonJsonBody`, newer-schema
  refusal, shared state lookup, collisions, multi-value lowering) have tests —
  Resolved, apart from the base-key collision below

### New Issues Introduced

- 🔵 **Compatibility / Test Coverage**: Truncation was undefined across the
  searches one multi-value filter splits into. Fixed after this pass: the page
  cap and deadline apply to all of a team's searches together, any truncated
  search truncates the merged result, and a test pins it.
- 🔵 **Test Coverage**: No test refused a colliding base `team_key`. Fixed
  after this pass: a test now refuses it before `Catalogue::team_for_key`
  picks a team.

### Assessment

No critical or major findings remain. Both new minor findings were fixed in
the plan straight after this pass. The other six lenses ran last in pass 3,
and they raised only minor findings, all of which pass 4's revision
addresses. The plan is ready to implement.
