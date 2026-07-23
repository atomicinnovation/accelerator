#!/usr/bin/env bash

# Shared path and template key arrays.
#
# Sourced transitively via config-common.sh so the arrays are available
# to config-dump.sh and any future config script that sources
# config-common.sh. Centralising the definitions here means the next
# default rename is a one-line edit at this site rather than a
# grep-and-replace across the consumer surface.
#
# Scope note: this file centralises PATH, TEMPLATE, WORK, REVIEW, AGENT and
# VISUALISER *keys* (and their defaults) — the whole shell catalogue the Rust
# `config` crate is drift-tested against. The review and agent arrays moved
# here from config-dump.sh when that script was retired, so the catalogue
# drift test can still read them via config-common.sh after deletion.
#
# config-read-path.sh sources this file directly (it cannot afford the VCS
# detection overhead pulled in via config-common.sh). All other consumers
# should source config-common.sh, which sources this file transitively.

# shellcheck disable=SC2034
# (variables are exported-by-sourcing; consumers are invisible to a
# per-file lint)

PATH_KEYS=(
  "paths.plans"
  "paths.research_codebase"
  "paths.decisions"
  "paths.prs"
  "paths.validations"
  "paths.review_plans"
  "paths.review_prs"
  "paths.review_work"
  "paths.templates"
  "paths.work"
  "paths.notes"
  "paths.tmp"
  "paths.integrations"
  "paths.research_design_inventories"
  "paths.research_design_gaps"
  "paths.global"
  "paths.research_issues"
)

PATH_DEFAULTS=(
  "meta/plans"
  "meta/research/codebase"
  "meta/decisions"
  "meta/prs"
  "meta/validations"
  "meta/reviews/plans"
  "meta/reviews/prs"
  "meta/reviews/work"
  ".accelerator/templates"
  "meta/work"
  "meta/notes"
  ".accelerator/tmp"
  ".accelerator/state/integrations"
  "meta/research/design-inventories"
  "meta/research/design-gaps"
  "meta/global"
  "meta/research/issues"
)

# Schema doc-type → path key (parallel arrays; bash 3.2 — no associative
# arrays). The single canonical link between a templates-schema.tsv type and the
# config-defaults.sh path key whose directory holds artifacts of that type.
# Drives the corpus validator's allowlist (and, transitively, doc-type
# inference). DOC_TYPE_NAMES MUST equal the `type` column of
# templates-schema.tsv; each DOC_TYPE_PATH_KEYS entry MUST be the bare suffix of
# a member of PATH_KEYS (i.e. "paths.<entry>" exists) — both are pinned by a
# coherence guard in test-config-read-doc-type-paths.sh.
DOC_TYPE_NAMES=(
  "work-item" "plan" "plan-validation" "pr-description" "adr"
  "codebase-research" "issue-research" "design-inventory" "design-gap"
  "plan-review" "work-item-review" "pr-review" "note"
)
DOC_TYPE_PATH_KEYS=(
  "work" "plans" "validations" "prs" "decisions"
  "research_codebase" "research_issues" "research_design_inventories"
  "research_design_gaps" "review_plans" "review_work" "review_prs" "notes"
)

TEMPLATE_KEYS=(
  "templates.plan"
  "templates.codebase-research"
  "templates.adr"
  "templates.validation"
  "templates.pr-description"
  "templates.work-item"
  "templates.rca"
  "templates.design-inventory"
  "templates.design-gap"
  "templates.plan-review"
  "templates.work-item-review"
  "templates.pr-review"
  "templates.note"
)

WORK_KEYS=(
  "work.integration"
  "work.id_pattern"
  "work.default_project_code"
)

WORK_DEFAULTS=(
  ""
  "{number:04d}"
  ""
)

