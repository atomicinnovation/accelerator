#!/usr/bin/env node
// usage: diagram.js check SPEC.json
//        diagram.js layout SPEC.json [--out SCENE.json]
//        diagram.js render SPEC.json --out FILE.html|FILE.svg
//
//        diagram.js prepare SPECS.json
//        diagram.js bundle [--out KIT.js]
//
// SPEC.json is one diagram, or a list of them. `check` reports every problem
// and exits 1 if there is one. `layout` writes where every node and edge
// goes. `render` writes a page that opens offline, or one drawing as SVG.
// `prepare` is for a page that embeds diagrams: for each one it prints the
// problems, the laid-out scene, the legend and the remarks, and it fails
// only if the file cannot be read.
// `--legend` puts under each figure what its notation's symbols mean.
// `--notations DIR` reads the draw-* skills under DIR instead of the ones
// beside this kit.

const fs = require('node:fs');
const path = require('node:path');
const { kitWith, svgOf, standalonePage, standaloneSvg } = require('../lib/index.js');
const { legendOf } = require('../lib/render.js');
const { browserBundle } = require('../lib/bundle.js');

const USAGE = 'usage: diagram.js check|layout|render|prepare SPEC.json [--out FILE] [--legend] [--notations DIR]\n       diagram.js bundle [--out FILE] [--notations DIR]';

function parsed(argv) {
  const options = { positional: [] };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--out') options.out = argv[++i];
    else if (argv[i] === '--notations') options.notations = argv[++i];
    else if (argv[i] === '--legend') options.legend = true;
    else options.positional.push(argv[i]);
  }
  return options;
}

class Refused extends Error {}

function diagramsIn(file) {
  let value;
  try { value = JSON.parse(fs.readFileSync(file, 'utf8')); } catch (error) {
    throw new Refused(`${path.basename(file)}: ${error instanceof SyntaxError ? `not valid JSON: ${error.message}` : error.message}`);
  }
  return Array.isArray(value) ? value : [value];
}

function refuseProblems(kit, diagrams) {
  const problems = diagrams.flatMap(diagram => kit.problemsIn(diagram));
  if (problems.length) throw new Refused(problems.join('\n'));
}

async function prepared(kit, diagram, legend) {
  const id = diagram && typeof diagram.id === 'string' ? diagram.id : null;
  const problems = kit.faultsIn(diagram);
  const selfDrawn = kit.drawsItself(diagram) ? { selfDrawn: true } : {};
  if (problems.length) return { id, problems, scene: null, legend: '', remarks: [], ...selfDrawn };
  const scene = await kit.laidOut(diagram);
  const symbols = legend ? { symbols: kit.symbolsOf(diagram, scene) } : {};
  return { id, problems, scene, legend: scene ? legendOf(scene) : '', remarks: kit.remarksOn(diagram), ...selfDrawn, ...symbols };
}

async function main(argv) {
  const { positional: [command, file], out, notations, legend } = parsed(argv);
  if (command === 'bundle') {
    const bundle = browserBundle(notations ? path.resolve(notations) : undefined);
    if (out) fs.writeFileSync(out, bundle); else process.stdout.write(bundle);
    return 0;
  }
  if (!['check', 'layout', 'render', 'prepare'].includes(command) || !file) { console.error(USAGE); return 2; }
  const kit = kitWith(notations ? path.resolve(notations) : undefined);
  const diagrams = diagramsIn(file);
  if (command === 'prepare') {
    const each = [];
    for (const diagram of diagrams) each.push(await prepared(kit, diagram, legend));
    process.stdout.write(`${JSON.stringify({ kinds: kit.kinds(), diagrams: each })}\n`);
    return 0;
  }
  if (!diagrams.length) throw new Refused(`${path.basename(file)}: no diagrams in it`);
  refuseProblems(kit, diagrams);
  if (command === 'check') return 0;
  const onPageOnly = diagrams.find(diagram => kit.drawsItself(diagram));
  if (onPageOnly && command === 'layout') throw new Refused(`diagram ${onPageOnly.id}: a ${onPageOnly.kind} draws itself, so it has no scene`);
  if (onPageOnly && out && out.endsWith('.svg')) throw new Refused(`diagram ${onPageOnly.id}: a ${onPageOnly.kind} draws itself; render it to .html`);
  const scenes = [];
  for (const diagram of diagrams) scenes.push(await kit.laidOut(diagram));
  if (command === 'layout') {
    const text = `${JSON.stringify(diagrams.length === 1 ? scenes[0] : scenes, null, 1)}\n`;
    if (out) fs.writeFileSync(out, text); else process.stdout.write(text);
    return 0;
  }
  if (!out || !/\.(html|svg)$/.test(out)) { console.error(USAGE); return 2; }
  if (out.endsWith('.svg')) {
    if (diagrams.length !== 1) throw new Refused('an .svg holds one diagram; render a list to .html');
    fs.writeFileSync(out, standaloneSvg(svgOf(scenes[0], { uid: diagrams[0].id, title: diagrams[0].title })));
  } else {
    fs.writeFileSync(out, standalonePage(diagrams[0].title, diagrams.map((diagram, i) => kit.figureOf(diagram, scenes[i], { legend }))));
  }
  console.log(`wrote ${out}`);
  return 0;
}

main(process.argv.slice(2)).then(
  status => { process.exitCode = status; },
  error => {
    if (!(error instanceof Refused) && !/^diagram /.test(error.message)) throw error;
    for (const line of error.message.split('\n')) console.error(`warning: ${line}`);
    process.exitCode = 1;
  },
);
