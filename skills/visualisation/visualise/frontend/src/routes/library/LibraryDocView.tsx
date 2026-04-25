import { useParams } from '@tanstack/react-router'
import type { DocTypeKey } from '../../api/types'

interface Props { type?: DocTypeKey; fileSlug?: string }

export function LibraryDocView({ type: propType, fileSlug: propSlug }: Props) {
  const params = useParams({ strict: false }) as { type?: string; fileSlug?: string }
  const type = propType ?? params.type
  const fileSlug = propSlug ?? params.fileSlug
  return <p>Doc: {type}/{fileSlug}</p>
}