# Allowed non-empty values for work.integration. Empty value is additionally
# permitted by both consumers (unset is the default state). Adding a fifth
# integration is a single-line edit here; both config-read-work.sh (hard-fail
# validation) and config-dump.sh (non-fatal annotation) source this array.
WORK_INTEGRATION_VALUES=(
  "jira"
  "linear"
  "trello"
  "github-issues"
)

# Review keys with their catalogue defaults (bash-3.2 has no associative
# arrays; REVIEW_DEFAULTS must stay index-aligned with REVIEW_KEYS). Drift-tested
# against the Rust catalogue key-for-key.
REVIEW_KEYS=(
  "review.max_inline_comments"
  "review.min_lenses"
  "review.max_lenses"
  "review.dedup_proximity"
  "review.core_lenses"
  "review.disabled_lenses"
  "review.pr_request_changes_severity"
  "review.plan_revise_severity"
  "review.plan_revise_major_count"
  "review.work_item_revise_severity"
  "review.work_item_revise_major_count"
)

REVIEW_DEFAULTS=(
  "10"
  "4"
  "8"
  "3"
  "[architecture, code-quality, test-coverage, correctness]"
  "[]"
  "critical"
  "critical"
  "3"
  "critical"
  "2"
)

# The namespace every default agent name carries. Kept here rather than in
# config-common.sh so AGENT_DEFAULTS below can reference it — config-defaults.sh
# is sourced before config-common.sh's own body runs.
AGENT_PREFIX="accelerator:"

AGENT_KEYS=(
  "agents.reviewer"
  "agents.browser-analyser"
  "agents.browser-locator"
  "agents.codebase-locator"
  "agents.codebase-analyser"
  "agents.codebase-pattern-finder"
  "agents.documents-locator"
  "agents.documents-analyser"
  "agents.web-search-researcher"
)

AGENT_DEFAULTS=(
  "${AGENT_PREFIX}reviewer"
  "${AGENT_PREFIX}browser-analyser"
  "${AGENT_PREFIX}browser-locator"
  "${AGENT_PREFIX}codebase-locator"
  "${AGENT_PREFIX}codebase-analyser"
  "${AGENT_PREFIX}codebase-pattern-finder"
  "${AGENT_PREFIX}documents-locator"
  "${AGENT_PREFIX}documents-analyser"
  "${AGENT_PREFIX}web-search-researcher"
)

# Visualiser keys that DO carry a catalogue default. These mirror VISUALISER_KEYS
# in the Rust catalogue (cli/config) and are drift-tested against it key-for-key;
# config-dump.sh renders them with their defaults. The visualiser server keeps a
# matching runtime fallback in its own crate (server/src/config.rs) because it
# cannot depend on the config crate — this catalogue is the authoritative
# declaration and write-visualiser-config.sh sources the kanban default from
# here. VISUALISER_DEFAULTS is index-aligned (bash 3.2 has no associative arrays).
VISUALISER_KEYS=(
  "visualiser.kanban_columns"
  "visualiser.idle_timeout"
)

VISUALISER_DEFAULTS=(
  "[draft, ready, in-progress, review, done, blocked, abandoned]"
  "8h"
)

# Integration and tool config keys read ad-hoc by their own consumers
# (jira-auth.sh, linear-auth.sh, the visualiser launcher) rather than through the
# catalogue. Unlike VISUALISER_KEYS above, these carry NO catalogue default — an
# unset key means "the consumer's own default applies" — so they live here as a
# plain registry, NOT in the Rust catalogue or the drift-tested key count.
# config-dump.sh iterates this to surface them; test-config.sh pins it to the
# consumers (every key here is read) and to the docs (every key is documented).
# Adding a key a consumer reads means adding it here — or to VISUALISER_KEYS if it
# has a default — or the drift test fails.
EXTRA_KEYS=(
  "jira.site"
  "jira.email"
  "jira.token"
  "jira.token_cmd"
  "linear.token"
  "linear.token_cmd"
  "visualiser.editor"
  "visualiser.editor_project"
  "visualiser.binary"
)
