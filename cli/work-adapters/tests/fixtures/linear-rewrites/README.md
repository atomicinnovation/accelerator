# Linear rewrite capture

`pushed.md` was created as a scratch Linear issue (`issueCreate`, via
`accelerator linear create --body-file`) in the `TES` team, read back with
`accelerator linear show` into `read-back.json`, then deleted. The read-back
went through the allowlist in `../work-item-sync-recovery/README.md`.
`R01`-`X09` were first captured on 2026-10-13. The current capture, `TES-32`
on 2026-11-24, added `X10`-`X25`, and `R01`-`X09` read back byte-identical to
the first.

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
| `X20` Lone parenthesised number | 157-159 | 153-155 | list item, content indent 2 | `⏎  0160)` → `⏎  160. `, an ordered list |
| `X24` Lone digit line in a blockquote | 179-180 | 175-176 | blockquote | none |

## Dotted tokens and boundaries

| Fragment | Pushed | Read back | Container | Observed rewrite |
|---|---|---|---|---|
| `X10` TLD file extensions | 117 | 113 | top level | `server.pid` → `[server.pid](<http://server.pid>)`; `foo.md` none |
| `X11` Line number | 121 | 117 | top level | `jql.rs:167` → `[jql.rs:167](<http://jql.rs:167>)` |
| `X12` Non-TLD extension | 125 | 121 | top level | none |
| `X13` Version, abbreviation | 129 | 125 | top level | none |
| `X14` Multi-part tokens | 133 | 129 | top level | `a.b.io` → `[a.b.io](<http://a.b.io>)`; `a.b.c` none |
| `X15` Mixed case | 137 | 133 | top level | `Crates.IO` → `[Crates.IO](<http://Crates.IO>)` |
| `X16` Sentence-final | 141 | 137 | top level | `crates.io.` → `[crates.io](<http://crates.io>).` |
| `X17` Bracketed | 145 | 141 | top level | `(crates.io)` → `([crates.io](<http://crates.io>))` |
| `X18` Path | 149 | 145 | top level | none |
| `X19` Email address | 153 | 149 | top level | `a@b.io` → `[a@b.io](<mailto:a@b.io>)` |
| `X21` Between inline tags | 163 | 159 | top level | none |
| `X22` Bold and heading | 167-169 | 163-165 | top level, heading | `**crates.io**` → `[**crates.io**](<http://crates.io>)`; heading `crates.io` → `[crates.io](<http://crates.io>)` |
| `X23` Table cell | 173-175 | 169-171 | table cell | `crates.io` → `[crates.io](<http://crates.io>)`; `\| - \|` → `\| -- \|` |
| `X25` Quoted scheme URLs | 184 | 180 | top level | each URL linked whole, `[u](<u>)`, quotes and `URL:` left outside |

Every link is `http` except `X19`'s, which is `mailto`. Each linked span is
the dotted token alone, with `X11`'s port, and without the sentence-final
`.`, the brackets of `X17` or the emphasis of `X22`, which wraps the link.

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
- Wrapped `<digits>)` becomes an ordered-list item as `<digits>.` does
  (`X20`), but a lone number in a blockquote is left alone (`X24`).
- Every token Linear linked in `X10`-`X19` ends in a label of the pinned
  IANA snapshot. The unlinked tokens ending in one, `foo.md` (bare `md`),
  `src/client.rs` (after `/`) and `X21`'s (touching an HTML tag), each fall
  in an over-escape class recorded in `../linear-autolink-probes/README.md`.
- Each linked span in `X10`-`X19` holds exactly the dotted token the rule
  finds in it: `jql.rs:167` with its port, `a.b.io` whole, and `b.io` within
  `X19`'s `mailto` link over `a@b.io`.

## Audit self-check codes

An audit of stored descriptions checks itself against these codes, each with
its stored line in `read-back.json`:

| Set | Codes |
|---|---|
| Linked dotted token | `R12` (56), `X10` (113), `X11` (117), `X14` (129), `X15` (133), `X16` (137), `X17` (141), `X19` (149), `X22` (163, 165), `X23` (171) |
| Stray list line | `X09` (105), `X20` (154) |

An escaped read-back must match neither set at these codes, each with its
stored line in `escaped/read-back.json`:

| Set | Codes |
|---|---|
| Neither | `R11` (52), `R12` (56), `X04` (82), `X05` (87), `X06` (92), `X09` (105, 109), `X10` (113), `X11` (117), `X14` (129), `X15` (133), `X16` (137), `X17` (141), `X18` (145), `X19` (149), `X20` (154), `X21` (159), `X22` (163, 165), `X23` (171), `X24` (176) |

Retaking a capture updates these lists in the same change.

## Refreshing the TLD snapshot

Download `https://data.iana.org/TLD/tlds-alpha-by-domain.txt` over
`cli/markdown/src/linear/tlds-alpha-by-domain.txt`, then run
`the_snapshot_is_iana_shaped` and the capture tests, and add a capture
fragment for any token whose verdict changes. A refresh is due whenever a
`rewritten on push` warning or mark, or an audit of stored descriptions,
finds a newly linked token.

## Comments

`comment-read-back.json` holds two comments on the scratch issue `TES-38`,
taken on 2026-11-24 and since trashed: `pushed.md` added first and
`escaped/pushed.md` second, each with `accelerator linear comment add
--body-file`, then read back with `accelerator linear show TES-38 --comments
2` through the allowlist in `../work-item-sync-recovery/README.md`. Linear
lists the newest comment first, so `comments.nodes[0]` is the escaped body and
`comments.nodes[1]` the unescaped one.

Linear stores a comment body byte for byte as sent, leading blank line
included. Against the description read-backs, fragment by fragment:

| Fragments | First comment against `read-back.json` | Second comment against `escaped/read-back.json` |
|---|---|---|
| `R02`, `X12`, `X13` | same | same |
| `R11`, `X04` | differs: the comment keeps `0160.` unescaped; the description stores `0160\.` | same |
| `X18`, `X21`, `X24` | same | differs: the comment keeps the pushed escapes; the description drops them |
| `R12`, `X10`, `X11`, `X14`-`X17`, `X19`, `X22`, `X23` | differs: no token is linked in the comment | differs: the comment keeps the pushed escapes |
| `X09`, `X20` | differs: no lone number becomes a list item in the comment | differs: the comment keeps the pushed escapes |
| Every other fragment | differs: none of the description's cosmetic rewrites happen in the comment | differs: as in the first column |

Gate outcome: no meaning-changing rewrite in comments. Comment bodies are
sent as given, and `comment add` has no `--verbatim`.
