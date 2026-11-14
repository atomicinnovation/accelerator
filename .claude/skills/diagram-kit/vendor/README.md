# Vendored files

Pinned third-party files the kit runs with. Replace one only together with
its version and checksum here, then run `node tools/measure-fonts.js` if a
font changed and `node --test "tests/*.test.js"` either way.

| File | Version | Licence | SHA-256 |
|---|---|---|---|
| `elkjs/elk.bundled.js` | elkjs 0.12.1 (`lib/elk.bundled.js` of the npm package) | EPL-2.0 OR GPL-3.0-or-later (`elkjs/LICENSE.md`) | `b1032b7cfd2a57c54c6f0e8369d7665e19a579954f7cd20a6dae28f1912ea0d7` |
| `fonts/ibm-plex-sans-latin-{400,600}-normal.woff2` | @fontsource/ibm-plex-sans 5.3.0 | OFL-1.1 (`fonts/IBM-Plex-Sans-LICENSE.txt`) | see `fonts/SHA256SUMS` |
| `fonts/jetbrains-mono-latin-{400,600}-normal.woff2` | @fontsource/jetbrains-mono 5.3.0 | OFL-1.1 (`fonts/JetBrains-Mono-LICENSE.txt`) | see `fonts/SHA256SUMS` |

elkjs 0.13.0 was published and deprecated on the same day as 0.12.1; stay on
0.12.1 until a later release supersedes it. The engine lays diagrams out at
build time and is not copied into a page unless the page draws diagrams from
state it computes in the browser.
