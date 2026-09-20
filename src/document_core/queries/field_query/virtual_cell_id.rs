//! 표 셀 자체에 field_name이 붙은 "가상 셀 필드"용 문서 전체 고유 ID 생성 (rhwp-2).
//!
//! `field_query.rs` 소스 파일이 frozen 200줄 baseline(920줄)을 넘지 않도록
//! 분리했다 -- 이 파일 자체는 새 코드다.

use super::{FieldInfo, FieldLocation, NestedEntry};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// `loc`는 이 셀의 TableCell 항목까지 push된 상태여야 하며, section_index +
/// para_index + nested_path 전체 체인이 문서 내에서 이 위치를 고유하게
/// 식별한다 (단순히 현재 문단 로컬 인덱스만 쓰면 서로 다른 문단의 표가
/// 우연히 같은 로컬 인덱스를 가질 때 충돌한다).
///
/// 비트 31을 예약해 실제 HWP 필드(에디터가 부여하는 작은 순차 정수)와의
/// 충돌 가능성을 최소화한다. 이 값은 제안값일 뿐이며, 실제 문서 전체
/// 고유성은 `resolve_virtual_field_id_collisions`가 보장한다 (해시 자체는
/// 31비트 공간이라 극단적으로 많은 가상 셀 필드가 있으면 충돌할 수 있다).
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

/// `collect_all_fields`가 만든 목록에서 가상 셀 필드(`is_virtual_cell_field`)의
/// 제안 ID가 서로 또는 실제 필드 ID와 충돌하면 재배정해 이 문서 내에서
/// 완전히 고유하게 만든다. 실제 필드의 ID는 절대 바꾸지 않는다 -- 오직
/// 이 함수 스스로 새로 만든 가상 ID만 조정 대상이다.
///
/// `field.ctrl_id == 0`으로 판별하면 안 된다: HWP3/HWPX 파서가
/// `Field::default()`로 만드는 실제 필드(메일머지, 색인 표시, HWPX
/// FIELD_BEGIN 등)도 ctrl_id를 채우지 않아 그대로 0이므로, 그 값을
/// 가상 필드로 오인해 실제 필드의 ID를 바꿔버릴 수 있다.
///
/// 해시 기반 제안값은 31비트 공간이므로 극단적으로 많은 가상 셀 필드가
/// 있는 문서에서는 충돌할 수 있다 (생일 문제로 대략 수만 개 규모부터).
/// 그런 드문 경우에도 `get_field_value_by_id`가 어느 셀의 값인지 알 수
/// 없는 채로 조용히 하나를 골라 반환하는 대신, 여기서 먼저 결정론적으로
/// 재배정해 항상 실제로 고유한 ID를 돌려준다.
pub(super) fn resolve_virtual_field_id_collisions(fields: &mut [FieldInfo]) {
    let mut used: HashSet<u32> = fields
        .iter()
        .filter(|fi| !fi.is_virtual_cell_field)
        .map(|fi| fi.field.field_id)
        .collect();
    for fi in fields.iter_mut().filter(|fi| fi.is_virtual_cell_field) {
        if used.insert(fi.field.field_id) {
            continue;
        }
        let mut candidate = fi.field.field_id;
        loop {
            candidate = 0x8000_0000 | (candidate.wrapping_add(1) & 0x7FFF_FFFF);
            if used.insert(candidate) {
                fi.field.field_id = candidate;
                break;
            }
        }
    }
}
