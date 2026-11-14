# Reading-pass brief

Fill this in once per pass, resolving every `{…}` before sending it: the
subagent cannot see the orchestrator's context or resolve placeholders. Keep
the scope explicit and narrow, and give each pass whole concepts, not slices
of files.

---

You are reading pass {pass id} for an interactive exploration page of pull
request #{number}, "{title}" in {repo}. A reviewer will use your output to
decide whether to merge the change and then to own the code. Read the code
yourself; do not delegate.

**Workspace**: `{workspace}`. Write only under `{workspace}/content/`. Never
edit the repository and never run a command that changes it.

**Contract**: `{skill dir}/reference/content-model.md`. Read it first; every
file you write must follow it exactly. `{skill dir}/examples/chapter-version-claims.json`
shows the standard expected of a chapter.

**Read for the design first.** The reviewer needs to understand how this
change is designed more than what any one line does. Before you read
line by line, work out each chapter's design: which abstractions it
introduces or reshapes, what each is responsible for, what it depends on,
which way the dependencies run, and which decisions it embodies, each with
the alternative it was chosen over, why, and what it costs. Compare it with
how the rest of the codebase does the same kind of thing. Record these as
the chapter's `abstractions` and `decisions`. Then choose stops as evidence
for that design: where a responsibility lives, where a decision is enforced,
where a boundary is crossed. Do not walk every hunk.

**Place every stop in the change.** Each stop's `fit` says, before anything
about the lines themselves, how this code fits the change as a whole: which of
the chapter's abstractions or decisions it carries, what calls or feeds it and
what it hands on to, and which other chapter or stop it connects to. The
`note` then explains what the code does. Write the `fit` lines of a chapter
together, in order, and read them back: they should tell the chapter's story
on their own. A `fit` that only restates the `note` has missed the point.

**Draw first, write second.** A reviewer understands a diagram at a glance
and needs three readings of the paragraph that says the same thing, so
figures are the most valuable part of what you produce. For each chapter,
decide the figures before you write any prose: what the change introduces (a
flow), the order things happen in (a sequence), what it leaves on disk or in
state (a tree), the decision it makes (a matrix), the windows it reasons
about (a timeline), and what is different (a comparison). Open every chapter
with at least one figure, and usually two or three. Give a stop a figure
whenever its point is structural. Then write only what the figures cannot
say: why the design is this way, what could go wrong, and what to check.

**Sources** (read-only):

- `{workspace}/head/<path>` and `{workspace}/base/<path>`: the changed files
  at the PR head and at the merge base. A file added by the PR has no base
  copy.
- `{workspace}/data/diff.json`: the per-file diff.
- `{workspace}/content/findings.json`: prior review findings, ids
  `{finding prefix}-NN`.
- Unchanged files at head: {how to read an unchanged file, e.g. the session
  VCS's file-at-revision command for {head sha}, or
  `gh api -H "Accept: application/vnd.github.raw" repos/{repo}/contents/<path>?ref={head sha}`}.
- Context: {PR description path, work item, plan, ADRs: whatever exists}.

**Write these chapters**, in this reading order:

{for each chapter: `content/chapter-<id>.json`: "<title>" — the concept it
covers, the files and symbols at its centre, the figures it should open with
(for example "a sequence of start-up from identify to the first lookup; a
tree of `claims/` after a prune"), and any question the reviewer most needs
answered, such as whether a claimed invariant really holds}

**Also write**:

- `content/ledger-{pass id}.json`: one row for every production call site in
  your files that {ledger subject, e.g. "removes, replaces or truncates
  something on disk"}. Use the facets {facet keys and allowed values}. Give
  each row a `start`/`end` spanning the call and the guards a reviewer must
  read to judge it: the page shows that range and counts it as read when the
  row gets a verdict. An empty array is a valid answer.
- `content/files-{pass id}.json`: one entry for each of these files:
  {file list}.
{optional: `content/specs/<name>.md`, a precise, implementable specification
for a custom view, e.g. a simulator of the decision procedure in
`<file>`. It needs every constant with its value and source line, the input
space, the exact decision for each input as rules and tables (not prose),
the order of effects, what is reported, and 4–7 named scenarios with
expected outcomes. A view is built from it, so it must be exact.}
{optional: `content/breaking.json`, `content/ci.json`: name who verifies what}

**Prior findings in your area**: {ids}. Give each a `findingChecks` entry in
the chapter it belongs to. Be adversarial: try to refute each finding against
the code before you agree with it, say exactly what the code shows, and
recommend a triage. Set its `level`: `design` when it concerns shape,
boundaries, responsibilities, dependencies or a decision, and `implementation`
when a line is wrong. Raise design concerns of your own in `ownNotes`. Matter most: {ids that bear on the merge decision, and the
precise question for each}.

**Figures**: follow the contract's Figures section. One idea per figure,
named in its title. Use the code's own names and real values, keep each one
small enough to read at a glance, mark what the PR adds or removes with
`tone`, and put `where` on every box, step or path that has a home in the
code. Write captions that say what to notice.

**Writing**: follow the contract's writing rules. One term per concept,
reusing the code's own names: {domain terms already established}. Record
anything you could not verify as unverified rather than guessing.

**Anchors**: give every stop and every ledger row an `anchor`, following the
contract's Anchors section: a piece of one line copied exactly from the
`head/` copy (the `base/` copy for a stop with `side: "old"`) that no other
line of that file contains. A stop's anchor is on its `start` line; when
that line is a comment, an attribute or not distinctive, anchor on a later
line of the range and name it in `anchorLine`. A ledger row's anchor is on
its `line`. Copy, never retype: the build compares the text.

**Citations**: every `where`, `pinnedBy` and `tests` entry must name a
changed file by its full repository path, and a test by the name it is
declared with. Refer to another stop by its id in backticks and to another
chapter as chapter `<id>`: the page turns both into links, and the build
reports any that do not resolve. Stops other passes write are named
`<chapter id>-s<n>`; name one only if you were told it exists, and otherwise
name its chapter.

**Before you finish**, run `python3 -m json.tool <file> > /dev/null` on every
JSON file you wrote. Check at least five `start`/`end` and ledger `line`,
`start` and `end` values, and three figure `where` values, against the
`head/` copies. Confirm that every stop and ledger row has an anchor that
appears on its line, and that every chapter you wrote has a figure and at
least one abstraction or decision.

**Your final message** (at most 150 words): the files you wrote, the
design each chapter describes in one sentence, the figures in each, anything
you think blocks the merge, and anything you could not verify. Do not repeat
the file contents.
