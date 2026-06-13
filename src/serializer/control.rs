//! 컨트롤 직렬화 (표, 도형, 그림, 머리말/꼬리말, 각주/미주 등)
//!
//! `parser::body_text::parse_ctrl_header` + `parser::control::parse_control`의 역방향.
//! 각 Control enum variant를 CTRL_HEADER 레코드(+자식 레코드)로 변환한다.

use super::body_text::serialize_paragraph_list;
use super::byte_writer::ByteWriter;

use crate::model::control::*;
use crate::model::document::SectionDef;
use crate::model::footnote::FootnoteShape;
use crate::model::footnote::{Endnote, Footnote};
use crate::model::header_footer::{Footer, Header, HeaderFooterApply};
use crate::model::image::{ImageEffect, Picture};
use crate::model::page::{ColumnDef, ColumnDirection, ColumnType, PageBorderFill, PageDef};
use crate::model::shape::{
    Caption, CaptionDirection, CaptionVertAlign, CommonObjAttr, DrawingObjAttr, ShapeComponentAttr,
    ShapeObject,
};
use crate::model::style::{Fill, FillType, ImageFillMode, ShapeBorderLine};
use crate::model::table::{Cell, Table, TablePageBreak, VerticalAlign};
use crate::parser::record::Record;
use crate::parser::tags;

/// Control을 CTRL_HEADER 레코드(+자식)로 직렬화
///
/// `ctrl_data_record`: 원본의 CTRL_DATA 레코드 데이터 (라운드트립 보존용).
/// CTRL_HEADER 바로 다음에 삽입된다.
pub fn serialize_control(
    ctrl: &Control,
    level: u16,
    ctrl_data_record: Option<&[u8]>,
    records: &mut Vec<Record>,
) {
    let insert_pos = records.len(); // CTRL_HEADER가 쓰이는 위치 기억

    match ctrl {
        Control::SectionDef(sd) => serialize_section_def(sd, level, records),
        Control::ColumnDef(cd) => serialize_column_def(cd, level, records),
        Control::Table(table) => serialize_table(table, level, records),
        Control::Header(header) => serialize_header_control(header, level, records),
        Control::Footer(footer) => serialize_footer_control(footer, level, records),
        Control::Footnote(fn_) => serialize_footnote(fn_, level, records),
        Control::Endnote(en) => serialize_endnote(en, level, records),
        Control::HiddenComment(comment) => serialize_hidden_comment(comment, level, records),
        Control::AutoNumber(an) => {
            records.push(make_ctrl_record(
                tags::CTRL_AUTO_NUMBER,
                level,
                &serialize_auto_number(an),
            ));
        }
        Control::NewNumber(nn) => {
            records.push(make_ctrl_record(
                tags::CTRL_NEW_NUMBER,
                level,
                &serialize_new_number(nn),
            ));
        }
        Control::PageNumberPos(pnp) => {
            records.push(make_ctrl_record(
                tags::CTRL_PAGE_NUM_POS,
                level,
                &serialize_page_num_pos(pnp),
            ));
        }
        Control::PageHide(ph) => {
            records.push(make_ctrl_record(
                tags::CTRL_PAGE_HIDE,
                level,
                &serialize_page_hide(ph),
            ));
        }
        Control::Bookmark(bm) => {
            records.push(make_ctrl_record(
                tags::CTRL_BOOKMARK,
                level,
                &serialize_bookmark(bm),
            ));
        }
        Control::Picture(pic) => serialize_picture_control(pic, level, ctrl_data_record, records),
        Control::Shape(shape) => serialize_shape_control(shape, level, ctrl_data_record, records),
        Control::CharOverlap(co) => {
            records.push(make_ctrl_record(
                tags::CTRL_TCPS,
                level,
                &serialize_char_overlap(co),
            ));
        }
        Control::Equation(eq) => serialize_equation_control(eq, level, records),
        Control::Field(f) => {
            // 필드 컨트롤 직렬화 (표 154)
            // ctrl_id(4) + 속성(4) + 기타속성(1) + command_len(2) + command(가변) + id(4)
            let cmd_utf16: Vec<u16> = f.command.encode_utf16().collect();
            let cmd_len = cmd_utf16.len();
            let mut data = Vec::with_capacity(4 + 4 + 1 + 2 + cmd_len * 2 + 4);
            data.extend_from_slice(&f.ctrl_id.to_le_bytes());
            data.extend_from_slice(&f.properties.to_le_bytes());
            data.push(f.extra_properties);
            data.extend_from_slice(&(cmd_len as u16).to_le_bytes());
            for ch in &cmd_utf16 {
                data.extend_from_slice(&ch.to_le_bytes());
            }
            data.extend_from_slice(&f.field_id.to_le_bytes());
            data.extend_from_slice(&f.memo_index.to_le_bytes());
            records.push(Record {
                tag_id: tags::HWPTAG_CTRL_HEADER,
                level,
                size: data.len() as u32,
                data,
            });
        }
        // 미구현 컨트롤은 최소한의 CTRL_HEADER만 생성
        Control::Hyperlink(_) | Control::Ruby(_) | Control::Form(_) | Control::Unknown(_) => {
            let ctrl_id = match ctrl {
                Control::Unknown(u) => u.ctrl_id,
                _ => 0,
            };
            if ctrl_id != 0 {
                let mut data = Vec::new();
                data.extend_from_slice(&ctrl_id.to_le_bytes());
                records.push(Record {
                    tag_id: tags::HWPTAG_CTRL_HEADER,
                    level,
                    size: data.len() as u32,
                    data,
                });
            }
        }
    }

    // CTRL_DATA 레코드 복원: CTRL_HEADER 바로 다음에 삽입 (라운드트립 보존)
    // Picture/Shape 컨트롤은 SHAPE_COMPONENT 내부(level+2)에도 추가 배치됨
    if let Some(data) = ctrl_data_record {
        let ctrl_data_pos = insert_pos + 1; // CTRL_HEADER 바로 다음
        records.insert(
            ctrl_data_pos,
            Record {
                tag_id: tags::HWPTAG_CTRL_DATA,
                level: level + 1,
                size: data.len() as u32,
                data: data.to_vec(),
            },
        );
    }
}

// ============================================================
// CTRL_HEADER 레코드 생성 헬퍼
// ============================================================

/// ctrl_id + ctrl_data로 CTRL_HEADER 레코드 생성
fn make_ctrl_record(ctrl_id: u32, level: u16, ctrl_data: &[u8]) -> Record {
    let mut data = Vec::with_capacity(4 + ctrl_data.len());
    data.extend_from_slice(&ctrl_id.to_le_bytes());
    data.extend_from_slice(ctrl_data);
    Record {
        tag_id: tags::HWPTAG_CTRL_HEADER,
        level,
        size: data.len() as u32,
        data,
    }
}

// ============================================================
// 구역 정의 ('secd')
// ============================================================

