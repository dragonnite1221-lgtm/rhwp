# PR #1088 처리 보고 — para-float 표 vertical_offset 정렬 (회귀로 거절)

## 1. 결정

**수정 요청 (거절)** — 자동 검증 통과했으나 시각 회귀 1건 확정.

| 항목 | 값 |
|------|-----|
| 번호 | #1088 |
| 작성자 | HaimLee-4869 (Lee eunjung) — 기존 컨트리뷰터 (6번째 PR) |
| 연결 이슈 | Closes #1087 (해소 **미완**)
| 상태 | **OPEN 유지, 수정 요청** (PR 댓글 게시) |

## 2. 검증 결과

### 자동 검증 (통과)
| 항목 | 결과 |
|------|------|
| cherry-pick `b7f6f6e2` (검증 전용) | ✅ 충돌 없음 |
| 전체 `cargo test` | ✅ 1596 passed, 0 failed |
| `svg_snapshot` (골든 불변) | ✅ 8 passed, 0 failed |
| `cargo clippy -- -D warnings` | ✅ 0 warnings |
| `cargo fmt --all -- --check` | ✅ 위반 0건 |
| WASM 빌드 | ✅ 성공 |
| CI | ⚠️ 미실행 (fork 첫 푸시 패턴, 본 PR 결함 아님) |

### 시각 검증 — **회귀 1건 확정**

`samples/hwp-multi-001.hwp` 1페이지 — 작업지시자 보고. PR 본문
주장 \"276 샘플 변동 0\" 과 모순. 자동 측정(페이지수·LAYOUT_OVERFLOW)
으로는 검출되지 않는 시각 회귀.

## 3. 결함 진단 (확정)

### 회귀 케이스 IR (문단 0.3)

| 컨트롤 | wrap | treat_as_char | vert_rel_to | v_off | PR 정렬 키 |
|--------|------|---------------|-------------|-------|------------|
| [0] 표 (\"제목명\") | 위아래 | **false** | Para | **2346** | **2346** |
| [1] 표 (\"보도시점\") | 어울림 | **true** (TAC) | Para | 0 | **0** (TAC 라 제외) |

### PR 정렬 후
- IR 배열: `[표0(2346), 표1(0)]`
- 정렬 후: **`[표1(0), 표0(2346)]`** — 순서 역전
- 한컴 정답: IR 배열 순서 (\"제목명\" 위, \"보도시점\" 아래)

### 정량 입증 (SVG y 좌표 PR 전/후)

| 글자 | baseline | PR 적용 후 | 차이 |
|------|----------|-----------|------|
| \"보\" 작은 글자 | 324.95 | 172.31 | **−152.6px** |
| \"제\" 큰글자 | 609.87 | 583.88 | −25.99 |
| \"제\" 다른 인스턴스 | 727.47 / 886.31 | 792.46 / 951.29 | **+65px** |

### Root cause — PR 정렬 키의 사각지대

PR 정렬 키는 **TAC 표를 무조건 키=0으로 처리**:
```rust
match ctrl {
    Control::Table(t)
        if !t.common.treat_as_char  // ← TAC 제외
            && matches!(t.common.text_wrap, TextWrap::TopAndBottom)
            && matches!(t.common.vert_rel_to, VertRelTo::Para) =>
        t.common.vertical_offset as i32,
    _ => 0,
}
```

PR 검증 케이스(공직기강·국립국어원·#103) 는 모두 **라벨(in-flow
text) + 표(out-of-flow)** 조합. 본 케이스는 **두 표 모두 out-of-flow
이고 한쪽이 TAC, 한쪽이 비-TAC 위아래** 인 혼합 — PR 정렬이
다루지 않은 사각지대.

## 4. 처리

- **PR #1088 OPEN 유지** — 작성자에게 수정 요청 댓글 게시
  (https://github.com/edwardkim/rhwp/pull/1088#issuecomment-4537590143)
- 회귀 증거 + 옵션 A/B 수정 권고 + 회귀 가드 추가 요청
- verify 브랜치 `pr1088-verify` 삭제
- 이슈 #1087 OPEN 유지 (PR 재푸시 시 재검토)
- local/devel `c756d42d` 으로 origin/devel 동기화

## 5. 평가

### PR 의 강점 (유효)
- root cause 진단 정밀 (배치 순서 vs 위치 분리)
- 공직기강·국립국어원·#103 케이스 개선 명확
- 안정 정렬 + ctrl_idx 보존 설계 영리

### 한계 — 사각지대
- TAC + 비-TAC 위아래 표 혼합 케이스 미고려
- PR 본문 \"276 샘플 변동 0\" 은 자동 측정 한정 — 시각 회귀
  검출 못 함
- 검증 가드도 페이지수·LAYOUT_OVERFLOW 만, 시각 위치 차이 미검증

## 6. 후속 권고 (작성자에게 전달됨)

옵션 A: TAC 표에도 동일 정렬 키 (`treat_as_char` 조건 제거)
옵션 B: 혼합 케이스(TAC + 비-TAC 위아래) 시 정렬 건너뛰고 IR
        배열 순서 유지

어느 쪽이든 `hwp-multi-001.hwp` 1페이지를 회귀 가드 테스트로 추가
부탁. 한컴 정답(IR 배열 순서) 확인 후 옵션 결정 권고.

본 PR 의 진단(같은 문단 내 컨트롤은 vertical_offset 순) 자체는
옳음 — 다만 혼합 케이스 정합 후 재푸시 필요.
