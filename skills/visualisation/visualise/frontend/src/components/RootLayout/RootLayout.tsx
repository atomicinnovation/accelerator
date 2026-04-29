import { Outlet } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Sidebar } from '../Sidebar/Sidebar'
import { useDocEvents, DocEventsContext } from '../../api/use-doc-events'
import { fetchTypes } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import styles from './RootLayout.module.css'

export function RootLayout() {
  const docEvents = useDocEvents()

  const { data: docTypes = [] } = useQuery({
    queryKey: queryKeys.types(),
    queryFn: fetchTypes,
  })

  return (
    <DocEventsContext.Provider value={docEvents}>
      <div className={styles.shell}>
        <Sidebar docTypes={docTypes} />
        <main className={styles.main}>
          <Outlet />
        </main>
      </div>
    </DocEventsContext.Provider>
  )
}