fn serialize_section_def(sd: &SectionDef, level: u16, records: &mut Vec<Record>) {
    let mut w = ByteWriter::new();
    w.write_u32(sd.flags);
    w.write_i16(sd.column_spacing);
    w.write_u16(0); // vertical_align
    w.write_u16(0); // horizontal_align
    w.write_u32(sd.default_tab_spacing);
    w.write_u16(sd.outline_numbering_id);
    w.write_u16(sd.page_num);
    w.write_u16(sd.picture_num);
    w.write_u16(sd.table_num);
    w.write_u16(sd.equation_num);
    // 원본 추가 바이트 복원 (라운드트립용)
    if !sd.raw_ctrl_extra.is_empty() {
        w.write_bytes(&sd.raw_ctrl_extra);
    }

    records.push(make_ctrl_record(
        tags::CTRL_SECTION_DEF,
        level,
        w.as_bytes(),
    ));

    // PAGE_DEF
    records.push(Record {
        tag_id: tags::HWPTAG_PAGE_DEF,
        level: level + 1,
        size: 0,
        data: serialize_page_def(&sd.page_def),
    });

    // FOOTNOTE_SHAPE (각주)
    records.push(Record {
        tag_id: tags::HWPTAG_FOOTNOTE_SHAPE,
        level: level + 1,
        size: 0,
        data: serialize_footnote_shape(&sd.footnote_shape),
    });

    // FOOTNOTE_SHAPE (미주)
    records.push(Record {
        tag_id: tags::HWPTAG_FOOTNOTE_SHAPE,
        level: level + 1,
        size: 0,
        data: serialize_footnote_shape(&sd.endnote_shape),
    });

    // PAGE_BORDER_FILL (첫 번째)
    records.push(Record {
        tag_id: tags::HWPTAG_PAGE_BORDER_FILL,
        level: level + 1,
        size: 0,
        data: serialize_page_border_fill(&sd.page_border_fill),
    });

    // 추가 PAGE_BORDER_FILL (2번째, 3번째 등)
    for pbf in &sd.extra_page_border_fills {
        records.push(Record {
            tag_id: tags::HWPTAG_PAGE_BORDER_FILL,
            level: level + 1,
            size: 0,
            data: serialize_page_border_fill(pbf),
        });
    }

    // 기타 자식 레코드 복원 (바탕쪽 LIST_HEADER + 문단 등)
    for raw in &sd.extra_child_records {
        records.push(Record {
            tag_id: raw.tag_id,
            level: raw.level,
            size: raw.data.len() as u32,
            data: raw.data.clone(),
        });
    }
}

fn serialize_page_def(pd: &PageDef) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.write_u32(pd.width);
    w.write_u32(pd.height);
    w.write_u32(pd.margin_left);
    w.write_u32(pd.margin_right);
    w.write_u32(pd.margin_top);
    w.write_u32(pd.margin_bottom);
    w.write_u32(pd.margin_header);
    w.write_u32(pd.margin_footer);
    w.write_u32(pd.margin_gutter);
    w.write_u32(pd.attr);
    w.into_bytes()
}

fn serialize_footnote_shape(fs: &FootnoteShape) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.write_u32(fs.attr);
    w.write_u16(fs.user_char as u16);
    w.write_u16(fs.prefix_char as u16);
    w.write_u16(fs.suffix_char as u16);
    w.write_u16(fs.start_number);
    w.write_i16(fs.separator_length);
    w.write_i16(fs.separator_margin_top);
    w.write_i16(fs.separator_margin_bottom);
    w.write_i16(fs.note_spacing);
    // 미문서화 2바이트 (원본 보존)
    w.write_u16(fs.raw_unknown);
    w.write_u8(fs.separator_line_type);
    w.write_u8(fs.separator_line_width);
    w.write_color_ref(fs.separator_color);
    w.into_bytes()
}

fn serialize_page_border_fill(pbf: &PageBorderFill) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.write_u32(pbf.attr);
    w.write_i16(pbf.spacing_left);
    w.write_i16(pbf.spacing_right);
    w.write_i16(pbf.spacing_top);
    w.write_i16(pbf.spacing_bottom);
    w.write_u16(pbf.border_fill_id);
    w.into_bytes()
}

// ============================================================
// 단 정의 ('cold')
// ============================================================

fn serialize_column_def(cd: &ColumnDef, level: u16, records: &mut Vec<Record>) {
    let mut w = ByteWriter::new();

    // 표 141: 속성 bit 0-15 (원본이 있으면 그대로, 없으면 재구성)
    let attr: u16 = if cd.raw_attr != 0 {
        cd.raw_attr
    } else {
        let mut a: u16 = match cd.column_type {
            ColumnType::Normal => 0,
            ColumnType::Distribute => 1,
            ColumnType::Parallel => 2,
        };
        // bit 2-9: 단 개수
        a |= (cd.column_count as u16 & 0xFF) << 2;
        // bit 10-11: 단 방향
        if cd.direction == ColumnDirection::RightToLeft {
            a |= 1 << 10;
        }
        // bit 12: 단 너비 동일
        if cd.same_width {
            a |= 1 << 12;
        }
        a
    };

    w.write_u16(attr);

    // hwplib 기준: same_width 여부에 따라 바이트 순서가 다름
    if !cd.same_width && cd.column_count > 1 {
        // same_width=false: [attr2(2)] [col0_width(2) col0_gap(2)] ...
        w.write_u16(0); // attr2
        for i in 0..cd.widths.len() {
            w.write_i16(cd.widths[i]);
            let gap = cd.gaps.get(i).copied().unwrap_or(0);
            w.write_i16(gap);
        }
    } else {
        // same_width=true: [gap(2)] [attr2(2)]
        w.write_i16(cd.spacing);
        w.write_u16(0); // attr2
    }

    w.write_u8(cd.separator_type);
    w.write_u8(cd.separator_width);
    w.write_color_ref(cd.separator_color);

    records.push(make_ctrl_record(tags::CTRL_COLUMN_DEF, level, w.as_bytes()));
}

// ============================================================
// 표 ('tbl ')
// ============================================================

fn serialize_table(table: &Table, level: u16, records: &mut Vec<Record>) {
    // CTRL_HEADER: raw_ctrl_data는 CommonObjAttr 전체 (attr 포함)
    // Task 271에서 파싱 변경: ctrl_data 전체 = CommonObjAttr
    records.push(make_ctrl_record(
        tags::CTRL_TABLE,
        level,
        if !table.raw_ctrl_data.is_empty() {
            &table.raw_ctrl_data
        } else {
            &[]
        },
    ));

    // 캡션 (TABLE 이전, level+1)
    if let Some(ref caption) = table.caption {
        serialize_caption(caption, level + 1, records);
    }

    // HWPTAG_TABLE 레코드
    records.push(Record {
        tag_id: tags::HWPTAG_TABLE,
        level: level + 1,
        size: 0,
        data: serialize_table_record(table),
    });

    // 셀 목록
    for cell in &table.cells {
        serialize_cell(cell, level + 1, records);
    }
}

