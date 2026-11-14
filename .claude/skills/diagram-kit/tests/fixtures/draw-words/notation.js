module.exports = {
  kind: 'words',
  problemsIn: spec => (spec.words ? [] : ['no words']),
  remarksOn: spec => (spec.words.length > 2 ? ['More than two words.'] : []),
  htmlOf: (spec, { resolveLink }) => `<p class="words">${spec.words.map(word => (resolveLink(word) ? `<a href="${resolveLink(word).href}">${word}</a>` : word)).join(' ')}</p>`,
};
