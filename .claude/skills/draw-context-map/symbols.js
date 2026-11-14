module.exports = [
  { name: 'Bounded context', means: 'A part of the system where one model holds, with its subdomain and team in brackets.', node: { shape: 'element', label: 'Name', type: 'Core' }, uses: node => !node.muted },
  { name: 'External context', means: 'A context whose model is not ours to change, shown greyed.', node: { shape: 'element', label: 'Name', type: 'External', muted: true }, uses: node => Boolean(node.muted) },
  { name: 'Upstream and downstream', means: 'A change to the model at the end marked `U` reaches the end marked `D`, never the reverse. Letters beside each say how that side takes part.', edge: { line: 'solid' }, uses: edge => Boolean(edge.fromLabel || edge.toLabel) },
  { name: 'Between equals', means: 'Neither context sets the terms: a partnership or a shared kernel, as the label says.', edge: { line: 'solid' }, uses: edge => !edge.fromLabel && !edge.toLabel },
];
