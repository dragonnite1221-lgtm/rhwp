//! WASM ↔ JavaScript 공개 API
//!
//! wasm-bindgen을 통해 JavaScript에서 호출 가능한 API를 정의한다.
//! 주요 API:
//! - `HwpDocument::new(data)` - HWP 파일 로드
//! - `HwpDocument::page_count()` - 페이지 수 조회
//! - `HwpDocument::render_page_svg(page_num)` - SVG로 렌더링
//! - `HwpDocument::render_page_html(page_num)` - HTML로 렌더링

// 하위 호환성: tests.rs에서 super::json_escape 등으로 접근 가능하도록 재내보내기
pub(crate) use crate::document_core::helpers::*;

use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

use crate::document_core::{DocumentCore, DEFAULT_FALLBACK_FONT};
use crate::error::HwpError;
use crate::model::control::Control;
use crate::model::document::{Document, Section};
use crate::model::page::ColumnDef;
use crate::model::paragraph::Paragraph;
use crate::model::path::{path_from_flat, DocumentPath, PathSegment};
use crate::renderer::canvas::CanvasRenderer;
use crate::renderer::composer::{
    compose_paragraph, compose_section, reflow_line_segs, ComposedParagraph,
};
use crate::renderer::height_measurer::{HeightMeasurer, MeasuredSection, MeasuredTable};
use crate::renderer::html::HtmlRenderer;
use crate::renderer::layout::LayoutEngine;
use crate::renderer::page_layout::PageLayoutInfo;
use crate::renderer::pagination::{PaginationResult, Paginator};
use crate::renderer::render_tree::PageRenderTree;
use crate::renderer::scheduler::{RenderEvent, RenderObserver, RenderScheduler, Viewport};
use crate::renderer::style_resolver::{
    resolve_font_substitution, resolve_styles, ResolvedStyleSet,
};
use crate::renderer::svg::SvgRenderer;
use crate::renderer::DEFAULT_DPI;

impl From<HwpError> for JsValue {
    fn from(err: HwpError) -> Self {
        JsValue::from_str(&err.to_string())
    }
}

#[cfg(any(target_arch = "wasm32", test))]
const MAX_CANVAS_DIMENSION: f64 = 16_384.0;

#[cfg(any(target_arch = "wasm32", test))]
fn normalize_canvas_scale(
    page_width: f64,
    page_height: f64,
    requested_scale: f64,
) -> Result<f64, &'static str> {
    if !page_width.is_finite()
        || !page_height.is_finite()
        || page_width <= 0.0
        || page_height <= 0.0
    {
        return Err("invalid page dimensions");
    }

    let scale = if requested_scale <= 0.0 || !requested_scale.is_finite() {
        1.0
    } else {
        requested_scale.clamp(0.25, 12.0)
    };

    let scaled_width = page_width * scale;
    let scaled_height = page_height * scale;
    if !scaled_width.is_finite() || !scaled_height.is_finite() {
        return Ok((MAX_CANVAS_DIMENSION / page_width)
            .min(MAX_CANVAS_DIMENSION / page_height)
            .min(scale));
    }

    if scaled_width > MAX_CANVAS_DIMENSION || scaled_height > MAX_CANVAS_DIMENSION {
        Ok((MAX_CANVAS_DIMENSION / page_width)
            .min(MAX_CANVAS_DIMENSION / page_height)
            .min(scale))
    } else {
        Ok(scale)
    }
}

#[cfg(target_arch = "wasm32")]
fn scaled_canvas_extent(page_extent: f64, scale: f64) -> u32 {
    (page_extent * scale).max(1.0).min(MAX_CANVAS_DIMENSION) as u32
}

/// WASM에서 사용할 HWP 문서 래퍼
///
/// 도메인 로직은 `DocumentCore`에 구현되어 있으며,
/// `Deref`/`DerefMut`를 통해 투명하게 접근한다.
#[wasm_bindgen]
pub struct HwpDocument {
    core: DocumentCore,
}

impl std::ops::Deref for HwpDocument {
    type Target = DocumentCore;
    fn deref(&self) -> &DocumentCore {
        &self.core
    }
}

impl std::ops::DerefMut for HwpDocument {
    fn deref_mut(&mut self) -> &mut DocumentCore {
        &mut self.core
    }
}

/// 네이티브(비-WASM) 환경용 래퍼 메서드.
///
/// 테스트 및 CLI 환경에서 `HwpDocument::from_bytes()` 등을 직접 호출할 수 있도록 한다.
impl HwpDocument {
    pub fn from_bytes(data: &[u8]) -> Result<HwpDocument, HwpError> {
        DocumentCore::from_bytes(data).map(|core| HwpDocument { core })
    }

    pub fn find_initial_column_def(paragraphs: &[Paragraph]) -> ColumnDef {
        DocumentCore::find_initial_column_def(paragraphs)
    }

    pub fn find_column_def_for_paragraph(paragraphs: &[Paragraph], para_idx: usize) -> ColumnDef {
        DocumentCore::find_column_def_for_paragraph(paragraphs, para_idx)
    }
}

