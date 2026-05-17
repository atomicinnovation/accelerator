/**
 * A "two folded sheets of paper" empty-state hero. Tinted by the supplied
 * `hue` so it colour-matches the doc-type's radial-gradient panel.
 * Mirrors the prototype's `PaperFold` in src/view-empty.jsx.
 */
export interface PaperFoldProps {
  size?: number
  hue: number
}

export function PaperFold({ size = 72, hue }: PaperFoldProps) {
  const stroke = `hsl(${hue} 50% 50%)`
  const fill = `hsl(${hue} 78% 96%)`
  const fold = `hsl(${hue} 50% 86%)`
  const line = `hsl(${hue} 30% 78%)`
  return (
    <svg
      viewBox="0 0 80 80"
      width={size}
      height={size}
      aria-hidden="true"
      style={{ display: 'block' }}
    >
      {/* back sheet, slightly rotated */}
      <g transform="rotate(-6 26 44)">
        <rect
          x="10"
          y="20"
          width="34"
          height="44"
          rx="2"
          fill={fill}
          stroke={stroke}
          strokeWidth="1.25"
        />
        <line
          x1="16"
          y1="30"
          x2="38"
          y2="30"
          stroke={line}
          strokeWidth="1.25"
          strokeLinecap="round"
        />
        <line
          x1="16"
          y1="36"
          x2="34"
          y2="36"
          stroke={line}
          strokeWidth="1.25"
          strokeLinecap="round"
        />
      </g>
      {/* front sheet, with corner fold */}
      <g transform="rotate(5 50 42)">
        <path
          d="M30 14 L58 14 L66 22 L66 64 L30 64 Z"
          fill={fill}
          stroke={stroke}
          strokeWidth="1.4"
          strokeLinejoin="round"
        />
        <path
          d="M58 14 L58 22 L66 22 Z"
          fill={fold}
          stroke={stroke}
          strokeWidth="1.4"
          strokeLinejoin="round"
        />
        {/* placeholder content lines */}
        <line
          x1="36"
          y1="32"
          x2="58"
          y2="32"
          stroke={line}
          strokeWidth="1.4"
          strokeLinecap="round"
        />
        <line
          x1="36"
          y1="40"
          x2="60"
          y2="40"
          stroke={line}
          strokeWidth="1.4"
          strokeLinecap="round"
          strokeDasharray="2 3"
        />
        <line
          x1="36"
          y1="48"
          x2="54"
          y2="48"
          stroke={line}
          strokeWidth="1.4"
          strokeLinecap="round"
          strokeDasharray="2 3"
        />
        <line
          x1="36"
          y1="56"
          x2="58"
          y2="56"
          stroke={line}
          strokeWidth="1.4"
          strokeLinecap="round"
          strokeDasharray="2 3"
        />
      </g>
      {/* subtle "plus" badge bottom-right — invitation to create */}
      <g transform="translate(54 54)">
        <circle r="9" fill="#ffffff" stroke={stroke} strokeWidth="1.4" />
        <line
          x1="-4"
          y1="0"
          x2="4"
          y2="0"
          stroke={stroke}
          strokeWidth="1.6"
          strokeLinecap="round"
        />
        <line
          x1="0"
          y1="-4"
          x2="0"
          y2="4"
          stroke={stroke}
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      </g>
    </svg>
  )
}
