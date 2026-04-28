---
date: "2026-04-28T00:00:00Z"
type: plan
skill: create-plan
ticket: null
status: ready
---

# Phase 10 — Error handling, accessibility, polish implementation plan

## Overview

Close out the visualiser's v1 quality bar before release. Phase 10 takes the
feature-complete server + SPA from Phases 1–9 and hardens it against the
spec's failure-modes matrix and non-functional accessibility/observability
requirements. Concretely:

1. The launcher refuses to start in a project that hasn't been initialised
   (missing `<meta/tmp>/.gitignore` sentinel) and emits a friendly JSON error
   pointing at `/accelerator:init`.
2. Every server log line is written as JSON to `<tmp>/visualiser/server.log`,
   with rotation at 5 MB (matching the spec's "Rotated at 5MB" rule) so
   the log directory cannot grow unbounded.
3. Every HTTP response carries `Accelerator-Visualiser-Version: <semver>`
   so the UI (and operators on `curl -I`) can identify the server
   version serving them. The header name is unprefixed per RFC 6648
   (which deprecates `X-` prefixes for new headers); the spec's
   `X-Accelerator-Visualiser` lands here as the unprefixed form. A
   spec amendment lands alongside this phase.
4. A new `GET /api/info` endpoint exposes server identity as JSON
   (`{"name": "accelerator-visualiser", "version": "<semver>"}`), and
   the sidebar renders the version in a footer.
5. The frontend's `EventSource` is wrapped in a small reconnect manager
   that backs off exponentially with jitter and runs an
   invalidate-all-queries sweep on every successful reconnect (per the
   spec's failure-modes matrix entry "SSE disconnect → Auto-reconnect with
   exponential backoff; invalidate-all on reconnect").
6. Library doc pages render a prominent banner — "We couldn't read this
   document's metadata header; showing the file as-is." — when the
   underlying entry's `frontmatterState === 'malformed'` (matching the
   spec failure-modes matrix). The smaller chip-row warning emitted by
   `FrontmatterChips` stays as secondary signal. The banner uses no
   ARIA live-region role (it is static page-load content, not a live
   update).
7. Global focus rings are visible on all interactive elements
   including in Windows High Contrast / forced-colors mode,
   dnd-kit publishes its screen-reader announcements via the
   `accessibility.announcements` hook (currently unset) with
   ticket-number-prefixed strings, and the default theme passes
   WCAG 2.2 AA contrast for text and focus indicators.

Test-driven throughout: every step writes a failing test first, then the
implementation that makes it pass. Each inner phase ships independently —
`cargo test -p accelerator-visualiser` and `npm test` (vitest) stay green
between inner phases except inside the step that introduces a new
failing test.

## Current State Analysis

Phases 1–9 left a feature-complete read+write surface with active SSE
invalidation, slug-cluster lifecycle, kanban drag-drop, and wiki-link
resolution. Phase 10 starts from that base. The relevant pieces of the
existing system, with file:line references:

### Server (`skills/visualisation/visualise/server/`)

- **Logging is JSON-formatted but goes to stdout.** `main.rs:17-20`
  initialises `tracing_subscriber::fmt().json().with_env_filter(...)`
  with no explicit `with_writer(...)` call, so events land on the
  stdout that `launch-server.sh` redirects to `<TMP>/visualiser/server.log`.
  This works as a coarse capture but rotates only when the launcher
  re-creates the file on next startup. The spec wants in-process
  size-based rotation at 5 MB — the launcher's append-on-restart trick
  doesn't satisfy "Rotated at 5MB."
- **`Config` already carries `log_path`.** `config.rs` deserialises a
  `log_path: PathBuf` field that the preprocessor writes; `server.rs:222`
  surfaces it in `ServerInfo`. Phase 10 routes the tracing subscriber's
  writer at this path instead of stdout.
- **Version is captured but not emitted as a header.** `server.rs:215`
  reads `env!("CARGO_PKG_VERSION")` into `ServerInfo.version`, but
  no middleware adds an `Accelerator-Visualiser-Version` header to
  responses. (The spec's original `X-Accelerator-Visualiser` lands
  unprefixed per RFC 6648; see Phase 10.3.)
- **`/api/healthz` exists but `/api/info` does not.** `server.rs:154,
  175-177` mounts a `healthz` route returning the literal `"ok\n"`.
  There is no JSON `info` endpoint yet — the sidebar would either need
  to read the response header from a known endpoint (fragile across
  proxies) or a dedicated endpoint. Phase 10 introduces the dedicated
  endpoint (cheap, idiomatic).
- **`FrontmatterState::Malformed` already exists and propagates.**
  `frontmatter.rs:76-89` defines `Parsed | Absent | Malformed`,
  `indexer.rs` populates it on every parse, and `sse_hub` already
  emits a `doc-invalid` SSE event when entries flip into
  `Malformed` (`sse_hub.rs:123` test asserts the wire shape).
  `IndexEntry.frontmatterState` is already serialised on the wire and
  consumed by the frontend.
- **`api/docs.rs` returns the document body even when frontmatter is
  malformed** — the body is always served raw with the parsed-or-not
  state as a flag — so the only Phase 10 work is presentational.
- **No request-summary log line is currently emitted.** Tower-http's
  `trace` feature is enabled in `Cargo.toml`, but no `TraceLayer` is
  composed onto the router. The spec's observability list — startup,
  config snapshot, request summary (method, path, status, duration),
  SSE subscriber count, FS events — implies a request-log layer here.
  Phase 10 adds the layer using `tower_http::trace::TraceLayer` so that
  every line lands in the rotating file.
- **Path safety + body-cap + timeout layers already wrap the router**
  (`server.rs:165-172`); the version-header middleware composes onto
  the same stack.

### Frontend (`skills/visualisation/visualise/frontend/`)

- **`useDocEvents` builds an `EventSource` directly** with no reconnect
  policy beyond the browser's built-in (~3s, no backoff, no
  invalidate-all on reconnect). `use-doc-events.ts:78-153` wires
  `source.onerror = () => invalidateQueries(['docs'])` but lets the
  browser handle the actual reconnect timing, and `source.onopen` only
  invalidates queued (drag-pending) keys, not all queries. This works
  on a single-error blip, but a long disconnection produces a tight
  retry loop and the partial invalidation leaves cluster/related/kanban
  caches stale until the next event lands.
- **`FrontmatterChips`** (`FrontmatterChips.tsx`) renders a small chip
  row when `state === 'parsed'`, returns `null` for `'absent'`, and a
  short "warning banner" for `'malformed'`. The banner is inline in
  the doc-header chip area — the existing test asserts a single
  warning element. The spec calls for a more prominent doc-page-level
  banner ("Frontmatter unparseable; showing raw content") that's
  immediately visible above the markdown body. Phase 10 leaves the
  chip-row warning in place (secondary signal) and adds the
  document-level banner above the body in `LibraryDocView`.
- **`LibraryDocView`** (`LibraryDocView.tsx`) already branches on
  `entry!.frontmatterState` for the chips. It does not yet render a
  page-level banner.
- **`Sidebar`** (`Sidebar.tsx`) has three sections (Documents / Views /
  Meta) with no footer. The version footer slots cleanly underneath
  the existing `<nav>`'s last `<section>`.
- **Kanban already wires `KeyboardSensor` and `aria-live` regions**
  (`KanbanBoard.tsx:4-8, 49, 175-177`). What's *not* wired is dnd-kit's
  own `accessibility.announcements` prop — by default dnd-kit publishes
  a generic announcement when picking up / dropping items, but the
  visualiser overrides nothing, so screen-reader users hear the
  framework default rather than ticket-specific text. Phase 10 adds a
  custom announcer.
- **No global focus-ring / contrast styles exist.** `styles/` contains
  per-component CSS modules. There is no `:focus-visible` rule
  forcing a visible outline, nor a tokens file enforcing contrast
  ratios. Phase 10 adds a single global stylesheet ensuring the AA
  baseline.

### Launcher (`skills/visualisation/visualise/scripts/`)

- **`launch-server.sh` resolves `<TMP_PATH>/visualiser/` and creates
  it.** `launch-server.sh:17-19` runs
  `mkdir -p "$PROJECT_ROOT/$TMP_REL/visualiser"` unconditionally — so
  even in an uninitialised project, the visualiser tmp dir gets
  created. There is no check for the `<TMP_PATH>/.gitignore` sentinel
  that `/accelerator:init` writes (per
  `skills/config/init/SKILL.md:58-77` and re-used as the sentinel
  in `scripts/config-summary.sh:19-21`). Without the sentinel check,
  the visualiser will happily run in a project where the user hasn't
  run `/accelerator:init`, leak server-info.json into a repo that
  doesn't gitignore `meta/tmp/`, and fail the
  `INIT_HINT` discipline that other skills already follow.
- **`die_json` helper exists in `launcher-helpers.sh`.** All current
  failure paths emit a single-line JSON object with `error` + `hint`.
  Phase 10's init check uses the same idiom.
- **Bash test harness pattern is established.**
  `test-launch-server.sh` + `test-helpers.sh` use the
  `assert_*` helpers from `${PLUGIN_ROOT}/scripts/test-helpers.sh` and
  exit non-zero on first failure. Phase 10 adds a new
  `test_init_sentinel_*` block to this file.

### Key Discoveries

- **`tracing-appender` does not provide size-based rotation**
  (only daily/hourly). The `file-rotate` crate
  (`https://crates.io/crates/file-rotate`) does — and exposes a
  `Write`-implementing handle that integrates cleanly with
  `tracing_subscriber::fmt::layer().with_writer(...)`. Phase 10 adds
  it to `Cargo.toml`.
- **The launcher currently redirects stdout/stderr to `server.log`** via
  `nohup ... >>"$LOG_FILE" 2>&1` (`launch-server.sh:179`). Phase 10
  must split the two writers to avoid byte-interleaved JSON corruption
  and double-counting against the rotation cap. The plan retargets the
  launcher's nohup capture to a *separate* `server.bootstrap.log` used
  only for pre-`log::init` panics and runtime aborts. The Rust process
  takes ownership of `server.log` on its own: `log::init` opens the
  file via `file-rotate`, then `main` immediately reopens fd 1 and
  fd 2 onto `/dev/null` (or onto a per-process tracing-stderr writer)
  so any post-init `eprintln!` / `panic!` payload doesn't slip into
  the rotated log via the launcher's inherited fds. The bootstrap log
  rotates by truncate-on-launcher-restart (no in-process rotation
  needed — it captures only the boot window).
- **`file-rotate` 0.7 API contract**: this plan locks in the
  constructor signature
  `FileRotate::new(path, AppendCount::new(MAX_FILES),
  ContentLimit::Bytes(MAX_BYTES), Compression::None, Some(0o600))`.
  The 5th argument (`Option<u32>` mode) is part of the signature on
  all targets and is ignored on non-unix; we pass `Some(0o600)`
  unconditionally rather than gating with `#[cfg(unix)]`. Phase 10
  also asserts the rotated-segment permission posture explicitly:
  every rotation re-applies `0o600` to the active path and any
  newly-created sibling so the owner-only posture survives the
  rename, regardless of the process umask. A 10-minute smoke against
  the real crate API is a prerequisite for Phase 10.2 — see
  Phase 10.2 prerequisites.
- **Browsers' built-in `EventSource` reconnect cannot be configured
  per-instance.** A common idiom is to wrap `EventSource` in a class
  that listens for `error` events, calls `close()`, then re-creates a
  fresh `EventSource` after a backoff timer. The custom wrapper still
  exposes the `EventSource` interface (`onmessage`, `onopen`,
  `onerror`, `close`) so `useDocEvents`'s test suite continues working
  with its existing `EventSourceFactory` injection point.
- **The wrapper must call `queryClient.invalidateQueries()` on every
  successful reconnect** to satisfy the spec's "invalidate-all on
  reconnect" rule. Implementing this inside `useDocEvents` rather than
  the wrapper keeps the wrapper a pure transport layer.
- **Header name: `Accelerator-Visualiser-Version`** (not the spec's
  `X-Accelerator-Visualiser`). RFC 6648 (2012) deprecates `X-`
  prefixes for new application headers; v1 is the cheap moment to
  fix this. The spec amendment lands alongside this phase. The
  header is purely informational; renaming is a no-op for any
  existing client (none consume it today).
- **dnd-kit's `accessibility.announcements` accepts an object of four
  callbacks** (`onDragStart`, `onDragOver`, `onDragEnd`, `onDragCancel`)
  each returning a string. The strings are written to a
  visually-hidden, `aria-live="assertive"` div that dnd-kit injects
  itself. We don't render or position it.
- **WCAG AA text contrast is 4.5:1 for normal text and 3:1 for large
  text and UI components.** The current default theme uses
  near-default browser colours (`#000` on `#fff`) for body text, which
  meets the bar. The risk areas are the muted-tag chips
  (`FrontmatterChips`), the "muted" sidebar links (Templates), and the
  conflict-banner background tint — all need an explicit ratio audit.
- **Init sentinel is the file `<TMP_PATH>/.gitignore`, not the
  directory `<TMP_PATH>/`.** Per the comment in
  `config-summary.sh:19`: "Check for tmp/.gitignore (not just tmp/) as
  the initialisation sentinel, because review-pr creates tmp/
  organically via mkdir -p." The launcher must adopt the same check.

## Desired End State

After this phase ships:

1. **Init-not-run detection.** Running `/accelerator:visualise` (or
   the CLI) in a project where `<TMP_PATH>/.gitignore` doesn't
   exist exits non-zero with a single-line JSON error of the shape
   `{"error":"accelerator not initialised","hint":"run /accelerator:init in <project-root> before launching the visualiser","project_root":"<project-root>"}`
   on stdout. The check runs *after* the existing reuse short-
   circuit (so an already-running server is not killed by a
   transient sentinel deletion) and *before* any `mkdir -p` of the
   visualiser tmp subdir for fresh launches, so a misfire leaves
   zero state behind. After `/accelerator:init` runs, re-invoking
   the launcher succeeds normally.
2. **JSON-line file logging with 5 MB rotation.** All server logs
   land as JSON lines in the file at `cfg.log_path`. When the
   active segment crosses 5 MB, `file-rotate` rolls it over to a
   numbered sibling (`server.log.1`) and resumes writing to a fresh
   `server.log`. Up to 3 rotated segments are retained; older
   segments are deleted automatically. All segments (active +
   rotated) are mode `0o600`. The launcher's nohup redirect now
   targets a separate `server.bootstrap.log`; the in-process
   writer owns `server.log` exclusively. The Rust process closes
   stdout/stderr after `log::init` so post-init `eprintln!` /
   `panic!` payload cannot interleave bytes into `server.log`. A
   request-summary log line (`method`, `uri`, `status`, `latency`)
   fires for every HTTP request.
3. **Server version response header.** Every HTTP response served
   by the visualiser carries `Accelerator-Visualiser-Version:
   <semver>` (e.g. `Accelerator-Visualiser-Version: 0.1.0`),
   regardless of route, status code, or middleware order —
   *including* responses produced by `host_header_guard` /
   `origin_guard` short-circuits. The header value is the
   `crate::VERSION` constant defined at the crate root.
4. **`/api/info` endpoint + sidebar version footer.** A new
   `GET /api/info` returns
   `{"name":"accelerator-visualiser","version":"0.1.0"}` with
   `Cache-Control: no-cache`. A new `<SidebarFooter>` component
   (extracted from `<Sidebar>` to avoid prop accretion) consumes
   `useServerInfo` and renders `Visualiser v0.1.0` in a small
   de-emphasised footer.
5. **SSE auto-reconnect with exponential backoff.** When the
   connection drops, the wrapper retries with exponential backoff
   starting at 1 s, doubling up to a cap of 30 s, with ±20%
   jitter. On every successful reconnect (including the first-
   ever-open after a from-boot connection failure), the wrapper
   runs `queryClient.invalidateQueries({ predicate })` excluding
   `'server-info'` so session-stable caches don't flicker. The
   `<SidebarFooter>` shows a "Reconnecting…" indicator while the
   wrapper is in retry mode and a transient "Reconnected —
   refreshing" message for ~3 s after recovery.
6. **Doc-page malformed-frontmatter banner.** When viewing a doc
   whose `frontmatterState === 'malformed'`, a banner appears
   above the markdown body reading
   "Warning: We couldn't read this document's metadata header;
   showing the file as-is." The banner uses no live-region role
   (it is static page-load content, not a live update); an
   `aria-label` distinguishes it for tests and SR users. The
   chip-row warning emitted by `FrontmatterChips` remains as a
   secondary signal. Both react to mid-session SSE `doc-invalid`
   events, appearing within one debounce cycle.
7. **Keyboard-navigable kanban with screen-reader announcements.**
   dnd-kit's `accessibility.announcements` is wired to ticket-
   number-prefixed strings using a colon separator (read as a
   brief pause by all major screen readers): "Picked up ticket
   0001: Three layer review system architecture", "Moved ticket
   0001: Three layer review system architecture to In progress",
   etc. Column ids are mapped to display labels via the existing
   `STATUS_COLUMNS` table. All interactive elements show a visible focus ring at
   ≥3:1 contrast (with `outline-color: Highlight` under
   `forced-colors: active`). Default theme passes WCAG 2.2 AA
   contrast for body text, sidebar links, chips, conflict banner,
   and warning banners. A token-vs-CSS drift test catches
   regressions.

