#!/usr/bin/env bash
# linear-attach-flow.sh — Attach a link or a binary file to a Linear issue.
#
# Usage:
#   linear-attach-flow.sh <IDENTIFIER> --url URL   [--title T] [flags]
#   linear-attach-flow.sh <IDENTIFIER> --file PATH [--title T] [flags]
#
# Exactly one of --url / --file is required.
#
# Flags:
#   --describe   Dry-run: print the planned operation and exit 0.
#   --quiet, -q  Suppress INFO stderr lines.
#   --help, -h   Print this banner and exit 0.
#
# Link mode:   attachmentCreate(input:{issueId, title, url}).
# Binary mode: fileUpload -> HTTP PUT the bytes to the pre-signed uploadUrl
#              (via a separate direct-curl path that sends NO Authorization
#              header) -> attachmentCreate(input:{issueId, title, url: assetUrl}).
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   130 E_ATTACH_NO_KEY        no issue identifier supplied
#   131 E_ATTACH_NO_TARGET     neither --url nor --file supplied
#   132 E_ATTACH_BOTH_TARGETS  both --url and --file supplied
#   133 E_ATTACH_FILE_MISSING  --file path missing / unreadable / device
#   134 E_ATTACH_BAD_URL       --url failed validation
#   135 E_ATTACH_BAD_UPLOAD_URL server uploadUrl/assetUrl host/scheme not allow-listed
#   136 E_ATTACH_UPLOAD_FAILED binary PUT failed (bounded retry exhausted)
#   137 E_ATTACH_REGISTER_FAILED attachmentCreate failed after a successful PUT
#   138 E_ATTACH_BAD_FLAG      unrecognised flag

set -euo pipefail

_LINEAR_ATTACH_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_LINEAR_ATTACH_SCRIPT_DIR/linear-common.sh"

readonly E_ATTACH_NO_KEY=130
readonly E_ATTACH_NO_TARGET=131
readonly E_ATTACH_BOTH_TARGETS=132
readonly E_ATTACH_FILE_MISSING=133
readonly E_ATTACH_BAD_URL=134
readonly E_ATTACH_BAD_UPLOAD_URL=135
readonly E_ATTACH_UPLOAD_FAILED=136
readonly E_ATTACH_REGISTER_FAILED=137
readonly E_ATTACH_BAD_FLAG=138

# The documented Linear uploads host. The allow-list also accepts any
# *.linear.app host, anchored at a label boundary (never a substring).
readonly LINEAR_UPLOADS_HOST="uploads.linear.app"

_linear_attach_usage() {
  cat <<'USAGE'
Usage:
  linear-attach-flow.sh <IDENTIFIER> --url URL   [--title T] [flags]
  linear-attach-flow.sh <IDENTIFIER> --file PATH [--title T] [flags]

  Attaches a link or a binary file to a Linear issue.

Flags:
  --describe   Dry-run: print the planned operation and exit 0.
  --quiet, -q  Suppress INFO stderr lines.
  --help, -h   Print this banner and exit 0.
USAGE
}

# Resolve the sleep function for the bounded PUT retry. Mirrors the transport's
# gating: LINEAR_RETRY_SLEEP_FN is honoured only under ACCELERATOR_TEST_MODE=1
# and only for an allow-listed test-hook name.
_attach_resolve_sleep_fn() {
  local fn="${LINEAR_RETRY_SLEEP_FN:-}"
  if [ -z "$fn" ]; then
    echo "sleep"
    return 0
  fi
  if [ "${ACCELERATOR_TEST_MODE:-}" != "1" ] || ! [[ "$fn" =~ ^_?test_[a-z_]+$ ]] ||
    ! declare -F "$fn" >/dev/null 2>&1; then
    echo "sleep"
    return 0
  fi
  echo "$fn"
}

# Extract the host component from an http(s) URL (no port, no path, no query).
_attach_url_host() {
  local url="$1" rest
  rest="${url#*://}"
  rest="${rest%%/*}"
  rest="${rest%%\?*}"
  rest="${rest%%:*}"
  printf '%s' "$rest"
}

# Redact the signed query string from an upload/asset URL for log messages —
# the query carries short-TTL bearer-style capabilities.
_attach_redact_url() {
  printf '%s' "${1%%\?*}"
}

