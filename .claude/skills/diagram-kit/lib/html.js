(function (root) {
  'use strict';

  const escaped = text => String(text ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const inlineOf = text => escaped(text).replace(/`([^`]+)`/g, '<code>$1</code>');
  const attributesOf = target => Object.entries(target.attributes || {}).map(([name, value]) => ` ${name}="${escaped(value)}"`).join('');
  const linked = (markup, target) => (target ? `<a href="${escaped(target.href)}"${attributesOf(target)}${target.label ? ` aria-label="${escaped(target.label)}"` : ''}>${markup}</a>` : markup);
  const addressOnly = reference => (/^(https?:|#)/.test(String(reference)) ? { href: String(reference), label: `Open ${reference}` } : null);

  const api = { escaped, inlineOf, linked, addressOnly };
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
