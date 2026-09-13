//! FIELD_BEGIN/FIELD_END gap-fill 직렬화-파싱 라운드트립 회귀 테스트 (rhwp-3).
//!
//! `body_text/tests.rs`가 커지는 것을 막기 위해 필드 범위(FieldRange) 관련
//! 회귀 테스트만 별도 파일로 분리했다.

use super::*;

/// rhwp-3 회귀 테스트: 인접한 두 필드(FIELD_END 직후 다른 필드의 FIELD_BEGIN이
/// 오는 경우)를 직렬화한 뒤 다시 파싱하면 필드 범위가 원래대로 복원되어야 한다.
///
/// 트리거: 텍스트 "A-B", controls=[필드0, 필드1], 필드0 범위 [0,1), 필드1 범위
/// [2,3), char_offsets=[8,17,26] (A와 '-' 사이, '-'와 B 사이에 각각 8-code-unit
/// 갭 하나씩). 수정 전에는 A와 '-' 사이 갭이 필드1의 FIELD_BEGIN에 잘못
/// 배정되어, 재파싱 시 필드1이 [1,1)의 빈 필드가 되고 필드0이 [0,3) 전체를
/// 먹어버렸다.
#[test]
fn test_roundtrip_adjacent_fields_preserve_ranges() {
    use crate::model::control::{Control, Field, FieldType};
    use crate::model::paragraph::{FieldRange, Paragraph};
    use crate::serializer::body_text::test_serialize_para_text;

    fn field_control(ctrl_id: u32) -> Control {
        Control::Field(Field {
            field_type: FieldType::Bookmark,
            command: String::new(),
            properties: 0,
            extra_properties: 0,
            field_id: ctrl_id,
            ctrl_id,
            ctrl_data_name: None,
            memo_index: 0,
        })
    }

    let para = Paragraph {
        text: "A-B".to_string(),
        controls: vec![field_control(100), field_control(200)],
        field_ranges: vec![
            FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 0 },
            FieldRange { start_char_idx: 2, end_char_idx: 3, control_idx: 1 },
        ],
        char_offsets: vec![8, 17, 26],
        ..Default::default()
    };

    let bytes = test_serialize_para_text(&para);
    let (text, _offsets, field_ranges, _tabs) = parse_para_text(&bytes);

    assert_eq!(text, "A-B");
    assert_eq!(field_ranges.len(), 2, "필드 두 개가 그대로 보존되어야 한다");

    let mut by_ctrl: Vec<_> = field_ranges.iter().collect();
    by_ctrl.sort_by_key(|fr| fr.control_idx);

    assert_eq!(by_ctrl[0].control_idx, 0);
    assert_eq!((by_ctrl[0].start_char_idx, by_ctrl[0].end_char_idx), (0, 1), "필드0은 [0,1)로 유지되어야 한다");

    assert_eq!(by_ctrl[1].control_idx, 1);
    assert_eq!((by_ctrl[1].start_char_idx, by_ctrl[1].end_char_idx), (2, 3), "필드1은 [2,3)으로 유지되어야 한다");
}

