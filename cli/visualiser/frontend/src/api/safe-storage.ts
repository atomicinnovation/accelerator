export function safeGetItem(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function safeSetItem(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* private-browsing mode etc. — fall through silently */
  }
}

// Session-scoped (per-tab) variants. Same exception-swallowing contract as the
// localStorage helpers above; used for the DevDesignSystem exit-target prior
// path, whose correct lifetime is the tab, not the origin.
export function safeSessionGetItem(key: string): string | null {
  try {
    return sessionStorage.getItem(key);
  } catch {
    return null;
  }
}

export function safeSessionSetItem(key: string, value: string): void {
  try {
    sessionStorage.setItem(key, value);
  } catch {
    /* private-browsing mode etc. — fall through silently */
  }
}
