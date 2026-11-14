module.exports = [
  { name: 'State', means: 'A condition the thing stays in for a while. Under the line: what it does on `entry`, as long as it lasts (`do`), and on `exit`.', node: { shape: 'state', label: 'Name', activities: ['entry / act'] } },
  { name: 'Composite state', means: 'A state with states of its own inside it.', node: { shape: 'composite', label: 'Name', width: 110, height: 52 } },
  { name: 'Initial state', means: 'Where it starts.', node: { shape: 'initial' } },
  { name: 'Final state', means: 'Where it ends.', node: { shape: 'final' } },
  { name: 'Choice', means: 'A branch: the guard on each way out decides which is taken.', node: { shape: 'choice' } },
  { name: 'Transition', means: 'A move from one state to another, labelled `event [guard] / action`: what causes it, the condition it needs, and what is done on the way.', edge: { line: 'solid', head: 'open' } },
];
