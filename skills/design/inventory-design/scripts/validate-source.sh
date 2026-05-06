#!/usr/bin/env bash

if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "validate-source.sh requires bash" >&2
  exit 2
fi

set -euo pipefail

# Validates the [location] argument for inventory-design.
#
# Usage: validate-source.sh [--allow-internal] [--allow-insecure-scheme] <location>
#
# Accepts:
#   - https:// URLs to public hosts
#   - http://localhost and http://127.0.0.1 (any port, any path) by default
#   - http:// URLs to other internal hosts with --allow-internal
#   - http:// URLs to public hosts with --allow-insecure-scheme
#   - Relative paths that exist as directories (no ../ escape)
#
# Rejects with exit code 1:
#   - file://, javascript:, data:, chrome:, about: (non-about:blank) schemes
#   - http:// to public hosts without --allow-insecure-scheme
#   - Internal/reserved hosts (except localhost/127.0.0.1) without --allow-internal
#   - Numeric IPv4 encodings (decimal, hex, octal) — no flag bypass
#   - Userinfo segments (@) in URL authority
#   - Relative paths with ../ escapes or that do not exist
#
# Exits with code 2 for usage errors (unknown flags, missing location).

# -- Helper: canonicalise_host -------------------------------------------------
# Input:  raw authority string (host[:port], possibly with IPv6 brackets/zone-id)
# Output: lowercased, trailing-dot-stripped, bracket-stripped, port-stripped,
#         zone-id-stripped canonical host (printed to stdout).
# Returns 1 if malformed: '@' in input, decimal-only long int, 0x prefix, octal dotted-quad.
canonicalise_host() {
  local raw="$1"

  # Reject userinfo / suffix-confusion forms
  [[ "$raw" == *@* ]] && return 1

  # Lowercase
  raw="${raw,,}"

  if [[ "$raw" == \[* ]]; then
    # IPv6 literal: strip surrounding brackets
    raw="${raw#\[}"
    raw="${raw%%\]*}"
    # Strip zone-id (anything after '%')
    raw="${raw%%%*}"
  else
    # IPv4 / hostname: strip port (everything from first ':')
    raw="${raw%%:*}"
  fi

  # Strip single trailing dot (FQDN form)
  raw="${raw%.}"

  # Reject decimal-only long integers (decimal IPv4 encoding, e.g. 2130706433)
  if [[ "$raw" =~ ^[0-9]+$ ]] && (( ${#raw} > 3 )); then
    return 1
  fi

  # Reject hex-encoded IPv4 (e.g. 0x7f000001)
  if [[ "$raw" =~ ^0x ]]; then
    return 1
  fi

  # Reject octal-encoded dotted-quad (e.g. 0177.0.0.1 — leading zero on first octet)
  if [[ "$raw" =~ ^0[0-9]+\. ]]; then
    return 1
  fi

  printf '%s' "$raw"
}

# -- Helper: is_localhost_default ----------------------------------------------
# Returns 0 if the canonical host is always-allowed localhost or 127.0.0.1.
is_localhost_default() {
  case "$1" in
    localhost|127.0.0.1) return 0 ;;
    *) return 1 ;;
  esac
}

# -- Helper: classify_internal -------------------------------------------------
# Returns 0 and prints classification to stdout if the canonical host is an
# internal/reserved address. Returns 1 (public) otherwise.
# Classifications: loopback, RFC1918, link-local, wildcard
classify_internal() {
  local h="$1"

  # Named IPv6 addresses
  case "$h" in
    ::1|::ffff:127.0.0.1) printf 'loopback'; return 0 ;;
    ::|0.0.0.0)           printf 'wildcard'; return 0 ;;
  esac

  # IPv4 loopback range: 127.x.x.x
  if [[ "$h" =~ ^127\. ]]; then printf 'loopback'; return 0; fi

  # RFC1918: 10.x.x.x
  if [[ "$h" =~ ^10\. ]]; then printf 'RFC1918'; return 0; fi

  # RFC1918: 192.168.x.x
  if [[ "$h" =~ ^192\.168\. ]]; then printf 'RFC1918'; return 0; fi

  # RFC1918: 172.16.0.0/12 (172.16.x.x through 172.31.x.x)
  if [[ "$h" =~ ^172\.([0-9]+)\. ]]; then
    local octet="${BASH_REMATCH[1]}"
    if (( octet >= 16 && octet <= 31 )); then
      printf 'RFC1918'
      return 0
    fi
  fi

  # Link-local: 169.254.x.x (includes cloud metadata services)
  if [[ "$h" =~ ^169\.254\. ]]; then printf 'link-local'; return 0; fi

  # Link-local IPv6: fe80::/10
  if [[ "$h" =~ ^fe80: ]]; then printf 'link-local'; return 0; fi

  return 1
}

