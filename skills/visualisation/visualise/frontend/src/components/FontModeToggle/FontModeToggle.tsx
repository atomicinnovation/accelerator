import { useFontModeContext } from '../../api/use-font-mode'
import { TopbarIconButton } from '../TopbarIconButton/TopbarIconButton'
import styles from './FontModeToggle.module.css'

export function FontModeToggle() {
  const { fontMode, toggleFontMode } = useFontModeContext()
  const icon = fontMode === 'display' ? 'mono' : 'display'
  return (
    <TopbarIconButton
      ariaLabel="Mono font"
      ariaPressed={fontMode === 'mono'}
      dataIcon={icon}
      onClick={toggleFontMode}
    >
      <span aria-hidden="true" className={icon === 'mono' ? styles.mono : styles.display}>
        Aa
      </span>
    </TopbarIconButton>
  )
}
