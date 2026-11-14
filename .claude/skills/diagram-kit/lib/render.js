(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { blockOf, ROLES } = inNode ? require('./text.js') : root.DiagramKit;
  const { SHAPES, LINK_CONTROL_WIDTH, ACTIVITY_LINE, MEMBER_LINE, TAB_HEIGHT, DEPTH, HEAD_ROOM, ROW_LINE } = inNode ? require('./shapes.js') : root.DiagramKit;

  const MARGIN = 8;
  const HEAD_LENGTH = 9, HEAD_HALF_WIDTH = 4.5, HOLLOW_LENGTH = 12, HOLLOW_HALF_WIDTH = 6, DIAMOND_LENGTH = 14, DIAMOND_HALF_WIDTH = 5;
  const END_LABEL_GAP = 12;
  const CORNER = 6;
  const STATUSES = { new: 'new in this change', changed: 'changed', removed: 'removed', existing: 'unchanged' };
  const TONES = { risk: 'risk', ok: 'safe' };

  const escaped = text => String(text ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const tidyNumber = number => Math.round(number * 100) / 100;
  const tidy = number => String(tidyNumber(number));
  const marks = item => `${item.muted ? ' muted' : ''}${STATUSES[item.status] ? ` s-${item.status}` : ''}${TONES[item.tone] ? ` t-${item.tone}` : ''}`;

  function addressOnly(reference) {
    return /^(https?:|#)/.test(String(reference)) ? { href: String(reference), label: `Open ${reference}` } : null;
  }

  function textOf(block, role, x, firstBaseline, anchor) {
    const open = `<text class="${role}" x="${tidy(x)}" y="${tidy(firstBaseline)}" text-anchor="${anchor}">`;
    if (block.lines.length === 1) return `${open}${escaped(block.lines[0])}</text>`;
    return `${open}${block.lines.map((line, i) => `<tspan x="${tidy(x)}" dy="${i ? ROLES[block.role].lineHeight : 0}">${escaped(line)}</tspan>`).join('')}</text>`;
  }

  const attributesOf = target => Object.entries(target.attributes || {}).map(([name, value]) => ` ${name}="${escaped(value)}"`).join('');

  function linkControl(node, target) {
    const right = node.x + node.width, left = right - LINK_CONTROL_WIDTH, top = node.y, bottom = node.y + node.height, r = 7;
    const cx = left + LINK_CONTROL_WIDTH / 2, cy = top + node.height / 2;
    const attributes = attributesOf(target);
    const hit = `M${tidy(left)} ${tidy(top)} H${tidy(right - r)} Q${tidy(right)} ${tidy(top)} ${tidy(right)} ${tidy(top + r)} V${tidy(bottom - r)} Q${tidy(right)} ${tidy(bottom)} ${tidy(right - r)} ${tidy(bottom)} H${tidy(left)} Z`;
    return `<a class="node-link" href="${escaped(target.href)}"${attributes} aria-label="${escaped(target.label)}"><title>${escaped(target.label)}</title>`
      + `<path class="hit" d="${hit}"/><line class="divider" x1="${tidy(left)}" y1="${tidy(top + 6)}" x2="${tidy(left)}" y2="${tidy(bottom - 6)}"/>`
      + `<path class="glyph" d="M${tidy(cx - 4)} ${tidy(cy + 4)} L${tidy(cx + 4)} ${tidy(cy - 4)} M${tidy(cx - 2)} ${tidy(cy - 4)} L${tidy(cx + 4)} ${tidy(cy - 4)} L${tidy(cx + 4)} ${tidy(cy + 2)}"/></a>`;
  }

  const bodyOf = (node, radius) => `<rect class="body" x="${tidy(node.x)}" y="${tidy(node.y)}" width="${tidy(node.width)}" height="${tidy(node.height)}" rx="${radius}"/>`;
  const untold = (node, target) => (node.link && !target ? `<title>${escaped(node.link)}</title>` : '');

  const rectOf = node => `x="${tidy(node.x)}" y="${tidy(node.y)}" width="${tidy(node.width)}" height="${tidy(node.height)}"`;
  const plainText = (text, role, x, baseline) => textOf(blockOf(text, role), role, x, baseline, 'start');

  function wordsIn(node, target) {
    const block = SHAPES[node.shape].textOf(node);
    const words = textOf(block, 'message', node.x + node.width / 2, node.y + (node.height - block.height) / 2 + 11, 'middle');
    return (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${words}</a>` : words) + untold(node, target);
  }

  function facesOf(node) {
    const left = node.x, top = node.y, right = node.x + node.width, bottom = node.y + node.height, inner = right - DEPTH, lower = top + DEPTH;
    return `<path class="face" d="M${tidy(left)} ${tidy(lower)} L${tidy(left + DEPTH)} ${tidy(top)} H${tidy(right)} L${tidy(inner)} ${tidy(lower)} Z `
      + `M${tidy(inner)} ${tidy(lower)} L${tidy(right)} ${tidy(top)} V${tidy(bottom - DEPTH)} L${tidy(inner)} ${tidy(bottom)} Z"/>`;
  }

  function elementWords(node, top, target) {
    const centre = node.x + node.width / 2, described = SHAPES.element.descriptionOf(node, node.width);
    const name = textOf(blockOf(node.label ?? node.id, 'title'), 'title', centre, top + 23, 'middle');
    return (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name)
      + (node.type ? textOf(blockOf(`[${node.type}]`, 'stereotype'), 'stereotype', centre, top + 37, 'middle') : '')
      + (described ? textOf(described, 'detail', centre, top + 27 + (node.type ? 14 : 0) + 6 + 10, 'middle') : '') + untold(node, target);
  }

  const UNNAMED = { initial: 'the start', final: 'the end', choice: 'the choice', bar: 'the bar', flowfinal: 'the end of this flow' };

  const DRAWN = {
    classifier(node, target) {
      const centre = node.x + (node.width - (target ? LINK_CONTROL_WIDTH : 0)) / 2, right = node.x + node.width;
      const head = SHAPES.classifier.headHeightOf(node), nameAt = node.y + 20 + (node.stereotype ? 14 : 0);
      const centred = (text, role, cls, baseline) => textOf(blockOf(text, role), cls, centre, baseline, 'middle');
      let top = node.y + head, listed = '';
      for (const members of node.compartments || []) {
        listed += `<line class="rule" x1="${tidy(node.x)}" y1="${tidy(top)}" x2="${tidy(right)}" y2="${tidy(top)}"/>`
          + members.map((member, i) => textOf(blockOf(typeof member === 'string' ? member : member.text, 'name'), `member${member.static ? ' static' : ''}`, node.x + 10, top + 17 + i * MEMBER_LINE, 'start')).join('');
        top += SHAPES.classifier.compartmentHeightOf(members);
      }
      return bodyOf(node, 3)
        + (node.stereotype ? centred(`«${node.stereotype}»`, 'stereotype', 'stereotype', node.y + 17) : '')
        + centred(node.label ?? node.id, 'title', `title${node.underlined ? ' underlined' : ''}`, nameAt)
        + (node.abstract ? centred('{abstract}', 'stereotype', 'stereotype', nameAt + 14) : '')
        + listed + (target ? linkControl({ ...node, height: head }, target) : '') + untold(node, target);
    },
    frame: (node, target) => DRAWN.boundary(node, target, 3),
    entity(node, target) {
      const right = node.x + node.width, centre = node.x + (node.width - (target ? LINK_CONTROL_WIDTH : 0)) / 2, keys = SHAPES.entity.keyColumnOf(node);
      const cell = (text, cls, row, x, anchor) => (text ? textOf(blockOf(text, cls === 'key' ? 'key' : 'name'), `${cls}${STATUSES[row.status] ? ` s-${row.status}` : ''}`, x, 0, anchor) : '');
      const rows = (node.rows || []).map((row, i) => [cell(row.key, 'key', row, node.x + 8, 'start'), cell(row.name, 'column', row, node.x + 8 + keys, 'start'), cell(row.type, 'type', row, right - 8, 'end')]
        .join('').replace(/ y="0"/g, ` y="${tidy(node.y + 47 + i * ROW_LINE)}"`)).join('');
      return bodyOf(node, 3) + textOf(blockOf(node.label ?? node.id, 'title'), 'title', centre, node.y + 20, 'middle')
        + ((node.rows || []).length ? `<line class="rule" x1="${tidy(node.x)}" y1="${tidy(node.y + 30)}" x2="${tidy(right)}" y2="${tidy(node.y + 30)}"/>${rows}` : '')
        + (target ? linkControl({ ...node, height: 30 }, target) : '') + untold(node, target);
    },
    cuboid(node, target) {
      const front = { ...node, y: node.y + DEPTH, width: node.width - DEPTH, height: node.height - DEPTH };
      const name = plainText(node.label ?? node.id, 'title', front.x + 14 + SHAPES.cuboid.nameOffsetOf(node), front.y + 20);
      return `<rect class="body" ${rectOf(front)}/>${facesOf(node)}` + (node.stereotype ? plainText(`«${node.stereotype}»`, 'stereotype', front.x + 14, front.y + 20) : '')
        + (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name) + untold(node, target);
    },
    cuboidbox(node, target) {
      const front = { ...node, y: node.y + DEPTH, width: node.width - DEPTH, height: node.height - DEPTH }, centre = front.x + front.width / 2;
      const nameAt = front.y + 20 + (node.stereotype ? 14 : 0);
      const name = textOf(blockOf(node.label ?? node.id, 'title'), 'title', centre, nameAt, 'middle');
      return `<rect class="body" ${rectOf(front)}/>${facesOf(node)}` + (node.stereotype ? textOf(blockOf(`«${node.stereotype}»`, 'stereotype'), 'stereotype', centre, front.y + 17, 'middle') : '')
        + (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name) + untold(node, target);
    },
    element: (node, target) => bodyOf(node, 7) + elementWords(node, node.y, target),
    person(node, target) {
      const box = { ...node, y: node.y + HEAD_ROOM, height: node.height - HEAD_ROOM };
      return bodyOf(box, 7) + `<circle class="head" cx="${tidy(node.x + node.width / 2)}" cy="${tidy(node.y + 12)}" r="12"/>` + elementWords(node, box.y, target);
    },
    package(node, target) {
      const tab = SHAPES.package.tabWidthOf(node), right = node.x + node.width, bottom = node.y + node.height, under = node.y + TAB_HEIGHT;
      const name = plainText(node.label ?? node.id, 'title', node.x + 12, node.y + 16);
      return `<path class="body" d="M${tidy(node.x)} ${tidy(node.y)} H${tidy(node.x + tab)} V${tidy(under)} H${tidy(right)} V${tidy(bottom)} H${tidy(node.x)} Z"/>`
        + `<path class="rule" d="M${tidy(node.x)} ${tidy(under)} H${tidy(node.x + tab)}"/>`
        + (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name) + untold(node, target);
    },
    folder(node, target) {
      const tab = 42, lip = 10, right = node.x + node.width, bottom = node.y + node.height;
      const name = textOf(blockOf(node.label ?? node.id, 'title'), 'title', node.x + node.width / 2, node.y + 34, 'middle');
      return `<path class="body" d="M${tidy(node.x)} ${tidy(node.y)} H${tidy(node.x + tab)} V${tidy(node.y + lip)} H${tidy(right)} V${tidy(bottom)} H${tidy(node.x)} Z"/>`
        + `<path class="rule" d="M${tidy(node.x)} ${tidy(node.y + lip)} H${tidy(node.x + tab)}"/>`
        + (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name) + untold(node, target);
    },
    ball(node, target) {
      const name = node.label && node.labelBox ? textOf(blockOf(node.label, 'name'), 'name outside', node.labelBox.x, node.labelBox.y + 11, 'start') : '';
      return `<circle class="body" cx="${tidy(node.x + node.width / 2)}" cy="${tidy(node.y + node.height / 2)}" r="${tidy(node.width / 2)}"/>`
        + (target && name ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name) + untold(node, target);
    },
    boundary(node, target, radius = 10) {
      const name = plainText(node.label ?? node.id, 'title', node.x + 14 + SHAPES.boundary.nameOffsetOf(node), node.y + 20);
      return bodyOf(node, radius) + (node.stereotype ? plainText(`«${node.stereotype}»`, 'stereotype', node.x + 14, node.y + 20) : '')
        + (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name) + untold(node, target);
    },
    state(node, target) {
      const centre = node.x + (node.width - (target ? LINK_CONTROL_WIDTH : 0)) / 2, activities = node.activities || [];
      const listed = activities.map((activity, i) => plainText(activity, 'detail', node.x + 12, node.y + 48 + i * ACTIVITY_LINE)).join('');
      return bodyOf(node, 12) + textOf(blockOf(node.label ?? node.id, 'name'), 'name', centre, node.y + 20, 'middle')
        + (activities.length ? `<line class="rule" x1="${tidy(node.x)}" y1="${tidy(node.y + 32)}" x2="${tidy(node.x + node.width)}" y2="${tidy(node.y + 32)}"/>${listed}` : '')
        + (target ? linkControl({ ...node, height: 32 }, target) : '') + untold(node, target);
    },
    composite(node, target) {
      const name = plainText(node.label ?? node.id, 'name', node.x + 14, node.y + 20);
      return bodyOf(node, 12) + (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name)
        + `<line class="rule" x1="${tidy(node.x)}" y1="${tidy(node.y + 30)}" x2="${tidy(node.x + node.width)}" y2="${tidy(node.y + 30)}"/>` + untold(node, target);
    },
    process: (node, target) => bodyOf(node, 2) + wordsIn(node, target),
    terminator: (node, target) => bodyOf(node, tidy(node.height / 2)) + wordsIn(node, target),
    subprocess: (node, target) => bodyOf(node, 2)
      + `<path class="rule" d="M${tidy(node.x + 8)} ${tidy(node.y)} V${tidy(node.y + node.height)} M${tidy(node.x + node.width - 8)} ${tidy(node.y)} V${tidy(node.y + node.height)}"/>` + wordsIn(node, target),
    io(node, target) {
      const lean = 12, right = node.x + node.width, bottom = node.y + node.height;
      return `<path class="body" d="M${tidy(node.x + lean)} ${tidy(node.y)} H${tidy(right)} L${tidy(right - lean)} ${tidy(bottom)} H${tidy(node.x)} Z"/>` + wordsIn(node, target);
    },
    decision(node, target) {
      const cx = node.x + node.width / 2, cy = node.y + node.height / 2;
      return `<path class="body" d="M${tidy(cx)} ${tidy(node.y)} L${tidy(node.x + node.width)} ${tidy(cy)} L${tidy(cx)} ${tidy(node.y + node.height)} L${tidy(node.x)} ${tidy(cy)} Z"/>` + wordsIn(node, target);
    },
    bubble: (node, target) => `<circle class="body" cx="${tidy(node.x + node.width / 2)}" cy="${tidy(node.y + node.height / 2)}" r="${tidy(node.width / 2)}"/>` + wordsIn(node, target),
    store: (node, target) => `<rect class="back" ${rectOf(node)}/>`
      + `<path class="body" d="M${tidy(node.x)} ${tidy(node.y)} H${tidy(node.x + node.width)} M${tidy(node.x)} ${tidy(node.y + node.height)} H${tidy(node.x + node.width)}"/>` + wordsIn(node, target),
    external: (node, target) => bodyOf(node, 0) + wordsIn(node, target),
    bar: node => `<rect class="body" ${rectOf(node)} rx="2"/>`,
    flowfinal(node) {
      const cx = node.x + node.width / 2, cy = node.y + node.height / 2, radius = node.width / 2, arm = radius * 0.7;
      return `<circle class="body" cx="${tidy(cx)}" cy="${tidy(cy)}" r="${tidy(radius)}"/>`
        + `<path class="cross" d="M${tidy(cx - arm)} ${tidy(cy - arm)} L${tidy(cx + arm)} ${tidy(cy + arm)} M${tidy(cx + arm)} ${tidy(cy - arm)} L${tidy(cx - arm)} ${tidy(cy + arm)}"/>`;
    },
    initial: node => `<circle class="body" cx="${tidy(node.x + node.width / 2)}" cy="${tidy(node.y + node.height / 2)}" r="${tidy(node.width / 2)}"/>`,
    final(node) {
      const cx = tidy(node.x + node.width / 2), cy = tidy(node.y + node.height / 2);
      return `<circle class="body" cx="${cx}" cy="${cy}" r="${tidy(node.width / 2)}"/><circle class="core" cx="${cx}" cy="${cy}" r="${tidy(node.width / 2 - 4.5)}"/>`;
    },
    choice(node) {
      const cx = node.x + node.width / 2, cy = node.y + node.height / 2;
      return `<path class="body" d="M${tidy(cx)} ${tidy(node.y)} L${tidy(node.x + node.width)} ${tidy(cy)} L${tidy(cx)} ${tidy(node.y + node.height)} L${tidy(node.x)} ${tidy(cy)} Z"/>`;
    },
    lane(node) {
      const right = node.x + node.width;
      return `<rect class="body" ${rectOf(node)}/><line class="rule" x1="${tidy(node.x)}" y1="${tidy(node.y + 28)}" x2="${tidy(right)}" y2="${tidy(node.y + 28)}"/>`
        + textOf(blockOf(String(node.label ?? node.id).toUpperCase(), 'heading'), 'heading', node.x + node.width / 2, node.y + 19, 'middle');
    },
    lifeline: node => `<line class="body" x1="${tidy(node.x)}" y1="${tidy(node.y)}" x2="${tidy(node.x)}" y2="${tidy(node.y + node.height)}"/>`,
    activation: node => `<rect class="body" ${rectOf(node)}/>`,
    destroy(node) {
      const right = node.x + node.width, bottom = node.y + node.height;
      return `<path class="body" d="M${tidy(node.x)} ${tidy(node.y)} L${tidy(right)} ${tidy(bottom)} M${tidy(right)} ${tidy(node.y)} L${tidy(node.x)} ${tidy(bottom)}"/>`;
    },
    fragment(node) {
      const tab = SHAPES.fragment.tabWidthOf(node), right = node.x + node.width;
      const dividers = (node.dividers || []).map(divider => `<line class="divider" x1="${tidy(node.x)}" y1="${tidy(divider.y)}" x2="${tidy(right)}" y2="${tidy(divider.y)}"/>`
        + (divider.guard ? plainText(`[${divider.guard}]`, 'guard', node.x + 8, divider.y + 15) : '')).join('');
      return `<rect class="body" ${rectOf(node)}/>`
        + `<path class="tab" d="M${tidy(node.x)} ${tidy(node.y)} H${tidy(node.x + tab)} V${tidy(node.y + 14)} L${tidy(node.x + tab - 6)} ${tidy(node.y + 20)} H${tidy(node.x)} Z"/>`
        + plainText(node.operator, 'operator', node.x + 8, node.y + 14)
        + (node.guard ? plainText(`[${node.guard}]`, 'guard', node.x + tab + 8, node.y + 14) : '') + dividers;
    },
    actor(node, target) {
      const cx = node.x + node.width / 2, top = node.y;
      const name = textOf(blockOf(node.label ?? node.id, 'name'), 'name', cx, top + 52, 'middle');
      return `<rect class="body" ${rectOf(node)}/><circle class="figure" cx="${tidy(cx)}" cy="${tidy(top + 8)}" r="6"/>`
        + `<path class="figure" d="M${tidy(cx)} ${tidy(top + 14)} V${tidy(top + 30)} M${tidy(cx - 12)} ${tidy(top + 20)} H${tidy(cx + 12)} M${tidy(cx)} ${tidy(top + 30)} L${tidy(cx - 9)} ${tidy(top + 40)} M${tidy(cx)} ${tidy(top + 30)} L${tidy(cx + 9)} ${tidy(top + 40)}"/>`
        + (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${name}</a>` : name) + untold(node, target);
    },
    note(node) {
      const right = node.x + node.width, bottom = node.y + node.height, fold = 8;
      return `<path class="body" d="M${tidy(node.x)} ${tidy(node.y)} H${tidy(right - fold)} L${tidy(right)} ${tidy(node.y + fold)} V${tidy(bottom)} H${tidy(node.x)} Z"/>`
        + `<path class="fold" d="M${tidy(right - fold)} ${tidy(node.y)} V${tidy(node.y + fold)} H${tidy(right)}"/>`
        + textOf(SHAPES.note.blockOf(node, node.width), 'message', node.x + 10, node.y + 20, 'start');
    },
    box(node, target) {
      const { label, detail } = SHAPES.box.blocksOf(node);
      const centre = node.x + (node.width - (target ? LINK_CONTROL_WIDTH : 0)) / 2;
      const firstBaseline = node.y + (detail ? 19 : 20);
      return bodyOf(node, 7) + textOf(label, 'name', centre, firstBaseline, 'middle')
        + (detail ? textOf(detail, 'detail', centre, firstBaseline + label.height + 1, 'middle') : '')
        + (target ? linkControl(node, target) : '') + untold(node, target);
    },
    group(node, target) {
      const heading = textOf(blockOf(String(node.label ?? node.id).toUpperCase(), 'heading'), 'heading', node.x + 14, node.y + 20, 'start');
      return bodyOf(node, 9) + (target ? `<a href="${escaped(target.href)}" aria-label="${escaped(target.label)}">${heading}</a>` : heading) + untold(node, target);
    },
  };

  function nodeMarkup(node, resolveLink) {
    const target = node.link ? resolveLink(node.link) : null;
    const name = escaped(node.label ?? UNNAMED[node.shape] ?? node.id);
    const leaf = node.inert || SHAPES[node.shape].holdsOthers ? '' : ' leaf';
    const told = node.inert ? `data-part="${escaped(node.id)}"` : `data-node="${escaped(node.id)}" tabindex="0" role="button" aria-label="Trace the arrows of ${name}"`;
    return `<g class="node ${node.shape}${leaf}${marks(node)}" ${told}>${DRAWN[node.shape](node, target)}</g>`;
  }

  function roundedPath(points, radius = CORNER, straightEnd = 0) {
    const lengthOf = (a, b) => Math.hypot(b[0] - a[0], b[1] - a[1]);
    const towards = (from, to, distance) => { const length = lengthOf(from, to) || 1; return [from[0] + (to[0] - from[0]) * distance / length, from[1] + (to[1] - from[1]) * distance / length]; };
    let path = `M${tidy(points[0][0])} ${tidy(points[0][1])}`;
    for (let i = 1; i < points.length - 1; i++) {
      const corner = points[i], isLast = i === points.length - 2;
      const room = isLast && straightEnd ? lengthOf(corner, points[i + 1]) - straightEnd : lengthOf(corner, points[i + 1]) / 2;
      const r = Math.max(0, Math.min(radius, lengthOf(points[i - 1], corner) / 2, room));
      if (!r) { path += ` L${tidy(corner[0])} ${tidy(corner[1])}`; continue; }
      const before = towards(corner, points[i - 1], r), after = towards(corner, points[i + 1], r);
      path += ` L${tidy(before[0])} ${tidy(before[1])} Q${tidy(corner[0])} ${tidy(corner[1])} ${tidy(after[0])} ${tidy(after[1])}`;
    }
    const last = points[points.length - 1];
    return `${path} L${tidy(last[0])} ${tidy(last[1])}`;
  }

  const unitFrom = (from, to) => { const length = Math.hypot(to[0] - from[0], to[1] - from[1]) || 1; return [(to[0] - from[0]) / length, (to[1] - from[1]) / length]; };

  function headOf(kind, points) {
    if (!kind || kind === 'none' || points.length < 2) return { path: '', cut: 0 };
    const tip = points[points.length - 1], along = unitFrom(points[points.length - 2], tip);
    const [long, half] = kind === 'hollow' ? [HOLLOW_LENGTH, HOLLOW_HALF_WIDTH] : [HEAD_LENGTH, HEAD_HALF_WIDTH];
    const base = [tip[0] - along[0] * long, tip[1] - along[1] * long];
    const wings = [-1, 1].map(side => [base[0] - along[1] * half * side, base[1] + along[0] * half * side])
      .sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    const strokes = `M${tidy(wings[0][0])} ${tidy(wings[0][1])} L${tidy(tip[0])} ${tidy(tip[1])} L${tidy(wings[1][0])} ${tidy(wings[1][1])}`;
    return kind === 'open' ? { path: strokes, cut: 0 } : { path: `${strokes} Z`, cut: long };
  }

  function tailOf(kind, points) {
    if (!kind || kind === 'none' || points.length < 2) return { path: '', cut: 0 };
    const start = points[0], along = unitFrom(start, points[1]);
    const middle = [start[0] + along[0] * DIAMOND_LENGTH / 2, start[1] + along[1] * DIAMOND_LENGTH / 2];
    const far = [start[0] + along[0] * DIAMOND_LENGTH, start[1] + along[1] * DIAMOND_LENGTH];
    const side = sign => [middle[0] + sign * along[1] * DIAMOND_HALF_WIDTH, middle[1] - sign * along[0] * DIAMOND_HALF_WIDTH];
    const corners = [start, side(1), far, side(-1)].map(([x, y]) => `${tidy(x)} ${tidy(y)}`);
    return { path: `M${corners.join(' L')} Z`, cut: DIAMOND_LENGTH };
  }

  const CROWS = ['one', 'zero-or-one', 'one-or-many', 'zero-or-many'];
  const CROW_REACH = 20;

  function crowAt(kind, at, towards) {
    const along = unitFrom(at, towards), across = [-along[1], along[0]];
    const out = distance => [at[0] + along[0] * distance, at[1] + along[1] * distance];
    const spot = ([x, y]) => `${tidy(x)} ${tidy(y)}`;
    const bar = distance => { const [x, y] = out(distance); return `M${spot([x + across[0] * 6, y + across[1] * 6])} L${spot([x - across[0] * 6, y - across[1] * 6])}`; };
    const ring = distance => { const [x, y] = out(distance); return `M${spot([x - 4, y])} a4 4 0 1 0 8 0 a4 4 0 1 0 -8 0`; };
    const foot = `M${spot(out(10))} L${spot([at[0] + across[0] * 6, at[1] + across[1] * 6])} M${spot(out(10))} L${spot([at[0] - across[0] * 6, at[1] - across[1] * 6])}`;
    return { one: `${bar(8)} ${bar(12)}`, 'zero-or-one': `${bar(7)} ${ring(16)}`, 'one-or-many': `${foot} ${bar(14)}`, 'zero-or-many': `${foot} ${ring(16)}` }[kind];
  }

  function endLabelAt(point, next, clearance, side = 1) {
    const along = unitFrom(point, next), reach = clearance + END_LABEL_GAP;
    if (along[0] === 0) return { x: point[0] + 7 * side, y: tidyNumber(point[1] + along[1] * reach + 4), anchor: side > 0 ? 'start' : 'end' };
    return { x: tidyNumber(point[0] + along[0] * (clearance + 6)), y: point[1] + (side > 0 ? -6 : 13), anchor: along[0] > 0 ? 'start' : 'end' };
  }

  function shortened(points, cut, tailCut = 0) {
    const trimmed = points.map(point => [...point]);
    if (cut) {
      const tip = trimmed[trimmed.length - 1], along = unitFrom(trimmed[trimmed.length - 2], tip);
      trimmed[trimmed.length - 1] = [tip[0] - along[0] * cut, tip[1] - along[1] * cut];
    }
    if (tailCut) {
      const along = unitFrom(trimmed[0], trimmed[1]);
      trimmed[0] = [trimmed[0][0] + along[0] * tailCut, trimmed[0][1] + along[1] * tailCut];
    }
    return trimmed;
  }

  function edgeLabelMarkup(edge, target) {
    if (!edge.label || !edge.labelBox) return '';
    const { x, y, width, height } = edge.labelBox;
    const opened = text => (target ? `<a href="${escaped(target.href)}"${attributesOf(target)} aria-label="${escaped(target.label)}">${text}</a>` : text);
    if (edge.plate === false) return opened(textOf(blockOf(edge.label, 'message', width), 'message', x + width / 2, y + 11, 'middle'));
    const block = blockOf(edge.label, 'edge', width - 12);
    return `<rect x="${tidy(x)}" y="${tidy(y)}" width="${tidy(width)}" height="${tidy(height)}" rx="9"/>${opened(textOf(block, 'edge-text', x + width / 2, y + (height - block.height) / 2 + 11, 'middle'))}`;
  }

  function endText(text, box, point, next, clearance, side) {
    if (!text) return '';
    if (box) return textOf(blockOf(text, 'end'), 'end', box.x, box.y + 11, 'start');
    const { x, y, anchor } = endLabelAt(point, next, clearance, side);
    return textOf(blockOf(text, 'end'), 'end', x, y, anchor);
  }

  function edgeMarkup(edge, resolveLink) {
    if (edge.points.length < 2) return '';
    const target = edge.link ? resolveLink(edge.link) : null;
    const last = edge.points.length - 1;
    const crowed = (kind, end, at, towards) => (CROWS.includes(kind) ? { path: crowAt(kind, at, towards), cut: 0, crow: true, cls: `${end} crow`, clear: CROW_REACH } : null);
    const head = crowed(edge.head, 'head', edge.points[last], edge.points[last - 1]) ?? headOf(edge.head, edge.points);
    const tail = crowed(edge.tail, 'tail', edge.points[0], edge.points[1]) ?? tailOf(edge.tail, edge.points);
    const straight = head.crow ? CROW_REACH : head.path ? Math.max(HEAD_LENGTH - head.cut, 0) : 0;
    const line = `<path class="line" d="${roundedPath(shortened(edge.points, head.cut, tail.cut), CORNER, straight)}"/>`;
    const arrow = (tail.path ? `<path class="${tail.cls ?? `tail ${edge.tail}`}" d="${tail.path}"/>` : '') + (head.path ? `<path class="${head.cls ?? `head ${edge.head}`}" d="${head.path}"/>` : '');
    const ends = endText(edge.fromLabel, edge.fromLabelBox, edge.points[0], edge.points[1], tail.clear ?? tail.cut, -1)
      + endText(edge.toLabel, edge.toLabelBox, edge.points[last], edge.points[last - 1], head.clear ?? (head.path ? Math.max(head.cut, HEAD_LENGTH) : 0), 1);
    return `<g class="edge${edge.line === 'dashed' ? ' dashed' : ''}${marks(edge)}" data-edge="${escaped(edge.id)}" data-from="${escaped(edge.from)}" data-to="${escaped(edge.to)}">${line}${arrow}${ends}${edgeLabelMarkup(edge, target)}${untold(edge, target)}</g>`;
  }

  function captionsOf(markup) {
    const words = (markup.match(/<text[^>]*>.*?<\/text>/g) || []).join('');
    return words ? `<g class="captions" aria-hidden="true">${words}</g>` : '';
  }

  function svgOf(scene, { uid = 'diagram', title = '', resolveLink = addressOnly } = {}) {
    const width = scene.width + 2 * MARGIN, height = scene.height + 2 * MARGIN;
    const isBehind = node => node.behind || SHAPES[node.shape].holdsOthers;
    const holders = scene.nodes.filter(isBehind), leaves = scene.nodes.filter(node => !isBehind(node));
    const draw = nodes => nodes.map(node => nodeMarkup(node, resolveLink)).join('');
    return `<svg class="dk" data-graph="${escaped(uid)}" xmlns="http://www.w3.org/2000/svg" width="${tidy(width)}" height="${tidy(height)}" viewBox="${-MARGIN} ${-MARGIN} ${tidy(width)} ${tidy(height)}" role="img" aria-label="${escaped(title)}">`
      + `${draw(holders)}${scene.edges.map(edge => edgeMarkup(edge, resolveLink)).join('')}${captionsOf(draw(holders))}${draw(leaves)}</svg>`;
  }

  function legendOf(scene) {
    const items = [...scene.nodes, ...scene.edges];
    const keys = [
      ...Object.keys(STATUSES).filter(status => items.some(item => item.status === status)).map(status => `<span><span class="dk-key s-${status}"></span>${STATUSES[status]}</span>`),
      ...Object.keys(TONES).filter(tone => items.some(item => item.tone === tone)).map(tone => `<span><span class="dk-key t-${tone}"></span>${TONES[tone]}</span>`),
    ];
    const spelled = (scene.key || []).map(([short, meaning]) => `<span><b>${escaped(short)}</b> ${escaped(meaning)}</span>`);
    return keys.length || spelled.length ? `<div class="dk-legend">${[...keys, ...spelled].join('')}</div>` : '';
  }

  const api = { svgOf, legendOf, roundedPath, headOf, tailOf, endLabelAt, crowAt };
  if (inNode) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
