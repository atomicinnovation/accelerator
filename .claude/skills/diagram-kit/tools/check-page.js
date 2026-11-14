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
const { DRAWING_CHECKS, SELF_DRAWN_CHECKS, TRACING_CHECK } = require('../lib/page-checks.js');

const USAGE = 'usage: node tools/check-page.js PAGE.html [--shots DIR] [--chrome PATH]';

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
    for (let i = 0; i < 50 && !(await browser.evaluate('document.readyState === "complete" && !!document.querySelector("svg.dk, .dk-drawn")')); i++) await browser.pause(100);
    report = await browser.evaluate(DRAWING_CHECKS) || { fontsLoaded: false, diagrams: [] };
    report.diagrams.push(...await browser.evaluate(SELF_DRAWN_CHECKS));
    report.tracing = await browser.evaluate(TRACING_CHECK);
    if (options.shots) {
      fs.mkdirSync(options.shots, { recursive: true });
      for (const { id } of report.diagrams) {
        const clip = await browser.evaluate(`(() => { const drawing = document.querySelector('svg.dk[data-graph="${id}"]'); const figure = document.getElementById('diagram-${id}') || drawing.closest('figure') || drawing; figure.scrollIntoView(); const r = figure.getBoundingClientRect(); return { x: r.x + scrollX, y: r.y + scrollY, width: r.width, height: r.height, scale: 2 }; })()`);
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
