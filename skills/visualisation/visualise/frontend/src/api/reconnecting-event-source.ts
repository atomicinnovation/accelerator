const INITIAL_BACKOFF_MS = 1000
const MAX_BACKOFF_MS = 30_000
const JITTER = 0.2
const MAX_ATTEMPTS = 32

export type ConnectionState = 'connecting' | 'open' | 'reconnecting' | 'closed'

export interface ReconnectOpts {
  factory: (url: string) => EventSource
  onReconnect?: () => void
  onStateChange?: (s: ConnectionState) => void
  onerror?: (e: Event) => void
  random?: () => number
}

export function computeBackoff(attempt: number, jitterSeed: number): number {
  const raw = INITIAL_BACKOFF_MS * Math.pow(2, attempt)
  const base = Math.min(raw, MAX_BACKOFF_MS)
  const mult = 1 - JITTER + jitterSeed * 2 * JITTER
  return base * mult
}

export class ReconnectingEventSource {
  private url: string
  private opts: ReconnectOpts
  private source: EventSource | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  private attempts = 0
  private state: ConnectionState = 'connecting'
  private rand: () => number

  public onmessage: ((e: MessageEvent) => void) | null = null

  constructor(url: string, opts: ReconnectOpts) {
    this.url = url
    this.opts = opts
    this.rand = opts.random ?? Math.random
    this.opts.onStateChange?.('connecting')
    this.connect()
  }

  get connectionState(): ConnectionState {
    return this.state
  }

  private setState(s: ConnectionState) {
    if (this.state === s) return
    this.state = s
    this.opts.onStateChange?.(s)
  }

  private connect() {
    if (this.state === 'closed') return
    let src: EventSource
    try {
      src = this.opts.factory(this.url)
    } catch {
      this.opts.onerror?.(new Event('error'))
      this.scheduleReconnect()
      return
    }
    src.onopen = () => {
      if (this.state === 'closed') return
      const wasReconnecting = this.state === 'reconnecting'
      this.attempts = 0
      this.setState('open')
      if (wasReconnecting) {
        this.opts.onReconnect?.()
      }
    }
    src.onerror = (e) => {
      if (this.state === 'closed') return
      this.opts.onerror?.(e)
      this.scheduleReconnect()
    }
    src.onmessage = (e) => {
      if (this.state === 'closed') return
      this.onmessage?.(e as MessageEvent)
    }
    this.source = src
  }

  private scheduleReconnect() {
    if (this.state === 'closed' || this.state === 'reconnecting') return
    this.detachAndCloseSource()
    this.setState('reconnecting')
    const delay = computeBackoff(this.attempts, this.rand())
    this.attempts = Math.min(this.attempts + 1, MAX_ATTEMPTS)
    this.timer = setTimeout(() => {
      this.timer = null
      this.connect()
    }, delay)
  }

  private detachAndCloseSource() {
    if (this.source) {
      this.source.onopen = null
      this.source.onerror = null
      this.source.onmessage = null
      try {
        this.source.close()
      } catch {
        /* ignore */
      }
      this.source = null
    }
  }

  close() {
    this.setState('closed')
    if (this.timer !== null) {
      clearTimeout(this.timer)
      this.timer = null
    }
    this.detachAndCloseSource()
  }
}
