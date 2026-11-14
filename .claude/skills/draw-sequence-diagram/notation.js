const { problemsIn, remarksOn } = require('./checks.js');
const { placedSceneOf } = require('./layout.js');

module.exports = { kind: 'sequence-diagram', symbols: require('./symbols.js'), problemsIn, remarksOn, placedSceneOf };
