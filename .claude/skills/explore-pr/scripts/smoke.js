// Usage: node smoke.js PAGE.html [--shots DIR] [--chrome PATH]
//
// Opens a built explore-pr page from file:// in headless Chrome, visits every
// route the page registers, and fails on any uncaught error, unresolved code
// slice, or sideways scroll at phone width. It then reads a tour stop and checks
// that All files counts and marks its lines as read and the rest as unread, and
// marks that stop's file reviewed and checks the stop shows as already read.
// With commits, it marks a line in By commit and finds it read in All files,
// gives a file a verdict and finds its lines read in By commit, and marks one
// line at a time in All files, checking a mark link sits above each run of
// unread lines (in a file with three changed lines in a row; with none, that
// check is skipped) and that the n key jumps to the next unread line. Needs Node
// 22+ (global WebSocket). It unfolds a run of unchanged lines in a file
// opened by link and in one picked from the file list. In All files it checks
// the navigator draws every file in a directory tree, that collapsing a folder
// hides its files and opening one reveals it, that a Role facet narrows the
// tree to that role and counts what it shows without moving the controls or
// the tree, that opening a facet moves no other facet, that grouping by None
// lists every file flat and sorted across them all, that the search and
// facets stay in place while the list scrolls, that choosing a file
// keeps the list's scroll and brings the file just below the top bar, that a
// folder's checkbox selects its files for one bulk verdict that leaves a file
// with an Issue alone and that Undo reverses, that the shortcuts move between
// files and chunks, mark a chunk and undo it, raise an issue noting the chunk,
// open and close their list, move on after a verdict and step between
// commits, that By commit opens only the selected commit, that j crosses
// into the next commit, that a commit's chevron opens it without selecting it,
// that a verdict on a file in a commit reads only that
// file's lines, that the pane shows one file or the whole commit, that a line
// too long to read is cut short, that the file list widens and resets, that
// risk edges show only when asked for, that on a touch screen every
// selection checkbox shows without a hover, that a
// !term search excludes
// paths, and that a verdict updates a folder's totals in place. Last, it renders prose
// naming a stop and its chapter, and checks both become links and that the
// stop's link opens its chapter at that stop. It checks that a stop names
// everything it carries, that each name opens what it names, and that every
// line a stop says to focus on is marked in its code. Each decision's control
// is met once in the tour, at the first stop that shows it; each chapter names
// what it introduces in a strip and draws no wall of abstraction cards. It also records a file verdict,
// pretends it was given at an earlier head, and checks that rewritten lines are
// unread again and announced, that untouched ones stay read, and that only a
// new verdict, not a note, moves the record to this head. Every diagram the
// kit drew is put through the kit's own checks: text inside its box, no box
// across another or across an arrow, tracing, and a linked box opening its link.
// Each catalogue in Reference draws one card per item, and a verdict given to a
// decision there shows at the stop that first carries it. The design reads as
// its article: every section drawn, every excerpt's code resolved, the sign-off
// at the end and no catalogue of cards. Findings lists every observation, its
// source filter narrows the list, and a chapter's finding opens in Findings.
// Readiness draws the gates its model works out, turns the design gate when a
// decision is disagreed with, its high-risk gate opens All files on exactly
// the high-risk files still to read, and its verdict control drives the top bar. Your
// notes saves what is typed in its freeform box and in a card's note, puts the
// freeform notes first in the export, and keeps the focus while it saves.
// A view with figures is captured at full height, in tiles 5,000 px tall when it
// is taller (`design-1.png`, `design-2.png`, …). A DevTools call that gets no
// reply within two minutes fails the check instead of hanging it.

const { spawn, execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { DRAWING_CHECKS, SELF_DRAWN_CHECKS, TRACING_CHECK } = require('../../diagram-kit/lib/page-checks.js');
const { CALL_DEADLINE_MS, tilesOf, withinDeadline } = require('./capture.js');

const CHROME_CANDIDATES = [
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
  'google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser',
];
const sleep = ms => new Promise(r => setTimeout(r, ms));

function args(argv) {
  const out = { page: null, shots: null, chrome: process.env.CHROME || null };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--shots') out.shots = argv[++i];
    else if (argv[i] === '--chrome') out.chrome = argv[++i];
    else out.page = argv[i];
  }
  if (!out.page) throw new Error('usage: node smoke.js PAGE.html [--shots DIR] [--chrome PATH]');
  return out;
}

function findChrome(preferred) {
  for (const candidate of [preferred, ...CHROME_CANDIDATES].filter(Boolean)) {
    if (candidate.includes('/')) { if (fs.existsSync(candidate)) return candidate; continue; }
    try { return execFileSync('which', [candidate], { encoding: 'utf8' }).trim(); } catch { /* not on PATH */ }
  }
  return null;
}

async function devtools(chrome) {
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'explore-pr-smoke-'));
  const proc = spawn(chrome, ['--headless=new', '--disable-gpu', '--no-first-run', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
  const portFile = path.join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 80 && !fs.existsSync(portFile); i++) await sleep(100);
  const port = fs.readFileSync(portFile, 'utf8').split('\n')[0];
  let targets = [];
  for (let i = 0; i < 40 && !targets.some(t => t.type === 'page'); i++) {
    try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); } catch { await sleep(100); }
  }
  const ws = new WebSocket(targets.find(t => t.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  let id = 0;
  const pending = new Map();
  const errors = [];
  ws.onmessage = message => {
    const data = JSON.parse(message.data);
    if (data.id && pending.has(data.id)) { pending.get(data.id)(data); pending.delete(data.id); }
    if (data.method === 'Runtime.exceptionThrown') errors.push(data.params.exceptionDetails.exception?.description || data.params.exceptionDetails.text);
    if (data.method === 'Runtime.consoleAPICalled' && data.params.type === 'error') errors.push(data.params.args.map(a => a.value ?? a.description).join(' '));
  };
  const send = (method, params = {}) => {
    const n = ++id;
    const reply = new Promise(resolve => { pending.set(n, resolve); ws.send(JSON.stringify({ id: n, method, params })); });
    return withinDeadline(reply, CALL_DEADLINE_MS, method).finally(() => pending.delete(n));
  };
  const evaluate = async expression => (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result.result.value;
  const close = async () => {
    ws.close();
    const exited = new Promise(resolve => proc.once('exit', resolve));
    proc.kill();
    await exited;
    try { fs.rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }); } catch { /* a leftover temp profile is harmless */ }
  };
  return { send, evaluate, errors, close };
}

