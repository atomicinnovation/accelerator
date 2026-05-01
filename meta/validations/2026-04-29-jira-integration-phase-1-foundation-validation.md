---
date: "2026-05-01T13:00:00+01:00"
type: plan-validation
skill: validate-plan
target: "meta/plans/2026-04-29-jira-integration-phase-1-foundation.md"
result: partial
status: complete
---

## Validation Report: Jira Integration Phase 1 — Foundation

### Implementation Status

✓ M1: Skeleton, config docs, plugin registration, test wiring — Fully implemented
✓ M2: jira-common.sh and jira-auth.sh — Fully implemented
✓ M3: jira-jql.sh — Fully implemented
✓ M4: jira-adf-to-md.sh, jira-md-to-adf.sh — Fully implemented
✓ M5: jira-request.sh — Fully implemented
✓ M6: jira-fields.sh — Fully implemented
✓ M7: init-jira/SKILL.md — Fully implemented
⚠ Exit code namespace documentation (`EXIT_CODES.md`) — Missing

### Automated Verification Results

✓ `mise run test` — 139 tests passed, 0 failed
✓ `bash scripts/config-read-value.sh jira.site '<default>'` — returns `<default>`
✓ `bash scripts/test-format.sh` — 1 passed, 0 failed (no `work item-` violations)
✓ `jq -e '.skills | index("./skills/integrations/jira/")' .claude-plugin/plugin.json` — index 2 (non-null)
✓ `bash skills/integrations/jira/scripts/test-jira-scripts.sh` — all sub-tests pass
✓ `bash scripts/test-config.sh` — 34 passed including new `jira.*` cases

### Code Review Findings

#### Matches Plan

**M1 — Skeleton, plugin, config:**
- `.claude-plugin/plugin.json` includes `"./skills/integrations/jira/"` at index 2 (after `./skills/github/` as specified)
- `skills/config/configure/SKILL.md` has the `### jira` section between `### work` and `### templates` with all documented keys (site, email, token, token_cmd), auth chain, and security rationale for `token_cmd` restriction
- `paths.integrations` row added to the paths table with correct default `meta/integrations`
- Skill registration order documented in configure SKILL.md as specified
- `meta/integrations/jira/.gitkeep` committed; `.gitignore` covers `.lock` and `.refresh-meta.json`
- `tasks/test.py` wired with the jira integration test block
- `scripts/test-config.sh` has three `jira.*` test blocks (reads, defaults, local override)

**M2 — jira-common.sh + jira-auth.sh:**
- `scripts/log-common.sh` exists with `log_die` / `log_warn` as the shared logging library
- `jira-common.sh` has all required functions: `jira_state_dir` (line 54), `jira_jq_field` (line 76), `jira_atomic_write_json` (line 82), `jira_with_lock` (line 123), `jira_require_dependencies` (line 209), `_jira_uuid_v4` (line 235)
- `jira_state_dir` reads `paths.integrations` via `config-read-path.sh integrations meta/integrations` — not hardcoded
- `jira_with_lock` implements mkdir-based atomic locking with PID+start-time stale detection and `mv`-then-`rm` reclaim sequence as specified
- `jira-auth.sh` implements the five-step resolution chain; `token_cmd` from shared config emits `E_TOKEN_CMD_FROM_SHARED_CONFIG` warning and is ignored
- Token redaction confirmed: `jira-auth-cli.sh` uses `***` for `--debug` stderr; `jira-request.sh` passes token via `--config` stdin pipe (not process args)

**M3 — jira-jql.sh:**
- `jira-jql.sh` has `jql_quote_value` with control-char detection and all composition functions
- Exit codes 30–33 documented in header; `jira-jql-cli.sh` CLI wrapper present

