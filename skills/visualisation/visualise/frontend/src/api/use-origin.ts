import { useState } from 'react'

export function useOrigin(read = () => window.location.host): string {
  const [host] = useState(read)
  return host
}
