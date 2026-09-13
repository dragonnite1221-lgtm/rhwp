//! FIELD_BEGIN/FIELD_END 인터리빙 바이트 순서 회귀 테스트 (rhwp-3, 2·3차 codex 발견).
//!
//! 이 두 테스트는 재파싱된 field_ranges만으로는 검증할 수 없는 버그를 다룬다
//! (폭이 0인 필드의 범위는 그 안의 다른 컨트롤/END가 어느 순서로 나오든
//! 동일하게 [p,p)로 남기 때문) — 그래서 직렬화 결과의 raw code unit을 직접
//! 검사한다. `field_range_tests.rs`가 200줄을 넘지 않도록 별도 파일로 분리했다.

use super::super::*;

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

/// Regression test for a third codex-flagged issue: two zero-width fields
/// NESTED at the same position (outer wraps inner, both [0,0)). The pending
/// FIELD_END list for this position holds inner's entry first (parser
/// stack-pop order), then outer's. Once outer's own BEGIN is placed, its END
/// becomes "due" by the naive `cidx < ctrl_idx` check -- but draining
/// whichever entry matches first (rather than strictly the front of the
/// list) let outer's END jump the queue and fire before inner's BEGIN was
/// even placed, closing outer prematurely and pushing inner outside it.
///
/// Correct order: BEGIN(outer), BEGIN(inner), END(inner), END(outer).
#[test]
fn test_serialize_nested_zero_width_fields_close_in_lifo_order() {
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
        // outer opens first (control_idx 0), inner opens second (control_idx 1)
        controls: vec![field_control(100), field_control(200)],
        // parser (stack-pop) order: inner closes first, so its FieldRange is
        // pushed before outer's.
        field_ranges: vec![
            FieldRange { start_char_idx: 0, end_char_idx: 0, control_idx: 1 }, // inner
            FieldRange { start_char_idx: 0, end_char_idx: 0, control_idx: 0 }, // outer
        ],
        // 2 controls (16) + 2 mid-text FIELD_END gaps, one per field (16) = 32.
        char_offsets: vec![32],
        ..Default::default()
    };

    let bytes = test_serialize_para_text(&para);
    let units: Vec<u16> = bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();

    let mut leading_codes = Vec::new();
    let mut pos = 0;
    while pos < units.len() && units[pos] != 0x0041 {
        leading_codes.push(units[pos]);
        pos += 8;
    }

    assert_eq!(
        leading_codes,
        vec![0x0003, 0x0003, 0x0004, 0x0004],
        "expected BEGIN(outer), BEGIN(inner), END(inner), END(outer) in strict \
         LIFO order -- outer's END must not jump ahead of inner's BEGIN"
    );

    // Also confirm the model still round-trips to the correct nesting.
    let (text, _offsets, field_ranges, _tabs) = parse_para_text(&bytes);
    assert_eq!(text, "A");
    assert_eq!(field_ranges.len(), 2);
    let mut by_ctrl: Vec<_> = field_ranges.iter().collect();
    by_ctrl.sort_by_key(|fr| fr.control_idx);
    assert_eq!((by_ctrl[0].start_char_idx, by_ctrl[0].end_char_idx), (0, 0), "outer");
    assert_eq!((by_ctrl[1].start_char_idx, by_ctrl[1].end_char_idx), (0, 0), "inner");
}
