# All Skills

Every user-invokable Accelerator skill, grouped by family. Each entry links to
its reference subsection on the family or concept page that homes it.

Note: `review-pr`, `review-plan`, and `review-work-item` run the multi-lens
[Review System](review-system.md); see that page for the lens catalogue.

## Development Loop

- [`/accelerator:research-codebase`](../development-loop.md#research-codebase) — Conduct
  comprehensive codebase research by spawning parallel sub-agents and
  synthesising findings into a research document.
- [`/accelerator:create-plan`](../development-loop.md#create-plan) — Create
  detailed implementation plans through interactive, iterative collaboration.
- [`/accelerator:implement-plan`](../development-loop.md#implement-plan) — Execute
  an approved implementation plan from the configured plans directory.

## Planning

- [`/accelerator:research-issue`](planning.md#research-issue) — Investigate
  production issues and bugs through hypothesis-driven debugging.
- [`/accelerator:create-note`](planning.md#create-note) — Interactively
  capture a short-form note.
- [`/accelerator:conduct-spike`](planning.md#conduct-spike) — Interactively
  conduct a time-boxed spike — collaboratively reduce uncertainty through
  discussion mixed with agent-driven research (and small throwaway prototypes
  where a question is empirical), then record the outcome on the spike's work
  item.
- [`/accelerator:review-plan`](planning.md#review-plan) — Review an
  implementation plan through multiple quality lenses and collaboratively
  iterate based on findings.
- [`/accelerator:stress-test-plan`](planning.md#stress-test-plan) — Interactively
  stress-test an implementation plan by grilling the user on decisions, edge
  cases, and assumptions to find issues, inconsistencies, and gaps before
  implementation begins.
- [`/accelerator:validate-plan`](planning.md#validate-plan) — Validate that
  an implementation plan was correctly executed by verifying success criteria
  and identifying deviations.

## Work Items

- [`/accelerator:create-work-item`](work-items.md#create-work-item) — Interactively
  create a well-formed work item.
- [`/accelerator:extract-work-items`](work-items.md#extract-work-items) — Extract
  work items in batch from existing documents (specs, PRDs, research, plans,
  meeting notes, design docs).
- [`/accelerator:refine-work-item`](work-items.md#refine-work-item) — Interactively
  refine a work item by decomposing it into children, enriching it with
  codebase context, sharpening its acceptance criteria, sizing it, or linking
  it to dependencies.
- [`/accelerator:review-work-item`](work-items.md#review-work-item) — Review
  a work item through multiple quality lenses and collaboratively iterate based
  on findings.
- [`/accelerator:stress-test-work-item`](work-items.md#stress-test-work-item) — Interactively
  stress-test a work item by grilling the user on scope, assumptions,
  acceptance criteria, edge cases, and dependencies to surface issues, gaps,
  and flawed assumptions before implementation is planned.
- [`/accelerator:update-work-item`](work-items.md#update-work-item) — Update
  fields (status, priority, tags, parent, etc.) of an existing work item.
- [`/accelerator:list-work-items`](work-items.md#list-work-items) — List and
  filter work items from the configured work directory.
- [`/accelerator:sync-work-items`](work-items.md#sync-work-items) — Reconcile
  local work items in meta/work/ with the active remote tracker named by
  work.integration.

## Issue Trackers

- [`/accelerator:init-jira`](issue-trackers.md#init-jira) — Set up the Jira
  Cloud integration for this project.
- [`/accelerator:search-jira-issues`](issue-trackers.md#search-jira-issues) — Use
  this skill whenever the user wants to search, list, or filter Jira tickets
  — by assignee, status, label, project, type, component, reporter, parent,
  or free text — even if they say 'find', 'show me', 'what's open', 'list my
  tickets', or similar phrasing rather than 'search Jira'.
- [`/accelerator:show-jira-issue`](issue-trackers.md#show-jira-issue) — Use
  this skill when the user asks about a specific Jira issue by key (e.g.
  PROJ-123, ENG-456) — for viewing the description, status, comments,
  transitions, or any other field.
- [`/accelerator:create-jira-issue`](issue-trackers.md#create-jira-issue) — Use
  this skill only when the user explicitly invokes /create-jira-issue to create
  a new Jira issue.
- [`/accelerator:update-jira-issue`](issue-trackers.md#update-jira-issue) — Use
  this skill only when the user explicitly invokes /update-jira-issue to modify
  an existing Jira issue.
- [`/accelerator:comment-jira-issue`](issue-trackers.md#comment-jira-issue) — Use
  this skill only when the user explicitly invokes /comment-jira-issue to add,
  list, edit, or delete comments on a Jira issue.
- [`/accelerator:transition-jira-issue`](issue-trackers.md#transition-jira-issue) — Use
  this skill only when the user explicitly invokes /transition-jira-issue to
  move a Jira issue through its workflow by state name.
- [`/accelerator:attach-jira-issue`](issue-trackers.md#attach-jira-issue) — Use
  this skill only when the user explicitly invokes /attach-jira-issue to upload
  one or more local files as attachments to a Jira issue.
- [`/accelerator:init-linear`](issue-trackers.md#init-linear) — Set up the
  Linear integration for this project.
- [`/accelerator:search-linear-issues`](issue-trackers.md#search-linear-issues) — Use
  this skill whenever the user wants to search, list, or filter Linear issues
  — by state, assignee, label, or free text — even if they say 'find',
  'show me', 'what's open', 'list my issues', or similar phrasing rather than
  'search Linear'.
- [`/accelerator:show-linear-issue`](issue-trackers.md#show-linear-issue) — Use
  this skill when the user asks about a specific Linear issue by identifier
  (e.g. BLA-123, ENG-456) — for viewing the description, state, assignee, or
  comments.
- [`/accelerator:create-linear-issue`](issue-trackers.md#create-linear-issue) — Use
  this skill only when the user explicitly invokes /create-linear-issue to
  create a new Linear issue from a local work-item file.
- [`/accelerator:update-linear-issue`](issue-trackers.md#update-linear-issue) — Use
  this skill only when the user explicitly invokes /update-linear-issue to
  change fields on an existing Linear issue (title, description, state,
  assignee, priority).
- [`/accelerator:comment-linear-issue`](issue-trackers.md#comment-linear-issue) — Use
  this skill only when the user explicitly invokes /comment-linear-issue to add
  a Markdown comment to an existing Linear issue.
- [`/accelerator:transition-linear-issue`](issue-trackers.md#transition-linear-issue) — Use
  this skill only when the user explicitly invokes /transition-linear-issue to
  move an existing Linear issue to a different workflow state.
- [`/accelerator:attach-linear-issue`](issue-trackers.md#attach-linear-issue) — Use
  this skill only when the user explicitly invokes /attach-linear-issue to
  attach a link or a binary file to an existing Linear issue.

## Architecture Decision Records

- [`/accelerator:create-adr`](adrs.md#create-adr) — Interactively create an
  architecture decision record (ADR).
- [`/accelerator:review-adr`](adrs.md#review-adr) — Review an architecture
  decision record for quality and completeness, then accept, reject, or suggest
  revisions.
- [`/accelerator:extract-adrs`](adrs.md#extract-adrs) — Extract architecture
  decision records from existing meta documents (research, plans).

## VCS & PR

- [`/accelerator:commit`](vcs-and-pr.md#commit) — Create VCS commits for
  session changes.
- [`/accelerator:describe-pr`](vcs-and-pr.md#describe-pr) — Generate a
  comprehensive pull request description following the repository's standard
  template.
- [`/accelerator:review-pr`](vcs-and-pr.md#review-pr) — Review a pull request
  through multiple quality lenses and present a compiled analysis with inline
  comments.
- [`/accelerator:respond-to-pr`](vcs-and-pr.md#respond-to-pr) — Respond to
  pull request review feedback interactively, working through each item with
  verification and code changes.

## Design Convergence

- [`/accelerator:inventory-design`](design-convergence.md#inventory-design) — Generate
  a structured design inventory for a frontend source — tokens, components,
  screens, and features — by crawling it with code analysis, live Playwright
  inspection, or both.
- [`/accelerator:analyse-design-gaps`](design-convergence.md#analyse-design-gaps) — Compare
  two design inventories produced by inventory-design and emit a structured gap
  artifact whose prose paragraphs satisfy the extract-work-items cue-phrase
  contract.

## Config & Maintenance

- [`/accelerator:configure`](../configuration.md#configure) — View, create,
  or edit Accelerator plugin configuration.
- [`/accelerator:init`](../configuration.md#init) — Prepare a repository with
  the directories and gitignore entries that Accelerator skills expect.
- [`/accelerator:migrate`](../migrations.md#migrate) — Apply pending
  Accelerator meta-directory migrations to bring a repo into line with the
  latest plugin schema.

## Visualiser

- [`/accelerator:visualise`](../visualiser.md#visualise) — Open the
  accelerator meta visualiser.
