//! quality-ledger 회귀 테스트 모음: 신뢰할 수 없는 char_offsets에서의 u32
//! 오버플로, PARA_HEADER 엔트리 개수 u16 절단 시 헤더/데이터 불일치,
//! trailing FIELD_END의 비결정적(HashMap) 순서를 각각 재현한다.

use super::*;
use crate::model::control::{Control, Field, FieldType};
use crate::model::paragraph::{CharShapeRef, FieldRange, LineSeg, Paragraph, RangeTag};

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

/// 회귀 테스트 (7654ba74464e): `para.char_offsets`는 파일에서 파싱된(또는
/// 프로그램적으로 조작될 수 있는) 신뢰할 수 없는 값이다. 이 값이
/// `u32::MAX` 근처이면 `offset + 8` 같은 평범한 덧셈은 디버그 빌드에서
/// panic하고 릴리즈 빌드에서는 랩어라운드로 잘못된 위치를 만든다.
#[test]
fn serialize_para_text_does_not_overflow_on_extreme_char_offset() {
    let para = Paragraph {
        text: "\t".to_string(),
        // u32::MAX에 매우 가까운 값 -- TAB 분기의 `offset + 8`이 그대로
        // 있었다면 오버플로한다.
        char_offsets: vec![u32::MAX - 3],
        char_shapes: vec![CharShapeRef { start_pos: 0, char_shape_id: 0 }],
        line_segs: vec![LineSeg { text_start: 0, ..Default::default() }],
        ..Default::default()
    };

    // panic하지 않아야 한다 (이전에는 debug 빌드에서 "attempt to add with
    // overflow"로 panic했다).
    let bytes = test_serialize_para_text(&para);
    assert!(!bytes.is_empty(), "serialization must still produce output");
}

/// 회귀 테스트 (b17c8341f57e): `PARA_RANGE_TAG` 엔트리가 65,535개를
/// 초과하면, 이전 구현은 헤더의 `numRangeTags` 필드(`u16`)에 잘린(모듈로)
/// 개수를 쓰면서도 실제 `PARA_RANGE_TAG` 레코드에는 원래 개수만큼의
/// 바이트를 그대로 써서 헤더와 실제 데이터가 어긋났다. 이제는 헤더 개수와
/// 실제로 쓰는 바이트 수가 항상 (잘라낸 뒤에도) 일치해야 한다.
#[test]
fn para_header_range_tag_count_matches_actual_serialized_data() {
    const N: usize = (u16::MAX as usize) + 5;
    let para = Paragraph {
        text: String::new(),
        controls: vec![],
        range_tags: (0..N).map(|i| RangeTag { start: i as u32, end: i as u32 + 1, tag: 0 }).collect(),
        char_shapes: vec![CharShapeRef { start_pos: 0, char_shape_id: 0 }],
        line_segs: vec![LineSeg { text_start: 0, ..Default::default() }],
        has_para_text: true,
        char_count: 1,
        ..Default::default()
    };

    let mut records = Vec::new();
    serialize_paragraph_list(std::slice::from_ref(&para), 0, &mut records);

    let header = records.iter().find(|r| r.tag_id == tags::HWPTAG_PARA_HEADER)
        .expect("PARA_HEADER record must be present");
    // 레이아웃: char_count(4) + control_mask(4) + para_shape_id(2) + style_id(1)
    // + break_type(1) + numCharShapes(2) + numRangeTags(2) + numLineSegs(2) + ...
    let declared_num_range_tags = u16::from_le_bytes([header.data[14], header.data[15]]) as usize;

    let range_tag_record = records.iter().find(|r| r.tag_id == tags::HWPTAG_PARA_RANGE_TAG)
        .expect("PARA_RANGE_TAG record must be present");
    // 각 RangeTag 엔트리는 12바이트(u32 * 3).
    let actual_num_range_tags = range_tag_record.data.len() / 12;

    assert_eq!(
        declared_num_range_tags, actual_num_range_tags,
        "PARA_HEADER's declared range-tag count must match the number of \
         entries actually written to PARA_RANGE_TAG, not silently diverge \
         after a u16 truncation"
    );
    assert_eq!(
        declared_num_range_tags,
        u16::MAX as usize,
        "when the source has more entries than u16::MAX can represent, both \
         the header count and the written data must be capped at u16::MAX, \
         not just the header"
    );
}

/// 회귀 테스트 (99f8261b023c / b6132fa8ccd0): 두 개의 폭 0 필드가 문단의
/// 텍스트 앞부분에서 열리고 둘 다 문단 끝에서 닫히면(trailing), 두 END
/// 모두 "orphan"으로 남는다 -- 자신의 FIELD_BEGIN이 이미 본문 갭에서
/// 배치되어 tail 루프(`while ctrl_idx < controls.len()`)가 이 필드들의
/// control_idx까지 도달하지 못하기 때문이다. 이전 구현은 이 orphan들을
/// `HashMap::values()` 순서(비결정적)로 내보냈는데, 안쪽 필드가 바깥쪽
/// 필드보다 반드시 먼저 닫혀야 하는 LIFO 스택 규칙을 어길 위험이 있었다.
#[test]
fn trailing_orphan_field_ends_are_emitted_in_correct_lifo_order() {
    const OUTER_CTRL_ID: u32 = 1;
    const INNER_CTRL_ID: u32 = 2;

    let para = Paragraph {
        text: "A".to_string(),
        // 두 컨트롤 모두 텍스트보다 앞에 온다 (8*2 = 16 code units gap).
        char_offsets: vec![16],
        controls: vec![
            make_field_control(OUTER_CTRL_ID), // control_idx 0: 바깥쪽 (먼저 열림)
            make_field_control(INNER_CTRL_ID), // control_idx 1: 안쪽 (나중에 열림, 먼저 닫혀야 함)
        ],
        // field_ranges 배열 순서 = 실제 닫힘(스택 pop) 순서: 안쪽이 먼저.
        field_ranges: vec![
            FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 1 }, // inner: index 0
            FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 0 }, // outer: index 1
        ],
        char_shapes: vec![CharShapeRef { start_pos: 0, char_shape_id: 0 }],
        line_segs: vec![LineSeg { text_start: 0, ..Default::default() }],
        ..Default::default()
    };

    let bytes = test_serialize_para_text(&para);
    let code_units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();

    // 0x0004(FIELD_END) 마커를 순서대로 찾아 그 ctrl_id를 복원한다.
    let mut end_ctrl_ids = Vec::new();
    let mut i = 0;
    while i < code_units.len() {
        if code_units[i] == 0x0004 {
            let lo = code_units[i + 1] as u32;
            let hi = code_units[i + 2] as u32;
            end_ctrl_ids.push(lo | (hi << 16));
            i += 8;
        } else {
            i += 1;
        }
    }

    assert_eq!(
        end_ctrl_ids,
        vec![INNER_CTRL_ID, OUTER_CTRL_ID],
        "the inner (later-opened) field's FIELD_END must always be emitted \
         before the outer field's, regardless of HashMap iteration order"
    );
}
