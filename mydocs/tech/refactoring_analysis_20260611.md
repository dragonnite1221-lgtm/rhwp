# rhwp 코드베이스 리팩토링 분석 및 단계적 개선 계획

작성일: 2026-06-11

## 1. 현황 요약

- `src/` Rust 약 184,400줄 (1,000줄 초과 파일 24개 이상)
- `rhwp-studio/` TypeScript 약 36,900줄, e2e 테스트 다수 (Puppeteer)
- TODO/FIXME/HACK 주석 5건 — 매우 깨끗함
- 50MB 초과 파일이 `pdf-large/` 외부에 없음 — LFS 격리 규약 준수 중
- CLAUDE.md의 "HWP3 전용 로직은 `src/parser/hwp3/` 안에서만" 규칙: 공통 모듈에서 hwp3 문자열이 검출되는 곳은 주석·통합 테스트·`FileFormat::Hwp3` enum 분기(converters)뿐 — **명백한 위반 없음** (converters의 포맷 분기는 경계 사례로 아래 기록)

## 2. 발견된 문제점

### High

- **`.unwrap()` 약 920건 / `.expect()` 약 202건 (src 전체, 테스트 포함)** — 다행히 파서 코어는 깨끗하지만(`parser/hwp3` 4건, `parser/hwpx` 12건), **`serializer/control.rs`에 331건**, `serializer/doc_info.rs` 117건이 집중. 대부분 `w.write_u32(..).unwrap()` 형태의 인메모리 쓰기라 실패 확률은 낮으나, 직렬화기가 panic 경로로 가득한 구조는 WASM 환경에서 문서 저장 실패를 즉시 크래시로 만든다. `Result` 전파(`?`) 구조로의 전환이 필요.
- **레이아웃/렌더러 거대 파일** — `renderer/layout.rs` 3,672줄 + `layout/paragraph_layout.rs` 3,211줄 + `layout/table_layout.rs` 2,735줄 + `typeset.rs` 3,012줄 + `svg.rs` 2,812줄 + `web_canvas.rs` 2,740줄. 레이아웃 버그 수정 빈도가 높은 영역(디버깅 워크플로우 문서가 있을 정도)인데 파일이 커서 변경 충돌·리뷰 비용이 크다.

### Medium

- **CLAUDE.md 문서-실제 불일치** — 문서의 파서 표는 HWP5 파서 위치를 `src/parser/hwp5/`로 안내하지만, 실제 HWP5 파서는 `src/parser/` 루트(`body_text.rs`, `doc_info.rs`, `record.rs`, `control/` 등)에 있고 `hwp5/` 디렉터리는 존재하지 않는다. 신규 기여자가 길을 잃는 지점.
- **`document_core/commands/object_ops.rs` 3,926줄** — 편집 커맨드가 단일 파일에 누적. `text_editing.rs`(2,336줄)도 동일 추세.
- **생성 데이터 테이블이 소스로 커밋** — `renderer/font_metrics_data.rs` 10,379줄, `parser/hwp3/johab_map.rs` 5,900줄, `renderer/pua_oldhangul.rs` 5,793줄. 빌드 시간·검색 노이즈 유발 (데이터 출처/재생성 방법 주석 여부 점검 필요).
- **`wasm_api/tests.rs` 15,759줄 단일 테스트 파일** — 모듈별 분할 필요 (unwrap 701건도 대부분 여기).
- **converters의 포맷 분기 경계 사례** — `document_core/converters/hwpx_to_hwp.rs:214`의 `FileFormat::Hwp3` matches는 변환 어댑터 차원의 분기로 합리적이나, HWP3 전용 동작이 추가되기 시작하면 규칙 위반으로 전이될 수 있는 지점. 가드 주석 권장.

### Low

