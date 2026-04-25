import { useParams } from '@tanstack/react-router'

interface Props { name?: string }

export function LibraryTemplatesView({ name: propName }: Props) {
  const params = useParams({ strict: false }) as { name?: string }
  const name = propName ?? params.name
  return <h1>{name}</h1>
}
