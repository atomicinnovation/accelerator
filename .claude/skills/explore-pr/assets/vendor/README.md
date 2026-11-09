# Vendored libraries

`scripts/build.py` inlines these into a self-contained page so it renders
offline; the hosted layout loads the same versions from their CDNs. Both are
unmodified release builds.

| File | Version | Source | Licence | SHA-256 |
|---|---|---|---|---|
| `marked.min.js` | 12.0.2 | `cdn.jsdelivr.net/npm/marked@12.0.2/marked.min.js` | MIT, `marked.LICENSE.md` | `15fabce5b65898b32b03f5ed25e9f891a729ad4c0d6d877110a7744aa847a894` |
| `highlight.min.js` | 11.9.0 | `cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js` | BSD-3-Clause, `highlight.LICENSE` | `837a6fa5b0c736b52bbde2b2b6190f305da3fc9ed41681db5321507057b5c846` |

To change a version, replace the file, update the matching `<script>` tag in
`assets/page.html` and `LIBRARIES` in `scripts/build.py`, and update this
table.
