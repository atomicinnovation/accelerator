import type { ReactElement } from 'react'
import { Glyph } from '../Glyph/Glyph'
import { DOC_TYPE_LABELS, type DocTypeKey } from '../../api/types'

interface Props {
  type: DocTypeKey
}

export function EyebrowLabel({ type }: Props): ReactElement {
  return (
    <span data-testid="eyebrow-label">
      <Glyph docType={type} size={16} framed />
      {DOC_TYPE_LABELS[type].toUpperCase()}
    </span>
  )
}
