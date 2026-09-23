//! `field_query.rs`의 200줄 제한 분할: 커서 위치 → 필드 컨트롤 인덱스
//! 탐색, 문단 내 필드 제거, char_offsets 재계산 헬퍼. `field_query.rs`의
//! 주 `impl DocumentCore` 블록에서만 호출되므로 가시성은 `pub(super)`.

use crate::document_core::DocumentCore;
use crate::error::HwpError;
use crate::model::control::{Control, FieldType};
use crate::model::paragraph::Paragraph;

impl DocumentCore {
    /// 본문 문단에서 커서 위치의 필드 컨트롤 인덱스를 찾는다.
    pub(super) fn find_field_control_idx(
        &self, section_idx: usize, para_idx: usize, char_offset: usize,
        _cell_path: Option<(usize, usize, usize)>,
    ) -> Option<usize> {
        let para = self.document.sections.get(section_idx)?
            .paragraphs.get(para_idx)?;
        find_field_ctrl_idx_in_para(para, char_offset)
    }

    /// 셀/글상자 내 문단에서 커서 위치의 필드 컨트롤 인덱스를 찾는다.
    pub(super) fn find_field_control_idx_in_cell(
        &self, section_idx: usize, parent_para_idx: usize, control_idx: usize,
        cell_idx: usize, cell_para_idx: usize, char_offset: usize, is_textbox: bool,
    ) -> Option<usize> {
        let host = self.document.sections.get(section_idx)?
            .paragraphs.get(parent_para_idx)?;
        let ctrl = host.controls.get(control_idx)?;
        let para = if is_textbox {
            if let Control::Shape(shape) = ctrl {
                let tb = shape.drawing()?.text_box.as_ref()?;
                tb.paragraphs.get(cell_para_idx)?
            } else { return None; }
        } else {
            if let Control::Table(table) = ctrl {
                table.cells.get(cell_idx)?.paragraphs.get(cell_para_idx)?
            } else { return None; }
        };
        find_field_ctrl_idx_in_para(para, char_offset)
    }
}

/// 문단에서 커서 위치의 ClickHere 필드 컨트롤 인덱스를 반환한다.
pub(super) fn find_field_ctrl_idx_in_para(para: &Paragraph, char_offset: usize) -> Option<usize> {
    for fr in &para.field_ranges {
        if let Some(Control::Field(field)) = para.controls.get(fr.control_idx) {
            if field.field_type != FieldType::ClickHere { continue; }
            if char_offset >= fr.start_char_idx && char_offset <= fr.end_char_idx {
                return Some(fr.control_idx);
            }
        }
    }
    None
}

/// 문단 내 커서 위치의 누름틀 필드를 제거한다 (FieldRange만 삭제, 텍스트 유지).
pub(super) fn remove_field_in_para(para: &mut Paragraph, char_offset: usize) -> Result<(), HwpError> {
    let idx = para.field_ranges.iter().position(|fr| {
        if let Some(Control::Field(field)) = para.controls.get(fr.control_idx) {
            if field.field_type != FieldType::ClickHere {
                return false;
            }
            char_offset >= fr.start_char_idx && char_offset <= fr.end_char_idx
        } else {
            false
        }
    });
    match idx {
        Some(i) => {
            para.field_ranges.remove(i);
            Ok(())
        }
        None => Err(HwpError::InvalidField("커서 위치에 누름틀 필드 없음".into())),
    }
}

/// 문단의 char_offsets를 컨트롤/필드/텍스트 배치 순서에 맞게 재생성한다.
///
/// 원본 char_offsets에서 컨트롤 배치 패턴을 보존하면서,
/// 텍스트 길이 변경(필드 값 삽입)에 맞게 오프셋을 재계산한다.
pub(super) fn rebuild_char_offsets(para: &mut Paragraph) {
    let text_chars: Vec<char> = para.text.chars().collect();
    let text_len = text_chars.len();

    if text_len == 0 {
        para.char_offsets = Vec::new();
        return;
    }

    // 원본 char_offsets에서 첫 문자 이전 컨트롤 수 추정
    // (원본 gap / 8 = 컨트롤 수)
    let ctrls_before_text = if !para.char_offsets.is_empty() {
        para.char_offsets[0] as usize / 8
    } else {
        para.controls.len()
    };

    // FIELD_BEGIN: control_idx >= ctrls_before_text이고 start > 0인 필드의 시작 위치에 갭 필요
    let mut field_begin_at: Vec<usize> = vec![0; text_len + 1];
    for fr in &para.field_ranges {
        if fr.control_idx >= ctrls_before_text && fr.start_char_idx > 0 {
            let idx = fr.start_char_idx.min(text_len);
            field_begin_at[idx] += 1;
        }
    }

    // FIELD_END 수: field_ranges에서 end가 텍스트 범위 내인 것
    let mut field_end_at: Vec<usize> = vec![0; text_len + 1];
    for fr in &para.field_ranges {
        let idx = fr.end_char_idx.min(text_len);
        field_end_at[idx] += 1;
    }

    let mut offset: u32 = ctrls_before_text as u32 * 8;
    let mut new_offsets = Vec::with_capacity(text_len);

    for (i, ch) in text_chars.iter().enumerate() {
        // 이 문자 앞에 FIELD_BEGIN 컨트롤 갭 삽입
        offset += field_begin_at[i] as u32 * 8;
        // 이 문자 앞에 FIELD_END 마커 갭 삽입
        offset += field_end_at[i] as u32 * 8;

        new_offsets.push(offset);

        let char_size = match *ch {
            '\t' => 8,
            '\n' | '\u{00A0}' => 1,
            c => {
                let mut buf = [0u16; 2];
                c.encode_utf16(&mut buf).len() as u32
            }
        };
        offset += char_size;
    }

    para.char_offsets = new_offsets;
}
