module.exports = {
  kind: 'words',
  problemsIn: spec => (spec.words ? [] : ['no words']),
  remarksOn: spec => (spec.words.length > 2 ? ['More than two words.'] : []),
  symbols: [
    { name: 'Word', means: 'One word.', html: '<b>word</b>', usedIn: () => true },
    { name: 'Link', means: 'A word that opens something.', html: '<a>word</a>', usedIn: spec => spec.words.some(word => word.startsWith('#')) },
  ],
  htmlOf: (spec, { resolveLink }) => `<p class="words">${spec.words.map(word => (resolveLink(word) ? `<a href="${resolveLink(word).href}">${word}</a>` : word)).join(' ')}</p>`,
};
