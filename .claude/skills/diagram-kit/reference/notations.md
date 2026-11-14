# Adding a notation

A notation is one `draw-<type>` skill beside this kit. The kit loads
`draw-<type>/notation.js` from every such skill.

## What a notation exports

```js
const { markProblems } = require('../diagram-kit/lib/notations.js');

module.exports = {
  kind: 'dependency-graph',        // the spec's "kind"
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
