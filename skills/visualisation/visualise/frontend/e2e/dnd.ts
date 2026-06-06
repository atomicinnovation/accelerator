import type { Page } from '@playwright/test'

interface Point {
  x: number
  y: number
}

async function centreOf(page: Page, selector: string): Promise<Point> {
  const locator = page.locator(selector)
  // Scroll into view before measuring: the mouse-driven drag uses absolute
  // viewport coordinates, so a target below the fold (e.g. a card low in a
  // tall column) would otherwise yield an off-screen centre and the pointer
  // events would miss it entirely.
  await locator.scrollIntoViewIfNeeded()
  const box = (await locator.boundingBox())!
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 }
}

/**
 * A drag in progress — the pointer is down and has crossed dnd-kit's 5px
 * PointerSensor activation threshold. Lets a test inspect mid-drag state
 * (between `moveTo` and `drop`) before releasing.
 */
export interface ActiveDrag {
  /** Move the pointer to the centre of `targetSelector` (no release). */
  moveTo(targetSelector: string): Promise<void>
  /** Move the pointer by a relative offset (no release). */
  moveBy(dx: number, dy: number): Promise<void>
  /** Release the pointer, completing the drop. */
  drop(): Promise<void>
}

/**
 * Begin a drag from `sourceSelector`, crossing the 5px activation threshold so
 * dnd-kit's PointerSensor fires (a single `dragTo` would not). Returns a handle
 * for the remaining decomposed steps so callers can assert state mid-drag.
 */
export async function startDrag(page: Page, sourceSelector: string): Promise<ActiveDrag> {
  const src = await centreOf(page, sourceSelector)
  await page.mouse.move(src.x, src.y)
  await page.mouse.down()
  // Move well past the 5px activation threshold so dnd-kit fires before the
  // anchor click.
  await page.mouse.move(src.x + 30, src.y, { steps: 10 })
  return {
    async moveTo(targetSelector: string) {
      const tgt = await centreOf(page, targetSelector)
      await page.mouse.move(tgt.x, tgt.y, { steps: 20 })
    },
    async moveBy(dx: number, dy: number) {
      const cur = await centreOf(page, sourceSelector)
      await page.mouse.move(cur.x + dx, cur.y + dy, { steps: 10 })
    },
    async drop() {
      await page.mouse.up()
    },
  }
}

/** Single-call convenience for drop-only tests (down → move → up). */
export async function dndDrag(
  page: Page,
  sourceSelector: string,
  targetSelector: string,
): Promise<void> {
  const drag = await startDrag(page, sourceSelector)
  await drag.moveTo(targetSelector)
  await drag.drop()
}
