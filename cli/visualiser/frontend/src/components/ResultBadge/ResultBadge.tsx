import { resultToVariant } from "../../api/result-variant";
import { FrontmatterChip } from "../FrontmatterChip/FrontmatterChip";

export interface ResultBadgeProps {
  value: unknown;
}

export function ResultBadge({ value }: ResultBadgeProps) {
  return (
    <FrontmatterChip
      name="result"
      value={value}
      variant={resultToVariant(value)}
      testId="result-badge"
    />
  );
}
