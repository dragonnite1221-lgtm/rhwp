# PR #1076 처리 보고 — set_field_text_at 메타데이터 불일치

## 1. 결정

**수정 요청 (거절)** — 한컴 수동 검증에서 버그 잔존 확인.
PR 코드에 인덱스 의미 불일치 결함.

| 항목 | 값 |
|------|-----|
| 번호 | #1076 |
| 제목 | fix: set_field_text_at 메타데이터 불일치 — ClickHere 필드 값 설정 시 파일 손상 (#838) |
| 작성자 | oksure (Hyunwoo Park) — 기존 컨트리뷰터 |
| base ← head | `devel` ← `contrib/fix-field-text-corruption` |
| 연결 이슈 | Closes #838 (Bug — **잔존**) |
| 상태 | **OPEN 유지, 수정 요청** (PR 댓글 게시) |

## 2. 검증 결과

### 2.1 자동 검증 (cherry-pick `92846063`)

| 항목 | 결과 |
|------|------|
| cherry-pick | ✅ 충돌 없음 |
| `cargo test` | ✅ 1586 passed, 0 failed |
| `cargo clippy -- -D warnings` | ✅ 0 warnings |
| `cargo fmt --all -- --check` | ✅ 위반 0건 |
| WASM 빌드 | ✅ 성공 |
| CI | ✅ 전부 pass |

자체 검증은 통과 — 그러나 **회귀 가드 테스트 부재**로 한컴
호환 회귀를 검출하지 못했음.

### 2.2 한컴 수동 검증 — **실패**

작업지시자 시나리오 (`samples/field-01.hwp` → 필드 2개 값 설정 →
저장 → `saved/111field-01.hwp`):

1. 한컴 오피스에서 "파일 손상" 경고 (#838 증상 잔존)
2. 한컴 렌더: 첫 필드 직후 모든 내용 출력 안 됨
3. rhwp-studio: 입력값 옆에 필드 플레이스홀더(안내문) 병기

## 3. 결함 진단 (확정)

`saved/111field-01.hwp` IR 비교 (문단 0.7 "회사명" 필드):

| 항목 | 원본 | 손상본 |
|------|------|--------|
| char_count | 38 | **45** (+7) |
| text_len | 7 | **14** (+7) |
| 텍스트 | `"회사명\t\t: "` | `"회사명\t\t: 첫회사명 필드"` |

→ **안내문 미삭제 + 입력값 텍스트 끝에 추가**.

### Root cause — 인덱스 의미 불일치

PR 코드 (`field_query.rs:337-348`):
```rust
let start_idx = fr.start_char_idx;            // = 7 (char_count 기준)
let count = fr.end_char_idx - fr.start_char_idx;
para.delete_text_at(start_idx, count);
para.insert_text_at(start_idx, value);
```

- `FieldRange.start_char_idx` / `end_char_idx`: **문단 char_count
  기준** offset (FIELD_BEGIN/END 컨트롤 문자 포함, 값=7)
- `delete_text_at(char_offset, count)` (`paragraph.rs:382`):
  `self.text.chars().collect()` 후 인덱싱 → **순수 텍스트 char
  offset** (컨트롤 제외)

두 인덱스 의미가 다른데 변환 없이 그대로 전달 →
`start_idx=7 >= text_len=7` 가드에 걸려 **0건 삭제 후 return**.
`insert_text_at(7, value)` 가 텍스트 끝에 추가 → 안내문 + 입력값
병기 + 메타데이터 불일치 → 한컴 파일 손상.

PR 본문 주장 "delete_text_at + insert_text_at 가 모든 메타데이터
시프트" 는 사실. **그러나 인덱스 변환을 누락한 것이 결함**.

## 4. 처리

- **PR #1076 OPEN 유지** — 작성자에게 수정 요청 댓글 게시
  (https://github.com/edwardkim/rhwp/pull/1076#issuecomment-4524325367)
- cherry-pick 브랜치 `pr1076-cherry` 삭제
- 이슈 #838 OPEN 유지 (PR 재푸시 시 재검토)
- Vite dev server 종료, 환경 정리 완료
- **자매 PR #1080** (`set_cell_field_text` 같은 패턴) 도 동일
  결함 가능성 → 별도 검토 시 동일 인덱스 변환 점검 필수

## 5. 후속 권고

- 작성자가 `FieldRange` char_count 기준 offset → 순수 텍스트
  char offset 변환 추가 (`para.char_offsets` 활용 또는 컨트롤
  문자 카운팅) 후 재푸시
- 회귀 가드 단위 테스트 (이슈 #838 재현 시나리오: 안내문 있는
  ClickHere + setFieldValue → 저장 → 메타데이터 정합 검증) 추가
  부탁
- PR #1080 도 같은 결함 가능성 → 작성자가 함께 확인하도록 안내됨
