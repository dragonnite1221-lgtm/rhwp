//! `set_field_text_at`의 "포함하는 부모 필드" 판정 회귀 테스트 (rhwp-2).
//!
//! `field_query.rs` 소스 파일이 200줄 baseline을 넘지 않도록 인라인 `mod
//! tests { ... }` 블록을 두 파일로 나눠 분리했다. 이 파일은 이 PR에서
//! 새로 추가/수정된 테스트만 담는다 — 기존 테스트는 `tests.rs` 참고.

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

/// rhwp-2 회귀 테스트: 내부 필드를 빈 문자열로 축소했을 때, 그 내부 필드를
/// 완전히 포함하는 외부 필드의 end_char_idx가 함께 보정되어야 한다.
///
/// 트리거: "ABCD"에 외부 필드 [0,4)와 내부 필드 [1,3)가 있고, 내부 필드를
/// 빈 문자열로 치환한다. 수정 전에는 외부 필드가 [0,4)로 그대로 남아 텍스트
/// 길이(2)를 초과했고, 그 상태로 외부 필드를 다시 치환하면
/// text_chars[fr.end_char_idx..] 슬라이스가 범위를 벗어나 panic했다.
#[test]
fn set_field_text_shrinking_inner_field_fixes_outer_field_end() {
    use crate::model::document::Section;

    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Section::default());
    let mut para = Paragraph::default();
    para.text = "ABCD".to_string();
    para.controls.push(make_field_control(1)); // 외부 필드 컨트롤 (control_idx 0)
    para.controls.push(make_field_control(2)); // 내부 필드 컨트롤 (control_idx 1)
    // field_ranges는 파서(parser/body_text.rs의 field_stack)가 실제로 채우는 순서를
    // 그대로 재현한다: FIELD_BEGIN/FIELD_END는 스택(LIFO)으로 처리되므로 안쪽
    // 필드가 항상 바깥쪽 필드보다 먼저 닫히고, 따라서 field_ranges 배열에도
    // 항상 더 작은 인덱스로 먼저 push된다 (index 0 = 내부, index 1 = 외부).
    para.field_ranges.push(FieldRange { start_char_idx: 1, end_char_idx: 3, control_idx: 1 }); // 내부 (index 0)
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 4, control_idx: 0 }); // 외부 (index 1)
    core.document.sections[0].paragraphs.push(para);

    let location = FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] };

    // 내부 필드(index 0)를 빈 문자열로 치환 → "ABCD" -> "AD"
    core.set_field_text_at(&location, 0, "").unwrap();

    let text_len = {
        let para = &core.document.sections[0].paragraphs[0];
        assert_eq!(para.text, "AD");
        para.text.chars().count()
    };
    let outer = &core.document.sections[0].paragraphs[0].field_ranges[1];
    assert_eq!(outer.start_char_idx, 0);
    assert!(
        outer.end_char_idx <= text_len,
        "외부 필드 end_char_idx({})가 텍스트 길이({})를 초과하면 안 된다",
        outer.end_char_idx,
        text_len
    );
    assert_eq!(outer.end_char_idx, 2, "outer는 [0,2)로 축소되어야 한다");

    // 수정 전에는 보정되지 않은 [0,4) 범위로 여기서 panic이 발생했다.
    core.set_field_text_at(&location, 1, "Z").unwrap();
    assert_eq!(core.document.sections[0].paragraphs[0].text, "Z");
}

/// Verification test for codex-flagged issue: the new "containment" branch
/// (other_fr.start <= fr.start && other_fr.end >= fr.end) also matches a
/// field that merely ENDS exactly where an empty target field sits, even
/// though it does not nest/contain that field at all. Text "ABCD" with a
/// field [0,4) that closes right where a separate empty field [4,4) begins.
/// Inserting into the empty field must NOT grow the preceding closed field.
#[test]
fn set_field_text_does_not_grow_adjacent_field_ending_at_empty_target_start() {
    use crate::model::document::Section;

    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Section::default());
    let mut para = Paragraph::default();
    para.text = "ABCD".to_string();
    para.controls.push(make_field_control(1)); // preceding, already-closed field
    para.controls.push(make_field_control(2)); // target empty field right after it
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 4, control_idx: 0 });
    para.field_ranges.push(FieldRange { start_char_idx: 4, end_char_idx: 4, control_idx: 1 });
    core.document.sections[0].paragraphs.push(para);

    let location = FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] };

    // Insert "X" into the empty target field (index 1) at position 4.
    core.set_field_text_at(&location, 1, "X").unwrap();

    let para = &core.document.sections[0].paragraphs[0];
    assert_eq!(para.text, "ABCDX");
    let preceding = &para.field_ranges[0];
    assert_eq!(
        (preceding.start_char_idx, preceding.end_char_idx),
        (0, 4),
        "preceding closed field [0,4) must stay unchanged, not grow to include the new text"
    );
}