### Verification

#### Server-side

- `cargo test -p accelerator-visualiser` is green (new tests for
  rotating writer, version-header middleware, info endpoint, init
  sentinel — see Inner Phases below).
- `cargo build --release` succeeds and produces a binary that, when
  fed a `config.json` and started, writes JSON lines to
  `<tmp>/visualiser/server.log` and rotates the file once it crosses
  5 MB (verifiable by writing a synthetic 5.1 MB stream of trace
  events in an integration test).
- `curl -I http://127.0.0.1:<port>/api/healthz | grep -i accelerator-visualiser-version`
  prints the version.
- `curl -I -H 'Host: evil.example' http://127.0.0.1:<port>/api/healthz`
  returns 403 *and* still carries the version header (locks layer
  ordering — see Phase 10.3).
- `curl -s http://127.0.0.1:<port>/api/info | jq` returns
  `{"name":"accelerator-visualiser","version":"<semver>"}` and the
  response carries `Cache-Control: no-cache`.
- The integration test harness confirms a request-summary log line
  appears in the rotating file for every request.

#### Frontend

- `npm test` (vitest) is green: new tests for
  `ReconnectingEventSource` backoff and reconnect-invalidate-all,
  for the malformed-frontmatter banner, for the sidebar version
  footer, and for the kanban announcement strings.
- `npm run typecheck` succeeds.
- `npm run lint` succeeds (no new warnings).
- A manual run shows `Visualiser v0.1.0` in the sidebar footer.
- Killing and restarting the server (or `kill -STOP` the server
  process briefly) shows the "reconnecting…" indicator until the
  server is back, then it disappears and queries refetch.

#### Launcher

- `./skills/visualisation/visualise/scripts/test-launch-server.sh` is
  green, including the four new sentinel + bootstrap-log tests
  listed in the Testing Strategy section.

#### Manual smoke

- **Onboarding flow**: in a fresh project that has *not* run
  `/accelerator:init`, invoke `/accelerator:visualise` from inside
  a real Claude Code session; verify the JSON error renders with
  the resolved `project_root` in the hint, run `/accelerator:init`,
  re-invoke `/accelerator:visualise`, confirm the server starts.
- Open the visualiser, navigate the kanban with `Tab` + `Space` +
  arrow keys, watch screen-reader output (VoiceOver on macOS) read
  ticket-number-prefixed announcements ("Picked up ticket 0001:
  Foo", "Moved ticket 0001: Foo to In progress").
- Open the visualiser, navigate to a fixture doc whose frontmatter
  is deliberately malformed, see the prominent doc-page banner
  with its "Warning:" prefix.
- Pause the server (Ctrl-Z in the launcher's terminal), watch the
  sidebar footer show "Reconnecting…", resume (`fg`), see a brief
  "Reconnected — refreshing" message, then a clean state. The
  sidebar version footer does NOT flicker during reconnect.
- Run the server, tail `<tmp>/visualiser/server.log` while loading
  pages, see structured JSON request-summary lines (with `method`,
  `uri`, `status`, `latency`) and FS-event lines flowing. Confirm
  `<tmp>/visualiser/server.bootstrap.log` exists and is empty
  post-init.
- Enable Windows High Contrast (or macOS "Increase contrast") and
  confirm focus rings repaint correctly under forced-colors mode.

## What We're NOT Doing

- **No new feature surface.** Phase 10 is hardening, not authoring.
- **No new write paths.** The only write surface remains the kanban
  status mutation introduced in Phase 8.
- **No alternative log destinations** (syslog, journald, structured
  log shipping). One file, locally rotated, is the v1 envelope.
- **No log compression on rotation.** `file-rotate`'s default plain
  rotation is fine; gzip-on-rotate is a future optimisation.
- **No runtime-configurable rotation thresholds.** 5 MB is hard-coded
  per the spec. Operators who need a different size can
  `truncate(1)` or restart.
- **No window of native `EventSource` polyfill or Web-Workers
  shenanigans.** The wrapper uses the native `EventSource` API as its
  underlying transport — only the close/reconnect/backoff state is
  bespoke.
- **No "reconnect storm" mitigation across browser tabs.** Each tab
  reconnects independently. Multi-tab coordination is post-v1.
- **No live-update indicator on individual sidebar doc-type badges**
  (the spec calls for "a subtle pulse on the sidebar badge of any doc
  type with unseen changes"). That's a Phase 11+ polish item — Phase 10
  delivers the global "Reconnecting…" / "Reconnected — refreshing"
  indicator pair only.
- **No collapsed/mobile sidebar.** v1 assumes the sidebar is always
  visible. The reconnecting indicator and version footer live
  inside it; if a future phase introduces a collapsed sidebar
  state, the indicator must move to a fixed-position toast region
  to remain visible.
- **No version-footer link to release notes.** Plain text only in
  v1; linking to the GitHub release for the current tag is a
  post-v1 ergonomics improvement.
- **No editor-launching from the malformed banner.** The
  "Open in editor" affordance already exists in the file aside; the
  banner just informs.
- **No global theming overhaul.** Phase 10 audits and fixes the
  contrast / focus-ring failures only; full design tokenisation is
  out of scope.
- **No automated WCAG audit tooling integration.** Manual contrast
  checks against the WebAIM contrast formula are the v1 bar; running
  `axe-core` in CI is post-v1.
- **No platform-specific logging tweaks** (e.g. UNC paths, Windows
  event log). The plugin targets macOS/Linux only.
- **No retroactive rewriting of older log files.** Rotation kicks in
  on first cross of 5 MB after the new build ships.

## Implementation Approach

Seven inner phases, ordered so that each is independently shippable
and the build stays green between them. TDD throughout: each step
introduces a failing test, then makes it pass. The dependency graph
is small — the only ordering constraint is that 10.4 (info endpoint
+ sidebar footer) reads from the version surface introduced in 10.3
(response header) for *cross-checking* in tests, but neither inner
phase strictly depends on the other in production code. We sequence
them in an ergonomic build-up order:

1. **10.1** — launcher init-not-run sentinel (smallest scope, pure
   bash, no Rust changes).
2. **10.2** — file-backed JSON logging with 5 MB rotation
   (server-only).
3. **10.3** — `Accelerator-Visualiser-Version` response header
   (server-only middleware; ordering critical — must be outermost).
4. **10.4** — `/api/info` endpoint + sidebar version footer
   (server endpoint + frontend hook + sidebar wiring).
5. **10.5** — `ReconnectingEventSource` wrapper + predicate-scoped
   invalidate-on-reconnect + `<SidebarFooter>` reconnecting /
   reconnected indicators (frontend-only).
6. **10.6** — malformed-frontmatter doc-page banner
   (frontend-only).
7. **10.7** — focus rings, dnd-kit announcements, contrast pass
   (frontend-only).

---

## Phase 10.1: Launcher init-not-run sentinel

### Overview

Reject launches in projects that haven't run `/accelerator:init` (no
`<TMP_PATH>/.gitignore`). Emit a single-line JSON error with a clear
remediation hint that names the resolved project root. Run *after*
the existing reuse short-circuit (so an already-running server is
not killed by a transient sentinel deletion) and *before* any
directory creation, binary download, or PID-file write — so a fresh-
launch misfire leaves zero state behind.

### Prerequisites — fix the existing harness first

Before the new tests can be written, the existing test harness must
be updated so the rest of `test-launch-server.sh` keeps passing once
the sentinel check is in place:

1. **Update `make_project()` in `test-launch-server.sh:26`** to also
   create the sentinel file:
   ```bash
   make_project() {
     local d="$1"
     mkdir -p "$d/.jj" "$d/.claude" "$d/meta/tmp"
     : > "$d/meta/tmp/.gitignore"
   }
   ```
   This single change makes every existing happy-path test compatible
   with the new sentinel check. Without it, all 12 existing tests will
   start rejecting launches with the new init-not-run error.
2. **Confirm `assert_dir_absent` exists** in
   `scripts/test-helpers.sh`. If not, add a one-liner next to its
   `assert_dir_exists` counterpart:
   ```bash
   assert_dir_absent() {
     local label="$1" path="$2"
     if [ -d "$path" ]; then
       echo "FAIL: $label — directory exists: $path" >&2
       exit 1
     fi
   }
   ```
3. **Note**: there is no `make_stub_binary` helper. The existing
   `make_fake_visualiser` helper in `test-helpers.sh:19-92` is the
   established pattern; the new test reuses it.

### TDD Steps

#### Step 10.1.a: Failing tests

**File**: `skills/visualisation/visualise/scripts/test-launch-server.sh`

Add a new test block (named to match the existing snake_case
`test_<feature>` convention):

```bash
test_uninitialised_project_is_rejected() {
  local proj="$TMPDIR_BASE/uninit"
  # NB: do not call make_project here — that helper now creates the
  # sentinel. Construct the project tree by hand to leave the sentinel
  # absent.
  mkdir -p "$proj/.jj" "$proj/.claude" "$proj/meta/tmp"
  cd "$proj"

  set +e
  out="$("$LAUNCH_SERVER" 2>&1)"
  local rc=$?
  set -e

  # Existing harness convention is `assert_eq "<label>" "<expected>" "<actual>"`.
  assert_eq "uninit: exit code" "1" "$rc"
  local err hint
  err="$(echo "$out" | jq -r '.error // empty' 2>/dev/null)"
  hint="$(echo "$out" | jq -r '.hint // empty' 2>/dev/null)"
  assert_eq "uninit: error field"   "accelerator not initialised" "$err"
  assert_contains "uninit: hint mentions /accelerator:init" "$hint" "/accelerator:init"
  assert_contains "uninit: hint mentions resolved root"     "$hint" "$proj"
  assert_dir_absent "uninit: no tmp subdir created" "$proj/meta/tmp/visualiser"
}

test_initialised_project_proceeds_past_sentinel() {
  local proj="$TMPDIR_BASE/init-ok"
  make_project "$proj"   # now seeds meta/tmp/.gitignore
  cd "$proj"

  # Use the existing make_fake_visualiser helper — same pattern every
  # other happy-path test uses. The fake binary writes a stubbed
  # server-info.json and exits cleanly.
  local fake
  fake="$(make_fake_visualiser "$TMPDIR_BASE/fake-bin")"
  set +e
  ACCELERATOR_VISUALISER_BIN="$fake" "$LAUNCH_SERVER" >/dev/null 2>&1
  local rc=$?
  set -e
  # The sentinel must not reject; subsequent steps (binary fetch /
  # exec) may still legitimately fail or succeed depending on the
  # fake's behaviour. The only assertion is "did not exit with the
  # init-not-run code path."
  assert_eq "init-ok: did not reject at sentinel" "0" "$rc"
}

test_sentinel_does_not_kill_already_running_server() {
  # Regression: deleting meta/tmp/.gitignore mid-session must not
  # lock the user out of an already-running server (the reuse
  # short-circuit must run before the sentinel check).
  local proj="$TMPDIR_BASE/init-then-deleted"
  make_project "$proj"
  cd "$proj"

  # Seed a fake "live server". Critical detail: the launcher's reuse
  # short-circuit verifies (pid, start_time) identity by reading the
  # *actual* process start time from /proc (Linux) or `ps -o lstart=`
  # (macOS) and comparing against the seeded value — so we MUST seed
  # `start_time` from `start_time_of $$` (the launcher's own helper),
  # not from `date +%s`. Otherwise the identity check always fails
  # and the test silently exercises the wrong code path.
  source "$SCRIPT_DIR/launcher-helpers.sh"

  mkdir -p "$proj/meta/tmp/visualiser"
  printf '%s\n' "$$" > "$proj/meta/tmp/visualiser/server.pid"
  local seeded_start
  seeded_start="$(start_time_of "$$")"
  jq -nc --arg url "http://127.0.0.1:65535/" \
        --arg start "$seeded_start" \
        '{url:$url,start_time:($start|tonumber)}' \
        > "$proj/meta/tmp/visualiser/server-info.json"

  # User deletes the sentinel after init (e.g. via `git clean`).
  rm -f "$proj/meta/tmp/.gitignore"

  set +e
  out="$("$LAUNCH_SERVER" 2>&1)"
  local rc=$?
  set -e
  # The reuse short-circuit returns the URL and exits 0; the sentinel
  # check never fires.
  assert_eq "reuse-after-sentinel-delete: exit code" "0" "$rc"
  assert_contains "reuse-after-sentinel-delete: prints URL" "$out" "http://127.0.0.1:65535/"
}
```

All three tests fail today — the launcher creates
`meta/tmp/visualiser/` unconditionally before any sentinel check, so
`test_uninitialised_project_is_rejected` fails on the directory-
absent assertion.

#### Step 10.1.b: Make the tests pass

**File**: `skills/visualisation/visualise/scripts/launch-server.sh`

Insert the sentinel check **after** the existing reuse short-circuit
(`launch-server.sh:46-58`) and **before** the first `mkdir -p
"$TMP_DIR"`. The exact insertion point is between the `rm -f
"$STOPPED"` line and the platform-detection block:

```bash
# (existing reuse short-circuit ran first; if it returned, we never
# reach this code.)
rm -f "$STOPPED"

# Init sentinel: only check on the fresh-launch path. A user who
# deleted meta/tmp/.gitignore mid-session can still re-attach to a
# live server via the reuse short-circuit above.
SENTINEL="$PROJECT_ROOT/$TMP_REL/.gitignore"
if [ ! -f "$SENTINEL" ]; then
  die_json "$(jq -nc \
    --arg error 'accelerator not initialised' \
    --arg hint  "run /accelerator:init in $PROJECT_ROOT before launching the visualiser" \
    --arg root  "$PROJECT_ROOT" \
    '{error:$error,hint:$hint,project_root:$root}')"
fi

# (existing platform detection follows)
```

The `project_root` field gives the slash-command renderer a stable
key to surface back to the user, in case the hint string ever needs
to grow more terse.

#### Step 10.1.c: Run + verify

```
./skills/visualisation/visualise/scripts/test-launch-server.sh
```

All three new tests pass. The 12 existing happy-path tests continue
to pass because `make_project()` now seeds the sentinel.

### Success Criteria

#### Automated Verification

- [ ] `./skills/visualisation/visualise/scripts/test-launch-server.sh`
  passes including the two new tests.
- [ ] `./skills/visualisation/visualise/scripts/test-stop-server.sh`
  remains green.
- [ ] `./skills/visualisation/visualise/scripts/test-cli-wrapper.sh`
  remains green.
- [ ] `cargo test -p accelerator-visualiser` remains green
  (no Rust changes in this inner phase).

#### Manual Verification

- [ ] In a fresh project without `/accelerator:init`, running
  `/accelerator:visualise` prints the JSON error with `/accelerator:init`
  hint and exits non-zero.
- [ ] After running `/accelerator:init`, `/accelerator:visualise` starts
  the server normally.

---

## Phase 10.2: File-backed JSON logging with 5 MB rotation

### Overview

Replace `tracing_subscriber::fmt().json().init()` (stdout) with a
file-backed JSON layer that writes to `cfg.log_path` and rotates at
5 MB, keeping the most recent 3 segments. Split the launcher's
nohup capture to a separate `server.bootstrap.log` so only one
writer ever touches `server.log`. Add a `tower_http::trace` layer
so every HTTP request is summarised in the log.

### Prerequisites — verify the file-rotate 0.7 API

Before writing any code in this phase, run a 10-minute smoke against
the real `file-rotate` 0.7 API to lock in three contracts the rest
of this phase depends on:

1. Constructor signature is exactly
   `FileRotate::new(path, suffix_scheme, content_limit, compression,
   mode)` with the `mode: Option<u32>` parameter present on all
   targets (ignored on non-unix). Pass `Some(0o600)` unconditionally
   — the `#[cfg(unix)]` attribute does not apply to a single function
   argument.
2. `Write::write` triggers rotation synchronously when the cumulative
   write count crosses `ContentLimit::Bytes(N)`, without requiring
   an explicit `flush()` call.
3. Newly-created rotated siblings (`server.log.1`, `server.log.2`)
   inherit the file mode from `mode`, *not* the process umask. If
   the crate doesn't enforce mode on rotation, plan for an explicit
   `chmod` post-rotation step (Phase 10.2.b's `make_writer` wraps
   the writer to enforce this; see below).

Document the verified shape in a comment at the top of `log.rs` so
future maintainers don't have to re-derive it.

### Dependencies

Add to `skills/visualisation/visualise/server/Cargo.toml`:

```toml
file-rotate = "0.7"
tracing-appender = "0.2"
libc = "0.2"        # confirm: only add if not already a direct dep —
                    # used by redirect_std_streams_to_devnull (10.2.c).
                    # If `libc` is currently transitive only, this
                    # entry is required for the new `unsafe { libc::dup2 }`
                    # call to compile. If `libc` is already direct, the
                    # entry is a no-op and the line can be omitted.
# tracing-subscriber: existing entry already covers `json` + `env-filter`.
# tower-http: already has the `trace` feature.
```

After bumping, run `cargo audit` (one-time gate; fold into CI in
Phase 11/12 as appropriate). Both crates are mature and widely used
but the dependency-graph delta warrants a one-time check.

### TDD Steps

#### Step 10.2.a: Failing tests — `make_writer` (subscriber-free)

**File**: `skills/visualisation/visualise/server/src/log.rs` (new)

Test the writer directly via `Write::write_all`. No subscriber is
installed — these tests are pure-function tests of the rotation
behaviour and live alongside the production module.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn count_segments(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let s = e.file_name().to_string_lossy().into_owned();
                s.starts_with("server.log.")
            })
            .count()
    }

    #[test]
    fn rotates_when_active_segment_crosses_5_megabytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();

        // Write enough bytes to *guarantee* rotation regardless of
        // whether file-rotate counts pre- or post-write.
        let line = b"x\n";
        let target = MAX_BYTES + 64 * 1024; // 5 MiB + 64 KiB
        let mut written = 0usize;
        while written < target {
            writer.write_all(line).unwrap();
            written += line.len();
        }
        writer.flush().unwrap();
        drop(writer);

        assert!(path.exists(), "active log must exist");
        let rotated = count_segments(dir.path());
        assert_eq!(rotated, 1, "expected exactly one rotated segment");

        // Active segment should be substantially smaller than the cap.
        let active_len = std::fs::metadata(&path).unwrap().len() as usize;
        assert!(
            active_len < MAX_BYTES,
            "active segment ({active_len}B) should be under cap ({MAX_BYTES}B) post-rotation"
        );
    }

    #[test]
    fn retains_at_most_three_rotated_segments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();
        let line = b"x\n";
        // Force five rotations.
        for _ in 0..5 {
            let mut written = 0usize;
            while written < MAX_BYTES + 64 * 1024 {
                writer.write_all(line).unwrap();
                written += line.len();
            }
            writer.flush().unwrap();
        }
        drop(writer);

        let rotated = count_segments(dir.path());
        assert!(
            rotated <= MAX_FILES,
            "rotated segment count ({rotated}) must not exceed MAX_FILES ({MAX_FILES})",
        );
    }

    #[cfg(unix)]
    #[test]
    fn active_and_rotated_segments_are_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();
        let line = b"x\n";
        let mut written = 0usize;
        while written < MAX_BYTES + 64 * 1024 {
            writer.write_all(line).unwrap();
            written += line.len();
        }
        writer.flush().unwrap();
        drop(writer);

        for entry in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let meta = entry.metadata().unwrap();
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(
                mode, 0o600,
                "{} must be owner-only (mode 0o600), got 0o{mode:o}",
                entry.file_name().to_string_lossy(),
            );
        }
    }

    #[test]
    fn make_writer_errors_when_parent_path_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let bad_path = blocker.join("server.log"); // parent is a file
        let err = make_writer(&bad_path).unwrap_err();
        assert!(
            matches!(err, LoggingError::CreateDir { .. }),
            "expected CreateDir error, got {err:?}",
        );
    }

    #[test]
    fn pre_existing_oversized_log_rotates_on_first_write() {
        // Migration safety: an upgrade from a Phase 1–9 build leaves
        // an arbitrarily large server.log on disk. The new writer must
        // not panic or truncate; a single byte should trigger rotation.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        std::fs::write(&path, vec![b'x'; MAX_BYTES + 1024]).unwrap();
        let mut writer = make_writer(&path).unwrap();
        writer.write_all(b"trigger\n").unwrap();
        writer.flush().unwrap();
        drop(writer);
        // After the trigger write, at least one rotated segment exists.
        assert!(count_segments(dir.path()) >= 1);
    }
}
```

#### Step 10.2.a-bis: Test-support helper + JSON-shape test

The JSON-shape test and the request-summary trace test (Phase
10.2.e) both need a `MakeWriter` impl that wraps a `Write`-able
`FileRotate` so they can install a scoped JSON subscriber that
writes to a known file. Rather than inlining the boilerplate twice
(or wrestling with the `'a` lifetime on a `MutexGuard` returned from
a closure — which doesn't satisfy `tracing-subscriber`'s
`for<'a> MakeWriter<'a>` contract), we extract one helper module
once. **This is required, not optional.**

