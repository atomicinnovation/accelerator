module.exports = [
  { name: 'Class', means: 'A type. Its attributes are listed above the line and its operations below; an underlined member is static.', node: { shape: 'classifier', label: 'Name', compartments: [['attribute'], ['operation()']] } },
  { name: 'Stereotype', means: 'The kind of type it is, in guillemets: an interface, a trait, a struct, an enumeration.', node: { shape: 'classifier', label: 'Name', stereotype: 'trait' }, uses: node => Boolean(node.stereotype) },
  { name: 'Abstract', means: 'A type that cannot be created itself, only through a more specific one.', node: { shape: 'classifier', label: 'Name', abstract: true }, uses: node => Boolean(node.abstract) },
  { name: 'Generalisation', means: 'The type at the tail is a more specific kind of the type at the triangle.', edge: { line: 'solid', head: 'hollow' } },
  { name: 'Realisation', means: 'The type at the tail implements the interface or trait at the triangle.', edge: { line: 'dashed', head: 'hollow' } },
  { name: 'Dependency', means: 'The type at the tail uses the type at the arrowhead, without holding it.', edge: { line: 'dashed', head: 'open' } },
  { name: 'Association', means: 'The two types know of each other: one holds or refers to the other.', edge: { line: 'solid' }, uses: edge => (edge.line || 'solid') === 'solid' && !edge.head && !edge.tail || edge.line === 'solid' && edge.head === 'none' && (!edge.tail || edge.tail === 'none') },
  { name: 'Navigable association', means: 'The type at the tail can reach the type at the arrowhead, and not the reverse.', edge: { line: 'solid', head: 'open' }, uses: edge => edge.line === 'solid' && edge.head === 'open' },
  { name: 'Aggregation', means: 'The type at the diamond holds the other, which has a life of its own.', edge: { line: 'solid', tail: 'diamond' }, uses: edge => edge.tail === 'diamond' },
  { name: 'Composition', means: 'The type at the diamond owns the other, which does not outlive it.', edge: { line: 'solid', tail: 'filled-diamond' }, uses: edge => edge.tail === 'filled-diamond' },
  { name: 'Multiplicity and role', means: 'Beside an end of a line: how many of that type take part (`1`, `0..1`, `*`), and what it is called by the other.', html: '<code>0..*</code>', usedIn: spec => (spec.relationships || []).some(each => each.fromMultiplicity || each.toMultiplicity || each.fromRole || each.toRole) },
];