/// Regression test for a second codex-flagged issue found in the first fix
/// attempt: a genuine parent whose start coincides EXACTLY with an empty
/// child's position at the very front of the parent. Text "AB" with
/// parent=[0,2) wrapping the whole text and child=[0,0) nested right at
/// its start. Because the child is empty, `other.start(0) >= fr.end(0)`
/// was also true for the parent, so the (checked-first) "entirely after"
/// branch fired before the containment branch ever got a chance, shifting
/// the parent's start along with its end and producing (1,3) instead of
/// (0,3) after inserting "X" into the child.
///
/// The fix requires BOTH order signals the parser leaves behind for a
/// genuine parent -- opens first (smaller control_idx) AND closes last
/// (larger field_ranges index) -- checked *before* the "entirely after"
/// branch, which correctly excludes this from ever being misclassified.
#[test]
fn set_field_text_growing_child_at_parent_start_keeps_parent_start_fixed() {
    use crate::model::document::Section;

    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Section::default());
    let mut para = Paragraph::default();
    para.text = "AB".to_string();
    para.controls.push(make_field_control(1)); // parent control (control_idx 0, opens first)
    para.controls.push(make_field_control(2)); // child control (control_idx 1, opens second)
    // field_ranges in real parser (stack-pop) order: child closes first (index 0),
    // parent closes last (index 1).
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 0, control_idx: 1 }); // child
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 2, control_idx: 0 }); // parent
    core.document.sections[0].paragraphs.push(para);

    let location = FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] };

    // Insert "X" into the empty child (index 0) at position 0.
    core.set_field_text_at(&location, 0, "X").unwrap();

    let para = &core.document.sections[0].paragraphs[0];
    assert_eq!(para.text, "XAB");
    let parent = &para.field_ranges[1];
    assert_eq!(
        (parent.start_char_idx, parent.end_char_idx),
        (0, 3),
        "parent must stay anchored at 0 and only its end should grow, not (1,3)"
    );
}

/// Independent stress test (not from codex): three levels of nesting all
/// sharing the same start position, exercising the combined open-order
/// (control_idx) + close-order (field_ranges index) check across more
/// than one ancestor at once. text "ABC" with outer=[0,3) wrapping
/// middle=[0,2) wrapping inner=[0,1); all three start at 0. Shrinking the
/// innermost field to empty must adjust BOTH ancestors' ends (and only
/// their ends), leaving all three starts pinned at 0.
#[test]
fn set_field_text_shrinking_innermost_of_three_nested_fields_adjusts_both_ancestors() {
    use crate::model::document::Section;

    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Section::default());
    let mut para = Paragraph::default();
    para.text = "ABC".to_string();
    para.controls.push(make_field_control(1)); // outer, control_idx 0 (opens first)
    para.controls.push(make_field_control(2)); // middle, control_idx 1
    para.controls.push(make_field_control(3)); // inner, control_idx 2 (opens last)
    // Parser stack-pop order: innermost closes first.
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 2 }); // inner (index 0)
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 2, control_idx: 1 }); // middle (index 1)
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 3, control_idx: 0 }); // outer (index 2)
    core.document.sections[0].paragraphs.push(para);

    let location = FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] };

    // Shrink inner (index 0) to empty: "ABC" -> "BC"
    core.set_field_text_at(&location, 0, "").unwrap();

    let para = &core.document.sections[0].paragraphs[0];
    assert_eq!(para.text, "BC");
    assert_eq!(
        (para.field_ranges[0].start_char_idx, para.field_ranges[0].end_char_idx),
        (0, 0),
        "inner collapses to empty"
    );
    assert_eq!(
        (para.field_ranges[1].start_char_idx, para.field_ranges[1].end_char_idx),
        (0, 1),
        "middle must stay anchored at 0, only its end shrinks"
    );
    assert_eq!(
        (para.field_ranges[2].start_char_idx, para.field_ranges[2].end_char_idx),
        (0, 2),
        "outer must stay anchored at 0, only its end shrinks"
    );
}
