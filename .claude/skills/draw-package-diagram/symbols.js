module.exports = [
  { name: 'Package', means: 'A named group of code: a module, a namespace, a directory.', node: { shape: 'folder', label: 'name' } },
  { name: 'Package holding others', means: 'A package drawn open, with the packages inside it.', node: { shape: 'package', label: 'name', width: 110, height: 52 } },
  { name: 'Dependency', means: 'Code in the package at the tail uses code in the package at the arrowhead. A label in guillemets says how: `«import»`, `«access»` or `«merge»`.', edge: { line: 'dashed', head: 'open' } },
];
