#!/usr/bin/env bash
set -euo pipefail
# linear-graphql.sh — authenticated Linear GraphQL request helper
# Usage:
#   linear-graphql.sh --query @file|<inline> [--variables @file|<inline>]
#                     [--paginate <jq-connection-path>] [--debug]
#
# Always POSTs {query, variables} to https://api.linear.app/graphql with
# Content-Type: application/json and an Authorization header carrying the
# resolved personal API key VERBATIM (no Bearer prefix). The token is supplied
# via `curl --config -` (piped on stdin) so it never appears in argv.
#
# Body → stdout, errors → stderr, outcome via exit code.
#
# Exit codes (see EXIT_CODES.md):
#   0  success
#   11 E_GQL_UNAUTHORIZED      — HTTP 401, or a GraphQL authentication error
#   16 E_GQL_BAD_RESPONSE      — non-JSON body on HTTP 200
#   18 E_TEST_OVERRIDE_REJECTED — base-URL override refused
#   20 E_GQL_SERVER_ERROR      — HTTP 5xx, retries exhausted
#   21 E_GQL_CONNECT           — connection / DNS / timeout
#   22 E_GQL_NO_CREDS          — no resolvable token
#   23 E_TEST_HOOK_REJECTED    — LINEAR_RETRY_SLEEP_FN refused
#   25 E_TOKEN_CMD_FAILED      — token_cmd exited non-zero (propagated from auth)
#   27 E_TOKEN_MALFORMED       — token would corrupt the curl --config - directive
#   29 E_LOCAL_PERMS_INSECURE  — config.local.md mode > 0600 (propagated from auth)
#   34 E_GQL_BAD_REQUEST       — HTTP 400 error that is neither auth, rate limit,
#                                nor complexity (validation / bad query)
#   35 E_GQL_RATELIMITED       — HTTP 400 RATELIMITED, retries exhausted
#   36 E_GQL_COMPLEXITY         — single-query complexity cap (10,000 points)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/linear-common.sh"
source "$SCRIPT_DIR/linear-auth.sh"

LINEAR_BASE="https://api.linear.app"
MAX_PAGES=20

# The complexity-rejection discriminator. Linear publishes no machine-readable
# code distinct from RATELIMITED for the 10,000-point single-query cap, so the
# only reliable signal is the message substring. Pinned in ONE place so the
# known-fragile heuristic has a single, test-anchored home. Requires the full
# word "complexity" (not the bare "complex" stem) to avoid matching an
# unrelated "complex query" phrasing; the 10,000 figure counts only as
# corroboration alongside it, never on its own.
LINEAR_COMPLEXITY_PATTERN="complexity"

# ---------------------------------------------------------------------------
# Test seam gating
# ---------------------------------------------------------------------------

_gql_test_mode() { [ "${ACCELERATOR_TEST_MODE:-}" = "1" ]; }

_gql_resolve_sleep_fn() {
  local fn="${LINEAR_RETRY_SLEEP_FN:-}"
  if [ -z "$fn" ]; then
    echo "sleep"
    return 0
  fi

  if ! _gql_test_mode; then
    echo "E_TEST_HOOK_REJECTED: LINEAR_RETRY_SLEEP_FN ignored — not in test mode" >&2
    echo "sleep"
    return 0
  fi

  if ! [[ "$fn" =~ ^_?test_[a-z_]+$ ]]; then
    echo "E_TEST_HOOK_REJECTED: LINEAR_RETRY_SLEEP_FN ignored — name '$fn' is not an allowed test hook" >&2
    echo "sleep"
    return 0
  fi

  if ! declare -F "$fn" >/dev/null 2>&1; then
    echo "E_TEST_HOOK_REJECTED: LINEAR_RETRY_SLEEP_FN ignored — '$fn' is not defined" >&2
    echo "sleep"
    return 0
  fi

  echo "$fn"
}

