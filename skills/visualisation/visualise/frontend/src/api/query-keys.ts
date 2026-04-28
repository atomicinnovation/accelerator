import type { DocTypeKey } from './types'

export const queryKeys = {
  types: () => ['types'] as const,
  docs: (type: DocTypeKey) => ['docs', type] as const,
  docContent: (relPath: string) => ['doc-content', relPath] as const,
  templates: () => ['templates'] as const,
  templateDetail: (name: string) => ['template-detail', name] as const,
  lifecycle: () => ['lifecycle'] as const,
  lifecycleClusterPrefix: () => ['lifecycle-cluster'] as const,
  lifecycleCluster: (slug: string) => ['lifecycle-cluster', slug] as const,
  kanban: () => ['kanban'] as const,
  related: (relPath: string) => ['related', relPath] as const,
  relatedPrefix: () => ['related'] as const,
  /** Sentinel for queries gated on a still-undefined dependency. The
   *  `__disabled__` token cannot collide with any real value-keyed
   *  query because the real keys take a typed parameter (e.g. a
   *  relPath) the caller had to provide. */
  disabled: (prefix: string) => [prefix, '__disabled__'] as const,
} as const
