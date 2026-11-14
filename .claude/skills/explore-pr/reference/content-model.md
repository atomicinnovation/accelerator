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
  and at what cost. The page draws how the abstractions relate as a class
  diagram automatically. The build warns about a chapter with neither.
  The Design view lists each abstraction once, merged by name across
  `design.json` and every chapter: the first declaration describes it, its
  collaborators are all of theirs, and it links to every chapter that
  declares it.
- **Stops are evidence for the design**, chosen to show where a responsibility
  lives or how a decision is enforced, not to walk every hunk. Every stop
  names what it is evidence for (`carries`) and the role its code plays in
  the design (`fit`): a reviewer who reads only the `fit` lines of a chapter
  should be able to trace how the change hangs together. The build warns
  about a stop without either, and about a decision or a high risk that no
  stop carries.
- **A stop says what its code cannot.** The code is shown beside the text,
  so a sentence that says what the lines do is wasted on a reader who can
  see them. See "What a stop says".
- **Findings carry a level**: `design` (shape, boundaries, responsibilities,
  dependencies, decisions) or `implementation` (a line that is wrong). Design
  findings are the expensive ones to change later, so they come first.

## What a stop says

The reviewer reads the code at a stop for themselves. The text beside it
is there for what they cannot get from those lines:

- **`carries`**: the decisions, abstractions, invariants and risks this
  code is evidence for, by id. The page names each one above the stop and
  links to it, so the stop inherits a decision's alternatives and cost
  instead of repeating them, and each decision and abstraction lists the
  stops that show it.
- **`fit`**: the role the code plays in the design: the responsibility
  that lives here or the decision enforced here, and what in the change
  depends on it. Not who calls it; the chapter's figures draw that.
- **`why`**: why the code is shaped this way: the constraint that forced
  it, the alternative rejected at this spot, and what the slice cannot
  show (how the base behaved, what callers assume, what is deliberately
  absent, which test pins it).
- **`risk`**: what breaks if this code is wrong, how far it spreads, and
  whether it fails loudly or silently. Leave it out where nothing is at
  stake.
- **`checks`**: questions with a falsifiable answer. Each says what to
  look at (`question`), where (`where`, when it is one place) and what
  answer would be a problem (`problem`). "Confirm that X does Y" is not a
  check: it restates the code and has no failing answer.
- **`focus`**: the one or two lines that matter, each with a few words on
  why. The page marks them in the code. Use it where a narrated walk
  through the block would otherwise go: "`<`, not `<=`, deliberately" is a
  pointer, not a paragraph.

**The deletion test.** Before keeping a sentence in `fit`, `why` or
`risk`, ask whether the reviewer could have written it from the lines
shown. If so, delete it, or turn it into a `focus` on the line it is
about.

**Where the why comes from.** The PR description, commit messages, ADRs,
the plan or work item, prior review findings, and the base copy of the
file. A why that none of them states and the code does not make evident
is not known: write it as a `question` in `ownNotes`, never as a reason
you inferred.

The build warns about a stop with no `why`, one that carries nothing or
carries an id nothing declares, one that states or carries a risk but has
no `checks`, a `focus` line outside the stop's range, and a stop that
still has the `note` or `check` these fields replaced. It leaves a `note:`
on a `why` that names four or more identifiers from its own lines: read
that stop again with the deletion test.

## Pictures first

A figure is worth more than any amount of prose about the same thing. A
reviewer takes in a sequence diagram of five steps at a glance, and needs
three readings of the paragraph that says the same. So the content is drawn
first and written second:

- **Every chapter opens with at least one figure**, and usually two or three,
  each chosen by the question it answers ("Choosing a figure"): how what
  the change introduces is put together, the order things happen in, the
  state it leaves behind, the decision it makes, or what is different. The
  build warns about a chapter without one.
- **The overview opens with the change at a glance**: usually a before/after
  comparison of the part of the system the PR changes. The build warns if it
  is missing.
