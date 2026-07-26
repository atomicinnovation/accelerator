import { useThemeContext } from "../../api/use-theme";
import { Icon } from "../Icon/Icon";
import { TopbarIconButton } from "../TopbarIconButton/TopbarIconButton";

export function ThemeToggle() {
  const { theme, toggleTheme } = useThemeContext();
  // Show the target mode: moon (→ dark) when light, sun (→ light) when dark.
  const icon = theme === "light" ? "moon" : "sun";
  return (
    <TopbarIconButton
      ariaLabel="Dark theme"
      ariaPressed={theme === "dark"}
      dataIcon={icon}
      onClick={toggleTheme}
    >
      <Icon name={icon} size={16} />
    </TopbarIconButton>
  );
}