**M4 — ADF converters:**
- `jira-md-to-adf.sh`, `jira-adf-to-md.sh`, `jira-md-tokenise.awk`, `jira-md-assemble.jq`, `jira-adf-render.jq` all present
- Test suite covers fixture-pair sweep, CRLF normalisation, rejection cases (table 41, nested-list 41, blockquote 41, control-chars 42), unsupported-node placeholders, and round-trip invariants
- Round-trip fixtures under `test-fixtures/adf-samples/`

**M5 — jira-request.sh:**
- Exit codes 11–23 documented in header (11=401, 12=403, 13=404, 14=410, 15=E_BAD_SITE, 16=E_REQ_BAD_RESPONSE, 17=E_REQ_BAD_PATH, 18=E_TEST_OVERRIDE_REJECTED, 19=429, 20=5xx, 21=E_REQ_CONNECT, 22=E_REQ_NO_CREDS, 23=E_TEST_HOOK_REJECTED)
- Mock server at `test-helpers/mock-jira-server.py` with scenario-driven fixtures

**M6 — jira-fields.sh:**
- `jira-fields.sh` has `refresh`, `resolve`, and `list` subcommands; `jira_with_lock` wraps refresh; `jira_field_slugify` is a public function
- Sourceable with `BASH_SOURCE` guard

**M7 — init-jira:**
- `init-jira/SKILL.md` has correct frontmatter (`disable-model-invocation: true`, scoped `allowed-tools`), bang-preprocessor lines, and dispatches to `jira-init-flow.sh`
- `jira-init-flow.sh` has `verify`, `discover`, `prompt-default`, `refresh-fields`, `list-projects`, `list-fields` subcommands; exit codes 60 (`E_INIT_NEEDS_CONFIG`) and 61 (`E_INIT_VERIFY_FAILED`) documented

#### Deviations from Plan

**F1 — EXIT_CODES.md missing** (major, required deliverable):

`skills/integrations/jira/scripts/EXIT_CODES.md` does not exist. The plan (lines 277–322) explicitly required this file as the canonical exit-code namespace manifest, including the test-seam policy documenting `ACCELERATOR_TEST_MODE=1` gating semantics. Two production files already reference it:
- `jira-fields.sh:18` — `# See also: EXIT_CODES.md`
- `jira-init-flow.sh:26` — `# See also: EXIT_CODES.md`

These are dangling cross-references. Exit code ranges exist only in per-file header comments. The file's content can be assembled from those comments.

**F2 — Auth resolver shared-token condition** (minor, behavioural deviation):

The plan (Desired End State item 2) states the shared `accelerator.md` token is honoured "only when `accelerator.local.md` does not exist." The implementation at `jira-auth.sh:221–229` falls through to the shared token whenever `JIRA_TOKEN` is still empty — including when `accelerator.local.md` exists but contains no token entry. A user with a `accelerator.local.md` for non-token settings would unexpectedly pick up a shared plaintext token. The plan's intent is to block this fallback entirely when `local.md` is present.

### Manual Testing Required

1. **`/configure help`**: confirm `### jira` section appears between `### work` and `### templates`.
2. **`/init-jira`** against a real Jira Cloud tenant: verify it walks site/email/token verification, discovers projects and fields, and populates `meta/integrations/jira/{site,fields,projects}.json`.
3. **Directory tree**: `tree skills/integrations/jira/scripts/test-fixtures meta/integrations/jira` confirms fixtures and `.gitkeep` are in place.

### Recommendations

1. **Create `skills/integrations/jira/scripts/EXIT_CODES.md`** — consolidate the exit code ranges from per-file header comments and add the test-seam policy section (which `ACCELERATOR_TEST_MODE=1`-gated env vars exist, their behaviour on a non-test run). Removes the dangling cross-references in `jira-fields.sh` and `jira-init-flow.sh`.

2. **Fix auth resolver shared-token guard** (`jira-auth.sh:221`) — add a `[ ! -f "$local_cfg" ]` condition before attempting the shared fallback token, so the presence of `accelerator.local.md` (for any reason) blocks the fallback regardless of whether it contains a token entry.