#[wasm_bindgen]
impl HwpDocument {
    /// HWP 파일 바이트를 로드하여 문서 객체를 생성한다.
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<HwpDocument, JsValue> {
        DocumentCore::from_bytes(data)
            .map(|core| HwpDocument { core })
            .map_err(|e| e.into())
    }

    /// 빈 문서 생성 (테스트/미리보기용)
    #[wasm_bindgen(js_name = createEmpty)]
    pub fn create_empty() -> HwpDocument {
        let mut core = DocumentCore::new_empty();
        core.paginate();
        HwpDocument { core }
    }

    /// 내장 템플릿에서 빈 문서를 생성한다.
    ///
    /// saved/blank2010.hwp를 WASM 바이너리에 포함하여 유효한 HWP 문서를 즉시 생성.
    /// DocInfo raw_stream이 온전하므로 FIX-4 워크어라운드와 호환됨.
    #[wasm_bindgen(js_name = createBlankDocument)]
    pub fn create_blank_document(&mut self) -> Result<String, JsValue> {
        self.create_blank_document_native().map_err(|e| e.into())
    }

    /// 문단부호(¶) 표시 여부를 설정한다.
    #[wasm_bindgen(js_name = setShowParagraphMarks)]
    pub fn set_show_paragraph_marks(&mut self, enabled: bool) {
        self.show_paragraph_marks = enabled;
        self.invalidate_page_tree_cache();
    }

    /// 조판부호 표시 여부를 반환한다.
    #[wasm_bindgen(js_name = getShowControlCodes)]
    pub fn get_show_control_codes(&self) -> bool {
        self.show_control_codes
    }

    /// 조판부호 표시 여부를 설정한다 (개체 마커 + 문단부호 포함).
    #[wasm_bindgen(js_name = setShowControlCodes)]
    pub fn set_show_control_codes(&mut self, enabled: bool) {
        self.show_control_codes = enabled;
        self.invalidate_page_tree_cache();
    }

    /// 투명선 표시 여부를 반환한다.
    #[wasm_bindgen(js_name = getShowTransparentBorders)]
    pub fn get_show_transparent_borders(&self) -> bool {
        self.show_transparent_borders
    }

    /// 투명선 표시 여부를 설정한다.
    #[wasm_bindgen(js_name = setShowTransparentBorders)]
    pub fn set_show_transparent_borders(&mut self, enabled: bool) {
        self.show_transparent_borders = enabled;
        self.invalidate_page_tree_cache();
    }

    #[wasm_bindgen(js_name = setClipEnabled)]
    pub fn set_clip_enabled(&mut self, enabled: bool) {
        self.clip_enabled = enabled;
        self.invalidate_page_tree_cache();
    }

    /// 디버그 오버레이 표시 여부를 설정한다.
    pub fn set_debug_overlay(&mut self, enabled: bool) {
        self.debug_overlay = enabled;
    }

    /// LINE_SEG vpos-reset 강제 분리 적용 여부를 설정한다.
    /// 변경 시 페이지네이션 결과가 달라지므로 모든 섹션을 재페이지네이션한다.
    pub fn set_respect_vpos_reset(&mut self, enabled: bool) {
        if self.respect_vpos_reset != enabled {
            self.respect_vpos_reset = enabled;
            // 모든 섹션 dirty 마킹 후 즉시 재페이지네이션
            for d in self.core.dirty_sections.iter_mut() {
                *d = true;
            }
            self.invalidate_page_tree_cache();
            self.core.paginate();
        }
    }

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

        let tree = self.build_page_layer_tree(page_num).map_err(JsValue::from)?;

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
        use crate::renderer::layer_renderer::LayerRenderer;
        use crate::renderer::web_canvas::{LayerFilter, WebCanvasRenderer};
        use crate::model::shape::TextWrap;

        let filter = match layer_kind {
            "all" => LayerFilter::All,
            "flow" => LayerFilter::FlowOnly,
            "behind" => LayerFilter::WrapOnly(TextWrap::BehindText),
            "front" => LayerFilter::WrapOnly(TextWrap::InFrontOfText),
            _ => return Err(JsValue::from_str(
                "invalid layer_kind: 'all' | 'flow' | 'behind' | 'front'",
            )),
        };

        let tree = self.build_page_layer_tree(page_num).map_err(JsValue::from)?;

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
            .map_err(|e| JsValue::from(e))?;

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
        let sec = self.core.document.sections.get(section_idx as usize)
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

    /// DPI를 설정한다.
    #[wasm_bindgen(js_name = setDpi)]
    pub fn set_dpi(&mut self, dpi: f64) {
        self.core.set_dpi(dpi);
    }

    /// 파일 이름을 설정한다 (머리말/꼬리말 필드 치환용).
    #[wasm_bindgen(js_name = setFileName)]
    pub fn set_file_name(&mut self, name: &str) {
        self.core.file_name = name.to_string();
    }

    /// 현재 DPI를 반환한다.
    #[wasm_bindgen(js_name = getDpi)]
    pub fn get_dpi(&self) -> f64 {
        self.dpi
    }

    /// 대체 폰트 경로를 설정한다.
    #[wasm_bindgen(js_name = setFallbackFont)]
    pub fn set_fallback_font(&mut self, path: &str) {
        self.fallback_font = path.to_string();
    }

