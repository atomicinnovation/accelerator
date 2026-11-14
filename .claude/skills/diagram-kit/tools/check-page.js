// usage: node tools/check-page.js PAGE.html [--shots DIR] [--chrome PATH]
//
// Opens a page of diagrams in headless Chrome and checks each drawing as a
// reader would see it: the fonts loaded, no text runs outside its box, no box
// lies across another or across an arrow that is not its own, and pressing a
// box traces its arrows until Esc. Prints a JSON report and exits 1 if
// anything is wrong. Needs Node 22 and Chrome or Chromium.

const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { chromePath, headlessPage } = require('../lib/browser.js');

const USAGE = 'usage: node tools/check-page.js PAGE.html [--shots DIR] [--chrome PATH]';

const DRAWINGS = `(async () => {
  await document.fonts.ready;
  const fontsLoaded = document.fonts.check('12px "JetBrains Mono"') && document.fonts.check('11px "IBM Plex Sans"');
  const boxOf = element => { const b = element.getBBox(); return { left: b.x, top: b.y, right: b.x + b.width, bottom: b.y + b.height }; };
  const overlap = (a, b, slack = 1) => a.left < b.right - slack && b.left < a.right - slack && a.top < b.bottom - slack && b.top < a.bottom - slack;
  const beyond = (inner, outer) => Math.ceil(Math.max(outer.left - inner.left, inner.right - outer.right, outer.top - inner.top, inner.bottom - outer.bottom, 0));
  const diagrams = [...document.querySelectorAll('svg.dk')].map(svg => {
    const problems = [];
    const boxes = [...svg.querySelectorAll('.node.box')].map(node => ({ id: node.getAttribute('data-node'), node, body: boxOf(node.querySelector('.body')) }));
    for (const node of svg.querySelectorAll('.node')) {
      const body = boxOf(node.querySelector('.body'));
      for (const text of node.querySelectorAll('text')) {
        const outside = beyond(boxOf(text), body);
        if (outside > 0) problems.push('node ' + node.getAttribute('data-node') + ': text runs ' + outside + 'px outside its box');
      }
    }
    boxes.forEach((a, i) => boxes.slice(i + 1).forEach(b => { if (overlap(a.body, b.body)) problems.push('node ' + a.id + ': overlaps node ' + b.id); }));
    for (const edge of svg.querySelectorAll('.edge')) {
      const id = edge.getAttribute('data-edge'), ends = [edge.getAttribute('data-from'), edge.getAttribute('data-to')];
      const line = edge.querySelector('.line'), length = line.getTotalLength();
      const label = edge.querySelector('rect'), text = edge.querySelector('text');
      if (label && text && beyond(boxOf(text), boxOf(label)) > 0) problems.push('edge ' + id + ': label runs outside its plate');
      const crossed = new Set();
      for (let at = 0; at <= length; at += 3) {
        const point = line.getPointAtLength(at);
        boxes.forEach(box => { if (!ends.includes(box.id) && point.x > box.body.left + 1 && point.x < box.body.right - 1 && point.y > box.body.top + 1 && point.y < box.body.bottom - 1) crossed.add(box.id); });
      }
      crossed.forEach(box => problems.push('edge ' + id + ': runs across node ' + box));
    }
    const extent = svg.viewBox.baseVal;
    const stray = [...svg.querySelectorAll('.node .body, .edge .line')].some(shape => { const b = boxOf(shape); return b.left < extent.x - 1 || b.top < extent.y - 1 || b.right > extent.x + extent.width + 1 || b.bottom > extent.y + extent.height + 1; });
    if (stray) problems.push('something is drawn outside the drawing');
    return { id: svg.getAttribute('data-graph'), nodes: svg.querySelectorAll('.node').length, edges: svg.querySelectorAll('.edge').length, problems };
  });
  return { fontsLoaded, diagrams };
})()`;

const TRACING = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 120));
  const svg = [...document.querySelectorAll('svg.dk')].find(each => each.querySelector('.edge'));
  if (!svg) return { skipped: 'no drawing with an arrow' };
  const from = svg.querySelector('.edge').getAttribute('data-from');
  const node = svg.querySelector('[data-node="' + CSS.escape(from) + '"] .body');
  node.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  await settle();
  const traces = svg.classList.contains('tracing') && !!svg.querySelector('.edge.traced-out');
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await settle();
  return { traces, clears: !svg.classList.contains('tracing') };
})()`;

async function main(argv) {
  const options = { page: null, shots: null, chrome: undefined };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--shots') options.shots = argv[++i];
    else if (argv[i] === '--chrome') options.chrome = argv[++i];
    else options.page = argv[i];
  }
  if (!options.page) { console.error(USAGE); return 2; }
  const chrome = chromePath(options.chrome);
  if (!chrome) { console.error('Chrome or Chromium is needed to check a page; pass --chrome PATH'); return 2; }
  const browser = await headlessPage(chrome);
  let report;
  try {
    await browser.send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false });
    await browser.send('Page.navigate', { url: pathToFileURL(path.resolve(options.page)).href });
    for (let i = 0; i < 50 && !(await browser.evaluate('document.readyState === "complete" && !!document.querySelector("svg.dk")')); i++) await browser.pause(100);
    report = await browser.evaluate(DRAWINGS) || { fontsLoaded: false, diagrams: [] };
    report.tracing = await browser.evaluate(TRACING);
    if (options.shots) {
      fs.mkdirSync(options.shots, { recursive: true });
      for (const { id } of report.diagrams) {
        const clip = await browser.evaluate(`(() => { const figure = document.querySelector('svg.dk[data-graph="${id}"]').closest('figure') || document.querySelector('svg.dk[data-graph="${id}"]'); figure.scrollIntoView(); const r = figure.getBoundingClientRect(); return { x: r.x + scrollX, y: r.y + scrollY, width: r.width, height: r.height, scale: 2 }; })()`);
        const shot = await browser.send('Page.captureScreenshot', { format: 'png', clip, captureBeyondViewport: true });
        fs.writeFileSync(path.join(options.shots, `${id}.png`), Buffer.from(shot.result.data, 'base64'));
      }
    }
    report.errors = browser.errors;
  } finally {
    await browser.close();
  }
  const tracing = report.tracing || {};
  report.ok = report.fontsLoaded && report.diagrams.length > 0 && report.diagrams.every(each => !each.problems.length)
    && !report.errors.length && (!!tracing.skipped || !!(tracing.traces && tracing.clears));
  console.log(JSON.stringify(report, null, 1));
  return report.ok ? 0 : 1;
}

main(process.argv.slice(2)).then(status => { process.exitCode = status; });
