# explore-pr content model

The page renders a **workspace**. `scripts/build.py` merges it, checks every
reference, and emits the page. Everything below is the contract between the
orchestrator, the reading passes and the page.

```text
<workspace>/
  explore.json            manifest: the PR, chapter order, layers, labels
  content/
    design.json           the design: summary, architecture figures, principles, extension points
    overview.json         lede, what changes, ways through the review
    chapter-<id>.json     one guided-tour chapter each
    ledger*.json          risky-operation rows (arrays, concatenated)
    files*.json           per-file summary, chapter and risk (arrays, concatenated)
    findings.json         prior review findings (scripts/parse_review.py)
    breaking.json         behaviour that changes for existing users
    ci.json               CI checks at head
    accepted-risks.json   risks the plan or description accepts by name
    criteria.json         acceptance criteria to tests (scripts/criteria_map.py)
    modmap.json           module map: layers, nodes, edges
  data/diff.json          per-file diff (scripts/extract_diff.py)
  data/commits.json       per-commit hunks, traced onto the diff (scripts/extract_commits.py)
  views/*.js              PR-specific views (reference/custom-views.md)
  page.html               optional: a modified copy of assets/page.html
```

Every file except the manifest and the diff is optional: a view or section
whose data is absent is not shown. Line numbers are **head** line numbers
(`side: "new"`) unless a stop points at deleted code (`side: "old"`, base line
numbers). Paths are repository paths.

## Design before lines

The reviewer comes to this page to understand how the change is designed,
more than to read its lines. Lines are evidence; the design is what they will
approve, maintain and extend. So the content leads with design at every
level:

- **`design.json`** states the design of the whole change: its shape in a
  paragraph and a figure, the principles of the existing architecture it
  follows or strains, and where the next change would go. It is the page's
  second view, before any chapter.
- **Every chapter names what it adds to the design**: the `abstractions` it
  introduces or reshapes (types, traits, modules, ports, file formats,
  protocols), with their responsibilities and collaborators, and the
  `decisions` it embodies, each with the alternatives it was chosen over, why,
  and at what cost. The page draws the abstractions' collaborations as a
  diagram automatically. The build warns about a chapter with neither.
  The Design view lists each abstraction once, merged by name across
  `design.json` and every chapter: the first declaration describes it, its
  collaborators are all of theirs, and it links to every chapter that
  declares it.
- **Stops are evidence for the design**, chosen to show where a responsibility
  lives or how a decision is enforced, not to walk every hunk. Every stop
  says first how its code fits the bigger picture (`fit`), then what the code
  does (`note`): a reviewer who reads only the `fit` lines of a chapter should
  be able to trace how the change hangs together. The build warns about a
  stop without one.
- **Findings carry a level**: `design` (shape, boundaries, responsibilities,
  dependencies, decisions) or `implementation` (a line that is wrong). Design
  findings are the expensive ones to change later, so they come first.

## Pictures first

A figure is worth more than any amount of prose about the same thing. A
reviewer takes in a sequence diagram of five steps at a glance, and needs
three readings of the paragraph that says the same. So the content is drawn
first and written second:

- **Every chapter opens with at least one figure**, and usually two or three:
  what the change introduces (a flow), the order things happen in (a
  sequence), the state it leaves behind (a tree), the decision it makes (a
  matrix), the windows or ages it reasons about (a timeline), or what is
  different (a comparison). The build warns about a chapter without one.
- **The overview opens with the change at a glance**: usually a before/after
  comparison of the part of the system the PR changes. The build warns if it
  is missing.
- **A stop whose point is structural carries a figure**: a call order, a
  layout on disk, a branch in a decision.
- **Prose says what the figure cannot**: why the design is this way, what
  could go wrong, and what to check. If a paragraph describes boxes and
  arrows, draw the boxes and arrows and delete the paragraph.
- When a mechanism deserves more than a static figure (a reviewer would want
  to vary the inputs and watch the outcome), build a custom view
  (`reference/custom-views.md`).

## Writing rules for every prose field

- British English. The reader is a senior engineer fluent in the stack, deciding
  whether to merge and preparing to own the code.
- Markdown, inline only unless stated: backticks on every identifier, path and
  literal, and around anything in angle brackets (`<version>` unquoted is read
  as an HTML tag and vanishes); no headings; no emoji (the page renders
  severity as chips). Diagrams go in figures, not in ASCII art.