fn serialize_table_record(table: &Table) -> Vec<u8> {
    let mut w = ByteWriter::new();

    // attr (원본이 있으면 그대로, 없으면 재구성)
    let attr = if table.raw_table_record_attr != 0 {
        table.raw_table_record_attr
    } else {
        let mut a: u32 = 0;
        match table.page_break {
            TablePageBreak::CellBreak => a |= 0x01,
            TablePageBreak::RowBreak => a |= 0x02,
            TablePageBreak::None => {}
        }
        if table.repeat_header {
            a |= 0x04;
        }
        a
    };
    w.write_u32(attr);

    w.write_u16(table.row_count);
    w.write_u16(table.col_count);
    w.write_i16(table.cell_spacing);

    // 안쪽 여백
    w.write_i16(table.padding.left);
    w.write_i16(table.padding.right);
    w.write_i16(table.padding.top);
    w.write_i16(table.padding.bottom);

    // 행별 셀 수 (HWP 스펙: UINT16[NRows])
    for &h in &table.row_sizes {
        w.write_i16(h);
    }

    w.write_u16(table.border_fill_id);

    // 원본 추가 바이트 복원 (라운드트립용)
    if !table.raw_table_record_extra.is_empty() {
        w.write_bytes(&table.raw_table_record_extra);
    }

    w.into_bytes()
}

fn serialize_cell(cell: &Cell, level: u16, records: &mut Vec<Record>) {
    let mut w = ByteWriter::new();

    // LIST_HEADER 공통 (6 + 2 = 8바이트)
    let n_paragraphs = cell.paragraphs.len() as u16;
    w.write_u16(n_paragraphs);

    // list_attr 재구성 (text_direction + vertical_align)
    let v_align_code: u32 = match cell.vertical_align {
        VerticalAlign::Top => 0,
        VerticalAlign::Center => 1,
        VerticalAlign::Bottom => 2,
    };
    let list_attr: u32 = ((cell.text_direction as u32) << 16) | (v_align_code << 21);
    w.write_u32(list_attr);
    w.write_u16(cell.list_header_width_ref);

    // 셀 속성
    w.write_u16(cell.col);
    w.write_u16(cell.row);
    w.write_u16(cell.col_span);
    w.write_u16(cell.row_span);
    w.write_u32(cell.width);
    w.write_u32(cell.height);
    w.write_i16(cell.padding.left);
    w.write_i16(cell.padding.right);
    w.write_i16(cell.padding.top);
    w.write_i16(cell.padding.bottom);
    w.write_u16(cell.border_fill_id);

    // 원본 추가 바이트 복원 (라운드트립용)
    if !cell.raw_list_extra.is_empty() {
        w.write_bytes(&cell.raw_list_extra);
    }

    records.push(Record {
        tag_id: tags::HWPTAG_LIST_HEADER,
        level,
        size: 0,
        data: w.into_bytes(),
    });

    // 셀 내부 문단 (원본 HWP에서는 LIST_HEADER와 같은 레벨)
    serialize_paragraph_list(&cell.paragraphs, level, records);
}

fn serialize_caption(caption: &Caption, level: u16, records: &mut Vec<Record>) {
    let mut w = ByteWriter::new();

    // LIST_HEADER 공통 (8바이트: n_para + list_attr + width_ref)
    let n_paragraphs = caption.paragraphs.len() as u16;
    w.write_u16(n_paragraphs);
    // list_attr: bit 21~22 = 세로 정렬 (Left/Right 캡션용)
    let vert_align_bits: u32 = match caption.vert_align {
        CaptionVertAlign::Top => 0,
        CaptionVertAlign::Center => 1,
        CaptionVertAlign::Bottom => 2,
    };
    let list_attr: u32 = vert_align_bits << 21;
    w.write_u32(list_attr);
    w.write_u16(0); // width_ref

    // 캡션 데이터
    let dir_val: u32 = match caption.direction {
        CaptionDirection::Left => 0,
        CaptionDirection::Right => 1,
        CaptionDirection::Top => 2,
        CaptionDirection::Bottom => 3,
    };
    let mut caption_attr = dir_val;
    if caption.include_margin {
        caption_attr |= 0x04;
    }
    w.write_u32(caption_attr);
    w.write_u32(caption.width);
    w.write_i16(caption.spacing);
    w.write_u32(caption.max_width);
    // 예약 필드 8바이트 (한컴 호환성: 원본 파일은 30바이트 LIST_HEADER)
    w.write_u32(0);
    w.write_u32(0);

    records.push(Record {
        tag_id: tags::HWPTAG_LIST_HEADER,
        level,
        size: 0,
        data: w.into_bytes(),
    });

    // 캡션 내부 문단 (LIST_HEADER와 같은 레벨)
    serialize_paragraph_list(&caption.paragraphs, level, records);
}

// ============================================================
// 머리말/꼬리말 ('head'/'foot')
// ============================================================

fn serialize_header_control(header: &Header, level: u16, records: &mut Vec<Record>) {
    let attr: u32 = if header.raw_attr != 0 {
        header.raw_attr
    } else {
        match header.apply_to {
            HeaderFooterApply::Both => 0,
            HeaderFooterApply::Even => 1,
            HeaderFooterApply::Odd => 2,
        }
    };
    let mut w = ByteWriter::new();
    w.write_u32(attr);
    if !header.raw_ctrl_extra.is_empty() {
        w.write_bytes(&header.raw_ctrl_extra);
    }
    records.push(make_ctrl_record(tags::CTRL_HEADER, level, w.as_bytes()));

    // LIST_HEADER + 문단
    serialize_list_header_with_paragraphs(&header.paragraphs, level + 1, records);
}

fn serialize_footer_control(footer: &Footer, level: u16, records: &mut Vec<Record>) {
    let attr: u32 = if footer.raw_attr != 0 {
        footer.raw_attr
    } else {
        match footer.apply_to {
            HeaderFooterApply::Both => 0,
            HeaderFooterApply::Even => 1,
            HeaderFooterApply::Odd => 2,
        }
    };
    let mut w = ByteWriter::new();
    w.write_u32(attr);
    if !footer.raw_ctrl_extra.is_empty() {
        w.write_bytes(&footer.raw_ctrl_extra);
    }
    records.push(make_ctrl_record(tags::CTRL_FOOTER, level, w.as_bytes()));

    serialize_list_header_with_paragraphs(&footer.paragraphs, level + 1, records);
}

// ============================================================
// 각주/미주 ('fn  '/'en  ')
// ============================================================

fn serialize_footnote(fn_: &Footnote, level: u16, records: &mut Vec<Record>) {
    let mut w = ByteWriter::new();
    w.write_u16(fn_.number);
    records.push(make_ctrl_record(tags::CTRL_FOOTNOTE, level, w.as_bytes()));

    serialize_list_header_with_paragraphs(&fn_.paragraphs, level + 1, records);
}

fn serialize_endnote(en: &Endnote, level: u16, records: &mut Vec<Record>) {
    let mut w = ByteWriter::new();
    w.write_u16(en.number);
    records.push(make_ctrl_record(tags::CTRL_ENDNOTE, level, w.as_bytes()));

    serialize_list_header_with_paragraphs(&en.paragraphs, level + 1, records);
}

// ============================================================
// 숨은 설명 ('tcmt')
// ============================================================

fn serialize_hidden_comment(comment: &HiddenComment, level: u16, records: &mut Vec<Record>) {
    records.push(make_ctrl_record(tags::CTRL_HIDDEN_COMMENT, level, &[]));
    serialize_list_header_with_paragraphs(&comment.paragraphs, level + 1, records);
}

// ============================================================
// 단순 컨트롤
// ============================================================

