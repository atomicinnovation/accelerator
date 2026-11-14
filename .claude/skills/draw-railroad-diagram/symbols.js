module.exports = [
  { name: 'Rule', means: 'A named piece of the syntax. Any route along its rails, from the bars at one end to the bars at the other, is something it accepts.', node: { shape: 'rulename', label: 'name', width: 40, height: 16 } },
  { name: 'Terminal', means: 'Text that appears in the input exactly as written.', node: { shape: 'token', label: ',' } },
  { name: 'Reference to a rule', means: 'Whatever the rule of that name accepts.', node: { shape: 'ruleref', label: 'name' } },
  { name: 'Rails', means: 'Read from left to right. A branch is a choice; a rail that passes something by makes it optional; a rail that loops back beneath lets it repeat, through whatever is on the loop.', node: { shape: 'rails', lines: [[[1, 8], [89, 8]], [[1, 8], [13, 8], [13, 28], [25, 28]], [[65, 28], [77, 28], [77, 8], [89, 8]], [[25, 28], [65, 28]]], width: 90, height: 34 } },
];
