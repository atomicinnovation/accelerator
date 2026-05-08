import { useThemeContext } from '../../api/use-theme'
import { TopbarIconButton } from '../TopbarIconButton/TopbarIconButton'
import styles from './ThemeToggle.module.css'

export function ThemeToggle() {
  const { theme, toggleTheme } = useThemeContext()
  const icon = theme === 'light' ? 'sun' : 'moon'
  return (
    <TopbarIconButton
      ariaLabel="Dark theme"
      ariaPressed={theme === 'dark'}
      dataIcon={icon}
      onClick={toggleTheme}
    >
      <span aria-hidden="true" className={styles.glyph}>
        {icon === 'sun' ? '☀︎' : '☽︎'}
      </span>
    </TopbarIconButton>
  )
}
