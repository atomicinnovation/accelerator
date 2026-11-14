const { markProblems } = require('../diagram-kit/lib/notations.js');
const { widthOf } = require('../diagram-kit/lib/text.js');

const UNITS = {
  bit: { perRow: 32, narrowest: 22, widest: 30 },
  byte: { perRow: 8, narrowest: 56, widest: 84 },
};
const ROW_HEIGHT = 38, SCALE_HEIGHT = 16, NAME_ROOM = 10, MARK_GAP = 6, BREAK_ROOM = 10, LONGEST_DRAWN_IN_FULL = 2;
const listOf = value => (Array.isArray(value) ? value : []);
const unitOf = spec => spec.unit || 'bit';
const perRowOf = spec => spec.perRow || UNITS[unitOf(spec)].perRow;
const isVariable = field => field.size === 'variable';
const idOf = (field, position) => field.id || `f${position + 1}`;
const letterFor = position => (position < 26 ? String.fromCharCode(97 + position) : `a${position - 25}`);

function problemsIn(spec) {
  if (!listOf(spec.fields).length) return ['no fields'];
  const unit = UNITS[unitOf(spec)] ? [] : [`unit "${spec.unit}" is not one of ${Object.keys(UNITS).join(', ')}`];
  const row = spec.perRow == null || (Number.isInteger(spec.perRow) && spec.perRow >= 1 && spec.perRow <= 64) ? [] : ['perRow must be a whole number from 1 to 64'];
  const known = new Set(), counted = UNITS[unitOf(spec)] ? unitOf(spec) : 'bit';
  const fields = spec.fields.flatMap((field, position) => {
    if (!field || typeof field.name !== 'string' || !field.name.trim()) return [`field ${position + 1}: no name`];
    const name = `field ${field.name}`;
    if (!isVariable(field) && !(Number.isInteger(field.size) && field.size > 0)) return [`${name}: size must be a whole number of ${counted}s, or "variable"`];
    if (field.id != null && known.has(field.id)) return [`${name}: id ${field.id} is used twice`];
    known.add(idOf(field, position));
    return [...(field.reserved == null || typeof field.reserved === 'boolean' ? [] : [`${name}: reserved must be true or false`]), ...markProblems(field, name)];
  });
  return [...unit, ...row, ...fields];
}

function remarksOn(spec) {
  if (unitOf(spec) !== 'bit' || spec.fields.some(isVariable)) return [];
  const total = spec.fields.reduce((sum, field) => sum + field.size, 0);
  return total % 8 ? [`The fields total ${total} bits, which is not a whole number of bytes`] : [];
}

function partsOf(spec) {
  const perRow = perRowOf(spec), unit = unitOf(spec), parts = [], rowStarts = [];
  let row = 0, column = 0, offset = 0;
  const begin = () => { if (column === 0 && offset != null) rowStarts.push({ row, offset }); };
  const place = (field, span, extra = {}) => {
    begin();
    parts.push({ field, row, column, span, ...extra });
    column += span;
    if (column === perRow) { row += 1; column = 0; }
  };
  spec.fields.forEach((field, position) => {
    const of = { ...field, id: idOf(field, position) };
    if (isVariable(field)) {
      place(of, perRow - column, { broken: true, detail: 'variable' });
      offset = null;
      return;
    }
    let left = field.size;
    while (left > 0) {
      const wholeRows = Math.floor(left / perRow);
      if (column === 0 && wholeRows > LONGEST_DRAWN_IN_FULL) {
        place(of, perRow, { broken: true, detail: `${field.size} ${unit}s` });
        offset += wholeRows * perRow;
        left -= wholeRows * perRow;
      } else {
        const span = Math.min(perRow - column, left);
        place(of, span);
        offset = offset == null ? null : offset + span;
        left -= span;
      }
    }
  });
  return { parts, rowStarts, rows: column === 0 ? row : row + 1 };
}

function placedSceneOf(spec) {
  const perRow = perRowOf(spec), unit = unitOf(spec), { narrowest, widest } = UNITS[unit];
  const { parts, rowStarts, rows } = partsOf(spec);
  const needOf = field => widthOf(field.name, 'name') + NAME_ROOM;
  const widestPartOf = new Map();
  for (const part of parts) widestPartOf.set(part.field.id, Math.max(widestPartOf.get(part.field.id) || 0, part.span));
  const across = Math.min(widest, Math.max(narrowest, ...parts.map(part => Math.ceil(needOf(part.field) / widestPartOf.get(part.field.id)))));
  const fits = part => needOf(part.field) <= part.span * across;
  const lettered = new Map();
  for (const part of parts) if (!fits(part) && widestPartOf.get(part.field.id) * across < needOf(part.field) && !lettered.has(part.field.id)) lettered.set(part.field.id, letterFor(lettered.size));

  const marks = [unit, ...rowStarts.map(start => String(start.offset))];
  const gutter = Math.ceil(Math.max(...marks.map(mark => widthOf(mark, 'key')))) + MARK_GAP;
  const left = gutter + BREAK_ROOM, seen = new Map();
  const cells = parts.map(part => {
    const { field } = part, nth = (seen.get(field.id) || 0) + 1;
    seen.set(field.id, nth);
    return {
      id: nth === 1 ? field.id : `${field.id}/${nth}`, shape: 'field', inert: true,
      label: lettered.get(field.id) ?? (fits(part) ? field.name : ''),
      detail: part.detail, broken: part.broken, muted: field.reserved ? true : undefined,
      status: field.status, tone: field.tone, link: field.link,
      x: left + part.column * across, y: SCALE_HEIGHT + part.row * ROW_HEIGHT, width: part.span * across, height: ROW_HEIGHT,
    };
  });
  const scale = (id, label, x, y, width, height, align) => ({ id, shape: 'scale', inert: true, label, align, x, y, width, height });
  const key = [...lettered].map(([id, letter]) => [letter, parts.find(part => part.field.id === id).field.name]);
  return {
    width: left + perRow * across + 1,
    height: SCALE_HEIGHT + rows * ROW_HEIGHT + 1,
    nodes: [
      scale('unit', unit, 0, 0, gutter, SCALE_HEIGHT - 2, 'end'),
      ...Array.from({ length: perRow }, (_, at) => scale(`at:${at}`, String(at), left + at * across, 0, across, SCALE_HEIGHT - 2)),
      ...rowStarts.map(start => scale(`row:${start.offset}`, String(start.offset), 0, SCALE_HEIGHT + start.row * ROW_HEIGHT, gutter, ROW_HEIGHT, 'end')),
      ...cells,
    ],
    edges: [],
    ...(key.length ? { key } : {}),
  };
}

module.exports = { kind: 'byte-field-diagram', problemsIn, sceneOf: placedSceneOf, placedSceneOf, remarksOn };
