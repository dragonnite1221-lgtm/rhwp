//! 각주(footnote) hit-test · 커서 위치 API (WASM 바인딩).
//!
//! 각주 영역 hit-test, 각주 내 커서 위치, 본문 각주 마커 hit-test 등
//! `HwpDocument`의 각주 위치 관련 `#[wasm_bindgen]` 메서드 모음.
//! 모두 `document_core`의 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 각주 영역 히트테스트
    #[wasm_bindgen(js_name = hitTestFootnote)]
    pub fn hit_test_footnote(&self, page_num: u32, x: f64, y: f64) -> Result<String, JsValue> {
        self.hit_test_footnote_native(page_num, x, y)
            .map_err(|e| e.into())
    }

    /// 각주 내부 텍스트 히트테스트
    #[wasm_bindgen(js_name = hitTestInFootnote)]
    pub fn hit_test_in_footnote(&self, page_num: u32, x: f64, y: f64) -> Result<String, JsValue> {
        self.hit_test_in_footnote_native(page_num, x, y)
            .map_err(|e| e.into())
    }

    /// 페이지의 각주 참조 정보
    #[wasm_bindgen(js_name = getPageFootnoteInfo)]
    pub fn get_page_footnote_info(
        &self,
        page_num: u32,
        footnote_index: u32,
    ) -> Result<String, JsValue> {
        self.get_page_footnote_info_native(page_num, footnote_index as usize)
            .map_err(|e| e.into())
    }

    /// 각주 내 커서 렉트 계산
    #[wasm_bindgen(js_name = getCursorRectInFootnote)]
    pub fn get_cursor_rect_in_footnote(
        &self,
        page_num: u32,
        footnote_index: u32,
        fn_para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.get_cursor_rect_in_footnote_native(
            page_num,
            footnote_index as usize,
            fn_para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 본문 인라인 각주 마커 히트테스트
    #[wasm_bindgen(js_name = hitTestBodyFootnoteMarker)]
    pub fn hit_test_body_footnote_marker(
        &self,
        page_num: u32,
        x: f64,
        y: f64,
    ) -> Result<String, JsValue> {
        self.hit_test_body_footnote_marker_native(page_num, x, y)
            .map_err(|e| e.into())
    }
}
