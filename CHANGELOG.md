# Changelog

## Unreleased

### Changed

- **BREAKING**: Renamed the `tickets` skill category to `work` and
  individual `ticket` references to `work-item`. Slash commands are
  now `/accelerator:create-work-item`,
  `/accelerator:extract-work-items`, `/accelerator:list-work-items`,
  `/accelerator:refine-work-item`, `/accelerator:review-work-item`,
  `/accelerator:stress-test-work-item`, and
  `/accelerator:update-work-item`. Default storage directory is
  `meta/work/`. Config keys `paths.work`/`paths.review_work` replace
  the former `paths.tickets`/`paths.review_tickets`. Template renamed
  `templates/ticket.md` → `templates/work-item.md`; frontmatter field
  `ticket_id` becomes `work_item_id`.
- **Upgrade procedure**: commit any pending changes to your
  repository, then run `/accelerator:migrate`. The skill renames
  `meta/tickets/` → `meta/work/` (preserving any custom
  `paths.tickets` directory location and rewriting only the
  config key), rewrites `ticket_id:` → `work_item_id:` in every
  file under the resolved work-item directory, and updates
  `.claude/accelerator*.md` config keys. The skill is destructive
  by default but refuses to run on a dirty working tree, and
  prints a one-line preview per pending migration before
  applying. Review the resulting `jj diff` / `git diff` before
  committing.

### Added

- **Work item management**: New `work/` skill category with four skills for
  capturing, discovering, and managing work items as structured documents
  under `meta/work/` (default; override via `paths.work`)
  - `create-work-item` — Interactively create a single work item (feature, bug,
    task, spike, or epic) through a collaborative, challenging conversation
    that contributes research and pushes back on under-specified inputs
    rather than transcribing what the user says
  - `extract-work-items` — Batch-extract work items from existing documents (specs,
    PRDs, research, plans, meeting notes, design docs), keeping
    source-derived content faithful while surfacing business-context gaps as
    assumptions, open questions, and drafting notes
  - `list-work-items` — List and filter work items by status, type, priority, tag,
    parent, or free-text title search. Supports natural language filters
    (`drafts`, `bugs in review`), explicit structured forms (`status todo`,
    `tagged backend`, `under 0042`), and hierarchy rendering with cycle
    detection. Read-only; no sub-agents spawned
  - `update-work-item` — Update frontmatter fields (status, priority, tags,
    parent, title, etc.) on an existing work item. Shows a diff preview and
    requires confirmation before writing. Syncs body labels
    (`**Status**:`, `**Type**:`, `**Priority**:`, `**Author**:`) and the
    H1 heading when the corresponding fields change. No status transition
    enforcement — arbitrary changes are allowed
- **Work item template**: New `templates/work-item.md` default template with YAML
  frontmatter (`work_item_id`, `title`, `date`, `author`, `type`, `status`,
  `priority`, `parent`, `tags`) and structured body sections (Summary,
  Context, Requirements, Acceptance Criteria, Open Questions, Dependencies,
  Assumptions, Technical Notes, Drafting Notes, References). Overridable via
  `templates.work-item`.
- **Work item numbering and frontmatter helpers**: Supporting shell scripts in
  `skills/work/scripts/`
  - `work-item-next-number.sh` — Assigns the next sequential work item number from
    the configured work items directory, enforcing a 4-digit ceiling
  - `work-item-read-field.sh` — Generic YAML frontmatter field reader that uses
    bash prefix matching to avoid regex metacharacter injection
  - `work-item-read-status.sh` — Thin convenience wrapper over
    `work-item-read-field.sh` for the common status lookup
  - `work-item-update-tags.sh` — Tag array mutation helper that parses the
    current value, applies add/remove operations, and emits canonical
    flow-style format. Detects block-style arrays and rejects them with
    guidance
  - `work-item-template-field-hints.sh` — Extracts hint values for a given
    frontmatter field from the work item template's trailing comments, with
    hardcoded fallback for type, status, and priority when comments are
    absent
