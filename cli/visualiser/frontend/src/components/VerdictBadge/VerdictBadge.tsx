import { verdictToVariant } from "../../api/verdict-variant";
import { FrontmatterChip } from "../FrontmatterChip/FrontmatterChip";

export interface VerdictBadgeProps {
  value: unknown;
}

export function VerdictBadge({ value }: VerdictBadgeProps) {
  return (
    <FrontmatterChip
      name="verdict"
      value={value}
      variant={verdictToVariant(value)}
      testId="verdict-badge"
    />
  );
}
