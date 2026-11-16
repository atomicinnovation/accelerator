const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'where-it-runs', kind: 'deployment-diagram', title: 'The launcher and its cache live on the developer\'s machine',
  nodes: [
    { id: 'laptop', name: 'developer machine', stereotype: 'device', status: 'changed', nodes: [
      { id: 'shell', name: 'shell', stereotype: 'execution environment', artifacts: [
        { id: 'launcher', name: 'accelerator', link: 'bin/accelerator:1' },
        { id: 'vcs', name: 'accelerator-vcs', status: 'new', tone: 'risk' },
      ] },
      { id: 'cache', name: '~/.cache/accelerator', artifacts: [{ id: 'claims', name: 'claims/', stereotype: 'directory', status: 'new' }] },
    ] },
    { id: 'cdn', name: 'release host', stereotype: 'device' },
  ],
  paths: [{ from: 'laptop', to: 'cdn', label: 'HTTPS', link: 'src/fetch.rs:10' }, { from: 'shell', to: 'cache', label: 'file system', status: 'new' }],
  dependencies: [{ from: 'launcher', to: 'vcs', label: 'runs' }, { from: 'launcher', to: 'claims', label: 'refreshes', status: 'new' }],
};
const scene = notation.sceneOf(SPEC);
const flat = nodes => nodes.flatMap(each => [each, ...flat(each.children || [])]);
const node = id => flat(scene.nodes).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });

test('it draws diagrams of kind deployment-diagram', () => {
  assert.equal(notation.kind, 'deployment-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a node that holds nodes or artefacts is a box with depth around them, and one that holds nothing a small one', () => {
  assert.deepEqual([node('laptop').shape, node('laptop').stereotype, node('laptop').label, node('laptop').status], ['cuboid', 'device', 'developer machine', 'changed']);
  assert.deepEqual(node('laptop').children.map(each => each.id), ['shell', 'cache']);
  assert.deepEqual(node('shell').children.map(each => each.id), ['launcher', 'vcs']);
  assert.deepEqual([node('cdn').shape, node('cdn').stereotype], ['cuboidbox', 'device']);
  assert.equal(node('cache').stereotype, undefined);
});

test('an artefact is a box marked «artifact», or with the stereotype it gives', () => {
  assert.deepEqual([node('launcher').shape, node('launcher').stereotype, node('launcher').label, node('launcher').link], ['classifier', 'artifact', 'accelerator', 'bin/accelerator:1']);
  assert.deepEqual([node('claims').stereotype, node('vcs').status, node('vcs').tone], ['directory', 'new', 'risk']);
});

test('a communication path is a plain line between nodes, saying how they talk', () => {
  const path = edge('laptop--cdn');
  assert.deepEqual([path.from, path.to, path.line, path.head, path.label, path.link], ['laptop', 'cdn', 'solid', 'none', 'HTTPS', 'src/fetch.rs:10']);
});

test('a dependency between artefacts is a dashed arrow', () => {
  const uses = edge('launcher->claims');
  assert.deepEqual([uses.line, uses.head, uses.label, uses.status], ['dashed', 'open', 'refreshes', 'new']);
});

test('a diagram needs nodes, and every node and artefact an id of its own', () => {
  assert.deepEqual(problems({ nodes: [] }), ['no nodes']);
  assert.deepEqual(problems({ nodes: [{ name: 'x' }, { id: 'a', artifacts: [{ id: 'a' }, { name: 'y' }] }, { id: 'b', stereotype: 5, status: 'fresh' }], paths: [], dependencies: [], direction: 'up' }), [
    'direction "up" is not one of down, right',
    'node 1: no id',
    'node a: artefact a: id is used twice',
    'node a: artefact 2: no id',
    'node b: stereotype must be text',
    'node b: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('a path joins nodes the diagram has, and a dependency joins artefacts', () => {
  assert.deepEqual(problems({ paths: [{ from: 'laptop', to: 'nowhere' }, { to: 'cdn' }, { from: 'laptop', to: 'launcher' }, { from: 'shell', to: 'cdn' }, { from: 'cdn', to: 'shell' }], dependencies: [{ from: 'launcher', to: 'cdn' }, { from: 'launcher', to: 'ghost' }, { from: 'launcher', to: 'vcs' }, { from: 'launcher', to: 'vcs', tone: 'scary' }] }), [
    'path laptop--nowhere: no node "nowhere"',
    'path 2: no from',
    'path laptop--launcher: a communication path joins nodes, and launcher is an artefact',
    'path cdn--shell: written twice',
    'dependency launcher->cdn: a dependency joins artefacts, and cdn is a node',
    'dependency launcher->ghost: no artefact "ghost"',
    'dependency launcher->vcs: written twice',
  ]);
});

test('artefacts on different nodes that depend on each other with no path between their nodes are remarked on', () => {
  const nodes = [...SPEC.nodes.slice(0, 1), { id: 'cdn', name: 'release host', artifacts: [{ id: 'index', name: 'index.json' }] }, { id: 'ci', name: 'CI runner', artifacts: [{ id: 'build', name: 'build.sh' }] }];
  const dependencies = [{ from: 'launcher', to: 'index' }, { from: 'build', to: 'index' }, { from: 'launcher', to: 'claims' }];
  assert.deepEqual(remarks({ nodes, dependencies }), ['build.sh on CI runner depends on index.json on release host, but no communication path joins them']);
});

test('a node with nothing on it and joined to nothing is remarked on', () => {
  assert.deepEqual(remarks({ nodes: [...SPEC.nodes, { id: 'spare', name: 'spare host' }] }), ['spare host has nothing deployed on it and is joined to nothing']);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.deepEqual(kit.remarksOn(example), []);
    assert.equal((await kit.laidOut(example)).edges.every(each => each.points.length >= 2), true);
  });
}
