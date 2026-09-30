//! Public API save/reload regressions for paired field removal.
#[path = "nested_field_removal/support.rs"]
mod support;
use rhwp::model::control::Control;
use rhwp::model::paragraph::{FieldRange, Paragraph};
use rhwp::DocumentCore;
use support::{core, field, nested, paragraph, ranges};

#[test]
fn nested_field_removal_survives_body_and_cell_roundtrip() {
    for in_cell in [false, true] {
        for (cursor, expected, offsets) in [
            (1, vec![(10, 0, 2), (30, 3, 4)], vec![8, 9, 18, 27]),
            (0, vec![(20, 1, 2), (30, 3, 4)], vec![0, 9, 18, 27]),
        ] {
            let mut doc = core(nested(), in_cell);
            assert_eq!(
                ranges(paragraph(&doc, in_cell)),
                vec![(20, 1, 2), (10, 0, 2), (30, 3, 4)]
            );
            if in_cell {
                doc.remove_field_at_in_cell(0, 0, 0, 0, 0, cursor, false)
                    .unwrap();
            } else {
                doc.remove_field_at(0, 0, cursor).unwrap();
            }
            let reopened = DocumentCore::from_bytes(&doc.export_hwp_native().unwrap()).unwrap();
            let para = paragraph(&reopened, in_cell);
            assert_eq!(para.text, "ABCD");
            assert_eq!(ranges(para), expected, "in_cell={in_cell}, cursor={cursor}");
            assert_eq!(para.char_offsets, offsets);
            assert_eq!(para.controls.len(), 2);
        }
    }
}

#[test]
fn nested_trailing_field_removal_keeps_outer_field() {
    for in_cell in [false, true] {
        let mut para = nested();
        para.text = "AB".into();
        para.char_offsets = vec![8, 17];
        para.controls.truncate(2);
        para.field_ranges.truncate(2);
        para.char_count = 35;
        let mut doc = core(para, in_cell);
        if in_cell {
            doc.remove_field_at_in_cell(0, 0, 0, 0, 0, 1, false)
                .unwrap();
        } else {
            doc.remove_field_at(0, 0, 1).unwrap();
        }
        let reopened = DocumentCore::from_bytes(&doc.export_hwp_native().unwrap()).unwrap();
        let para = paragraph(&reopened, in_cell);
        assert_eq!(para.text, "AB");
        assert_eq!(ranges(para), vec![(10, 0, 2)]);
        assert_eq!(para.char_offsets, vec![8, 9]);
    }
}

#[test]
fn field_removal_preserves_shared_gap_controls_and_utf16_metadata() {
    use rhwp::model::control::Bookmark;
    use rhwp::model::paragraph::{CharShapeRef, LineSeg, RangeTag};
    // Zero-width inner around a bookmark, followed by a surrogate and a tab.
    let para = Paragraph {
        text: "😀\tZ".into(),
        char_offsets: vec![32, 34, 50],
        char_count: 60,
        controls: vec![
            field(10),
            field(20),
            Control::Bookmark(Bookmark {
                name: "inside".into(),
            }),
            Control::Bookmark(Bookmark {
                name: "after-tab".into(),
            }),
        ],
        field_ranges: vec![
            FieldRange {
                start_char_idx: 0,
                end_char_idx: 0,
                control_idx: 1,
            },
            FieldRange {
                start_char_idx: 0,
                end_char_idx: 3,
                control_idx: 0,
            },
        ],
        ..Paragraph::default()
    };
    // Four slots before text: outer BEGIN, inner BEGIN, bookmark, inner END.
    let mut doc = core(para, false);
    let para = &mut doc.document_mut().sections[0].paragraphs[0];
    para.ctrl_data_records = vec![Some(vec![1]), Some(vec![2]), Some(vec![3]), Some(vec![4])];
    para.char_shapes = vec![CharShapeRef {
        start_pos: 50,
        char_shape_id: 0,
    }];
    para.line_segs = vec![LineSeg {
        text_start: 50,
        ..LineSeg::default()
    }];
    para.range_tags = vec![RangeTag {
        start: 32,
        end: 51,
        tag: 7,
    }];
    doc.remove_field_at(0, 0, 0).unwrap();
    let para = paragraph(&doc, false);
    assert_eq!(
        para.ctrl_data_records,
        vec![Some(vec![1]), Some(vec![3]), Some(vec![4])]
    );
    assert_eq!(para.char_shapes[0].start_pos, 34);
    assert_eq!(para.line_segs[0].text_start, 34);
    assert_eq!((para.range_tags[0].start, para.range_tags[0].end), (16, 35));
    let reopened = DocumentCore::from_bytes(&doc.export_hwp_native().unwrap()).unwrap();
    let para = paragraph(&reopened, false);
    assert_eq!(para.text, "😀\tZ");
    assert_eq!(ranges(para), vec![(10, 0, 3)]);
    assert_eq!(para.char_offsets, vec![16, 18, 34]);
    assert!(matches!(&para.controls[1], Control::Bookmark(b) if b.name == "inside"));
    assert!(matches!(&para.controls[2], Control::Bookmark(b) if b.name == "after-tab"));
}