const COVERAGE_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = () => new Promise(r => setTimeout(r, 500));
  const changed = (file, stop) => (api.diff[file]?.hunks || []).some(h => {
    let o = h.o, n = h.n;
    return h.l.some(l => { const hit = l[0] === '+' ? stop.side !== 'old' && n >= stop.start && n <= stop.end : l[0] === '-' ? stop.side === 'old' && o >= stop.start && o <= stop.end : false; if (l[0] !== '+') o++; if (l[0] !== '-') n++; return hit; });
  });
  const chapter = api.content.chapters.find(c => (c.stops || []).some(s => changed(s.file, s)));
  if (!chapter) return { skipped: 'no stop spans a changed line' };
  const stop = chapter.stops.find(s => changed(s.file, s));
  const forget = target => api.store.put(target, target.split(':')[0], { status: '' }, {});
  api.store.put('stop:' + stop.id, 'stop', { status: 'ok' }, {});
  api.openFile(stop.file, stop.start, stop.side || 'new');
  await settle();
  const bar = document.querySelector('[data-coverage-of="' + CSS.escape(stop.file) + '"]');
  const stopReachesFiles = !!bar && +bar.dataset.read > 0;
  const changedRows = document.querySelectorAll('#file-code tr.add, #file-code tr.del').length;
  const readRows = document.querySelectorAll('#file-code tr.add.read, #file-code tr.del.read').length;
  const unreadRows = document.querySelectorAll('#file-code tr.add.unread, #file-code tr.del.unread').length;
  const linesMarked = readRows > 0 && readRows + unreadRows === changedRows;
  forget('stop:' + stop.id);
  api.store.put('file:' + stop.file, 'file', { status: 'ok' }, {});
  location.hash = 'tour-' + chapter.id;
  await settle();
  const control = document.querySelector('.ctl[data-ctl="' + CSS.escape('stop:' + stop.id) + '"]');
  const fileReachesStops = !!control && control.classList.contains('via');
  forget('file:' + stop.file);
  return { stop: stop.id, stopReachesFiles, linesMarked, fileReachesStops };
})()`;

const COMMITS_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = () => new Promise(r => setTimeout(r, 600));
  if (!(api.content.commits || []).length) return { skipped: 'no commits' };
  for (let i = 0; i < 30 && !Object.keys(api.commits || {}).length; i++) await settle();
  const commits = Object.values(window.ExplorePR.api.commits);
  const counted = commits.flatMap(c => window.ReviewCoverage.commitLines(c).filter(r => r.counted && api.diff[r.path]).map(r => ({ ...r, sha: c.sha })));
  const holdsARunOfThree = path => window.ReviewCoverage.changedLinesOf(api.diff[path]).some((l, i, all) => i >= 2 && [1, 2].every(back => all[i - back].side === l.side && all[i - back].line === l.line - back));
  const runnable = counted.find(r => holdsARunOfThree(r.path));
  const row = runnable || counted[0];
  if (!row) return { skipped: 'no commit line in the whole diff' };
  const sel = (lp, lk) => 'tr[data-lp="' + CSS.escape(lp) + '"][data-lk="' + lk + '"]';
  const forget = target => api.store.put(target, target.split(':')[0], { status: '', lines: [] }, {});
  location.hash = 'commits'; await settle();
  api.openCommit(row.sha, row.file.headPath || row.file.path); await settle();
  document.querySelector('#commit-code ' + sel(row.path, row.key) + ' td.ln').click(); await settle();
  api.openFile(row.path, +row.key.split(':')[1], row.key.startsWith('old') ? 'old' : 'new'); await settle();
  const commitReachesFiles = !!document.querySelector('#file-code ' + sel(row.path, row.key) + '.read');
  forget('lines:' + row.path);
  api.store.put('file:' + row.path, 'file', { status: 'ok' }, {});
  location.hash = 'commits'; await settle();
  api.openCommit(row.sha, row.file.headPath || row.file.path); await settle();
  const filesReachCommits = !!document.querySelector('#commit-code ' + sel(row.path, row.key) + '.read');
  forget('file:' + row.path);
  api.openFile(row.path, null, 'new'); await settle();
  const before = document.querySelectorAll('#file-code tr.read').length;
  const unread = document.querySelector('#file-code tr.unread[data-lk] td.ln');
  if (unread) unread.click();
  await settle();
  const linesMarkOneByOne = document.querySelectorAll('#file-code tr.read').length === before + 1;
  const marked = [...document.querySelectorAll('#file-code tr[data-lk]')];
  const middle = marked.find((tr, i) => tr.classList.contains('unread') && marked[i - 1]?.classList.contains('unread') && marked[i + 1]?.classList.contains('unread') && tr.previousElementSibling === marked[i - 1] && tr.nextElementSibling === marked[i + 1]);
  if (middle) { middle.querySelector('td.ln').click(); await settle(); }
  const rows = [...document.querySelectorAll('#file-code tr')];
  const unreadRun = i => rows[i].matches('tr.unread[data-lk]');
  const runStarts = rows.map((_, i) => i).filter(i => unreadRun(i) && !(i > 0 && (unreadRun(i - 1) || rows[i - 1].matches('tr.anno') && i > 1 && unreadRun(i - 2))));
  const linkAboveEachUnreadRun = !runnable ? null : !!middle && runStarts.every(i => rows[i - 1] && rows[i - 1].matches('tr.blk'))
    && document.querySelectorAll('#file-code tr.blk').length === runStarts.length;
  document.querySelectorAll('tr.flash').forEach(tr => tr.classList.remove('flash'));
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true }));
  await new Promise(r => setTimeout(r, 50));
  const nextUnreadKey = !!document.querySelector('#file-code tr.unread.flash');
  forget('lines:' + row.path);
  return { sha: row.sha, line: row.path + ' ' + row.key, commitReachesFiles, filesReachCommits, linesMarkOneByOne, linkAboveEachUnreadRun, nextUnreadKey };
})()`;

