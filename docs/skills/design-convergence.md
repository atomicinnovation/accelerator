# Design Convergence

Design convergence skills capture two design surfaces — a current frontend and a
target prototype — as structured inventory artifacts, then compute a structured
gap between them. The gap artifact's prose paragraphs satisfy the cue-phrase
contract that `extract-work-items` consumes, so the workflow plugs straight into
the existing work-item lifecycle. Each inventory snapshot is self-contained
(markdown plus screenshots in a dated directory); re-running for the same source
supersedes the prior snapshot without losing it.

```
inventory-design (current)  ─┐
                             ├─▶ analyse-design-gaps ─▶ extract-work-items ─▶ meta/work/*
inventory-design (target)   ─┘
```

### inventory-design

**What it does** — Generate a structured design inventory for a frontend
source — tokens, components, screens, and features — by crawling it with code
analysis, live Playwright inspection, or both.

**How to use it** — `/accelerator:inventory-design [source-id] [location] [--crawler code|runtime|hybrid] [--allow-internal] [--allow-insecure-scheme]`

**Advice & guidelines** — Pick the crawler mode to match the source: `code` for
a local repo, `runtime` for a hosted prototype, `hybrid` (default for code
repos) for both. See [Requirements](#requirements) before using
`runtime`/`hybrid`.

### analyse-design-gaps

**What it does** — Compare two design inventories produced by inventory-design
and emit a structured gap artifact whose prose paragraphs satisfy the
extract-work-items cue-phrase contract.

**How to use it** — `/accelerator:analyse-design-gaps [current-source-id] [target-source-id]`

**Advice & guidelines** — The gap artifact feeds straight into
`/accelerator:extract-work-items`, so run both inventories first, then this, then
extract.

Three-step example:

```
/accelerator:inventory-design current ./apps/webapp
/accelerator:inventory-design prototype https://prototype.example.com
/accelerator:analyse-design-gaps current prototype
```

The resulting gap artifact under `meta/research/design-gaps/` feeds straight into
`/accelerator:extract-work-items <gap-file>`.

`inventory-design` supports three crawler modes: `code` (static analysis only,
no Playwright needed), `runtime` (Playwright executor only), and `hybrid`
(both, default for code-repo sources).

New flags: `--allow-internal` (permit RFC1918 / loopback-variant hosts) and
`--allow-insecure-scheme` (permit plain `http://` to non-localhost public
hosts). `http://localhost` and `http://127.0.0.1` are accepted without any
flag.

## Requirements

For `--crawler runtime` or `--crawler hybrid`:

- **Node ≥ 20** — executor bootstrap and daemon require Node.js 20 or later
- **macOS or Linux** — Windows is not supported
- **~500 MB free disk** — first-run Chromium install writes to the cache

Run `ensure-playwright.sh` to bootstrap the executor manually; the skill runs
it automatically on first use.

## Runtime browser dependency

The executor (`run.sh`) wraps a Node.js daemon that drives Playwright's
Chromium. On first use with `--crawler runtime|hybrid`, the skill runs
`ensure-playwright.sh` to install Playwright and Chromium into a per-machine
cache. Subsequent runs reuse the cache without a network round-trip.

Cache root: `~/.cache/accelerator/playwright/<sha8>/` (namespace keyed on the
skill-shipped `package-lock.json` hash).

For the executor wire protocol see
[`skills/design/inventory-design/PROTOCOL.md`](../../skills/design/inventory-design/PROTOCOL.md).

## Cache & cleanup

| Path                                                      | Purpose                                   |
|-----------------------------------------------------------|-------------------------------------------|
| `~/.cache/accelerator/playwright/`                        | Per-machine Playwright + Chromium cache   |
| `<project>/.accelerator/tmp/inventory-design-playwright/` | Per-project daemon state (port file, PID) |

To reset both:

```bash
run.sh daemon-stop
rm -rf ~/.cache/accelerator/playwright .accelerator/tmp/inventory-design-playwright
```

## Troubleshooting

- **Hung daemon**: run `run.sh daemon-stop` to shut it down cleanly.
- **Bootstrap failure**: run `ensure-playwright.sh` directly to see the full
  error output. Check `NPM_CONFIG_REGISTRY`, `NODE_EXTRA_CA_CERTS`,
  `HTTPS_PROXY`, and `PLAYWRIGHT_DOWNLOAD_HOST`.
- **Downgrade to code**: if bootstrap fails in `hybrid` mode the skill
  automatically falls back to `code`-only crawl with a printed notice.

## Authenticated browser crawls

`/accelerator:inventory-design` reads the following environment variables when
the location is a hosted prototype or running app and authentication is
required. They are also read by any future skill that uses the `browser-*`
agents.

| Variable                          | Purpose                                                          |
|-----------------------------------|------------------------------------------------------------------|
| `ACCELERATOR_BROWSER_AUTH_HEADER` | Header injected on navigations to the resolved location's origin |
| `ACCELERATOR_BROWSER_USERNAME`    | Form-login username (used with `_PASSWORD` and `_LOGIN_URL`)     |
| `ACCELERATOR_BROWSER_PASSWORD`    | Form-login password                                              |
| `ACCELERATOR_BROWSER_LOGIN_URL`   | Login form URL                                                   |

Precedence: if `AUTH_HEADER` is set it takes precedence and the form-login
vars are ignored (with a warning). If `AUTH_HEADER` is unset, all three of
`USERNAME`, `PASSWORD`, and `LOGIN_URL` must be set together — partial sets
cause the skill to fail with a clear error. With none set, auth-walled routes
are skipped and noted in the inventory's Crawl Notes.

Security: `AUTH_HEADER` is sent **only** on navigations whose origin matches
the resolved location (or the login URL); cross-origin navigations strip it.
Screenshots mask password and token fields. The skill refuses to write an
inventory if any env-var literal appears in the generated body.

## Security considerations

- **Env vars**: store `ACCELERATOR_BROWSER_*` values in your shell profile or
  a local `.env` file (gitignored), not in committed config.
- **Screenshots**: each inventory directory contains screenshots committed to
  the repo. Avoid pointing `inventory-design` at screens that display
  sensitive personal data, PII, or credentials.
- **Side-effecting forms**: the `browser-analyser` agent uses `browser_click`
  and `browser_type` for state-transition testing. Do not point
  `inventory-design` at production systems with forms that have real-world
  side effects (payments, email sends, account mutations).
- **Executor isolation**: the Playwright executor runs as a local TCP daemon on
  `127.0.0.1` only. It never binds to an external interface. Screenshots mask
  `[type=password]`, `[autocomplete*=token]`, and `[data-secret]` fields
  automatically.
