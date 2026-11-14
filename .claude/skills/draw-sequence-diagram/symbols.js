module.exports = [
  { name: 'Participant', means: 'A component, object or process that takes part.', node: { shape: 'box', label: 'name' } },
  { name: 'Actor', means: 'A person, or a system outside, that takes part.', node: { shape: 'actor', label: 'name' } },
  { name: 'Lifeline', means: 'The participant over time, which runs down the page.', node: { shape: 'lifeline', x: 10, y: 1, width: 0, height: 30, room: 10 } },
  { name: 'Activation', means: 'The participant is busy handling a call for as long as the bar lasts.', node: { shape: 'activation', width: 8, height: 30 } },
  { name: 'Destruction', means: 'The participant ceases to exist.', node: { shape: 'destroy', width: 12, height: 12 } },
  { name: 'Synchronous call', means: 'The sender waits for the receiver to finish.', edge: { line: 'solid', head: 'filled' } },
  { name: 'Asynchronous message', means: 'The sender carries on without waiting.', edge: { line: 'solid', head: 'open' } },
  { name: 'Reply or creation', means: 'What a call returns to its sender, or a message that brings a participant into being.', edge: { line: 'dashed', head: 'open' } },
  { name: 'Fragment', means: 'Messages that happen only under a condition: `alt` one of several, `opt` perhaps, `loop` repeatedly, `par` in parallel, `break` and then stop. The condition is in square brackets.', node: { shape: 'fragment', operator: 'alt', guard: 'when', width: 96, height: 40 } },
  { name: 'Note', means: 'A remark about what happens at that point.', node: { shape: 'note', text: 'a remark', width: 80, height: 29 } },
];