const REFERENCES_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const api = window.ExplorePR.api;
  const chapters = api.content.chapters || [];
  const last = [...chapters].reverse().find(chapter => (chapter.stops || []).length);
  if (!last) return { skipped: 'no chapter with a stop' };
  const stop = last.stops[last.stops.length - 1];
  location.hash = 'overview'; await settle();
  const holder = document.createElement('p');
  holder.innerHTML = api.md('See \`' + stop.id + '\` in chapter \`' + last.id + '\`.', true);
  document.querySelector('#main .view').prepend(holder);
  const [toStop, toChapter] = holder.querySelectorAll('a.ref');
  const named = !!toStop && !!toChapter && !holder.querySelector('code') && toStop.textContent.includes(stop.title || stop.id) && toChapter.textContent === (last.title || last.id);
  if (!named) return { named, html: holder.innerHTML };
  toStop.click(); await settle();
  const target = document.getElementById('stop-' + stop.id);
  const top = target ? target.getBoundingClientRect().top : null;
  return { named, opensChapter: location.hash === '#tour-' + last.id, reachesStop: top !== null && top >= -2 && top < innerHeight };
})()`;

const EVIDENCE_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const api = window.ExplorePR.api;
  const chapters = api.content.chapters || [];
  const evidence = window.StopEvidence.index(api.content);
  const chapter = chapters.find(each => (each.stops || []).some(stop => evidence.carriedBy(stop).length));
  if (!chapter) return { skipped: 'no stop carries anything the page declares' };
  const stop = chapter.stops.find(each => evidence.carriedBy(each).length);
  location.hash = 'tour-' + chapter.id; await settle();
  const shown = document.getElementById('stop-' + stop.id);
  const links = shown ? [...shown.querySelectorAll('a.carried')] : [];
  const named = links.length === evidence.carriedBy(stop).length;
  const pointed = chapters.flatMap(each => each.stops || []).filter(each => (each.focus || []).some(focus => focus.why));
  const focusMarked = pointed.every(each => {
    const there = document.getElementById('stop-' + each.id);
    return !there || !there.querySelector('.code table') || !!there.querySelector('tr.focus + tr.point');
  });
  const decisions = window.Catalogue.of(api.content).decisions.map(each => each.item.id).filter(id => id && evidence.shownAt(id).length);
  const controlsAt = Object.fromEntries(decisions.map(id => [id, []]));
  const stripMissing = [], gridKept = [];
  for (const each of chapters) {
    location.hash = 'tour-' + each.id; await settle();
    const introduces = (each.stops || []).some(one => evidence.introducedAt(one).length) || (each.decisions || []).length || (each.abstractions || []).length;
    if (introduces && !document.querySelector('#main .design-strip')) stripMissing.push(each.id);
    if (document.querySelector('#main .abs-grid')) gridKept.push(each.id);
    decisions.forEach(id => document.querySelectorAll('#main .ctl[data-ctl="' + CSS.escape('design:' + id) + '"]')
      .forEach(control => controlsAt[id].push((control.closest('.stop') || {}).id || 'outside a stop')));
  }
  const misplaced = decisions.filter(id => controlsAt[id].length !== 1 || controlsAt[id][0] !== 'stop-' + evidence.shownAt(id)[0]);
  const decisionsMetOnce = !misplaced.length;
  location.hash = 'tour-' + chapter.id; await settle();
  const again = [...document.getElementById('stop-' + stop.id).querySelectorAll('a.carried')];
  if (!named) return { named, focusMarked, decisionsMetOnce, stripMissing, gridKept };
  again[0].click(); await settle();
  const target = document.getElementById(again[0].dataset.scrollTo);
  const top = target ? target.getBoundingClientRect().top : null;
  return { stop: stop.id, named, focusMarked, reachesWhatItCarries: top !== null && top >= -2 && top < innerHeight, decisionsMetOnce, misplaced, stripMissing, gridKept };
})()`;

const READINESS_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const api = window.ExplorePR.api, store = api.store, content = api.content;
  const drawnGates = () => [...document.querySelectorAll('#gates [data-gate]')].map(gate => gate.dataset.gate + ':' + gate.dataset.state);
  location.hash = 'readiness'; await settle();
  const expected = window.Readiness.gates(content, target => store.get(target), path => api.coverageOf(path)).map(gate => gate.id + ':' + gate.state);
  const gatesMatch = JSON.stringify(drawnGates()) === JSON.stringify(expected);
  const decision = window.Catalogue.of(content).decisions.find(each => each.item.id);
  let designTurns = null;
  if (decision) {
    const target = 'design:' + decision.item.id, kept = store.get(target);
    store.put(target, 'design', { status: 'disagree' }, {}); await settle();
    designTurns = drawnGates().includes('design:not-met');
    store.put(target, 'design', { status: kept.status || '' }, {}); await settle();
  }
  const approve = () => document.querySelector('#main .ctl[data-ctl="decision"] button[data-v="approve"]');
  const before = store.get('decision').status || '';
  approve().click(); await settle();
  const chipFollows = document.getElementById('decision-chip').textContent === 'Approve';
  store.put('decision', 'decision', { status: before }, {});
  const highRisk = content.files.filter(f => f.risk === 'high' && (api.coverageOf(f.path) || {}).state !== 'read').map(f => f.path);
  let highRiskOpens = null;
  if (highRisk.length) {
    location.hash = 'readiness'; await settle();
    document.querySelector('#gates [data-gate="high-risk"] [data-jump]').click(); await settle(700);
    const shown = [...document.querySelectorAll('#nav-tree .nav-file')].map(row => row.dataset.file);
    highRiskOpens = location.hash === '#files' && shown.length === highRisk.length && shown.every(path => highRisk.includes(path));
    document.querySelector('[data-clear-facets]').click(); await settle();
  }
  return { gatesMatch, designTurns, chipFollows, highRiskOpens };
})()`;

const FINDINGS_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const content = window.ExplorePR.api.content;
  const observed = content.findings.filter(finding => finding.source && finding.source !== 'review');
  if (!observed.length) return { skipped: 'no observations' };
  location.hash = 'overview'; await settle();
  location.hash = 'findings'; await settle();
  const listed = observed.every(finding => document.getElementById('f-' + finding.id));
  const source = document.querySelector('[data-ff="source"]');
  source.value = 'reading-pass'; source.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  const narrowed = document.querySelectorAll('#main article.finding').length === content.findings.filter(finding => finding.source === 'reading-pass').length;
  document.querySelector('[data-ff="source"]').value = '';
  document.querySelector('[data-ff="source"]').dispatchEvent(new Event('change', { bubbles: true })); await settle();
  const chapter = content.chapters.find(each => content.findings.some(finding => finding.chapter === each.id));
  location.hash = 'tour-' + chapter.id; await settle();
  const link = document.querySelector('#main .finding-list a[data-jump="findings"]');
  const wanted = JSON.parse(link.dataset.filter).q;
  link.click(); await settle();
  const shown = [...document.querySelectorAll('#main article.finding')].map(article => article.id);
  return { listed, narrowed, opensItsFinding: location.hash === '#findings' && shown.length === 1 && shown[0] === 'f-' + wanted };
})()`;

