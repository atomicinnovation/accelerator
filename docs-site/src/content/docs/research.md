---
title: Research CLI
---

`accelerator research` is the sub-binary `/accelerator:research-topic` uses
to reach the scholarly literature. `research fetch` queries OpenAlex and arXiv
and returns normalised records, each carrying a reputation tier the CLI
derives. `research guard` is a `PreToolUse` hook that confines the researcher
agent to that fetch and to writing its own finding. `research topic
outstanding` plans a `conduct` round from a topic's set on disk. All three are
plumbing: the academic source profiles invoke `research fetch` through fenced
command blocks (see [Anatomy of a skill
invocation](internals.md#anatomy-of-a-skill-invocation)), `conduct` calls
`research topic outstanding`, and Claude Code runs `research guard` on every
tool call that could run a command or write a file.
Running `fetch` by hand is mainly useful for reproducing what a researcher saw.

| Verb                | What it does                                                         |
|---------------------|----------------------------------------------------------------------|
| `fetch`             | Search or look up scholarly records, tiered, within a 100 s deadline |
| `topic outstanding` | List a round's outstanding (focus area, profile) pairs as JSON       |
| `guard`             | Judge one hook call, blocking a researcher outside its confinement   |

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
takes 1–25 and defaults to 10. `--ticket` re-presents the ticket a `waiting`
arXiv call printed (see [Output](#output)); an OpenAlex call refuses it.

**Every call finishes within 100 s of the process starting**, whatever it is
waiting on: the credential command, pacing, retries, and each request's own
30 s timeout all draw on that one budget. Give the call a Bash timeout of at
least 120 s. An arXiv call that prints `waiting` keeps its place across
re-presentations instead, so its result can take up to about 1,000 s from the
first call to settle.

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
| `rate_limited`     | Throttled past every retry; `"cause":"lock_contention"` when a queued arXiv fetch passed its 900 s cap, or a fresh one ran out of budget while the queue was unusable (a re-presented one keeps its place and waits). Either way, a call that ran out mid-retry reports that retry's reason instead |
| `upstream_error`   | Server errors, connection failures, or timeouts past every retry       |

`authenticated` appears only for OpenAlex, which is how `conduct` tells a
missing key from a spent one. A failed or unavailable call also writes one
line to stderr naming the source, verb, final status, and attempt count; a
waiting call writes one naming its position and the `--ticket` to re-present.
Neither line ever names the key.

An arXiv call that could not be served in time keeps its place in the queue
(see [Pacing](#pacing)):

```json
{"status":"waiting","source":"arxiv","ticket":"42-9f1c2a","position":3}
```

Re-present the same call with `--ticket 42-9f1c2a`, and repeat until the
status is no longer `waiting`.

A caller should treat any status other than `ok`, `unavailable` or `waiting`
as `unavailable`, reporting the status verbatim as its reason.

### Exit codes

| Exit | Meaning                                                          |
|------|------------------------------------------------------------------|
| 0    | `ok`, `unavailable` or `waiting` — the call worked; the source may not have |
| 1    | A credential refusal or a rejected request, with one `E_*` line  |
| 2    | Usage error or refused ticket, before any request is sent; only `E_ARXIV_TICKET_LIVE` can clear without changing the call |

| Code                              | Exit | Cause                                                        |
|-----------------------------------|------|--------------------------------------------------------------|
| `E_RESEARCH_USAGE`                | 2    | Unknown family or verb, `--limit` outside 1–25, a missing or empty query, or `--ticket` on an OpenAlex call |
| `E_OPENALEX_ID_MALFORMED`         | 2    | A `lookup` ID that is neither a `W…` ID nor a DOI            |
| `E_ARXIV_ID_MALFORMED`            | 2    | A `lookup` ID that is not an arXiv ID                        |
| `E_ARXIV_TICKET_MALFORMED`        | 2    | A `--ticket` that is not a ticket such as `42-9f1c2a`        |
| `E_ARXIV_TICKET_MISMATCH`         | 2    | A ticket presented with other arguments than it was issued for; names the call it was issued for and which arguments differ |
| `E_ARXIV_TICKET_LIVE`             | 2    | A ticket another call is presenting right now; clears once that call returns |
| `E_OPENALEX_KEY_REJECTED`         | 1    | OpenAlex refused the key; names the rung that supplied it    |
| `E_OPENALEX_UNAUTHENTICATED`      | 1    | OpenAlex refused a keyless request                           |
| `E_RESEARCH_CLIENT_ERROR`         | 1    | Any other `4xx`, an unfollowed redirect, or an arXiv error feed |
| `E_RESEARCH_UNDECODABLE`          | 1    | A response the CLI could not parse                           |
| `E_RESEARCH_TRANSPORT`            | 1    | The HTTP client could not start                              |
| `E_TOKEN_*`, `E_COMMAND_*`, `E_CONSENT_KEY_*`, `E_LOCAL_PERMS_INSECURE` | 1 | The key could not be resolved safely; see [Credentials](#credentials) |

### Retries and deadlines

A retryable response waits 3 s, 6 s, then 12 s between attempts, or the
server's `Retry-After` capped at 30 s. When the deadline would not admit the
next wait, an arXiv call stops early with `waiting` and an OpenAlex call with
`unavailable`. OpenAlex budget
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
`.accelerator/tmp`), which holds one connection at a time and spaces each
request at least 3 s from the end of the previous request's response. When a
call was killed before its response ended, the next request is spaced 3.1 s
from the killed request's send instead, since its arrival was never seen. A
`429` or `403` defers every waiting process together, by up to 30 s. The
lock does not coordinate across repositories on one machine. OpenAlex is not paced; its budget is enforced server-side.

### The fetch queue

arXiv calls take turns in a first-in, first-out queue of tickets under
`<paths.tmp>/research/arxiv-queue/`:

- **Whole-call admission.** The front ticket is admitted once the lock is
  free and at least 33 s of its 100 s budget remain: one 3 s spacing and one
  30 s request. It then holds the lock for every request, retry and backoff
  it makes, including its withdrawal confirmations, and checks the same
  serving window before each attempt.
- **`waiting`.** A call that runs out of budget, before or after admission,
  prints its ticket and position and keeps its place. Re-presenting the same
  call with `--ticket` resumes it; the researcher does this until the call
  settles. A ticket presented with other arguments exits 2 with
  `E_ARXIV_TICKET_MISMATCH`, and one another call is presenting exits 2 with
  `E_ARXIV_TICKET_LIVE`.
- **Expiry and the cap.** A ticket keeps its place for 300 s after its last
  call ended; after that it rejoins at the back with a fresh ticket. More than
  900 s after issue, a re-presentation ends `rate_limited` with
  `cause: lock_contention`, unless its last call ran out mid-retry, which
  reports that retry's reason instead. A ticket issued more than 1000 s ago
  is treated as abandoned, whatever its lock says.
- **Killed calls.** A killed call releases its ticket's lock, so the tickets
  behind it stop waiting on it at once. The ticket keeps its place for
  re-presentation until 300 s after the call could last have ended.

### Logs

`arxiv-requests.log` beside the lock records one timestamped line per request
sent. `arxiv-contention.log` records one line per call that ended in
`lock_contention` with no upstream failure behind it, in the form
`<wall ms> <kind>[ <ticket>]`. A line with only a timestamp comes from a
session running an earlier prerelease of the plugin.

| Kind              | Means                                      | Next step                                                                                                   |
|-------------------|--------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| `ticket_past_cap` | The ticket waited past its 900 s cap       | Avoid overlapping `conduct` runs in one project, or re-run once the other run has finished                  |
| `queue_unusable`  | The call ran out of budget with no place to keep | Check that `<paths.tmp>/research/arxiv-queue/` is a writable directory on a filesystem with working `flock`. While no `conduct` run is active, remove it; it is rebuilt on demand |

### Limits

- When `arxiv.lock` cannot be locked, on a filesystem without working `flock`
  or after a per-call lock error, pacing runs unlocked. Neither one
  connection at a time nor 3 s spacing is then guaranteed, and only the
  stderr diagnostic records it.
- When the queue itself cannot be used, a call is served without a place, and
  ends in `lock_contention` rather than `waiting` if its budget runs out. A
  re-presented call keeps the place its ticket already holds and still ends
  in `waiting`.
- The queue is fair only while every session in the project runs a plugin
  version with this queue. An earlier one still paces through `arxiv.lock`
  but ignores the queue, so calls queued behind it see more
  `lock_contention`. A version whose queue records it cannot read removes
  them once their tickets are free.
- A ticket that has just found itself at the front takes `arxiv.lock` a
  moment later. An earlier ticket re-presented in that moment can be passed
  for one turn; spacing is unaffected.
- Wall-clock steps, and a host that sleeps mid-run, can delay or hasten
  expiry, abandonment and the cap. That costs fairness or an occasional node,
  never spacing, which the lock enforces on every request.

## Credentials

An OpenAlex API key is optional. Without one, requests run on OpenAlex's
keyless daily allowance, which a research round can spend quickly; with one,
it is sent as `Authorization: Bearer`, never in a URL, and never appears in
output or errors. The key resolves through the same ladder as the tracker
tokens. The first rung that yields a usable value wins; a failed command or
refused value is reported as a `warning:` and the chain continues:

1. `ACCELERATOR_OPENALEX_API_KEY`
2. `ACCELERATOR_OPENALEX_API_KEY_CMD`, run in a fresh temporary directory
   outside the repository, with a scrubbed environment and a filtered `PATH`,
   its output capped at 65,536 bytes and its stdout trimmed — see the
   command runner in [`/accelerator:configure`](reference/skills/config/configure.md)
3. `openalex.api_key` in `.accelerator/config.local.md`
4. `openalex.api_key_cmd` in `.accelerator/config.local.md`
5. `openalex.api_key` in `.accelerator/config.md`, only when
   `config.local.md` does not exist

The key command runs under whatever remains of the 100 s deadline. Each
refusal is a `warning:` when a later rung supplies a key. When none does, the
first refusal exits `1` before any request, and the rest are printed as
warnings:

| Code                             | Refuses                                                        |
|----------------------------------|----------------------------------------------------------------|
| `E_CONSENT_KEY_TEAM_LEVEL`       | `openalex.api_key_cmd` in the shared `config.md`               |
| `E_TOKEN_FROM_TRACKED_FILE`      | `openalex.api_key` in a `config.local.md` tracked by version control, or whose tracking status cannot be determined |
| `E_CONSENT_KEY_TRACKED`          | `openalex.api_key_cmd` in a tracked `config.local.md`          |
| `E_CONSENT_KEY_TRACKING_UNKNOWN` | `openalex.api_key_cmd` in a `config.local.md` whose tracking status cannot be determined |
| `E_LOCAL_PERMS_INSECURE`         | A `config.local.md` looser than `0600` or a symlink — not read, with a warning; fatal only when nothing usable remains |
| `E_TOKEN_CMD_FAILED`             | A key command that could not start or exited non-zero          |
| `E_COMMAND_TIMED_OUT`            | A key command that outlasted the deadline                      |
| `E_COMMAND_OUTPUT_EXCEEDED`      | A key command that printed more than 65,536 bytes across stdout and stderr |
| `E_TOKEN_MALFORMED`              | A key carrying a control character                             |

The [consent keys](reference/skills/config/configure.md#consent-keys)
reference gives each code's remedy.

A tracked `config.local.md` that supplies neither key leaves the call
keyless. A shared `openalex.api_key_cmd` does not: beside a `config.local.md`
that sets no key, the call fails with `E_CONSENT_KEY_TEAM_LEVEL` rather than
going keyless. `accelerator config dump` hides both keys. See
[`/accelerator:configure`](reference/skills/config/configure.md) for the
settings reference.

## `guard`

```bash
accelerator research guard --fail-safe --non-blocking < hook-input.json
```

`hooks/hooks.json` registers the guard under `PreToolUse` for `Bash` and for
`Write|Edit|MultiEdit|NotebookEdit`, so it runs on every such call in every
session. It judges only a subagent confined to one of two roles: the
researcher, whose type is `accelerator:researcher` or the name configured as
`agents.researcher`, and the composer, whose type is `accelerator:composer` or
the name configured as `agents.composer`. A name configured for both roles is
confined as the researcher. Every other call passes untouched, before any
config is read. For the researcher the guard blocks, by exiting `2`:

- a command other than `accelerator research fetch …`, or one carrying shell
  syntax that could run anything else;
- a write anywhere but an indexed finding,
  `<research_topics>/<set>/findings/<nn>-<name>.md`, or a level note,
  `<research_topics>/<set>/findings/<nn>-<name>.levels/<lineage>.md`,
  including through a `..` component or a symlink;
- a call whose command or path cannot be read.

For the composer it blocks every command, the fetch included, every write but
an indexed finding, and a call whose command or path cannot be read. A
finding's `<nn>-<name>` stem is ASCII digits, a `-`, then lowercase ASCII
letters, digits and `-`.

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
| `E_RESEARCH_GUARD_COMMAND`    | A researcher command other than `accelerator research fetch …`, or any composer command, the fetch included |
| `E_RESEARCH_GUARD_SYNTAX`     | A construct that could run anything else, named in the message     |
| `E_RESEARCH_GUARD_WRITE`      | A write outside the role's scope, with its cause: an indexed finding for both roles, plus a level note for the researcher |
| `E_RESEARCH_GUARD_UNREADABLE` | A confined agent's call with no readable command or path           |
| `E_RESEARCH_GUARD_INTERNAL`   | The guard itself failed while judging a confined agent's call      |

Each message names the matched agent type, adding `(agents.researcher)` or
`(agents.composer)` when it matched a configured name. A confined agent reads
a block as a call to correct, not the end of its work.

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

In both cases Claude Code's own permission rules still apply, and a confined
agent's call can no longer write outside `findings/` to induce either failure. A
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
  is skipped and reported. At `depth` above 1, a pair is researched as a tree
  of level notes under `findings/<stem>.levels/`, each node recording the
  follow-up questions the next level researches, and the `composer` agent
  then writes the pair's one finding from those notes.

Each pair's finding lives at `findings/<nn>-<question-slug>-<profile>.md`,
where every profile of one focus area shares its `<nn>`. The finding's
`question` and `source_profile` frontmatter, not its filename, identify the
pair: sets written before profiles existed hold one `findings/<nn>-<slug>.md`
per focus area, and still count as their `web` pair. Consumers group findings
by frontmatter, never by parsing the filename. Consumers read only the
top-level `findings/*.md`: a `<stem>.levels/` directory holds working notes,
never findings.

### `research topic outstanding`

`conduct` asks the research CLI which pairs are outstanding and where their
findings go, rather than allocating paths itself:

```bash
accelerator research topic outstanding SLUG --profiles-dir DIR
```

It prints JSON whose fields are listed below: `items` (each outline item's
`line`, `question`, and `complete`), `pairs` (each outstanding pair's
`question`, `profile`, and absolute `path`), `skipped` (with a `reason`),
`warnings`, `concurrency` (the most spawns this plan offers, after the clamp),
`remaining` (the spawns held back by the concurrency), `subagent_cap` (its
`slots`, and the `source` they came from: the variable's name, or `default`)
and `search_cap` (its `searches`, and the `source` they came from, in the same
form).
The JSON is additive-only: fields may be added, never renamed or removed, and
consumers ignore fields they do not know. It exits `1` with
`E_TOPIC_RESEARCH_UNRESOLVED` for an unknown set.

`--depth N` researches each pair as a tree of level notes under
`findings/<stem>.levels/`, to `N` levels from 1 to 10. It defaults to `1`
and is never read from `research.topic.depth`, so a hand run must pass the
depth `conduct` resolved.
Every plan also carries these fields, whether or not `--depth` is given:

- `depth`, the depth planned for, `1` unless `--depth` says otherwise;
- each pair's `stage`: `single_pass` (one researcher writes the finding),
  `research_nodes` (its missing `nodes`, each with `lineage`, `level`,
  `question`, `cap`, `id`, absolute `path`, `known_questions`, and a
  `rejected` reason when the note on disk was refused) or `compose` (the
  `notes` the composer reads, as absolute paths in lineage order);
- on a run plan, a `search_budget` on every `web` `single_pass` pair and
  every node of a `web` pair: the most `WebSearch` calls its researcher may
  make;
- a `spawn` ref on every `single_pass` or `compose` pair and every node;
- `trims`, one `{stem, lineage, recorded, cap}` per note that recorded more
  follow-ups than its cap, each also printed to stderr as a `warning:` line;
- `shallow`, each retained finding researched below `--depth`, which is
  not deepened again;
- `unfinished`, each `{spawn, rejected}` a run attempted that is still
  outstanding;
- `unanswered`, each outstanding pair as `{stem, question, profile, stage}`:
  the full outstanding set, which `pairs` narrows to the spawns offered now.

`--concurrency N` falls back to `research.topic.concurrency` and is reported
as `concurrency`. A value that is not a positive integer clamps to 1, and one
above the subagent cap clamps to the cap, each with a warning; only an empty
value exits 1, with `E_TOPIC_RESEARCH_CONCURRENCY`. Each diagnostic names the
flag, or the config file that set the value. The cap is
read from `ACCELERATOR_SUBAGENT_CAP`, then
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, each read as Claude Code reads its own
setting, else 20, so a hand run in the same shell sees the cap `conduct` does.
A set `ACCELERATOR_SUBAGENT_CAP` Claude Code would not honour is ignored with a
warning.
Every plan is bounded, a plain query included. The
reported `concurrency` is the clamped value, so it can differ from the
configured knob; the reason appears in `warnings` on `--start` only.

The search cap is Claude Code's limit on `WebSearch` calls per session. It is
read from `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` as Claude Code reads it,
else 200. A set value Claude Code would not honour is ignored, with a warning
on `--start` when the run has web pairs. There is no `ACCELERATOR_*`
counterpart.

A run sizes per-level search budgets once, on its first plan, and stores them
in its ledger, so later plans keep them rather than resize to what remains.
Sizing keeps back a twentieth of the cap, rounded up, then shares the rest
over the worst-case tree of every outstanding web pair: each unresearched
node is assumed to record its level's full follow-up cap. Budgets shrink, or
stay level, with depth. When even budgets of 1 do not fit, every budget is 1
and the sizing plan warns, naming the worst-case demand and the cap. Expect a
halt, then run `/clear` and re-run `conduct`; for later runs, raise
`CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` and restart Claude Code, or lower
the depth.

The run flags exist for `conduct`. `--start` begins a run and returns its
`run` id; `--run ID` continues it, and `--spawned N` acknowledges batch `N`
as spawned. `--withhold-profile P`, which needs `--run` and may be repeated,
holds back researcher spawns of profile `P` for the rest of the run; the run
ledger keeps each withheld profile until `end-run`, so a later `--run` need
not repeat the flag. Compositions are still offered. During a run the JSON
also carries `run`, `batch` (the number the next `--spawned` must pass),
`withheld` (the profiles the run withholds, sorted) and `unexpected` (the
spawn refs of notes or findings nobody was asked to write, or whose content
changed). When the run
has web pairs, it also carries `search_budgets` (each `level` with its
`budget`) and `search_demand` (the searches the worst-case tree would make
spending every budget).
`--start` over another run's readable ledger also carries `replaced_run`: that
run's `run`, its last offered batch split into `valid`, `invalid`, `unwritten`
and `unasked`, each entry a `spawn` ref with the outline `question` and
`profile` that own its stem (or, for an unasked finding, the pair it names; both
are `null` when neither exists), the absolute `quarantine` paths of its invalid
findings, and `owed`, the stems of findings an earlier `--start` listed for
quarantine that are still on disk, whose paths `quarantine` also holds. That
plan is drawn as if each `quarantine` finding were already renamed to its
`.<name>.invalid` marker. A `warning:` line on stderr names the replaced run
and, when `quarantine` is non-empty, asks for those renames; no `warnings` entry
accompanies it. The run's ledger owes each rename whose path the plan does not
offer again: until each is done, `--run` exits 1 with
`E_TOPIC_RESEARCH_QUARANTINE_OWED`, and a later `--start` lists them again in
`owed`. A corrupt ledger yields a `warnings` entry alone.
`accelerator research topic end-run SLUG --run ID` removes the run's
ledger when the run finishes.

It is read-only unless `--start` or `--run` is given; a run records its ledger
in `<set>/.conduct-run.json`, which `end-run` removes. A leftover ledger means a
run did not end. If its session showed an agent usage limit, wait until the
reset time in the message, or raise the limit when it gives none. Then re-run
`conduct`, which reports what that run left of its last batch and quarantines
the invalid findings it wrote. Deleting the ledger instead skips both and leaves
those findings counted. If that run is still going, an agent it spawned can
finish a finding after `--start` judged it; the rename then quarantines it
anyway, and a write after the rename to a path the plan does not offer again
stops the next `--run` with `E_TOPIC_RESEARCH_QUARANTINE_OWED`. No finding is
deleted either way. Consider ignoring `**/.conduct-run.json`.

A web researcher whose search returns Claude Code's search-limit notice
writes nothing and reports "web search quota exhausted". `conduct` then
halts: it passes `--withhold-profile web` so no later plan offers a web
researcher, still runs the run's other researchers and composers, ends the
run, and opens its summary with the halt and the web pairs left unfinished.
To recover, run `/clear`, then re-run `conduct`. Waiting for the quota to
refill is not enough: a re-run in the same session sizes its budgets against
the full cap, so it would halt again.

Each exit code below exits `1`:

| Code | Cause | Recovery |
|---|---|---|
| `E_TOPIC_RESEARCH_DEPTH` | `--depth` is not 1 to 10 | pass 1 to 10 |
| `E_TOPIC_RESEARCH_CONCURRENCY` | `--concurrency` or `research.topic.concurrency` is empty | pass a positive integer, or, in the config file the error names, set the value to one or remove it |
| `E_TOPIC_RESEARCH_RUN` | a malformed `--run`, or `--start` with `--run` | pass the id `--start` returned |
| `E_TOPIC_RESEARCH_SPAWNED` | a malformed `--spawned`, or one without `--run` | pass the `batch` last printed |
| `E_TOPIC_RESEARCH_WITHHOLD_PROFILE` | an empty `--withhold-profile`, or one without `--run` | pass a profile name, with `--run` |
| `E_TOPIC_RESEARCH_RUN_SUPERSEDED` | another run owns the set's ledger | let that run finish |
| `E_TOPIC_RESEARCH_RUN_LEDGER` | the ledger is missing, corrupt, or cannot be written | re-run `conduct` to start a fresh run |
| `E_TOPIC_RESEARCH_QUARANTINE_OWED` | a finding the run's `--start` listed in `replaced_run.quarantine` is still on disk | rename each path the error names to `.<name>.invalid` beside it, then re-run `conduct` |

## Local development

| Mechanism                  | Purpose                                                                                                                                             |
|----------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| `ACCELERATOR_RESEARCH_BIN` | One-shot override pointing `accelerator research …` at a locally-built `accelerator-research` binary, bypassing the normal fetch-and-cache dispatch |

This mirrors `ACCELERATOR_CORPUS_BIN` and `ACCELERATOR_DESIGN_BIN`. Because
the guard runs on every `Bash` and write call, an override naming a missing
binary leaves every researcher unconfined, with a stderr diagnostic on each
call, until it is unset.
