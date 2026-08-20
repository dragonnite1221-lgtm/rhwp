import test from 'node:test';
import assert from 'node:assert/strict';

import { fetchPublicResource } from './secure-fetch.js';

const publicDns = async () => ['93.184.216.34'];

test('rejects literal and DNS-resolved private hosts before fetch', async () => {
  let fetchCalls = 0;
  const fetchImpl = async () => {
    fetchCalls += 1;
    return new Response('unexpected');
  };
  await assert.rejects(
    fetchPublicResource('http://127.0.0.1/a.hwp', { fetchImpl, dnsResolver: publicDns }),
    /차단/,
  );
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      fetchImpl,
      dnsResolver: async () => ['169.254.169.254'],
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
    fetchPublicResource('https://example.com/a.hwp', { fetchImpl, dnsResolver: publicDns }),
    /redirect rejected/,
  );
  assert.equal(fetchCalls, 1);
});

test('enforces declared and streamed byte limits', async () => {
  await assert.rejects(
    fetchPublicResource('https://example.com/a.hwp', {
      maxBytes: 4,
      dnsResolver: publicDns,
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
      fetchImpl: async () => new Response(stream),
    }),
    /실제 응답 크기 제한 초과/,
  );
});

test('returns bounded bytes for an approved public response', async () => {
  const result = await fetchPublicResource('https://example.com/a.hwp', {
    maxBytes: 16,
    dnsResolver: publicDns,
    fetchImpl: async () => new Response(new Uint8Array([1, 2, 3]), {
      headers: { 'content-type': 'application/octet-stream' },
    }),
  });
  assert.deepEqual(Array.from(result.data), [1, 2, 3]);
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
    fetchImpl: async () => new Response(new Uint8Array([7])),
  });
  assert.deepEqual(Array.from(result.data), [7]);
  assert.equal(dnsCalls, 0);
});
