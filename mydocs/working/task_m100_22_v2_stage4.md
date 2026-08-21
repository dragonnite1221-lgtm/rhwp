# M100 / #22 Stage 4 — Fail-closed HWP/HWPX thumbnail parser

## Scope

- Replace the duplicate Chrome and Firefox thumbnail parsers with one canonical
  bounded implementation.
- Reject malformed CFB and ZIP structures without out-of-range reads, unbounded
  allocation, or non-terminating sector traversal.
- Preserve preview extraction for real repository HWP and HWPX samples.

## Implementation

- The shared CFB parser validates the full magic, byte order, major-version and
  sector-size pairing, mini-sector size, cutoff, DIFAT/FAT counts and sector
  locations, directory objects, 64-bit stream sizes, and every byte range.
- FAT, DIFAT, miniFAT, directory, mini-stream, and document-stream walks detect
  repeated sectors and are bounded by container-derived limits.
- The shared ZIP parser validates EOCD placement, single-disk metadata, central
  and local entry agreement, exact preview paths, compression method, encryption
  flags, offsets, and both compressed and uncompressed sizes.
- Deflate output is streamed through a 10 MiB actual-byte ceiling. Parsed PNG,
  BMP, and GIF previews also have 8192-pixel dimension and 40-megapixel limits.
- Chrome and Firefox source wrappers use the canonical module during development;
  each build overwrites the wrapper with a self-contained packaged copy.

## Verification

- Shared parser tests — 7 passed: stored and deflated ZIP, advertised ZIP bomb,
  dimension bomb, exact-path enforcement, local/central disagreement, trailing
  data, cyclic CFB FAT, invalid sector version, and 500 deterministic malformed
  inputs.
- Chrome security suite — 26 passed; Firefox parser suite — 7 passed.
- Chrome and Firefox production builds passed, and both packaged parser module
  graphs loaded without source-tree imports.
- Six repository HWP/HWPX samples retained valid PNG preview extraction,
  including regular-FAT and mini-stream images from 1x1 through 724x1024.
- `gemini-3.7-flash` independently returned `NO_ISSUES` for the parser source,
  parser tests, and extension adapters. One earlier malformed model response was
  discarded rather than counted as approval.

## Follow-up discovered during this stage

Firefox still duplicates the pre-Stage-3 automatic-prefetch and privileged-fetch
boundary that was already removed from Chrome. This is not treated as a residual
risk: Stage 5 extends the same sender, capability, public-network, redirect,
timeout, and streamed-byte invariants to Firefox before PR creation.
