(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { SHAPES } = inNode ? require('./shapes.js') : root.DiagramKit;
  const { blockOf, widthOf } = inNode ? require('./text.js') : root.DiagramKit;
  const { Routes } = inNode ? require('./routes.js') : root.DiagramKit;
  const { onOutline } = inNode ? require('./layout.js') : root.DiagramKit;

  const LANE_HEAD = 28, LANE_PAD = 24, LANE_MINIMUM = 170, NODE_GAP = 22, HEAD_ROOM = 20, FOOT_ROOM = 24, EDGE_LABEL_LIMIT = 180;
  const even = number => 2 * Math.ceil(number / 2);
  const settled = number => Math.round(number * 100) / 100;

  function ranksOf(nodes, edges) {
    const state = new Map(), forward = [];
    const visit = id => {
      state.set(id, 'open');
      for (const edge of edges.filter(each => each.from === id)) {
        if (state.get(edge.to) === 'open') continue;
        forward.push(edge);
        if (!state.has(edge.to)) visit(edge.to);
      }
      state.set(id, 'done');
    };
    nodes.forEach(node => { if (!state.has(node.id)) visit(node.id); });
    const rank = new Map();
    const rankOf = id => {
      if (!rank.has(id)) {
        rank.set(id, 0);
        rank.set(id, Math.max(0, ...forward.filter(edge => edge.to === id && edge.from !== id).map(edge => rankOf(edge.from) + 1)));
      }
      return rank.get(id);
    };
    nodes.forEach(node => rankOf(node.id));
    return rank;
  }

  function laidOutInLanes(scene) {
    const rank = ranksOf(scene.nodes, scene.edges);
    const sized = scene.nodes.map(node => ({ node, rank: rank.get(node.id), ...SHAPES[node.shape].sizeOf(node) }));
    const rows = Math.max(...sized.map(each => each.rank)) + 1;
    const cell = (row, lane) => sized.filter(each => each.rank === row && each.node.lane === lane);
    const cellWidth = members => members.reduce((sum, each) => sum + each.width, 0) + Math.max(0, members.length - 1) * NODE_GAP;

    const lanes = [];
    let left = 0;
    for (const lane of scene.lanes) {
      const widest = Math.max(0, ...Array.from({ length: rows }, (_, row) => cellWidth(cell(row, lane.id))));
      const width = even(Math.max(LANE_MINIMUM, widest + 2 * LANE_PAD, widthOf(String(lane.label ?? lane.id).toUpperCase(), 'heading') + 28));
      lanes.push({ ...lane, x: left, width });
      left += width;
    }
    for (const lane of lanes) {
      for (let row = 0; row < rows; row++) {
        const members = cell(row, lane.id);
        let x = lane.x + lane.width / 2 - cellWidth(members) / 2;
        for (const each of members) { each.x = x; x += each.width + NODE_GAP; }
      }
    }

    const boxes = sized.map(each => {
      const inset = Math.min(10, each.width / 4);
      return { id: each.node.id, rank: each.rank, x: each.x, y: 0, w: each.width, h: each.height, portLeft: each.x + inset, portRight: each.x + each.width - inset };
    });
    const plan = Routes.plan(boxes, scene.edges);
    const gapAt = gap => (plan.gaps[gap] ? Routes.gapHeight(plan.gaps[gap]) : gap < 0 || gap >= rows - 1 ? 0 : Routes.gapHeight({}));
    const rowHeight = Array.from({ length: rows }, (_, row) => Math.max(...sized.filter(each => each.rank === row).map(each => each.height)));
    const rowTop = [];
    let y = LANE_HEAD + HEAD_ROOM + gapAt(-1);
    for (let row = 0; row < rows; row++) { rowTop[row] = y; y += rowHeight[row] + gapAt(row); }
    sized.forEach((each, i) => { each.y = rowTop[each.rank] + (rowHeight[each.rank] - each.height) / 2; boxes[i].y = each.y; });
    const top = LANE_HEAD + HEAD_ROOM;
    const gapTop = gap => (gap < 0 ? top : rowTop[gap] + rowHeight[gap]);
    const routes = Routes.draw(plan, boxes, { gapTop, gapBottom: gap => gapTop(gap) + gapAt(gap) });

    const placedById = new Map(sized.map(each => [each.node.id, { ...each.node, x: each.x, y: each.y, width: each.width, height: each.height }]));
    const edges = scene.edges.map((edge, i) => {
      const points = routes[i].points.map(point => [...point]);
      points[0] = onOutline(placedById.get(edge.from), points[0]);
      points[points.length - 1] = onOutline(placedById.get(edge.to), points[points.length - 1]);
      const block = edge.label ? blockOf(edge.label, 'edge', EDGE_LABEL_LIMIT) : null;
      const width = block ? Math.ceil(block.width) + 12 : 0, height = block ? block.height + 4 : 0;
      const [atX, atY] = routes[i].labelAt;
      return { ...edge, points, labelBox: block ? { x: atX - width / 2, y: atY - height / 2, width, height } : null };
    });

    const reach = edges.flatMap(edge => [...edge.points.map(point => point[0]), ...(edge.labelBox ? [edge.labelBox.x, edge.labelBox.x + edge.labelBox.width] : [])]);
    const shift = Math.max(0, Math.ceil(1 - Math.min(0, ...reach)));
    const width = Math.max(left, ...reach) + shift + 1, height = y + FOOT_ROOM;
    const moved = each => ({ ...each, x: settled(each.x + shift) });
    return {
      width: Math.ceil(width), height: Math.ceil(height),
      nodes: [
        ...lanes.map(lane => moved({ id: `lane:${lane.id}`, shape: 'lane', label: lane.label ?? lane.id, status: lane.status, tone: lane.tone, link: lane.link, inert: true, behind: true, x: lane.x, y: 0, width: lane.width, height: Math.ceil(height) })),
        ...scene.nodes.map(node => { const at = placedById.get(node.id); return moved({ ...at, y: settled(at.y) }); }),
      ],
      edges: edges.map(edge => ({ ...edge, points: edge.points.map(([x, at]) => [settled(x + shift), settled(at)]), labelBox: edge.labelBox && { ...edge.labelBox, x: settled(edge.labelBox.x + shift), y: settled(edge.labelBox.y) } })),
    };
  }

  const api = { laidOutInLanes };
  if (inNode) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
