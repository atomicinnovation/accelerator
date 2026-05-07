# Playwright Executor — Wire Protocol Reference

`run.sh` wraps a Node.js TCP daemon (`run.js`) that drives Playwright.
The daemon and its clients speak a JSON protocol over HTTP/1.1 on a
`127.0.0.1` loopback port. This document is the canonical reference for
the protocol so that agent bodies and future callers can stay in sync
without reading the source.

For the executor wire schema see this document.

## Transport

- **Protocol**: HTTP/1.1, `POST /`, body is a UTF-8 JSON object.
- **Address**: `127.0.0.1:<port>` (OS-assigned; port is written to
  `server-info.json` in the state directory on daemon startup).
- **One request per connection**: each TCP connection carries exactly one
  request/response pair. The client closes after reading the response.
- **Loopback only**: the daemon never binds to a non-loopback interface.
  External callers cannot reach it.

## Request Envelope

```json
{ "protocol": 1, "command": "<op>", ...op-specific fields }
```

| Field       | Type   | Required | Description                    |
|-------------|--------|----------|--------------------------------|
| `protocol`  | number | yes      | Must be `1`                    |
| `command`   | string | yes      | Op name (see table below)      |
| *op fields* | varies | varies   | Per-op args (see table below)  |

## Success Response Envelope

Success responses always include `"protocol": 1` plus op-specific fields.
Unknown fields in a success response are safe to ignore.

## Error Response Envelope

```json
{
  "protocol": 1,
  "error":    "<kebab-code>",
  "message":  "<human-readable string>",
  "category": "<category>",
  "retryable": false,
  "details":  { ... }
}
```

| Field       | Type    | Always present | Description                                    |
|-------------|---------|----------------|------------------------------------------------|
| `protocol`  | number  | yes            | Always `1`                                     |
| `error`     | string  | yes            | Kebab-case error code (see per-op table)       |
| `message`   | string  | yes            | Human-readable explanation                     |
| `category`  | string  | yes            | One of `usage | protocol | browser | bootstrap | filesystem` |
| `retryable` | boolean | yes            | `true` if the caller may retry immediately     |
| `details`   | object  | no             | Structured extras (op-specific)                |

### Category enum

| Category    | Meaning                                                              |
|-------------|----------------------------------------------------------------------|
| `usage`     | Caller error — missing required field, unknown command               |
| `protocol`  | Protocol-level error — invalid JSON, protocol version mismatch       |
| `browser`   | Browser or Playwright runtime error — navigation, screenshot, timeout |
| `bootstrap` | Chromium not installed or not found                                  |
| `filesystem`| Path guard rejection — screenshot path outside allowed directory     |

## Subcommand Reference

### `ping`

Check daemon health and verify the Chromium binary is accessible.

**Request**
```json
{ "protocol": 1, "command": "ping" }
```

**Success response**
```json
{ "protocol": 1, "ok": true, "node": "v22.0.0", "playwright": "1.49.1", "chromium": "/path/to/chromium" }
```

**Error codes**

| Code                | Category    | Retryable | Condition                            |
|---------------------|-------------|-----------|--------------------------------------|
| `chromium-not-found`| `bootstrap` | false     | Chromium binary missing from disk    |

---

### `navigate`

Navigate the browser page to a URL.

**Request**
```json
{ "protocol": 1, "command": "navigate", "url": "http://localhost:3000/" }
```

**Success response**
```json
{ "protocol": 1, "ok": true, "url": "http://localhost:3000/" }
```

*`url` in the response is the final URL after any redirects.*

**Error codes**

| Code                | Category  | Retryable | Condition                           |
|---------------------|-----------|-----------|-------------------------------------|
| `missing-url`       | `usage`   | false     | `url` field absent                  |
| `wall-clock-exceeded`| `browser`| false     | Op exceeded per-op wall-clock budget|
| `internal-error`    | `browser` | false     | Unexpected Playwright exception     |

---

### `snapshot`

Capture the accessibility tree of the current page.

**Request**
```json
{ "protocol": 1, "command": "snapshot" }
```

**Success response**
```json
{ "protocol": 1, "snapshot": { ...accessibility-tree } }
```

**Error codes**

| Code                | Category  | Retryable | Condition                            |
|---------------------|-----------|-----------|--------------------------------------|
| `wall-clock-exceeded`| `browser`| false     | Op exceeded per-op wall-clock budget |
| `internal-error`    | `browser` | false     | Unexpected Playwright exception      |

---