# Validate a server-supplied uploadUrl/assetUrl. Returns 0 if allowed.
#   - Always: https:// on uploads.linear.app or any *.linear.app host
#     (anchored at a label boundary — a look-alike like
#     uploads.linear.app.evil.com or evil-linear.app is rejected).
#   - Loopback (http://127.0.0.1|localhost) is admitted ONLY under
#     ACCELERATOR_TEST_MODE=1 (mirrors the transport's loopback gate), so the
#     guard cannot be disabled outside test mode via an env var.
_attach_upload_url_ok() {
  local url="$1"
  if [ "${ACCELERATOR_TEST_MODE:-}" = "1" ] &&
    [[ "$url" =~ ^http://(127\.0\.0\.1|localhost)(:[0-9]+)?(/.*)?$ ]]; then
    return 0
  fi
  [[ "$url" == https://* ]] || return 1
  local host
  host=$(_attach_url_host "$url")
  [ "$host" = "$LINEAR_UPLOADS_HOST" ] && return 0
  case "$host" in
    *.linear.app) return 0 ;;
  esac
  return 1
}

# PUT the file bytes to the pre-signed uploadUrl. Distinct from the GraphQL
# transport and from _linear_attach: it sends NO Authorization header, never
# follows redirects, restricts the protocol, allow-lists the echoed headers,
# and retries a bounded number of times. Returns 0 on a 2xx PUT, else 136.
_linear_upload_asset() {
  local upload_url="$1" path="$2" content_type="$3" headers_json="$4"

  if ! _attach_upload_url_ok "$upload_url"; then
    printf 'E_ATTACH_BAD_UPLOAD_URL: refusing to PUT to non-allow-listed URL host (%s)\n' \
      "$(_attach_url_host "$upload_url")" >&2
    return $E_ATTACH_BAD_UPLOAD_URL
  fi

  # Build the header set: Content-Type + Cache-Control, plus the returned
  # signed headers filtered to the documented allow-list (x-amz-*). Reject any
  # header outside the allow-list, any value containing CR/LF, and never echo
  # Authorization / Host.
  local -a header_args=(
    -H "Content-Type: ${content_type}"
    -H "Cache-Control: public, max-age=31536000"
  )
  local hentry name val lname
  while IFS= read -r hentry; do
    [ -z "$hentry" ] && continue
    name=$(printf '%s' "$hentry" | jq -r '.key // empty')
    val=$(printf '%s' "$hentry" | jq -r '.value // empty')
    [ -z "$name" ] && continue
    lname=$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]')
    case "$lname" in
      authorization | host) continue ;;
      x-amz-*) ;;
      *) continue ;; # outside the allow-list → drop
    esac
    # CR/LF in the value → header-injection vector → drop.
    if printf '%s' "$val" | LC_ALL=C tr -dc '\r\n' | grep -q .; then
      continue
    fi
    header_args+=(-H "${name}: ${val}")
  done < <(printf '%s' "$headers_json" | jq -c '.[]?' 2>/dev/null)

  # Restrict the protocol to the URL's own scheme; never follow redirects.
  local proto="=https"
  [[ "$upload_url" == http://* ]] && proto="=http"

  local sleep_fn
  sleep_fn=$(_attach_resolve_sleep_fn)

  local max_attempts=3 attempt=0 status
  while [ "$attempt" -lt "$max_attempts" ]; do
    attempt=$((attempt + 1))
    status=$(curl -sS --max-time 60 --proto "$proto" --max-redirs 0 \
      -X PUT --data-binary "@${path}" \
      "${header_args[@]}" \
      -o /dev/null -w '%{http_code}' "$upload_url" 2>/dev/null) || status="000"
    case "$status" in
      2*) return 0 ;;
    esac
    if [ "$attempt" -lt "$max_attempts" ]; then
      "$sleep_fn" 1
    fi
  done
  printf 'E_ATTACH_UPLOAD_FAILED: PUT to %s failed after %d attempts (last status %s)\n' \
    "$(_attach_redact_url "$upload_url")" "$max_attempts" "$status" >&2
  return $E_ATTACH_UPLOAD_FAILED
}

# Call the GraphQL transport with a query + variables; echo body, propagate exit.
_linear_attach_gql() {
  bash "$_LINEAR_ATTACH_SCRIPT_DIR/linear-graphql.sh" \
    --query "$1" --variables "$2"
}

