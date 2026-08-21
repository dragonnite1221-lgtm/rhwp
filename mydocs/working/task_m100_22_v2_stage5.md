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
- `rhwp-shared/sw/fetch-grants.js` issues exact-URL capabilities backed by local
  storage containing only a SHA-256 URL digest and bounded expiry metadata. A
  five-minute active lease can be renewed by the same unguessable viewer-URL
  capability for up to seven days, so refresh and browser tab restore work
  without persisting the document URL indefinitely.
- Fetched bytes cross the background/viewer boundary through an extension-origin
  IndexedDB record, not a runtime message. Messages carry only a random one-time
  transfer ID; records expire after two minutes. Active records are never evicted
  before consumption; an atomic 128 MiB byte-accounting gate rejects new work
  when capacity is exhausted. This avoids Chrome's JSON message serialization corrupting
  `ArrayBuffer` responses or forcing a large base64/number-array expansion.
- Firefox no longer queues automatic thumbnail prefetches. Privileged document
  and thumbnail paths require a browser-trusted event and validated extension
  sender/tab/frame metadata. User-selected public CDN/object-store targets are
  accepted after the same public-network checks as same-origin targets.
- Firefox host permissions were narrowed from `<all_urls>` to HTTP(S), and its
  background listener returns Promises so policy failures propagate closed.
- Safari now uses its supported non-persistent background script event page,
  bundling the shared validators, grant,
  bounded fetch, and thumbnail parser. Its former local/private-network bypass
  was removed; preferences state that private and local destinations are always
  blocked and cap document size at 64 MiB.
- Safari's iOS overlay asks the validated background sender path to prepare the
  grant-bearing internal viewer URL instead of constructing an ungranted iframe.
- Public `@rhwp/editor` embeds use a 256-bit fragment capability bound to the
  direct parent `WindowProxy` and exact response origin. This preserves arbitrary
  consumer origins without restoring wildcard RPC trust; the Studio scrubs the
  capability from its visible location after startup.
- The workflow policy scans both `.yml` and `.yaml`, detects single-line and
  multiline network-to-shell pipelines, and runs synthetic bypass tests. CI now
  executes the browser-extension, Safari, Studio, and npm editor security suites.
- Safari's build bundles the shared module graph into one background resource,
  preserving the existing Xcode project resource list.
- The privileged fetch path observes `webRequest.onResponseStarted` and verifies
  the actual connected socket IP before reading any response body. Requests for
  the same canonical URL are serialized, and the event must originate from the
  extension, closing the DNS-rebinding gap between the DoH precheck and browser
  connection while retaining cross-origin user-selected documents.
- Chrome and Firefox use Vite 8.2.2. Studio's direct build dependencies were
  updated to their compatible current versions; all three npm trees now audit
  clean.
- The fork's remote Dependabot inventory exposed a remaining high-severity
  `fast-uri` advisory in `rhwp-vscode`. Its lockfile now resolves 3.1.5; the
  VS Code extension compiles successfully and its npm audit also reports zero.

## Verification

- Chrome: 60 security tests passed, production build passed, `npm audit` found
  zero vulnerabilities.
- Firefox: 60 security tests passed, production build passed, `npm audit` found
  zero vulnerabilities.
- Safari: 9 sender/target/grant/signature/manifest/trusted-event tests passed;
  shell syntax passed; the production Rolldown command produced a self-contained
  45,689-byte non-module IIFE background bundle that passed syntax validation.
- Studio: TypeScript and production build passed, `npm audit` found zero
  vulnerabilities, and the headless-browser postMessage E2E passed forged-origin
  rejection, same-origin request/response, a real cross-origin `@rhwp/editor`
  capability handshake, exact byte-view preservation, one-time consumption,
  invalid-ID rejection, and concurrent active-transfer retention assertions.
- VS Code extension: production Webpack compile passed and `npm audit` found
  zero vulnerabilities.
- Rust 1.98: `cargo test` passed the 1,230-test main suite (2 ignored) and every
  subsequently executed integration suite; the CI-equivalent `cargo clippy --
  -D warnings` passed. The first remote CI run exposed 12 lints newly enabled by
  the floating `stable` toolchain. They were fixed with meaning-preserving slice
  fill, fixed-width chunk, and expression initialization refactors. CI and WASM
  jobs now pin Rust 1.98.0 and include it in cache keys so a future stable release
  cannot silently change the gate.
- The actual dependency/lockfile minimum was measured: 1.75 cannot read lockfile
  v4, 1.85 is rejected by current `image`/`zip`, and a pristine Rust 1.88
  checkout resolves and passes `cargo check`. `Cargo.toml`, Korean/English
  onboarding docs, and a dedicated CI MSRV job now declare and continuously
  enforce 1.88. The root lockfile is intentionally untracked for this library,
  so the MSRV gate tests the publishable dependency-resolution state.
- The broader `cargo clippy --all-targets --all-features -- -D warnings` audit
  still exposes pre-existing test-only warnings outside the CI target set; those
  are recorded separately rather than hidden by weakening the production gate.
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

The PR review first reported eight actionable compatibility/policy gaps: Safari 15
grant storage, the iOS overlay grant, public iframe consumers, `.yaml` coverage,
multiline network-to-shell detection, fragment comparison, redirected download
reuse, and the stale Rust 1.75 requirement. All eight were fixed with regression
tests and explicit compatibility gates. The extension/Safari, Studio/editor,
workflow policy, download final-URL, and Rust MSRV remediation diffs were independently re-reviewed by the exact
`gemini-3.7-flash` endpoint and each returned `NO_ISSUES`.

A later review reported six more gaps: cross-origin document compatibility,
DNS rebinding between precheck and fetch, premature transfer eviction, viewer
grant refresh/restore, the Vite 8 Node floor, and Safari's background manifest
format. These were addressed with actual socket-IP verification, lifecycle-aware
byte admission, bounded renewable capabilities, Node 22.12 engines/docs, and a
Safari-compatible non-module event-page bundle. Each remediation lane is tested
and re-reviewed before merge.

The server-wide `codex_second_review_gate.py` was also invoked against the
pre-edit snapshot. It returned `blocked` because its independent fallback model
provider was unavailable (`fallback_model_unavailable`), not because it emitted
a code finding. This infrastructure result is kept distinct from the completed
direct `gemini-3.7-flash` reviews above.
