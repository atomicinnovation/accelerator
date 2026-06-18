#!/usr/bin/env bash
# Path-based doc-type classification, single-sourced by the 0007 migration and
# the corpus validator (previously byte-identical duplicated copies).
#
# Pure functions only — no top-level side effects, safe to source under
# `set -euo pipefail` on bash 3.2 (no associative arrays, no ${var,,}).
#
# NB: the awk rewrite (0007-frontmatter-rewrite.awk:path_to_typed) encodes the
# SAME directory→type fact for a DIFFERENT input — the referenced meta-path
# inside a linkage value, not the current file — so it cannot consume the
# file-level `-v type` channel and must stay a third, in-runtime copy. The two
# encodings MUST be kept aligned; a fixture in test-migrate-0007.sh asserts a
# meta/prs/ path resolves to pr-description in both surfaces.

# Location → doc-type (exhaustive; reviews discriminated by subdirectory, which
# MUST precede the generic */work/* and */plans/* and the bare */prs/* arms).
infer_type_from_path() {
  case "$1" in
    */reviews/plans/*) echo plan-review ;;
    */reviews/work/*) echo work-item-review ;;
    */reviews/prs/*) echo pr-review ;;
    */prs/*) echo pr-description ;; # after reviews/prs so it can't shadow it
    */work/*) echo work-item ;;
    */plans/*) echo plan ;;
    */decisions/*) echo adr ;;
    */research/codebase/*) echo codebase-research ;;
    */research/issues/*) echo issue-research ;;
    */research/design-gaps/*) echo design-gap ;;
    */research/design-inventories/*) echo design-inventory ;;
    */validations/*) echo plan-validation ;;
    */notes/*) echo note ;;
    *) echo "" ;;
  esac
}

# Out of scope (skip entirely): specs/talks/global (freeform) and meta/docs/
# (freeform docs the plugin does not own; no schema type). The docs/ arm is
# anchored to */meta/docs/* (NOT a bare */docs/*) so it excludes only the
# top-level corpus docs tree and cannot over-match a nested `…/docs/…` segment
# elsewhere in the corpus.
out_of_scope() {
  case "$1" in
    */specs/* | */talks/* | */global/* | */meta/docs/*) return 0 ;;
    *) return 1 ;;
  esac
}
