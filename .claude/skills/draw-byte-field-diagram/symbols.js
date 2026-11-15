module.exports = [
  { name: 'Field', means: 'A run of bits or bytes with one meaning, as wide as its size. A field too long for its row carries on in the next.', node: { shape: 'field', label: 'name', width: 84, height: 28 }, uses: node => !node.broken && !node.muted },
  { name: 'Field drawn shorter than it is', means: 'A field many rows long, or of no fixed size. The text under its name says how long it is.', node: { shape: 'field', label: 'name', broken: true, x: 6, width: 78, height: 28, room: 6 }, uses: node => Boolean(node.broken) },
  { name: 'Reserved', means: 'Bits that carry nothing, shown greyed.', node: { shape: 'field', label: 'name', muted: true, width: 84, height: 28 }, uses: node => Boolean(node.muted) },
  { name: 'Scale and offsets', means: 'Along the top, the number of each bit or byte in a row, counted from 0 at the left. Beside each row, the offset it starts at.', node: { shape: 'scale', label: '0', width: 22, height: 14 } },
];
