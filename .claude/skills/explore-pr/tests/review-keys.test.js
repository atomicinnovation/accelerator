const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const ReviewKeys = require(path.join(__dirname, '..', 'assets', 'review-keys.js'));

const press = (key, over = {}) => ({ key, shiftKey: false, metaKey: false, ctrlKey: false, altKey: false, inField: false, ...over });
const on = (route, over = {}) => ({ route, treeFocused: false, helpOpen: false, menuOpen: false, ...over });
const commandFor = (key, context = on('files'), pressed = {}) => ReviewKeys.commandFor(press(key, pressed), context);

const ON_BOTH = {
  j: 'next-file', k: 'previous-file', ']': 'next-chunk', '[': 'previous-chunk', n: 'next-unread', N: 'previous-unread',
  m: 'mark-chunk', M: 'mark-file', r: 'reviewed', s: 'skimmed', i: 'issue', Backspace: 'clear-verdict', z: 'undo',
  x: 'toggle-selected', X: 'select-shown', Escape: 'deselect', '/': 'focus-search', u: 'toggle-still-to-read',
  g: 'cycle-grouping', '?': 'help',
};

test('each shortcut resolves to its command in All files and By commit', () => {
  for (const [key, command] of Object.entries(ON_BOTH)) {
    assert.equal(commandFor(key, on('files')), command, key);
    assert.equal(commandFor(key, on('commits')), command, key);
  }
});

test('J and K step between commits, in By commit only', () => {
  assert.equal(commandFor('J', on('commits')), 'next-commit');
  assert.equal(commandFor('K', on('commits')), 'previous-commit');
  assert.equal(commandFor('J', on('files')), null);
});

test('no shortcut works outside the two code-reading views', () => {
  for (const key of Object.keys(ON_BOTH)) assert.equal(commandFor(key, on('tour')), null, key);
});

test('a press with Meta, Control or Alt is left to the browser', () => {
  assert.equal(commandFor('j', on('files'), { metaKey: true }), null);
  assert.equal(commandFor('r', on('files'), { ctrlKey: true }), null);
  assert.equal(commandFor('z', on('files'), { altKey: true }), null);
});

test('typing in a field triggers nothing, and Esc leaves the field', () => {
  assert.equal(commandFor('j', on('files'), { inField: true }), null);
  assert.equal(commandFor('/', on('files'), { inField: true }), null);
  assert.equal(commandFor('Escape', on('files'), { inField: true }), 'blur-field');
});

test('a lower-case and an upper-case key are different shortcuts', () => {
  assert.equal(commandFor('j', on('commits')), 'next-file');
  assert.equal(commandFor('J', on('commits')), 'next-commit');
  assert.equal(commandFor('m'), 'mark-chunk');
  assert.equal(commandFor('M'), 'mark-file');
});

test('the arrow keys move through the tree only while it has focus', () => {
  const tree = on('files', { treeFocused: true });

  assert.equal(commandFor('ArrowDown', tree), 'tree-next');
  assert.equal(commandFor('ArrowUp', tree), 'tree-previous');
  assert.equal(commandFor('ArrowLeft', tree), 'tree-collapse');
  assert.equal(commandFor('ArrowRight', tree), 'tree-expand');
  assert.equal(commandFor('ArrowDown', on('files')), null);
});

test('while the shortcut list is open, only Esc and ? work, and both close it', () => {
  const helpOpen = on('files', { helpOpen: true });

  assert.equal(commandFor('Escape', helpOpen), 'close-help');
  assert.equal(commandFor('?', helpOpen), 'close-help');
  assert.equal(commandFor('j', helpOpen), null);
});

test('while a filter menu is open, keys are left to the menu', () => {
  assert.equal(commandFor('Escape', on('files', { menuOpen: true })), null);
  assert.equal(commandFor('j', on('files', { menuOpen: true })), null);
});

test('every binding is labelled and grouped, and no key is bound twice on a view', () => {
  for (const binding of ReviewKeys.BINDINGS) {
    assert.ok(binding.label, binding.command);
    assert.ok(binding.group, binding.command);
    assert.ok(binding.keys.length, binding.command);
  }
  for (const route of ['files', 'commits']) {
    for (const tree of [false, true]) {
      const keys = ReviewKeys.BINDINGS
        .filter(binding => binding.routes.includes(route) && (!binding.inTree || tree))
        .flatMap(binding => binding.keys);
      assert.equal(new Set(keys).size, keys.length, `${route} ${tree}`);
    }
  }
});
