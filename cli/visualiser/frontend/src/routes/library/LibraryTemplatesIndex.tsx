import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { fetchTemplates } from "../../api/fetch";
import { queryKeys } from "../../api/query-keys";
import type {
  TemplateSummary,
  TemplateTier,
  TemplateTierSource,
} from "../../api/types";
import { EyebrowLabel } from "../../components/EyebrowLabel/EyebrowLabel";
import { Glyph } from "../../components/Glyph/Glyph";
import { Icon } from "../../components/Icon/Icon";
import { Page } from "../../components/Page/Page";
import styles from "./LibraryTemplatesIndex.module.css";
import {
  glyphKeyForTemplate,
  TIER_ORDER,
  TIER_SHORT_LABELS,
} from "./template-tier";

type TierState = "absent" | "present" | "active";

function tierStateFor(t: TemplateTier | undefined): TierState {
  if (!t?.present) return "absent";
  if (t.active) return "active";
  return "present";
}

/** Chevron-right marker — used as the inter-pill separator and the row
 *  disclosure marker. Thin wrapper over the unified `Icon` primitive. */
export function ChevronRightIcon({
  size = 10,
  className,
}: {
  size?: number;
  className?: string;
}) {
  return <Icon name="chevron-right" size={size} className={className} />;
}

interface TierPillsProps {
  tiers: TemplateTier[];
}

export function TierPills({ tiers }: TierPillsProps) {
  const byKey = new Map<TemplateTierSource, TemplateTier>(
    tiers.map((t) => [t.source, t]),
  );
  return (
    <span className={styles.tierChain}>
      {TIER_ORDER.map((source, idx) => {
        const t = byKey.get(source);
        const state = tierStateFor(t);
        return (
          <span key={source} className={styles.tierChainItem}>
            {idx > 0 ? (
              <ChevronRightIcon size={10} className={styles.tierSeparator} />
            ) : null}
            <span className={styles.tierPill} data-state={state}>
              <span className={styles.tierPillBullet} aria-hidden="true" />
              <span className={styles.tierPillLabel}>
                {TIER_SHORT_LABELS[source]}
              </span>
            </span>
          </span>
        );
      })}
    </span>
  );
}

interface TemplatesIndexListProps {
  templates: TemplateSummary[];
  selectedName?: string;
}

export function TemplatesIndexList({
  templates,
  selectedName,
}: TemplatesIndexListProps) {
  return (
    <ul className={styles.list}>
      {templates.map((t) => {
        const glyphKey = glyphKeyForTemplate(t.name);
        const isSelected = selectedName === t.name;
        return (
          <li key={t.name} className={styles.row}>
            <Link
              to="/library/templates/$name"
              params={{ name: t.name }}
              className={styles.rowLink}
              data-selected={isSelected ? "true" : undefined}
              aria-current={isSelected ? "page" : undefined}
            >
              <span className={styles.rowName}>
                <span className={styles.rowGlyph} aria-hidden="true">
                  {glyphKey ? (
                    <Glyph docType={glyphKey} size={24} framed />
                  ) : (
                    <span className={styles.rowGlyphFallback} />
                  )}
                </span>
                {t.name}.md
              </span>
              <TierPills tiers={t.tiers} />
              <ChevronRightIcon size={14} className={styles.rowChevron} />
            </Link>
          </li>
        );
      })}
    </ul>
  );
}

interface PageProps {
  selectedName?: string;
  extraContent?: ReactNode;
}

export function TemplatesPage({ selectedName, extraContent }: PageProps) {
  const { data, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.templates(),
    queryFn: fetchTemplates,
  });

  let listContent: ReactNode = <p>Loading…</p>;
  if (isError) {
    listContent = (
      <p role="alert" className={styles.error}>
        Failed to load templates:{" "}
        {error instanceof Error ? error.message : String(error)}
      </p>
    );
  } else if (!isLoading && data) {
    listContent = (
      <TemplatesIndexList
        templates={data.templates}
        selectedName={selectedName}
      />
    );
  }

  return (
    <Page
      eyebrow={<EyebrowLabel type="templates" />}
      title="Templates"
      subtitle="The starting shape for every new doc. Pick a template to see which version is active and what the other tiers look like."
    >
      {listContent}
      {extraContent}
    </Page>
  );
}

export function LibraryTemplatesIndex() {
  return <TemplatesPage />;
}
