import {
  type CSSProperties,
  type ReactNode,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  type Completeness,
  DOC_TYPE_KEYS,
  DOC_TYPE_LABELS,
  WORKFLOW_PIPELINE_STEPS,
} from "../../api/types";
import { useThemeContext } from "../../api/use-theme";
import {
  DOC_TYPE_HUE,
  RADIUS_TOKENS,
  SPACING_TOKENS,
} from "../../styles/tokens";
import { AtomicMark } from "../AtomicMark/AtomicMark";
import { BigGlyph } from "../BigGlyph/BigGlyph";
import { Chip, type ChipSize, type ChipVariant } from "../Chip/Chip";
import { Glyph } from "../Glyph/Glyph";
import { ICON_NAMES, Icon } from "../Icon/Icon";
import { PipelineMini } from "../PipelineMini/PipelineMini";
import { ResultBadge } from "../ResultBadge/ResultBadge";
import { StatusBadge } from "../StatusBadge/StatusBadge";
import { ThemeToggle } from "../ThemeToggle/ThemeToggle";
import { VerdictBadge } from "../VerdictBadge/VerdictBadge";
import styles from "./DevDesignSystem.module.css";
import { DEV_CHORD_HINT, DEV_SECTIONS } from "./dev-constants";
import { pickActiveSection } from "./pick-active-section";
import { useDevActivationContext } from "./use-dev-activation";

// The scroll-spy binds to the explicit scroll root RootLayout marks on its
// <main> (not `closest("main")`, which would silently resolve to a nested
// <main> if one were ever introduced).
const SCROLL_ROOT_SELECTOR = "[data-scroll-root]";
// Active-band top offset — matches the observer's rootMargin top so the pure
// picker and the observer agree on where the band begins.
const BAND_TOP = 80;
const SCROLL_SPY_MARGIN = "-80px 0px -55% 0px";
const SCROLL_SPY_THRESHOLD = [0, 0.25, 0.5];

// Scroll a section to the active-band top within the scroll root. Robust to the
// offsetParent chain (the live .main declares no `position`, so the prototype's
// `el.offsetTop` would be unreliable). A no-op when the scroll root or section
// is absent (e.g. unit tests that render the page without RootLayout).
function scrollToSection(id: string, behavior: ScrollBehavior) {
  const main = document.querySelector<HTMLElement>(SCROLL_ROOT_SELECTOR);
  const el = document.getElementById(`ds-${id}`);
  if (!main || !el) return;
  const target =
    el.getBoundingClientRect().top -
    main.getBoundingClientRect().top +
    main.scrollTop -
    BAND_TOP;
  main.scrollTo({ top: target, behavior });
}

/** Section wrapper — `§ <id>` eyebrow + title + optional hint, then the body. */
function DSSection({
  id,
  title,
  hint,
  children,
}: {
  id: string;
  title: string;
  hint?: string;
  children?: ReactNode;
}) {
  return (
    <section id={`ds-${id}`} className={styles.section}>
      <header className={styles.sectionHead}>
        <div className={styles.sectionHeadL}>
          <span className={styles.sectionId}>§ {id}</span>
          <h2 className={styles.sectionTitle}>{title}</h2>
        </div>
        {hint ? <div className={styles.sectionHint}>{hint}</div> : null}
      </header>
      <div className={styles.sectionBody}>{children}</div>
    </section>
  );
}

/** One logical run of the marquee content. Rendered twice inside the animated
 *  inner block so the `-50%` translate loops seamlessly. */
function MarqueeRun() {
  return (
    <>
      <span className={styles.marqueeTag}>DEV</span>
      <span className={styles.marqueeSep}>·</span>
      <span className={styles.marqueeTitle}>Design system reference</span>
      <span className={styles.marqueeSep}>·</span>
      <span className={styles.marqueeRoute}>/dev</span>
      <span className={styles.marqueeSep}>·</span>
      <span className={styles.marqueeKbd}>{DEV_CHORD_HINT} toggles</span>
      <span className={styles.marqueeSep}>·</span>
      <span>Not exposed in the sidebar — share the link with the team</span>
      <span className={styles.marqueeSep}>·</span>
    </>
  );
}

// ─── Section content ────────────────────────────────────────────────────────
// Per-section bodies live in this one file by design (only the scroll-spy
// active-section pick is extracted as a pure helper). Demonstration values —
// type sizes, spacing-bar widths, radii — are rendered as inline `var(--token)`
// styles so the page reads the LIVE tokens directly (and so the ADR-0039 module
// CSS gate, which scans only *.module.css, stays clean); the module CSS holds
// only structural, fully-tokenised layout.

