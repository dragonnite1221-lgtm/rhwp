//! 필드(누름틀) 값 조회/설정·누름틀 속성 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    // ─── 필드 API (Task 230) ─────────────────────────────────

    /// 문서 내 모든 필드 목록을 JSON 배열로 반환한다.
    ///
    /// 반환: `[{fieldId, fieldType, name, guide, command, value, location}]`
    #[wasm_bindgen(js_name = getFieldList)]
    pub fn get_field_list(&self) -> String {
        self.get_field_list_json()
    }

    /// field_id로 필드 값을 조회한다.
    ///
    /// 반환: `{ok, value}`
    #[wasm_bindgen(js_name = getFieldValue)]
    pub fn get_field_value(&self, field_id: u32) -> Result<String, JsValue> {
        self.get_field_value_by_id(field_id).map_err(|e| e.into())
    }

    /// 필드 이름으로 값을 조회한다.
    ///
    /// 반환: `{ok, fieldId, value}`
    #[wasm_bindgen(js_name = getFieldValueByName)]
    pub fn get_field_value_by_name_api(&self, name: &str) -> Result<String, JsValue> {
        self.get_field_value_by_name(name).map_err(|e| e.into())
    }

    /// field_id로 필드 값을 설정한다.
    ///
    /// 반환: `{ok, fieldId, oldValue, newValue}`
    #[wasm_bindgen(js_name = setFieldValue)]
    pub fn set_field_value(&mut self, field_id: u32, value: &str) -> Result<String, JsValue> {
        self.set_field_value_by_id(field_id, value)
            .map_err(|e| e.into())
    }

    /// 필드 이름으로 값을 설정한다.
    ///
    /// 반환: `{ok, fieldId, oldValue, newValue}`
    #[wasm_bindgen(js_name = setFieldValueByName)]
    pub fn set_field_value_by_name_api(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<String, JsValue> {
        self.set_field_value_by_name(name, value)
            .map_err(|e| e.into())
    }

}
