# jira-md-assemble.jq — Assembles an ADF document from the block-tokeniser record stream.
# Read via: jq -R -s -f jira-md-assemble.jq --arg uuid_seed "$JIRA_ADF_LOCALID_SEED"
#
# Input: raw binary record stream (fields separated by , records by )
# Args:
#   $uuid_seed  — empty for live UUIDs (generated externally via env), non-empty for
#                 deterministic UUIDs (00000000-0000-4000-8000-00000000000N)
#
# For deterministic mode, $uuid_seed must be the string "1" (or any non-empty value);
# the assembler uses an internal counter starting at 1.

# ---------------------------------------------------------------------------
# Inline parser
# ---------------------------------------------------------------------------

# Inline token scan pattern (POSIX ERE, priority order):
#   1. code span
#   2. bold+italic  ***...***
#   3. bold         **...**
#   4. italic       *...*
#   5. link         [text](url)
#   6. non-link bracket expression  [text]
#   7. plain text run (no special chars)
#   8. catch-all (single char)
def INLINE_RE:
  "`[^`]+`|\\*\\*\\*[^*]+\\*\\*\\*|\\*\\*[^*]+\\*\\*|\\*[^*]+\\*|\\[[^\\]]*\\]\\([^)]+\\)|\\[[^\\]]*\\]|[^`*\\[]+|.";

def parse_inlines:
  . as $text |
  if ($text | length) == 0 then []
  else
    [ $text | scan(INLINE_RE) |
      . as $tok |
      if ($tok | startswith("`")) then
        {"type": "text", "text": ($tok[1:-1]), "marks": [{"type": "code"}]}
      elif ($tok | startswith("***")) then
        {"type": "text", "text": ($tok[3:-3]), "marks": [{"type": "strong"}, {"type": "em"}]}
      elif ($tok | startswith("**")) then
        {"type": "text", "text": ($tok[2:-2]), "marks": [{"type": "strong"}]}
      elif ($tok | startswith("*")) then
        {"type": "text", "text": ($tok[1:-1]), "marks": [{"type": "em"}]}
      elif ($tok | test("^\\[") and ($tok | test("\\]\\("))) then
        # Link: parse text and href
        ($tok | capture("^\\[(?<lt>[^\\]]+)\\]\\((?<href>[^)]+)\\)")) |
        . as {lt: $lt, href: $href} |
        ($lt | parse_inlines) |
        map(. + {"marks": ((.marks // []) + [{"type": "link", "attrs": {"href": $href}}])}) |
        .[]
      else
        {"type": "text", "text": $tok}
      end
    ]
  end;

# ---------------------------------------------------------------------------
# UUID helpers
# ---------------------------------------------------------------------------

def make_det_uuid(n):
  "00000000-0000-4000-8000-" + ("000000000000" + (n | tostring))[-12:];

# ---------------------------------------------------------------------------
# State-machine helpers
# ---------------------------------------------------------------------------

def flush_para:
  if .para == null then .
  else
    .nodes += [{"type": "paragraph", "content": .para}] |
    .para = null | .prev_hbr = false
  end;

def flush_list:
  if .list_type == null then .
  else
    if .list_type == "bullet" then
      .nodes += [{"type": "bulletList", "content": .list_items}]
    elif .list_type == "ordered" then
      .nodes += [{"type": "orderedList", "attrs": {"order": 1}, "content": .list_items}]
    elif .list_type == "task" then
      (.uuid_n + 1) as $n |
      .uuid_n = $n |
      (if ($ENV.JIRA_ADF_LOCALID_SEED // "") != "" then make_det_uuid($n) else $n | tostring end) as $lid |
      .nodes += [{"type": "taskList", "attrs": {"localId": $lid}, "content": .list_items}]
    else .
    end |
    .list_type = null | .list_items = []
  end;

def flush_code:
  if .code_open then
    .nodes += [{"type": "codeBlock", "attrs": {"language": .code_lang},
                "content": [{"type": "text", "text": (.code_lines | join("\n"))}]}] |
    .code_open = false | .code_lang = "" | .code_lines = []
  else .
  end;

def flush_all: flush_para | flush_list | flush_code;

# ---------------------------------------------------------------------------
# Record processing
# ---------------------------------------------------------------------------

def process(fields):
  fields[0] as $type |
  if $type == "P" then
    fields[1] as $text |
    ($text | parse_inlines) as $inlines |
    if .prev_hbr then
      .para += $inlines | .prev_hbr = false
    else
      flush_para | flush_list |
      .para = $inlines | .prev_hbr = false
    end

  elif $type == "HBR" then
    if .para != null then
      .para += [{"type": "hardBreak"}] | .prev_hbr = true
    else .
    end

  elif ($type | startswith("H")) and ($type | length == 2) and ($type[1:2] | test("^[1-6]$")) then
    flush_para | flush_list |
    ($type[1:2] | tonumber) as $level |
    (fields[1] | parse_inlines) as $inlines |
    .nodes += [{"type": "heading", "attrs": {"level": $level}, "content": $inlines}]

  elif $type == "BUL" then
    flush_para |
    (if .list_type != "bullet" then flush_list | .list_type = "bullet" else . end) |
    (fields[1] | parse_inlines) as $inlines |
    .list_items += [{"type": "listItem",
                     "content": [{"type": "paragraph", "content": $inlines}]}]

  elif $type == "ORD" then
    flush_para |
    (if .list_type != "ordered" then flush_list | .list_type = "ordered" else . end) |
    (fields[2] | parse_inlines) as $inlines |
    .list_items += [{"type": "listItem",
                     "content": [{"type": "paragraph", "content": $inlines}]}]

  elif $type == "TASK_TODO" or $type == "TASK_DONE" then
    flush_para |
    (if .list_type != "task" then flush_list | .list_type = "task" else . end) |
    (fields[1] | parse_inlines) as $inlines |
    (.uuid_n + 1) as $n |
    .uuid_n = $n |
    (if ($ENV.JIRA_ADF_LOCALID_SEED // "") != "" then make_det_uuid($n) else $n | tostring end) as $lid |
    ($type == "TASK_DONE") as $done |
    .list_items += [{"type": "taskItem",
                     "attrs": {"localId": $lid,
                               "state": (if $done then "DONE" else "TODO" end)},
                     "content": $inlines}]

  elif $type == "CODE_OPEN" then
    flush_para | flush_list |
    .code_open = true | .code_lang = fields[1] | .code_lines = []

  elif $type == "CODE_LINE" then
    if .code_open then .code_lines += [fields[1]] else . end

  elif $type == "CODE_CLOSE" then
    flush_code

  elif $type == "ERR" then
    error("E_TOKENISER: " + fields[1] + ": " + fields[2])

  else .
  end;

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

# Initial assembler state
{nodes: [], para: null, prev_hbr: false,
 list_type: null, list_items: [],
 code_open: false, code_lang: "", code_lines: [],
 uuid_n: 0} as $init |

# Parse the binary record stream
(split("") | map(select(length > 0)) | map(split(""))) as $records |

# Process all records
reduce $records[] as $fields ($init; process($fields)) |

# Flush any open structures
flush_all |

# Emit ADF document
{"version": 1, "type": "doc", "content": .nodes}
