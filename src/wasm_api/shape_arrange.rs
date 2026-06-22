//! 도형 정렬·그룹·연결선·이동(Shape arrange) API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;
use crate::document_core::helpers::json_u32;

#[wasm_bindgen]
impl HwpDocument {
    /// Shape z-order 변경
    /// operation: "front" | "back" | "forward" | "backward"
    #[wasm_bindgen(js_name = changeShapeZOrder)]
    pub fn change_shape_z_order(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        operation: &str,
    ) -> Result<String, JsValue> {
        self.change_shape_z_order_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            operation,
        )
        .map_err(|e| e.into())
    }

    /// 선택된 개체들을 하나의 GroupShape로 묶는다.
    /// json: `{"sectionIdx":N, "targets":[{"paraIdx":N,"controlIdx":N},...]}`
    /// 반환: JSON `{"ok":true, "paraIdx":N, "controlIdx":N}`
    #[wasm_bindgen(js_name = groupShapes)]
    pub fn group_shapes(&mut self, json: &str) -> Result<String, JsValue> {
        let sec = json_u32(json, "sectionIdx").unwrap_or(0) as usize;
        // targets 배열 파싱
        let targets: Vec<(usize, usize)> = {
            let mut result = Vec::new();
            // 간단한 JSON 배열 파싱: "targets":[{"paraIdx":N,"controlIdx":N},...]
            if let Some(start) = json.find("\"targets\"") {
                let rest = &json[start..];
                if let Some(arr_start) = rest.find('[') {
                    if let Some(arr_end) = rest.find(']') {
                        let arr = &rest[arr_start + 1..arr_end];
                        // 각 {} 블록에서 paraIdx, controlIdx 추출
                        let mut pos = 0;
                        while let Some(obj_start) = arr[pos..].find('{') {
                            let obj_start = pos + obj_start;
                            if let Some(obj_end) = arr[obj_start..].find('}') {
                                let obj = &arr[obj_start..obj_start + obj_end + 1];
                                let pi = json_u32(obj, "paraIdx").unwrap_or(0) as usize;
                                let ci = json_u32(obj, "controlIdx").unwrap_or(0) as usize;
                                result.push((pi, ci));
                                pos = obj_start + obj_end + 1;
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
            result
        };
        self.group_shapes_native(sec, &targets)
            .map_err(|e| e.into())
    }

    /// GroupShape를 풀어 자식 개체들을 개별로 복원한다.
    #[wasm_bindgen(js_name = ungroupShape)]
    pub fn ungroup_shape(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.ungroup_shape_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 직선 끝점 이동 (글로벌 HWPUNIT 좌표)
    #[wasm_bindgen(js_name = moveLineEndpoint)]
    pub fn move_line_endpoint(
        &mut self,
        sec: u32,
        para: u32,
        ci: u32,
        sx: i32,
        sy: i32,
        ex: i32,
        ey: i32,
    ) -> Result<String, JsValue> {
        self.move_line_endpoint_native(sec as usize, para as usize, ci as usize, sx, sy, ex, ey)
            .map_err(|e| e.into())
    }

    /// 구역 내 모든 연결선의 좌표를 연결된 도형 위치에 맞게 갱신한다.
    #[wasm_bindgen(js_name = updateConnectorsInSection)]
    pub fn update_connectors_in_section_wasm(&mut self, section_idx: u32) {
        self.update_connectors_in_section(section_idx as usize);
    }

    /// 수직 커서 이동 (ArrowUp/Down) — 단일 호출로 줄/문단/표/구역 경계를 모두 처리한다.
    ///
    /// delta: -1=위, +1=아래
    /// preferred_x: 이전 반환값의 preferredX (최초 이동 시 -1.0 전달)
    /// 셀 컨텍스트: 본문이면 모두 0xFFFFFFFF 전달
    ///
    /// 반환: JSON `{DocumentPosition + CursorRect + preferredX}`
    #[wasm_bindgen(js_name = moveVertical)]
    pub fn move_vertical(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        delta: i32,
        preferred_x: f64,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
    ) -> Result<String, JsValue> {
        let cell_ctx = if parent_para_idx == u32::MAX {
            None
        } else {
            Some((
                parent_para_idx as usize,
                control_idx as usize,
                cell_idx as usize,
                cell_para_idx as usize,
            ))
        };
        self.move_vertical_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            delta,
            preferred_x,
            cell_ctx,
        )
        .map_err(|e| e.into())
    }

}
