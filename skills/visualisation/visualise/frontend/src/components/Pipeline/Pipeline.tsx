import type { Completeness } from '../../api/types'
import { WORKFLOW_PIPELINE_STEPS } from '../../api/types'
import { Glyph } from '../Glyph/Glyph'
import styles from './Pipeline.module.css'

export type PipelineVariant = 'card' | 'panel'

interface Props {
  completeness: Completeness
  variant?: PipelineVariant
}

const GLYPH_SIZE: Record<PipelineVariant, 16 | 24> = {
  card: 16,
  panel: 24,
}

export function Pipeline({ completeness, variant = 'card' }: Props) {
  const present = new Set(completeness.present)
  return (
    <ol
      className={`${styles.chain} ac-stagechain`}
      data-variant={variant}
      aria-label={`Lifecycle pipeline, ${present.size} of 8 stages complete`}
    >
      {WORKFLOW_PIPELINE_STEPS.map((step, i) => {
        const active = present.has(step.docType)
        const nextActive =
          i < WORKFLOW_PIPELINE_STEPS.length - 1 &&
          present.has(WORKFLOW_PIPELINE_STEPS[i + 1].docType)
        const accent = `var(--ac-stage-${step.docType})`
        return (
          <li
            key={step.docType}
            className={`${styles.stage} ac-stagechain__stage`}
            data-stage={step.docType}
            data-active={String(active)}
            style={active ? { color: accent } : undefined}
          >
            <span className={styles.tile} aria-hidden="true">
              <Glyph docType={step.docType} size={GLYPH_SIZE[variant]} />
            </span>
            <span className={styles.label}>{step.label}</span>
            {i < WORKFLOW_PIPELINE_STEPS.length - 1 && (
              <span
                className={styles.connector}
                data-active={String(active && nextActive)}
                style={active && nextActive ? { background: accent } : undefined}
                aria-hidden="true"
              />
            )}
          </li>
        )
      })}
    </ol>
  )
}
