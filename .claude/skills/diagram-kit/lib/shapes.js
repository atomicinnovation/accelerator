(function (root) {
  'use strict';
  const { blockOf, widthOf } = typeof module === 'object' && module.exports ? require('./text.js') : root.DiagramKit;

  const LINK_CONTROL_WIDTH = 24;
  const LABEL_LIMIT = 220;
  const linkRoom = node => (node.link ? LINK_CONTROL_WIDTH : 0);

  const box = {
    holdsOthers: false,
    blocksOf: node => ({ label: blockOf(node.label ?? node.id, 'name', LABEL_LIMIT), detail: node.detail ? blockOf(node.detail, 'detail', LABEL_LIMIT) : null }),
    sizeOf(node) {
      const { label, detail } = box.blocksOf(node);
      return {
        width: Math.ceil(Math.max(96, Math.max(label.width, detail ? detail.width : 0) + 28)) + linkRoom(node),
        height: label.height + (detail ? detail.height : 0) + 17,
      };
    },
  };

  const group = {
    holdsOthers: true,
    frameOf: node => ({
      padding: { top: 34, left: 14, bottom: 14, right: 14 },
      minimumWidth: Math.ceil(widthOf(String(node.label ?? node.id).toUpperCase(), 'heading')) + 28 + linkRoom(node),
    }),
  };

  const actor = {
    holdsOthers: false,
    sizeOf: node => ({ width: Math.ceil(Math.max(30, widthOf(node.label ?? node.id, 'name'))) + 8, height: 56 }),
  };

  const NOTE_PADDING = 10;
  const note = {
    holdsOthers: false,
    blockOf: (node, width) => blockOf(node.text, 'message', width - 2 * NOTE_PADDING),
    sizeOf: (node, width) => ({ width, height: note.blockOf(node, width).height + 14 }),
  };

  const fragment = {
    holdsOthers: false,
    tabWidthOf: node => Math.ceil(widthOf(node.operator, 'operator')) + 22,
    headWidthOf: node => fragment.tabWidthOf(node) + (node.guard ? Math.ceil(widthOf(`[${node.guard}]`, 'guard')) + 16 : 0),
  };

  const supporting = { holdsOthers: false };
  const SHAPES = { box, group, actor, note, fragment, lifeline: supporting, activation: supporting, destroy: supporting };
  const api = { SHAPES, LINK_CONTROL_WIDTH };
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
