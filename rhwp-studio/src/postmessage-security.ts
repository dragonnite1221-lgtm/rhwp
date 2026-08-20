export interface TrustedMessageChannel {
  source: Window;
  origin: string;
}

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

/**
 * Accept only this window or its direct parent. A configured origin alone is
 * insufficient: source-window identity must also match to prevent sibling,
 * opener, and synthetic message sources from reaching document export APIs.
 */
export function trustedRpcChannel(
  event: MessageEvent,
  selfWindow: Window,
  allowedOrigins: ReadonlySet<string>,
): TrustedMessageChannel | null {
  if (!allowedOrigins.has(event.origin)) return null;

  const fromSelf = event.source === selfWindow
    && event.origin === selfWindow.location.origin;
  const fromDirectParent = selfWindow.parent !== selfWindow
    && event.source === selfWindow.parent;
  if (!fromSelf && !fromDirectParent) return null;

  return { source: event.source as Window, origin: event.origin };
}

export function postRpcResponse(
  channel: TrustedMessageChannel,
  payload: unknown,
): void {
  channel.source.postMessage(payload, channel.origin);
}
