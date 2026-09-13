//! `clear_initial_field_texts` 회귀 테스트 모음 (rhwp-1).
//!
//! `document.rs` 소스 파일이 커지는 것을 막기 위해 인라인 `mod
//! clear_initial_field_texts_tests { ... }` 블록을 별도 파일로 분리했다
//! (내용은 변경 없음).

    use super::*;
    use crate::model::control::{Control, Field, FieldType};
    use crate::model::document::{Document, Section};
    use crate::model::paragraph::{FieldRange, Paragraph};

    fn click_here_field(ctrl_id: u32, guide: &str) -> Control {
        Control::Field(Field {
            field_type: FieldType::ClickHere,
            command: format!("Direction:wstring:{}:{}", guide.chars().count(), guide),
            properties: 0, // bit 15 == 0 → 초기 상태
            extra_properties: 0,
            field_id: ctrl_id,
            ctrl_id,
            ctrl_data_name: None,
            memo_index: 0,
        })
    }

    /// 트리거: 문단 텍스트 "X"를 범위가 완전히 동일한([0,1)) 중첩 ClickHere 필드
    /// 두 개가 감싼 문서를 로드한다. 수정 전에는 두 번째(뒤에서부터 처리되는)
    /// 삭제가 stale end=1을 빈 문자열에 적용해 슬라이스 out-of-range panic이 발생했다.
    #[test]
    fn clear_initial_field_texts_survives_nested_same_range_fields() {
        let mut doc = Document::default();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        para.text = "X".to_string();
        para.controls.push(click_here_field(1, "X")); // 외부 필드 (control_idx 0)
        para.controls.push(click_here_field(2, "X")); // 내부 필드 (control_idx 1)
        para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 0 });
        para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 1 });
        section.paragraphs.push(para);
        doc.sections.push(section);

        // 수정 전: 두 번째 반복에서 chars[1..] (len 0) 슬라이스로 panic.
        DocumentCore::clear_initial_field_texts(&mut doc);

        let para = &doc.sections[0].paragraphs[0];
        assert_eq!(para.text, "", "두 안내문이 모두 제거되어 빈 문자열이어야 한다");
        for fr in &para.field_ranges {
            assert_eq!(fr.start_char_idx, 0);
            assert_eq!(fr.end_char_idx, 0, "빈 필드로 정규화되어야 한다 (start==end)");
        }
    }

    /// 서로 다른 범위의 중첩 안내문(겹치지 않음)도 정상적으로 정규화되는지 확인.
    #[test]
    fn clear_initial_field_texts_handles_disjoint_ranges() {
        let mut doc = Document::default();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        para.text = "AB".to_string();
        para.controls.push(click_here_field(1, "A"));
        para.controls.push(click_here_field(2, "B"));
        para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 0 });
        para.field_ranges.push(FieldRange { start_char_idx: 1, end_char_idx: 2, control_idx: 1 });
        section.paragraphs.push(para);
        doc.sections.push(section);

        DocumentCore::clear_initial_field_texts(&mut doc);

        let para = &doc.sections[0].paragraphs[0];
        assert_eq!(para.text, "");
        assert_eq!(para.field_ranges[0].start_char_idx, 0);
        assert_eq!(para.field_ranges[0].end_char_idx, 0);
        assert_eq!(para.field_ranges[1].start_char_idx, 0);
        assert_eq!(para.field_ranges[1].end_char_idx, 0);
    }

    /// Regression test for codex-flagged issue: nested but NON-identical ranges
    /// sharing an end index. Parser emits field_ranges in stack-pop order, so an
    /// inner field's range is pushed BEFORE its enclosing outer field's range.
    /// If both guide texts match (inner="B" at [1,2), outer="AB" at [0,2) over
    /// text "AB"), processing removals in reverse handles the OUTER field first
    /// (it has the higher field_ranges index). The old shift-based adjustment
    /// only decremented `end_char_idx` when it was >= the removed end, leaving
    /// `start_char_idx` untouched — producing an inverted range (start > end)
    /// for the inner field. The fix maps both endpoints through a monotonic
    /// position function, so both endpoints collapse to the same value and the
    /// inner field ends up correctly normalized to an empty range.
    #[test]
    fn clear_initial_field_texts_normalizes_nested_unequal_range_sharing_endpoint() {
        let mut doc = Document::default();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        para.text = "AB".to_string();
        // controls[0] = outer field (guide "AB"), controls[1] = inner field (guide "B")
        para.controls.push(click_here_field(1, "AB"));
        para.controls.push(click_here_field(2, "B"));
        // field_ranges pushed in parser (stack-pop / inner-first) order:
        // index 0 = inner [1,2) over control_idx 1, index 1 = outer [0,2) over control_idx 0
        para.field_ranges.push(FieldRange { start_char_idx: 1, end_char_idx: 2, control_idx: 1 });
        para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 2, control_idx: 0 });
        section.paragraphs.push(para);
        doc.sections.push(section);

        DocumentCore::clear_initial_field_texts(&mut doc);

        let para = &doc.sections[0].paragraphs[0];
        for fr in &para.field_ranges {
            assert!(
                fr.start_char_idx <= fr.end_char_idx,
                "field_range must never be inverted: {:?}", fr
            );
        }
        assert_eq!(para.text, "", "both guide texts covered the whole paragraph");
        // inner (index 0) and outer (index 1) both collapse to the empty range at 0
        assert_eq!(para.field_ranges[0].start_char_idx, 0);
        assert_eq!(para.field_ranges[0].end_char_idx, 0);
        assert_eq!(para.field_ranges[1].start_char_idx, 0);
        assert_eq!(para.field_ranges[1].end_char_idx, 0);
    }

    /// Independent stress test (not from codex): three-level-deep nesting that
    /// all share the same START instead of the same END (the mirror image of
    /// the case above). text "ABC" with inner="A" [0,1), middle="AB" [0,2),
    /// outer="ABC" [0,3), all guide texts matching. This is a distinct
    /// reproduction from the codex-flagged one, added because this exact
    /// field-offset logic has already been "fixed" twice before with adjacent
    /// cases left broken — a triple-nesting case with a different shared
    /// endpoint gives independent confidence the monotonic mapping generalizes
    /// beyond the two-field case it was derived from.
    #[test]
    fn clear_initial_field_texts_normalizes_triple_nested_fields_sharing_start() {
        let mut doc = Document::default();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        para.text = "ABC".to_string();
        para.controls.push(click_here_field(1, "ABC")); // outer, control_idx 0
        para.controls.push(click_here_field(2, "AB"));  // middle, control_idx 1
        para.controls.push(click_here_field(3, "A"));   // inner, control_idx 2
        // Parser stack-pop order: innermost closes first, so it is pushed first.
        para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 1, control_idx: 2 }); // inner
        para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 2, control_idx: 1 }); // middle
        para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 3, control_idx: 0 }); // outer
        section.paragraphs.push(para);
        doc.sections.push(section);

        DocumentCore::clear_initial_field_texts(&mut doc);

        let para = &doc.sections[0].paragraphs[0];
        assert_eq!(para.text, "", "all three guide texts covered the whole paragraph");
        for fr in &para.field_ranges {
            assert!(
                fr.start_char_idx <= fr.end_char_idx,
                "field_range must never be inverted: {:?}", fr
            );
            assert_eq!((fr.start_char_idx, fr.end_char_idx), (0, 0));
        }
    }
