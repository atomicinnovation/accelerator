---
type: "plan-validation"
id: "2026-08-17-0210-provider-client-crates-over-the-tracker-port-validation"
title: "Validation Report: Provider Client Crates over the RemoteTracker Port"
date: "2026-08-19T00:21:06+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-08-17-0210-provider-client-crates-over-the-tracker-port"
tags: ["rust", "jira", "linear", "integrations", "tracker", "validation"]
last_updated: "2026-08-19T00:21:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Provider Client Crates over the RemoteTracker Port

**Result: partial** — all ten phases are implemented and committed, and every
automated check that can run on this host is green. The report is `partial`, not
`pass`, for one reason only: a set of the plan's own manual acceptance criteria
require a live credentialed tenant or per-process network disablement that this
machine does not provide. Nothing was found defective. The gap is unperformed
verification, not broken implementation.

### Implementation Status

✓ Phase 1: HTTP test-support crate — implemented, committed `bf7e7192`
✓ Phase 2: Shared crates and oracle transcriptions — implemented, committed `7bb170ed`
✓ Phase 3: `jira-client` foundation — implemented, committed `9dd6aff2`
✓ Phase 4: `jira-client` ADF conversion — implemented, committed `ee93deec`
✓ Phase 5: `jira-client` `impl RemoteTracker` — implemented, committed `8fe8521a`
✓ Phase 6a: `linear-client` foundation — implemented, committed `7adda8a3`
✓ Phase 6b: `linear-client` `impl RemoteTracker` — implemented, committed `caa75991`
✓ Phase 7: Composition root — implemented, committed `de655cb2`
✓ Phase 8: Jira provider surface — implemented, committed `efe0a2a6`
✓ Phase 9: Linear provider surface — implemented, committed `3282ee59`
⚠️ Phase 10: Enforcement close-out — offline enforcement + evidence machinery + live contract evidence committed (`wmuqprzs`, `sylpukpl`); one criterion (networking-disabled transcript) blocked by the host

All eleven referenced commits exist in history, plus `91129dfb` (contract lane
taken out of `default`) and `wqmxqysv` (stale-comment cleanup). The working copy
is clean — every phase is committed, nothing is staged uncommitted.

### Automated Verification Results

✓ `mise run check` — the exact read-only CI mirror (format + lint + types +
  `public-api` + `pup` + `store-duplication` + shellcheck across all four
  components): **exit 0**. Ran it independently; the new crates
  (`remote-projection`, `tracker-support`, `tracker`, `jira-client`,
  `linear-client`, `work-cli`) all check and document green.
✓ All five new crate directories present and registered in `cli/Cargo.toml`
  `[workspace].members`.
✓ Contract lane is opt-in: `cli/.config/nextest.toml` sets
  `default-filter = 'not binary(=contract)'` on the default profile and
  `'binary(=contract)'` on the `contract` profile; the reason is recorded in
  `_NOT_IN_INTEGRATION_ROLLUP` (`tasks/README.md:221`).
