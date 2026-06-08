# 0007-frontmatter-rewrite.awk — deterministic base-field/identity/provenance
# rewrite for one already-fenced meta/ artifact. Assembled after the shared
# fragment: awk -f frontmatter-frag.awk -f 0007-frontmatter-rewrite.awk.
#
# It owns all BEGIN/state/rules; the fragment supplies fm_normalise_value /
# fm_semantic_inner / fm_refuses / fm_is_fence. Only the frontmatter fence
# region is rewritten; the body is passed through verbatim.
#
# -v inputs (all caller-computed):
#   file            diagnostics label
#   type            inferred/explicit doc-type
#   anchored        1 if code_state_anchored
#   own_id_key      legacy own-identity key for this type (work_item_id / adr_id
#                   / ""); renamed to id. Foreign refs (own_id_key="") are kept.
#   id_from_stem    filename-stem id, used only when neither id: nor an own-id
#                   key is present
#   repo_name       repository name for the provenance bundle (may be empty)
#   statusvocab     pipe-joined canonical status vocabulary for this type
#   statusmap       space-joined legacy=canonical pairs for this type
#   has_type has_id has_tags has_schema has_lu has_lub has_date has_author
#   has_producer has_revision has_repository   presence flags (1/0)
#
# Diagnostics: 0007-DIVERGE[...] / 0007-REFUSE / 0007-MALFORMED to stderr.

function trim(s) { sub(/^[ \t]+/, "", s); sub(/[ \t]+$/, "", s); return s }

# Normalise a date(-only) value to a full ISO-8601 timestamp. A bare
# YYYY-MM-DD gets a midnight-UTC suffix by string concatenation (no date(1));
# an already-full timestamp (Z or ±HH:MM) is returned unchanged; anything else
# is returned unchanged and flagged by the caller.
function norm_date(v,   inner, base, off) {
  inner = fm_semantic_inner(v)
  if (inner ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/)
    return "\"" inner "T00:00:00+00:00\""
  if (inner ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9](Z|[+-][0-9][0-9]:[0-9][0-9])$/)
    return "\"" inner "\""
  # Legacy colon-less offset (e.g. "…T21:49:56+0000") → insert the colon.
  if (inner ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9][+-][0-9][0-9][0-9][0-9]$/) {
    base = substr(inner, 1, 19)
    off = substr(inner, 20)
    return "\"" base substr(off, 1, 3) ":" substr(off, 4, 2) "\""
  }
  return ""   # non-conforming, non-normalisable (e.g. space-separated / TZ abbrev)
}

# Linkage vocabulary membership (keys whose values are typed references).
function is_linkage_key(k) {
  return (k == "parent" || k == "target" || k == "source" || k == "superseded_by" ||
    k == "supersedes" || k == "blocks" || k == "blocked_by" ||
    k == "derived_from" || k == "relates_to")
}

