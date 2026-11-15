const test = require('node:test');
const assert = require('node:assert/strict');

const { DRAWING_CHECKS, TRACING_CHECK } = require('../lib/page-checks.js');

test('the checks a page is put through are expressions a browser can evaluate', () => {
  for (const check of [DRAWING_CHECKS, TRACING_CHECK]) {
    assert.match(check, /^\(async \(\) => \{/);
    assert.doesNotThrow(() => new Function(`return ${check}`));
  }
});