const DESIGN_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const article = (window.ExplorePR.api.content.design || {}).article || [];
  if (!article.length) return { skipped: 'no design article' };
  location.hash = 'design'; await settle();
  const excerpts = article.flatMap(section => section.blocks || []).filter(block => block.kind === 'code').length;
  return {
    sections: document.querySelectorAll('#main .article-section').length === article.length,
    excerptsResolved: document.querySelectorAll('#main .excerpt .code table').length === excerpts && !document.querySelector('#main [data-pending-slice]'),
    signOff: !!document.querySelector('#main .ctl[data-ctl="design-overall"]'),
    noCatalogue: !document.querySelector('#main .abs-grid'),
  };
})()`;

const NOTES_PROBE = `(async () => {
  const settle = ms => new Promise(resolve => setTimeout(resolve, ms));
  const api = window.ExplorePR.api, store = api.store;
  const stop = (api.content.chapters || []).flatMap(chapter => chapter.stops || [])[0];
  const kept = { general: store.get('notes:general'), stop: stop && store.get('stop:' + stop.id) };
  const type = (box, text) => { box.focus(); box.value = text; box.dispatchEvent(new Event('input', { bubbles: true })); };
  location.hash = 'notes'; await settle(300);
  const general = document.getElementById('general-notes');
  type(general, 'Smoke: ask about the lock');
  await settle(900);
  const exported = document.getElementById('notes-md').value;
  const generalSaved = store.get('notes:general').note === 'Smoke: ask about the lock';
  const generalExported = /\\n## General notes\\n\\nSmoke: ask about the lock\\n/.test(exported) && exported.indexOf('## General notes') === exported.indexOf('## ');
  const generalKeepsFocus = document.activeElement === general;
  general.blur();
  let cardSaved = null, cardKeepsFocus = null;
  if (stop) {
    store.put('stop:' + stop.id, 'stop', { status: 'question' }, { label: 'Smoke stop' }); await settle(300);
    const note = document.querySelector('.ctl[data-ctl="' + CSS.escape('stop:' + stop.id) + '"] textarea[data-note]');
    type(note, 'Smoke: why here?');
    await settle(900);
    cardSaved = store.get('stop:' + stop.id).note === 'Smoke: why here?';
    cardKeepsFocus = document.activeElement === note;
    note.blur();
  }
  const restore = (target, kind, record) => store.put(target, kind, { status: record.status || '', note: record.note || '' }, {});
  restore('notes:general', 'notes', kept.general);
  if (stop) restore('stop:' + stop.id, 'stop', kept.stop);
  return { generalSaved, generalExported, generalKeepsFocus, cardSaved, cardKeepsFocus };
})()`;

const CATALOGUE_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const api = window.ExplorePR.api;
  const catalogue = window.Catalogue.of(api.content);
  const expected = {
    decisions: ['article.decision', catalogue.decisions.length],
    abstractions: ['.abs', catalogue.abstractions.length],
    invariants: ['.inv, tr[id^="risk-"]', catalogue.invariants.length + catalogue.risks.length],
    tests: ['tr.test-row', catalogue.tests.length],
  };
  const routes = window.ExplorePR.routes();
  const counted = {};
  for (const [route, [cards, count]] of Object.entries(expected)) {
    if (!routes.includes(route)) continue;
    location.hash = route; await settle();
    const drawn = document.querySelectorAll('#main ' + cards.split(', ').join(', #main ')).length;
    counted[route] = drawn === count || { drawn, count };
  }
  const evidence = window.StopEvidence.index(api.content);
  const decision = catalogue.decisions.find(each => each.item.id && evidence.shownAt(each.item.id).length);
  if (!decision) return { counted, skipped: 'no decision a stop carries' };
  const target = 'design:' + decision.item.id;
  const firstStop = evidence.shownAt(decision.item.id)[0];
  const chapter = api.content.chapters.find(each => (each.stops || []).some(stop => stop.id === firstStop));
  const control = () => document.querySelector('.ctl[data-ctl="' + CSS.escape(target) + '"]');
  location.hash = 'decisions'; await settle();
  control().querySelector('button[data-v="question"]').click(); await settle();
  location.hash = 'tour-' + chapter.id; await settle();
  const elsewhere = control();
  const verdictTravels = !!elsewhere && elsewhere.querySelector('button[data-v="question"]').getAttribute('aria-pressed') === 'true';
  api.store.put(target, 'design', { status: '' }, {});
  return { counted, verdictTravels };
})()`;

const HEADS_PROBE = `(async () => {
  const api = window.ExplorePR.api, store = api.store, content = api.content;
  const settle = () => new Promise(r => setTimeout(r, 500));
  const file = content.files.find(f => window.ReviewCoverage.changedLinesOf((api.diff || {})[f.path]).length);
  if (!file || !content.pr.head) return { skipped: 'no head, or no file with a changed line' };
  const path = file.path, target = 'file:' + path, earlier = 'earlier-head';
  const bar = () => document.querySelector('[data-coverage-of="' + CSS.escape(path) + '"]');
  const since = () => document.querySelector('.ctl[data-ctl="' + CSS.escape(target) + '"] [data-since-note]');
  const recordedAt = (head, lineage) => {
    content.lineage = { ...(content.lineage || {}), [earlier]: { [path]: lineage } };
    store.docs.set(target, { ...store.get(target), head });
    store.version++;
    api.rerender();
  };
  store.put(target, 'file', { status: 'ok' }, {});
  const saved = store.get(target);
  const stamped = saved.head === content.pr.head && Array.isArray(saved.claimed) && saved.claimed.length > 0;
  api.openFile(path); await settle();
  const readAtHead = +bar().dataset.read;
  recordedAt(earlier, { new: [], old: [], changed: true }); api.openFile(path); await settle();
  const rewrittenUnread = readAtHead > 0 && +bar().dataset.read === 0;
  const marked = !!since() && !since().hidden && since().textContent.includes(earlier.slice(0, 7));
  const banner = document.querySelector('#store-banner');
  const announced = !!banner && !banner.hidden && /moved/.test(banner.textContent);
  recordedAt(earlier, 'same'); api.openFile(path); await settle();
  const untouchedCarried = +bar().dataset.read === readAtHead && !!since() && since().hidden;
  store.put(target, 'file', { note: 'checked' }, {});
  const noteKeepsHead = store.get(target).head === earlier;
  store.put(target, 'file', { status: 'issue' }, {});
  const verdictRestamps = store.get(target).head === content.pr.head;
  store.put(target, 'file', { status: '', note: '' }, {});
  delete content.lineage[earlier];
  return { stamped, rewrittenUnread, marked, announced, untouchedCarried, noteKeepsHead, verdictRestamps };
})()`;