/// rhwp-3 추가 회귀 테스트: 같은 gap-fill pass 안에서 한 필드의 FIELD_BEGIN이
/// 막 배치된 시점(cidx == ctrl_idx)에 그 필드 자신의 FIELD_END도 같은 위치에서
/// 닫혀야 하는 경우, 다음 필드의 FIELD_BEGIN보다 반드시 먼저(인터리브되어) 나와야
/// 한다. 그렇지 않으면 스택(LIFO) 파서가 잘못된 BEGIN을 pop하게 되어 두 필드의
/// 범위가 뒤바뀐다.
///
/// 트리거: 텍스트 "A", field0=빈 필드 [0,0) (control_idx 0), field1=텍스트
/// 전체를 감싸는 [0,1) (control_idx 1). char_offsets=[24]는
/// `rebuild_char_offsets`가 실제로 계산하는 값과 동일하다: 컨트롤 2개(기본
/// 16 code unit) + field0의 FIELD_END 갭(8 code unit) = 24.
///
/// 올바른 직렬화 순서는 BEGIN(field0), END(field0), BEGIN(field1), 'A',
/// END(field1)이어야 한다 (field0의 BEGIN/END가 field1의 BEGIN보다 먼저
/// 인터리브되어야 스택이 올바르게 짝지어진다). 수정 전에는 두 BEGIN이 먼저
/// 한꺼번에 나오고(post-pass가 gap 전체에 대해 한 번만 실행되어) END(field0)가
/// 그 뒤에 나와, 파서가 END(field0)을 field1의 BEGIN과 잘못 짝지어 두 필드의
/// 범위가 뒤바뀌었다.
#[test]
fn test_roundtrip_empty_field_nested_at_same_start_as_spanning_field() {
    use crate::model::control::{Control, Field, FieldType};
    use crate::model::paragraph::{FieldRange, Paragraph};
    use crate::serializer::body_text::test_serialize_para_text;

    fn field_control(ctrl_id: u32) -> Control {
        Control::Field(Field {
            field_type: FieldType::Bookmark,
            command: String::new(),
            properties: 0,
            extra_properties: 0,
            field_id: ctrl_id,
            ctrl_id,
            ctrl_data_name: None,
            memo_index: 0,
        })
    }

    let para = Paragraph {
        text: "A".to_string(),
        controls: vec![field_control(100), field_control(200)],
        field_ranges: vec![
            FieldRange { start_char_idx: 0, end_char_idx: 0, control_idx: 0 },
            FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 1 },
        ],
        char_offsets: vec![24],
        ..Default::default()
    };

    let bytes = test_serialize_para_text(&para);
    let (text, _offsets, field_ranges, _tabs) = parse_para_text(&bytes);

    assert_eq!(text, "A");
    assert_eq!(field_ranges.len(), 2, "필드 두 개가 그대로 보존되어야 한다");

    let mut by_ctrl: Vec<_> = field_ranges.iter().collect();
    by_ctrl.sort_by_key(|fr| fr.control_idx);

    assert_eq!(by_ctrl[0].control_idx, 0);
    assert_eq!(
        (by_ctrl[0].start_char_idx, by_ctrl[0].end_char_idx),
        (0, 0),
        "필드0은 빈 필드 [0,0)으로 유지되어야 한다"
    );

    assert_eq!(by_ctrl[1].control_idx, 1);
    assert_eq!(
        (by_ctrl[1].start_char_idx, by_ctrl[1].end_char_idx),
        (0, 1),
        "필드1은 [0,1)로 유지되어야 한다 (텍스트 전체를 포함)"
    );
}

/// Regression test for a second codex-flagged issue found in the first
/// interleaving fix: draining a due FIELD_END *immediately* after its own
/// FIELD_BEGIN is placed (unconditionally, on every control) incorrectly
/// closes a zero-width field before a non-field control (e.g. Bookmark,
/// Table, Picture) that is meant to sit *inside* it. Only FIELD_BEGIN/
/// FIELD_END participate in the parser's LIFO stack -- other control types
/// never affect it -- so a zero-text field can legitimately wrap a non-field
/// control without any ordering hazard, and closing it too early physically
/// moves that control outside the field in the byte stream.
///
/// Trigger: controls=[Field (zero-width, wraps nothing in text), Bookmark],
/// field_ranges=[[0,0) over control_idx 0], text "A". The field's own
/// control is control_idx 0, Bookmark is control_idx 1; both must be placed
/// before 'A', with the field's END emitted only after the enclosed
/// Bookmark, not immediately after the field's own BEGIN. This cannot be
/// observed by re-parsing field_ranges (a zero-width field's recorded range
/// doesn't change no matter where among these zero-advance controls its END
/// lands), so this test inspects the raw serialized code units directly.
#[test]
fn test_serialize_field_end_waits_for_enclosed_non_field_control() {
    use crate::model::control::{Bookmark, Control, Field, FieldType};
    use crate::model::paragraph::{FieldRange, Paragraph};
    use crate::serializer::body_text::test_serialize_para_text;

    let field_ctrl = Control::Field(Field {
        field_type: FieldType::Bookmark,
        command: String::new(),
        properties: 0,
        extra_properties: 0,
        field_id: 100,
        ctrl_id: 100,
        ctrl_data_name: None,
        memo_index: 0,
    });
    let bookmark_ctrl = Control::Bookmark(Bookmark { name: "bm".to_string() });

    let para = Paragraph {
        text: "A".to_string(),
        controls: vec![field_ctrl, bookmark_ctrl],
        field_ranges: vec![FieldRange { start_char_idx: 0, end_char_idx: 0, control_idx: 0 }],
        // 2 controls (16) + 1 mid-text FIELD_END gap (8) = 24, same derivation
        // as `rebuild_char_offsets` would produce for this layout.
        char_offsets: vec![24],
        ..Default::default()
    };

    let bytes = test_serialize_para_text(&para);
    let units: Vec<u16> = bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();

    // Each control/marker occupies an 8-code-unit block whose first unit is
    // its char code. Collect just those leading codes, in order, up to 'A'
    // (0x0041).
    let mut leading_codes = Vec::new();
    let mut pos = 0;
    while pos < units.len() && units[pos] != 0x0041 {
        leading_codes.push(units[pos]);
        pos += 8;
    }

    assert_eq!(
        leading_codes,
        vec![0x0003, 0x0016, 0x0004],
        "expected BEGIN(field), BOOKMARK, END(field) -- the bookmark must stay \
         inside the zero-width field, not be pushed out after a premature END"
    );
}
