import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'
import starlightLinksValidator from 'starlight-links-validator'
import rehypeMermaid from 'rehype-mermaid'
import rehypeAstroRelativeMarkdownLinks from 'astro-rehype-relative-markdown-links'

export default defineConfig({
  site: 'https://atomicinnovation.github.io',
  base: '/accelerator',
  markdown: {
    syntaxHighlight: { type: 'shiki', excludeLangs: ['mermaid'] },
    rehypePlugins: [
      [rehypeMermaid, { strategy: 'img-svg', dark: true }],
      [
        rehypeAstroRelativeMarkdownLinks,
        { base: '/accelerator', collectionBase: false },
      ],
    ],
  },
  integrations: [
    starlight({
      title: 'Accelerator',
      plugins: [starlightLinksValidator({ errorOnRelativeLinks: false })],
      sidebar: [
        'philosophy',
        'workflow',
        { slug: 'development-loop', label: 'Development Loop' },
        'visualiser',
        'internals',
        'configuration',
        'migrations',
        'releases-and-compatibility',
        {
          label: 'Skills Reference',
          items: [
            'skills',
            {
              slug: 'skills/development-loop',
              label: 'Development Loop (skills)',
            },
            'skills/investigation',
            'skills/work-items',
            'skills/issue-trackers',
            'skills/adrs',
            'skills/vcs-and-pr',
            'skills/review-system',
            'skills/design-convergence',
          ],
        },
      ],
    }),
  ],
})
