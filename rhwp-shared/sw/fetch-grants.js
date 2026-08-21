const GRANT_PREFIX = 'fetch-grant:';
const GRANT_TTL_MS = 5 * 60 * 1000;
const GRANT_RENEWAL_TTL_MS = 7 * 24 * 60 * 60 * 1000;
const GRANT_TOKEN_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

function grantKey(token) {
  return `${GRANT_PREFIX}${token}`;
}

async function urlDigest(url) {
  const bytes = new TextEncoder().encode(url);
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, '0')).join('');
}

function grantStorage() {
  const storage = globalThis.browser?.storage || globalThis.chrome?.storage;
  // A viewer URL may be restored after a browser restart, so the exact-URL
  // capability record must outlive storage.session. Records contain only a
  // URL digest and are removed after the absolute renewal window.
  const selected = storage?.local;
  if (!selected) throw new Error('grant 저장소를 사용할 수 없음');
  return selected;
}

async function pruneExpiredGrants(storage, now) {
  const entries = await storage.get(null);
  for (const [key, value] of Object.entries(entries || {})) {
    const expired = !value
      || !Number.isSafeInteger(value.renewUntil)
      || value.renewUntil <= now;
    if (key.startsWith(GRANT_PREFIX) && expired) {
      await storage.remove(key);
    }
  }
}

export async function createFetchGrant(url) {
  const canonicalUrl = new URL(url).href;
  const token = crypto.randomUUID();
  const storage = grantStorage();
  const now = Date.now();
  await pruneExpiredGrants(storage, now);
  await storage.set({
    [grantKey(token)]: {
      urlDigest: await urlDigest(canonicalUrl),
      expiresAt: now + GRANT_TTL_MS,
      renewUntil: now + GRANT_RENEWAL_TTL_MS,
    },
  });
  return token;
}

export async function validateFetchGrant(token, url) {
  if (typeof token !== 'string' || !GRANT_TOKEN_PATTERN.test(token)) return false;
  let canonicalUrl;
  try {
    canonicalUrl = new URL(url).href;
  } catch {
    return false;
  }
  const key = grantKey(token);
  const storage = grantStorage();
  const stored = (await storage.get(key))[key];
  const expectedDigest = await urlDigest(canonicalUrl);
  const now = Date.now();
  if (!stored
      || stored.urlDigest !== expectedDigest
      || !Number.isSafeInteger(stored.expiresAt)
      || !Number.isSafeInteger(stored.renewUntil)
      || stored.renewUntil <= now) {
    if (stored) await storage.remove(key);
    return false;
  }
  if (stored.expiresAt <= now) {
    await storage.set({
      [key]: {
        ...stored,
        expiresAt: Math.min(now + GRANT_TTL_MS, stored.renewUntil),
      },
    });
  }
  return true;
}

export { GRANT_RENEWAL_TTL_MS, GRANT_TTL_MS };
