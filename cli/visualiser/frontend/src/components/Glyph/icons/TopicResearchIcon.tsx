import type { ReactElement } from "react";

export function TopicResearchIcon(): ReactElement {
  return (
    <g
      fill="none"
      stroke="currentColor"
      strokeWidth="1.25"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M3 15V4A1.5 1.5 0 0 1 4.5 2.5h8.5" strokeOpacity="0.24" />
      <path d="M5 17.5V5.5A1.5 1.5 0 0 1 6.5 4H15" strokeOpacity="0.45" />
      <rect x="7" y="5.5" width="13.5" height="15" rx="1.4" />
      <path d="M10 9h7.5" />
      <path
        d="M10 12.5 14 15.5M10 15.5h3.6M10 18.5 14 15.5"
        strokeWidth="1.15"
      />
      <circle cx="15.6" cy="15.5" r="1.15" fill="currentColor" stroke="none" />
    </g>
  );
}
