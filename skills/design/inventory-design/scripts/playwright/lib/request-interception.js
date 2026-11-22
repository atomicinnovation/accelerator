// Every request hop on the page's own target, redirects included, through CDP's
// Fetch domain. Other targets go unjudged: the daemon keeps cross-site frames
// on this one by launching without site isolation, and blocks service workers.
//
// Playwright's page.route sees only the first hop of a redirect chain and
// carries its header overrides onto the hops after it, so through it a redirect
// could reach a refused host unjudged and carry the bearer to another origin.
// The Fetch domain pauses each hop on its own, and an override set on one hop
// does not extend to the next.
//
// This must be the page's only Fetch client. Playwright's own routing also
// runs on the Fetch domain, so any page or context routing beside it would
// pause a hop twice or let the wrong handler release it.

import { classifyNavigationRequest } from './access-policy.js';
import { applyHeader, shouldAttachHeader } from './auth-header.js';

const NAVIGATION_RESOURCE_TYPE = 'Document';

function navigationDecision({ request, resourceType }, getAllowances) {
  try {
    return classifyNavigationRequest(
      {
        url: request.url,
        isNavigation: resourceType === NAVIGATION_RESOURCE_TYPE,
      },
      getAllowances()
    );
  } catch {
    return { abort: true, classification: 'malformed', url: request.url };
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
    try {
      return await send('Fetch.continueRequest', {
        requestId,
        headers: headersFor(event.request, getExpectedOrigin, getAuthHeader),
      });
    } catch (refusal) {
      await failQuietly(send, requestId);
      throw refusal;
    }
  };
}

// The hop may already be gone, cancelled by the page, in which case Chromium
// refuses the failure too and there is nothing left to release.
async function failQuietly(send, requestId) {
  try {
    await send('Fetch.failRequest', { requestId, errorReason: 'Failed' });
  } catch {}
}

// A query string can carry a credential, so a logged hop keeps only its
// origin and path.
function hopName(url) {
  try {
    const { origin, pathname } = new URL(url);
    return `${origin}${pathname}`;
  } catch {
    return 'a malformed URL';
  }
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
        `request interception error on ${hopName(event.request.url)}: ` +
          `${error?.message ?? String(error)}`
      );
    });
  });
  await cdp.send('Fetch.enable', {
    patterns: [{ urlPattern: '*', requestStage: 'Request' }],
  });
}