_linear_attach() {
  linear_require_dependencies

  local key="" url="" file="" title="" describe=0 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_attach_usage
        exit 0
        ;;
      --url)
        url="$2"
        shift 2
        ;;
      --file)
        file="$2"
        shift 2
        ;;
      --title)
        title="$2"
        shift 2
        ;;
      --describe)
        describe=1
        shift
        ;;
      --quiet | -q)
        quiet=1
        shift
        ;;
      -*)
        printf 'E_ATTACH_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        _linear_attach_usage >&2
        return $E_ATTACH_BAD_FLAG
        ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"
          shift
        else
          printf 'E_ATTACH_BAD_FLAG: unexpected positional argument: %s\n' "$1" >&2
          return $E_ATTACH_BAD_FLAG
        fi
        ;;
    esac
  done

  if [[ -z "$key" ]]; then
    printf 'E_ATTACH_NO_KEY: issue identifier required\n' >&2
    return $E_ATTACH_NO_KEY
  fi
  if [[ -z "$url" && -z "$file" ]]; then
    printf 'E_ATTACH_NO_TARGET: supply exactly one of --url or --file\n' >&2
    return $E_ATTACH_NO_TARGET
  fi
  if [[ -n "$url" && -n "$file" ]]; then
    printf 'E_ATTACH_BOTH_TARGETS: --url and --file are mutually exclusive\n' >&2
    return $E_ATTACH_BOTH_TARGETS
  fi

  # --------------------------------------------------------------- link branch
  if [[ -n "$url" ]]; then
    if ! [[ "$url" =~ ^https?:// ]]; then
      printf 'E_ATTACH_BAD_URL: --url must be an http(s) URL: %s\n' "$url" >&2
      return $E_ATTACH_BAD_URL
    fi
    local link_title="${title:-$url}"
    if ((describe)); then
      jq -n --arg op "attachmentCreate" --arg id "$key" --arg t "$link_title" --arg u "$url" \
        '{operation: $op, mode: "link", input: {issueId: $id, title: $t, url: $u}}'
      return 0
    fi
    if ! ((quiet)); then printf 'INFO: attaching link to %s\n' "$key" >&2; fi
    local lvars
    lvars=$(jq -cn --arg id "$key" --arg t "$link_title" --arg u "$url" \
      '{input: {issueId: $id, title: $t, url: $u}}')
    # shellcheck disable=SC2016 # $input is a GraphQL variable
    local lquery='mutation($input: AttachmentCreateInput!) {
      attachmentCreate(input: $input) { success attachment { id } }
    }'
    local resp req_exit=0
    resp=$(_linear_attach_gql "$lquery" "$lvars") || req_exit=$?
    if ((req_exit != 0)); then
      _linear_emit_generic_hint "$req_exit" || true
      return "$req_exit"
    fi
    printf '%s\n' "$resp"
    return 0
  fi

  # ------------------------------------------------------------- binary branch
  # File validation (mirrors jira-attach-flow.sh; readlink -f is a BSD no-op on
  # stock macOS so the [[ -f && -r ]] check is the real cross-platform gate).
  if [[ "$file" == -* ]]; then
    printf 'E_ATTACH_FILE_MISSING: file path must not begin with "-": %s\n' "$file" >&2
    return $E_ATTACH_FILE_MISSING
  fi
  if [[ -L "$file" ]]; then
    local resolved
    resolved=$(readlink -f "$file" 2>/dev/null || true)
    case "$resolved" in
      /dev/* | /proc/* | /sys/*)
        printf 'E_ATTACH_FILE_MISSING: file path resolves to a device path: %s\n' "$file" >&2
        return $E_ATTACH_FILE_MISSING
        ;;
    esac
  fi
  if ! [[ -f "$file" && -r "$file" ]]; then
    printf 'E_ATTACH_FILE_MISSING: file not found or not readable: %s\n' "$file" >&2
    return $E_ATTACH_FILE_MISSING
  fi

  local size
  size=$(wc -c <"$file" 2>/dev/null || echo 0)
  local ten_mb=$((10 * 1024 * 1024))
  if ((size > ten_mb)); then
    # Format the MB figure with awk (a declared dependency), not bc.
    printf 'Warning: %s is %s MB — large uploads may fail\n' \
      "$file" "$(awk -v s="$size" 'BEGIN { printf "%.1f", s / 1048576 }')" >&2
  fi

  local filename content_type
  filename=$(basename "$file")
  content_type=$(file -b --mime-type "$file" 2>/dev/null || echo "application/octet-stream")
  [ -z "$content_type" ] && content_type="application/octet-stream"

  local attach_title="${title:-$filename}"

  if ((describe)); then
    jq -n --arg op "fileUpload" --arg id "$key" --arg f "$filename" \
      --arg ct "$content_type" --argjson sz "$size" --arg t "$attach_title" \
      '{operation: $op, mode: "binary", issueId: $id, filename: $f,
        contentType: $ct, size: $sz, title: $t}'
    return 0
  fi

  if ! ((quiet)); then
    printf 'INFO: uploading %s (%s) to %s\n' "$filename" "$content_type" "$key" >&2
  fi

  # Step 1: fileUpload — request a pre-signed upload URL.
  local fuvars
  fuvars=$(jq -cn --arg ct "$content_type" --arg f "$filename" --argjson sz "$size" \
    '{contentType: $ct, filename: $f, size: $sz}')
  # shellcheck disable=SC2016 # $contentType/$filename/$size are GraphQL variables
  local fuquery='mutation($contentType: String!, $filename: String!, $size: Int!) {
    fileUpload(contentType: $contentType, filename: $filename, size: $size) {
      success
      uploadFile { uploadUrl assetUrl headers { key value } }
    }
  }'
  local fu_resp fu_exit=0
  fu_resp=$(_linear_attach_gql "$fuquery" "$fuvars") || fu_exit=$?
  if ((fu_exit != 0)); then
    _linear_emit_generic_hint "$fu_exit" || true
    return "$fu_exit"
  fi

  local upload_url asset_url headers_json
  upload_url=$(linear_jq_field "$fu_resp" '.data.fileUpload.uploadFile.uploadUrl')
  asset_url=$(linear_jq_field "$fu_resp" '.data.fileUpload.uploadFile.assetUrl')
  headers_json=$(printf '%s' "$fu_resp" | jq -c '.data.fileUpload.uploadFile.headers // []')

  # Validate BOTH urls before touching the network.
  if ! _attach_upload_url_ok "$upload_url"; then
    printf 'E_ATTACH_BAD_UPLOAD_URL: uploadUrl host/scheme not allow-listed (%s)\n' \
      "$(_attach_url_host "$upload_url")" >&2
    return $E_ATTACH_BAD_UPLOAD_URL
  fi
  if ! _attach_upload_url_ok "$asset_url"; then
    printf 'E_ATTACH_BAD_UPLOAD_URL: assetUrl host/scheme not allow-listed (%s)\n' \
      "$(_attach_url_host "$asset_url")" >&2
    return $E_ATTACH_BAD_UPLOAD_URL
  fi

  # Step 2: PUT the bytes (no Authorization header, no redirects).
  local put_exit=0
  _linear_upload_asset "$upload_url" "$file" "$content_type" "$headers_json" || put_exit=$?
  if ((put_exit != 0)); then
    return "$put_exit"
  fi

  # Step 3: register the uploaded asset as an attachment.
  local acvars
  acvars=$(jq -cn --arg id "$key" --arg t "$attach_title" --arg u "$asset_url" \
    '{input: {issueId: $id, title: $t, url: $u}}')
  # shellcheck disable=SC2016 # $input is a GraphQL variable
  local acquery='mutation($input: AttachmentCreateInput!) {
    attachmentCreate(input: $input) { success attachment { id } }
  }'
  local ac_resp ac_exit=0
  ac_resp=$(_linear_attach_gql "$acquery" "$acvars") || ac_exit=$?
  if ((ac_exit != 0)); then
    # PUT succeeded but registration failed — the uploaded asset is orphaned.
    # The three steps are not atomic and a re-run re-uploads (not idempotent).
    # Surface which step failed and the resulting remote state; redact the
    # signed query string of the asset URL.
    printf 'E_ATTACH_REGISTER_FAILED: the file was uploaded to %s but attachmentCreate (step 3) failed (transport exit %d). The asset is orphaned in Linear; the operation is NOT idempotent across steps — a re-run re-uploads.\n' \
      "$(_attach_redact_url "$asset_url")" "$ac_exit" >&2
    return $E_ATTACH_REGISTER_FAILED
  fi
  printf '%s\n' "$ac_resp"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_attach "$@"
fi