**File**: `skills/visualisation/visualise/server/src/log.rs` (extend
with a `pub(crate) test_support` module)

```rust
#[cfg(test)]
pub(crate) mod test_support {
    use std::io::Write;
    use std::sync::{Arc, Mutex, MutexGuard};
    use tracing_subscriber::fmt::MakeWriter;

    /// `MakeWriter` impl over an `Arc<Mutex<W>>`. Returns a
    /// `MutexGuardWriter` per event; the guard's lifetime is tied
    /// to `&self` (the borrow in `MakeWriter::make_writer`), which
    /// satisfies the `for<'a> MakeWriter<'a>` contract because
    /// `MutexGuardWriter<'a, W>` borrows from `&'a Mutex<W>`.
    pub struct MutexWriter<W: Write + Send + 'static>(Arc<Mutex<W>>);

    impl<W: Write + Send + 'static> MutexWriter<W> {
        pub fn new(w: W) -> Self { Self(Arc::new(Mutex::new(w))) }
    }

    pub struct MutexGuardWriter<'a, W: Write + Send + 'static>(MutexGuard<'a, W>);

    impl<W: Write + Send + 'static> Write for MutexGuardWriter<'_, W> {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { self.0.write(b) }
        fn flush(&mut self) -> std::io::Result<()> { self.0.flush() }
    }

    impl<'a, W: Write + Send + 'static> MakeWriter<'a> for MutexWriter<W> {
        type Writer = MutexGuardWriter<'a, W>;
        fn make_writer(&'a self) -> Self::Writer {
            MutexGuardWriter(self.0.lock().expect("MutexWriter poisoned"))
        }
    }

    /// Build a JSON-formatted scoped subscriber writing to `path`.
    /// Returned subscriber is intended for use with
    /// `tracing::subscriber::with_default(...)`.
    pub fn build_test_json_subscriber(
        path: &std::path::Path,
    ) -> impl tracing::Subscriber + Send + Sync {
        let writer = super::make_writer(path).expect("make_writer");
        tracing_subscriber::fmt()
            .json()
            .with_writer(MutexWriter::new(writer))
            .finish()
    }
}
```

**File**: `skills/visualisation/visualise/server/src/log.rs` (add the
JSON-shape test using the helper)

```rust
#[test]
fn emits_json_lines_with_message_and_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.log");
    let subscriber = test_support::build_test_json_subscriber(&path);

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(field = "value", "hello");
    });

    let body = std::fs::read_to_string(&path).unwrap();
    let line = body.lines().next().expect("at least one log line");
    let v: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_eq!(v["fields"]["message"], "hello");
    assert_eq!(v["fields"]["field"], "value");
}
```

The Phase 10.2.e trace-summary test uses
`test_support::build_test_json_subscriber` directly — same helper,
no duplication. Verify the `MakeWriter` impl compiles against the
pinned `tracing-subscriber 0.3` API surface before locking the
plan; the `for<'a> MakeWriter<'a>` shape with an `'a`-bound
`Self::Writer` is the documented pattern in `tracing-subscriber`'s
own examples.

#### Step 10.2.b: Implementation — `log.rs`

```rust
use std::path::Path;
use file_rotate::{compression::Compression, suffix::AppendCount, ContentLimit, FileRotate};
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("failed to create log directory {path}: {source}")]
    CreateDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("logging subscriber already installed")]
    AlreadyInitialised,
}

pub(crate) const MAX_BYTES: usize = 5 * 1024 * 1024;
pub(crate) const MAX_FILES: usize = 3;

pub fn make_writer(path: &Path) -> Result<FileRotate<AppendCount>, LoggingError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| LoggingError::CreateDir {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    // file-rotate 0.7: the `mode` parameter is part of the constructor
    // signature on all targets; ignored on non-unix. Always pass
    // Some(0o600) — owner-only matches server-info.json / pid file.
    Ok(FileRotate::new(
        path,
        AppendCount::new(MAX_FILES),
        ContentLimit::Bytes(MAX_BYTES),
        Compression::None,
        Some(0o600),
    ))
}

pub fn init(log_path: &Path) -> Result<WorkerGuard, LoggingError> {
    let writer = make_writer(log_path)?;
    let (nb, guard) = tracing_appender::non_blocking(writer);
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(nb)
        .try_init()
        .map_err(|_| LoggingError::AlreadyInitialised)?;
    Ok(guard)
}
```

Notes:
- `try_init` (not `init`) so a duplicate install surfaces as
  `LoggingError::AlreadyInitialised` instead of a panic.
- No `with_ansi(false)` — the JSON formatter ignores ANSI settings.
- If the file-rotate 0.7 prerequisite check shows that rotated
  siblings *don't* inherit the `mode` argument, wrap the writer in a
  small adapter that `chmod`s the active path after every rotation
  (file-rotate exposes a `Notify` callback for this).

#### Step 10.2.c: Wire into `main.rs`

```rust
use accelerator_visualiser::{config::Config, log, server};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let cfg = match Config::from_path(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::from(2);
        }
    };

    let _log_guard = match log::init(&cfg.log_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("failed to init logging: {e}");
            return ExitCode::from(2);
        }
    };

    // Hand off stdout/stderr ownership: the launcher's
    // `>>"$LOG_FILE.bootstrap"` capture only sees the brief window
    // between exec and this point. Post-init writes go nowhere via
    // the inherited fds — only the in-process JSON writer touches
    // server.log. This eliminates the dual-writer hazard on
    // server.log and keeps any post-init `eprintln!` / `panic!`
    // payload from corrupting JSON lines or sneaking past the size
    // cap into the rotated segment. /dev/null open failure is a
    // hard error: silently leaving inherited fds in place would
    // re-enable the dual-writer hazard.
    if let Err(e) = redirect_std_streams_to_devnull() {
        tracing::error!(error = %e, "failed to redirect std streams to /dev/null");
        return ExitCode::from(2);
    }

    info!(
        config = %cli.config.display(),
        log_path = %cfg.log_path.display(),
        "bootstrapping server"
    );

    let info_path = cfg.tmp_path.join("server-info.json");
    let result = server::run(cfg, &info_path).await;
    // Log the server error via tracing BEFORE dropping the guard,
    // so it actually lands in the rotating log file (not /dev/null).
    if let Err(ref e) = result {
        tracing::error!(error = %e, "server error");
    }
    // Explicitly drop the guard here so the worker thread flushes
    // before main returns. (Drop runs at scope-end either way; this
    // makes the ordering visible to readers.)
    drop(_log_guard);
    if result.is_err() { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

/// Re-open fds 1 and 2 onto /dev/null so post-`log::init`
/// `eprintln!` / `panic!` payload cannot interleave bytes into the
/// rotating `server.log`. `dup2` duplicates the open file
/// description, so the kernel keeps the /dev/null reference alive
/// after the local `devnull` File goes out of scope (its drop only
/// closes the *original* fd, which we no longer use). MUST be
/// called *after* `log::init` — tracing writes to the FileRotate
/// handle directly via the channel, never via stdio, so closing
/// fd 2 has no impact on logging.
#[cfg(unix)]
fn redirect_std_streams_to_devnull() -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let devnull = std::fs::OpenOptions::new().write(true).open("/dev/null")?;
    let fd = devnull.as_raw_fd();
    // SAFETY: fd is a valid file descriptor we just opened. dup2
    // accepts any positive target fd (1, 2 always exist in a
    // well-formed unix process). On error, dup2 returns -1; we
    // surface a generic IO error so main() can exit with the same
    // exit-2 path it uses for other bootstrap failures.
    let r1 = unsafe { libc::dup2(fd, 1) };
    let r2 = unsafe { libc::dup2(fd, 2) };
    if r1 == -1 || r2 == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
#[cfg(not(unix))]
fn redirect_std_streams_to_devnull() -> std::io::Result<()> { Ok(()) }
```

Update the call site in `main` to treat `/dev/null`-open failure
as a hard error (rather than silently leaving the dual-writer
hazard in place — which would silently let stderr panic payloads
slip into the rotated `server.log` segment via the launcher's
inherited fds):

```rust
if let Err(e) = redirect_std_streams_to_devnull() {
    tracing::error!(error = %e, "failed to redirect std streams to /dev/null");
    return ExitCode::from(2);
}
```

Notes:
- The shutdown path returns from `main` (no `process::exit`), so
  `_log_guard.drop()` runs synchronously and joins the
  `tracing-appender` worker thread, flushing pending lines.
- The trailing `eprintln!("server error: {e}")` after the guard
  drop is removed in favour of a `tracing::error!(...)` call
  *before* `drop(_log_guard)`, so the shutdown error lands in the
  rotating log instead of being silently dropped to /dev/null.

#### Step 10.2.c-bis: Tests for `redirect_std_streams_to_devnull`

The redirect is critical to the dual-writer-hazard fix and must
not silently regress. Two tests, both `#[cfg(unix)]`:

```rust
#[cfg(unix)]
#[test]
fn redirect_std_streams_to_devnull_succeeds_on_normal_unix() {
    // Run in-process — dup2 on fds 1/2 within the test runner is
    // safe because cargo test captures stdout/stderr separately
    // via its own pipes (and we only redirect *write* targets).
    // We can't easily assert stdout is now /dev/null without
    // forking, but we CAN assert the function returns Ok and a
    // subsequent write to fd 2 does not raise an error.
    redirect_std_streams_to_devnull().expect("redirect should succeed");
    use std::io::Write;
    // After redirect, writes to stderr go to /dev/null and succeed
    // silently. (The cargo test harness's captured stderr is no
    // longer the destination — that's the whole point.)
    writeln!(std::io::stderr(), "post-redirect sentinel").expect("write should succeed");
}

#[cfg(unix)]
#[test]
fn redirect_std_streams_to_devnull_in_subprocess_isolates_stderr() {
    // Hermetic version: spawn a child process, have it run the
    // redirect and then write a sentinel to stderr; assert the
    // parent captures NO sentinel on the child's stderr pipe.
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(&exe)
        .env("VISUALISER_TEST_REDIRECT_HARNESS", "1")
        .output()
        .expect("spawn child");
    assert!(out.status.success(), "child exited non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("REDIRECT_SENTINEL_xyzzy"),
        "post-redirect stderr leaked into child's captured stderr: {stderr}",
    );
}

// Subprocess entry point — invoked when the env var is set. The
// child re-execs cargo's test binary; the same `main`-style
// pre-redirect / post-redirect sequence runs.
#[cfg(unix)]
#[ctor::ctor]   // runs before fn main / before any #[test]
fn maybe_run_redirect_harness() {
    if std::env::var("VISUALISER_TEST_REDIRECT_HARNESS").is_ok() {
        // Pre-redirect sentinel — should appear in captured stderr.
        eprintln!("PRE_REDIRECT_SENTINEL_visible");
        redirect_std_streams_to_devnull().expect("redirect");
        // Post-redirect sentinel — must NOT appear.
        eprintln!("REDIRECT_SENTINEL_xyzzy");
        std::process::exit(0);
    }
}
```

(`ctor` is a tiny crate that runs static init before `main`. If
adding it is unwelcome, replace `#[ctor::ctor]` with a `build.rs`-
emitted `#[cfg(test)]` constant that the test reads via env var to
gate its own pre-redirect behaviour. Either pattern keeps the test
hermetic.)

#### Step 10.2.d: Update the launcher to use a separate bootstrap log

**File**: `skills/visualisation/visualise/scripts/launch-server.sh`

The launcher's nohup line currently appends stdout/stderr to
`server.log`. Retarget to `server.bootstrap.log` so the in-process
writer owns `server.log` exclusively. Pre-create the bootstrap log
at mode `0o600` *before* the nohup append so it inherits the same
owner-only posture as the rest of `<tmp>/visualiser/` (the launcher
shell's default umask would otherwise create the file at `0o644`,
making backtraces and panic payloads world-readable on multi-user
hosts):

```bash
# (Was: nohup "$BIN" --config "$CFG" >> "$LOG_FILE" 2>&1 &)
BOOTSTRAP_LOG="$LOG_FILE.bootstrap"
# Truncate-on-launch so the bootstrap log captures only this single
# boot window; an empty pre-create + chmod gives the file 0o600 mode
# regardless of the user's umask.
: > "$BOOTSTRAP_LOG"
chmod 0600 "$BOOTSTRAP_LOG"
nohup "$BIN" --config "$CFG" >> "$BOOTSTRAP_LOG" 2>&1 &
```

Where `LOG_FILE` was set earlier as `$TMP_DIR/server.log`.
`server.bootstrap.log` lives next to `server.log` under
`<tmp>/visualiser/`, captures only pre-`log::init` panics and the
brief race between exec and stream redirect, and is truncated on
each launch (no in-process rotation needed — single boot window).

Add launcher tests exercising the rename and the file mode:

```bash
test_bootstrap_log_is_separate_and_exclusive_of_server_log() {
  local proj="$TMPDIR_BASE/bootstrap-split"
  make_project "$proj"
  cd "$proj"
  # Fake binary prints a known sentinel to stderr — must land in
  # bootstrap.log only, never in server.log.
  local fake
  fake="$(make_fake_visualiser "$TMPDIR_BASE/fake-bootstrap" \
            --stderr 'BOOTSTRAP_SENTINEL_xyzzy')"
  ACCELERATOR_VISUALISER_BIN="$fake" "$LAUNCH_SERVER" >/dev/null 2>&1 || true

  assert_file_present "bootstrap log present" \
    "$proj/meta/tmp/visualiser/server.bootstrap.log"
  assert_file_contains "sentinel in bootstrap.log" \
    "$proj/meta/tmp/visualiser/server.bootstrap.log" "BOOTSTRAP_SENTINEL_xyzzy"
  # server.log either does not exist (fake didn't initialise tracing)
  # or exists without the sentinel — the dual-writer hazard is fixed.
  if [ -f "$proj/meta/tmp/visualiser/server.log" ]; then
    assert_file_not_contains "sentinel NOT in server.log" \
      "$proj/meta/tmp/visualiser/server.log" "BOOTSTRAP_SENTINEL_xyzzy"
  fi
}

test_bootstrap_log_is_owner_only() {
  local proj="$TMPDIR_BASE/bootstrap-mode"
  make_project "$proj"
  cd "$proj"
  local fake
  fake="$(make_fake_visualiser "$TMPDIR_BASE/fake-mode")"
  # Force a permissive umask in the test shell to prove the launcher's
  # explicit chmod overrides it.
  ( umask 022
    ACCELERATOR_VISUALISER_BIN="$fake" "$LAUNCH_SERVER" >/dev/null 2>&1 || true
  )
  local mode
  mode="$(stat_mode "$proj/meta/tmp/visualiser/server.bootstrap.log")"
  assert_eq "bootstrap.log mode" "600" "$mode"
}
```

`stat_mode` is a small helper that wraps the macOS / Linux
divergence (`stat -f '%Lp'` vs `stat -c '%a'`) — add it to
`test-helpers.sh` if not already present. `make_fake_visualiser`
already accepts a `--stderr` flag for emitting a fixed string;
extend it once if the existing helper doesn't take that flag.
`assert_file_contains` / `assert_file_not_contains` are one-line
wrappers around `grep -q`.

#### Step 10.2.e: Add request-summary trace layer

**File**: `skills/visualisation/visualise/server/src/server.rs`

Add to the layer stack in `build_router_with_spa`. Position is
inside the body-limit / timeout layers but *outside* the activity
middleware so the latency reflects handler time only (compression
and version-header sit outside, see Phase 10.3 for the latter):

```rust
// TraceLayer position — INSIDE compression and version_header so
// recorded latency excludes compression cost. Move it outward later
// only if we want to log post-compression body size.
.layer(
    tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(
            tower_http::trace::DefaultMakeSpan::new()
                .level(tracing::Level::INFO)
                .include_headers(false),
        )
        .on_response(
            tower_http::trace::DefaultOnResponse::new()
                .level(tracing::Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Millis),
        ),
)
```

Test (`server.rs:tests`):

```rust
#[tokio::test]
async fn request_emits_trace_summary_line_with_method_uri_status_and_latency() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt as _;

    let dir = tempfile::tempdir().unwrap();
    let log_path = dir.path().join("server.log");
    // Scoped subscriber wired through the shared MutexWriter helper
    // from log::test_support — no global state, no test-ordering
    // hazards, no MakeWriter boilerplate duplication.
    let subscriber = crate::log::test_support::build_test_json_subscriber(&log_path);

    tracing::subscriber::with_default(subscriber, || {
        // Use a tokio current-thread runtime via block_on so the
        // scoped subscriber stays in scope for the request.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let dir2 = tempfile::tempdir().unwrap();
                let state = build_minimal_state(dir2.path()).await;
                let app = build_router(state);
                let _ = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/healthz")
                            .header("host", "127.0.0.1")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
            });
    });

    // Parse each captured line as JSON and assert structural keys
    // are present. tower-http 0.5's exact field placement (e.g.
    // `status` numeric vs "200 OK", `latency` numeric vs "42 ms")
    // varies across patch versions — structural-key matching is
    // more resilient than substring search.
    //
    // PREREQUISITE: before locking these assertions, run a short
    // smoke against tower-http 0.5 with the JSON subscriber and
    // pin the exact emitted shape (numeric vs string for `status`
    // / `latency`, exact field names: `span.method` / `fields.method`,
    // etc.).
    let body = std::fs::read_to_string(&log_path).unwrap();
    let mut saw_summary = false;
    for line in body.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let has_method = v.pointer("/span/method").and_then(|m| m.as_str()) == Some("GET")
            || v.pointer("/fields/method").and_then(|m| m.as_str()) == Some("GET");
        let has_uri = v.pointer("/span/uri").and_then(|u| u.as_str()) == Some("/api/healthz")
            || v.pointer("/fields/uri").and_then(|u| u.as_str()) == Some("/api/healthz");
        if has_method && has_uri { saw_summary = true; break; }
    }
    assert!(saw_summary, "no trace-summary line found in log:\n{body}");
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p accelerator-visualiser` is green, including:
  - `log::tests::rotates_when_active_segment_crosses_5_megabytes`
  - `log::tests::retains_at_most_three_rotated_segments`
  - `log::tests::active_and_rotated_segments_are_owner_only`
  - `log::tests::make_writer_errors_when_parent_path_is_a_file`
  - `log::tests::pre_existing_oversized_log_rotates_on_first_write`
  - `log::tests::emits_json_lines_with_message_and_field`
  - `server::tests::request_emits_trace_summary_line_with_method_uri_status_and_latency`
- [ ] `./skills/visualisation/visualise/scripts/test-launch-server.sh`
  passes `test_bootstrap_log_separate_from_server_log`.
- [ ] `cargo build --release` succeeds.
- [ ] `cargo build --release --features dev-frontend` succeeds.
- [ ] `cargo audit` (one-time gate post-bump) reports no advisories.

#### Manual Verification

- [ ] Run the server, hit a few endpoints, `tail` the log file —
  see JSON lines with `method`, `uri`, `status`, `latency`.
- [ ] Start the server and pipe synthetic traffic to grow the log past
  5 MB; observe `server.log.1` appear, `server.log` reset to small,
  and both files at mode `0o600` (`stat -f '%Lp' server.log*` on
  macOS, `stat -c '%a'` on Linux).
- [ ] Stop the server cleanly; confirm `_log_guard`'s drop flushed all
  pending lines (no missing trailing requests).
- [ ] Confirm `server.bootstrap.log` exists in `<tmp>/visualiser/`
  and contains nothing post-`log::init` (only the brief
  exec→redirect window).

---

## Phase 10.3: `Accelerator-Visualiser-Version` response header

### Overview

Add a tiny middleware that sets `Accelerator-Visualiser-Version:
<CARGO_PKG_VERSION>` on every response, including responses
produced by guard-rejection short-circuits (host-header guard,
origin guard) and the 404 fallback. The header is emitted under an
unprefixed name (RFC 6648); the spec amendment renaming
`X-Accelerator-Visualiser` lands alongside this phase.

### Prerequisite — centralise the version constant

Define once at the crate root so the middleware, `/api/info`, and
`server-info.json` all read from the same place:

**File**: `skills/visualisation/visualise/server/src/lib.rs`

```rust
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

Update `server.rs:215` (`ServerInfo.version`) to read `crate::VERSION`
instead of `env!(...)` directly. Phase 10.4 will use the same
constant for the `/api/info` body.

### TDD Steps

#### Step 10.3.a: Failing tests

**File**: `skills/visualisation/visualise/server/src/server.rs` (tests)

Three tests cover the layer-ordering invariant: header must appear
on (1) a normal route's response, (2) the API 404 fallback, and
(3) the `host_header_guard`'s 403 short-circuit (the most easily
broken case if the middleware is wired in the wrong layer
position).

```rust
#[tokio::test]
async fn responses_carry_version_header_on_normal_route() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt as _;

    let dir = tempfile::tempdir().unwrap();
    let state = build_minimal_state(dir.path()).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .header("host", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let header = resp
        .headers()
        .get("accelerator-visualiser-version")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(header, crate::VERSION);
}

#[tokio::test]
async fn version_header_present_on_404_fallback() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt as _;

    let dir = tempfile::tempdir().unwrap();
    let state = build_minimal_state(dir.path()).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/does-not-exist")
                .header("host", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
    assert!(resp.headers().contains_key("accelerator-visualiser-version"));
}

#[tokio::test]
async fn version_header_present_on_host_header_guard_rejection() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt as _;

    let dir = tempfile::tempdir().unwrap();
    let state = build_minimal_state(dir.path()).await;
    let app = build_router(state);

    // host_header_guard rejects non-loopback Host with 403.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .header("host", "evil.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
    assert!(
        resp.headers().contains_key("accelerator-visualiser-version"),
        "version header must survive guard short-circuits — middleware must be outermost",
    );
}
```

All three tests fail today (no header). The third test specifically
catches the layer-ordering mistake from the previous draft of this
plan.

#### Step 10.3.b: Implementation

**File**: `skills/visualisation/visualise/server/src/server.rs`

```rust
async fn version_header(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        "accelerator-visualiser-version",
        axum::http::HeaderValue::from_static(crate::VERSION),
    );
    resp
}
```

**Layer ordering — critical**: in axum/tower, `.layer(...)` calls
build an onion; the *last* call in source is the **outermost** layer
(runs first on the inbound request, last on the outbound response).
For `version_header` to wrap every response — including ones
produced by `host_header_guard` / `origin_guard` short-circuits —
its `.layer(...)` call must be the **last** call in the builder
chain, not the first.

Update `build_router_with_spa` so the chain ends with
`version_header`:

```rust
attach_spa(api_router)
    .layer(tower_http::compression::CompressionLayer::new())
    .layer(/* TraceLayer (Phase 10.2.e) */)
    .layer(axum::middleware::from_fn_with_state(
        state.activity.clone(),
        crate::activity::middleware,
    ))
    .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
    .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
    .layer(middleware::from_fn(origin_guard))
    .layer(middleware::from_fn(host_header_guard))
    .layer(middleware::from_fn(version_header))    // outermost — wraps every response
```

The `version_header_present_on_host_header_guard_rejection` test
locks this ordering in.

Note: `HeaderValue::from_static(crate::VERSION)` is fine — the
version is an ASCII semver and `from_static` accepts only valid
header bytes.

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p accelerator-visualiser` is green.
- [ ] All three new tests pass:
  - `responses_carry_version_header_on_normal_route`
  - `version_header_present_on_404_fallback`
  - `version_header_present_on_host_header_guard_rejection`

#### Manual Verification

- [ ] `curl -I http://127.0.0.1:<port>/api/healthz` shows
  `accelerator-visualiser-version: <version>`.
- [ ] `curl -I http://127.0.0.1:<port>/api/bogus` (404) also shows
  the header.
- [ ] `curl -I -H 'Host: evil.example' http://127.0.0.1:<port>/api/healthz`
  returns 403 *and* shows the header.

---

## Phase 10.4: `/api/info` endpoint and sidebar version footer

### Overview

Server exposes its identity as JSON at `GET /api/info` (`{name,
version}`). Frontend fetches it once on app boot via
`useServerInfo` and renders the version in a new `<SidebarFooter>`
component (extracted to keep the existing `<Sidebar>` focused on
navigation).

### Architectural choices locked in

- **Endpoint placement**: `/api/info` lives next to `/api/healthz`
  in `server.rs::build_router_with_spa` rather than inside
  `api::mount`. Both are infrastructure routes (no `AppState`
  graph dependency); `api::mount` is reserved for stateful domain
  endpoints.
- **Module surface**: `api/info.rs` exports a handler function
  (`pub(crate) async fn get_info(...) -> Json<ServerInfoBody>`),
  matching every sibling `api/<x>.rs` module. No `pub fn router()`
  builder.
- **Payload shape**: `{name: "accelerator-visualiser", version:
  "<semver>"}`. The `name` field is inert for the sidebar (which
  reads `version` only) but lets `curl`-based callers verify
  endpoint identity unambiguously and provides a stable extension
  point.
- **Cache headers**: `Cache-Control: no-cache` on the response, so
  any future proxy/tunnel exposure (post-v1) doesn't apply
  heuristic freshness.
- **queryKey casing**: kebab-case in the tuple
  (`['server-info']`) to match existing `queryKeys` patterns
  (`'doc-content'`, `'lifecycle-cluster'`, `'template-detail'`).
- **Sidebar separation**: extract a new `<SidebarFooter>` component
  that consumes `useServerInfo` and the connection state (Phase
  10.5) directly via hooks. `<Sidebar>` keeps its `docTypes` prop
  only — no accretion of `versionLabel`, `connectionState`, or
  future status surfaces.
- **Footer ARIA**: no `role="contentinfo"` (that landmark is for
  the page-level footer, of which there should be one; nesting
  inside `<nav>` is a landmark misuse). The footer is queried in
  tests via `aria-label`.

### TDD Steps

#### Step 10.4.a: Failing test — server endpoint

**File**: `skills/visualisation/visualise/server/src/api/info.rs` (new)

```rust
//! `GET /api/info` — server identity for the UI footer and `curl`-
//! based version checks. Static body; cache-control prevents proxies
//! from caching it.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerInfoBody {
    pub name: &'static str,
    pub version: &'static str,
}

pub(crate) async fn get_info() -> impl IntoResponse {
    let body = Json(ServerInfoBody {
        name: "accelerator-visualiser",
        version: crate::VERSION,
    });
    (
        [(axum::http::header::CACHE_CONTROL, "no-cache")],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn info_returns_name_and_version_with_no_cache() {
        // No AppState needed — this is an infrastructure route.
        let app: Router = Router::new().route("/api/info", get(get_info));

        let resp = app
            .oneshot(Request::builder().uri("/api/info").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok()),
            Some("no-cache"),
        );
        let bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["name"], "accelerator-visualiser");
        assert_eq!(v["version"], crate::VERSION);
    }
}
```

#### Step 10.4.b: Mount alongside `/api/healthz`

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`

Add `pub(crate) mod info;` (no router export — the handler is
mounted directly in `server.rs`).

**File**: `skills/visualisation/visualise/server/src/server.rs`

In `build_router_with_spa`, add the route directly to `api_router`
next to `/api/healthz`:

```rust
let api_router = Router::new()
    .route("/api/healthz", get(healthz))
    .route("/api/info", get(crate::api::info::get_info))
    .merge(crate::api::mount(state.clone()))
    .route("/api/*rest", any(api_not_found))
    .with_state(state.clone());
```

This keeps `api::mount` reserved for stateful domain endpoints and
mirrors `/api/healthz`'s placement.

#### Step 10.4.c: Failing test — frontend hook

**File**: `skills/visualisation/visualise/frontend/src/api/use-server-info.test.tsx`
(note `.tsx` extension — the wrapper renders JSX)

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { ReactNode } from 'react'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useServerInfo } from './use-server-info'

function makeWrapper() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
}

describe('useServerInfo', () => {
  beforeEach(() => {
    vi.unstubAllGlobals()
  })

  it('returns name and version from /api/info', async () => {
    vi.stubGlobal('fetch', vi.fn(async (url: string) => {
      if (url.endsWith('/api/info')) {
        return new Response(
          JSON.stringify({ name: 'accelerator-visualiser', version: '0.1.0' }),
          { status: 200 },
        )
      }
      return new Response('not found', { status: 404 })
    }))
    const { result } = renderHook(() => useServerInfo(), { wrapper: makeWrapper() })
    await waitFor(() => expect(result.current.data?.version).toBe('0.1.0'))
    expect(result.current.data?.name).toBe('accelerator-visualiser')
  })

  it('surfaces an error when /api/info returns 500', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('boom', { status: 500 })))
    const { result } = renderHook(() => useServerInfo(), { wrapper: makeWrapper() })
    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(result.current.data).toBeUndefined()
  })

  it('does not throw when the response body is missing the version field', async () => {
    vi.stubGlobal('fetch', vi.fn(async () =>
      new Response(JSON.stringify({}), { status: 200 }),
    ))
    const { result } = renderHook(() => useServerInfo(), { wrapper: makeWrapper() })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    // Consumers must guard against missing version — no throw / crash here.
    expect(result.current.data?.version).toBeUndefined()
  })
})
```

#### Step 10.4.d: Implementation — hook + queryKey

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.ts` (extend)

```ts
serverInfo: () => ['server-info'] as const,
```

(Kebab-case in the tuple matches existing `'doc-content'`,
`'lifecycle-cluster'`, `'template-detail'`.)

**File**: `skills/visualisation/visualise/frontend/src/api/use-server-info.ts` (new)

```ts
import { useQuery } from '@tanstack/react-query'
import { queryKeys } from './query-keys'

export interface ServerInfo {
  name?: string
  version?: string
}

async function fetchServerInfo(): Promise<ServerInfo> {
  const resp = await fetch('/api/info')
  if (!resp.ok) {
    throw new Error(`/api/info returned ${resp.status}`)
  }
  // Body shape may evolve; consumers null-check fields.
  return resp.json() as Promise<ServerInfo>
}