fn serialize_auto_number(an: &AutoNumber) -> Vec<u8> {
    let type_val: u32 = match an.number_type {
        AutoNumberType::Page => 0,
        AutoNumberType::Footnote => 1,
        AutoNumberType::Endnote => 2,
        AutoNumberType::Picture => 3,
        AutoNumberType::Table => 4,
        AutoNumberType::Equation => 5,
    };
    let mut attr: u32 = type_val & 0x0F;
    attr |= ((an.format as u32) & 0xFF) << 4; // bit 4~11: 번호 모양
    if an.superscript {
        attr |= 0x1000; // bit 12: 위 첨자
    }
    let mut data = Vec::new();
    data.extend_from_slice(&attr.to_le_bytes());
    // number가 0이면 assigned_number를 사용 (캡션 등 새로 생성된 AutoNumber)
    let num = if an.number > 0 {
        an.number
    } else {
        an.assigned_number
    };
    data.extend_from_slice(&num.to_le_bytes());
    data.extend_from_slice(&(an.user_symbol as u16).to_le_bytes());
    data.extend_from_slice(&(an.prefix_char as u16).to_le_bytes());
    data.extend_from_slice(&(an.suffix_char as u16).to_le_bytes());
    data
}

fn serialize_new_number(nn: &NewNumber) -> Vec<u8> {
    let type_val: u32 = match nn.number_type {
        AutoNumberType::Page => 0,
        AutoNumberType::Footnote => 1,
        AutoNumberType::Endnote => 2,
        AutoNumberType::Picture => 3,
        AutoNumberType::Table => 4,
        AutoNumberType::Equation => 5,
    };
    let attr: u32 = type_val & 0x0F;
    let mut data = Vec::new();
    data.extend_from_slice(&attr.to_le_bytes());
    data.extend_from_slice(&nn.number.to_le_bytes());
    data
}

fn serialize_page_num_pos(pnp: &PageNumberPos) -> Vec<u8> {
    let attr: u32 = (pnp.format as u32 & 0xFF) | ((pnp.position as u32 & 0x0F) << 8);
    let mut data = Vec::new();
    data.extend_from_slice(&attr.to_le_bytes());
    data.extend_from_slice(&(pnp.user_symbol as u16).to_le_bytes());
    data.extend_from_slice(&(pnp.prefix_char as u16).to_le_bytes());
    data.extend_from_slice(&(pnp.suffix_char as u16).to_le_bytes());
    data.extend_from_slice(&(pnp.dash_char as u16).to_le_bytes());
    data
}

fn serialize_page_hide(ph: &PageHide) -> Vec<u8> {
    let mut attr: u32 = 0;
    if ph.hide_header {
        attr |= 0x01;
    }
    if ph.hide_footer {
        attr |= 0x02;
    }
    if ph.hide_master_page {
        attr |= 0x04;
    }
    if ph.hide_border {
        attr |= 0x08;
    }
    if ph.hide_fill {
        attr |= 0x10;
    }
    if ph.hide_page_num {
        attr |= 0x20;
    }
    attr.to_le_bytes().to_vec()
}

fn serialize_bookmark(bm: &Bookmark) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.write_hwp_string(&bm.name);
    w.into_bytes()
}

/// 글자 겹침 직렬화 (HWP 스펙 표 152)
fn serialize_char_overlap(co: &CharOverlap) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.write_u16(co.chars.len() as u16);
    for &ch in &co.chars {
        w.write_u16(ch as u16);
    }
    w.write_u8(co.border_type);
    w.write_i8(co.inner_char_size);
    w.write_u8(co.expansion);
    w.write_u8(co.char_shape_ids.len() as u8);
    for &id in &co.char_shape_ids {
        w.write_u32(id);
    }
    w.into_bytes()
}

// ============================================================
// 그림 ('gso ' + Picture)
// ============================================================

fn serialize_picture_control(
    pic: &Picture,
    level: u16,
    ctrl_data_record: Option<&[u8]>,
    records: &mut Vec<Record>,
) {
    // CTRL_HEADER: ctrl_id(gso) + common_obj_attr
    records.push(make_ctrl_record(
        tags::CTRL_GEN_SHAPE,
        level,
        &serialize_common_obj_attr(&pic.common),
    ));

    // 캡션 (SHAPE_COMPONENT 앞, level+1)
    if let Some(ref caption) = pic.caption {
        serialize_caption(caption, level + 1, records);
    }

    // SHAPE_COMPONENT
    records.push(Record {
        tag_id: tags::HWPTAG_SHAPE_COMPONENT,
        level: level + 1,
        size: 0,
        data: serialize_shape_component(tags::SHAPE_PICTURE_ID, &pic.shape_attr, true),
    });

    // CTRL_DATA: SHAPE_COMPONENT 자식으로 배치 (level+2)
    if let Some(data) = ctrl_data_record {
        records.push(Record {
            tag_id: tags::HWPTAG_CTRL_DATA,
            level: level + 2,
            size: data.len() as u32,
            data: data.to_vec(),
        });
    }

    // SHAPE_COMPONENT_PICTURE (SHAPE_COMPONENT의 자식)
    records.push(Record {
        tag_id: tags::HWPTAG_SHAPE_COMPONENT_PICTURE,
        level: level + 2,
        size: 0,
        data: serialize_picture_data(pic),
    });
}

fn serialize_picture_data(pic: &Picture) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.write_color_ref(pic.border_color);
    w.write_i32(pic.border_width);
    w.write_u32(0); // border_attr

    for &x in &pic.border_x {
        w.write_i32(x);
    }
    for &y in &pic.border_y {
        w.write_i32(y);
    }

    // 자르기 정보
    w.write_i32(pic.crop.left);
    w.write_i32(pic.crop.top);
    w.write_i32(pic.crop.right);
    w.write_i32(pic.crop.bottom);

    // 안쪽 여백
    w.write_i16(pic.padding.left);
    w.write_i16(pic.padding.right);
    w.write_i16(pic.padding.top);
    w.write_i16(pic.padding.bottom);

    // 이미지 속성
    w.write_i8(pic.image_attr.brightness);
    w.write_i8(pic.image_attr.contrast);
    let effect_val: u8 = match pic.image_attr.effect {
        ImageEffect::RealPic => 0,
        ImageEffect::GrayScale => 1,
        ImageEffect::BlackWhite => 2,
        ImageEffect::Pattern8x8 => 3,
    };
    w.write_u8(effect_val);
    w.write_u16(pic.image_attr.bin_data_id);

    // 원본 추가 바이트 복원 (라운드트립 보존)
    if !pic.raw_picture_extra.is_empty() {
        w.write_bytes(&pic.raw_picture_extra);
    } else {
        // border_opacity(1) + instance_id(4) + image_effect(4) = 9바이트
        w.write_u8(pic.border_opacity);
        w.write_u32(pic.instance_id);
        w.write_u32(0); // image_effect_extra
                        // 원본 이미지 크기(HWPUNIT) + 플래그(1): 한컴 호환 추가 9바이트
        w.write_u32(pic.crop.right as u32); // original width in HWPUNIT
        w.write_u32(pic.crop.bottom as u32); // original height in HWPUNIT
        w.write_u8(0); // flag
    }

    w.into_bytes()
}

