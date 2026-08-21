# M100 / #22 보안 신뢰 경계 강화 결과

## 결과

워크플로 입력부터 브라우저 문서 로드까지 이어지는 다섯 보안 경계를
각각 fail-closed 정책과 실행 가능한 회귀 테스트로 교체했다.

1. GitHub Actions는 신뢰된 참여자만 AI 워크플로를 호출할 수 있고,
   최소 권한·고정 action SHA·검증된 wasm-pack 아카이브를 사용한다.
2. Studio RPC는 정확한 HTTP(S) origin과 `WindowProxy` identity를 함께
   인증하며, wildcard 응답과 임의 parent/opener 내보내기를 차단한다.
3. Chrome 확장은 사용자 gesture, 검증된 sender, public-only URL, 갱신 상한이
   있는 exact-URL capability를 모두 요구한다.
4. 공용 HWP/HWPX 썸네일 파서는 CFB/ZIP 범위·크기·순환·압축 해제 상한을
   검사하고 malformed 입력에서 닫힌다.
5. Firefox와 Safari에도 같은 capability 및 public-network 불변식을
   적용하고 자동 prefetch, private/local 우회, 무제한 fetch를 제거했다.
6. 브라우저 background가 받은 문서 bytes는 JSON 메시지로 보내지 않고
   extension-origin IndexedDB의 2분·1회용 record로 전달한다. 활성 record는
   소비 전에 축출하지 않고 128 MiB 총량 admission으로 새 요청을 거부한다.
7. URL 원문을 남기지 않는 SHA-256 digest 기반 local grant는 5분 lease와
   7일 절대 갱신 상한을 사용해 refresh/브라우저 복원을 지원하고, iOS overlay도 background가
   발급한 grant-bearing viewer URL만 사용한다.
8. 공개 `@rhwp/editor` iframe은 256-bit fragment capability, direct parent
   `WindowProxy`, 정확한 응답 origin을 함께 인증해 임의 소비자 origin 호환성과
   RPC 경계를 동시에 보존한다.
9. DoH 공개주소 precheck 뒤 실제 브라우저 연결의 socket IP를 body 읽기 전에
   재검증하고, 같은 URL 요청을 직렬화하며 extension initiator만 결속한다.

## 검증 요약

- Workflow 정책 검사, actionlint, ShellCheck, wasm-pack 실다운로드와
  SHA-256 검증을 통과했다.
- Studio production build 및 실제 headless Chrome postMessage E2E를
  통과했다.
- Chrome 61개, Firefox 61개, Safari 9개 보안 회귀 테스트를 통과했다.
- Chrome/Firefox/Studio production build와 VS Code extension production
  compile이 통과했다. 네 npm audit 모두 통과했고 보고된 취약점은 0개다.
- 공용 파서는 500개 deterministic malformed corpus와 실제 저장소 HWP/HWPX
  샘플을 모두 검증했다.
- Rust 1.98 `cargo test`는 1,230개 main test(2 ignored) 및 모든 후속
  integration suite를 통과했고, CI와 동일한 `cargo clippy -- -D warnings`도
  통과했다. CI/WASM toolchain은 1.98.0으로 고정하고 cache key에도 버전을
  넣어 floating stable에 따른 gate drift를 제거했다.
- 실측 최소 Rust는 1.88이다. 1.75는 lockfile v4를 읽지 못하고 1.85는 현재
  `image`/`zip` MSRV에 거부됐으며, clean checkout의 1.88 `cargo check`는 통과했다.
  Cargo metadata와 한/영 문서에 1.88을 선언하고 독립 MSRV CI job으로
  계속 검증한다.
- 각 단계와 Stage 5의 다섯 staged-diff lane을 실제 `gemini-3.7-flash`로
  검토했다. 전체 브랜치 재검토에서 잘린 응답 하나는 승인으로 폐기했고,
  그 과정에서 찾은 binary-message 경계를 수정·실브라우저 검증한 뒤
  store, Chrome, Firefox/Safari, Studio/E2E, 문서 lane으로 재검토해 모든
  유효 응답에서 `NO_ISSUES`를 확인했다. 재차 잘린 combined-lane 응답도
  승인으로 세지 않았다.
- 원격 PR 리뷰의 1차 8개 지적(Safari 15 저장소, iOS overlay grant, 공개 iframe
  호환성, `.yaml` 누락, multiline pipe 우회, fragment 오탐, download
  `finalUrl` 누락, 실제 Rust MSRV 불일치)을 모두 테스트와 함께 수정했다.
  extension/Safari, Studio/editor, workflow policy, download final-URL, Rust
  MSRV lane을 `gemini-3.7-flash`로 다시 검토해 각각 `NO_ISSUES`를 확인했다.
- 후속 6개 지적(교차 출처 문서, DNS rebinding, 활성 transfer 축출, viewer
  grant 갱신, Vite 8 Node 하한, Safari background 형식)도 실제 연결 IP 검증,
  byte admission, 7일 상한 갱신, Node 22.12 engines/docs, Safari event-page
  IIFE로 수정하고 회귀 테스트를 추가했다.
- 실제 headless Chrome에서 서로 다른 origin의 소비자 페이지가 기본
  `@rhwp/editor`로 Studio capability handshake를 완료했다. CI에도 Web security
  boundary job을 추가해 이 계층의 단위 회귀를 상시 실행한다.
- CodeGraph index를 최종 변경에 맞게 동기화했고 `git diff --check`를
  통과했다.
- 서버 공통 second-review gate도 사전 snapshot으로 실행했으나 독립 fallback
  provider가 unavailable 상태라 `blocked`를 반환했다. 코드 finding은
  없었으며, 이 인프라 결과를 실제 `gemini-3.7-flash` 호출 성공과 분리해
  기록한다.

## 알려진 검증 경계

- 현재 Linux 호스트에서는 Xcode, Apple signing, 실제 Safari 로드를
  수행할 수 없다. Safari source/test와 배포에 사용하는 단일 Rolldown
  bundle 생성·모의 API 로드는 검증했지만 signed macOS build로 표현하지
  않는다.
- 브라우저 fetch는 DoH 결과에 socket을 직접 pin할 수 없으므로, 권한 fetch의
  실제 `onResponseStarted` 연결 IP가 공개 주소인지 body 소비 전에 확인한다.
  private 연결은 즉시 abort하며, credential/referrer/redirect 없이 사용자
  gesture로 선택된 exact URL만 처리한다.
- 더 넓은 `cargo clippy --all-targets --all-features -- -D warnings` audit에는
  기존 test-only warning이 남아 있다. production CI target의 warning을
  숨기거나 gate를 약화하지 않았으며, 전체 Rust test와 CI 동일 Clippy는
  통과했다. test-only 정리는 별도 품질 작업으로 분리한다.

## 병합 조건

직접 `gemini-3.7-flash` 재검토는 완료됐다. 공통 gate의 provider 장애는
코드 승인으로 간주하지 않고 위와 같이 남기며, 원격 PR 필수 체크와 PR
diff 상태를 다시 확인한 후에만 병합한다. 이 문서는 실제 병합이 완료되기
전에는 병합 완료를 주장하지 않는다.
