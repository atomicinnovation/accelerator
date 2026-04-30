# jira-adf-render.jq — ADF JSON → Markdown renderer
# Called via: jq -r -f jira-adf-render.jq <adf.json
#
# Mark precedence (innermost → outermost applied): code → em → strong → link
# Each render_block returns a string WITHOUT a trailing newline.
# Blocks are joined with "\n\n" (one blank line between them).
# jq -r adds exactly one trailing newline to the final output.
# Empty doc uses `empty` so jq -r outputs nothing.

def render_inline:
  . as $node |
  if $node.type == "text" then
    ($node.marks // []) as $marks |
    $node.text |
    (if ($marks | any(.type == "code"))   then "`"  + . + "`"  else . end) |
    (if ($marks | any(.type == "em"))     then "*"  + . + "*"  else . end) |
    (if ($marks | any(.type == "strong")) then "**" + . + "**" else . end) |
    . as $inner |
    if ($marks | any(.type == "link")) then
      ($marks | map(select(.type == "link")) | .[0].attrs.href) as $href |
      "[" + $inner + "](" + $href + ")"
    else $inner end
  elif $node.type == "hardBreak" then "  \n"
  else "[unsupported ADF inline: \($node.type)]"
  end;

def render_inlines:
  if (. == null or length == 0) then ""
  else (map(render_inline) | join(""))
  end;

def render_listitem_text:
  .content[0].content // [] | render_inlines;

# Returns block content WITHOUT trailing newline; caller joins with "\n\n"
def render_block:
  if .type == "paragraph" then
    (.content // [] | render_inlines)
  elif .type == "heading" then
    (.attrs.level as $n | [range($n)] | map("#") | join("")) + " " +
    (.content // [] | render_inlines)
  elif .type == "bulletList" then
    (.content | map("- " + render_listitem_text) | join("\n"))
  elif .type == "orderedList" then
    (.attrs.order // 1) as $start |
    (.content | to_entries |
      map((.key + $start | tostring) + ". " + (.value | render_listitem_text)) |
      join("\n"))
  elif .type == "taskList" then
    (.content | map(
      (if .attrs.state == "DONE" then "- [x] " else "- [ ] " end) +
      (.content // [] | render_inlines)
    ) | join("\n"))
  elif .type == "codeBlock" then
    "```" + (.attrs.language // "") + "\n" +
    (.content[0].text // "") + "\n" +
    "```"
  else
    "[unsupported ADF node: \(.type)]"
  end;

# Main
if (.type // "") != "doc" then error("E_BAD_JSON")
else
  (.content // []) as $blocks |
  if ($blocks | length) == 0 then empty
  else $blocks | map(render_block) | join("\n\n")
  end
end
