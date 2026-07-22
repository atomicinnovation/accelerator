---
title: Visualiser
---

`/visualise` opens a browser-based companion view of your project's
`meta/` directory. Because every Accelerator artefact is a Markdown
file with structured frontmatter (see [Internals](internals.md)), the
directory is effectively a small database of your project's history —
the visualiser is its reader. Three views cover the breadth of it:

| View          | What it shows                                                              |
|---------------|----------------------------------------------------------------------------|
| **Library**   | Markdown reader for every doc type (plans, research, ADRs, work items …)   |
| **Lifecycle** | Typed-linkage-clustered timelines grouping related documents across phases |
| **Kanban**    | Work-item board driven by `status:` frontmatter; drag-and-drop to update   |

## The three views

### Library

The Library is a rendered-Markdown reader over every document type in
`meta/` — plans, research, ADRs, work items, reviews, validations, PR
descriptions, and notes — organised in a sidebar by category. Each
document's frontmatter is rendered as a structured header (status,
dates, authorship, links to related documents), and code blocks,
tables, and mermaid diagrams render as they would on a docs site
rather than as raw text. A detail-page **"Open in editor"** action
deep-links the file into your editor (configurable — see
[Customisation](#customisation)).

### Lifecycle

Documents in `meta/` link to each other through typed frontmatter
fields (`parent`, `derived_from`, `relates_to`). The Lifecycle view
follows those links to cluster related documents — a work item, the
research it prompted, the plan derived from that research, the
decisions and reviews along the way — and lays each cluster out as a
timeline across the workflow's phases. It answers "what is the full
story of this piece of work?" in one screen; the
[case study](case-study.md) walks through one such cluster in prose.

### Kanban

The Kanban view reads the `status:` frontmatter of every work item in
`meta/work/` and presents them as a board. Dragging a card between
columns writes the new status back to the file's frontmatter — the
board is a UI over the files, not a separate store, so changes made by
skills (or by hand) and changes made on the board are the same kind of
change.

## Beyond the core views

The sidebar's **META** section browses auto-discovered **templates**
(each showing its active resolution tier and content), and root-cause
analyses from `meta/research/issues/` are browsable as a first-class
document type under an **Operate** category. The reader also supports
global search (focus the sidebar box with `/`) and recovers gracefully
from missing or unreadable documents with not-found ("Did you mean…")
and load-error pages.

## Live reload

The server watches `meta/` for changes and pushes updates to the
browser, so the visualiser works as a passive second screen during a
session: as `research-codebase` writes its findings or `implement-plan`
ticks success criteria, the open document updates in place without a
refresh. This is the easiest way to follow a long-running skill's
progress without touching the conversation.

## Under the hood

The visualiser is a single native binary: a Rust (axum) HTTP server
with the React frontend embedded in it at build time. It reads `meta/`
directly from disk on request — there is no index to build, no
database, and nothing to keep in sync.

## Launching

```bash
/visualise            # from inside a Claude Code session
accelerator-visualiser            # CLI wrapper — optionally symlink onto $PATH
```

The server binds to `localhost` on a dynamic port. It has no authentication
and emits no telemetry. Re-running the command while the server is alive
returns the same URL.

## Lifecycle

```bash
/visualise status     # JSON: running | stale | not_running
/visualise stop       # SIGTERM, escalating to SIGKILL after 2s
```

Both subcommands also work via the `accelerator-visualiser` CLI wrapper.
The server auto-exits after 8 hours idle or when the process that
launched it exits, so explicit `stop` is rarely necessary.

## First-run binary download

The server is distributed as a pre-compiled native binary (~8 MB). On first
run the launcher:

1. Reads `bin/checksums.json` (committed in the plugin) to find the SHA-256
   for your platform and the current plugin version.
2. Downloads the matching binary from the plugin's GitHub Releases over HTTPS.
3. Verifies the download against the manifest and caches it under the plugin
   root. Subsequent launches skip the download.

Every plugin version — pre-release (`X.Y.Z-pre.N`) and stable (`X.Y.Z`) —
ships four-platform binaries. There is no need to build locally to use a
pre-release version.

## Customisation

| Mechanism                                  | Purpose                                                                                                   |
|--------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| `ACCELERATOR_VISUALISER_BIN`               | One-shot override pointing at a locally-built binary                                                      |
| `visualiser.binary` config key             | Persistent binary override in `.accelerator/config.local.md`                                              |
| `ACCELERATOR_VISUALISER_IDLE_TIMEOUT`      | One-shot override of the idle auto-shutdown window (duration string, or `never`/`0` to disable)           |
| `visualiser.idle_timeout` config key       | Persistent idle auto-shutdown window (humantime duration; default `8h`; `never`/`0` to disable)           |
| `visualiser.editor` config key             | Editor deep-link for the detail-page "Open in editor" action (preset key or `{abs}`/`{rel}` URL template) |
| `ACCELERATOR_VISUALISER_EDITOR`            | One-shot override of `visualiser.editor`                                                                  |
| `visualiser.editor_project` config key     | JetBrains project name for the editor deep-link (defaults to the project directory basename)              |
| `ACCELERATOR_VISUALISER_EDITOR_PROJECT`    | One-shot override of `visualiser.editor_project`                                                          |
| `ACCELERATOR_VISUALISER_RELEASES_URL`      | Alternative HTTPS mirror for air-gapped or self-hosted installs                                           |

The `ACCELERATOR_VISUALISER_RELEASES_URL` mirror must be HTTPS. A localhost
exemption (`127.0.0.1`, `::1`, `localhost`) accepts HTTP for integration
testing; any other plaintext URL is rejected by the launcher.

## Provenance verification

Every released binary carries a SLSA build-provenance attestation
(sigstore-keyless, GitHub Actions OIDC, transparency-log-backed). The default
SHA-256 check proves the cached binary matches what the build runner produced.
For an independent, out-of-band check you can run `gh attestation verify
<binary> --repo atomicinnovation/accelerator` yourself (requires
`gh >= 2.49.0` and network reachability to `api.github.com`).

## Skill reference

For invocation and arguments, see the
[`visualise`](reference/skills/visualisation/visualise.md) skill reference.
