---
name: diagram-kit
description: Shared library behind the draw-* diagram skills — measures text,
  lays a diagram out, draws it as themed, interactive SVG and checks the
  result in a browser. Read it when adding a diagram notation, embedding
  diagrams in a page, or changing how diagrams are laid out, drawn or
  checked. It draws nothing by itself — to draw a diagram, use the draw-*
  skill for that diagram's type.
user-invocable: false
---

# Diagram kit

Every `draw-*` skill describes one notation. This kit is everything they
share, so that a class diagram and a dependency graph are measured, laid
out, themed, linked and checked the same way, and so a page that embeds
diagrams embeds one renderer.

**Kit directory**: `${CLAUDE_SKILL_DIR}`

## How a diagram is made

```text
spec (JSON, in the notation's own terms)
  → check     the envelope, then the notation's own rules
  → scene     boxes, groups and arrows, as the notation compiles them
  → layout    elkjs places every node and routes every edge   (Node, at build time)
  → drawing   SVG with classes, data attributes and links      (Node or the browser)
```

- A **spec** is what an author writes: `id`, `kind`, `title`, an optional
  `caption`, and the notation's own fields. The `kind` picks the notation.
- A **scene** is what the kit lays out and draws: nodes (each a shape from
  the kit's shape library, possibly holding other nodes) and edges (each a
  line with a head). A notation's whole job is to check a spec and turn it
  into a scene.
- **Layout runs in Node**, never in the reader's browser, so a page carries
  no layout engine and a diagram cannot be laid out differently twice. The
  engine is a vendored, pinned elkjs (`vendor/README.md`).
- A **laid-out scene** is plain JSON: every node's place and size, every
  edge's points. A page may store it and draw it later with the renderer.

Three things hold for every element of every notation:

- a **status**, `new`, `changed`, `removed` or `existing`: what the change
  under review did to it;
- a **tone**, `risk` or `ok`: where it can go wrong, or what makes it safe;
- a **link**, any text. The page drawing the diagram decides where a link
  leads; alone, the kit opens web addresses and shows anything else on hover.

## Commands

```bash
node ${CLAUDE_SKILL_DIR}/bin/diagram.js check SPEC.json
node ${CLAUDE_SKILL_DIR}/bin/diagram.js layout SPEC.json --out SCENE.json
node ${CLAUDE_SKILL_DIR}/bin/diagram.js render SPEC.json --out PAGE.html   # or DRAWING.svg
node ${CLAUDE_SKILL_DIR}/tools/check-page.js PAGE.html --shots DIR
node --test "${CLAUDE_SKILL_DIR}/tests/*.test.js"
```

- `check` prints a `warning:` for every problem and exits 1 if there is one.
  Nothing is laid out or drawn from a spec that has a problem.
- `render` to `.html` writes one page holding every diagram in the spec file,
  with the fonts embedded: it opens offline and asks nothing of the network.
- `check-page.js` opens a page in headless Chrome and reports, per drawing,
  text outside its box, boxes across each other, an arrow across a box that
  is not its own, and whether pressing a box traces its arrows. Run it on
  every page before it is delivered, and look at the pictures it keeps.

The command line finds notations in the `draw-*` skills beside this kit.

## Working on the kit

- `reference/notations.md`: how to add a notation, the scene model, the
  shape library, and what each new notation must come with.
- `reference/embedding.md`: how a page embeds diagrams and resolves links.
- `vendor/README.md`: what is vendored, at which version, and how to
  replace it. After changing a font, run `node tools/measure-fonts.js`:
  layout sizes every box from the table that script writes.

Write a failing test first. The kit's modules are plain scripts that load
both with `require` and in a page, so a module may use nothing that only
one of them has; `lib/page.js`, `lib/browser.js` and `lib/index.js` are the
exceptions, and run in Node only.
