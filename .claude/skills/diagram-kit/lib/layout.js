(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { SHAPES } = inNode ? require('./shapes.js') : root.DiagramKit;
  const { blockOf } = inNode ? require('./text.js') : root.DiagramKit;

  const DIRECTIONS = { down: 'DOWN', right: 'RIGHT' };
  const EDGE_LABEL_LIMIT = 180;
  const HEAD_RUN = 16;
  const SPACING = {
    'elk.layered.spacing.nodeNodeBetweenLayers': '46',
    'elk.spacing.nodeNode': '26',
    'elk.spacing.edgeNode': '18',
    'elk.spacing.edgeEdge': '12',
    'elk.spacing.edgeLabel': '4',
    'elk.layered.spacing.edgeNodeBetweenLayers': '18',
  };

  const shapeOf = node => {
    const shape = SHAPES[node.shape];
    if (!shape) throw new Error(`node ${node.id}: no shape "${node.shape}"`);
    return shape;
  };

  function everyNode(nodes, visit, parent = null, depth = 0) {
    for (const node of nodes || []) {
      visit(node, parent, depth);
      everyNode(node.children, visit, node, depth + 1);
    }
  }

  // elkjs applies a holder's minimum size along its own flow, so for a scene
  // flowing downwards the minimum width has to be given as the height.
  const minimumWidth = (width, direction) => (direction === 'DOWN' ? `(0,${width})` : `(${width},0)`);

  function layoutNodeOf(node, direction) {
    const shape = shapeOf(node);
    if (!shape.holdsOthers) return { id: node.id, ...shape.sizeOf(node) };
    const { padding, minimumWidth: width } = shape.frameOf(node);
    return {
      id: node.id,
      children: (node.children || []).map(child => layoutNodeOf(child, direction)),
      layoutOptions: {
        'elk.padding': `[top=${padding.top},left=${padding.left},bottom=${padding.bottom},right=${padding.right}]`,
        'elk.nodeSize.constraints': 'MINIMUM_SIZE',
        'elk.nodeSize.minimum': minimumWidth(width, direction),
      },
    };
  }

  function layoutEdgeOf(edge, known) {
    for (const end of [edge.from, edge.to]) if (!known.has(end)) throw new Error(`edge ${edge.id}: no node "${end}"`);
    const label = edge.label ? blockOf(edge.label, 'edge', EDGE_LABEL_LIMIT) : null;
    return {
      id: edge.id, sources: [edge.from], targets: [edge.to],
      labels: label ? [{ text: edge.label, width: Math.ceil(label.width) + 12, height: label.height + 4 }] : [],
    };
  }

  function layoutGraphOf(scene) {
    const known = new Set();
    everyNode(scene.nodes, node => known.add(node.id));
    const direction = DIRECTIONS[scene.direction || 'down'];
    return {
      id: 'root',
      layoutOptions: {
        'elk.algorithm': 'layered',
        'elk.direction': direction,
        'elk.edgeRouting': 'ORTHOGONAL',
        'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
        'elk.layered.edgeLabels.sideSelection': 'ALWAYS_UP',
        'elk.layered.cycleBreaking.strategy': 'MODEL_ORDER',
        'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES',
        ...SPACING,
      },
      children: (scene.nodes || []).map(node => layoutNodeOf(node, direction)),
      edges: (scene.edges || []).map(edge => layoutEdgeOf(edge, known)),
    };
  }

  const settled = number => Math.round(number * 100) / 100;

  function onOutline(node, point) {
    const outline = SHAPES[node.shape].outline;
    if (!outline) return point;
    const cx = node.x + node.width / 2, cy = node.y + node.height / 2, halfWide = node.width / 2, halfTall = node.height / 2;
    const fromAboveOrBelow = Math.abs(Math.abs(point[1] - cy) - halfTall) < 0.5;
    const across = fromAboveOrBelow ? point[0] - cx : point[1] - cy, reach = fromAboveOrBelow ? halfWide : halfTall, depth = fromAboveOrBelow ? halfTall : halfWide;
    if (Math.abs(across) >= reach) return point;
    const inward = outline === 'disc' ? Math.sqrt(reach * reach - across * across) * depth / reach : depth * (1 - Math.abs(across) / reach);
    const side = Math.sign((fromAboveOrBelow ? point[1] - cy : point[0] - cx)) || 1;
    return fromAboveOrBelow ? [point[0], settled(cy + side * inward)] : [settled(cx + side * inward), point[1]];
  }

  function positionedFrom(laid, scene) {
    const said = new Map(), origins = new Map([[laid.id, [0, 0]]]), nodes = [], edges = [];
    everyNode(scene.nodes, (node, parent, depth) => said.set(node.id, { node, parent: parent ? parent.id : null, depth }));
    const place = (laidNode, left, top) => {
      for (const each of laidNode.children || []) {
        const x = left + each.x, y = top + each.y;
        const { node, parent, depth } = said.get(each.id);
        const { children, ...own } = node;
        origins.set(each.id, [x, y]);
        nodes.push({ ...own, parent, depth, x: settled(x), y: settled(y), width: settled(each.width), height: settled(each.height) });
        place(each, x, y);
      }
    };
    place(laid, 0, 0);
    const placedById = new Map(nodes.map(node => [node.id, node]));
    const route = laidNode => {
      for (const each of laidNode.edges || []) {
        const [left, top] = origins.get(each.container) || [0, 0];
        const points = (each.sections || []).flatMap(section => [section.startPoint, ...(section.bendPoints || []), section.endPoint])
          .map(point => [settled(left + point.x), settled(top + point.y)]);
        const [label] = each.labels || [];
        const labelBox = label ? { x: settled(left + label.x), y: settled(top + label.y), width: label.width, height: label.height } : null;
        const said = (scene.edges || []).find(edge => edge.id === each.id);
        if (points.length) {
          points[0] = onOutline(placedById.get(said.from), points[0]);
          points[points.length - 1] = onOutline(placedById.get(said.to), points[points.length - 1]);
        }
        edges.push({ ...said, points: withRoomForHead(points), labelBox });
      }
      (laidNode.children || []).forEach(route);
    };
    route(laid);
    const inSceneOrder = new Map((scene.edges || []).map((edge, position) => [edge.id, position]));
    edges.sort((a, b) => inSceneOrder.get(a.id) - inSceneOrder.get(b.id));
    return { width: settled(laid.width), height: settled(laid.height), nodes, edges };
  }

  function withRoomForHead(points) {
    if (points.length < 4) return points;
    const [before, turn, corner, tip] = points.slice(-4);
    const axis = corner[0] === tip[0] ? 1 : 0;
    const lastRun = Math.abs(tip[axis] - corner[axis]), spare = Math.abs(turn[axis] - before[axis]);
    const short = HEAD_RUN - lastRun;
    const parallel = before[1 - axis] === turn[1 - axis] && turn[axis] === corner[axis];
    if (short <= 0 || !parallel || spare - short < HEAD_RUN / 2) return points;
    const away = Math.sign(corner[axis] - tip[axis]) * short;
    const moved = point => (axis ? [point[0], settled(point[1] + away)] : [settled(point[0] + away), point[1]]);
    return [...points.slice(0, -3), moved(turn), moved(corner), tip];
  }

  let engine = null;
  function layoutEngine() {
    if (!engine) {
      const ELK = inNode ? require('../vendor/elkjs/elk.bundled.js') : root.ELK;
      if (!ELK) throw new Error('the layout engine is not loaded on this page');
      engine = new ELK();
    }
    return engine;
  }

  async function laidOut(scene) {
    const graph = layoutGraphOf(scene);
    return positionedFrom(await layoutEngine().layout(graph), scene);
  }

  const api = { layoutGraphOf, positionedFrom, laidOut };
  if (inNode) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
