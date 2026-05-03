#!/usr/bin/env bash
set -euo pipefail
# jira-request.sh — authenticated Jira Cloud API request helper
# Usage:
#   jira-request.sh GET  <path> [--query KEY=VAL]...
#   jira-request.sh POST <path> --json '@file' | --json '<inline>'
#   jira-request.sh POST <path> --multipart 'file=@./path' [--multipart ...]
#   jira-request.sh PUT  <path> --json '@file'
#   jira-request.sh DELETE <path>
#
# Exit codes:
#   0  success
#   11 401 Unauthorized
#   12 403 Forbidden
#   13 404 Not Found
#   14 410 Gone
#   15 E_BAD_SITE       — jira.site failed validation
#   16 E_REQ_BAD_RESPONSE — non-JSON body on 200
#   17 E_REQ_BAD_PATH   — path argument failed validation
#   18 E_TEST_OVERRIDE_REJECTED — test URL override refused
#   19 429 retries exhausted
#   20 5xx server error
#   21 E_REQ_CONNECT    — connection / DNS / timeout
#   22 E_REQ_NO_CREDS   — no resolvable credentials
#   23 E_TEST_HOOK_REJECTED — JIRA_RETRY_SLEEP_FN refused

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/jira-common.sh"
source "$SCRIPT_DIR/jira-auth.sh"

# ---------------------------------------------------------------------------
# Test seam gating
# ---------------------------------------------------------------------------

_req_test_mode() { [ "${ACCELERATOR_TEST_MODE:-}" = "1" ]; }

_req_resolve_sleep_fn() {
  local fn="${JIRA_RETRY_SLEEP_FN:-}"
  if [ -z "$fn" ]; then echo "sleep"; return 0; fi

  if ! _req_test_mode; then
    echo "E_TEST_HOOK_REJECTED: JIRA_RETRY_SLEEP_FN ignored — not in test mode" >&2
    echo "sleep"; return 0
  fi

  if ! [[ "$fn" =~ ^_?test_[a-z_]+$ ]]; then
    echo "E_TEST_HOOK_REJECTED: JIRA_RETRY_SLEEP_FN ignored — name '$fn' is not an allowed test hook" >&2
    echo "sleep"; return 0
  fi

  if ! declare -F "$fn" > /dev/null 2>&1; then
    echo "E_TEST_HOOK_REJECTED: JIRA_RETRY_SLEEP_FN ignored — '$fn' is not defined" >&2
    echo "sleep"; return 0
  fi

  echo "$fn"
}

# ---------------------------------------------------------------------------
# Path validation
# ---------------------------------------------------------------------------

_req_validate_path() {
  local raw="$1"

  # Must start with /rest/api/3/ and only contain safe chars
  if ! [[ "$raw" =~ ^/rest/api/3/[A-Za-z0-9._/?=\&,:%@-]*$ ]]; then
    echo "E_REQ_BAD_PATH: '$raw' rejected — not under /rest/api/3/ or contains disallowed characters" >&2
    return 17
  fi

  # Check literal path for traversal and consecutive slashes
  if [[ "$raw" =~ (^|/)\.\.(/|$) ]]; then
    echo "E_REQ_BAD_PATH: '$raw' rejected — path traversal sequence" >&2
    return 17
  fi

  if [[ "$raw" =~ // ]]; then
    echo "E_REQ_BAD_PATH: '$raw' rejected — consecutive slashes" >&2
    return 17
  fi

  # Iterative URL-decode to catch encoded traversal/control sequences, cap at 8 rounds.
  # awk builds a hex lookup table and decodes %XX sequences one round at a time,
  # checking for path traversal and control characters (0x00-0x1f, 0x7f) after each round.
  local result
  result=$(LC_ALL=C printf '%s\n' "$raw" | LC_ALL=C awk '
  BEGIN {
    for (d = 0; d <= 9; d++) hv[d""] = d
    for (d = 0; d <= 5; d++) {
      hv[sprintf("%c", 65+d)] = 10+d
      hv[sprintf("%c", 97+d)] = 10+d
    }
  }
  {
    current = $0
    result = "OK"
    for (iter = 1; iter <= 8; iter++) {
      prev = current
      s = current; out = ""
      while (length(s) > 0) {
        c = substr(s, 1, 1)
        if (c == "%" && length(s) >= 3 && \
            substr(s,2,2) ~ /^[0-9A-Fa-f][0-9A-Fa-f]$/) {
          n = hv[substr(s,2,1)] * 16 + hv[substr(s,3,1)]
          out = out sprintf("%c", n)
          s = substr(s, 4)
        } else {
          out = out c
          s = substr(s, 2)
        }
      }
      current = out
      if (current == prev) break
      if (iter == 8) { result = "ITERATIONS"; break }
      if (current ~ /(^|\/)\.\.($|\/)/) { result = "TRAVERSAL"; break }
      s = current
      while (length(s) > 0) {
        c = substr(s, 1, 1)
        if (c < " " || c == "\177") { result = "CONTROL"; s = ""; break }
        s = substr(s, 2)
      }
      if (result != "OK") break
    }
    print result
  }')

  case "$result" in
    TRAVERSAL)
      echo "E_REQ_BAD_PATH: '$raw' rejected — path traversal sequence" >&2
      return 17 ;;
    CONTROL)
      echo "E_REQ_BAD_PATH: '$raw' rejected — control character" >&2
      return 17 ;;
    ITERATIONS)
      echo "E_REQ_BAD_PATH: '$raw' rejected — URL-decode iteration cap exceeded" >&2
      return 17 ;;
  esac
  return 0
}