- Keep `purpose` to 2–3 sentences and `mechanism` to about 120 words when a
  figure carries the mechanism, which it should.
- Lead with the point; 2–5 sentences per note. Quantify. Name specifics
  (`store.rs:212`, not "the relevant code").
- Use the PR's own domain terms, one term per concept, exactly as the code
  names them. Define each in its chapter's `domain`. The page shows the
  chapter's terms straight after its figures, before any of its prose, so
  each meaning must stand on its own.
- Refer to a stop by its id (`claims-s3`) and to a chapter as chapter
  `claims` or the `claims` chapter, in backticks. The page replaces each with
  a link named by the stop's number and title or the chapter's title, so
  never write a stop number or a chapter title out by hand. The build warns
  about a stop or chapter named this way that does not exist.
- State only what was verified by reading the code. Mark the rest unverified,
  or make it a `question`.

## What the build requires

`scripts/build.py` exits 1 while any `warning:` remains, and prints each one
with the file or item it concerns. A `note:` may stand and does not fail the
build. Before it checks anything else, the build makes sure the page can use
the content, and leaves out what it cannot:

- every content file is valid JSON of the kind shown here: an object, or an
  array for `ledger*`, `files*`, `findings`, `breaking`, `accepted-risks`
  and `criteria`;
- a chapter has an `id`; a stop has `id`, `file`, `start` and `end`; a
  ledger row has `id`, `file` and `line`; a file summary has `path`; a
  finding, a finding check and an accepted risk have an `id`. Line numbers
  are whole numbers, not text;
- a flow's nodes and layers and a sequence's actors each have an `id`, and a
  collaborator given as an object has a `name`;
- a field holds the kind of value shown here: text for an `id`, `file`,
  `where` or `anchor`, a whole number for a line, a list of ids for
  `findings`, `refs` and `pinnedBy`. A field of the wrong kind is reported
  and ignored; `null` is read as absent;
- a field whose values this document lists with `|` between them holds one
  of those values: `side`, `status`, `assessment`, `holds`, `level`,
  `verdict`, `triage`, `risk`, `severity`, `state`, a check's `status`, and
  every `tone`. An abstraction's `kind` is free text.

## References the build checks

Every citation must open something. `scripts/build.py` warns about:

