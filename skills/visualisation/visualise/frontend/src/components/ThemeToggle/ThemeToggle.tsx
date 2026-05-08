import { useThemeContext } from '../../api/use-theme'
import { TopbarIconButton } from '../TopbarIconButton/TopbarIconButton'

export function ThemeToggle() {
  const { theme, toggleTheme } = useThemeContext()
  // Show the target mode: moon (→ dark) when light, sun (→ light) when dark.
  const icon = theme === 'light' ? 'moon' : 'sun'
  return (
    <TopbarIconButton
      ariaLabel="Dark theme"
      ariaPressed={theme === 'dark'}
      dataIcon={icon}
      onClick={toggleTheme}
    >
      {icon === 'moon' ? (
        <svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
      ) : (
        <svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="4"/>
          <line x1="12" y1="3" x2="12" y2="5.5"/>
          <line x1="12" y1="18.5" x2="12" y2="21"/>
          <line x1="3" y1="12" x2="5.5" y2="12"/>
          <line x1="18.5" y1="12" x2="21" y2="12"/>
          <line x1="5.64" y1="5.64" x2="7.4" y2="7.4"/>
          <line x1="16.6" y1="7.4" x2="18.36" y2="5.64"/>
          <line x1="16.6" y1="16.6" x2="18.36" y2="18.36"/>
          <line x1="5.64" y1="18.36" x2="7.4" y2="16.6"/>
        </svg>
      )}
    </TopbarIconButton>
  )
}
