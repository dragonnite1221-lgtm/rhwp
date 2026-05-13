//! Stage 5 회귀 — yangsik-fragments 디렉터리의 58개 부품 fragment를 모두 paste 시도해
//! 알고리즘 정확성 + ID 재사용률 + (옵션) 한컴 통과률을 측정한다.
//!
//! 시드 파일: `~/rhwp-layout-profiles/personal-templates/양식_3bf55ee3.hwpx` (양식.hwpx 자체)
//! - section0.xml + header.xml 만 paste 결과로 갈아끼움
//! - 다른 entries (BinData, mimetype, version.xml, settings.xml, content.hpf, META-INF) 는
//!   compress_type 을 보존하여 그대로 복사한다.
//!
//! 한컴 통과 검증은 무거우므로 환경변수 `TEST_HWP_OPEN=1` 설정 시에만 실행.

use rhwp::document_core::{paste_fragment_into_section, SourceDefinitions};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const FRAGMENTS_DIR_REL: &str = "rhwp-layout-profiles/personal-templates/yangsik-fragments";
const SEED_HWPX_REL: &str = "rhwp-layout-profiles/personal-templates/양식_3bf55ee3.hwpx";

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME not set"))
}

#[derive(Debug)]
struct FragmentMeta {
    part_name: String,
    category: String,
    kind: String,
    fragment_file: String,
    used_char_pr_ids: Vec<u32>,
    used_para_pr_ids: Vec<u32>,
    used_style_ids: Vec<u32>,
    used_border_fill_ids: Vec<u32>,
    source_char_prs: String,
    source_para_prs: String,
    source_styles: String,
    source_border_fills: String,
}

/// 매우 단순한 manifest.json 파서 — serde_json 의존성 회피.
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
        let category = extract_string_field(block, "category").unwrap_or_default();
        let kind = extract_string_field(block, "kind").unwrap_or_default();
        let fragment_file = extract_string_field(block, "fragment_file").unwrap_or_default();
        let used_char_pr_ids = extract_int_array(block, "used_char_pr_ids");
        let used_para_pr_ids = extract_int_array(block, "used_para_pr_ids");
        let used_style_ids = extract_int_array(block, "used_style_ids");
        let used_border_fill_ids = extract_int_array(block, "used_border_fill_ids");
        let source_char_prs = extract_string_field(block, "char_prs").unwrap_or_default();
        let source_para_prs = extract_string_field(block, "para_prs").unwrap_or_default();
        let source_styles = extract_string_field(block, "styles").unwrap_or_default();
        let source_border_fills =
            extract_string_field(block, "border_fills").unwrap_or_default();
        out.push(FragmentMeta {
            part_name,
            category,
            kind,
            fragment_file,
            used_char_pr_ids,
            used_para_pr_ids,
            used_style_ids,
            used_border_fill_ids,
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

fn extract_int_array(block: &str, field: &str) -> Vec<u32> {
    let needle = format!("\"{field}\":");
    let Some(idx) = block.find(&needle) else { return Vec::new() };
    let after = &block[idx + needle.len()..];
    let after = after.trim_start();
    if !after.starts_with('[') {
        return Vec::new();
    }
    let Some(end) = after.find(']') else { return Vec::new() };
    after[1..end]
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect()
}

fn read_hwpx_entries(path: &Path) -> Vec<(String, Vec<u8>, u16)> {
    let bytes = fs::read(path).expect("hwpx read");
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader).expect("zip open");
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).expect("zip entry");
        let name = entry.name().to_string();
        let method = match entry.compression() {
            zip::CompressionMethod::Stored => 0u16,
            _ => 8u16,
        };
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data).expect("read entry");
        out.push((name, data, method));
    }
    out
}

fn write_hwpx(path: &Path, entries: &[(String, Vec<u8>, u16)]) {
    use zip::write::SimpleFileOptions;
    let file = fs::File::create(path).expect("create hwpx");
    let mut writer = zip::ZipWriter::new(file);
    let mut sorted_idx: Vec<usize> = (0..entries.len()).collect();
    sorted_idx.sort_by_key(|&i| if entries[i].0 == "mimetype" { 0 } else { 1 });
    for &i in &sorted_idx {
        let (name, data, method) = &entries[i];
        let opts = if name == "mimetype" || *method == 0 {
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
        } else {
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
        };
        writer.start_file(name.as_str(), opts).expect("start_file");
        writer.write_all(data).expect("write data");
    }
    writer.finish().expect("zip finish");
}

