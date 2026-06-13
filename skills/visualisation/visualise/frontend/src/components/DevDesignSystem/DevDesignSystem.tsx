import { type ReactNode, useEffect, useRef, useState } from "react";
import { Icon } from "../Icon/Icon";
import { ThemeToggle } from "../ThemeToggle/ThemeToggle";
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
