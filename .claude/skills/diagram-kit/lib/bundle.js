const fs = require('node:fs');
const path = require('node:path');

const KIT = path.resolve(__dirname, '..');
const SKILLS = path.resolve(KIT, '..');
const REQUIRED = /\brequire\((['"])([^'"]+)\1\)/g;

const isVendored = file => file.startsWith(path.join(KIT, 'vendor') + path.sep);

function notationFilesUnder(directory) {
  return fs.readdirSync(directory, { withFileTypes: true })
    .filter(entry => entry.isDirectory() && entry.name.startsWith('draw-'))
    .map(entry => path.join(directory, entry.name, 'notation.js'))
    .filter(file => fs.existsSync(file))
    .sort();
}

function modulesFrom(entries) {
  const ids = new Map(), modules = [];
  const add = file => {
    if (ids.has(file)) return ids.get(file);
    const id = modules.length, source = fs.readFileSync(file, 'utf8'), needs = {};
    ids.set(file, id);
    modules.push({ source, needs });
    for (const [, , named] of isVendored(file) ? [] : source.matchAll(REQUIRED)) {
      if (!named.startsWith('.')) throw new Error(`${path.relative(SKILLS, file)} requires ${named.replace(/^node:/, '')}; a browser cannot load it`);
      needs[named] = add(path.resolve(path.dirname(file), named));
    }
    return id;
  };
  return { idOf: entries.map(add), modules };
}

// Returns a script that gives a page `DiagramKit.live`: the kit with every
// notation under the directory, laying diagrams out in the page itself.
function browserBundle(notationsDirectory = SKILLS) {
  const notations = notationFilesUnder(notationsDirectory);
  const { idOf: [kit, ...drawn], modules } = modulesFrom([path.join(KIT, 'lib/notations.js'), ...notations]);
  const table = modules.map(({ source, needs }) => `[function (module, exports, require) {\n${source}\n}, ${JSON.stringify(needs)}]`).join(',\n');
  return `(function (root) {
  'use strict';
  var modules = [${table}];
  var loaded = [];
  function load(id) {
    if (loaded[id]) return loaded[id].exports;
    var module = loaded[id] = { exports: {} };
    modules[id][0].call(module.exports, module, module.exports, function (named) { return load(modules[id][1][named]); });
    return module.exports;
  }
  var live = load(${kit}).notations(${JSON.stringify(drawn)}.map(load));
  root.DiagramKit = Object.assign(root.DiagramKit || {}, { live: live });
})(typeof window !== 'undefined' ? window : globalThis);
`;
}

module.exports = { browserBundle };
