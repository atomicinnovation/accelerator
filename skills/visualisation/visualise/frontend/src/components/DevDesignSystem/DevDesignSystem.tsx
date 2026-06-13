import { type ReactNode, useEffect, useRef, useState } from "react";
import { Icon } from "../Icon/Icon";
import styles from "./DevDesignSystem.module.css";
import { DEV_CHORD_HINT, DEV_SECTIONS } from "./dev-constants";
import { useDevActivationContext } from "./use-dev-activation";

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

  // Phase 4: a TOC click scrolls and sets the active highlight. Phase 5 adds the
  // IntersectionObserver scroll-spy + the canonical `#<section>` hash write.
  const jump = (id: string) => {
    setActiveSection(id);
    document
      .getElementById(`ds-${id}`)
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
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
          {DEV_SECTIONS.map((s) => (
            <DSSection key={s.id} id={s.id} title={s.label} />
          ))}
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
