# Linear autolink and stray-list probes

Six bodies were pushed to scratch issues in the `TES` team on 2026-11-24,
read back with `accelerator linear show`, projected through the allowlist in
`../work-item-sync-recovery/README.md`, and deleted.

| Body | Issue | Read back | Holds |
|---|---|---|---|
| `probes.md` | `TES-33` by `issueCreate`, `TES-34` by `issueUpdate` | `probes.created.json`, `probes.updated.json` | `Q001`-`Q310` |
| `follow-ups.md` | `TES-36` | `follow-ups.read-back.json` | `S001`-`S209` |
| `split-tokens.md` | `TES-41` | `split-tokens.read-back.json` | `M001`-`M036`, `N001`-`N020` |
| `entities.md` | `TES-39` | `entities.read-back.json` | `E001`-`E026` |
| `entities.read-back.json`'s description | `TES-40` | `entities.second-push.read-back.json` | `E001`-`E026`, as a pull writes them |
| `tld-sweep.md` | `TES-35` | `tld-sweep.read-back.json` | `x foo.<tld> y` for every label in `cli/markdown/src/linear/tlds-alpha-by-domain.txt` |

Each probe in `probes.md`, `follow-ups.md`, `split-tokens.md` and
`entities.md` sits under a `#### <code>
<group>` heading; each sweep line is its own paragraph led by `T<nnnn>`.
Create and update read back byte-identically, so both mutations share one
parser.

## Dotted tokens

A bare dotted token is linked as `[t](<http://t>)` when every condition holds:

| Condition | Linked | Not linked |
|---|---|---|
| Last label | any IANA label in any case | bare `md` and `merck`, the only sweep exceptions (`T0731`, `T0741`); non-IANA labels |
| `md` with a suffix | `foo.md:1`, `foo.md/`, `foo.md#x`, `foo.md?x`, `a@b.md` | `foo.md`, `a.b.md`, `www.md`, `foo.md:x` |
| Labels | 1-63 characters of Unicode letters, digits, combining marks, format characters, `-` inside, and `~` (`café.io` composed or decomposed, `日本.jp`, `हिन्दी.com`, `a` U+200D `.io`, `[\~a.io](<http://~a.io>)`) | a 64-character label, `_`, a leading or trailing `-`, an empty label |
| Preceding character | whitespace and most punctuation: `( [ { < " ' * + = ; , ! ? # $ % & \| ^`, `—`, `’`, `«`, `…`, no-break space | `-`, `_`, `:`, `@`, `/`, `\`, `.` |
| Following character | ends before `.` `,` `)` `]` `}` `>` `"` `'` `*` `:` `;` `!` `?` `…` `’`; extends over `:<port>`, `/path`, `?query`, `#fragment` | cancelled by `_`, `~`, `-`, `+`, `=`, `$`, `\|`, `^`, `<`, `` ` ``, a digit or a letter |
| Context | emphasis (`[**crates.io**](<http://crates.io>)`), headings, table cells, list items, blockquotes, emphasis touching it (`crates.io**z**`) | code, `~~strikethrough~~`, a token touching an inline HTML tag, a token split by inline markup |

Scheme URLs are linked whole after any boundary, quotes and `URL:` included,
when the scheme is `http`, `https` or `ftp` in any case, or the URL starts
`//`. No other scheme links, and the host inside one is not linked either
(`ssh://github.com`, `file://crates.io`, `s3://bucket.io/x`).

`local@host` is linked as `mailto:` when the host is a valid name, `md` hosts
included. `mailto:` is kept in the link text when written.

## Split tokens

`split-tokens.md` sends dotted tokens broken by inline markup (group
`markup`), eleven of them again with the dot escaped (`remedy`), and labels
holding combining marks or format characters (groups `nfd`, `nfc` and
`marks`).

No token split by markup is linked: not `crates.**io**`, `**crates**.io`,
`crates.i**o**`, `crates.*io*`, `crates.~~io~~`, `foo.<b>rs</b>`,
`<b>foo</b>.rs`, `crates.<!-- c -->io`, ``foo.`rs` ``, `` `foo`.rs ``,
`[foo](x).rs`, `foo.[rs](x)` nor `foo.<br>rs`. A whole token beside markup
is linked (`M023` `crates.io**z**`, `M024` `**z**crates.io`), except before a
code span: ``jql.rs`:167` `` is left bare (`M019`). Linear stores each
`markup` probe as sent but for cosmetic rewrites: `crates**.io**`'s literal
asterisks escaped, `_io_` as `*io*`, and a link destination in angle
brackets. Each `remedy` probe is stored without its backslash.

Linear reads a label whole when it holds a combining mark or a format
character, whether the text is composed or decomposed:
NFD `café.io`, `naïve.io`, `ñ.io` and `é.io`, `हिन्दी.com`, a leading mark
(`N015`), a doubled mark (`N017`), an enclosing mark (`N018`) and a zero-width
joiner (`N019`) are all linked, to the label's punycode. A mark opening the
last label (`N016`) and a non-ASCII last label (`N008`, `N012`) are left
bare. The escaped `N020` is stored as plain text.

