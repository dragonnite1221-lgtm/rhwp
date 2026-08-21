const GRANT_PREFIX = 'fetch-grant:';
const GRANT_TTL_MS = 5 * 60 * 1000;

function grantKey(token) {
  return `${GRANT_PREFIX}${token}`;
}

function sessionStorage() {
  const storage = globalThis.browser?.storage?.session || globalThis.chrome?.storage?.session;
  if (!storage) throw new Error('세션 저장소를 사용할 수 없음');
  return storage;
}

export async function createFetchGrant(url) {
  const canonicalUrl = new URL(url).href;
  const token = crypto.randomUUID();
  await sessionStorage().set({
    [grantKey(token)]: {
      url: canonicalUrl,
      expiresAt: Date.now() + GRANT_TTL_MS,
    },
  });
  return token;
}

export async function validateFetchGrant(token, url) {
  if (typeof token !== 'string' || token.length > 128) return false;
  let canonicalUrl;
  try {
    canonicalUrl = new URL(url).href;
  } catch {
    return false;
  }
  const key = grantKey(token);
  const storage = sessionStorage();
  const stored = (await storage.get(key))[key];
  if (!stored || stored.url !== canonicalUrl || stored.expiresAt < Date.now()) {
    if (stored) await storage.remove(key);
    return false;
  }
  return true;
}

export { GRANT_TTL_MS };
