import { Button } from '../components/Button'

export function Settings() {
  return (
    <main className="p-8">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>
      <div className="space-y-4">
        <p className="text-gray-600">Manage your preferences here.</p>
        <Button variant="secondary">Save Changes</Button>
      </div>
    </main>
  )
}