## Escapes

One `\.` anywhere in a token stops the link (`a\.b.io` and `a.b\.io` alike),
as do `\@` and `\:`. Linear stores none of these escapes: the read-back holds
`crates.io` as plain text with no backslash. An escape inside a scheme URL
cuts it short: `https://crates\.io` reads back as
`[https://crates](<https://crates>).io`.

## Stray lists

A continuation line holding only one to nine digits and `.` or `)` becomes
an ordered-list item numbered `N. `, zeros stripped and `)` turned into `.`,
when it continues the first paragraph of a list item. That holds under
bullet, ordered, task and nested items, after a tab indent, on a lazy line at
column 0, for `0.` and `00.`, for consecutive lone lines, and with more
continuation lines after it. A hard break before the line is lost.

It does not happen in a blockquote, even one inside a list item, in a list
item's later paragraph, at top level, for ten digits, or with text after the
number; there `.` is escaped and `)` kept. `0160\.` and `0160\)` both prevent
it and are stored as plain `0160.` and `0160)`.

## Other rewrites

- An image is re-hosted to `uploads.linear.app` and split into its own
  paragraph (`Q256`).
- `&lt;`, `&gt;`, `&amp;` and `&#46;` are decoded (`Q262`-`Q264`), and
  `<br>` in a table cell becomes `&#10;` (`S207`).
- A token of 130 labels is linked with an empty destination (`S167`).

## Entities

`entities.md` sends each character CommonMark gives meaning to as an entity
(group `entities`), and again behind a backslash (group `remedy`). "Reads
alike" is PP-880's equivalence of the probe as sent and as stored, pinned by
`each_entity_probe_reads_as_recorded` in
`corpus-adapters/tests/body_equivalence.rs`.

| Probe | Sent | Stored | Reads alike |
|---|---|---|---|
| `E001` | `&lt;b&gt;bold&lt;/b&gt;` | `<b>bold</b>` | no: a live tag |
| `E002` | `&lt;!-- c --&gt;` | `<!-- c -->` | no: a live comment |
| `E003` | `&lt;https://crates.io&gt;` | `<[https://crates.io](<https://crates.io>)>` | yes |
| `E004` | `&amp;lt;b&amp;gt;` | `&lt;b&gt;` | no: shows `<b>` |
| `E005` | `&#42;emph&#42;` | `\*emph\*` | yes |
| `E006` | `&#95;emph&#95;` | `\_emph\_` | yes |
| `E007` | `&#91;t&#93;(https://x.io)` | `\[t\]([https://x.io](<https://x.io>))` | yes |
| `E008` | `&#96;code&#96;` | ``\`code\` `` | yes |
| `E009` | `\| x &#124; y \| z \|` | `\| x \| y \| z \|` | no: a third cell |
| `E010` | `&#35; heading` | `\# heading` | yes |
| `E011` | `&#45; item` | `\- item` | yes |
| `E012` | `&#49;. item` | `1\. item` | yes |
| `E013` | `a&nbsp;b` | `a` U+00A0 `b` | yes |
| `E014` | `&copy;` | `©` | yes |
| `E015` | `\<b>bold\</b>` | `<b>bold</b>` | no: a live tag |
| `E016` | `\<!-- c -->` | `<!-- c -->` | no: a live comment |
| `E017` | `\<https://crates.io>` | `<[https://crates.io](<https://crates.io>)>` | yes |
| `E018` | `\&lt;b\&gt;` | `&lt;b&gt;` | no: shows `<b>` |
| `E019`-`E022`, `E024`-`E026` | `\*`, `\_`, `\[`, `` \` ``, `\#`, `\-`, `1\.` | as sent, but `E021`'s URL is linked | yes |
| `E023` | `\| x \\| y \| z \|` | `\| x \| y \| z \|` | no: a third cell |

Linear decodes every entity. Where the character is Markdown punctuation it
stores a backslash escape in its place, so the reading holds, except for
`<`, `&` and a `|` in a table cell, where it stores the bare character. It
also drops the backslash before those three, so no form sent as text
survives: `\<`, `\&` and `\|` change meaning as their entities do.

### Pulling and pushing again

`TES-40` was created, escaped as `work update --push` sends a body, from
`TES-39`'s stored description, which is what a pull writes locally.
`each_pulled_entity_probe_reads_as_recorded_after_a_second_push` pins
it. Every probe then reads alike except `E004` and `E018`: the pulled
`&lt;b&gt;` is decoded again, to a live `<b>`. The raw tag and comment are
stored byte for byte, and Linear drops a row's extra table cell, which the
table ignored.

### Decision

No form of `<`, `&` or a table cell's `|` sent as text survives Linear, as
an entity or behind a backslash, and none is kept by special handling.
The push's `rewritten on push` warning reports the rewrite, the baseline's
rewrite mark keeps it visible, and a pull adopts Linear's form and clears
the mark. The next
push then reads alike, except that a doubly-encoded `<` or `&` takes a
second push and pull to settle. No work item is raised.
