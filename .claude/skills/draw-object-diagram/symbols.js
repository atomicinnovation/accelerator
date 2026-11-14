module.exports = [
  { name: 'Object', means: 'One instance at one moment. Its underlined heading is its name and type, and its values are listed under it.', node: { shape: 'classifier', underlined: true, label: 'name: Type', compartments: [['field = value']] } },
  { name: 'Link', means: 'One object holds or refers to the other at that moment.', edge: { line: 'solid' } },
];
