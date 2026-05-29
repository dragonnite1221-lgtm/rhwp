# PR #1088 검토 — 같은 문단 컨트롤 배치를 vertical_offset 기준으로 정합

## 1. PR 정보

| 항목 | 값 |
|------|-----|
| 번호 | #1088 |
| 제목 | fix(typeset): 같은 문단 컨트롤 배치를 vertical_offset 기준으로 정합 — para-float 표 vs in-flow 라벨 순서 (closes #1087) |
| 작성자 | **HaimLee-4869 (Lee eunjung)** — 기존 컨트리뷰터 (6번째 PR, #1020/#1021/#1026/#1047/#1059) |
| base ← head | `devel` ← `HaimLee-4869:pr/para-control-vertical-order` (fork) |
| 연결 이슈 | Closes #1087 (#103/#157/#266 후속), assignee 본인 지정 완료 |
| mergeable | MERGEABLE / BEHIND |
| CI | ⚠️ **no checks reported** (작성자 다른 fork PR #1059도 동일 — fork 첫 푸시 패턴, 본 PR 결함 아님) |
| 변경 | `src/renderer/typeset.rs` 단일 (+119, -17) |
| 커밋 | 1 (5efd0470) |

## 2. 배경 (이슈 #1087)

`typeset.rs:2277` `typeset_table_paragraph` 의 표 루프가
`para.controls` 를 **배열 인덱스 순**으로 순회. 한컴 정답 배치
순서는 컨트롤의 **vertical_offset** 순. 배열 순서와 무관.

**증거 (PR 본문 명시):**
- 공직기강 pi407: 라벨 v_off=0, 표 v_off=+3063 → **라벨이 표 위**
- 국립국어원 pi586: 표 v_off=-1796, 라벨 v_off=0 → **표가 라벨 위**
- 둘 다 IR 배열은 `[표, 라벨]` 인데 정답 반대

#103/#157/#266 이 비-TAC wrap=위아래 표의 **위치(vpos·높이)** 는
정합했으나 **배치 순서** 는 본선이 배열 인덱스 순으로 남아 있던
잔존.

## 3. 트러블슈팅 사전 검색 (feedback_search_troubleshootings_first)

관련 문서 3건:
- `typeset_partial_table_wrap_around.md`
- `task_m100_103_attempt1_postmortem.md` (#103 시도 실패 분석 — 본 PR 직접 선행 작업)
- `typeset_page_num_newumber_application.md`

→ PR 의 진단(배치 순서 vs 위치는 별개 문제) 이 이들의 결론과 정합.
   #266 (#103 후속) 이후 미해결로 남았던 \"같은 문단 내 정렬\" 사안.

## 4. 변경 내용 (typeset.rs 한 파일)

### 4.1 정렬 키 (`float_table_voffset`)
```rust
let float_table_voffset = |ctrl: &Control| -> i32 {
    match ctrl {
        Control::Table(t)
            if !t.common.treat_as_char
                && matches!(t.common.text_wrap, TextWrap::TopAndBottom)
                && matches!(t.common.vert_rel_to, VertRelTo::Para) =>
        {
            t.common.vertical_offset as i32
        }
        _ => 0,
    }
};
```
**비-TAC + wrap=위아래 + vert=Para 인 표만** vertical_offset 사용.
나머지 in-flow/TAC/다른 wrap 은 key 0 → 배열 순서 유지.

### 4.2 안정 정렬
```rust
let mut ctrl_order: Vec<usize> = (0..para.controls.len()).collect();
ctrl_order.sort_by_key(|&i| float_table_voffset(&para.controls[i]));
```
- `sort_by_key` 는 안정 정렬 — 동률 시 배열 순서 유지 (TAC 다수
  연속 시 회귀 면 격리).
- **`ctrl_idx` 는 원래 배열 인덱스를 유지** —
  `format_table`/`measured_tables`/`PageItem` 조회가 의존.

### 4.3 `is_first_placed` / `is_last_placed`
배열순서 기반 `is_first/last_table` 을 **배치순서 기반** 으로 재산출:
```rust
let first_placed_table = ctrl_order.iter().copied().find(|&i| matches!(...));
let last_placed_table  = ctrl_order.iter().copied().rev().find(|&i| matches!(...));
```
→ pre/post 텍스트와 spacing 이 실제 배치 첫/마지막 표에 붙도록.

### 4.4 시그니처 변경
- `typeset_tac_table`, `typeset_block_table`, `place_table_with_text`
  세 함수에 `is_first_placed`/`is_last_placed: bool` 인자 추가
  (`#[allow(clippy::too_many_arguments)]` 동반)

## 5. 검토 의견

### 5.1 강점

- **root cause 진단 정밀**: 두 실제 문서(공직기강·국립국어원) 의
  반대 케이스(라벨 먼저 vs 표 먼저) 로 정렬 알고리즘 검증.
  vertical_offset 순이 한컴의 진실인 결정적 증거.
- **적용 범위 명확히 격리**: 비-TAC + wrap=위아래 + vert=Para 조합
  만 정렬 key 사용. TAC/in-flow/Paper-absolute 는 key 0 → 회귀 면 0.
- **안정 정렬 + ctrl_idx 보존**: 동률 시 배열 순서 유지 →
  다른 코드가 의존하는 ctrl_idx 가 정렬과 분리. 영리한 설계.
- **광범위 회귀 검증 (PR 본문 명시)**:
  - `cargo test --lib 1336 passed`
  - `svg_snapshot 8골든 불변`
  - `fmt·clippy 통과`
  - native + WASM 빌드 통과
  - **전 samples 276개: 페이지수·LAYOUT_OVERFLOW 변동 0**
  - 한컴 PDF 좌표 대조: 공직기강(22p)·국립국어원(35p) 정합
  - #103 원본 케이스(hwpspec sec3.pi238) 도 라벨앞으로 교정
- 주석 풍부 — 진단 근거(공직기강 v_off=+3063, 국립국어원
  v_off=-1796, pic-in-* 전부 0) 코드 내 보존.
- `task_m100_103_attempt1_postmortem.md` 의 교훈을 직접 후속 작업
  으로 흡수.

### 5.2 검토 포인트

- **CI 미실행** (\"no checks reported\"): 작성자 다른 fork PR #1059
  도 동일 — fork 첫 push 패턴. 본 PR 자체 결함 아니나 자체 검증
  으로 대체 필요. PR 본문이 매우 철저한 자체 검증 (test 1336 +
  svg 골든 + 276 샘플 회귀 0) 명시 — 사실상 CI 가 잡았을 항목
  모두 자체 검증.
- typeset core 변경이라 회귀 위험 영역이긴 하나, 적용 범위 격리
  (비-TAC + wrap=위아래 + vert=Para) 와 안정 정렬로 위험 최소화.
- 본질이 시각 판정 대상 — \"라벨/표 순서\" 가 한컴과 일치 여부.
  PR 첨부 스크린샷이 공직기강 케이스 시각 정합을 보임.

## 6. 처리 방식 — **GitHub 머지** (작업지시자 지시)

> 작업지시자 지시: \"이번 PR 은 cherry-pick 하지 말고 머지로
> 처리. first-time contributor 가 사라질 것 같다.\"

비고: 작성자 통계상 6번째 PR (#1020/#1021/#1026/#1047/#1059 +
본 PR) 이라 \"기존 컨트리뷰터\" 분류이긴 하나, GitHub 의
\"first-time contributor\" 배지·기여 인식이 cherry-pick 처리
누적으로 사라질 우려가 있다는 작업지시자 판단 — fork base PR 의
정식 머지(`gh pr merge`) 로 author co-authored merge 기록을 GitHub
에 남기는 방향.

## 7. 검증 계획

- [ ] 로컬 검증용 cherry-pick (검증 후 폐기, 머지는 GitHub 에서) —
      또는 fetch + 작업 브랜치 분리
- [ ] 전체 `cargo test` + `cargo clippy -- -D warnings` +
      `cargo fmt --all -- --check`
- [ ] WASM 빌드 (typeset core 변경)
- [ ] svg_snapshot 골든 불변 확인
- [ ] 시각 판정 (작업지시자 판단) — 공직기강·국립국어원 샘플
      라벨/표 순서, #103 원본 케이스 회귀
- [ ] BEHIND 해소 (devel update branch 또는 작업지시자 `--admin`
      merge)
- [ ] GitHub 머지 — `gh pr merge 1088 --merge` (필요 시 `--admin`)

## 7. 판단 (잠정)

root cause 정밀 + 적용 범위 격리 + 안정 정렬 + 매우 철저한 자체
검증 (276 샘플 회귀 0) + #103/#266 잔존 문제의 직접적 마무리.
fork CI 미실행은 자체 검증으로 대체 충분.

검증 + 시각 판정 통과 시 수용 권고. 최근 #1076/#1077 처럼 회귀
가드 부재로 인한 잔존 버그 위험이 본 PR 은 자체 광범위 검증
(276 샘플 + 페이지수·LAYOUT_OVERFLOW 변동 0) 으로 상당히
완화됨.

검증 결과에 따라 `pr_1088_report.md` 작성.
