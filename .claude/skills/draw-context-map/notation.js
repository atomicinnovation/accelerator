const { markProblems } = require('../diagram-kit/lib/notations.js');
const { cyclesIn, loopThrough } = require('../diagram-kit/lib/graphs.js');

const DIRECTIONS = ['down', 'right'];
const SUBDOMAINS = ['core', 'supporting', 'generic'];
const BETWEEN_EQUALS = ['partnership', 'shared kernel'];
const PATTERNS = { upstreamPatterns: ['open host service', 'published language'], downstreamPatterns: ['anticorruption layer', 'conformist'] };
const SHORT = { 'open host service': 'OHS', 'published language': 'PL', 'anticorruption layer': 'ACL', conformist: 'CF' };

const listOf = value => (Array.isArray(value) ? value : []);
const isText = value => typeof value === 'string' && value.length > 0;
const capitalised = word => word[0].toUpperCase() + word.slice(1);
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const isDirected = relationship => 'upstream' in relationship || 'downstream' in relationship;
const endsOf = relationship => (isDirected(relationship) ? [relationship.upstream, relationship.downstream] : listOf(relationship.between));

function contextProblems(contexts) {
  const problems = [], known = new Set();
  contexts.forEach((context, position) => {
    if (!context || !isText(context.id)) { problems.push(`context ${position + 1}: no id`); return; }
    const name = `context ${context.id}`;
    if (known.has(context.id)) { problems.push(`${name}: id is used twice`); return; }
    known.add(context.id);
    if (!isText(context.name)) { problems.push(`${name}: no name`); return; }
    problems.push(
      ...(context.subdomain == null || SUBDOMAINS.includes(context.subdomain) ? [] : [`${name}: subdomain "${context.subdomain}" is not one of ${SUBDOMAINS.join(', ')}`]),
      ...(context.external == null || typeof context.external === 'boolean' ? [] : [`${name}: external must be true or false`]),
      ...['team', 'description'].filter(field => context[field] != null && typeof context[field] !== 'string').map(field => `${name}: ${field} must be text`),
      ...markProblems(context, name),
    );
  });
  return { problems, known };
}

function patternProblems(relationship, name) {
  return Object.entries(PATTERNS).flatMap(([field, allowed]) => {
    if (relationship[field] == null) return [];
    if (!Array.isArray(relationship[field])) return [`${name}: ${field} must be a list`];
    return relationship[field].filter(pattern => !allowed.includes(pattern)).map(pattern => `${name}: ${field} "${pattern}" is not one of ${allowed.join(', ')}`);
  });
}

function shapeProblems(relationship, name) {
  if (!isDirected(relationship)) {
    if (relationship.kind == null) return [`${name}: no kind; say ${BETWEEN_EQUALS.join(' or ')}`];
    return [
      ...(BETWEEN_EQUALS.includes(relationship.kind) ? [] : [`${name}: kind "${relationship.kind}" is not one of ${BETWEEN_EQUALS.join(', ')}`]),
      ...Object.keys(PATTERNS).filter(field => relationship[field] != null).map(field => `${name}: a relationship between equals has no ${field}`),
    ];
  }
  const kind = relationship.kind == null || relationship.kind === 'customer-supplier' ? [] : [`${name}: kind "${relationship.kind}" is not customer-supplier; a partnership or a shared kernel is between equals`];
  const patterns = patternProblems(relationship, name);
  if (kind.length || patterns.length) return [...kind, ...patterns];
  const downstream = listOf(relationship.downstreamPatterns);
  if (downstream.includes('conformist') && downstream.includes('anticorruption layer')) return [`${name}: a downstream either conforms or keeps an anticorruption layer, not both`];
  if (downstream.includes('conformist') && relationship.kind === 'customer-supplier') return [`${name}: a conformist takes what it is given, so it is not a customer`];
  return [];
}

