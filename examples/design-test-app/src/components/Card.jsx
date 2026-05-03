export function Card({ title, children }) {
  return (
    <div className="bg-surface rounded-lg border border-gray-200 p-6 shadow-sm">
      {title && <h2 className="text-lg font-semibold mb-4">{title}</h2>}
      {children}
    </div>
  )
}
