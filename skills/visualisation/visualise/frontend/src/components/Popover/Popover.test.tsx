import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { useState } from 'react'
import { Popover, type PopoverTriggerProps } from './Popover'

function ControlledPopover({
  onOpenChange,
  items = ['Option A', 'Option B', 'Option C'],
}: {
  onOpenChange?: (open: boolean) => void
  items?: string[]
}) {
  const [open, setOpen] = useState(false)
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        onOpenChange?.(next)
      }}
      trigger={(props: PopoverTriggerProps) => (
        <button {...props} ref={props.ref as React.Ref<HTMLButtonElement>}>
          Open
        </button>
      )}
    >
      {items.map((label) => (
        <div key={label} role="menuitem" tabIndex={-1}>
          {label}
        </div>
      ))}
    </Popover>
  )
}

beforeEach(() => {
  // Stub bounding rects so position assertions are deterministic.
  Object.defineProperty(HTMLElement.prototype, 'getBoundingClientRect', {
    configurable: true,
    value: function () {
      return { top: 10, left: 20, right: 60, bottom: 30, width: 40, height: 20, x: 20, y: 10, toJSON() {} }
    },
  })
})

describe('Popover', () => {
  it('renders the trigger', () => {
    render(<ControlledPopover />)
    expect(screen.getByRole('button', { name: 'Open' })).toBeInTheDocument()
  })

  it('exposes aria-haspopup="menu", aria-expanded, and aria-controls on the trigger', () => {
    render(<ControlledPopover />)
    const trigger = screen.getByRole('button', { name: 'Open' })
    expect(trigger).toHaveAttribute('aria-haspopup', 'menu')
    expect(trigger).toHaveAttribute('aria-expanded', 'false')
    expect(trigger).toHaveAttribute('aria-controls')
  })

  it('panel is hidden when closed and visible when open', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    const panel = document.querySelector('[role="menu"]')!
    expect(panel.hasAttribute('hidden')).toBe(true)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    expect(document.querySelector('[role="menu"]')!.hasAttribute('hidden')).toBe(false)
  })

  it('updates aria-expanded when toggled', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    const trigger = screen.getByRole('button', { name: 'Open' })
    await user.click(trigger)
    expect(trigger).toHaveAttribute('aria-expanded', 'true')
    await user.click(trigger)
    expect(trigger).toHaveAttribute('aria-expanded', 'false')
  })

  it('closes on click outside', async () => {
    const user = userEvent.setup()
    const onOpenChange = vi.fn()
    render(
      <div>
        <ControlledPopover onOpenChange={onOpenChange} />
        <div data-testid="outside">outside</div>
      </div>,
    )
    await user.click(screen.getByRole('button', { name: 'Open' }))
    onOpenChange.mockClear()
    await user.click(screen.getByTestId('outside'))
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })

  it('closes on Escape', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    await user.keyboard('{Escape}')
    expect(document.querySelector('[role="menu"]')!.hasAttribute('hidden')).toBe(true)
  })

  it('positions the panel below the trigger using getBoundingClientRect', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    const panel = document.querySelector('[role="menu"]') as HTMLElement
    // Trigger bottom: 30, left: 20 from beforeEach stub. Panel top = 30 + 4 = 34.
    expect(panel.style.top).toBe('34px')
    expect(panel.style.left).toBe('20px')
  })

  it('moves focus to the first menuitem on open', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    const items = screen.getAllByRole('menuitem')
    expect(document.activeElement).toBe(items[0])
  })

  it('returns focus to the trigger on close', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    const trigger = screen.getByRole('button', { name: 'Open' })
    await user.click(trigger)
    await user.keyboard('{Escape}')
    expect(document.activeElement).toBe(trigger)
  })

  it('navigates with ArrowDown/Up among menuitems', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    const items = screen.getAllByRole('menuitem')
    expect(document.activeElement).toBe(items[0])
    await user.keyboard('{ArrowDown}')
    expect(document.activeElement).toBe(items[1])
    await user.keyboard('{ArrowDown}')
    expect(document.activeElement).toBe(items[2])
    await user.keyboard('{ArrowUp}')
    expect(document.activeElement).toBe(items[1])
  })

  it('Home and End jump to first and last items', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    const items = screen.getAllByRole('menuitem')
    await user.keyboard('{End}')
    expect(document.activeElement).toBe(items[items.length - 1])
    await user.keyboard('{Home}')
    expect(document.activeElement).toBe(items[0])
  })

  it('Enter activates the focused item', async () => {
    const user = userEvent.setup()
    const onItemClick = vi.fn()
    function WithClick() {
      const [open, setOpen] = useState(false)
      return (
        <Popover
          open={open}
          onOpenChange={setOpen}
          trigger={(props) => (
            <button {...props} ref={props.ref as React.Ref<HTMLButtonElement>}>Open</button>
          )}
        >
          <div role="menuitem" tabIndex={-1} onClick={onItemClick}>A</div>
        </Popover>
      )
    }
    render(<WithClick />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    await user.keyboard('{Enter}')
    expect(onItemClick).toHaveBeenCalled()
  })

  it('Space also activates the focused item', async () => {
    const user = userEvent.setup()
    const onItemClick = vi.fn()
    function WithClick() {
      const [open, setOpen] = useState(false)
      return (
        <Popover
          open={open}
          onOpenChange={setOpen}
          trigger={(props) => (
            <button {...props} ref={props.ref as React.Ref<HTMLButtonElement>}>Open</button>
          )}
        >
          <div role="menuitem" tabIndex={-1} onClick={onItemClick}>A</div>
        </Popover>
      )
    }
    render(<WithClick />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    await user.keyboard(' ')
    expect(onItemClick).toHaveBeenCalled()
  })

  it('Tab closes the popover', async () => {
    const user = userEvent.setup()
    render(<ControlledPopover />)
    await user.click(screen.getByRole('button', { name: 'Open' }))
    await user.keyboard('{Tab}')
    expect(document.querySelector('[role="menu"]')!.hasAttribute('hidden')).toBe(true)
  })

  it('panel has id matching aria-controls', () => {
    render(<ControlledPopover />)
    const trigger = screen.getByRole('button', { name: 'Open' })
    const controls = trigger.getAttribute('aria-controls')!
    expect(document.getElementById(controls)).not.toBeNull()
  })

  it('opening a second popover dismisses the first', async () => {
    const user = userEvent.setup()
    function TwoPopovers() {
      const [openA, setOpenA] = useState(false)
      const [openB, setOpenB] = useState(false)
      return (
        <>
          <Popover
            open={openA}
            onOpenChange={setOpenA}
            trigger={(props) => (
              <button {...props} ref={props.ref as React.Ref<HTMLButtonElement>}>A</button>
            )}
          >
            <div role="menuitem" tabIndex={-1}>A1</div>
          </Popover>
          <Popover
            open={openB}
            onOpenChange={setOpenB}
            trigger={(props) => (
              <button {...props} ref={props.ref as React.Ref<HTMLButtonElement>}>B</button>
            )}
          >
            <div role="menuitem" tabIndex={-1}>B1</div>
          </Popover>
        </>
      )
    }
    render(<TwoPopovers />)
    await user.click(screen.getByRole('button', { name: 'A' }))
    await act(async () => {})
    expect(screen.getByRole('button', { name: 'A' })).toHaveAttribute('aria-expanded', 'true')
    await user.click(screen.getByRole('button', { name: 'B' }))
    await act(async () => {})
    expect(screen.getByRole('button', { name: 'A' })).toHaveAttribute('aria-expanded', 'false')
    expect(screen.getByRole('button', { name: 'B' })).toHaveAttribute('aria-expanded', 'true')
  })
})
