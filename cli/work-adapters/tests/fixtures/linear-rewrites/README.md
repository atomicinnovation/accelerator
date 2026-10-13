# Linear rewrite capture

`pushed.md` was created as a scratch Linear issue (`issueCreate`, via
`accelerator linear create --body-file`) on 2026-10-13, read back with
`accelerator linear show` into `read-back.json`, then deleted. The read-back
went through the allowlist in `../work-item-sync-recovery/README.md`.

Each `##` section is one fragment. Line ranges are 1-based: in `pushed.md`,
and in the string at `.data.issue.description` of `read-back.json`. Every
heading survives unchanged, so the sections pair up by name.

The leading blank line of `pushed.md` does not survive: the description opens
with the first heading. Re-capturing the same body gave the same read-back
three times, apart from a trailing newline.

## Equivalence-table rows

| Row | Pushed | Read back | Container | Observed rewrite |
|---|---|---|---|---|
| `R01` Escaping | 4 | 3 | top level | `~550` → `\~550` |
| `R02` Joined continuation lines | 8-10 | 7-9 | top level | none: wrapped prose keeps its line breaks |
| `R03` Per-line bold | 14-15 | 13-14 | top level | `**a⏎b**` → `**a**⏎**b**` |
| `R04` Bold hoisted off code | 19 | 18 | top level | ``**run `cargo test`**`` → ``**run** `cargo test` `` |
| `R05` Link destinations | 23 | 22 | top level | `[t](u)` → `[t](<u>)` |
| `R06` Autolinked URLs | 27 | 26 | top level | `u` → `[u](<u>)` |
| `R07` Bullet markers | 31-32 | 30-31 | top level | `- a` → `* a` |
| `R08` Task-marker case | 36-37 | 35-36 | top level | `[x]` → `[X]`; task items keep `-` |
| `R09` Table delimiters | 41-43 | 40-42 | top level | `\|---\|---\|` → `\| -- \| -- \|` |
| `R10` Inserted blank lines | 47-48 | 46-47 | top level | none between items; only `-` → `*` |
| `R11` Wrapped digit lines | 52-53 | 51-52 | top level | `0160.` → `0160\.` (escaping only) |
| `R12` Dotted tokens autolinked | 57 | 56 | top level | `crates.io` → `[crates.io](<http://crates.io>)` |
| `R13` Code-span indentation | 61-62 | 60 | list item, content indent 2 | lines joined; the span keeps one space |

## Variants

| Fragment | Pushed | Read back | Container | Observed rewrite |
|---|---|---|---|---|
| `X01` Code-span line ending | 66-67 | 64 | top level | `` `a⏎b` `` → `` `a b` ``, lines joined |
| `X02` Blockquote | 71-73 | 68-70 | blockquote | escaping and per-line bold, as at top level |
| `X03` Paragraph then list | 77-79 | 74-77 | top level | `p⏎- a` → `p⏎⏎* a` |
| `X04` Digit line, no indent | 83-84 | 81-82 | top level | `0160.` → `0160\.` |
| `X05` Digit line in an item | 88-89 | 86-87 | list item, content indent 2 | `0160.` → `0160\.` |
| `X06` Digit line past the indent | 93-94 | 91-92 | list item, content indent 2 | `0160.` → `0160\.`; indentation kept |
| `X07` Code span, indented | 98-99 | 96 | top level | `` `a⏎  b` `` → `` `a   b` `` |
| `X08` Code span past a task indent | 103-104 | 100 | task item, content indent 2 | `` `a⏎      b` `` → `` `a     b` `` |
| `X09` Lone digit line | 108-113 | 104-109 | list item, then top level | in the item `⏎  0160.` → `⏎  160. `; at top level unchanged |

## Against 0296's Equivalence table

- Ten rows reproduce as tabulated: `R01`, `R03`-`R09`, `R12`, and
  code-span indentation (`X07`, `X08`).
- Joined continuation lines occur only where a line break falls inside a code
  span (`X01`, `X07`, `X08`); wrapped prose is left alone (`R02`).
- Inserted blank lines fall between a paragraph and the list that follows it
  (`X03`), never between items (`R10`).
- Wrapped digit lines become an ordered-list marker only when the continuation
  line inside a list item holds the number alone (`X09`); with text after it
  the number is escaped (`R11`, `X04`-`X06`).