# ---------------------------------------------------------------------------
# GraphQL error classification
#
# Returns a symbolic class on stdout: auth | complexity | ratelimited |
# bad-request. THE ORDER IS LOAD-BEARING:
#   1. auth        — first, so a body-borne auth error never falls through to
#                    bad-request.
#   2. complexity  — before rate limit, because the complexity cap most likely
#                    carries the SAME extensions code/type as a throttle
#                    (RATELIMITED) and the only discriminator is the message
#                    substring; checking it first stops the two collapsing.
#   3. ratelimited — extensions.code/type == RATELIMITED.
#   4. bad-request — fail-safe: anything else (incl. an unrecognised error or a
#                    complexity-wording drift) takes the non-retried path.
# Case-folding is done with `tr` (bash 3.2 floor: no ${var,,}).
# ---------------------------------------------------------------------------

_linear_classify_gql_error() {
  local body="$1"
  local types_lc codes_lc messages_lc
  types_lc=$(printf '%s' "$body" | jq -r '[.errors[]?.extensions.type // empty] | join("\n")' 2>/dev/null |
    tr '[:upper:]' '[:lower:]' || true)
  codes_lc=$(printf '%s' "$body" | jq -r '[.errors[]?.extensions.code // empty] | join("\n")' 2>/dev/null |
    tr '[:upper:]' '[:lower:]' || true)
  messages_lc=$(printf '%s' "$body" | jq -r '[.errors[]?.message // empty] | join("\n")' 2>/dev/null |
    tr '[:upper:]' '[:lower:]' || true)

  # 1. Auth
  case "$types_lc" in *"authentication error"*)
    echo "auth"
    return 0
    ;;
  esac
  case "$codes_lc" in *"authentication_error"*)
    echo "auth"
    return 0
    ;;
  esac

  # 2. Complexity (full word, case-insensitive)
  case "$messages_lc" in *"$LINEAR_COMPLEXITY_PATTERN"*)
    echo "complexity"
    return 0
    ;;
  esac

  # 3. Rate limit
  case "$codes_lc" in *"ratelimited"*)
    echo "ratelimited"
    return 0
    ;;
  esac
  case "$types_lc" in *"ratelimited"*)
    echo "ratelimited"
    return 0
    ;;
  esac

  # 4. Else
  echo "bad-request"
}