export function useServerInfo() {
  return useQuery({
    queryKey: queryKeys.serverInfo(),
    queryFn: fetchServerInfo,
    // Server version is stable for the life of the connection.
    // The reconnect sweep in Phase 10.5 deliberately excludes this
    // queryKey so the footer doesn't flicker on every reconnect.
    staleTime: Infinity,
  })
}
```

#### Step 10.4.e: Failing test — `<SidebarFooter>`

**File**: `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.test.tsx`
(new component + test — extracts the footer from `<Sidebar>`)

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { SidebarFooter } from './SidebarFooter'

vi.mock('../../api/use-server-info', () => ({
  useServerInfo: vi.fn(),
}))
vi.mock('../../api/use-doc-events', () => ({
  useDocEvents: vi.fn(),
}))
import { useServerInfo } from '../../api/use-server-info'
import { useDocEvents } from '../../api/use-doc-events'

// Shared helper: returns a fully-typed mock matching DocEventsHandle
// so any future field addition fails to compile rather than silently
// flowing as `undefined`.
function mockDocEvents(overrides: Partial<DocEventsHandle> = {}): DocEventsHandle {
  return {
    setDragInProgress: () => {},
    connectionState: 'open',
    justReconnected: false,
    ...overrides,
  }
}

describe('SidebarFooter', () => {
  it('renders the version label by visible text when /api/info has resolved', () => {
    vi.mocked(useServerInfo).mockReturnValue({
      data: { name: 'accelerator-visualiser', version: '0.1.0' },
    } as any)
    vi.mocked(useDocEvents).mockReturnValue(mockDocEvents())

    render(<SidebarFooter />)
    // Query by visible text — SR users hear the actual version
    // number, not a generic aria-label.
    expect(screen.getByText('Visualiser v0.1.0')).toBeInTheDocument()
  })

  it('renders the reserved empty footer (no indicator, no label) on first connect', () => {
    vi.mocked(useServerInfo).mockReturnValue({ data: undefined } as any)
    vi.mocked(useDocEvents).mockReturnValue(mockDocEvents({ connectionState: 'connecting' }))

    render(<SidebarFooter />)
    // Footer container is present (reserves layout space) but
    // contains no version label and no reconnecting indicator.
    expect(screen.queryByRole('status')).toBeNull()
    expect(screen.queryByText(/Visualiser v/)).toBeNull()
  })

  it('does not show "Reconnecting…" while in the initial `connecting` state', () => {
    vi.mocked(useServerInfo).mockReturnValue({ data: undefined } as any)
    vi.mocked(useDocEvents).mockReturnValue(mockDocEvents({ connectionState: 'connecting' }))

    render(<SidebarFooter />)
    expect(screen.queryByRole('status', { name: /reconnecting/i })).toBeNull()
  })

  it('renders the reconnecting indicator when connectionState is "reconnecting"', () => {
    vi.mocked(useServerInfo).mockReturnValue({
      data: { name: 'accelerator-visualiser', version: '0.1.0' },
    } as any)
    vi.mocked(useDocEvents).mockReturnValue(mockDocEvents({ connectionState: 'reconnecting' }))

    render(<SidebarFooter />)
    expect(screen.getByRole('status', { name: /reconnecting/i })).toBeInTheDocument()
  })

  it('renders the "Reconnected — refreshing" toast when justReconnected and open', () => {
    vi.mocked(useServerInfo).mockReturnValue({
      data: { name: 'accelerator-visualiser', version: '0.1.0' },
    } as any)
    vi.mocked(useDocEvents).mockReturnValue(
      mockDocEvents({ connectionState: 'open', justReconnected: true }),
    )

    render(<SidebarFooter />)
    expect(screen.getByRole('status', { name: /reconnected/i })).toBeInTheDocument()
  })

  it('renders ONLY the reconnecting indicator if both reconnecting and justReconnected are true', () => {
    // Mutual-exclusion regression guard.
    vi.mocked(useServerInfo).mockReturnValue({ data: undefined } as any)
    vi.mocked(useDocEvents).mockReturnValue(
      mockDocEvents({ connectionState: 'reconnecting', justReconnected: true }),
    )

    render(<SidebarFooter />)
    const statuses = screen.getAllByRole('status')
    expect(statuses).toHaveLength(1)
    expect(statuses[0]).toHaveTextContent(/reconnecting/i)
  })
})
```

#### Step 10.4.f: Implementation — `<SidebarFooter>` + wiring

**File**: `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.tsx` (new — initial scaffold; Phase 10.5 extends with the `Reconnected — refreshing` transient and the mutual-exclusion logic)

```tsx
import { useServerInfo } from '../../api/use-server-info'
import { useDocEvents } from '../../api/use-doc-events'
import styles from './SidebarFooter.module.css'

export function SidebarFooter() {
  const { data: serverInfo } = useServerInfo()
  const { connectionState } = useDocEvents()
  const versionLabel = serverInfo?.version ? `Visualiser v${serverInfo.version}` : null

  // No `'connecting'` indicator — only show "Reconnecting…" once a
  // previous open has been lost. The footer container always renders
  // (reserved layout space via `min-height` in CSS) so the version
  // label fades in without shifting other sidebar content.
  return (
    <div className={styles.footer}>
      {connectionState === 'reconnecting' && (
        <span className={styles.reconnecting} role="status">
          Reconnecting…
        </span>
      )}
      {versionLabel && <span className={styles.version}>{versionLabel}</span>}
    </div>
  )
}
```

(Tests query the version label by visible text via
`getByText('Visualiser v0.1.0')` rather than `aria-label` so SR
users hear the actual version number.)

**File**: `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.module.css`

```css
.footer {
  margin-top: auto;          /* push to the bottom of the flex column */
  min-height: 1.5rem;        /* reserve layout space — no first-load jank */
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  color: var(--color-muted-text);
  border-top: 1px solid var(--color-divider);
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.reconnecting,
.reconnected {
  color: var(--color-warning-text);
}
```

(`--color-muted-text` is the AA-grade text token from Phase 10.7;
the decorative `--color-muted` token used elsewhere stays at the
existing lighter value to avoid the cascading visual regression
the review flagged.)

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`

Add `<SidebarFooter />` at the bottom; do **not** add any new
props:

```tsx
import { SidebarFooter } from '../SidebarFooter/SidebarFooter'

export function Sidebar({ docTypes }: Props) {
  // existing body unchanged …
  return (
    <nav className={styles.sidebar}>
      {/* existing sections */}
      <SidebarFooter />
    </nav>
  )
}
```

(Sidebar's `Props` interface stays as `{ docTypes }` — no
`versionLabel`, no `connectionState`, no future-status accretion.)

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`

Unchanged from the pre-Phase-10 shape — `<RootLayout>` does not
need to know about server info or connection state.

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p accelerator-visualiser` green; new test
  `api::info::tests::info_returns_name_and_version_with_no_cache`
  passes.
- [ ] `npm test` (vitest) green; new tests:
  - `use-server-info.test.tsx` (3 cases: success, 500, missing
    version)
  - `SidebarFooter.test.tsx` (3 cases: version, empty, reconnecting)
- [ ] `npm run typecheck` green.
- [ ] Existing `Sidebar.test.tsx` continues to pass unchanged
  (props surface didn't change).

#### Manual Verification

- [ ] `curl -s http://127.0.0.1:<port>/api/info | jq` returns
  `{"name":"accelerator-visualiser","version":"<semver>"}` and the
  response carries `Cache-Control: no-cache`.
- [ ] Sidebar shows "Visualiser v0.1.0" at the bottom.

---

## Phase 10.5: `ReconnectingEventSource` + invalidate-all on reconnect

### Overview

Wrap the native `EventSource` in a small class that handles
reconnect with exponential backoff (1 s → 2 s → 4 s → 8 s → 16 s →
30 s, capped, ±20% jitter), and a single-field state machine
`'open' | 'reconnecting' | 'closed'`. On every transition into
`'open'` from `'reconnecting'`, fire `onReconnect`; the wiring in
`useDocEvents` then invalidates every query *except* session-stable
keys (e.g. `['server-info']`) so the sidebar version footer doesn't
flicker. While in `'reconnecting'` state, `<SidebarFooter>` (Phase
10.4) renders a "Reconnecting…" indicator. When the connection
recovers, render a transient "Reconnected" message for ~3 s before
clearing.

### Design choices locked in

- **Four-value state machine** —
  `'connecting' | 'open' | 'reconnecting' | 'closed'`. Initial
  state is `'connecting'` (set in the constructor *before*
  `connect()` runs), so the first connection actually happens.
  `'closed'` is reserved exclusively for the post-`close()`
  resurrection guard. State transitions:
  - `'connecting' → 'open'` on the first successful `onopen`.
    Does **not** fire `onReconnect` (the invariant "no sweep on
    first-ever connect").
  - `'open' → 'reconnecting'` on `onerror` (and on a
    factory-throw inside `connect()`).
  - `'connecting' → 'reconnecting'` on a from-boot error before
    the first open ever lands. The eventual `'reconnecting' →
    'open'` then fires `onReconnect`, satisfying the spec's
    "invalidate-all on reconnect" rule on the boot-recovery path.
  - `'reconnecting' → 'open'` on a successful re-open. Fires
    `onReconnect`.
  - `'open' | 'connecting' | 'reconnecting' → 'closed'` on
    `close()`. One-way; never returns.
  Replacing the previous `'closed' | 'open' | 'reconnecting'`
  triple resolves the constructor short-circuit and gives
  `<SidebarFooter>` a clean signal to differentiate
  bootstrap (`'connecting'`, render no indicator) from
  recovery (`'reconnecting'`, render "Reconnecting…").
- **Hard `'closed'` guard.** Every state-mutating handler
  (`onopen`, `onerror`, `setTimeout` callback, `scheduleReconnect`,
  `connect()` itself) early-returns when `state === 'closed'`.
  `close()` sets the state *first*, then detaches
  `source.onerror = null` *before* `source.close()`, so post-close
  error events from Firefox can't resurrect the wrapper.
- **Idempotent `scheduleReconnect`.** Early-returns when state is
  already `'reconnecting'` or `'closed'`, so repeated browser
  `error` events on a flapping connection don't reset the
  in-flight timer.
- **Local-binding pattern in `connect()`.** Handlers are bound to
  a local `src` variable; `this.source = src` is assigned only
  after binding completes. So a re-entrant `close()` from inside
  the consumer's `onerror` callback (during binding) cannot race
  the rebind — `close()` observes `this.source` as the previous
  attempt's source (or null) and `detachAndCloseSource()` doesn't
  try to detach handlers we haven't attached yet.
- **Math.random injectable.** `ReconnectOpts.random` defaults to
  `Math.random` in production; tests pass a deterministic seed so
  the `vi.advanceTimersByTime` assertions don't flake on jitter.
- **Initial state notification.** `setState` does *not* short-
  circuit on the very first call, so subscribers registered in
  the constructor see the initial `'connecting' → 'open'`
  transition.
- **`attempts` counter.** Resets to 0 on every successful
  `'open'`. Capped at 32 in `scheduleReconnect` to prevent leak
  on long-lived disconnected instances. Flapping behaviour (open
  → error → open → error within seconds) is acceptable v1
  behaviour — we are not trying to discourage a flapping
  localhost server.
- **`onopen` removed from the public surface.** `onReconnect` is
  the only semantically interesting open transition. `onmessage`
  and `onerror` remain for the consumer.
- **`onerror` accepted at construction time.** `ReconnectOpts`
  accepts `onerror?: (e: Event) => void`, so a from-boot
  factory-throw inside the constructor can surface to the
  consumer via the same callback as later errors. The previous
  pattern (assign `wrapper.onerror = ...` after construction)
  missed the very first error event.
- **Reactive connection state.** `useDocEvents` exposes
  `connectionState` via `useState` (not a ref) so consumers
  re-render on transitions. The state also feeds an `aria-live`
  announcement for the post-reconnect "Reconnected" toast.
- **Session-stable query keys live in one place.**
  `SESSION_STABLE_QUERY_ROOTS: ReadonlySet<unknown>` exported
  from `query-keys.ts` (currently `new Set(['server-info'])`).
  The reconnect predicate reads from this set so a queryKey
  rename in one place won't desync the predicate.

### TDD Steps

#### Step 10.5.a: Failing tests — pure reconnect logic

**File**: `skills/visualisation/visualise/frontend/src/api/reconnecting-event-source.test.ts` (new)

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  ReconnectingEventSource,
  computeBackoff,
} from './reconnecting-event-source'

describe('computeBackoff', () => {
  it('starts at 1s and doubles', () => {
    expect(computeBackoff(0, 0.5)).toBeCloseTo(1000, -2)
    expect(computeBackoff(1, 0.5)).toBeCloseTo(2000, -2)
    expect(computeBackoff(2, 0.5)).toBeCloseTo(4000, -2)
  })

  it('caps at 30s', () => {
    expect(computeBackoff(99, 0.5)).toBeCloseTo(30000, -2)
  })

  it('caps cleanly even at extreme attempt values', () => {
    // 2 ** 1000 overflows to Infinity; Math.min should still cap to 30000.
    const v = computeBackoff(1000, 0.5)
    expect(Number.isFinite(v)).toBe(true)
    expect(v).toBeCloseTo(30000, -2)
  })

  it('applies +/-20% jitter across the seed range', () => {
    const samples = Array.from({ length: 100 }, (_, i) => computeBackoff(2, i / 100))
    const min = Math.min(...samples)
    const max = Math.max(...samples)
    expect(min).toBeGreaterThanOrEqual(4000 * 0.8)
    expect(max).toBeLessThanOrEqual(4000 * 1.2)
  })
})

describe('ReconnectingEventSource', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  // Each call to the factory produces a *fresh* fake — so the test
  // can verify per-attempt handler binding rather than retaining
  // stale references.
  function makeFakeFactory() {
    const fakes: Array<{
      onopen: ((e: any) => void) | null
      onerror: ((e: any) => void) | null
      onmessage: ((e: any) => void) | null
      close: ReturnType<typeof vi.fn>
    }> = []
    const factory = vi.fn(() => {
      const fake = {
        onopen: null as any,
        onerror: null as any,
        onmessage: null as any,
        close: vi.fn(),
      }
      fakes.push(fake)
      return fake as unknown as EventSource
    })
    return { fakes, factory }
  }

  it('opens, errors, then reconnects after exactly the deterministic backoff', () => {
    const { fakes, factory } = makeFakeFactory()
    const onReconnect = vi.fn()
    // random() = 0 → multiplier 0.8 → first delay = 800 ms
    new ReconnectingEventSource('/api/events', {
      factory, onReconnect, random: () => 0,
    })
    expect(factory).toHaveBeenCalledTimes(1)
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})
    expect(fakes[0].close).toHaveBeenCalled()

    // Below the deterministic delay → no reconnect yet.
    vi.advanceTimersByTime(799)
    expect(factory).toHaveBeenCalledTimes(1)

    // Crossing the boundary triggers a fresh fake.
    vi.advanceTimersByTime(2)
    expect(factory).toHaveBeenCalledTimes(2)
    expect(fakes.length).toBe(2)
    expect(fakes[0]).not.toBe(fakes[1])

    // Successful re-open on the new fake fires onReconnect.
    fakes[1].onopen?.({})
    expect(onReconnect).toHaveBeenCalledTimes(1)
  })

  it('does not call onReconnect on the very first open', () => {
    const { fakes, factory } = makeFakeFactory()
    const onReconnect = vi.fn()
    new ReconnectingEventSource('/api/events', { factory, onReconnect, random: () => 0 })
    fakes[0].onopen?.({})
    expect(onReconnect).not.toHaveBeenCalled()
  })

  it('fires onReconnect on first-ever-open after a from-boot error', () => {
    // Server unreachable from boot: error before any open.
    const { fakes, factory } = makeFakeFactory()
    const onReconnect = vi.fn()
    new ReconnectingEventSource('/api/events', { factory, onReconnect, random: () => 0 })
    fakes[0].onerror?.({})           // first ever error
    vi.advanceTimersByTime(801)      // wait past the deterministic 800 ms
    fakes[1].onopen?.({})            // first-ever successful open
    expect(onReconnect).toHaveBeenCalledTimes(1)
  })

  it('reports `connecting` from the constructor before the first open lands', () => {
    const { factory } = makeFakeFactory()
    const states: string[] = []
    new ReconnectingEventSource('/api/events', {
      factory, random: () => 0,
      onStateChange: s => states.push(s),
    })
    // Constructor transitions closed → connecting before factory() runs.
    expect(states).toEqual(['connecting'])
  })

  it('exposes a state observable: connecting → open → reconnecting → open → closed', () => {
    const { fakes, factory } = makeFakeFactory()
    const states: string[] = []
    const r = new ReconnectingEventSource('/api/events', {
      factory, random: () => 0,
      onStateChange: s => states.push(s),
    })
    fakes[0].onopen?.({})           // connecting → open
    fakes[0].onerror?.({})          // open → reconnecting
    vi.advanceTimersByTime(801)
    fakes[1].onopen?.({})           // reconnecting → open
    r.close()                       // open → closed
    expect(states).toEqual(['connecting', 'open', 'reconnecting', 'open', 'closed'])
  })

  it('reports `connecting` then `reconnecting` on a from-boot error before any open', () => {
    const { fakes, factory } = makeFakeFactory()
    const states: string[] = []
    new ReconnectingEventSource('/api/events', {
      factory, random: () => 0,
      onStateChange: s => states.push(s),
    })
    fakes[0].onerror?.({})          // connecting → reconnecting
    expect(states).toEqual(['connecting', 'reconnecting'])
  })

  it('surfaces a constructor-time factory throw via onerror passed at construction', () => {
    const { factory } = makeFakeFactory()
    const seen: Event[] = []
    new ReconnectingEventSource('/api/events', {
      factory: () => { throw new Error('CSP block') },
      random: () => 0,
      onerror: (e) => seen.push(e),
    })
    // The from-construction throw fires onerror exactly once.
    expect(seen.length).toBe(1)
  })

  it('re-entrant close() from inside onerror leaves wrapper closed with no scheduled timer', () => {
    const { fakes, factory } = makeFakeFactory()
    const r = new ReconnectingEventSource('/api/events', {
      factory, random: () => 0,
      onerror: () => r.close(),     // consumer closes from inside the error
    })
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})
    expect(r.connectionState).toBe('closed')
    vi.advanceTimersByTime(60_000)
    expect(factory).toHaveBeenCalledTimes(1)  // no resurrection
  })

  it('repeated browser errors during reconnect do not reset the timer', () => {
    const { fakes, factory } = makeFakeFactory()
    new ReconnectingEventSource('/api/events', { factory, random: () => 0 })
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})           // → reconnecting, attempt 0
    vi.advanceTimersByTime(400)
    fakes[0].onerror?.({})           // ignored — already reconnecting
    fakes[0].onerror?.({})           // ignored
    vi.advanceTimersByTime(401)      // total 801 ms — original timer fires
    expect(factory).toHaveBeenCalledTimes(2)
  })

  it('close() prevents resurrection by post-close error events', () => {
    const { fakes, factory } = makeFakeFactory()
    const r = new ReconnectingEventSource('/api/events', { factory, random: () => 0 })
    fakes[0].onopen?.({})
    r.close()
    // Browsers (notably Firefox) sometimes deliver an error after close().
    fakes[0].onerror?.({})
    vi.advanceTimersByTime(60_000)
    expect(factory).toHaveBeenCalledTimes(1) // no resurrection
  })

  it('factory throwing during reconnect is caught and re-tried', () => {
    let throwOnce = true
    const { fakes, factory } = makeFakeFactory()
    const wrapped = vi.fn((url: string) => {
      if (throwOnce) { throwOnce = false; throw new Error('CSP block') }
      return factory(url)
    })
    new ReconnectingEventSource('/api/events', { factory: wrapped, random: () => 0 })
    // Constructor's first connect() throws — wrapper catches and
    // schedules a retry. The retry succeeds.
    vi.advanceTimersByTime(801)
    expect(wrapped).toHaveBeenCalledTimes(2)
    expect(fakes.length).toBe(1)
  })

  it('useDocEvents reconnect sweep preserves session-stable cache entries', async () => {
    // Integration-level test for the predicate carve-out. Seeds the
    // QueryClient with both 'server-info' and 'docs' state, drives a
    // reconnect via the wrapper, then asserts 'server-info' survives
    // while 'docs' is invalidated.
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    queryClient.setQueryData(['server-info'], { name: 'x', version: '1.2.3' })
    queryClient.setQueryData(['docs', 'tickets'], [{ relPath: 'a' }])

    const { fakes, factory } = makeFakeFactory()
    const Wrapped = makeUseDocEvents(factory)
    function Probe() { Wrapped(); return null }
    render(
      <QueryClientProvider client={queryClient}>
        <Probe />
      </QueryClientProvider>,
    )
    // Drive: open → error → backoff → re-open → onReconnect sweep.
    fakes[0].onopen?.({})
    fakes[0].onerror?.({})
    vi.advanceTimersByTime(801)
    fakes[1].onopen?.({})
    // Allow the queued setState/invalidate microtasks to settle.
    await Promise.resolve()

    // 'server-info' stays cached (its queryKey root is in
    // SESSION_STABLE_QUERY_ROOTS). 'docs' was invalidated.
    expect(queryClient.getQueryData(['server-info'])).toEqual({ name: 'x', version: '1.2.3' })
    const docsState = queryClient.getQueryState(['docs', 'tickets'])
    expect(docsState?.isInvalidated).toBe(true)
  })
})
```

#### Step 10.5.b: Implementation

**File**: `skills/visualisation/visualise/frontend/src/api/reconnecting-event-source.ts` (new)

```ts
const INITIAL_BACKOFF_MS = 1000
const MAX_BACKOFF_MS = 30_000
const JITTER = 0.2
const MAX_ATTEMPTS = 32       // cap to prevent counter leak on long-lived instances