const DIAGRAMS_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 300));
  const seen = {};
  let tracing = { skipped: 'no diagram with an arrow' };
  for (const route of [...window.ExplorePR.figureViews(), ...window.ExplorePR.customViews()]) {
    location.hash = route; await settle();
    for (let waited = 0; waited < 40 && document.querySelector('[data-drawing]'); waited++) await settle();
    if (document.querySelector('[data-drawing]')) { seen[route] = ['a diagram was still being drawn after 12 seconds']; continue; }
    if (!document.querySelector('svg.dk, .dk-drawn')) continue;
    const checked = await ${DRAWING_CHECKS};
    [...checked.diagrams, ...${SELF_DRAWN_CHECKS}].forEach(each => { seen[each.id] = each.problems; });
    const traced = await ${TRACING_CHECK};
    if (!traced.skipped) tracing = traced;
    const link = document.querySelector('svg.dk .node-link');
    if (link && !('linkOpens' in seen)) {
      const before = location.hash;
      link.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      await settle();
      seen.linkOpens = location.hash !== before || !!document.querySelector('#file-pane');
    }
  }
  const { linkOpens = null, ...problems } = seen;
  if (!Object.keys(problems).length) return { skipped: 'no kit diagram on the page' };
  return { problems, tracing, linkOpens };
})()`;

const NAVIGATOR_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = (ms = 400) => new Promise(r => setTimeout(r, ms));
  location.hash = 'files'; await settle();
  const choose = (pref, value) => { const select = document.querySelector('[data-nav-pref="' + pref + '"]'); select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); };
  choose('groupBy', 'tree'); choose('sortBy', 'path'); await settle();
  const fileRows = () => [...document.querySelectorAll('#nav-tree .nav-file')];
  const shownPaths = () => fileRows().map(row => row.dataset.file);
  const folderHolding = path => [...document.querySelectorAll('#nav-tree .nav-folder')].find(row => path.startsWith(row.dataset.toggleNode.replace(/^folder::/, '')));
  const nested = api.content.files.find(f => f.path.includes('/'));
  if (!nested) return { skipped: 'no file in a directory' };
  const everyFileDrawn = shownPaths().length === api.content.files.length;
  const folder = folderHolding(nested.path);
  const treeDrawn = !!folder && everyFileDrawn;
  folder.click(); await settle();
  const collapseHides = !shownPaths().includes(nested.path);
  api.openFile(nested.path, null, 'new'); await settle(700);
  const openingReveals = shownPaths().includes(nested.path);
  const layers = [...new Set(api.content.files.map(f => f.layer))];
  const layer = layers.find(id => api.content.files.some(f => f.layer !== id)) || layers[0];
  const role = document.querySelector('#nav-facets [data-facet="layer"]');
  role.open = true;
  const box = role.querySelector('[data-facet-value="' + CSS.escape(layer) + '"]');
  const chip = role.querySelector('summary'), chipTop = chip.getBoundingClientRect().top, treeTop = document.querySelector('#nav-tree').getBoundingClientRect().top;
  box.checked = true; box.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  const controlsStay = chip.isConnected && role.open && chip.getBoundingClientRect().top === chipTop && document.querySelector('#nav-tree').getBoundingClientRect().top === treeTop;
  const ofLayer = new Set(api.content.files.filter(f => f.layer === layer).map(f => f.path));
  const count = +document.querySelector('#nav-facets [data-facet="layer"] [data-facet-value="' + CSS.escape(layer) + '"]').closest('label').querySelector('[data-facet-count]').textContent.replace(/,/g, '');
  const roleNarrows = shownPaths().length === ofLayer.size && shownPaths().every(path => ofLayer.has(path)) && count === ofLayer.size;
  document.querySelector('[data-clear-facets]').click(); await settle();
  const query = document.querySelector('#fq');
  query.value = '!test'; query.dispatchEvent(new Event('input', { bubbles: true })); await settle(700);
  const excludes = shownPaths().length > 0 && shownPaths().every(path => !/test/i.test(path));
  query.value = ''; query.dispatchEvent(new Event('input', { bubbles: true })); await settle(700);
  const inFolder = api.content.files.find(f => f.path.includes('/') && api.coverageOf(f.path) && api.coverageOf(f.path).changed);
  const row = folderHolding(inFolder.path);
  const before = row.querySelector('[data-folder-count-of]').textContent;
  api.store.put('file:' + inFolder.path, 'file', { status: 'ok' }, {}); await settle();
  const totalsInPlace = row.isConnected && row.querySelector('[data-folder-count-of]').textContent !== before;
  api.store.put('file:' + inFolder.path, 'file', { status: '' }, {}); await settle();
  const pillsAt = () => [...document.querySelectorAll('#nav-facets summary')].map(pill => { const at = pill.getBoundingClientRect(); return at.left + ',' + at.top; }).join(' ');
  const pillsBefore = pillsAt();
  const progress = document.querySelector('#nav-facets [data-facet="progress"]');
  progress.querySelector('summary').click(); await settle();
  const pillsStay = progress.open && pillsAt() === pillsBefore;
  document.body.click(); await settle();
  choose('groupBy', 'none'); choose('sortBy', 'size'); await settle();
  const sizes = fileRows().map(row => { const f = api.content.files.find(each => each.path === row.dataset.file); return f.add + f.del; });
  const flattened = !document.querySelector('#nav-tree .nav-folder, #nav-tree .nav-group') && sizes.length === api.content.files.length && sizes.every((n, i) => !i || sizes[i - 1] >= n);
  const nav = document.querySelector('#nav-tree');
  const searchTop = document.querySelector('#fq').getBoundingClientRect().top;
  nav.scrollTop = nav.scrollHeight;
  const filtersPinned = nav.scrollTop > 0 && document.querySelector('#fq').getBoundingClientRect().top === searchTop;
  nav.scrollTop = Math.min(200, nav.scrollHeight - nav.clientHeight);
  const kept = nav.scrollTop, frame = nav.getBoundingClientRect();
  const inView = fileRows().filter(row => { const at = row.getBoundingClientRect(); return at.top >= frame.top && at.bottom <= frame.bottom; });
  inView[inView.length - 1].click(); await settle();
  const barBottom = document.querySelector('.topbar').getBoundingClientRect().bottom, paneTop = document.querySelector('#file-pane').getBoundingClientRect().top;
  const navigatorStays = nav.scrollTop === kept;
  const paneBelowTopBar = paneTop >= barBottom && paneTop - barBottom <= 16;
  choose('groupBy', 'tree'); choose('sortBy', 'path'); await settle();
  const handle = document.querySelector('.nav-resizer'), drawerWidth = () => Math.round(document.querySelector('.nav-drawer').getBoundingClientRect().width);
  const narrow = drawerWidth();
  handle.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true })); await settle();
  const widens = drawerWidth() === narrow + 24;
  handle.dispatchEvent(new MouseEvent('dblclick', { bubbles: true })); await settle();
  const resets = drawerWidth() === narrow;
  const edged = () => [...document.querySelectorAll('#nav-tree .fl-item')].filter(row => getComputedStyle(row, '::before').content !== 'none').length;
  const picks = [...document.querySelectorAll('#nav-tree .pick')].map(box => Math.round(box.getBoundingClientRect().left));
  const picksAligned = picks.length > 1 && picks.every(left => left === picks[0]);
  const quietByDefault = edged() === 0;
  document.querySelector('[data-toggle-risk]').click(); await settle();
  const riskEdges = quietByDefault && edged() === document.querySelectorAll('#nav-tree .fl-item').length;
  document.querySelector('[data-toggle-risk]').click(); await settle();
  return { file: nested.path, layer, treeDrawn, collapseHides, openingReveals, roleNarrows, controlsStay, excludes, totalsInPlace, pillsStay, flattened, filtersPinned, navigatorStays, paneBelowTopBar, widens, resets, riskEdges, picksAligned };
})()`;

