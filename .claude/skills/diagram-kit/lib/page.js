const fs = require('node:fs');
const path = require('node:path');

const KIT = path.resolve(__dirname, '..');
const FACES = [
  ['IBM Plex Sans', 400, 'ibm-plex-sans-latin-400-normal.woff2'],
  ['IBM Plex Sans', 600, 'ibm-plex-sans-latin-600-normal.woff2'],
  ['JetBrains Mono', 400, 'jetbrains-mono-latin-400-normal.woff2'],
  ['JetBrains Mono', 600, 'jetbrains-mono-latin-600-normal.woff2'],
];
const read = file => fs.readFileSync(path.join(KIT, file), 'utf8');
const escaped = text => String(text ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

function embeddedFonts() {
  return FACES.map(([family, weight, file]) => {
    const data = fs.readFileSync(path.join(KIT, 'vendor/fonts', file)).toString('base64');
    return `@font-face{font-family:"${family}";font-weight:${weight};font-display:block;src:url(data:font/woff2;base64,${data}) format("woff2")}`;
  }).join('\n');
}

const styles = () => `${embeddedFonts()}\n${read('lib/tokens.css')}\n${read('lib/diagram.css')}`;
const inlineScript = code => `<script>\n${code.replace(/<\/(script)/gi, '<\\/$1')}\n</script>`;

function standalonePage(title, figures) {
  return `<!doctype html>
<html lang="en-GB"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${escaped(title)}</title>
<style>
${styles()}
html, body { background: var(--bg); color: var(--ink); }
body { margin: 0; padding: 24px 16px; font-family: var(--f-body); font-size: 15px; line-height: 1.55; }
main { max-width: 1100px; margin: 0 auto; display: grid; gap: 16px; }
</style></head>
<body><main>${figures.join('\n')}</main>
${inlineScript(read('lib/trace.js'))}
${inlineScript('DiagramKit.attachTracing(document);')}
</body></html>
`;
}

function standaloneSvg(svg) {
  const style = `<style>${styles()}\nsvg.dk { background: var(--panel); }</style>`;
  return svg.replace(/^(<svg[^>]*>)/, `$1${style}`);
}

module.exports = { standalonePage, standaloneSvg };
