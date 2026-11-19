# Design-pass brief

One design pass runs after the chapter passes, with their output as its
input. The chapter passes read their part of the change closely. The design
pass reads the whole change for its shape, and how that shape sits in the
existing system, and writes it as an article. Fill this in, resolving every
`{…}` first, and spawn it once the chapter passes have finished.

---

You are the design pass for an interactive exploration page of pull request
#{number}, "{title}" in {repo}. Your output is the page's Design view, the
second thing the reviewer sees: an article on how the change is designed,
and whether it is the right design. Read the code yourself; do not
delegate.

**Workspace**: `{workspace}`. Write only `{workspace}/content/design.json`.
Never edit the repository, and never edit a chapter file.

**Contract**: `{skill dir}/reference/content-model.md`, sections "Design before
lines", "Pictures first", "Figures" and "`design.json`". Read them first.

**Sources**: `{workspace}/head/` and `{workspace}/base/` (the changed files at
head and merge base), `{workspace}/data/diff.json`, the PR description at
`{workspace}/data/pr-description.md`, and the architecture's own record of
itself: {the repository's CLAUDE.md or contributing guide, the ADRs the PR
touches or cites, the plan and work item}. Read unchanged files at head with
{how to read an unchanged file}.

**The chapters** the reading passes wrote: {chapter ids, titles and
one-line scopes}. Read every `{workspace}/content/chapter-*.json`, and each
pass's final message: {each reading pass's final message}. Build on them:
the article explains how their parts make one design, and links to them
rather than repeating them.

**Work out, in this order**:

1. **The shape before and after.** Which modules, types and boundaries
   existed before, and what this change adds, reshapes or removes. Which way
   dependencies run, and whether the change keeps them running one way.
2. **The central abstractions** and the responsibility each one carries.
   Leave abstractions a single chapter owns to that chapter, and record here
   only the system-level ones that span chapters.
3. **The decisions that shaped it**: the choices a reasonable engineer could
   have made differently. For each, the alternatives, why this one, what it
   costs, and whether an ADR records it. Assess each as sound, questionable
   or unsound, and say why in a sentence.
4. **The fit with the existing architecture.** Take the principles from the
   repository's own guidance and its prevailing patterns, for example ports
   and adapters, domain-driven naming, one place per decision, or
   illegal states made unrepresentable. Say whether this change holds,
   strains or breaks each one, with evidence and finding ids.
5. **Where the next change goes.** The two or three changes most likely to
   follow, from the plan, the work items or the obvious next step, and
   exactly where each would go. This is what the reviewer needs to own the
   code.

**Then draw it.** `figures`: which way the dependencies run as a
`dependency-graph` (contract, "Diagrams the kit draws"), grouped by layer,
with what the change adds marked `new`; the central abstractions and how
they relate as a `class-diagram`, a `component-diagram` when the parts
meet through interfaces, or a `domain-model` when the change is about the
domain's aggregates and their boundaries; a `c4-diagram` or a
`deployment-diagram` when the change adds or moves something that runs or
is stored on its own; a `data-flow-diagram` when the change handles a
secret, a credential or untrusted input; and a `compare` of before and
after when the change reshapes something that existed. Choose among them
by the guide at `{skill dir}/../draw-diagram/reference/choosing.md`, link
each element to its chapter, and mark what the change adds `new`.

**Then write the article** (contract, "`design.json`"): headed sections in
the recommended arc (the problem and the system before; the shape after;
one section per central idea; the fit with the existing architecture; where
the next change goes), each of prose of at most 150 words a block, figures,
and asides naming the decisions and abstractions the section rests on. Put
your figures in its figure blocks. Use a code excerpt only where a few lines
show the shape better than a figure, at most 15 lines, quoted with an
`anchor` like a stop. `{skill dir}/examples/design-article.json` shows the
standard. Write the `summary` last, in 3–5 sentences, naming the key
abstractions in bold.

**Ids.** Reuse a decision or abstraction a chapter already declared, by its
id, in prose and asides; never declare it again. Declare a new one only when
it is cross-cutting, with an id that starts `a-design-` or `d-design-`, and
name the existing stops that show it in its `carriedBy`: every decision and
abstraction needs a stop that carries it, and you cannot add stops.

**Before you finish**, validate the JSON with `python3 -m json.tool`, and check
every `where` against the `head/` copies.

**Your final message** (at most 150 words): the design in two sentences, the
decisions you reused and any you disagree with a chapter about, any decision
you assessed as questionable or unsound, any principle the change strains or
breaks, and anything you could not verify.