/** A reference cell that reads + shows its token's resolved colour at runtime. */
function Swatch({ token, label }: { token: string; label: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [resolved, setResolved] = useState("");
  useEffect(() => {
    if (!ref.current) return;
    // Resolves the var() to its computed rgb in the browser; jsdom leaves it
    // empty (it does not resolve custom properties), so the hex line is simply
    // omitted under unit test — the light↔dark oracle is a Playwright check.
    setResolved(getComputedStyle(ref.current).backgroundColor);
  }, []);
  return (
    <div className={styles.swatch} data-token={token}>
      <div
        ref={ref}
        className={styles.swatchChip}
        style={{ background: `var(${token})` }}
      />
      <div className={styles.swatchMeta}>
        <div className={styles.swatchName}>{label}</div>
        <div className={styles.swatchToken}>{token}</div>
        {resolved ? <div className={styles.swatchHex}>{resolved}</div> : null}
      </div>
    </div>
  );
}

function SwatchGroup({
  heading,
  testid,
  swatches,
}: {
  heading: string;
  testid: string;
  swatches: ReadonlyArray<readonly [token: string, label: string]>;
}) {
  return (
    <>
      <h3 className={styles.h3}>{heading}</h3>
      <div className={styles.swatches} data-testid={testid}>
        {swatches.map(([token, label]) => (
          <Swatch key={token} token={token} label={label} />
        ))}
      </div>
    </>
  );
}

// Curated semantic-colour groups (live --ac-* tokens). The counts are the
// section oracle; the labels mirror the prototype.
const SURFACE_SWATCHES = [
  ["--ac-bg", "Page"],
  ["--ac-bg-raised", "Raised"],
  ["--ac-bg-sunken", "Sunken"],
  ["--ac-bg-chrome", "Chrome"],
  ["--ac-bg-sidebar", "Sidebar"],
  ["--ac-bg-card", "Card"],
  ["--ac-bg-hover", "Hover"],
  ["--ac-bg-active", "Active"],
] as const;
const FOREGROUND_SWATCHES = [
  ["--ac-fg", "Body"],
  ["--ac-fg-strong", "Strong"],
  ["--ac-fg-muted", "Muted"],
  ["--ac-fg-faint", "Faint"],
] as const;
const ACCENT_SWATCHES = [
  ["--ac-accent", "Accent (indigo)"],
  ["--ac-accent-2", "Accent 2 (red)"],
  ["--ac-accent-tint", "Accent tint"],
  ["--ac-accent-faint", "Accent faint"],
  ["--ac-ok", "OK"],
  ["--ac-warn", "Warn"],
  ["--ac-err", "Error"],
  ["--ac-violet", "Violet"],
] as const;
const STROKE_SWATCHES = [
  ["--ac-stroke", "Default"],
  ["--ac-stroke-soft", "Soft"],
  ["--ac-stroke-strong", "Strong"],
] as const;
// The 19 named brand colours the prototype showcased — the curated identity
// set (the live --atomic-* palette carries 37 incl. aliases + neutrals).
const BRAND_SWATCHES = [
  ["--atomic-night", "Night"],
  ["--atomic-night-2", "Night 2"],
  ["--atomic-night-3", "Night 3"],
  ["--atomic-ink", "Ink"],
  ["--atomic-red", "Atomic red"],
  ["--atomic-red-2", "Red hover"],
  ["--atomic-indigo", "Indigo"],
  ["--atomic-indigo-tint", "Indigo tint"],
  ["--atomic-medium-purple", "Medium purple"],
  ["--atomic-cream-can", "Cream can"],
  ["--atomic-steel-blue", "Steel blue"],
  ["--atomic-pastel-green", "Pastel green"],
  ["--atomic-aquamarine", "Aquamarine"],
  ["--atomic-tradewind", "Tradewind"],
  ["--atomic-malibu", "Malibu"],
  ["--atomic-marigold", "Marigold"],
  ["--atomic-bone", "Bone"],
  ["--atomic-ash", "Ash"],
  ["--atomic-slate", "Slate"],
] as const;

// The seven type ramps (hero → eyebrow), expressed against the live --size-*
// and --ac-font-* tokens so the specimens ARE the design system, not a copy.
const TYPE_SAMPLES: ReadonlyArray<{
  text: string;
  meta: string;
  style: CSSProperties;
}> = [
  {
    text: "Sora · display hero",
    meta: "var(--ac-font-display) · var(--size-hero) · 600",
    style: {
      fontFamily: "var(--ac-font-display)",
      fontSize: "var(--size-hero)",
      fontWeight: 600,
      lineHeight: "var(--lh-tight)",
      color: "var(--ac-fg-strong)",
    },
  },
  {
    text: "Sora 28 · page title",
    meta: "var(--ac-font-display) · var(--size-h3) · 600",
    style: {
      fontFamily: "var(--ac-font-display)",
      fontSize: "var(--size-h3)",
      fontWeight: 600,
      lineHeight: "var(--lh-snug)",
      color: "var(--ac-fg-strong)",
    },
  },
  {
    text: "Sora 18 · section heading",
    meta: "var(--ac-font-display) · var(--size-md) · 600",
    style: {
      fontFamily: "var(--ac-font-display)",
      fontSize: "var(--size-md)",
      fontWeight: 600,
      color: "var(--ac-fg-strong)",
    },
  },
  {
    text: "Inter · body copy. The visualiser uses Inter for everything that isn't a heading or code, with mono metadata interleaved at smaller sizes.",
    meta: "var(--ac-font-body) · var(--size-prose) · 400",
    style: {
      fontFamily: "var(--ac-font-body)",
      fontSize: "var(--size-prose)",
      fontWeight: 400,
      lineHeight: "var(--lh-prose)",
      color: "var(--ac-fg)",
    },
  },
  {
    text: "Inter 13 · UI label",
    meta: "var(--ac-font-body) · var(--size-subtitle) · 500",
    style: {
      fontFamily: "var(--ac-font-body)",
      fontSize: "var(--size-subtitle)",
      fontWeight: 500,
      color: "var(--ac-fg-strong)",
    },
  },
  {
    text: "Fira Code 12 · PR-0042 · 14d ago · /meta/work/0001.md",
    meta: "var(--ac-font-mono) · var(--size-xxs) · metadata",
    style: {
      fontFamily: "var(--ac-font-mono)",
      fontSize: "var(--size-xxs)",
      color: "var(--ac-fg-muted)",
    },
  },
  {
    text: "FIRA CODE 11 · EYEBROW LABEL",
    meta: "var(--ac-font-mono) · var(--size-eyebrow) · var(--tracking-caps)",
    style: {
      fontFamily: "var(--ac-font-mono)",
      fontSize: "var(--size-eyebrow)",
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--ac-fg-faint)",
    },
  },
];

