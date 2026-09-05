---
type: "codebase-research"
id: "2026-08-31-0206-classify-navigation-urls"
title: "Research: Classify Navigation URLs, Not Only The Initial Location (0206)"
date: "2026-08-31T20:24:19+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0206"
parent: "work-item:0206"
topic: "Carrying the AccessPolicy verdict (reachability + scheme) from validate-source through the executor into per-request navigate and links classification"
tags: ["research", "codebase", "design", "security", "ssrf", "playwright", "executor", "access-policy"]
revision: "9bce19ef9c82b2f80944e131090fa8bc46cc6b7e"
repository: "accelerator"
last_updated: "2026-08-31T20:24:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Classify Navigation URLs, Not Only The Initial Location (0206)

**Date**: 2026-08-31T20:24:19+00:00
**Author**: Toby Clemson
**Git Commit**: 9bce19ef9c82b2f80944e131090fa8bc46cc6b7e
**Branch**: (jj working copy; ticket-management workspace)
**Repository**: accelerator

## Research Question

For story 0206, understand the codebase surface needed to carry the
`AccessPolicy` verdict — reachability and scheme together — from the
`validate-source` front door down to per-request `navigate` and `links`
classification in the design crawl, including where allowances must be plumbed,
where redirect enforcement can run, and what already exists versus what is
greenfield.

## Summary

The story's core premise holds exactly: the reachability + scheme verdict is
enforced **only** at `validate-source`, and the executor `navigate`/`links`
path receives no classification at all. The domain logic (`host_reach::classify`,
`access_policy::evaluate`, `Allowances`) is pure, well-tested, Rust-only, and
already the `validate-source` template — so the *decision* is solved. **The work
is entirely about the carrier**, and the carrier is harder than "plumb the
verdict through the executor" suggests, for three structural reasons the story
under-weights.

- **The executor is not a request loop — it is a per-call `execve`.** Each
  `executor navigate '{"url":...}'` is a fresh process that `execve`s
  `node run.js`; the URL is supplied by the **browser agents**
  (`browser-analyser.md`, `browser-locator.md`), not by Rust code that could
  classify it. Per-request allowances therefore ride on each `executor`
  invocation and the browser agents must forward them — the same consumer
  rewiring the story flags for `inventory-design`, but the actual call sites are
  the two agent templates.
- **Redirect enforcement cannot run in Rust.** `page.goto` follows redirects
  inside Chromium; Rust never sees the hops. A route handler is the only place
  to abort before the internal fetch — and it runs in the Node daemon, where the
  Rust classifier does not exist. ⚠️ There is **no active route handler today**:
  `makeAuthHeaderHandler` is imported into `daemon.js` but never installed, and
  no `route.abort()` exists anywhere. The Rust↔JS classifier crossing is the
  unresolved crux.
- **The acceptance-criteria `details` vocabulary does not match the code.**
  `access_policy::evaluate` returns a `Verdict<String>` — a human sentence, not a
  structured `details` object — and `HostReach::description()` emits `RFC1918`,
  not `private`. The AC asks for `details` naming `private` / `link-local` /
  `reserved` / `insecure-scheme`; none of those tokens exist as a machine field
  today.

## Detailed Findings

### Domain layer — the verdict is solved (Rust, `cli/design/`)

The `cli/design/` crate is a pure-logic library; no `clap`, no `main`, no JS
binding. Two functions produce the verdict.

`host_reach::classify(host: &Host) -> HostReach` (`cli/design/src/host_reach.rs:71`)
dispatches on a pre-parsed, canonicalised `Host` (`cli/design/src/host.rs:60`).
The encoding hardening the story cites lives in `Host::canonicalise`
(`cli/design/src/host.rs:72`), not in `classify`: it percent-decodes, rejects
userinfo and control characters, and errors with `HostError::NumericEncoding`
(`host.rs:91`) for decimal/hex/octal/short-form numerics that would otherwise be
mistaken for hostnames. `HostReach` variants are `Loopback`, `Private`,
`LinkLocal`, `Reserved`, `Unspecified`, `Public` (`host_reach.rs:27`); IPv6
transition forms (6to4, Teredo, NAT64, IPv4-mapped, IPv4-compatible) are
unwrapped and re-classified as v4 in `embedded_v4` (`host_reach.rs:142`).

