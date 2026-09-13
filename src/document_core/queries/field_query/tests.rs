//! `rebuild_char_offsets` 관련 기존 회귀 테스트.
//!
//! `field_query.rs` 소스 파일이 200줄 baseline을 넘지 않도록 인라인
//! `mod tests { ... }` 블록을 두 파일로 나눠 분리했다. 이 파일은 이 PR
//! (rhwp-2)의 수정 대상이 아닌 기존 테스트를 담는다.

use super::*;
use crate::model::control::{Control, Field, FieldType};
use crate::model::paragraph::{FieldRange, Paragraph};

fn make_field_control(ctrl_id: u32) -> Control {
    Control::Field(Field {
        field_type: FieldType::ClickHere,
        command: String::new(),
        properties: 0,
        extra_properties: 0,
        field_id: ctrl_id,
        ctrl_id,
        ctrl_data_name: None,
        memo_index: 0,
    })
}

#[test]
fn rebuild_preserves_mid_text_field_begin_gap() {
    // Stream: [ColumnDef 8B] A(1) B(1) C(1) [FIELD_BEGIN 8B] X(1) Y(1) [FIELD_END 8B]
    let mut para = Paragraph {
        text: "ABCXY".into(),
        controls: vec![
            Control::ColumnDef(Default::default()),
            make_field_control(100),
        ],
        field_ranges: vec![FieldRange {
            start_char_idx: 3,
            end_char_idx: 5,
            control_idx: 1,
        }],
        char_offsets: vec![8, 9, 10, 19, 20],
        ..Default::default()
    };

    rebuild_char_offsets(&mut para);

    // A=8(+1) B=9(+1) C=10(+1) → gap 8 for FIELD_BEGIN → X=19(+1) Y=20
    assert_eq!(para.char_offsets, vec![8, 9, 10, 19, 20]);
}

#[test]
fn rebuild_field_at_start_no_double_count() {
    // FIELD_BEGIN is pre-text control (control_idx=0 < ctrls_before_text=1)
    let mut para = Paragraph {
        text: "XY".into(),
        controls: vec![make_field_control(100)],
        field_ranges: vec![FieldRange {
            start_char_idx: 0,
            end_char_idx: 2,
            control_idx: 0,
        }],
        char_offsets: vec![8, 9],
        ..Default::default()
    };

    rebuild_char_offsets(&mut para);

    assert_eq!(para.char_offsets, vec![8, 9]);
}

#[test]
fn rebuild_after_set_field_creates_serializable_gap() {
    // After set_field: "라벨: " [FIELD_BEGIN] "NEW" [FIELD_END]
    let mut para = Paragraph {
        text: "라벨: NEW".into(), // 7 chars: 라 벨 : ' ' N E W
        controls: vec![
            Control::ColumnDef(Default::default()),
            make_field_control(200),
        ],
        field_ranges: vec![FieldRange {
            start_char_idx: 4,
            end_char_idx: 7,
            control_idx: 1,
        }],
        // 원본 offsets (stale after text change, but char_offsets[0] still valid for ctrls_before_text)
        char_offsets: vec![8, 9, 10, 11, 20, 21, 22],
        ..Default::default()
    };

    rebuild_char_offsets(&mut para);

    // ctrls_before_text = 8/8 = 1
    // 라=8(+1) 벨=9(+1) :=10(+1) ' '=11(+1) → field_begin gap +8 → N=20(+1) E=21(+1) W=22
    assert_eq!(para.char_offsets[0], 8);  // 라
    assert_eq!(para.char_offsets[3], 11); // ' '
    assert_eq!(para.char_offsets[4], 20); // N — 8-byte gap after ' ' for FIELD_BEGIN
    let gap = para.char_offsets[4] as i64 - (para.char_offsets[3] as i64 + 1);
    assert_eq!(gap, 8); // serializer needs exactly 8 code units for FIELD_BEGIN
}
