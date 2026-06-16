# Linear Integration — Exit Code Namespace

Every helper in `skills/integrations/linear/scripts/` draws from this table.
Gaps within ranges are reserved.

This is a **per-integration** namespace — there is **no** top-level shared
`scripts/EXIT_CODES.md`; the Jira integration owns its own table the same way.
The transport band (11–23) keeps **positional parity** with Jira's transport
codes so the two integrations read alike across files, with two deliberate
divergences:

- Jira's per-status codes for 403/404/410 are **not** mirrored. GraphQL returns
  these as HTTP-200/400 `errors[]` bodies, so they collapse into
  `E_GQL_BAD_REQUEST` (34) / `E_GQL_UNAUTHORIZED` (11). The vacated transport
  slots **12–15, 17, 19** correspond to those Jira-only HTTP-status codes and
  are reserved here.
- Code `27` is `E_TOKEN_MALFORMED` (Jira's `27`/`28` are site/email, which
  Linear lacks — it resolves a token only).

The flow bands (init/search/show/comment/create/update/transition/attach) reuse
Jira's range boundaries for cross-integration reading, but assign code
*meanings* independently per the differing skill semantics — so readers must
**not** assume per-number parity outside the transport band.

Each flow declares its codes as `readonly E_*=NN` constants near the top of its
script. **The constants are the source of truth**; this document is derived. A
lightweight check in the `test-linear-*.sh` suites greps each flow's
`readonly E_*=NN` declarations and asserts each appears with the same value
here (mirroring the gitignore-rules equality assertion). This `readonly`
idiom is an intentional upgrade over Jira's bare-literal `return 100`.

## Codes

