import { Button } from '../components/Button'
import { Card } from '../components/Card'

export function Home() {
  return (
    <main className="p-8">
      <h1 className="text-2xl font-bold mb-6">Home</h1>
      <Card title="Welcome">
        <p className="text-gray-600 mb-4">
          This is the home screen of the design test app.
        </p>
        <Button variant="primary">Get Started</Button>
      </Card>
    </main>
  )
}