export type ConnectionState = 'connecting' | 'open' | 'reconnecting' | 'closed'

export interface ReconnectOpts {
  factory: (url: string) => EventSource
  onReconnect?: () => void
  onStateChange?: (s: ConnectionState) => void
  /** Surfaces transport errors, including factory-throws during
   *  the initial constructor `connect()`. Use this rather than
   *  assigning `wrapper.onerror = ...` after construction so the
   *  very first error event is observable. */
  onerror?: (e: Event) => void
  /** Injection point for tests; defaults to Math.random. */
  random?: () => number
}

export function computeBackoff(attempt: number, jitterSeed: number): number {
  // Math.pow guards against Infinity for very large attempts.
  const raw = INITIAL_BACKOFF_MS * Math.pow(2, attempt)
  const base = Math.min(raw, MAX_BACKOFF_MS)
  // jitterSeed in [0,1) → multiplier in [1-JITTER, 1+JITTER]
  const mult = 1 - JITTER + jitterSeed * 2 * JITTER
  return base * mult
}

export class ReconnectingEventSource {
  private url: string
  private opts: ReconnectOpts
  private source: EventSource | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  private attempts = 0
  // Initial state is 'connecting', NOT 'closed'. 'closed' is reserved
  // exclusively for the post-close() resurrection guard. This split
  // is what lets the constructor's first connect() actually run.
  private state: ConnectionState = 'connecting'
  private rand: () => number

  // Consumer-facing handlers for ongoing events. `onopen` is
  // intentionally not exposed — `onReconnect` (constructor opt)
  // is the only semantically interesting open transition.
  public onmessage: ((e: MessageEvent) => void) | null = null

  constructor(url: string, opts: ReconnectOpts) {
    this.url = url
    this.opts = opts
    this.rand = opts.random ?? Math.random
    // Notify subscribers of the initial 'connecting' transition
    // before the first connect() runs so a synchronous factory-throw
    // doesn't bypass the state observable.
    this.opts.onStateChange?.('connecting')
    this.connect()
  }

  /** Public read-only state for snapshot inspection. */
  get connectionState(): ConnectionState { return this.state }

  private setState(s: ConnectionState) {
    if (this.state === s) return
    this.state = s
    this.opts.onStateChange?.(s)
  }

  private connect() {
    if (this.state === 'closed') return
    let src: EventSource
    try {
      src = this.opts.factory(this.url)
    } catch {
      // factory threw (e.g. CSP block) — surface to the consumer
      // and schedule a retry.
      this.opts.onerror?.(new Event('error'))
      this.scheduleReconnect()
      return
    }
    // Bind handlers to the local `src` before publishing to
    // `this.source`. A re-entrant close() from inside any of these
    // handlers' consumer callbacks therefore sees the *previous*
    // attempt's source (or null) rather than a half-bound one.
    src.onopen = (e) => {
      if (this.state === 'closed') return
      const wasReconnecting = this.state === 'reconnecting'
      this.attempts = 0
      this.setState('open')
      if (wasReconnecting) {
        this.opts.onReconnect?.()
      }
      void e
    }
    src.onerror = (e) => {
      if (this.state === 'closed') return
      this.opts.onerror?.(e)
      this.scheduleReconnect()
    }
    src.onmessage = (e) => {
      if (this.state === 'closed') return
      this.onmessage?.(e as MessageEvent)
    }
    this.source = src
  }

  private scheduleReconnect() {
    // Idempotent: ignore repeat error events while already
    // reconnecting, and never resurrect after close.
    if (this.state === 'closed' || this.state === 'reconnecting') return
    this.detachAndCloseSource()
    this.setState('reconnecting')
    const delay = computeBackoff(this.attempts, this.rand())
    this.attempts = Math.min(this.attempts + 1, MAX_ATTEMPTS)
    this.timer = setTimeout(() => {
      this.timer = null
      this.connect()
    }, delay)
  }

  private detachAndCloseSource() {
    if (this.source) {
      // Detach handlers before close() so any post-close error
      // events the browser delivers don't fire our handler.
      this.source.onopen = null
      this.source.onerror = null
      this.source.onmessage = null
      try { this.source.close() } catch { /* ignore */ }
      this.source = null
    }
  }

  close() {
    // Set state first so any in-flight handlers early-return.
    this.setState('closed')
    if (this.timer !== null) { clearTimeout(this.timer); this.timer = null }
    this.detachAndCloseSource()
  }
}
```

#### Step 10.5.c: Wire into `useDocEvents`

Make connection state reactive (via `useState`, not a ref), expose
it through `DocEventsHandle`, and exclude session-stable queries
from the invalidate-all sweep.

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`

```ts
export interface DocEventsHandle {
  setDragInProgress(v: boolean): void
  connectionState: ConnectionState
  /** True for the ~3 s window after a successful reconnect. */
  justReconnected: boolean
}

export function makeUseDocEvents(
  createSource: EventSourceFactory,
  registry: SelfCauseRegistry = defaultSelfCauseRegistry,
) {
  return function useDocEvents(): DocEventsHandle {
    const queryClient = useQueryClient()
    const isDraggingRef = useRef(false)
    const pendingRef = useRef(new Set<string>())
    const [connectionState, setConnectionState] = useState<ConnectionState>('closed')
    const [justReconnected, setJustReconnected] = useState(false)

    const setDragInProgress = useCallback(
      (v: boolean) => {
        isDraggingRef.current = v
        if (!v) {
          for (const qkStr of pendingRef.current) {
            void queryClient.invalidateQueries({ queryKey: JSON.parse(qkStr) as unknown[] })
          }
          pendingRef.current.clear()
        }
      },
      [queryClient],
    )

    useEffect(() => {
      let reconnectedTimer: ReturnType<typeof setTimeout> | null = null
      const reconnecting = new ReconnectingEventSource('/api/events', {
        factory: createSource,
        onStateChange: setConnectionState,
        onerror: () => {
          console.warn('useDocEvents: SSE error — reconnecting')
          // Eager-invalidate on error is intentionally removed: the
          // wrapper backs off and the onReconnect sweep covers cache
          // staleness on recovery. (Migration note: prior builds called
          // queryClient.invalidateQueries({ queryKey: ['docs'] }) here;
          // that produced a refetch storm during outages.)
        },
        onReconnect: () => {
          // Self-cause registry is per-mutation; reconnect drops it.
          registry.reset()
          // Drain any drag-pending invalidations.
          for (const qkStr of pendingRef.current) {
            void queryClient.invalidateQueries({ queryKey: JSON.parse(qkStr) as unknown[] })
          }
          pendingRef.current.clear()
          // Invalidate every cached query EXCEPT session-stable ones.
          // queryClient.invalidateQueries({ predicate }) ignores
          // staleTime, so without the predicate the version footer
          // would refetch and briefly clear on every reconnect.
          // The set lives next to queryKeys so a key rename in one
          // place can't desync the predicate.
          void queryClient.invalidateQueries({
            predicate: (q) => !SESSION_STABLE_QUERY_ROOTS.has(q.queryKey[0]),
          })
          // Surface a transient "Reconnected" announcement; clear any
          // previous timer so a flap-then-recovery within 3 s extends
          // the toast rather than ending it early.
          if (reconnectedTimer !== null) clearTimeout(reconnectedTimer)
          setJustReconnected(true)
          reconnectedTimer = setTimeout(() => {
            setJustReconnected(false)
            reconnectedTimer = null
          }, 3_000)
        },
      })

      reconnecting.onmessage = (e: MessageEvent) => {
        try {
          const event = JSON.parse(e.data as string) as SseEvent
          if (event.type === 'doc-changed' && registry.has(event.etag)) return
          if (isDraggingRef.current) {
            for (const k of queryKeysForEvent(event)) {
              pendingRef.current.add(JSON.stringify(k))
            }
          } else {
            dispatchSseEvent(event, queryClient)
          }
        } catch (err) {
          console.warn('useDocEvents: failed to parse SSE message', { data: e.data, err })
        }
      }

      return () => {
        if (reconnectedTimer !== null) clearTimeout(reconnectedTimer)
        reconnecting.close()
      }
    }, [queryClient, createSource, registry])

    return { setDragInProgress, connectionState, justReconnected }
  }
}
```

Add to `query-keys.ts`:

```ts
/** Query-key roots whose data must NOT be invalidated by the
 *  reconnect-recovery sweep (Phase 10.5). Add a key here when its
 *  data is stable for the lifetime of the connection (e.g. server
 *  identity, plugin metadata). */
export const SESSION_STABLE_QUERY_ROOTS: ReadonlySet<unknown> = new Set([
  'server-info',
])
```

And import `SESSION_STABLE_QUERY_ROOTS` in `use-doc-events.ts`.

Notes:
- `connectionState` and `justReconnected` are pieces of `useState`
  so consumers re-render on transitions.
- The invalidate-all predicate excludes `'server-info'`. As more
  session-stable keys land (none in v1), extend the predicate.
- The eager `invalidateQueries(['docs'])` previously called from
  `onerror` is removed; the recovery sweep in `onReconnect`
  covers it. Documented in Migration Notes below.

#### Step 10.5.d: `<SidebarFooter>` reconnecting indicator + reconnected toast

`<SidebarFooter>` was extracted in Phase 10.4. Extend its rendering
to also handle the `justReconnected` transient:

```tsx
export function SidebarFooter() {
  const { data: serverInfo } = useServerInfo()
  const { connectionState, justReconnected } = useDocEvents()
  const versionLabel = serverInfo?.version ? `Visualiser v${serverInfo.version}` : null

  // No `'connecting'` indicator on first load — only show
  // "Reconnecting…" once a previous open has been lost. Avoids the
  // confusing "Reconnecting…" flash on a fresh app boot.
  const showReconnecting = connectionState === 'reconnecting'
  const showReconnected = justReconnected && connectionState === 'open'

  // Reserve the footer's vertical space unconditionally so the
  // version label fades in without shifting other sidebar content.
  // (CSS `min-height` matches one line of footer text.)
  return (
    <div className={styles.footer}>
      {/* Mutual-exclusion of role="status" regions: pick the most
          recent transition. Folding both signals into one
          aria-live region prevents SR vendors concatenating two
          polite live regions at once. */}
      {showReconnecting && (
        <span className={styles.reconnecting} role="status">
          Reconnecting…
        </span>
      )}
      {showReconnected && !showReconnecting && (
        <span className={styles.reconnected} role="status">
          Reconnected — refreshing
        </span>
      )}
      {versionLabel && <span className={styles.version}>{versionLabel}</span>}
    </div>
  )
}
```

Tests for `<SidebarFooter>`:
- `connectionState='connecting'` + no version → renders the empty
  reserved footer (no indicator, no version label, but `min-height`
  preserves layout).
- `connectionState='reconnecting'` → `Reconnecting…` announced via
  `getByRole('status', { name: /reconnecting/i })`.
- `justReconnected=true, connectionState='open'` → `Reconnected —
  refreshing` announced.
- Pinned mutual-exclusion test: `connectionState='reconnecting'` +
  `justReconnected=true` → only `Reconnecting…` renders.
- The version footer queries by visible text
  (`getByText(/Visualiser v\d/)`) rather than `aria-label`, so SR
  users hear the actual version number.

### Success Criteria

#### Automated Verification

- [ ] `npm test` (vitest) green; new tests for backoff math
  (including extreme attempts), reconnect flow with deterministic
  random, error-then-first-open, repeated-error idempotency,
  post-close resurrection guard, and factory-throws-then-retries
  pass.
- [ ] `npm test` green for an extended `useDocEvents` test that
  asserts: (a) `connectionState` is reactive (consumer re-renders
  on transition), (b) `onReconnect` calls
  `queryClient.invalidateQueries` with a predicate that *excludes*
  `'server-info'`, (c) `'server-info'` cache survives reconnect
  while `'docs'` is invalidated.
- [ ] `npm run typecheck` green.
- [ ] Existing `dispatchSseEvent` test suite still passes
  unchanged (transport upgrade, dispatch unchanged).

#### Manual Verification

- [ ] Stop the server (Ctrl-C). Sidebar shows "Reconnecting…" badge
  within ≤30 s. Watch the network panel: SSE retries get
  progressively spaced out.
- [ ] Restart the server. Within ≤30 s the badge clears, a brief
  "Reconnected — refreshing" message appears, every doc-list,
  lifecycle, and kanban view refetches, and the version footer
  does NOT flicker.
- [ ] Edit a ticket on disk while the server is down, then restart.
  Kanban reflects the new state immediately on reconnect (no
  manual refresh).
- [ ] Open the visualiser in a project where the server is
  unreachable from boot (server not yet started). The badge shows
  "Reconnecting…"; once started, the page recovers and runs the
  invalidate-all sweep on the first-ever-open.

---

## Phase 10.6: Malformed-frontmatter doc-page banner

### Overview

When the entry being shown in `LibraryDocView` has
`frontmatterState === 'malformed'`, render a prominent banner
above the markdown body reading
"We couldn't read this document's metadata header; showing the
file as-is." The banner is **not** a live region — it is static
page-load content. We use a plain `<div>` with a styled "Warning:"
prefix and an `aria-label` for tests. Screen readers announce it
through normal document flow rather than as a polite live update.

### Design choices locked in

- **Copy**: avoid the term "frontmatter" — it's Markdown/Hugo
  jargon unfamiliar to many users. "Metadata header" is plainer
  English.
- **No `role="status"`**: that role is for polite live updates
  (e.g. "Reconnecting…"); a static page-load warning would be
  misannounced as a live update. The banner uses no role; an
  `aria-label` distinguishes it for tests and screen readers.