| Code | Name                             | Owner                       | Description                                                                                  |
|------|----------------------------------|-----------------------------|----------------------------------------------------------------------------------------------|
| 0    | —                                | all                         | Success                                                                                       |
| 1    | —                                | all                         | Generic/unclassified error                                                                    |
| 2    | —                                | all                         | Argument/usage error (`set -e` default)                                                       |
| 11   | `E_GQL_UNAUTHORIZED`             | `linear-graphql.sh`         | HTTP 401, or a GraphQL `extensions.type == "authentication error"` / `.code == "AUTHENTICATION_ERROR"` |
| 12   | —                                | reserved                    | Reserved (Jira-only: HTTP 403)                                                                 |
| 13   | —                                | reserved                    | Reserved (Jira-only: HTTP 404)                                                                 |
| 14   | —                                | reserved                    | Reserved (Jira-only: HTTP 410)                                                                 |
| 15   | —                                | reserved                    | Reserved (Jira-only: `E_BAD_SITE`)                                                             |
| 16   | `E_GQL_BAD_RESPONSE`             | `linear-graphql.sh`         | Non-JSON body on HTTP 200                                                                      |
| 17   | —                                | reserved                    | Reserved (Jira-only: `E_REQ_BAD_PATH`)                                                         |
| 18   | `E_TEST_OVERRIDE_REJECTED`       | `linear-graphql.sh`         | Base-URL override refused (gate: `ACCELERATOR_TEST_MODE=1`)                                    |
| 19   | —                                | reserved                    | Reserved (Jira-only: HTTP 429)                                                                 |
| 20   | `E_GQL_SERVER_ERROR`             | `linear-graphql.sh`         | HTTP 5xx — retries exhausted                                                                   |
| 21   | `E_GQL_CONNECT`                  | `linear-graphql.sh`         | Connection / DNS / timeout failure                                                            |
| 22   | `E_GQL_NO_CREDS`                 | `linear-graphql.sh`         | No resolvable token                                                                           |
| 23   | `E_TEST_HOOK_REJECTED`           | `linear-graphql.sh`         | `LINEAR_RETRY_SLEEP_FN` hook refused (gate: `ACCELERATOR_TEST_MODE=1`)                         |
| 24   | `E_NO_TOKEN`                     | `linear-auth.sh`            | No token found in any source                                                                  |
| 25   | `E_TOKEN_CMD_FAILED`             | `linear-auth.sh`            | `token_cmd` exited non-zero                                                                    |
| 26   | `E_TOKEN_CMD_FROM_SHARED_CONFIG` | `linear-auth.sh`            | `linear.token_cmd` in `config.md` ignored (stderr warning only; not a fatal exit code)        |
| 27   | `E_TOKEN_MALFORMED`              | `linear-auth.sh`            | Token contains control chars / quotes / backslash / newline (would corrupt `curl --config -`) |
| 29   | `E_LOCAL_PERMS_INSECURE`         | `linear-auth.sh`            | `config.local.md` mode > 0600                                                                  |
| 34   | `E_GQL_BAD_REQUEST`              | `linear-graphql.sh`         | HTTP 400 error that is neither auth, rate limit, nor complexity (validation / bad query)      |
| 35   | `E_GQL_RATELIMITED`              | `linear-graphql.sh`         | HTTP 400 + `extensions.code == "RATELIMITED"` — retries exhausted                             |
| 36   | `E_GQL_COMPLEXITY`               | `linear-graphql.sh`         | Single-query complexity cap (10,000 points) exceeded                                          |
| 53   | `E_REFRESH_LOCKED`               | `linear-common.sh`          | `linear_with_lock` timed out waiting for the integration lock                                 |
| 60   | `E_INIT_NEEDS_CONFIG`            | `linear-init-flow.sh`       | Required config missing in non-interactive mode                                               |
| 61   | `E_INIT_VERIFY_FAILED`           | `linear-init-flow.sh`       | `viewer` verification failed (incl. a `Bearer`-prefixed token failing auth)                   |
| 62   | `E_INIT_NO_TEAM`                 | `linear-init-flow.sh`       | Selected team not found or has no WorkflowStates                                              |
| 70   | `E_SEARCH_BAD_FLAG`              | `linear-search-flow.sh`     | Unrecognised flag passed to the search flow                                                   |
| 71   | `E_SEARCH_BAD_LIMIT`             | `linear-search-flow.sh`     | `--limit` is not a positive integer in range                                                  |
| 72   | `E_SEARCH_NO_CATALOGUE`          | `linear-search-flow.sh`     | `catalogue.json` missing; run `/init-linear`                                                  |
| 73   | `E_SEARCH_BAD_STATE`             | `linear-search-flow.sh`     | `--state` value not found in the catalogue                                                    |
| 80   | `E_SHOW_NO_KEY`                  | `linear-show-flow.sh`       | No issue identifier supplied as positional argument                                           |
| 81   | `E_SHOW_BAD_FLAG`               | `linear-show-flow.sh`       | Unrecognised flag passed to the show flow                                                     |
| 82   | `E_SHOW_NOT_FOUND`              | `linear-show-flow.sh`       | No issue matches the given identifier                                                         |
| 90   | `E_COMMENT_NO_KEY`               | `linear-comment-flow.sh`    | No issue identifier positional argument                                                       |
| 91   | `E_COMMENT_NO_BODY`              | `linear-comment-flow.sh`    | No resolvable comment body                                                                    |
| 92   | `E_COMMENT_BAD_FLAG`             | `linear-comment-flow.sh`    | Unrecognised flag                                                                             |
| 100  | `E_CREATE_NO_FILE`               | `linear-create-flow.sh`     | No work-item file path supplied, or path not readable (also a no-file `--body-file` not readable) |
| 101  | `E_CREATE_BAD_FRONTMATTER`       | `linear-create-flow.sh`     | Work-item file has missing/unclosed frontmatter                                               |
| 102  | `E_CREATE_ALREADY_SYNCED`        | `linear-create-flow.sh`     | `external_id` is already present (non-empty after trimming quotes/whitespace) — nothing created |
| 103  | `E_CREATE_NO_TITLE`              | `linear-create-flow.sh`     | No `title` (frontmatter `title` in file mode, or `--title` in no-file mode)                   |
| 104  | `E_CREATE_BAD_FLAG`              | `linear-create-flow.sh`     | Unrecognised flag                                                                             |
| 105  | `E_CREATE_NO_CATALOGUE`          | `linear-create-flow.sh`     | `catalogue.json` missing; run `/init-linear`                                                  |
| 106  | `E_CREATE_BAD_IDENTIFIER`        | `linear-create-flow.sh`     | Returned identifier failed `^[A-Z][A-Z0-9]*-[0-9]+$` validation (tampered response)           |
| 107  | `E_CREATE_WRITEBACK_FAILED`      | `linear-create-flow.sh`     | Issue created remotely, but the local `external_id` writeback failed (surfaced loudly)        |
| 108  | `E_CREATE_PRE_SEND`              | `linear-create-flow.sh`     | No-file mode: failure provably before the `issueCreate` mutation was transmitted — safe to retry |
| 109  | `E_CREATE_POST_SEND`             | `linear-create-flow.sh`     | No-file mode: request was/may have been transmitted (issue may exist) — NOT safe to retry      |
| 110  | `E_UPDATE_NO_KEY`                | `linear-update-flow.sh`     | No issue identifier positional argument                                                       |
| 111  | `E_UPDATE_NO_OPS`                | `linear-update-flow.sh`     | No mutating flags supplied                                                                     |
| 112  | `E_UPDATE_BAD_FLAG`              | `linear-update-flow.sh`     | Unrecognised flag                                                                             |
| 113  | `E_UPDATE_NO_CATALOGUE`          | `linear-update-flow.sh`     | `--state` used but `catalogue.json` missing; run `/init-linear`                               |
| 114  | `E_UPDATE_BAD_STATE`             | `linear-update-flow.sh`     | `--state` value not found in the catalogue                                                     |
| 120  | `E_TRANSITION_NO_KEY`            | `linear-transition-flow.sh` | No issue identifier positional argument                                                       |
| 121  | `E_TRANSITION_NO_STATE`          | `linear-transition-flow.sh` | No target state name supplied                                                                  |
| 122  | `E_TRANSITION_STATE_NOT_IN_CATALOGUE` | `linear-transition-flow.sh` | Target state name not present in `catalogue.json`                                        |
| 123  | `E_TRANSITION_STATE_AMBIGUOUS`   | `linear-transition-flow.sh` | Two catalogue states share the target display name                                            |
| 124  | `E_TRANSITION_NO_CATALOGUE`      | `linear-transition-flow.sh` | `catalogue.json` missing; run `/init-linear`                                                  |
| 125  | `E_TRANSITION_BAD_FLAG`          | `linear-transition-flow.sh` | Unrecognised flag                                                                             |
| 130  | `E_ATTACH_NO_KEY`                | `linear-attach-flow.sh`     | No issue identifier positional argument                                                       |
| 131  | `E_ATTACH_NO_TARGET`             | `linear-attach-flow.sh`     | Neither `--url` nor `--file` supplied                                                          |
| 132  | `E_ATTACH_BOTH_TARGETS`          | `linear-attach-flow.sh`     | Both `--url` and `--file` supplied (mutually exclusive)                                        |
| 133  | `E_ATTACH_FILE_MISSING`          | `linear-attach-flow.sh`     | `--file` path does not exist or is not readable                                               |
| 134  | `E_ATTACH_BAD_URL`               | `linear-attach-flow.sh`     | `--url` value failed validation                                                               |
| 135  | `E_ATTACH_BAD_UPLOAD_URL`        | `linear-attach-flow.sh`     | Server-supplied `uploadUrl`/`assetUrl` host or scheme not allow-listed                         |
| 136  | `E_ATTACH_UPLOAD_FAILED`         | `linear-attach-flow.sh`     | Binary PUT to the pre-signed URL failed (bounded retry exhausted)                             |
| 137  | `E_ATTACH_REGISTER_FAILED`       | `linear-attach-flow.sh`     | `attachmentCreate` (step 3) failed after a successful PUT — names the orphaned asset           |
| 138  | `E_ATTACH_BAD_FLAG`              | `linear-attach-flow.sh`     | Unrecognised flag                                                                             |

