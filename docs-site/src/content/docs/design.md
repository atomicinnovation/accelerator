---
title: Design CLI
---

`accelerator design` is the sub-binary the two design skills
(`inventory-design`, `analyse-design-gaps`) use to validate what they are
about to crawl, resolve how to authenticate to it, check what they are about
to write, and explain a fallback to the code-only crawler. It is plumbing
rather than a feature you reach for directly: skills invoke it through the
`!`-preprocessor and fenced command blocks (see
[Anatomy of a skill invocation](internals.md#anatomy-of-a-skill-invocation)).
Running it by hand is mainly useful for reproducing what a skill did.

| Subcommand           | What it does                                                             |
|----------------------|--------------------------------------------------------------------------|
| `validate-source`    | Whether a URL or repository path may be inventoried                      |
| `resolve-auth`       | Which authentication mode the `ACCELERATOR_BROWSER_*` environment selects |
| `scrub-secrets`      | Whether a produced artefact repeats a configured credential              |
| `notify-downgrade`   | The notice explaining a fallback to the code-only crawler                |
| `audit-cue-phrases`  | Whether every substantive H2 section of a gap document carries a cue phrase |

See [Internals](internals.md#terminal-invocation) for how to reach
`accelerator` at all from a terminal; everything below assumes that's set up.

## Exit codes

Every subcommand uses the same three, which is what lets a caller tell "the
tool worked and refused your input" from "the tool broke" from "you invoked it
wrongly":

| Exit | Meaning                                                          |
|------|-------------------------------------------------------------------|
| 0    | Accepted                                                          |
| 1    | Evaluated and rejected — a verdict about the input                |
| 2    | Usage error — a malformed invocation the tool could not interpret |

`scrub-secrets` and `audit-cue-phrases` previously conflated the last two on
exit 1. A file path naming nothing readable is now exit 2, because the
argument cannot be interpreted as a file to scan at all.

## `validate-source`

```bash
accelerator design validate-source https://prototype.example.com
accelerator design validate-source http://10.0.0.1 --allow-internal
accelerator design validate-source http://example.com --allow-insecure-scheme
accelerator design validate-source ./src/pages
```

Accepts `https://` to a public host, any loopback destination on either
scheme, `about:blank`, and a path that exists as a directory. Refuses
`file://`, `javascript:`, `data:`, `chrome://`, other `about:` URLs, any
userinfo segment, and any path containing `..`.

Two flags recover otherwise-refused locations:

| Flag                       | Recovers                                              |
|----------------------------|-------------------------------------------------------|
| `--allow-internal`         | Private, link-local and reserved addresses            |
| `--allow-insecure-scheme`  | `http://` to a public host                            |

### What changed from the shell implementation

The reachability check now parses the host as an address rather than matching
it with regular expressions, which **narrows the accepted set** in two
different ways.

Some addresses are now classified as internal and so are refused *unless* you
pass `--allow-internal`. These were accepted unconditionally before:

- IPv4-mapped and IPv4-compatible IPv6 forms (`::ffff:10.0.0.1`,
  `::169.254.169.254`)
- IPv6 unique-local (`fc00::/7`)
- Carrier-grade NAT (`100.64.0.0/10`), and the other reserved ranges
  (`192.0.0.0/24`, `198.18.0.0/15`, `240.0.0.0/4`, multicast)
- The transition encodings that embed an IPv4 address — 6to4 (`2002::/16`),
  Teredo (`2001::/32`) and NAT64 (`64:ff9b::/96`) — classified on the address
  they carry

Other hosts are refused **unconditionally, with no flag that recovers them**,
because they are alternate numeric encodings rather than addresses the tool
can reason about — decimal (`2130706433`), hexadecimal (`0x7f000001`) and
zero-padded octal (`0177.0.0.1`, `127.0.0.01`) forms, including a non-first
octal octet. Rewrite the location in dotted-quad form.

Two things were **widened**, and need no flag where they previously did:

- The whole of `127.0.0.0/8` and `::1`, not only the literal strings
  `localhost` and `127.0.0.1`. A loopback destination is the local machine
  talking to itself.
- `::ffff:127.0.0.1`, which unwraps to a loopback address.

One was narrowed with no recovery: the unspecified address (`0.0.0.0`, `::`)
names no host, so there is nothing for `--allow-internal` to recover into.

### What this check does not cover

It is the front door, not a boundary. It is **pre-resolution** — a public
hostname resolving to `169.254.169.254` still passes, and nothing re-checks
after DNS. It covers **only the initial location**: the executor's `navigate`
command follows whatever URL each request supplies, with no classification at
all. And it does not confirm a path location lies inside a repository.

## `resolve-auth`

```bash
accelerator design resolve-auth
```

Prints `header`, `form` or `none`. `ACCELERATOR_BROWSER_AUTH_HEADER` wins over
the form-login trio, warning on stderr about the variables it is ignoring. All
three of `ACCELERATOR_BROWSER_USERNAME`, `ACCELERATOR_BROWSER_PASSWORD` and
`ACCELERATOR_BROWSER_LOGIN_URL` give `form`; none of them gives `none`. Some
but not all is exit 2, naming the missing variables.

:::caution
The `header` mode is currently **inert downstream**. The executor daemon
imports its auth-header handler and never calls it, and the origin allowlist
that handler requires is set nowhere — so an authenticated crawl silently
produces an unauthenticated inventory. Do not put a live credential in
`ACCELERATOR_BROWSER_AUTH_HEADER` until that is wired up.
:::

## `scrub-secrets`

```bash
accelerator design scrub-secrets path/to/inventory.md
```

Refuses to let an artefact through when it repeats the literal value of a set
`ACCELERATOR_BROWSER_*` variable. The report names the *variable* and never
its value, so it is safe to print, log and commit.

`ACCELERATOR_BROWSER_AUTH_HEADER` holds a whole `Name: value` pair, so the
value half is checked separately — an artefact rendering only the bearer token
is caught, which the shell implementation missed.

Matching is literal-substring only. A credential that appears base64-encoded,
percent-encoded or truncated is not detected.

## `notify-downgrade`

```bash
accelerator design notify-downgrade --reason node-missing
accelerator design notify-downgrade --from hybrid --to code --reason bootstrap-failed
```

Prints the notice for one of six reasons: `node-missing`, `node-too-old`,
`bootstrap-failed`, `executor-ping-failed`, `cache-unwritable`,
`disk-floor-not-met`. `--from` and `--to` are accepted for forward
compatibility and do not affect the message. An unknown reason is exit 2 and
lists the whole vocabulary.

## `audit-cue-phrases`

```bash
accelerator design audit-cue-phrases path/to/gaps.md
```

Checks every H2 section carrying prose for at least one canonical cue phrase —
*we need*, *users need*, *the system must*, or *implement* followed by a
capitalised word. A section holding only whitespace is skipped. Exit 1 names
every offending section; exit 2 means the file could not be read at all, so
there is nothing to revise.

The patterns come from `scripts/extract-work-items-cue-phrases.txt`, the same
file `extract-work-items` reads, and a test pins the two against each other.

## Environment

| Variable                | Effect                                                                                                                        |
|-------------------------|--------------------------------------------------------------------------------------------------------------------------------|
| `ACCELERATOR_DESIGN_BIN` | One-shot override pointing `accelerator design …` at a locally-built `accelerator-design` binary, bypassing the normal fetch-and-cache dispatch |

This mirrors `ACCELERATOR_CORPUS_BIN` and `ACCELERATOR_VCS_BIN` for the
sub-binaries they name.