# ---------------------------------------------------------------------------
# Site validation
# ---------------------------------------------------------------------------

_req_validate_site() {
  local site="$1"
  if ! [[ "$site" =~ ^[a-z0-9][a-z0-9-]{0,61}[a-z0-9]$ ]]; then
    echo "E_BAD_SITE: jira.site '$site' is not a valid Cloud subdomain" >&2
    return 15
  fi
}

# ---------------------------------------------------------------------------
# HTTP-date parser for Retry-After
# ---------------------------------------------------------------------------

_jira_parse_http_date() {
  local datestr="$1"
  local epoch
  # GNU date (Linux)
  if epoch=$(LC_ALL=C date -d "$datestr" +%s 2>/dev/null); then
    echo "$epoch"; return 0
  fi
  # BSD date RFC-1123 (macOS)
  if epoch=$(LC_ALL=C date -j -f "%a, %d %b %Y %H:%M:%S %Z" "$datestr" +%s 2>/dev/null); then
    echo "$epoch"; return 0
  fi
  # BSD date RFC-850
  if epoch=$(LC_ALL=C date -j -f "%A, %d-%b-%y %H:%M:%S %Z" "$datestr" +%s 2>/dev/null); then
    echo "$epoch"; return 0
  fi
  return 1
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

METHOD="${1:-}"
PATH_ARG="${2:-}"

if [ -z "$METHOD" ] || [ -z "$PATH_ARG" ]; then
  echo "Usage: jira-request.sh <METHOD> <path> [options]" >&2
  exit 1
fi
shift 2

# Validate path first (no credentials needed)
_req_validate_path "$PATH_ARG" || exit $?

# Resolve credentials (sets JIRA_SITE, JIRA_EMAIL, JIRA_TOKEN)
if ! jira_resolve_credentials 2>/dev/null; then
  echo "E_REQ_NO_CREDS: no resolvable credentials (check jira.email and jira.token)" >&2
  exit 22
fi
if [ -z "${JIRA_EMAIL:-}" ] || [ -z "${JIRA_TOKEN:-}" ]; then
  echo "E_REQ_NO_CREDS: no resolvable credentials (check jira.email and jira.token)" >&2
  exit 22
fi

# Resolve sleep function early so the hook warning is visible before any further early exit
sleep_fn=$(_req_resolve_sleep_fn)

# Check if sleep fn resolution produced a test-hook rejection code
if [ "${sleep_fn%%:*}" = "E_TEST_HOOK_REJECTED" ]; then
  exit 23
fi

# Validate JIRA_SITE
if [ -z "${JIRA_SITE:-}" ]; then
  echo "E_BAD_SITE: jira.site is not configured" >&2
  exit 15
fi
_req_validate_site "$JIRA_SITE" || exit $?

# Resolve base URL
BASE_URL="https://${JIRA_SITE}.atlassian.net"
if [ -n "${ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST:-}" ]; then
  if ! _req_test_mode; then
    echo "E_TEST_OVERRIDE_REJECTED: ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST ignored — production code path" >&2
    exit 18
  fi
  override="${ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST}"
  if ! [[ "$override" =~ ^http://(127\.0\.0\.1|localhost)(:[0-9]+)?(/.*)?$ ]]; then
    echo "E_TEST_OVERRIDE_REJECTED: ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST rejected — not a loopback URL" >&2
    exit 18
  fi
  BASE_URL="$override"
fi

# Parse options
debug_mode=false
json_body=""
multipart_args=()
query_parts=()

while [ $# -gt 0 ]; do
  case "$1" in
    --json)      json_body="$2"; shift 2 ;;
    --multipart) multipart_args+=("$2"); shift 2 ;;
    --query)     query_parts+=("$2"); shift 2 ;;
    --debug)     debug_mode=true; shift ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# Build query string
