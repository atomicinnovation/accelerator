(function (root) {
  'use strict';

  const BOTH = ['files', 'commits'];
  const COMMITS = ['commits'];
  const binding = (keys, command, group, label, extra = {}) => ({ keys, command, group, label, routes: BOTH, ...extra });

  const BINDINGS = [
    binding(['j'], 'next-file', 'Moving', 'Next file'),
    binding(['k'], 'previous-file', 'Moving', 'Previous file'),
    binding(['J'], 'next-commit', 'Moving', 'Next commit', { routes: COMMITS }),
    binding(['K'], 'previous-commit', 'Moving', 'Previous commit', { routes: COMMITS }),
    binding([']'], 'next-chunk', 'Moving', 'Next chunk of changed lines'),
    binding(['['], 'previous-chunk', 'Moving', 'Previous chunk of changed lines'),
    binding(['n'], 'next-unread', 'Moving', 'Next line still to read'),
    binding(['N'], 'previous-unread', 'Moving', 'Previous line still to read'),
    binding(['m'], 'mark-chunk', 'Reading', 'Mark the chunk read, or unmark it'),
    binding(['M'], 'mark-file', 'Reading', 'Mark the rest of the file read'),
    binding(['r'], 'reviewed', 'Verdicts', 'Reviewed: the selection, or else this file'),
    binding(['s'], 'skimmed', 'Verdicts', 'Skimmed: the selection, or else this file'),
    binding(['i'], 'issue', 'Verdicts', 'Raise an issue on this file, noting the chunk'),
    binding(['Backspace'], 'clear-verdict', 'Verdicts', 'Clear the verdict: the selection, or else this file'),
    binding(['z'], 'undo', 'Verdicts', 'Undo the last change'),
    binding(['x'], 'toggle-selected', 'Selecting', 'Select this file, or deselect it'),
    binding(['X'], 'select-shown', 'Selecting', 'Select every file shown'),
    binding(['Escape'], 'deselect', 'Selecting', 'Deselect every file'),
    binding(['/'], 'focus-search', 'Finding files', 'Search the paths'),
    binding(['u'], 'toggle-still-to-read', 'Finding files', 'Show only files still to read, or every file'),
    binding(['g'], 'cycle-grouping', 'Finding files', 'Group the files the next way'),
    binding(['?'], 'help', 'Finding files', 'Show these shortcuts'),
    binding(['ArrowDown'], 'tree-next', 'In the file list', 'Next row', { inTree: true }),
    binding(['ArrowUp'], 'tree-previous', 'In the file list', 'Previous row', { inTree: true }),
    binding(['ArrowLeft'], 'tree-collapse', 'In the file list', 'Collapse the folder, or go to its parent', { inTree: true }),
    binding(['ArrowRight'], 'tree-expand', 'In the file list', 'Expand the folder', { inTree: true }),
  ];

  const ESCAPE = 'Escape';
  const servesRoute = route => BINDINGS.some(each => each.routes.includes(route));

  function commandFor(press, context) {
    if (press.metaKey || press.ctrlKey || press.altKey || !servesRoute(context.route)) return null;
    if (context.helpOpen) return press.key === ESCAPE || press.key === '?' ? 'close-help' : null;
    if (context.menuOpen) return null;
    if (press.inField) return press.key === ESCAPE ? 'blur-field' : null;
    const found = BINDINGS.find(each => each.keys.includes(press.key) && each.routes.includes(context.route) && (!each.inTree || context.treeFocused));
    return found ? found.command : null;
  }

  const ReviewKeys = { BINDINGS, commandFor };
  if (typeof module === 'object' && module.exports) module.exports = ReviewKeys;
  else root.ReviewKeys = ReviewKeys;
})(typeof window !== 'undefined' ? window : globalThis);
