/* Always-dark code palette duplicated from the visualiser's --code-*
   and --tk-* tokens (frontend/src/styles/global.css); drift-guarded by
   tests/unit/tasks/test_docs_theme_drift.py, which parses this map. */
export const codeColours = {
  bg: '#0e1320',
  fg: '#d7dcec',
  comment: '#6f7796',
  string: '#6be58b',
  number: '#f9de6f',
  keyword: '#c1c5ff',
  literal: '#f9a66b',
  type: '#73e4e2',
  function: '#ffc1a8',
  attribute: '#c18cf0',
  variable: '#72cbf5',
  punctuation: '#8990b0',
  tag: '#df5758',
  diffInserted: '#6be58b',
  diffDeleted: '#e56b7e',
}

export const atomicCodeTheme = {
  name: 'atomic-code',
  type: 'dark',
  colors: {
    'editor.background': codeColours.bg,
    'editor.foreground': codeColours.fg,
  },
  tokenColors: [
    {
      scope: ['comment', 'punctuation.definition.comment'],
      settings: { foreground: codeColours.comment },
    },
    {
      scope: ['string', 'punctuation.definition.string'],
      settings: { foreground: codeColours.string },
    },
    {
      scope: ['constant.numeric'],
      settings: { foreground: codeColours.number },
    },
    {
      scope: [
        'keyword',
        'keyword.control',
        'keyword.operator',
        'storage',
        'storage.type',
        'storage.modifier',
      ],
      settings: { foreground: codeColours.keyword },
    },
    {
      scope: [
        'constant.language',
        'constant.character',
        'constant.other',
        'support.constant',
      ],
      settings: { foreground: codeColours.literal },
    },
    {
      scope: [
        'entity.name.type',
        'entity.name.class',
        'entity.other.inherited-class',
        'support.type',
        'support.class',
      ],
      settings: { foreground: codeColours.type },
    },
    {
      scope: [
        'entity.name.function',
        'support.function',
        'meta.function-call entity.name.function',
      ],
      settings: { foreground: codeColours.function },
    },
    {
      scope: [
        'entity.other.attribute-name',
        'entity.name.tag.yaml',
        'meta.attribute',
        'meta.annotation',
        'meta.decorator',
        'punctuation.definition.annotation',
        'punctuation.definition.decorator',
      ],
      settings: { foreground: codeColours.attribute },
    },
    {
      scope: [
        'variable',
        'variable.other',
        'variable.parameter',
        'support.variable',
      ],
      settings: { foreground: codeColours.variable },
    },
    {
      scope: [
        'punctuation',
        'meta.brace',
        'punctuation.definition.tag',
      ],
      settings: { foreground: codeColours.punctuation },
    },
    {
      scope: ['entity.name.tag'],
      settings: { foreground: codeColours.tag },
    },
    {
      scope: ['markup.inserted'],
      settings: { foreground: codeColours.diffInserted },
    },
    {
      scope: ['markup.deleted'],
      settings: { foreground: codeColours.diffDeleted },
    },
  ],
}
