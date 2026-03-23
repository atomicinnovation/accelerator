# Changelog

## Unreleased

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
