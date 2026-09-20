//! 표 셀 자체에 field_name이 붙은 "가상 셀 필드"용 문서 전체 고유 ID 생성 (rhwp-2).
//!
//! `field_query.rs` 소스 파일이 frozen 200줄 baseline(920줄)을 넘지 않도록
//! 분리했다 -- 이 파일 자체는 새 코드다.

use super::{FieldLocation, NestedEntry};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// `loc`는 이 셀의 TableCell 항목까지 push된 상태여야 하며, section_index +
/// para_index + nested_path 전체 체인이 문서 내에서 이 위치를 고유하게
/// 식별한다 (단순히 현재 문단 로컬 인덱스만 쓰면 서로 다른 문단의 표가
/// 우연히 같은 로컬 인덱스를 가질 때 충돌한다).
///
/// 비트 31을 예약해 실제 HWP 필드(에디터가 부여하는 작은 순차 정수)와의
/// 충돌 가능성을 최소화한다. 해시 충돌(또는 비정상적으로 조작된 문서의
/// 실제 필드 ID가 이 비트를 우연히 사용하는 경우)로 인한 잔여 위험은
/// get_field_value_by_id/by_name의 중복 ID 검사가 조용히 잘못된 값을
/// 반환하는 대신 명확한 오류로 잡아낸다.
pub(super) fn virtual_cell_field_id(loc: &FieldLocation) -> u32 {
    let mut hasher = DefaultHasher::new();
    loc.section_index.hash(&mut hasher);
    loc.para_index.hash(&mut hasher);
    for entry in &loc.nested_path {
        match entry {
            NestedEntry::TableCell { control_index, cell_index, para_index } => {
                0u8.hash(&mut hasher);
                control_index.hash(&mut hasher);
                cell_index.hash(&mut hasher);
                para_index.hash(&mut hasher);
            }
            NestedEntry::TextBox { control_index, para_index } => {
                1u8.hash(&mut hasher);
                control_index.hash(&mut hasher);
                para_index.hash(&mut hasher);
            }
        }
    }
    let digest = hasher.finish();
    0x8000_0000 | ((digest as u32) & 0x7FFF_FFFF)
}
