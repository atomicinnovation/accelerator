
## R01 Escaping

Roughly ~550 lines remain.

## R02 Joined continuation lines

This paragraph is wrapped
across three source lines
before it ends.

## R03 Per-line bold

**first bold line
second bold line**

## R04 Bold hoisted off code

**run `cargo test`**

## R05 Link destinations

See [the guide](https://example.com/guide) for details.

## R06 Autolinked URLs

Visit https://example.com/autolink today.

## R07 Bullet markers

- alpha
- beta

## R08 Task-marker case

- [x] done task
- [ ] open task

## R09 Table delimiters

| Name | Value |
|---|---|
| a | 1 |

## R10 Inserted blank lines

- first item
- second item

## R11 Wrapped digit lines

The removed items are 0158, 0159 and
  0160. Nothing else moved.

## R12 Dotted tokens autolinked

Published to crates.io last week.

## R13 Code-span indentation

- Run `cargo nextest
  run` inside the list item.

## X01 Code-span line ending at top level

Run `cargo nextest
run` at the top level.

## X02 Blockquote container

> Roughly ~550 lines
> remain in **quoted
> bold** text.

## X03 Paragraph directly followed by a list

Proposed children:
- first child
- second child

## X04 Wrapped digit line without indentation

The removed items are 0158, 0159 and
0160. Nothing else moved.

## X05 Wrapped digit line inside a list item

- The removed items are 0158, 0159 and
  0160. Nothing else moved.

## X06 Wrapped digit line indented past the content indent

- The removed items are 0158, 0159 and
      0160. Nothing else moved.

## X07 Code-span continuation indented at top level

Run `cargo nextest
  run` at the top level.

## X08 Code-span continuation indented past a task item's content indent

- [ ] Given the filters, then Jira `labels IN ('a') AND labels NOT IN
      ('b')` is composed.

## X09 Wrapped digit line holding only the number

- The tier infrastructure and the migration of existing evals — both owned by
  0160.
- Running evals on every CI build.

The tier infrastructure and the migration of existing evals — both owned by
0160.

## X10 File names whose extension is a TLD

Remove server.pid and foo.md first.

## X11 File name with a line number

See jql.rs:167 for the filter.

## X12 File name whose extension is not a TLD

Edit foo.toml before the release.

## X13 Version and abbreviation

Upgrade to v1.2.3 first, e.g. tomorrow.

## X14 Multi-part tokens

Both a.b.c and a.b.io resolve.

## X15 Mixed case

Published to Crates.IO last week.

## X16 Sentence-final dotted token

Published to crates.io.

## X17 Bracketed dotted token

Published (crates.io) last week.

## X18 Path ending in a dotted token

Edit src/client.rs today.

## X19 Email address

Mail a@b.io today.

## X20 Wrapped parenthesised number holding only the number

- The tier infrastructure and the migration of existing evals — both owned by
  0160)
- Running evals on every CI build.

## X21 Dotted token between inline HTML tags

Published to <span>crates.io</span> last week.

## X22 Dotted token in bold and in a heading

Published to **crates.io** last week.

### Notes on crates.io

## X23 Dotted token in a table cell

| Registry | Use |
| - | - |
| crates.io | Rust |

## X24 Wrapped number in a blockquote

> The tier infrastructure and the migration of existing evals — both owned by
> 0160.

## X25 Quoted scheme URL

See "https://crates.io/x" and URL:https://crates.io for details.
