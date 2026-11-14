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

  const SHAPES = { box, group };
  const api = { SHAPES, LINK_CONTROL_WIDTH };
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