- **Visible affordance**: a leading "Warning:" prefix (or a small
  warning icon) makes the banner self-describing without leaning
  on ARIA.
- **Mid-session reactivity**: an SSE `doc-invalid` event flips
  the entry's state from `parsed` to `malformed` mid-session;
  `dispatchSseEvent` already invalidates `queryKeys.docs(type)`,
  so the list query refetches and the banner appears on next
  render. A test pins this transition.

### TDD Steps

#### Step 10.6.a: Failing tests

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.test.tsx` (extend)

```tsx
it('renders a malformed-frontmatter banner when entry.frontmatterState is malformed', async () => {
  const entry = {
    ...buildIndexEntry({ relPath: 'meta/plans/2026-04-18-foo.md' }),
    frontmatterState: 'malformed' as const,
  }
  seedDocs([entry])
  seedDocContent(entry.relPath, '# body')

  renderLibraryDocView({ type: 'plans', fileSlug: '2026-04-18-foo' })
  const banner = await screen.findByLabelText(/metadata header/i)
  expect(banner).toHaveTextContent(/We couldn't read this document's metadata header/i)
  // Not a live region — must NOT be role=status.
  expect(banner).not.toHaveAttribute('role', 'status')
})

it('does not render the banner for parsed or absent state', async () => {
  for (const state of ['parsed', 'absent'] as const) {
    const entry = {
      ...buildIndexEntry({ relPath: `meta/plans/2026-04-18-${state}.md` }),
      frontmatterState: state,
    }
    seedDocs([entry])
    seedDocContent(entry.relPath, '# body')
    renderLibraryDocView({ type: 'plans', fileSlug: `2026-04-18-${state}` })
    await screen.findByRole('article')
    expect(screen.queryByLabelText(/metadata header/i)).toBeNull()
  }
})

it('shows the banner mid-session when an SSE doc-invalid event flips state to malformed', async () => {
  // Seed initial state: entry parses cleanly.
  const entry = {
    ...buildIndexEntry({ relPath: 'meta/plans/2026-04-18-foo.md' }),
    frontmatterState: 'parsed' as const,
  }
  seedDocs([entry])
  seedDocContent(entry.relPath, '# body')

  const { queryClient, dispatchSse } = renderLibraryDocView({
    type: 'plans',
    fileSlug: '2026-04-18-foo',
  })
  await screen.findByRole('article')
  expect(screen.queryByLabelText(/metadata header/i)).toBeNull()

  // Simulate a disk edit that breaks the frontmatter — the server
  // re-indexes and emits a doc-invalid event with the new state.
  seedDocs([{ ...entry, frontmatterState: 'malformed' }])
  dispatchSse({
    type: 'doc-invalid',
    docType: 'plans',
    path: entry.relPath,
    etag: 'sha256-newetag',
  })
  await screen.findByLabelText(/metadata header/i)
})
```

(`renderLibraryDocView` returns the `queryClient` and a `dispatchSse`
helper that invokes `dispatchSseEvent` against the mounted query
client. If the existing helper doesn't yet expose `dispatchSse`,
extend it once at the top of the test file.)

#### Step 10.6.b: Implementation

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`

Add the banner above the `<div className={styles.body}>`:

```tsx
{entry!.frontmatterState === 'malformed' && (
  <div className={styles.malformedBanner} aria-label="Document metadata header notice">
    <strong className={styles.malformedPrefix}>Warning:</strong>{' '}
    We couldn&rsquo;t read this document&rsquo;s metadata header; showing the file as-is.
  </div>
)}
```

Add the rule to `LibraryDocView.module.css`:

```css
.malformedBanner {
  margin: 1rem 0;
  padding: 0.75rem 1rem;
  background: var(--color-warning-bg);
  border-left: 4px solid var(--color-warning-border);
  color: var(--color-warning-text);
  font-size: 0.95rem;
}

.malformedPrefix {
  margin-right: 0.25rem;
}
```

Tokens (`--color-warning-bg`, `--color-warning-border`,
`--color-warning-text`) are introduced by the Phase 10.7 contrast
pass. Until 10.7 lands they can be inlined as the AA-passing pair
`#fff8e6` / `#a35b00` (text contrast 5.6:1 against the bg) without
blocking 10.6's tests.

### Success Criteria

#### Automated Verification

- [ ] `npm test` (vitest) green; the three new banner tests
  (parsed-no-banner, malformed-shows-banner, mid-session
  transition) pass.
- [ ] Existing `LibraryDocView` tests still pass.

#### Manual Verification

- [ ] Visit a fixture doc with deliberately broken frontmatter; see
  the banner with the "Warning:" prefix.
- [ ] Visit a normal doc; no banner.
- [ ] Edit a parsing-clean doc on disk to break its frontmatter
  while the doc page is open; the banner appears within one
  debounce cycle without a manual refresh.

---

## Phase 10.7: Focus rings, dnd-kit announcements, contrast pass

### Overview

Three independent micro-tasks bundled because they all live in
frontend CSS / accessibility props. Each gets its own TDD step.
Locks accessibility scope at **WCAG 2.2 AA** (the current published
version; 2.5.7 "Dragging Movements" and 2.4.11 "Focus Not Obscured
(Minimum)" both intersect this work).

### Prerequisites — Vite `?raw` typings + import discipline

Two of the new tests (`global.test.tsx`, `contrast.test.ts`) read
CSS source via `import css from './global.css?raw'`. Vite supports
the `?raw` query suffix natively; vitest inherits it through the
shared resolver. **Two pre-flight checks are required before these
tests can be written**:

1. Confirm `frontend/src/vite-env.d.ts` exists and contains
   `/// <reference types="vite/client" />`. The Vite client types
   declare `*?raw` as `{ default: string }`, which is what the test
   imports rely on. If the file is absent or doesn't reference
   `vite/client`, create / extend it before writing the tests —
   otherwise `npm run typecheck` will fail with TS2307 on the
   `?raw` import.
2. Confirm that production code imports `global.css` only as a
   side effect (the conventional `import './global.css'` in
   `main.tsx`, which Vite injects as a `<style>` tag) — never via
   `?raw` outside tests. Mixing the two import shapes for the same
   file is supported by Vite but invisible to readers; restricting
   `?raw` to test code keeps the production stylesheet path
   conventional.
3. As a 30-second smoke before writing the assertions: import
   any tracked CSS file via `?raw` in a throwaway vitest case and
   `console.log(css)` — confirm the value is the unmodified source
   text (no minification, no class-name hashing). If `?raw`
   doesn't survive the project's vitest config, fall back to
   `fs.readFileSync(new URL('./global.css', import.meta.url),
   'utf8')`, which is config-independent.

### Design-token discipline (shared by 10.4, 10.5, 10.6, 10.7)

To prevent the contrast-test-drift gap the review flagged
(hex literals duplicated between CSS and tests), tokens live in a
single TypeScript module and the CSS imports them at build time
via Vite's `?raw` mechanism. The contrast test then reads from the
same source as production:

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
(new)

```ts
// Single source of truth for design tokens. CSS pulls these via
// the small `tokens-to-css.ts` adapter; tests assert contrast
// against the same values.
export const COLOR_TOKENS = {
  // AA-grade body / muted text on white.
  'color-text':              '#0f172a',
  'color-muted-text':        '#4b5563',  // 7.6:1 on #fff
  // Decorative-only muted (icons, dividers, secondary fills);
  // *not* used for text. Kept lighter to preserve hierarchy on
  // surfaces previously using `--color-muted` for non-text uses.
  'color-muted-decorative':  '#9ca3af',
  'color-divider':           '#e5e7eb',
  'color-focus-ring':        '#2563eb',  // 4.7:1 on #fff
  'color-warning-bg':        '#fff8e6',
  'color-warning-border':    '#d97706',
  'color-warning-text':      '#7c2d12',  // 5.6:1 on #fff8e6
} as const
export type ColorToken = keyof typeof COLOR_TOKENS
```

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
(new or extend) — emits a `:root { … }` block from the tokens
module via a tiny build-time adapter (or, if the build pipeline
isn't suitable for that, by hand-mirroring the values *with* the
contrast test reading them back from the same module — see Sub-
step C).

```css
/* Generated from styles/tokens.ts — keep in sync (test enforces). */
:root {
  --color-text:              #0f172a;
  --color-muted-text:        #4b5563;
  --color-muted-decorative:  #9ca3af;
  --color-divider:           #e5e7eb;
  --color-focus-ring:        #2563eb;
  --color-warning-bg:        #fff8e6;
  --color-warning-border:    #d97706;
  --color-warning-text:      #7c2d12;
}
```

A separate `tokens-vs-css.test.ts` (Sub-step C) parses
`global.css` and asserts every `--color-*` value equals
`COLOR_TOKENS[name]`, so drift in either direction is caught.

### Sub-step A: Visible focus rings + forced-colors support

#### Failing test

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.tsx` (new)

```ts
import { describe, it, expect } from 'vitest'
import globalCss from './global.css?raw'

describe('global focus rings', () => {
  it('declares :focus-visible with an outline', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline:[^;]+;/)
  })

  it('declares an outline-offset for breathing room', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline-offset:[^;]+;/)
  })

  it('overrides the focus-ring colour under forced-colors mode', () => {
    expect(globalCss).toMatch(
      /@media\s*\(forced-colors:\s*active\)\s*\{[^}]*:focus-visible[^}]*outline-color:\s*Highlight/i,
    )
  })
})
```

(Reading the source via `?raw` — Vite's raw-import suffix — keeps
the test cheap and avoids jsdom stylesheet quirks.)

#### Implementation

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css` (extend the `:root` block above)

```css
:focus-visible {
  outline: 2px solid var(--color-focus-ring);
  outline-offset: 2px;
}

/* Windows High Contrast / forced-colors: let the UA repaint the
   outline using its system colour palette. WCAG 2.4.7 / 2.4.11. */
@media (forced-colors: active) {
  :focus-visible {
    outline-color: Highlight;
  }
}
```

Import once in `main.tsx`.

The "outline-offset clipping on dense surfaces" risk is bounded
to a manual-verification sweep (see Manual Verification below).
If clipping is observed in practice during the sweep, swap the
affected card-level styles to `box-shadow: 0 0 0 2px
var(--color-focus-ring)` (which is not clipped by ancestor
`overflow: hidden`).

### Sub-step B: dnd-kit screen-reader announcements

#### Failing tests

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/announcements.test.ts` (new)

The announcement strings include the ticket *number* (parsed from
the entry's `relPath` `NNNN-` prefix) and map column ids to human-
readable labels via the existing `STATUS_COLUMNS` table.

```ts
import { describe, it, expect } from 'vitest'
import { buildKanbanAnnouncements, ticketNumberFromRelPath } from './announcements'
import type { IndexEntry } from '../../api/types'

const fooEntry = {
  type: 'tickets' as const,
  path: '/abs/meta/tickets/0001-foo.md',
  relPath: 'meta/tickets/0001-foo.md',
  slug: 'foo',
  frontmatter: {},
  frontmatterState: 'parsed' as const,
  title: 'Foo',
  mtime: 0,
  size: 0,
  etag: 'sha256-x',
} as unknown as IndexEntry

describe('ticketNumberFromRelPath', () => {
  it('extracts the NNNN- prefix', () => {
    expect(ticketNumberFromRelPath('meta/tickets/0001-foo.md')).toBe('0001')
    expect(ticketNumberFromRelPath('meta/tickets/0042-bar.md')).toBe('0042')
  })

  it('returns null when the prefix is missing', () => {
    expect(ticketNumberFromRelPath('meta/tickets/foo.md')).toBeNull()
  })
})

describe('buildKanbanAnnouncements', () => {
  const entriesMap = new Map([[fooEntry.relPath, fooEntry]])
  const a = buildKanbanAnnouncements({ entries: () => entriesMap })

  it('onDragStart includes the ticket number and title (colon separator)', () => {
    const msg = a.onDragStart!({ active: { id: 'meta/tickets/0001-foo.md' } } as any)
    // Colon (not em-dash) — VoiceOver/NVDA/JAWS all read ':' as a
    // brief pause; em-dash pronunciation varies across vendors.
    expect(msg).toBe('Picked up ticket 0001: Foo.')
  })

  it('onDragEnd maps column id to its display label', () => {
    const msg = a.onDragEnd!({
      active: { id: 'meta/tickets/0001-foo.md' },
      over:   { id: 'in-progress' },
    } as any)
    expect(msg).toBe('Moved ticket 0001: Foo to In progress.')
  })

  it('onDragOver omits announcement when there is no over target', () => {
    const msg = a.onDragOver!({
      active: { id: 'meta/tickets/0001-foo.md' },
      over:   null,
    } as any)
    expect(msg).toBeUndefined()
  })

  it('onDragCancel labels the cancellation', () => {
    const msg = a.onDragCancel!({ active: { id: 'meta/tickets/0001-foo.md' } } as any)
    expect(msg).toBe('Drag of ticket 0001: Foo cancelled.')
  })
})
```

#### Implementation

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/announcements.ts` (new)

```ts
import type { Announcements } from '@dnd-kit/core'
import type { IndexEntry } from '../../api/types'
import { STATUS_COLUMNS, OTHER_COLUMN } from '../../api/types'

interface Deps {
  entries: () => Map<string, IndexEntry>
}

export function ticketNumberFromRelPath(relPath: string): string | null {
  const m = /(\d{4})-/.exec(relPath.split('/').pop() ?? '')
  return m ? m[1] : null
}

function labelFor(columnId: unknown): string {
  const id = String(columnId)
  return STATUS_COLUMNS.find(c => c.key === id)?.label
    ?? (OTHER_COLUMN.key === id ? OTHER_COLUMN.label : id)
}

function describe(id: unknown, entries: Map<string, IndexEntry>): string {
  if (typeof id !== 'string') return `ticket ${String(id)}`
  const entry = entries.get(id)
  const num = ticketNumberFromRelPath(id)
  const title = entry?.title
  // Use ':' (not em-dash) as the number/title separator — colons
  // are read as a brief pause across all major screen readers,
  // whereas em-dash pronunciation varies (VoiceOver: "em dash" or
  // silent pause; NVDA: depends on punctuation level; JAWS: often
  // skipped). The intended-improvement audience is the most
  // affected by inconsistent pronunciation.
  if (num && title) return `ticket ${num}: ${title}`
  if (title) return `ticket ${title}`
  return `ticket ${id}`
}

export function buildKanbanAnnouncements(
  { entries }: Deps,
): Announcements {
  return {
    onDragStart({ active }): string {
      return `Picked up ${describe(active.id, entries())}.`
    },
    onDragOver({ active, over }): string | undefined {
      if (!over) return undefined
      return `${describe(active.id, entries())} is over ${labelFor(over.id)}.`
    },
    onDragEnd({ active, over }): string {
      if (!over) return `Drop of ${describe(active.id, entries())} cancelled, no target.`
      return `Moved ${describe(active.id, entries())} to ${labelFor(over.id)}.`
    },
    onDragCancel({ active }): string {
      return `Drag of ${describe(active.id, entries())} cancelled.`
    },
  }
}
```

Note: explicit `: string` / `: string | undefined` return
annotations on each callback lock the contract at the call site
rather than relying on `Announcements`-type inference (which has
shifted across `@dnd-kit/core` minor versions).

Wire into `KanbanBoard.tsx`:

```tsx
// Stable announcements object across renders. The `entries` thunk
// reads via a ref so dnd-kit always sees the freshest map at
// drag-event time; the `Announcements` object itself never needs
// to be re-built when entries change.
const entriesRef = useRef(entriesByRelPath)
useEffect(() => { entriesRef.current = entriesByRelPath }, [entriesByRelPath])

const announcements = useMemo(
  () => buildKanbanAnnouncements({ entries: () => entriesRef.current }),
  [],   // built once; thunk reads through the ref
)

return (
  <DndContext
    sensors={sensors}
    collisionDetection={closestCorners}
    onDragStart={handleDragStart}
    onDragEnd={handleDragEnd}
    accessibility={{ announcements }}
  >
    {/* ... */}
  </DndContext>
)
```

The `useRef` indirection makes the deferred-read intent of the
thunk meaningful — the `Announcements` object is created once and
always observes the latest map at drag-event time.

### Sub-step C: WCAG 2.2 AA contrast pass with token-vs-CSS drift guard

#### Manual audit

