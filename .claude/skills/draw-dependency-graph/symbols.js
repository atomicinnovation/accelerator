module.exports = [
  { name: 'Module', means: 'A unit of code: a crate, a package, a module, a layer.', node: { shape: 'box', label: 'name' } },
  { name: 'Module holding others', means: 'A module drawn open, with the modules inside it.', node: { shape: 'group', label: 'name', width: 100, height: 44 } },
  { name: 'Depends on', means: 'The module at the tail cannot be built or run without the module at the arrowhead. One marked as a risk is part of a cycle.', edge: { line: 'dashed', head: 'open' } },
];
