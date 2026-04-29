---
date: "2026-04-30T00:55:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-29-jira-integration-phase-1-foundation.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, security, test-coverage, code-quality, correctness, usability, standards, portability]
review_pass: 4
status: complete
---

## Plan Review: Jira Integration Phase 1 — Foundation

**Verdict:** REVISE

The plan is structurally sound: TDD discipline is end-to-end, conventions are faithfully reused (`jira_*` namespacing, `set -euo pipefail` split, atomic writes, `find_repo_root`), the dependency cone is acyclic, and load-bearing concerns (token redaction, JQL safe quoting, ADF round-trip) are treated as first-class testable contracts. However three independent token-exfiltration vectors are present in the design as written — a `token_cmd` supply-chain sink, `curl -u` putting credentials in the process command line (contradicting the plan's explicit claim), and a `JIRA_BASE_URL_OVERRIDE` test seam shipped in production code — and several major correctness gaps (round-trip property too weak, malformed UUID fallback, single-newline token trim, EMPTY sentinel collision, concurrent-refresh race) plus portability landmines (bash-4 `${var,,}`, BSD-vs-GNU awk, missing `xxd`) require resolution before implementation.

### Cross-Cutting Themes

- **Token leakage surface is broader than the plan's redaction tests cover** (flagged by: security, test-coverage, architecture, code-quality, correctness) — `bash -c` from committed config, `-u` on the process command line, `curl -v`/`--trace` could re-introduce leaks under `--debug`, `token_cmd` stderr is not sanitised, the `accelerator.local.md` plaintext path has no permission warning, and the redaction test only greps stderr for the literal token bytes (not base64, not `/proc/<pid>/cmdline`, not temp files).
- **`JIRA_BASE_URL_OVERRIDE` is a live test seam in production code** (flagged by: security [critical], architecture, standards) — three lenses independently flagged that an undocumented env var redirecting authenticated requests is an exfiltration vector that violates separation of concerns.
- **ADF round-trip property is defined as a fixed point on the second compile** (flagged by: test-coverage, correctness) — the chosen invariant `compile(render(compile(md))) == compile(md)` masks first-compile lossiness; what users actually rely on is `render(compile(md)) == md` for the supported subset.
- **Exit-code namespace mixes HTTP-status mirrors and abstract names with sparse gaps** (flagged by: code-quality, usability) — 11/12/13/14 are HTTP-status codes; 19/20/21/22/30/31/32/40/41/42/50/51/52 are abstract; gaps (15-18) are unexplained; auth codes are name-only without numbers.
- **bash 4+ syntax `${var,,}` will fail on macOS default bash 3.2** (flagged by: portability, correctness) — `mise.toml` does not pin a bash version; the recent "Fix migration tests on linux" commit shows CI/local drift is already a real signal.
- **`uuidgen` / `xxd` fallback chain does not cover absence of both, and the fallback isn't actually a UUID** (flagged by: portability, correctness, code-quality) — minimal Linux containers lack both; the 32-hex-char output has no version/variant nibbles, so Atlassian rejects it; POSIX `od` or Python (already a dep) would close the gap.
- **`meta/integrations/jira/` path is hard-coded throughout despite a deferred reorg note recommending a one-line indirection** (flagged by: architecture, standards) — every SKILL.md, helper, and success criterion that mentions the literal path becomes a migration site when the reorg lands. `paths.integrations` has no entry in the configure schema.
- **Mock server fidelity and CI portability are unverified** (flagged by: test-coverage, portability, standards) — Python version is unpinned, mock startup race has no timeout, no contract test against real Jira responses, scenario JSON drift is undetected.

### Tradeoff Analysis

- **Convenience of `jira.token` in shared config vs. credential leakage**: usability flags listing `jira.token` in the recognised-keys table invites accidental commits; security agrees plaintext in `accelerator.local.md` needs a permissions warning. Recommendation: drop `jira.token` from the recognised-keys table or move it to a "local-only, do not commit" sub-paragraph; emit a warning when the loader sees `jira.token` in `accelerator.md`.
- **Strict JQL unsafe-character rejection vs. legitimate Jira data**: correctness and usability both flag the broad denylist (`% ^ $ # @ [ ] ; ? | * /`) — many of these appear legitimately in real Jira labels and titles, training users to reflexively pass `--unsafe`. Recommendation: tighten the rule to control-chars + backslash + lone single-quotes (the genuine quoting hazards) and let printable punctuation through after single-quote doubling.
- **JSON-only output (no `--plain`/`--csv`) vs. shell pipeline ergonomics**: usability flags the friction; the plan's layering rationale is that skills format from JSON. Recommendation: keep raw JSON as default but document recommended `jq` snippets in helper headers so common interactive operations have a copy-pasteable starting point.

### Findings

#### Critical
- 🔴 **Security**: token_cmd from committed accelerator.md is a supply-chain command-injection sink
  **Location**: Phase 2: jira-auth.sh — token_cmd execution via bash -c
  Anyone landing a PR can set `jira.token_cmd: "<arbitrary shell>"` and get RCE on every contributor and CI runner that invokes a Jira helper. Restrict `token_cmd` to `accelerator.local.md`/env vars only, or require an opt-in flag in the local file.

- 🔴 **Security**: curl -u places token in the process command line, contradicting plan's redaction claim
  **Location**: Phase 5: jira-request.sh — "uses `-u` (not `-H Authorization`) so the token never appears verbatim on the process command line"
  The plan's stated reason is incorrect: `-u email:token` is a normal argv entry, visible to any local user via `ps`/`/proc/<pid>/cmdline`. The redaction test only checks stderr/stdout. Pass credentials via `curl --config -` from stdin or `--netrc-file <(...)`; add a process-listing assertion.

- 🔴 **Security**: JIRA_BASE_URL_OVERRIDE is a live token-exfiltration sink in production code
  **Location**: Phase 5: jira-request.sh — JIRA_BASE_URL_OVERRIDE test seam
  An env var that redirects authenticated requests to an arbitrary URL ships in production. "Not advertised in the SKILL.md" is not a security control. Gate behind a second sentinel (`ACCELERATOR_TEST_MODE=1`), validate the override is `127.0.0.1`/`localhost`, or move the seam to a test-only wrapper script.

#### Major
- 🟡 **Architecture**: State path hard-coded in skill prose despite a deferred reorg note recommending indirection
  **Location**: Overview / Phase 7 init-jira SKILL.md / Migration Notes
  The reorg note explicitly recommends a `config-read-path.sh integrations` helper; the plan rejects that mitigation without justification. Route every reference through `jira_state_dir`.

- 🟡 **Architecture**: JIRA_BASE_URL_OVERRIDE creates a production-visible test seam with no encapsulation
  **Location**: Phase 5: jira-request.sh
  Same surface as the security finding, viewed architecturally: a test concern leaks into the production code path with no abstraction.

- 🟡 **Security**: jira.site is interpolated into URL without validation
  **Location**: Phase 5: jira-request.sh — base URL construction from jira.site
  A malicious PR can set `site: evil` (or worse) to redirect authenticated requests. Validate against `^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$` before URL construction.

- 🟡 **Security**: stderr from token_cmd execution is not sanitised
  **Location**: Phase 2: jira-auth.sh — token_cmd execution and stderr handling
  Password-manager errors can include the secret name, vault path, or fragments. Capture and discard `bash -c "$cmd" 2>/dev/null` by default; emit a generic `E_TOKEN_CMD_FAILED: command exited <N>` instead of reproducing captured stderr.

- 🟡 **Security**: --debug must not enable curl -v / --trace
  **Location**: Phase 5: jira-request.sh — --debug flag
  curl verbose modes print the full Authorization header. Document the prohibition in the helper header; add a regression test that asserts `Authorization:` does not appear in `--debug` output.

- 🟡 **Security**: site.json schema is unspecified — token/PII leakage to committed file plausible
  **Location**: Phase 7: init-jira — persisted state under meta/integrations/jira/site.json
  `/myself` returns email, accountId, displayName, avatarUrls. The file is checked in. Specify the exact allow-list (`{site, accountId, lastVerified}`) and assert it in tests; exclude `emailAddress` etc.

- 🟡 **Security**: accelerator.local.md file permissions not enforced
  **Location**: Phase 2: jira-auth.sh
  Plaintext token in a file with default umask is world-readable. Warn at runtime when looser than 0600; label `jira.token` in the configure docs as "discouraged — prefer `token_cmd`".

- 🟡 **Test Coverage**: Round-trip invariant is fixed-point on ADF, not Markdown
  **Location**: Phase 4: test-jira-adf-roundtrip.sh
  `compile(render(compile(md))) == compile(md)` masks first-compile content erasure. Strengthen to `render(compile(md)) == md` for the supported subset (with documented canonicalisation), and add a "no silent drop" length-comparison check.

- 🟡 **Test Coverage**: Mock server fidelity to real Jira is unverified
  **Location**: Phase 5: Mock Jira server fixture
  No contract test, no recorded cassettes, no Retry-After-as-HTTP-date case. Capture real responses into `test-fixtures/api-responses/`; assert the mock returns byte-identical bodies/headers.

- 🟡 **Test Coverage**: Backoff timing assertions invite CI flakiness
  **Location**: Phase 5: Cases 8 and 9 (backoff timing)
  Wall-clock bounds `≥ 1.0 s and < 3.0 s` are an anti-pattern. Add `JIRA_RETRY_SLEEP_FN` / `JIRA_BACKOFF_BASE_SECONDS=0` test seam; assert *number* of retries and *sequence* of sleep durations rather than wall time.

- 🟡 **Test Coverage**: Mock server lifecycle and CI portability untested
  **Location**: Phase 5: mock-jira-server.py
  Python availability not asserted, URL-file wait has no timeout, orphan-process cleanup undefined, IPv6-only host case unconsidered.

- 🟡 **Test Coverage**: Token redaction test covers only one stream and one format
  **Location**: Phase 2: case 9
  Should also grep for base64-encoded form, URL-encoded form, `ps`/`/proc/<pid>/cmdline` content, and any temp files. Use a sentinel token (`tok-SENTINEL-xyz`) and grep all output streams + temp dirs.

- 🟡 **Test Coverage**: No malformed/empty/large response coverage
  **Location**: Phase 5: jira-request.sh
  Missing: empty 200 body, HTML error page from a transparent proxy, chunked mid-stream close, multi-MB payload (already mentioned under Performance), unicode `displayName`.

- 🟡 **Test Coverage**: Unicode and edge-case ADF fixtures missing
  **Location**: Phase 4: ADF fixture set
  Add `unicode-mixed`, `link-with-parens`, `large-paragraph`, `crlf-input`, `empty-doc` fixtures. ADF unicode bugs are exactly the silent-corruption class users report.

- 🟡 **Test Coverage**: No concurrency / partial-write coverage for atomic writes
  **Location**: Phase 2: jira_atomic_write_json
  Add tests for two concurrent writers, kill -9 mid-write (no `.tmp` leftover), cross-device rename, symlink target, unwritable dir.

- 🟡 **Code Quality**: Markdown-to-ADF compiler in pure bash will be hard to maintain
  **Location**: Phase 4: jira-md-to-adf.sh (250–400 lines bash + awk + jq)
  Tighten the awk-record-stream contract upfront; if the file grows beyond ~300 lines during implementation, split into `jira-md-tokenise.sh` + `jira-md-inlines.sh` rather than letting one file accrete.

- 🟡 **Code Quality**: jira-common.sh shaping up as a kitchen-sink module
  **Location**: Phase 2: jira-common.sh (eight functions across four concerns)
  Lift `jira_die`/`jira_warn` to `scripts/log-common.sh` for cross-integration reuse, or add a header-comment table grouping by concern. Make `jira_require_dependencies` its own function with a clear name.

- 🟡 **Code Quality**: Exit-code namespace mixes HTTP-status mirrors with abstract codes
  **Location**: Implementation Approach — exit-code allocation across phases
  11–14 are HTTP statuses, 19–22 are abstract; gaps 15–18 unexplained; auth codes are name-only. Document the full namespace in one place (e.g. `EXIT_CODES.md`); pin auth codes to numbers; pick one encoding scheme and stick to it.

- 🟡 **Correctness**: Token-cmd trim only strips a single trailing newline
  **Location**: Phase 2: jira-auth.sh
  `${var%$'\n'}` does not handle `\r\n`, multiple trailing newlines, or trailing spaces. Trim all trailing whitespace; test with `dummy-token\r\n` input.

- 🟡 **Correctness**: EMPTY sentinel conflates legitimate string with IS EMPTY clause
  **Location**: Phase 3: test-jira-jql.sh case 5
  `jql_quote_value ""` returning `EMPTY` collapses two distinct intents. Require `--empty <field>` for IS EMPTY; reject empty-string with `E_JQL_EMPTY_VALUE`.

- 🟡 **Correctness**: Round-trip property is fixed point on second compile, masking first-compile lossiness
  **Location**: Phase 4: test-jira-adf-roundtrip.sh
  Mirrored by the test-coverage finding; both lenses agree the invariant is too weak.

- 🟡 **Correctness**: UUID fallback produces 32 hex chars without UUID structure or version bits
  **Location**: Phase 4: jira-md-to-adf.sh — taskItem.localId
  `head -c 16 /dev/urandom | xxd -p` is not "formatted as a UUID". Atlassian may reject malformed `localId`. Format properly with version-4 nibbles or require `uuidgen`.

- 🟡 **Correctness**: No locking on persisted JSON cache; concurrent refresh interleaves files
  **Location**: Phase 6 / Phase 7
  `init-jira` writes site/projects/fields sequentially; concurrent runs can mix tenants. Acquire `flock` on `meta/integrations/jira/.lock` for multi-file refreshes.

- 🟡 **Correctness**: Retry-After + jitter composition is unspecified
  **Location**: Phase 5: jira-request.sh
  Does jitter apply on top of `Retry-After`? Does the parser handle HTTP-date format? Specify: `min(Retry-After, 60)` with no jitter when present; jitter only on the exponential schedule when absent.

- 🟡 **Correctness**: lastUpdated timestamp makes refresh byte-non-idempotent and noisy in VCS
  **Location**: Phase 6 / Phase 7
  `fields.json` is committed but every refresh changes the timestamp. Either drop `lastUpdated` (track out-of-band in a sibling gitignored file), or only update it when the `fields` array actually changed.

- 🟡 **Usability**: Sprawling exit-code namespace hard to memorise/document
  **Location**: Phases 3, 5, 6
  Mirrored by code-quality. Document one matrix; tighten ranges; consider a shared 1=usage / 2=auth / 3=network / 4=API / 5=data convention.

- 🟡 **Usability**: JQL `~` negation prefix is non-standard and surprising
  **Location**: Phase 3: jira-jql.sh
  JQL itself uses `~` as the *contains* operator. Consider `--not-status` flags, `!` prefix, or runtime warnings for `~`-prefixed values. If kept, document prominently.

- 🟡 **Usability**: Hard refusal of `--plain`/`--csv` forces jq for every shell pipeline
  **Location**: What We're NOT Doing — output formats
  Document recommended jq snippets in helper headers; revisit in Phase 2+ once usage patterns are clear.

- 🟡 **Standards**: plugin.json skills array is workflow-ordered, not alphabetical
  **Location**: Phase 1 §2: Plugin registration
  The "alphabetically between github and planning" claim is misleading. State the ordering rule explicitly (workflow grouping) or commit to alphabetical sorting as a separate isolated change.

- 🟡 **Standards**: init-jira frontmatter omits disable-model-invocation
  **Location**: Phase 7 §1: init-jira SKILL.md
  Every other slash-only skill in the repo sets `disable-model-invocation: true`. Add it.

- 🟡 **Standards**: Eval scaffolding under-specifies evals.json + benchmark.json contract
  **Location**: Phase 7 §3: Eval scaffolding
  `test-evals-structure.sh` requires `benchmark.json` alongside `evals.json` with `pass_rate.mean >= 0.9`. A bare `evals.json` will fail the linter. Either ship a paired `benchmark.json`, or omit the `evals/` directory entirely.

- 🟡 **Portability**: `${var,,}` requires bash 4+; macOS ships 3.2
  **Location**: Phase 6: jira_field_slugify
  Replace with `tr '[:upper:]' '[:lower:]'` or pin bash 5.x in `mise.toml`. Recent "Fix migration tests on linux" commit shows version-drift is already biting.

- 🟡 **Portability**: uuidgen / xxd fallback chain does not cover absence of both
  **Location**: Phase 4: jira-md-to-adf.sh
  Mirrored by correctness. Use POSIX `od -An -N16 -tx1 /dev/urandom | tr -d ' \n'` or Python (already a dep).

- 🟡 **Portability**: BSD vs GNU awk divergence not addressed
  **Location**: Phase 4: ADF block tokeniser
  Pin POSIX awk in the script header; ban gawk-isms; add a CI lint that runs the awk script under both dialects, or rewrite in Python.

- 🟡 **Portability**: Python version for mock server unspecified
  **Location**: Phase 5: mock-jira-server.py
  `mise.toml` pins 3.14.4 inside `mise` activations only. Pin a minimum (`sys.version_info < (3, 9)` guard) and document in the test script header.

#### Minor
- 🔵 **Architecture**: Deferring HTTP/transport abstraction is reasonable but criterion is vague
  **Location**: What We're NOT Doing
  Note in Phase 5 which functions are intended to lift cleanly (retry loop, status-mapper, redacted-debug printer); structure them as standalone functions inside `jira-request.sh`.

- 🔵 **Architecture**: ADF compiler concentrates structural complexity in one bash+awk+jq pipeline
  **Location**: Phase 4
  Make awk record types a documented contract between passes; mirrors code-quality finding.

- 🔵 **Architecture**: Six-step token resolution chain not a first-class element
  **Location**: Phase 2: jira-auth.sh
  Promote resolution path to a data structure (`JIRA_RESOLUTION_SOURCE_TOKEN=env|env_cmd|...`); the `--debug` line becomes a reflection of state rather than an independent computation.

- 🔵 **Architecture**: Phase 7 SKILL-vs-helper split decision deferred to skill-creator session
  **Location**: Phase 7 §2
  Decide upfront: lift the eight-step flow into `jira-init-flow.sh` so it is testable; SKILL.md becomes thin user-facing wrapper.

- 🔵 **Security**: API response bodies on stderr can echo attacker-supplied control characters
  **Location**: Phase 5: jira-request.sh
  Filter through `tr -d '\000-\010\013\014\016-\037\177'` or write to a tempfile and emit a path on stderr.

- 🔵 **Security**: Path argument not validated against absolute URLs / traversal
  **Location**: Phase 5: jira-request.sh
  Validate `<path>` matches `^/rest/api/3/[A-Za-z0-9._/-]+$`; reject otherwise with `E_REQ_BAD_PATH`.

- 🔵 **Security**: ADF compiler should pass Markdown content via jq `--arg` not string interpolation
  **Location**: Phase 4
  Mandate `--arg`/`--rawfile`/`--argjson` for Markdown text payloads; add adversarial-content fixture (`"}, {"type":"mention"`) asserting one paragraph node with literal text.

- 🔵 **Test Coverage**: Field-name collision and case-sensitivity not tested
  **Location**: Phase 6 case 9
  Add tests for ambiguous slugs (two fields with same slug), case-insensitive vs sensitive name match, empty cache.

- 🔵 **Test Coverage**: JQL unsafe-character set hard-coded, not derived from grammar
  **Location**: Phase 3 case 16
  Add a fuzz-style test generating 100 random ASCII inputs; assert quoted output parses as a single-quoted JQL string.

- 🔵 **Test Coverage**: init-jira skill has no automated behaviour coverage
  **Location**: Phase 7
  If orchestration lifts to `jira-init-flow.sh`, add `test-jira-init-flow.sh` covering each flag combination against the mock.

- 🔵 **Test Coverage**: Umbrella runner aggregation not robust to per-script failure
  **Location**: Phase 1: tasks/test.py wiring
  Document that every per-helper script must end with `test_summary || exit 1`; add a meta-test asserting the call is present.

- 🔵 **Test Coverage**: Non-determinism in ADF compiler output complicates fixture comparison
  **Location**: Phase 4: taskItem.localId
  Specify a `JIRA_ADF_LOCALID_SEED` env var, or pre-process via `jq 'walk(...)'` to mask localIds before byte-equality assertion.

- 🔵 **Code Quality**: `bash -c "$cmd"` is fragile for keychain helpers
  **Location**: Phase 2: jira-auth.sh
  Document explicitly that token_cmd is shell-evaluated (so users single-quote arg lists); test commands with embedded spaces.

- 🔵 **Code Quality**: uuidgen fallback chain hides portability concern in hot path
  **Location**: Phase 4
  Pick one primitive (awk-based) and document in script header; add a test asserting localId matches a documented regex.

- 🔵 **Code Quality**: JQL unsafe-character set enumerated by exclusion, inviting drift
  **Location**: Phase 3 case 16
  Anchor in JQL grammar: only single-quote and backslash genuinely need escaping. Drop the broad denylist; reserve `--unsafe` for unquoted-operator cases.

- 🔵 **Code Quality**: Defer-to-skill-creator decision on lifting orchestration
  **Location**: Phase 7 §2
  Mirror of architecture finding.

- 🔵 **Code Quality**: Retry policy lives inside request helper, mixing transport and policy
  **Location**: Phase 5
  Extract retry/backoff into a `_jira_with_retry` inline function so it can be tested as a unit; honouring `Retry-After` should bypass jitter entirely.

- 🔵 **Correctness**: bash 4+ `${s,,}` lowercase expansion will not run on macOS bash 3.2
  **Location**: Phase 6: slugify
  Mirror of portability finding.

- 🔵 **Correctness**: Backslash-in-value rejection via --unsafe is wrong default
  **Location**: Phase 3 case 16
  Tighten rejection list to control chars + backslash + lone single-quotes; let printable punctuation through after single-quote doubling.

- 🔵 **Correctness**: Exit code 14 (410 Gone) overlaps semantically with 13 (404 Not Found)
  **Location**: Phase 5 case 7
  Either collapse 14 into 13, or reserve a code for 'response shape unexpected' (Jira returns HTML 200 during outages).

- 🔵 **Correctness**: Field resolver match precedence (name → slug → id → key) ambiguous on collisions
  **Location**: Phase 6 case 9
  Detect ambiguity at refresh time; exit `E_FIELD_AMBIGUOUS` (new code) on resolve when multiple matches found.

- 🔵 **Correctness**: Site/email cannot be overridden via env var, breaking parity with token resolution
  **Location**: Phase 2 case 11
  Either support `ACCELERATOR_JIRA_SITE` / `ACCELERATOR_JIRA_EMAIL` consistently, or justify the asymmetry explicitly.

- 🔵 **Correctness**: Greedy non-overlapping inline matches in fixed order miss combinations
  **Location**: Phase 4 inline tokeniser
  Specify whether inline tokenising recurses into structural inlines (links); add fixtures for `**[link](url)**`, `[*italic*](url)`, `**_bold-italic_**`.

- 🔵 **Usability**: `work.default_project_code` reuse is clever but undiscoverable
  **Location**: Configure SKILL.md jira section
  Add a bold cross-reference callout in both the `jira` and `work` sections; surface a `Default project: <key>` line in `/init-jira` output.

- 🔵 **Usability**: Discoverability of `--list-fields`/`--list-projects`/`--refresh-fields` relies on argument-hint
  **Location**: Phase 7 §1
  Include `(hint: run /init-jira --list-fields)` in error paths surfacing missing/stale-cache state; add `--help` mode.

- 🔵 **Usability**: 8-step `/init-jira` flow may feel heavy on first run
  **Location**: Phase 7
  Clarify that pre-configured values short-circuit silently; add `--non-interactive` flag failing fast with `E_INIT_NEEDS_CONFIG`; emit a final summary line.

- 🔵 **Usability**: `E_JQL_UNSAFE_VALUE` doesn't tell users which character was the problem
  **Location**: Phase 3 §2
  Specify error message names offending character and value (e.g. `E_JQL_UNSAFE_VALUE: character '/' in 'feature/auth' is not safely quotable`).

- 🔵 **Usability**: Asymmetric ADF unsupported-feature handling (read=placeholder, write=error) confuses users
  **Location**: Phase 4
  Use a more conspicuous marker like `<!-- ADF-PLACEHOLDER:panel -->`; emit friendly compiler error explaining the placeholder must be preserved verbatim.

- 🔵 **Usability**: `jira.token` documented as recognised key invites secret commits
  **Location**: Phase 1 §3 configure SKILL.md
  Move to a separate "Local-only token (do not commit)" sub-paragraph; or omit entirely from the recognised-keys table.

- 🔵 **Usability**: Inconsistent CLI argument styles across helpers
  **Location**: Phases 3, 5, 6
  Pin one subcommand name for jira-jql (`compose` vs `build`); document a shared CLI convention for the helper family.

- 🔵 **Standards**: `meta/integrations/` is a new top-level meta/ subdir not in paths.* schema
  **Location**: Phase 1 §1: Directory skeleton
  Add `paths.integrations` (default `meta/integrations`) to the configure SKILL.md table; read via `config-read-path.sh integrations` from `jira_state_dir`.

- 🔵 **Standards**: Dual-mode (executable + sourceable) script breaks established split
  **Location**: Phase 2 §4: jira-auth.sh
  Split into `jira-auth.sh` (sourceable-only) and a thin CLI wrapper, matching the work-item-common.sh / work-item-resolve-id.sh pattern.

- 🔵 **Standards**: Python fixture under bash/jq test-fixtures/ introduces a new convention
  **Location**: Phase 5 §1: mock-jira-server.py location
  Move the mock to `test-helpers/` or a sibling location distinct from `test-fixtures/` (which currently holds inert data files only).

- 🔵 **Standards**: YAML examples with `---` may interfere with configure SKILL.md frontmatter parser
  **Location**: Phase 1 §3
  Cross-check after implementation; use ` ```text ` fence if parser issues appear.

- 🔵 **Portability**: `${var%$'\n'}` only strips single trailing newline
  **Location**: Phase 2: jira-auth.sh
  Mirror of correctness finding.

- 🔵 **Portability**: `sed -E` slugify is locale-dependent
  **Location**: Phase 6: jira_field_slugify
  Force `LC_ALL=C` so character-class behaviour is deterministic; add a UTF-8 fixture.

- 🔵 **Portability**: curl --write-out and timing primitives differ across versions
  **Location**: Phase 5: jira-request.sh
  Use `EPOCHREALTIME` (bash 5+) or python3 for portable elapsed-time measurement; `date +%s.%N` is GNU-only.

- 🔵 **Portability**: jq version requirements not pinned
  **Location**: Implementation Approach
  Add jq to `mise.toml`'s `[tools]` or document minimum 1.6 in `jira-common.sh` header with a startup version check.

- 🔵 **Portability**: Vendor-coupling boundary implicit; no abstraction seam between request layer and skill layer
  **Location**: Overall plan
  Document in `jira-request.sh` header which contract elements are integration-specific (URL shape, auth scheme, 410/429 semantics) vs. universal — becomes the migration checklist when the second integration arrives.

#### Suggestions
- 🔵 **Standards**: Test-only env var `JIRA_BASE_URL_OVERRIDE` lacks documented naming convention
  **Location**: Phase 5 §3
  Rename to `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST` so the prefix and `_TEST` suffix make scope obvious; gate behind `JIRA_TEST_MODE=1`.

- 🔵 **Standards**: Markdown bold/italic alias canonicalisation undocumented
  **Location**: Phase 4
  State explicitly which form (`**bold**` vs `__bold__`) the renderer emits; add fixture pair using underscore form expecting canonical asterisk on second-compile output.

### Strengths
- ✅ Clean acyclic dependency cone (init-jira → fields → request → auth → common); each layer has one reason to change.
- ✅ Reuses existing extension points (content-agnostic config reader, plugin.json category-level registration, atomic-common.sh, vcs-common.sh) rather than forking infrastructure.
- ✅ Reuses `work.default_project_code` rather than introducing a parallel `jira.default_project_key` — avoids semantic drift.
- ✅ Functional core / imperative shell separation honoured: jira-jql.sh and ADF converters are pure; jira-request.sh is the I/O shell; jira-fields.sh composes the two.
- ✅ Resilience strategy at the request layer is explicit: bounded retries (4 attempts), Retry-After honoured with 60s cap, jittered backoff, per-status exit codes, connection-error mapping.
- ✅ TDD discipline is end-to-end with paired test scripts shipping before/alongside helpers; tests wired through an umbrella runner into `tasks/test.py`.
- ✅ ADF round-trip is treated as a property test against a fixture set rather than ad-hoc per-fixture assertions.
- ✅ JQL quoting exhaustively enumerated, including reserved words, embedded quotes, EMPTY sentinel, IN/NOT IN composition, and unsafe-character rejection.
- ✅ Network behaviour tested against a deterministic mock fixture rather than a live tenant, keeping CI hermetic.
- ✅ Token-redaction designed in, not retrofitted: dedicated test that greps stderr for the literal token value; `--debug` documented as redacted.
- ✅ Error messages use stable `E_*` prefixes that are greppable and scriptable.
- ✅ `E_FIELD_CACHE_MISSING` explicitly points users to `init-jira` or `jira-fields.sh refresh` — exemplary actionable error guidance.
- ✅ Sub-modes (`--list-projects`, `--list-fields`, `--refresh-fields`) provide composable progressive disclosure for power users.
- ✅ Bash style correctly characterised: sourceable libraries omit `set -euo pipefail`; CLI wrappers include it.
- ✅ Namespace prefix discipline (`jira_*` / `_jira_*`) faithfully mirrors `wip_*` / `_wip_*`.
- ✅ Out-of-scope section explicitly acknowledges Cloud-only coupling — vendor coupling is a conscious choice.
- ✅ Auth chain treats `op`/`pass`/`security`/`secret-tool` as opaque shell commands behind `token_cmd`, so the plugin never platform-couples to a specific password manager.
- ✅ Atomic writes via `atomic_write` are mandated for every persisted JSON cache.
- ✅ Out-of-scope section deliberately defers a generalised `http-request.sh` until a second integration appears — appropriate YAGNI restraint.

### Recommended Changes

Ordered by impact. Address critical findings before implementation begins; majors before merge; minors as time permits.

1. **Eliminate the three token-exfiltration vectors** (addresses: token_cmd supply-chain RCE; curl -u command-line leak; JIRA_BASE_URL_OVERRIDE)
   - Restrict `token_cmd` consumption to `accelerator.local.md` + env vars; refuse to honour it from `accelerator.md` (or require an opt-in `jira.allow_team_token_cmd: true` in the local file).
   - Pass curl credentials via `--config -` from stdin (or `--netrc-file <(...)`) so they do not appear in argv. Update the Phase 5 redaction test to assert the token is absent from `ps -o args=` output during a request.
   - Either gate `JIRA_BASE_URL_OVERRIDE` behind `ACCELERATOR_TEST_MODE=1` and validate it points to `127.0.0.1`/`localhost`, or move the seam out of `jira-request.sh` into a test-only wrapper.

2. **Strengthen the ADF round-trip property** (addresses: round-trip invariant too weak; "no silent drop" not asserted)
   - Add a second invariant: `render(compile(md)) == md` for every fixture in the supported subset, with documented Markdown canonicalisation rules.
   - Add a length-comparison check that catches silent content erasure.

3. **Fix portability landmines that would break tests on macOS or in minimal Linux containers** (addresses: bash 4 ${var,,}; uuidgen+xxd absence; BSD vs GNU awk; Python version; locale-dependent slugify)
   - Replace `${var,,}` with `tr '[:upper:]' '[:lower:]'`; add `LC_ALL=C` to slugify.
   - Use POSIX `od -An -N16 -tx1 /dev/urandom` (or python3, already a dep) for the localId fallback; format the result as a real UUID v4 with version/variant nibbles set.
   - Pin POSIX awk in the script header and ban gawk-isms; add a CI check or rewrite the tokeniser in Python.
   - Pin a minimum Python version in `mock-jira-server.py` with a `sys.version_info` guard.

4. **Eliminate hard-coded `meta/integrations/jira/` paths** (addresses: state-path indirection; paths.integrations missing from schema)
   - Add `paths.integrations` to the configure SKILL.md table and `paths` config schema.
   - Route every reference through `jira_state_dir` (which reads `paths.integrations`); avoid embedding the literal path in any SKILL.md prose.

5. **Validate `jira.site` and the request `<path>` argument** (addresses: site URL injection; path traversal)
   - Validate `jira.site` against `^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$` at every call site; reject with `E_BAD_SITE`.
   - Validate `<path>` against `^/rest/api/3/[A-Za-z0-9._/-]+$`; reject with `E_REQ_BAD_PATH`.
   - Add tests for `site: evil.com#`, `path: https://evil/x`, `path: /../../etc/passwd`.

6. **Fix the token-cmd whitespace trim and stderr handling** (addresses: single-newline trim; password-manager error leakage)
   - Trim all trailing whitespace including `\r\n\t ` (e.g. `awk '{sub(/[[:space:]]+$/,""); print}'`).
   - Discard `bash -c "$cmd" 2>/dev/null` by default; emit generic `E_TOKEN_CMD_FAILED: command exited <N>` rather than reproducing captured stderr.
   - Test with `dummy-token\r\n` input.

7. **Specify the `init-jira` site.json schema explicitly** (addresses: PII/token leakage to committed file)
   - Pin the schema to `{site, accountId, lastVerified}`; exclude `emailAddress`, `displayName`, `avatarUrls`.
   - Assert in tests that the persisted file contains exactly the documented keys.

8. **Replace eval scaffolding stub** (addresses: Phase 7 automated success criterion will fail as written)
   - Either ship a paired `benchmark.json` matching the schema in `skills/work/create-work-item/evals/benchmark.json`, or omit the `evals/` directory entirely. The linter only inspects directories that contain `evals.json`.

9. **Replace EMPTY sentinel and tighten JQL unsafe-character set** (addresses: EMPTY collision; over-restrictive denylist)
   - Require explicit `--empty <field>` flag for IS EMPTY clauses; exit `E_JQL_EMPTY_VALUE` on empty-string input.
   - Tighten unsafe-character rejection to control chars + backslash only; let printable punctuation (`#`, `?`, `/`, `[`, `]`, `*`, `;`, `|`, `@`, `^`, `$`, `%`) through after single-quote doubling.

10. **Fix `lastUpdated` byte-non-idempotency and add cache locking** (addresses: VCS churn on no-op refresh; concurrent-refresh tenant interleaving)
    - Drop `lastUpdated` from persisted files (track in a sibling gitignored file), or only update it when the `fields` array actually changed.
    - Acquire `flock` on `meta/integrations/jira/.lock` for multi-file refreshes.

11. **Rationalise the exit-code namespace** (addresses: sprawling and inconsistent codes; documentation gap)
    - Document the full map in one place (e.g. `EXIT_CODES.md` next to `jira-common.sh`); pin auth codes to numbers; explain the 14→19 gap or fill it.
    - Decide between HTTP-status mirroring and abstract names; do not mix.

12. **Specify Retry-After + jitter composition and HTTP-date parsing** (addresses: retry math ambiguity)
    - When `Retry-After` is present: sleep `min(Retry-After, 60)` with no jitter.
    - When absent: exponential schedule with ±30% jitter.
    - Parse delta-seconds and HTTP-date forms (or document the date-form fallback).
    - Add a `JIRA_RETRY_SLEEP_FN` test seam; replace wall-clock timing assertions with retry-count and sleep-sequence assertions.

13. **Add the missing test fixtures and contract tests** (addresses: ADF unicode/CRLF/large/empty fixtures; mock-server fidelity; concurrency on atomic writes; token-redaction stream coverage)
    - ADF: `unicode-mixed`, `link-with-parens`, `large-paragraph`, `crlf-input`, `empty-doc`.
    - Mock: capture real `/myself`, `/field`, `/project` responses into `test-fixtures/api-responses/`; assert byte-identical playback.
    - Atomic writes: concurrent writers, kill-9 mid-write, cross-device, symlink, unwritable dir.
    - Redaction: pollute token with `tok-SENTINEL-xyz`; grep stderr/stdout/temp-dirs/ps-output for sentinel after every test.

14. **Standards alignment fixes** (addresses: plugin.json ordering, disable-model-invocation, mock script location)
    - Drop the "alphabetically" framing in Phase 1 §2; document the workflow ordering.
    - Add `disable-model-invocation: true` to init-jira frontmatter.
    - Move `mock-jira-server.py` out of `test-fixtures/` (which holds inert data) into a `test-helpers/` sibling directory.
    - Split `jira-auth.sh` into sourceable-only library + thin CLI wrapper, matching the existing convention.

15. **Make Phase 7 orchestration testable** (addresses: SKILL-vs-helper deferred decision; init-jira lacks automated coverage)
    - Lift the eight-step flow into `jira-init-flow.sh` (bash, testable); SKILL.md becomes a thin user-facing wrapper.
    - Add `test-jira-init-flow.sh` covering each flag combination against the mock.

16. **Document the resolution path as data, not prose** (addresses: six-step chain not first-class; site/email asymmetry)
    - Have `jira_resolve_credentials` set `JIRA_RESOLUTION_SOURCE_TOKEN=env|env_cmd|local|local_cmd|shared|shared_cmd`; the `--debug` line becomes a reflection of that variable.
    - Decide on env-var support for site/email and apply consistently (or document the asymmetry).

17. **Improve usability of error messages and discoverability** (addresses: `E_JQL_UNSAFE_VALUE` opacity, advanced-flag discoverability, `~` negation surprise, `jira.token` invitation, default-project visibility)
    - `E_JQL_UNSAFE_VALUE` names offending character and value.
    - Error paths surfacing missing/stale cache include `(hint: run /init-jira --list-fields)`.
    - Document `~` negation prominently in helper headers; consider adding `--not-status` flag aliases.
    - Move `jira.token` to a "Local-only — do not commit" sub-paragraph in the configure SKILL; warn at runtime if seen in `accelerator.md`.
    - Surface a `Default project: <key>` line in `/init-jira` summary output.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally well-structured: follows established skill-category and bash-helper conventions, layers seven helpers on a clean dependency cone (common → auth → jql/adf → request → fields → init-jira), and reuses existing config reader and atomic-write primitives unchanged. The principal risks are (a) hard-coding `meta/integrations/jira/` despite an explicit deferred reorg note, (b) the JIRA_BASE_URL_OVERRIDE env-var test seam acting as a load-bearing implementation detail without abstraction, and (c) lack of any explicit abstraction at the boundary between Jira-specific helpers and the future second integration.

**Strengths**: acyclic dependency cone, reuse of existing extension points, reuse of `work.default_project_code`, functional-core/imperative-shell separation, explicit resilience strategy, token-redaction as cross-cutting concern, TDD ordering aligned with dependency order.

**Findings**: 6 (1 major architecture concern on hard-coded state path; 1 major on JIRA_BASE_URL_OVERRIDE encapsulation; 4 minors on transport abstraction deferral, ADF compiler complexity, six-step resolution chain, deferred SKILL-vs-helper split).

### Security

**Summary**: Solid foundations for redaction and JQL safety, but several high-impact attack surfaces are under-specified: token_cmd executed via bash -c against committed config (supply-chain RCE), curl -u places the token in the process command line (contradicting plan's claim), JIRA_BASE_URL_OVERRIDE is a live token-exfil sink in production code, jira.site is interpolated into the URL with no validation.

**Strengths**: token redaction explicitly tested via stderr grep, JQL single-quote doubling with --unsafe gate, atomic writes prevent partial-state corruption, CI tests use 127.0.0.1 mock fixture, retry budget bounded, helpers emit raw JSON narrowing output-encoding bugs.

**Findings**: 11 (3 critical: token_cmd injection, curl -u in argv, JIRA_BASE_URL_OVERRIDE; 5 major: site validation, token_cmd stderr, --debug curl verbose, site.json schema, accelerator.local.md permissions; 3 minor: stderr control chars, path validation, ADF jq --arg).

### Test Coverage

**Summary**: Admirably TDD-driven with per-helper test scripts and a mock HTTP server, but several concrete gaps exist: ADF round-trip invariant is too weak (fixed-point on ADF rather than Markdown equivalence), mock server fidelity unverified against real Jira, backoff timing tests at risk of flakiness, and important categories (unicode, large payloads, malformed/empty server responses, atomic-write concurrency, port-binding portability, exhaustive token-redaction across all streams/formats) are not exercised.

**Strengths**: TDD discipline explicit; round-trip as property test; JQL quoting exhaustively enumerated; network behaviour against deterministic mock; token-redaction first-class testable; umbrella aggregator pattern; rejection-only and rendering-only ADF fixture categories.

**Findings**: 13 (8 major: round-trip invariant, mock fidelity, timing flakiness, mock lifecycle, redaction coverage, malformed/large responses, ADF unicode fixtures, atomic-write concurrency; 5 minor: field-name collisions, JQL fuzz testing, init-jira automation, umbrella aggregation robustness, localId determinism).

### Code Quality

**Summary**: Well-structured: follows existing project conventions, commits to TDD throughout, divides work into seven coherent milestones. Main concerns: ADF compiler size in pure bash (250-400 lines is at the upper edge of maintainability), kitchen-sink shape of jira-common.sh, sprawling exit-code namespace partitioning, and a few places where bash conventions risk fragility (token-cmd via bash -c, ad-hoc Markdown tokenisation).

**Strengths**: pure/network helper separation, namespace discipline, TDD, token-redaction designed in, test seam acknowledged, atomic writes layered on existing primitive.

**Findings**: 8 (3 major: ADF compiler maintainability, jira-common.sh kitchen-sink, exit-code namespace inconsistency; 5 minor: bash -c fragility, uuidgen fallback, JQL denylist drift, deferred SKILL-vs-helper, retry policy mixing).

### Correctness

**Summary**: Strong correctness instincts (TDD-first, atomic writes, explicit exit-code mapping, ADF round-trip property) but several specific logic gaps will produce incorrect behaviour for plausible real-world inputs. Most consequential: under-specified token-cmd trimming, JQL EMPTY sentinel collision, round-trip property defined as fixed point on second compile, UUID fallback that does not produce a valid UUID. Idempotency and concurrency claims stated more strongly than the design supports.

**Strengths**: TDD-first ordering, atomic writes mandated, exit-code mapping explicit per helper, JQL single-quote doubling identified as canonical, ADF round-trip as property test, token redaction asserted under --debug.

**Findings**: 13 (7 major: token-cmd \n trim, EMPTY sentinel, round-trip property, UUID fallback, concurrent refresh, Retry-After+jitter, lastUpdated non-idempotency; 6 minor: bash 4 ${s,,}, JQL backslash, 410/404 overlap, field resolver ambiguity, env-var asymmetry, inline tokeniser order).

### Usability

**Summary**: Strong usability fundamentals (stable error prefixes, idempotency, atomic writes, clear init flow, thoughtful auth chain), but several friction points: sprawling and inconsistent exit-code namespace, JQL ~ negation prefix is non-standard and surprising, reusing work.default_project_code creates implicit coupling, explicit removal of --plain/--csv forces every shell user to pipe through jq. Discoverability of advanced flags and absence of --help convention are also gaps.

**Strengths**: well-designed auth resolution chain, token redaction taken seriously, stable E_* error prefixes, idempotency stated and supported, concrete first-run experience, sub-modes provide progressive disclosure, E_FIELD_CACHE_MISSING points users to recovery actions, TDD pins error-message contracts.

**Findings**: 10 (3 major: sprawling exit codes, ~ negation prefix, hard refusal of --plain/--csv; 7 minor: work.default_project_code reuse discoverability, sub-mode discoverability, 8-step flow weight, E_JQL_UNSAFE_VALUE messaging, ADF placeholder asymmetry, jira.token in shared config, inconsistent CLI argument styles).

### Standards

**Summary**: Adheres closely to most conventions (namespace prefixes, set -e split, E_* prefixes, find_repo_root, test-helpers.sh sourcing, configure SKILL.md formatting). Real deviations: plugin.json insertion claim is misleading (array is workflow-ordered, not alphabetical), init-jira frontmatter omits disable-model-invocation that every other slash-only skill uses, eval scaffolding under-specifies the benchmark.json gate, and meta/integrations/ introduces a new top-level meta/ subdir not in paths.* schema.

**Strengths**: bash style correctly characterised, namespace discipline mirrored, stable E_* prefixes, test-script conventions match, work.default_project_code reuse, configure SKILL.md table format, bang-prefix preprocessor invocations match, argument-hint and allowed-tools syntax match.

**Findings**: 9 (3 major: plugin.json ordering claim, missing disable-model-invocation, eval scaffolding; 4 minor: paths.integrations missing, dual-mode jira-auth.sh, Python under test-fixtures, YAML --- inside fences; 2 suggestions: JIRA_BASE_URL_OVERRIDE naming, bold/italic alias canonicalisation).

### Portability

**Summary**: Targets macOS and Linux but builds on tool assumptions that diverge across them: bash 4+ syntax (${var,,}) on macOS where /bin/bash is 3.2, BSD-vs-GNU sed/awk differences in slugifier and ADF tokeniser, optional-binary fallback chain for uuidgen/xxd that does not cover absence of both. Python 3 required for mock server but no minimum version pinned, mise.toml does not declare a bash version.

**Strengths**: Cloud-only coupling explicit, auth chain treats password managers as opaque shell commands, jira-request.sh has stable internal contract with deferred generalisation, test seam allows mock substitution, inherits established repo conventions.

**Findings**: 9 (4 major: ${var,,} bash 4, uuidgen/xxd fallback, BSD vs GNU awk, Python version unspecified; 5 minor: ${var%$'\n'} single-newline trim, sed -E locale, curl --write-out and timing, jq version unpinned, vendor-coupling boundary implicit).

---

## Re-Review (Pass 2) — 2026-04-30

**Verdict:** REVISE

The revision substantively closes the prior critical and most major findings — all three token-exfiltration vectors are gone, the round-trip property is now strengthened with three layered invariants, the exit-code namespace is consolidated, the kitchen-sink helpers are split, and portability landmines (bash 4 syntax, missing UUID fallback, BSD/GNU awk, Python pinning) have concrete fixes. The plan is materially safer and more correct than v1. However the revision introduced a smaller second tier of issues — some are real bugs from the edits themselves (a JSON file cannot have a header comment; a `lastVerified` timestamp recreates the byte-non-idempotency that `lastUpdated` was just removed for; the path regex still admits `/../` traversals; flock is not on macOS; the awk record format uses tab as separator without specifying tab-in-content escape rules). Recommend one more targeted pass focused on these regressions and the few persistent usability issues (`~` negation, `--plain`/`--csv`, eight-step flow), then APPROVE.

### Previously Identified Issues

#### Critical (all resolved)
- ✅ **Security**: token_cmd from committed accelerator.md is a supply-chain command-injection sink — **Resolved**: Phase 1 §3 + Phase 2 §3 case 6 now refuse `token_cmd` from `accelerator.md` with `E_TOKEN_CMD_FROM_SHARED_CONFIG`.
- ✅ **Security**: curl -u places token in process command line — **Resolved**: Phase 5 §3 replaced `-u` with `printf | curl --config -`; cases 13/14 assert sentinel absent from `ps -o args=` and `/proc/<pid>/cmdline`.
- ✅ **Security**: JIRA_BASE_URL_OVERRIDE is a live token-exfiltration sink — **Resolved**: renamed to `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST`, double-gated by `ACCELERATOR_TEST_MODE=1` + loopback validation; cases 17/18 verify both gates fail-closed.

#### Major (mostly resolved; ~3 partially resolved)
- ✅ Architecture: state path hard-coded — Resolved (paths.integrations + jira_state_dir).
- ✅ Architecture: JIRA_BASE_URL_OVERRIDE encapsulation — Resolved.
- ✅ Security: jira.site URL injection — Resolved (DNS-label regex validation, case 24).
- ✅ Security: token_cmd stderr leakage — Resolved (`2>/dev/null` default, `--debug-token-cmd` opt-in).
- ✅ Security: --debug enabling curl verbose — Resolved (explicit prohibition + cases 14-15).
- ✅ Security: site.json schema unspecified — Resolved (pinned to `{site, accountId, lastVerified}`).
- ⚠️ Security: accelerator.local.md permissions — **Partially resolved**: warning emitted, but action proceeds; reviewer flagged this should be fail-closed.
- ✅ Test Coverage: ADF round-trip too weak — Resolved (3 invariants).
- ✅ Test Coverage: Mock fidelity unverified — Resolved (cassettes + fidelity test).
- ✅ Test Coverage: Backoff timing flakiness — Resolved (JIRA_RETRY_SLEEP_FN seam).
- ✅ Test Coverage: Mock lifecycle untested — Resolved (Python guard, URL-file timeout, orphan sweep).
- ✅ Test Coverage: Token redaction single-stream — Resolved (sentinel across 6+ surfaces).
- ✅ Test Coverage: Malformed/empty/large responses — Resolved (cases 20-23).
- ✅ Test Coverage: ADF unicode/edge fixtures — Resolved (8 new fixtures added).
- ✅ Test Coverage: Atomic-write concurrency — Resolved (concurrent + kill-9 + cross-device + symlink + unwritable).
- ⚠️ Code Quality: ADF compiler maintainability — **Partially resolved**: split into 3 artefacts with documented record contract, but reviewer notes the cross-language burden remains and the awk-record format has tab-handling concerns.
- ✅ Code Quality: jira-common.sh kitchen-sink — Resolved (log_die/log_warn lifted; header-comment grouping).
- ✅ Code Quality: Exit-code namespace inconsistency — Resolved (EXIT_CODES.md manifest, auth codes pinned).
- ✅ Correctness: Token-cmd single-newline trim — Resolved (awk-based whitespace strip; case 12).
- ✅ Correctness: EMPTY sentinel collision — Resolved (`--empty <field>` flag; `E_JQL_EMPTY_VALUE`).
- ✅ Correctness: Round-trip fixed-point too weak — Resolved (Markdown round-trip invariant added).
- ✅ Correctness: Malformed UUID fallback — Resolved (uuidgen → python3 → POSIX od with v4 nibbles).
- ✅ Correctness: No locking on cache — Resolved (flock + mkdir fallback; `E_REFRESH_LOCKED`).
- ✅ Correctness: Retry-After+jitter unspecified — Resolved (server-supplied Retry-After bypasses jitter; HTTP-date supported).
- ⚠️ Correctness: lastUpdated byte-non-idempotency — **Partially resolved**: `lastUpdated` moved to sibling `.refresh-meta.json`, but reviewer flagged `lastVerified` field in `site.json` recreates the same problem.
- ⚠️ Usability: Sprawling exit-code namespace — **Partially resolved**: EXIT_CODES.md helps; reviewer notes the namespace still spans 11–69 with mixed encoding rules.
- ❌ Usability: `~` JQL negation prefix — **Still present**: no change; reviewer reflagged as major.
- ❌ Usability: Hard refusal of --plain/--csv — **Still present**: no change; reviewer reflagged as major.
- ✅ Standards: plugin.json alphabetical claim — Resolved (workflow ordering documented).
- ✅ Standards: missing disable-model-invocation — Resolved.
- ✅ Standards: Eval scaffolding gate — Resolved (omitted entirely).
- ✅ Portability: bash 4 ${var,,} — Resolved (tr-based slugify).
- ⚠️ Portability: uuidgen/xxd fallback — **Resolved with new caveat**: chain is correct now, but reviewer flags `/dev/urandom` availability as a fourth-tier consideration in restricted containers.
- ⚠️ Portability: BSD vs GNU awk — **Partially resolved**: POSIX awk pinned; reviewer flags tab-as-FS interaction with tab-bearing content as a new concern.
- ✅ Portability: Python version unspecified — Resolved (3.9+ guard).

### New Issues Introduced

#### Major (real regressions and new gaps)

- 🟡 **Standards**: `plugin.json` header comment is invalid JSON — Phase 1 §2 says "Document this ordering rule in a header comment in plugin.json", but JSON does not support comments and Phase 1's own success criterion runs `jq -e ... plugin.json`. Move the rationale to `skills/config/configure/SKILL.md` or a sibling `.claude-plugin/README.md`.
- 🟡 **Correctness**: `lastVerified` in `site.json` recreates byte-non-idempotency — the field is documented as part of the committed schema, so consecutive `/init-jira` runs against the same tenant produce different bytes. Move to `.refresh-meta.json` to match the `fields.json` fix.
- 🟡 **Correctness**: Path validation regex still admits `/../` traversal — `^/rest/api/3/[A-Za-z0-9._/?=&,:%@-]+$` matches `/rest/api/3/issue/../../field` because `.` and `/` are in the class. Add an explicit reject of `(^|/)\.\.(/|$)` and reject `//`.
- 🟡 **Correctness**: Awk record format uses tab as separator without tab-in-content escaping — code blocks legitimately contain tabs; a `CODE_LINE\t<text>` record where `<text>` contains a tab corrupts the field stream. Either escape tabs in payloads or use ASCII RS/US separators.
- 🟡 **Correctness**: Canonicalisation under-specified for nested mark forms — `__bold _italic_ bold__` has multiple plausible canonical outputs depending on regex order. Specify a delimiter-stack algorithm and add nested-mark fixtures.
- 🟡 **Correctness**: Invariant 3 (90% length floor) too coarse — a compiler bug that drops every URL while keeping link text passes the 90% threshold. Replace with structural marker-counting assertions.
- 🟡 **Correctness**: HTTP-date Retry-After can produce negative or zero sleep — clock skew or expired dates yield negative deltas; bash `sleep` rejects negatives. Specify `max(0, min(parsed, 60))` with a 1s floor.
- 🟡 **Portability**: flock is Linux-only — macOS does not ship `flock(1)`; the test asserting concurrent serialisation (Phase 6 §1 case 13) will exercise the mkdir fallback on the user's primary dev platform, which the test does not explicitly cover. Either commit to mkdir-only locking or add a CI matrix entry exercising both paths.
- 🟡 **Portability**: HTTP-date parser lacks tz/version-error handling — `parsedate_to_datetime` returns naive datetime on tz-less input; subtracting raises TypeError on Python 3.9. Specify the try/except + UTC normalisation.
- 🟡 **Test Coverage**: HTTP-date Retry-After parser has only happy-path test — add cases for past dates, malformed dates, RFC-850/asctime forms, >60s clamp boundary.
- 🟡 **Test Coverage**: mkdir-based lock fallback never exercised by tests — the macOS-default path is untested; force the fallback via `JIRA_LOCK_IMPL=mkdir` env override and run the same concurrent-acquisition assertions.
- 🟡 **Test Coverage**: Mock fidelity test pass criteria vague — "shape matches" is undefined; specify `jq -S .` byte-equal comparison + exact-match on a header allow-list.
- 🟡 **Test Coverage**: UUID determinism seed and POSIX-od fallback have no direct tests — both code paths are dead unless explicitly exercised. Add tests that PATH-strip uuidgen+python3 and assert v4-shape regex + version/variant nibbles.
- 🟡 **Test Coverage**: Slugify locale-independence asserted only via fixtures, not multi-locale execution — run slugify under `LC_ALL=tr_TR.UTF-8` and `LC_ALL=C` and assert byte-identical output.
- 🟡 **Security**: accelerator.local.md permissions warning is fail-open — change to fail-closed (`E_LOCAL_PERMS_INSECURE`) with an opt-out env var for unusual filesystems.
- 🟡 **Security**: Path regex permits encoded traversal forms — validate after one URL-decode; reject `%2e%2e%2f` and other encoded `..` forms.
- 🟡 **Security**: API response bodies on stderr can leak Authorization header echoes — run a redaction pass before emit; cap to 8 KiB; link to tempfile for full body.
- 🟡 **Security**: JIRA_RETRY_SLEEP_FN is an ungated code-injection sink — gate behind `ACCELERATOR_TEST_MODE=1` matching the URL override, validate function name against an allow-list.
- 🟡 **Security**: ADF compiler lacks input-size cap — specify `E_ADF_TOO_LARGE` (e.g. 1 MB) for both directions.
- 🟡 **Standards**: `log_die`/`log_warn` prefix breaks repo-level convention — existing `find_repo_root`/`atomic_write` are unprefixed; either drop the prefix or document the new convention explicitly.
- 🟡 **Code Quality**: Helper directory now hosts ~12 production files in flat layout — introduce `scripts/lib/` subdirectory for sourceable libs, or add a `SCRIPTS.md` catalogue tagging each artefact's role.
- 🟡 **Code Quality**: ADF compiler still spans bash + awk + jq with cross-language contract — extract jq into `--from-file` proactively; add a sentinel test round-tripping payloads with literal tabs/newlines through the awk record format.
- 🟡 **Usability**: Double env-var test override is confusing for test authors — collapse `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST` + `ACCELERATOR_TEST_MODE` into a single var (the loopback-only check provides the safety guarantee), or have the rejection error name the missing companion.

#### Minor

- 🔵 Architecture: jira-fields.sh diverges from the lib/CLI split adopted elsewhere; either split it too or document the rule for when to split.
- 🔵 Architecture: Lock primitive is Jira-scoped but reusable across integrations; consider lifting to `scripts/lock-common.sh` later.
- 🔵 Architecture: Auth library couples directly to config-file resolution; consider a `_jira_resolve_from_source` indirection when a second auth source is added.
- 🔵 Architecture: Awk-record contract enforced only by jq input parser; add a fuzz/grammar test asserting every emitted record matches the documented regex.
- 🔵 Code Quality: Resolver mutates 6 caller-scope globals; either document explicitly or return via stdout `KEY=VALUE` lines.
- 🔵 Code Quality: jira-common.sh still bundles 5 disjoint concerns post-revision; consider relocating `_jira_uuid_v4` to ADF family and `jira_with_lock` to a shared `lock-common.sh`.
- 🔵 Code Quality: Test seams are not catalogued; add `TEST_SEAMS.md` (or section in EXIT_CODES.md) listing every test-only env var.
- 🔵 Code Quality: EXIT_CODES.md is load-bearing without enforcement; add `scripts/test-jira-exit-codes.sh` asserting every literal exit/return is documented.
- 🔵 Correctness: Inline tokeniser recursion rule still ambiguous for code-inside-link, multiple-marks-in-link-text; add fixtures.
- 🔵 Correctness: Backoff jitter test asserts ±30% from captured values, but seed is wall-clock — add `JIRA_RETRY_JITTER_SEED` env var for determinism.
- 🔵 Correctness: tr `[:upper:]` `[:lower:]` under `LC_ALL=C` does not lowercase non-ASCII — explicitly document slug as ASCII-only or transliterate via `iconv//translit`.
- 🔵 Correctness: URL-file readiness wait races against partial writes — write through `.tmp` + atomic rename.
- 🔵 Correctness: JQL fuzz regex has typo (`'''` vs `''`); fix to `^'([^'\x00-\x1f]|'')*'$`.
- 🔵 Test Coverage: Wall-clock band still in Phase 5 Success Criteria despite seam introduction; drop it.
- 🔵 Test Coverage: Multi-MB / Unicode body cases assert length but not memory/streaming behaviour.
- 🔵 Test Coverage: init-jira tests do not exercise interactive prompt branch — pipe scripted stdin.
- 🔵 Test Coverage: Canonicalise pre-pass has no dedicated unit-level coverage.
- 🔵 Test Coverage: Site validation regex test corpus is thin; add 8–10 boundary inputs.
- 🔵 Test Coverage: Multipart upload case is single-file happy-path only; add filename-with-spaces and missing-file cases.
- 🔵 Test Coverage: JQL fuzz check verifies syntax not round-trip; assert quoted output parses back to input.
- 🔵 Test Coverage: Byte-idempotency check ignores key ordering produced by jq; specify `jq -S` for committed caches.
- 🔵 Usability: lib-vs-CLI naming split is implicit; add a one-line guard in libs that errors when executed directly.
- 🔵 Usability: `work.default_project_code` reuse remains undiscoverable from the work side; add reciprocal note.
- 🔵 Usability: Sub-mode discoverability (`--list-fields`, `--refresh-fields`) relies on argument-hint; add `--help` and surface in summary line.
- 🔵 Usability: Eight-step `/init-jira` flow unchanged; add `--quick` mode for users with everything pre-configured.
- 🔵 Usability: Splitting compiler into 3 files multiplies surface area; add ASCII pipeline diagram in orchestrator header.
- 🔵 Usability: Two debug flags (`--debug` vs `--debug-token-cmd`) is a confusing matrix; either merge or surface the second flag in the relevant error message.
- 🔵 Usability: `jira.token` documented as recognised key still invites accidental commits; either remove from `accelerator.md`'s recognised set or add a pre-commit guard.
- 🔵 Standards: New `test-helpers/` category not formalised project-wide; add a note to Testing Strategy or a CONTRIBUTING doc.
- 🔵 Standards: `paths.integrations` description differs in tone from existing rows ("future Linear/Trello"); use present-tense.
- 🔵 Standards: YAML fence examples still contain `---` delimiters; verify the existing `work` section's shape and align.
- 🔵 Standards: EXIT_CODES.md is a new artefact format with no precedent; commit to it project-wide or remove and use inline header comments.
- 🔵 Standards: Markdown alias canonicalisation rule belongs in user-facing prose, not just plan.
- 🔵 Portability: `/proc/<pid>/cmdline` does not exist on macOS; specify the macOS-equivalent (`ps -ww -o command=`) and assert at least one introspection path runs on every platform.
- 🔵 Portability: `EPOCHREALTIME` requires bash 5+; specify the python3 fallback detection and document the bash 5 dependency.
- 🔵 Portability: jq version detection across distros is non-uniform (Debian buster's `jq-1.5-1-a5b5cbe` form); specify the regex.
- 🔵 Portability: `/dev/urandom` assumed available without explicit handling for restricted containers; document the fourth-tier failure trigger.
- 🔵 Portability: curl `--config` quoting differs across versions; validate token shape (`^[A-Za-z0-9_=+/.-]+$`) before piping; pin curl ≥7.55.
- 🔵 Portability: Interactive prompt portability not specified; check `[ -t 0 ]` and exit `E_INIT_NEEDS_CONFIG` on non-TTY.
- 🔵 Portability: JSON byte-idempotency requires deterministic key order; mandate `jq -S` for all committed cache writes.
- 🔵 Security: token_cmd ignored from shared config — promote rejection to a structural lint rather than runtime check.
- 🔵 Security: Test-determinism seeds (`JIRA_ADF_LOCALID_SEED`, `JIRA_RETRY_SLEEP_FN`) inconsistent with `ACCELERATOR_TEST_MODE` gating; unify the policy.
- 🔵 Security: curl `--config` heredoc requires escaping when token contains `"` or `\`; validate token shape or escape.
- 🔵 Security: Committed `projects.json` / `fields.json` schemas filtered but PII-free assertion not in tests; mirror the site.json allow-list assertion.
- 🔵 Security: HTTP-date parser locale; force `LC_ALL=C` for the python invocation.

### Assessment

The plan is materially safer than v1 — all three critical security findings are closed, the round-trip property is now strong, and most architectural and standards concerns have concrete fixes. Mathematically the verdict remains REVISE because the count of new and partially-resolved major findings (≈22) substantially exceeds the threshold, but the *shape* of the remaining work is qualitatively different: instead of three load-bearing critical defects, we have a tier of polish-level bugs (the JSON-comment regression, the `lastVerified` reintroduction, the path-regex traversal gap, the macOS flock divergence) plus persistent usability concerns the user has explicitly chosen to defer (`~` negation, `--plain`/`--csv`, eight-step flow).

A focused third pass addressing the ten "real regression" items (JSON-comment, lastVerified, path regex, awk tab handling, canonicalisation rules, Invariant 3 strengthening, HTTP-date negative sleep, flock platform coverage, accelerator.local.md fail-closed, JIRA_RETRY_SLEEP_FN gating) would land the plan at APPROVE. The remaining minors can be queued for a Group H follow-up after implementation begins, since most are about test-coverage hardening rather than design changes.

---

## Re-Review (Pass 3) — 2026-04-30

**Verdict:** REVISE

The third-pass edits closed 8 of the 10 named regressions cleanly: the JSON header comment is gone, `lastVerified` moved out of `site.json`, the awk record format uses ASCII US/RS separators, Invariant 3 became structural-marker counting, the HTTP-date parser clamps `max(1, min(parsed, 60))`, `flock` is gone in favour of mkdir-only locking, `accelerator.local.md` permissions fail-closed, and `JIRA_RETRY_SLEEP_FN` requires `ACCELERATOR_TEST_MODE=1` plus a name allow-list. **Two of the ten are partially resolved**: the path regex still admits double-encoded `%252e%252e` traversal because URL-decoding runs only once, and the canonicalisation algorithm's "top-of-stack-only" matching rule mishandles interleaved `__` and `_` delimiters in inputs like `_a __b_ c__` or `snake_case_variable`. The third pass also revealed several **prose-vs-spec drifts** — Phase 2 §2b still describes `jira_with_lock` as flock-based with mkdir fallback, and Phase 2 §5 manual verification still describes a "world-readable warning" from the old warn-only model — and surfaced a new tier of design concerns the second-pass edits did not anticipate, most notably a PID-recycling race in the new mkdir-lock stale-recovery loop, a stat-then-read TOCTOU on `accelerator.local.md`, and an `ACCELERATOR_ALLOW_INSECURE_LOCAL` opt-out that any dotfile can set to neutralise the fail-closed protection.

The plan continues to improve materially. Verdict remains REVISE because the major-finding count exceeds the threshold, but the work is bounded and well-defined: a fourth pass closing the four prose drifts, the three real regression remainders (URL double-decode, canonicalisation rule, perms TOCTOU), and one missing exit code would land the plan at APPROVE.

### Previously Identified Issues (10 regressions from Pass 2)

- ✅ **#1 plugin.json header comment** — Resolved. Phase 1 §2 now points at a "Skill registration order" subsection in `configure/SKILL.md`.
- ✅ **#2 lastVerified in site.json** — Resolved. Moved to gitignored `.refresh-meta.json`; `projects.json` likewise; site.json schema pinned to `{site, accountId}` only.
- ⚠️ **#3 path regex traversal** — Partially resolved. Four-check pipeline added (regex + `..` + `//` + control chars, both literal and URL-decoded), but the URL-decode step runs only once, so double-encoded forms (`%252e%252e%252f`) survive both passes. Fix: iterate decode to fixed point with a small cap.
- ✅ **#4 awk tab-FS** — Resolved. ASCII US (`\x1f`) / RS (`\x1e`) separators with literal-byte rejection in input; payloads carry tabs/newlines verbatim.
- ⚠️ **#5 canonicalisation under-specified** — Partially resolved. Delimiter-stack algorithm specified with longest-match rule, but the "top-of-stack only" matching mis-handles interleaved `__`/`_` (e.g. `_a __b_ c__` leaves the stack stuck; `snake_case_variable` produces wrong output). Fix: tighten the supported subset to disallow underscore-form, or adopt CommonMark's left/right-flanking heuristic.
- ✅ **#6 Invariant 3 too coarse** — Resolved. Structural-marker counting (markers in URLs/code lang/task state) replaces the 90% length floor; length floor retained as cheap secondary check.
- ✅ **#7 HTTP-date negative sleep** — Resolved. `max(1, min(parsed, 60))` clamp + try/except + UTC normalisation + LC_ALL=C; five new test scenarios (past, malformed, >60s, no-tz, RFC-850).
- ✅ **#8 flock platform** — Resolved at the Implementation Approach level (mkdir-only locking on every platform with kill -0 stale recovery). **Drift:** Phase 2 §2b's `jira-common.sh` header comment still describes `jira_with_lock` as "flock-based ... falls back to mkdir" — stale prose from Pass 2.
- ✅ **#9 accelerator.local.md fail-closed** — Resolved at the contract level (`E_LOCAL_PERMS_INSECURE` exit 29; `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` opt-out). **Drift:** Phase 2 §5 manual verification still says "confirm the `Warning: accelerator.local.md is world-readable` message appears on stderr" — wording carried over from the warn-only model.
- ✅ **#10 JIRA_RETRY_SLEEP_FN gating** — Resolved. Requires `ACCELERATOR_TEST_MODE=1` + `^_?test_[a-z_]+$` name match + `declare -F` caller-scope check; `JIRA_ADF_LOCALID_SEED` follows the same policy. New test cases 24-25 verify both gates.

### New Issues Introduced

#### Major (regressions and new design concerns)

- 🟡 **Architecture / Code Quality**: Phase 2 §2b's `jira-common.sh` header comment still describes `jira_with_lock` as "flock-based ... falls back to mkdir advisory lock" — stale from Pass 2's earlier dual-path design. Update to "mkdir-based atomic exclusive lock with kill -0 stale recovery". Phase 2 §1 also refers to "the flock wrapper".
- 🟡 **Usability / Architecture**: Phase 2 §5 manual verification step contradicts the new fail-closed perms behaviour — describes a "world-readable warning" appearing for a chmod 644 file, but the new contract is a hard exit with `E_LOCAL_PERMS_INSECURE`. Replace with two steps: (1) chmod 644 → exit 29; (2) `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` chmod 644 → downgrade warning + success.
- 🟡 **Standards**: `E_TEST_HOOK_REJECTED` is referenced repeatedly (test-seam policy in EXIT_CODES.md, Phase 5 cases 24-25) but has no assigned exit-code number. The request range 11-22 has no unassigned slots and 23 is reserved for `E_REQ_TIMEOUT`. Pin to a specific number — claim 23 for it (and renumber the timeout reservation), or carve a new shared "test-seam rejection" range.
- 🟡 **Standards**: `log_die`/`log_warn` naming still inconsistent with cited convention. Phase 2 §2a says they "match the existing `find_repo_root`/`atomic_write` style" but those helpers are unprefixed. Either rename to `die`/`warn`, or rewrite the rationale to acknowledge the deliberate departure (collision avoidance for short identifiers).
- 🟡 **Security**: PID-recycling race in mkdir-lock stale-recovery — `kill -0 $holder_pid` succeeds against a recycled PID, blocking the contender for 60 s; the rm-rf branch has a TOCTOU window where a freshly-acquired holder can be silently kicked out. Stamp `holder.pid` with both PID and process start-time (`ps -o lstart=` or `/proc/$$/stat`) and require both to match before declaring the holder stale.
- 🟡 **Security**: `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` is honoured from any environment source — a `.envrc`, malicious shell rc, compromised dependency, or single-line PR to a CI YAML neutralises the fail-closed protection. Tighten the opt-out to require both the env var AND a committed marker file (`.claude/insecure-local-ok`), or rotate to a per-invocation CLI flag (`jira-auth-cli.sh --allow-insecure-local`).
- 🟡 **Security**: Stat-then-read TOCTOU on `accelerator.local.md` — between `stat` and `cat`, an attacker with write access to `.claude/` can swap the file or symlink. Use `lstat` to reject symlinks, then open with `O_NOFOLLOW` and `fstat` the open fd before reading.
- 🟡 **Correctness**: URL-decode path check decodes only once — admits `%252e%252e%252f` because one decode produces `%2e%2e%2f` (no literal `..`) and intermediaries may complete the second decode server-side. Iterate decode to fixed point with a small cap (e.g. 8 iterations).
- 🟡 **Correctness**: Canonicalisation top-of-stack-only matching mishandles interleaved delimiters. `_a __b_ c__` leaves the stack in an irreducible state; `snake_case_variable` produces unexpected output because three `_` push/pop/push leaves a literal `_` in the middle. Either restrict the supported subset to disallow underscore-form delimiters, or adopt CommonMark's left/right-flanking heuristic.
- 🟡 **Test Coverage**: Dead-holder lock recovery is mentioned but has no explicit named test case. Phase 6 case 13 covers live-holder serialisation only. Add a Phase 2 case: write a known-dead PID to `holder.pid`, invoke `jira_with_lock`, assert acquisition within ~200 ms.
- 🟡 **Test Coverage**: Malformed HTTP-date fallback warning is specified but not asserted by any numbered test case. The expanded coverage list mentions "malformed string falls back to jittered backoff" but no enumerated case asserts the warning string + sleep behaviour. Add Phase 5 case 9b.
- 🟡 **Test Coverage**: New mark-canonicalisation fixtures (`nested-mixed-marks`, `unmatched-delimiter`, `code-block-with-tabs`, `reject-control-chars`) are listed without explicit named test cases — the fixture-pair-sweep convention assumes paired fixtures, but rejection-only and canonicalisation-only fixtures need explicit logic. Add named cases asserting each.
- 🟡 **Code Quality**: Canonicalisation delimiter-stack scanner specified prose-only with no test coverage of stack mechanics in isolation. Either restrict the supported subset to a simpler algorithm or extract canonicalisation into its own `jira-md-canonicalise.awk` mirroring the tokenise/inlines split.
- 🟡 **Code Quality**: Three-tier file layout (committed JSON cache / gitignored `.refresh-meta.json` / process-lifetime `.lock/holder.pid`) is implicit. Add a one-paragraph file-layout taxonomy in the Implementation Approach naming the three tiers and their lifecycle/visibility rules.
- 🟡 **Code Quality**: Stale-lock recovery has TOCTOU races (read-pid vs holder cleanup; kill-0 vs rm-rf) that the spec does not address. Either write `holder.pid` atomically with start-time, or document the known-race envelope.
- 🟡 **Portability**: `stat` argv differs across BSD (`-f '%Lp'`) and GNU (`-c '%a'`) — the perms check could fail in a non-portable way on macOS, the user's primary dev platform. Specify a portable idiom (try GNU then BSD form, or use `find "$f" -perm -077` which is POSIX).
- 🟡 **Portability**: `kill -0` and `/proc/<pid>/cmdline` assumptions in PID-namespaced containers (Alpine, gVisor, sandboxes) — stale recovery could produce false negatives, breaking the lock under contention. Document the limitation in the lock helper header; consider an mtime-based fallback for the holder file.
- 🟡 **Portability**: `mkdir` atomicity not guaranteed on NFS/SMB-mounted state directories — a user who relocates `paths.integrations` onto a network mount loses the serialisation guarantee. Document the limitation; optionally detect filesystem type at lock acquisition.

#### Minor

- 🔵 **Architecture**: Auth library still reads config files directly; consider a `_jira_resolve_from_source` indirection at the next auth-source addition.
- 🔵 **Code Quality**: Test-seam gating duplicated across helpers; lift to `_jira_gate_test_hook <var-name> <name-regex>` shared primitive.
- 🔵 **Code Quality**: HTTP-date parser embedded as inline `python3 -c '...'` heredoc; lift to `scripts/test-helpers/parse-http-date.py` for testability.
- 🔵 **Code Quality**: POSIX awk constraint asserted by header comment only; add a lint (`grep -nE 'gensub|asorti|systime'`) or run scripts under `awk -W posix`.
- 🔵 **Code Quality**: `jira-init-flow.sh` exposes seven subcommands with refresh-fields aliasing `jira-fields.sh`; reconsider whether list/refresh subcommands belong in the orchestrator at all.
- 🔵 **Code Quality**: `E_LOCAL_PERMS_INSECURE` placed in jira-auth range; flag for promotion to a shared scripts-level convention when the second integration arrives.
- 🔵 **Correctness**: `trap … EXIT` does not run on SIGKILL — lock leaks under kill -9. Add a SIGKILL test case and rely on stale-recovery for it.
- 🔵 **Correctness**: Float clamp arithmetic — specify clamp completes inside the python sub-shell so bash never sees `-0.4`.
- 🔵 **Correctness**: tz-naive HTTP-date treated as UTC may misinterpret asctime-form local-time. Treat tz-naive as parser failure, fall through to jittered backoff.
- 🔵 **Correctness**: Some shell printf builtins strip `\x1f`/`\x1e`. Document fixture authoring rule (`printf '%s'` only; never `echo -e` or `printf '%b'`); add fixture-integrity check `grep -c $'\x1f'`.
- 🔵 **Test Coverage**: Phase 7 case 2 byte-idempotency assertion implicit about which files are compared; reword to mirror Phase 6 case 12's precision (compare `site.json`/`fields.json`/`projects.json` only; exclude `.refresh-meta.json`).
- 🔵 **Test Coverage**: Case 19 (path validation) lacks per-input rule identification in the asserted error message; mirror `E_JQL_UNSAFE_VALUE` precedent and assert each rejection names the rule it tripped.
- 🔵 **Test Coverage**: Case 25 (allow-list) covers one rejected name; expand to a small table including uppercase, metacharacter, and undefined-but-matching cases.
- 🔵 **Test Coverage**: Cross-device and symlink atomic-write cases environment-fragile; add explicit skip-with-message guards or move to opt-in suite.
- 🔵 **Usability**: `E_TEST_HOOK_REJECTED` conflates "test-mode unset" and "name disallowed" under one message; branch the message or split into two codes.
- 🔵 **Usability**: `E_REQ_BAD_PATH` covers six rejection rules but emits no rule identifier; mirror `E_JQL_UNSAFE_VALUE` and name the rule that fired.
- 🔵 **Usability**: Lock-timeout error names PID but not the holder's invocation; add `holder.cmd` alongside `holder.pid` so the timeout message reads "lock held by jira-init-flow.sh (pid 12345)".
- 🔵 **Usability** (suggestion): `E_BAD_SITE` could include the validation rule (subdomain vs full hostname) to help first-time users who type `my-company.atlassian.net`.
- 🔵 **Standards**: Configure SKILL.md "Skill registration order" subsection placement unspecified; mirror the §3b precedent (target heading level, location, prose structure).
- 🔵 **Standards**: New `scripts/log-common.sh` lacks an explicit scope-boundary header comment; add one to prevent future kitchen-sink growth.
- 🔵 **Security**: `token_cmd` inherits full env including pre-resolved tokens; run with `env -i PATH=$PATH HOME=$HOME` to scrub.
- 🔵 **Security**: Cassette recorder PII redaction described as "etc."; specify an explicit allow-list and add a pre-commit scanner.
- 🔵 **Security**: `JIRA_ADF_LOCALID_SEED` produces predictable localIds; add a one-line rule that fixtures with seeded UUIDs must not be POSTed to live tenants; have compiler refuse seeded localIds when stdout is a TTY.
- 🔵 **Security**: Retry-After is server-driven so a hostile origin can extend wall-clock budget; cap total retry-time at 180 s end-to-end.
- 🔵 **Portability**: `EPOCHREALTIME` requires bash 4.4+; macOS is 3.2. Make the python3 fallback the documented default in the retry-policy block.
- 🔵 **Portability**: POSIX awk RS=`\x1f` works on gawk/mawk/BWK but BusyBox awk historically restricted; add a smoke test on whichever awk the runner picks up.
- 🔵 **Portability**: `/dev/urandom` not guaranteed in restricted sandboxes; clarify that python3 is effectively required (not just preferred fallback).
- 🔵 **Portability**: `/proc/<pid>/cmdline` doesn't exist on macOS; specify the conditional gate explicitly in case 13 (`if [ -r /proc/<pid>/cmdline ]; then …`).

### Assessment

The plan is now in qualitatively better shape than at any previous review pass. All three original critical security findings are gone, all but two of the named regressions from Pass 2 are cleanly resolved, and the new findings are smaller in scope: roughly half are prose drifts from incomplete propagation through the second pass (residual flock prose, manual-verification step, missing exit-code number), and the other half are design concerns the second pass did not anticipate but that have well-bounded fixes (PID-race stamping, URL-decode iteration, canonicalisation algorithm tightening, stat TOCTOU).

Mathematically the verdict remains REVISE because the major-finding count (≈18) still exceeds the threshold, but the trajectory is clearly converging. A fourth pass focused on:

1. The four prose drifts (Phase 2 §2b flock comment, Phase 2 §5 manual step, `E_TEST_HOOK_REJECTED` exit code, `log_die` rationale)
2. The three real regression remainders (URL double-decode, canonicalisation algorithm, perms TOCTOU + holder.pid PID-race)
3. The one new design tightening (`ACCELERATOR_ALLOW_INSECURE_LOCAL` requires committed marker)
4. Test coverage cases for: dead-holder lock, malformed HTTP-date warning, named cases for new ADF fixtures

would land the plan at APPROVE. The remaining ≈30 minor concerns are appropriate to defer to implementation-time review.

---

## Re-Review (Pass 4) — 2026-04-30

**Verdict:** APPROVE (with minor cleanup deferred to implementation)

The Pass 4 edits and the subsequent Pass 5 cleanup land the plan at a state where every Pass-3-flagged regression and every Pass-4-flagged real bug has been closed. Verdict trajectory: Pass 1 (3 critical / 36 major) → Pass 2 (0 critical / 22 major) → Pass 3 (0 critical / 18 major) → Pass 4 (0 critical / 5 major) → final cleanup (0 critical / 0 major regressions; ≈25 minors).

### Pass 4 Verification

Eight lenses ran against the post-Pass-4 plan. Confirmed resolution status for the 11 Pass-3 regressions:

- ✅ Stale flock prose in jira-common.sh header — resolved.
- ✅ Phase 2 §5 manual verification fail-closed scenarios — resolved (and the Pass 5 cleanup expanded the opt-out step into three sub-cases matching the dual-gate contract).
- ✅ E_TEST_HOOK_REJECTED pinned to exit code 23 — resolved.
- ✅ log_die/log_warn rationale (deliberate departure framing) — resolved.
- ✅ URL-decode iteration to fixed point with 8-cap — resolved.
- ✅ Canonicalisation: asterisk-only supported subset — resolved (eliminates the entire delimiter-stack ambiguity class).
- ✅ Stat-then-read TOCTOU + PID-recycling race — resolved (lstat + O_NOFOLLOW + fstat via single python3 subprocess; PID + start-time stamp; mv-then-rm reclaim).
- ✅ ACCELERATOR_ALLOW_INSECURE_LOCAL dual-gate — resolved (Pass 5 cleanup specified marker check: VCS-tracked + lstat-rejects-symlinks + regular-file).
- ✅ Portability: NFS/kill-0/stat argv — resolved (Pass 5 cleanup replaced non-portable `stat -f -c %T` / `df -T` with python3 helper).
- ✅ Test cases: 5 lock cases, 5 HTTP-date cases (9, 9a-9d), 13 named ADF cases, 6 marker-rejection sub-cases — all explicitly enumerated.
- ✅ python3 as load-bearing auth dep — Pass 5 documented prominently in configure SKILL.md prerequisites.

### Pass 4 New Findings — Resolution

**Pass 4 surfaced four real majors. All resolved by Pass 5:**

- 🟡 **Security**: marker file presence check unspecified (symlink/untracked bypass) — **Resolved**: now requires `lstat` regular-file check + VCS-tracked verification via `jj file list` / `git ls-files --error-unmatch`.
- 🟡 **Usability**: Phase 2 §5 manual verification opt-out step contradicted the new dual-gate — **Resolved**: rewritten as three sub-cases (env-only fails, env+marker succeeds, etc.) matching the automated test layout.
- 🟡 **Portability**: `stat -f -c %T` / `df -T` non-portable filesystem detection — **Resolved**: replaced with `_jira_pyhelper fstype` (python3-based, works on macOS and Linux uniformly).
- 🟡 **Portability**: python3 hard dependency on auth path not documented — **Resolved**: configure SKILL.md `### jira` section now opens with a Prerequisites paragraph naming python3 ≥3.9 as load-bearing for credential resolution.

### Remaining Concerns

All remaining concerns are minor and appropriate for implementation-time review (i.e. tighten during code review of the actual scripts, not during plan iteration):

- 🔵 Test-case count mentions in Phase 2 / Phase 5 success criteria are slightly stale (e.g. "thirteen assertion groups" vs the now-expanded sub-cases); refresh during implementation or replace with "all documented cases".
- 🔵 Phase 5 case 9 (HTTP-date 2s in future) asserts `≤60s` rather than the exact recorded sleep value; sibling cases 9a/9c assert exact values — tighten case 9 for symmetry.
- 🔵 Dead-holder lock case relies on PID 999999 being absent; allocate a PID dynamically (fork + reap a `true` subshell, capture `$!`) for determinism.
- 🔵 Underscore Notice severity (`Notice:` vs project's existing `Warning:` prefix) — choose one for vocabulary consistency.
- 🔵 init-jira frontmatter `Bash(jq), Bash(curl)` bare entries deviate from the `${CLAUDE_PLUGIN_ROOT}`-rooted convention — consider dropping (the scripts/* glob already covers transitive jq/curl use).
- 🔵 Inline `python3 -c '...'` invocations across multiple helpers — consider consolidating into a `jira-pyhelpers.py` dispatcher.
- 🔵 Asterisk-only edge cases (`*foo*bar*`, `**foo*bar**`) need a documented inline-tokeniser rule + fixture to pin behaviour.
- 🔵 `ps -o lstart=` locale handling — run under `LC_ALL=C` for byte-deterministic output.
- 🔵 Soft-warning grep for `__bold__` should run after block tokenisation so dunders inside fenced code blocks don't trigger false positives.
- 🔵 Plus ~15 other minors covering documentation, fixture cohesion, and test-tightening — all defer-to-implementation candidates.

### Assessment

The plan has converged to a state where every load-bearing concern is addressed and the remaining work is polish:

- **Security**: Three Pass-1 critical token-exfiltration vectors closed; defence in depth across token redaction, path validation, perms enforcement, dual-gate overrides, TOCTOU-safe credential reads, and test-seam isolation.
- **Correctness**: ADF round-trip strengthened with three layered invariants (Markdown round-trip, ADF fixed-point, structural marker counting); locking has portable PID + start-time stale recovery with documented fallback; URL-decode is iterative; HTTP-date arithmetic is bounded and validated; emphasis subset is asterisk-only and unambiguous.
- **Test coverage**: TDD discipline maintained; mock fidelity grounded in real-tenant cassettes; concurrent-write, kill-9, PID-recycle, malformed-input, and unicode/CRLF/large-payload edge cases all explicitly covered.
- **Portability**: All command-line argv divergences (BSD vs GNU stat/df) routed through python3 helpers; awk pinned POSIX; bash 3.2 compatible; documented degradation modes for sandboxed/PID-namespaced environments.
- **Standards**: Conventions either followed or explicitly framed as deliberate departures with rationale; exit-code namespace consolidated; SKILL frontmatter aligned.
- **Code quality**: Lib/CLI splits, three-layer file taxonomy (committed cache / .refresh-meta.json / .lock dir), single-source-of-truth EXIT_CODES.md, extracted log-common.sh.

**Verdict: APPROVE** for implementation. Spawn the implementation work in TDD order per the milestone diagram (M1 skeleton → M2 common+auth → M3 jql → M4 ADF → M5 request → M6 fields → M7 init-jira). Implementation-time review can address the ~25 remaining minors as they surface in actual code; none block the start of work.
