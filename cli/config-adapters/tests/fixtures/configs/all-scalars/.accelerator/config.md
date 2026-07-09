---
paths:
  plans: custom/plans
  research_codebase: custom/research/codebase
  decisions: custom/decisions
  prs: custom/prs
  validations: custom/validations
  review_plans: custom/reviews/plans
  review_prs: custom/reviews/prs
  review_work: custom/reviews/work
  templates: custom/templates
  work: custom/work
  notes: custom/notes
  tmp: custom/tmp
  integrations: custom/state/integrations
  research_design_inventories: custom/research/design-inventories
  research_design_gaps: custom/research/design-gaps
  global: custom/global
  research_issues: custom/research/issues
templates:
  plan: tpl/plan.md
  codebase-research: tpl/codebase-research.md
  adr: tpl/adr.md
  validation: tpl/validation.md
  pr-description: tpl/pr-description.md
  work-item: tpl/work-item.md
  rca: tpl/rca.md
  design-inventory: tpl/design-inventory.md
  design-gap: tpl/design-gap.md
  plan-review: tpl/plan-review.md
  work-item-review: tpl/work-item-review.md
  pr-review: tpl/pr-review.md
  note: tpl/note.md
work:
  integration: linear
  id_pattern: PROJ-{number}
  default_project_code: PROJ
review:
  max_inline_comments: 20
  min_lenses: 5
  max_lenses: 9
  dedup_proximity: 6
  pr_request_changes_severity: major
  plan_revise_severity: major
  plan_revise_major_count: 7
  work_item_revise_severity: major
  work_item_revise_major_count: 8
agents:
  reviewer: custom:reviewer
  codebase-locator: custom:codebase-locator
  codebase-analyser: custom:codebase-analyser
  codebase-pattern-finder: custom:codebase-pattern-finder
  documents-locator: custom:documents-locator
  documents-analyser: custom:documents-analyser
  web-search-researcher: custom:web-search-researcher
---
body