`access_policy::evaluate(location: &SourceLocation, allowances: Allowances) ->
Verdict<String>` (`cli/design/src/access_policy.rs:26`) combines both halves of
the verdict:

1. `Loopback` → accepted unconditionally; `Unspecified` → rejected regardless of
   flags (`access_policy.rs:40`).
2. `Private | LinkLocal | Reserved` → rejected unless `allowances.internal`
   (`access_policy.rs:48`).
3. `Public` + `Scheme::Http` + `!allowances.insecure_scheme` → rejected
   (`access_policy.rs:61`). The insecure-scheme gate applies **only** to public
   hosts; an internal `http` host is judged on `--allow-internal`, not
   `--allow-insecure-scheme`. Ordering: reachability before scheme.

The allowance type is one struct because the two flags "only ever travel
together" (`cli/design/src/access_policy.rs:13`):

```rust
pub struct Allowances {
    pub internal: bool,
    pub insecure_scheme: bool,
}
```

⚠️ **Field names are `internal` / `insecure_scheme`**, not
`allow_internal` / `allow_insecure_scheme`; the `allow_*` names are the clap flag
identifiers only.

### The `validate-source` template (Rust, `cli/design-cli/`)

The clap surface lives in the sibling binary crate. `Command::ValidateSource`
(`cli/design-cli/src/cli.rs:21`) declares both flags; dispatch builds the
`Allowances` and calls the handler (`cli/design-cli/src/main.rs:26`):

```rust
Command::ValidateSource { location, allow_internal, allow_insecure_scheme } =>
    Ok(commands::validate_source(
        &location,
        Allowances { internal: allow_internal, insecure_scheme: allow_insecure_scheme },
        &filesystem::check_directory,
    )),
```

The handler (`cli/design-cli/src/commands.rs:35`) runs `access_policy::evaluate`
and maps `Verdict::Accepted → Report::silent()`,
`Verdict::Rejected(reason) → Report::rejected(&reason)` — the rejection reason
becomes **stderr text at exit 1**. This is the exact pattern the executor must
replicate for the flags, but its output contract (stderr + exit code) is not the
one `navigate` needs (JSON envelope on stdout at exit 0).

### The executor path — no allowances, and a per-call `execve`

`Command::Executor` (`cli/design-cli/src/cli.rs:66`) takes only a positional
`command: String` and `arguments: Vec<String>` (trailing var-args). **No
allowance flags exist** — confirmed against the whole enum. It is special-cased
before the normal dispatch match (`cli/design-cli/src/main.rs:77`):

```rust
if let Command::Executor { command, arguments } = command {
    return executor::run(&command, &arguments);
}
```

`executor::run` (`cli/design-cli/src/executor.rs:100`) gates `command` against
`FORWARDABLE_COMMANDS` (`cli/design/src/executor/forwardable.rs:20` — the only
validation), resolves the runtime, then `launch` hands over. The forward is
verbatim (`cli/design-cli/src/executor.rs:427`):

```rust
let mut forwarded = vec![command.to_owned()];
forwarded.extend_from_slice(arguments);
launcher.launch(&forwarded, Box::new(client))
```

`ExecClient::run` (`cli/design-adapters/src/process.rs:278`) ends in
`command.exec()` — an `execve` of `node run.js <command> <arguments...>`. **The
Rust process replaces itself; there is no persistent Rust loop that sees each
`navigate` URL.** The URL arrives as a JSON trailing arg the browser agents
supply.

```text
browser agent ──exec──▶ design executor navigate '{"url":U}'
                          │  (Rust: forwardable gate only, no classify)
                          ▼  execve
                        node run.js navigate '{"url":U}'
                          │  run.js parses args[1] as JSON
                          ▼  client.js:52  body = {...args, protocol, command}
                        daemon.js  navigate ──▶ page.goto(req.url)   ← no check
```

### The daemon — where URLs are actually dialled (JS)

