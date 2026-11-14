# Adding a notation

A notation is one `draw-<type>` skill beside this kit. The kit loads
`draw-<type>/notation.js` from every such skill.

Place each new notation in the guide to choosing a diagram, which
`draw-diagram` keeps: the question it answers, and how to tell it from
its neighbours. A test fails until the guide places every kind.

## What a notation exports

```js
const { markProblems } = require('../diagram-kit/lib/notations.js');

module.exports = {
  kind: 'dependency-graph',        // the spec's "kind"
  name: 'Dependency graph',        // what a reader calls it; it heads the legend
  problemsIn(spec) { return []; }, // every problem, as text; never throws on purpose
  sceneOf(spec) { return scene; }, // only ever called with a spec that has no problems
  remarksOn(spec) { return []; },  // optional: findings shown under the figure
};
```

- **`problemsIn`** names the item each problem is about (`module a: id is used
  twice`). The kit puts the diagram's id in front and checks the envelope
  itself. Use `markProblems(item, where)` for `status`, `tone` and `link`.
- **`sceneOf`** is a pure function of the spec. Do no measuring or placing in
  it: say which shape each thing is and the kit does the rest.
- **`remarksOn`** is for what a reviewer should not have to find by eye: a
  cycle, an unreachable state, a relationship that breaks a rule.

## Saying what the symbols mean

A notation lists every symbol it can draw, in `symbols.js` beside it:

```js
module.exports = [
  { name: 'Process', means: 'Something that reads, changes or forwards data.', node: { shape: 'bubble', label: 'Check it' } },
  { name: 'Data flow', means: 'Data moves from the tail to the arrowhead.', edge: { line: 'solid', head: 'filled' } },
  { name: 'External', means: 'Outside what is described.', node: { shape: 'element', label: 'Name', muted: true }, uses: node => Boolean(node.muted) },
];
```

- `node` or `edge` is a sample the kit draws with its own renderer, so
  the legend cannot differ from the drawing. A shape that holds others, or
  has no size of its own, is given a `width` and `height`.
- `means` is a sentence: what the symbol says in this notation, not how it
  looks.
- A symbol is shown as used when the diagram has a node of that shape, or
  an edge with that line, head and tail. Give `uses` to say otherwise:
  which nodes of the shape, or which edges, it stands for.
- A notation that draws itself gives `html` for the sample and
  `usedIn(spec)`.

A legend is drawn only when asked for (`--legend`, or `legend: true` to
`figureOf`). It is closed until opened, shows the symbols the diagram
uses, keeps the rest behind a switch, and ends with how to use the
diagram. Tests fail for a notation with no symbols, a shape it draws that
no symbol explains, or anything in its example that none does.

## Running in a page

`bin/diagram.js bundle` carries every notation into a browser, with the
modules it requires. Require only the kit's `lib/` modules and files
beside the notation; a notation that requires anything else (`node:fs`, a
package) cannot be carried, and the bundle refuses it by name.

## A notation that draws itself

A notation whose figure is better as HTML than as a laid-out drawing (a
table that should reflow, a listing of text) exports `htmlOf` in place of
`sceneOf`:

```js
htmlOf(spec, { resolveLink }) { return '<div class="dk-table">…</div>'; }
```

- The kit gives it no scene and no layout: it returns the figure's markup,
  styled from the kit's tokens in `lib/diagram.css`.
- `resolveLink(link)` returns `{ href, label, attributes }` or nothing;
  wrap what opens the link with `linked` from `lib/html.js`, and make text
  safe with `escaped` or `inlineOf`.
- Write it to run in a browser as well as in Node, as `draw-matrix` does,
  so a page can draw one from data it computes. In a browser it adds
  itself to `DiagramKit.NOTATIONS`.
- `bin/diagram.js layout` and rendering to `.svg` refuse it; rendering to
  `.html` and `prepare` take it.

## The scene

```jsonc
{
  "direction": "down",                       // or "right"
  "nodes": [{
    "id": "prune",                           // unique in the scene
    "shape": "box",                          // from the shape library
    "label": "prune", "detail": "removes stale trees",
    "status": "new", "tone": "risk", "link": "src/prune.rs:540",
    "children": [ /* only for a shape that holds others */ ]
  }],
  "edges": [{
    "id": "resolve->prune", "from": "resolve", "to": "prune",
    "label": "on a cold start",
    "line": "dashed",                        // or "solid"
    "head": "open",                          // "open", "filled" or "none"
    "status": "new", "tone": "risk", "link": "…"
  }]
}
```

An edge may join nodes at any depth, and may end on a node that holds
others.

## The shape library (`lib/shapes.js`, drawn in `lib/render.js`)

| Shape | Holds others | Draws |
|---|---|---|
| `box` | no | A rounded rectangle with a name and an optional detail line |
| `group` | yes | A titled frame around its children |

A notation that needs a new shape adds it to the library, with its size in
`lib/shapes.js`, its drawing in `lib/render.js`, its styles in
`lib/diagram.css`, and tests for all three. Shapes belong to the kit, never
to one notation: a second notation will want them.

Text is sized from `lib/text.js` roles (`name`, `title`, `detail`, `edge`,
`heading`). A new role needs its face in the font table; a new face needs
the font vendored and `tools/measure-fonts.js` run.

## What every notation comes with

- `SKILL.md`: when to use it, and how to draw one from a request alone.
- `reference/notation.md`: the spec, field by field; which parts of the
  standard it draws and which it leaves out; the conventions a reader of
  the standard expects.
- `examples/*.json`: at least one diagram at the standard expected, which
  the tests check and lay out.
- `tests/`: `node:test` suites for `problemsIn`, `sceneOf` and `remarksOn`.
- A page of its examples that passes `tools/check-page.js`.

Draw only what is correct in the standard. A construct the notation cannot
draw correctly is left out and listed in `reference/notation.md`, never
approximated.
