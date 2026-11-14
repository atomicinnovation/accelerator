module.exports = [
  { name: 'Start or end', means: 'Where the procedure begins, or one of the places it ends.', node: { shape: 'terminator', label: 'Start' } },
  { name: 'Process', means: 'One step that does something.', node: { shape: 'process', label: 'Do it' } },
  { name: 'Decision', means: 'A question, with one way out for each answer.', node: { shape: 'decision', label: 'Ok?' } },
  { name: 'Input or output', means: 'Data read in or written out.', node: { shape: 'io', label: 'Read' } },
  { name: 'Subprocess', means: 'A step that is a procedure of its own, defined elsewhere.', node: { shape: 'subprocess', label: 'Do it' } },
  { name: 'Lane', means: 'Everything in it is done by the one it is named for.', node: { shape: 'lane', label: 'name', width: 90, height: 48 } },
  { name: 'Flow', means: 'The step at the tail is followed by the step at the arrowhead. Out of a decision, the label is the answer.', edge: { line: 'solid', head: 'filled' } },
];
