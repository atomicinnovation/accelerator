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

  const ACTIVITY_LINE = 14;
  const state = {
    holdsOthers: false,
    sizeOf(node) {
      const activities = node.activities || [];
      const widest = Math.max(widthOf(node.label ?? node.id, 'name') + 28, ...activities.map(activity => widthOf(activity, 'detail') + 24));
      return { width: Math.ceil(Math.max(96, widest)) + linkRoom(node), height: 32 + (activities.length ? 8 + activities.length * ACTIVITY_LINE + 4 : 0) };
    },
  };

  const composite = {
    holdsOthers: true,
    frameOf: node => ({
      padding: { top: 40, left: 14, bottom: 14, right: 14 },
      minimumWidth: Math.ceil(widthOf(node.label ?? node.id, 'name')) + 28 + linkRoom(node),
    }),
  };

  const MEMBER_LINE = 15;
  const memberText = member => (typeof member === 'string' ? member : member.text);
  const classifier = {
    holdsOthers: false,
    headHeightOf: node => 32 + (node.stereotype ? 14 : 0) + (node.abstract ? 14 : 0),
    compartmentHeightOf: members => 11 + members.length * MEMBER_LINE,
    sizeOf(node) {
      const compartments = node.compartments || [];
      const widest = Math.max(
        widthOf(node.label ?? node.id, 'title') + 28,
        node.stereotype ? widthOf(`«${node.stereotype}»`, 'stereotype') + 28 : 0,
        ...compartments.flat().map(member => widthOf(memberText(member), 'name') + 20),
      );
      return {
        width: Math.ceil(Math.max(96, widest)) + linkRoom(node),
        height: classifier.headHeightOf(node) + compartments.reduce((sum, members) => sum + classifier.compartmentHeightOf(members), 0),
      };
    },
  };

  const boundary = {
    holdsOthers: true,
    nameOffsetOf: node => (node.stereotype ? Math.ceil(widthOf(`«${node.stereotype}»`, 'stereotype')) + 6 : 0),
    frameOf: node => ({
      padding: { top: 36, left: 14, bottom: 14, right: 14 },
      minimumWidth: boundary.nameOffsetOf(node) + Math.ceil(widthOf(node.label ?? node.id, 'title')) + 28 + linkRoom(node),
    }),
  };

  const frame = { holdsOthers: true, nameOffsetOf: boundary.nameOffsetOf, frameOf: boundary.frameOf };

  const TAB_HEIGHT = 22;
  const packageShape = {
    holdsOthers: true,
    tabWidthOf: node => Math.ceil(widthOf(node.label ?? node.id, 'title')) + 24,
    frameOf: node => ({ padding: { top: TAB_HEIGHT + 16, left: 14, bottom: 14, right: 14 }, minimumWidth: packageShape.tabWidthOf(node) + 28 + linkRoom(node) }),
  };
  const folder = {
    holdsOthers: false,
    sizeOf: node => ({ width: Math.ceil(Math.max(96, widthOf(node.label ?? node.id, 'title') + 28)) + linkRoom(node), height: 50 }),
  };
  const ball = {
    holdsOthers: false, outline: 'disc',
    sizeOf: () => ({ width: 16, height: 16 }),
    nameBesideOf: node => (node.label ? { text: node.label, width: Math.ceil(widthOf(node.label, 'name')) + 2, height: 15 } : null),
  };

  const fixed = (side, outline) => ({ holdsOthers: false, outline, sizeOf: () => ({ width: side, height: side }) });

  const STEP_TEXT_LIMIT = 220, QUESTION_LIMIT = 130;
  const worded = (padding, limit = STEP_TEXT_LIMIT) => ({
    holdsOthers: false,
    textOf: node => blockOf(node.label ?? node.id, 'message', limit),
    sizeOf(node) {
      const text = this.textOf(node);
      return { width: Math.ceil(Math.max(96, text.width + padding)) + linkRoom(node), height: text.height + 21 };
    },
  });
  const decision = {
    holdsOthers: false, outline: 'diamond',
    textOf: node => blockOf(node.label ?? node.id, 'message', QUESTION_LIMIT),
    sizeOf(node) {
      const text = decision.textOf(node);
      return { width: Math.ceil(2 * text.width) + 16, height: 2 * text.height + 18 };
    },
  };

  const bar = { holdsOthers: false, sizeOf: node => (node.upright ? { width: 6, height: 84 } : { width: 84, height: 6 }) };

  const supporting = { holdsOthers: false };
  const SHAPES = {
    box, group, actor, note, fragment, lane: supporting, lifeline: supporting, activation: supporting, destroy: supporting,
    classifier, boundary, frame, package: packageShape, folder, ball, process: worded(28), terminator: worded(40), io: worded(52), subprocess: worded(44), decision, bar, flowfinal: fixed(20, 'disc'), state, composite, initial: fixed(14, 'disc'), final: fixed(20, 'disc'), choice: fixed(28, 'diamond'),
  };
  const api = { SHAPES, LINK_CONTROL_WIDTH, ACTIVITY_LINE, MEMBER_LINE, TAB_HEIGHT };
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
