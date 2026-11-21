// Auth-header decisions for one request hop. The expected origin and the header
// come from the daemon's per-request state, never from the environment.

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