Audit with WebAIM contrast checker (https://webaim.org/resources/contrastchecker/):

| Surface                         | Foreground / Background      | Current ratio | Target | Action |
|---------------------------------|------------------------------|---------------|--------|--------|
| Body text                       | `#0f172a` on `#fff`          | 19.8:1        | 4.5:1  | OK     |
| Sidebar muted link (text)       | `#9ca3af` on `#fff`          | 2.85:1        | 4.5:1  | Switch to `--color-muted-text` (`#4b5563`) — 7.6:1. Decorative `--color-muted-decorative` (`#9ca3af`) preserves the existing hierarchy on non-text surfaces. |
| FrontmatterChip text            | `#6b7280` on `#f3f4f6`       | 3.6:1         | 4.5:1  | Bump foreground to `#374151` — 7.7:1 |
| Conflict banner text            | `#7f1d1d` on `#fef2f2`       | 8.2:1         | 4.5:1  | OK |
| Focus ring vs. white            | `#2563eb` on `#fff`          | 4.7:1         | 3:1    | OK |
| `unresolved-wiki-link` underline| `#9ca3af` on `#fff`          | 2.85:1        | 3:1    | Bump to `#6b7280` — 4.7:1 |
| Warning banner text             | `#7c2d12` on `#fff8e6`       | 5.6:1         | 4.5:1  | OK (used by Phase 10.6 banner and SidebarFooter reconnect text) |

#### Failing tests — token / CSS drift guard + ratio guard

**File**: `skills/visualisation/visualise/frontend/src/styles/contrast.test.ts` (new)

Reads tokens from the single source of truth; reads CSS values
from the actual `global.css` source — drift in either direction
fails the test.

```ts
import { describe, it, expect } from 'vitest'
import globalCss from './global.css?raw'
import { contrastRatio } from './contrast'
import { COLOR_TOKENS } from './tokens'

function readCssVar(name: string): string | null {
  const re = new RegExp(`--${name}:\\s*([^;]+);`)
  const m = re.exec(globalCss)
  return m ? m[1].trim() : null
}

describe('tokens.ts is the single source of truth for :root colour values', () => {
  for (const [name, value] of Object.entries(COLOR_TOKENS)) {
    it(`global.css :root --${name} matches COLOR_TOKENS.${name}`, () => {
      expect(readCssVar(name)).toBe(value)
    })
  }
})

describe('design-token contrast (WCAG 2.2 AA)', () => {
  it('muted-text on white passes AA for normal text (4.5:1)', () => {
    expect(
      contrastRatio(COLOR_TOKENS['color-muted-text'], '#ffffff'),
    ).toBeGreaterThanOrEqual(4.5)
  })
  it('warning-text on warning-bg passes AA for normal text (4.5:1)', () => {
    expect(
      contrastRatio(COLOR_TOKENS['color-warning-text'], COLOR_TOKENS['color-warning-bg']),
    ).toBeGreaterThanOrEqual(4.5)
  })
  it('focus-ring on white passes AA for UI components (3:1)', () => {
    expect(
      contrastRatio(COLOR_TOKENS['color-focus-ring'], '#ffffff'),
    ).toBeGreaterThanOrEqual(3)
  })
})
```

`contrast.ts` is a small WebAIM-luminance helper (~30 lines).
Add 3-4 fixture cases to a sibling `contrast-helper.test.ts`
against WebAIM-published reference ratios so a bug in the helper
itself doesn't silently invalidate every token assertion:
- `contrastRatio('#000000', '#ffffff')` ≈ 21:1
- `contrastRatio('#777777', '#ffffff')` ≈ 4.48:1
- `contrastRatio('#ffffff', '#ffffff')` ≈ 1:1
- `contrastRatio('#ff0000', '#ffffff')` ≈ 4:1 (≥3.99, ≤4.01)

### Success Criteria

#### Automated Verification

- [ ] `npm test` (vitest) green: focus-ring rule (3 tests), dnd-kit
  announcement builder (4 tests + `ticketNumberFromRelPath` 2
  tests), token-vs-CSS drift (one per token), AA-contrast (3
  tests).
- [ ] `npm run typecheck` green.
- [ ] `npm run lint` green.

#### Manual Verification

- [ ] Tab through the entire app — every focusable element shows
  a visible blue outline.
- [ ] **Outline-clipping sweep**: scan the kanban (cards inside
  columns), the lifecycle pipeline-dots row, the chip rows, the
  wiki-link inline spans, the SidebarFooter elements. Note any
  surface where the focus ring is partly clipped by an ancestor
  with `overflow: hidden`; for those, swap the `outline` rule to
  `box-shadow: 0 0 0 2px var(--color-focus-ring)` at the
  component level.
- [ ] **`--color-muted-*` cascade sweep**: open the sidebar
  Templates section, the version footer, frontmatter chips, and
  any other surface previously consuming `--color-muted`. Confirm
  visual hierarchy still reads as intended after the
  text/decorative split.
- [ ] **Forced-colors mode**: enable Windows High Contrast (or
  macOS "Increase contrast") and confirm focus rings repaint
  correctly using system colours.
- [ ] With VoiceOver (or NVDA) enabled, drag a ticket via
  keyboard and hear "Picked up ticket 0001: Foo", "Moved ticket
  0001: Foo to In progress" rather than the dnd-kit defaults.
- [ ] Visually scan the sidebar muted-link group, frontmatter
  chips, and unresolved wiki-link spans — all read as legible
  text against their background.

---

## Testing Strategy

### Unit tests

#### Server (`cargo test -p accelerator-visualiser`)

Phase 10.2 (logging):
- `log::tests::rotates_when_active_segment_crosses_5_megabytes`
- `log::tests::retains_at_most_three_rotated_segments`
- `log::tests::active_and_rotated_segments_are_owner_only` (unix only)
- `log::tests::make_writer_errors_when_parent_path_is_a_file`
- `log::tests::pre_existing_oversized_log_rotates_on_first_write`
- `log::tests::emits_json_lines_with_message_and_field` (scoped subscriber via `test_support`)
- `server::tests::request_emits_trace_summary_line_with_method_uri_status_and_latency`
  (scoped subscriber via `test_support`; structural-key matching)
- `main::tests::redirect_std_streams_to_devnull_succeeds_on_normal_unix` (unix only)
- `main::tests::redirect_std_streams_to_devnull_in_subprocess_isolates_stderr` (unix only)

Phase 10.3 (version header — three layer-ordering tests):
- `server::tests::responses_carry_version_header_on_normal_route`
- `server::tests::version_header_present_on_404_fallback`
- `server::tests::version_header_present_on_host_header_guard_rejection`

Phase 10.4 (`/api/info`):
- `api::info::tests::info_returns_name_and_version_with_no_cache`

#### Frontend (`npm test`, vitest)

Phase 10.4:
- `use-server-info.test.tsx` — 3 cases (success, 500, missing field)
- `SidebarFooter.test.tsx` — 6 cases (visible-text version label,
  reserved empty footer in `'connecting'`, `'connecting'` does NOT
  show "Reconnecting…", `'reconnecting'` shows "Reconnecting…",
  `justReconnected` shows "Reconnected — refreshing",
  mutual-exclusion of the two `role="status"` regions)

Phase 10.5 (`reconnecting-event-source.test.ts`):
- `computeBackoff` — start/double, cap, extreme-attempt overflow,
  jitter range
- `ReconnectingEventSource` — deterministic-backoff reconnect,
  no-reconnect-on-first-open, error-then-first-open recovery sweep,
  state-observable
  (`['connecting', 'open', 'reconnecting', 'open', 'closed']`),
  initial `'connecting'` notification, from-boot
  `'connecting' → 'reconnecting'` transition, constructor
  factory-throw surfaces via `onerror` opt, repeated-error
  idempotency, post-close resurrection guard, re-entrant
  `close()` from inside `onerror`, factory-throws-then-retries
- `useDocEvents` integration — `connectionState` is reactive;
  `onReconnect` invokes `invalidateQueries` with a predicate
  excluding `'server-info'`; `serverInfo` cache survives
  reconnect while `'docs'` is invalidated; `justReconnected`
  3 s timer cleanup on unmount; flap-then-recovery within 3 s
  extends the toast rather than ending early

Phase 10.6 (`LibraryDocView.test.tsx` extension):
- malformed → banner present (queried via `aria-label`, must NOT
  carry `role="status"`)
- parsed / absent → no banner
- mid-session SSE `doc-invalid` flips state and the banner appears
  without a manual refresh

Phase 10.7:
- `styles/global.test.tsx` — `:focus-visible` rule + outline + offset
  + forced-colors override (3 assertions)
- `kanban/announcements.test.ts` — `ticketNumberFromRelPath` (2
  cases) + `buildKanbanAnnouncements` (4 callbacks; ticket-number
  + column-label assertions; colon separator pinned)
- `styles/contrast.test.ts` — token-vs-CSS drift (one assertion per
  token in `COLOR_TOKENS`) + 3 AA contrast assertions
- `styles/contrast-helper.test.ts` — 4 WebAIM-fixture cases for the
  `contrastRatio` helper itself

### Integration tests

- Server: existing axum oneshot tests automatically pick up the
  `accelerator-visualiser-version` header on every response (header is
  set by an outermost middleware; no per-endpoint test changes
  needed). Add a single sanity check to one existing endpoint test
  if convenient.
- Server: `/api/info` endpoint integration is covered by the
  module-level `info_returns_name_and_version_with_no_cache` test.
- Server: log file presence + rotation verified by the unit tests
  above (no separate integration test needed — `make_writer` is
  exercised end-to-end against a real `tempfile`).

### Bash tests

`test-launch-server.sh` adds:
- `test_uninitialised_project_is_rejected` (10.1)
- `test_initialised_project_proceeds_past_sentinel` (10.1)
- `test_sentinel_does_not_kill_already_running_server` (10.1 —
  reuse short-circuit precedes sentinel; seeds `start_time` from
  `start_time_of $$` so the identity check actually matches)
- `test_bootstrap_log_is_separate_and_exclusive_of_server_log`
  (10.2 — sentinel-string assertion ensures launcher's nohup
  capture no longer touches `server.log`)
- `test_bootstrap_log_is_owner_only` (10.2 — locks the 0o600 mode
  override even under a permissive shell umask)

Prerequisites:
- `make_project()` updated to also seed `meta/tmp/.gitignore`.
- `assert_dir_absent`, `assert_file_contains`,
  `assert_file_not_contains`, `stat_mode` added to
  `scripts/test-helpers.sh` if not already present.
- `make_fake_visualiser` extended to accept a `--stderr` flag for
  emitting a fixed sentinel string (one-line addition).

### Manual verification

The "Manual Verification" checklists in each inner phase form the
end-to-end smoke test for Phase 10. Run them once on macOS arm64
(primary dev host); rerun on Linux x64 once if a release follows
this phase before Phase 12. The Phase 10.1 onboarding flow (fresh
uninitialised project → see hint → run `/accelerator:init` → re-
invoke) should be exercised inside a real Claude Code session so
the slash-command rendering of the JSON error is observed end to
end.

### End-to-end (deferred to Phase 11)

The user-visible reconnecting indicator (Phase 10.5) and the
malformed-frontmatter banner (Phase 10.6) are both candidates for
Playwright coverage in Phase 11. Phase 10 ships them with vitest
unit / integration coverage and manual smoke; Phase 11's
Playwright suite picks up the real-browser SSE-reconnect flow
explicitly.

---

## Performance Considerations

- **`tracing-appender::non_blocking`** moves all log writes off the
  Tokio runtime onto a background thread. The trade-off is a small
  in-memory buffer (~128 KB by default) and a worker guard whose
  drop must complete before the process exits — `_log_guard` in
  `main` covers this for graceful shutdown. Worst case: a log
  burst that exceeds the buffer drops oldest events, which is
  acceptable for an observability feature on a localhost service.
  **Crash-time observability is partial**: on `panic!` (with
  `panic = abort`) or `std::process::abort`, the worker thread is
  terminated with pending lines still in the channel. Operators
  diagnosing a crash should also consult
  `<tmp>/visualiser/server.bootstrap.log` for any pre-init or
  panic-time payload that escaped via the launcher's nohup
  capture.
- **`file-rotate`** keeps the active file open and does the rotation
  rename in-process. At 5 MB / segment with 3 segments retained,
  peak disk usage is ~20 MB, far below the launcher's tmp space
  budget. Rotation is synchronous on the calling write — under
  exceptional traffic the cross-cap write may take a few ms.
- **`Accelerator-Visualiser-Version` middleware** adds one
  `HeaderMap::insert` per response — sub-microsecond.
- **`/api/info`** serves a static JSON object built from `&'static
  str`s at build time. `Cache-Control: no-cache` adds one header.
- **`ReconnectingEventSource`** sleeps in `setTimeout` between
  attempts; its only RAM cost is the closure and timer handle. The
  invalidate-on-reconnect sweep uses a predicate that excludes
  `'server-info'` (and any future session-stable keys) to avoid
  refetching state that doesn't change for the connection's
  lifetime. Refetches fire for every other mounted observer —
  typically 2–6 in v1 — well within budget for a localhost server.
- **Visible focus ring** is a single CSS rule — zero runtime cost.

---

## Migration Notes

- **No data migrations.** The visualiser writes only to ticket
  frontmatter (Phase 8); Phase 10 adds no new on-disk format.
- **Log file ownership splits.** Phase 1–9 launchers redirected the
  Rust process's stdout/stderr into `<tmp>/visualiser/server.log`
  via `nohup … >>"$LOG_FILE" 2>&1`. Phase 10 retargets that
  redirection to `<tmp>/visualiser/server.bootstrap.log` and the
  Rust process takes exclusive ownership of `server.log` via
  `file-rotate`. Pre-existing oversized `server.log` files from
  earlier builds rotate cleanly on first write under the new
  binary (covered by
  `pre_existing_oversized_log_rotates_on_first_write`). Operators
  who tail-watched `server.log` see no change; tools that grepped
  the file for stderr panic payloads should now also consult
  `server.bootstrap.log`.
- **Spec amendment for the response header**. The spec's
  `X-Accelerator-Visualiser` lands as the unprefixed
  `Accelerator-Visualiser-Version` (RFC 6648). The header is
  brand-new in this phase, so no consumer change is required —
  but the spec amendment is part of the phase's deliverable.
- **`/api/info` is additive.** Existing clients continue to work
  unchanged. The payload includes `name` so callers can verify
  endpoint identity unambiguously.
- **`ReconnectingEventSource` transport upgrade**. Existing
  event-handling code in `useDocEvents` continues to receive the
  same SSE events through the same callback shape, but three
  behaviours change: (1) `useDocEvents.onerror` no longer
  eager-invalidates `['docs']` — the wrapper backs off and the
  `onReconnect` sweep covers cache staleness on recovery; (2) the
  reconnect sweep now uses `invalidateQueries({ predicate })`
  excluding `SESSION_STABLE_QUERY_ROOTS` (currently
  `'server-info'`) so the version footer doesn't flicker; (3)
  `connectionState` exposes a four-value enum (`'connecting' |
  'open' | 'reconnecting' | 'closed'`) — the new `'connecting'`
  initial state lets `<SidebarFooter>` distinguish bootstrap
  from recovery and show no indicator on first load.
- **Sidebar prop shape unchanged.** `<Sidebar>` keeps its
  `{ docTypes }` prop. Version footer + reconnecting indicator
  live in a new `<SidebarFooter>` consuming `useServerInfo` /
  `useDocEvents` directly. Existing `Sidebar.test.tsx` tests pass
  unchanged.
- **Design-token split.** The previous unitary `--color-muted`
  splits into `--color-muted-text` (AA-grade text) and
  `--color-muted-decorative` (lighter, non-text uses only).
  Component-level usages auditable via grep — only text usages
  switch to the new `-text` variant.
- **Plugin version bump.** This phase's diff bumps the plugin
  patch version once it lands (consistent with prior phases). The
  release pipeline (Phase 12) handles the actual binary cut.

---

## References

### Spec & research

- `meta/specs/2026-04-17-meta-visualisation-design.md` — spec, esp.
  "Failure modes" matrix and "Non-functional > Observability,
  Accessibility, Versioning."
- `meta/research/2026-04-17-meta-visualiser-implementation-context.md` —
  Phase 10 description (lines 1128–1138), §3 init sentinel, D8/D10
  release pipeline.

### Existing code touched

- `skills/visualisation/visualise/scripts/launch-server.sh:14-19` —
  current TMP resolution (sentinel check inserts here).
- `skills/visualisation/visualise/scripts/test-launch-server.sh` —
  bash test harness pattern.
- `skills/visualisation/visualise/server/src/main.rs:17-20` —
  current `tracing_subscriber::fmt().json().init()` (replaced).
- `skills/visualisation/visualise/server/src/server.rs:154,175-177` —
  `/api/healthz` route (sibling to new `/api/info`).
- `skills/visualisation/visualise/server/src/server.rs:215` —
  `env!("CARGO_PKG_VERSION")` already in scope for the header.
- `skills/visualisation/visualise/server/src/frontmatter.rs:76-89` —
  `FrontmatterState` (already drives `frontmatterState` on the wire).
- `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:78-153`
  — current `EventSource` wiring (transport upgraded).
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
  — version footer + reconnecting indicator.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:68-107`
  — banner injection point.
- `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.tsx:154-159`
  — `DndContext` accessibility prop wiring point.

### Sentinel pattern reference

- `scripts/config-summary.sh:19-21` — exact form of the
  `[ ! -f "$TMP_PATH/.gitignore" ] && INITIALISED=false` check the
  launcher mirrors.
- `skills/config/init/SKILL.md:58-77` — origin of the sentinel.

### External crates introduced

- `file-rotate` 0.7 — size-based rotating writer.
- `tracing-appender` 0.2 — non-blocking writer adapter.

### Spec-mandated header / endpoint shapes

- `Accelerator-Visualiser-Version: <semver>` — spec "Versioning"
  section. Spec amendment in this phase replaces the
  `X-`-prefixed name (RFC 6648 deprecates `X-` for new headers).
- `GET /api/info` returning `{"name":"accelerator-visualiser",
  "version":"<semver>"}` with `Cache-Control: no-cache` — derived
  from the spec's response-header + "UI shows the version in a
  sidebar footer" pair (the response header alone wouldn't survive
  a future proxy/tunnel, hence the JSON endpoint).
