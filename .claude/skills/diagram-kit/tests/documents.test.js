const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const { kitWith } = require('../lib/index.js');

const SKILLS = path.resolve(__dirname, '..', '..');
const NAMED = /(?<![\w/.-])((?:lib|bin|tools|vendor|reference|examples|tests)\/[\w./-]*\w)/g;

function documentsOf(skill) {
  const root = path.join(SKILLS, skill), reference = path.join(root, 'reference');
  const references = fs.existsSync(reference) ? fs.readdirSync(reference).map(name => path.join(reference, name)) : [];
  return [path.join(root, 'SKILL.md'), ...references].filter(file => file.endsWith('.md'));
}

const drawing = fs.readdirSync(SKILLS).filter(name => name.startsWith('draw-'));

for (const skill of ['diagram-kit', ...drawing]) {
  test(`every file the ${skill} documents name exists in it, the kit or a notation`, () => {
    const missing = documentsOf(skill).flatMap(document => [...fs.readFileSync(document, 'utf8').matchAll(NAMED)]
      .map(match => match[1])
      .filter(named => ![skill, 'diagram-kit', ...drawing].some(owner => fs.existsSync(path.join(SKILLS, owner, named))))
      .map(named => `${path.basename(document)}: ${named}`));
    assert.deepEqual(missing, []);
  });
}

const CHOOSER = 'draw-diagram';
const notations = drawing.filter(skill => skill !== CHOOSER);

test('the guide to choosing a diagram places every kind the kit draws, and no other', () => {
  const guide = fs.readFileSync(path.join(SKILLS, CHOOSER, 'reference', 'choosing.md'), 'utf8');
  const placed = [...new Set([...guide.matchAll(/^\|[^|]*\| `([a-z0-9-]+)` \|/gm)].map(match => match[1]))].sort();

  assert.deepEqual(placed, kitWith().kinds().sort());
});

test('the guide names the skill that draws each kind', () => {
  const guide = fs.readFileSync(path.join(SKILLS, CHOOSER, 'reference', 'choosing.md'), 'utf8');
  const named = [...new Set([...guide.matchAll(/`(draw-[a-z0-9-]+)`/g)].map(match => match[1]))].sort();

  assert.deepEqual(named, [...notations].sort());
});

test('the skill that chooses a diagram draws none itself', () => {
  assert.equal(fs.existsSync(path.join(SKILLS, CHOOSER, 'notation.js')), false);
  assert.ok(fs.existsSync(path.join(SKILLS, CHOOSER, 'SKILL.md')));
});

for (const skill of notations) {
  test(`${skill} comes with what every notation needs`, () => {
    for (const needed of ['SKILL.md', 'notation.js', 'reference/notation.md', 'examples', 'tests']) {
      assert.ok(fs.existsSync(path.join(SKILLS, skill, needed)), `${skill} has no ${needed}`);
    }
  });
}

for (const skill of ['diagram-kit', ...drawing]) {
  test(`the description of ${skill} is one a loader reads to its end`, () => {
    const frontMatter = fs.readFileSync(path.join(SKILLS, skill, 'SKILL.md'), 'utf8').split('\n---')[0];
    const description = frontMatter.slice(frontMatter.indexOf('description:') + 'description:'.length).split(/\n[a-z-]+:/)[0];

    assert.doesNotMatch(description, /:\s/);
    assert.ok(description.trim().length > 80);
  });
}
