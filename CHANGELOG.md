# Changelog

## Unreleased

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
    research, decisions, PRs, validations, reviews, templates, tickets, notes)
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
  (`date`, `type`, `skill`, `ticket`, `status`) in the plan template
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
