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
} as const
