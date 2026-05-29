use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 문서 내 모든 책갈피 목록 반환
    #[wasm_bindgen(js_name = getBookmarks)]
    pub fn get_bookmarks(&self) -> Result<String, JsValue> {
        self.core.get_bookmarks_native().map_err(|e| e.into())
    }

    /// 책갈피 추가
    #[wasm_bindgen(js_name = addBookmark)]
    pub fn add_bookmark(
        &mut self,
        sec: u32,
        para: u32,
        char_offset: u32,
        name: &str,
    ) -> Result<String, JsValue> {
        self.core
            .add_bookmark_native(sec as usize, para as usize, char_offset as usize, name)
            .map_err(|e| e.into())
    }

    /// 책갈피 삭제
    #[wasm_bindgen(js_name = deleteBookmark)]
    pub fn delete_bookmark(
        &mut self,
        sec: u32,
        para: u32,
        ctrl_idx: u32,
    ) -> Result<String, JsValue> {
        self.core
            .delete_bookmark_native(sec as usize, para as usize, ctrl_idx as usize)
            .map_err(|e| e.into())
    }

    /// 책갈피 이름 변경
    #[wasm_bindgen(js_name = renameBookmark)]
    pub fn rename_bookmark(
        &mut self,
        sec: u32,
        para: u32,
        ctrl_idx: u32,
        new_name: &str,
    ) -> Result<String, JsValue> {
        self.core
            .rename_bookmark_native(sec as usize, para as usize, ctrl_idx as usize, new_name)
            .map_err(|e| e.into())
    }
}
