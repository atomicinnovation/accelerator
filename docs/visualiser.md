# Visualiser

`/accelerator:visualise` opens a browser-based companion view of your project's
`meta/` directory. Three views cover the breadth of the directory:

| View          | What it shows                                                              |
|---------------|----------------------------------------------------------------------------|
| **Library**   | Markdown reader for every doc type (plans, research, ADRs, work items …)   |
| **Lifecycle** | Typed-linkage-clustered timelines grouping related documents across phases |
| **Kanban**    | Work-item board driven by `status:` frontmatter; drag-and-drop to update   |

Beyond the three core views, the sidebar's **META** section browses
auto-discovered **templates** (each showing its active resolution tier and
content), and root-cause analyses from `meta/research/issues/` are browsable as
a first-class document type under an **Operate** category. The reader also
supports global search (focus the sidebar box with `/`) and recovers gracefully
from missing or unreadable documents with not-found ("Did you mean…") and
load-error pages.

## Launching

```bash
/accelerator:visualise            # from inside a Claude Code session
accelerator-visualiser            # CLI wrapper — optionally symlink onto $PATH
```

The server binds to `localhost` on a dynamic port. It has no authentication
and emits no telemetry. Re-running the command while the server is alive
returns the same URL.

## Lifecycle

```bash
/accelerator:visualise status     # JSON: running | stale | not_running
/accelerator:visualise stop       # SIGTERM, escalating to SIGKILL after 2s
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
| `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE` | Set to `1` to verify SLSA build-provenance after the SHA-256 check                                        |

The `ACCELERATOR_VISUALISER_RELEASES_URL` mirror must be HTTPS. A localhost
exemption (`127.0.0.1`, `::1`, `localhost`) accepts HTTP for integration
testing; any other plaintext URL is rejected by the launcher.

## Provenance verification

Every released binary carries a SLSA build-provenance attestation
(sigstore-keyless, GitHub Actions OIDC, transparency-log-backed). The default
SHA-256 check proves the cached binary matches what the build runner produced.
Setting `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` adds a second layer: the
launcher calls `gh attestation verify --repo atomicinnovation/accelerator`
and refuses to start if the attestation is missing or invalid. Requires
`gh >= 2.49.0` and network reachability to `api.github.com`.