- **`paths.review_work` configuration key**: New configurable path
  (default: `meta/reviews/work`) for future work item review artifacts,
  included in `/accelerator:init` directory creation and reported in
  `config-dump.sh`.
- **Work item review system**: Five-lens work item review capability
  (`/review-work-item`) combining completeness, testability, clarity, scope, and
  dependency lenses.
  - `review-work-item` — Orchestrator skill that reviews a work item through
    completeness, testability, clarity, scope, and dependency lenses in
    parallel, aggregates findings into an APPROVE/REVISE/COMMENT verdict,
    persists results to
    `meta/reviews/work/{work-item-stem}-review-{N}.md`, and supports
    appendable re-review passes
  - `completeness-lens` — Work item review lens for evaluating section presence,
    content density, type-appropriate content (bug/story/spike/epic), and
    frontmatter integrity
  - `testability-lens` — Work item review lens for evaluating whether Acceptance
    Criteria and requirements admit a concrete verification strategy
  - `clarity-lens` — Work item review lens for evaluating unambiguous referents,
    internal consistency, and jargon/acronym handling
  - `scope-lens` — Work item review lens for evaluating work item sizing,
    decomposition, and orthogonality of requirements
  - `dependency-lens` — Work item review lens for evaluating whether implied
    couplings (blockers, consumers, external systems, ordering) are
    explicitly captured
  - `work-item-review-output-format` — Output format specification for work item
    review agents (JSON schema, location examples anchored to work item sections)
- **Work item review configuration keys**: Two new keys in the `review` section
  - `work_item_revise_severity` (default: `critical`) — minimum severity for REVISE
  - `work_item_revise_major_count` (default: `2`) — major-findings count to trigger REVISE
- **Per-review-type lens partitioning**: `config-read-review.sh` now accepts
  `work-item` as a third mode, emitting only the catalogue for the active mode.
  The 13 code-review lenses remain exclusive to `pr` and `plan`; the five
  work item lenses are exclusive to `work-item`
- **`applies_to` field for custom lenses**: Custom lenses in
  `.claude/accelerator/lenses/` can now declare an optional
  `applies_to: [pr, plan, work-item]` frontmatter field to restrict their
  appearance to specific review modes. Absent means all modes
  (backwards-compatible)
- **Review system ADRs**: Extracted nine architecture decision records
  (ADR-0002 through ADR-0010) from existing research and planning documents,
  covering the three-layer review architecture, PBR lens design, lens
  catalogue expansion, the single generic reviewer agent pattern, the
  structured agent output contract, divergent verdict semantics for plan
  and PR reviews, the shared temp directory for PR diff delivery, dual-gate
  finding deduplication, and atomic review posting via the GitHub REST API.
  Related work items (0001–0007, 0009, 0014) and `skills/decisions/create-adr`
  now cross-reference the corresponding ADRs.

## 1.18.0 — 2026-04-17

_Versions 1.12.0 through 1.18.0 were iterative build-system and release-pipeline
changes and are not recorded as separate entries. The combined work is
described below._

### Added