#[test]
fn yangsik_58_fragments_paste_smoke() {
    let frags_dir = home().join(FRAGMENTS_DIR_REL);
    let manifest_path = frags_dir.join("manifest.json");
    if !manifest_path.is_file() {
        eprintln!("[skip] manifest not found: {}", manifest_path.display());
        return;
    }
    let manifest = load_manifest(&manifest_path);
    assert!(
        !manifest.is_empty(),
        "manifest empty — extract_yangsik_fragments.py 실행 필요"
    );

    let seed_path = home().join(SEED_HWPX_REL);
    let seed_entries = read_hwpx_entries(&seed_path);
    let seed_section = seed_entries
        .iter()
        .find(|(n, _, _)| n == "Contents/section0.xml")
        .map(|(_, b, _)| String::from_utf8_lossy(b).to_string())
        .expect("section0.xml in seed");
    let seed_header = seed_entries
        .iter()
        .find(|(n, _, _)| n == "Contents/header.xml")
        .map(|(_, b, _)| String::from_utf8_lossy(b).to_string())
        .expect("header.xml in seed");

    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut total_id_reused: u64 = 0;
    let mut total_id_new: u64 = 0;
    let mut paste_failures: Vec<String> = Vec::new();
    let mut hwp_open_attempts = 0;
    let mut hwp_open_accepted = 0;
    let mut hwp_open_rejected: Vec<(String, i32)> = Vec::new();
    let do_hwp_open = std::env::var("TEST_HWP_OPEN").ok().as_deref() == Some("1");

    for f in &manifest {
        let frag_path = frags_dir.join(&f.fragment_file);
        let fragment_xml = fs::read_to_string(&frag_path).expect("fragment read");
        let source = SourceDefinitions {
            char_prs: f.source_char_prs.clone(),
            para_prs: f.source_para_prs.clone(),
            styles: f.source_styles.clone(),
            border_fills: f.source_border_fills.clone(),
        };
        let mut header_mut = seed_header.clone();
        let wrapped = if f.kind == "table" {
            format!("<hp:p paraPrIDRef=\"0\">{fragment_xml}</hp:p>")
        } else {
            fragment_xml.clone()
        };
        let result =
            paste_fragment_into_section(&seed_section, &mut header_mut, 0, &wrapped, &source);
        match result {
            Ok(r) => {
                *by_kind.entry(f.kind.clone()).or_insert(0) += 1;
                let new_in_header = (header_mut.len() as i64) - (seed_header.len() as i64);
                if new_in_header == 0 {
                    let source_total = (f.used_char_pr_ids.len()
                        + f.used_para_pr_ids.len()
                        + f.used_style_ids.len()
                        + f.used_border_fill_ids.len()) as u64;
                    total_id_reused += source_total;
                } else {
                    for src_id in &f.used_char_pr_ids {
                        if r.id_remap.char_pr.get(src_id).copied() == Some(*src_id) {
                            total_id_reused += 1;
                        } else {
                            total_id_new += 1;
                        }
                    }
                    for src_id in &f.used_para_pr_ids {
                        if r.id_remap.para_pr.get(src_id).copied() == Some(*src_id) {
                            total_id_reused += 1;
                        } else {
                            total_id_new += 1;
                        }
                    }
                    for src_id in &f.used_style_ids {
                        if r.id_remap.style.get(src_id).copied() == Some(*src_id) {
                            total_id_reused += 1;
                        } else {
                            total_id_new += 1;
                        }
                    }
                    for src_id in &f.used_border_fill_ids {
                        if r.id_remap.border_fill.get(src_id).copied() == Some(*src_id) {
                            total_id_reused += 1;
                        } else {
                            total_id_new += 1;
                        }
                    }
                }
                if do_hwp_open {
                    hwp_open_attempts += 1;
                    let mut new_entries = seed_entries.clone();
                    for entry in &mut new_entries {
                        if entry.0 == "Contents/section0.xml" {
                            entry.1 = r.new_section_xml.as_bytes().to_vec();
                        } else if entry.0 == "Contents/header.xml" {
                            entry.1 = header_mut.as_bytes().to_vec();
                        }
                    }
                    let safe_name = f.part_name.replace('/', "_");
                    let tmp_path =
                        PathBuf::from(format!("/tmp/yangsik-stage5-{safe_name}.hwpx"));
                    write_hwpx(&tmp_path, &new_entries);
                    let res = hwp_open(&tmp_path, 5);
                    match res {
                        HwpResult::Accepted => hwp_open_accepted += 1,
                        HwpResult::Rejected(c) => {
                            hwp_open_rejected.push((f.part_name.clone(), c));
                        }
                        HwpResult::Skipped(reason) => {
                            eprintln!("[hwp skip] {}: {reason}", f.part_name);
                        }
                    }
                    let _ = fs::remove_file(&tmp_path);
                }
            }
            Err(e) => {
                paste_failures.push(format!("{} ({}): {e}", f.part_name, f.category));
            }
        }
    }

    let total = manifest.len();
    let succeeded = total - paste_failures.len();
    let id_total = total_id_reused + total_id_new;
    let reuse_pct = if id_total == 0 {
        100.0
    } else {
        (total_id_reused as f64 / id_total as f64) * 100.0
    };

    eprintln!("=== Stage 5 회귀 메트릭 ===");
    eprintln!("paste 성공: {succeeded}/{total}");
    eprintln!("by kind: {by_kind:?}");
    eprintln!("ID 재사용률: {reuse_pct:.1}% ({total_id_reused}/{id_total})");
    if !paste_failures.is_empty() {
        eprintln!("실패 fragment ({}):", paste_failures.len());
        for fail in &paste_failures {
            eprintln!("  - {fail}");
        }
    }
    if do_hwp_open {
        eprintln!(
            "한컴 통과률: {hwp_open_accepted}/{hwp_open_attempts} ({} 거부)",
            hwp_open_rejected.len()
        );
        for (name, code) in &hwp_open_rejected {
            eprintln!("  reject {name} exit={code}");
        }
    } else {
        eprintln!("한컴 통과 검증 SKIP — TEST_HWP_OPEN=1 환경변수로 활성화");
    }

    assert_eq!(
        paste_failures.len(),
        0,
        "{} fragments failed: {:?}",
        paste_failures.len(),
        paste_failures
    );
    assert!(
        reuse_pct >= 70.0,
        "ID 재사용률 {reuse_pct:.1}% < 70%"
    );
    if do_hwp_open {
        let table_count = by_kind.get("table").copied().unwrap_or(0);
        let hwp_min_required = (table_count as f64 * 0.96) as usize;
        assert!(
            hwp_open_accepted >= hwp_min_required,
            "한컴 통과 {hwp_open_accepted} < 최소 {hwp_min_required} (table={table_count})"
        );
    }
}

enum HwpResult {
    Accepted,
    Rejected(i32),
    Skipped(String),
}

fn hwp_open(path: &Path, timeout_sec: u32) -> HwpResult {
    let bin = Path::new("/opt/hnc/hoffice11/Bin/hwp");
    if !bin.is_file() {
        return HwpResult::Skipped(format!("hwp not found at {}", bin.display()));
    }
    if !path.is_file() {
        return HwpResult::Skipped(format!("input not found: {}", path.display()));
    }
    let out = std::process::Command::new("timeout")
        .arg(timeout_sec.to_string())
        .arg("env")
        .arg("DISPLAY=:0")
        .arg(bin)
        .arg(path)
        .output();
    let Ok(out) = out else {
        return HwpResult::Skipped("failed to spawn timeout/hwp".into());
    };
    match out.status.code() {
        Some(143) | Some(124) => HwpResult::Accepted,
        Some(c) => HwpResult::Rejected(c),
        None => HwpResult::Skipped("no exit code".into()),
    }
}
