//! FIELD_BEGIN/FIELD_END 인터리빙 — 갭 용량 예약 회귀 테스트 (rhwp-3, 자동 리뷰 발견).
//!
//! `field_end_interleave_tests.rs`가 200줄을 넘지 않도록 별도 파일로 분리했다.

use super::super::*;

/// Regression test for an automated-review-flagged issue: a due FIELD_END
/// sharing a gap with an unrelated non-field control (Bookmark) can be
/// starved of room when a SECOND, later non-field control (belonging to
/// the NEXT gap) is allowed to jump ahead and consume this gap's remaining
/// capacity. That pushes the due END past this gap's boundary into the
/// safety-net flush, which drags the later control along with it --
/// corrupting that control's anchor position (it moves from after the next
/// text character to before it).
///
/// Trigger (mirrors a parsed source with this same field_ranges/text
/// shape): a field wrapping only "A", followed by two independent
/// bookmarks -- one anchored between 'A' and 'B', one anchored between 'B'
/// and 'C'. char_offsets=[8, 25, 34] give exactly 16 units (2 slots) for
/// the gap before 'B', which must fit BOOKMARK1 and the field's END
/// (their relative order is not something this data model can determine,
/// since neither advances char_count -- either order is a faithful
/// reconstruction). The one invariant the model DOES fix is which gap
/// each control falls in: BOOKMARK1 must stay in the gap before 'B',
/// and BOOKMARK2 must stay in the gap before 'C', never pulled forward
/// into the earlier gap ahead of the due FIELD_END.
#[test]
fn test_serialize_reserves_gap_room_for_due_field_end_over_later_bookmark() {
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

    let para = Paragraph {
        text: "ABC".to_string(),
        controls: vec![
            field_ctrl,
            Control::Bookmark(Bookmark { name: "bm1".to_string() }),
            Control::Bookmark(Bookmark { name: "bm2".to_string() }),
        ],
        field_ranges: vec![FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 0 }],
        char_offsets: vec![8, 25, 34],
        ..Default::default()
    };

    let bytes = test_serialize_para_text(&para);
    let units: Vec<u16> = bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();

    // Decode the stream into a flat sequence of "tokens": each control is
    // its 8-unit leading code, each text char is its own code unit.
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < units.len() {
        let u = units[pos];
        if u == 0x0041 || u == 0x0042 || u == 0x0043 {
            tokens.push(u);
            pos += 1;
        } else if u == 0x000D {
            break; // paragraph end marker
        } else {
            tokens.push(u);
            pos += 8;
        }
    }

    // BOOKMARK1 and the field's END may appear in either relative order --
    // both are equally faithful reconstructions -- but both must land in
    // the gap between 'A' and 'B', and BOOKMARK2 must land strictly between
    // 'B' and 'C', never pulled forward into the earlier gap.
    let a_pos = tokens.iter().position(|&t| t == 0x0041).unwrap();
    let b_pos = tokens.iter().position(|&t| t == 0x0042).unwrap();
    let c_pos = tokens.iter().position(|&t| t == 0x0043).unwrap();
    let begin_pos = tokens.iter().position(|&t| t == 0x0003).unwrap();
    let end_pos = tokens.iter().position(|&t| t == 0x0004).unwrap();
    let bookmark_positions: Vec<usize> =
        tokens.iter().enumerate().filter(|&(_, &t)| t == 0x0016).map(|(i, _)| i).collect();
    assert_eq!(bookmark_positions.len(), 2, "both bookmarks must survive the round-trip");
    let (bm1_pos, bm2_pos) = (bookmark_positions[0], bookmark_positions[1]);

    assert!(begin_pos < a_pos, "field must open before 'A'");
    assert!(a_pos < end_pos && end_pos < b_pos, "field must close between 'A' and 'B'");
    assert!(
        a_pos < bm1_pos && bm1_pos < b_pos,
        "BOOKMARK1 must stay anchored between 'A' and 'B', not moved elsewhere"
    );
    assert!(
        b_pos < bm2_pos && bm2_pos < c_pos,
        "BOOKMARK2 must stay anchored between 'B' and 'C' -- not pulled forward \
         into the earlier gap ahead of the due FIELD_END"
    );
}