### `screenshot`

Save a screenshot of the current page to an absolute path.

**Request**
```json
{
  "protocol": 1,
  "command": "screenshot",
  "path": "/abs/path/to/screenshot.png",
  "mask": ["[type=password]", "[data-secret]"],
  "full_page": false
}
```

| Field      | Type     | Required | Default | Description                              |
|------------|----------|----------|---------|------------------------------------------|
| `path`     | string   | yes      | —       | Absolute path for the output file        |
| `mask`     | string[] | no       | `[]`    | Additional CSS selectors to mask         |
| `full_page`| boolean  | no       | `false` | Capture the full scrollable page         |

The executor always masks `[type=password]`, `[autocomplete*=token]`, and
`[data-secret]` regardless of the `mask` field. The `mask` field appends
additional selectors.

**Success response**
```json
{ "protocol": 1, "ok": true, "path": "/abs/path/to/screenshot.png" }
```

**Error codes**

| Code                  | Category      | Retryable | Condition                                   |
|-----------------------|---------------|-----------|---------------------------------------------|
| `path-outside-allowed`| `filesystem`  | false     | `path` resolves outside `<state-dir>/screenshots/` |
| `wall-clock-exceeded` | `browser`     | false     | Op exceeded per-op wall-clock budget        |
| `internal-error`      | `browser`     | false     | Unexpected Playwright exception             |

---

### `evaluate`

Run a JavaScript expression in the page context and return its value.

**Request**
```json
{ "protocol": 1, "command": "evaluate", "expression": "document.title" }
```

**Success response**
```json
{ "protocol": 1, "result": "My App" }
```

**v1 default behaviour**: `expression` is forwarded verbatim to
`page.evaluate`. No payload filtering is applied. See *Stability
commitment* below.

**Error codes**

| Code                | Category  | Retryable | Condition                            |
|---------------------|-----------|-----------|--------------------------------------|
| `missing-expression`| `usage`   | false     | `expression` field absent            |
| `wall-clock-exceeded`| `browser`| false     | Op exceeded per-op wall-clock budget |
| `internal-error`    | `browser` | false     | Unexpected Playwright exception      |

---

### `click`

Click an element identified by an accessibility ref or CSS selector.

**Request**
```json
{ "protocol": 1, "command": "click", "ref": "button[type=submit]" }
```

**Success response**
```json
{ "protocol": 1, "ok": true }
```

**Error codes**

| Code                | Category  | Retryable | Condition                            |
|---------------------|-----------|-----------|--------------------------------------|
| `missing-ref`       | `usage`   | false     | `ref` field absent                   |
| `wall-clock-exceeded`| `browser`| false     | Op exceeded per-op wall-clock budget |
| `internal-error`    | `browser` | false     | Unexpected Playwright exception      |

---

### `type`

Fill a form field identified by an accessibility ref or CSS selector.

**Request**
```json
{ "protocol": 1, "command": "type", "ref": "input[name=email]", "text": "user@example.com" }
```

**Success response**
```json
{ "protocol": 1, "ok": true }
```

**Error codes**

| Code                | Category  | Retryable | Condition                            |
|---------------------|-----------|-----------|--------------------------------------|
| `missing-ref`       | `usage`   | false     | `ref` field absent                   |
| `missing-text`      | `usage`   | false     | `text` field absent                  |
| `wall-clock-exceeded`| `browser`| false     | Op exceeded per-op wall-clock budget |
| `internal-error`    | `browser` | false     | Unexpected Playwright exception      |

---

### `wait_for`

Wait until the specified text appears in the page.

**Request**
```json
{ "protocol": 1, "command": "wait_for", "text": "Welcome", "timeout_ms": 5000 }
```

| Field        | Type   | Required | Default                   | Description                             |
|--------------|--------|----------|---------------------------|-----------------------------------------|
| `text`       | string | yes      | —                         | Text to wait for (CSS `:text=` selector) |
| `timeout_ms` | number | no       | per-op wall-clock budget  | Maximum wait in milliseconds; capped to wall-clock budget |

**Success response**
```json
{ "protocol": 1, "ok": true }
```

If `timeout_ms` was capped by the wall-clock budget:
```json
{ "protocol": 1, "ok": true, "truncated": true, "caller_timeout_ms": 30000 }
```

**Error codes**

