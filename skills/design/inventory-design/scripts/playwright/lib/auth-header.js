// Auth-header route factory. The expected origin and the header are read per
// request through daemon-supplied getters, never from the environment.

export function makeAuthHeaderHandler(
  page,
  { getExpectedOrigin, getAuthHeader } = {}
) {
  return async () => {
    await page.route('**/*', async (route) => {
      try {
        const header = getAuthHeader?.() ?? null;
        const attach =
          !!header &&
          shouldAttachHeader(route.request().url(), getExpectedOrigin?.());
        const headers = applyHeader(
          await route.request().allHeaders(),
          attach,
          header
        );
        await route.continue({ headers });
      } catch (error) {
        const detail = error?.message ?? String(error);
        console.error(`auth-header route error: ${detail}`);
        try {
          await route.continue();
        } catch {}
      }
    });
  };
}

export function parseAuthHeader(raw) {
  if (!raw) return null;
  const colonIdx = raw.indexOf(':');
  if (colonIdx === -1) return null;
  const name = raw.slice(0, colonIdx).trim();
  const value = raw.slice(colonIdx + 1).trim();
  if (!name || !value) return null;
  return { name, value };
}

export function applyHeader(current, attach, header) {
  const headers = { ...current };
  if (!header) return headers;
  const key = header.name.toLowerCase();
  if (attach) {
    headers[key] = header.value;
  } else {
    delete headers[key];
  }
  return headers;
}

export function shouldAttachHeader(requestUrl, expectedOrigin) {
  if (!expectedOrigin) return false;
  try {
    return new URL(requestUrl).origin === expectedOrigin;
  } catch {
    return false;
  }
}
