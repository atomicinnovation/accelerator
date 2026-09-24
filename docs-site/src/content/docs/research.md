---
title: Research CLI
---

`accelerator research` is the sub-binary `/accelerator:research-topic` uses
to reach the scholarly literature. `research fetch` queries OpenAlex and arXiv
and returns normalised records, each carrying a reputation tier the CLI
derives. `research guard` is a `PreToolUse` hook that confines the researcher
agent to that fetch and to writing its own finding. Both are plumbing: the
academic source profiles invoke `research fetch` through fenced command blocks
(see [Anatomy of a skill
invocation](internals.md#anatomy-of-a-skill-invocation)), and Claude Code runs
`research guard` on every tool call that could run a command or write a file.
Running `fetch` by hand is mainly useful for reproducing what a researcher saw.

| Verb    | What it does                                                           |
|---------|------------------------------------------------------------------------|
| `fetch` | Search or look up scholarly records, tiered, within a 100 s deadline   |
| `guard` | Judge one hook call, blocking a researcher outside its confinement     |

See [Internals](internals.md#terminal-invocation) for how to reach
`accelerator` at all from a terminal; everything below assumes that's set up.

## `fetch`

```bash
accelerator research fetch openalex search 'graph neural networks' --limit 3
accelerator research fetch openalex lookup W2741809807
accelerator research fetch openalex lookup 10.1038/nature14539
accelerator research fetch arxiv search 'attention heads specialise'
accelerator research fetch arxiv lookup 2608.21129
```

| Family     | `search`                                  | `lookup` accepts                                                   |
|------------|-------------------------------------------|--------------------------------------------------------------------|
| `openalex` | Title-and-abstract search over every work | `W123`, `https://openalex.org/W123`, or a DOI (bare, `doi:`, or `https://doi.org/`) |
| `arxiv`    | Relevance-sorted search over preprints    | A new-style (`2608.21129`) or old-style (`hep-th/9901001`) ID, with or without a version |

Several search terms are joined with single spaces. Characters a source
reserves for its own query syntax become spaces (`,`, `|`, `!`, and `:` for
OpenAlex; parentheses, quotes, and the `AND`, `OR`, and `ANDNOT` operators for
arXiv, which then requires every remaining word), and the query is
percent-encoded as one value, so it cannot smuggle an operator or a second
parameter. A query empty once they are removed is a usage error. `--limit`
takes 1–25 and defaults to 10.

**Every call finishes within 100 s of the process starting**, whatever it is
waiting on: the credential command, pacing, retries, and each request's own
30 s timeout all draw on that one budget. Give the call a Bash timeout of at
least 120 s.

### Output

On stdout, one line of compact JSON. A source that answered:

```json
{"status":"ok","records":[{"title":"…","authors":["…"],"url":"https://doi.org/10.1038/nature14539","venue":"Nature","venue_signals":{"source_type":"journal","version":"publishedVersion","is_core":true,"listed_in":["doaj"],"type":"article","is_retracted":false},"abstract":"…","tier":"tier-1","retracted":false,"withdrawn":false}]}
```

- `url` is `https://doi.org/<doi>` when the work has a valid DOI, else its
  OpenAlex ID; an arXiv record's is `https://arxiv.org/abs/<id>`, version
  stripped.
- `abstract` is capped at 600 characters on a word boundary.
- `venue_signals` holds the raw evidence behind the tier: the six keys above
  for OpenAlex, `journal_ref` and `doi` for arXiv.
- A `lookup` that finds nothing is `ok` with no records.

A source that cannot answer now:

```json
{"status":"unavailable","source":"openalex","reason":"budget_exhausted","authenticated":false}
```

| `reason`           | Means                                                                  |
|--------------------|------------------------------------------------------------------------|
| `budget_exhausted` | OpenAlex's daily budget is spent; `authenticated` says whether a key was sent |
| `rate_limited`     | Throttled past every retry; `"cause":"lock_contention"` when arXiv's pacing lock stayed held elsewhere |
| `upstream_error`   | Server errors, connection failures, or timeouts past every retry       |

`authenticated` appears only for OpenAlex, which is how `conduct` tells a
missing key from a spent one. A failed or unavailable call also writes one
line to stderr naming the source, verb, final status, and attempt count, and
never the key.

### Exit codes

| Exit | Meaning                                                          |
|------|------------------------------------------------------------------|
| 0    | `ok` or `unavailable` — the call worked; the source may not have |
| 1    | A credential refusal or a rejected request, with one `E_*` line  |
| 2    | Usage error, before any request is sent                          |

| Code                              | Exit | Cause                                                        |
|-----------------------------------|------|--------------------------------------------------------------|
| `E_RESEARCH_USAGE`                | 2    | Unknown family or verb, `--limit` outside 1–25, a missing or empty query |
| `E_OPENALEX_ID_MALFORMED`         | 2    | A `lookup` ID that is neither a `W…` ID nor a DOI            |
| `E_ARXIV_ID_MALFORMED`            | 2    | A `lookup` ID that is not an arXiv ID                        |
| `E_OPENALEX_KEY_REJECTED`         | 1    | OpenAlex refused the key; names the rung that supplied it    |
| `E_OPENALEX_UNAUTHENTICATED`      | 1    | OpenAlex refused a keyless request                           |
| `E_RESEARCH_CLIENT_ERROR`         | 1    | Any other `4xx`, an unfollowed redirect, or an arXiv error feed |
| `E_RESEARCH_UNDECODABLE`          | 1    | A response the CLI could not parse                           |
| `E_RESEARCH_TRANSPORT`            | 1    | The HTTP client could not start                              |
| `E_TOKEN_*`, `E_LOCAL_PERMS_INSECURE` | 1 | The key could not be resolved safely; see [Credentials](#credentials) |

### Retries and deadlines

A retryable response waits 3 s, 6 s, then 12 s between attempts, or the
server's `Retry-After` capped at 30 s. The call stops early with
`unavailable` when the deadline would not admit the next wait. OpenAlex budget
exhaustion (`409`, or a `429` whose remaining budget is below the cost of the
request) is not retried.

Redirects are followed at most three times, and only within the original
`https` origin; any other redirect is a client error, so the key never
reaches a second host. Bodies over 8 MiB are refused.

## Reputation tiers

The CLI assigns every record's tier, so a model never judges a venue's
standing. The first matching rule wins; "source" and "version" mean the
OpenAlex work's primary location's.

| Order | Tier     | When                                                                         |
|-------|----------|------------------------------------------------------------------------------|
| 1     | `tier-3` | An OpenAlex work with `is_retracted`, or a withdrawn arXiv entry             |
| 2     | `tier-1` | A non-`preprint` OpenAlex work whose source is a `journal` or `conference` with a published or accepted version, or whose source is core or `medline`-listed |
| 3     | `tier-2` | An OpenAlex `preprint`, a work in a `repository`, a `journal` or `conference` work with a submitted version, and every arXiv entry |
| 4     | `tier-3` | Anything else, such as a work with no primary source                         |

An arXiv entry stays `tier-2` even when its `journal_ref` or `doi` names a
journal: both are author-supplied and unvalidated, so they are reported in
`venue_signals` and never raise the tier. A finding cites a retracted source
as `tier-3 (retracted)` and a withdrawn one as `tier-3 (withdrawn)`.

### Withdrawal detection

An arXiv entry is withdrawn only when both hold:

1. its comment starts, case-insensitively, with "withdrawn" or "this paper
   (article, submission, manuscript) has been (is) withdrawn";
2. its OAI-PMH `arXivRaw` record shows the latest version at `0kb` with
   source type `I`.

The comment alone produces false positives, so the second request confirms
every candidate. Verdicts are cached in
`<paths.tmp>/research/arxiv-withdrawals.json`, keyed by the ID and its latest
version, so a new version is re-checked. If a confirmation cannot be made, the
whole call is `unavailable`: an unconfirmed withdrawal is never cited as
`tier-2`.

## Pacing

arXiv admits one request every three seconds. Every arXiv request in a
project, across processes, passes through an exclusive file lock at
`<paths.tmp>/research/arxiv.lock` (`paths.tmp` defaults to
`.accelerator/tmp`), which spaces requests at least 3 s from the end of the
previous response and holds one connection at a time. A `429` or `403` defers
every waiting process together, by up to 30 s. A call that cannot take the
lock before its deadline ends `rate_limited` with `cause: lock_contention`,
which in practice means too many concurrent arXiv researchers in one round.

`arxiv-requests.log` and `arxiv-contention.log` beside the lock record one
timestamped line per request sent and per contention. The lock does not
coordinate across repositories on one machine. OpenAlex is not paced; its
budget is enforced server-side.

## Credentials

An OpenAlex API key is optional. Without one, requests run on OpenAlex's
keyless daily allowance, which a research round can spend quickly; with one,
it is sent as `Authorization: Bearer`, never in a URL, and never appears in
output or errors. The key resolves through the same ladder as the tracker
tokens, first non-empty wins:

1. `ACCELERATOR_OPENALEX_API_KEY`
2. `ACCELERATOR_OPENALEX_API_KEY_CMD`, run with its stdout trimmed
3. `openalex.api_key` in `.accelerator/config.local.md`
4. `openalex.api_key_cmd` in `.accelerator/config.local.md`
5. `openalex.api_key` in `.accelerator/config.md`, only when
   `config.local.md` does not exist

The key command runs under whatever remains of the 100 s deadline. Refusals,
each exiting `1` before any request:

| Code                             | Refuses                                                        |
|----------------------------------|----------------------------------------------------------------|
| `E_TOKEN_CMD_FROM_SHARED_CONFIG` | `openalex.api_key_cmd` in the shared `config.md`               |
| `E_TOKEN_FROM_TRACKED_FILE`      | `openalex.api_key` in a `config.local.md` tracked by version control |
| `E_TOKEN_CMD_FROM_TRACKED_FILE`  | `openalex.api_key_cmd` in a tracked `config.local.md`          |
| `E_LOCAL_PERMS_INSECURE`         | A `config.local.md` looser than `0600`                         |
| `E_TOKEN_CMD_FAILED`             | A key command that failed or outlasted the deadline            |
| `E_TOKEN_MALFORMED`              | A key carrying a control character                             |

A tracked `config.local.md` that supplies neither key leaves the call
keyless. `accelerator config dump` hides both keys. See
[`/accelerator:configure`](reference/skills/config/configure.md) for the
settings reference.

## `guard`

```bash
accelerator research guard --fail-safe --non-blocking < hook-input.json
```

`hooks/hooks.json` registers the guard under `PreToolUse` for `Bash` and for
`Write|Edit|MultiEdit|NotebookEdit`, so it runs on every such call in every
session. It judges only a subagent whose type is `accelerator:researcher` or
the name configured as `agents.researcher`; every other call passes untouched,
before any config is read. For the researcher it blocks, by exiting `2`:

- a command other than `accelerator research fetch …`, or one carrying shell
  syntax that could run anything else;
- a write anywhere but `<research_topics>/<set>/findings/<name>.md`, including
  through a `..` component or a symlink;
- a call whose command or path cannot be read.

:::caution
The guard keys on the agent type, not on who spawned it. Every subagent of the
configured researcher type, wherever it is spawned, may only run
`accelerator research fetch` and write findings. Point `agents.researcher` at
a dedicated agent, never at one you use for other work.
:::

### Command matching

The guard matches the command the way Claude Code itself matches a
`Bash(accelerator research fetch *)` allow rule, so it refuses nothing Claude
Code would accept except two constructs:

| Construct                                        | Why the guard blocks it                                                                   |
|--------------------------------------------------|-------------------------------------------------------------------------------------------|
| Any unescaped `$` outside single quotes and comments | Claude Code accepts `$HOME`, which lets a researcher read an environment secret back through a usage error's echo |
| Any unquoted `<` outside comments                | Claude Code accepts `< file`, which includes bash's `</dev/tcp/…` and so sends data to an arbitrary host |

The fetch needs neither: its arguments are literal and it reads no stdin.
Everything else follows Claude Code's measured behaviour, including globs, `~`,
comments, a quoted separator, and discarding output to `/dev/null`. The
separators, pipes, background operators, substitutions, braces, other
redirections, and newlines Claude Code denies, the guard denies too. The
researcher's unconfined `Read` remains an accepted residual path for
disclosure.

The measured behaviour lives in `cli/research/tests/fixtures/claude-bash-baseline.tsv`,
one row per construct and one column per Claude Code release, measured on
2.1.281 and 2.1.144. The guard follows the latest column, and its tests fail
if it disagrees with it anywhere but those two constructs. On each new Claude
Code release, re-probe and add its column:

```bash
uv run python -m tasks.probe.claude_permissions \
    --claude "$(command -v claude)" --config-dir ~/.claude-probe --update
```

It spends one headless session per row, and runs outside every `mise` task. A
moved verdict moves the guard with it, except that a re-probe never loosens
the two constructs above.

### Block messages

| Code                          | Cause                                                              |
|-------------------------------|--------------------------------------------------------------------|
| `E_RESEARCH_GUARD_COMMAND`    | A command other than `accelerator research fetch …`               |
| `E_RESEARCH_GUARD_SYNTAX`     | A construct that could run anything else, named in the message     |
| `E_RESEARCH_GUARD_WRITE`      | A write outside the findings directory, with its cause             |
| `E_RESEARCH_GUARD_UNREADABLE` | A researcher call with no readable command or path                 |
| `E_RESEARCH_GUARD_INTERNAL`   | The guard itself failed while judging a researcher call            |

Each message names the matched agent type, adding `(agents.researcher)` when
it matched the configured name. A researcher reads a block as a call to
correct, not the end of its work.

### Granting the fetch

The guard never allows anything; it only blocks. The fetch itself must be
granted by a permission rule that reaches the researcher subagent.
`research-topic`'s `allowed-tools` carries `Bash(accelerator research fetch *)`,
which researchers it spawns inherit. Any other skill that injects an academic
profile must grant the same rule, and a contract test enforces this for skills
in this repository.

Where that grant does not reach the researcher, add the rule to the project's
allow rules:

```json
{
  "permissions": {
    "allow": ["Bash(accelerator research fetch *)"]
  }
}
```

A `deny` or `ask` rule covering `accelerator research fetch` overrides the
grant. `conduct` reports either case as "fetch denied by permissions".

### Failure signatures

The guard dispatches with `--fail-safe --non-blocking`, so a failure to run
the guard binary never blocks a call. A guard block is exit `2` with an
`E_RESEARCH_GUARD_*` line; anything else is the launcher failing to run the
guard.

| Symptom                                         | Cause                                       | Recovery                                                  |
|-------------------------------------------------|---------------------------------------------|-----------------------------------------------------------|
| Nothing; the researcher runs unconfined         | The `research` binary could not be fetched   | Restore network access to the release host                |
| A non-blocking hook error (exit `1`) on every `Bash` and write call | The cached binary failed its signature check | Delete the named cached binary and its `.minisig`, or set `ACCELERATOR_RESEARCH_BIN` |

In both cases Claude Code's own permission rules still apply, and the
researcher can no longer write outside `findings/` to induce either failure. A
guard panic on a crafted command blocks instead, with
`E_RESEARCH_GUARD_INTERNAL`.

## Academic profiles in `research-topic`

[`research-topic`](reference/skills/research/research-topic.md) reaches each
source through a **source profile**, a skill
the researcher agent follows: `web-profile` (web search and fetch),
`openalex-profile`, and `arxiv-profile` (each through `research fetch`).

- **`brief`** offers all three and records the chosen subset as the brief's
  `source_profiles`, defaulting to `["web"]`. It suggests the academic
  profiles for a scholarly subject and recommends an OpenAlex key when
  `openalex` is chosen.
- **`outline`** assigns each focus area one or more profiles drawn from the
  brief, as a suffix:

  ```markdown
  - [ ] How do attention heads specialise? — profiles: web, openalex
  - [ ] What limits long-context attention? — profiles: arxiv
  ```

  An item with no suffix is researched through `web` alone. `breadth` caps
  focus areas, not profiles.
- **`conduct`** spawns one researcher per outstanding (focus area, profile)
  pair, and ticks an item only once every one of its eligible pairs has a
  valid finding. A pair whose profile is not in the brief, or not installed,
  is skipped and reported.

Each pair's finding lives at `findings/<nn>-<question-slug>-<profile>.md`,
where every profile of one focus area shares its `<nn>`. The finding's
`question` and `source_profile` frontmatter, not its filename, identify the
pair: sets written before profiles existed hold one `findings/<nn>-<slug>.md`
per focus area, and still count as their `web` pair. Consumers group findings
by frontmatter, never by parsing the filename.

### `corpus topic-research outstanding`

`conduct` asks the corpus CLI which pairs are outstanding and where their
findings go, rather than allocating paths itself:

```bash
accelerator corpus topic-research outstanding SLUG --profiles-dir DIR
```

It prints JSON with four arrays: `items` (each outline item's `line`,
`question`, and `complete`), `pairs` (each outstanding pair's `question`,
`profile`, and absolute `path`), `skipped` (with a `reason`), and `warnings`.
The JSON is additive-only: fields may be added, never renamed or removed, and
consumers ignore fields they do not know. It is read-only, and exits `1` with
`E_TOPIC_RESEARCH_UNRESOLVED` for an unknown set.

## Local development

| Mechanism                  | Purpose                                                                                                                                             |
|----------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| `ACCELERATOR_RESEARCH_BIN` | One-shot override pointing `accelerator research …` at a locally-built `accelerator-research` binary, bypassing the normal fetch-and-cache dispatch |

This mirrors `ACCELERATOR_CORPUS_BIN` and `ACCELERATOR_DESIGN_BIN`. Because
the guard runs on every `Bash` and write call, an override naming a missing
binary leaves every researcher unconfined, with a stderr diagnostic on each
call, until it is unset.