# Emit only errors[].message / extensions.code|type to stderr — never the
# whole raw body (a verbose backend error or request-echoed input must not be
# blanket-surfaced). The token is header-only, so it cannot leak this way.
_linear_emit_gql_error_messages() {
  local body="$1"
  local msgs
  msgs=$(printf '%s' "$body" | jq -r '
    .errors[]? |
    "GraphQL error: " + (.message // "(no message)") +
    (if .extensions.code then " [code=" + (.extensions.code | tostring) + "]"
     elif .extensions.type then " [type=" + (.extensions.type | tostring) + "]"
     else "" end)
  ' 2>/dev/null) || true
  [ -n "$msgs" ] && printf '%s\n' "$msgs" >&2
  return 0
}

# ---------------------------------------------------------------------------
# Backoff arithmetic
# ---------------------------------------------------------------------------

# Exponential backoff with ±30% jitter, clamped to [1,60]s.
_linear_expo_backoff() {
  local attempt="$1"
  local base=$((1 << (attempt - 1)))
  [ "$base" -gt 60 ] && base=60
  local seed=$(((RANDOM ^ $(date +%s)) % 1000))
  local jitter=$((base * 30 / 100))
  local sign=$((seed % 2))
  local rand=$((seed % (jitter + 1)))
  local s
  if [ "$sign" -eq 0 ]; then
    s=$((base + rand))
  else
    s=$((base - rand))
  fi
  [ "$s" -lt 1 ] && s=1
  [ "$s" -gt 60 ] && s=60
  echo "$s"
}

# Rate-limit backoff: prefer X-RateLimit-Requests-Reset (epoch MILLIseconds);
# backoff seconds = reset_ms/1000 − now_s (second granularity — BSD/macOS date
# has no GNU %N), clamped to [1,60]s. If the header is absent/empty/non-numeric
# fall back to exponential backoff (also clamped) — NEVER the bare subtraction,
# which would otherwise clamp to a 1s tight-loop retry.
_linear_ratelimit_backoff() {
  local hdr_file="$1" attempt="$2"
  local reset_ms
  reset_ms=$(grep -i '^X-RateLimit-Requests-Reset:' "$hdr_file" | head -1 |
    sed 's/^[^:]*:[[:space:]]*//' | tr -d '\r\n') || reset_ms=""
  if [ -n "$reset_ms" ] && [[ "$reset_ms" =~ ^[0-9]+$ ]]; then
    local now reset_s delta
    now=$(date +%s)
    reset_s=$((reset_ms / 1000))
    delta=$((reset_s - now))
    local s=$((delta < 1 ? 1 : delta > 60 ? 60 : delta))
    echo "$s"
    return 0
  fi
  _linear_expo_backoff "$attempt"
}

# ---------------------------------------------------------------------------
# Single request (retry loop lives strictly inside one request)
#
# Globals consumed: LINEAR_TOKEN, LINEAR_ENDPOINT (resolved), CURL_FLAGS,
# SLEEP_FN, HDR_FILE, BODY_FILE. Prints the success body to stdout and
# returns 0, or emits an error to stderr and returns the mapped exit code.
# ---------------------------------------------------------------------------

_linear_do_request() {
  local query="$1" variables="$2"
  [ -z "$variables" ] && variables='{}'
  local req_body
  if ! req_body=$(jq -cn --arg q "$query" --argjson v "$variables" \
    '{query: $q, variables: $v}' 2>/dev/null); then
    echo "E_GQL_BAD_REQUEST: could not assemble GraphQL request body" >&2
    return 34
  fi

  local max_attempts=4
  local attempt=0
  while [ "$attempt" -lt "$max_attempts" ]; do
    attempt=$((attempt + 1))
    : >"$HDR_FILE"
    : >"$BODY_FILE"

    local curl_ok=true
    printf 'header = "Authorization: %s"\n' "$LINEAR_TOKEN" |
      curl --config - "${CURL_FLAGS[@]}" --data "$req_body" \
        -D "$HDR_FILE" -o "$BODY_FILE" "$LINEAR_ENDPOINT" 2>/dev/null ||
      curl_ok=false

    if ! $curl_ok || [ ! -s "$HDR_FILE" ]; then
      echo "E_GQL_CONNECT: curl failed to connect to $LINEAR_ENDPOINT" >&2
      return 21
    fi

    local status_code
    status_code=$(head -1 "$HDR_FILE" | awk '{print $2}' | tr -d '\r')
    if ! [[ "$status_code" =~ ^[0-9]+$ ]]; then
      echo "E_GQL_CONNECT: unexpected response (no HTTP status)" >&2
      return 21
    fi

    local body
    body=$(cat "$BODY_FILE")

    case "$status_code" in
      2*)
        if [ -n "$body" ] && ! printf '%s' "$body" | jq -e . >/dev/null 2>&1; then
          echo "E_GQL_BAD_RESPONSE: non-JSON body on HTTP $status_code" >&2
          return 16
        fi
        if printf '%s' "$body" | jq -e 'has("errors") and (.errors | length > 0)' \
          >/dev/null 2>&1; then
          # A 200-body error is TERMINAL — never routed into the retrying
          # RATELIMITED path (retrying a 200 risks re-issuing a non-idempotent
          # mutation). Classify auth → complexity → (everything else collapses
          # to bad-request).
          local cls
          cls=$(_linear_classify_gql_error "$body")
          _linear_emit_gql_error_messages "$body"
          case "$cls" in
            auth) return 11 ;;
            complexity)
              echo "E_GQL_COMPLEXITY: query exceeded the 10,000-point complexity cap" >&2
              return 36
              ;;
            *) return 34 ;;
          esac
        fi
        printf '%s' "$body"
        return 0
        ;;
      401)
        _linear_emit_gql_error_messages "$body"
        echo "E_GQL_UNAUTHORIZED: authentication failed (HTTP 401)" >&2
        return 11
        ;;
      400)
        local cls
        cls=$(_linear_classify_gql_error "$body")
        case "$cls" in
          auth)
            _linear_emit_gql_error_messages "$body"
            echo "E_GQL_UNAUTHORIZED: authentication failed (GraphQL auth error)" >&2
            return 11
            ;;
          complexity)
            echo "E_GQL_COMPLEXITY: query exceeded the 10,000-point complexity cap" >&2
            _linear_emit_gql_error_messages "$body"
            return 36
            ;;
          ratelimited)
            if [ "$attempt" -ge "$max_attempts" ]; then
              echo "E_GQL_RATELIMITED: rate limited by Linear; retries exhausted" >&2
              _linear_emit_gql_error_messages "$body"
              return 35
            fi
            local sleep_secs
            sleep_secs=$(_linear_ratelimit_backoff "$HDR_FILE" "$attempt")
            $SLEEP_FN "$sleep_secs"
            ;;
          *)
            echo "E_GQL_BAD_REQUEST: request rejected by Linear (HTTP 400)" >&2
            _linear_emit_gql_error_messages "$body"
            return 34
            ;;
        esac
        ;;
      5*)
        if [ "$attempt" -ge "$max_attempts" ]; then
          _linear_emit_gql_error_messages "$body"
          echo "E_GQL_SERVER_ERROR: Linear server error (HTTP $status_code); retries exhausted" >&2
          return 20
        fi
        local sleep_secs
        sleep_secs=$(_linear_expo_backoff "$attempt")
        $SLEEP_FN "$sleep_secs"
        ;;
      *)
        _linear_emit_gql_error_messages "$body"
        echo "E_GQL_SERVER_ERROR: unexpected HTTP $status_code" >&2
        return 20
        ;;
    esac
  done

  # Unreachable in practice: retryable branches handle their own exhaustion.
  echo "E_GQL_RATELIMITED: rate limited by Linear; retries exhausted" >&2
  return 35
}

