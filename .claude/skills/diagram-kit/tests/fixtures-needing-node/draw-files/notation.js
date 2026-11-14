const fs = require('node:fs');

module.exports = { kind: 'files', problemsIn: () => [], sceneOf: () => ({ nodes: fs.readdirSync('.').map(id => ({ id, shape: 'box' })), edges: [] }) };
