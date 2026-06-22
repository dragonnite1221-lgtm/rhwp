//! 클립보드 HTML · 이미지 추출 API (WASM 바인딩).
//!
//! 선택 영역/컨트롤을 HTML로 변환, HTML 붙여넣기, 컨트롤 이미지 데이터 추출 등
//! `HwpDocument`의 클립보드 HTML 관련 `#[wasm_bindgen]` 메서드 모음.
//! 모두 `document_core`의 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 선택 영역을 HTML 문자열로 변환한다 (본문).
    #[wasm_bindgen(js_name = exportSelectionHtml)]
    pub fn export_selection_html(
        &self,
        section_idx: u32,
        start_para_idx: u32,
        start_char_offset: u32,
        end_para_idx: u32,
        end_char_offset: u32,
    ) -> Result<String, JsValue> {
        self.export_selection_html_native(
            section_idx as usize,
            start_para_idx as usize,
            start_char_offset as usize,
            end_para_idx as usize,
            end_char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 선택 영역을 HTML 문자열로 변환한다 (셀 내부).
    #[wasm_bindgen(js_name = exportSelectionInCellHtml)]
    pub fn export_selection_in_cell_html(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        start_cell_para_idx: u32,
        start_char_offset: u32,
        end_cell_para_idx: u32,
        end_char_offset: u32,
    ) -> Result<String, JsValue> {
        self.export_selection_in_cell_html_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            start_cell_para_idx as usize,
            start_char_offset as usize,
            end_cell_para_idx as usize,
            end_char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 컨트롤 객체를 HTML 문자열로 변환한다.
    #[wasm_bindgen(js_name = exportControlHtml)]
    pub fn export_control_html(
        &self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.export_control_html_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 컨트롤의 이미지 바이너리 데이터를 반환한다 (Uint8Array).
    #[wasm_bindgen(js_name = getControlImageData)]
    pub fn get_control_image_data(
        &self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
    ) -> Result<Vec<u8>, JsValue> {
        self.get_control_image_data_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 컨트롤의 이미지 MIME 타입을 반환한다.
    #[wasm_bindgen(js_name = getControlImageMime)]
    pub fn get_control_image_mime(
        &self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_control_image_mime_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// HTML 문자열을 파싱하여 캐럿 위치에 삽입한다 (본문).
    #[wasm_bindgen(js_name = pasteHtml)]
    pub fn paste_html(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        html: &str,
    ) -> Result<String, JsValue> {
        self.paste_html_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            html,
        )
        .map_err(|e| e.into())
    }

    /// HTML 문자열을 파싱하여 셀 내부 캐럿 위치에 삽입한다.
    #[wasm_bindgen(js_name = pasteHtmlInCell)]
    pub fn paste_html_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
        html: &str,
    ) -> Result<String, JsValue> {
        self.paste_html_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
            html,
        )
        .map_err(|e| e.into())
    }
}
