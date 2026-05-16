#!/usr/bin/env bash
# shellcheck shell=bash

# install_fake_gh <bin-dir>
# Writes a fake `gh` binary into <bin-dir>. The fake records argv to
# $GH_ARGV_LOG and, for `gh api` calls that pass `--input <file>`,
# copies the file's contents into $GH_STDIN_LOG so tests can inspect
# the JSON payload uniformly (production code uses --input, not stdin).
#
# Reserved exit codes (the script-under-test MUST NOT return these):
#   98 --input <path> argv supplied but path is not a readable file
#      (loud trap against silent stdin-fallback hangs in CI)
#   99 unknown verb / unexpected invocation (loud trap against typos
#      and accidental subcommand additions)
# Tests that assert on script-under-test exit codes must not collide
# with 98 or 99.
install_fake_gh() {
  local bin_dir="$1"
  mkdir -p "$bin_dir"
  cat >"$bin_dir/gh" <<'FAKE_GH'
#!/usr/bin/env bash
echo "$@" >> "${GH_ARGV_LOG:?GH_ARGV_LOG must be set}"
case "$1 ${2:-}" in
  "pr view")
    if [ -n "${GH_PR_VIEW_OUT:-}" ] && [ -f "$GH_PR_VIEW_OUT" ]; then
      cat "$GH_PR_VIEW_OUT"
    fi
    if [ -n "${GH_PR_VIEW_ERR:-}" ] && [ -f "$GH_PR_VIEW_ERR" ]; then
      cat "$GH_PR_VIEW_ERR" >&2
    fi
    exit "${GH_PR_VIEW_RC:-0}"
    ;;
  "api "*|"api")
    # Real gh reads --input <file> when present, otherwise from stdin.
    # Mirror that so round-trip body assertions work regardless of the
    # caller's posting style. Fail loudly if --input <path> is in
    # argv but the path is not a readable file (a CI/hang trap).
    if [ -n "${GH_STDIN_LOG:-}" ]; then
      input_path=""
      input_explicit=0
      prev=""
      for arg in "$@"; do
        if [ "$prev" = "--input" ]; then
          if [ "$arg" = "-" ]; then
            : # explicit stdin sentinel — fall through to cat below
          else
            input_path="$arg"
            input_explicit=1
          fi
          break
        fi
        prev="$arg"
      done
      if [ "$input_explicit" -eq 1 ]; then
        if [ -f "$input_path" ] && [ -r "$input_path" ]; then
          cat "$input_path" >> "$GH_STDIN_LOG"
        else
          echo "fake-gh: --input path $input_path is not a readable file" >&2
          exit 98
        fi
      else
        cat >> "$GH_STDIN_LOG"
      fi
    fi
    if [ -n "${GH_API_OUT:-}" ] && [ -f "$GH_API_OUT" ]; then
      cat "$GH_API_OUT"
    fi
    if [ -n "${GH_API_ERR:-}" ] && [ -f "$GH_API_ERR" ]; then
      cat "$GH_API_ERR" >&2
    fi
    exit "${GH_API_RC:-0}"
    ;;
  *)
    echo "fake-gh: unexpected invocation: $*" >&2
    exit 99
    ;;
esac
FAKE_GH
  chmod +x "$bin_dir/gh"
}

# setup_gh_stub <tmpdir>
# Creates a bin directory under tmpdir, installs the fake gh, scopes
# TMPDIR to a fresh empty subdir so tempfile-cleanup assertions can
# detect leaks, and exports defaults for all env vars the fake reads.
setup_gh_stub() {
  local tmpdir="$1"
  local bin_dir="$tmpdir/bin"
  install_fake_gh "$bin_dir"
  export PATH="$bin_dir:$PATH"
  export TMPDIR="$tmpdir/mktemp"
  mkdir -p "$TMPDIR"
  export GH_ARGV_LOG="$tmpdir/gh-argv.log"
  export GH_STDIN_LOG="$tmpdir/gh-stdin.log"
  : >"$GH_ARGV_LOG"
  : >"$GH_STDIN_LOG"
  unset GH_PR_VIEW_OUT GH_PR_VIEW_ERR GH_PR_VIEW_RC
  unset GH_API_OUT GH_API_ERR GH_API_RC
}

# install_fake_jq <bin-dir>
# Writes a fake `jq` into <bin-dir> that mimics the real binary closely
# enough for the resolver (`jq -r '<filter>'`) to keep working while
# forcing the encoder (`jq -Rs '<filter>'`) to fail. Used by test 18 to
# force the encode-failure branch without short-circuiting the resolver
# stage that runs earlier in pr-update-body.sh.
#
# Reserved exit codes (must not collide with the script-under-test):
#   5  encode-mode simulated failure (the documented test signal)
#   97 no real jq locatable on FAKE_JQ_REAL_PATH
install_fake_jq() {
  local bin_dir="$1"
  mkdir -p "$bin_dir"
  cat >"$bin_dir/jq" <<'FAKE_JQ'
#!/usr/bin/env bash
# Mode of operation is decided by the first flag. -Rs forces the
# encoder branch to exit non-zero; any other invocation delegates to
# the real jq located via FAKE_JQ_REAL_PATH (snapshotted before the
# fake bin was prepended). Self-detection uses `-ef` (inode equality)
# so macOS symlink-vs-realpath PATH mismatches can't cause exec-loops.
case "${1:-}" in
  -Rs)
    echo "fake-jq: simulated encode failure" >&2
    exit 5
    ;;
  *)
    real_jq=""
    IFS=":" read -ra parts <<<"${FAKE_JQ_REAL_PATH:-$PATH}"
    for dir in "${parts[@]}"; do
      [ -n "$dir" ] || continue
      candidate="$dir/jq"
      if [ -x "$candidate" ] && ! [ "$candidate" -ef "${BASH_SOURCE[0]}" ]; then
        real_jq="$candidate"
        break
      fi
    done
    if [ -z "$real_jq" ]; then
      echo "fake-jq: no real jq found on FAKE_JQ_REAL_PATH" >&2
      exit 97
    fi
    exec "$real_jq" "$@"
    ;;
esac
FAKE_JQ
  chmod +x "$bin_dir/jq"
}

# setup_fake_jq <bin-dir>
# Installs install_fake_jq into <bin-dir>. Snapshots the current PATH
# into FAKE_JQ_REAL_PATH BEFORE prepending the fake's bin so the
# delegation can locate the real jq. Call after setup_gh_stub so both
# fakes co-exist on PATH.
#
# Preflight: asserts a real jq is locatable on the snapshotted PATH
# before installing the fake. Returns 1 (without `exit`) if jq is
# absent, so the caller can convert the missing-dependency case into a
# skip rather than a hard harness abort under `set -e`.
#
# Caller idiom (REQUIRED — direct invocation under `set -e` would
# terminate the entire harness if jq is missing):
#
#   if ! setup_fake_jq "$tmpdir/jqbin"; then
#     skip_test "test 18" "real jq required for fake-jq delegation"
#     return  # or `continue` / early-return from the test function
#   fi
#
# Single-use contract: must be called at most once per shell process
# with a clean PATH (no prior fake-jq prepended). The harness's
# per-test tempdir + subshell pattern enforces this naturally.
setup_fake_jq() {
  local bin_dir="$1"
  if ! command -v jq >/dev/null 2>&1; then
    echo "setup_fake_jq: real jq required on PATH for delegation fallback" >&2
    return 1
  fi
  export FAKE_JQ_REAL_PATH="$PATH"
  install_fake_jq "$bin_dir"
  export PATH="$bin_dir:$PATH"
}
