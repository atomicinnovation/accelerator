# frontmatter-frag.awk — shared frontmatter awk primitives (story 0070).
#
# Include-only fragment: defines FUNCTIONS ONLY (no BEGIN/END, no top-level
# pattern/action rules). It is never executed directly — it is concatenated
# ahead of a body program that owns all state and rules:
#
#   awk -f frontmatter-frag.awk -f body.awk input
#
# It is the forward home for the value-quoting / refusal primitives first
# written inline in migration 0006 (which is frozen as applied); migration 0007
# and any later frontmatter-rewriting migration source these instead of
# re-deriving them, so the quote/refuse contract has one definition. A parity
# fixture in test-migrate-0007.sh pins byte-for-byte equivalence with 0006.

# fm_is_fence(line) — 1 if the line is a strict frontmatter fence `---`
# (no trailing whitespace), else 0. The strict form is deliberate: a fence
# line with trailing whitespace routes a file to the by-location backfill,
# never to the legacy-key rewrite.
function fm_is_fence(line) {
  return (line == "---")
}

# fm_normalise_value(line) — return the value already double-quoted: a
# double-quoted value is returned verbatim; a single-quoted value is
# re-quoted as double (escaping backslash and double-quote in its inner);
# a bare value is wrapped in double quotes.
function fm_normalise_value(line,    inner) {
  if (line ~ /^".*"$/) return line
  if (line ~ /^'.*'$/) {
    inner = substr(line, 2, length(line) - 2)
    gsub(/\\/, "\\\\", inner)
    gsub(/"/, "\\\"", inner)
    return "\"" inner "\""
  }
  return "\"" line "\""
}

# fm_semantic_inner(line) — the value's semantic content with one layer of
# surrounding quotes removed (double or single); a bare value is returned
# unchanged. Used to compare two differently-quoted values for equality.
function fm_semantic_inner(line,    inner) {
  if (line ~ /^".*"$/) return substr(line, 2, length(line) - 2)
  if (line ~ /^'.*'$/) return substr(line, 2, length(line) - 2)
  return line
}

# fm_refuses(line) — 1 if a bare (unquoted) value carries a shape the
# migration must refuse to quote mechanically (an embedded `#` comment
# introducer or a stray `"`), else 0. An already-quoted value never refuses.
function fm_refuses(line) {
  if (line ~ /^".*"$/ || line ~ /^'.*'$/) return 0
  if (line ~ /#/) return 1
  if (line ~ /"/) return 1
  return 0
}
