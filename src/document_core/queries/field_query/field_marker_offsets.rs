//! Locate the exact UTF-16 slots removed by a field deletion.
use crate::model::paragraph::Paragraph;

/// Use the writer's placement rules, including zero-width fields and non-field
/// controls sharing a gap. Text indices alone cannot identify a marker's slot.
pub(super) fn field_marker_offsets(para: &Paragraph, target: usize) -> Option<[u32; 2]> {
    let data = crate::serializer::body_text::serialize_para_text(para);
    let units: Vec<u16> = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();
    let mut stack = Vec::new();
    let mut control_idx = 0;
    let mut pos = 0;
    while pos < units.len() {
        let code = units[pos];
        if code == 13 {
            break;
        } // PARA_BREAK, matching the parser.
        if code == 3 {
            stack.push((control_idx, pos as u32));
        } else if code == 4 {
            if let Some((idx, begin)) = stack.pop() {
                if idx == target {
                    return Some([begin, pos as u32]);
                }
            }
        }
        if matches!(code, 1..=3 | 11..=12 | 14..=18 | 21..=23) {
            control_idx += 1;
        }
        pos += if matches!(code, 1..=9 | 11..=12 | 14..=23) {
            8
        } else {
            1
        };
    }
    None
}

/// Collapse both eight-unit markers, also clamping positions inside a marker
/// to its former start. All coordinates here are from the original paragraph.
pub(super) fn without_markers(pos: u32, markers: [u32; 2]) -> u32 {
    pos - markers
        .iter()
        .map(|&start| pos.saturating_sub(start).min(8))
        .sum::<u32>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::control::{Control, Field};
    use crate::model::paragraph::FieldRange;

    #[test]
    fn every_serialized_control_advances_exactly_one_control_slot() {
        let controls = vec![
            Control::SectionDef(Box::default()),
            Control::ColumnDef(Default::default()),
            Control::Table(Box::default()),
            Control::Shape(Box::new(crate::model::shape::ShapeObject::Rectangle(
                Default::default(),
            ))),
            Control::Picture(Box::default()),
            Control::Header(Box::default()),
            Control::Footer(Box::default()),
            Control::Footnote(Box::default()),
            Control::Endnote(Box::default()),
            Control::AutoNumber(Default::default()),
            Control::NewNumber(Default::default()),
            Control::PageNumberPos(Default::default()),
            Control::Bookmark(Default::default()),
            Control::Hyperlink(Default::default()),
            Control::Ruby(Default::default()),
            Control::CharOverlap(Default::default()),
            Control::PageHide(Default::default()),
            Control::HiddenComment(Box::default()),
            Control::Equation(Box::default()),
            Control::Field(Field::default()),
            Control::Form(Box::default()),
            Control::Unknown(Default::default()),
        ];
        for control in controls {
            let outer_field = matches!(control, Control::Field(_));
            let mut para = Paragraph {
                text: "A".into(),
                char_offsets: vec![16],
                controls: vec![control, Control::Field(Field::default())],
                field_ranges: vec![FieldRange {
                    start_char_idx: 0,
                    end_char_idx: 1,
                    control_idx: 1,
                }],
                ..Paragraph::default()
            };
            if outer_field {
                para.field_ranges.push(FieldRange {
                    start_char_idx: 0,
                    end_char_idx: 1,
                    control_idx: 0,
                });
            }
            assert_eq!(
                field_marker_offsets(&para, 1),
                Some([8, 17]),
                "{:?}",
                para.controls[0]
            );
        }
    }
}
