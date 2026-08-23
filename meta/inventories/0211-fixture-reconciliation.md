---
type: inventory
id: "0211-fixture-reconciliation"
title: "0211 Fixture Reconciliation Ledger"
date: "2026-08-22T00:00:00+00:00"
author: Toby Clemson
work_item_id: "work-item:0211"
parent: "work-item:0211"
tags: [jira, linear, integrations, fixtures, cutover]
schema_version: 1
---

# 0211 Fixture Reconciliation Ledger

Every bash-cluster fixture is either carried into the Rust corpus (where a Rust
test consumes it) or ledgered here with a reason, per Decision 15. Silence is
impossible: each section's row count is pinned against the pre-deletion file
list, and a "ported" row means "consumed" — a Rust test drives it.

Sections fill in as their phase lands:

- **ADF samples** (Phase 0): the 43 `skills/integrations/jira/scripts/test-fixtures/adf-samples/` files reconciled against the 56 committed `cli/jira-client/tests/fixtures/adf/` cases.
- **Linear scenarios** (Phase 1/2): the 40 `skills/integrations/linear/scripts/test-fixtures/scenarios/` files.
- **Jira scenarios** (Phase 3/4): the 95 `skills/integrations/jira/scripts/test-fixtures/scenarios/` files.

---

## ADF samples — 43 files (Phase 0)

Disposition of every file under
`skills/integrations/jira/scripts/test-fixtures/adf-samples/`. A "represented"
row names the committed case in `cli/jira-client/tests/fixtures/adf/` that
already exercises the same condition; a "ported" row names the new case that
consumes it; a "dropped" row states why porting adds no differential coverage.

| File | Disposition | Represented by / ported as / reason |
|---|---|---|
| `bold-italic-asterisk.md` | represented | `assemble-inline-priority` (`**strong** *em*`) |
| `bold-italic-asterisk.adf.json` | represented | `assemble-inline-priority` |
| `bold-italic-code-link.md` | represented | `assemble-inline-priority` (code + strong + em + link) |
| `bold-italic-code-link.adf.json` | represented | `assemble-inline-priority` |
| `bullet-list-flat.md` | represented | `assemble-bullet-list` |
| `bullet-list-flat.adf.json` | represented | `assemble-bullet-list` |
| `checklist-mixed.md` | represented | `assemble-task-list` |
| `checklist-mixed.adf.json` | represented | `assemble-task-list` |
| `code-block-no-lang.md` | represented | `assemble-code-fence-no-language` |
| `code-block-no-lang.adf.json` | represented | `assemble-code-fence-no-language` |
| `code-block-with-lang.md` | represented | `assemble-code-fence` (language variant) |
| `code-block-with-lang.adf.json` | represented | `assemble-code-fence` |
| `crlf-input.md` | represented | `assemble-crlf-line-endings` |
| `empty-doc.md` | represented | `assemble-empty-input` |
| `empty-doc.adf.json` | represented | `render-empty-doc` |
| `hard-break.md` | represented | `assemble-hard-break` |
| `hard-break.adf.json` | represented | `assemble-hard-break` |
| `headings-h1-to-h6.md` | represented | `assemble-headings` |
| `headings-h1-to-h6.adf.json` | represented | `assemble-headings` |
| `inline-combinations.md` | represented | `assemble-link-nested-marks` (link wrapping marks) |
| `inline-combinations.adf.json` | represented | `assemble-link-nested-marks` |
| `mixed-asterisk-emphasis.md` | represented | `assemble-inline-priority` (`***both***`) |
| `mixed-asterisk-emphasis.adf.json` | represented | `assemble-inline-priority` |
| `mixed-everything.md` | represented | constituent elements covered by `assemble-headings`, `assemble-inline-priority`, `assemble-mixed-lists-flush`, `assemble-code-fence`, `assemble-hard-break`; the integration-of-all adds no differential coverage the parts do not |
| `mixed-everything.adf.json` | represented | as above |
| `ordered-list-flat.md` | represented | `assemble-ordered-order-always-one` |
| `ordered-list-flat.adf.json` | represented | `assemble-ordered-order-always-one` |
| `paragraph-only.md` | represented | `assemble-paragraph` |
| `paragraph-only.adf.json` | represented | `assemble-paragraph` |
| `placeholder-collision.md` | **ported** | `assemble-placeholder-collision` (input.md) — literal text matching the placeholder format round-trips; no committed case exercised it |
| `placeholder-collision.adf.json` | **ported** | `assemble-placeholder-collision` (expected.adf.json) — the independent anchor cross-checking the frozen capture |
| `reject-blockquote.md` | represented | `assemble-reject-blockquote` |
| `reject-control-chars.md` | represented | `assemble-reject-control-byte` |
| `reject-jq-injection.md` | dropped | jq string interpolation is a bash-pipeline concern; the Rust `markdown_to_document` serialises text through serde_json, which escapes every text node uniformly (exercised by every `assemble-*` case), so a dedicated differential case adds no coverage the serde path does not already carry |
| `reject-nested-list.md` | represented | `assemble-reject-nested-list` |
| `reject-table.md` | represented | `assemble-reject-table` |
| `underscore-warning.md` | represented | `assemble-notice-underscores` (the `__ __` notice) |
| `underscore-warning.adf.json` | represented | `assemble-notice-underscores` |
| `underscores-as-literals.md` | represented | the `__…__` notice is `assemble-notice-underscores`; the `snake_case`/`_leading_trailing_` literals are ordinary paragraph text carried by `assemble-paragraph` |
| `underscores-as-literals.adf.json` | represented | as above |
| `unsupported-mention.adf.json` | represented | `render-inline-placeholders` (mention → `[unsupported ADF inline: mention]`) |
| `unsupported-panel.adf.json` | represented | `render-block-placeholders` (panel → `[unsupported ADF node: panel]`) |
| `.gitkeep` | dropped | directory placeholder, not a fixture |

