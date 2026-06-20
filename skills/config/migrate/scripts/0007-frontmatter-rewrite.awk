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
  # Last resort: a value that merely STARTS with a YYYY-MM-DD date but is
  # otherwise non-ISO (space-separated time, TZ abbreviation like GMT/CEST) —
  # keep the date, drop the unrepresentable time, normalise to midnight UTC.
  # Deterministic, so applied mechanically (the lost time is recorded in the
  # dogfood gap-fix log rather than DIVERGEd).
  if (inner ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9][^0-9]/)
    return "\"" substr(inner, 1, 10) "T00:00:00+00:00\""
  return ""   # genuinely non-normalisable
}

# Legacy artifact-type aliases → canonical ADR-0033 type. Work-item kind-values
# (story/bug/…) are NOT handled here — migration 0005 owns those.
function canonical_type(t) {
  if (t == "validation") return "plan-validation"
  return t
}

# Linkage vocabulary membership (keys whose values are typed references).
function is_linkage_key(k) {
  return (k == "parent" || k == "target" || k == "source" || k == "superseded_by" ||
    k == "supersedes" || k == "blocks" || k == "blocked_by" ||
    k == "derived_from" || k == "relates_to")
}

# Map a project-relative meta path to a typed "doc-type:id" reference, per the
# target doc-type's identity convention (work-item/ADR → bare number / ADR-NNNN;
# every other type → full filename stem). Returns "" for a path under no
# configured doc-type directory so the caller can DIVERGE and leave it untouched.
#
# Directory→type classification is config-aware: it matches against the injected
# DT_DIR[]/DT_TYPE[] table (parsed in BEGIN from the -v doc_type_table channel,
# the SAME resolved allowlist the shell-side infer_type_from_path consumes), most-
# specific (longest configured dir) wins. This is a THIRD encoding of the
# directory→type fact for a DIFFERENT input (a referenced meta-path inside a
# linkage value, not the current file), so it cannot consume the file-level
# `-v type` channel. It MUST stay aligned with the shared helper — a
# test-migrate-0007.sh fixture asserts representative arms in step with it.
# The id-derivation halves (work-item stem trim, ADR prefix) are preserved here.
function path_to_typed(p,   i, dir, blen, btype, base, id) {
  blen = -1; btype = ""
  for (i = 1; i <= DT_COUNT; i++) {
    dir = DT_DIR[i]
    if (dir == "") continue
    # p is project-relative and begins with `dir/` when in that doc-type dir.
    # Literal prefix compare (no regex) — config dirs match as-is, longest wins.
    if (substr(p, 1, length(dir) + 1) == dir "/") {
      if (length(dir) > blen) { blen = length(dir); btype = DT_TYPE[i] }
    }
  }
  if (btype == "") return ""
  base = p; sub(/.*\//, "", base); sub(/\.md$/, "", base)
  if (btype == "work-item") { id = base; sub(/-.*/, "", id) }
  else if (btype == "adr") { if (match(base, /^ADR-[0-9]+/)) id = substr(base, RSTART, RLENGTH); else id = base }
  else id = base
  return btype ":" id
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

# Deterministic target doc-type for a bare-number value on (source_type, key)
# per ADR-0034's table — only the single-candidate pairings. Multi-candidate
# pairings (work-item derived_from → note|work-item; plan derived_from →
# codebase-research|issue-research) and loose keys return "" → routed to the
# interactive hook (the §2 side-channel) rather than guessed.
function bare_target_type(t, k) {
  if (k == "parent") { if (t == "work-item" || t == "plan") return "work-item"; return "" }
  if (k == "blocks" || k == "blocked_by") { if (t == "work-item") return "work-item"; return "" }
  if (k == "supersedes" || k == "superseded_by") { if (t == "adr") return "adr"; return "" }
  if (k == "target") {
    if (t == "plan-review" || t == "plan-validation") return "plan"
    if (t == "work-item-review") return "work-item"
    return ""
  }
  if (k == "source") { if (t == "work-item") return "note"; return "" }
  return ""   # relates_to / derived_from etc. — multi-candidate or loose
}

# Convert a bare-number linkage value ("NNNN") to "doc-type:NNNN" where the
# (source_type, key) pairing is single-candidate; an already-typed token (the
# `"` is not immediately followed by a digit) is left untouched, and a
# multi-candidate bare number is left and flagged (BARE_AMBIG) for the hook.
function normalize_bare(val, t, k,   out, rest, pre, tok, num, tt) {
  out = ""; rest = val
  while (match(rest, /"[0-9]+"/)) {
    pre = substr(rest, 1, RSTART - 1)
    tok = substr(rest, RSTART, RLENGTH)
    num = tok; gsub(/"/, "", num)
    tt = bare_target_type(t, k)
    if (tt != "") out = out pre "\"" tt ":" num "\""
    else { out = out pre tok; BARE_AMBIG = 1 }
    rest = substr(rest, RSTART + RLENGTH)
  }
  return out rest
}

# Coerce non-canonical PR-reference tokens — "PR #N", "PR#N", "pr #N",
# "PR-N"/"pr-N", and bare "#N" — to the canonical "pr:N". POSIX awk has no inline
# case-insensitive flag, hence the explicit [Pp][Rr] class. Idempotent: a
# rewritten "pr:N" has ':' immediately after pr, so the [ -]?#? tail fails to
# re-match, and an already-typed "plan:…"/"pr:…" ref is left untouched.
function normalize_pr_ref(val,   out, rest, pre, tok, num) {
  out = ""; rest = val
  while (match(rest, /"([Pp][Rr][ -]?#?|#)[0-9]+"/)) {
    pre = substr(rest, 1, RSTART - 1)
    tok = substr(rest, RSTART, RLENGTH)
    num = tok; gsub(/[^0-9]/, "", num)
    out = out pre "\"pr:" num "\""
    rest = substr(rest, RSTART + RLENGTH)
  }
  return out rest
}

function in_vocab(s,   n, a, i) {
  n = split(statusvocab, a, "|")
  for (i = 1; i <= n; i++) if (trim(a[i]) == s) return 1
  return 0
}

# Schema-driven forbidden own-id keys (space-separated, from TSV col 6).
function is_forbidden(k,   n, a, i) {
  n = split(forbidden, a, " ")
  for (i = 1; i <= n; i++) if (trim(a[i]) == k) return 1
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

function diverge_backfill(k) {
  print "0007-DIVERGE[backfilled-extra]: " file " — " k " backfilled with sentinel; review manually" > "/dev/stderr"
}

# Parse "name=value\037name=value…" (built shell-side in rewrite_file) and emit
# one frontmatter line per record. Pure/callable so a BEGIN{} probe can exercise
# the empty / single-record / =-in-value cases. Split on the octal Unit Separator
# (== the shell builder's $'\x1F' — same byte, two encodings); index() splits each
# record on the FIRST `=` so a value may itself contain `=`.
#
# The list-vs-scalar cardinality is HARD-CODED here (lenses is the sole list
# extra today; the TSV carries no cardinality column, so the schema-column guard
# cannot pin it) — a future list-valued required extra needs a branch here. The
# bare-print branch covers the BARE TYPED scalars: pr_number/review_number (bare
# strings) plus the numeric/boolean typed defaults sequence/review_pass/
# screenshots_incomplete (which must stay YAML numbers/booleans, not be quoted by
# fm_normalise_value); every other scalar (topic, the unknown string sentinels)
# goes through the quoting else-branch.
function emit_backfill_extras(packed,   nbf, bfa, bi, eq, bk, bv) {
  if (packed == "") return
  nbf = split(packed, bfa, "\037")
  for (bi = 1; bi <= nbf; bi++) {
    eq = index(bfa[bi], "=")
    if (eq == 0) continue # malformed record → skip
    bk = substr(bfa[bi], 1, eq - 1)
    bv = substr(bfa[bi], eq + 1)
    if (bk == "lenses") { print "lenses: [\"" bv "\"]"; diverge_backfill(bk) }
    else if (bk == "verdict") { print "verdict: " fm_normalise_value(bv); diverge_backfill(bk) }
    else if (bk == "pr_number" || bk == "review_number" ||
             bk == "sequence" || bk == "review_pass" ||
             bk == "screenshots_incomplete") print bk ": " bv
    else print bk ": " fm_normalise_value(bv) # topic and any future scalar extra
  }
}

BEGIN {
  in_fm = 0; seen_open = 0
  date_value = ""        # normalised date, for seeding last_updated
  author_value = ""      # author, for seeding last_updated_by
  emitted_id = 0; emitted_revision = 0
  emitted_title = 0     # set when a pr_title value is folded into title:
  UNMAPPED_PATH = 0
  # Parse the injected doc-type table for path_to_typed: type<TAB>dir records
  # joined by the ASCII Record Separator 0x1E (octal \036). A newline record
  # separator is unusable here — the one-true-awk (macOS) rejects a newline in a
  # -v value ("newline in string"); 0x1E cannot occur in a type name or path and
  # passes through -v cleanly. POSIX/BWK awk — single-literal-character
  # separators only (no gensub/length(array)/asort).
  DT_COUNT = 0
  dt_nrows = split(doc_type_table, dt_rows, "\036")
  for (dt_i = 1; dt_i <= dt_nrows; dt_i++) {
    if (dt_rows[dt_i] == "") continue
    split(dt_rows[dt_i], dt_kv, "\t")
    if (dt_kv[1] == "" || dt_kv[2] == "") continue
    DT_COUNT++
    DT_TYPE[DT_COUNT] = dt_kv[1]
    DT_DIR[DT_COUNT] = dt_kv[2]
  }
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
  if (!has_title && !emitted_title && title_default != "") print "title: \"" title_default "\""
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
  # Work-item priority is an always-valued extra; default it to the template's
  # `medium` when absent (a deliberate decision recorded in the dogfood log).
  if (type == "work-item" && !has_priority) print "priority: medium"
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
  # Backfill required type-extras the file lacked (topic/pr_number/review_number/
  # verdict/lenses), computed caller-side as a packed name=value channel.
  emit_backfill_extras(backfill_extras)
  in_fm = 0
  print; next
}

# Frontmatter key lines.
in_fm && /^[A-Za-z_][A-Za-z0-9_]*:/ {
  key = $0; sub(/:.*/, "", key)
  val = $0; sub(/^[A-Za-z_][A-Za-z0-9_]*:[ \t]*/, "", val); val = trim(val)

  # Canonicalise a present legacy artifact-type alias (e.g. validation →
  # plan-validation). A conforming type: passes through unchanged. An EMPTY
  # type: value (the typeless meta/prs/ shape) is dropped so the closing-fence
  # backfill emits the path-inferred `type` instead of leaving a duplicate.
  if (key == "type") {
    if (is_empty_val(val)) next
    print "type: " canonical_type(val); next
  }

  # Drop legacy provenance keys (git_commit migrates to revision; branch goes).
  if (key == "git_commit") {
    if (!has_revision && val != "") { print "revision: " fm_normalise_value(val); emitted_revision = 1 }
    next
  }
  if (key == "branch") { next }

  # Obsolete legacy keys (cross-cutting, any type) — migrated out by 0001 (in
  # meta/tickets/) and dropped everywhere else here. A non-empty value is logged
  # so a real external-tracker reference is recoverable via the breadcrumb + VCS.
  if (key == "ticket" || key == "ticket_id") {
    if (!is_empty_val(val))
      print "0007-DIVERGE[dropped-legacy-key]: " file " — dropped " key ": " val > "/dev/stderr"
    next
  }

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

  # Forbidden own-id keys (schema TSV col 6): drop. pr_title additionally folds
  # into title: ONLY when the file has no title: AND the value is non-empty;
  # otherwise it is discarded — and a non-empty discard (a real pr_title lost
  # because a differing title: already exists) is surfaced as a breadcrumb so it
  # is auditable rather than silently destroyed. An empty forbidden key drops
  # cleanly (no title: "" fold, no breadcrumb): the stem-derived title_default
  # then supplies a non-empty title:. Placed before the linkage and
  # omit-when-empty arms so the fold runs before omit-when-empty could drop it.
  if (is_forbidden(key)) {
    if (key == "pr_title" && !has_title && !emitted_title && !is_empty_val(val)) {
      print "title: " fm_normalise_value(val); emitted_title = 1
    } else if (key == "pr_title" && !is_empty_val(val)) {
      print "0007-DIVERGE[discarded-key]: " file " — pr_title discarded (title present): " val > "/dev/stderr"
    }
    next
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
    UNMAPPED_PATH = 0; BARE_AMBIG = 0
    newval = normalize_bare(normalize_pr_ref(normalize_paths(val)), type, key)
    print key ": " newval
    if (UNMAPPED_PATH)
      print "0007-DIVERGE[unmapped-dir]: " file " — " key " has a path under an unmapped directory: " val > "/dev/stderr"
    if (BARE_AMBIG)
      print "0007-DIVERGE[parent-ambiguous]: " file " — " key " has a multi-candidate bare-number target (route to hook): " val > "/dev/stderr"
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