## Range summary

| Range     | Owner                       | Notes                                                  |
|-----------|-----------------------------|--------------------------------------------------------|
| 11–23     | `linear-graphql.sh`         | Transport; positional parity with Jira (12–15/17/19 reserved) |
| 24–29     | `linear-auth.sh`            | 26 = stderr warning only; 27 = `E_TOKEN_MALFORMED`     |
| 34–36     | `linear-graphql.sh`         | 34 bad-request, 35 ratelimited, 36 complexity          |
| 53        | `linear-common.sh`          | `E_REFRESH_LOCKED`                                     |
| 60–62     | `linear-init-flow.sh`       |                                                        |
| 70–73     | `linear-search-flow.sh`     |                                                        |
| 80–82     | `linear-show-flow.sh`       |                                                        |
| 90–96     | `linear-comment-flow.sh`    |                                                        |
| 100–109   | `linear-create-flow.sh`     | `external_id` writeback + no-file pre/post-send codes   |
| 110–117   | `linear-update-flow.sh`     |                                                        |
| 120–126   | `linear-transition-flow.sh` | Cache-resolved transitions (no live lookup)            |
| 130–139   | `linear-attach-flow.sh`     | Dual link/binary attach                                |

## Test-seam policy

Test seams let the test suite override production behaviour (point at a mock
server, stub retry delays). **Every seam is gated: honoured only when
`ACCELERATOR_TEST_MODE=1` is also set.** Without the gate the seam env var is
silently ignored and the helper uses its production default. A rejected gate
produces one of the `E_TEST_*_REJECTED` codes (18, 23) and the helper continues
with production behaviour.

| Env var                                     | Gate required             | Owner               | Purpose                                                        |
|---------------------------------------------|---------------------------|---------------------|----------------------------------------------------------------|
| `ACCELERATOR_TEST_MODE`                     | —                         | all                 | Master gate; must be `1` to activate any seam below            |
| `ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST` | `ACCELERATOR_TEST_MODE=1` | `linear-graphql.sh` | Redirect API calls to a loopback mock-server URL               |
| `LINEAR_RETRY_SLEEP_FN`                     | `ACCELERATOR_TEST_MODE=1` | `linear-graphql.sh` | Shell function name to call instead of `sleep` between retries |
| `LINEAR_LOCK_TIMEOUT_SECS`                  | `ACCELERATOR_TEST_MODE=1` | `linear-common.sh`  | Override 60 s lock-acquisition timeout                         |
| `LINEAR_LOCK_SLEEP_SECS`                    | `ACCELERATOR_TEST_MODE=1` | `linear-common.sh`  | Override 0.1 s sleep between lock-acquisition retries          |
