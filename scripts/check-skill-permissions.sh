#!/usr/bin/env bash
# shellcheck disable=SC2016 # single quotes are deliberate throughout: this
# script matches the literal token ${CLAUDE_PLUGIN_ROOT} and shell
# metacharacters as text, and must never expand them.
set -euo pipefail

# Guards the invocation contract between a SKILL.md's `!`-preprocessor commands
# and its `allowed-tools` Bash rules. For every SKILL.md:
#
#   1. Every `!`-preprocessor command (bare-path and accelerator alike) is
#      covered by at least one Bash(...) frontmatter rule, matched as a
#      prefix/glob where `*` spans `/` — the empirically-verified matcher
#      semantics. An uncovered command would throw a permission prompt at load.
#   2. No Bash rule authorises the launcher without naming a subcommand — an
#      ancestor glob would silently pre-authorise every future sub-binary. The
#      rule is evaluated against the binary path, so `.../*` and `.../bin/*` are
#      caught, not just the literal.
#   3. Every `accelerator config` command in a `!` block carries `--fail-safe`;
#      without it a read failure exits non-zero and discards the whole prompt.
#   4. No `!` command contains a shell metacharacter — the matcher is a literal
#      prefix, so a chained command could smuggle an unmatched call past a rule.
#
# Exits non-zero on any violation.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# A launcher command naming no subcommand — any rule matching this is too broad.
BARE_LAUNCHER='${CLAUDE_PLUGIN_ROOT}/bin/accelerator zz-external-subcommand-zz'

fail_count=0
report() {
  echo "check-skill-permissions: $1" >&2
  fail_count=$((fail_count + 1))
}

# The `Bash(...)` rule inners from a SKILL.md's frontmatter (between the first
# two `---` lines). One rule per line.
frontmatter_bash_rules() {
  awk '
    NR == 1 && $0 == "---" { infm = 1; next }
    infm && $0 == "---" { exit }
    infm {
      while (match($0, /Bash\([^)]*\)/)) {
        rule = substr($0, RSTART + 5, RLENGTH - 6)
        print rule
        $0 = substr($0, RSTART + RLENGTH)
      }
    }
  ' "$1"
}

# Every `!`-preprocessor command in a SKILL.md body: the text between `` !` ``
# and the next backtick.
preprocessor_commands() {
  grep -oE '!`[^`]*`' "$1" 2>/dev/null | sed -E 's/^!`//; s/`$//' || true
}

# Whether the frontmatter declares a bare `Bash` tool (no parentheses), which
# authorises every Bash command — the integration write-skills use this.
has_bare_bash() {
  awk '
    NR == 1 && $0 == "---" { infm = 1; next }
    infm && $0 == "---" { exit }
    infm && $0 ~ /^[[:space:]]*-?[[:space:]]*Bash[[:space:]]*$/ { found = 1 }
    END { exit(found ? 0 : 1) }
  ' "$1"
}

# Whether a `!` command invokes a plugin script or the launcher — the only
# invocations `allowed-tools` governs. Shell builtins (printf, echo, …) are
# auto-approved and out of scope.
is_plugin_invocation() {
  case "$1" in
    '${CLAUDE_PLUGIN_ROOT}/'*) return 0 ;;
  esac
  return 1
}

# Whether $1 (command) is covered by rule-pattern $2, matched as a prefix glob:
# a rule not ending in `*` still matches the command plus trailing arguments.
covered_by() {
  local cmd="$1" pat="$2"
  case "$pat" in
    *'*') ;;
    *) pat="$pat"'*' ;;
  esac
  # shellcheck disable=SC2254 # $pat is a deliberate glob, not a literal
  case "$cmd" in
    $pat) return 0 ;;
  esac
  return 1
}

has_metacharacter() {
  case "$1" in
    *'&&'* | *'||'* | *';'* | *'|'* | *'$('* | *'`'* | *'<('* | *'>('*)
      return 0
      ;;
  esac
  return 1
}

check_skill() {
  local skill="$1" rel="${1#"$PLUGIN_ROOT"/}"
  local rules cmd rule covered bare=0

  rules="$(frontmatter_bash_rules "$skill")"
  if has_bare_bash "$skill"; then
    bare=1
  fi

  # A rule that authorises the launcher without a subcommand is too broad.
  while IFS= read -r rule; do
    [ -n "$rule" ] || continue
    if covered_by "$BARE_LAUNCHER" "$rule"; then
      report "$rel: rule 'Bash($rule)' authorises the launcher without a \
subcommand — name 'config' (or the specific subcommand) so it does not grant \
the whole dispatch surface"
    fi
  done <<EOF
$rules
EOF

  while IFS= read -r cmd; do
    [ -n "$cmd" ] || continue
    is_plugin_invocation "$cmd" || continue

    if has_metacharacter "$cmd"; then
      report "$rel: '!\`$cmd\`' contains a shell metacharacter — the matcher is \
a literal prefix and cannot see past it"
      continue
    fi

    case "$cmd" in
      *"/bin/accelerator config "*)
        case "$cmd" in
          *' --fail-safe'*) ;;
          *) report "$rel: '!\`$cmd\`' is missing --fail-safe — a read failure \
would exit non-zero and discard the prompt" ;;
        esac
        ;;
    esac

    [ "$bare" -eq 1 ] && continue

    covered=0
    while IFS= read -r rule; do
      [ -n "$rule" ] || continue
      if covered_by "$cmd" "$rule"; then
        covered=1
        break
      fi
    done <<EOF
$rules
EOF
    if [ "$covered" -eq 0 ]; then
      report "$rel: '!\`$cmd\`' is not covered by any Bash(...) rule — it will \
prompt at load"
    fi
  done <<EOF
$(preprocessor_commands "$skill")
EOF
}

while IFS= read -r skill; do
  check_skill "$skill"
done < <(find "$PLUGIN_ROOT/skills" -name SKILL.md -type f | sort)

if [ "$fail_count" -gt 0 ]; then
  echo "check-skill-permissions: $fail_count violation(s)" >&2
  exit 1
fi
echo "check-skill-permissions: OK"
