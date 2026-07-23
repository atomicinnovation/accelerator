import type { ReactElement } from "react";

// Fishbone / root-cause diagram: a central spine with converging branch
// bones and a filled node at the head (the identified root cause). Lifted
// from the design prototype's `root-cause-analyses` TypeGlyph.
export function RootCauseAnalysesIcon(): ReactElement {
  return (
    <g
      fill="none"
      stroke="currentColor"
      strokeWidth="1.25"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M3.5 12h12" />
      <path d="M6 6.5 8.5 12M11 6.5 13.5 12M6 17.5 8.5 12M11 17.5 13.5 12" />
      <circle cx="6" cy="6.5" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="11" cy="6.5" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="6" cy="17.5" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="11" cy="17.5" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="18" cy="12" r="2.2" fill="currentColor" stroke="none" />
    </g>
  );
}
