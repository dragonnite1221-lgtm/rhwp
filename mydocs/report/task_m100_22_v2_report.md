# M100 / #22 보안 신뢰 경계 강화 결과

## 결과

워크플로 입력부터 브라우저 문서 로드까지 이어지는 다섯 보안 경계를
각각 fail-closed 정책과 실행 가능한 회귀 테스트로 교체했다.

1. GitHub Actions는 신뢰된 참여자만 AI 워크플로를 호출할 수 있고,
   최소 권한·고정 action SHA·검증된 wasm-pack 아카이브를 사용한다.
2. Studio RPC는 정확한 HTTP(S) origin과 `WindowProxy` identity를 함께
   인증하며, wildcard 응답과 임의 parent/opener 내보내기를 차단한다.
3. Chrome 확장은 사용자 gesture, 검증된 sender, 동일 출처 또는 정확한
   GitHub adapter, 단기 exact-URL capability를 모두 요구한다.
4. 공용 HWP/HWPX 썸네일 파서는 CFB/ZIP 범위·크기·순환·압축 해제 상한을
   검사하고 malformed 입력에서 닫힌다.
5. Firefox와 Safari에도 같은 capability 및 public-network 불변식을
   적용하고 자동 prefetch, private/local 우회, 무제한 fetch를 제거했다.

## 검증 요약

- Workflow 정책 검사, actionlint, ShellCheck, wasm-pack 실다운로드와
  SHA-256 검증을 통과했다.
- Studio production build 및 실제 headless Chrome postMessage E2E를
  통과했다.
- Chrome 26개, Firefox 26개, Safari 6개 보안 회귀 테스트를 통과했다.
- Chrome/Firefox/Studio production build와 세 npm audit가 모두 통과했고
  보고된 취약점은 0개다.
- 공용 파서는 500개 deterministic malformed corpus와 실제 저장소 HWP/HWPX
  샘플을 모두 검증했다.
- `cargo test`는 1,230개 main test(2 ignored) 및 모든 후속 integration
  suite를 통과했다.
- 각 단계와 최종 Stage 5의 다섯 staged-diff lane을 실제
  `gemini-3.7-flash`로 검토했고 모두 `NO_ISSUES`를 받았다.
- CodeGraph index를 최종 변경에 맞게 동기화했고 `git diff --check`를
  통과했다.

## 알려진 검증 경계

- 현재 Linux 호스트에서는 Xcode, Apple signing, 실제 Safari 로드를
  수행할 수 없다. Safari source/test와 배포에 사용하는 단일 Rolldown
  bundle 생성·모의 API 로드는 검증했지만 signed macOS build로 표현하지
  않는다.
- 브라우저 fetch에는 연결 IP를 고정하는 안정적인 API가 없어 DNS
  time-of-check/time-of-use window가 남는다. exact-URL capability,
  same-origin 정책, public DNS 검증, redirect 거부, timeout, 실제 byte
  제한으로 위험을 축소했다. 임의 cross-origin 자동 수집이 향후 제품
  요구가 되면 connection-level IP pinning이 가능한 relay가 다음 경계다.
- `cargo clippy --all-targets --all-features -- -D warnings`는 이번 변경과
  무관한 기존 Rust warning 84건에서 실패한다. 이번 변경에는 Rust 파일이
  없고 전체 Rust test는 통과했으므로 별도 품질 정리 대상으로 분리한다.

## 병합 조건

공통 second-review gate, 원격 PR 필수 체크, PR diff 상태를 다시 확인한
후에만 병합한다. 이 문서는 실제 병합이 완료되기 전에는 병합 완료를
주장하지 않는다.
