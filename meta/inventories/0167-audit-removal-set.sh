#!/usr/bin/env bash
# Mechanical audit: every reference to every removal-set member, in every form.
set -uo pipefail

cd "${1:?repo root required}"

REMOVAL_SET="
scripts/config-read-value.sh
scripts/config-read-path.sh
scripts/config-read-all-paths.sh
scripts/config-read-doc-type-paths.sh
scripts/config-read-work.sh
scripts/config-read-agents.sh
scripts/config-read-agent-name.sh
scripts/config-read-context.sh
scripts/config-read-review.sh
scripts/config-read-skill-context.sh
scripts/config-read-skill-instructions.sh
scripts/config-read-template.sh
scripts/config-list-template.sh
scripts/config-show-template.sh
scripts/config-eject-template.sh
scripts/config-diff-template.sh
scripts/config-reset-template.sh
scripts/config-dump.sh
scripts/config-summary.sh
skills/config/init/scripts/init.sh
hooks/config-detect.sh
"

# Files that SURVIVE the migration and must therefore not reference the set.
is_removal_set() {
  case "$1" in
    scripts/config-read-*.sh | scripts/config-*-template.sh | \
    scripts/config-dump.sh | scripts/config-summary.sh | \
    skills/config/init/scripts/init.sh | hooks/config-detect.sh) return 0 ;;
    *) return 1 ;;
  esac
}

classify() {
  case "$1" in
    meta/*) echo "PROSE (meta/ — migration docs)" ;;
    */SKILL.md) echo "SKILL.md (Phase 5 §2/§3)" ;;
    *test-*.sh | */test-*.sh) echo "TEST SUITE (Phase 2 §7 audit)" ;;
    scripts/test-shims/*) echo "SHIM (Phase 4, deleted Phase 7)" ;;
    *.rs) echo "RUST" ;;
    tasks/*.py | tasks/*/*.py) echo "PYTHON (tasks/)" ;;
    *.sh) echo "SHELL CONSUMER (Phase 5 §4b)" ;;
    *) echo "OTHER" ;;
  esac
}

echo "########## MECHANICAL REMOVAL-SET REFERENCE AUDIT ##########"
echo

for path in $REMOVAL_SET; do
  base="$(basename "$path")"
  # `init.sh` is too generic to grep bare — require a path-ish context.
  if [ "$base" = "init.sh" ]; then
    pattern='config/init/scripts/init\.sh'
  else
    pattern="$(printf '%s' "$base" | sed 's/\./\\./g')"
  fi

  hits="$(grep -rn --binary-files=without-match "$pattern" . \
    --exclude-dir=.git --exclude-dir=.jj --exclude-dir=target \
    --exclude-dir=node_modules --exclude-dir=.venv --exclude-dir=dist \
    2>/dev/null | sed 's|^\./||')"

  [ -z "$hits" ] && continue

  echo "=== $path"
  printf '%s\n' "$hits" | while IFS=: read -r file line rest; do
    [ -z "$file" ] && continue
    if is_removal_set "$file"; then
      tag="self (removal set)"
    else
      tag="$(classify "$file")"
    fi
    printf '    %-46s %-32s :%s\n' "$file" "$tag" "$line"
  done | sort -u
  echo
done
