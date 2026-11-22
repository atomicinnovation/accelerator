(function (root) {
  'use strict';

  const ITEM_SEPARATOR = '/';

  function readItem(written) {
    try { return decodeURIComponent(written); } catch (e) { return written; }
  }

  function parse(hash) {
    const address = String(hash || '').replace(/^#/, '');
    const at = address.indexOf(ITEM_SEPARATOR);
    if (at < 0) return { view: address, item: null };
    return { view: address.slice(0, at), item: readItem(address.slice(at + 1)) || null };
  }

  const to = (view, item) => (item ? view + ITEM_SEPARATOR + encodeURIComponent(item) : view);

  const ItemAddresses = { parse, to };
  if (typeof module === 'object' && module.exports) module.exports = ItemAddresses;
  else root.ItemAddresses = ItemAddresses;
})(typeof window !== 'undefined' ? window : globalThis);
