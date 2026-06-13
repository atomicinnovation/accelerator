import { useEffect, useState } from "react";

/** Returns `true` only once `active` has held continuously for `delayMs`.
 *  Mirrors the deferral convention of `useDeferredFetchingHint` but keyed on
 *  an arbitrary boolean (the suggestion fan-out's `isPending`) rather than a
 *  query's refetch status — so the common warm-cache path (the fan-out settles
 *  near-instantly) never flashes the working hint. Resets immediately when
 *  `active` goes false. */
export function useDelayedFlag(active: boolean, delayMs = 250): boolean {
  const [show, setShow] = useState(false);
  useEffect(() => {
    if (!active) {
      setShow(false);
      return;
    }
    const id = setTimeout(() => setShow(true), delayMs);
    return () => clearTimeout(id);
  }, [active, delayMs]);
  return show;
}
