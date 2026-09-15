---
type: "codebase-research"
id: "2026-09-10-0207-detect-encoded-credentials-in-scrubbed-artefacts"
title: "Research: Detect encoded credentials in scrubbed artefacts (0207)"
date: "2026-09-10T23:03:03+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0207"
parent: "work-item:0207"
topic: "Extending the scrub-secrets credential-leak scanner to catch encoded, whitespace-reflowed, and truncated credentials"
tags: ["research", "codebase", "design-cli", "scrub-secrets", "leaked-credentials", "security", "secrets"]
revision: "ad8172ca94e09723373c645ff53ea97e7b78489f"
repository: "accelerator"
last_updated: "2026-09-10T23:03:03+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Resolved the three open questions (message wording, needles() type, cargo-deny)"
schema_version: 1
---

# Research: Detect encoded credentials in scrubbed artefacts (0207)

**Date**: 2026-09-10T23:03:03+00:00
**Author**: Toby Clemson
**Git Commit**: ad8172ca94e09723373c645ff53ea97e7b78489f
**Branch**: HEAD (detached; build-system jj workspace)
**Repository**: accelerator

## Research Question

What is the current implementation surface for `accelerator design scrub-secrets`,
and what must change to detect configured browser credentials that a model has
base64-encoded, percent-encoded, whitespace-reflowed across a line wrap, or
truncated after their head — the scope of work item 0207?

## Summary

The scanner is small, pure, and well-seamed for this change. All matching logic
lives in `cli/design/src/leaked_credentials.rs` — 134 lines, no error type, no
I/O. Needle derivation is the private `NamedSecret::needles()`; the match core is
`scan()`, literal `str::contains` today. The work item's plan maps cleanly onto
this structure: **encoding derivation extends `needles()`**, and the
**whitespace transform belongs in `scan()` on the haystack**, exactly as the
module already separates the two concerns.

Three facts shape the implementation:

- **The binary never writes an artefact.** `scrub_secrets` only reads and scans;
  "The artifact was not written" is an informational string. The guarantee 0207
  must preserve is the exit-1 verdict with no content emitted — not a suppressed
  write. Write suppression is the caller's job, keyed on the exit code.
- **`base64` and `percent-encoding` are already in `cli/Cargo.lock` transitively.**
  Declaring them as direct dependencies pulls no new package, but `cli/design`
  today depends only on `kernel`, so both must be added to `[workspace.dependencies]`
  and `cli/design/Cargo.toml`. ⚠️ cargo-deny runs in CI (`tasks/README.md`) and
  will see the new direct dependencies.
- **Tests exist at three layers with mirrored naming.** Domain unit tests in
  `leaked_credentials.rs`, command tests in `commands.rs`, and binary integration
  tests in `subcommands.rs`. The TDD loop for 0207 adds cases at the domain layer
  first, mirroring the existing behaviour-sentence test names.

The `AUTH_HEADER` value-half split — the contract 0207 consumes — lives in
`needles()`, not in `environment.rs`. `environment.rs` reads the whole raw
`Name: value` pair; the first-colon split happens only for needle generation.

## Detailed Findings

### The scanner core (`cli/design/src/leaked_credentials.rs`)

The module has one input type, one function, one private helper, and no report
struct. A "match" is just the offending variable's name as a `String`.