    /// 현재 대체 폰트 경로를 반환한다.
    #[wasm_bindgen(js_name = getFallbackFont)]
    pub fn get_fallback_font(&self) -> String {
        self.fallback_font.clone()
    }

    /// 문단에 텍스트를 삽입한다.
    ///
    /// 삽입 후 구역을 재구성하고 재페이지네이션한다.
    /// 반환값: JSON `{"ok":true,"charOffset":<new_offset>}`
    #[wasm_bindgen(js_name = insertText)]
    pub fn insert_text(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        text: &str,
    ) -> Result<String, JsValue> {
        self.insert_text_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            text,
        )
        .map_err(|e| e.into())
    }

    /// 논리적 오프셋으로 텍스트를 삽입한다.
    ///
    /// logical_offset: 텍스트 문자 + 인라인 컨트롤을 각각 1로 세는 위치.
    /// 예: "abc[표]XYZ" → a(0) b(1) c(2) [표](3) X(4) Y(5) Z(6)
    /// logical_offset=4이면 표 뒤의 X 앞에 삽입.
    /// 반환값: JSON `{"ok":true,"logicalOffset":<new_logical_offset>}`
    #[wasm_bindgen(js_name = insertTextLogical)]
    pub fn insert_text_logical(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        logical_offset: u32,
        text: &str,
    ) -> Result<String, JsValue> {
        let sec = section_idx as usize;
        let pi = para_idx as usize;
        if sec >= self.document.sections.len() || pi >= self.document.sections[sec].paragraphs.len()
        {
            return Err(JsValue::from_str("인덱스 범위 초과"));
        }
        let (text_offset, _) = crate::document_core::helpers::logical_to_text_offset(
            &self.document.sections[sec].paragraphs[pi],
            logical_offset as usize,
        );
        let result = self.insert_text_native(sec, pi, text_offset, text)?;
        // 삽입 후 논리적 오프셋 반환
        let new_text_offset = text_offset + text.chars().count();
        let new_logical = crate::document_core::helpers::text_to_logical_offset(
            &self.document.sections[sec].paragraphs[pi],
            new_text_offset,
        );
        Ok(format!("{{\"ok\":true,\"logicalOffset\":{}}}", new_logical))
    }

    /// 문단의 논리적 길이를 반환한다 (텍스트 문자 + 인라인 컨트롤 수).
    #[wasm_bindgen(js_name = getLogicalLength)]
    pub fn get_logical_length(&self, section_idx: u32, para_idx: u32) -> Result<u32, JsValue> {
        let sec = section_idx as usize;
        let pi = para_idx as usize;
        if sec >= self.document.sections.len() || pi >= self.document.sections[sec].paragraphs.len()
        {
            return Err(JsValue::from_str("인덱스 범위 초과"));
        }
        Ok(crate::document_core::helpers::logical_paragraph_length(
            &self.document.sections[sec].paragraphs[pi],
        ) as u32)
    }

    /// 논리적 오프셋 → 텍스트 오프셋 변환.
    #[wasm_bindgen(js_name = logicalToTextOffset)]
    pub fn logical_to_text_offset(
        &self,
        section_idx: u32,
        para_idx: u32,
        logical_offset: u32,
    ) -> Result<u32, JsValue> {
        let sec = section_idx as usize;
        let pi = para_idx as usize;
        if sec >= self.document.sections.len() || pi >= self.document.sections[sec].paragraphs.len()
        {
            return Err(JsValue::from_str("인덱스 범위 초과"));
        }
        let (text_offset, _) = crate::document_core::helpers::logical_to_text_offset(
            &self.document.sections[sec].paragraphs[pi],
            logical_offset as usize,
        );
        Ok(text_offset as u32)
    }

    /// 텍스트 오프셋 → 논리적 오프셋 변환.
    #[wasm_bindgen(js_name = textToLogicalOffset)]
    pub fn text_to_logical_offset(
        &self,
        section_idx: u32,
        para_idx: u32,
        text_offset: u32,
    ) -> Result<u32, JsValue> {
        let sec = section_idx as usize;
        let pi = para_idx as usize;
        if sec >= self.document.sections.len() || pi >= self.document.sections[sec].paragraphs.len()
        {
            return Err(JsValue::from_str("인덱스 범위 초과"));
        }
        Ok(crate::document_core::helpers::text_to_logical_offset(
            &self.document.sections[sec].paragraphs[pi],
            text_offset as usize,
        ) as u32)
    }

    /// 문단에서 텍스트를 삭제한다.
    ///
    /// 삭제 후 구역을 재구성하고 재페이지네이션한다.
    /// 반환값: JSON `{"ok":true,"charOffset":<offset_after_delete>}`
    #[wasm_bindgen(js_name = deleteText)]
    pub fn delete_text(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        self.delete_text_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            count as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 셀 내부 문단에 텍스트를 삽입한다.
    ///
    /// 반환값: JSON `{"ok":true,"charOffset":<new_offset>}`
    #[wasm_bindgen(js_name = insertTextInCell)]
    pub fn insert_text_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
        text: &str,
    ) -> Result<String, JsValue> {
        self.insert_text_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
            text,
        )
        .map_err(|e| e.into())
    }

    /// 표 셀 내부 문단에서 텍스트를 삭제한다.
    ///
    /// 반환값: JSON `{"ok":true,"charOffset":<offset_after_delete>}`
    #[wasm_bindgen(js_name = deleteTextInCell)]
    pub fn delete_text_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        self.delete_text_in_cell_native(
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

    /// 셀 내부 문단을 분할한다 (셀 내 Enter 키).
    ///
    /// 반환값: JSON `{"ok":true,"cellParaIndex":<new_idx>,"charOffset":0}`
    #[wasm_bindgen(js_name = splitParagraphInCell)]
    pub fn split_paragraph_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.split_paragraph_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 셀 내부 문단을 이전 문단에 병합한다 (셀 내 Backspace at start).
    ///
    /// 반환값: JSON `{"ok":true,"cellParaIndex":<prev_idx>,"charOffset":<merge_point>}`
    #[wasm_bindgen(js_name = mergeParagraphInCell)]
    pub fn merge_paragraph_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
    ) -> Result<String, JsValue> {
        self.merge_paragraph_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
        )
        .map_err(|e| e.into())
    }





    /// 캐럿 위치에서 문단을 분할한다 (Enter 키).
    ///
    /// char_offset 이후의 텍스트가 새 문단으로 이동한다.
    /// 반환값: JSON `{"ok":true,"paraIdx":<new_para_idx>,"charOffset":0}`
    #[wasm_bindgen(js_name = splitParagraph)]
    pub fn split_paragraph(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.split_paragraph_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 강제 쪽 나누기 삽입 (Ctrl+Enter)
    #[wasm_bindgen(js_name = insertPageBreak)]
    pub fn insert_page_break(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.insert_page_break_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 단 나누기 삽입 (Ctrl+Shift+Enter)
    #[wasm_bindgen(js_name = insertColumnBreak)]
    pub fn insert_column_break(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.insert_column_break_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 다단 설정 변경
    /// column_type: 0=일반, 1=배분, 2=평행
    /// same_width: 0=다른 너비, 1=같은 너비
    #[wasm_bindgen(js_name = setColumnDef)]
    pub fn set_column_def(
        &mut self,
        section_idx: u32,
        column_count: u32,
        column_type: u32,
        same_width: u32,
        spacing_hu: i32,
    ) -> Result<String, JsValue> {
        self.set_column_def_native(
            section_idx as usize,
            column_count as u16,
            column_type as u8,
            same_width != 0,
            spacing_hu as i16,
        )
        .map_err(|e| e.into())
    }

    /// 현재 문단을 이전 문단에 병합한다 (Backspace at start).
    ///
    /// para_idx의 텍스트가 para_idx-1에 결합되고 para_idx는 삭제된다.
    /// 반환값: JSON `{"ok":true,"paraIdx":<merged_para_idx>,"charOffset":<merge_point>}`
    #[wasm_bindgen(js_name = mergeParagraph)]
    pub fn merge_paragraph(&mut self, section_idx: u32, para_idx: u32) -> Result<String, JsValue> {
        self.merge_paragraph_native(section_idx as usize, para_idx as usize)
            .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = deleteParagraph)]
    pub fn delete_paragraph(&mut self, section_idx: u32, para_idx: u32) -> Result<String, JsValue> {
        self.delete_paragraph_native(section_idx as usize, para_idx as usize)
            .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = insertParagraph)]
    pub fn insert_paragraph(&mut self, section_idx: u32, para_idx: u32) -> Result<String, JsValue> {
        self.insert_paragraph_native(section_idx as usize, para_idx as usize)
            .map_err(|e| e.into())
    }

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

    // ─── Phase 2: 커서/히트 테스트 API ──────────────────────────

    /// 커서 위치의 픽셀 좌표를 반환한다.
    ///
    /// 반환: JSON `{"pageIndex":N,"x":F,"y":F,"height":F}`
    #[wasm_bindgen(js_name = getCursorRect)]
    pub fn get_cursor_rect(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.get_cursor_rect_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 페이지 좌표에서 문서 위치를 찾는다.
    ///
    /// 반환: JSON `{"sectionIndex":N,"paragraphIndex":N,"charOffset":N}`
    #[wasm_bindgen(js_name = hitTest)]
    pub fn hit_test(&self, page_num: u32, x: f64, y: f64) -> Result<String, JsValue> {
        self.hit_test_native(page_num, x, y).map_err(|e| e.into())
    }

    /// 머리말/꼬리말 내 커서 위치의 픽셀 좌표를 반환한다.
    ///
    /// preferred_page: 선호 페이지 (더블클릭한 페이지). -1이면 첫 번째 발견 페이지 사용.
    /// 반환: JSON `{"pageIndex":N,"x":F,"y":F,"height":F}`
    #[wasm_bindgen(js_name = getCursorRectInHeaderFooter)]
    pub fn get_cursor_rect_in_header_footer(
        &self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: u32,
        char_offset: u32,
        preferred_page: i32,
    ) -> Result<String, JsValue> {
        self.get_cursor_rect_in_header_footer_native(
            section_idx as usize,
            is_header,
            apply_to,
            hf_para_idx as usize,
            char_offset as usize,
            preferred_page,
        )
        .map_err(|e| e.into())
    }

    /// 페이지 좌표가 머리말/꼬리말 영역에 해당하는지 판별한다.
    ///
    /// 반환: JSON `{"hit":true/false,"isHeader":bool,"sectionIndex":N,"applyTo":N}`
    #[wasm_bindgen(js_name = hitTestHeaderFooter)]
    pub fn hit_test_header_footer(&self, page_num: u32, x: f64, y: f64) -> Result<String, JsValue> {
        self.hit_test_header_footer_native(page_num, x, y)
            .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 내부 텍스트 히트테스트.
    ///
    /// 편집 모드에서 클릭한 좌표의 문단·문자 위치를 반환.
    /// 반환: JSON `{"hit":true,"paraIndex":N,"charOffset":N,"cursorRect":{...}}`
    #[wasm_bindgen(js_name = hitTestInHeaderFooter)]
    pub fn hit_test_in_header_footer(
        &self,
        page_num: u32,
        is_header: bool,
        x: f64,
        y: f64,
    ) -> Result<String, JsValue> {
        self.hit_test_in_header_footer_native(page_num, is_header, x, y)
            .map_err(|e| e.into())
    }


    /// 표 셀 내부 커서 위치의 픽셀 좌표를 반환한다.
    ///
    /// 반환: JSON `{"pageIndex":N,"x":F,"y":F,"height":F}`
    #[wasm_bindgen(js_name = getCursorRectInCell)]
    pub fn get_cursor_rect_in_cell(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.get_cursor_rect_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    // ─── Phase 3: 커서 이동 API ──────────────────────────────

    /// 문단 내 줄 정보를 반환한다 (커서 수직 이동/Home/End용).
    ///
    /// 반환: JSON `{"lineIndex":N,"lineCount":N,"charStart":N,"charEnd":N}`
    #[wasm_bindgen(js_name = getLineInfo)]
    pub fn get_line_info(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.get_line_info_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 셀 내 문단의 줄 정보를 반환한다.
    ///
    /// 반환: JSON `{"lineIndex":N,"lineCount":N,"charStart":N,"charEnd":N}`
    #[wasm_bindgen(js_name = getLineInfoInCell)]
    pub fn get_line_info_in_cell(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.get_line_info_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 문서에 저장된 캐럿 위치를 반환한다 (문서 로딩 시 캐럿 자동 배치용).
    ///
    /// 반환: JSON `{"sectionIndex":N,"paragraphIndex":N,"charOffset":N}`
    #[wasm_bindgen(js_name = getCaretPosition)]
    pub fn get_caret_position(&self) -> Result<String, JsValue> {
        self.get_caret_position_native().map_err(|e| e.into())
    }







    // ─── Phase 4: Selection API ──────────────────────────────

    /// 본문 선택 영역의 줄별 사각형을 반환한다.
    ///
    /// 반환: JSON 배열 `[{"pageIndex":N,"x":F,"y":F,"width":F,"height":F}, ...]`
    #[wasm_bindgen(js_name = getSelectionRects)]
    pub fn get_selection_rects(
        &self,
        section_idx: u32,
        start_para_idx: u32,
        start_char_offset: u32,
        end_para_idx: u32,
        end_char_offset: u32,
    ) -> Result<String, JsValue> {
        self.get_selection_rects_native(
            section_idx as usize,
            start_para_idx as usize,
            start_char_offset as usize,
            end_para_idx as usize,
            end_char_offset as usize,
            None,
        )
        .map_err(|e| e.into())
    }

    /// 셀 내 선택 영역의 줄별 사각형을 반환한다.
    ///
    /// 반환: JSON 배열 `[{"pageIndex":N,"x":F,"y":F,"width":F,"height":F}, ...]`
    #[wasm_bindgen(js_name = getSelectionRectsInCell)]
    pub fn get_selection_rects_in_cell(
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
        self.get_selection_rects_native(
            section_idx as usize,
            start_cell_para_idx as usize,
            start_char_offset as usize,
            end_cell_para_idx as usize,
            end_char_offset as usize,
            Some((
                parent_para_idx as usize,
                control_idx as usize,
                cell_idx as usize,
            )),
        )
        .map_err(|e| e.into())
    }

    /// 본문 선택 영역을 삭제한다.
    ///
    /// 반환: JSON `{"ok":true,"paraIdx":N,"charOffset":N}`
    #[wasm_bindgen(js_name = deleteRange)]
    pub fn delete_range(
        &mut self,
        section_idx: u32,
        start_para_idx: u32,
        start_char_offset: u32,
        end_para_idx: u32,
        end_char_offset: u32,
    ) -> Result<String, JsValue> {
        self.delete_range_native(
            section_idx as usize,
            start_para_idx as usize,
            start_char_offset as usize,
            end_para_idx as usize,
            end_char_offset as usize,
            None,
        )
        .map_err(|e| e.into())
    }

    /// 셀 내 선택 영역을 삭제한다.
    ///
    /// 반환: JSON `{"ok":true,"paraIdx":N,"charOffset":N}`
    #[wasm_bindgen(js_name = deleteRangeInCell)]
    pub fn delete_range_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        start_cell_para_idx: u32,
        start_char_offset: u32,
        end_cell_para_idx: u32,
        end_char_offset: u32,
    ) -> Result<String, JsValue> {
        self.delete_range_native(
            section_idx as usize,
            start_cell_para_idx as usize,
            start_char_offset as usize,
            end_cell_para_idx as usize,
            end_char_offset as usize,
            Some((
                parent_para_idx as usize,
                control_idx as usize,
                cell_idx as usize,
            )),
        )
        .map_err(|e| e.into())
    }

    // ─── Phase 4 끝 ─────────────────────────────────────────

    // ─── Phase 3 끝 ─────────────────────────────────────────

    // ─── Phase 2 끝 ─────────────────────────────────────────

    /// 캐럿 위치의 글자 속성을 조회한다.
    ///
    /// 반환값: JSON 객체 (fontFamily, fontSize, bold, italic, underline, strikethrough, textColor 등)
    #[wasm_bindgen(js_name = getCharPropertiesAt)]
    pub fn get_char_properties_at(
        &self,
        sec_idx: usize,
        para_idx: usize,
        char_offset: usize,
    ) -> Result<String, JsValue> {
        self.get_char_properties_at_native(sec_idx, para_idx, char_offset)
            .map_err(|e| e.into())
    }

    /// 셀 내부 문단의 글자 속성을 조회한다.
    #[wasm_bindgen(js_name = getCellCharPropertiesAt)]
    pub fn get_cell_char_properties_at(
        &self,
        sec_idx: usize,
        parent_para_idx: usize,
        control_idx: usize,
        cell_idx: usize,
        cell_para_idx: usize,
        char_offset: usize,
    ) -> Result<String, JsValue> {
        self.get_cell_char_properties_at_native(
            sec_idx,
            parent_para_idx,
            control_idx,
            cell_idx,
            cell_para_idx,
            char_offset,
        )
        .map_err(|e| e.into())
    }

    /// 캐럿 위치의 문단 속성을 조회한다.
    ///
    /// 반환값: JSON 객체 (alignment, lineSpacing, marginLeft, marginRight, indent 등)
    #[wasm_bindgen(js_name = getParaPropertiesAt)]
    pub fn get_para_properties_at(
        &self,
        sec_idx: usize,
        para_idx: usize,
    ) -> Result<String, JsValue> {
        self.get_para_properties_at_native(sec_idx, para_idx)
            .map_err(|e| e.into())
    }

    /// 셀 내부 문단의 문단 속성을 조회한다.
    #[wasm_bindgen(js_name = getCellParaPropertiesAt)]
    pub fn get_cell_para_properties_at(
        &self,
        sec_idx: usize,
        parent_para_idx: usize,
        control_idx: usize,
        cell_idx: usize,
        cell_para_idx: usize,
    ) -> Result<String, JsValue> {
        self.get_cell_para_properties_at_native(
            sec_idx,
            parent_para_idx,
            control_idx,
            cell_idx,
            cell_para_idx,
        )
        .map_err(|e| e.into())
    }


    /// 특정 문단의 스타일을 조회한다.
    ///
    /// 반환값: JSON { id, name }
    #[wasm_bindgen(js_name = getStyleAt)]
    pub fn get_style_at(&self, sec_idx: u32, para_idx: u32) -> String {
        let sec = sec_idx as usize;
        let para = para_idx as usize;
        let style_id = self
            .core
            .document
            .sections
            .get(sec)
            .and_then(|s| s.paragraphs.get(para))
            .map(|p| p.style_id as usize)
            .unwrap_or(0);
        let name = self
            .core
            .document
            .doc_info
            .styles
            .get(style_id)
            .map(|s| s.local_name.as_str())
            .unwrap_or("");
        format!(
            "{{\"id\":{},\"name\":\"{}\"}}",
            style_id,
            name.replace('"', "\\\"")
        )
    }

    /// 셀 내부 문단의 스타일을 조회한다.
    #[wasm_bindgen(js_name = getCellStyleAt)]
    pub fn get_cell_style_at(
        &self,
        sec_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
    ) -> String {
        let style_id = self
            .core
            .get_cell_paragraph_ref(
                sec_idx as usize,
                parent_para_idx as usize,
                control_idx as usize,
                cell_idx as usize,
                cell_para_idx as usize,
            )
            .map(|p| p.style_id as usize)
            .unwrap_or(0);
        let name = self
            .core
            .document
            .doc_info
            .styles
            .get(style_id)
            .map(|s| s.local_name.as_str())
            .unwrap_or("");
        format!(
            "{{\"id\":{},\"name\":\"{}\"}}",
            style_id,
            name.replace('"', "\\\"")
        )
    }

    /// 스타일을 적용한다 (본문 문단).
    #[wasm_bindgen(js_name = applyStyle)]
    pub fn apply_style(
        &mut self,
        sec_idx: u32,
        para_idx: u32,
        style_id: u32,
    ) -> Result<String, JsValue> {
        self.core
            .apply_style_native(sec_idx as usize, para_idx as usize, style_id as usize)
            .map_err(|e| e.into())
    }

    /// 스타일을 적용한다 (셀 내 문단).
    #[wasm_bindgen(js_name = applyCellStyle)]
    pub fn apply_cell_style(
        &mut self,
        sec_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        style_id: u32,
    ) -> Result<String, JsValue> {
        self.core
            .apply_cell_style_native(
                sec_idx as usize,
                parent_para_idx as usize,
                control_idx as usize,
                cell_idx as usize,
                cell_para_idx as usize,
                style_id as usize,
            )
            .map_err(|e| e.into())
    }

    /// 표 셀에서 계산식을 실행한다.
    ///
    /// formula: "=SUM(A1:A5)", "=A1+B2*3" 등
    /// write_result: true이면 결과를 셀에 기록
    #[wasm_bindgen(js_name = evaluateTableFormula)]
    pub fn evaluate_table_formula(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        target_row: u32,
        target_col: u32,
        formula: &str,
        write_result: bool,
    ) -> Result<String, JsValue> {
        self.core
            .evaluate_table_formula(
                section_idx as usize,
                parent_para_idx as usize,
                control_idx as usize,
                target_row as usize,
                target_col as usize,
                formula,
                write_result,
            )
            .map_err(|e| e.into())
    }

    /// 글꼴 이름으로 font_id를 조회하거나 새로 생성한다.
    ///
    /// 한글(0번) 카테고리에서 이름 검색 → 없으면 7개 전체 카테고리에 신규 등록.
    /// 반환값: font_id (u16), 실패 시 -1
    #[wasm_bindgen(js_name = findOrCreateFontId)]
    pub fn find_or_create_font_id(&mut self, name: &str) -> i32 {
        self.find_or_create_font_id_native(name)
    }

    /// 특정 언어 카테고리에서 글꼴 이름으로 ID를 찾거나 등록한다.
    #[wasm_bindgen(js_name = findOrCreateFontIdForLang)]
    pub fn wasm_find_or_create_font_id_for_lang(&mut self, lang: u32, name: &str) -> i32 {
        self.core
            .find_or_create_font_id_for_lang(lang as usize, name)
    }

    /// 글자 서식을 적용한다 (본문 문단).
    #[wasm_bindgen(js_name = applyCharFormat)]
    pub fn apply_char_format(
        &mut self,
        sec_idx: usize,
        para_idx: usize,
        start_offset: usize,
        end_offset: usize,
        props_json: &str,
    ) -> Result<String, JsValue> {
        self.apply_char_format_native(sec_idx, para_idx, start_offset, end_offset, props_json)
            .map_err(|e| e.into())
    }

    /// 글자 서식을 적용한다 (셀 내 문단).
    #[wasm_bindgen(js_name = applyCharFormatInCell)]
    pub fn apply_char_format_in_cell(
        &mut self,
        sec_idx: usize,
        parent_para_idx: usize,
        control_idx: usize,
        cell_idx: usize,
        cell_para_idx: usize,
        start_offset: usize,
        end_offset: usize,
        props_json: &str,
    ) -> Result<String, JsValue> {
        self.apply_char_format_in_cell_native(
            sec_idx,
            parent_para_idx,
            control_idx,
            cell_idx,
            cell_para_idx,
            start_offset,
            end_offset,
            props_json,
        )
        .map_err(|e| e.into())
    }

    /// 감추기 설정
    #[wasm_bindgen(js_name = setPageHide)]
    pub fn set_page_hide(
        &mut self,
        sec: u32,
        para: u32,
        hide_header: bool,
        hide_footer: bool,
        hide_master: bool,
        hide_border: bool,
        hide_fill: bool,
        hide_page_num: bool,
    ) -> Result<String, JsValue> {
        self.set_page_hide_native(
            sec as usize,
            para as usize,
            hide_header,
            hide_footer,
            hide_master,
            hide_border,
            hide_fill,
            hide_page_num,
        )
        .map_err(|e| e.into())
    }

    /// 감추기 조회
    #[wasm_bindgen(js_name = getPageHide)]
    pub fn get_page_hide(&self, sec: u32, para: u32) -> Result<String, JsValue> {
        self.get_page_hide_native(sec as usize, para as usize)
            .map_err(|e| e.into())
    }

    /// 문단 서식을 적용한다 (본문 문단).
    /// 문단 번호 시작 방식 설정
    #[wasm_bindgen(js_name = setNumberingRestart)]
    pub fn set_numbering_restart(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        mode: u8,
        start_num: u32,
    ) -> Result<String, JsValue> {
        self.set_numbering_restart_native(section_idx as usize, para_idx as usize, mode, start_num)
            .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = applyParaFormat)]
    pub fn apply_para_format(
        &mut self,
        sec_idx: usize,
        para_idx: usize,
        props_json: &str,
    ) -> Result<String, JsValue> {
        self.apply_para_format_native(sec_idx, para_idx, props_json)
            .map_err(|e| e.into())
    }

    /// 문단 서식을 적용한다 (셀 내 문단).
    #[wasm_bindgen(js_name = applyParaFormatInCell)]
    pub fn apply_para_format_in_cell(
        &mut self,
        sec_idx: usize,
        parent_para_idx: usize,
        control_idx: usize,
        cell_idx: usize,
        cell_para_idx: usize,
        props_json: &str,
    ) -> Result<String, JsValue> {
        self.apply_para_format_in_cell_native(
            sec_idx,
            parent_para_idx,
            control_idx,
            cell_idx,
            cell_para_idx,
            props_json,
        )
        .map_err(|e| e.into())
    }

    /// 문단별 줄 폭 측정 진단 (WASM)
    #[wasm_bindgen(js_name = measureWidthDiagnostic)]
    pub fn measure_width_diagnostic(
        &self,
        section_idx: u32,
        para_idx: u32,
    ) -> Result<String, JsValue> {
        self.measure_width_diagnostic_native(section_idx as usize, para_idx as usize)
            .map_err(|e| e.into())
    }
}

pub(crate) mod event;
mod clipboard;
mod clipboard_html;
mod lifecycle;
mod footnote;
mod footnote_hittest;
mod field;
mod form;
mod search;
mod picture;
mod equation;
mod shape;
mod shape_arrange;
mod field_value;
mod field_clickhere;
mod field_clickhere_update;
mod style;
mod style_edit;
mod style_create;
mod numbering;
mod picture_external;
mod cell_by_path;
mod header_footer;
mod table_row_col;
mod table_cell_split;
mod header_footer_format;
mod table_cell_props;
mod table_props;
mod path_ops;

/// WASM 뷰어 컨트롤러 (뷰포트 관리 + 스케줄링)
#[wasm_bindgen]
pub struct HwpViewer {
    /// 문서 참조 (소유)
    document: HwpDocument,
    /// 렌더링 스케줄러
    scheduler: RenderScheduler,
}

#[wasm_bindgen]
impl HwpViewer {
    /// 뷰어 생성
    #[wasm_bindgen(constructor)]
    pub fn new(document: HwpDocument) -> Self {
        let page_count = document.page_count();
        let scheduler = RenderScheduler::new(page_count);
        Self {
            document,
            scheduler,
        }
    }

    /// 뷰포트 업데이트 (스크롤/리사이즈 시 호출)
    #[wasm_bindgen(js_name = updateViewport)]
    pub fn update_viewport(&mut self, scroll_x: f64, scroll_y: f64, width: f64, height: f64) {
        let event = RenderEvent::ViewportChanged(Viewport {
            scroll_x,
            scroll_y,
            width,
            height,
            zoom: self.scheduler_zoom(),
        });
        self.scheduler.on_event(&event);
    }

    /// 줌 변경
    #[wasm_bindgen(js_name = setZoom)]
    pub fn set_zoom(&mut self, zoom: f64) {
        let event = RenderEvent::ZoomChanged(zoom);
        self.scheduler.on_event(&event);
    }

    /// 현재 보이는 페이지 목록 반환
    #[wasm_bindgen(js_name = visiblePages)]
    pub fn visible_pages(&self) -> Vec<u32> {
        self.scheduler.visible_pages()
    }

    /// 대기 중인 렌더링 작업 수
    #[wasm_bindgen(js_name = pendingTaskCount)]
    pub fn pending_task_count(&self) -> u32 {
        self.scheduler.pending_count() as u32
    }

    /// 총 페이지 수
    #[wasm_bindgen(js_name = pageCount)]
    pub fn page_count(&self) -> u32 {
        self.document.page_count()
    }

    /// 특정 페이지 SVG 렌더링
    #[wasm_bindgen(js_name = renderPageSvg)]
    pub fn render_page_svg(&self, page_num: u32) -> Result<String, JsValue> {
        self.document.render_page_svg(page_num)
    }

    /// 특정 페이지 HTML 렌더링
    #[wasm_bindgen(js_name = renderPageHtml)]
    pub fn render_page_html(&self, page_num: u32) -> Result<String, JsValue> {
        self.document.render_page_html(page_num)
    }
}

impl HwpViewer {
    fn scheduler_zoom(&self) -> f64 {
        1.0
    }
}

#[wasm_bindgen]
impl HwpDocument {
    // ── 책갈피 API ──

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

// ─── 독립 함수 (문서 로드 없이 사용 가능) ───────────────

/// HWP 파일에서 썸네일 이미지만 경량 추출 (전체 파싱 없이)
///
/// 반환: JSON `{ "format": "png"|"gif", "base64": "...", "width": N, "height": N }`
/// PrvImage가 없으면 `null` 반환
#[wasm_bindgen(js_name = extractThumbnail)]
pub fn extract_thumbnail(data: &[u8]) -> JsValue {
    match crate::parser::extract_thumbnail_only(data) {
        Some(result) => {
            let base64 = base64_encode(&result.data);
            let mime = match result.format.as_str() {
                "png" => "image/png",
                "bmp" => "image/bmp",
                "gif" => "image/gif",
                _ => "application/octet-stream",
            };
            let json = format!(
                r#"{{"format":"{}","base64":"{}","dataUri":"data:{};base64,{}","width":{},"height":{}}}"#,
                result.format, base64, mime, base64, result.width, result.height
            );
            JsValue::from_str(&json)
        }
        None => JsValue::NULL,
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[cfg(test)]
mod tests;
