export const THEME_STORAGE_KEY = 'ac-theme'
export const FONT_MODE_STORAGE_KEY = 'ac-font-mode'
export const SEEN_DOC_TYPES_STORAGE_KEY = 'ac-seen-doc-types'

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
})()`
