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
