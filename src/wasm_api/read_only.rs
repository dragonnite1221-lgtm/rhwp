use wasm_bindgen::prelude::*;

use super::HwpDocument;
use crate::document_core::DocumentCore;
use crate::model::control::Control;

#[wasm_bindgen]
impl HwpDocument {
    // ─── Phase 1: 기본 편집 보조 API ───────────────────────────

    /// 구역(Section) 수를 반환한다.
    #[wasm_bindgen(js_name = getSectionCount)]
    pub fn get_section_count(&self) -> u32 {
        self.document.sections.len() as u32
    }

    /// 구역 내 문단 수를 반환한다.
    #[wasm_bindgen(js_name = getParagraphCount)]
    pub fn get_paragraph_count(&self, section_idx: u32) -> Result<u32, JsValue> {
        self.get_paragraph_count_native(section_idx as usize)
            .map(|v| v as u32)
            .map_err(|e| e.into())
    }

    /// 문단의 글자 수(char 개수)를 반환한다.
    #[wasm_bindgen(js_name = getParagraphLength)]
    pub fn get_paragraph_length(&self, section_idx: u32, para_idx: u32) -> Result<u32, JsValue> {
        self.get_paragraph_length_native(section_idx as usize, para_idx as usize)
            .map(|v| v as u32)
            .map_err(|e| e.into())
    }

    /// 문단에 텍스트박스가 있는 Shape 컨트롤이 있으면 해당 control_index를 반환한다.
    /// 없으면 -1을 반환한다.
    #[wasm_bindgen(js_name = getTextBoxControlIndex)]
    pub fn get_textbox_control_index(&self, section_idx: u32, para_idx: u32) -> i32 {
        self.get_textbox_control_index_native(section_idx as usize, para_idx as usize)
    }

    /// 문서 트리에서 다음 편집 가능한 컨트롤/본문을 찾는다.
    /// delta=+1(앞), delta=-1(뒤). ctrl_idx=-1이면 본문 텍스트에서 출발.
    #[wasm_bindgen(js_name = findNextEditableControl)]
    pub fn find_next_editable_control(
        &self,
        section_idx: u32,
        para_idx: u32,
        ctrl_idx: i32,
        delta: i32,
    ) -> String {
        self.find_next_editable_control_native(
            section_idx as usize,
            para_idx as usize,
            ctrl_idx,
            delta,
        )
    }

