import { test } from 'node:test';
import assert from 'node:assert/strict';

import { makeRequestInterceptor } from './request-interception.js';

const MAIN_FRAME = 'main-frame';
const SUB_FRAME = 'sub-frame';
const LOCATION_ORIGIN = 'https://app.example.com';
const BEARER = { name: 'Authorization', value: 'Bearer abc123' };

function hop({
  url,
  resourceType = 'Document',
  frameId = MAIN_FRAME,
  headers = {},
  redirectedRequestId,
}) {
  return {
    requestId: 'interception-1',
    request: { url, headers },
    resourceType,
    frameId,
    ...(redirectedRequestId ? { redirectedRequestId } : {}),
  };
}

function interceptor({
  allowances = { allowInternal: false, allowInsecureScheme: false },
  expectedOrigin = null,
  authHeader = null,
  getAllowances = () => allowances,
  refusedMethod = null,
} = {}) {
  const refusals = [];
  const sent = [];
  const intercept = makeRequestInterceptor({
    mainFrameId: MAIN_FRAME,
    getAllowances,
    getExpectedOrigin: () => expectedOrigin,
    getAuthHeader: () => authHeader,
    onMainFrameRefusal: refusal => refusals.push(refusal),
  });
  const refusal = new Error(`${refusedMethod} refused`);
  const send = async (method, params) => {
    sent.push({ method, params });
    if (method === refusedMethod) throw refusal;
  };
  return {
    refusals,
    sent,
    refusal,
    pause: event => intercept(event, send),
  };
}

function headersSent(sent) {
  const [{ method, params }] = sent;
  assert.equal(method, 'Fetch.continueRequest');
  return Object.fromEntries(
    params.headers.map(({ name, value }) => [name, value])
  );
}

test('a main-frame navigation to a link-local host is failed and recorded', async () => {
  const { pause, sent, refusals } = interceptor();

  await pause(hop({ url: 'http://169.254.169.254/' }));

  assert.deepEqual(sent, [
    {
      method: 'Fetch.failRequest',
      params: { requestId: 'interception-1', errorReason: 'BlockedByClient' },
    },
  ]);
  assert.equal(refusals.length, 1);
  assert.equal(refusals[0].classification, 'link-local');
  assert.equal(refusals[0].url, 'http://169.254.169.254/');
});

test('a redirect hop is judged on its own, not by the hop that led to it', async () => {
  const { pause, sent, refusals } = interceptor();

  await pause(hop({ url: `${LOCATION_ORIGIN}/start` }));
  await pause(
    hop({
      url: 'http://169.254.169.254/',
      redirectedRequestId: 'interception-1',
    })
  );

  assert.deepEqual(
    sent.map(({ method }) => method),
    ['Fetch.continueRequest', 'Fetch.failRequest']
  );
  assert.equal(refusals.length, 1);
  assert.equal(refusals[0].classification, 'link-local');
});

test('a sub-frame navigation is failed without masking the main frame', async () => {
  const { pause, sent, refusals } = interceptor();

  await pause(hop({ url: 'http://169.254.169.254/', frameId: SUB_FRAME }));

  assert.equal(sent[0].method, 'Fetch.failRequest');
  assert.deepEqual(refusals, []);
});

test('allow_internal lets a link-local navigation continue', async () => {
  const { pause, sent, refusals } = interceptor({
    allowances: { allowInternal: true, allowInsecureScheme: false },
  });

  await pause(hop({ url: 'http://169.254.169.254/' }));

  assert.equal(sent[0].method, 'Fetch.continueRequest');
  assert.deepEqual(refusals, []);
});

test('a subresource is not judged as a navigation', async () => {
  const { pause, sent } = interceptor();

  await pause(
    hop({ url: 'http://169.254.169.254/x.png', resourceType: 'Image' })
  );

  assert.equal(sent[0].method, 'Fetch.continueRequest');
});

test('a classifier that throws fails the navigation closed as malformed', async () => {
  const { pause, sent, refusals } = interceptor({
    getAllowances: () => {
      throw new Error('boom');
    },
  });

  await pause(hop({ url: 'https://app.example.com/' }));

  assert.equal(sent[0].method, 'Fetch.failRequest');
  assert.equal(refusals[0].classification, 'malformed');
});

test('the header is attached to a hop on the location origin', async () => {
  const { pause, sent } = interceptor({
    expectedOrigin: LOCATION_ORIGIN,
    authHeader: BEARER,
  });

  await pause(hop({ url: 'https://app.example.com/api' }));

  assert.equal(headersSent(sent).authorization, 'Bearer abc123');
});

test('the header is stripped from a hop off the location origin, whatever its case', async () => {
  const { pause, sent } = interceptor({
    expectedOrigin: LOCATION_ORIGIN,
    authHeader: BEARER,
  });

  await pause(
    hop({
      url: 'https://cdn.other.com/asset.js',
      resourceType: 'Script',
      headers: { Authorization: 'Bearer abc123', Accept: '*/*' },
    })
  );

  assert.deepEqual(headersSent(sent), { accept: '*/*' });
});

test('a redirect hop off the location origin carries no header', async () => {
  const { pause, sent } = interceptor({
    expectedOrigin: LOCATION_ORIGIN,
    authHeader: BEARER,
  });

  await pause(
    hop({
      url: 'https://elsewhere.example.net/',
      redirectedRequestId: 'interception-0',
    })
  );

  assert.ok(!('authorization' in headersSent(sent)));
});

test('a hop continues with its own headers when no header is configured', async () => {
  const { pause, sent } = interceptor({ expectedOrigin: LOCATION_ORIGIN });

  await pause(
    hop({
      url: 'https://app.example.com/',
      headers: { Authorization: 'pre-existing' },
    })
  );

  assert.deepEqual(headersSent(sent), { authorization: 'pre-existing' });
});

test('a hop whose continuation is refused is failed rather than left paused', async () => {
  const { pause, sent, refusal } = interceptor({
    expectedOrigin: LOCATION_ORIGIN,
    authHeader: BEARER,
    refusedMethod: 'Fetch.continueRequest',
  });

  await assert.rejects(pause(hop({ url: 'https://app.example.com/' })), refusal);

  assert.deepEqual(
    sent.map(({ method }) => method),
    ['Fetch.continueRequest', 'Fetch.failRequest']
  );
  assert.deepEqual(sent[1].params, {
    requestId: 'interception-1',
    errorReason: 'Failed',
  });
});
