# Jira Integration — Exit Code Namespace

Every helper in `skills/integrations/jira/scripts/` draws from this
table. Gaps within ranges are reserved.

## Codes

| Code | Name                             | Owner               | Description                                                                              |
|------|----------------------------------|---------------------|------------------------------------------------------------------------------------------|
| 0    | —                                | all                 | Success                                                                                  |
| 1    | —                                | all                 | Generic/unclassified error                                                               |
| 2    | —                                | all                 | Argument/usage error (`set -e` default)                                                  |
| 11   | —                                | `jira-request.sh`   | HTTP 401 Unauthorized                                                                    |
| 12   | —                                | `jira-request.sh`   | HTTP 403 Forbidden                                                                       |
| 13   | —                                | `jira-request.sh`   | HTTP 404 Not Found                                                                       |
| 14   | —                                | `jira-request.sh`   | HTTP 410 Gone                                                                            |
| 15   | `E_BAD_SITE`                     | `jira-request.sh`   | `jira.site` failed validation                                                            |
| 16   | `E_REQ_BAD_RESPONSE`             | `jira-request.sh`   | Non-JSON body on 200                                                                     |
| 17   | `E_REQ_BAD_PATH`                 | `jira-request.sh`   | Path argument failed validation                                                          |
| 18   | `E_TEST_OVERRIDE_REJECTED`       | `jira-request.sh`   | Test URL override refused (gate: `ACCELERATOR_TEST_MODE=1`)                              |
| 19   | —                                | `jira-request.sh`   | HTTP 429 — retries exhausted                                                             |
| 20   | —                                | `jira-request.sh`   | HTTP 5xx server error                                                                    |
| 21   | `E_REQ_CONNECT`                  | `jira-request.sh`   | Connection / DNS / timeout failure                                                       |
| 22   | `E_REQ_NO_CREDS`                 | `jira-request.sh`   | No resolvable credentials                                                                |
| 23   | `E_TEST_HOOK_REJECTED`           | `jira-request.sh`   | `JIRA_RETRY_SLEEP_FN` hook refused (gate: `ACCELERATOR_TEST_MODE=1`)                     |
| 24   | `E_NO_TOKEN`                     | `jira-auth.sh`      | No token found in any source                                                             |
| 25   | `E_TOKEN_CMD_FAILED`             | `jira-auth.sh`      | `token_cmd` exited non-zero                                                              |
| 26   | `E_TOKEN_CMD_FROM_SHARED_CONFIG` | `jira-auth.sh`      | `jira.token_cmd` in `accelerator.md` ignored (stderr prefix only; not a fatal exit code) |
| 27   | `E_AUTH_NO_SITE`                 | `jira-auth.sh`      | `jira.site` not configured                                                               |
| 28   | `E_AUTH_NO_EMAIL`                | `jira-auth.sh`      | `jira.email` not configured                                                              |
| 29   | `E_LOCAL_PERMS_INSECURE`         | `jira-auth.sh`      | `accelerator.local.md` mode > 0600                                                       |
| 30   | `E_JQL_NO_PROJECT`               | `jira-jql.sh`       | `compose` called without `--project` or `--all-projects`                                 |
| 31   | `E_JQL_UNSAFE_VALUE`             | `jira-jql.sh`       | Value contains a control character                                                       |
| 32   | `E_JQL_BAD_FLAG`                 | `jira-jql.sh`       | Unrecognised flag                                                                        |
| 33   | `E_JQL_EMPTY_VALUE`              | `jira-jql.sh`       | Empty string where a value was expected                                                  |
| 40   | `E_BAD_JSON`                     | `jira-adf-to-md.sh` | Stdin is not valid JSON, or not an ADF document                                          |
| 41   | `E_ADF_UNSUPPORTED_*`            | `jira-md-to-adf.sh` | Markdown input contains an unsupported ADF construct                                     |
| 42   | `E_ADF_BAD_INPUT`                | `jira-md-to-adf.sh` | Malformed or unacceptable Markdown input                                                 |
| 50   | `E_FIELD_NOT_FOUND`              | `jira-fields.sh`    | No field matches the given query                                                         |
| 51   | `E_FIELD_CACHE_MISSING`          | `jira-fields.sh`    | `fields.json` absent; run `refresh` or `/init-jira`                                      |
| 52   | `E_FIELD_CACHE_CORRUPT`          | `jira-fields.sh`    | `fields.json` present but not valid JSON                                                 |
| 53   | `E_REFRESH_LOCKED`               | `jira-common.sh`    | `jira_with_lock` timed out waiting for the integration lock                              |
| 60   | `E_INIT_NEEDS_CONFIG`            | `jira-init-flow.sh`        | Required config missing in non-interactive mode                                          |
| 61   | `E_INIT_VERIFY_FAILED`           | `jira-init-flow.sh`        | `/rest/api/3/myself` verification failed                                                 |
| 90   | `E_RENDER_BAD_INPUT`             | `jira-render-adf-fields.sh` | Stdin is not valid JSON                                                                  |

## Test-seam policy

Test seams let the test suite override production behaviour (e.g. point
at a mock server, stub retry delays). **Every seam is gated: it is
honoured only when `ACCELERATOR_TEST_MODE=1` is also set.** Without the
gate, the seam env var is silently ignored and the helper uses its
production default. A rejected gate produces one of the
`E_TEST_*_REJECTED` codes (18, 23) and the helper continues with
production behaviour.

| Env var                                   | Gate required             | Owner             | Purpose                                                        |
|-------------------------------------------|---------------------------|-------------------|----------------------------------------------------------------|
| `ACCELERATOR_TEST_MODE`                   | —                         | all               | Master gate; must be `1` to activate any seam below            |
| `ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST` | `ACCELERATOR_TEST_MODE=1` | `jira-request.sh` | Redirect API calls to a mock server URL                        |
| `JIRA_RETRY_SLEEP_FN`                     | `ACCELERATOR_TEST_MODE=1` | `jira-request.sh` | Shell function name to call instead of `sleep` between retries |
| `JIRA_ADF_LOCALID_SEED`                   | `ACCELERATOR_TEST_MODE=1` | `jira-common.sh`  | Integer seed for deterministic `_jira_uuid_v4` output          |
| `JIRA_LOCK_TIMEOUT_SECS`                  | `ACCELERATOR_TEST_MODE=1` | `jira-common.sh`  | Override 60 s lock-acquisition timeout                         |
| `JIRA_LOCK_SLEEP_SECS`                    | `ACCELERATOR_TEST_MODE=1` | `jira-common.sh`              | Override 0.1 s sleep between lock-acquisition retries          |
| `ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST` | `ACCELERATOR_TEST_MODE=1` | `jira-render-adf-fields.sh`   | Override the `fields.json` path used to look up custom textarea field IDs |
| `ACCELERATOR_JIRA_ADF_RENDERER_TEST`      | `ACCELERATOR_TEST_MODE=1` | `jira-render-adf-fields.sh`   | Override the `jira-adf-to-md.sh` renderer path (used by idempotency stub tests) |