- rhwp-studio `e2e/`에 `debug-*.test.mjs`(디버깅용)와 정규 테스트가 혼재 — 폴더 분리 또는 명명 규칙 필요
- rhwp-studio `src/engine/input-handler.ts` 2,878줄 — UI 입력 처리 단일 파일 비대화

## 3. 단계적 개선 계획

> 본 프로젝트는 하이퍼-워터폴 절차(이슈 등록 → `local/task{N}` 브랜치 → 수행계획서 → 승인 → 구현)를 따른다. 아래 각 항목은 GitHub 이슈 1건 단위로 등록 가능하도록 분리했다. **본 문서는 분석 자료이며, 실제 구현은 각 항목별 이슈·계획서·승인 절차를 거친다.**

### Phase 1 — 즉시 (안정성)

| 이슈 후보 | 내용 | 규모 | 리스크 | 검증 |
|---|---|---|---|---|
| serializer panic 제거 | `serializer/control.rs`(331건)·`doc_info.rs`(117건)의 write unwrap을 `?` 전파로 전환, 시그니처를 `Result`로 | M | 호출부 연쇄 수정 — serializer 경계에서 단계적 적용 | `cargo test` + 재현검증(re_sample_gen) |
| CLAUDE.md 파서 표 정정 | HWP5 파서 실제 위치(`src/parser/` 루트) 반영 또는 `parser/hwp5/`로 물리 이동 중 택일 (이동은 Phase 2) | S | 없음(문서) | 리뷰 |
| converters 가드 주석 | hwpx_to_hwp의 FileFormat 분기에 "HWP3 전용 로직 추가 금지" 가드 주석 | S | 없음 | 리뷰 |

### Phase 2 — 구조 (모듈 분해)

| 이슈 후보 | 내용 | 규모 | 리스크 | 검증 |
|---|---|---|---|---|
| layout.rs 분해 | `renderer/layout.rs` 3,672줄을 이미 존재하는 `layout/` 하위 모듈 체계로 흡수 | L | 레이아웃 회귀 — ir-diff·pdf 권위자료 시각 비교 필수 | `cargo test` + `export-svg` 회귀 비교 |
| object_ops.rs 분해 | 개체 타입별(표/그림/도형) 커맨드 모듈 분리 | L | 편집 회귀 | wasm_api 테스트 |
| wasm_api/tests.rs 분할 | 기능 영역별 테스트 모듈로 분할 | M | 낮음 | `cargo test` 동일 통과 수 |
| HWP5 파서 디렉터리 정리 | `src/parser/` 루트의 HWP5 파일들을 `src/parser/hwp5/`로 이동해 문서와 일치시킴 | L | import 경로 대량 변경 — 기계적이지만 충돌 큼, 한 번에 단독 수행 | `cargo test` 전체 |
| studio input-handler 분해 | 마우스/키보드/IME 등 입력 도메인별 분리 (`input-handler-keyboard.ts` 분리 전례 따름) | M | e2e 회귀 | e2e 텍스트 플로우 테스트 |

### Phase 3 — 장기

- 생성 데이터 테이블(font_metrics_data 등)의 생성 스크립트 정비 및 build.rs 생성 또는 별도 데이터 파일화 검토
- 렌더러 unwrap 잔여분(전 영역) 점진 제거 — 파일 단위 이슈로 분할, fuzz 입력(손상 HWP) 테스트 추가
- e2e의 debug-* 테스트 분리(`e2e/debug/`) 및 CI 대상 명확화

## 4. 검증 체크리스트

- [ ] `cargo test` 전체 통과 (각 단계 완료 시)
- [ ] `rhwp ir-diff sample.hwpx sample.hwp` 차이 건수 리팩토링 전후 동일
- [ ] `export-svg` 출력과 `pdf/` 권위 자료 시각 비교 (레이아웃 변경 시)
- [ ] `grep -rc "\.unwrap()" src/serializer/` 감소 추이 확인
- [ ] 50MB 초과 파일 `pdf-large/` 외부 0건 유지
