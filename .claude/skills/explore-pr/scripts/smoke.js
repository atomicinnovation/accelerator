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
// unread lines and that the n key jumps to the next unread line. Needs Node
// 22+ (global WebSocket). It unfolds a run of unchanged lines in a file
// opened by link and in one picked from the file list. Last, it renders prose
// naming a stop and its chapter, and checks both become links and that the
// stop's link opens its chapter at that stop. It also records a file verdict,
// pretends it was given at an earlier head, and checks that rewritten lines are
// unread again and announced, that untouched ones stay read, and that only a
// new verdict, not a note, moves the record to this head.

const { spawn, execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

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
  const send = (method, params = {}) => new Promise(resolve => { const n = ++id; pending.set(n, resolve); ws.send(JSON.stringify({ id: n, method, params })); });
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
  const row = commits.flatMap(c => window.ReviewCoverage.commitLines(c).filter(r => r.counted && api.diff[r.path]).map(r => ({ ...r, sha: c.sha })))[0];
  if (!row) return { skipped: 'no commit line in the whole diff' };
  const sel = (lp, lk) => 'tr[data-lp="' + CSS.escape(lp) + '"][data-lk="' + lk + '"]';
  const forget = target => api.store.put(target, target.split(':')[0], { status: '', lines: [] }, {});
  location.hash = 'commits'; await settle();
  document.querySelector('[data-commit="' + row.sha + '"]').click(); await settle();
  document.querySelector('#commit-code ' + sel(row.path, row.key) + ' td.ln').click(); await settle();
  api.openFile(row.path, +row.key.split(':')[1], row.key.startsWith('old') ? 'old' : 'new'); await settle();
  const commitReachesFiles = !!document.querySelector('#file-code ' + sel(row.path, row.key) + '.read');
  forget('lines:' + row.path);
  api.store.put('file:' + row.path, 'file', { status: 'ok' }, {});
  location.hash = 'commits'; await settle();
  document.querySelector('[data-commit="' + row.sha + '"]').click(); await settle();
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
  const linkAboveEachUnreadRun = !!middle && runStarts.every(i => rows[i - 1] && rows[i - 1].matches('tr.blk'))
    && document.querySelectorAll('#file-code tr.blk').length === runStarts.length;
  document.querySelectorAll('tr.flash').forEach(tr => tr.classList.remove('flash'));
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true }));
  await new Promise(r => setTimeout(r, 50));
  const nextUnreadKey = !!document.querySelector('#file-code tr.unread.flash');
  forget('lines:' + row.path);
  return { sha: row.sha, line: row.path + ' ' + row.key, commitReachesFiles, filesReachCommits, linesMarkOneByOne, linkAboveEachUnreadRun, nextUnreadKey };
})()`;

const ARROWS_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 150));
  const crooked = [];
  let checked = 0;
  for (const route of ['overview', 'design', 'pictures']) {
    location.hash = route; await settle();
    for (const path of document.querySelectorAll('svg path[marker-end]')) {
      if (!path.getTotalLength) continue;
      const length = path.getTotalLength();
      if (length < 12) continue;
      const bends = ([a, b, c]) => Math.abs((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) > 0.5;
      const at = d => path.getPointAtLength(d);
      checked++;
      if (bends([at(0), at(4), at(8)])) crooked.push(route + ' start: ' + path.getAttribute('d'));
      if (bends([at(length), at(length - 4), at(length - 8)])) crooked.push(route + ' end: ' + path.getAttribute('d'));
    }
  }
  const slanted = [], throughBoxes = [], shared = [];
  let measured = 0, crossings = 0;
  for (const route of ['overview', 'design']) {
    location.hash = route; await settle();
    for (const svg of document.querySelectorAll('svg[data-graph]')) {
      const boxes = [...svg.querySelectorAll('[data-node] > rect')].map(el => ['x', 'y', 'width', 'height'].map(k => +el.getAttribute(k)));
      const runs = [];
      for (const path of svg.querySelectorAll('path[data-to]')) {
        measured++;
        const d = path.getAttribute('d');
        const tokens = d.match(/[MLQ]|-?[0-9.]+/g);
        let at = null;
        for (let i = 0; i < tokens.length;) {
          const command = tokens[i++];
          const take = n => tokens.slice(i, (i += n)).map(Number);
          if (command === 'M') at = take(2);
          else if (command === 'L') {
            const to = take(2);
            if (Math.abs(to[0] - at[0]) > 0.5 && Math.abs(to[1] - at[1]) > 0.5) slanted.push(route + ': ' + d);
            runs.push({ edge: path, from: at, to });
            at = to;
          } else if (command === 'Q') at = take(4).slice(2);
        }
        const length = path.getTotalLength();
        for (let s = 0; s <= length; s += 2) {
          const p = path.getPointAtLength(s);
          if (boxes.some(([x, y, w, h]) => p.x > x + 2 && p.x < x + w - 2 && p.y > y + 2 && p.y < y + h - 2)) { throughBoxes.push(route + ': ' + d); break; }
        }
      }
      runs.forEach((r, i) => runs.slice(i + 1).forEach(q => {
        if (r.edge === q.edge) return;
        const flat = u => Math.abs(u.from[1] - u.to[1]) < 0.5, upright = u => Math.abs(u.from[0] - u.to[0]) < 0.5;
        const overlap = (a0, a1, b0, b1) => Math.min(Math.max(a0, a1), Math.max(b0, b1)) - Math.max(Math.min(a0, a1), Math.min(b0, b1));
        if (flat(r) && flat(q) && Math.abs(r.from[1] - q.from[1]) < 0.5 && overlap(r.from[0], r.to[0], q.from[0], q.to[0]) > 2) shared.push(route);
        if (upright(r) && upright(q) && Math.abs(r.from[0] - q.from[0]) < 0.5 && overlap(r.from[1], r.to[1], q.from[1], q.to[1]) > 2) shared.push(route);
        const [h, v] = flat(r) && upright(q) ? [r, q] : flat(q) && upright(r) ? [q, r] : [null, null];
        if (h && v.from[0] > Math.min(h.from[0], h.to[0]) + 0.5 && v.from[0] < Math.max(h.from[0], h.to[0]) - 0.5 && h.from[1] > Math.min(v.from[1], v.to[1]) + 0.5 && h.from[1] < Math.max(v.from[1], v.to[1]) - 0.5) crossings++;
      }));
    }
  }
  if (!checked && !measured) return { skipped: 'no graph with arrows on the page' };
  return { checked, crooked: crooked.slice(0, 5), straightAtBothEnds: checked > 0 && !crooked.length, measured, crossings, slanted: slanted.slice(0, 3), rightAngled: measured > 0 && !slanted.length, throughBoxes: throughBoxes.slice(0, 3), clearOfBoxes: !throughBoxes.length, sharedRuns: shared.length, noSharedRuns: !shared.length };
})()`;

