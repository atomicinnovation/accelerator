---
type: "pr-description"
id: "64"
title: "accelerator-design: CLI migration and shell-free executor"
date: "2026-08-13T14:02:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0196"
parent: "work-item:0196"
relates_to: ["work-item:0206", "work-item:0207", "work-item:0208", "work-item:0209"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/64"
pr_number: 64
tags: ["rust", "design", "cli", "sub-binary", "executor", "playwright"]
revision: "da9b1803d352bf9a3ed64ec924491d0fbe40b5a2"
repository: "accelerator"
last_updated: "2026-08-13T15:24:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# accelerator-design: CLI migration and shell-free executor

## Summary

Moves the design inventory and gap tooling into an `accelerator-design` dispatched sub-binary and reproduces the Playwright launcher in Rust, so the delegation chain is CLI → Node with no shell in between (ADR-0058). Nine shell scripts and their bash suites are deleted; `run.sh`'s 203 lines of hard-won process, lock and identity handling become domain logic behind injected ports.

The runtime still comes from where it comes from today — `ensure-playwright.sh` survives, and a system Node ≥20 is still required. Vendoring that away is the sibling plan's job, not this one.

## Changes

**New crates.** `cli/design/` (domain), `cli/design-adapters/` (filesystem, process, clock), `cli/design-cli/` (the `accelerator-design` binary), following the corpus/vcs/work precedent. Plus `cli/process-probe/`, which extracts the per-pid start-time probe out of `visualiser/server` onto `libc` alone rather than dragging that crate's axum/tokio graph into the executor's hot path.

**Six subcommands.** `validate-source`, `resolve-auth`, `scrub-secrets`, `notify-downgrade`, `audit-cue-phrases` and `executor`. Exit codes are 0 accept, 1 domain rejection, 2 usage — which splits the conflated `1` two of the scripts used, so a caller can finally tell "the tool refused your input" from "the tool broke". Rejection is a domain verdict rather than a `kernel::Error`, keeping `Refusal`'s documented meaning intact.

**`validate-source` no longer transcribes the shell's regexes.** `classify_internal` matched `::ffff:127.0.0.1` but missed `::ffff:169.254.169.254`, IPv6 unique-local, carrier-grade NAT, and inspected only the first octet for octal encoding — each a route to a link-local metadata endpoint for a tool that then drives a headless browser at the supplied location. Reachability is now classified by parsing with `std::net::IpAddr`, with 6to4, Teredo, NAT64 and IPv4-mapped forms unwrapped and re-classified on their embedded address. Alternate numeric encodings are refused unconditionally, with no recovering flag.

⚠️ **This narrows the accepted set.** Two classes, documented separately because their remedies differ: internal-reach addresses recover with `--allow-internal`; alternate numeric encodings do not recover at all. Loopback keeps its no-flags carve-out, widened from the shell's two literal strings to the whole of `127.0.0.0/8` and `::1`.

**The executor is a port composition.** The reuse verdict is a total pure function over recorded and observed daemon state, so cold start, warm reuse, stale-PID recovery, PID-recycle rejection, lock contention and start timeout are tested without real processes or real elapsed time. `RunClient` is typed as diverging, making it a compile error to sequence domain logic after the `exec` that never returns. The `flock`-or-`mkdir` dichotomy collapses to one backend, since it only existed because the `flock(1)` binary is absent on macOS.

🔒 **The daemon gains a request token.** It served JSON commands on loopback with no authentication, and a loopback TCP socket is not a uid boundary — a different local user on a shared host could drive a browser holding the user's authenticated session. The launcher generates a 128-bit CSPRNG token pre-spawn, hands it over an inherited pipe alongside the daemon's identity, and `daemon.js` requires it by constant-time comparison from its first accepted connection. Requests carrying an `Origin` header are refused outright, closing browser-origin CSRF and DNS rebinding from the pages the crawl itself visits.

**One writer for the daemon's identity.** The launcher observes the start time at fork with the same probe it will later check against, and sends pid, start time, provenance and token down a pipe the daemon reads to EOF *before* publishing anything or binding its socket. `state.js` stops computing a start time entirely — Node has no `sysctl` binding, so its Darwin path was always the weakest fallback. A launcher killed mid-handoff now leaves a daemon that logs and exits before creating any Chromium process, rather than an unsupervised one with a partial record.

**`browser-executor` retires** with the path it existed to resolve: both browser agents call `accelerator design executor` as a bare command, since a plugin's `bin/` is on the Bash tool's `PATH`.

🔒 **CI now rejects a design script grant nothing invokes.** Emptying two script directories left their `allowed-tools` grants behind — Bash access to a directory that no longer exists, and to a `playwright/` tree the skill stopped invoking. The conformance suite gains two checks: concrete paths named in a design SKILL.md or browser agent must exist on disk, and every script grant must have a matching call site in its own skill body. The second is what catches the `playwright/` case, where the directory is still present but nothing reaches into it.

**Four ADRs** land as accepted decisions: 0057 (browser automation as a glibc-only capability), 0058 (shell-free CLI-to-Node delegation), and — governing the *sibling* plan rather than this one — 0059 (build-time assembly of vendored artifacts) and 0060 (launcher-resolved tree artifacts).

⏱️ **Warm executor path: 108.38 ms → 43.95 ms median**, ratio 0.406 against a no-regression gate, measured with interleaved sampling before `run.sh` was deleted. A crawl makes 100–200 of these calls, so roughly 6–13 seconds per crawl. Recorded in `meta/migrations/0196-warm-path-measurement.md`; necessarily a one-time comparison, since the baseline disappears in the same change that measures it.

## Context

- Work item: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- Plan: `meta/plans/2026-08-11-0196-design-cli-migration.md`
- Validation: `meta/validations/2026-08-11-0196-design-cli-migration-validation.md`
- Sibling plan, deliberately not implemented here: `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`

The work item was planned as one eight-phase document and reviewed three times. Every pass closed the previous pass's findings and introduced new criticals, and all of them landed in the tree-artifact, release-pipeline and runtime-swap phases. Splitting on that line let the settled half proceed. Phase numbers are inherited (1, 2, 3, 6 here) rather than renumbered, because a missed cross-reference was precisely the defect class three review passes kept finding.

## Testing

- [x] `mise run check` exits 0
- [x] `mise run test` exits 0
- [x] `mise run test:unit:design-automation` — 78 passed, 0 failed, **0 skipped**, verified against `node --test`'s own TAP accounting rather than a file-count proxy
- [x] `mise run test:integration:design-automation` — 23 of 23, against a real Chromium
- [x] `mise run cli:check`, `lint:dispatch-coherence:check`, `deny:check`, `public-api:check`, `docs:check` — all inside the green `check` roll-up
- [x] Characterization coverage traced through a committed migration checklist (`meta/migrations/0196-design-cli-migration-checklist.md`): every assertion in the deleted bash suites maps to a named Rust test or a recorded deliberate drop naming its replacement property
- [x] `validate-source` exercised by hand across the numeric encodings, transition-encoded metadata endpoints, loopback carve-out, userinfo rejection and `about:blank`
- [ ] The bare default `mise run` — not executed. It is `build` + `fix` + `check` + `test`; the latter two pass and `fix` is mechanical, so the gate is met in substance but not by that exact task
- [ ] Both design skills end to end in a live session, on a machine with a bootstrapped Playwright namespace
- [ ] A live inventory crawl preserving page state across consecutive executor commands

## Notes for Reviewers

**Read the validation report first.** It is committed here and records `result: partial` deliberately. Four defects were found by validating this branch and fixed in it, in the last five commits:

| Defect | Consequence | Fix |
|---|---|---|
| Forwarding allowlist named 7 of 11 commands | `click`, `type`, `wait_for`, `daemon-status` unreachable at exit 2 — the analyser agent's whole interaction surface | allowlist completed; a test now holds it equal to the daemon's own dispatch set |
| `WriterUnavailable` unreachable from disk | daemon respawned on every invocation where `/proc` is unreadable | source read before value; pinned by a fixture both languages assert |
| `test-run.js` never updated for the identity handoff | all 14 of its tests died before asserting anything | handoff pipe and token supplied |
| Daemon wall clock (pre-existing, not from this migration) | backstop pre-empted the graceful path it guards; expiry envelope never terminated its HTTP response | grace period past the budget; complete response |

The first three are migration defects. The fourth was already in the retained daemon and only became visible once the third was fixed.

**Where to focus:**

- **`cli/design/src/executor/reuse.rs`** — the verdict table. Three rows reuse on liveness alone, deliberately, because none carries a validated start time to compare; getting that wrong means either respawning on every command or trusting a recycled pid. No row signals during recovery, and a test asserts that: the contradiction proves the live process is not the recorded daemon, not what it actually is, so SIGTERM would go to whatever now owns the pid.
- **`cli/design-adapters/src/process.rs`** — the `pre_exec` block: `setsid` (not a double fork, so the launcher keeps the pid it must observe), `umask`, and the `dup2` of the pipe's read end. ⚠️ That `dup2` is a POSIX no-op when the descriptor is already 3, which would leave `FD_CLOEXEC` set and kill the daemon at exec. It works today only because the lock fd and bootstrap log are opened first. Known, not fixed here.
- **The exit-code asymmetry** — daemon-side errors exit 0 with the envelope on stdout; launcher-level failures go to stderr non-zero. `SKILL.md` discriminates on exactly that, so collapsing it breaks the skill.

**Residue found by validation, three of four now closed.** The dangling-call-site guard the plan promised but never landed is built; the two dead `allowed-tools` grants it catches are removed; and the migration checklist's thirteen references to five nonexistent test names are corrected, so all 63 Rust tests it cites resolve. Still open, and deliberately: `evals/benchmark.json` grades against deleted scripts, which sits outside the plan's stated documentation scope of `docs-site/`, `README.md` and `CHANGELOG.md`.

**Follow-ups raised:** 0206 (`navigate` URLs are unclassified, so this hardens the front door and not the navigation surface), 0207 (credential scanning is literal-substring only), 0208 (the runtime test lane runs in no build — six of the defects above hid behind that), 0209 (the header-auth path is imported and never called, while documented as security-critical — documentation corrected here, wiring deferred).

**For whoever takes the sibling plan:** its stated edit set is now wrong. It reserved `test-design.sh`'s `SKILL=` assignment and an `SC2016` comment as adjacency traps to leave behind; both were re-homed into the frontmatter-conformance suite anyway, and `test-design.sh` is down to 13 lines. It also still owes the `_EXPECTED_CONFIG_SUITES` 15→14 move in the same change that deletes that file.