query_string=""
if [ ${#query_parts[@]} -gt 0 ]; then
  query_string="?"
  for kv in "${query_parts[@]}"; do
    query_string+="${kv}&"
  done
  query_string="${query_string%&}"
fi

URL="${BASE_URL}${PATH_ARG}${query_string}"

# Build curl args
curl_flags=(-sS --max-time 30)

case "$METHOD" in
  GET)    curl_flags+=(-X GET) ;;
  DELETE) curl_flags+=(-X DELETE) ;;
  POST)   curl_flags+=(-X POST) ;;
  PUT)    curl_flags+=(-X PUT) ;;
  *)
    echo "E_REQ_BAD_PATH: unsupported method '$METHOD'" >&2
    exit 17
    ;;
esac

if [ -n "$json_body" ]; then
  curl_flags+=(-H "Content-Type: application/json" --data "$json_body")
fi

if [ ${#multipart_args[@]} -gt 0 ]; then
  curl_flags+=(-H "X-Atlassian-Token: no-check")
  for part in "${multipart_args[@]}"; do
    curl_flags+=(-F "$part")
  done
fi

if $debug_mode; then
  echo "[jira-request debug] $METHOD $URL (token: ***)" >&2
fi

# Temp files for headers and body
hdr_file=$(mktemp)
body_file=$(mktemp)
trap 'rm -f "$hdr_file" "$body_file"' EXIT

# Retry loop
max_attempts=4
attempt=0

while [ $attempt -lt $max_attempts ]; do
  attempt=$((attempt + 1))
  : > "$hdr_file"
  : > "$body_file"

  curl_ok=true
  printf 'user = "%s:%s"\n' "$JIRA_EMAIL" "$JIRA_TOKEN" \
    | curl --config - "${curl_flags[@]}" -D "$hdr_file" -o "$body_file" "$URL" 2>/dev/null \
    || curl_ok=false

  if ! $curl_ok || [ ! -s "$hdr_file" ]; then
    echo "E_REQ_CONNECT: curl failed to connect" >&2
    exit 21
  fi

  # Extract HTTP status from first header line: "HTTP/1.1 200 OK"
  status_code=$(head -1 "$hdr_file" | awk '{print $2}' | tr -d '\r')

  if ! [[ "$status_code" =~ ^[0-9]+$ ]]; then
    echo "E_REQ_CONNECT: unexpected response (no HTTP status)" >&2
    exit 21
  fi

  case "$status_code" in
    2*)
      body=$(cat "$body_file")
      if [ -n "$body" ] && ! printf '%s' "$body" | jq -e . > /dev/null 2>&1; then
        printf '%s' "$body" >&2
        exit 16
      fi
      printf '%s' "$body"
      exit 0
      ;;
    400) cat "$body_file" >&2; exit 34 ;;
    401) cat "$body_file" >&2; exit 11 ;;
    403) cat "$body_file" >&2; exit 12 ;;
    404) cat "$body_file" >&2; exit 13 ;;
    410) cat "$body_file" >&2; exit 14 ;;
    429|5*)
      if [ $attempt -ge $max_attempts ]; then
        cat "$body_file" >&2
        [ "$status_code" = "429" ] && exit 19 || exit 20
      fi

      # Compute sleep duration from Retry-After header
      retry_after=$(grep -i "^Retry-After:" "$hdr_file" | head -1 \
        | sed 's/^[Rr][Ee][Tt][Rr][Yy]-[Aa][Ff][Tt][Ee][Rr]:[[:space:]]*//' \
        | tr -d '\r\n') || retry_after=""

      sleep_secs=""
      if [ -n "$retry_after" ]; then
        if [[ "$retry_after" =~ ^[0-9]+$ ]]; then
          delta="$retry_after"
          sleep_secs=$(( delta < 1 ? 1 : delta > 60 ? 60 : delta ))
        else
          if parsed_epoch=$(_jira_parse_http_date "$retry_after" 2>/dev/null); then
            now=$(date +%s)
            delta=$(( parsed_epoch - now ))
            sleep_secs=$(( delta < 1 ? 1 : delta > 60 ? 60 : delta ))
          else
            echo "Warning: malformed Retry-After header; falling back to exponential backoff" >&2
          fi
        fi
      fi

      if [ -z "$sleep_secs" ]; then
        # Exponential backoff with ±30% jitter
        base=$(( 1 << (attempt - 1) ))
        [ $base -gt 60 ] && base=60
        seed=$(( (RANDOM ^ $(date +%s)) % 1000 ))
        jitter=$(( base * 30 / 100 ))
        sign=$(( seed % 2 ))
        rand=$(( (seed % (jitter + 1)) ))
        if [ $sign -eq 0 ]; then
          sleep_secs=$(( base + rand ))
        else
          sleep_secs=$(( base - rand ))
        fi
        [ $sleep_secs -lt 1 ] && sleep_secs=1
        [ $sleep_secs -gt 60 ] && sleep_secs=60
      fi

      $sleep_fn "$sleep_secs"
      ;;
    *)
      cat "$body_file" >&2
      exit 20
      ;;
  esac
done

cat "$body_file" >&2
exit 19