const PRESS_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 150));
  location.hash = 'design'; await settle();
  const svg = [...document.querySelectorAll('svg[data-graph]')].find(each => each.querySelector('[data-from]') && each.querySelector('.node-link'));
  if (!svg) return { skipped: 'no linked graph with edges on the design view' };
  const link = svg.querySelector('.node-link');
  const node = link.closest('[data-node]');
  const id = node.getAttribute('data-node');
  if (!svg.querySelector('[data-from="' + CSS.escape(id) + '"], [data-to="' + CSS.escape(id) + '"]')) return { skipped: 'linked box has no edges' };
  const press = (target, pointerType) => {
    target.dispatchEvent(new PointerEvent('pointerover', { bubbles: true, pointerType }));
    target.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerType }));
    target.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerType }));
    if (pointerType === 'touch') target.dispatchEvent(new PointerEvent('pointerout', { bubbles: true, pointerType, relatedTarget: document.body }));
    target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  };
  const result = {};
  for (const pointerType of ['touch', 'mouse']) {
    const body = node.querySelector('rect');
    press(body, pointerType);
    const pinned = svg.classList.contains('tracing') && location.hash === '#design';
    body.dispatchEvent(new PointerEvent('pointerout', { bubbles: true, pointerType, relatedTarget: document.body }));
    const stays = svg.classList.contains('tracing');
    press(body, pointerType);
    result[pointerType] = pinned && stays && !svg.classList.contains('tracing');
  }
  const others = [...svg.querySelectorAll('[data-node]')].filter(n => n !== node && svg.querySelector('[data-from="' + CSS.escape(n.getAttribute('data-node')) + '"], [data-to="' + CSS.escape(n.getAttribute('data-node')) + '"]'));
  if (others.length) {
    const other = others[0];
    press(node.querySelector('rect'), 'touch');
    press(other.querySelector('rect'), 'mouse');
    result.additive = node.classList.contains('traced') && other.classList.contains('traced');
    press(svg.closest('figure') || document.body, 'mouse');
    result.pressAwayKeeps = node.classList.contains('traced') && other.classList.contains('traced');
    press(node.querySelector('rect'), 'touch');
    result.untoggleOne = !node.classList.contains('traced') && other.classList.contains('traced');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    result.escapeClears = !svg.classList.contains('tracing');
  }
  press(link.querySelector('path') || link, 'touch');
  await settle();
  result.linkNavigates = location.hash !== '#design';
  result.linkLeavesTraceAlone = !svg.classList.contains('tracing');
  return result;
})()`;

const GRAPH_PROBE = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 150));
  location.hash = 'design'; await settle();
  const svg = [...document.querySelectorAll('svg[data-graph]')].find(each => each.querySelector('[data-from]'));
  if (!svg) return { skipped: 'no graph with edges on the design view' };
  const edge = svg.querySelector('path[data-from]');
  const node = svg.querySelector('[data-node="' + CSS.escape(edge.getAttribute('data-from')) + '"]');
  const target = node.querySelector('rect');
  target.dispatchEvent(new PointerEvent('pointerover', { bubbles: true, pointerType: 'mouse' }));
  target.dispatchEvent(new FocusEvent('focusin', { bubbles: true }));
  const hoverLeavesAlone = !svg.classList.contains('tracing');
  target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  const traced = svg.classList.contains('tracing') && edge.classList.contains('traced-out');
  const outward = getComputedStyle(edge).stroke;
  target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  const released = !svg.classList.contains('tracing') && !edge.classList.contains('traced-out');
  const graphs = [...document.querySelectorAll('svg[data-graph]')];
  const labelsOnTop = graphs.every(g => {
    const order = [...g.querySelectorAll('path.edge, .elabel')];
    const lastPath = order.map(el => el.matches('path.edge')).lastIndexOf(true);
    const firstLabel = order.findIndex(el => el.matches('.elabel'));
    return firstLabel === -1 || firstLabel > lastPath;
  });
  const linked = graphs.flatMap(g => [...g.querySelectorAll('[data-node]')]).filter(n => n.querySelector('.node-link'));
  const bordersOverLinks = linked.length > 0 && linked.every(n => n.lastElementChild.matches('rect.frame'));
  return { node: node.getAttribute('data-node'), hoverLeavesAlone, traced, highlighted: outward !== getComputedStyle(edge).stroke, released, labelsOnTop, bordersOverLinks };
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
    for (const route of routes) {
      await cdp.evaluate(`location.hash = ${JSON.stringify(route)}`);
      await sleep(700);
      report.routes[route] = await cdp.evaluate(`({ heading: (document.querySelector('main h1') || {}).textContent || null, unresolvedSlices: document.querySelectorAll('[data-pending-slice]').length, highlighted: document.querySelectorAll('[class^="hljs-"]').length })`);
      if (options.shots) {
        const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
        fs.writeFileSync(path.join(options.shots, `${route}.png`), Buffer.from(shot.result.data, 'base64'));
      }
    }
    report.coverage = await cdp.evaluate(COVERAGE_PROBE);
    report.commits = await cdp.evaluate(COMMITS_PROBE);
    report.unfold = await cdp.evaluate(UNFOLD_PROBE);
    report.graph = await cdp.evaluate(GRAPH_PROBE);
    report.arrows = await cdp.evaluate(ARROWS_PROBE);
    report.press = await cdp.evaluate(PRESS_PROBE);
    report.references = await cdp.evaluate(REFERENCES_PROBE);
    report.heads = await cdp.evaluate(HEADS_PROBE);
    await viewport(400);
    for (const route of routes) {
      await cdp.evaluate(`location.hash = ${JSON.stringify(route)}`);
      await sleep(400);
      const width = await cdp.evaluate('document.documentElement.scrollWidth');
      if (width > 400) report.overflow.push({ route, scrollWidth: width });
    }
    if (options.shots) {
      await cdp.evaluate("location.hash = 'overview'");
      await sleep(500);
      const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
      fs.writeFileSync(path.join(options.shots, 'overview-phone.png'), Buffer.from(shot.result.data, 'base64'));
    }
  } finally {
    await cdp.close();
  }
  const missing = Object.entries(report.routes).filter(([, r]) => !r.heading || r.unresolvedSlices).map(([route]) => route);
  const coverage = report.coverage || {};
  const coverageHolds = !!coverage.skipped || (coverage.stopReachesFiles && coverage.linesMarked && coverage.fileReachesStops);
  const commits = report.commits || {};
  const commitsHold = !!commits.skipped || !!(commits.commitReachesFiles && commits.filesReachCommits && commits.linesMarkOneByOne && commits.linkAboveEachUnreadRun && commits.nextUnreadKey);
  const unfold = report.unfold || {};
  const unfoldHolds = !!unfold.skipped || (unfold.afterOpening !== false && unfold.afterPicking !== false && (unfold.afterOpening || unfold.afterPicking) !== null);
  const graph = report.graph || {};
  const graphHolds = !!graph.skipped || !!(graph.hoverLeavesAlone && graph.traced && graph.highlighted && graph.released && graph.labelsOnTop && graph.bordersOverLinks);
  const arrows = report.arrows || {};
  const arrowsHold = !!arrows.skipped || !!(arrows.straightAtBothEnds && arrows.rightAngled && arrows.clearOfBoxes && arrows.noSharedRuns);
  const press = report.press || {};
  const pressHolds = !!press.skipped || !!(press.touch && press.mouse && press.additive && press.pressAwayKeeps && press.untoggleOne && press.escapeClears && press.linkNavigates && press.linkLeavesTraceAlone);
  const references = report.references || {};
  const referencesHold = !!references.skipped || !!(references.named && references.opensChapter && references.reachesStop);
  const heads = report.heads || {};
  const headsHold = !!heads.skipped || !!(heads.stamped && heads.rewrittenUnread && heads.marked && heads.announced && heads.untouchedCarried && heads.noteKeepsHead && heads.verdictRestamps);
  report.ok = headsHold && referencesHold && pressHolds && arrowsHold && graphHolds && commitsHold && unfoldHolds && !report.errors.length && !report.overflow.length && !missing.length && Object.keys(report.routes).length > 0 && coverageHolds;
  if (missing.length) report.incomplete = missing;
  console.log(JSON.stringify(report, null, 1));
  process.exitCode = report.ok ? 0 : 1;
}

main().catch(error => { console.error(error); process.exitCode = 2; });
