/** Diff-semantic tints for the `pr-reviews` illustration ONLY. These use fixed
 *  hues (140 green / 0 red) that deliberately ignore the doc-type hue, so they
 *  are NOT members of the seven-tone `bigPalette`. They are sanctioned
 *  non-palette structural constants and must equal the prototype's
 *  big-glyphs.jsx diff tints exactly. */
export const PR_REVIEW_DIFF_TINTS = {
  addedBg: "hsl(140 60% 85%)",
  addedMarker: "hsl(140 50% 40%)",
  removedBg: "hsl(0 65% 88%)",
  removedMarker: "hsl(0 55% 45%)",
} as const;
