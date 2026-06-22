//! 양식 개체(Form Object) API (WASM 바인딩).
//!
//! 페이지 좌표 기반 양식 개체 hit-test, 양식 값 조회/설정 등
//! `HwpDocument`의 양식 개체 관련 `#[wasm_bindgen]` 메서드 모음.
//! 모두 `self.core`의 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 페이지 좌표에서 양식 개체를 찾는다.
    ///
    /// 반환: `{found, sec, para, ci, formType, name, value, caption, text, bbox}`
    #[wasm_bindgen(js_name = getFormObjectAt)]
    pub fn get_form_object_at(&self, page_num: u32, x: f64, y: f64) -> Result<String, JsValue> {
        self.core
            .get_form_object_at_native(page_num, x, y)
            .map_err(|e| e.into())
    }

    /// 양식 개체 값을 조회한다.
    ///
    /// 반환: `{ok, formType, name, value, text, caption, enabled}`
    #[wasm_bindgen(js_name = getFormValue)]
    pub fn get_form_value(&self, sec: u32, para: u32, ci: u32) -> Result<String, JsValue> {
        self.core
            .get_form_value_native(sec as usize, para as usize, ci as usize)
            .map_err(|e| e.into())
    }

    /// 양식 개체 값을 설정한다.
    ///
    /// value_json: `{"value":1}` 또는 `{"text":"입력값"}`
    /// 반환: `{ok}`
    #[wasm_bindgen(js_name = setFormValue)]
    pub fn set_form_value(
        &mut self,
        sec: u32,
        para: u32,
        ci: u32,
        value_json: &str,
    ) -> Result<String, JsValue> {
        self.core
            .set_form_value_native(sec as usize, para as usize, ci as usize, value_json)
            .map_err(|e| e.into())
    }

    /// 셀 내부 양식 개체 값을 설정한다.
    ///
    /// table_para: 표를 포함한 최상위 문단 인덱스
    /// table_ci: 표 컨트롤 인덱스
    /// cell_idx: 셀 인덱스
    /// cell_para: 셀 내 문단 인덱스
    /// form_ci: 셀 내 양식 컨트롤 인덱스
    /// value_json: `{"value":1}` 또는 `{"text":"입력값"}`
    /// 반환: `{ok}`
    #[wasm_bindgen(js_name = setFormValueInCell)]
    pub fn set_form_value_in_cell(
        &mut self,
        sec: u32,
        table_para: u32,
        table_ci: u32,
        cell_idx: u32,
        cell_para: u32,
        form_ci: u32,
        value_json: &str,
    ) -> Result<String, JsValue> {
        self.core
            .set_form_value_in_cell_native(
                sec as usize,
                table_para as usize,
                table_ci as usize,
                cell_idx as usize,
                cell_para as usize,
                form_ci as usize,
                value_json,
            )
            .map_err(|e| e.into())
    }

    /// 양식 개체 상세 정보를 반환한다 (properties 포함).
    ///
    /// 반환: `{ok, formType, name, value, text, caption, enabled, width, height, foreColor, backColor, properties}`
    #[wasm_bindgen(js_name = getFormObjectInfo)]
    pub fn get_form_object_info(&self, sec: u32, para: u32, ci: u32) -> Result<String, JsValue> {
        self.core
            .get_form_object_info_native(sec as usize, para as usize, ci as usize)
            .map_err(|e| e.into())
    }
}
