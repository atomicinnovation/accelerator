#!/usr/bin/env bash
set -euo pipefail

# Reads a path configuration value.
# Usage: config-read-path.sh <path_key> [default]
#
# Path keys:
#   plans         → where plans are written (default: meta/plans)
#   research      → where research docs are written (default: meta/research)
#   decisions     → where ADRs are written (default: meta/decisions)
#   prs           → where PR descriptions are written (default: meta/prs)
#   validations   → where validation reports are written (default: meta/validations)
#   review_plans  → where plan reviews are written (default: meta/reviews/plans)
#   review_prs    → where PR review working dirs go (default: meta/reviews/prs)
#   review_work   → where work-item reviews are written (default: meta/reviews/work)
#   templates     → where user templates are found (default: .accelerator/templates)
#   work          → where work item files are stored (default: meta/work)
#   notes         → where notes are stored (default: meta/notes)
#   tmp           → ephemeral working data, gitignored (default: .accelerator/tmp)
#   integrations  → per-integration cached state (default: .accelerator/state/integrations)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Delegate to config-read-value.sh with paths. prefix
exec "$SCRIPT_DIR/config-read-value.sh" "paths.${1:-}" "${2:-}"