// ============================================================
// 도형 ('gso ' + Shape)
// ============================================================

fn serialize_shape_control(
    shape: &ShapeObject,
    level: u16,
    ctrl_data_record: Option<&[u8]>,
    records: &mut Vec<Record>,
) {
    // CTRL_DATA를 SHAPE_COMPONENT 자식으로 배치하는 헬퍼
    let emit_ctrl_data = |records: &mut Vec<Record>| {
        if let Some(data) = ctrl_data_record {
            records.push(Record {
                tag_id: tags::HWPTAG_CTRL_DATA,
                level: level + 2,
                size: data.len() as u32,
                data: data.to_vec(),
            });
        }
    };

    match shape {
        ShapeObject::Line(line) => {
            let is_connector = line.connector.is_some();
            let sc_ctrl_id = if is_connector {
                tags::SHAPE_CONNECTOR_ID
            } else {
                tags::SHAPE_LINE_ID
            };
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&line.common),
            ));
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(sc_ctrl_id, &line.drawing, true),
            });
            emit_ctrl_data(records);
            serialize_text_box_if_present(&line.drawing, level + 2, records);
            let mut w = ByteWriter::new();
            w.write_i32(line.start.x);
            w.write_i32(line.start.y);
            w.write_i32(line.end.x);
            w.write_i32(line.end.y);
            if let Some(ref conn) = line.connector {
                // 연결선 확장 데이터
                w.write_u32(conn.link_type as u32);
                w.write_u32(conn.start_subject_id);
                w.write_u32(conn.start_subject_index);
                w.write_u32(conn.end_subject_id);
                w.write_u32(conn.end_subject_index);
                w.write_u32(conn.control_points.len() as u32);
                for cp in &conn.control_points {
                    w.write_i32(cp.x);
                    w.write_i32(cp.y);
                    w.write_u16(cp.point_type);
                }
                w.write_bytes(&conn.raw_trailing);
            } else {
                w.write_i32(if line.started_right_or_bottom { 1 } else { 0 });
            }
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_LINE,
                level: level + 2,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Rectangle(rect) => {
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&rect.common),
            ));
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(tags::SHAPE_RECT_ID, &rect.drawing, true),
            });
            emit_ctrl_data(records);
            // 글상자(텍스트) 내용 직렬화
            serialize_text_box_if_present(&rect.drawing, level + 2, records);
            let mut w = ByteWriter::new();
            w.write_u8(rect.round_rate);
            for i in 0..4 {
                w.write_i32(rect.x_coords[i]);
                w.write_i32(rect.y_coords[i]);
            }
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_RECTANGLE,
                level: level + 2,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Ellipse(ellipse) => {
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&ellipse.common),
            ));
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(
                    tags::SHAPE_ELLIPSE_ID,
                    &ellipse.drawing,
                    true,
                ),
            });
            emit_ctrl_data(records);
            serialize_text_box_if_present(&ellipse.drawing, level + 2, records);
            let mut w = ByteWriter::new();
            w.write_u32(ellipse.attr);
            w.write_i32(ellipse.center.x);
            w.write_i32(ellipse.center.y);
            w.write_i32(ellipse.axis1.x);
            w.write_i32(ellipse.axis1.y);
            w.write_i32(ellipse.axis2.x);
            w.write_i32(ellipse.axis2.y);
            w.write_i32(ellipse.start1.x);
            w.write_i32(ellipse.start1.y);
            w.write_i32(ellipse.end1.x);
            w.write_i32(ellipse.end1.y);
            w.write_i32(ellipse.start2.x);
            w.write_i32(ellipse.start2.y);
            w.write_i32(ellipse.end2.x);
            w.write_i32(ellipse.end2.y);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_ELLIPSE,
                level: level + 2,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Polygon(poly) => {
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&poly.common),
            ));
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(
                    tags::SHAPE_POLYGON_ID,
                    &poly.drawing,
                    true,
                ),
            });
            emit_ctrl_data(records);
            serialize_text_box_if_present(&poly.drawing, level + 2, records);
            let mut w = ByteWriter::new();
            w.write_i32(poly.points.len() as i32);
            for p in &poly.points {
                w.write_i32(p.x);
                w.write_i32(p.y);
            }
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_POLYGON,
                level: level + 2,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Arc(arc) => {
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&arc.common),
            ));
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(tags::SHAPE_ARC_ID, &arc.drawing, true),
            });
            emit_ctrl_data(records);
            serialize_text_box_if_present(&arc.drawing, level + 2, records);
            let mut w = ByteWriter::new();
            w.write_u8(arc.arc_type);
            w.write_i32(arc.center.x);
            w.write_i32(arc.center.y);
            w.write_i32(arc.axis1.x);
            w.write_i32(arc.axis1.y);
            w.write_i32(arc.axis2.x);
            w.write_i32(arc.axis2.y);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_ARC,
                level: level + 2,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Curve(curve) => {
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&curve.common),
            ));
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(tags::SHAPE_CURVE_ID, &curve.drawing, true),
            });
            emit_ctrl_data(records);
            serialize_text_box_if_present(&curve.drawing, level + 2, records);
            let mut w = ByteWriter::new();
            w.write_i32(curve.points.len() as i32);
            for p in &curve.points {
                w.write_i32(p.x);
                w.write_i32(p.y);
            }
            for &t in &curve.segment_types {
                w.write_u8(t);
            }
            // hwplib: sr.skip(4) — 4바이트 패딩
            w.write_u32(0);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_CURVE,
                level: level + 2,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Group(group) => {
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&group.common),
            ));
            // 그룹 컨테이너: SHAPE_COMPONENT + 자식 수 + 자식 ctrl_id 목록 (한컴 호환)
            {
                let mut data = serialize_shape_component(0x24636f6e, &group.shape_attr, true); // '$con'
                                                                                               // 자식 수 (u16)
                let mut w = ByteWriter::new();
                w.write_u16(group.children.len() as u16);
                // 각 자식의 ctrl_id (u32)
                for child in &group.children {
                    let child_ctrl_id = match child {
                        ShapeObject::Line(_) => tags::SHAPE_LINE_ID,
                        ShapeObject::Rectangle(_) => tags::SHAPE_RECT_ID,
                        ShapeObject::Ellipse(_) => tags::SHAPE_ELLIPSE_ID,
                        ShapeObject::Arc(_) => tags::SHAPE_ARC_ID,
                        ShapeObject::Polygon(_) => tags::SHAPE_POLYGON_ID,
                        ShapeObject::Curve(_) => tags::SHAPE_CURVE_ID,
                        ShapeObject::Group(_) => tags::CTRL_GEN_SHAPE,
                        ShapeObject::Picture(_) => tags::SHAPE_PICTURE_ID,
                        ShapeObject::Chart(c) => c.drawing.shape_attr.ctrl_id,
                        ShapeObject::Ole(o) => o.drawing.shape_attr.ctrl_id,
                    };
                    w.write_u32(child_ctrl_id);
                }
                // instance_id (한컴 호환)
                w.write_u32(group.common.instance_id);
                data.extend_from_slice(&w.into_bytes());
                records.push(Record {
                    tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                    level: level + 1,
                    size: 0,
                    data,
                });
            }
            emit_ctrl_data(records);
            // 자식 개체 직렬화 (CTRL_HEADER 없이 SHAPE_COMPONENT + 도형별 태그)
            let child_comp_level = level + 2;
            let child_type_level = level + 3;
            for child in &group.children {
                serialize_group_child(child, child_comp_level, child_type_level, records);
            }
        }
        ShapeObject::Picture(_pic) => {
            // 그룹 내 그림: 그룹 직렬화 시 자식으로 처리됨 (단독 Picture는 Control::Picture로 직렬화)
        }
        ShapeObject::Chart(chart) => {
            // Task #195 단계 2: raw_chart_data를 그대로 보존하여 라운드트립 유지
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&chart.common),
            ));
            let sc_ctrl_id = chart.drawing.shape_attr.ctrl_id;
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(sc_ctrl_id, &chart.drawing, true),
            });
            emit_ctrl_data(records);
            serialize_text_box_if_present(&chart.drawing, level + 2, records);
            records.push(Record {
                tag_id: tags::HWPTAG_CHART_DATA,
                level: level + 2,
                size: 0,
                data: chart.raw_chart_data.clone(),
            });
        }
        ShapeObject::Ole(ole) => {
            // Task #195 단계 2: raw_tag_data를 그대로 보존하여 라운드트립 유지
            records.push(make_ctrl_record(
                tags::CTRL_GEN_SHAPE,
                level,
                &serialize_common_obj_attr(&ole.common),
            ));
            let sc_ctrl_id = ole.drawing.shape_attr.ctrl_id;
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: level + 1,
                size: 0,
                data: serialize_drawing_shape_component(sc_ctrl_id, &ole.drawing, true),
            });
            emit_ctrl_data(records);
            serialize_text_box_if_present(&ole.drawing, level + 2, records);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_OLE,
                level: level + 2,
                size: 0,
                data: ole.raw_tag_data.clone(),
            });
        }
    }
}

