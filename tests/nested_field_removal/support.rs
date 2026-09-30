//! Removing a ClickHere field must survive HWP save/reload without stealing a sibling's END.
use rhwp::model::control::{Control, Field, FieldType};
use rhwp::model::document::Section;
use rhwp::model::paragraph::{FieldRange, Paragraph};
use rhwp::model::table::{Cell, Table};
use rhwp::DocumentCore;

pub(super) fn field(id: u32) -> Control {
    Control::Field(Field {
        field_type: FieldType::ClickHere,
        command: String::new(),
        properties: 0,
        extra_properties: 0,
        field_id: id,
        ctrl_id: rhwp::parser::tags::FIELD_CLICKHERE,
        ctrl_data_name: None,
        memo_index: 0,
    })
}

pub(super) fn nested() -> Paragraph {
    // BEGIN outer, A, BEGIN inner, B, END inner, END outer, C, BEGIN later, D, END later.
    Paragraph {
        text: "ABCD".into(),
        char_offsets: vec![8, 17, 34, 43],
        char_count: 53,
        controls: vec![field(10), field(20), field(30)],
        field_ranges: vec![
            FieldRange {
                start_char_idx: 1,
                end_char_idx: 2,
                control_idx: 1,
            },
            FieldRange {
                start_char_idx: 0,
                end_char_idx: 2,
                control_idx: 0,
            },
            FieldRange {
                start_char_idx: 3,
                end_char_idx: 4,
                control_idx: 2,
            },
        ],
        ..Paragraph::default()
    }
}

pub(super) fn core(para: Paragraph, in_cell: bool) -> DocumentCore {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().unwrap();
    let para = if in_cell {
        let mut cell = Cell::new_empty(0, 0, 1000, 1000, 0);
        cell.paragraphs = vec![para];
        Paragraph {
            controls: vec![Control::Table(Box::new(Table {
                cells: vec![cell],
                ..Table::default()
            }))],
            ..Paragraph::default()
        }
    } else {
        para
    };
    core.document_mut().sections = vec![Section {
        paragraphs: vec![para],
        ..Section::default()
    }];
    // Exercise a parsed document, including raw_stream invalidation.
    DocumentCore::from_bytes(&core.export_hwp_native().unwrap()).unwrap()
}

pub(super) fn paragraph(core: &DocumentCore, in_cell: bool) -> &Paragraph {
    let para = &core.document().sections[0].paragraphs[0];
    if in_cell {
        let Control::Table(table) = &para.controls[0] else {
            panic!("table lost")
        };
        &table.cells[0].paragraphs[0]
    } else {
        para
    }
}

pub(super) fn ranges(para: &Paragraph) -> Vec<(u32, usize, usize)> {
    para.field_ranges
        .iter()
        .map(|r| {
            let Control::Field(f) = &para.controls[r.control_idx] else {
                panic!("field lost")
            };
            (f.field_id, r.start_char_idx, r.end_char_idx)
        })
        .collect()
}
