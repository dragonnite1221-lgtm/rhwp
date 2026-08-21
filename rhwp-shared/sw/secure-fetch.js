import {
  isIpAddressLiteral,
  validatePublicUrl,
  validateResolvedAddresses,
} from '../security/url-validator.js';

const networkUrlLocks = new Map();

async function withNetworkUrlLock(url, callback) {
  const previous = networkUrlLocks.get(url) || Promise.resolve();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const tail = previous.then(() => gate);
  networkUrlLocks.set(url, tail);
  await previous;
  try {
    return await callback();
  } finally {
    release();
    if (networkUrlLocks.get(url) === tail) networkUrlLocks.delete(url);
  }
}

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

function webRequestApi() {
  return globalThis.browser?.webRequest || globalThis.chrome?.webRequest || null;
}

function extensionBaseUrl() {
  const runtime = globalThis.browser?.runtime || globalThis.chrome?.runtime;
  return runtime?.getURL ? runtime.getURL('') : null;
}

function sameExtensionOrigin(value, extensionBase) {
  if (!value || !extensionBase) return false;
  try {
    const actual = new URL(value);
    const expected = new URL(extensionBase);
    return actual.protocol === expected.protocol && actual.hostname === expected.hostname;
  } catch {
    return false;
  }
}

function sameNetworkUrl(left, right) {
  try {
    const first = new URL(left);
    const second = new URL(right);
    first.hash = '';
    second.hash = '';
    return first.href === second.href;
  } catch {
    return false;
  }
}

/** Verify the IP address of the actual browser connection before reading its body. */
function verifyConnectedAddress(
  url,
  signal,
  api = webRequestApi(),
  extensionBase = extensionBaseUrl(),
) {
  if (!api?.onResponseStarted?.addListener || !api.onResponseStarted.removeListener) {
    return Promise.reject(new Error('실제 연결 주소 검증 API를 사용할 수 없음'));
  }
  return new Promise((resolve, reject) => {
    const cleanup = () => {
      api.onResponseStarted.removeListener(listener);
      signal.removeEventListener('abort', onAbort);
    };
    const onAbort = () => {
      cleanup();
      reject(signal.reason || new Error('요청 중단'));
    };
    const listener = (details) => {
      if (!sameNetworkUrl(details?.url, url)) return;
      if (!sameExtensionOrigin(details.initiator || details.originUrl, extensionBase)) return;
      const validation = validateResolvedAddresses([details.ip]);
      cleanup();
      if (validation.allowed) resolve(details.ip);
      else reject(new Error(`실제 연결 주소 차단: ${validation.reason}`));
    };
    signal.addEventListener('abort', onAbort, { once: true });
    api.onResponseStarted.addListener(listener, { urls: ['<all_urls>'] });
  });
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
    connectedAddressVerifier = verifyConnectedAddress,
  } = options;
  if (!Number.isSafeInteger(maxBytes) || maxBytes <= 0) {
    throw new Error('maxBytes는 양의 안전한 정수여야 함');
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(new Error('요청 시간 제한 초과')), timeoutMs);
  try {
    const parsed = await assertPublicDestination(url, dnsResolver, controller.signal);
    const response = await withNetworkUrlLock(parsed.href, async () => {
      const connectionVerification = connectedAddressVerifier(parsed.href, controller.signal);
      const responseRequest = fetchImpl(parsed.href, {
        cache: 'no-store',
        credentials: 'omit',
        redirect: 'error',
        referrerPolicy: 'no-referrer',
        signal: controller.signal,
      });
      const [verifiedResponse] = await Promise.all([responseRequest, connectionVerification]);
      return verifiedResponse;
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
    if (!controller.signal.aborted) controller.abort();
  }
}

export { assertPublicDestination, readBoundedBody, verifyConnectedAddress };
