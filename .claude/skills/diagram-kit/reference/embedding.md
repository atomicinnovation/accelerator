# Embedding diagrams in a page

A page that shows diagrams lays them out when it is built and draws them
when it is read.

## At build time (Node)

```js
const { kitWith } = require('<kit>/lib/index.js');
const kit = kitWith();                      // the draw-* skills beside the kit

const problems = kit.problemsIn(spec);      // report these; do not go on if any
const scene = await kit.laidOut(spec);      // plain JSON: store it with the page
const figure = kit.figureOf(spec, scene);   // or draw it now, as HTML
```

From another language, run `bin/diagram.js layout SPEC.json`, which prints
the same laid-out scene.

## In the page

Inline, in this order, `lib/font-metrics.js`, `lib/text.js`,
`lib/shapes.js`, `lib/render.js`, `lib/layout.js`, `lib/notations.js` and
`lib/trace.js`, and include `lib/diagram.css`. They add their functions to
`window.DiagramKit`. Then:

```js
const figure = DiagramKit.svgOf(scene, { uid: spec.id, title: spec.title, resolveLink });
DiagramKit.attachTracing(document);         // once per page
```

The page supplies the colours: `lib/diagram.css` uses the custom properties
that `lib/tokens.css` defines (`--panel`, `--ink`, `--rule`, `--accent`,
`--ok`, `--warn`, `--bad` and the three font stacks). A page with its own
theme defines those and leaves `tokens.css` out.

Boxes are sized for IBM Plex Sans and JetBrains Mono. A page must load
both, or text will not fit: embed `vendor/fonts/` as `lib/page.js` does, or
load them from a font host.

## Links

Every element may carry a `link`, which is any text. `resolveLink(link)`
returns `{ href, label, attributes }` for a link the page can open, or
`null` for one it cannot, which is then shown on hover. `attributes` are set
on the anchor, so a page can open a link its own way:

```js
const resolveLink = link => {
  const place = /^(.+):(\d+)$/.exec(link);
  return place ? { href: '#files', label: `Open ${link}`, attributes: { 'data-open': place[1], 'data-line': place[2] } } : null;
};
```

## Drawing in the browser from computed state

A page that draws diagrams from state it computes while open, such as a
simulator, must also load `vendor/elkjs/elk.bundled.js` and call
`DiagramKit.laidOut(scene)`. That copies the layout engine into the page,
so do it only for a page that needs it.
