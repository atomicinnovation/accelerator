import type { CSSProperties } from 'react'
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

interface StageStyle extends CSSProperties {
  '--next-accent'?: string
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
        const nextStep =
          i < WORKFLOW_PIPELINE_STEPS.length - 1
            ? WORKFLOW_PIPELINE_STEPS[i + 1]
            : null
        const nextActive = nextStep !== null && present.has(nextStep.docType)
        const accent = `var(--ac-stage-${step.docType})`
        const nextAccent =
          nextStep !== null ? `var(--ac-stage-${nextStep.docType})` : undefined
        // `color` is always set to the stage's accent so the tile
        // styling (active fill, inactive pastel/translucent via
        // `currentColor` color-mix in CSS) and the connector gradient
        // both resolve to the same hue. `--next-accent` (when present)
        // supplies the gradient end so a both-active connector blends
        // this stage → next stage.
        const stageStyle: StageStyle = { color: accent }
        if (nextAccent) stageStyle['--next-accent'] = nextAccent
        return (
          <li
            key={step.docType}
            className={`${styles.stage} ac-stagechain__stage`}
            data-stage={step.docType}
            data-active={String(active)}
            style={stageStyle}
          >
            <span className={styles.tile} aria-hidden="true">
              <Glyph
                docType={step.docType}
                size={GLYPH_SIZE[variant]}
                colorVar={active ? 'var(--atomic-white)' : accent}
              />
            </span>
            <span className={styles.label}>{step.label}</span>
            {nextStep !== null && (
              <span
                className={styles.connector}
                data-active={String(active && nextActive)}
                aria-hidden="true"
              />
            )}
          </li>
        )
      })}
    </ol>
  )
}
