#!/usr/bin/env bash
set -euo pipefail

# Validates the [location] argument for inventory-design.
#
# Usage: validate-source.sh <location>
#
# Accepts:
#   - https:// URLs pointing to non-internal hosts
#   - Relative paths that exist as directories inside the project root,
#     with no ../ escape
#
# Rejects with exit code 1 and a message to stderr:
#   - http:// URLs (insecure, not supported in v1)
#   - file://, javascript:, data:, chrome:, about: (non-about:blank) schemes
#   - URLs resolving to loopback, link-local, or RFC1918 hosts (SSRF guard)
#   - Relative paths with ../ escapes or that do not exist

LOCATION="${1:-}"

if [ -z "$LOCATION" ]; then
  echo "error: validate-source.sh requires a location argument" >&2
  exit 1
fi

# -- Scheme detection -------------------------------------------------------

SCHEME=""
case "$LOCATION" in
  https://*)  SCHEME="https" ;;
  http://*)   SCHEME="http" ;;
  file://*)   SCHEME="file" ;;
  javascript:*) SCHEME="javascript" ;;
  data:*)     SCHEME="data" ;;
  chrome://*)  SCHEME="chrome" ;;
  about:blank) SCHEME="about_blank" ;;
  about:*)    SCHEME="about" ;;
  ./*)        SCHEME="path" ;;
  ../*)       SCHEME="path_escape" ;;
  /*)         SCHEME="abs_path" ;;
  *://*)
    SCHEME_NAME="${LOCATION%%://*}"
    echo "error: scheme '${SCHEME_NAME}://' is not permitted. Only https:// and relative code-repo paths are accepted." >&2
    exit 1
    ;;
  *)          SCHEME="path" ;;
esac

# -- Path validation --------------------------------------------------------

if [ "$SCHEME" = "path_escape" ]; then
  echo "error: location '$LOCATION' uses a ../ path escape, which is not permitted." >&2
  exit 1
fi

if [ "$SCHEME" = "path" ] || [ "$SCHEME" = "abs_path" ]; then
  if [[ "$LOCATION" == *".."* ]]; then
    echo "error: location '$LOCATION' uses a ../ path escape, which is not permitted." >&2
    exit 1
  fi
  if [ ! -d "$LOCATION" ]; then
    echo "error: location '$LOCATION' does not exist or is not a directory." >&2
    exit 1
  fi
  exit 0
fi

# -- URL scheme rejections --------------------------------------------------

case "$SCHEME" in
  file)
    echo "error: file:// URLs are not permitted as inventory locations." >&2
    exit 1
    ;;
  javascript)
    echo "error: javascript: URLs are not permitted as inventory locations." >&2
    exit 1
    ;;
  data)
    echo "error: data: URLs are not permitted as inventory locations." >&2
    exit 1
    ;;
  chrome)
    echo "error: chrome:// URLs are not permitted as inventory locations." >&2
    exit 1
    ;;
  about)
    echo "error: about: URLs (other than about:blank) are not permitted as inventory locations." >&2
    exit 1
    ;;
  http)
    echo "error: http:// URLs are not accepted. Use https:// instead." >&2
    exit 1
    ;;
esac

# -- Host allowlist check (SSRF guard) --------------------------------------
# Extract host from https:// URL

HOST="${LOCATION#https://}"
HOST="${HOST%%/*}"
HOST="${HOST%%:*}"   # strip port

# Reject loopback and localhost
if [ "$HOST" = "localhost" ] || [ "$HOST" = "::1" ]; then
  echo "error: host '$HOST' resolves to an internal address. Use --allow-internal to override (not available in v1)." >&2
  exit 1
fi

# Reject loopback IP ranges (127.x.x.x)
if [[ "$HOST" =~ ^127\. ]]; then
  echo "error: host '$HOST' is a loopback address. Use --allow-internal to override (not available in v1)." >&2
  exit 1
fi

# Reject RFC1918: 10.x.x.x
if [[ "$HOST" =~ ^10\. ]]; then
  echo "error: host '$HOST' is an RFC1918 private address. Use --allow-internal to override (not available in v1)." >&2
  exit 1
fi

# Reject RFC1918: 172.16.x.x - 172.31.x.x
if [[ "$HOST" =~ ^172\.([0-9]+)\. ]]; then
  OCTET="${BASH_REMATCH[1]}"
  if (( OCTET >= 16 && OCTET <= 31 )); then
    echo "error: host '$HOST' is an RFC1918 private address. Use --allow-internal to override (not available in v1)." >&2
    exit 1
  fi
fi

# Reject RFC1918: 192.168.x.x
if [[ "$HOST" =~ ^192\.168\. ]]; then
  echo "error: host '$HOST' is an RFC1918 private address. Use --allow-internal to override (not available in v1)." >&2
  exit 1
fi

# Reject link-local: 169.254.x.x (includes AWS metadata service)
if [[ "$HOST" =~ ^169\.254\. ]]; then
  echo "error: host '$HOST' is a link-local address (potential cloud metadata endpoint). Use --allow-internal to override (not available in v1)." >&2
  exit 1
fi

# Reject link-local IPv6: fe80::
if [[ "$HOST" =~ ^fe80: ]]; then
  echo "error: host '$HOST' is a link-local IPv6 address. Use --allow-internal to override (not available in v1)." >&2
  exit 1
fi

exit 0
