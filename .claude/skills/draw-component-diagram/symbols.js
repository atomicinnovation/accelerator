module.exports = [
  { name: 'Component', means: 'A replaceable part of the system, known to the rest only through its interfaces.', node: { shape: 'classifier', label: 'Name', stereotype: 'component' } },
  { name: 'Subsystem', means: 'A component that holds other components.', node: { shape: 'frame', stereotype: 'subsystem', label: 'Name', width: 150, height: 44 } },
  { name: 'Interface', means: 'A set of operations one component offers and others call.', node: { shape: 'ball' } },
  { name: 'Provides', means: 'The component implements the interface the line reaches.', edge: { line: 'solid' } },
  { name: 'Requires or depends on', means: 'The component at the tail calls the interface, or uses the component, at the arrowhead.', edge: { line: 'dashed', head: 'open' } },
];
