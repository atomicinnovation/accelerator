// Auth header injection handler factory.
// Installs a Playwright route() handler that adds an auth header only for
// requests matching the expected origin exactly (URL.origin comparison).

export function makeAuthHeaderHandler(page, { env = process.env } = {}) {
  const rawHeader = env.ACCELERATOR_BROWSER_AUTH_HEADER;
  const rawOrigin = env.ACCELERATOR_BROWSER_LOCATION_ORIGIN;

  if (!rawHeader || !rawOrigin) {
    // No-op when env vars not set
    return () => {};
  }

  const colonIdx = rawHeader.indexOf(':');
  if (colonIdx === -1) return () => {};
  const headerName = rawHeader.slice(0, colonIdx).trim();
  const headerValue = rawHeader.slice(colonIdx + 1).trim();

  let expectedOrigin;
  try {
    expectedOrigin = new URL(rawOrigin).origin;
  } catch {
    return () => {};
  }

  return async () => {
    await page.route('**/*', async (route) => {
      const requestUrl = route.request().url();
      let requestOrigin;
      try {
        requestOrigin = new URL(requestUrl).origin;
      } catch {
        await route.continue();
        return;
      }

      if (requestOrigin === expectedOrigin) {
        const headers = { ...await route.request().allHeaders(), [headerName]: headerValue };
        await route.continue({ headers });
      } else {
        const headers = { ...await route.request().allHeaders() };
        delete headers[headerName.toLowerCase()];
        await route.continue({ headers });
      }
    });
  };
}

// Pure function: determine whether a request URL should receive the auth header.
// Used for unit testing without a live browser.
export function shouldAttachHeader(requestUrl, expectedOrigin) {
  try {
    const reqOrigin = new URL(requestUrl).origin;
    const expOrigin = new URL(expectedOrigin).origin;
    return reqOrigin === expOrigin;
  } catch {
    return false;
  }
}