`NamedSecret` — the input, at `leaked_credentials.rs:13-16`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedSecret {
    pub name: String,
    pub value: String,
}
```

Both fields are `pub`; there is no constructor — callers use struct-literal
syntax. `named_secrets_from_env()` builds these in production; a free `secret()`
helper builds them in tests.

`NamedSecret::needles()` — the extension seam, **private**, at
`leaked_credentials.rs:25-34`:

```rust
fn needles(&self) -> Vec<&str> {
    let mut needles = vec![self.value.as_str()];
    if let Some((_, value)) = self.value.split_once(':') {
        let value = value.trim();
        if !value.is_empty() {
            needles.push(value);
        }
    }
    needles
}
```

Two needle forms today: the whole literal value, and — for a `Name: value`
header — the trimmed value-half via `split_once(':')` (first colon only, name
discarded). It borrows `&str` from `self.value`; adding derived encodings means
returning owned `String`s, so the signature changes to `Vec<String>`.

`scan()` — the match core, at `leaked_credentials.rs:40-49`:

```rust
#[must_use]
pub fn scan(body: &str, secrets: &[NamedSecret]) -> Vec<String> {
    secrets
        .iter()
        .filter(|secret| !secret.value.is_empty())
        .filter(|secret| {
            secret.needles().iter().any(|needle| body.contains(needle))
        })
        .map(|secret| secret.name.clone())
        .collect()
}
```

Literal `body.contains(needle)`, no case folding, no normalisation. The
empty-value guard (line 43) sits in `scan`, not `needles`, so any new needle form
still runs behind it. Returns names in input-slice order, no deduplication. The
module doc-comment (lines 6-9) explicitly disclaims encoding derivation — that
disclaimer is what 0207 removes.

**Where 0207's transforms land**: derived encodings (`base64(V)`,
`percent_encode(V)`) and the 12-char prefix forms extend `needles()`; the
haystack whitespace-strip belongs in `scan()`, matching each needle against both
`body` and `whitespace_strip(body)`. This preserves the existing separation and
keeps the empty-value guard authoritative.

### The credential-variable source (`cli/design-adapters/src/environment.rs`)

`CREDENTIAL_VARIABLES`, at `environment.rs:16-17`, is the fixed vocabulary:

```rust
pub const CREDENTIAL_VARIABLES: [&str; 4] =
    [AUTH_HEADER, USERNAME, PASSWORD, LOGIN_URL];
```

Four variables, `LOCATION` deliberately excluded (it is an origin selector, not a
secret — see `cli/design/src/credentials.rs:1-8`). The exact set and order are
pinned by a test, `the_variable_set_matches_the_scrubber_s_own_vocabulary`
(`environment.rs:53-64`). 0207 does not change this set.

`named_secrets_from_env()` (`environment.rs:37-47`) iterates
`CREDENTIAL_VARIABLES`, reads each via `non_empty()` (treats unset and blank
alike), and builds one `NamedSecret { name, value }` per non-empty variable. The
`AUTH_HEADER` value is the whole raw `Name: value` pair — **the colon split is
not here**. This is the `Vec<NamedSecret>` handed to `scan`.

### The command and report flow (`cli/design-cli/`)

Call chain: `main.rs:41-44` dispatches `Command::ScrubSecrets { file }` to
`commands::scrub_secrets(&file, &named_secrets_from_env())`. The handler, at
`commands.rs:83-97`:

```rust
let body = filesystem::read_document(file)?;
let leaked = leaked_credentials::scan(&body, secrets);
let Some(name) = leaked.first() else {
    return Ok(Report::silent());
};
Ok(Report::rejected(&format!(
    "the literal value of {name} appears in the generated inventory body. \
     The artifact was not written. Check your content for accidental \
     secret leakage."
)))
```

⚠️ The rejection message interpolates only `leaked.first()` — the first offending
variable, not all. 0207's report requirement ("names only the offending
variable") is satisfied by this shape unchanged, but note the message text says
"the literal value of {name}", which becomes inaccurate once an *encoded* or
*prefix* form is what matched. The wording likely needs a wording update to stay
truthful across encodings; it is not merely mechanical.

Exit codes, at `main.rs:93-104`: `Report::Accepted` → `ExitCode::SUCCESS` (0);
`Report::Rejected` → `eprint!(stderr)` + `ExitCode::FAILURE` (1);
`kernel::Error::Refusal` (e.g. missing file) → exit 2 via `report_error`. The
three-way contract is **2 = usage error, 1 = domain rejection, 0 = clean**.

**No artefact is ever written by this binary** — `scrub_secrets` reads and scans
only. 0207's "artefact not written" acceptance criteria are satisfied by exit 1
with no content emitted, which the current structure already guarantees.

### Available encoding utilities

| Capability | Crate + version | Status | Where declared |
| --- | --- | --- | --- |
| RFC 4648 base64 + padding | `base64` 0.22.1 | 🟡 in lock, undeclared | `cli/Cargo.lock:470` only |
| Percent-encode unreserved set | `percent-encoding` 2.3.2 | 🟡 in lock, undeclared | `cli/Cargo.lock:3482` only |
| Form encoding (wrong tool) | `form_urlencoded` 1 | 🔴 not applicable | `cli/visualiser/server/Cargo.toml:57` |

Both needed crates are already resolved transitively (via `reqwest`, `url`,
`octocrab`, etc.), so a direct declaration adds no package to the graph. Neither
is declared today, and no in-repo helper does encode-direction base64 or
percent-encoding (the only in-repo `base64` references are minisign-verify's
decode-only `PublicKey::from_base64`).

- **base64**: `base64` 0.22's `STANDARD` engine is RFC 4648 with padding — exactly
  what the work item specifies.
- **percent-encoding**: `percent_encode` with a custom `AsciiSet` starting from
  `NON_ALPHANUMERIC` then `.remove()`-ing `-` `.` `_` `~` matches the required
  unreserved set (`A-Z a-z 0-9 - . _ ~`) exactly.
- ⚠️ `form_urlencoded` (the lone directly-declared url-family dep) is the wrong
  tool: it does `application/x-www-form-urlencoded` (space → `+`), decode-oriented,
  and lives only in the visualiser server.

The workspace uses centralised `[workspace.dependencies]` with `{ workspace = true }`
references (`cli/Cargo.toml:52-195`). `cli/design/Cargo.toml:12-13` currently
declares only `kernel = { path = "../kernel" }`, so 0207 adds the first external
dependencies to that crate.

### Test surface and idiom

Three layers, all using behaviour-sentence snake_case names (never `test_*`):

| Layer | File | Drives | Asserts |
| --- | --- | --- | --- |
| Domain unit | `cli/design/src/leaked_credentials.rs:51-133` | `scan()` directly | exact `Vec` equality, `is_empty()` |
| Command | `cli/design-cli/src/commands.rs:254-298` | `scrub_secrets()` | `Report` variant, name-in/value-out |
| Binary integration | `cli/design-cli/tests/subcommands.rs:340-394` | real binary | exit code, stderr `contains` |

The domain layer is where TDD for 0207 starts. Helper `secret(name, value)` at
`leaked_credentials.rs:56-61` builds a fixture; assertions pair
`assert!(report.contains(name))` with `assert!(!report.contains(value))` to pin
the name-only invariant. The integration harness `run(args, env)` clears the
environment (`env_clear()`) so a developer's own `ACCELERATOR_BROWSER_*` cannot
leak in — new integration cases inject secrets via env constants at
`subcommands.rs:272-276`. Non-obvious cases carry a one-line `///` explaining
*why*, consistent with the repo's low-comment convention.

