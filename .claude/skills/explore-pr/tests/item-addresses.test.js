const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const ItemAddresses = require(path.join(__dirname, '..', 'assets', 'item-addresses.js'));

test('a hash naming only a view opens that view at no item', () => {
  assert.deepEqual(ItemAddresses.parse('#decisions'), { view: 'decisions', item: null });
});

test('a hash naming a view and an item opens the view at that item', () => {
  assert.deepEqual(ItemAddresses.parse('#decisions/dec-cap-depth'), { view: 'decisions', item: 'dec-cap-depth' });
});

test('a chapter of the tour keeps its own view, with the stop as its item', () => {
  assert.deepEqual(ItemAddresses.parse('#tour-claims/stop-claims-s1'), { view: 'tour-claims', item: 'stop-claims-s1' });
});

test('an empty hash opens no view in particular', () => {
  assert.deepEqual(ItemAddresses.parse(''), { view: '', item: null });
});

test('an item whose name had to be escaped reads back as written', () => {
  const address = ItemAddresses.to('decisions', 'dec-a/b c');

  assert.deepEqual(ItemAddresses.parse('#' + address), { view: 'decisions', item: 'dec-a/b c' });
});

test('an item escaped wrongly is kept as written rather than failing', () => {
  assert.deepEqual(ItemAddresses.parse('#decisions/dec-%E0'), { view: 'decisions', item: 'dec-%E0' });
});

test('an address with no item is the view alone', () => {
  assert.equal(ItemAddresses.to('decisions'), 'decisions');
  assert.equal(ItemAddresses.to('decisions', ''), 'decisions');
});

test('an address with an item joins the two with a slash', () => {
  assert.equal(ItemAddresses.to('invariants', 'inv-claims-i1'), 'invariants/inv-claims-i1');
});
