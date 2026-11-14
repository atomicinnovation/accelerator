module.exports = [
  { name: 'Action', means: 'One step of work.', node: { shape: 'state', label: 'Do it' } },
  { name: 'Initial node', means: 'Where the activity starts.', node: { shape: 'initial' } },
  { name: 'Activity final', means: 'Where the whole activity ends.', node: { shape: 'final' } },
  { name: 'Flow final', means: 'Where one path ends while others carry on.', node: { shape: 'flowfinal' } },
  { name: 'Decision or merge', means: 'With one way in, a branch: the guard on each way out decides which is taken. With one way out, paths rejoining.', node: { shape: 'choice' } },
  { name: 'Fork or join', means: 'With one way in, the paths out all run at once. With one way out, the work waits for every path in.', node: { shape: 'bar' } },
  { name: 'Partition', means: 'A swimlane: everything in it is done by the one it is named for.', node: { shape: 'lane', label: 'name', width: 90, height: 48 } },
  { name: 'Control flow', means: 'The step at the tail is followed by the step at the arrowhead, under the guard in square brackets if there is one.', edge: { line: 'solid', head: 'open' } },
];
