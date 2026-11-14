const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];
const KEYS = { primary: 'PK', foreign: 'FK', unique: 'UK' };
const CARDINALITIES = ['one', 'zero-or-one', 'one-or-many', 'zero-or-many'];
const MANY = ['one-or-many', 'zero-or-many'];

const listOf = value => (Array.isArray(value) ? value : []);
const nameOfEntity = entity => entity.name ?? entity.id;
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const keysOf = attribute => (attribute.key == null ? [] : Array.isArray(attribute.key) ? attribute.key : [attribute.key]);
const asAttribute = attribute => (typeof attribute === 'string' ? { name: attribute } : attribute);
const nameOf = (relationship, position) => (typeof relationship.from === 'string' && typeof relationship.to === 'string' ? `relationship ${relationship.from}->${relationship.to}` : `relationship ${position + 1}`);

function attributeProblems(attributes, entity) {
  const seen = new Set();
  return attributes.flatMap((written, position) => {
    const attribute = asAttribute(written);
    if (attribute === null || typeof attribute !== 'object') return [`${entity}: attribute ${position + 1}: must be a name, or an object with a name`];
    if (typeof attribute.name !== 'string' || !attribute.name) return [`${entity}: attribute ${position + 1}: no name`];
    const name = `${entity}: attribute ${attribute.name}`;
    if (seen.has(attribute.name)) return [`${name}: name is used twice`];
    seen.add(attribute.name);
    return [
      ...keysOf(attribute).filter(key => !KEYS[key]).map(key => `${name}: key "${key}" is not one of ${Object.keys(KEYS).join(', ')}`),
      ...(attribute.type == null || typeof attribute.type === 'string' ? [] : [`${name}: type must be text`]),
      ...(attribute.nullable == null || typeof attribute.nullable === 'boolean' ? [] : [`${name}: nullable must be true or false`]),
      ...markProblems(attribute, name),
    ];
  });
}

function entityProblems(entities) {
  const problems = [], known = new Set();
  entities.forEach((entity, position) => {
    if (!entity || typeof entity.id !== 'string' || !entity.id) { problems.push(`entity ${position + 1}: no id`); return; }
    const name = `entity ${entity.id}`;
    if (known.has(entity.id)) { problems.push(`${name}: id is used twice`); return; }
    known.add(entity.id);
    if (entity.attributes != null && !Array.isArray(entity.attributes)) problems.push(`${name}: attributes must be a list`);
    problems.push(...attributeProblems(listOf(entity.attributes), name), ...markProblems(entity, name));
  });
  return { problems, known };
}

function relationshipProblems(relationships, known) {
  const written = new Set();
  return relationships.flatMap((relationship, position) => {
    const name = nameOf(relationship, position);
    const missing = ['from', 'to'].find(end => typeof relationship[end] !== 'string' || !relationship[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [relationship.from, relationship.to].find(end => !known.has(end));
    if (unknown) return [`${name}: no entity "${unknown}"`];
    const ends = ['fromCardinality', 'toCardinality'].flatMap(end => {
      if (relationship[end] == null) return [`${name}: no ${end}`];
      return CARDINALITIES.includes(relationship[end]) ? [] : [`${name}: ${end} "${relationship[end]}" is not one of ${CARDINALITIES.join(', ')}`];
    });
    if (ends.length) return ends;
    const key = [relationship.from, relationship.to, relationship.label ?? ''].join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return [...markProblems(relationship, name), ...(relationship.identifying == null || typeof relationship.identifying === 'boolean' ? [] : [`${name}: identifying must be true or false`])];
  });
}

function problemsIn(spec) {
  if (!listOf(spec.entities).length) return ['no entities'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, known } = entityProblems(spec.entities);
  return [...direction, ...problems, ...relationshipProblems(listOf(spec.relationships), known)];
}

function remarksOn(spec) {
  const relationships = listOf(spec.relationships), names = new Map(spec.entities.map(entity => [entity.id, nameOfEntity(entity)]));
  const unkeyed = spec.entities.filter(entity => !listOf(entity.attributes).map(asAttribute).some(attribute => keysOf(attribute).includes('primary')))
    .map(entity => `${nameOfEntity(entity)} has no primary key`);
  const related = new Set(relationships.flatMap(each => [each.from, each.to]));
  const alone = spec.entities.filter(entity => !related.has(entity.id)).map(nameOfEntity);
  const crossed = relationships.filter(each => MANY.includes(each.fromCardinality) && MANY.includes(each.toCardinality))
    .map(each => `${names.get(each.from)} and ${names.get(each.to)} are related many to many; in a database that needs a table between them`);
  return [...unkeyed, ...(relationships.length && alone.length ? [`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} related to nothing in the diagram`] : []), ...crossed];
}

function rowOf(written) {
  const attribute = asAttribute(written), keys = keysOf(attribute).map(key => KEYS[key]);
  return {
    key: keys.length ? keys.join(', ') : undefined, name: attribute.name,
    type: attribute.type ? `${attribute.type}${attribute.nullable ? ' ?' : ''}` : undefined, status: attribute.status,
  };
}

function sceneOf(spec) {
  const drawn = new Map();
  return {
    direction: spec.direction || 'down',
    nodes: spec.entities.map(entity => ({ id: entity.id, shape: 'entity', label: nameOfEntity(entity), rows: listOf(entity.attributes).map(rowOf), status: entity.status, tone: entity.tone, link: entity.link })),
    edges: listOf(spec.relationships).map(relationship => {
      const ends = `${relationship.from}->${relationship.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      return {
        id: nth > 1 ? `${ends}:${nth}` : ends, from: relationship.from, to: relationship.to, label: relationship.label,
        line: relationship.identifying === false ? 'dashed' : 'solid', tail: relationship.fromCardinality, head: relationship.toCardinality,
        status: relationship.status, tone: relationship.tone, link: relationship.link,
      };
    }),
  };
}

module.exports = { kind: 'entity-relationship-diagram', problemsIn, sceneOf, remarksOn };
