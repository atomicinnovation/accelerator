module.exports = [
  { name: 'Person', means: 'Someone who uses the system.', node: { shape: 'person', label: 'Name', type: 'Person' } },
  { name: 'Software system or container', means: 'A system, or one of the applications and stores it is made of. The bracket says which, and with what it is built.', node: { shape: 'element', label: 'Name', type: 'Container: Rust' }, uses: node => !node.muted },
  { name: 'External', means: 'A person or system outside what is being described, shown greyed.', node: { shape: 'element', label: 'Name', type: 'Software System', muted: true }, uses: node => Boolean(node.muted) },
  { name: 'System boundary', means: 'A system opened up to show the containers inside it.', node: { shape: 'boundary', stereotype: 'software system', label: 'Name', width: 170, height: 44 } },
  { name: 'Relationship', means: 'The one at the tail uses the one at the arrowhead, for the purpose on the label and by the technology in brackets.', edge: { line: 'solid', head: 'open' } },
];
