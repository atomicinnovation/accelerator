# Linear autolink and stray-list probes

Three bodies were pushed to scratch issues in the `TES` team on 2026-11-24,
read back with `accelerator linear show`, projected through the allowlist in
`../work-item-sync-recovery/README.md`, and deleted.

| Body | Issue | Read back | Holds |
|---|---|---|---|
| `probes.md` | `TES-33` by `issueCreate`, `TES-34` by `issueUpdate` | `probes.created.json`, `probes.updated.json` | `Q001`-`Q310` |
| `follow-ups.md` | `TES-36` | `follow-ups.read-back.json` | `S001`-`S209` |
| `tld-sweep.md` | `TES-35` | `tld-sweep.read-back.json` | `x foo.<tld> y` for every label in `cli/markdown/src/linear/tlds-alpha-by-domain.txt` |

Each probe in `probes.md` and `follow-ups.md` sits under a `#### <code>
<group>` heading; each sweep line is its own paragraph led by `T<nnnn>`.
Create and update read back byte-identically, so both mutations share one
parser.

## Dotted tokens

A bare dotted token is linked as `[t](<http://t>)` when every condition holds:

| Condition | Linked | Not linked |
|---|---|---|
| Last label | any IANA label in any case | bare `md` and `merck`, the only sweep exceptions (`T0731`, `T0741`); non-IANA labels |
| `md` with a suffix | `foo.md:1`, `foo.md/`, `foo.md#x`, `foo.md?x`, `a@b.md` | `foo.md`, `a.b.md`, `www.md`, `foo.md:x` |
| Labels | 1-63 characters of Unicode letters, digits, `-` inside, and `~` (`café.io`, `日本.jp`, `[\~a.io](<http://~a.io>)`) | a 64-character label, `_`, a leading or trailing `-`, an empty label |
| Preceding character | whitespace and most punctuation: `( [ { < " ' * + = ; , ! ? # $ % & \| ^`, `—`, `’`, `«`, `…`, no-break space | `-`, `_`, `:`, `@`, `/`, `\`, `.` |
| Following character | ends before `.` `,` `)` `]` `}` `>` `"` `'` `*` `:` `;` `!` `?` `…` `’`; extends over `:<port>`, `/path`, `?query`, `#fragment` | cancelled by `_`, `~`, `-`, `+`, `=`, `$`, `\|`, `^`, `<`, a digit or a letter |
| Context | emphasis (`[**crates.io**](<http://crates.io>)`), headings, table cells, list items, blockquotes | code, `~~strikethrough~~`, a token touching an inline HTML tag |

Scheme URLs are linked whole after any boundary, quotes and `URL:` included,
when the scheme is `http`, `https` or `ftp` in any case, or the URL starts
`//`. No other scheme links, and the host inside one is not linked either
(`ssh://github.com`, `file://crates.io`, `s3://bucket.io/x`).

`local@host` is linked as `mailto:` when the host is a valid name, `md` hosts
included. `mailto:` is kept in the link text when written.

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
