// Kanban board chrome icons, composed from the unified `Icon` primitive.
// Kept as thin named wrappers (rather than inlined at call sites) because the
// eyebrow needs the tinted `IconFrame` treatment and the activity/link marks
// are dropped into the page header and card foot at fixed chrome sizes.

import { IconFrame, iconFrameInner } from "../../components/Glyph/IconFrame";
import { Icon } from "../../components/Icon/Icon";

interface IconProps {
  size?: number;
}

/** Framed kanban eyebrow glyph for `Page`'s eyebrow slot — the tinted-square
 *  treatment shared with the lifecycle eyebrow and per-doc-type framed glyphs.
 *  The three-bar mark matches the prototype `kanban` Icon and the sidebar. */
export function KanbanEyebrowIcon({ size = 16 }: IconProps) {
  return (
    <IconFrame size={size}>
      <Icon name="kanban" size={iconFrameInner(size)} />
    </IconFrame>
  );
}

/** Pulse glyph — the leading icon on the "live" chip (prototype `activity`). */
export function ActivityIcon({ size = 10 }: IconProps) {
  return <Icon name="activity" size={size} />;
}

/** Link glyph for the card foot's "N linked" meta (prototype `link`). */
export function LinkIcon({ size = 11 }: IconProps) {
  return <Icon name="link" size={size} />;
}
