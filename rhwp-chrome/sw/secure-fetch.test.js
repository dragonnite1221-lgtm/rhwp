import test from 'node:test';
import assert from 'node:assert/strict';

import { fetchPublicResource, verifyConnectedAddress } from './secure-fetch.js';

const publicDns = async () => ['93.184.216.34'];
const publicConnection = async () => '93.184.216.34';

test('rejects literal and DNS-resolved private hosts before fetch', async () => {
  let fetchCalls = 0;
  const fetchImpl = async () => {
    fetchCalls += 1;
    return new Response('unexpected');
  };
  await assert.rejects(
    fetchPublicResource('http://127.0.0.1/a.hwp', {
      fetchImpl, dnsResolver: publicDns, connectedAddressVerifier: publicConnection,
    }),
    /차단/,
  );
  await assert.rejects(
    fetchPublicResource('http://93.184.216.34/a.hwp', {
      fetchImpl, dnsResolver: publicDns, connectedAddressVerifier: publicConnection,
    }),
    /HTTPS만 허용/,
  );
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      fetchImpl,
      dnsResolver: async () => ['169.254.169.254'],
      connectedAddressVerifier: publicConnection,
    }),
    /DNS가/,
  );
  assert.equal(fetchCalls, 0);
});

test('rejects redirects instead of following an unvalidated destination', async () => {
  let fetchCalls = 0;
  const fetchImpl = async (_url, options) => {
    fetchCalls += 1;
    assert.equal(options.redirect, 'error');
    throw new TypeError('redirect rejected');
  };
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      fetchImpl, dnsResolver: publicDns, connectedAddressVerifier: publicConnection,
    }),
    /redirect rejected/,
  );
  assert.equal(fetchCalls, 1);
});

test('enforces declared and streamed byte limits', async () => {
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      maxBytes: 4,
      dnsResolver: publicDns,
      connectedAddressVerifier: publicConnection,
      fetchImpl: async () => new Response('12345', {
        headers: { 'content-length': '5' },
      }),
    }),
    /크기 제한 초과/,
  );

  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new Uint8Array([1, 2, 3]));
      controller.enqueue(new Uint8Array([4, 5, 6]));
      controller.close();
    },
  });
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      maxBytes: 4,
      dnsResolver: publicDns,
      connectedAddressVerifier: publicConnection,
      fetchImpl: async () => new Response(stream),
    }),
    /실제 응답 크기 제한 초과/,
  );
});

test('returns bounded bytes for an approved public response', async () => {
  const result = await fetchPublicResource('https://example.com/a.hwp', {
    maxBytes: 16,
    dnsResolver: publicDns,
    connectedAddressVerifier: publicConnection,
    fetchImpl: async () => new Response(new Uint8Array([1, 2, 3]), {
      headers: { 'content-type': 'application/octet-stream' },
    }),
  });
  assert.deepEqual(Array.from(result.data), [1, 2, 3]);
  assert.equal(result.finalUrl, 'https://example.com/a.hwp');
});

test('strips URL fragments before fetch and final-response comparison', async () => {
  let fetchedUrl;
  const result = await fetchPublicResource('https://example.com/a.hwp#section-2', {
    maxBytes: 16,
    dnsResolver: publicDns,
    connectedAddressVerifier: publicConnection,
    fetchImpl: async (url) => {
      fetchedUrl = url;
      return new Response(new Uint8Array([4]), {
        headers: { 'content-type': 'application/octet-stream' },
      });
    },
  });
  assert.equal(fetchedUrl, 'https://example.com/a.hwp');
  assert.equal(result.finalUrl, 'https://example.com/a.hwp');
});

test('public IP literals are classified locally without a DNS query', async () => {
  let dnsCalls = 0;
  const result = await fetchPublicResource('https://93.184.216.34/a.hwp', {
    maxBytes: 16,
    dnsResolver: async () => {
      dnsCalls += 1;
      return [];
    },
    connectedAddressVerifier: publicConnection,
    fetchImpl: async () => new Response(new Uint8Array([7])),
  });
  assert.deepEqual(Array.from(result.data), [7]);
  assert.equal(dnsCalls, 0);
});

test('rejects a private address observed on the actual browser connection', async () => {
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      dnsResolver: publicDns,
      connectedAddressVerifier: async () => {
        throw new Error('실제 연결 주소 차단: 192.168.1.10');
      },
      fetchImpl: async () => new Response(new Uint8Array([1, 2, 3])),
    }),
    /실제 연결 주소 차단/,
  );
});

test('browser connection verification uses the response socket IP', async () => {
  let listener;
  const event = {
    addListener(value) { listener = value; },
    removeListener(value) {
      if (listener === value) listener = undefined;
    },
  };
  const api = { onResponseStarted: event };
  const publicResult = verifyConnectedAddress(
    'https://example.com/a.hwp', new AbortController().signal, api,
    'chrome-extension://trusted/',
  );
  listener({
    url: 'https://example.com/a.hwp',
    ip: '93.184.216.34',
    initiator: 'chrome-extension://trusted',
  });
  assert.equal(await publicResult, '93.184.216.34');

  const privateResult = verifyConnectedAddress(
    'https://example.com/a.hwp', new AbortController().signal, api,
    'chrome-extension://trusted/',
  );
  listener({
    url: 'https://example.com/a.hwp',
    ip: '192.168.1.10',
    initiator: 'chrome-extension://trusted',
  });
  await assert.rejects(privateResult, /실제 연결 주소 차단/);
});

test('a pre-response fetch failure aborts connection verification cleanup', async () => {
  let verifierAborted = false;
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      dnsResolver: publicDns,
      connectedAddressVerifier: (_url, signal) => new Promise((resolve, reject) => {
        signal.addEventListener('abort', () => {
          verifierAborted = true;
          reject(new Error('verification aborted'));
        }, { once: true });
      }),
      fetchImpl: async () => { throw new Error('network failed before response'); },
    }),
    /network failed before response/,
  );
  assert.equal(verifierAborted, true);
});
