use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

use super::HwpDocument;
#[cfg(target_arch = "wasm32")]
use super::{normalize_canvas_scale, scaled_canvas_extent};

#[wasm_bindgen]
impl HwpDocument {
    /// 총 페이지 수를 반환한다.
    #[wasm_bindgen(js_name = pageCount)]
    pub fn page_count(&self) -> u32 {
        self.core.page_count()
    }

    /// 특정 페이지를 SVG 문자열로 렌더링한다.
    #[wasm_bindgen(js_name = renderPageSvg)]
    pub fn render_page_svg(&self, page_num: u32) -> Result<String, JsValue> {
        self.render_page_svg_native(page_num).map_err(|e| e.into())
    }

    /// 특정 페이지를 HTML 문자열로 렌더링한다.
    #[wasm_bindgen(js_name = renderPageHtml)]
    pub fn render_page_html(&self, page_num: u32) -> Result<String, JsValue> {
        self.render_page_html_native(page_num).map_err(|e| e.into())
    }

    /// 특정 페이지를 Canvas 명령 수로 반환한다.
    #[wasm_bindgen(js_name = renderPageCanvas)]
    pub fn render_page_canvas(&self, page_num: u32) -> Result<u32, JsValue> {
        self.render_page_canvas_native(page_num)
            .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = renderPageCanvasLegacy)]
    pub fn render_page_canvas_legacy(&self, page_num: u32) -> Result<u32, JsValue> {
        self.render_page_canvas_legacy_native(page_num)
            .map_err(|e| e.into())
    }

    /// 특정 페이지를 Canvas 2D에 직접 렌더링한다.
    ///
    /// WASM 환경에서만 사용 가능하다. Canvas 크기는 페이지 크기 × scale로 설정된다.
    /// scale이 0 이하이면 1.0으로 처리한다 (하위호환).
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = renderPageToCanvas)]
    pub fn render_page_to_canvas(
        &self,
        page_num: u32,
        canvas: &HtmlCanvasElement,
        scale: f64,
    ) -> Result<(), JsValue> {
        use crate::renderer::layer_renderer::LayerRenderer;
        use crate::renderer::web_canvas::WebCanvasRenderer;

        let tree = self
            .build_page_layer_tree(page_num)
            .map_err(JsValue::from)?;

        let scale = normalize_canvas_scale(tree.page_width, tree.page_height, scale)
            .map_err(JsValue::from_str)?;

        // 캔버스 크기 = 페이지 크기 × scale
        canvas.set_width(scaled_canvas_extent(tree.page_width, scale));
        canvas.set_height(scaled_canvas_extent(tree.page_height, scale));

        let mut renderer = WebCanvasRenderer::new(canvas)?;
        renderer.show_paragraph_marks = self.show_paragraph_marks;
        renderer.show_control_codes = self.show_control_codes;
        renderer.set_scale(scale);
        renderer.render_page(&tree).map_err(JsValue::from)?;
        Ok(())
    }

    /// 다층 레이어 필터를 적용한 Canvas 렌더링 (Task #516, Stage 5.2).
    ///
    /// `layer_kind`:
    /// - `"all"` → 모든 그림 렌더 (기본 `renderPageToCanvas` 와 동일)
    /// - `"flow"` → 본문 layer (BehindText / InFrontOfText 그림 제외)
    /// - `"behind"` → BehindText overlay layer
    /// - `"front"` → InFrontOfText overlay layer
    ///
    /// 본문 Canvas 와 overlay 컨테이너를 분리하는 다층 layer 아키텍처에서 사용.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = renderPageToCanvasFiltered)]
    pub fn render_page_to_canvas_filtered(
        &self,
        page_num: u32,
        canvas: &HtmlCanvasElement,
        scale: f64,
        layer_kind: &str,
    ) -> Result<(), JsValue> {
        use crate::model::shape::TextWrap;
        use crate::renderer::layer_renderer::LayerRenderer;
        use crate::renderer::web_canvas::{LayerFilter, WebCanvasRenderer};

        let filter = match layer_kind {
            "all" => LayerFilter::All,
            "flow" => LayerFilter::FlowOnly,
            "behind" => LayerFilter::WrapOnly(TextWrap::BehindText),
            "front" => LayerFilter::WrapOnly(TextWrap::InFrontOfText),
            _ => {
                return Err(JsValue::from_str(
                    "invalid layer_kind: 'all' | 'flow' | 'behind' | 'front'",
                ))
            }
        };

        let tree = self
            .build_page_layer_tree(page_num)
            .map_err(JsValue::from)?;

        let scale = normalize_canvas_scale(tree.page_width, tree.page_height, scale)
            .map_err(JsValue::from_str)?;

        canvas.set_width(scaled_canvas_extent(tree.page_width, scale));
        canvas.set_height(scaled_canvas_extent(tree.page_height, scale));

        let mut renderer = WebCanvasRenderer::new(canvas)?;
        renderer.show_paragraph_marks = self.show_paragraph_marks;
        renderer.show_control_codes = self.show_control_codes;
        renderer.set_scale(scale);
        renderer.set_layer_filter(filter);
        renderer.render_page(&tree).map_err(JsValue::from)?;
        Ok(())
    }

    /// 특정 페이지를 기존 PageRenderTree 경로로 Canvas 2D에 직접 렌더링한다.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = renderPageToCanvasLegacy)]
    pub fn render_page_to_canvas_legacy(
        &self,
        page_num: u32,
        canvas: &HtmlCanvasElement,
        scale: f64,
    ) -> Result<(), JsValue> {
        use crate::renderer::web_canvas::WebCanvasRenderer;

        let tree = self
            .build_page_tree_cached(page_num)
            .map_err(JsValue::from)?;

        let scale = normalize_canvas_scale(tree.root.bbox.width, tree.root.bbox.height, scale)
            .map_err(JsValue::from_str)?;

        // 캔버스 크기 = 페이지 크기 × scale
        canvas.set_width(scaled_canvas_extent(tree.root.bbox.width, scale));
        canvas.set_height(scaled_canvas_extent(tree.root.bbox.height, scale));

        let mut renderer = WebCanvasRenderer::new(canvas)?;
        renderer.show_paragraph_marks = self.show_paragraph_marks;
        renderer.show_control_codes = self.show_control_codes;
        renderer.set_scale(scale);
        renderer.render_tree(&tree);
        Ok(())
    }

    /// 페이지 렌더 트리를 JSON 문자열로 반환한다.
    #[wasm_bindgen(js_name = getPageRenderTree)]
    pub fn get_page_render_tree(&self, page_num: u32) -> Result<String, JsValue> {
        let tree = self
            .build_page_tree_cached(page_num)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(tree.root.to_json())
    }

    /// 페이지 레이어 트리를 JSON 문자열로 반환한다.
    #[wasm_bindgen(js_name = getPageLayerTree)]
    pub fn get_page_layer_tree(&self, page_num: u32) -> Result<String, JsValue> {
        self.get_page_layer_tree_native(page_num)
            .map_err(|e| e.into())
    }

    /// CanvasKit direct replay/compat overlay 정책 진단을 JSON 문자열로 반환한다.
    ///
    /// `mode` 는 `"default"` 또는 `"compat"` 를 받는다. 빈 문자열은 `"default"` 로 처리한다.
    #[wasm_bindgen(js_name = getCanvasKitReplayPlan)]
    pub fn get_canvaskit_replay_plan(&self, page_num: u32, mode: &str) -> Result<String, JsValue> {
        self.get_canvaskit_replay_plan_native(page_num, mode)
            .map_err(|e| e.into())
    }

    /// 페이지 overlay 이미지 정보만 JSON 문자열로 반환한다.
    #[wasm_bindgen(js_name = getPageOverlayImages)]
    pub fn get_page_overlay_images(&self, page_num: u32) -> Result<String, JsValue> {
        self.get_page_overlay_images_native(page_num)
            .map_err(|e| e.into())
    }

    /// 페이지 정보를 JSON 문자열로 반환한다.
    #[wasm_bindgen(js_name = getPageInfo)]
    pub fn get_page_info(&self, page_num: u32) -> Result<String, JsValue> {
        self.get_page_info_native(page_num).map_err(|e| e.into())
    }

    /// 구역의 용지 설정(PageDef)을 HWPUNIT 원본값으로 반환한다.
    #[wasm_bindgen(js_name = getPageDef)]
    pub fn get_page_def(&self, section_idx: u32) -> Result<String, JsValue> {
        self.get_page_def_native(section_idx as usize)
            .map_err(|e| e.into())
    }

    /// 구역의 용지 설정(PageDef)을 변경하고 재페이지네이션한다.
    #[wasm_bindgen(js_name = setPageDef)]
    pub fn set_page_def(&mut self, section_idx: u32, json: &str) -> Result<String, JsValue> {
        self.set_page_def_native(section_idx as usize, json)
            .map_err(|e| e.into())
    }

    /// 구역 정의(SectionDef)를 JSON으로 반환한다.
    #[wasm_bindgen(js_name = getSectionDef)]
    pub fn get_section_def(&self, section_idx: u32) -> Result<String, JsValue> {
        self.get_section_def_native(section_idx as usize)
            .map_err(|e| e.into())
    }

    /// 구역 정의(SectionDef)를 변경하고 재페이지네이션한다.
    #[wasm_bindgen(js_name = setSectionDef)]
    pub fn set_section_def(&mut self, section_idx: u32, json: &str) -> Result<String, JsValue> {
        self.set_section_def_native(section_idx as usize, json)
            .map_err(|e| e.into())
    }

    /// 모든 구역의 SectionDef를 일괄 변경하고 재페이지네이션한다.
    #[wasm_bindgen(js_name = setSectionDefAll)]
    pub fn set_section_def_all(&mut self, json: &str) -> Result<String, JsValue> {
        self.set_section_def_all_native(json).map_err(|e| e.into())
    }

    /// 현재 구역의 다단 설정을 JSON으로 반환한다.
    #[wasm_bindgen(js_name = getColumnDef)]
    pub fn get_column_def(&self, section_idx: u32) -> Result<String, JsValue> {
        let sec = self
            .core
            .document
            .sections
            .get(section_idx as usize)
            .ok_or_else(|| JsValue::from_str("구역 인덱스 범위 초과"))?;
        let col_def = HwpDocument::find_initial_column_def(&sec.paragraphs);
        let col_type = match col_def.column_type {
            crate::model::page::ColumnType::Normal => 0,
            crate::model::page::ColumnType::Distribute => 1,
            crate::model::page::ColumnType::Parallel => 2,
        };
        Ok(format!(
            "{{\"columnCount\":{},\"columnType\":{},\"sameWidth\":{},\"spacing\":{}}}",
            col_def.column_count, col_type, col_def.same_width, col_def.spacing,
        ))
    }

    /// 문서 정보를 JSON 문자열로 반환한다.
    #[wasm_bindgen(js_name = getDocumentInfo)]
    pub fn get_document_info(&self) -> String {
        self.core.get_document_info()
    }

    /// 특정 페이지의 텍스트 레이아웃 정보를 JSON 문자열로 반환한다.
    ///
    /// 각 TextRun의 위치, 텍스트, 글자별 X 좌표 경계값을 포함한다.
    #[wasm_bindgen(js_name = getPageTextLayout)]
    pub fn get_page_text_layout(&self, page_num: u32) -> Result<String, JsValue> {
        self.get_page_text_layout_native(page_num)
            .map_err(|e| e.into())
    }

    /// 컨트롤(표, 이미지 등) 레이아웃 정보를 반환한다.
    #[wasm_bindgen(js_name = getPageControlLayout)]
    pub fn get_page_control_layout(&self, page_num: u32) -> Result<String, JsValue> {
        self.get_page_control_layout_native(page_num)
            .map_err(|e| e.into())
    }
}
