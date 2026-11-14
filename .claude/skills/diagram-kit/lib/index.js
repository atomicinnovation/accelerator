const fs = require('node:fs');
const path = require('node:path');

const { notations, markProblems, STATUSES, TONES } = require('./notations.js');
const { svgOf } = require('./render.js');
const { standalonePage, standaloneSvg } = require('./page.js');

const SKILLS = path.resolve(__dirname, '..', '..');

function notationsUnder(directory) {
  return fs.readdirSync(directory, { withFileTypes: true })
    .filter(entry => entry.isDirectory() && entry.name.startsWith('draw-'))
    .map(entry => path.join(directory, entry.name, 'notation.js'))
    .filter(file => fs.existsSync(file))
    .sort()
    .map(file => require(file));
}

const kitWith = (directory = SKILLS) => notations(notationsUnder(directory));

module.exports = { kitWith, notations, markProblems, STATUSES, TONES, svgOf, standalonePage, standaloneSvg };
