const { markProblems } = require('../../../lib/notations.js');

module.exports = {
  kind: 'boxes',
  name: 'Box diagram',
  problemsIn: spec => (spec.boxes || []).flatMap(box => markProblems(box, `box ${box.id}`)),
  sceneOf: spec => ({
    nodes: spec.boxes.map(box => ({ ...box, shape: 'box' })),
    edges: spec.boxes.slice(1).map((box, i) => ({ id: `${spec.boxes[i].id}->${box.id}`, from: spec.boxes[i].id, to: box.id, head: 'open', line: 'dashed' })),
  }),
  remarksOn: spec => (spec.boxes.length > 2 ? ['More than two boxes.'] : []),
  symbols: [
    { name: 'Box', means: 'One thing.', node: { shape: 'box', label: 'name' } },
    { name: 'Follows', means: 'The box at the tail comes before the one at the head.', edge: { line: 'dashed', head: 'open' } },
    { name: 'Group', means: 'Boxes that belong together.', node: { shape: 'group', label: 'name', width: 80, height: 44 } },
    { name: 'Owns', means: 'The box at the diamond owns the other.', edge: { line: 'solid', tail: 'filled-diamond' } },
  ],
};