# ---------------------------------------------------------------------------
# Pagination — wraps the single-request transport, follows Relay cursors.
#
# The per-request retry loop lives strictly INSIDE _linear_do_request; this
# loop wraps it, keeping the single-request transport contract intact.
# ---------------------------------------------------------------------------

_linear_paginate() {
  local path="$1" query="$2" base_vars="$3"
  local accumulated='[]'
  local cursor='null'
  local prev_cursor='__LINEAR_PAGINATE_START__'
  local page=0
  local truncated=false
  local last_body='{}'

  while :; do
    page=$((page + 1))
    local vars
    if ! vars=$(printf '%s' "$base_vars" | jq -c --argjson c "$cursor" \
      '(. // {}) + {cursor: $c}' 2>/dev/null); then
      echo "E_GQL_BAD_REQUEST: could not assemble pagination variables" >&2
      return 34
    fi

    local page_body rc=0
    page_body=$(_linear_do_request "$query" "$vars") || rc=$?
    # Partial-page failure: discard accumulated nodes, fail with this page's
    # code, emit no partial result (matches the complexity no-partial invariant).
    [ "$rc" -ne 0 ] && return "$rc"
    last_body="$page_body"

    local nodes
    nodes=$(printf '%s' "$page_body" | jq -c "${path}.nodes // []" 2>/dev/null) || nodes='[]'
    accumulated=$(jq -n --argjson acc "$accumulated" --argjson n "$nodes" '$acc + $n')

    local has_next end_cursor
    has_next=$(printf '%s' "$page_body" | jq -r "${path}.pageInfo.hasNextPage // false" \
      2>/dev/null) || has_next=false
    end_cursor=$(printf '%s' "$page_body" | jq -r "${path}.pageInfo.endCursor // empty" \
      2>/dev/null) || end_cursor=""

    # Genuinely exhausted (incl. a first page that already says false).
    [ "$has_next" != "true" ] && break

    # hasNextPage is true — apply the safety bounds.
    if [ "$page" -ge "$MAX_PAGES" ]; then
      truncated=true
      break
    fi
    # Non-advancing cursor → stop (the equality check applies only from the
    # second iteration; the first relies on the null/empty-endCursor guard).
    if [ -z "$end_cursor" ] || [ "$end_cursor" = "$prev_cursor" ]; then
      truncated=true
      break
    fi
    prev_cursor="$end_cursor"
    cursor=$(printf '%s' "$end_cursor" | jq -R '.')
  done

  local result
  if [ "$truncated" = "true" ]; then
    echo "WARN: pagination stopped after ${page} page(s) while more remain; result is incomplete (truncated=true)" >&2
    result=$(printf '%s' "$last_body" | jq -c \
      --argjson nodes "$accumulated" \
      "${path}.nodes = \$nodes | ${path}.truncated = true")
  else
    result=$(printf '%s' "$last_body" | jq -c \
      --argjson nodes "$accumulated" \
      "${path}.nodes = \$nodes | ${path}.pageInfo.hasNextPage = false | ${path}.truncated = false")
  fi
  printf '%s\n' "$result"
}

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