- **A stop whose point is structural carries a figure**: a call order, a
  layout on disk, a branch in a decision. A drawing too wide for the
  column beside the code is shown across the row, above the code.
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
- Lead with the point; 1–3 sentences per field of a stop. Quantify. Name
  specifics (`store.rs:212`, not "the relevant code").
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
- a collaborator given as an object has a `name`;
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

A changed file is carried whole, so any line of it can be cited and shown,
with two exceptions: a lockfile or generated file, and a file of more than
20,000 lines. Of those the page holds only the five lines either side of
each change. A stop or ledger row on other lines is a warning, since the
page cannot show them, and a test named there is noted as unverified.
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
  `start`, `end`, `anchorLine`, `line` and a stop's `focus` lines together
  and rewriting the content files;
- it is on several lines: choose a line only that code has;
- it is nowhere in the file: the code changed, so read it again and rewrite
  the stop or row.

Re-anchoring restores the position, not the prose. A stop that moved cleanly
may still describe lines that were edited, so after a head move check each
moved stop's range and `focus` lines against the new diff before trusting
what it says.

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
  "figures": [ /* the architecture, chosen by the guide; a before/after compare */ ],
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
    "collaborators": ["Clock", { "name": "Disk", "kind": "port", "relation": "implements" }],
                                              // what it relates to: an abstraction's name, or an
                                              // object. "kind" is needed for one that is not an
                                              // abstraction. "relation" is implements|extends|
                                              // owns|holds|uses, and "uses" when left out
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
  "risks": [{ "id": "claims-r1",              // needed on a high risk, so a stop can carry it
    "level": "high|medium|low", "text": "md", "where": "path:line", "refs": ["R1-06"] }],
  "stops": [{                                 // the tour, in reading order; 6–12
    "id": "claims-s1", "file": "path", "side": "new", "start": 120, "end": 160,
    "anchor": "fn claim_version(",            // code copied from the anchor line; see Anchors
    "anchorLine": 124,                        // optional: the line it is on; defaults to start
    "title": "≤ 8 words",
    "carries": ["d-claim-heartbeat", "claims-i1"],
                                              // ids of the decisions, abstractions, invariants and
                                              // risks this code is evidence for; at least one
    "fit": "md, 1–2 sentences: the role this code plays in the design: the responsibility that lives here or the decision enforced here, and what depends on it. Not who calls it",
    "why": "md, 1–3 sentences: why it is shaped this way: the constraint, the alternative rejected here, what these lines cannot show",
    "risk": "md, optional, 1–2 sentences: what breaks if this is wrong, how far it spreads, loudly or silently",
    "checks": [{                              // needed when the stop states or carries a risk
      "question": "md: something with a falsifiable answer",
      "where": "path:line, optional: where to look",
      "problem": "md: the answer that would be a problem"
    }],
    "focus": [{ "line": 131, "why": "md, ≤ 20 words: why this line matters" }],
                                              // optional: lines inside start–end, marked in the code
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
standard: three chapter figures (a timeline, a sequence diagram and a tree), three
abstractions, two decisions with their alternatives and costs, a figure on a
stop, an anchor on every stop, and a level on each finding check. Each of
its stops says what it carries, why its code is shaped as it is, what is at
risk and how to check it, with the lines that matter marked, and none says
what the code does. Stops should cover what a reviewer must read to own the chapter, not
every hunk; ranges of 10–60 lines read best. A stop may sit on unchanged lines
of a changed file when the change cannot be understood without them.

## Figures

The page draws every figure from data, laid out and coloured for light and
dark themes, each shown where it explains something: in the overview, the
Design view, its chapter or its stop.

### Choosing a figure

Choose by the question a reviewer has, not by the subject. The guide the
diagram skills keep, at `{skill dir}/../draw-diagram/reference/choosing.md`,
gives the question each kit diagram answers, how to tell neighbouring
kinds apart, and which views go together for a new abstraction, a
lifecycle, a schema change, anything that handles a secret, a parser or a
change in how code is organised. Read it before sketching a chapter's
figures.

Two kinds are this page's own, and the guide does not cover them:

- `compare`: the same part of the system before and after, as two figures
  of any kind side by side. The overview usually opens with one.
- `svg`: the rare picture nothing else draws.

`flow`, the page's old boxes and arrows, is no longer drawn, and the build
warns about one. What it showed is a `block-diagram`, or better the kind
the guide names for it.

### How the page draws them

Pressing boxes in a diagram that has arrows traces them, those a box sends
in one colour and those it receives in another, and dims the rest; pressing
a box again drops it from the trace, and Esc clears it. A box with a `link`
opens it from its own ↗ control, not from the box.

Under every figure the kit draws is a legend, closed until the reviewer
opens it, headed by the kind of diagram it is: what each symbol of that notation means, drawn as it appears in
the figure, with the symbols the figure does not use behind a switch, and
how to trace and open things. The build writes it. So a caption never
needs to explain the notation: it says what to notice.

Every figure has:

```jsonc
{ "id": "claims-f1",                          // unique across the page
  "kind": "class-diagram|object-diagram|domain-model|context-map|component-diagram|package-diagram|deployment-diagram|c4-diagram|entity-relationship-diagram|dependency-graph|data-flow-diagram|sequence-diagram|state-machine|activity-diagram|flowchart|decision-tree|byte-field-diagram|railroad-diagram|block-diagram|tree|matrix|timeline|compare|svg",
  "title": "≤ 8 words: the point, not the topic ('Start-up claims before any lookup')",
  "caption": "md, 1–2 sentences: what to notice",
  "wide": false }                             // optional: span the full row; a drawing too wide for half of it does anyway
```

| Kind | Draw it for | Shape |
|---|---|---|
| `class-diagram` | How types relate: what implements, extends, owns, holds or uses what | A UML class diagram: classes with stereotypes, attributes and operations, and the six relationships with multiplicities and roles; drawn by the diagram kit |
| `object-diagram` | One case at one moment: a worked example, an edge case, what is on disk | A UML object diagram: named instances with their values, and the links between them; drawn by the diagram kit |
| `domain-model` | The domain a change models and its boundaries | Aggregates as boundaries around their root, entities and value objects, with services, repositories and events beside them; the kit remarks on a broken aggregate rule |
| `context-map` | Whose model sets the terms between bounded contexts | Contexts joined by upstream and downstream relationships and the pattern each side follows; the kit remarks on a strategic smell |
| `component-diagram` | The replaceable parts and the interfaces they plug together through | A UML component diagram: components, subsystems, and provided and required interfaces; drawn by the diagram kit |
| `package-diagram` | How the source is organised and which package may use which | A UML package diagram: nested folders with dependencies, imports, accesses and merges; drawn by the diagram kit |
| `deployment-diagram` | What runs or lives where, and what crosses a boundary | A UML deployment diagram: nested nodes, the artefacts on them, communication paths; drawn by the diagram kit |
| `c4-diagram` | The big picture: who uses the system, what it works with, and its containers | A C4 context or container diagram; drawn by the diagram kit |
| `entity-relationship-diagram` | What is stored and how it relates; what a migration changes | A crow's foot diagram: entities with their attributes, types and keys, and how many at each end of every relationship; drawn by the diagram kit |
| `dependency-graph` | What depends on what: modules, crates, packages, layers | Nested boxes joined by UML dependencies, with every cycle marked; drawn by the diagram kit (see "Diagrams the kit draws") |
| `data-flow-diagram` | Where data goes, what handles and keeps it, and which trust boundaries it crosses | Processes, stores and external entities joined by named flows, inside dashed trust boundaries; the kit lists every flow that crosses one; drawn by the diagram kit |
| `state-machine` | A lifecycle: the states one thing can be in and what moves it between them | A UML state machine: states with `entry`, `do` and `exit`, transitions labelled `event [guard] / action`, initial and final states, choices, composite states; drawn by the diagram kit |
| `activity-diagram` | What is done, in what order, and by whom; what runs in parallel | A UML activity diagram: actions, decisions and merges, forks and joins, in swimlanes when it has partitions; drawn by the diagram kit |
| `flowchart` | A procedure step by step, with its loops and where paths rejoin | Terminators, processes, decisions with the question inside, inputs and outputs, subprocesses; drawn by the diagram kit |
| `decision-tree` | Which conditions lead to which outcome: every case of one decision | Questions, the answers branching from each, and the outcome every path ends in; drawn by the diagram kit |
| `byte-field-diagram` | How a binary format, a wire frame or a register is laid out | Fields side by side on a scale of bits or bytes, wrapped into rows, with a broken row for a field of no fixed size; drawn by the diagram kit |
| `railroad-diagram` | What a grammar, a command line or a pattern accepts | One track per rule: terminals and references to rules joined by rail, with branches for choices, bypasses for optional parts and loops for repeats; drawn by the diagram kit |
| `block-diagram` | How a few things connect when no kind above fits: a derivation, a pipeline of transformations | Named boxes, optionally grouped, joined by labelled arrows; it has no rules of its own, so reach for it last; drawn by the diagram kit |
| `sequence-diagram` | Protocols, start-up order, lock ordering, races, retries: one scenario, in time order | A UML sequence diagram: lifelines, numbered messages with replies, activation bars, `alt`, `opt`, `loop`, `par` and `break` frames, notes; drawn by the diagram kit |
| `tree` | Layouts on disk, config trees, module trees, what a run leaves behind | An indented listing with added, removed, changed and kept states; drawn by the diagram kit |
| `matrix` | Decision tables, permission and configuration matrices, gates | A table of rows by columns, each cell a short outcome with a tone; drawn by the diagram kit |
| `timeline` | Ages, windows, timeouts, backoff, ordering in time | An axis (linear or log) with spans in lanes and labelled marks; drawn by the diagram kit |
| `compare` | Anything the PR changes shape: before on the left, after on the right | Two figures side by side |
| `svg` | The rare picture none of the above can draw | Inline SVG you write; use `currentColor` and `var(--ink)`, `var(--accent)`, `var(--bad)`, `var(--ok)` so it works in both themes |

```jsonc
// compare
{ "kind": "compare", "before": { /* any figure */ }, "after": { /* any figure */ } }

// svg
{ "kind": "svg", "svg": "<svg viewBox='0 0 600 200'>…</svg>" }
```

### Diagrams the kit draws

A figure whose `kind` is a notation one of the `draw-*` skills draws is
written in that notation's own terms, checked and laid out by the diagram
kit when the page is built, and drawn by the kit's renderer. Today those
are the kinds in the table above that say "drawn by the diagram kit".

- Write it as the notation's reference says. Each kind has a skill beside
  this one, named `draw-<kind>` (`draw-state-machine` for `state-machine`,
  `draw-flowchart` for `flowchart`, and so on). Its notation reference is
  the spec, and beside it is an example at the standard expected: for a
  class diagram, `../draw-class-diagram/reference/notation.md`. A kit
  diagram takes the same `id`, `title`, `caption` and `wide` as any
  figure.
- A `matrix` and a `tree` are drawn as a table and a listing, in HTML, so
  they reflow and scroll as text does; a `timeline` is a drawing. In a
  tree's items, write `link` where this page once took `where`.
- A sequence diagram is one scenario written as a call tree: a call holds,
  as its `steps`, what its receiver does before answering, and its `reply`
  if what comes back matters.
- Every element may carry a `status` (`new`, `changed`, `removed`,
  `existing`), a `tone` (`risk`, `ok`) and a `link`.
- A `link` is where the element lives: `path:line` opens the file,
  `chapter:<id>` opens a chapter, and a web address opens as it is. The
  build checks a `path:line` and a chapter exactly as it checks a `where`.
- Anything the kit finds wrong with a diagram is a build warning naming
  the figure, and the figure is not drawn until it is fixed.
- A dependency graph with more than about twice as many dependencies as
  modules should set `"reduce": true`.
- A class diagram makes one point about a design with the types that
  point needs; an object diagram shows one case at one moment; a domain
  model shows aggregates and what crosses their boundaries.
- An activity diagram with `partitions`, or a flowchart with `lanes`, is
  drawn in swimlanes down the page. A decision tree is written nested, as
  it is drawn.
- What the kit remarks on under a figure (a state that cannot be reached,
  a decision with nothing to decide by, a question whose answers all lead
  to one outcome, a reference into an aggregate past its root) is either a
  mistake in the figure or a finding about the code: settle which against
  the code, and raise a finding in `ownNotes` if it is true.

The build writes two diagrams for you. The Design view's diagram of how
the abstractions work together is a class diagram drawn from their
`collaborators`: each abstraction is a class with its `kind` as the
stereotype, and each collaborator's `relation` decides the line.

| `relation` | Drawn as | Say it when |
|---|---|---|
| `implements` | A dashed line to a hollow triangle | The abstraction implements the collaborator: an adapter and its port, a type and its trait |
| `extends` | A solid line to a hollow triangle | It is a more specific kind of the collaborator |
| `owns` | A filled diamond on the abstraction | The collaborator is part of it and does not outlive it |
| `holds` | A hollow diamond on the abstraction | It keeps the collaborator, which has a life of its own |
| `uses` | A dashed arrow | It calls or depends on the collaborator; the default |

Where two declarations of the same pair disagree, the first stands and the
build notes it. The module map is a dependency graph drawn from
`modmap.json`.

The build needs Node 22 to lay diagrams out. Without it, the build warns
and no kit diagram is drawn.

What makes a figure worth its space:

- **One idea per figure**, stated in its title. Two ideas make two figures.
- **The code's own names** on boxes, actors and paths, and real values on
  axes and in cells: `CLAIM_WINDOW` at 14 d, not "the window".
- **Small enough to read at a glance**: up to about 12 nodes, 8
  participants, 20 messages, 6 columns. Split anything bigger.
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

// modmap.json — ≤ 12 nodes; edges point from caller to callee; drawn as a dependency graph, a group per layer
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
`old:4`; `<path>` is `commit:<sha>:<path>` for a superseded line). Each
record carries `target`, `kind`, `status`, `note`, `label`, `where`,
`updatedAt`, the `head` its verdict was given at and, for a stop, ledger row
or file, the lines it `claimed` there. Hosted on claude.ai, records live in
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

### When the head moves

A verdict is about the code as it stood at one head. Each build keeps its
diff in `data/heads/<head>.json`, and the page carries a `lineage` from every
earlier head it finds there: for each file, which changed lines still stand
unchanged and where they are now. A verdict recorded at an earlier head then
claims only the lines that survive, at their new places, so:

- a line added or rewritten since is unread again, in every view;
- a stop, ledger row or file whose lines changed since its verdict says so
  under its control, as does a chapter any of whose files changed, and the
  overall and design verdicts when any file changed;
- a banner counts the verdicts recorded before the head moved, and **Your
  notes** marks each one;
- line marks move with their lines;
- giving the verdict again records it at the current head. Adding a note
  does not.

A commit's verdict needs none of this: its lines are traced onto whatever the
whole diff now is. A verdict from a head with no kept diff claims no lines,
and one recorded before heads were kept counts as given at the current head.
`pr.head` in `explore.json` names the head, so keep it current and never
delete `data/heads/`.
