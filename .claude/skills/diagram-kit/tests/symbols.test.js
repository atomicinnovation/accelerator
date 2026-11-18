const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const { kitWith } = require('../lib/index.js');
const { sampleOf, isUsed } = require('../lib/legend.js');
const { SHAPES } = require('../lib/shapes.js');

const SKILLS = path.resolve(__dirname, '..', '..');
const skills = fs.readdirSync(SKILLS).filter(name => name.startsWith('draw-') && fs.existsSync(path.join(SKILLS, name, 'notation.js')));
const kit = kitWith();
const everyNode = nodes => (nodes || []).flatMap(node => [node, ...everyNode(node.children)]);
const examplesOf = skill => fs.readdirSync(path.join(SKILLS, skill, 'examples')).map(file => JSON.parse(fs.readFileSync(path.join(SKILLS, skill, 'examples', file), 'utf8')));
const told = item => `${item.shape || [item.line || 'solid', item.head || 'none', item.tail || 'none'].join('/')} (${item.id})`;

function shapesNamedIn(skill) {
  const sources = fs.readdirSync(path.join(SKILLS, skill)).filter(file => file.endsWith('.js')).map(file => fs.readFileSync(path.join(SKILLS, skill, file), 'utf8')).join('\n');
  const said = [...sources.matchAll(/\bshape: [^,}]*/g), ...sources.matchAll(/^const SHAPES = .*$/gm)].map(match => match[0]);
  return [...new Set(said.flatMap(text => [...text.matchAll(/'([a-z]+)'/g)].map(match => match[1])).filter(word => SHAPES[word]))];
}

for (const skill of skills) {
  const notation = require(path.join(SKILLS, skill, 'notation.js'));
  const symbols = notation.symbols || [];

  test(`${skill} says what each of its symbols means`, () => {
    assert.ok(symbols.length > 0, 'no symbols');
    assert.match(notation.name || '', /^[A-Z].{3,}/, 'no name to head its legend');
    for (const symbol of symbols) {
      assert.ok(symbol.name && symbol.means, `a symbol has no name or no meaning: ${JSON.stringify(symbol)}`);
      assert.equal(['node', 'edge', 'html'].filter(kind => symbol[kind]).length, 1, `${symbol.name} is not exactly one of a node, an edge or markup`);
      assert.match(symbol.means, /[.]$/, `${symbol.name}: its meaning is not a sentence`);
      assert.ok(sampleOf(symbol).length > 0, symbol.name);
    }
    assert.equal(new Set(symbols.map(symbol => symbol.name)).size, symbols.length, 'two symbols share a name');
  });

  test(`${skill} explains every shape it can draw`, () => {
    const explained = new Set(symbols.filter(symbol => symbol.node).map(symbol => symbol.node.shape));

    assert.deepEqual(shapesNamedIn(skill).filter(shape => !explained.has(shape)), []);
  });

  for (const example of examplesOf(skill)) {
    test(`${skill} explains everything ${example.id} draws`, async () => {
      const scene = await kit.laidOut(example);
      if (!scene) { assert.ok(symbols.some(symbol => isUsed(symbol, example, null)), 'no symbol is used'); return; }
      const placed = { nodes: everyNode(scene.nodes), edges: scene.edges };
      const explains = (item, kind) => symbols.some(symbol => symbol[kind] && isUsed(symbol, example, kind === 'node' ? { nodes: [item], edges: [] } : { nodes: [], edges: [item] }));

      assert.deepEqual(placed.nodes.filter(node => !explains(node, 'node')).map(told), []);
      assert.deepEqual(placed.edges.filter(edge => !explains(edge, 'edge')).map(told), []);
    });
  }
}