    /// 커서에서 이전 방향으로 가장 가까운 선택 가능 컨트롤을 찾는다 (F11 키).
    #[wasm_bindgen(js_name = findNearestControlBackward)]
    pub fn find_nearest_control_backward(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> String {
        self.find_nearest_control_backward_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
    }

    /// 현재 위치 이후의 가장 가까운 선택 가능 컨트롤을 찾는다 (Shift+F11).
    #[wasm_bindgen(js_name = findNearestControlForward)]
    pub fn find_nearest_control_forward(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> String {
        self.find_nearest_control_forward_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
    }

    /// 문단 내 컨트롤의 텍스트 위치 배열을 반환한다.
    #[wasm_bindgen(js_name = getControlTextPositions)]
    pub fn get_control_text_positions(&self, section_idx: u32, para_idx: u32) -> String {
        let sections = &self.document.sections;
        if let Some(sec) = sections.get(section_idx as usize) {
            if let Some(para) = sec.paragraphs.get(para_idx as usize) {
                let positions = crate::document_core::find_control_text_positions(para);
                return format!(
                    "[{}]",
                    positions
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
        }
        "[]".to_string()
    }

    /// 문서 트리 DFS 기반 다음/이전 편집 가능 위치를 반환한다.
    /// context_json: NavContextEntry 배열의 JSON (빈 배열 "[]" = body)
    #[wasm_bindgen(js_name = navigateNextEditable)]
    pub fn navigate_next_editable_wasm(
        &self,
        sec: u32,
        para: u32,
        char_offset: u32,
        delta: i32,
        context_json: &str,
    ) -> String {
        let raw_context = DocumentCore::parse_nav_context(context_json);
        // TypeScript에서 ctrl_text_pos=0으로 전달되므로 실제 값으로 보정
        let context = DocumentCore::fix_context_text_positions(
            &self.core.document.sections,
            sec as usize,
            &raw_context,
        );

        // 오버플로우 링크 계산 (캐시됨)
        let overflow_links = self.core.get_overflow_links(sec as usize);

        // 컨텍스트가 있으면 (컨테이너 내부) 렌더링된 마지막 문단 인덱스를 조회
        let max_para = if !context.is_empty() {
            let last = &context[context.len() - 1];
            self.core.last_rendered_para_in_container(
                sec as usize,
                last.parent_para,
                last.ctrl_idx,
                last.cell_idx,
            )
        } else {
            None
        };

        let result = self.core.navigate_next_editable(
            sec as usize,
            para as usize,
            char_offset as usize,
            delta,
            &context,
            max_para,
            &overflow_links,
        );
        DocumentCore::nav_result_to_json(&result)
    }

    /// 문단에서 텍스트 부분 문자열을 반환한다 (Undo용 텍스트 보존).
    #[wasm_bindgen(js_name = getTextRange)]
    pub fn get_text_range(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        self.get_text_range_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            count as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 셀 내 문단 수를 반환한다.
    #[wasm_bindgen(js_name = getCellParagraphCount)]
    pub fn get_cell_paragraph_count(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
    ) -> Result<u32, JsValue> {
        self.get_cell_paragraph_count_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
        )
        .map(|v| v as u32)
        .map_err(|e| e.into())
    }

    /// 표 셀 내 문단의 글자 수를 반환한다.
    #[wasm_bindgen(js_name = getCellParagraphLength)]
    pub fn get_cell_paragraph_length(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
    ) -> Result<u32, JsValue> {
        self.get_cell_paragraph_length_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
        )
        .map(|v| v as u32)
        .map_err(|e| e.into())
    }

    /// 경로 기반: 셀/글상자 내 문단 수를 반환한다 (중첩 표/글상자 지원).
    #[wasm_bindgen(js_name = getCellParagraphCountByPath)]
    pub fn get_cell_paragraph_count_by_path(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
    ) -> Result<u32, JsValue> {
        let path = DocumentCore::parse_cell_path(path_json)?;
        let count = self
            .resolve_container_para_count_by_path(
                section_idx as usize,
                parent_para_idx as usize,
                &path,
            )
            .map_err(|e| -> JsValue { e.into() })?;
        Ok(count as u32)
    }

    /// 경로 기반: 셀 내 문단의 글자 수를 반환한다 (중첩 표 지원).
    #[wasm_bindgen(js_name = getCellParagraphLengthByPath)]
    pub fn get_cell_paragraph_length_by_path(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
    ) -> Result<u32, JsValue> {
        let path = DocumentCore::parse_cell_path(path_json)?;
        let para = self
            .resolve_paragraph_by_path(section_idx as usize, parent_para_idx as usize, &path)
            .map_err(|e| -> JsValue { e.into() })?;
        Ok(para.text.chars().count() as u32)
    }

    /// 표 셀의 텍스트 방향을 반환한다 (0=가로, 1=세로/영문눕힘, 2=세로/영문세움).
    #[wasm_bindgen(js_name = getCellTextDirection)]
    pub fn get_cell_text_direction(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
    ) -> Result<u32, JsValue> {
        let para = self
            .document
            .sections
            .get(section_idx as usize)
            .ok_or_else(|| JsValue::from_str("구역 인덱스 범위 초과"))?
            .paragraphs
            .get(parent_para_idx as usize)
            .ok_or_else(|| JsValue::from_str("문단 인덱스 범위 초과"))?;
        match para.controls.get(control_idx as usize) {
            Some(Control::Table(table)) => {
                let cell = table
                    .cells
                    .get(cell_idx as usize)
                    .ok_or_else(|| JsValue::from_str("셀 인덱스 범위 초과"))?;
                Ok(cell.text_direction as u32)
            }
            _ => Ok(0), // 글상자 등은 가로쓰기
        }
    }

    /// 표 셀 내 문단에서 텍스트 부분 문자열을 반환한다.
    #[wasm_bindgen(js_name = getTextInCell)]
    pub fn get_text_in_cell(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        self.get_text_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
            count as usize,
        )
        .map_err(|e| e.into())
    }

    // ─── Phase 1 끝 ─────────────────────────────────────────
}
