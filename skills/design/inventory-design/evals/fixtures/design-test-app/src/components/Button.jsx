export function Button({ variant = 'primary', children, onClick }) {
  const styles = {
    primary: 'bg-primary text-white hover:opacity-90',
    secondary: 'bg-surface border border-primary text-primary hover:bg-primary-muted',
  }
  return (
    <button
      className={`px-4 py-2 rounded font-medium ${styles[variant]}`}
      onClick={onClick}
    >
      {children}
    </button>
  )
}
