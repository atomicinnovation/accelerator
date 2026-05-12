import { Outlet } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Sidebar } from '../Sidebar/Sidebar'
import { Topbar } from '../Topbar/Topbar'
import { useDocEvents, DocEventsContext } from '../../api/use-doc-events'
import { useTheme, ThemeContext } from '../../api/use-theme'
import { useFontMode, FontModeContext } from '../../api/use-font-mode'
import {
  useUnseenDocTypes,
  UnseenDocTypesContext,
} from '../../api/use-unseen-doc-types'
import { fetchTypes } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import styles from './RootLayout.module.css'

export function RootLayout() {
  const unseen = useUnseenDocTypes()
  const docEvents = useDocEvents({
    onEvent: unseen.onEvent,
    onReconnect: unseen.onReconnect,
  })
  const theme = useTheme()
  const fontMode = useFontMode()

  const { data: docTypes = [] } = useQuery({
    queryKey: queryKeys.types(),
    queryFn: fetchTypes,
  })

  return (
    <ThemeContext.Provider value={theme}>
      <FontModeContext.Provider value={fontMode}>
      <DocEventsContext.Provider value={docEvents}>
        <UnseenDocTypesContext.Provider value={unseen}>
        <div className={styles.root}>
          <Topbar />
          <div className={styles.body}>
            <Sidebar docTypes={docTypes} />
            <main className={styles.main}>
              <Outlet />
            </main>
          </div>
        </div>
        </UnseenDocTypesContext.Provider>
      </DocEventsContext.Provider>
      </FontModeContext.Provider>
    </ThemeContext.Provider>
  )
}
