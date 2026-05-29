use wasm_bindgen::prelude::*;

use super::HwpDocument;
use crate::renderer::scheduler::{RenderEvent, RenderObserver, RenderScheduler, Viewport};

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