/// 그룹 자식 개체 직렬화 (CTRL_HEADER 없이 SHAPE_COMPONENT + 도형별 태그)
fn serialize_group_child(
    child: &ShapeObject,
    comp_level: u16, // SHAPE_COMPONENT level
    type_level: u16, // 도형별 태그 level
    records: &mut Vec<Record>,
) {
    use crate::parser::tags;

    match child {
        ShapeObject::Line(line) => {
            let sc_ctrl_id = if line.connector.is_some() {
                tags::SHAPE_CONNECTOR_ID
            } else {
                tags::SHAPE_LINE_ID
            };
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(sc_ctrl_id, &line.drawing, false),
            });
            serialize_text_box_if_present(&line.drawing, type_level, records);
            let mut w = ByteWriter::new();
            w.write_i32(line.start.x);
            w.write_i32(line.start.y);
            w.write_i32(line.end.x);
            w.write_i32(line.end.y);
            if let Some(ref conn) = line.connector {
                w.write_u32(conn.link_type as u32);
                w.write_u32(conn.start_subject_id);
                w.write_u32(conn.start_subject_index);
                w.write_u32(conn.end_subject_id);
                w.write_u32(conn.end_subject_index);
                w.write_u32(conn.control_points.len() as u32);
                for cp in &conn.control_points {
                    w.write_i32(cp.x);
                    w.write_i32(cp.y);
                    w.write_u16(cp.point_type);
                }
                w.write_bytes(&conn.raw_trailing);
            } else {
                w.write_i32(if line.started_right_or_bottom { 1 } else { 0 });
            }
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_LINE,
                level: type_level,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Rectangle(rect) => {
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(tags::SHAPE_RECT_ID, &rect.drawing, false),
            });
            serialize_text_box_if_present(&rect.drawing, type_level, records);
            let mut w = ByteWriter::new();
            w.write_u8(rect.round_rate);
            for i in 0..4 {
                w.write_i32(rect.x_coords[i]);
                w.write_i32(rect.y_coords[i]);
            }
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_RECTANGLE,
                level: type_level,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Ellipse(ellipse) => {
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(
                    tags::SHAPE_ELLIPSE_ID,
                    &ellipse.drawing,
                    false,
                ),
            });
            serialize_text_box_if_present(&ellipse.drawing, type_level, records);
            let mut w = ByteWriter::new();
            w.write_u32(ellipse.attr);
            w.write_i32(ellipse.center.x);
            w.write_i32(ellipse.center.y);
            w.write_i32(ellipse.axis1.x);
            w.write_i32(ellipse.axis1.y);
            w.write_i32(ellipse.axis2.x);
            w.write_i32(ellipse.axis2.y);
            w.write_i32(ellipse.start1.x);
            w.write_i32(ellipse.start1.y);
            w.write_i32(ellipse.end1.x);
            w.write_i32(ellipse.end1.y);
            w.write_i32(ellipse.start2.x);
            w.write_i32(ellipse.start2.y);
            w.write_i32(ellipse.end2.x);
            w.write_i32(ellipse.end2.y);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_ELLIPSE,
                level: type_level,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Arc(arc) => {
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(tags::SHAPE_ARC_ID, &arc.drawing, false),
            });
            serialize_text_box_if_present(&arc.drawing, type_level, records);
            let mut w = ByteWriter::new();
            w.write_u8(arc.arc_type);
            w.write_i32(arc.center.x);
            w.write_i32(arc.center.y);
            w.write_i32(arc.axis1.x);
            w.write_i32(arc.axis1.y);
            w.write_i32(arc.axis2.x);
            w.write_i32(arc.axis2.y);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_ARC,
                level: type_level,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Polygon(poly) => {
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(
                    tags::SHAPE_POLYGON_ID,
                    &poly.drawing,
                    false,
                ),
            });
            serialize_text_box_if_present(&poly.drawing, type_level, records);
            let mut w = ByteWriter::new();
            w.write_i32(poly.points.len() as i32);
            for p in &poly.points {
                w.write_i32(p.x);
                w.write_i32(p.y);
            }
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_POLYGON,
                level: type_level,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Curve(curve) => {
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(
                    tags::SHAPE_CURVE_ID,
                    &curve.drawing,
                    false,
                ),
            });
            serialize_text_box_if_present(&curve.drawing, type_level, records);
            let mut w = ByteWriter::new();
            w.write_i32(curve.points.len() as i32);
            for p in &curve.points {
                w.write_i32(p.x);
                w.write_i32(p.y);
            }
            for &t in &curve.segment_types {
                w.write_u8(t);
            }
            w.write_u32(0);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_CURVE,
                level: type_level,
                size: 0,
                data: w.into_bytes(),
            });
        }
        ShapeObject::Group(group) => {
            // 중첩 그룹
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_CONTAINER,
                level: comp_level,
                size: 0,
                data: serialize_shape_component(tags::CTRL_GEN_SHAPE, &group.shape_attr, false),
            });
            for nested_child in &group.children {
                serialize_group_child(nested_child, comp_level + 1, comp_level + 2, records);
            }
        }
        ShapeObject::Picture(pic) => {
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_shape_component(tags::SHAPE_PICTURE_ID, &pic.shape_attr, false),
            });
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_PICTURE,
                level: type_level,
                size: 0,
                data: serialize_picture_data(pic),
            });
        }
        ShapeObject::Chart(chart) => {
            // Task #195 단계 2: 그룹 내 차트는 raw_chart_data로 라운드트립
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(
                    chart.drawing.shape_attr.ctrl_id,
                    &chart.drawing,
                    false,
                ),
            });
            serialize_text_box_if_present(&chart.drawing, type_level, records);
            records.push(Record {
                tag_id: tags::HWPTAG_CHART_DATA,
                level: type_level,
                size: 0,
                data: chart.raw_chart_data.clone(),
            });
        }
        ShapeObject::Ole(ole) => {
            // Task #195 단계 2: 그룹 내 OLE는 raw_tag_data로 라운드트립
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT,
                level: comp_level,
                size: 0,
                data: serialize_drawing_shape_component(
                    ole.drawing.shape_attr.ctrl_id,
                    &ole.drawing,
                    false,
                ),
            });
            serialize_text_box_if_present(&ole.drawing, type_level, records);
            records.push(Record {
                tag_id: tags::HWPTAG_SHAPE_COMPONENT_OLE,
                level: type_level,
                size: 0,
                data: ole.raw_tag_data.clone(),
            });
        }
    }
}