# Resolve a --query/--variables value: @file reads a file, anything else is
# the literal inline value.
_gql_resolve_value() {
  local raw="$1"
  if [ "${raw:0:1}" = "@" ]; then
    local file="${raw:1}"
    if [ ! -f "$file" ]; then
      echo "E_GQL_BAD_REQUEST: file not found: $file" >&2
      return 34
    fi
    cat "$file"
  else
    printf '%s' "$raw"
  fi
}

QUERY=""
VARIABLES="{}"
PAGINATE_PATH=""
DEBUG_MODE=false

while [ $# -gt 0 ]; do
  case "$1" in
    --query)
      QUERY=$(_gql_resolve_value "$2") || exit $?
      shift 2
      ;;
    --variables)
      VARIABLES=$(_gql_resolve_value "$2") || exit $?
      shift 2
      ;;
    --paginate)
      PAGINATE_PATH="$2"
      shift 2
      ;;
    --debug)
      DEBUG_MODE=true
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 2
      ;;
  esac
done

if [ -z "$QUERY" ]; then
  echo "Usage: linear-graphql.sh --query @file|<inline> [--variables ...] [--paginate <path>]" >&2
  exit 2
fi

# Validate variables parse as JSON (so a malformed --variables is caught up
# front rather than mid-pagination).
if ! printf '%s' "$VARIABLES" | jq -e . >/dev/null 2>&1; then
  echo "E_GQL_BAD_REQUEST: --variables is not valid JSON" >&2
  exit 34
fi

linear_require_dependencies

# Resolve credentials (sets LINEAR_TOKEN). Surface malformed / perms / cmd
# failures distinctly; collapse "no token" and anything else to E_GQL_NO_CREDS.
_cred_rc=0
linear_resolve_credentials || _cred_rc=$?
if [ "$_cred_rc" -ne 0 ]; then
  case "$_cred_rc" in
    25 | 27 | 29) exit "$_cred_rc" ;;
    *)
      echo "E_GQL_NO_CREDS: no resolvable Linear token" >&2
      exit 22
      ;;
  esac
fi
if [ -z "${LINEAR_TOKEN:-}" ]; then
  echo "E_GQL_NO_CREDS: no resolvable Linear token" >&2
  exit 22
fi

# Resolve sleep function (emits a hook-rejection warning before any request).
SLEEP_FN=$(_gql_resolve_sleep_fn)
if [ "${SLEEP_FN%%:*}" = "E_TEST_HOOK_REJECTED" ]; then
  exit 23
fi

# Base-URL override (loopback only, gated on ACCELERATOR_TEST_MODE=1).
if [ -n "${ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST:-}" ]; then
  if ! _gql_test_mode; then
    echo "E_TEST_OVERRIDE_REJECTED: ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST ignored — production code path" >&2
    exit 18
  fi
  override="${ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST}"
  if ! [[ "$override" =~ ^http://(127\.0\.0\.1|localhost)(:[0-9]+)?(/.*)?$ ]]; then
    echo "E_TEST_OVERRIDE_REJECTED: ACCELERATOR_LINEAR_BASE_URL_OVERRIDE_TEST rejected — not a loopback URL" >&2
    exit 18
  fi
  LINEAR_BASE="$override"
fi

# The GraphQL endpoint is always <base>/graphql (production base carries no
# path; the loopback override supplies host:port only, so /graphql is appended).
LINEAR_ENDPOINT="${LINEAR_BASE%/}/graphql"

CURL_FLAGS=(-sS --max-time 30 -X POST -H "Content-Type: application/json")

if $DEBUG_MODE; then
  echo "[linear-graphql debug] POST $LINEAR_ENDPOINT (token: ***)" >&2
fi

HDR_FILE=$(mktemp)
BODY_FILE=$(mktemp)
trap 'rm -f "$HDR_FILE" "$BODY_FILE"' EXIT

if [ -n "$PAGINATE_PATH" ]; then
  _linear_paginate "$PAGINATE_PATH" "$QUERY" "$VARIABLES"
  exit $?
else
  _linear_do_request "$QUERY" "$VARIABLES"
  exit $?
fi