const BULK_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = (ms = 400) => new Promise(r => setTimeout(r, ms));
  location.hash = 'files'; await settle();
  const choose = (pref, value) => { const select = document.querySelector('[data-nav-pref="' + pref + '"]'); select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); };
  choose('groupBy', 'tree'); choose('sortBy', 'path'); await settle();
  const filesIn = row => api.content.files.map(f => f.path).filter(path => path.startsWith(row.dataset.toggleNode.replace(/^folder::/, '')));
  const folder = [...document.querySelectorAll('#nav-tree .nav-folder')].find(row => filesIn(row).length >= 2);
  if (!folder) return { skipped: 'no folder holding two files' };
  const paths = filesIn(folder), flagged = paths[0];
  api.store.put('file:' + flagged, 'file', { status: 'issue' }, {}); await settle();
  const before = Object.fromEntries(paths.map(path => [path, api.store.get('file:' + path).status || '']));
  const row = () => document.querySelector('#nav-tree [data-toggle-node="' + CSS.escape(folder.dataset.toggleNode) + '"]');
  row().parentElement.querySelector('.pick').click(); await settle();
  const barShown = !document.querySelector('#bulk-bar').hidden && document.querySelector('#bulk-count').textContent.startsWith(String(paths.length));
  document.querySelector('#bulk-bar [data-bulk="ok"]').click(); await settle();
  const issueKept = api.store.get('file:' + flagged).status === 'issue';
  const othersReviewed = paths.slice(1).every(path => api.store.get('file:' + path).status === 'ok');
  const [read, changed] = row().querySelector('[data-folder-count-of]').textContent.replace(/,/g, '').split('/').map(Number);
  const folderRead = changed > 0 && read === changed;
  const toast = document.querySelector('#undo-toast');
  const toastSays = !toast.hidden && /with an Issue left as it was/.test(toast.textContent) && !!toast.querySelector('[data-undo]');
  const selectionCleared = document.querySelector('#bulk-bar').hidden;
  toast.querySelector('[data-undo]').click(); await settle();
  const undone = paths.every(path => (api.store.get('file:' + path).status || '') === before[path]);
  api.store.put('file:' + flagged, 'file', { status: '' }, {}); await settle();
  return { folder: folder.dataset.toggleNode, files: paths.length, barShown, issueKept, othersReviewed, folderRead, toastSays, selectionCleared, undone };
})()`;

const KEYS_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = (ms = 350) => new Promise(r => setTimeout(r, ms));
  const press = (key, on = document.body) => on.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  location.hash = 'files'; await settle();
  const choose = (pref, value) => { const select = document.querySelector('[data-nav-pref="' + pref + '"]'); select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); };
  choose('groupBy', 'none'); choose('sortBy', 'path'); await settle();
  const rows = () => [...document.querySelectorAll('#nav-tree .nav-file')];
  const current = () => (document.querySelector('#nav-tree .nav-file[aria-current="true"]') || {}).dataset?.file;
  const unreadFiles = rows().map(row => row.dataset.file).filter(path => { const cov = api.coverageOf(path); return cov && cov.changed && cov.read === 0; });
  if (unreadFiles.length < 3) return { skipped: 'fewer than three unread files' };
  const start = unreadFiles[0];
  rows().find(row => row.dataset.file === start).click(); await settle();
  const order = rows().map(row => row.dataset.file), at = order.indexOf(start);
  press('j'); await settle();
  const jMoves = current() === order[at + 1];
  press('k'); await settle();
  const kReturns = current() === start;
  press(']'); await settle();
  const cursor = () => [...document.querySelectorAll('#file-code tr.chunk-cursor')];
  const chunkCursor = cursor().length > 0;
  press('m'); await settle();
  const chunkMarked = cursor().length > 0 && cursor().every(tr => tr.classList.contains('read'));
  press('z'); await settle();
  const markUndone = cursor().every(tr => tr.classList.contains('unread'));
  press('i'); await settle();
  const note = document.querySelector('#file-pane .ctl[data-ctl="file:' + CSS.escape(start) + '"] textarea');
  const issueNoted = api.store.get('file:' + start).status === 'issue' && /^(head|base) \\d+/.test(note.value) && note.value.endsWith(': ') && document.activeElement === note;
  press('Escape', note); await settle();
  const fieldLeft = document.activeElement !== note;
  press('z'); await settle(900);
  const issueUndone = !api.store.get('file:' + start).status;
  api.store.put('file:' + start, 'file', { note: '' }, {}); await settle();
  press('?'); await settle();
  const helpOpens = !document.querySelector('#keys-help').hidden && document.querySelectorAll('#keys-help dt').length > 10;
  press('Escape'); await settle();
  const helpCloses = document.querySelector('#keys-help').hidden;
  press('r'); await settle();
  const advances = api.store.get('file:' + start).status === 'ok' && current() !== start && unreadFiles.includes(current());
  press('z'); await settle();
  api.store.put('file:' + start, 'file', { status: '' }, {}); await settle();
  press('x'); await settle();
  const xSelects = !document.querySelector('#bulk-bar').hidden;
  press('Escape'); await settle();
  const escapeDeselects = document.querySelector('#bulk-bar').hidden;
  const shownBefore = rows().length;
  api.store.put('file:' + order[0], 'file', { status: 'ok' }, {}); await settle();
  press('u'); await settle();
  const uNarrows = rows().length < shownBefore && !rows().some(row => row.dataset.file === order[0]);
  press('u'); await settle();
  api.store.put('file:' + order[0], 'file', { status: '' }, {}); await settle();
  choose('groupBy', 'tree'); await settle();
  press('g'); await settle();
  const gCycles = document.querySelector('[data-nav-pref="groupBy"]').value === 'none' && !document.querySelector('#nav-tree .nav-folder');
  choose('groupBy', 'tree'); await settle();
  const treeRow = document.querySelector('#nav-tree .fl-item[tabindex="0"]');
  treeRow.focus();
  press('ArrowDown', treeRow); await settle();
  const arrowMoves = document.activeElement !== treeRow && document.activeElement.classList.contains('fl-item');
  const folder = document.querySelector('#nav-tree .nav-folder[aria-expanded="true"]');
  folder.focus();
  press('ArrowLeft', folder); await settle();
  const arrowCollapses = document.querySelector('#nav-tree [data-toggle-node="' + CSS.escape(folder.dataset.toggleNode) + '"]').getAttribute('aria-expanded') === 'false';
  press('ArrowRight', document.activeElement); await settle();
  press('f'); await settle();
  const fFocusesList = !!document.activeElement.closest('#nav-tree') && document.activeElement.getAttribute('aria-current') === 'true';
  press('c', document.activeElement); await settle();
  const cFocusesCode = document.activeElement === document.querySelector('#file-pane');
  let commitKeys = null, commitFileKeys = null;
  if ((api.content.commits || []).length > 1) {
    location.hash = 'commits'; await settle(700);
    const selected = () => document.querySelector('#nav-tree .nav-commit[aria-current="true"]').dataset.commit;
    const before = selected();
    press('J'); await settle();
    const after = selected();
    press('K'); await settle();
    commitKeys = after !== before && selected() === before;
    const many = Object.values(api.commits).find(c => !c.merge && c.files.length > 1);
    if (many) {
      api.openCommit(many.sha); await settle(700);
      const inCommit = api.navigator.order().filter(key => key.startsWith(many.sha + ':'));
      press('j'); await settle();
      const block = document.querySelector('#commit-code > .code');
      const bar = document.querySelector('.topbar').getBoundingClientRect().bottom, top = block.getBoundingClientRect().top;
      commitFileKeys = api.navigator.current() === inCommit[1] && block.dataset.commitFile === inCommit[1].slice(many.sha.length + 1) && top >= bar && top - bar <= 16;
    }
  }
  return { file: start, jMoves, kReturns, chunkCursor, chunkMarked, markUndone, issueNoted, fieldLeft, issueUndone, helpOpens, helpCloses, advances, xSelects, escapeDeselects, uNarrows, gCycles, arrowMoves, arrowCollapses, fFocusesList, cFocusesCode, commitKeys, commitFileKeys };
})()`;

