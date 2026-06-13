// Lifecycle-page chrome icons, composed from the unified `Icon` primitive.
// Kept as thin named wrappers (rather than inlined at every call site) because
// the eyebrow needs the tinted `IconFrame` treatment and the clock is dropped
// into the meta row at a fixed chrome size; both read more clearly named.

import { IconFrame, iconFrameInner } from "../../components/Glyph/IconFrame";
import { Icon } from "../../components/Icon/Icon";

interface IconProps {
  size?: number;
}

/** Framed lifecycle eyebrow glyph — drop-in for `Page`'s eyebrow slot on
 *  lifecycle index/detail. Matches the tinted-square treatment that
 *  per-doc-type `Glyph framed` uses on library/templates pages. */
export function LifecycleEyebrowIcon({ size = 16 }: IconProps) {
  return (
    <IconFrame size={size}>
      <Icon name="lifecycle" size={iconFrameInner(size)} />
    </IconFrame>
  );
}

export function ClockIcon({ size = 11 }: IconProps) {
  return <Icon name="clock" size={size} />;
}
