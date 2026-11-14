# Custom views

Pictures carry this page. The figure kinds in `reference/content-model.md`
(dependency graph, sequence diagram, tree, matrix, timeline,
comparison) draw most of what a PR
needs, and every chapter opens with them. A custom view is the next step up:
a picture the reviewer can operate. What makes a page worth opening is usually
a view only this PR could have: a simulator of its decision procedure, a
stepper through its protocol, a state machine, a before/after explorer for its
data format. Build one or two per PR when the change has a mechanism a
reviewer must otherwise model in their head. Do not build one for a change
that is mostly wiring; static figures serve it better.

## Choosing one

| The PR changes… | A view that helps |
|---|---|
| A decision procedure (what to keep, retry, allow, route) | A simulator: inputs on one side, every item's fate and the rule that decided it on the other |
| A protocol or lifecycle across processes | A sequence stepper: actors as columns, events in order, with the lock or state held at each step |
| A state machine | An interactive state diagram: click a transition, see the guard and the code |
| A data format, schema or migration | A before/after explorer: a sample record, transformed step by step |
| Configuration, flags or a permission matrix | A matrix: every combination, the resulting behaviour, the code that decides it |
| Performance-sensitive paths | A chart of the measurements the PR records, with the budget marked |

Agree the choice with the user before building, as part of agreeing the
page's shape.

## The extension API

A view is a script in `<workspace>/views/`. The build inlines it after the
page's own script. It registers itself:

```js
(() => {
  const ExplorePR = window.ExplorePR;
  if (!ExplorePR) return;
  ExplorePR.registerView({
    id: 'retry-sim',                // the hash route: letters, digits, hyphens
    title: 'Retry simulator',       // rail label
    group: 'Read',                  // Start | Read | Decide | Reference
    render(api) { return `<div class="view-head">…</div>…`; },  // HTML string
    mount(root, api) { /* bind events after the HTML is in the page */ },
  });
})();
```

`api` (also `ExplorePR.api`) exposes the page's own building blocks, so a
view looks and behaves like the rest of the page:

| Member | Use |
|---|---|
| `content`, `diff` | The assembled content model and the diff (`diff` may be null while it loads) |
| `figure(spec)` | A tree, matrix or timeline from the content model, rendered as HTML, so a view can draw one from the state it computes. A diagram the kit draws (a dependency graph, a sequence diagram) is laid out when the page is built, so a view cannot yet draw one from state it computes in the browser |
| `md(text, inline)`, `esc(text)` | Markdown and HTML escaping |
| `codeSlice(path, start, end, side)` | A highlighted diff slice with links to the file view |
| `locBtn(path, line)`, `whereBtn('path:line')`, `testRef('path::name')` | Links that open the file view at a line |
| `sevChip(level)`, `fmt(n)`, `plural(n, word)` | Chips and numbers in the page's style |
| `ctl(target, kind, meta)` | A review control whose verdict and note are saved and appear in Your notes |
| `store` | The review store (`store.get(target)`) |
| `rerender()`, `openFile(path, line)` | Navigation |

`ExplorePR.registerKind(name, { label, opts: [[value, label, tone]] })` adds a
review-control vocabulary (tones: `ok`, `warn`, `bad`, `info`, `none`).

Use the page's CSS classes: `view-head`, `eyebrow`, `lede`, `panel`,
`section`, `stack`, `row`, `grid-2`, `chip c-ok|c-warn|c-bad|c-info`, `btn`,
`btn primary`, `field`, `table-wrap` with `table.data`, `listing`, `report`,
`ruler`. Colours come from the page's tokens (`var(--ok)`, `var(--bad)`,
`var(--accent)`, `var(--ink-3)`), so a view works in light and dark themes.
The shell is a starting point, not a cage. When a view needs more, copy
`assets/page.html` to `<workspace>/page.html` and change it; the build uses the
workspace copy.

## Making a model trustworthy

A simulator that disagrees with the code is worse than none. The PR 170 one
earned its place this way:

1. **Specify before building.** A reading pass writes `content/specs/<name>.md`
   from the code: every constant with its value and source line, the input
   space, the decision rules as tables, the order of effects, the report, and
   named scenarios with expected outcomes (`examples/prune-simulator-spec.md`).
2. **Keep the engine pure.** Separate `run(state) → decisions` from the DOM and
   export it (`window.__simEngine` in `examples/prune-simulator.js`), so node
   can test it without a browser.
3. **Test it against the scenarios.** Every scenario's expected outcome is an
   assertion. When one fails, read the code before changing either side. In
   PR 170 the engine was right and the spec had missed that removing the last
   tree also orphans its lock (`tree/prune.rs:540`, `:569`); the scenario was
   corrected, not the engine.
4. **Show the reasoning.** Every decision carries the rule that made it, a
   source line that opens the file view, and a trace of the checks. Each
   scenario states what the code should do and whether the model agrees.
5. **Say what it is.** The view's lede says it is a model, and that the code
   wins where they differ.

`examples/prune-simulator.js` is the complete PR 170 simulator: a pure engine,
13 scenarios, editable inputs, a claim-age ruler on a log scale, per-entry
traces, and "keep the result" to step time forward. Read it as a pattern, not
a template; another PR's view will look different.
