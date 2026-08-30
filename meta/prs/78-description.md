---
type: "pr-description"
id: "78"
title: "[0211] Migrate the Jira integration to accelerator-jira and retire its bash cluster"
date: "2026-08-23T22:54:57+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0211"
parent: "work-item:0211"
relates_to: ["work-item:0171", "work-item:0210", "work-item:0212"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/78"
pr_number: 78
tags: ["rust", "jira", "integrations", "cli", "cutover", "exit-codes", "registration"]
revision: "fa463fdd19e2e8b6dddd0b63fff7f54a8d76b8ae"
repository: "accelerator"
last_updated: "2026-08-24T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0211] Migrate the Jira integration to accelerator-jira and retire its bash cluster

## Summary

Ships `accelerator-jira` as a thin dispatched CLI adapter over the `jira-client` crate, repoints all eight jira `SKILL.md` bodies onto `accelerator jira …`, and deletes the 197-file jira bash cluster with its mock server and fixtures. This is the Jira binary and cutover — the first of a stacked pair: it is **stacked on the Linear PR (#77)** (base `0211-linear-integration-binary-and-cutover`), and the cross-cluster residue retirement that completes work item 0211 follows in the PR stacked on top of this one.

## Changes

- **`accelerator-jira` binary** — a thin adapter over `jira-client` with the full subcommand surface (`create`/`update`/`show`/`search`/`comment`/`transition`/`attach`/`init`/`fields`/`resolve-fields`), a `work-cli`-style `exit_codes.rs` document of record pinned to a captured `bash-exit-codes.txt`, a typed keyword discriminant, and the `test-loopback` base-URL seam (compile guard + staged-binary byte scan).
- **Rich create/update surface** — `create_op`/`update_op` compose the full mutation payload (priority, labels, components with set-vs-incremental channels, parent with clear, assignee/reporter with `@me` resolution and unassign, custom fields, inline `--body`, `--issuetype-id`, per-call `--project`, `--no-notify`). New `custom_fields` (schema-typed coercion + `@json:` escape) and `principal` (`@me`/accountId/email-refusal) modules do the resolution in the binary over the caches; the shared `tracker` port and its wire-failure funnel are unchanged.
- **Search read-side projection** — `search_detailed`/`show_detailed` return the requested `fields` map plus the `nextPageToken` cursor, so `--fields`/`--render-adf`/`--page-token` survive over the stamps-only port `search`.
- **Token registration** — `jira` added to the dispatched sub-binary registry (paths, manifests, release binaries, descriptions + tuple pin, `.gitignore`), registered in the same commit as the skill repointing so dispatch coherence holds.
- **Skill repointing** — the three read/init skills drop the `jq`/`curl` and scripts-glob grants for the scoped `accelerator jira *` grant (`search-jira-issues` is the metacharacter-free token witness); the five write skills keep bare `Bash`, preview the resolved intent instead of a wire payload, and branch on the stdout keyword or an `E_*` stderr name.
- **Cluster deletion** — the whole `skills/integrations/jira/scripts/` subtree (17 executables, 5 libraries, 3 ADF data assets, `EXIT_CODES.md`, 21 suites, `mock-jira-server.py`, 148 fixtures), plus the `integrations` task and its floor and the `_DUAL_USE_SCRIPTS` exemplar. The `jira-client` differential no longer shells to the deleted ADF drivers — Phase 0 (on the Linear branch) froze it to a committed oracle.
- **Two build-system guards extended** — `skill_keyword_parity` and `skill_write_gate` now cover the jira provider (`lint:integration-skills:check`).
- **Jira artefacts** — the removal-set (197-file breakdown, revival anchor), suite-audit (21 suites → Rust coverage), and the jira sections of the divergences and fixture-reconciliation ledgers.

## Context

Implements the Jira half of `work-item:0211` per `meta/plans/2026-08-19-0211-integration-binaries-and-bash-cluster-retirement.md` (Phases 3–4). Parent epic `work-item:0171`; unblocked by 0210 (the client crates) and 0212 (the work-item cutover). Stacked on the Linear cutover PR #77 — the Jira track depends on the Phase 0 ADF-oracle freeze and the `cli-test-support` crate that live on the Linear branch.

## Testing

- [x] Full local CI mirror green at the Phase 4 boundary: `mise run` exits 0 end to end (the one failure across runs was `design-adapters::spawn_properties …bootstrap_log`, a documented load-flake that passes 3/3 in isolation and is unrelated to this work).
- [x] `jira-client` (196/196) and `jira-cli` (59/59) pass, including the widened `create`/`update` payloads, custom-field coercion, principal resolution, exit-code parity, keyword surface, and per-flow goldens.
- [x] `jira-client` is green with the cluster deleted — the frozen ADF oracle holds under real deletion.
- [x] `mise run check`, `build-system:check`, `dispatch-coherence`, `integration-skills`, and the stricter server rustfmt all green.
- [ ] Live-tenant spot-check: run each subcommand against a disposable Jira project and diff against 0210's committed contract evidence (needs a credentialed sandbox tenant).

## Notes for Reviewers

- **Stacked PR** — base is `0211-linear-integration-binary-and-cutover` (#77), not `main`; review #77 first. The residue-retirement PR is stacked on top of this one and completes 0211. GitHub reports a large diff dominated by the bash-cluster deletion (`gh pr diff 78` may 406 as too-large — use `gh pr view 78 --json changedFiles,additions,deletions`).
- **Deliberate surface widening** — Phase 3 shipped a thin `create`/`update`; the surface was widened back to bash parity before repointing so the skills keep their capability. This does not re-open the mutation compose seam for the sync port — resolution is binary-side, the port stays minimal. Recorded as an amendment in `meta/inventories/0211-divergences.md`.
- **Recorded divergences worth a look** — `comment` drops `--render-adf`/`--no-editor`; an ambiguous `transition` directs to `--transition-id` rather than printing a candidate table; `create` defaults a missing type to `Task` rather than refusing; `resolve-fields` and `work create --push --dry-run` share only the project source. All are in the divergences ledger, each naming a passing test.
