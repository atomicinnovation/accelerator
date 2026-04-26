import type { Completeness } from '../../api/types'
import { WORKFLOW_PIPELINE_STEPS } from '../../api/types'
import styles from './PipelineDots.module.css'

interface Props {
  completeness: Completeness
}

export function PipelineDots({ completeness }: Props) {
  return (
    <ul className={styles.pipeline} aria-label="Lifecycle pipeline">
      {WORKFLOW_PIPELINE_STEPS.map(step => {
        const present = Boolean(completeness[step.key])
        return (
          <li
            key={step.key}
            data-stage={step.key}
            data-present={present}
            title={step.label}
            aria-label={`${step.label}: ${present ? 'present' : 'missing'}`}
            className={`${styles.dot} ${present ? styles.present : styles.absent}`}
          />
        )
      })}
    </ul>
  )
}
