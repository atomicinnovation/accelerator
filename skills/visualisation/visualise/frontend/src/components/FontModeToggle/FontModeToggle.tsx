import { useFontModeContext } from '../../api/use-font-mode'
import { TopbarIconButton } from '../TopbarIconButton/TopbarIconButton'

export function FontModeToggle() {
  const { fontMode, toggleFontMode } = useFontModeContext()
  return (
    <TopbarIconButton
      ariaLabel="Mono font"
      ariaPressed={fontMode === 'mono'}
      dataIcon={fontMode}
      onClick={toggleFontMode}
    >
      <svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="4 7 4 4 20 4 20 7"/>
        <line x1="9" y1="20" x2="15" y2="20"/>
        <line x1="12" y1="4" x2="12" y2="20"/>
      </svg>
    </TopbarIconButton>
  )
}
