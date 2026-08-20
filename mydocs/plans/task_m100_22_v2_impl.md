# 구현계획서 — 사용자 포크 Task #22

수행계획서: [`task_m100_22_v2.md`](./task_m100_22_v2.md)

## Stage 1 — GitHub Actions 신뢰 경계와 공급망

- 이벤트별 `author_association` 검증으로 비신뢰 AI 호출 차단
- 권한 최소화, 서드파티 액션 커밋 SHA 고정
- wasm-pack 설치를 고정 버전 다운로드 + SHA-256 검증 방식으로 통합
- workflow 정적 회귀 테스트와 `actionlint` 검증

## Stage 2 — Studio 메시지 채널

- 부모 origin을 URL/config 기반 allowlist로 제한
- 최초 연결된 `WindowProxy`와 origin을 채널 capability로 고정
- 모든 응답에 요청 origin을 정확한 `targetOrigin`으로 사용
- 허용/거부 origin, 다른 source, 내보내기 API 회귀 테스트

## Stage 3 — Chrome 확장 네트워크 경계

- 자동 프리패치를 사용자 동작 기반 요청으로 축소
- sender/tab/frame 검증을 라우터 진입점에서 강제
- DNS 및 리다이렉트 단계마다 public HTTP(S) 목적지 검증
- timeout, Content-Length 및 스트리밍 실제 바이트 상한 적용
- localhost/사설망/redirect/대용량 응답 회귀 테스트

## Stage 4 — CFB 파서 fail-closed 강화와 통합 검증

- sector 지수, checked offset/length, FAT·miniFAT 순환 및 최대 단계 검증
- 정상 썸네일 호환 테스트와 잘못된 입력 fuzz 성격 테스트
- Rust/TypeScript/Chrome 전체 테스트, CodeGraph sync, 공통 2차 리뷰
- 정확한 `gemini-3.7-flash` 단계 리뷰 후 PR 생성·필수 체크·병합

## 변경 예상 파일

- `.github/workflows/*.yml`, `.github/scripts/` 또는 `scripts/`
- `rhwp-studio/src/main.ts` 및 메시지 채널 테스트
- `rhwp-chrome/content-script.js`, `sw/message-router.js`, `sw/thumbnail-extractor.js`, URL/sender 검증 모듈과 테스트
- CFB lenient parser 구현 및 테스트
- `mydocs/working/task_m100_22_v2_stage{1..4}.md`
- `mydocs/report/task_m100_22_v2_report.md`

## 위험 통제

- 공개 웹 문서의 정상 HWP 미리보기는 사용자 제스처 이후 유지한다.
- iframe 호스트는 allowlist 설정 방법을 제공하되 기본값은 same-origin으로 닫는다.
- CFB lenient 모드는 복구 가능한 비정상 문서를 계속 읽되 메모리·범위 안전성은 완화하지 않는다.
