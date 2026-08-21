export interface TrustedMessageChannel {
  source: Window;
  origin: string;
  rpcToken: string | null;
}

const RPC_TOKEN_PATTERN = /^[0-9a-f]{64}$/;

function normalizeHttpOrigin(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  try {
    const url = new URL(trimmed);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
    if (url.username || url.password || url.search || url.hash) return null;
    if (url.pathname !== '/' && url.pathname !== '') return null;
    return url.origin;
  } catch {
    return null;
  }
}

/**
 * Build the RPC origin allowlist. The Studio's own origin is always allowed;
 * cross-origin embedding requires an explicit build-time origin list.
 */
export function buildAllowedRpcOrigins(
  configuredOrigins: string | undefined,
  selfOrigin: string,
): ReadonlySet<string> {
  const origins = new Set<string>();
  const normalizedSelf = normalizeHttpOrigin(selfOrigin);
  if (normalizedSelf) origins.add(normalizedSelf);

  for (const value of (configuredOrigins ?? '').split(',')) {
    const normalized = normalizeHttpOrigin(value);
    if (normalized) origins.add(normalized);
  }
  return origins;
}

export function readRpcToken(hash: string): string | null {
  const params = new URLSearchParams(hash.replace(/^#/, ''));
  const token = params.get('rhwp-rpc-token');
  return token && RPC_TOKEN_PATTERN.test(token) ? token : null;
}

/**
 * Accept only this window or its direct parent. A configured origin alone is
 * insufficient: source-window identity must also match to prevent sibling,
 * opener, and synthetic message sources from reaching document export APIs.
 */
export function trustedRpcChannel(
  event: MessageEvent,
  selfWindow: Window,
  allowedOrigins: ReadonlySet<string>,
  rpcToken: string | null,
): TrustedMessageChannel | null {
  const fromSelf = event.source === selfWindow
    && event.origin === selfWindow.location.origin;
  const fromDirectParent = selfWindow.parent !== selfWindow
    && event.source === selfWindow.parent;
  if (!fromSelf && !fromDirectParent) return null;

  const suppliedToken = event.data && typeof event.data === 'object'
    ? event.data.rpcToken
    : null;
  const tokenAuthorized = fromDirectParent
    && rpcToken !== null
    && suppliedToken === rpcToken;
  if (!allowedOrigins.has(event.origin) && !tokenAuthorized) return null;

  return {
    source: event.source as Window,
    origin: event.origin,
    rpcToken: tokenAuthorized ? rpcToken : null,
  };
}

export function postRpcResponse(
  channel: TrustedMessageChannel,
  payload: unknown,
): void {
  const response = channel.rpcToken && payload && typeof payload === 'object'
    ? { ...payload, rpcToken: channel.rpcToken }
    : payload;
  channel.source.postMessage(response, channel.origin);
}
