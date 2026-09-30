//! Locate the exact UTF-16 slots removed by a field deletion.
use crate::model::paragraph::Paragraph;

/// Use the writer's placement rules, including zero-width fields and non-field
/// controls sharing a gap. Text indices alone cannot identify a marker's slot.
pub(super) fn field_marker_offsets(para: &Paragraph, target: usize) -> Option<[u32; 2]> {
    let data = crate::serializer::body_text::serialize_para_text(para);
    let units: Vec<u16> = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let mut stack = Vec::new();
    let mut control_idx = 0;
    let mut pos = 0;
    while pos < units.len() {
        let code = units[pos];
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
