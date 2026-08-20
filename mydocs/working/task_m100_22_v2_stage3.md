# M100 / #22 Stage 3 — Chrome network and message boundary

## Scope

- Remove page-driven automatic network activity.
- Authenticate extension message senders and bind privileged fetches to a
  short-lived document capability.
- Prevent requests to loopback, private, link-local, reserved, or DNS-rebound
  destinations, including through redirects.
- Bound request time and actual bytes read rather than trusting headers.

## Implementation

- Automatic thumbnail prefetch was removed. Badge, card, and hover actions now
  require browser-trusted user events.
- Content-script document/thumbnail requests are same-origin only, except for
  the exact GitHub blob-to-raw provider adapter. Cross-origin links remain
  available through Chrome's explicit context-menu gesture.
- Message sender checks require the current extension ID and distinguish
  extension pages from HTTP(S) content scripts by URL, tab, and frame identity.
- Viewer launches mint a random five-minute capability in `storage.session`.
  The viewer passes it back with the exact canonical URL; direct privileged
  `fetch(fileUrl)` from the extension page was removed.
- `secure-fetch.js` validates HTTP(S), credentials, host form, literal IPs,
  public DNS answers, rejects redirects, and enforces timeout, declared length, and
  streamed byte count. Documents are capped at 64 MiB and thumbnail source
  files at 32 MiB.
- DNS validation uses bounded Cloudflare DNS-over-HTTPS because Chrome's
  `chrome.dns` API is documented as Dev-channel-only and is not a stable
  extension runtime dependency.
- Manifest matching was narrowed from `<all_urls>` to HTTP(S), and the build now
  copies canonical shared security modules into a self-contained package.

## Verification

- `npm test` — 19 sender, origin, capability, IP/DNS, redirect, user-event, and
  byte-limit regression tests passed.
- Live bounded network smoke — public DNS resolution and a 559-byte HTTPS
  response completed through `fetchPublicResource`.
- `npm run build` in `rhwp-studio` — passed.
- `npm run build` in `rhwp-chrome` — passed and produced a self-contained dist.
- Packaged `dist/background.js` module graph loaded with mocked Chrome APIs;
  no source-tree imports or test files leaked into the extension package.
- Changed JavaScript files passed `node --check`; manifest JSON parsed; and
  `git diff --check` passed.

## Residual boundary

Browser fetch APIs do not expose a stable connection-IP binding primitive.
The remaining DNS time-of-check/time-of-use window is reduced by exact URL
capabilities, same-origin content-script policy, trusted user gestures, public
DNS validation, redirect rejection, and fail-closed
network errors. A future server relay could provide connection-level IP pinning
if arbitrary cross-origin automatic retrieval becomes a product requirement.
