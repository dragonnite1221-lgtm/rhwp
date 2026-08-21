const GRANT_PREFIX = 'fetch-grant:';
const GRANT_TTL_MS = 5 * 60 * 1000;
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
  const selected = storage?.session || storage?.local;
  if (!selected) throw new Error('grant 저장소를 사용할 수 없음');
  return selected;
}

async function pruneExpiredGrants(storage, now) {
  const entries = await storage.get(null);
  for (const [key, value] of Object.entries(entries || {})) {
    const expired = !value
      || !Number.isSafeInteger(value.expiresAt)
      || value.expiresAt <= now;
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
  if (!stored
      || stored.urlDigest !== expectedDigest
      || !Number.isSafeInteger(stored.expiresAt)
      || stored.expiresAt <= Date.now()) {
    if (stored) await storage.remove(key);
    return false;
  }
  return true;
}

export { GRANT_TTL_MS };
