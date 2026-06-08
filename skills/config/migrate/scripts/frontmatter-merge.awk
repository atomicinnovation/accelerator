# frontmatter-merge.awk — merge one typed-linkage value into an artifact's
# frontmatter block (story 0070, interactive apply path). Fence-aware and
# idempotent: re-merging an already-present value reproduces the same bytes, so
# the cmp -s gate and interactive resume stay no-ops.
#
# -v lkey   the linkage key (parent / blocks / derived_from / …)
# -v lval   the typed "doc-type:id" target value
# -v card   "single" or "list"
#
# A single key is set (replacing any present value); a list key gains lval if
# absent (existing members preserved, in order). Omit-when-empty upstream means
# an existing list is always non-empty, so no empty placeholder is produced.

BEGIN { in_fm = 0; seen_open = 0; done_key = 0 }

!seen_open && $0 == "---" { seen_open = 1; in_fm = 1; print; next }

in_fm && $0 == "---" {
  if (!done_key) {
    if (card == "single") print lkey ": \"" lval "\""
    else print lkey ": [\"" lval "\"]"
    done_key = 1
  }
  in_fm = 0; print; next
}

in_fm && $0 ~ ("^" lkey ":") {
  done_key = 1
  if (card == "single") { print lkey ": \"" lval "\""; next }
  v = $0
  sub("^" lkey ":[ \t]*", "", v)
  sub(/^\[/, "", v); sub(/\][ \t]*$/, "", v)
  cnt = split(v, arr, ",")
  out = ""; present = 0
  for (i = 1; i <= cnt; i++) {
    t = arr[i]
    gsub(/^[ \t"]+/, "", t); gsub(/[ \t"]+$/, "", t)
    if (t == "") continue
    if (t == lval) present = 1
    out = (out == "" ? "\"" t "\"" : out ", \"" t "\"")
  }
  if (!present) out = (out == "" ? "\"" lval "\"" : out ", \"" lval "\"")
  print lkey ": [" out "]"
  next
}

{ print }