Existing names to mirror: `a_verbatim_value_names_its_variable`,
`the_value_half_of_a_header_pair_is_a_needle_of_its_own`,
`the_header_name_alone_does_not_false_positive`,
`the_report_never_carries_the_value`. 0207's acceptance criteria map to new
sibling names (base64 detection, percent-encoding detection, whitespace reflow,
12-char prefix boundary at P and P−1, too-short-no-prefix, mid/tail-truncation
out of scope, false-positive candidate).

## Code References

- `cli/design/src/leaked_credentials.rs:13-16` — `NamedSecret` struct (both fields `pub`, no constructor)
- `cli/design/src/leaked_credentials.rs:25-34` — `needles()`, the private encoding seam; `split_once(':')` value-half split
- `cli/design/src/leaked_credentials.rs:40-49` — `scan()`, literal `str::contains` match core; empty-value guard on line 43
- `cli/design/src/leaked_credentials.rs:6-9` — module doc-comment disclaiming encoding derivation (removed by 0207)
- `cli/design/src/leaked_credentials.rs:51-133` — domain unit tests and `secret()` helper
- `cli/design-adapters/src/environment.rs:16-17` — `CREDENTIAL_VARIABLES` (4 vars, no `LOCATION`)
- `cli/design-adapters/src/environment.rs:37-47` — `named_secrets_from_env()`; raw `AUTH_HEADER` value, no split here
- `cli/design-cli/src/commands.rs:83-97` — `scrub_secrets()` handler; surfaces `leaked.first()` only; "literal value of" wording
- `cli/design-cli/src/main.rs:93-104` — exit-code mapping (0 / 1 / 2)
- `cli/design-cli/src/report.rs:10-44` — `Report` enum and `Report::rejected`
- `cli/design-cli/tests/subcommands.rs:340-394` — binary integration tests; `run`/`write` harness
- `cli/design/Cargo.toml:12-13` — `cli/design` deps (only `kernel` today)
- `cli/Cargo.toml:52-195` — `[workspace.dependencies]` (no base64 / percent-encoding)
- `cli/Cargo.lock:470`, `cli/Cargo.lock:3482` — base64 0.22.1 and percent-encoding 2.3.2 resolved transitively

## Architecture Insights

