# M100 / #22 Stage 1 — CI trust and supply-chain boundary

## Scope

- Restrict the Claude workflow to trusted repository participants.
- Remove unnecessary repository write and OIDC permissions.
- Pin every third-party GitHub Action to a full commit SHA.
- Replace network-to-shell `wasm-pack` installation with an immutable release
  archive and a verified SHA-256 digest.
- Prevent workflow input and publishing-token interpolation into shell source.
- Make the workflow security invariants executable in CI.

## Implementation

- `.github/workflows/claude.yml` now gates all supported events using
  `author_association` and runs with read-only contents permission.
- All eight workflow files use full 40-character action SHAs.
- `.github/scripts/install-wasm-pack.sh` installs wasm-pack 0.13.1 from its
  versioned upstream asset only after matching the recorded SHA-256 digest.
- Pages and release write permissions are scoped to the deployment/release job.
- Release tag input is passed through environment variables, validated against
  a strict filename-safe alphabet, and never rendered into shell source.
- Marketplace secrets are passed through environment variables and quoted.
- `scripts/check_workflow_security.py` is run by CI and rejects mutable actions,
  network-to-shell pipelines, or regressions in the Claude trust boundary.

## Verification

- `python3 scripts/check_workflow_security.py` — passed (8 workflows).
- `actionlint` — passed.
- `bash -n .github/scripts/install-wasm-pack.sh` — passed.
- `shellcheck .github/scripts/install-wasm-pack.sh` — passed.
- Live installer smoke test — downloaded the pinned asset, verified the digest,
  and executed `wasm-pack 0.13.1` from an isolated temporary directory.
- `git diff --check` — passed.

## Residual boundary

This stage does not change the Studio message channel or browser-extension
network/parser boundaries. Those are isolated in stages 2–4.
