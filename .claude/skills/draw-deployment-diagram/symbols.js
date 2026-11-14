module.exports = [
  { name: 'Node', means: 'Somewhere things run or are stored: a machine, a process, a directory. Its stereotype says which.', node: { shape: 'cuboidbox', label: 'name', stereotype: 'device' } },
  { name: 'Node holding others', means: 'A node drawn open, with the nodes and artefacts on it.', node: { shape: 'cuboid', label: 'name', stereotype: 'device', width: 130, height: 56 } },
  { name: 'Artefact', means: 'A file that is deployed: a binary, a script, a configuration file.', node: { shape: 'classifier', label: 'name', stereotype: 'artifact' } },
  { name: 'Communication path', means: 'The two nodes exchange data.', edge: { line: 'solid' } },
  { name: 'Dependency', means: 'The artefact at the tail needs the one at the arrowhead.', edge: { line: 'dashed', head: 'open' } },
];
