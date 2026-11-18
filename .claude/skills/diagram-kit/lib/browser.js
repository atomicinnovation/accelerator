const { spawn, execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const CANDIDATES = [
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
  'google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser',
];
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

function chromePath(preferred = process.env.CHROME) {
  for (const candidate of [preferred, ...CANDIDATES].filter(Boolean)) {
    if (candidate.includes('/')) { if (fs.existsSync(candidate)) return candidate; continue; }
    try { return execFileSync('which', [candidate], { encoding: 'utf8' }).trim(); } catch { /* not on PATH */ }
  }
  return null;
}

async function headlessPage(chrome) {
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-chrome-'));
  const browser = spawn(chrome, ['--headless=new', '--disable-gpu', '--no-first-run', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
  const portFile = path.join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 80 && !fs.existsSync(portFile); i++) await pause(100);
  const port = fs.readFileSync(portFile, 'utf8').split('\n')[0];
  let targets = [];
  for (let i = 0; i < 40 && !targets.some(target => target.type === 'page'); i++) {
    try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); } catch { await pause(100); }
  }
  const socket = new WebSocket(targets.find(target => target.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let sent = 0;
  const waiting = new Map(), errors = [];
  socket.onmessage = message => {
    const data = JSON.parse(message.data);
    if (data.id && waiting.has(data.id)) { waiting.get(data.id)(data); waiting.delete(data.id); }
    if (data.method === 'Runtime.exceptionThrown') errors.push(data.params.exceptionDetails.exception?.description || data.params.exceptionDetails.text);
    if (data.method === 'Runtime.consoleAPICalled' && data.params.type === 'error') errors.push(data.params.args.map(arg => arg.value ?? arg.description).join(' '));
  };
  const send = (method, params = {}) => new Promise(resolve => { const id = ++sent; waiting.set(id, resolve); socket.send(JSON.stringify({ id, method, params })); });
  const evaluate = async expression => (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result.result.value;
  const close = async () => {
    socket.close();
    const exited = new Promise(resolve => browser.once('exit', resolve));
    browser.kill();
    await exited;
    try { fs.rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }); } catch { /* a leftover temp profile is harmless */ }
  };
  await send('Runtime.enable');
  await send('Page.enable');
  return { send, evaluate, errors, close, pause };
}

module.exports = { chromePath, headlessPage };
