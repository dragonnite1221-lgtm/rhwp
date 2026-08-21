import {
  isIpAddressLiteral,
  validatePublicUrl,
  validateResolvedAddresses,
} from '../security/url-validator.js';

async function resolveDnsRecord(hostname, type, signal) {
  const endpoint = new URL('https://cloudflare-dns.com/dns-query');
  endpoint.searchParams.set('name', hostname);
  endpoint.searchParams.set('type', type);
  const response = await fetch(endpoint, {
    cache: 'no-store',
    credentials: 'omit',
    headers: { accept: 'application/dns-json' },
    redirect: 'error',
    referrerPolicy: 'no-referrer',
    signal,
  });
  if (!response.ok) throw new Error(`DNS-over-HTTPS 실패: HTTP ${response.status}`);
  const body = await readBoundedBody(response, 128 * 1024);
  const record = JSON.parse(new TextDecoder().decode(body));
  if (record.Status !== 0 && record.Status !== 3) {
    throw new Error(`DNS-over-HTTPS 실패: Status=${record.Status}`);
  }
  const expectedType = type === 'A' ? 1 : 28;
  return (record.Answer || [])
    .filter(answer => answer.type === expectedType && typeof answer.data === 'string')
    .map(answer => answer.data);
}

async function defaultDnsResolver(hostname, signal) {
  const results = await Promise.all([
    resolveDnsRecord(hostname, 'A', signal),
    resolveDnsRecord(hostname, 'AAAA', signal),
  ]);
  return results.flat();
}

async function assertPublicDestination(url, dnsResolver, signal) {
  const validation = validatePublicUrl(url);
  if (!validation.allowed) throw new Error(validation.reason);

  // URL fragments are client-side identifiers and are never sent on the wire.
  // Strip them before DNS/fetch/final-URL comparison so an approved fragment
  // cannot create a false redirect mismatch.
  validation.parsed.hash = '';

  const hostname = validation.parsed.hostname.replace(/^\[|\]$/g, '');
  const addresses = isIpAddressLiteral(hostname)
    ? [hostname]
    : await dnsResolver(hostname, signal);
  const resolved = validateResolvedAddresses(addresses);
  if (!resolved.allowed) throw new Error(resolved.reason);
  return validation.parsed;
}

async function readBoundedBody(response, maxBytes) {
  const contentLength = response.headers.get('content-length');
  if (contentLength && (!/^\d+$/.test(contentLength) || Number(contentLength) > maxBytes)) {
    throw new Error(`응답 크기 제한 초과: ${contentLength} > ${maxBytes}`);
  }
  if (!response.body) return new Uint8Array();

  const reader = response.body.getReader();
  const chunks = [];
  let total = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > maxBytes) {
        await reader.cancel('response size limit exceeded');
        throw new Error(`실제 응답 크기 제한 초과: ${total} > ${maxBytes}`);
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }

  const data = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    data.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return data;
}

/**
 * Fetch a public HTTP(S) resource with DNS, redirect, timeout, and byte limits.
 * Redirects are rejected because browser manual redirects expose neither the
 * status nor Location header needed to validate the next destination safely.
 */
export async function fetchPublicResource(url, options = {}) {
  const {
    maxBytes = 64 * 1024 * 1024,
    timeoutMs = 30_000,
    fetchImpl = globalThis.fetch,
    dnsResolver = defaultDnsResolver,
  } = options;
  if (!Number.isSafeInteger(maxBytes) || maxBytes <= 0) {
    throw new Error('maxBytes는 양의 안전한 정수여야 함');
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(new Error('요청 시간 제한 초과')), timeoutMs);
  try {
    const parsed = await assertPublicDestination(url, dnsResolver, controller.signal);
    const response = await fetchImpl(parsed.href, {
      cache: 'no-store',
      credentials: 'omit',
      redirect: 'error',
      referrerPolicy: 'no-referrer',
      signal: controller.signal,
    });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    if (response.url && new URL(response.url).href !== parsed.href) {
      throw new Error('검증되지 않은 최종 응답 URL 감지');
    }
    const data = await readBoundedBody(response, maxBytes);
    return {
      data,
      finalUrl: parsed.href,
      contentType: response.headers.get('content-type'),
    };
  } finally {
    clearTimeout(timeout);
  }
}

export { assertPublicDestination, readBoundedBody };
