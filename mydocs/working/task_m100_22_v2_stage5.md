# M100 / #22 Stage 5 — Cross-browser boundary parity and package hygiene

## Scope

- Extend the Chrome sender, capability, public-network, redirect, timeout, and
  actual-byte invariants to Firefox and Safari.
- Remove automatic document prefetch so a page cannot trigger privileged fetches
  without a trusted user action.
- Keep one canonical network policy implementation for all three extensions.
- Remove currently reported npm vulnerabilities without weakening the security
  controls added in the earlier stages.

## Implementation

- `rhwp-shared/sw/secure-fetch.js` now owns public HTTP(S) validation, public DNS
  resolution, redirect rejection, request timeout, declared-size checks, and
  streamed actual-byte ceilings. Chrome, Firefox, and Safari package that same
  implementation.
- `rhwp-shared/sw/fetch-grants.js` issues exact-URL, five-minute capabilities in
  extension session storage. Viewer fetches require a matching unexpired grant.
- Fetched bytes cross the background/viewer boundary through an extension-origin
  IndexedDB record, not a runtime message. Messages carry only a random one-time
  transfer ID; records expire after two minutes and the pending backlog is capped
  at two. This avoids Chrome's JSON message serialization corrupting
  `ArrayBuffer` responses or forcing a large base64/number-array expansion.
- Firefox no longer queues automatic thumbnail prefetches. Privileged document
  and thumbnail paths require a browser-trusted event, validated extension
  sender/tab/frame metadata, and a same-origin target or the exact GitHub
  blob-to-raw adapter.
- Firefox host permissions were narrowed from `<all_urls>` to HTTP(S), and its
  background listener returns Promises so policy failures propagate closed.
- Safari now uses a module service worker with the shared validators, grant,
  bounded fetch, and thumbnail parser. Its former local/private-network bypass
  was removed; preferences state that private and local destinations are always
  blocked and cap document size at 64 MiB.
- Safari's build bundles the shared module graph into one background resource,
  preserving the existing Xcode project resource list.
- Chrome and Firefox use Vite 8.2.2. Studio's direct build dependencies were
  updated to their compatible current versions; all three npm trees now audit
  clean.

## Verification

- Chrome: 27 security tests passed, production build passed, `npm audit` found
  zero vulnerabilities.
- Firefox: 27 security tests passed, production build passed, `npm audit` found
  zero vulnerabilities.
- Safari: 7 sender/target/grant/signature/manifest/trusted-event tests passed;
  shell syntax passed; the production Rolldown command produced a self-contained
  36,816-byte background bundle that loaded under mocked Safari extension APIs.
- Studio: TypeScript and production build passed, `npm audit` found zero
  vulnerabilities, and the headless-browser postMessage E2E passed forged-origin
  rejection, same-origin request/response, exact byte-view preservation,
  one-time consumption, invalid-ID rejection, and bounded-backlog assertions.
- Rust: `cargo test` passed the 1,230-test main suite (2 ignored) and every
  subsequently executed integration suite. `cargo clippy --all-targets
  --all-features -- -D warnings` remains red on 84 pre-existing warnings in
  untouched Rust tests and modules; Stage 5 changes contain no Rust files.
- `git diff --check` passed.

## Platform boundary

The current Linux host cannot run Apple's converter, Xcode, code signing, or a
real Safari extension load. The source tests and exact single-bundle production
step are verified here; an actual signed macOS/Safari build remains a platform
verification boundary and is not represented as completed.

## Review gate

The original full staged diff was split into shared/Chrome, Firefox, Safari,
dependency/configuration/documentation, and generated-lockfile lanes. The exact
`gemini-3.7-flash` endpoint returned `NO_ISSUES` for all five lanes. Generated
lockfiles were also independently validated by clean installs already present in
the worktree, successful builds, and zero-vulnerability npm audits. A whole-branch
rerun returned a malformed truncated response for the combined shared/Chrome
lane; it was rejected as approval and exposed the binary-message boundary above.
The fix was re-reviewed as separate store, Chrome adapter, Firefox/Safari,
Studio/E2E, and documentation lanes; every valid exact-model response was
`NO_ISSUES`. A second malformed combined-lane response was likewise discarded
and replaced by the smaller store and Chrome reviews rather than counted.