// The "brand" shadow is --shadow-card; there is deliberately no --shadow-brand.
const SHADOW_SPECS = [
  ["--ac-shadow-soft", "soft"],
  ["--ac-shadow-lift", "lift"],
  ["--shadow-card", "brand (--shadow-card)"],
] as const;

function OverviewSection() {
  const { theme, toggleTheme } = useThemeContext();
  const other = theme === "dark" ? "light" : "dark";
  return (
    <>
      <p className={styles.prose}>
        A private inventory of every visual primitive the Accelerator visualiser
        renders — so engineers can spot token drift, a missing variant, or a
        component they didn't know already existed.
      </p>
      <p className={styles.prose}>
        Tokens are drawn from <code>global.css</code> (the layered{" "}
        <code>--ac-*</code> semantic set over the <code>--atomic-*</code> brand
        palette); components live under <code>src/components</code> and the
        per-route files.
      </p>
      <div className={styles.overviewGrid}>
        <div className={styles.overviewCard} data-testid="overview-card-fonts">
          <div className={styles.overviewCardNum}>3</div>
          <div className={styles.overviewCardLbl}>font families</div>
          <div className={styles.overviewCardSub}>Sora · Inter · Fira Code</div>
        </div>
        <div className={styles.overviewCard} data-testid="overview-card-icons">
          <div className={styles.overviewCardNum}>{ICON_NAMES.length}</div>
          <div className={styles.overviewCardLbl}>stroke icons</div>
          <div className={styles.overviewCardSub}>
            Feather-style · 2px stroke · currentColor
          </div>
        </div>
        <div className={styles.overviewCard} data-testid="overview-card-glyphs">
          <div className={styles.overviewCardNum}>{DOC_TYPE_KEYS.length}</div>
          <div className={styles.overviewCardLbl}>doc-type glyphs</div>
          <div className={styles.overviewCardSub}>
            Hue-tinted square + line drawing per type
          </div>
        </div>
        <div className={styles.overviewCard} data-testid="overview-card-themes">
          <div className={styles.overviewCardNum}>2</div>
          <div className={styles.overviewCardLbl}>themes</div>
          <div className={styles.overviewCardSub}>
            <button type="button" className={styles.link} onClick={toggleTheme}>
              flip to {other} →
            </button>
          </div>
        </div>
      </div>

      <aside className={styles.deviations} data-testid="ds-deviations">
        <div className={styles.deviationsTitle}>
          Deviations from the prototype
        </div>
        <ul className={styles.deviationsList}>
          <li>
            Canonical deep-link is <code>{"/dev#<section>"}</code>;{" "}
            <code>#dev</code> and <code>{"#dev/<section>"}</code> are accepted
            aliases the bridge normalises.
          </li>
          <li>
            Doc-type glyphs use the live <code>16/24/32</code> sizes plus a
            net-new <code>48</code> (not the prototype's 22/28/36/48).
          </li>
          <li>
            Radii are a px-ladder (<code>--radius-0…12</code>,{" "}
            <code>--radius-pill</code>), not <code>sm/md/lg</code>.
          </li>
          <li>
            There is no <code>--shadow-brand</code> token — the "brand" shadow
            is <code>--shadow-card</code>.
          </li>
          <li>
            Status, verdict and result are three separate live components, not
            one.
          </li>
          <li>
            <code>PipelineMini</code> has no compact mode (no live equivalent).
          </li>
          <li>
            Code blocks omit the traffic-light dots (the live chrome does too).
          </li>
          <li>
            Some composites (breadcrumbs, the library table, toasts) are
            hand-authored — the live components can't render standalone here.
          </li>
        </ul>
      </aside>
    </>
  );
}

function ColoursSection() {
  return (
    <>
      <SwatchGroup
        heading="Surfaces"
        testid="ds-swatches-surfaces"
        swatches={SURFACE_SWATCHES}
      />
      <SwatchGroup
        heading="Foreground"
        testid="ds-swatches-foreground"
        swatches={FOREGROUND_SWATCHES}
      />
      <SwatchGroup
        heading="Accent & status"
        testid="ds-swatches-accent"
        swatches={ACCENT_SWATCHES}
      />
      <SwatchGroup
        heading="Strokes"
        testid="ds-swatches-stroke"
        swatches={STROKE_SWATCHES}
      />
      <h3 className={styles.h3}>Brand palette</h3>
      <div className={styles.swatches} data-testid="ds-swatches-brand">
        {BRAND_SWATCHES.map(([token, label]) => (
          <Swatch key={token} token={token} label={label} />
        ))}
      </div>
      <h3 className={styles.h3}>Doc-type hues</h3>
      <div className={styles.typeHues} data-testid="ds-typehues">
        {DOC_TYPE_KEYS.map((key) => (
          <div key={key} className={styles.typeHue} data-doc-type={key}>
            <div
              className={styles.typeHueChip}
              style={{ background: `hsl(${DOC_TYPE_HUE[key]} 68% 56%)` }}
            />
            <div>
              <div className={styles.typeHueName}>{DOC_TYPE_LABELS[key]}</div>
              <div className={styles.caption}>hue {DOC_TYPE_HUE[key]}</div>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}

function TypeSection() {
  return (
    <div className={styles.typeSamples} data-testid="ds-typesamples">
      {TYPE_SAMPLES.map((sample) => (
        <div key={sample.text} className={styles.typeSample} data-type-sample>
          <div style={sample.style}>{sample.text}</div>
          <div className={styles.typeSampleMeta}>{sample.meta}</div>
        </div>
      ))}
    </div>
  );
}

function SpacingSection() {
  return (
    <div className={styles.spacingRows} data-testid="ds-spacing">
      {Object.entries(SPACING_TOKENS).map(([name, value]) => (
        <div key={name} className={styles.spacingCell} data-sp={name}>
          <div
            className={styles.spacingBar}
            style={{ width: `var(--${name})` }}
          />
          <div className={styles.spacingLbl}>--{name}</div>
          <div className={styles.spacingVal}>{value}</div>
        </div>
      ))}
    </div>
  );
}

function RadiiSection() {
  return (
    <>
      <h3 className={styles.h3}>Corner radii</h3>
      <div className={styles.radii} data-testid="ds-radii">
        {Object.entries(RADIUS_TOKENS).map(([name, value]) => (
          <div key={name} className={styles.radiiCell} data-radius={name}>
            <div
              className={styles.radiiBox}
              style={{ borderRadius: `var(--${name})` }}
            />
            <div className={styles.radiiLbl}>--{name}</div>
            <div className={styles.radiiVal}>{value}</div>
          </div>
        ))}
      </div>
      <h3 className={styles.h3}>Shadows</h3>
      <div className={styles.shadows} data-testid="ds-shadows">
        {SHADOW_SPECS.map(([token, label]) => (
          <div
            key={token}
            className={styles.shadow}
            style={{ boxShadow: `var(${token})` }}
            data-shadow={token}
          >
            <span>{label}</span>
          </div>
        ))}
      </div>
    </>
  );
}

// Icon size ramp (prototype's 12→32 set); the `hex` icon is the specimen.
const ICON_SIZE_RAMP = [12, 14, 16, 18, 20, 24, 28, 32] as const;
// Doc-type glyph sizes — the live 16/24/32 plus the net-new 48 (the glyph VR
// spec's contract; Phase 10 repoints it here, adding the 48 cell).
const GLYPH_SIZES = [16, 24, 32, 48] as const;
// Empty-state (BigGlyph) size ramp.
const BIG_GLYPH_SIZES = [48, 64, 80, 96, 128] as const;
// Atomic-mark sizes.
const MARK_SIZES = [20, 24, 32, 48, 72] as const;

function IconsSection() {
  return (
    <>
      <div className={styles.iconsGrid} data-testid="ds-icons">
        {ICON_NAMES.map((name) => (
          <div
            key={name}
            className={styles.iconCell}
            data-icon={name}
            title={name}
          >
            <div className={styles.iconChip}>
              <Icon name={name} size={20} />
            </div>
            <div className={styles.iconName}>{name}</div>
          </div>
        ))}
      </div>
      <h3 className={styles.h3}>Sizes</h3>
      <div className={styles.iconSizes}>
        {ICON_SIZE_RAMP.map((sz) => (
          <div key={sz} className={styles.iconSizeCell}>
            <Icon name="hex" size={sz} />
            <div className={styles.caption}>{sz}px</div>
          </div>
        ))}
      </div>
    </>
  );
}

function GlyphsSection() {
  // `glyph-cell-<docType>-<size>` reproduces the GlyphShowcase locator contract
  // (tests/visual-regression/glyph-showcase.spec.ts + glyph-resolved-fill.spec.ts,
  // migrated here in Phase 10) at the live 16/24/32 sizes + the net-new 48.
  return (
    <div className={styles.glyphGrid} data-testid="ds-glyphs">
      {DOC_TYPE_KEYS.map((docType) => (
        <div key={docType} className={styles.glyphRow}>
          <div className={styles.glyphRowLabel}>
            <code>{docType}</code>
            <span className={styles.caption}>{DOC_TYPE_LABELS[docType]}</span>
          </div>
          <div className={styles.glyphRowCells}>
            {GLYPH_SIZES.map((size) => (
              <span
                key={size}
                className={styles.glyphCell}
                data-testid={`glyph-cell-${docType}-${size}`}
              >
                <Glyph docType={docType} size={size} />
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function BigGlyphsSection() {
  // `big-glyph-cell-<docType>` reproduces the BigGlyphShowcase locator contract
  // (tests/visual-regression/big-glyph-showcase.spec.ts, migrated here in Phase
  // 10). The cell keeps `background: var(--ac-bg-card)` because that spec's
  // dark-commit poll reads the cell's computed background-color; the hue halo
  // lives on the inner hero so the poll still sees the card surface.
  return (
    <>
      <p className={styles.prose}>
        Hero illustrations for per-type empty pages — each hue-tinted from{" "}
        <code>DOC_TYPE_HUE</code> and rendered on an 80×80 viewBox so it scales
        cleanly between 64–128px. Used in the library empty states.
      </p>
      <div className={styles.bigGlyphGrid} data-testid="ds-bigglyphs">
        {DOC_TYPE_KEYS.map((docType) => (
          <div
            key={docType}
            className={styles.bigGlyphCell}
            data-testid={`big-glyph-cell-${docType}`}
          >
            <div
              className={styles.bigGlyphHero}
              style={{ "--bg-hue": DOC_TYPE_HUE[docType] } as CSSProperties}
            >
              <BigGlyph docType={docType} size={96} />
            </div>
            <div className={styles.bigGlyphMeta}>
              <div className={styles.bigGlyphName}>
                {DOC_TYPE_LABELS[docType]}
              </div>
              <div className={styles.caption}>{docType}</div>
            </div>
          </div>
        ))}
      </div>
      <h3 className={styles.h3}>Sizes</h3>
      <div className={styles.bigGlyphSizes}>
        {BIG_GLYPH_SIZES.map((sz) => (
          <div
            key={sz}
            className={styles.bigGlyphSizeCell}
            data-big-glyph-size={sz}
          >
            <BigGlyph docType="plans" size={sz} />
            <div className={styles.caption}>{sz}px</div>
          </div>
        ))}
      </div>
    </>
  );
}

function MarkSection() {
  return (
    <div className={styles.marks}>
      {MARK_SIZES.map((sz) => (
        <div key={sz} className={styles.markCell} data-mark>
          <AtomicMark size={sz} />
          <div className={styles.caption}>{sz}</div>
        </div>
      ))}
      <div
        className={styles.markCell}
        data-mark
        data-mark-night
        style={{ background: "var(--atomic-night)" }}
      >
        <AtomicMark size={48} />
        <div
          className={styles.caption}
          style={{ color: "rgba(255,255,255,0.7)" }}
        >
          on night
        </div>
      </div>
    </div>
  );
}

const CHIP_VARIANTS: ReadonlyArray<ChipVariant> = [
  "neutral",
  "indigo",
  "green",
  "amber",
  "red",
  "violet",
];
const CHIP_SIZES: ReadonlyArray<ChipSize> = ["sm", "md"];
// The 8 statuses the badges section showcases (the live `statusToVariant` colours
// done/accepted green, in-progress/proposed indigo, the rest neutral).
const STATUS_VALUES = [
  "todo",
  "in-progress",
  "done",
  "draft",
  "accepted",
  "proposed",
  "open",
  "merged",
] as const;

function ChipsSection() {
  // `chip-cell-<variant>-<size>` reproduces the ChipShowcase locator contract
  // (tests/visual-regression/chip-showcase.spec.ts + chip-resolved-colours.spec.ts,
  // migrated here in Phase 10). Each cell wraps a live <Chip>.
  return (
    <div className={styles.chipGrid} data-testid="ds-chips">
      {CHIP_VARIANTS.map((variant) => (
        <div key={variant} className={styles.chipRow}>
          <span className={styles.chipRowLabel}>
            <code>{variant}</code>
          </span>
          {CHIP_SIZES.map((size) => (
            <span
              key={size}
              className={styles.chipCell}
              data-testid={`chip-cell-${variant}-${size}`}
            >
              <Chip variant={variant} size={size}>
                {variant}
              </Chip>
            </span>
          ))}
        </div>
      ))}
    </div>
  );
}

function BadgesSection() {
  // The three-component status/verdict/result split: approve/request-changes go
  // through VerdictBadge, approve-with-changes through StatusBadge (amber), pass
  // through ResultBadge. Each renders a <Chip> carrying its own data-testid.
  return (
    <>
      <div className={styles.row} data-testid="ds-badges-status">
        {STATUS_VALUES.map((value) => (
          <StatusBadge key={value} value={value} />
        ))}
      </div>
      <h3 className={styles.h3}>Verdicts &amp; results</h3>
      <div className={styles.row} data-testid="ds-badges-verdict">
        <VerdictBadge value="approve" />
        <StatusBadge value="approve-with-changes" />
        <VerdictBadge value="request-changes" />
        <ResultBadge value="pass" />
      </div>
    </>
  );
}

// PipelineMini reads only `completeness.present` (kebab DocTypeKey strings); the
// has* flags are required by the type but ignored at render, so derive them.
function completenessFromPresent(present: string[]): Completeness {
  const has = (k: string) => present.includes(k);
  return {
    hasWorkItem: has("work-items"),
    hasResearch: has("research"),
    hasPlan: has("plans"),
    hasPlanReview: has("plan-reviews"),
    hasValidation: has("validations"),
    hasPrDescription: has("pr-descriptions"),
    hasPrReview: has("pr-reviews"),
    hasDecision: has("decisions"),
    hasNotes: has("notes"),
    hasDesignInventory: has("design-inventories"),
    hasDesignGap: has("design-gaps"),
    present,
  };
}

function StageDotsSection() {
  const allPresent = WORKFLOW_PIPELINE_STEPS.map((s) => s.docType);
  return (
    <div data-testid="ds-stagedots">
      <div className={styles.row}>
        <PipelineMini completeness={completenessFromPresent(allPresent)} />
        <span className={styles.caption}>all present</span>
      </div>
      <div className={styles.row}>
        <PipelineMini
          completeness={completenessFromPresent([
            "work-items",
            "research",
            "plans",
          ])}
        />
        <span className={styles.caption}>partial</span>
      </div>
      <div className={styles.row}>
        <PipelineMini completeness={completenessFromPresent([])} />
        <span className={styles.caption}>none</span>
      </div>
    </div>
  );
}

function TierPillsSection() {
  // Hand-authored: the live `.tierPill` has only three states and is coupled to
  // the templates table, so the four presence states are reproduced here.
  return (
    <div className={styles.row} data-testid="ds-tierpills">
      <span
        className={`${styles.tierPill} ${styles.tierPillPresent}`}
        data-tier="present"
      >
        <span className={styles.tierDot} />
        present
      </span>
      <span
        className={`${styles.tierPill} ${styles.tierPillActive}`}
        data-tier="active"
      >
        <span className={styles.tierDot} />
        active
      </span>
      <span
        className={`${styles.tierPill} ${styles.tierPillAbsent}`}
        data-tier="absent"
      >
        <span className={styles.tierDot} />
        absent
      </span>
      <span className={styles.tierPill} data-tier="default">
        <span className={styles.tierDot} />
        default
      </span>
    </div>
  );
}

function ButtonsSection() {
  return (
    <div className={styles.row} data-testid="ds-buttons">
      <button type="button" className={styles.btn} data-btn>
        <Icon name="moon" size={14} />
      </button>
      <button
        type="button"
        className={`${styles.btn} ${styles.btnBordered}`}
        data-btn
      >
        <Icon name="filter" size={14} /> Filter
      </button>
      <button
        type="button"
        className={`${styles.btn} ${styles.btnBordered}`}
        data-btn
      >
        <Icon name="sort" size={14} /> Sort
      </button>
      <button
        type="button"
        className={`${styles.btn} ${styles.btnBordered} ${styles.btnActive}`}
        data-btn
      >
        <Icon name="sort" size={14} /> updated ↓
      </button>
      <button
        type="button"
        className={`${styles.btn} ${styles.btnBordered}`}
        data-btn
      >
        <Icon name="filter" size={14} /> Filter{" "}
        <span className={styles.filterBadge}>3</span>
      </button>
      <a href="#buttons" className={styles.inlineLink} data-btn>
        standard inline link
      </a>
      <button type="button" className={styles.link} data-btn>
        ds-link button
      </button>
    </div>
  );
}

function FormSection() {
  return (
    <>
      <h3 className={styles.h3}>Search input</h3>
      <div className={styles.searchWrap}>
        <div className={styles.searchInput} data-testid="ds-form-search">
          <Icon name="search" size={14} className={styles.searchIcon} />
          <input type="text" placeholder="Search docs…" />
          <kbd className={styles.searchKbd}>⌘K</kbd>
        </div>
      </div>
      <h3 className={styles.h3}>Checkboxes</h3>
      <div className={styles.checkRows}>
        <label className={styles.checkRow} data-check>
          <input type="checkbox" defaultChecked />
          <span>checked</span>
          <span className={styles.checkCount}>7</span>
        </label>
        <label className={styles.checkRow} data-check>
          <input type="checkbox" />
          <span>unchecked</span>
          <span className={styles.checkCount}>12</span>
        </label>
      </div>
    </>
  );
}

function NavSection() {
  return (
    <div className={styles.navDemo} data-testid="ds-nav">
      <div className={styles.navLabel} data-nav="label">
        GROUP LABEL
      </div>
      <div className={styles.navSublabel} data-nav="sublabel">
        Sub-group
      </div>
      <div className={styles.navItem} data-nav="default">
        <span className={styles.navItemL}>
          <Icon name="doc" size={14} /> default item
        </span>
        <span className={styles.navItemR}>
          <span className={styles.navCount}>12</span>
        </span>
      </div>
      <div
        className={`${styles.navItem} ${styles.navItemActive}`}
        data-nav="active"
      >
        <span className={styles.navItemL}>
          <Icon name="doc" size={14} /> active item
        </span>
        <span className={styles.navItemR}>
          <span className={styles.navCount}>8</span>
        </span>
      </div>
      <div className={styles.navItem} data-nav="pulse">
        <span className={styles.navItemL}>
          <Icon name="doc" size={14} /> with pulse
        </span>
        <span className={styles.navItemR}>
          <span className={styles.navPulse} />
          <span className={styles.navCount}>3</span>
        </span>
      </div>
      <div
        className={`${styles.navItem} ${styles.navItemMeta}`}
        data-nav="faded"
      >
        <span className={styles.navItemL}>
          <Icon name="doc" size={14} /> meta (faded)
        </span>
        <span className={styles.navItemR}>
          <span className={styles.navCount}>5</span>
        </span>
      </div>
    </div>
  );
}

// Per-section content + hint, keyed by the DEV_SECTIONS slug. Sections without
// an entry render as empty stubs (filled by later phases).
const SECTION_CONTENT: Record<string, { hint?: string; body: ReactNode }> = {
  overview: { body: <OverviewSection /> },
  colors: {
    hint: "from global.css · respond to data-theme",
    body: <ColoursSection />,
  },
  type: { hint: "Sora · Inter · Fira Code", body: <TypeSection /> },
  spacing: { hint: "--sp-1 … --sp-11", body: <SpacingSection /> },
  radii: { hint: "--radius-* ladder · 3 shadows", body: <RadiiSection /> },
  icons: {
    hint: `${ICON_NAMES.length} stroke icons · <Icon name size />`,
    body: <IconsSection />,
  },
  glyphs: {
    hint: `${DOC_TYPE_KEYS.length} types · <Glyph docType size />`,
    body: <GlyphsSection />,
  },
  bigglyphs: { hint: "<BigGlyph docType size />", body: <BigGlyphsSection /> },
  mark: { body: <MarkSection /> },
  chips: { hint: '<Chip variant size="sm|md">', body: <ChipsSection /> },
  badges: { hint: "status · verdict · result", body: <BadgesSection /> },
  stagedots: {
    hint: `${WORKFLOW_PIPELINE_STEPS.length}-stage pipeline presence`,
    body: <StageDotsSection />,
  },
  tierpills: { hint: "presence states", body: <TierPillsSection /> },
  buttons: { body: <ButtonsSection /> },
  form: { body: <FormSection /> },
  nav: { body: <NavSection /> },
};

export function DevDesignSystem() {
  const dev = useDevActivationContext();
  const headingRef = useRef<HTMLHeadingElement>(null);
  const [activeSection, setActiveSection] = useState<string>(() => {
    const fromHash = window.location.hash.replace(/^#/, "");
    return fromHash || "overview";
  });

  // Move keyboard focus into the page on activation so keyboard-only users do
  // not stay where the chord was pressed. Exit restores focus to the app anchor
  // (see use-dev-activation `exitDev`).
  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  // Deep-link landing: scroll the requested section into the active region on
  // cold load. `/dev#overview` and bare `/dev` stay at the top.
  useEffect(() => {
    const section = window.location.hash.replace(/^#/, "");
    if (!section || section === "overview") return;
    scrollToSection(section, "auto");
  }, []);

  // Scroll-spy: drive the active highlight + canonical hash from actual scroll
  // position. Recompute ALL sections' live tops on each observer dispatch and
  // pick via the pure total-order helper (never the prototype's pinned-to-
  // Colours single-dispatch highest-ratio). `replaceState` does not fire
  // `hashchange`, so the hash write never re-enters the activation bridge.
  useEffect(() => {
    const main = document.querySelector<HTMLElement>(SCROLL_ROOT_SELECTOR);
    if (!main) return;
    const ids = DEV_SECTIONS.map((s) => s.id);
    let lastWritten: string | null = null;

    const recompute = () => {
      const mainTop = main.getBoundingClientRect().top;
      const tops = ids.map((id) => {
        const el = document.getElementById(`ds-${id}`);
        return {
          id,
          top: el
            ? el.getBoundingClientRect().top - mainTop
            : Number.POSITIVE_INFINITY,
        };
      });
      const active = pickActiveSection(tops, BAND_TOP);
      if (!active) return;
      setActiveSection(active);
      if (active !== lastWritten) {
        lastWritten = active;
        const base = main.ownerDocument.location.pathname;
        // Overview clears the hash (canonical bare /dev — never #overview);
        // every other section writes the bare #<section> hash.
        const url = active === "overview" ? base : `${base}#${active}`;
        window.history.replaceState(null, "", url);
        dev?.recordProgrammaticHash(active === "overview" ? "" : `#${active}`);
      }
    };

    const observer = new IntersectionObserver(recompute, {
      root: main,
      rootMargin: SCROLL_SPY_MARGIN,
      threshold: SCROLL_SPY_THRESHOLD,
    });
    for (const id of ids) {
      const el = document.getElementById(`ds-${id}`);
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  }, [dev]);

  const jump = (id: string) => {
    setActiveSection(id);
    scrollToSection(id, "smooth");
  };

  return (
    <div className={styles.root}>
      <div className={styles.marquee}>
        <div className={styles.marqueeInner}>
          <MarqueeRun />
          <MarqueeRun />
        </div>
      </div>

      <div className={styles.layout}>
        <aside className={styles.tocAside}>
          <div className={styles.tocHead}>
            <div className={styles.tocEyebrow}>CONTENTS</div>
            <div className={styles.tocActions}>
              <ThemeToggle />
              <button
                type="button"
                className={styles.tocExit}
                onClick={() => dev?.exitDev()}
              >
                <Icon
                  name="arrow-right"
                  size={12}
                  className={styles.tocExitIcon}
                />{" "}
                exit to app
              </button>
            </div>
          </div>
          <nav className={styles.toc} aria-label="Design system sections">
            {DEV_SECTIONS.map((s, i) => {
              const active = activeSection === s.id;
              return (
                <a
                  key={s.id}
                  href={`#${s.id}`}
                  className={`${styles.tocItem} ${active ? styles.tocItemActive : ""}`}
                  // "location" (not the codebase's "page"): the scroll-spy marks
                  // the current position WITHIN one page, not the current page in
                  // a nav set. Do not normalise this back to "page".
                  aria-current={active ? "location" : undefined}
                  title={`${s.label} — #${s.id}`}
                  onClick={(e) => {
                    e.preventDefault();
                    jump(s.id);
                  }}
                >
                  <span className={styles.tocNum}>
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <span>{s.label}</span>
                </a>
              );
            })}
          </nav>
          <div className={styles.tocFoot}>
            <div>accelerator-visualiser</div>
            <div className={styles.tocFootFaint}>design-system reference</div>
          </div>
        </aside>

        <div className={styles.content}>
          {/* Visually-hidden page heading — the document's h1 and the activation
              focus target; the marquee is the visual identity. */}
          <h1
            ref={headingRef}
            tabIndex={-1}
            data-dev-focus-anchor
            className="srOnly"
          >
            Design system reference
          </h1>
          {DEV_SECTIONS.map((s) => {
            const content = SECTION_CONTENT[s.id];
            return (
              <DSSection
                key={s.id}
                id={s.id}
                title={s.label}
                hint={content?.hint}
              >
                {content?.body}
              </DSSection>
            );
          })}
          <footer className={styles.footer}>
            <div className={styles.footerEnd}>— end of design system —</div>
            <div className={styles.footerHint}>
              press <kbd>{DEV_CHORD_HINT}</kbd> to leave · open via the{" "}
              <span className={styles.footerRoute}>#dev</span> hash
            </div>
          </footer>
        </div>
      </div>
    </div>
  );
}
