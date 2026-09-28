use super::*;
use crate::model::control::Control;
use crate::model::document::Section;
use crate::model::paragraph::{Paragraph, RangeTag};
use crate::model::table::{Cell, Table};

#[test]
fn test_serialize_hwp_rejects_oversized_nested_paragraph_metadata() {
    let oversized = Paragraph {
        range_tags: vec![RangeTag { start: 0, end: 1, tag: 0 }; u16::MAX as usize + 1],
        ..Default::default()
    };
    let doc = Document {
        sections: vec![Section {
            paragraphs: vec![Paragraph {
                controls: vec![Control::Table(Box::new(Table {
                    cells: vec![Cell { paragraphs: vec![oversized], ..Default::default() }],
                    ..Default::default()
                }))],
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    assert!(matches!(
        serialize_hwp(&doc),
        Err(SerializeError::UnsupportedInput(message)) if message.contains("range_tags")
    ));
}
