---
date: "2026-05-08T21:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-08-0034-theme-and-font-mode-toggles.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, compatibility]
review_pass: 3
status: complete
---

## Plan Review: 0034 Theme and Font-Mode Toggles

**Verdict:** REVISE

The plan is competently structured: it slices the work into TDD-friendly phases, correctly surfaces a latent inert-token bug from 0033 as Phase 1 prerequisite, and faithfully imitates the canonical `use-doc-events.ts` hook pattern. However, three cross-cutting concerns recurred across multiple lenses: the `suppressHydrationWarning` rationale rests on a likely-false premise (the app uses `createRoot`, not `hydrateRoot`, so React never reconciles `<html>`); the boot script's unconditional first-visit attribute write breaks the OS-follow-the-system path that the existing visual-regression suite already tests; and substantial logic duplication between the two hook files plus a parameterless factory wrapper is YAGNI on inspection. Several test-coverage gaps compound the risk — most notably, the boot script's runtime behaviour is verified only by source-text regex grepping, never executed in any automated test, and a likely race between the boot script's write and the unconditional `useEffect` write has no regression test.

### Cross-Cutting Themes

- **`suppressHydrationWarning` premise is incorrect** (flagged by: Correctness, Compatibility, Standards) — `src/main.tsx:11` uses `createRoot`, not `hydrateRoot`. React mounts into `<div id="root">` and never reconciles `<html>`. The attribute as written is a no-op; the test that asserts it locks in a misleading invariant. Either drop the attribute and rationale, or cite a verified mechanism.

- **Boot script unconditionally writes `data-theme` on first visit** (flagged by: Usability, Compatibility) — Breaks OS-follow-the-system mode (users who switch OS theme after first visit get a static page) and breaks the existing `library (prefers-color-scheme: dark, no data-theme attribute)` visual-regression test at `tokens.spec.ts:78-87`, which now exercises the explicit-attribute path instead of the no-attribute MIRROR-B path. Either skip the write when no `localStorage` entry exists, or update the visual-regression test to remove the attribute before screenshotting.

- **Hook implementation duplication** (flagged by: Architecture, Code Quality, Usability) — `use-theme.ts` and `use-font-mode.ts` are line-for-line near-identical (~60 of ~80 lines), including duplicated `safeGetItem`/`safeSetItem` helpers. The plan defers a generic preferences abstraction "until a third consumer arrives" — sound Rule of Three, but the *helpers* are pure utility code that can be extracted now without committing to that abstraction.

- **Boot script behaviour is never executed by any test** (flagged by: Test Coverage, Architecture, Code Quality) — `index-html.test.ts` consists of regex assertions over the raw HTML source. A typo in attribute names, swapped truthy/falsy ternary branches, or broken `try/catch` would pass every automated test as long as the literal text still mentioned `getItem('ac-theme')`. Extract the IIFE body to a testable function or evaluate it in jsdom.

- **No `storage` event listener — multi-tab desync** (flagged by: Correctness, Compatibility, Test Coverage) — Two tabs of the visualiser will diverge silently after one toggles theme. Either subscribe to `window.addEventListener('storage', ...)` and update state, or document the behaviour explicitly under "What We're NOT Doing".

- **Toggle button accessibility gaps** (flagged by: Standards, Usability) — Buttons lack `aria-pressed` despite a precedent at `LifecycleIndex.tsx:136`; `title` attribute duplicating `aria-label` is a recognised WCAG anti-pattern (not exposed to assistive tech, hidden on touch). Drop `title` (and the test asserting it), add `aria-pressed`.

