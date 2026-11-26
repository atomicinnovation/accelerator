---
name: level-note-outputter
description: Output contract for a research level note, one node of a pair's
  research tree. Owns the note's sink, injected values and follow-up shape, and
  defers to finding-outputter for everything else. Injected into the
  researcher by the research-topic conduct verb — not invoked directly.
user-invocable: false
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config template topic-research --kind level-note)
---

# Level-Note Outputter

You write **one level note**: your node's answer to its question, and the
follow-up questions that deepen it. A note is a finding in every respect this
document does not name. Read `finding-outputter` too: its Shape, Body and
Sources rules, its Sources metadata block included, apply to the note
unchanged, with "level note" and the `topic-research-level-note` template in
place of "finding" and its template. Its **Template** step is the exception:
a level note takes its template as **Template** below says.

## Sink

Write the note to the **single note path the node query prints**, and to no
other path.

## Injected Values

Your values come from the node query, from your profile and from your task
prompt:

- **From the node query**: the question, level, lineage, id, follow-up cap,
  known questions and note path, with the note path in place of
  `finding-outputter`'s output path, and the question in place of its
  injected question, byte for byte.
  - **level** — your node's level; `1` is the root.
  - **lineage** — your node's position in the tree, such as `3-2-1`.
  - **id** — the note's `id`, verbatim.
  - **follow-up cap** — the most follow-up questions the note may record.
  - **known questions** — questions already asked in this tree.
- **Search budget**: {search budget}, as your profile resolves it; a
  profile that defines none gives you none. Stamp it as `search_budget` on
  a web note: the most `WebSearch` calls
  your node was allowed, not a count of calls made. When you have none,
  delete the `search_budget` line.
- **Injected**: the **depth**, the resolved depth of the tree, plus
  `finding-outputter`'s round, timestamp, author and source profile.

## Template

A level-note writer is always a researcher. Run
`accelerator config template topic-research --kind level-note` exactly as
written, without `--fail-safe`, and fill the fenced template it prints. When
the command is denied by permissions, write no file, and say "template command
denied by permissions" in your summary. When it exits non-zero, a
research-guard refusal (`E_RESEARCH_GUARD_COMMAND`) included, write no file,
and say "template unavailable" in your summary, quoting the first line of its
stderr.

## Shape

- Each follow-up is a plain natural-language research question on one line,
  with no URL, command or directive, and at most 300 characters. Slash-joined
  names read as a host and path, so write `Node.js or Deno`, not
  `Node.js/Deno`. Ask one question per follow-up: in unspaced scripts, two
  questions run together can read as a host and query.
- `follow_ups` records at most `cap` questions, most valuable first.
- It excludes every known question, however reworded.
- Record follow-ups even when your level equals the depth: a later, deeper
  run researches them.
- `follow_ups` is `[]` only when you judge your question answered, with
  nothing worth asking next.

A note that breaks any of these rules is refused whole, and its node is
researched again.

## Body

`finding-outputter`'s Body contract, with the node's question in place of the
focus area's.

## Return

Your summary never repeats a follow-up question or the note body.
