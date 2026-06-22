//! 수식(Equation) 삭제·속성·미리보기 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    // ─── Equation(수식) API ──────────────────────────────

    /// 수식 컨트롤을 문단에서 삭제한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = deleteEquationControl)]
    pub fn delete_equation_control(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.delete_equation_control_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 수식 컨트롤의 속성을 조회한다.
    ///
    /// 반환: JSON `{ script, fontSize, color, baseline, fontName }`
    #[wasm_bindgen(js_name = getEquationProperties)]
    pub fn get_equation_properties(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: i32,
        cell_para_idx: i32,
    ) -> Result<String, JsValue> {
        let ci = if cell_idx >= 0 {
            Some(cell_idx as usize)
        } else {
            None
        };
        let cpi = if cell_para_idx >= 0 {
            Some(cell_para_idx as usize)
        } else {
            None
        };
        self.get_equation_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            ci,
            cpi,
        )
        .map_err(|e| e.into())
    }

    /// 수식 컨트롤의 속성을 변경한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = setEquationProperties)]
    pub fn set_equation_properties(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: i32,
        cell_para_idx: i32,
        props_json: &str,
    ) -> Result<String, JsValue> {
        let ci = if cell_idx >= 0 {
            Some(cell_idx as usize)
        } else {
            None
        };
        let cpi = if cell_para_idx >= 0 {
            Some(cell_para_idx as usize)
        } else {
            None
        };
        self.set_equation_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            ci,
            cpi,
            props_json,
        )
        .map_err(|e| e.into())
    }

    /// 수식 스크립트를 SVG로 렌더링하여 반환한다 (미리보기 전용).
    ///
    /// 반환: 완전한 `<svg>` 문자열
    #[wasm_bindgen(js_name = renderEquationPreview)]
    pub fn render_equation_preview(
        &self,
        script: &str,
        font_size_hwpunit: u32,
        color: u32,
    ) -> Result<String, JsValue> {
        self.render_equation_preview_native(script, font_size_hwpunit, color)
            .map_err(|e| e.into())
    }

}
