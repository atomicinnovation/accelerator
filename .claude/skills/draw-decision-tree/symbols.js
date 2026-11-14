module.exports = [
  { name: 'Question', means: 'A condition that is tested.', node: { shape: 'process', label: 'Is it?' } },
  { name: 'Outcome', means: 'What is decided when the questions on the way to it have been answered.', node: { shape: 'terminator', label: 'Keep' } },
  { name: 'Answer', means: 'The answer to the question at the tail that leads this way.', edge: { line: 'solid', head: 'open' } },
];
