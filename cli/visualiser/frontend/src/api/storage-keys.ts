export const THEME_STORAGE_KEY = "ac-theme";
export const FONT_MODE_STORAGE_KEY = "ac-font-mode";
export const SEEN_DOC_TYPES_STORAGE_KEY = "ac-seen-doc-types";

// Prior app route to restore when leaving the DevDesignSystem (`/dev`) page.
// SESSION-scoped (per-tab) so a fresh window's cold-load deep-link never
// restores another tab's or session's route — see use-dev-activation.ts.
export const DEV_PRIOR_PATH_STORAGE_KEY = "ac-dev-prior-path";

/**
 * Inlined boot script sourced by the Vite bootThemePlugin. A classic
 * (non-module) IIFE that reads localStorage before React mounts so
 * attributes are set before the first paint (FOUC prevention).
 * Strings are derived from the canonical storage-key constants above
 * so a key rename is mechanically safe.
 */
export const BOOT_SCRIPT_SOURCE = `(function(){
var d=document.documentElement;
try{var t=localStorage.getItem(${JSON.stringify(THEME_STORAGE_KEY)});
if(t==='light'||t==='dark')d.setAttribute('data-theme',t)}catch(e){}
try{var f=localStorage.getItem(${JSON.stringify(FONT_MODE_STORAGE_KEY)});
if(f==='display'||f==='mono')d.setAttribute('data-font',f)}catch(e){}
})()`;