function relationshipProblems(relationships, known) {
  const written = new Set();
  return relationships.flatMap((relationship, position) => {
    const numbered = `relationship ${position + 1}`;
    if (relationship === null || typeof relationship !== 'object') return [`${numbered}: needs an upstream and a downstream, or two contexts it is between`];
    if (isDirected(relationship) && relationship.between != null) return [`${numbered}: has an upstream and a downstream, so it is not between equals`];
    const ends = endsOf(relationship);
    if (ends.length !== 2 || !ends.every(isText)) return [`${numbered}: needs an upstream and a downstream, or two contexts it is between`];
    const name = `relationship ${ends.join(isDirected(relationship) ? '->' : '--')}`;
    const unknown = ends.find(end => !known.has(end));
    if (unknown) return [`${name}: no context "${unknown}"`];
    if (ends[0] === ends[1]) return [`${name}: a context is not ${isDirected(relationship) ? 'upstream of' : 'related to'} itself`];
    const shaped = shapeProblems(relationship, name);
    if (shaped.length) return shaped;
    const key = (isDirected(relationship) ? ends : [...ends].sort()).join('\n') + (isDirected(relationship) ? '\n>' : `\n${relationship.kind}`);
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return markProblems(relationship, name);
  });
}

function problemsIn(spec) {
  if (!listOf(spec.contexts).length) return ['no contexts'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, known } = contextProblems(spec.contexts);
  return [...direction, ...problems, ...relationshipProblems(listOf(spec.relationships), known)];
}

function remarksOn(spec) {
  const relationships = listOf(spec.relationships), byId = new Map(spec.contexts.map(context => [context.id, context])), remarks = [];
  const related = new Set(relationships.flatMap(endsOf));
  const alone = spec.contexts.filter(context => !related.has(context.id)).map(context => context.name);
  if (relationships.length && alone.length) remarks.push(`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} related to nothing on the map`);
  const influence = relationships.filter(isDirected).map(each => ({ from: each.upstream, to: each.downstream }));
  remarks.push(...cyclesIn(influence).map(knot => `Influence runs in a ring: ${loopThrough(knot, influence).map(id => byId.get(id).name).join(' → ')}`));
  for (const each of relationships.filter(isDirected)) {
    const upstream = byId.get(each.upstream), downstream = byId.get(each.downstream), taken = listOf(each.downstreamPatterns);
    if (upstream.external && !taken.length) remarks.push(`${downstream.name} is downstream of ${upstream.name}, which is not ours, and says neither that it conforms nor that it keeps an anticorruption layer`);
    if (downstream.subdomain === 'core' && taken.includes('conformist')) remarks.push(`${downstream.name} is core, yet conforms to ${upstream.name}'s model`);
  }
  return remarks;
}

const endLabel = (letter, patterns) => [letter, listOf(patterns).map(pattern => SHORT[pattern]).join(', ')].filter(Boolean).join(' · ');
const named = (kind, label) => [kind, label].filter(Boolean).join(': ') || undefined;

function sceneOf(spec) {
  const relationships = listOf(spec.relationships), directed = relationships.filter(isDirected);
  const used = new Set(directed.flatMap(each => [...listOf(each.upstreamPatterns), ...listOf(each.downstreamPatterns)]));
  const key = directed.length ? [['U', 'upstream'], ['D', 'downstream'], ...Object.entries(SHORT).filter(([pattern]) => used.has(pattern)).map(([pattern, short]) => [short, pattern])] : undefined;
  return {
    direction: spec.direction || 'down',
    ...(key ? { key } : {}),
    nodes: spec.contexts.map(context => ({
      id: context.id, shape: 'element', label: context.name, description: context.description, muted: context.external ? true : undefined,
      type: [context.external ? 'External' : context.subdomain ? capitalised(context.subdomain) : null, context.team].filter(Boolean).join(' · ') || undefined,
      status: context.status, tone: context.tone, link: context.link,
    })),
    edges: relationships.map(each => {
      const marks = { status: each.status, tone: each.tone, link: each.link }, [from, to] = endsOf(each);
      if (!isDirected(each)) return { id: `${from}--${to}`, from, to, line: 'solid', head: 'none', label: named(each.kind, each.label), ...marks };
      return {
        id: `${from}->${to}`, from, to, line: 'solid', head: 'none', label: named(each.kind === 'customer-supplier' ? 'customer/supplier' : null, each.label),
        fromLabel: endLabel('U', each.upstreamPatterns), toLabel: endLabel('D', each.downstreamPatterns), ...marks,
      };
    }),
  };
}

module.exports = { kind: 'context-map', name: 'Context map (domain-driven design)', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
