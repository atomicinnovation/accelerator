import { Outlet } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Sidebar } from '../Sidebar/Sidebar'
import { Topbar } from '../Topbar/Topbar'
import { useDocEvents, DocEventsContext } from '../../api/use-doc-events'
import { useTheme, ThemeContext } from '../../api/use-theme'
import { fetchTypes } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import styles from './RootLayout.module.css'

export function RootLayout() {
  const docEvents = useDocEvents()
  const theme = useTheme()

  const { data: docTypes = [] } = useQuery({
    queryKey: queryKeys.types(),
    queryFn: fetchTypes,
  })

  return (
    <ThemeContext.Provider value={theme}>
      <DocEventsContext.Provider value={docEvents}>
        <div className={styles.root}>
          <Topbar />
          <div className={styles.body}>
            <Sidebar docTypes={docTypes} />
            <main className={styles.main}>
              <Outlet />
            </main>
          </div>
        </div>
      </DocEventsContext.Provider>
    </ThemeContext.Provider>
  )
}