# -- Main logic ----------------------------------------------------------------
main() {
  local allow_internal=0
  local allow_insecure_scheme=0
  local location=""

  while (( $# > 0 )); do
    case "$1" in
      --allow-internal)        allow_internal=1; shift ;;
      --allow-insecure-scheme) allow_insecure_scheme=1; shift ;;
      --)                      shift; location="${1:-}"; break ;;
      -*)
        echo "error: unknown flag $1" >&2
        exit 2
        ;;
      *)
        if [[ -n "$location" ]]; then
          echo "error: unexpected positional argument '$1'" >&2
          exit 2
        fi
        location="$1"
        shift
        ;;
    esac
  done

  if [[ -z "$location" ]]; then
    echo "error: validate-source.sh requires a location argument" >&2
    exit 2
  fi

  # -- Scheme detection --------------------------------------------------------

  local scheme=""
  case "$location" in
    https://*)    scheme="https" ;;
    http://*)     scheme="http" ;;
    file://*)     scheme="file" ;;
    javascript:*) scheme="javascript" ;;
    data:*)       scheme="data" ;;
    chrome://*)   scheme="chrome" ;;
    about:blank)  scheme="about_blank" ;;
    about:*)      scheme="about" ;;
    ./*)          scheme="path" ;;
    ../*)         scheme="path_escape" ;;
    /*)           scheme="abs_path" ;;
    *://*)
      local scheme_name="${location%%://*}"
      echo "error: scheme '${scheme_name}://' is not permitted. Only https:// and relative code-repo paths are accepted." >&2
      exit 1
      ;;
    *) scheme="path" ;;
  esac

  # -- Path validation ---------------------------------------------------------

  if [[ "$scheme" == "path_escape" ]]; then
    echo "error: location '$location' uses a ../ path escape, which is not permitted." >&2
    exit 1
  fi

  if [[ "$scheme" == "path" ]] || [[ "$scheme" == "abs_path" ]]; then
    if [[ "$location" == *".."* ]]; then
      echo "error: location '$location' uses a ../ path escape, which is not permitted." >&2
      exit 1
    fi
    if [[ ! -d "$location" ]]; then
      echo "error: location '$location' does not exist or is not a directory." >&2
      exit 1
    fi
    exit 0
  fi

  # -- URL scheme rejections ---------------------------------------------------

  case "$scheme" in
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
  esac

  # -- Extract and canonicalise host -------------------------------------------

  # Strip scheme prefix, then strip path/query to isolate authority
  local authority="${location#*://}"
  authority="${authority%%/*}"
  authority="${authority%%\?*}"

  # Reject userinfo before canonicalisation (catches user@host, user:pass@host)
  if [[ "$authority" == *@* ]]; then
    echo "error: URL contains a userinfo segment (user@host), which is not permitted." >&2
    exit 1
  fi

  local canonical_host=""
  if ! canonical_host="$(canonicalise_host "$authority")"; then
    echo "error: host '$authority' uses a numeric IPv4 encoding (decimal, hex, or octal), which is not permitted." >&2
    exit 1
  fi

  # -- Host classification and allow/deny logic --------------------------------

  # localhost-default: always allowed on both http and https
  if is_localhost_default "$canonical_host"; then
    exit 0
  fi

  # Internal / reserved ranges
  local classification=""
  if classification="$(classify_internal "$canonical_host")"; then
    if (( allow_internal )); then
      exit 0
    fi
    echo "error: host '$canonical_host' is a $classification address. Pass --allow-internal to permit." >&2
    exit 1
  fi

  # Public host — https is always allowed; http requires --allow-insecure-scheme
  if [[ "$scheme" == "http" ]]; then
    if (( allow_insecure_scheme )); then
      exit 0
    fi
    echo "error: http:// to public host '$canonical_host' is rejected. Use https:// or pass --allow-insecure-scheme." >&2
    exit 1
  fi

  # https to public host — always allowed
  exit 0
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
