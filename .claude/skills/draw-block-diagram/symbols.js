module.exports = [
  { name: 'Block', means: 'Whatever its name says: this kind of diagram gives a box no meaning of its own.', node: { shape: 'box', label: 'name' } },
  { name: 'Group', means: 'Blocks that belong together under one name.', node: { shape: 'group', label: 'name', width: 100, height: 44 } },
  { name: 'Arrow', means: 'Whatever its label or the caption says it means.', edge: { line: 'solid', head: 'filled' } },
  { name: 'Dashed arrow', means: 'A weaker or indirect link of the same kind.', edge: { line: 'dashed', head: 'filled' } },
];
