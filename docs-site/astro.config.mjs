import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'
import starlightLinksValidator from 'starlight-links-validator'
import starlightImageZoom from 'starlight-image-zoom'
import rehypeMermaid from 'rehype-mermaid'
import rehypeAstroRelativeMarkdownLinks from 'astro-rehype-relative-markdown-links'
import { atomicCodeTheme } from './shiki-atomic.mjs'

// The hosting decision lives here alone: everything below derives from
// these two values, overridable for forks or a future custom domain.
const site = process.env.DOCS_SITE ?? 'https://atomicinnovation.github.io'
const base = process.env.DOCS_BASE ?? '/accelerator'
const ogImage = `${site}${base}/accelerator_logo_light_bg.png`

export default defineConfig({
  site,
  base,
  markdown: {
    syntaxHighlight: { type: 'shiki', excludeLangs: ['mermaid'] },
    shikiConfig: { theme: atomicCodeTheme },
    rehypePlugins: [
      [rehypeMermaid, { strategy: 'img-svg', dark: true }],
      [
        rehypeAstroRelativeMarkdownLinks,
        { base, collectionBase: false },
      ],
    ],
  },
  integrations: [
    starlight({
      title: 'Accelerator',
      expressiveCode: {
        themes: [atomicCodeTheme],
        minSyntaxHighlightingColorContrast: 0,
        styleOverrides: { borderColor: 'rgba(255, 255, 255, 0.07)' },
      },
      customCss: ['./src/styles/theme.css', './src/styles/custom.css'],
      lastUpdated: true,
      editLink: {
        baseUrl:
          'https://github.com/atomicinnovation/accelerator/edit/main/docs-site/',
      },
      favicon: '/accelerator_logo_light_bg.png',
      logo: {
        light: './src/assets/accelerator_logo_light_bg.png',
        dark: './src/assets/accelerator_logo_dark_bg.png',
        replacesTitle: true,
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/atomicinnovation/accelerator',
        },
      ],
      head: [
        {
          tag: 'meta',
          attrs: {
            property: 'og:image',
            content: ogImage,
          },
        },
        {
          tag: 'meta',
          attrs: {
            name: 'twitter:image',
            content: ogImage,
          },
        },
      ],
      plugins: [
        starlightImageZoom(),
        starlightLinksValidator({ errorOnRelativeLinks: false }),
      ],
      sidebar: [
        {
          label: 'Start Here',
          items: [
            'getting-started',
            'philosophy',
            'workflow',
            'case-study',
            { slug: 'development-loop', label: 'Development Loop' },
          ],
        },
        {
          label: 'Guides',
          items: [
            'guides/which-skill',
            'guides/plan-a-feature',
            'guides/review-a-pr',
            'guides/capture-a-decision',
            'guides/sync-work-items',
            'guides/configuration-cookbook',
            'guides/faq',
            'configuration',
            'migrations',
            'releases-and-compatibility',
            'visualiser',
          ],
        },
        {
          label: 'Reference',
          items: [
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
            {
              label: 'Skills',
              collapsed: true,
              items: [{ autogenerate: { directory: 'reference/skills' } }],
            },
            'reference/agents',
            'reference/meta-directory',
            'internals',
          ],
        },
      ],
    }),
  ],
})
