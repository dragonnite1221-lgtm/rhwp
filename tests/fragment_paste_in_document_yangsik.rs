//! Stage 3 회귀 — yangsik 58 fragment 를 Document 경유(`paste_hwpx_fragment_in_document_native`)
//! 로 누적 paste 하면서 raw XML + IR 일관성 + ID 재사용률을 측정한다.
//!
//! Phase 2 회귀 테스트(`fragment_paste_yangsik.rs`)가 raw API 단독 알고리즘 검증이라면,
//! 본 테스트는 Document IR 까지 포함한 wasm bridge 전체 흐름의 회귀.
//!
//! 시드: `~/rhwp-layout-profiles/personal-templates/양식_3bf55ee3.hwpx` (양식.hwpx 자체)
//! manifest: `~/rhwp-layout-profiles/personal-templates/yangsik-fragments/manifest.json`

use rhwp::document_core::{DocumentCore, SourceDefinitions};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const FRAGMENTS_DIR_REL: &str = "rhwp-layout-profiles/personal-templates/yangsik-fragments";
const SEED_HWPX_REL: &str = "rhwp-layout-profiles/personal-templates/양식_3bf55ee3.hwpx";

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME not set"))
}

#[derive(Debug)]
struct FragmentMeta {
    part_name: String,
    kind: String,
    fragment_file: String,
    source_char_prs: String,
    source_para_prs: String,
    source_styles: String,
    source_border_fills: String,
}

fn load_manifest(path: &Path) -> Vec<FragmentMeta> {
    let raw = fs::read_to_string(path).expect("manifest.json read");
    let mut out = Vec::new();
    let mut pos = 0usize;
    while let Some(rel) = raw[pos..].find("\"part_name\":") {
        let start = pos + rel;
        let next_pos = raw[start + 1..].find("\"part_name\":").map(|r| start + 1 + r);
        let block_end = next_pos.unwrap_or_else(|| raw.len());
        let block = &raw[start..block_end];
        let part_name = extract_string_field(block, "part_name").unwrap_or_default();
        let kind = extract_string_field(block, "kind").unwrap_or_default();
        let fragment_file = extract_string_field(block, "fragment_file").unwrap_or_default();
        let source_char_prs = extract_string_field(block, "char_prs").unwrap_or_default();
        let source_para_prs = extract_string_field(block, "para_prs").unwrap_or_default();
        let source_styles = extract_string_field(block, "styles").unwrap_or_default();
        let source_border_fills =
            extract_string_field(block, "border_fills").unwrap_or_default();
        out.push(FragmentMeta {
            part_name,
            kind,
            fragment_file,
            source_char_prs,
            source_para_prs,
            source_styles,
            source_border_fills,
        });
        pos = block_end;
    }
    out
}

fn extract_string_field(block: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":");
    let idx = block.find(&needle)?;
    let after = &block[idx + needle.len()..];
    let after = after.trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = after[1..].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(esc) = chars.next() {
                match esc {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'u' => {
                        let h: String = (&mut chars).take(4).collect();
                        if let Ok(n) = u32::from_str_radix(&h, 16) {
                            if let Some(ch) = char::from_u32(n) {
                                out.push(ch);
                            }
                        }
                    }
                    other => out.push(other),
                }
            }
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

