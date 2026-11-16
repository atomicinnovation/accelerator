module.exports = [
  { name: 'Process', means: 'Something that reads, changes, checks or forwards data.', node: { shape: 'bubble', label: 'Check it' } },
  { name: 'Data store', means: 'Somewhere data rests: a file, a table, a cache.', node: { shape: 'store', label: 'name' } },
  { name: 'External entity', means: 'A person or system outside what is under review, where data comes from or goes to.', node: { shape: 'external', label: 'name' } },
  { name: 'Trust boundary', means: 'What is inside is trusted in a way that what is outside is not. Data must be checked or protected wherever a flow crosses one.', node: { shape: 'boundary', label: 'name', width: 130, height: 44 } },
  { name: 'Data flow', means: 'Data moves from the tail to the arrowhead, named on the line for what it is at that point.', edge: { line: 'solid', head: 'filled' } },
];
