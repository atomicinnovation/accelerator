# Escaped Linear rewrite capture

`pushed.md` is `../pushed.md` with a `\` inserted wherever the digit rule
and the dotted-token rule call for one, and nothing else changed. It was
created as the scratch Linear issue `TES-37` on 2026-11-24 with `accelerator
linear create --body-file`, read back with `accelerator linear show` into
`read-back.json` through the allowlist in
`../../work-item-sync-recovery/README.md`, checked in Linear's web UI the
same day, then trashed with `issueDelete`.

Insertions only, so every line number matches `../pushed.md`, and the
read-back's line numbers match `../read-back.json`.

## Fragments

| Fragment | Pushed escapes | Read back | Web UI |
|---|---|---|---|
| `R11` Wrapped digit lines | `0160\.` | `0160\.` | plain `0160.`, no list |
| `R12` Dotted token | `crates\.io` | `crates.io`, unlinked | plain, unlinked |
| `X04` Digit line, no indent | `0160\.` | `0160\.` | plain `0160.`, no list |
| `X05` Digit line in an item | `0160\.` | `0160\.` | plain `0160.`, no list |
| `X06` Digit line past the indent | `0160\.` | `0160\.` | plain `0160.`, no list |
| `X09` Lone digit line | `0160\.` twice | `0160.` twice | plain `0160.`, no list |
| `X10` TLD file extensions | `server\.pid`, `foo\.md` | both plain, unlinked | plain, unlinked |
| `X11` Line number | `jql\.rs:167` | `jql.rs:167`, unlinked | plain, unlinked |
| `X14` Multi-part tokens | `a\.b\.io` | `a.b.io`, unlinked | plain, unlinked |
| `X15` Mixed case | `Crates\.IO` | `Crates.IO`, unlinked | plain, unlinked |
| `X16` Sentence-final | `crates\.io.` | `crates.io.`, unlinked | plain, unlinked |
| `X17` Bracketed | `(crates\.io)` | `(crates.io)`, unlinked | plain, unlinked |
| `X18` Path | `src/client\.rs` | `src/client.rs` | plain, unlinked |
| `X19` Email address | `a@b\.io` | `a@b.io`, unlinked | plain, unlinked |
| `X20` Lone parenthesised number | `0160\)` | `0160)` | plain `0160)`, no list |
| `X21` Between inline tags | `<span>crates\.io</span>` | `<span>crates.io</span>` | plain, unlinked |
| `X22` Bold and heading | `**crates\.io**`, `### Notes on crates\.io` | both plain, unlinked | plain, unlinked |
| `X23` Table cell | `\| crates\.io \|` | `\| crates.io \|`, unlinked | plain, unlinked |
| `X24` Lone digit line in a blockquote | `> 0160\.` | `> 0160.` | plain `0160.`, no list |
| `X25` Quoted scheme URLs | none | each URL linked whole | each URL linked whole |

Every other fragment was pushed as in `../pushed.md` and read back as in
`../read-back.json`.

## Stored escapes

Linear stores none of the escapes the rules insert. `0160\.` survives in
`R11` and `X04`-`X06` only because Linear's serialiser writes it there
unasked, as the unescaped capture shows; a pull already writes that form
locally. Every lone-number escape (`X09`, `X20`, `X24`) and every dotted-token
escape reads back without its backslash, as the probe corpus predicted.

## Web-UI gate

Passed on 2026-11-24: no backslash before `.` or `)`, no stray list, and no
link on an escaped token, `X21` included. The only links are `R05`'s, `R06`'s
URL and `X25`'s two URLs, each linked whole.