`navigate` (`skills/design/inventory-design/scripts/playwright/lib/daemon.js:203`)
reads `req.url` and calls `page.goto(req.url, ...)` with no classification;
success is `{ protocol: 1, ok: true, url: page.url() }` (post-redirect final
URL). `links` (`daemon.js:214`) computes `same_origin` **inside the browser** via
`page.evaluate` comparing `u.origin === location.origin`, with an opaque-scheme
guard (`daemon.js:239`). ⚠️ `links` deliberately **strips the href** and returns
only `pathname` + `same_origin` + `scheme` (`daemon.js:247`; PROTOCOL.md:269) to
keep tokens out of agent context — so a policy check that needs the full host
must run *inside* `page.evaluate`, or `evaluate` must return hosts to Node for
classification before stripping.

Error envelopes are built by `makeError` (`lib/errors.js:6`); the `details`
field is optional and object-shaped. A real `retryable: false` + `details`
example already on the wire is `wall-clock-exceeded` (`daemon.js:109`):

```json
{ "protocol": 1, "error": "wall-clock-exceeded", "category": "browser",
  "retryable": false, "details": { "op": "navigate", "wall_clock_ms": 300000 } }
```

PROTOCOL.md:661 notes callers SHOULD NOT branch on `details` — a caveat worth
reconciling with the AC that wants `details` to carry the classification.

All command outcomes return on stdout at exit 0 via the **client** path
(`lib/client.js:77` writes the JSON envelope; `run.js:45` awaits without setting
an exit code). This confirms the story's "an error envelope does not itself abort
the crawl" reasoning.

### Config channels — per-request is the JSON body, not env

Env vars are read once at daemon spawn and are daemon-lifetime
(`ACCELERATOR_PLAYWRIGHT_STATE_DIR`, `..._NS_ROOT`,
`ACCELERATOR_DESIGN_BROWSER_EXECUTABLE`; set at
`cli/design-cli/src/executor.rs:369`, applied to both daemon and client spawn).
Per-request data is the JSON body merged at `lib/client.js:52`
(`{ ...args, protocol, command }`). For per-request allowance scope, the flag
state must ride in that body — the env channel would leak an allowance into a
later reused-daemon invocation, exactly the failure the story rules out.

### Consumer call sites — the browser agents, not just the skill

`inventory-design/SKILL.md:56` forwards `--allow-internal` /
`--allow-insecure-scheme` to `validate-source`, but its only executor calls are
`ping` (SKILL.md:141) and `daemon-stop` (SKILL.md:324) — no allowances. The
actual `navigate`/`links` calls live in the agent templates:
`agents/browser-analyser.md:43` and `agents/browser-locator.md:64` both issue
`accelerator design executor navigate '{"url":...}'` with no allowance flags.
**Those two templates are the rewiring surface** the story's consumer-coupling
note points at.

## Code References

- `cli/design/src/host_reach.rs:71` — `classify`; variants at `:27`; encoding
  unwrap at `:142`; "front door, not a boundary" doc at `:12`
- `cli/design/src/host.rs:72` — `canonicalise`; `NumericEncoding` reject at `:91`
- `cli/design/src/access_policy.rs:26` — `evaluate`; `Allowances` at `:13`;
  insecure-scheme gate at `:61`
- `cli/design/src/host_reach.rs:47` — `HostReach::description()` returns
  `RFC1918` / `link-local` / `reserved` / `wildcard` / `loopback` / `public`
- `cli/design-cli/src/cli.rs:21` — `ValidateSource` flags; `:66` — `Executor`
  (no flags)
- `cli/design-cli/src/main.rs:26` — allowance build; `:77` — executor
  special-case dispatch
- `cli/design-cli/src/commands.rs:35` — `validate_source` handler
  (stderr + exit 1)
- `cli/design-cli/src/executor.rs:427` — verbatim forward seam; `:369` — env vars
- `cli/design-adapters/src/process.rs:278` — `ExecClient::run` `execve`
- `cli/design/src/executor/forwardable.rs:20` — command allowlist
- `.../playwright/lib/daemon.js:203` — `navigate`; `:214` — `links`; `:239` —
  `same_origin`; `:247` — href stripped
