// Screenshot mask selector merge utility.

export const DEFAULT_MASK_SELECTORS = Object.freeze(['[type=password]', '[autocomplete*=token]', '[data-secret]']);

export function mergeMaskSelectors(extra = []) {
  return [...new Set([...DEFAULT_MASK_SELECTORS, ...extra])];
}