#[test]
fn yangsik_58_fragments_paste_in_document_smoke() {
    let frags_dir = home().join(FRAGMENTS_DIR_REL);
    let manifest_path = frags_dir.join("manifest.json");
    if !manifest_path.is_file() {
        eprintln!("[skip] manifest not found: {}", manifest_path.display());
        return;
    }
    let manifest = load_manifest(&manifest_path);
    assert!(!manifest.is_empty(), "manifest empty");

    let seed_path = home().join(SEED_HWPX_REL);
    if !seed_path.is_file() {
        eprintln!("[skip] seed not found: {}", seed_path.display());
        return;
    }
    let seed_bytes = fs::read(&seed_path).expect("seed read");
    let mut doc = DocumentCore::from_bytes(&seed_bytes).expect("from_bytes seed");
    assert!(
        doc.has_source_xmls(),
        "seed 가 HWPX 인데 raw 보존 실패"
    );
    let initial_section_count = doc.source_section_xml_count();
    assert!(initial_section_count >= 1);

    let initial_para_count = doc.document().sections[0].paragraphs.len();

    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut paste_failures: Vec<String> = Vec::new();
    let mut total_id_reused: u64 = 0;
    let mut total_id_new: u64 = 0;
    let mut accumulated_inserts: usize = 0;

    // 누적 paste — 매 호출 결과가 다음 paste 의 source 가 됨.
    // anchor 는 항상 0 (시드의 첫 paragraph 직후) — 클라이언트 caret 시뮬레이션.
    for f in &manifest {
        let frag_path = frags_dir.join(&f.fragment_file);
        let fragment_xml = fs::read_to_string(&frag_path).expect("fragment read");
        let source = SourceDefinitions {
            char_prs: f.source_char_prs.clone(),
            para_prs: f.source_para_prs.clone(),
            styles: f.source_styles.clone(),
            border_fills: f.source_border_fills.clone(),
        };
        match doc.paste_hwpx_fragment_in_document_native(0, 0, &fragment_xml, &source) {
            Ok(result) => {
                *by_kind.entry(f.kind.clone()).or_default() += 1;
                accumulated_inserts += result.inserted_para_count;

                // ID 재사용/신규 카운트
                for kind in [
                    &result.id_remap.char_pr,
                    &result.id_remap.para_pr,
                    &result.id_remap.style,
                    &result.id_remap.border_fill,
                ] {
                    for (src, tgt) in kind {
                        if src == tgt {
                            total_id_reused += 1;
                        } else {
                            total_id_new += 1;
                        }
                    }
                }
            }
            Err(e) => {
                paste_failures.push(format!("{}: {e}", f.part_name));
            }
        }
    }

    let success_count = manifest.len() - paste_failures.len();
    let total_id = total_id_reused + total_id_new;
    let id_reuse_pct = if total_id == 0 {
        0.0
    } else {
        100.0 * (total_id_reused as f64) / (total_id as f64)
    };

    println!("=== Stage 3 (Document 경유) 회귀 메트릭 ===");
    println!("paste 성공: {}/{}", success_count, manifest.len());
    println!("by kind: {:?}", by_kind);
    println!(
        "ID 재사용률: {:.1}% ({}/{})",
        id_reuse_pct, total_id_reused, total_id
    );
    println!(
        "누적 inserted paragraph: {} (시드 {} → 결과 {})",
        accumulated_inserts,
        initial_para_count,
        doc.document().sections[0].paragraphs.len()
    );
    if !paste_failures.is_empty() {
        println!("실패: {:?}", paste_failures);
    }

    assert_eq!(success_count, manifest.len(), "paste 실패가 있음");

    // raw 갱신 검증 — 누적 paste 후 raw 길이는 초기보다 커야 함
    let raw_after = doc.get_source_section_xml(0).expect("raw section 0");
    assert!(
        raw_after.len() > 1000,
        "raw section 길이가 너무 짧음 ({}B)",
        raw_after.len()
    );

    // IR 동기화 검증 — paragraphs 수 증가
    let final_para_count = doc.document().sections[0].paragraphs.len();
    assert!(
        final_para_count > initial_para_count,
        "IR paragraph count 증가 안함 ({} → {})",
        initial_para_count,
        final_para_count
    );

    // ID 재사용률 ≥ 70% (W5 게이트)
    assert!(
        id_reuse_pct >= 70.0,
        "ID 재사용률 {:.1}% — W5 게이트 (≥70%) 미달",
        id_reuse_pct
    );
}
