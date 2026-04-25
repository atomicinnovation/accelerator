import { useParams } from '@tanstack/react-router'
import type { DocTypeKey } from '../../api/types'

interface Props { type?: DocTypeKey }

export function LibraryTypeView({ type: propType }: Props) {
  const params = useParams({ strict: false }) as { type?: string }
  const type = propType ?? params.type
  return <p>Library: {type}</p>
}