- **Icon and glyph design choices are unvalidated** (flagged by: Usability, Standards, Compatibility) — "Next-state preview" convention is asserted to match GitHub/Notion (it doesn't — GitHub shows current state); `Aa`/`M` is asymmetric and `M` is poor affordance for "monospace"; Unicode glyphs `☀`/`☾` (U+263E is unconventional vs. U+263D) render inconsistently across platforms without text-presentation variation selectors.

- **Boot script's outer `try/catch` is too wide** (flagged by: Correctness, Code Quality, Standards) — A successful theme write is overwritten by the catch block if the subsequent font-mode read throws. Split into two independent try/catch blocks, one per attribute.

### Tradeoff Analysis

- **Hook duplication vs. abstraction**: Code Quality and Architecture push for extracting at least `safeGetItem`/`safeSetItem` (and ideally a `makeUseAttributeMode` primitive) now. The plan's stated YAGNI deferral is defensible for the full preferences abstraction but harder to defend for the storage helpers, which are pure utility code. Recommend: extract helpers now, defer the full abstraction.

- **Boot-script test fidelity vs. plan complexity**: Test Coverage wants the boot script extracted and behaviour-tested in jsdom; the plan keeps it inline for FOUC-prevention simplicity. Recommend: extract to `public/boot-theme.js` (or generate via Vite `transformIndexHtml`) so behaviour can be tested while preserving parser-blocking semantics.

- **`title` tooltip vs. accessibility**: Usability notes touch users get nothing without `title` since the icon is opaque; Standards says `title` is a WCAG anti-pattern. Recommend: drop `title`, but add a more discoverable affordance (visible label, tooltip component, or wider button).

### Findings

#### Major

- 🟡 **Correctness / Compatibility / Standards**: `suppressHydrationWarning` premise is incorrect — React never reconciles `<html>` under `createRoot`
  **Location**: Phase 5.1, 5.2 (`index.html` rewrite); Performance Considerations §3
  The app uses `createRoot(document.getElementById('root'))` (CSR, not hydration); React mounts into `<div id="root">` and never owns `<html>`. The hydration warning the plan claims to silence cannot fire. The `index-html.test.ts` assertion locks in a no-op marker.

- 🟡 **Usability / Compatibility**: Boot script unconditionally writes `data-theme` on first visit — breaks OS-follow mode and existing visual-regression test
  **Location**: Phase 5.2 (lines 1240-1258); Phase 9
  The existing test `library (prefers-color-scheme: dark, no data-theme attribute)` at `tests/visual-regression/tokens.spec.ts:78-87` is silently neutralised — by screenshot time the boot script has already written `data-theme`, so the MIRROR-B `:root:not([data-theme="light"])` branch is no longer exercised. Users who switch OS theme after first visit no longer follow the OS.

- 🟡 **Code Quality**: Near-identical hook implementations duplicate ~60 lines
  **Location**: Phase 3.2 / Phase 4.2 (`use-theme.ts`, `use-font-mode.ts`)
  The two files share `safeGetItem`/`safeSetItem`, `readInitial`, factory, attribute-writing `useEffect`, setter, toggler, default handle, context, consumer hook — differing only in attribute name, storage key, value union, and one DI point. Extract a shared `makeUseAttributeMode<T>` primitive or at minimum a `safe-storage.ts` utility.

- 🟡 **Test Coverage**: Boot script behaviour is never executed — only its source text is grepped
  **Location**: Phase 5.1 (`src/styles/index-html.test.ts`)
  Seven regex assertions verify that `getItem('ac-theme')` etc. *appear in* `index.html`, but the IIFE is never run. A typo in attribute names, swapped ternary branches, or broken `try/catch` would pass every automated test. Extract to a testable function or evaluate the script in jsdom.

- 🟡 **Test Coverage**: No test for the boot-script ↔ useEffect race / overwrite
  **Location**: Phase 3.2 (use-theme.ts useEffect) + Phase 5 boot-script
  The unconditional `useEffect(() => setAttribute(...))` at line 816-818 will overwrite the boot script's write if React state initialises differently. No test sets up *conflicting* initial sources (`data-theme="dark"` AND `localStorage="light"`) and asserts precedence. Lock in the priority ordering with explicit precedence tests.

- 🟡 **Test Coverage**: FOUC prevention and hydration-warning suppression have no automated coverage
  **Location**: Phase 5 Manual Verification
  The two highest-risk acceptance criteria — "no flash" and "no hydration warning" — are verified manually only. A `defer` accidentally added to the boot script, a stylesheet `<link>` injected before it, or a removed `suppressHydrationWarning` would silently break the AC. Add a Playwright first-paint test and a `console.error` spy.

- 🟡 **Test Coverage**: Brittle regex assertions couple Phase 1 tests to CSS formatting
  **Location**: Phase 1.1 / Phase 1.2 (global.test.ts body-rule regex; migration.test.ts route-title regex)
  `[^}]*` body matches fail on multiple `body { ... }` rules; the plan even notes `global.css` currently has two and asks the implementer to consolidate — encoding an invariant in prose rather than a test. Use the existing `extractBlockBody` brace-balanced helper.

- 🟡 **Correctness**: Boot script catch block overwrites a successfully-applied theme on partial failure
  **Location**: Phase 5.2 (lines 1234-1258)
  The outer `try` wraps both theme and font-mode reads. If the theme write succeeds but the font-mode read throws, the catch handler unconditionally re-writes `data-theme` from `prefers-color-scheme`, discarding the user's stored preference. Split into two independent try/catch blocks.

- 🟡 **Correctness / Compatibility**: No `storage` event listener — multi-tab state diverges silently
  **Location**: Phase 3.2 / Phase 4.2 (use-theme.ts, use-font-mode.ts)
  Two tabs that toggle theme will diverge until reload. Either subscribe to `storage` events or list cross-tab sync explicitly under "What We're NOT Doing".

- 🟡 **Standards**: Toggle buttons should use `aria-pressed` per existing codebase convention
  **Location**: Phase 6.2 / Phase 7.2 (component definitions)
  `LifecycleIndex.tsx:136` already uses `aria-pressed={active}` for an analogous toggle. Screen-reader users currently get no audible state cue.

- 🟡 **Standards**: `title` attribute duplicating `aria-label` is a known a11y anti-pattern
  **Location**: Phase 6.2 line 1413, Phase 7.2 line 1598
  Not exposed reliably to assistive tech, unreachable to keyboard, hidden on touch; some screen readers double-announce when it duplicates `aria-label`. Tests lock the duplication in. Drop `title` and the matching test.

- 🟡 **Usability**: "Next-state preview" icon convention is asserted but not actually conventional
  **Location**: Phase 6.1 lines 1388-1391 (claim that GitHub/Notion use this)
  GitHub shows current state, not next state. macOS, iOS, Slack, VS Code, Linear show current state. The unsubstantiated justification mis-frames the design choice.

- 🟡 **Usability**: `M` glyph for mono toggle is poor affordance; `Aa`/`M` pairing is asymmetric
  **Location**: Phase 7.2 lines 1599-1611
  Most non-designer users won't know what "mono" means; a bare `M` is just a letter. Asymmetric (two chars vs one) reduces visual cohesion. Try `Aa`(Inter) ↔ `Aa`(Fira Code) — same characters in different fonts directly previews the change.

- 🟡 **Usability**: Boot script writes attribute on every first visit, breaking the OS-follow mode
  **Location**: Phase 5.2 (already covered above as cross-cutting with Compatibility)
  Same root cause; usability impact is that users on macOS with auto light/dark scheduling get a static page after first visit.

- 🟡 **Compatibility**: Boot script unconditionally writes `data-theme` — breaks the existing 'no data-theme attribute' visual-regression test
  **Location**: Phase 5.2, Phase 9 (already covered above)

- 🟡 **Compatibility**: `suppressHydrationWarning` premise is incorrect — `createRoot`, not `hydrateRoot`
  **Location**: Phase 5 (already covered above)

#### Minor

- 🔵 **Code Quality**: Parameterless `makeUseFontMode()` factory preserved "for symmetry" — YAGNI on its face
  **Location**: Phase 4.2 lines 1086-1090

- 🔵 **Code Quality / Architecture**: `safeGetItem`/`safeSetItem` duplicated across two hook files — extract to `safe-storage.ts`
  **Location**: Phase 3.2 / Phase 4.2

- 🔵 **Code Quality**: Boot script's nested `prefersDark2` try block defends against an implausible failure mode
  **Location**: Phase 5.2 (matchMedia inner try)

- 🔵 **Code Quality / Test Coverage**: `index-html.test.ts` regex assertions are brittle to formatting changes
  **Location**: Phase 5.1

- 🔵 **Code Quality**: `AC5_FLOOR` adjusted three separate times in one plan creates churn
  **Location**: Phases 1.5, 6.4, 7.4 — bump once at end

- 🔵 **Code Quality**: Two near-identical button components with the same styles
  **Location**: Phase 6.2/6.3 vs Phase 7.2/7.3 — extract a `<TopbarIconButton>`

- 🔵 **Code Quality**: `suppressHydrationWarning` on `<html>` is broader than needed (assuming it applied at all)
  **Location**: Phase 5

- 🔵 **Architecture**: Storage keys + value sets duplicated across HTML boot script and TS hooks with no enforced contract
  **Location**: Phase 5 vs Phase 3/4

- 🔵 **Architecture**: Provider nesting order in RootLayout is not motivated
  **Location**: Phase 4.3

- 🔵 **Architecture**: Two writers to `<html data-theme>` (boot script + useEffect) with implicit handoff
  **Location**: Phase 3.2

- 🔵 **Architecture**: Topbar hard-couples to specific toggle components, eroding the slot abstraction
  **Location**: Phase 8.2

- 🔵 **Architecture**: Hooks couple directly to `document.documentElement` and global `localStorage`
  **Location**: Phase 3.2 / Phase 4.2

- 🔵 **Test Coverage**: Topbar test mounts real toggles + mocks their hooks — pyramid level confusion
  **Location**: Phase 8.1

- 🔵 **Test Coverage**: Boot-script ordering test is conditionally a no-op (only fires when `<link>` already present)
  **Location**: Phase 5.1 lines 1196-1208

- 🔵 **Test Coverage**: Manual baseline re-capture is the only protection against shipping broken dark mode
  **Location**: Phase 9 — add a luminance-channel assertion as a sanity check

- 🔵 **Correctness**: First-visit OS-preference is not persisted, so behaviour shifts if OS pref changes between visits
  **Location**: Phase 5.2 (also surfaces under Usability)

- 🔵 **Correctness**: Unconditional `useEffect` write can clobber a divergent attribute set by another agent
  **Location**: Phase 3.2

- 🔵 **Correctness**: Test "initial state reflects pre-existing data-theme attribute" doesn't lock in precedence over conflicting `localStorage`
  **Location**: Phase 3.1 lines 689-694

- 🔵 **Correctness**: `AC5_FLOOR` delta documented as +4 may undercount LibraryTemplatesIndex new `.title` rule
  **Location**: Phase 1.5

- 🔵 **Correctness**: `data-icon` test doesn't verify the rendered glyph text
  **Location**: Phase 6.2 / 7.2 tests

- 🔵 **Correctness**: `safeSetItem` called inside `setThemeState` updater violates React purity (Strict Mode double-invoke)
  **Location**: Phase 3.2 lines 825-831

- 🔵 **Standards**: `prefers-reduced-motion` not addressed despite codebase precedent (`OriginPill`, `SseIndicator`)
  **Location**: Phase 6.3 / 7.3

- 🔵 **Standards**: Forced-colors mode handling asserted in manual verification but not specified in CSS
  **Location**: Phase 6.3 line 1439

- 🔵 **Usability**: `makeUseFontMode()` no-arg factory is awkward DX
  **Location**: Phase 4.2

- 🔵 **Usability**: `THEME_STORAGE_KEY`/`FONT_MODE_STORAGE_KEY` exported but only used internally
  **Location**: Phase 3.2 / Phase 4.2

- 🔵 **Usability**: `useTheme`/`useThemeContext` naming collision risk — wrong import gives independent state
  **Location**: Phase 3.2 / Phase 4.2

- 🔵 **Usability**: Tooltip via `title` is desktop-only — touch users get only the icon
  **Location**: Phase 6.2 / 7.2 (also see Standards major)

- 🔵 **Usability**: No keyboard shortcut for the toggles
  **Location**: Plan as a whole

- 🔵 **Usability**: `data-font="display"` is a confusable token name (CSS `display` property)
  **Location**: Overview, Phase 10

- 🔵 **Usability**: Phases 1-7 produce nothing user-visible — incremental review/demo not possible until Phase 8
  **Location**: Implementation Approach

- 🔵 **Compatibility**: `useState` lazy initialiser reads `document` directly — assumes browser-only execution (forward-compat with SSR)
  **Location**: Phase 3.2 / Phase 4.2

- 🔵 **Compatibility**: Mocking `Storage.prototype.setItem` under jsdom requires verification (jsdom may bypass prototype)
  **Location**: Phase 3.1 / 4.1

- 🔵 **Compatibility**: Inner `matchMedia` try/catch in boot script is unjustified
  **Location**: Phase 5.2

- 🔵 **Compatibility**: `index-html.test.ts` parses source — won't catch Vite plugin reordering at build time
  **Location**: Phase 5.1

- 🔵 **Compatibility**: Unicode glyphs `☀`/`☾` render inconsistently across platforms (some emoji-fy, some text-fy)
  **Location**: Phase 6.2 — append `︎` text-presentation selector

#### Suggestions

- 🔵 **Architecture**: Cost of the third preference-hook copy is not explicitly bounded — name the trigger in "What We're NOT Doing"

- 🔵 **Code Quality**: `toggleTheme` could route through `setTheme` to avoid duplicating persistence logic

- 🔵 **Standards**: ☾ (U+263E) is unconventional; ☽ (U+263D) pairs better with ☀ at small sizes

- 🔵 **Standards**: Type guards `isTheme`/`isFontMode` are not exported — boot script reimplements value lists inline

- 🔵 **Standards**: Boot script's outer `try` wraps `setAttribute` calls too — narrow to only the `localStorage.getItem` reads

- 🔵 **Usability**: No documented way for users to reset preferences (clear localStorage)

- 🔵 **Compatibility**: Playwright dark visual-regression timing now races with boot script — use `addInitScript` or `emulateMedia`

### Strengths

- ✅ Phases are TDD-disciplined and each leaves the system in a compilable, testable state
- ✅ Phase 1 surfaces and fixes the inert-token bug from 0033 before the toggle ships, preventing the new feature from exposing a structural defect
- ✅ Faithfully imitates the canonical `use-doc-events.ts` factory + context + consumer-hook pattern
- ✅ Type guards (`isTheme`/`isFontMode`) reject malformed `localStorage` values cleanly
- ✅ Private-mode `localStorage` SecurityError is explicitly tested and the safe-helpers swallow only the expected failure mode
- ✅ Hook tests use dependency injection via `makeUseTheme(prefersDark)` — avoids `matchMedia` stubbing flakiness
- ✅ Token usage in toggle CSS consumes pre-baked `--ac-bg-hover`/`--ac-bg-active`, respecting ADR-0026's locked percentages
- ✅ `data-icon` attribute idiom is consistent with existing `data-state`/`data-animated` patterns
- ✅ Glyph spans correctly carry `aria-hidden="true"` so visible characters aren't double-announced
- ✅ Phase 10 reconciles the `default` vs `display` AC wording — surfaces and resolves a real inconsistency
- ✅ Visual regression baselines explicitly scheduled for re-capture in Phase 9
- ✅ "What We're NOT Doing" section explicitly defers a generic preferences abstraction — sound YAGNI discipline
- ✅ Setting `color-scheme: light dark` ensures user-agent form controls and scrollbars track the toggle

### Recommended Changes

Prioritised by impact:

1. **Verify `suppressHydrationWarning` is actually needed; if not, drop it and the test** (addresses: `suppressHydrationWarning` premise is incorrect — 3 lenses)
   Open `main.tsx`, confirm `createRoot` (not `hydrateRoot`) — there is no hydration pass and React doesn't render `<html>`. Remove the attribute from `index.html`, remove the test in `index-html.test.ts`, and update Performance Considerations §3 to remove the hydration-warning narrative. If a future SSR change makes it relevant, add it then.

2. **Fix the boot-script first-visit OS-follow break** (addresses: Boot script unconditionally writes `data-theme`, Visual regression test silently neutralised)
   Change Phase 5.2 to skip the `setAttribute` call when `localStorage` has no entry — let the CSS `prefers-color-scheme` mirror handle the no-attribute path. The flash-prevention guarantee still holds because the parser-blocking stylesheet already paints correctly. Update `tests/visual-regression/tokens.spec.ts:78-87` if a `removeAttribute` step is needed; otherwise the test continues to verify MIRROR-B as designed.

3. **Split the boot-script `try/catch` per attribute** (addresses: Catch block overwrites theme on partial failure)
   Wrap the `ac-theme` read+write in its own try/catch, and the `ac-font-mode` read+write in another. Each falls back to its own default on its own failure.

4. **Make boot-script behaviour testable in jsdom** (addresses: Boot script behaviour never executed; brittle regex assertions; storage-key duplication)
   Extract the IIFE body to `public/boot-theme.js` (or generate it from a shared TS module via Vite's `transformIndexHtml` hook). Reference it from `index.html` as a classic `<script src>` (still parser-blocking). Write `boot-theme.test.ts` that imports and runs it against a stubbed `localStorage`/`matchMedia`/`document`. The shared TS module then exports `THEME_STORAGE_KEY`/`FONT_MODE_STORAGE_KEY` (and `isTheme`/`isFontMode`) for both the boot script and the hooks, eliminating the cross-file duplication.

5. **Add `aria-pressed` and drop `title`** (addresses: Toggle buttons should use aria-pressed, title duplicates aria-label)
   In `ThemeToggle` and `FontModeToggle`: add `aria-pressed={theme === 'dark'}` / `aria-pressed={fontMode === 'mono'}`. Remove the `title` attribute and the matching test assertions. If a hover tooltip is desired, build one with a tooltip component sourced from the same `label` string.

6. **Decide on cross-tab sync explicitly** (addresses: No storage event listener; multi-tab desync)
   Either add a `useEffect` subscribing to `storage` events with a corresponding test, or list "cross-tab synchronisation" under "What We're NOT Doing" in the plan.

7. **Extract `safeGetItem`/`safeSetItem` to `src/api/safe-storage.ts`** (addresses: Hook implementation duplication, helper duplication)
   This is a small, low-risk extraction that doesn't commit to a generic preferences abstraction. Test the helper once; both hooks import from it. Defer the larger `makeUseAttributeMode` extraction until a third consumer arrives, but document the trigger in "What We're NOT Doing".

8. **Replace brittle Phase 1 regexes with `extractBlockBody`** (addresses: Brittle regex assertions; AC5_FLOOR delta inaccuracy)
   Reuse the existing brace-balanced helper from `global.test.ts:110-122`. Assert there is exactly one top-level `body { ... }` rule, then check declarations on its extracted body. For migration.test.ts route titles, use the same extraction approach.

9. **Drop the parameterless `makeUseFontMode` factory** (addresses: Parameterless factory YAGNI; awkward DX)
   Export `useFontMode` directly. If a real DI need emerges later, introduce the factory mechanically.

10. **Add precedence + race tests for the hooks** (addresses: No test for boot-script ↔ useEffect race; weak precedence assertion)
    Add tests that establish *conflicting* initial state: `data-theme="dark"` + `localStorage="light"` + `prefersDark()=>false` → assert `result.current.theme === "dark"` and that `<html data-theme>` remains `"dark"` after mount. Mirror in `use-font-mode.test.ts`.

11. **Validate icon design choices** (addresses: "Next-state preview" claim, `M` glyph affordance, Unicode rendering inconsistency)
    Either flip to current-state preview (matches macOS/VS Code) or own the next-state choice without the unsubstantiated GitHub/Notion claim. Replace `Aa`/`M` with `Aa`(Inter)/`Aa`(Fira Code) for direct preview. Append `︎` to `☀`/`☾` for monochrome cross-platform rendering, and consider U+263D in place of U+263E.

12. **Bump `AC5_FLOOR` once at the end** (addresses: AC5_FLOOR adjusted three separate times)
    Remove the per-phase ratchet bumps from Phases 1.5, 6.4, 7.4. Add a single bump after Phase 7 lands with one comment naming the work item.

13. **Drop the inner `matchMedia` try/catch and narrow the outer** (addresses: Implausible failure-mode defence; outer try too wide)
    The inner try/catch in the boot script defends against `matchMedia` throwing — universally available since Safari 5.1, no plausible target. Narrow the outer try to wrap only the `localStorage.getItem` calls, leaving `matchMedia` and `setAttribute` outside.

14. **Add a `prefers-reduced-motion`-aware comment to toggle CSS** (addresses: Reduced-motion not addressed)
    Either declare hover/active changes are intentionally unanimated (one-line comment), or pre-emptively wrap any future transitions in `@media (prefers-reduced-motion: no-preference)` per the existing `OriginPill`/`SseIndicator` pattern.

15. **Add a `forced-colors` block to toggle CSS** (addresses: Forced-colors handling asserted but not specified)
    `@media (forced-colors: active) { .toggle { border: 1px solid ButtonText; } }` so the button is visibly bordered in High Contrast mode. Add a corresponding test mirroring `OriginPill.test.tsx:30`.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan adopts the codebase's canonical context pattern (use-doc-events) faithfully and slices the work into TDD-friendly phases that each leave the system in a working state. From an architecture lens, the main concerns are: the factory pattern is half-applied, low-value duplication between the two hooks could be reduced today without committing to a generic 'preferences' abstraction, and the boot script and React hooks duplicate state-loading logic across a TS/HTML boundary with no formalised contract.

**Findings**:
- 🔵 *minor (high)*: Factory pattern for useFontMode adds no value (Phase 4)
- 🔵 *minor (high)*: safeGetItem/safeSetItem duplicated across two hook files (Phase 3 + Phase 4)
- 🔵 *minor (high)*: Storage keys and value sets duplicated across HTML boot script and TS hooks (Phase 5 vs Phase 3/4)
- 🔵 *minor (medium)*: Provider nesting order is not motivated and increases visual noise (Phase 4: RootLayout)
- 🔵 *minor (medium)*: Two writers to `<html data-theme>` with implicit handoff (Phase 3)
- 🔵 *minor (medium)*: Topbar now hard-couples to specific toggle components, eroding the slot abstraction (Phase 8)
- 🔵 *minor (medium)*: Hook implementations couple directly to document.documentElement and global localStorage (Phase 3 + Phase 4)
- 🔵 *suggestion (low)*: Cost of the third copy is not explicitly bounded (Overview)

### Code Quality

**Summary**: The plan is well-structured, follows TDD discipline, and faithfully imitates an existing canonical hook pattern, which keeps the codebase consistent. However, the symmetric duplication between useTheme/useFontMode (including helpers, factory shape, and tests) and the brittle assertions in several test sections are likely maintenance liabilities.

**Findings**:
- 🟡 *major (high)*: Near-identical hook implementations duplicate ~60 lines of logic and storage helpers (Phase 3.2 + Phase 4.2)
- 🔵 *minor (high)*: Parameterless factory preserved 'purely for symmetry' is YAGNI on its face (Phase 4.2 lines 1086-1090)
- 🔵 *minor (high)*: Storage safety helpers duplicated verbatim across two files (Phase 3.2 + Phase 4.2)
- 🔵 *minor (medium)*: Nested fallback `prefersDark2` block defends against an implausible failure mode (Phase 5.2)
- 🔵 *minor (high)*: Boot-script tests assert content via brittle regex literals (Phase 5.1)
- 🔵 *minor (medium)*: Single-block `body { … }` regex relies on an unstated CSS-source invariant (Phase 1.1)
- 🔵 *minor (medium)*: AC5_FLOOR adjusted three separate times in one plan creates churn (Phases 1.5, 6.4, 7.4)
- 🔵 *minor (medium)*: Two near-identical button components with the same styles (Phase 6.2 + 7.2)
- 🔵 *minor (medium)*: `suppressHydrationWarning` on `<html>` is broader than needed (Phase 5)
- 🔵 *suggestion (low)*: Both `setTheme` and `toggleTheme` write to localStorage independently (Phase 3.2/4.2)

### Test Coverage

**Summary**: The plan is unusually thorough on TDD discipline, but several critical paths are tested only by static text/regex assertions on source files rather than runtime behaviour: the boot script's actual attribute-writing logic is never executed in any test, FOUC and hydration-warning claims are verified manually only, and a likely race condition between the boot script and the `useEffect` write has no regression test.

**Findings**:
- 🟡 *major (high)*: Boot script behaviour is never executed — only its source text is grepped (Phase 5.1)
- 🟡 *major (high)*: No test for the boot-script ↔ useEffect race / overwrite (Phase 3.2 + Phase 5)
- 🟡 *major (medium)*: FOUC prevention and hydration-warning suppression have no automated coverage (Phase 5)
- 🟡 *major (high)*: Brittle regex assertions couple tests to CSS formatting (Phase 1.1, 1.2)
- 🔵 *minor (high)*: Topbar test now mounts real toggle components — pyramid level mismatch (Phase 8.1)
- 🔵 *minor (high)*: Boot-script ordering test is conditionally a no-op (Phase 5.1 line 1196-1208)
- 🔵 *minor (medium)*: Missing cross-tab synchronisation and storage-event coverage (Phase 3.1 + 4.1)
- 🔵 *minor (medium)*: Manual baseline re-capture is the only protection against shipping broken dark mode (Phase 9)

### Correctness

**Summary**: The plan is broadly logically sound, but there are several real correctness defects: a partial-failure path in the boot script silently overwrites a valid theme attribute when font-mode read throws; the suppressHydrationWarning premise rests on a likely-false assumption; and there is no cross-tab synchronisation.

**Findings**:
- 🟡 *major (high)*: Boot script catch block overwrites a successfully-applied theme on partial failure (Phase 5.2)
- 🟡 *major (high)*: `suppressHydrationWarning` on `<html>` is likely a no-op — React does not reconcile the `<html>` element (Phase 5.1/5.2)
- 🟡 *major (medium)*: No `storage` event listener — multi-tab state diverges silently (Phase 3.2/4.2)
- 🔵 *minor (medium)*: First-visit OS-preference choice is never persisted (Phase 5.2)
- 🔵 *minor (medium)*: Unconditional `useEffect` write can clobber a divergent attribute (Phase 3.2)
- 🔵 *minor (high)*: Test does not assert React-state init wins over conflicting localStorage (Phase 3.1)
- 🔵 *minor (medium)*: `AC5_FLOOR` delta documented as +4 may undercount (Phase 1.5)
- 🔵 *minor (medium)*: `data-icon` attribute test, but unit doesn't verify rendered glyph (Phase 6.2/7.2)
- 🔵 *minor (high)*: `safeSetItem` called inside `setThemeState` updater violates React purity (Phase 3.2)

### Standards

**Summary**: The plan is largely well-aligned with established project conventions (uses canonical patterns, ADR-0026-conformant tokens, data-attribute idiom). The main standards gaps are accessibility-shaped: missing aria-pressed, title duplicating aria-label, no prefers-reduced-motion, and the boot script's reliance on a JSX-cased suppressHydrationWarning attribute embedded in static HTML.

**Findings**:
- 🟡 *major (high)*: Toggle buttons should use `aria-pressed` per existing codebase convention (Phase 6.2 / 7.2)
- 🟡 *major (high)*: `title` attribute duplicating `aria-label` is a known a11y anti-pattern (Phase 6.2 line 1413, Phase 7.2 line 1598)
- 🟡 *major (high)*: `suppressHydrationWarning` in static HTML is JSX-only — needs verified mechanism or doc reference (Phase 5.2)
- 🔵 *minor (medium)*: No `prefers-reduced-motion` consideration despite codebase precedent (Phase 6.3 + 7.3)
- 🔵 *minor (medium)*: Forced-colors mode handling is asserted but not specified (Phase 6.3)
- 🔵 *suggestion (medium)*: Unconventional moon glyph (U+263E) (Phase 6.2)
- 🔵 *suggestion (low)*: Type guards `isTheme` / `isFontMode` not exported (Phase 3.2 / 4.2)
- 🔵 *suggestion (low)*: Boot-script wraps `setAttribute` calls in the same `try` as `localStorage` (Phase 5.2)

### Usability

**Summary**: Competently structured but has end-user UX gaps and DX smells. Most concerning: the boot script eliminates OS-follow mode that the work item AC implies should be available; the icon convention claim is unsubstantiated; the Aa/M glyphs are picked without validation.

**Findings**:
- 🟡 *major (high)*: "Next-state preview" icon convention is asserted but not actually conventional (Phase 6.1, 6.2, 7.1, 7.2)
- 🟡 *major (high)*: `M` glyph for mono toggle is poor affordance; `Aa`/`M` pairing is asymmetric (Phase 7.2)
- 🟡 *major (high)*: Boot script writes attribute on every first visit, breaking the OS-follow mode (Phase 5.2)
- 🔵 *minor (high)*: `makeUseFontMode()` factory takes no parameters — awkward DX (Phase 4.2)
- 🔵 *minor (high)*: Two near-identical hook files create maintenance burden (Phase 3.2 + 4.2)
- 🔵 *minor (high)*: `THEME_STORAGE_KEY`/`FONT_MODE_STORAGE_KEY` exported but only used internally (Phase 3.2 + 4.2)
- 🔵 *minor (medium)*: `useTheme`/`useThemeContext` (and font equivalents) — collision risk (Phase 3.2/4.2)
- 🔵 *minor (medium)*: Tooltip via `title` is desktop-only (Phase 6.2/7.2)
- 🔵 *minor (medium)*: No keyboard shortcut and no Esc handling for the toggles (Phase 6.2/7.2)
- 🔵 *minor (medium)*: Attribute value `display` is a confusable token (Overview, Phase 10)
- 🔵 *minor (medium)*: Phases 1-7 have no user-visible output — incremental review/demo not possible until Phase 8
- 🔵 *suggestion (medium)*: No documented way for users to reset preferences

### Compatibility

**Summary**: Targets a modern Vite + React 19 SPA with well-supported web platform features and appropriate fallbacks. Most significant compatibility issues are conceptual: the suppressHydrationWarning rationale rests on a false premise (createRoot, no hydration), and the boot script unconditionally writing data-theme breaks an existing visual-regression test.

**Findings**:
- 🟡 *major (high)*: `suppressHydrationWarning` premise is incorrect — app uses `createRoot`, not `hydrateRoot` (Phase 5)
- 🟡 *major (high)*: Boot script unconditionally writes `data-theme` — breaks the existing 'no data-theme attribute' visual-regression test (Phase 5.2 / Phase 9)
- 🔵 *minor (high)*: No 'storage' event listener — multi-tab usage will silently desync (Phase 3 & 4)
- 🔵 *minor (medium)*: `useState` lazy initialiser reads document directly — assumes browser-only execution (Phase 3 & 4)
- 🔵 *minor (medium)*: Mocking Storage.prototype.setItem under jsdom requires verification (Phase 3.1 / 4.1)
- 🔵 *minor (low)*: Nested matchMedia try/catch is unjustified (Phase 5.1)
- 🔵 *minor (medium)*: Test parses index.html with regex — brittle to formatting changes Vite may introduce (Phase 5.1)
- 🔵 *minor (medium)*: Unicode sun/moon glyphs render inconsistently across platforms (Phase 6.2 / 7.2)
- 🔵 *suggestion (medium)*: Playwright colorScheme emulation now interacts with the boot script (Phase 9)

## Re-Review (Pass 2) — 2026-05-08

**Verdict:** REVISE (10 new/persisting major findings; substantially improved overall but new structural issues introduced)

### Previously Identified Issues

**Resolved:**
- ✅ **Architecture**: Factory pattern for useFontMode adds no value — Resolved (factory dropped; rationale documented)
- ✅ **Architecture**: Storage keys + value sets duplicated across HTML/TS — Resolved for keys (BOOT_SCRIPT_SOURCE generated from constants); value sets still duplicated but acknowledged
- ✅ **Architecture**: Cost of third copy not bounded — Resolved (cross-tab sync entry added to "What We're NOT Doing")
- ✅ **Code Quality**: Parameterless makeUseFontMode factory YAGNI — Resolved
- ✅ **Code Quality**: Storage helpers duplicated — Resolved (Phase 3.0 safe-storage extraction)
- ✅ **Code Quality**: Boot script nested prefersDark2 try block — Resolved (matchMedia removed entirely)
- ✅ **Code Quality**: Brittle index-html.test.ts regex — Resolved (replaced with behavioural boot-theme.test.ts + dist-only structural test)
- ✅ **Code Quality**: Single-block body regex — Resolved (extractBlockBody + countTopLevelBodyRules invariant)
- ✅ **Code Quality**: AC5_FLOOR adjusted three times — Resolved (single bump in Phase 7.4)
- ✅ **Code Quality**: suppressHydrationWarning broader than needed — Resolved (attribute dropped entirely)
- ✅ **Test Coverage**: Boot script behaviour never executed — Resolved (applyBootAttributes tested with 7 scenarios; *but see new finding below*)
- ✅ **Test Coverage**: No test for boot-script ↔ useEffect race — Resolved (new precedence test in Phase 3.1)
- ✅ **Test Coverage**: FOUC + hydration warning manual only — Hydration moot (createRoot); FOUC still manual
- ✅ **Test Coverage**: Brittle regex assertions for body/title — Resolved (extractBlockBody)
- ✅ **Test Coverage**: Boot-script ordering test conditionally a no-op — Resolved (dedicated boot-theme.html.test.ts on dist/index.html)
- ✅ **Correctness**: Boot script catch overwrites theme on partial failure — Resolved (per-attribute try/catch; dedicated test)
- ✅ **Correctness**: suppressHydrationWarning premise wrong — Resolved (dropped + corrected rationale)
- ✅ **Correctness**: First-visit OS-pref not persisted — Resolved (boot script writes nothing; CSS mirror governs dynamically)
- ✅ **Correctness**: Test doesn't lock in precedence — Resolved
- ✅ **Correctness**: AC5_FLOOR delta off — Resolved
- ✅ **Correctness**: data-icon test doesn't verify glyph — Resolved (textContent assertion added)
- ✅ **Standards**: Toggle buttons should use aria-pressed — Resolved (added; test asserts both states)
- ✅ **Standards**: title attribute duplicating aria-label — Resolved (title removed)
- ✅ **Standards**: suppressHydrationWarning in static HTML — Resolved (dropped)
- ✅ **Standards**: prefers-reduced-motion not addressed — Resolved (rationale comment added; *but see new finding on the comment's wording*)
- ✅ **Standards**: forced-colors handling not specified — Resolved (@media block + presence test)
- ✅ **Standards**: ☾ glyph unconventional — Resolved (☽ U+263D + variation selector)
- ✅ **Standards**: Type guards isTheme/isFontMode not exported — Resolved (now exported)
- ✅ **Standards**: Boot-script try too wide — Resolved (per-attribute scoping)
- ✅ **Usability**: "Next-state preview" claim unsubstantiated — Resolved (current-state convention; unsubstantiated GitHub/Notion claim removed)
- ✅ **Usability**: M glyph poor affordance / asymmetry — Resolved (replaced with Aa rendered in target font; *but see new finding on plan's stale Phase 7 manual-verification text*)
- ✅ **Usability**: Boot script breaks OS-follow mode — Resolved (no-write-on-first-visit)
- ✅ **Usability**: makeUseFontMode awkward DX — Resolved
- ✅ **Usability**: THEME_STORAGE_KEY/FONT_MODE_STORAGE_KEY exported but unused — Resolved (boot-theme.ts imports them)
- ✅ **Usability**: title is desktop-only — Resolved (title removed)
- ✅ **Compatibility**: suppressHydrationWarning premise — Resolved
- ✅ **Compatibility**: Boot script breaks no-data-theme visual-regression test — Resolved (no-write-on-first-visit preserves MIRROR-B path)
- ✅ **Compatibility**: vi.spyOn Storage.prototype needs verification — Resolved (`expect(spy).toHaveBeenCalled()` added)
- ✅ **Compatibility**: Inner matchMedia try unjustified — Resolved (removed)
- ✅ **Compatibility**: index-html.test.ts brittle to Vite reorderings — Resolved (dist-output test)
- ✅ **Compatibility**: Unicode glyphs render inconsistently — Resolved (U+FE0E variation selector)

**Still present:**
- 🟡 **Code Quality**: Two near-identical button components / CSS modules duplicate ~30 lines (TopbarIconButton not extracted) — Still present (major)
- 🔵 **Code Quality**: Hook implementation duplication (~50 lines per hook) — Still present, but explicitly deferred under "What We're NOT Doing"
- 🔵 **Code Quality**: setTheme/toggleTheme persistence duplication — Still present
- 🟡 **Test Coverage**: Topbar 'integration' test never click-throughs — Still present (major)
- 🔵 **Test Coverage**: Manual baseline re-capture has no luminance assertion — Still present
- 🔵 **Test Coverage**: FOUC verification still manual-only — Still present
- 🟡 **Correctness**: safeSetItem inside setStateUpdater violates React purity — Still present (major)
- 🔵 **Correctness**: Unconditional useEffect can clobber external writes — Still present
- 🟡 **Usability**: useTheme/useThemeContext naming collision — Still present (major)
- 🔵 **Usability**: No keyboard shortcut — Still present
- 🔵 **Usability**: "display" awkward in user-visible aria-label — Still present (now framed via screen-reader UX)
- 🔵 **Usability**: Phases 1-7 produce nothing user-visible — Still present
- 🔵 **Usability**: No reset-to-OS-follow affordance — Still present
- 🔵 **Compatibility**: useState lazy initialiser reads document directly (SSR forward-compat) — Still present
- 🔵 **Architecture**: Provider nesting order not motivated — Still present (suggestion)
- 🔵 **Architecture**: Topbar hard-couples to specific toggle components — Still present (suggestion)
- 🔵 **Architecture**: Hooks reach to document.documentElement directly — Still present (suggestion)

### New Issues Introduced

#### Major

- 🟡 **Code Quality / Test Coverage / Correctness**: `applyBootAttributes` and `BOOT_SCRIPT_SOURCE` are parallel implementations of the same logic
  **Location**: Phase 5.1 (boot-theme.ts), Phase 5.3 (boot-theme.test.ts)
  Three lenses converged on this. The TS function is unit-tested but never runs in production; the IIFE string is what actually runs but is never executed by any test. They share storage-key constants but the value-validation literals (`t === 'light' || t === 'dark'`) and control flow are duplicated by hand. A divergence between them would ship undetected, defeating the purpose of the refactor that was meant to make boot-script behaviour testable.
  **Suggestion**: Either (a) execute `BOOT_SCRIPT_SOURCE` via `new Function(BOOT_SCRIPT_SOURCE)()` against stubbed deps in the same test suite, or (b) generate `BOOT_SCRIPT_SOURCE` mechanically from `applyBootAttributes.toString()`, or (c) at minimum add a parity assertion confirming both share the same valid-value sets.

- 🟡 **Test Coverage**: `boot-theme.html.test.ts` verifies script position but not script body
  **Location**: Phase 5.4
  Asserts the first `<head>` child is a classic `<script>` and that it precedes stylesheet links — does not assert the inlined script body equals `BOOT_SCRIPT_SOURCE` or contains canonical strings like `localStorage.getItem('ac-theme')`. A Vite plugin regression that injects an empty script tag would pass.
  **Suggestion**: Add `expect(html).toContain(BOOT_SCRIPT_SOURCE)` (or a substring check for the literal storage-key strings) to lock in the contents.

- 🟡 **Compatibility**: Phase 5.2 silently switches `defineConfig` import from `vitest/config` to `vite`
  **Location**: Phase 5.2 (vite.config.ts plugin)
  The plan's snippet uses `import { defineConfig, type Plugin } from 'vite'` — but the existing vite.config.ts imports `defineConfig` from `vitest/config` so the embedded `test: { ... }` block is type-checked. Switching the import demotes that typing.
  **Suggestion**: Keep `import { defineConfig } from 'vitest/config'` and import only `type Plugin` from `vite` (`import type { Plugin } from 'vite'`). Call this out explicitly in §5.2.

- 🟡 **Standards / Correctness / Usability**: aria-pressed paired with action-describing aria-label conflicts with WAI-ARIA APG
  **Location**: Phase 6.2, Phase 7.2
  Combination produces screen-reader output like "Switch to dark theme, toggle button, pressed" when dark is already active — semantically self-contradicting. APG recommends function-describing labels (`Dark theme`) so the press state semantically completes the meaning. Codebase precedent at `LifecycleIndex.tsx:136` uses function-describing labels.
  **Suggestion**: Change aria-label to function-describing form (`Dark theme` / `Mono font`) and keep aria-pressed. Update test assertions accordingly. Alternatively drop aria-pressed and keep the action-describing label — option (a) is preferred for codebase consistency.

#### Minor

- 🔵 **Architecture / Compatibility**: Vite config now imports React-bearing modules to extract two strings
  Pulling boot-theme → use-theme/use-font-mode into `vite.config.ts` brings React into the build-config evaluation graph just for storage-key constants. Suggestion: extract a tiny `src/api/storage-keys.ts` (no React imports) and import from both hooks and boot-theme.

- 🔵 **Code Quality**: `extractBlockBody` copy-pasted into `migration.test.ts`
  Phase 1.2 instructs adding a verbatim copy of the brace-balanced extractor. Promote to a shared `src/styles/__tests__/css-helpers.ts` and import from both test files.

- 🔵 **Correctness**: `countTopLevelBodyRules` regex mishandles two-level at-rule nesting and CSS comments
  Non-greedy alternation only handles one nesting level; comments are not stripped. Use a brace-depth walker that ignores depth>0 and skips `/* ... */`.

- 🔵 **Correctness**: `findBlockBodyForSelector` uses bare `indexOf` and can match substrings
  `':root '` could match `:root.preview ` in future. Anchor with `/(^|\})\s*body\s*\{/`-style regex.

- 🔵 **Correctness**: First-tag regex in `boot-theme.html.test.ts` skips past leading HTML comments
  `<!--` doesn't match `[a-zA-Z]` so the regex finds the next match, masking comment-prefix injections.

- 🔵 **Standards**: Future-guidance comment uses `prefers-reduced-motion: no-preference` (inverse of codebase precedent)
  `OriginPill.module.css:19` and `SseIndicator.module.css:19` use `prefers-reduced-motion: reduce`. Update the rationale comment to match.

- 🔵 **Compatibility**: `fakeStorage` stub in `boot-theme.test.ts` uses `k in items` exposing Object.prototype keys
  `getItem('toString')` returns `Object.prototype.toString` rather than null. Use `Object.create(null)` or `hasOwnProperty.call`.

- 🔵 **Usability**: Phase 7 manual-verification step (line ~2102) still references the old `M`-glyph design
  Stale text from prior design — implementer reading the checklist will be confused. Update to describe the new `Aa`(Inter) ↔ `Aa`(Fira Code) preview.

- 🔵 **Usability**: Symmetric `Aa`-`Aa` preview may not be self-evident at 48px button size
  Inter and Fira Code's `Aa` silhouettes differ subtly; users without typography training may not notice. Consider weight/size emphasis or accept as a minor papercut given aria-pressed and immediate page-wide feedback compensate.

#### Suggestions

- 🔵 **Standards**: `safe-storage.ts` location — `src/api/` convention is hooks/external bindings, not pure utilities. Either rename to `storage.ts` (binding to Web Storage API) or move to `src/lib/`. (Rename probably unnecessary — Web Storage is an external API, so `api/` semantics actually fit.)

- 🔵 **Code Quality**: Hook duplication remains acknowledged-but-deferred — flag explicitly to next plan author that the trigger is "next consumer, no exceptions".

- 🔵 **Code Quality**: `toggleTheme` could route through `setTheme` to collapse persistence to one call site.

- 🔵 **Code Quality**: Precedence test cases mirror across the two hook test files (parallel to the production duplication).

- 🔵 **Test Coverage**: bootThemePlugin regex injection has untested edge cases (no `<head>`, `<HEAD>` casing, comments before head). Lower priority since build-output test catches gross failures.

### Assessment

The plan is **substantially improved**: 41 of the 51 prior findings (80%) are resolved, including all 16 prior majors. The Phase 5 rewrite into a shared `boot-theme.ts` module + Vite plugin, the safe-storage extraction, the brace-balanced CSS extractors, the precedence/race tests, the dropped factory, the dropped `suppressHydrationWarning`, the no-write-on-first-visit semantics, the aria-pressed addition, the dropped `title` attribute, and the symmetric Aa-glyph preview together address the bulk of the prior critique cleanly.

However, the boot-theme.ts refactor traded one duplication problem for another: `applyBootAttributes` (testable, never runs in production) and `BOOT_SCRIPT_SOURCE` (runs in production, never tested) are now parallel implementations and three lenses (Code Quality, Test Coverage, Correctness) converged on this as a major concern. Three other newly-introduced majors are mechanical to fix (Vite defineConfig import regression; missing script-body assertion in the html test; aria-pressed + action-label APG conflict). Two prior majors that were not addressed in pass 1 also remain (TopbarIconButton not extracted; `safeSetItem` inside state updater violates React purity; useTheme/useThemeContext naming collision).

A second revision pass focusing on these seven majors would likely flip the verdict to APPROVE. The remaining minors are predominantly carry-forwards or genuinely deferred items already documented under "What We're NOT Doing".

## Re-Review (Pass 3) — 2026-05-08

**Verdict:** COMMENT (all 8 majors from pass 2 resolved; only minor polish items remain)

### Previously Identified Issues

**Resolved (8 of 8 majors from pass 2):**
- ✅ **Code Quality / Test Coverage / Correctness**: `applyBootAttributes` ↔ `BOOT_SCRIPT_SOURCE` parallel implementations — Resolved via parity suite that evaluates `BOOT_SCRIPT_SOURCE` through `new Function('document','localStorage', BOOT_SCRIPT_SOURCE)` against the same scenarios as `applyBootAttributes`. Drift between the two now triggers a parity test failure.
- ✅ **Test Coverage**: `boot-theme.html.test.ts` doesn't verify script body — Resolved with `expect(html).toContain(BOOT_SCRIPT_SOURCE)` plus a `stripLeadingCommentsAndWhitespace` helper that defends the first-tag regex against future plugin-injected comments.
- ✅ **Compatibility**: Vite config `defineConfig` import regression — Resolved with explicit instruction to keep `defineConfig` from `vitest/config` and import only `type Plugin` from `vite`. Rationale comment added.
- ✅ **Standards / Correctness / Usability**: aria-pressed + action-describing aria-label conflict — Resolved by switching to function-describing aria-labels (`"Dark theme"`, `"Mono font"`); aria-pressed retained. All tests updated.
- ✅ **Correctness**: `safeSetItem` inside state updater — Resolved: `toggleTheme` and `toggleFontMode` now route through `setTheme(next)` / `setFontMode(next)` with the next value computed from the current closure-captured state, eliminating the React purity violation and double-write under StrictMode.
- ✅ **Code Quality**: TopbarIconButton not extracted — Resolved with a new Phase 6.0 introducing `src/components/TopbarIconButton/` (component + CSS module + 5-case test). `ThemeToggle` and `FontModeToggle` collapsed to their state-mapping logic; their CSS modules contain only glyph-specific styles.
- ✅ **Usability**: `useTheme` / `useThemeContext` naming collision — Resolved with explicit JSDoc on both hooks marking owning vs consumer (with explicit "DO NOT call from leaf components" warning). Codebase-pattern parity (use-doc-events.ts) preserved.
- ✅ **Test Coverage**: Topbar 'integration' test never click-throughs — Resolved with two new `fireEvent.click` tests asserting `toggleTheme` and `toggleFontMode` mocks are invoked through real `ThemeToggle` / `FontModeToggle` components.

**Still present (carried forward, all minor or suggestion):**
- 🔵 **Architecture**: Provider nesting order, Topbar→Toggle coupling, hooks reach to documentElement — all suggestions, deferred deliberately
- 🔵 **Code Quality**: Hook implementation duplication acknowledged under "NOT Doing"
- 🔵 **Code Quality**: `extractBlockBody` copy-pasted into `migration.test.ts`
- 🔵 **Test Coverage**: No luminance/contrast assertion on dark mode
- 🔵 **Test Coverage**: FOUC verification still manual-only
- 🔵 **Test Coverage**: bootThemePlugin edge cases untested
- 🔵 **Correctness**: `countTopLevelBodyRules` regex handles only one nesting level
- 🔵 **Correctness**: `findBlockBodyForSelector` uses `indexOf` substring matching
- 🔵 **Correctness**: useEffect can clobber external attribute writes
- 🔵 **Compatibility**: `useState` lazy initialiser reads `document` (SSR forward-compat)
- 🔵 **Compatibility**: `fakeStorage` `in` operator exposes Object.prototype keys
- 🔵 **Usability**: Stale `M (Fira Code)` reference in Phase 7 manual verification — **fixed in pass 3** as a follow-up to the agent finding (now reads "the icon glyph remains `Aa` in both modes...")
- 🔵 **Usability**: No keyboard shortcut, no reset-to-OS-follow mechanism

### New Issues Introduced (all minor or suggestion)

- 🔵 **Code Quality**: `BootDeps.matchPrefersDark` is declared and supplied but never read by `applyBootAttributes` — vestigial dependency. Suggest dropping from the interface and from test invocations.
- 🔵 **Standards**: `TopbarIconButtonProps` uses camelCase `ariaLabel` / `ariaPressed` instead of the codebase-wide JSX-native dash form `aria-label` / `aria-pressed`. Suggest renaming the prop interface to use quoted `'aria-label'` / `'aria-pressed'` (or `Pick<ButtonHTMLAttributes, ...>`) for convention parity.
- 🔵 **Correctness**: `import { BOOT_SCRIPT_SOURCE } from './boot-theme'` in `boot-theme.test.ts` is placed mid-file (between describe blocks). ES modules hoist imports, so the code works, but a linter with `import/first` would flag it. Suggest combining with the existing `applyBootAttributes` import at the top.
- 🔵 **Test Coverage**: Click-through tests in Topbar duplicate `mountTopbar` setup. Suggest extending `mountTopbar` to accept `overrides` so future click-through cases are 3 lines.
- 🔵 **Test Coverage**: `applyBootAttributes` is exported but never imported by production code (only `BOOT_SCRIPT_SOURCE` is injected). Suggest a JSDoc note clarifying it's the canonical specification for parity testing.
- 🔵 **Standards / Usability**: aria-label `"Mono font"` uses informal abbreviation; `"Monospace font"` would parallel `"Dark theme"` and be clearer for screen-reader users without typography fluency.

### Assessment

**The plan is ready to ship.** All 8 majors flagged in pass 2 are resolved. The 17 remaining items are split between carry-forward deferrals already documented under "What We're NOT Doing" or accepted minor scope (no luminance assertion, no keyboard shortcut, no SSR forward-compat) and small polish items (vestigial `matchPrefersDark` field, prop-name camelCase divergence, mid-file import placement) that can either be addressed inline during implementation or rolled into a follow-up.

The Phase 5 boot-theme architecture has materially improved across two revisions: the original brittle regex-only test was replaced with `applyBootAttributes` behavioural tests, then with a parity suite that actually evaluates `BOOT_SCRIPT_SOURCE` to close the dual-implementation drift gap. The Phase 6.0 `TopbarIconButton` extraction cleanly removed ~30 lines of duplicated CSS shell across the two toggles and the new `JSDoc`-as-contract pattern on the owning/consumer hooks — while not preventing the trap structurally — makes the misuse self-documenting at the import site.

The verdict downgrade from REVISE → COMMENT reflects that no major findings remain. Implementation can proceed; the polish items can be picked up opportunistically.