✓ Committed evidence files present, reduced form, no payloads: Jira 4 records,
  Linear 5 records, both dated 2026-08-18 (after Phase 9's `3282ee59`).
✓ Enforcement test files present: `tests/integration/deny/test_licence_closure.py`,
  `cli/work-cli/tests/no_network_by_default.rs`, and `evidence_hygiene.rs` in
  both clients.

⚠️ **Not independently run: the full test suite.** I ran `mise run check`, not
the heavier `mise run` default (which adds `test:*` and `docs:check`). The plan
records the full mirror as green with documented load-sensitive flakes in
unrelated lanes (`jira_with_lock`, circus readiness) that pass standalone. I did
not reproduce that run; the committed per-crate test counts and the green
read-only mirror are the evidence relied on here.

### Code Review Findings

#### Matches plan

- **The frozen port is untouched.** `cli/tracker/Cargo.toml` declares no
  `[dependencies]` and no `[dev-dependencies]`; `src/lib.rs` is 343 lines with
  the same six public items; `tests/fixtures/public-api.txt` is 77 lines
  unchanged; `structure.rs:54-77` still asserts no dependency tables and no
  `tracker-adapters` sibling. The added `E_DISPATCH_UNCONFIGURED=74` row in
  `dispatch-codes.txt:15` is tagged `above-the-port`, so the `Class`-tagged
  count guard (`errors.rs:109-122`) still tallies exactly 2 and stays green —
  exactly as the deviation note claims.
- **The composition root resolves real clients.**
  `tracker_registry.rs:169-179` constructs `JiraClient`/`LinearClient` via
  `from_config` where every arm previously returned `Err`; `trello` and
  `github-issues` remain `NotAvailable`. `ConfiguredTrackers<'a>` carries the
  config reference and an owned root (`new(config, root)`); all three sites in
  `main.rs` (:214/:267/:383) compute the root via
  `FileConfigStore::discover_root`. The production `VcsProvenance` and
  `InProcessProbe::is_tracked` (`vcs-adapters/src/library/tracked.rs`) exist —
  git via `entry_by_path`, jj via the working-copy commit tree's `path_value`,
  no snapshot written.
- **The D10 differential guarantee holds.** Both differentials genuinely shell
  out to the oracle. The five-mapper test (`tracker-support/tests/mapper_differential.rs`)
  sources the real bash and sweeps codes 0–130, asserting `compared == 4 × 131 = 524`;
  the ADF test (`jira-client/tests/adf_differential.rs`) drives `jira-adf-to-md.sh`
  and `jira-md-to-adf.sh` (hence `jira-adf-render.jq`, `jira-md-tokenise.awk`,
  `jira-md-assemble.jq`) in both directions and asserts `compared > 0`. Both
  `.expect(...)` on spawn so they **fail rather than skip** when the oracle is
  absent. Each has a committed sibling self-test planting a deliberate mismatch,
  proving the comparison function can fail. This is the plan's core defence
  against a mutually-consistent fixture-plus-code pair, and it is real.

#### Deviations from plan

All deviations are documented in the plan's own "Deviations from the plan as
written" section (lines 428–734) with reasons; they are corrections the running
oracle forced or constraints the plan did not foresee, not silent drift. Spot-checks
confirmed the load-bearing ones (74-row taxonomy, `from_config` taking a
`CredentialContext`, `VcsProvenance` built in Phase 7 rather than pre-existing in
`cli/vcs`, cache lock reusing `corpus_adapters::acquire`). No undocumented
deviation surfaced in this validation.

#### Potential issues

- **The new network failure surface is unverified end-to-end on this host.** The
  plan itself flags Phase 7 onward as "a new failure surface — network, auth,
  rate limits, partial fetches — on a user-invocable path". The offline contract
  (mock) enforces the port properties, and committed live-tenant evidence shows
  the contract harnesses passed against real Jira and Linear on 2026-08-18. But
  the user-path manual checks (`create --push`, `update --push`, interrupted-push
  recovery, live `remote_hash` parity, live attach and discovery diff) remain
  unperformed — they need a credentialed tenant this machine lacks.
- **One criterion is structurally unsatisfiable on this host**: the
  networking-disabled default-suite transcript. Per-process network disable is
  unavailable on macOS here; the mechanical property (no `contract` binary in the
  default profile) is instead pinned by `no_network_by_default.rs` and
  `test_nextest_filter.py`.

### Manual Testing Required

These are the plan's own open manual criteria, all blocked on a live credentialed
tenant or host capability. None is a regression; each is verification that could
not run here.

1. Live Jira (needs `jira.site`, `jira.email`, `jira.token`):
  - [ ] `mise run test:integration:tracker-contract` green against a live project
  - [ ] `accelerator work sync` behaviour matches the bash bridge, `remote_hash` parity
  - [ ] `create --push` / `update --push`, including the terminal-but-applied shape
  - [ ] Interrupted-create pending-push crash-recovery path
  - [ ] Attach a real file; run discovery and diff the cache shapes
2. Live Linear (needs `linear.token`, `linear.team_id`):
  - [ ] Contract lane green; 250-item truncation genuinely yields `indeterminate`
  - [ ] Upload a real binary to a live issue; diff `catalogue.json` against `init`
3. Host capability:
  - [ ] Default suite green with networking disabled, transcript committed
        (blocked: no per-process network disable on this macOS host)
4. Deliberately deferred (cost, not capability):
  - [ ] `cargo nextest list --message-format json` behavioural assertion that the
        default profile selects `contract_offline` and no `contract` binary — a
        full workspace test-binary build; the exact-match filter is pinned by
        `test_nextest_filter.py` instead

### Recommendations

- **Close the plan on an authorised credentialed run.** Batch the live Jira and
  Linear manual checks into one session, capture the two contract-run evidence
  files (already committed from the 2026-08-18 run — re-confirm they still pass),
  and tick the live-tenant criteria. That single session clears the bulk of the
  outstanding items.
- **Leave the networking-disabled transcript to a Linux CI lane** rather than
  forcing it locally; the mechanical guard already holds the property.
- **Treat a single full `mise run` failure in `test:integration:integrations` or
  `:dev` as suspect and re-run the lane standalone**, per the plan's documented
  load-sensitive flakes — before attributing it to this work.
- **Do not repoint any skill or delete any bash oracle yet.** That is 0211/0212
  work; the per-asset ordering gate in this plan is the checklist those siblings
  must satisfy. All named oracle scripts are still present, as required.
