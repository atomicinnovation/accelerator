#!/usr/bin/env bash
set -euo pipefail

# Ejects (copies) a plugin default template to the user's templates
# directory for customisation.
#
# Usage: config-eject-template.sh [--force] [--dry-run] <template_name|--all>
#
# Options:
#   --force    Overwrite existing template files
#   --dry-run  Show what would happen without writing files
#   --all      Eject all templates
#
# Exit codes:
#   0 - Successfully ejected (or dry-run with no conflicts)
#   1 - Error (unknown template, missing default, usage error)
#   2 - Target already exists (use --force to overwrite)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

FORCE=false
DRY_RUN=false
TEMPLATE_NAME=""

# Parse arguments
while [ $# -gt 0 ]; do
  case "$1" in
    --force)
      FORCE=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --all)
      if [ -n "$TEMPLATE_NAME" ]; then
        echo "Error: cannot combine --all with a template name" >&2
        exit 1
      fi
      TEMPLATE_NAME="--all"
      shift
      ;;
    -*)
      echo "Error: unknown option '$1'" >&2
      exit 1
      ;;
    *)
      if [ -n "$TEMPLATE_NAME" ]; then
        echo "Error: unexpected argument '$1' (only one template name allowed)" >&2
        exit 1
      fi
      TEMPLATE_NAME="$1"
      shift
      ;;
  esac
done

if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-eject-template.sh [--force] [--dry-run] <template_name|--all>" >&2
  exit 1
fi

PROJECT_ROOT=$(config_project_root)

# Resolve target directory
TEMPLATES_DIR=$("$SCRIPT_DIR/config-read-path.sh" templates .accelerator/templates)
if [[ "$TEMPLATES_DIR" != /* ]]; then
  TEMPLATES_DIR="$PROJECT_ROOT/$TEMPLATES_DIR"
fi

# Eject a single template. Returns 0 on success, 1 on error, 2 if exists.
_eject_one() {
  local key="$1"
  local source_path="$PLUGIN_ROOT/templates/${key}.md"
  local target_path="$TEMPLATES_DIR/${key}.md"
  local display_target
  display_target=$(config_display_path "$target_path" "$PLUGIN_ROOT")

  if [ ! -f "$source_path" ]; then
    AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
    echo "Error: No plugin default template for '$key'. Available: $AVAILABLE" >&2
    return 1
  fi

  if [ -f "$target_path" ] && [ "$FORCE" = false ]; then
    if [ "$DRY_RUN" = true ]; then
      echo "Would skip: $key (exists at $display_target, use --force to overwrite)"
    else
      echo "Exists: $display_target (use --force to overwrite)" >&2
    fi
    return 2
  fi

  if [ "$DRY_RUN" = true ]; then
    if [ -f "$target_path" ]; then
      echo "Would overwrite: $key -> $display_target"
    else
      echo "Would eject: $key -> $display_target"
    fi
    return 0
  fi

  local existed=false
  [ -f "$target_path" ] && existed=true

  mkdir -p "$TEMPLATES_DIR"
  cp "$source_path" "$target_path"
  if [ "$existed" = true ]; then
    echo "Overwritten: $key -> $display_target"
  else
    echo "Ejected: $key -> $display_target"
  fi
}

if [ "$TEMPLATE_NAME" = "--all" ]; then
  HAD_ERROR=false
  HAD_EXISTS=false
  for KEY in $(config_enumerate_templates "$PLUGIN_ROOT"); do
    RC=0
    _eject_one "$KEY" || RC=$?
    if [ "$RC" -eq 1 ]; then
      HAD_ERROR=true
    elif [ "$RC" -eq 2 ]; then
      HAD_EXISTS=true
    fi
  done
  if [ "$HAD_ERROR" = true ]; then
    echo "Some templates were not ejected. Fix the errors above and re-run with --force to complete." >&2
    exit 1
  elif [ "$HAD_EXISTS" = true ]; then
    echo "Some templates already exist. Re-run with --force to overwrite." >&2
    exit 2
  fi
  exit 0
else
  _eject_one "$TEMPLATE_NAME"
fi