| Code                | Category  | Retryable | Condition                              |
|---------------------|-----------|-----------|----------------------------------------|
| `missing-text`      | `usage`   | false     | `text` field absent                    |
| `wait-for-timeout`  | `browser` | true      | Text did not appear within timeout     |
| `wall-clock-exceeded`| `browser`| false     | Op exceeded per-op wall-clock budget   |
| `internal-error`    | `browser` | false     | Unexpected Playwright exception        |

---

### `daemon-status`

Query daemon liveness without triggering a browser action.

**Request**
```json
{ "protocol": 1, "command": "daemon-status" }
```

**Success response**
```json
{ "protocol": 1, "state": "running", "pid": 12345 }
```

No error codes — if the daemon is not running, the connection fails before
the request is sent.

---

### `daemon-stop`

Ask the daemon to shut down gracefully.

**Request**
```json
{ "protocol": 1, "command": "daemon-stop" }
```

**Success response**
```json
{ "protocol": 1, "ok": true }
```

The daemon shuts down after sending this response. Subsequent requests
will fail at connection time.

---

## Cross-cutting Error Codes

These codes can be returned by any command regardless of op:

| Code                | Category   | Retryable | Condition                                      |
|---------------------|------------|-----------|------------------------------------------------|
| `protocol-mismatch` | `protocol` | false     | `protocol` field is not `1`                    |
| `invalid-json`      | `protocol` | false     | Request body is not valid JSON                 |
| `unknown-command`   | `usage`    | false     | `command` field names an unrecognised op       |
| `internal-error`    | `browser`  | false     | Unexpected exception during op handling        |
| `no-daemon`         | `usage`    | false     | Client-side: no running daemon; use `run.sh`   |
| `connection-failed` | `protocol` | false     | Client-side: TCP connection refused            |
| `invalid-response`  | `protocol` | false     | Client-side: daemon returned non-JSON response |

---

## Detected-Condition → `notify-downgrade.sh` Enum Mapping

`ensure-playwright.sh` and `run.sh ping` emit structured downgrade signals
so SKILL.md Step 4 can select the right user-facing message.

| Condition                                | Exit | `ACCELERATOR_DOWNGRADE_REASON` | `notify-downgrade.sh --reason` |
|------------------------------------------|------|--------------------------------|--------------------------------|
| `node` not found on `$PATH`              | 10   | `node-missing`                 | `node-missing`                 |
| Node < 20 detected                       | 11   | `node-too-old`                 | `node-too-old`                 |
| Cache filesystem < 500 MB free           | 12   | `disk-floor-not-met`           | `disk-floor-not-met`           |
| Cache directory not writable             | 13   | `cache-unwritable`             | `cache-unwritable`             |
| `npm ci` failed                          | 14   | `bootstrap-failed`             | `bootstrap-failed`             |
| `playwright install chromium` failed     | 15   | `bootstrap-failed`             | `bootstrap-failed`             |
| `run.sh ping` returns error or non-zero  | —    | (caller uses `executor-ping-failed`) | `executor-ping-failed`   |

SKILL.md Steps 4–5 read `ACCELERATOR_DOWNGRADE_REASON` from stderr, then
pass it verbatim to `notify-downgrade.sh --reason <enum>`.

---

## User-Facing Error Rendering

When an agent body surfaces an error envelope to the user, the recommended
format is:

```
inventory-design: <category>: <message> (<error>)
```

Example:
```
inventory-design: browser: Operation exceeded the 300000ms wall-clock budget. (wall-clock-exceeded)
```

The `<error>` code in parentheses aids support diagnostics and log searches.

---

## Stability Commitment

### What v1 guarantees

- All ops listed in this document are stable. Callers can depend on them
  without coordination.
- Additive fields in success responses are safe to ignore.
- The `details` field in error envelopes is informational; callers SHOULD
  NOT branch on its contents.

### What v1 permits as additive (non-breaking) changes

- Opt-in tightening via documented env vars (e.g. a hypothetical
  `ACCELERATOR_PLAYWRIGHT_DENY_LIST=1` that activates a payload deny-list
  off-by-default). Callers that do not set the env var see no behaviour
  change.
- New ops, new error codes, new fields in the response envelope.

### What requires a v2 protocol bump

- Default-on filtering of `evaluate` payloads (i.e. active without an
  env-var opt-in).
- Removing or renaming any op currently in this surface.
- Required new request fields (callers without them break).
- Semantic change to existing error categories or codes.

### Versioning

Clients send `"protocol": 1`; the daemon rejects mismatches with
`protocol-mismatch` (category `protocol`, non-retryable). Future major
versions may be supported side-by-side if needed.
