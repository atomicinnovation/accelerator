import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { fetchTemplateDetail } from "../../api/fetch";
import { queryKeys } from "../../api/query-keys";
import type { TemplateDetail, TemplateTier } from "../../api/types";
import { Chip } from "../../components/Chip/Chip";
import { TemplatesPage } from "./LibraryTemplatesIndex";
import styles from "./LibraryTemplatesView.module.css";
import { TemplateHighlight } from "./template-highlight";
import { TIER_LABELS } from "./template-tier";

interface Props {
  name?: string;
}

export function LibraryTemplatesView({ name: propName }: Props) {
  const params = useParams({ strict: false }) as { name?: string };
  const name = propName ?? params.name;

  if (!name) {
    return <p role="alert">Missing template name.</p>;
  }

  return (
    <TemplatesPage
      selectedName={name}
      extraContent={<TemplateDetailSection name={name} />}
    />
  );
}

function TemplateDetailSection({ name }: { name: string }) {
  const { data, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.templateDetail(name),
    queryFn: () => fetchTemplateDetail(name),
  });

  if (isError) {
    return (
      <section
        className={styles.detail}
        aria-labelledby="template-detail-heading"
      >
        <p role="alert" className={styles.error}>
          Failed to load template:{" "}
          {error instanceof Error ? error.message : String(error)}
        </p>
      </section>
    );
  }
  if (isLoading || !data) {
    return (
      <section
        className={styles.detail}
        aria-labelledby="template-detail-heading"
      >
        <p>Loading…</p>
      </section>
    );
  }

  return (
    <section
      className={styles.detail}
      aria-labelledby="template-detail-heading"
    >
      <h2 id="template-detail-heading" className={styles.detailHeading}>
        TIERS · {name.toLowerCase()}.md
      </h2>
      <div className={styles.twoColumn} data-testid="templates-detail-layout">
        <div className={styles.tiers}>
          {data.tiers.map((tier, index) => (
            <TierCard
              key={tier.source}
              tier={tier}
              tierIndex={index + 1}
              isActive={tier.source === data.activeTier}
            />
          ))}
        </div>
        <TemplatePreviewPane data={data} />
      </div>
    </section>
  );
}

/** Derive a parent-directory display string from a tier path, with a
 *  trailing `/`. Falls back to the path itself if it has no parent
 *  (a single-segment relative path is treated as living at the
 *  project root → "./"). */
function parentDirOf(path: string): string {
  const idx = path.lastIndexOf("/");
  if (idx < 0) return "./";
  return `${path.slice(0, idx)}/`;
}

/** Description text shown under the tier path. Mirrors the prototype's
 *  `tierDesc()` in view-templates.jsx — tier 1 is highest priority,
 *  tier 2 is "<override-dir> in this repo", tier 3 is the always-present
 *  plugin default. */
function tierDescription(tier: TemplateTier): string {
  switch (tier.source) {
    case "config-override":
      return tier.configSource
        ? `highest priority · ${tier.configSource}`
        : "highest priority";
    case "user-override":
      return `${parentDirOf(tier.path)} in this repo`;
    case "plugin-default":
      return "plugin-default · always present";
    default:
      return "";
  }
}

function TierCard({
  tier,
  tierIndex,
  isActive,
}: {
  tier: TemplateTier;
  tierIndex: number;
  isActive: boolean;
}) {
  return (
    <section
      className={`${styles.panel} ${!tier.present ? styles.absent : ""}`}
      data-active={isActive ? "true" : undefined}
    >
      <div className={styles.tierEyebrow}>TIER {tierIndex}</div>
      <header className={styles.panelHeader}>
        <span className={styles.tierLabel}>
          {TIER_LABELS[tier.source] ?? tier.source}
        </span>
        {isActive && <Chip variant="indigo">active</Chip>}
        {!tier.present && <Chip variant="neutral">absent</Chip>}
      </header>
      <code className={styles.tierPath}>{tier.path}</code>
      <span className={styles.tierNote}>{tierDescription(tier)}</span>
    </section>
  );
}

function findWinningTier(data: TemplateDetail): TemplateTier | undefined {
  return data.tiers.find((t) => t.source === data.activeTier && t.present);
}

/** Truncate a `sha256-<hex>` etag for compact display: keeps the
 *  `sha256-` prefix + 5 hex characters (total 12 chars) followed by an
 *  ellipsis. The untruncated value is surfaced via a `title` attribute. */
function truncateSha256(sha: string): string {
  if (sha.length <= 13) return sha;
  return `${sha.slice(0, 12)}…`;
}

function TemplatePreviewPane({ data }: { data: TemplateDetail }) {
  const winning = findWinningTier(data);

  if (!winning) {
    return (
      <div className={styles.previewPane} data-testid="template-preview-pane">
        <p className={styles.absentNote}>No winning tier resolved.</p>
      </div>
    );
  }

  return (
    <div className={styles.previewPane} data-testid="template-preview-pane">
      <div
        className={styles.previewHeader}
        data-testid="template-preview-header"
      >
        <span className={styles.previewPath}>{winning.path}</span>
        {data.sha256 ? (
          // biome-ignore lint/a11y/useAriaPropsSupportedByRole: this label is contractually non-interactive (AC13 — see the "content-hash label is non-interactive" test asserting role/tabindex are null), so it cannot take a role; the aria-label names the truncated digest for assistive tech and the title surfaces the full hash
          <span
            className={styles.contentHashLabel}
            aria-label="Content hash"
            title={data.sha256}
            data-full-sha={data.sha256}
          >
            {truncateSha256(data.sha256)}
          </span>
        ) : null}
      </div>
      <div className={styles.previewBody}>
        {winning.content != null ? (
          <TemplateHighlight content={winning.content} />
        ) : (
          <span className={styles.absentNote}>tier not present</span>
        )}
      </div>
    </div>
  );
}