**Count**: 43 files — 39 represented, 2 ported (one scenario,
`placeholder-collision`, consuming its `.md` and `.adf.json`), 2 dropped
(`reject-jq-injection.md` and `.gitkeep`). The corpus grows from 56 to **57**
cases; `cli/jira-client/tests/adf_oracle_manifest.rs` pins that count.

---

## Linear scenarios — 40 files (Phase 1)

Disposition of every file under
`skills/integrations/linear/scripts/test-fixtures/scenarios/`. A **ported** row
names the `cli/linear-cli/tests/fixtures/scenarios/` fixture that consumes it and
the flow test that drives it; a **superseded** row names the existing
`cli/linear-client` crate test that already drives the same condition against its
own fixture — the binary is a thin adapter, so a read/pagination/error-
classification condition is a client-crate concern, and re-porting it into
`linear-cli` would duplicate coverage the client already carries. Every ported
fixture is pinned as consumed by `scenario_inventory.rs`.

| File | Disposition | Ported as / superseded by |
|---|---|---|
| `attach-link-200.json` | **ported** | `attach-link-200.json` — `flow_attach::attach_link_posts_one_attachment_create` |
| `attach-binary-success.json.tmpl` | **ported** | `attach-binary-success.json` — `flow_attach::attach_file_uploads_then_registers_across_two_posts` (the multi-POST + PUT case; `__MOCK_URL__` templated at install) |
| `bad-request-400.json` | **ported** | `bad-request-400.json` — `flow_errors::network_classes_route_to_their_shared_codes` (400 → 34) |
| `bearer-401.json` | **ported** | `bearer-401.json` — `flow_errors::network_classes_route_to_their_shared_codes` (401 → 11) |
| `comment-201-capture.json` | **ported** | `comment-201.json` — `flow_comment::comment_add_posts_the_body_and_reports_the_added_keyword` |
| `create-201-capture.json` | **ported** | `create-201.json` — `flow_create::create_emits_the_created_keyword_and_identifier` |
| `create-malformed-identifier-201.json` | **ported** | `create-malformed-identifier-201.json` — `flow_create::create_with_an_unusable_identifier_fails_closed` (the writeback-failed orphan case) |
| `issue-update-200.json` | **ported** | `issue-update-200.json` — `flow_update::update_posts_the_fields_and_reports_the_updated_keyword` |
| `search-filter-state-200.json` | **ported** | `search-filter-state-200.json` — `flow_search` + `stdout_goldens::search_stdout_matches_the_golden` |
| `show-issue-200.json` | **ported** | `show-issue-200.json` — `flow_show` + `stdout_goldens::show_stdout_matches_the_golden` |
| `show-issue-404.json` | **ported** | `show-issue-404.json` — `flow_errors::show_not_found_routes_to_its_own_code` (200 `issue: null` → 82) |
| `teams-200.json` | **ported** | `teams-200.json` — `flow_init::init_list_teams_renders_the_teams_with_the_listed_keyword` |
| `transition-update-200.json` | **ported** | `transition-update-200.json` — `flow_transition::transition_resolves_the_state_and_posts_the_stateid` |
| `viewer-200.json` | **ported** | `viewer-200.json` — `flow_init::init_verify_never_emits_the_token…` + `flow_errors` |
| `attach-binary-bad-upload-url.json` | superseded | `linear-client/tests/attach.rs::an_upload_url_off_linear_app_is_refused_before_any_bytes_move` (`BadUploadUrl`; the binary maps it to 135, pinned by `exit_codes_parity.rs`) |
| `attach-binary-crlf-header.json.tmpl` | superseded | `attach.rs::an_echoed_header_carrying_crlf_is_refused` |
| `attach-binary-redirect.json.tmpl` | superseded | `attach.rs::a_redirect_response_to_the_put_is_refused_rather_than_followed` |
| `attach-binary-register-fail.json.tmpl` | superseded | `attach.rs::a_step_three_failure_after_a_successful_put_reports_an_orphaned_asset` (`RegisterFailed` → 137) |
| `attach-binary-upload-fail.json.tmpl` | superseded | `attach.rs` upload-failure path (`UploadFailed` → 136) |
| `bad-request-mentions-10000.json` | superseded | `linear-client/tests/classify.rs` (the mentions-10000 bad-request classification) |
| `complexity-400.json` | superseded | `classify.rs` (complexity → 36) |
| `graphql-auth-error-200.json` | superseded | `classify.rs` (200-body auth error → 11) |
| `graphql-errors-200.json` | superseded | `classify.rs` (200-body `errors[]` classification) |
| `ratelimited-400-no-reset-header.json` | superseded | `linear-client/tests/transport.rs` (rate-limit without a reset header) |
| `ratelimited-400-then-200.json` | superseded | `transport.rs` (retry after a 429, then success) |
| `ratelimited-exhausted.json.tmpl` | superseded | `transport.rs` (retry exhaustion) |
| `fetch-keys-complete-200.json` | superseded | `linear-client/tests/port.rs` (`fetch_all` complete) |
| `fetch-keys-truncated-200.json` | superseded | `port.rs` (`fetch_all` truncated) |
| `paginate-3x50.json` | superseded | `port.rs` (multi-page bulk read) |
| `paginate-nonadvancing.json` | superseded | `port.rs` (non-advancing-cursor guard) |
| `paginate-runaway.json` | superseded | `port.rs` (runaway-pagination guard) |
| `paginate-zero.json` | superseded | `port.rs` (empty result) |
| `search-paginate-200.json` | superseded | `linear-client/tests/search_projection.rs` (Decision 20 projection paging) |
| `team-no-states-200.json` | superseded | `linear-client/tests/discovery.rs` (a team with no states) |
| `team-states-200.json` | superseded | `discovery.rs` (`discover_team` states) |
| `team-states-y-200.json` | superseded | `discovery.rs` (a second team's states) |
| `create-response-dropped-200.json` | superseded | `linear-client/tests/projection_corpus.rs` (create response missing projected fields) |
| `issue-update-dropped-200.json` | superseded | `projection_corpus.rs` (update response missing projected fields) |
| `viewer-slow-200.json` | superseded | `linear-client/tests/timeouts.rs` (a slow response exercising the read timeout) |
| `update-200-capture.json` | superseded | ported `issue-update-200.json` above — the same `issueUpdate` condition, capture-body variant |

**Count**: 40 files — **14 ported** into `cli/linear-cli/tests/fixtures/
scenarios/` (each consumed, pinned by `scenario_inventory.rs`), **26
superseded** by an existing `cli/linear-client` crate test driving the same
condition. Porting all 40 mechanically is declined (Decision 15): it would
re-create test surface the client crate already carries.
