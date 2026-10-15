//! 표 계산식 실행 및 대상 셀 검증

use crate::model::control::Control;
use crate::document_core::DocumentCore;
use crate::error::HwpError;

impl DocumentCore {
    /// 표 셀에서 계산식을 실행하고 결과를 반환한다.
    ///
    /// # Arguments
    /// * `section_idx` - 구역 인덱스
    /// * `parent_para_idx` - 표가 포함된 문단 인덱스
    /// * `control_idx` - 표 컨트롤 인덱스
    /// * `target_row` - 계산식이 입력될 셀 행 (0-based)
    /// * `target_col` - 계산식이 입력될 셀 열 (0-based)
    /// * `formula` - 계산식 문자열 (예: "=SUM(A1:A5)")
    /// * `write_result` - true이면 결과를 셀에 기록
    pub fn evaluate_table_formula(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        control_idx: usize,
        target_row: usize,
        target_col: usize,
        formula: &str,
        write_result: bool,
    ) -> Result<String, HwpError> {
        // 표 가져오기
        let section = self.document.sections.get(section_idx)
            .ok_or_else(|| HwpError::RenderError("구역 초과".into()))?;
        let para = section.paragraphs.get(parent_para_idx)
            .ok_or_else(|| HwpError::RenderError("문단 초과".into()))?;
        let table = match para.controls.get(control_idx) {
            Some(Control::Table(t)) => t,
            _ => return Err(HwpError::RenderError("표 컨트롤이 아님".into())),
        };

        let row_count = table.row_count as usize;
        let col_count = table.col_count as usize;

        if target_row >= row_count || target_col >= col_count {
            return Err(HwpError::RenderError("대상 셀이 표 범위를 벗어남".into()));
        }
        let cell_idx = table.cell_index_at(target_row as u16, target_col as u16)
            .ok_or_else(|| HwpError::RenderError("대상 셀이 없음".into()))?;

        // 셀 값 조회 함수: 셀의 첫 문단 텍스트를 숫자로 파싱
        let cells = &table.cells;
        let get_cell = |col: usize, row: usize| -> Option<f64> {
            if row >= row_count || col >= col_count { return None; }
            let idx = row * col_count + col;
            cells.get(idx)
                .and_then(|cell| cell.paragraphs.first())
                .and_then(|p| parse_cell_number(&p.text))
        };

        let ctx = crate::document_core::table_calc::TableContext {
            row_count,
            col_count,
            current_row: target_row,
            current_col: target_col,
        };

        let result = crate::document_core::table_calc::evaluate_formula(formula, &ctx, &get_cell)
            .map_err(|e| HwpError::RenderError(format!("계산식 오류: {}", e)))?;

        // 결과를 셀에 기록
        if write_result {
            let section_mut = self.document.sections.get_mut(section_idx).unwrap();
            let para_mut = section_mut.paragraphs.get_mut(parent_para_idx).unwrap();
            if let Some(Control::Table(ref mut t)) = para_mut.controls.get_mut(control_idx) {
                if let Some(cell) = t.cells.get_mut(cell_idx) {
                    if let Some(cell_para) = cell.paragraphs.first_mut() {
                        // 정수이면 정수로, 아니면 소수점 표시
                        let text = if result == result.trunc() && result.abs() < 1e15 {
                            format!("{}", result as i64)
                        } else {
                            format!("{}", result)
                        };
                        cell_para.text = text;
                        let new_len = cell_para.text.chars().count();
                        cell_para.char_offsets = (0..new_len).map(|i| i as u32).collect();
                    }
                }
            }
            // raw_stream 무효화
            if let Some(sec) = self.document.sections.get_mut(section_idx) {
                sec.raw_stream = None;
            }
            self.recompose_section(section_idx);
        }

        Ok(format!("{{\"ok\":true,\"result\":{},\"formula\":{}}}", result, json_escape(formula)))
    }
}

/// 셀 텍스트에서 숫자를 추출한다 (콤마 제거, 공백 무시).
fn parse_cell_number(text: &str) -> Option<f64> {
    let cleaned: String = text.chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

fn json_escape(s: &str) -> String {
    let mut r = String::with_capacity(s.len() + 2);
    r.push('"');
    for c in s.chars() {
        match c {
            '"' => r.push_str("\\\""),
            '\\' => r.push_str("\\\\"),
            _ => r.push(c),
        }
    }
    r.push('"');
    r
}
