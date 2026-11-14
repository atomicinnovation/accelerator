(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { SHAPES } = inNode ? require('./shapes.js') : root.DiagramKit;
  const { svgOf } = inNode ? require('./render.js') : root.DiagramKit;
  const { escaped, inlineOf } = inNode ? require('./html.js') : root.DiagramKit;

  const EDGE_SAMPLE_LENGTH = 56, EDGE_SAMPLE_HEIGHT = 20;
  const TRACING = 'Press a box to trace its arrows, those it sends <span class="dk-key trace-out"></span> and those it receives <span class="dk-key trace-in"></span>. Press it again to drop it, or Esc to clear.';
  const OPENING = '↗ on a box opens where it is in the code. So does a name or a label shown as a link.';
  const OPENING_TEXT = 'A name shown as a link opens where it is in the code.';

  const drawnAs = edge => [edge.line || 'solid', edge.head || 'none', edge.tail || 'none'].join('/');
  const everyNode = nodes => (nodes || []).flatMap(node => [node, ...everyNode(node.children)]);

  function isUsed(symbol, spec, scene) {
    if (symbol.usedIn) return Boolean(symbol.usedIn(spec));
    if (!scene) return false;
    if (symbol.node) return scene.nodes.some(node => node.shape === symbol.node.shape && (!symbol.uses || symbol.uses(node)));
    return scene.edges.some(edge => (symbol.uses ? symbol.uses(edge) : drawnAs(edge) === drawnAs(symbol.edge)));
  }

  function sampleOf(symbol) {
    if (symbol.html) return symbol.html;
    if (symbol.edge) {
      const edge = { id: 'sample', from: 'a', to: 'b', points: [[2, EDGE_SAMPLE_HEIGHT / 2], [EDGE_SAMPLE_LENGTH - 2, EDGE_SAMPLE_HEIGHT / 2]], ...symbol.edge };
      return svgOf({ width: EDGE_SAMPLE_LENGTH, height: EDGE_SAMPLE_HEIGHT, nodes: [], edges: [edge] }, { sample: true });
    }
    const shape = SHAPES[symbol.node.shape];
    const size = symbol.node.width != null ? { width: symbol.node.width, height: symbol.node.height } : shape.sizeOf(symbol.node);
    const node = { id: 'sample', inert: true, x: symbol.node.x ?? 1, y: symbol.node.y ?? 1, ...size, ...symbol.node, width: size.width, height: size.height };
    return svgOf({ width: (symbol.node.x ?? 1) + size.width + 1 + (symbol.node.room ?? 0), height: (symbol.node.y ?? 1) + size.height + 1, nodes: [node], edges: [] }, { sample: true });
  }

  function usageOf(spec, scene) {
    const marked = scene ? [...scene.nodes, ...scene.edges] : [];
    if (!scene) return /"link":/.test(JSON.stringify(spec)) ? [OPENING_TEXT] : [];
    return [...(scene.edges.length ? [TRACING] : []), ...(marked.some(item => item.link) ? [OPENING] : [])];
  }

  function symbolsOf(notation, spec, scene) {
    const symbols = notation.symbols || [];
    if (!symbols.length) return '';
    const placed = scene ? { nodes: everyNode(scene.nodes), edges: scene.edges } : null;
    const entry = (symbol, state) => `<li class="dk-${state}"><span class="dk-sample">${sampleOf(symbol)}</span><span class="dk-meaning"><b>${escaped(symbol.name)}</b> ${inlineOf(symbol.means)}</span></li>`;
    const used = symbols.filter(symbol => isUsed(symbol, spec, placed)), unused = symbols.filter(symbol => !used.includes(symbol));
    const more = unused.length ? `<label class="dk-more"><input type="checkbox"> Show the ${unused.length} ${unused.length === 1 ? 'symbol' : 'symbols'} this diagram does not use</label>` : '';
    return `<details class="dk-symbols"><summary>Legend <span class="dk-notation">${escaped(notation.name || notation.kind)}</span></summary><ul>${used.map(symbol => entry(symbol, 'used')).join('')}${unused.map(symbol => entry(symbol, 'unused')).join('')}</ul>${more}`
      + `${usageOf(spec, placed).map(line => `<p class="dk-usage">${line}</p>`).join('')}</details>`;
  }

  const api = { symbolsOf, isUsed, sampleOf };
  if (inNode) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, { symbolLegendOf: symbolsOf });
})(typeof window !== 'undefined' ? window : globalThis);
