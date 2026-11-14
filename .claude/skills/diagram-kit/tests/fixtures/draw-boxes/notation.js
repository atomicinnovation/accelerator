const { markProblems } = require('../../../lib/notations.js');

module.exports = {
  kind: 'boxes',
  problemsIn: spec => (spec.boxes || []).flatMap(box => markProblems(box, `box ${box.id}`)),
  sceneOf: spec => ({
    nodes: spec.boxes.map(box => ({ ...box, shape: 'box' })),
    edges: spec.boxes.slice(1).map((box, i) => ({ id: `${spec.boxes[i].id}->${box.id}`, from: spec.boxes[i].id, to: box.id, head: 'open', line: 'dashed' })),
  }),
  remarksOn: spec => (spec.boxes.length > 2 ? ['More than two boxes.'] : []),
};
