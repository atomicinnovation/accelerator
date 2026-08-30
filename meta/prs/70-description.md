---
type: "pr-description"
id: "70"
title: "[0210] Provider client crates"
date: "2026-08-19T00:30:47+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0210"
parent: "work-item:0210"
relates_to: ["work-item:0171", "work-item:0211", "work-item:0212"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/70"
pr_number: 70
tags: ["rust", "jira", "linear", "integrations", "reqwest", "tracker", "adf", "graphql"]
revision: "5126a4694550d96f81879f73beef7f9f6112c0c6"
repository: "accelerator"
last_updated: "2026-08-19T00:30:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0210] Provider client crates

## Summary

Builds `jira-client` and `linear-client` as adapter crates over the workspace's pinned `reqwest` + rustls + serde stack, each implementing the frozen `RemoteTracker` port, and wires both into `accelerator-work`'s composition root so `accelerator work sync` resolves real providers instead of returning `Err` from every arm. Beyond the four port operations it ports the complete provider surface — `comment`, `transition`, `attach` and `init`'s discovery calls, plus the full ADF and GraphQL request construction — so the follow-on retirement work (0211/0212) finds no provider request construction still living in bash.

No skill changes when this merges — the jira, linear and work skills all still shell out to bash until 0211/0212 repoint them. The behaviour that changes is below the port: from the composition-root phase onward `work sync`, `create --push` and `update --push` resolve real clients and issue live network calls for `jira` and `linear`, and the configured-but-credential-less exit code moves off 72 to a dedicated `E_DISPATCH_UNCONFIGURED=74`. That is a new failure surface — network, auth, rate limits, partial fetches — on a user-invocable path.

## Changes

- **Two provider client crates.** `cli/jira-client` and `cli/linear-client`, each `impl RemoteTracker` over `reqwest::blocking`, with credential resolution, a bounded 4-attempt retry transport (30s per request, operation-level deadline), status/error classification, and the full provider surface (comment, transition, attach, init discovery) as inherent methods.
- **Jira ADF conversion, hand-built in both directions.** A byte-faithful transcription of the bash `jira-adf-render.jq` / `jira-md-assemble.jq` / `jira-md-tokenise.awk` dialect, including its deliberate lossy behaviours; no third-party ADF crate reproduces it.
- **Shared policy and test infrastructure crates.** `cli/tracker-support` (credential resolution, retry policy, `TransportConfig`, identifier-safety predicate, MIME sniffer, `port_body` newline adapter), `cli/remote-projection` (the projection recipe moved out of `work-adapters`), and `cli/http-test-support` (the three hand-rolled mock servers unioned into one, with request-body/header/query capture).
- **Composition root resolves real clients.** `ConfiguredTrackers` gains a config reference and repo root; the `jira`/`linear` arms build clients via `from_config`; `trello`/`github-issues` stay `NotAvailable`. Adds a production `InProcessProbe::is_tracked` (git via index, jj via the working-copy commit tree) behind `VcsProvenance`.
- **Security hardening on the credential trust boundary.** Command-valued and allowlist-valued config keys (`*.token_cmd`, `jira.allowed_sites`) are refused when their provenance file is VCS-tracked; `jira.site` is validated as a credential destination (absolute `https://`, label-boundary host match, no userinfo/query/fragment); a team-level `token_cmd` is refused rather than silently ignored.
- **Offline correctness anchors.** Differential tests that execute the running bash/jq/awk oracle (five exit-code mappers over codes 0–130; the ADF pipeline both directions), each asserting a non-zero comparison count, failing rather than skipping when the oracle is absent, and each proven falsifiable by a sibling self-test. Offline contract conformance against a mock runs in the default profile for both providers.
- **Enforcement and evidence.** The live contract lane is opt-in (`binary(=contract)`), recorded in `_NOT_IN_INTEGRATION_ROLLUP` with a guard that it is unreachable from `default`/`test`/`check`. Committed licence-closure evidence (production and dev trees add no new SPDX id), a network-free default-suite guard, and reduced-form live contract-run evidence for both providers (no payloads or secrets).

## Context

- Work item: `meta/work/0210-provider-client-crates-over-the-tracker-port.md`
- Parent epic: `meta/work/0171-jira-and-linear-integrations.md` (carries the D1–D16 decision register and the copyleft answer)
- Plan: `meta/plans/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md` (status `done`)
- Validation: `meta/validations/2026-08-17-0210-provider-client-crates-over-the-tracker-port-validation.md`
- Follow-ons: 0211 (integration-binary and bash-cluster retirement), 0212 (deletes the oracle scripts and the differential tests they drive)

## Testing

- [x] `mise run check` (read-only CI mirror: format + lint + types + public-api + pup + store-duplication + shellcheck across all four components) — green.
- [x] `mise run` full local mirror — green per the plan's Implementation Progress; two full runs hit documented load-sensitive flakes in unrelated lanes (`jira_with_lock`, circus readiness) that pass standalone.
- [x] Offline contract conformance for both providers selected and passing in the default profile.
- [x] Differential tests agree with the running bash oracle (mappers and ADF pipeline), with provable-failure self-tests.
- [x] Live contract-run evidence committed for both providers (Jira 4 records, Linear 5), dated 2026-08-18, passing the reduced-form hygiene guard.
- [x] Frozen port untouched — `cli/tracker` still declares no dependencies and its public-API snapshot is unchanged (the `74` dispatch row is `above-the-port` and stays outside the class-count guard).
- [ ] Live-tenant manual verification — `create --push` / `update --push`, interrupted-push recovery, live `remote_hash` parity, real attach, and discovery-cache diff — deferred; needs a credentialed Jira/Linear tenant.
- [ ] Networking-disabled default-suite transcript — blocked; per-process network disable is unavailable on the macOS host. The mechanical property is pinned by `no_network_by_default.rs` and `test_nextest_filter.py` instead.

## Notes for Reviewers

- **Review focus: classification.** The subtlest logic is Linear's status/error classification (Phase 6a/6b), which cannot mirror Jira's status-only mapping — Linear returns 403/404/410/429 equivalents as HTTP 200/400 bodies carrying `errors[]`. The row-coverage guard on each table-driven test fails the build on an unconsumed row.
- **No skill is repointed and no bash oracle is deleted here** — that is 0211/0212 work. The per-asset ordering gate in the plan is the checklist those siblings must satisfy; every named oracle script is still present.
- **Deliberate divergences from the bash are documented**: webpki-roots replaces curl's system trust store (a user-visible narrowing for TLS-intercepting proxies and private-CA self-hosted Jira, D8); a CRLF-bearing echoed upload header is refused rather than dropped (Linear attach); `jira.site` accepts both the bare Cloud subdomain and the absolute URL form. The full deviation log lives in the plan.
- **Follow-up**: the live-tenant manual checks and the networking-disabled transcript remain open (see Testing); the plan validation ruled `partial` on that basis alone, with no implementation defect found.
