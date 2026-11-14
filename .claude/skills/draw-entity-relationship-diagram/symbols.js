module.exports = [
  { name: 'Entity', means: 'A table: its columns with their types, marked `PK` for the primary key, `FK` for a foreign key and `UK` for a unique one.', node: { shape: 'entity', label: 'name', rows: [{ key: 'PK', name: 'id', type: 'int' }] } },
  { name: 'Exactly one', means: 'Each row at the other end relates to one, and only one, row at this end.', edge: { line: 'solid', head: 'one' }, uses: edge => edge.head === 'one' || edge.tail === 'one' },
  { name: 'Zero or one', means: 'Each row at the other end relates to at most one row at this end.', edge: { line: 'solid', head: 'zero-or-one' }, uses: edge => edge.head === 'zero-or-one' || edge.tail === 'zero-or-one' },
  { name: 'One or many', means: 'Each row at the other end relates to at least one row at this end.', edge: { line: 'solid', head: 'one-or-many' }, uses: edge => edge.head === 'one-or-many' || edge.tail === 'one-or-many' },
  { name: 'Zero or many', means: 'Each row at the other end relates to any number of rows at this end.', edge: { line: 'solid', head: 'zero-or-many' }, uses: edge => edge.head === 'zero-or-many' || edge.tail === 'zero-or-many' },
  { name: 'Identifying relationship', means: 'A solid line: the child cannot be identified without its parent, whose key is part of its own.', edge: { line: 'solid' }, uses: edge => edge.line !== 'dashed' },
  { name: 'Non-identifying relationship', means: 'A dashed line: the child refers to its parent but has a key of its own.', edge: { line: 'dashed' }, uses: edge => edge.line === 'dashed' },
];