- **Needle-vs-haystack separation is deliberate and load-bearing.** `needles()`
  derives what to search for; `scan()` decides where and how to search. 0207's own
  drafting notes reach the same split independently — encodings are needles,
  whitespace-strip is a haystack transform, because a configured value has no
  interior whitespace so `whitespace_strip(V)` would equal `V` and catch nothing.
- **The scanner is pure and infallible.** No `Result`, no I/O, no error type. This
  keeps the domain-unit test layer fast and total, and is why TDD starts there.
- **Name-only reporting is enforced at the source.** `scan()` maps to
  `secret.name.clone()`; values never leave the function. Every downstream layer
  inherits the invariant, and three tests pin it. 0207 must extend this guarantee
  to every encoding and prefix form without widening what the report carries. 🔒
- **The `AUTH_HEADER` value-half split is a fixed contract** consumed by 0207 (per
  the work item's Dependencies section). It lives in `needles()`; the derived
  encodings take the trimmed value-half as their input, so any change to the split
  shape couples to this scanner.
- **Exit codes carry the whole accept/reject signal.** Because the binary never
  writes, the caller keys artefact suppression on exit 1. 0207 changes what
  triggers exit 1, not the exit-code contract.

## Historical Context

- `meta/plans/2026-08-11-0196-design-cli-migration.md` — the CLI migration that
  built `leaked_credentials.rs` and introduced the `AUTH_HEADER` value-half split.
  Scanner design at lines 367-368; value-half acceptance criterion at lines
  744-748; "literal-substring only" at lines 2087-2088. 0207 is `derived_from`
  this plan.
- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md` —
  0196 implementation-surface research covering the scrub-secrets script and
  scanner being migrated.
- `meta/research/codebase/2026-09-10-0209-wire-up-browser-auth-header-path.md` —
  browser auth-header path research; 0209 also modifies `leaked_credentials.rs`
  (file coupling with 0207).
- No ADR covers secret scanning specifically. Nearest architectural context:
  `ADR-0053` (thin CLI over hexagonal core), `ADR-0054` (modular CLI of static
  binaries), `ADR-0057` / `ADR-0062` (browser-automation boundary, source of the
  `ACCELERATOR_BROWSER_*` credentials).

## Related Research

- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
- `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- `meta/research/codebase/2026-09-10-0209-wire-up-browser-auth-header-path.md`
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`

Sibling work items: `0196` (parent epic), `0209` (wire-up header auth, shares
`leaked_credentials.rs`), `0243` (browser auth-header in design skills), `0206`
(navigation-URL classification, same 0196 lineage).

## Resolved Questions

All three resolved on 2026-09-10.

- ✅ **Rejection message wording — reword form-agnostically.** Drop "literal" from
  `commands.rs:92-96`; the message states the value of `{name}` appears in the
  body in some form, without naming which. Truthful across literal, encoded,
  prefix, and reflow matches; stays name-only; adds no new reporting dimension.
  An encoding-aware message was rejected as scope the work item never asked for.
  The message text is not asserted by the existing tests (`commands.rs:268-284`,
  `subcommands.rs:349-364` check `contains(name)` / `!contains(value)` only), so
  the reword needs no test change beyond those invariants.
- ✅ **`needles()` return type — uniform `Vec<String>`.** Every needle owned,
  raw value included. The method is private, so no external API break; the borrow
  optimisation buys nothing measurable (`needles()` runs once per secret per
  scan, behind `read_document`); and it keeps all derivation in one place. Matches
  the work item's owned-value pseudocode. ⚠️ Knock-on: `scan()` line 45 becomes
  `body.contains(needle)` on a `&String` (derefs to `&str`) — a one-line change.
- ✅ **cargo-deny admits both crates — verified, `mise run deny:check` exits 0.**
  cargo-deny evaluates the resolved graph, and `base64` 0.22.1 and
  `percent-encoding` 2.3.2 are already in it transitively, so a green run is
  proof, not merely a present-state pass. Gate-by-gate against `cli/deny.toml`:
  licences (both dual MIT / Apache-2.0, allowed at `deny.toml:52-64`), bans
  (neither on the deny-list, `deny.toml:125-132`), sources (both crates.io,
  `deny.toml:137`), advisories (ignore-list is hickory-proto only,
  `deny.toml:39-42`). ⚠️ Two declaration constraints: `wildcards = "deny"`
  (`deny.toml:111`) means pin a concrete version, never `"*"`; and
  `multiple-versions = "warn"` (`deny.toml:110`) means a duplicate version warns
  rather than fails.