/// DrawingObjAttr의 text_box가 있으면 LIST_HEADER + 문단 목록 직렬화
fn serialize_text_box_if_present(drawing: &DrawingObjAttr, level: u16, records: &mut Vec<Record>) {
    if let Some(ref text_box) = drawing.text_box {
        // LIST_HEADER
        let mut w = ByteWriter::new();
        // para_count: 스펙은 INT16이지만 실제 HWP 파일에서는 UINT32로 저장됨
        w.write_u32(text_box.paragraphs.len() as u32);
        w.write_u32(text_box.list_attr);
        // 여백 + 최대 폭 (글상자 고유 데이터)
        w.write_i16(text_box.margin_left);
        w.write_i16(text_box.margin_right);
        w.write_i16(text_box.margin_top);
        w.write_i16(text_box.margin_bottom);
        w.write_u32(text_box.max_width);
        // 원본 추가 바이트 복원 (라운드트립 보존)
        if !text_box.raw_list_header_extra.is_empty() {
            w.write_bytes(&text_box.raw_list_header_extra);
        }
        records.push(Record {
            tag_id: tags::HWPTAG_LIST_HEADER,
            level,
            size: 0,
            data: w.into_bytes(),
        });

        // 문단 목록 (LIST_HEADER와 같은 레벨)
        serialize_paragraph_list(&text_box.paragraphs, level, records);
    }
}

// ============================================================
// 공통 직렬화 헬퍼
// ============================================================

/// CommonObjAttr 직렬화
fn serialize_common_obj_attr(common: &CommonObjAttr) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.write_u32(common.attr);
    w.write_u32(common.vertical_offset);
    w.write_u32(common.horizontal_offset);
    w.write_u32(common.width);
    w.write_u32(common.height);
    w.write_i32(common.z_order);
    w.write_i16(common.margin.left);
    w.write_i16(common.margin.right);
    w.write_i16(common.margin.top);
    w.write_i16(common.margin.bottom);
    w.write_u32(common.instance_id);
    // 쪽나눔 방지 (INT32)
    w.write_i32(common.prevent_page_break);
    // 설명문 (항상 길이 포함, 빈 문자열이면 0)
    w.write_hwp_string(&common.description);
    // 원본 추가 바이트 복원 (라운드트립 보존)
    if !common.raw_extra.is_empty() {
        w.write_bytes(&common.raw_extra);
    }
    w.into_bytes()
}

/// SHAPE_COMPONENT 데이터 직렬화 (ShapeComponentAttr만 — Picture, Group용)
///
/// 구조: ctrl_id(×1 or ×2) + ShapeComponentAttr + rendering_info
fn serialize_shape_component(
    default_ctrl_id: u32,
    attr: &ShapeComponentAttr,
    top_level: bool,
) -> Vec<u8> {
    let mut w = ByteWriter::new();
    write_shape_component_base(&mut w, default_ctrl_id, attr, top_level);
    w.into_bytes()
}

/// SHAPE_COMPONENT 데이터 직렬화 (DrawingObjAttr 전체 — 도형용)
///
/// 구조: ctrl_id(×1 or ×2) + ShapeComponentAttr + rendering_info + border_line + fill + shadow
fn serialize_drawing_shape_component(
    default_ctrl_id: u32,
    drawing: &DrawingObjAttr,
    top_level: bool,
) -> Vec<u8> {
    let mut w = ByteWriter::new();
    write_shape_component_base(&mut w, default_ctrl_id, &drawing.shape_attr, top_level);

    // 테두리 선 정보 (13바이트: color 4 + width 4 + attr 4 + outline 1)
    w.write_color_ref(drawing.border_line.color);
    w.write_i32(drawing.border_line.width);
    w.write_u32(drawing.border_line.attr);
    w.write_u8(drawing.border_line.outline_style);

    // 채우기 정보
    serialize_shape_fill(&mut w, &drawing.fill);

    // 그림자 정보 (16바이트)
    w.write_u32(drawing.shadow_type);
    w.write_color_ref(drawing.shadow_color);
    w.write_i32(drawing.shadow_offset_x);
    w.write_i32(drawing.shadow_offset_y);

    // 인스턴스 ID (4바이트) + 예약 (1바이트) + 그림자 투명도 (1바이트)
    w.write_u32(drawing.inst_id);
    w.write_u8(0); // 예약
    w.write_u8(drawing.shadow_alpha);

    w.into_bytes()
}

