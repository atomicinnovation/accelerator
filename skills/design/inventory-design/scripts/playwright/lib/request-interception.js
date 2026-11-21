// Every request hop, redirects included, through CDP's Fetch domain.
//
// Playwright's page.route sees only the first hop of a redirect chain and
// carries its header overrides onto the hops after it, so through it a redirect
// could reach a refused host unjudged and carry the bearer to another origin.
// The Fetch domain pauses each hop on its own, and an override set on one hop
// does not extend to the next.

import { classifyNavigationRequest } from './access-policy.js';
import { applyHeader, shouldAttachHeader } from './auth-header.js';

const NAVIGATION_RESOURCE_TYPE = 'Document';

function asPlaywrightRequest({ request, resourceType }) {
  return {
    url: () => request.url,
    isNavigationRequest: () => resourceType === NAVIGATION_RESOURCE_TYPE,
  };
}

function navigationDecision(event, getAllowances) {
  try {
    return classifyNavigationRequest(
      asPlaywrightRequest(event),
      getAllowances()
    );
  } catch {
    return { abort: true, classification: 'malformed', url: event.request.url };
  }
}

function lowercased(headers) {
  return Object.fromEntries(
    Object.entries(headers).map(([name, value]) => [name.toLowerCase(), value])
  );
}

function headersFor(request, getExpectedOrigin, getAuthHeader) {
  const header = getAuthHeader() ?? null;
  const attach =
    !!header && shouldAttachHeader(request.url, getExpectedOrigin());
  return Object.entries(
    applyHeader(lowercased(request.headers), attach, header)
  ).map(([name, value]) => ({ name, value }));
}

export function makeRequestInterceptor({
  mainFrameId,
  getAllowances,
  getExpectedOrigin,
  getAuthHeader,
  onMainFrameRefusal,
}) {
  return async (event, send) => {
    const { requestId } = event;
    const decision = navigationDecision(event, getAllowances);
    if (decision.abort) {
      if (event.frameId === mainFrameId) onMainFrameRefusal(decision);
      return send('Fetch.failRequest', {
        requestId,
        errorReason: 'BlockedByClient',
      });
    }
    return send('Fetch.continueRequest', {
      requestId,
      headers: headersFor(event.request, getExpectedOrigin, getAuthHeader),
    });
  };
}

export async function interceptRequests(page, options) {
  const cdp = await page.context().newCDPSession(page);
  const { frameTree } = await cdp.send('Page.getFrameTree');
  const intercept = makeRequestInterceptor({
    ...options,
    mainFrameId: frameTree.frame.id,
  });
  const send = (method, params) => cdp.send(method, params);
  cdp.on('Fetch.requestPaused', event => {
    intercept(event, send).catch(error => {
      console.error(
        `request interception error: ${error?.message ?? String(error)}`
      );
    });
  });
  await cdp.send('Fetch.enable', {
    patterns: [{ urlPattern: '*', requestStage: 'Request' }],
  });
}