# Map a project-relative meta path to a typed "doc-type:id" reference, per the
# target doc-type's identity convention (work-item/ADR → bare number / ADR-NNNN;
# every other type → full filename stem). Returns "" for an unmapped directory
# (specs/talks/global/typo) so the caller can DIVERGE and leave it untouched.
function path_to_typed(p,   type, base, id) {
  if (p ~ /^meta\/work\//) type = "work-item"
  else if (p ~ /^meta\/plans\//) type = "plan"
  else if (p ~ /^meta\/decisions\//) type = "adr"
  else if (p ~ /^meta\/research\/codebase\//) type = "codebase-research"
  else if (p ~ /^meta\/research\/issues\//) type = "issue-research"
  else if (p ~ /^meta\/research\/design-gaps\//) type = "design-gap"
  else if (p ~ /^meta\/research\/design-inventories\//) type = "design-inventory"
  else if (p ~ /^meta\/reviews\/plans\//) type = "plan-review"
  else if (p ~ /^meta\/reviews\/work\//) type = "work-item-review"
  else if (p ~ /^meta\/reviews\/prs\//) type = "pr-review"
  else if (p ~ /^meta\/validations\//) type = "plan-validation"
  else if (p ~ /^meta\/notes\//) type = "note"
  else return ""
  base = p; sub(/.*\//, "", base); sub(/\.md$/, "", base)
  if (type == "work-item") { id = base; sub(/-.*/, "", id) }
  else if (type == "adr") { if (match(base, /^ADR-[0-9]+/)) id = substr(base, RSTART, RLENGTH); else id = base }
  else id = base
  return type ":" id
}

# Rewrite every quoted meta-path token inside a linkage value to its typed form;
# tokens already typed (or pointing at an unmapped directory) are left as-is, the
# latter setting UNMAPPED_PATH for the caller to DIVERGE.
function normalize_paths(val,   out, rest, pre, tok, path, typed) {
  out = ""; rest = val
  while (match(rest, /"meta\/[^"]*\.md"/)) {
    pre = substr(rest, 1, RSTART - 1)
    tok = substr(rest, RSTART, RLENGTH)
    path = tok; gsub(/"/, "", path)
    typed = path_to_typed(path)
    if (typed != "") out = out pre "\"" typed "\""
    else { out = out pre tok; UNMAPPED_PATH = 1 }
    rest = substr(rest, RSTART + RLENGTH)
  }
  return out rest
}

function in_vocab(s,   n, a, i) {
  n = split(statusvocab, a, "|")
  for (i = 1; i <= n; i++) if (trim(a[i]) == s) return 1
  return 0
}
function map_status(s,   n, a, i, kv) {
  n = split(statusmap, a, " ")
  for (i = 1; i <= n; i++) {
    split(a[i], kv, "=")
    if (kv[1] == s) return kv[2]
  }
  return ""
}

# Is this an omit-when-empty key (drop when value is "" or [])? Every key
# except tags and the always-valued numeric/bool extras.
function omit_when_empty_key(k) {
  if (k == "tags") return 0
  return 1
}
function is_empty_val(v) { return (v == "" || v == "\"\"" || v == "[]") }

BEGIN {
  in_fm = 0; seen_open = 0
  date_value = ""        # normalised date, for seeding last_updated
  author_value = ""      # author, for seeding last_updated_by
  emitted_id = 0; emitted_revision = 0
  UNMAPPED_PATH = 0
}

# First fence opens the frontmatter region.
!seen_open && fm_is_fence($0) {
  seen_open = 1; in_fm = 1
  print; next
}

# Closing fence: emit any missing base fields, then the fence. The "hard" base
# fields a file may lack (title/author/date/revision) are derived caller-side
# (H1, VCS, filename) and passed in as *_default — the same derivation the
# fence-less backfill uses, applied here to fenced files missing them.
in_fm && fm_is_fence($0) {
  if (!has_type && type != "") print "type: " type
  if (!emitted_id && !has_id) {
    # Covers any type with neither an existing id: nor a legacy own-id key —
    # including a work-item/ADR missing its own-id key (id_from_stem is then the
    # bare number / ADR-NNNN, computed caller-side).
    if (id_from_stem != "") print "id: \"" id_from_stem "\""
    else print "0007-REFUSE: " file " — no id and no derivable filename stem" > "/dev/stderr"
  }
  if (!has_title && title_default != "") print "title: \"" title_default "\""
  if (!has_date && date_default != "") {
    print "date: \"" date_default "\""
    if (date_value == "") date_value = "\"" date_default "\""
  }
  if (!has_author && author_default != "") print "author: " author_default
  if (!has_tags) print "tags: []"
  if (!has_schema) print "schema_version: 1"
  if (!has_lu) {
    seed = (date_value != "" ? date_value : (date_default != "" ? "\"" date_default "\"" : ""))
    if (seed != "") print "last_updated: " seed
  }
  if (!has_lub) {
    seedby = (author_value != "" ? author_value : author_default)
    if (seedby != "") print "last_updated_by: " seedby
  }
  if (anchored == 1) {
    if (!emitted_revision && !has_revision) {
      if (revision_default != "") print "revision: \"" revision_default "\""
      else print "0007-DIVERGE[author-lookup-failed]: " file " — anchored type missing revision (no git_commit, no VCS revision)" > "/dev/stderr"
    }
    if (!has_repository) {
      if (repo_name != "") print "repository: \"" repo_name "\""
      else print "0007-DIVERGE[nonconforming-base-field]: " file " — anchored type missing repository" > "/dev/stderr"
    }
  }
  in_fm = 0
  print; next
}

# Frontmatter key lines.
in_fm && /^[A-Za-z_][A-Za-z0-9_]*:/ {
  key = $0; sub(/:.*/, "", key)
  val = $0; sub(/^[A-Za-z_][A-Za-z0-9_]*:[ \t]*/, "", val); val = trim(val)

  # Drop legacy provenance keys (git_commit migrates to revision; branch goes).
  if (key == "git_commit") {
    if (!has_revision && val != "") { print "revision: " fm_normalise_value(val); emitted_revision = 1 }
    next
  }
  if (key == "branch") { next }

  # Producer rename.
  if (key == "skill") { print "producer: " val; next }

  # Own-identity → quoted id.
  if (own_id_key != "" && key == own_id_key) {
    if (fm_refuses(val)) { print $0; print "0007-REFUSE: " file " — refused " key " (unsafe value shape)" > "/dev/stderr"; next }
    print "id: " fm_normalise_value(val); emitted_id = 1; next
  }
  if (key == "id") {
    if (fm_refuses(val)) { print $0; print "0007-REFUSE: " file " — refused id (unsafe value shape)" > "/dev/stderr"; next }
    print "id: " fm_normalise_value(val); emitted_id = 1; next
  }

  # Dates → full ISO.
  if (key == "date" || key == "last_updated") {
    nd = norm_date(val)
    if (nd == "") {
      print $0
      print "0007-DIVERGE[nonconforming-base-field]: " file " — " key " not a normalisable ISO date: " val > "/dev/stderr"
    } else {
      print key ": " nd
      if (key == "date") date_value = nd
    }
    next
  }

  # Capture author (for seeding last_updated_by); pass through.
  if (key == "author") { author_value = val; print $0; next }

  # Status normalisation.
  if (key == "status") {
    inner = fm_semantic_inner(val)
    if (inner == "") { next }                 # empty status → omit (absent permitted)
    if (in_vocab(inner)) { print $0; next }    # already canonical
    mapped = map_status(inner)
    if (mapped != "") { print "status: " mapped; next }
    print $0
    print "0007-DIVERGE[unmapped-status]: " file " — status '" inner "' not in vocab and unmapped" > "/dev/stderr"
    next
  }

  # Typed-linkage keys: drop when empty (omit-when-empty); otherwise normalise
  # any pre-existing path-shape value (e.g. "meta/work/0030-foo.md") to its typed
  # "doc-type:id" form. Only fence-region values are touched. An unmapped
  # directory is left untouched and counted as a DIVERGE.
  if (is_linkage_key(key)) {
    if (is_empty_val(val)) { next }
    UNMAPPED_PATH = 0
    newval = normalize_paths(val)
    print key ": " newval
    if (UNMAPPED_PATH)
      print "0007-DIVERGE[unmapped-dir]: " file " — " key " has a path under an unmapped directory: " val > "/dev/stderr"
    next
  }

  # Omit-when-empty: drop empty placeholders (any key except tags).
  if (omit_when_empty_key(key) && is_empty_val(val)) { next }

  print $0; next
}

{ print }

END {
  if (!seen_open)
    print "0007-MALFORMED: " file " — no frontmatter fence detected" > "/dev/stderr"
}
