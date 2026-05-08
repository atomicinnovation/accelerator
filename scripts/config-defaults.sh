#!/usr/bin/env bash

# Shared path and template key arrays.
#
# Sourced transitively via config-common.sh so the arrays are available
# to config-dump.sh and any future config script that sources
# config-common.sh. Centralising the definitions here means the next
# default rename is a one-line edit at this site rather than a
# grep-and-replace across the consumer surface.
#
# Scope note: this file currently centralises only PATH and TEMPLATE
# *keys* (and the path defaults). Review-key DEFAULTS, AGENT_KEYS, and
# AGENT_DEFAULTS remain inline in config-dump.sh because they have no
# external consumers. DIR_KEYS/DIR_DEFAULTS in
# skills/config/init/scripts/init.sh use a different vocabulary
# (bare keys vs paths.*-prefixed) and are tracked for unification in a
# follow-on work item.
#
# config-read-path.sh sources this file directly (it cannot afford the VCS
# detection overhead pulled in via config-common.sh). All other consumers
# should source config-common.sh, which sources this file transitively.

# shellcheck disable=SC2034
# (variables are exported-by-sourcing; consumers are invisible to a
# per-file lint)

PATH_KEYS=(
  "paths.plans"
  "paths.research"
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
  "paths.design_inventories"
  "paths.design_gaps"
)

PATH_DEFAULTS=(
  "meta/plans"
  "meta/research"
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
  "meta/design-inventories"
  "meta/design-gaps"
)

TEMPLATE_KEYS=(
  "templates.plan"
  "templates.research"
  "templates.adr"
  "templates.validation"
  "templates.pr-description"
  "templates.work-item"
)