const COMMIT_NAVIGATOR_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = (ms = 400) => new Promise(r => setTimeout(r, ms));
  const press = key => document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  if ((api.content.commits || []).length < 2) return { skipped: 'fewer than two commits' };
  const reviewable = api.content.commits.filter(c => !c.merge && (api.commits[c.sha] || {}).files?.length);
  const pair = reviewable.findIndex((c, i) => reviewable[i + 1] && api.commits[c.sha].files.length > 1);
  if (pair < 0) return { skipped: 'no commit with two files before another' };
  const first = reviewable[pair], second = reviewable[pair + 1];
  api.openCommit(first.sha); await settle(700);
  const expanded = [...document.querySelectorAll('#nav-tree .nav-commit[aria-expanded="true"]')].map(row => row.dataset.commit);
  const onlySelectedOpen = expanded.length === 1 && expanded[0] === first.sha;
  const order = api.navigator.order();
  const lastOfFirst = order.filter(key => key.startsWith(first.sha + ':')).pop();
  const firstOfSecond = order.find(key => key.startsWith(second.sha + ':'));
  api.openCommit(first.sha, lastOfFirst.slice(first.sha.length + 1)); await settle();
  press('j'); await settle();
  const crossesCommits = api.navigator.current() === firstOfSecond;
  const commit = api.commits[first.sha];
  const counted = file => window.ReviewCoverage.commitLines({ ...commit, files: [file] }).filter(row => row.counted && api.diff[row.path]);
  const [marked, other] = commit.files.filter(file => counted(file).length);
  let verdictScoped = null;
  if (marked && other) {
    const path = marked.headPath || marked.path, otherPath = other.headPath || other.path;
    const otherBefore = (api.coverageOf(otherPath) || {}).read;
    const before = (api.coverageOf(path) || {}).read;
    api.openCommit(first.sha, path); await settle();
    press('r'); await settle();
    verdictScoped = api.store.get('commitfile:' + first.sha + ':' + path).status === 'ok'
      && api.coverageOf(path).read > before && (otherPath === path || api.coverageOf(otherPath).read === otherBefore);
    api.store.put('commitfile:' + first.sha + ':' + path, 'commitfile', { status: '' }, {}); await settle();
  }
  api.openCommit(first.sha); await settle();
  const commitRow = sha => document.querySelector('#nav-tree .nav-commit[data-commit="' + sha + '"]');
  commitRow(second.sha).querySelector('.chev').click(); await settle();
  const chevronOpensWithoutSelecting = commitRow(second.sha).getAttribute('aria-expanded') === 'true'
    && commitRow(first.sha).getAttribute('aria-expanded') === 'true' && commitRow(first.sha).getAttribute('aria-current') === 'true';
  commitRow(second.sha).querySelector('.chev').click(); await settle();
  document.querySelector('[data-commit-pane="whole"]').click(); await settle();
  const wholeShowsAll = document.querySelectorAll('#commit-code > .code').length === commit.files.length;
  document.querySelector('[data-commit-pane="one"]').click(); await settle();
  const oneShowsOne = document.querySelectorAll('#commit-code > .code').length === 1;
  const longest = Object.entries(api.diff).map(([path, f]) => [path, Math.max(0, ...(f.hunks || []).flatMap(h => h.l.map(l => l.length)))]).sort((a, b) => b[1] - a[1])[0];
  let longLinesCut = null;
  if (longest && longest[1] > 2000) {
    api.openFile(longest[0], null, 'new'); await settle(700);
    longLinesCut = [...document.querySelectorAll('#file-code td.src')].every(td => td.textContent.length <= 2100) && !!document.querySelector('#file-code td.src .cut');
  }
  return { commit: first.sha, onlySelectedOpen, crossesCommits, verdictScoped, chevronOpensWithoutSelecting, wholeShowsAll, oneShowsOne, longLinesCut };
})()`;

const TOUCH_PROBE = `(async () => {
  location.hash = 'files'; await new Promise(r => setTimeout(r, 500));
  const picks = [...document.querySelectorAll('#nav-tree .pick')];
  return { hoverNone: matchMedia('(hover: none)').matches, mobile: matchMedia('(pointer: coarse)').matches, picks: picks.length, picksShown: picks.length > 0 && picks.every(box => getComputedStyle(box).visibility === 'visible') };
})()`;

const UNFOLD_PROBE = `(async () => {
  const api = window.ExplorePR.api;
  const settle = () => new Promise(r => setTimeout(r, 500));
  const folded = api.content.files.find(f => f.status !== 'added' && window.ReviewCoverage && api.diff[f.path] && (api.diff[f.path].hunks[0]?.l.length || 0) > 30);
  if (!folded) return { skipped: 'no file long enough to fold' };
  const unfolds = async () => {
    const button = document.querySelector('#file-code [data-unfold]');
    if (!button) return null;
    const before = document.querySelectorAll('#file-code tr').length;
    button.click(); await settle();
    return document.querySelectorAll('#file-code tr').length > before;
  };
  api.openFile(folded.path, null, 'new'); await settle();
  const afterOpening = await unfolds();
  const other = api.content.files.find(f => f.path !== folded.path);
  if (other) { document.querySelector('.fl-item[data-file="' + CSS.escape(other.path) + '"]').click(); await settle(); }
  document.querySelector('.fl-item[data-file="' + CSS.escape(folded.path) + '"]').click(); await settle();
  const afterPicking = await unfolds();
  return { file: folded.path, afterOpening, afterPicking };
})()`;

async function captureWholePage(cdp, file) {
  const { result: { cssContentSize: size } } = await cdp.send('Page.getLayoutMetrics');
  for (const tile of tilesOf(size)) {
    const shot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip: tile.clip });
    fs.writeFileSync(`${file}${tile.name}.png`, Buffer.from(shot.result.data, 'base64'));
  }
}

async function captureViewport(cdp, file) {
  const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync(`${file}.png`, Buffer.from(shot.result.data, 'base64'));
}

async function main() {
  const options = args(process.argv.slice(2));
  const chrome = findChrome(options.chrome);
  if (!chrome) { console.log(JSON.stringify({ skipped: 'no Chrome or Chromium found; set CHROME' })); return; }
  const page = path.resolve(options.page);
  const cdp = await devtools(chrome);
  const report = { page, routes: {}, overflow: [], errors: cdp.errors };
  try {
    await cdp.send('Runtime.enable');
    const viewport = width => cdp.send('Emulation.setDeviceMetricsOverride', { width, height: 1000, deviceScaleFactor: 1, mobile: width < 500 });
    await viewport(1440);
    await cdp.send('Page.navigate', { url: pathToFileURL(page).href + '#overview' });
    for (let i = 0; i < 50 && !(await cdp.evaluate('!!(window.ExplorePR && window.ExplorePR.routes().length)')); i++) await sleep(100);
    const routes = await cdp.evaluate('window.ExplorePR ? window.ExplorePR.routes() : []');
    if (options.shots) fs.mkdirSync(options.shots, { recursive: true });
    const figureViews = await cdp.evaluate('window.ExplorePR.figureViews()');
    const chapters = figureViews.filter(view => !routes.includes(view));
    for (const route of [...routes, ...chapters]) {
      await cdp.evaluate(`location.hash = ${JSON.stringify(route)}`);
      await sleep(700);
      report.routes[route] = await cdp.evaluate(`({ heading: (document.querySelector('main h1') || {}).textContent || null, unresolvedSlices: document.querySelectorAll('[data-pending-slice]').length, highlighted: document.querySelectorAll('[class^="hljs-"]').length })`);
      if (options.shots) {
        const capture = figureViews.includes(route) ? captureWholePage : captureViewport;
        await capture(cdp, path.join(options.shots, route));
      }
    }
    report.coverage = await cdp.evaluate(COVERAGE_PROBE);
    report.commits = await cdp.evaluate(COMMITS_PROBE);
    report.unfold = await cdp.evaluate(UNFOLD_PROBE);
    report.navigator = await cdp.evaluate(NAVIGATOR_PROBE);
    report.bulk = await cdp.evaluate(BULK_PROBE);
    report.keys = await cdp.evaluate(KEYS_PROBE);
    report.commitNavigator = await cdp.evaluate(COMMIT_NAVIGATOR_PROBE);
    report.references = await cdp.evaluate(REFERENCES_PROBE);
    report.evidence = await cdp.evaluate(EVIDENCE_PROBE);
    report.catalogue = await cdp.evaluate(CATALOGUE_PROBE);
    report.notes = await cdp.evaluate(NOTES_PROBE);
    report.design = await cdp.evaluate(DESIGN_PROBE);
    report.findings = await cdp.evaluate(FINDINGS_PROBE);
    report.readiness = await cdp.evaluate(READINESS_PROBE);
    report.heads = await cdp.evaluate(HEADS_PROBE);
    report.diagrams = await cdp.evaluate(DIAGRAMS_PROBE);
    await viewport(400);
    for (const route of routes) {
      await cdp.evaluate(`location.hash = ${JSON.stringify(route)}`);
      await sleep(400);
      const width = await cdp.evaluate('document.documentElement.scrollWidth');
      if (width > 400) report.overflow.push({ route, scrollWidth: width });
    }
    await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
    report.touch = await cdp.evaluate(TOUCH_PROBE);
    await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: false });
    if (options.shots) {
      await cdp.evaluate("location.hash = 'overview'");
      await sleep(500);
      await captureViewport(cdp, path.join(options.shots, 'overview-phone'));
    }
  } finally {
    await cdp.close();
  }
  const missing = Object.entries(report.routes).filter(([, r]) => !r.heading || r.unresolvedSlices).map(([route]) => route);
  const coverage = report.coverage || {};
  const coverageHolds = !!coverage.skipped || (coverage.stopReachesFiles && coverage.linesMarked && coverage.fileReachesStops);
  const commits = report.commits || {};
  const commitsHold = !!commits.skipped || !!(commits.commitReachesFiles && commits.filesReachCommits && commits.linesMarkOneByOne && commits.linkAboveEachUnreadRun !== false && commits.nextUnreadKey);
  const unfold = report.unfold || {};
  const unfoldHolds = !!unfold.skipped || (unfold.afterOpening !== false && unfold.afterPicking !== false && (unfold.afterOpening || unfold.afterPicking) !== null);
  const references = report.references || {};
  const referencesHold = !!references.skipped || !!(references.named && references.opensChapter && references.reachesStop);
  const evidence = report.evidence || {};
  const evidenceHolds = !!evidence.skipped || !!(evidence.named && evidence.focusMarked && evidence.reachesWhatItCarries && evidence.decisionsMetOnce && !evidence.stripMissing.length && !evidence.gridKept.length);
  const heads = report.heads || {};
  const headsHold = !!heads.skipped || !!(heads.stamped && heads.rewrittenUnread && heads.marked && heads.announced && heads.untouchedCarried && heads.noteKeepsHead && heads.verdictRestamps);
  const catalogue = report.catalogue || {};
  const catalogueHolds = Object.values(catalogue.counted || {}).every(held => held === true) && (!!catalogue.skipped || !!catalogue.verdictTravels);
  const notes = report.notes || {};
  const notesHold = !!(notes.generalSaved && notes.generalExported && notes.generalKeepsFocus && notes.cardSaved !== false && notes.cardKeepsFocus !== false);
  const design = report.design || {};
  const designHolds = !!design.skipped || !!(design.sections && design.excerptsResolved && design.signOff && design.noCatalogue);
  const navigator = report.navigator || {};
  const navigatorHolds = !!navigator.skipped || !!(navigator.treeDrawn && navigator.collapseHides && navigator.openingReveals && navigator.roleNarrows && navigator.controlsStay && navigator.excludes && navigator.totalsInPlace && navigator.pillsStay && navigator.flattened && navigator.filtersPinned && navigator.navigatorStays && navigator.paneBelowTopBar && navigator.widens && navigator.resets && navigator.riskEdges && navigator.picksAligned);
  const bulk = report.bulk || {};
  const bulkHolds = !!bulk.skipped || !!(bulk.barShown && bulk.issueKept && bulk.othersReviewed && bulk.folderRead && bulk.toastSays && bulk.selectionCleared && bulk.undone);
  const keys = report.keys || {};
  const keysHold = !!keys.skipped || !!(keys.jMoves && keys.kReturns && keys.chunkCursor && keys.chunkMarked && keys.markUndone && keys.issueNoted && keys.fieldLeft && keys.issueUndone && keys.helpOpens && keys.helpCloses && keys.advances && keys.xSelects && keys.escapeDeselects && keys.uNarrows && keys.gCycles && keys.arrowMoves && keys.arrowCollapses && keys.fFocusesList && keys.cFocusesCode && keys.commitKeys !== false && keys.commitFileKeys !== false);
  const commitNavigator = report.commitNavigator || {};
  const commitNavigatorHolds = !!commitNavigator.skipped || !!(commitNavigator.onlySelectedOpen && commitNavigator.crossesCommits && commitNavigator.verdictScoped !== false && commitNavigator.chevronOpensWithoutSelecting && commitNavigator.wholeShowsAll && commitNavigator.oneShowsOne && commitNavigator.longLinesCut !== false);
  const touch = report.touch || {};
  const touchHolds = !!touch.picksShown;
  const findings = report.findings || {};
  const findingsHold = !!findings.skipped || !!(findings.listed && findings.narrowed && findings.opensItsFinding);
  const readiness = report.readiness || {};
  const readinessHolds = !!(readiness.gatesMatch && readiness.designTurns !== false && readiness.chipFollows && readiness.highRiskOpens !== false);
  const diagrams = report.diagrams || {};
  const tracesHold = !diagrams.tracing || !!diagrams.tracing.skipped || !!(diagrams.tracing.traces && diagrams.tracing.clears);
  const diagramsHold = !!diagrams.skipped || (Object.values(diagrams.problems || { none: ['not checked'] }).every(found => !found.length) && tracesHold && diagrams.linkOpens !== false);
  report.ok = touchHolds && navigatorHolds && bulkHolds && keysHold && commitNavigatorHolds && diagramsHold && catalogueHolds && notesHold && designHolds && findingsHold && readinessHolds && headsHold && evidenceHolds && referencesHold && commitsHold && unfoldHolds && !report.errors.length && !report.overflow.length && !missing.length && Object.keys(report.routes).length > 0 && coverageHolds;
  if (missing.length) report.incomplete = missing;
  console.log(JSON.stringify(report, null, 1));
  process.exitCode = report.ok ? 0 : 1;
}

main().catch(error => { console.error(error); process.exitCode = 2; });
