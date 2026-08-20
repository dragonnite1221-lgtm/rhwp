# M100 / #22 Stage 2 — Studio message-channel boundary

## Scope

- Prevent arbitrary parent/opener windows from invoking document RPC methods.
- Stop broadcasting document exports with a wildcard response origin.
- Preserve supported same-origin RPC and explicit trusted embedding.

## Implementation

- Added `postmessage-security.ts` as the single trust-policy module.
- Requests are accepted only when both the exact HTTP(S) origin and the source
  window identity match.
- Same-window requests are restricted to Studio's own origin.
- Framed requests must come from the direct parent and from an origin listed at
  build time in `VITE_RHWP_ALLOWED_PARENT_ORIGINS`.
- Sibling frames, opener windows, missing sources, opaque origins, malformed
  configured values, and unlisted parents fail closed.
- Replies use the authenticated request origin as `targetOrigin`; wildcard
  response delivery was removed.
- The legacy HwpCtrl load route and all generic export methods share the same
  boundary rather than maintaining separate checks.

## Verification

- `npm run build` — TypeScript and production Vite/PWA build passed.
- `npm run e2e:postmessage-security` — forged origin and missing-source
  requests produced no response; same-origin `ready` RPC succeeded.
- `node e2e/export-hwpx.test.mjs --mode=headless` — existing HWP/HWPX export,
  round-trip, and verification RPC checks all passed.
- `node --check` on both changed E2E scripts — passed.
- `git diff --check` — passed.

## Residual boundary

The build-time parent-origin list is intentionally a deployment decision. With
no value configured, cross-origin embedding cannot invoke RPC. Browser-extension
fetch and parser hardening remain isolated in stages 3–4.