- `.../playwright/lib/auth-header.js:27` — `makeAuthHeaderHandler` (imported at
  `daemon.js:13`, never installed); no `route.abort()` anywhere
- `.../playwright/lib/client.js:52` — per-request JSON merge; `:77` — stdout
  envelope
- `.../playwright/lib/errors.js:6` — `makeError`
- `skills/design/inventory-design/SKILL.md:56` — validate-source forwarding
- `agents/browser-analyser.md:43`, `agents/browser-locator.md:64` — executor
  navigate/links call sites (no allowances)
- `skills/design/inventory-design/PROTOCOL.md:152` — navigate wire shape; `:215`
  — links; `:636` — env-var-lifetime note; `:661` — do-not-branch-on-details

## Architecture Insights

- **Two enforcement points, two mechanisms.** The `navigate` request's own URL
  can be classified in Rust before `execve` (at `executor.rs:427`) or in JS at
  `daemon.js:203`. Redirect hops **must** be a JS route handler
  (`ensureBrowser`, `daemon.js:132`) because Rust never sees them. `links`
  refusal must run where the full host survives — inside `page.evaluate` or in
  Node before href-stripping.
- **The Rust↔JS classifier crossing is the real design decision.** Reusing the
  hardened Rust logic from a JS route handler means one of: (a) reimplement
  `host_reach`/`access_policy` in JS — duplicates the subtle encoding defences
  most at risk of drift; (b) a napi/wasm binding for `evaluate`; (c) shell out
  to `design validate-source` per intercepted request. The story says "greenfield
  on both sides of the boundary" but does not name the mechanism — this is the
  load-bearing open question.
- **Per-request scope is naturally satisfied by the `execve` model.** Because
  each `executor navigate` is a fresh process carrying its own JSON body, an
  allowance placed in that body cannot leak into a later invocation — provided it
  rides the JSON channel (`client.js:52`), not env.
- **The `same_origin: false` overload has a token-hygiene wrinkle.** Folding
  policy-refusal into `same_origin` is clean on the wire, but the classification
  must happen before the daemon discards the href, or the signal cannot be
  computed at all.

## Historical Context

- `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md` — parent;
  delivered `validate-source`, `host_reach`, `access_policy`, and the executor
  port this work extends.
- `meta/plans/2026-08-11-0196-design-cli-migration.md` — the plan 0206 derives
  from; the executor/daemon seam originates here.
- `meta/work/0209-wire-up-or-retire-the-header-auth-path.md` — the coordinating
  story; directly relevant because the dormant `makeAuthHeaderHandler`
  (imported, never installed) is 0209's subject and shares the exact
  `ensureBrowser` route seam 0206's redirect handler needs. Sequencing 0206
  first establishes the route-interception plumbing 0209 then extends.
- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
  — prior mapping of the executor/daemon surface.
- `meta/reviews/work/0206-...-review-1.md` — existing review of this story.

## Related Research

- `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- `meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`
- `meta/research/codebase/2026-05-19-inventory-design-and-browser-agent-fixes.md`

## Open Questions

- ❓ **Where does the classifier run for redirect interception?** The route
  handler is JS; the classifier is Rust. Reimplement, bind, or shell out? This
  gates the whole redirect AC and is unresolved in the story.
- ❓ **What structured `details` does `navigate` emit?** The AC names
  `private` / `link-local` / `reserved` / `insecure-scheme`, but the code has no
  machine token — `evaluate` returns a sentence and `description()` says
  `RFC1918`. A new classification enum → wire token is needed, and it must
  reconcile with PROTOCOL.md:661 ("do not branch on `details`").
- ❓ **Which call sites forward allowances?** The story names `inventory-design`,
  but the `navigate`/`links` invocations are in `browser-analyser.md` and
  `browser-locator.md`. Confirm those two agent templates are in scope.
- ❓ **How does `links` classify without the href?** Refusal must fold into
  `same_origin` before the daemon strips the URL — inside `page.evaluate`, or by
  returning hosts to Node. Which?
