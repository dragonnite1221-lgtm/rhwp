//! 외부 이미지(external file path) basename 조회·바이너리 주입 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// [Task #741 후속] 외부 file path 그림 영역 영역 영역 영역 basename 목록 영역 반환.
    ///
    /// HWP3 파일 영역 image 영역 영역 절대 경로 영역 저장 영역. WASM 환경 영역 영역 file
    /// system access 부재 영역, JS 영역 영역 영역 영역 fetch 영역 영역 영역 file 영역 load
    /// 영역 후 `injectExternalImage` 영역 영역 영역 inject 영역.
    ///
    /// 반환: JSON 배열 `["oracle.gif", "rdb02.gif", ...]` (중복 제거)
    #[wasm_bindgen(js_name = getExternalImageBasenames)]
    pub fn get_external_image_basenames(&self) -> String {
        use crate::model::control::Control;
        use crate::model::shape::ShapeObject;
        use std::collections::BTreeSet;

        let mut names: BTreeSet<String> = BTreeSet::new();
        for section in &self.document().sections {
            for para in &section.paragraphs {
                for ctrl in &para.controls {
                    let pic = match ctrl {
                        Control::Picture(p) => p,
                        Control::Shape(s) => match s.as_ref() {
                            ShapeObject::Picture(p) => p,
                            _ => continue,
                        },
                        _ => continue,
                    };
                    if let Some(ref path) = pic.image_attr.external_path {
                        let id = pic.image_attr.bin_data_id;
                        let already_loaded = self.document().bin_data_content.iter()
                            .any(|c| c.id == id && !c.data.is_empty());
                        if already_loaded { continue; }
                        let basename = path.rsplit(|c| c == '/' || c == '\\').next().unwrap_or(path);
                        names.insert(basename.to_string());
                    }
                }
            }
        }
        let arr: Vec<String> = names.into_iter().collect();
        serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
    }

    /// [Task #741 후속] 외부 file path 그림 영역 영역 binary data 영역 inject.
    ///
    /// JS 영역 영역 영역 fetch 영역 영역 영역 file 영역 load 영역 후 본 메서드 영역 호출 영역
    /// IR 영역 영역 영역 image binary 영역 영역 → renderer 영역 영역 표시.
    ///
    /// `basename`: 영역 영역 file 영역 영역 (예: "oracle.gif")
    /// `data`: 영역 영역 binary 영역
    /// `display_path`: dialog 영역 영역 영역 영역 표시 영역 영역 path. 빈 문자열 ("") 영역
    ///                 영역 영역 fallback 영역 영역 `/samples/<basename>` 영역 사용. 한컴 viewer
    ///                 정합 영역 영역 OS 영역 절대 경로 영역 영역 (예: "/Users/.../samples/rdb02.gif")
    #[wasm_bindgen(js_name = injectExternalImage)]
    pub fn inject_external_image(&mut self, basename: &str, data: &[u8], display_path: &str) -> u32 {
        use crate::model::control::Control;
        use crate::model::shape::ShapeObject;

        let mut injected: u32 = 0;
        // 영역 외부 image 영역 영역 영역 영역 basename 매칭 영역 영역 (id, ext) 수집
        let mut targets: Vec<(u16, String)> = Vec::new();
        for section in &self.document().sections {
            for para in &section.paragraphs {
                for ctrl in &para.controls {
                    let pic = match ctrl {
                        Control::Picture(p) => p,
                        Control::Shape(s) => match s.as_ref() {
                            ShapeObject::Picture(p) => p,
                            _ => continue,
                        },
                        _ => continue,
                    };
                    if let Some(ref path) = pic.image_attr.external_path {
                        let path_basename = path.rsplit(|c| c == '/' || c == '\\').next().unwrap_or(path);
                        if path_basename != basename { continue; }
                        let id = pic.image_attr.bin_data_id;
                        let already_loaded = self.document().bin_data_content.iter()
                            .any(|c| c.id == id && !c.data.is_empty());
                        if already_loaded { continue; }
                        let ext = std::path::Path::new(basename)
                            .extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
                        targets.push((id, ext));
                    }
                }
            }
        }

        for (id, ext) in targets {
            let idx = (id as usize).saturating_sub(1);
            if idx < self.document().bin_data_content.len() {
                self.document_mut().bin_data_content[idx].id = id;
                self.document_mut().bin_data_content[idx].data = data.to_vec();
                self.document_mut().bin_data_content[idx].extension = ext;
            } else {
                self.document_mut().bin_data_content.push(
                    crate::model::bin_data::BinDataContent {
                        id, data: data.to_vec(), extension: ext,
                    }
                );
            }
            injected += 1;

            // [한컴 viewer 정합] 원본 path 영역 영역 access 부재 시 HWP file 영역 영역
            // 같은 영역 영역 image 영역 영역 영역 dialog 영역 영역 resolved path 영역 영역 갱신.
            // display_path 영역 영역 영역 영역 (Vite middleware 영역 영역 X-File-Path header
            // 영역 영역 영역 OS 절대 경로) 영역 사용, 빈 문자열 영역 fallback 영역 영역
            // `/samples/<basename>` 영역 사용.
            let resolved = if display_path.is_empty() {
                format!("/samples/{}", basename)
            } else {
                display_path.to_string()
            };
            for section in &mut self.document_mut().sections {
                for para in &mut section.paragraphs {
                    for ctrl in &mut para.controls {
                        let pic = match ctrl {
                            crate::model::control::Control::Picture(p) => p,
                            crate::model::control::Control::Shape(s) => match s.as_mut() {
                                crate::model::shape::ShapeObject::Picture(p) => p,
                                _ => continue,
                            },
                            _ => continue,
                        };
                        if pic.image_attr.bin_data_id == id
                            && pic.image_attr.external_path.is_some() {
                            pic.image_attr.external_path = Some(resolved.clone());
                        }
                    }
                }
            }
        }
        injected
    }

}