- **Build system**: New Python-based build system under `tasks/` using
  [invoke](https://www.pyinvoke.org/), bootstrapped through `mise.toml` with
  pinned `uv`, Python, and `gh` toolchains. `pyproject.toml` declares a `build`
  dependency group (`invoke`, `keepachangelog`, `rich`, `semver`) installed via
  `uv sync` on `mise install`.
- **Integration test task**: `mise run test` (backed by
  `invoke test.integration`) runs the existing config and ADR shell test
  suites so they execute consistently in CI and locally.
- **Continuous integration pipeline**: `.github/workflows/main.yml` runs the
  test suite on every push and pull request.
- **Prerelease automation**: The CI pipeline now bumps the prerelease
  identifier, tags the commit, and pushes to `main` after tests pass
  (`mise run prerelease` → `invoke prerelease`). Git identity is configured
  during the job to attribute automated commits to "Atomic Maintainers".
- **Release automation**: The CI pipeline now promotes prereleases to full
  releases on `main` pushes (`mise run release` → `invoke release`). The
  release task finalises the semver version, updates
  `.claude-plugin/marketplace.json` to point at the released tag, marks the
  `Unreleased` changelog section as the new version via `keepachangelog`,
  tags and pushes, creates a GitHub release with auto-generated notes via
  `gh release create`, and then bumps to the next minor prerelease ready for
  continued development.

### Fixed

- **Tests failing on bash 5.2**: Test harness adjustments so suites pass on
  the bash version available on GitHub Actions runners.

## 1.11.0 — 2026-04-15

### Fixed

- **`describe-pr` hardcoded `/tmp` path**: The temporary PR body file was
  written to `/tmp/pr-body-{number}.md` instead of using the configured tmp
  directory. Now uses `{tmp directory}` resolved via `config-read-path.sh`,
  consistent with `review-pr` and `init`. Adds a `mkdir -p` step to ensure the
  directory exists.
- **`review-pr` template variable resolution**: Added explicit substitution
  instructions after the bold-label definitions and a reminder at the sub-agent
  prompt composition site, preventing silent fallback to `/tmp` when the LLM
  fails to resolve `{tmp directory}` placeholders — particularly in two-hop
  sub-agent prompt composition.
- **Bare agent name defaults**: Default agent names emitted by configuration
  scripts and skill fallback lines now include the `accelerator:` plugin prefix
  (e.g., `accelerator:reviewer` instead of `reviewer`). Without the prefix, the
  Agent tool could not resolve bare names to the correct plugin-provided agent
  definitions. Affects `config-read-agents.sh`, `config-read-agent-name.sh`,
  `config-dump.sh`, and fallback lines in all 10 skills that reference agents.
  User-provided overrides are passed through unchanged.

## 1.10.0 — 2026-03-30

### Added

- **Template management subcommands**: New `/accelerator:configure templates`
  subcommands for inspecting, customising, and managing document templates
  without manually locating plugin internals
  - `templates list` — List all template keys with their resolution source
    (plugin default / user override / config path) and resolved file path
  - `templates show <key>` — Display a template's raw content with source
    metadata (no code fence wrapping, unlike `config-read-template.sh`)
  - `templates eject <key|--all>` — Copy plugin default template(s) to the
    user's templates directory for customisation; supports `--force` to
    overwrite existing files and `--dry-run` to preview changes
  - `templates diff <key>` — Show unified diff between a user's customised
    template and the plugin default
  - `templates reset <key>` — Remove a user's customised template to revert to
    the plugin default, with confirmation flow and config entry cleanup
  - `/accelerator:configure help` now includes a Template Management Commands
    reference table

### Fixed

- **Missing `templates.pr-description` in config dump**: `config-dump.sh` now
  includes the `pr-description` template key alongside the other four template
  keys
- **`config-summary.sh` and `config-detect.sh` test failures**: Tests for
  uninitialised repos now correctly account for the SessionStart initialisation
  detection added in v1.8.0

### Changed

- **Refactored template resolution into shared helpers**: The three-tier
  template resolution logic (config path → user override → plugin default) has
  been extracted from `config-read-template.sh` into reusable functions in
  `config-common.sh`: `config_enumerate_templates()`,
  `config_resolve_template()`, `config_format_available_templates()`, and
  `config_display_path()`. Existing behaviour is preserved.

## 1.9.0 — 2026-03-29

### Changed

- **Renamed `/accelerator:initialise` to `/accelerator:init`**: Shorter name
  for parity with Claude Code's own `/init` command. The skill directory,
  frontmatter name, config-summary hint, and README reference have all been
  updated. No backwards-compatibility shim — the old name was unreleased.

## 1.8.0 — 2026-03-29

### Added

- **`/accelerator:initialise` skill**: Prepares a consumer repository with all
  directories and gitignore entries that Accelerator skills expect. Creates all
  11 configured output directories with `.gitkeep` files, sets up the tmp
  directory with a self-contained `.gitignore`, and ensures
  `.claude/accelerator.local.md` is in the consumer's root `.gitignore`. Safe
  to run repeatedly — reports what was created versus what already existed.
- **`tmp` configurable path key**: New `paths.tmp` configuration key (default:
  `meta/tmp`) for ephemeral working data. The tmp directory uses an inner
  `.gitignore` to ignore its contents while remaining tracked by git.
- **SessionStart initialisation detection**: The SessionStart hook now detects
  when a consumer repository has not been initialised and suggests running
  `/accelerator:initialise`.

### Changed

- **`review-pr` ephemeral file location**: Ephemeral working files (diff,
  changed-files, PR description, commits, head SHA, repo info, review payload)
  are now written to `{tmp directory}/pr-review-{number}/` instead of
  `{pr reviews directory}/pr-review-{number}/`. Persistent review artifacts
  remain at `{pr reviews directory}/{number}-review-{N}.md`. After upgrading,
  you can safely remove any existing `pr-review-*/` directories under your PR
  reviews path (default: `meta/reviews/prs/`):
  ```
  rm -rf meta/reviews/prs/pr-review-*/
  ```

## 1.7.0 — 2026-03-28

### Added

- **Per-skill customisation**: Users can now provide skill-specific context and
  additional instructions by placing files in
  `.claude/accelerator/skills/<skill-name>/` directories
  - `context.md` — Skill-specific context injected after global project context;
    use for information only one skill needs
  - `instructions.md` — Additional instructions appended to the end of a skill's
    prompt; use to add steps, enforce conventions, or modify output format
  - Both files are optional and apply to all 13 user-facing skills (the
    `configure` skill is excluded)
  - SessionStart hook reports detected per-skill customisations and warns about
    unrecognised skill directory names
  - `/accelerator:configure help` documents the feature with examples and
    troubleshooting guidance
  - `/accelerator:configure view` enumerates active per-skill customisations

## 1.6.0 — 2026-03-27

### Added

- **Userspace configuration**: New `configure` skill and configuration
  infrastructure allowing per-project and per-user customisation via
  `.claude/accelerator.md` (team-shared) and `.claude/accelerator.local.md`
  (personal overrides) files using YAML frontmatter and markdown body
  - `agents:` section to override which agents skills spawn as sub-agents
    (e.g., swap in a custom reviewer or locator)
  - `review:` section to customise review behaviour — lens selection
    (`min_lenses`, `max_lenses`, `core_lenses`, `disabled_lenses`), verdict
    thresholds (`pr_request_changes_severity`, `plan_revise_severity`,
    `plan_revise_major_count`), and inline comment limits
  - `paths:` section to override where skills write output documents (plans,
    research, decisions, PRs, validations, reviews, templates, work items, notes)
  - `templates:` section to override document templates (plan, ADR, research,
    validation, PR description) with custom formats from a configurable
    templates directory
  - Custom review lenses via `.claude/accelerator/lenses/` with auto-discovery
  - Project context injected into all skills from the markdown body
  - `/accelerator:configure` skill with `view`, `create`, and `help` actions
  - Config preprocessing via shell scripts and a SessionStart hook, working
    around plugin permission limitations
- **Default document templates**: Extractable templates for plans, ADRs,
  research documents, validation reports, and PR descriptions in `templates/`
  directory, used as defaults when no user override is provided

### Improved

- **Reliable agent name resolution**: All 10 agent-spawning skills now resolve
  agent names via labeled variable definitions from `config-read-agents.sh`,
  replacing the non-deterministic override table pattern. Agent names are always
  emitted (with defaults when no overrides configured), and skills reference
  them via `{agent name agent}` variables. Includes a fallback instruction for
  graceful degradation if the preprocessor fails.
  - `implement-plan` and `validate-plan` now include `config-read-agents.sh`
    (previously missing despite spawning sub-tasks)
- **Dynamic operational paths**: All `mkdir`, `glob`, and artifact `target`
  fields now use dynamic `{directory}` variables from `config-read-path.sh`
  instead of hardcoded `meta/` paths
  - `respond-to-pr` now declares a `review_prs` path variable (previously
    missing)
  - `implement-plan`, `validate-plan`, `review-plan`, and `stress-test-plan`
    now declare a `plans` path variable
- **Review numeric defaults via preprocessor**: `config-read-review.sh` now
  always emits labeled variable definitions for all numeric and threshold
  values, even when they match defaults. Review skill prose references these
  variables instead of hardcoding numbers (fixes incorrect "6 to 8" default
  lens range — actual default is 4 to 8).
- **`describe-pr` template handling**: Switched from manual filesystem check to
  `config-read-template.sh`, consistent with all other template-using skills.
  The `templates.<name>` config key now supports `pr-description`.
- **Dynamic example paths**: Replaced ~14 hardcoded `meta/` paths in examples,
  argument hints, and illustrative text across 7 skills with dynamic
  `{directory variable}` references. Argument hints in `review-adr` and
  `extract-adrs` use generic descriptions since frontmatter cannot reference
  preprocessor variables.

### Fixed

- Emoji rendering issues in review output format skills
- `config-read-template.sh` error message now dynamically lists available
  templates instead of a hardcoded list

## 1.5.0 — 2026-03-23

### Added

- **Persistent review artifacts**: `review-plan` and `review-pr` now write
  structured review documents to `meta/reviews/` so findings survive across
  sessions and are visible to the whole team
  - `review-plan` writes to `meta/reviews/plans/{stem}-review-{N}.md` with
    YAML frontmatter, the full review summary, and per-lens results
  - `review-pr` writes to `meta/reviews/prs/{number}-review-{N}.md` with
    YAML frontmatter, inline comments, and per-lens results
  - `review-plan` checks for prior reviews when starting a new review cycle,
    enabling cross-session continuity
  - Re-reviews in `review-plan` append to the existing review file and update
    frontmatter (`verdict`, `review_pass`, `date`)
  - `documents-locator` agent now discovers review artifacts in
    `meta/reviews/`
- **Persistent validation artifacts**: `validate-plan` now writes validation
  reports to `meta/validations/` with YAML frontmatter (`type`, `target`,
  `result`, `status`), completing the plan lifecycle audit trail
  - A passing validation (`result: pass`) updates the plan's `status`
    frontmatter to `complete`
  - `documents-locator` agent now discovers validation artifacts in
    `meta/validations/`
- **Review cross-referencing in `respond-to-pr`**: When a structured review
  artifact exists at `meta/reviews/prs/{number}-review-*.md`, `respond-to-pr`
  loads it and uses severity, confidence, and lens data to inform triage
  categorisation
- **Frontmatter for plans**: `create-plan` now includes YAML frontmatter
  (`date`, `type`, `skill`, `work-item`, `status`) in the plan template
- **Frontmatter for PR descriptions**: `describe-pr` now includes YAML
  frontmatter (`date`, `type`, `skill`, `pr_number`, `pr_title`, `status`) in
  `meta/prs/` output, stripped before posting to GitHub

## 1.3.0 — 2026-03-18

_Versions 1.1.0–1.2.1 added VCS detection, jujutsu support, and bug fixes
but were not recorded in the changelog._

### Added

- **Architecture Decision Records (ADRs)**: New `decisions/` skill category
  with three skills for managing architectural decisions
  - `create-adr` — Interactively create ADRs with context gathering and
    quality guidelines
  - `extract-adrs` — Extract decisions from existing research and planning
    documents into formal ADRs
  - `review-adr` — Review proposed ADRs for quality; accept, reject, or
    deprecate with append-only lifecycle enforcement
- Companion scripts `adr-next-number.sh` and `adr-read-status.sh` for ADR
  number assignment and status checking
- `meta/decisions/` directory for storing ADRs with sequential `ADR-NNNN`
  numbering

## 1.0.0 — 2026-03-14

Initial extraction from `~/.claude/` into a standalone Claude Code plugin.

- 7 agents: codebase-analyser, codebase-locator, codebase-pattern-finder,
  documents-analyser, documents-locator, reviewer, web-search-researcher
- 9 user-invocable skills: commit, create-plan, describe-pr, implement-plan,
  research-codebase, respond-to-pr, review-plan, review-pr, validate-plan
- 9 supporting skills: 7 review lenses + 2 output formats
- Skills organized into logical groups: git/, planning/, review/, research/
