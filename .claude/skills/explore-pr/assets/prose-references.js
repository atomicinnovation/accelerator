(function (root) {
  'use strict';

  const SHIELDS = new Set(['a', 'pre']);
  const INTRODUCES_CHAPTERS = /\bchapters?[\s:]*$/i;
  const NAMES_A_CHAPTER = /^\s+chapters?\b/i;
  const JOINS_CHAPTERS = /^\s*(,|,?\s*(and|or|to|through)|&amp;|–)?\s*$/i;
  const TOKEN = /<[^>]*>|[^<]+/g;
  const TOKEN_CHARACTER = '[A-Za-z0-9_-]';

  const escapeHtml = s => String(s == null ? '' : s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const escapeRegExp = s => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const tagName = token => (token.match(/^<\/?\s*([A-Za-z0-9]+)/) || [])[1]?.toLowerCase();
  const isClosing = token => token.startsWith('</');
  const ENTRIES = { decision: ['decisions', 'dec-'], abstraction: ['abstractions', 'abs-'], invariant: ['invariants', 'inv-'] };

  const ItemAddresses = root.ItemAddresses || require('./item-addresses.js');
  const VerdictMarks = root.VerdictMarks || require('./verdict-marks.js');

  const hrefTo = (view, item) => escapeHtml('#' + ItemAddresses.to(view, item));
  function markSlot(kind, id) {
    const target = VerdictMarks.targetOf(kind, id);
    return target ? `<span class="mark" data-mark-of="${escapeHtml(target)}"></span>` : '';
  }

  function entryLinks(declared, except) {
    const links = new Map();
    (declared || []).filter(item => ENTRIES[item.kind] && item.id !== except).forEach(item => {
      const [view, prefix] = ENTRIES[item.kind];
      const label = escapeHtml(String(item.label || item.id).replace(/`/g, ''));
      const peek = escapeHtml(`${item.kind}:${item.id}`);
      links.set(item.id, `<a class="ref" href="${hrefTo(view, prefix + item.id)}" data-peek="${peek}">${markSlot(item.kind, item.id)}${label}</a>`);
    });
    return links;
  }

  function targets(chapters) {
    const chapterLinks = new Map(), stopLinks = new Map();
    chapters.forEach((chapter, i) => {
      const view = `tour-${chapter.id}`;
      chapterLinks.set(chapter.id, `<a class="ref" href="${hrefTo(view)}">${markSlot('chapter', chapter.id)}${escapeHtml(chapter.title || chapter.id)}</a>`);
      (chapter.stops || []).forEach((stop, j) => {
        const label = escapeHtml(`${i + 1}.${j + 1} ${stop.title || stop.id}`);
        stopLinks.set(stop.id, `<a class="ref" href="${hrefTo(view, 'stop-' + stop.id)}">${markSlot('stop', stop.id)}${label}</a>`);
      });
    });
    return { chapterLinks, stopLinks };
  }

  function bareLinker(links) {
    const ids = [...links.keys()].filter(id => id.includes('-')).sort((a, b) => b.length - a.length);
    if (!ids.length) return text => text;
    const bare = new RegExp(`(^|[^${TOKEN_CHARACTER.slice(1)})(${ids.map(escapeRegExp).join('|')})(?!${TOKEN_CHARACTER})`, 'g');
    return text => text.replace(bare, (_, before, id) => before + links.get(id));
  }

  function link(html, chapters, declared, except) {
    if (!html || !((chapters || []).length || (declared || []).length)) return html;
    const { chapterLinks, stopLinks } = targets(chapters || []);
    const itemLinks = entryLinks(declared, except);
    const linkBare = bareLinker(new Map([...stopLinks, ...itemLinks]));
    const tokens = String(html).match(TOKEN) || [];
    const out = [];
    let shielded = 0, inCode = 0, inChapterRun = false;
    for (let i = 0; i < tokens.length; i++) {
      const token = tokens[i];
      if (!token.startsWith('<')) {
        if (shielded || inCode) { out.push(token); continue; }
        out.push(linkBare(token));
        inChapterRun = (inChapterRun && JOINS_CHAPTERS.test(token)) || INTRODUCES_CHAPTERS.test(token);
        continue;
      }
      const name = tagName(token);
      if (SHIELDS.has(name)) { shielded += isClosing(token) ? -1 : 1; inChapterRun = false; out.push(token); continue; }
      if (name !== 'code') { out.push(token); continue; }
      const id = tokens[i + 1];
      const wholeCode = token === '<code>' && tokens[i + 2] === '</code>';
      if (shielded || !wholeCode) { inCode += isClosing(token) ? -1 : 1; out.push(token); continue; }
      const namesChapter = chapterLinks.has(id) && (inChapterRun || NAMES_A_CHAPTER.test(tokens[i + 3] || ''));
      if (stopLinks.has(id)) out.push(stopLinks.get(id));
      else if (itemLinks.has(id)) out.push(itemLinks.get(id));
      else if (namesChapter) out.push(chapterLinks.get(id));
      else out.push(token, id, tokens[i + 2]);
      inChapterRun = namesChapter;
      i += 2;
    }
    return out.join('');
  }

  const ProseReferences = { link };
  if (typeof module === 'object' && module.exports) module.exports = ProseReferences;
  else root.ProseReferences = ProseReferences;
})(typeof window !== 'undefined' ? window : globalThis);
