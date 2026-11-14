const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

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

for (const skill of drawing) {
  test(`${skill} comes with what every notation needs`, () => {
    for (const needed of ['SKILL.md', 'notation.js', 'reference/notation.md', 'examples', 'tests']) {
      assert.ok(fs.existsSync(path.join(SKILLS, skill, needed)), `${skill} has no ${needed}`);
    }
  });
}
