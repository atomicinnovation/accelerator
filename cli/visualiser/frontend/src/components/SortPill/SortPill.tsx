import { useState } from "react";
import { Icon } from "../Icon/Icon";
import { Popover } from "../Popover/Popover";
import styles from "./SortPill.module.css";

export type SortOption =
  | "recently-modified"
  | "oldest-first"
  | "title-asc"
  | "title-desc"
  | "id-asc";

interface OptionMeta {
  id: SortOption;
  label: string;
}

const OPTIONS: OptionMeta[] = [
  { id: "recently-modified", label: "Recently modified" },
  { id: "oldest-first", label: "Oldest first" },
  { id: "title-asc", label: "Title (A → Z)" },
  { id: "title-desc", label: "Title (Z → A)" },
  { id: "id-asc", label: "ID (ascending)" },
];

export interface SortPillProps {
  value: SortOption;
  onChange: (next: SortOption) => void;
}

export function SortPill({ value, onChange }: SortPillProps) {
  const [open, setOpen] = useState(false);
  const current = OPTIONS.find((o) => o.id === value) ?? OPTIONS[0];

  return (
    <Popover
      open={open}
      onOpenChange={setOpen}
      ariaLabel="Sort options"
      trigger={(triggerProps) => (
        <button
          {...triggerProps}
          ref={triggerProps.ref as React.Ref<HTMLButtonElement>}
          className={`${styles.trigger} ${open ? styles.triggerOpen : ""}`}
          data-testid="sort-trigger"
        >
          <Icon name="sort" size={12} />
          <span>{current.label}</span>
          <Icon name="chevron-down" size={11} />
        </button>
      )}
    >
      <div className={styles.menu}>
        <div className={styles.menuHeader}>Sort by</div>
        <ul className={styles.menuList}>
          {OPTIONS.map((opt) => {
            const selected = opt.id === value;
            const select = () => {
              onChange(opt.id);
              setOpen(false);
            };
            return (
              // biome-ignore lint/a11y/useAriaPropsSupportedByRole: this single-select menu deliberately pairs role="menuitem" with aria-checked to expose the active sort; the unit tests assert getAllByRole("menuitem") + aria-checked together, so switching to menuitemradio would break the contract
              <li
                key={opt.id}
                // biome-ignore lint/a11y/noNoninteractiveElementToInteractiveRole: <li role="menuitem"> is the canonical menu-item markup inside this role-less menu container; the unit tests query getByRole("menuitem"), so it cannot be downgraded
                role="menuitem"
                tabIndex={-1}
                className={`${styles.menuItem} ${selected ? styles.menuItemActive : ""}`}
                aria-checked={selected}
                onClick={select}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    select();
                  }
                }}
              >
                <span>{opt.label}</span>
                {selected && <Icon name="check" size={12} />}
              </li>
            );
          })}
        </ul>
      </div>
    </Popover>
  );
}
