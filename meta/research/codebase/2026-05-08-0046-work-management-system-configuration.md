---
date: 2026-05-08T20:42:32+01:00
researcher: Toby Clemson
git_commit: 8f03965c736cef0f31a56eefa0852e50be23f513
branch: omvluvulyxnmuwpvplrykkwronryozuv (jj change)
repository: accelerator
topic: "Work Management System Configuration (work item 0046) — implementation prep"
tags: [research, codebase, work-management, integrations, configuration, work.integration]
status: complete
last_updated: 2026-05-08
last_updated_by: Toby Clemson
---

# Research: Work Management System Configuration (0046)

**Date**: 2026-05-08T20:42:32+01:00
**Researcher**: Toby Clemson
**Git Commit**: 8f03965c736cef0f31a56eefa0852e50be23f513
**Branch**: omvluvulyxnmuwpvplrykkwronryozuv
**Repository**: accelerator

## Research Question

Educate the implementation of `meta/work/0046-work-management-system-configuration.md`,
which adds a new `work.integration` config key (allowed values
`jira | linear | trello | github-issues`). The story is the activation gate
for the entire work-management integration epic (0045) — when set, integration
skills auto-scope to `work.default_project_code`; when unset, all work skills
operate purely against `meta/work/` with no external API calls. Acceptance
criteria require informative error surfacing on unrecognised values and a
warning when `work.integration` is set but `work.default_project_code` is empty.

## Summary

Implementation should be **small and surgical**. The codebase already exposes
all the primitives needed; nothing new is required architecturally.

1. **No central registration is required** for `work.integration`. The two
   existing `work.*` keys (`work.id_pattern`, `work.default_project_code`) are
   not in `scripts/config-defaults.sh` either — they are read inline at every
   consumer via `scripts/config-read-value.sh <key> <default>`. Following that
   precedent keeps churn minimal.
2. **Validation precedent is `validate_severity`** in
   `scripts/config-read-review.sh:129-138` (warn + fall back to default). For
   `work.integration` the work item demands a stricter behaviour (informative
   error naming valid values), so the close cousin to copy is the
   `wip_validate_pattern`/exit-non-zero pattern in
   `skills/work/scripts/work-item-common.sh`. A new dedicated reader
   `scripts/config-read-work.sh` (mirroring `config-read-review.sh`) is the
   cleanest home for this validation.