/// ShapeComponentAttr 공통 기록 (ctrl_id + 속성 + 렌더링 행렬)
fn write_shape_component_base(
    w: &mut ByteWriter,
    default_ctrl_id: u32,
    attr: &ShapeComponentAttr,
    top_level: bool,
) {
    // ctrl_id: 원본에서 보존된 값 사용, 없으면 기본값
    let actual_id = if attr.ctrl_id != 0 {
        attr.ctrl_id
    } else {
        default_ctrl_id
    };
    let is_two = if attr.ctrl_id != 0 {
        attr.is_two_ctrl_id
    } else {
        top_level
    };

    w.write_u32(actual_id);
    if is_two {
        w.write_u32(actual_id);
    }

    // ShapeComponentAttr
    w.write_i32(attr.offset_x);
    w.write_i32(attr.offset_y);
    w.write_u16(attr.group_level);
    w.write_u16(attr.local_file_version);
    w.write_u32(attr.original_width);
    w.write_u32(attr.original_height);
    w.write_u32(attr.current_width);
    w.write_u32(attr.current_height);

    // flip: 원본 전체 값 사용 (상위 비트 보존)
    let flip = if attr.flip != 0 {
        attr.flip
    } else {
        let mut f: u32 = 0;
        if attr.horz_flip {
            f |= 0x01;
        }
        if attr.vert_flip {
            f |= 0x02;
        }
        f
    };
    w.write_u32(flip);

    w.write_i16(attr.rotation_angle);
    w.write_i32(attr.rotation_center.x);
    w.write_i32(attr.rotation_center.y);

    // Rendering 정보 (원본이 있으면 복원, 없으면 적절한 행렬 생성)
    if !attr.raw_rendering.is_empty() {
        w.write_bytes(&attr.raw_rendering);
    } else {
        let is_group_child = attr.group_level > 0;
        let cnt: u16 = if is_group_child { 2 } else { 1 };
        w.write_u16(cnt);
        // translation matrix [1, 0, tx, 0, 1, ty]
        w.write_f64(1.0);
        w.write_f64(0.0);
        w.write_f64(attr.offset_x as f64); // tx (그룹 자식: 로컬 offset)
        w.write_f64(0.0);
        w.write_f64(1.0);
        w.write_f64(attr.offset_y as f64); // ty
                                           // scale matrix = identity [1, 0, 0, 0, 1, 0]
                                           // (스케일은 current_width/original_width 값으로 표현 — 행렬에 중복 기록하면 이중 적용됨)
        w.write_f64(1.0);
        w.write_f64(0.0);
        w.write_f64(0.0);
        w.write_f64(0.0);
        w.write_f64(1.0);
        w.write_f64(0.0);
        // rotation matrix = identity [1, 0, 0, 0, 1, 0]
        w.write_f64(1.0);
        w.write_f64(0.0);
        w.write_f64(0.0);
        w.write_f64(0.0);
        w.write_f64(1.0);
        w.write_f64(0.0);
        // 그룹 자식 (cnt=2): 두 번째 scale + rotation 세트 (identity)
        if is_group_child {
            // scale2 = identity
            w.write_f64(1.0);
            w.write_f64(0.0);
            w.write_f64(0.0);
            w.write_f64(0.0);
            w.write_f64(1.0);
            w.write_f64(0.0);
            // rotation2 = identity
            w.write_f64(1.0);
            w.write_f64(0.0);
            w.write_f64(0.0);
            w.write_f64(0.0);
            w.write_f64(1.0);
            w.write_f64(0.0);
        }
    }
}

/// 도형 채우기 직렬화 (SHAPE_COMPONENT 내부 — parse_fill과 동일한 형식)
fn serialize_shape_fill(w: &mut ByteWriter, fill: &Fill) {
    let fill_type_val: u32 = match fill.fill_type {
        FillType::None => 0,
        FillType::Solid => 1,
        FillType::Image => 2,
        FillType::Gradient => 4,
    };
    w.write_u32(fill_type_val);

    if fill_type_val == 0 {
        // 채우기 없음: 4바이트 추가 (additional_size = 0)
        w.write_u32(0);
        return;
    }

    // bit 0: 단색 채우기
    if fill_type_val & 0x01 != 0 {
        if let Some(ref solid) = fill.solid {
            w.write_color_ref(solid.background_color);
            w.write_color_ref(solid.pattern_color);
            w.write_i32(solid.pattern_type);
        }
    }

    // bit 2: 그라데이션 채우기 (parse_fill 형식: kind=u8, angle/cx/cy/blur/count=u32)
    if fill_type_val & 0x04 != 0 {
        if let Some(ref grad) = fill.gradient {
            w.write_u8(grad.gradient_type as u8);
            w.write_u32(grad.angle as u32);
            w.write_u32(grad.center_x as u32);
            w.write_u32(grad.center_y as u32);
            w.write_u32(grad.blur as u32);
            w.write_u32(grad.colors.len() as u32);
            // change_points: count > 2일 때만 기록
            if grad.colors.len() > 2 {
                for &pos in &grad.positions {
                    w.write_i32(pos);
                }
            }
            for &color in &grad.colors {
                w.write_color_ref(color);
            }
        }
    }

    // bit 1: 이미지 채우기
    if fill_type_val & 0x02 != 0 {
        if let Some(ref img) = fill.image {
            let mode_val: u8 = match img.fill_mode {
                ImageFillMode::TileAll => 0,
                ImageFillMode::TileHorzTop => 1,
                ImageFillMode::TileHorzBottom => 2,
                ImageFillMode::TileVertLeft => 3,
                ImageFillMode::TileVertRight => 4,
                ImageFillMode::FitToSize => 5,
                ImageFillMode::Center => 6,
                ImageFillMode::CenterTop => 7,
                ImageFillMode::CenterBottom => 8,
                ImageFillMode::LeftCenter => 9,
                ImageFillMode::LeftTop => 10,
                ImageFillMode::LeftBottom => 11,
                ImageFillMode::RightCenter => 12,
                ImageFillMode::RightTop => 13,
                ImageFillMode::RightBottom => 14,
                ImageFillMode::None => 15,
            };
            w.write_u8(mode_val);
            w.write_i8(img.brightness);
            w.write_i8(img.contrast);
            w.write_u8(img.effect);
            w.write_u16(img.bin_data_id);
        }
    }

    // 추가 속성 (additional_size = 0)
    w.write_u32(0);

    // alpha 바이트 (채우기 종류별 각 1바이트)
    if fill_type_val & 0x01 != 0 {
        w.write_u8(fill.alpha);
    }
    if fill_type_val & 0x04 != 0 {
        w.write_u8(fill.alpha);
    }
    if fill_type_val & 0x02 != 0 {
        w.write_u8(fill.alpha);
    }
}

/// LIST_HEADER(간단) + 문단 목록 직렬화
fn serialize_list_header_with_paragraphs(
    paragraphs: &[crate::model::paragraph::Paragraph],
    level: u16,
    records: &mut Vec<Record>,
) {
    let mut w = ByteWriter::new();
    w.write_u16(paragraphs.len() as u16);
    w.write_u32(0); // list_attr

    records.push(Record {
        tag_id: tags::HWPTAG_LIST_HEADER,
        level,
        size: 0,
        data: w.into_bytes(),
    });

    serialize_paragraph_list(paragraphs, level + 1, records);
}

// ============================================================
// 수식 ('eqed')
// ============================================================

/// 수식 컨트롤 직렬화
///
/// raw_ctrl_data를 보존하여 라운드트립 무손실 직렬화.
fn serialize_equation_control(eq: &Equation, level: u16, records: &mut Vec<Record>) {
    // CTRL_HEADER with CommonObjAttr (또는 원본 ctrl_data)
    let ctrl_data = if eq.raw_ctrl_data.is_empty() {
        serialize_common_obj_attr(&eq.common)
    } else {
        eq.raw_ctrl_data.clone()
    };
    records.push(make_ctrl_record(tags::CTRL_EQUATION, level, &ctrl_data));

    // HWPTAG_EQEDIT 자식 레코드
    let mut w = ByteWriter::new();
    // attr: u32
    w.write_u32(0);
    // script: HWP string (length-prefixed UTF-16LE)
    w.write_hwp_string(&eq.script);
    // font_size: u32
    w.write_u32(eq.font_size);
    // color: u32
    w.write_u32(eq.color);
    // baseline: i16
    w.write_i16(eq.baseline);
    // version_info: HWP string
    w.write_hwp_string(&eq.version_info);
    // font_name: HWP string
    w.write_hwp_string(&eq.font_name);

    records.push(Record {
        tag_id: tags::HWPTAG_EQEDIT,
        level: level + 1,
        size: 0,
        data: w.into_bytes(),
    });
}

#[cfg(test)]
mod tests;