- a `where` (or a breaking change's `changelog`) that is not `path:line` or
  `path:start-end`, that names a file the PR does not change, that matches
  more than one changed file, or whose lines fall outside the file. A path
  may be a suffix (`store.rs:212`) only while it matches one changed file;
- a test in `pinnedBy`, a ledger row's `tests` or a chapter's `tests` whose
  file the PR does not change, whose name does not appear in that file at
  head, or whose `line` falls outside it. Write tests as `path::name`; only
  the last `::` segment is looked for;
- a `chapter` on a file summary, a figure box, a module-map box, an
  abstraction or a decision that names no chapter;
- prose that names a stop or chapter that does not exist.

The workspace holds only the files the PR changes, so a citation into an
unchanged file cannot be checked or opened: the page shows it as plain text
and the build reports it in a note as *not a changed file; unverified*.
Keep one only when the point cannot be made from a changed file, and verify
it by hand.

## Anchors

Line numbers alone go stale without anyone noticing: a range a few lines off
still falls inside its file, and when the PR's head moves every stop below
the edit points at other code. So each stop and ledger row quotes the code
it stands on. The `anchor` is a piece of one line, copied from the code and
never retyped or paraphrased, that no other line of the file contains: a
signature, a constant's declaration, the call a ledger row is about. Spacing
is ignored. A stop's anchor sits on its `start` line, or on `anchorLine`
when the range opens on a comment, an attribute or a line that is not
distinctive; a ledger row's sits on its `line`.

The build compares each anchor with the code and warns when:

- there is none: every stop and ledger row needs one;
- it is on one other line of the file: the code moved, and the warning gives
  the shift. `build.py --reanchor` applies every such move, shifting
  `start`, `end`, `anchorLine` and `line` together and rewriting the content
  files;
- it is on several lines: choose a line only that code has;
- it is nowhere in the file: the code changed, so read it again and rewrite
  the stop or row.

Re-anchoring restores the position, not the prose. A stop that moved cleanly
may still describe lines that were edited, so after a head move check each
moved stop's range against the new diff before trusting its note.

## `explore.json`

```jsonc
{
  "title": "PR 170 Review Desk",              // page name: 2–4 words, no explanation
  "pr": {
    "number": 170, "title": "…", "author": "…",
    "repo": "owner/name",                     // gives the GitHub links
    "head": "<sha>", "base": "<merge-base sha>", "branch": "…", "commits": 81
  },
  "chapters": ["identity", "claims", "…"],    // reading order; others follow by name
  "layers": [                                 // optional; default: prod, tests, build, records
    { "id": "prod", "label": "Production code", "match": ["^src/"] },
    { "id": "rest", "label": "Everything else", "fallback": true }
  ],
  "pathAliases": [["very/long/prefix/", "prefix/"]],
  "ledger": {
    "title": "Deletion ledger",               // rail label and eyebrow
    "short": "Removals",                      // top-bar meter label
    "heading": "Every place the launcher deletes something",
    "lede": "…",                              // optional extra sentence
    "noun": "production call site",
    "facets": [{ "key": "root", "label": "Root" }],
    "labels": { "context": "Lock held", "concurrency": "Race", "guards": "Guards", "onFailure": "On failure" }
  },
  "findings": { "source": "review-1", "title": "Review-1, re-checked against the code" },
  "tour": { "lede": "One sentence on the arc of the chapters." },
  "views": ["prune-simulator.js"]             // optional order for views/*.js
}
```

Layers decide the attention map's grouping. Rules are regular expressions
tried in order; the `fallback` layer takes what nothing else matches. The
`prod` layer id is what "production lines only" sizes by.

## `design.json`

```jsonc
{
  "title": "How the change is designed",       // optional heading
  "summary": "md, 3–5 sentences: the shape of the change in design terms: the new abstractions, how responsibility is divided, which way dependencies run, and what makes it safe or testable. Bold the key abstractions.",
  "figures": [ /* the architecture: a layered flow, a before/after compare */ ],
  "abstractions": [ /* system-level ones no single chapter owns; same shape as a chapter's */ ],
  "decisions": [ /* cross-cutting decisions; same shape as a chapter's, with "chapter" if one owns it */ ],
  "principles": [{                              // the existing architecture's rules, from CLAUDE.md, ADRs and the code
    "principle": "Ports and adapters: core logic reaches I/O only through ports",
    "verdict": "holds|strained|broken",
    "evidence": "md, ≤ 2 sentences, with finding ids where they apply",
    "where": "path:line"
  }],
  "extension": [{                               // for owning it: where the next change goes
    "change": "Support a group-shared root", "where": "path:line", "how": "md, ≤ 2 sentences"
  }],
  "signOff": "md: the question to answer about the design as a whole"
}
```

## `overview.json`

```jsonc
{
  "figures": [ /* the change at a glance: one or two figures, see Figures */ ],
  "lede": "md, 2 short paragraphs: what the PR does and the headline assessment",
  "whatChanges": "md bullet list: one bullet per concept, bold label first",
  "paths": [{ "title": "Merge decision", "time": "about 1 hour", "body": "md, numbered steps through the page" }]
}
```

## `chapter-<id>.json`

```jsonc
{
  "id": "claims",
  "title": "Version claims",                  // ≤ 5 words, a label not a teaser
  "oneLiner": "…",                            // ≤ 25 words: what this code does
  "figures": [ /* at least one; shown first, before any prose. See Figures */ ],
  "abstractions": [{                          // what this chapter adds to the design
    "id": "a-fresh-versions", "name": "FreshVersions",
    "kind": "type|trait|enum|module|port|adapter|function|file format|protocol",
    "status": "new|changed|removed|existing",
    "responsibility": "≤ 25 words: what it is for, in domain terms",
    "collaborators": ["Clock", { "name": "Disk", "kind": "port" }],
                                              // what it uses, drawn as arrows: an abstraction's
                                              // name, or a name and kind for one that is not
    "where": "path:line"
  }],
  "decisions": [{                             // design choices this chapter embodies
    "id": "d-claim-heartbeat", "decision": "≤ 15 words: what was chosen",
    "alternatives": ["what it was chosen over"],
    "rationale": "md: why",
    "consequences": "md: what it costs, and what it makes easy or hard later",
    "where": "path:line", "adr": "path to an ADR, if one records it",
    "assessment": "sound|questionable|unsound", "note": "md: the reading pass's view, ≤ 2 sentences"
  }],
  "purpose": "…",                             // md, 2–3 sentences: the problem it solves
  "mechanism": "…",                           // md, ≤ ~120 words: what the figures cannot show
  "beforeAfter": [{ "aspect": "…", "before": "…", "after": "…" }],
  "domain": [{ "term": "version claim", "meaning": "≤ 25 words", "code": "VersionClaim" }],
  "invariants": [{
    "id": "claims-i1", "text": "…", "where": "path:line",
    "pinnedBy": ["path/to/test.rs::test_name"],
    "holds": "yes|partly|unsure", "why": "md, only when partly or unsure"
  }],
  "risks": [{ "level": "high|medium|low", "text": "md", "where": "path:line", "refs": ["R1-06"] }],
  "stops": [{                                 // the tour, in reading order; 6–12
    "id": "claims-s1", "file": "path", "side": "new", "start": 120, "end": 160,
    "anchor": "fn claim_version(",            // code copied from the anchor line; see Anchors
    "anchorLine": 124,                        // optional: the line it is on; defaults to start
    "title": "≤ 8 words",
    "fit": "md, 1–3 sentences: where this code sits in the change as a whole: the abstraction or decision it carries, what reaches it and what it hands on to, and which other chapter or stop it connects to; shown before the note",
    "note": "md, 2–5 sentences: what it does, why it is shaped this way, what could go wrong",
    "check": "md, optional: the one thing to verify here",
    "figure": { /* optional, one figure shown beside the code; see Figures */ },
    "findings": ["R1-06"]
  }],
  "tests": [{ "file": "path", "name": "test_fn", "line": 123, "proves": "≤ 20 words" }],
  "findingChecks": [{                         // every prior finding in this chapter's area
    "id": "R1-06", "verdict": "confirmed|partly|disputed|unverified",
    "note": "md, ≤ 3 sentences: what the code shows",
    "triage": "block|fix-now|follow-up|drop", // a recommendation; the reviewer decides
    "level": "design|implementation"          // design: shape, boundaries, dependencies, decisions
  }],
  "ownNotes": [{ "level": "major|minor|question", "text": "md", "where": "path:line" }],
  "signOff": "md, 1–2 sentences: the question to answer before signing this chapter off"
}
```

`examples/chapter-version-claims.json` is a real chapter at the expected
standard: three chapter figures (a timeline, a sequence and a tree), three
abstractions, two decisions with their alternatives and costs, a figure on a
stop, an anchor on every stop, and a level on each finding check. Stops should cover what a reviewer must read to own the chapter, not
every hunk; ranges of 10–60 lines read best. A stop may sit on unchanged lines
of a changed file when the change cannot be understood without them.

## Figures

The page draws seven kinds of figure from data, laid out and coloured for
light and dark themes, and gathers every one into the **In pictures** view.
Pressing boxes in a flow (or the module map) traces their arrows, those they
send in one colour and those they receive in another, and dims the rest;
pressing a box again drops it from the trace, and Esc clears it. A box with a
`where` or `chapter` opens it from its own ↗ control, not from the box. Arrows
are routed as horizontal and vertical runs with rounded corners, around boxes
rather than behind them; a flow keeps the left-to-right order its nodes are
given in, so order each row's nodes to keep arrows from crossing. Tracing keeps
a busy flow legible; a flow that cannot be read without it should still be
split.

Every figure has:

```jsonc
{ "id": "claims-f1",                          // unique across the page
  "kind": "flow|sequence|tree|matrix|timeline|compare|svg",
  "title": "≤ 8 words: the point, not the topic ('Start-up claims before any lookup')",
  "caption": "md, 1–2 sentences: what to notice",
  "wide": false }                             // optional: span the full row
```

| Kind | Draw it for | Shape |
|---|---|---|
| `flow` | Architecture, call graphs, data flow, what depends on what | Boxes in rows with arrows; layered by `layers`, or by edge depth when absent |
| `sequence` | Protocols, start-up order, lock ordering, races, retries | Actors across, numbered messages down, notes over actors |
| `tree` | Layouts on disk, config trees, module trees, what a run leaves behind | An indented tree with added, removed, changed and kept states |
| `matrix` | Decision tables, permission and configuration matrices, gates | Rows by columns, each cell a short outcome with a tone |
| `timeline` | Ages, windows, timeouts, backoff, ordering in time | An axis (linear or log) with spans in lanes and labelled marks |
| `compare` | Anything the PR changes shape: before on the left, after on the right | Two figures side by side |
| `svg` | The rare picture none of the above can draw | Inline SVG you write; use `currentColor` and `var(--ink)`, `var(--accent)`, `var(--bad)`, `var(--ok)` so it works in both themes |

```jsonc
// flow
{ "kind": "flow",
  "layers": [{ "id": "entry", "label": "Entry" }],                 // optional rows, top to bottom
  "nodes": [{ "id": "a", "label": "start_up", "sub": "identify, claim", "layer": "entry",
              "tone": "new|changed|removed|risk|ok", "where": "path:line", "chapter": "identity" }],
  "edges": [{ "from": "a", "to": "b", "label": "Try", "tone": "risk", "dashed": true }] }

// sequence
{ "kind": "sequence",
  "actors": [{ "id": "launcher", "label": "launcher" }],
  "steps": [{ "from": "launcher", "to": "claims", "label": "lstat the claim", "where": "path:line", "tone": "risk" },
            { "over": ["launcher", "claims"], "note": "Young enough: stop here.", "tone": "risk" },
            { "from": "launcher", "to": "launcher", "label": "retries once" }] }

// tree
{ "kind": "tree", "root": "<root>/claims/",
  "items": [{ "path": "sub/dir/name", "state": "added|removed|changed|kept", "note": "md", "where": "path:line" }] }

// matrix
{ "kind": "matrix", "rowLabel": "Root", "colLabel": "Trigger",
  "rows": ["Default", "Relocated"], "cols": ["Start-up", "Cold path"],
  "cells": [["no claim", { "text": "prune, `Try`", "tone": "ok|warn|bad|info|none" }], ["…", "…"]] }

// timeline (values in one unit; ticks name them)
{ "kind": "timeline", "scale": "log|linear", "from": 60, "to": 5184000,
  "ticks": [{ "at": 3600, "label": "1 h" }],
  "spans": [{ "from": 60, "to": 120960, "label": "young", "tone": "info|ok|warn|bad|none" }],
  "marks": [{ "at": 1728000, "label": "1.24.0", "tone": "bad" }] }

// compare
{ "kind": "compare", "before": { /* any figure */ }, "after": { /* any figure */ } }

// svg
{ "kind": "svg", "svg": "<svg viewBox='0 0 600 200'>…</svg>" }
```

What makes a figure worth its space:

- **One idea per figure**, stated in its title. Two ideas make two figures.
- **The code's own names** on boxes, actors and paths, and real values on
  axes and in cells: `CLAIM_WINDOW` at 14 d, not "the window".
- **Small enough to read at a glance**: up to about 12 nodes, 8 actors, 10
  steps, 6 columns. Split anything bigger.
- **Tone marks what changed**: `new` for what the PR adds, `changed`,
  `removed`, and `risk` for where it can go wrong. The page draws a legend.
- **`where` on everything that has a home in the code**, so a click opens it.
- **A caption that says what to notice**, never one that restates the title.

## `ledger*.json`

One row per production call site in the class of operation the ledger tracks:
deletions, migrations, writes to shared state, authorisation decisions,
external side effects — whatever this PR can do that is hard to undo.

```jsonc
{
  "id": "L-tree-01",
  "file": "path", "line": 561,                // head line of the call
  "anchor": "fs::remove_dir_all(",            // code copied from that line; see Anchors
  "start": 548, "end": 563,                   // optional: the call and the guards read with it; defaults to line
  "op": "unlink",                             // short operation name; counted in the lede
  "subject": "≤ 15 words: what is affected",
  "facets": { "root": ["relocated"], "trigger": ["verb", "cold-store"] },  // "any" matches every filter
  "context": "≤ 20 words: locks, transactions or permissions held at the call",
  "guards": ["≤ 15 words each: every condition that must hold to reach the call"],
  "concurrency": "md ≤ 3 sentences: the concurrent actor and why it is safe, or not",
  "onFailure": "≤ 15 words",
  "tests": ["path::test_name"],
  "findings": ["R1-34"],
  "concern": null                             // or { "level": "major|minor|question", "text": "md" }
}
```

## `files*.json`

One entry per changed file.

```jsonc
{ "path": "src/a.rs", "chapter": "claims", "risk": "high|medium|low|skim",
  "summary": "≤ 20 words: what changed and why",
  "prodLines": 215, "testLines": 381 }       // added lines in production and inline-test code
```

Risk tiers: `high` destroys or rewrites data, decides trust or access, or sits
on a hot path; `medium` is wiring or logic a bug could hide in; `low` is tests
and support code; `skim` is docs, lockfiles, generated or mechanical change.

## `findings.json`

Produced by `scripts/parse_review.py` from a review-pr review. Any other source
can be mapped to the same shape:

```jsonc
{ "id": "R1-06", "severity": "critical|major|minor|suggestion", "lenses": ["safety"],
  "file": "path or null", "start": 138, "end": 138,
  "kind": "inline|comment|general", "title": "…", "summary": "md", "body": "md or absent" }
```

## Smaller files

```jsonc
// breaking.json
[{ "id": "B1", "title": "≤ 8 words", "before": "md", "after": "md",
   "affects": "≤ 20 words: who notices", "where": "path:line", "changelog": "CHANGELOG.md:line" }]

// ci.json
{ "checks": [{ "name": "…", "status": "pass|fail|cancelled|skipped", "cause": "md or null", "fix": "md or null" }] }

// accepted-risks.json
[{ "id": "AR1", "title": "…", "gist": "≤ 40 words", "findings": ["R1-12"] }]

// criteria.json — scripts/criteria_map.py
[{ "n": 1, "label": "…", "phase": "4", "note": "md or null",
   "tests": [{ "ref": "name as written", "hits": [{ "name": "…", "file": "…", "line": 10 }] }] }]

// modmap.json — ≤ 12 nodes; edges point from caller to callee and run down the layers
{ "title": "How the modules fit", "caption": "…",
  "layers": [{ "id": "entry", "label": "Entry", "sub": "…" }],
  "nodes": [{ "id": "start_up", "layer": "entry", "label": "start_up", "sub": "…", "isNew": true, "chapter": "identity" }],
  "edges": [["start_up", "root_retention"]] }
```

## Review state

Every review control writes one record keyed by its target: `decision` (the
overall verdict), `design-overall`, `design:<decision id>`, `chapter:<id>`,
`stop:<id>`, `inv:<id>`, `finding:<id>`, `own:<chapter>-<n>`,
`ledger:<id>`, `file:<path>`, `commit:<sha>`, and `lines:<path>` for the
lines marked one at a time (its `lines` field lists them as `new:12-15` or
`old:4`; `<path>` is `commit:<sha>:<path>` for a superseded line). Each record carries `target`, `kind`, `status`,
`note`, `label`, `where` and `updatedAt`. Hosted on claude.ai, records live in
the artifact's `reviews` collection, with `/` in a target written as `~2f`.
Opened as a file, they live in the browser's local storage under
`explore-pr:<repo>#<number>`.

### Lines read

Which changed lines the reviewer has read is derived from those records, never
stored. A verdict of any kind on a stop claims its `start`–`end` on its
`side`; on a ledger row, its `start`–`end`, or its `line` alone; on a file,
the whole file; on a commit, every line it changes; and a `lines` record, the
lines it lists. A file's changed lines are its added head lines and removed
base lines; context never needs reading. **All files** shows each file read in
full, in part or not at all, marks each changed line in the code as read (a
green edge, a tick, faded text, and what read it) or still to read (an amber
edge), and the Files meter counts files read in full. In the other direction, a stop, ledger row or
file whose lines other verdicts have already read shows as *read via* them,
without a verdict of its own. In **By commit**, each line a commit changes is traced onto the whole diff, so
it is the same line there as in All files and the tour. A line a later commit
rewrites or removes, a line removed after an earlier commit added it, and a
change a later commit restores are **superseded**: they can be read and
marked, but are not counted. `assets/review-coverage.js` is the model; **Your
notes** ends with the lines still to read.