3. **Auto-scoping is already half-built**. `jira-search-flow.sh:200-210` and
   `jira-create-flow.sh:171-188` already fall back to
   `work.default_project_code` when `--project` is omitted. The 0046 acceptance
   criterion ("when `work.integration` is configured … the skill defaults to
   `PROJ`") is **already true unconditionally** for those two flows. The change
   needed is essentially documentation/guard work, not new fallback logic.
4. **The "no external API calls when unset" criterion is trivially satisfied
   today**: zero of the seven local work skills (`create`, `update`, `list`,
   `extract`, `refine`, `review`, `stress-test`) make external calls — verified
   by grep. So the acceptance criterion is met by the absence of new code, not
   by adding gating.
5. **Documentation surface** is concentrated in `skills/config/configure/SKILL.md`
   (lines 427–520 are the `### work` section, including the recognised-keys
   allow-list at lines 519–520).
6. **Tests** belong in `scripts/test-config.sh`. The canonical enum-validation
   template is at `test-config.sh:1594-1611` (`Test: Invalid severity value ->
   warning to stderr, default used`).

The narrowest viable implementation is approximately:
**1 new validator helper + 4–6 documentation edits + 4–6 test cases**. No
existing skill needs runtime branching to satisfy 0046's acceptance criteria;
that branching is the subject of subsequent stories (0047 sync status, 0051
sync skill).

## Detailed Findings

### A. Configuration system architecture

**Defaults registry** — `scripts/config-defaults.sh` (recently centralised
under work item 0030; uncommitted edits in this workspace):

- Defines only `PATH_KEYS` (15 entries, `paths.*`), `PATH_DEFAULTS`,
  `TEMPLATE_KEYS` (6 entries, `templates.*`).
- Header comment at lines 11–17 explicitly notes "this file currently
  centralises only PATH and TEMPLATE keys". Review/agent defaults remain
  inline in `config-dump.sh`.
- The single-definition-site invariant is enforced by `test-config.sh:2476-2483`
  via the regex `(PATH_KEYS|PATH_DEFAULTS|TEMPLATE_KEYS)`. Adding a new
  parallel array (e.g. `WORK_KEYS`) would not violate that invariant — it
  scans only those three names.

**Reader chain**:

- `scripts/config-read-value.sh` is the generic key reader (line 24-25 takes
  `KEY` and `DEFAULT`; line 33–39 splits dotted key into `SECTION.SUBKEY`;
  awk-based YAML parser at lines 60–90).
- Precedence (lines 117–130): team `.accelerator/config.md` then local
  `.accelerator/config.local.md`, last-writer-wins. Loop does **not** break
  on first match.
- Missing-key behaviour: returns the `DEFAULT` argument (or empty if none).
  **There is no error path for missing keys.** Callers cannot distinguish
  "unset" from "set to default value".
- `scripts/config-read-path.sh` is the thin wrapper for `paths.*` keys; uses
  `PATH_DEFAULTS` registry; warns on unknown key (line 38).
- `scripts/config-dump.sh` is the aggregator that powers
  `/accelerator:configure view`. Iterates `PATH_KEYS`, `TEMPLATE_KEYS`,
  `REVIEW_KEYS`, `AGENT_KEYS` (lines 159–192). **It currently has no
  `work.*` iteration**, which is why neither `work.id_pattern` nor
  `work.default_project_code` appears in the dump output today.

**Existing `work.*` keys**:

- Documentation surface: `skills/config/configure/SKILL.md:427-520` (the
  `### work` section). Table at lines 431–434 lists `id_pattern` and
  `default_project_code`. Lines 517–520 are the explicit "Recognised keys"
  paragraph: "Only `work.id_pattern` and `work.default_project_code` are
  recognised. Other `work.*` keys are not consumed by any plugin script."
- Read sites for `work.id_pattern`: `work-item-next-number.sh:58`,
  `work-item-resolve-id.sh:45`, `write-visualiser-config.sh:95`,
  `migrations/0002-rename-work-items-with-project-prefix.sh:19-20`,
  `extract-work-items/SKILL.md:349`, `list-work-items/SKILL.md:24`.
- Read sites for `work.default_project_code`:
  `work-item-next-number.sh:59`, `work-item-resolve-id.sh:46`,
  `write-visualiser-config.sh:96`,
  `migrations/0002-…sh:21-22`, `jira-create-flow.sh:175-180`,
  `jira-search-flow.sh:207`, `jira-init-flow.sh:167-177`,
  `extract-work-items/SKILL.md:350`, `list-work-items/SKILL.md:25`,
  `create-jira-issue/SKILL.md:54,158,…`.
- Tests: `scripts/test-config.sh:244-293` — read from team config, default
  when unset, local override.

### B. Validation patterns

**There is no plugin-wide enum validator.** Three idioms exist; pick by
required behaviour:

1. **Warn + default fallback** (`config-read-review.sh:129-138`):
   ```bash
   validate_severity() {
     local name="$1" value="$2" default="$3"
     case "$value" in
       critical|major|none) echo "$value" ;;
       *)
         echo "Warning: review.$name must be 'critical', 'major', or 'none', got '$value' — using default ($default)" >&2
         echo "$default"
         ;;
     esac
   }
   ```
   Used at `config-read-review.sh:208-210` for three review-severity keys.

2. **Hard fail with named error code** (`work-item-common.sh:17-23` defines
   error names; `work-item-resolve-id.sh:48-50`):
   ```bash
   if ! wip_validate_pattern "$PATTERN"; then
     exit 1
   fi
   ```
   Validator emits stderr message with named code (`E_PATTERN_NO_NUMBER_TOKEN`,
   etc.) and the caller exits non-zero.

3. **Stable-prefixed error codes** (Jira pattern, e.g. `jira-auth.sh:241`):
   ```bash
   echo "E_AUTH_NO_SITE: jira.site not configured in .accelerator/config.md" >&2; return 27
   ```
   Codes registered in `skills/integrations/jira/scripts/EXIT_CODES.md`.

**For `work.integration`** the acceptance criterion ("informative error is
surfaced naming the valid values") rules out pattern (1) — there is no
sensible default for "active integration", and silent fallback would mask the
misconfiguration. Pattern (2) or (3) fits. Suggested shape:

```bash
validate_integration() {
  local value="$1"
  case "$value" in
    "" | jira | linear | trello | github-issues) echo "$value" ;;
    *)
      echo "Error: work.integration must be one of: jira, linear, trello, github-issues (got '$value')" >&2
      return 1
      ;;
  esac
}
```

A separate decision worth making: **where** the validator lives and **when** it
runs. Three placements:

- **At every consumer call site** (mirrors current `work.*` pattern, but
  duplicates validation). Rejected — proliferation risk.
- **In a new `scripts/config-read-work.sh`** that exposes a single entry
  point returning validated values for all `work.*` keys (mirrors
  `config-read-review.sh`). Recommended — pulls the existing inline reads
  into one place and gives a natural home for `work.integration` validation
  plus the "default project code required" warning.
- **In `config-read-value.sh` with a registry-driven validation hook**.
  Rejected — generalises something used in exactly one place; violates "no
  premature abstraction".

**Error surfacing**: there is a centralised `scripts/log-common.sh` providing
`log_die`/`log_warn` (under-adopted; only Jira aliases them as
`jira_die`/`jira_warn`). New code should prefer these helpers over inline
`echo … >&2`.

### C. Empty-vs-missing semantics

Plugin convention: **empty == missing**. Both trigger the default-fallback
branch. Downstream consumers test with `[ -z "$var" ]`. This matters for the
acceptance criterion "Given `work.default_project_code` is empty and
`work.integration` is set, … warns that a default project code is required":
the warning logic must `[ -z ]`-test, not attempt to detect "key absent vs.
key present-but-empty".

### D. Jira integration: `--project` and project scoping

The Jira integration is the only existing concrete data point for what
`work.integration: jira` means in practice. Findings:

| Skill | Has `--project`? | Falls back to config? |
|---|---|---|
| `search-jira-issues` | Yes | Yes → `work.default_project_code` |
| `create-jira-issue` | Yes | Yes → `work.default_project_code` |
| `show-jira-issue` | No | N/A (uses ISSUE-KEY) |
| `update-jira-issue` | No | N/A |
| `comment-jira-issue` | No | N/A |
| `transition-jira-issue` | No | N/A |
| `attach-jira-issue` | No | N/A |

**Implication**: For 0046's acceptance criterion "Given `work.integration:
jira` … invoked without `--project`, the skill defaults to `PROJ`", the
behaviour **already works** for `search-jira-issues` and `create-jira-issue`,
and **does not apply** to the per-issue skills (they are key-scoped, not
project-scoped). No code change is required to the Jira flows for 0046 itself.

The `work.integration` check is **not yet present anywhere in
`skills/integrations/jira/`** (grep confirmed). The fallback today is
unconditional — happens whether or not `work.integration` is set. That is
arguably fine: setting `work.default_project_code` without `work.integration`
is a no-op for Jira flows that nobody invokes if Jira isn't configured.
But if 0046 is meant to make the fallback *gated* on `work.integration: jira`,
then `jira-search-flow.sh:200-210` and `jira-create-flow.sh:171-188` need
adjustment. The work item text is ambiguous on this; the conservative reading
is "leave the existing unconditional fallback alone" since both criteria
remain satisfied.

A useful refactor (optional but small): introduce
`jira_resolve_default_project()` in `jira-common.sh` to deduplicate the two
identical fallback blocks. Not strictly required by 0046.

### E. Local work skills — external-call audit

All seven skills are local-only. Verdict per skill (verified by grep for
`curl`, `wget`, `http`, `api`, `jira`):

| Skill | External calls today | Path-level config consumed |
|---|---|---|
| `create-work-item` | None | `paths.work` + transitive `work.id_pattern`, `work.default_project_code` |
| `update-work-item` | None | `paths.work` |
| `list-work-items` | None | `paths.work`, `work.id_pattern`, `work.default_project_code` (read for display) |
| `extract-work-items` | None | `paths.work`, `paths.research`, `paths.plans`, `work.id_pattern`, `work.default_project_code` |
| `refine-work-item` | None | `paths.work` + transitive |
| `review-work-item` | None | `paths.work`, `paths.review_work` |
| `stress-test-work-item` | None | `paths.work` |

The acceptance criterion "Given `work.integration` is not configured, … all
skills function against `meta/work/` with no external API calls" is
**inherently satisfied** by the current implementation. No gating logic is
required. (Future work — story 0047 — will add `work.integration`-gated
sync-status branches in these skills, but that is **not** in 0046's scope.)

### F. Documentation surface (`skills/config/configure/SKILL.md`)

The `### work` section spans lines 425–520. Required edits for 0046:

1. **Table** at lines 431–434 — add a third row:
   ```
   | `integration`                | (empty)          | Active remote tracker (`jira`, `linear`, `trello`, `github-issues`). When set, integration skills auto-scope to `default_project_code`. |
   ```
2. **Recognised keys paragraph** at lines 517–520 — extend the allow-list to
   include `work.integration`.
3. (Optional) A short subsection between the table and the existing
   `id_pattern` deep-dive explaining the local-first / additive-integration
   semantic from the work item ("Work items are always written to `meta/work/`
   regardless of whether `work.integration` is configured").

### G. Test surface (`scripts/test-config.sh`)

Test-style is heredoc-fixture + subprocess + assertion via
`scripts/test-helpers.sh`. The canonical template for an enum validation test
is at lines 1594–1611. Required new tests for 0046, modelled on the existing
work-key tests at lines 244–293:

- `Test: work.integration unset -> default empty string`
- `Test: work.integration: jira -> reads jira`
- `Test: work.integration: linear / trello / github-issues -> reads value`
  (one test per allowed value or a parameterised loop)
- `Test: work.integration: garbage -> error to stderr naming valid values`
- `Test: work.integration set, work.default_project_code empty -> warning`
  (the second part of acceptance criterion 5, distinct from the validation
  itself; depends on placement decision in §B)
- `Test: local override of work.integration wins over team config`
  (mirrors lines 273-293 for the existing keys)

If a `WORK_KEYS` array is added to `config-defaults.sh`, the
single-definition-site test at lines 2476-2483 will need its regex widened.

### H. Documents already in scope

The closest precedent — and a good template for any plan that follows from
this research — is the configurable-id-pattern feature. It added a `work.*`
config key, validation, documentation edits, tests, and a migration. Every
artefact has a counterpart for 0046:

- Research: `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`
- Plan: `meta/plans/2026-04-28-configurable-work-item-id-pattern.md`
- Plan review: `meta/reviews/plans/2026-04-28-configurable-work-item-id-pattern-review-1.md`

Foundational decisions to respect:

- `meta/decisions/ADR-0016-userspace-configuration-model.md` — userspace
  config layering (team `.accelerator/config.md` + local
  `.accelerator/config.local.md`); `work.integration` is workspace-level,
  consistent with this model.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — extension
  points design.

Recent precedent for centralisation:

- `meta/plans/2026-05-08-0030-centralise-path-defaults.md` and follow-up
  `2026-05-08-0030-remove-inline-path-defaults-from-consumers.md`. Establishes
  the "centralise keys + defaults in `config-defaults.sh`, source it from
  consumers" pattern. **Whether to apply this pattern to `work.*` is a design
  question for the implementing plan** — the existing two `work.*` keys are
  still inline. Adding `work.integration` is a natural moment to either
  follow the path precedent (centralise all three `work.*` keys) or defer
  that consolidation to a separate cleanup story.

## Code References

- `scripts/config-defaults.sh:11-17` — scope-note comment
- `scripts/config-defaults.sh:27-70` — `PATH_KEYS` / `PATH_DEFAULTS` /
  `TEMPLATE_KEYS` arrays
- `scripts/config-read-value.sh:24-25` — KEY/DEFAULT entry
- `scripts/config-read-value.sh:33-39` — dotted-key split
- `scripts/config-read-value.sh:117-130` — last-writer-wins precedence loop
- `scripts/config-read-review.sh:129-138` — `validate_severity` (enum
  validator, warn + default)
- `scripts/config-read-review.sh:208-210` — call sites for severity validation
- `scripts/log-common.sh` — `log_die` / `log_warn` (preferred error helpers)
- `scripts/test-config.sh:244-293` — existing `work.*` key tests
- `scripts/test-config.sh:1594-1611` — canonical enum-validation test template
- `scripts/test-config.sh:2476-2483` — single-definition-site invariant for
  registry arrays
- `scripts/test-helpers.sh:19-301` — assertion library
- `skills/config/configure/SKILL.md:425-520` — `### work` documentation section
- `skills/config/configure/SKILL.md:431-434` — work keys table (insertion
  point for `integration` row)
- `skills/config/configure/SKILL.md:519-520` — "Recognised keys" allow-list
- `skills/work/scripts/work-item-next-number.sh:58-59` — current `work.*`
  read pattern (template for new key consumption)
- `skills/work/scripts/work-item-resolve-id.sh:45-46` — same
- `skills/work/scripts/work-item-common.sh:17-23` — named error codes
  precedent
- `skills/integrations/jira/scripts/jira-search-flow.sh:200-210` — existing
  unconditional fallback to `work.default_project_code`
- `skills/integrations/jira/scripts/jira-create-flow.sh:171-188` — same
- `skills/integrations/jira/scripts/jira-init-flow.sh:163-197` —
  `_jira_prompt_default` (UX precedent for prompting on missing
  `work.default_project_code`)
- `skills/integrations/jira/scripts/EXIT_CODES.md` — stable error-code
  registry
- `meta/work/0045-work-management-integration.md` — parent epic
- `meta/work/0046-work-management-system-configuration.md` — the work item

## Architecture Insights

1. **Inline default + on-demand validation is the dominant pattern**.
   Centralisation in `config-defaults.sh` is recent (0030) and currently
   covers only `paths.*` and `templates.*`. The codebase has not standardised
   on "registry first" — adding `work.integration` doesn't have to either.
2. **Validation is a per-domain concern**, never centralised. The
   `config-read-review.sh` model — one reader script per top-level config
   section, with all section-specific validation in that file — is the
   established pattern.
3. **No precedent exists for "skill conditionally branches on which
   integration is configured"**. 0046 sets up the condition; the actual
   conditional branching is the subject of 0047 (sync status in
   `/list-work-items`, push offer in `/create-work-item`) and 0051
   (`sync-work-items` skill). 0046 should resist the temptation to add any
   such branching prematurely.
4. **Empty-vs-missing is intentionally not distinguished** in the reader.
   This forces "warn when set but empty" logic to live downstream of
   `config-read-value.sh`, never inside it.
5. **Stable error codes are reserved for cross-process contracts** (Jira
   skills test for them in their test suites). For internal helpers, plain
   `Error: …` text is the convention.
6. **The `allowed-tools` line in each SKILL.md** is a real security boundary.
   Any new helper script (e.g. `config-read-work.sh`) added to skills'
   workflows must be added to their `allowed-tools` patterns.

## Historical Context

- `meta/decisions/ADR-0016-userspace-configuration-model.md` — establishes
  team config / local config layering. Any new key respects this layering
  automatically via `config-read-value.sh`.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — defines what
  it means to add a config key. Reading this is recommended before
  implementing.
- `meta/decisions/ADR-0022-work-item-terminology.md` — the rationale for
  "work item" over "ticket"; relevant only for naming consistency in the
  documentation edits.
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` — closest
  prior research; same author, same author's voice for plans, similar
  scope. Worth re-reading before drafting an implementation plan.
- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` — establishes
  the Jira integration shape that `work.integration: jira` activates.
- `meta/notes/2026-04-29-accelerator-config-state-reorg.md` — recent reorg of
  config-state directory layout. Confirms `meta/integrations/<system>/` as
  the per-integration state location.

## Related Research

- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` — the
  closest precedent for adding a `work.*` config key.
- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` — Jira
  integration design; informs what `work.integration: jira` activates.
- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — original
  ticket-management research (pre-rename).
- `meta/research/codebase/2026-05-08-0030-centralise-path-defaults-implementation.md`
  — most recent precedent for centralising config defaults.

## Open Questions

1. **Centralise `work.*` keys in `config-defaults.sh`?** The existing two
   keys are inline; adding `integration` is a natural moment to consolidate.
   Doing so would slightly widen scope of 0046 but eliminate the divergence
   between `paths.*` (centralised) and `work.*` (inline). **Recommendation
   for the plan**: defer. Keep 0046 narrow; consolidate `work.*` defaults in
   a separate cleanup story modelled on 0030.
2. **Should the unconditional `work.default_project_code` fallback in
   `jira-search-flow.sh` and `jira-create-flow.sh` be gated on
   `work.integration: jira`?** The work item is ambiguous. The strict
   reading of acceptance criterion 2 ("Given `work.integration: jira` …
   defaults to `PROJ`") permits either interpretation. The conservative
   choice is to leave the existing fallbacks unconditional — they are
   already correct under both interpretations.
3. **Where does `validate_integration` live?** Inline at consumers vs. a new
   `scripts/config-read-work.sh`. Recommendation: new file, mirroring
   `config-read-review.sh`, since the acceptance criteria require both
   enum validation and a "default project code missing" warning — two
   pieces of section-specific logic that belong in one place.
4. **Should `config-dump.sh` learn about `work.*` as part of 0046?** It
   currently doesn't surface `work.id_pattern` or `work.default_project_code`
   in the dump. Same recommendation as Q1 — defer to a follow-up cleanup,
   keep 0046 narrow.
5. **Are stable error codes (e.g. `E_WORK_INTEGRATION_INVALID`) warranted
   here?** No external consumer parses these codes today. Recommendation:
   plain `Error: …` text is sufficient; reserve named codes for the
   integration skills themselves (per Jira precedent).
