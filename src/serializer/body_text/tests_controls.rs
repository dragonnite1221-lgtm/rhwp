//! body_text 직렬화 라운드트립 테스트 (3/3): 컨트롤 문자 코드 / section_def / break_type.
//!
//! `serializer/body_text.rs` 소스 파일이 커지는 것을 막기 위해 인라인
//! `mod tests { ... }` 블록을 세 파일로 나눠 분리했다 (내용은 변경 없음).

use super::*;
use crate::model::control::{AutoNumber, Control};
use crate::model::document::SectionDef;
use crate::model::paragraph::{ColumnBreakType, Paragraph};
use crate::parser::body_text::parse_body_text_section;

    /// 컨트롤 문자 코드 매핑 테스트
    #[test]
    fn test_control_char_code() {
        assert_eq!(
            control_char_code_and_id(&Control::SectionDef(Box::default())).0,
            0x0002
        );
        assert_eq!(
            control_char_code_and_id(&Control::AutoNumber(AutoNumber::default())).0,
            0x0012
        );
    }

    /// 확장 컨트롤 포함 문단 라운드트립
    #[test]
    fn test_roundtrip_with_section_def_control() {
        let sd = SectionDef {
            flags: 0,
            default_tab_spacing: 800,
            page_num: 1,
            ..Default::default()
        };

        let para = Paragraph {
            // char_count는 PARA_TEXT의 실제 UTF-16 code unit 총합이어야 한다:
            // A(1) + SectionDef 확장 컨트롤(8) + B(1) + 문단 끝 마커 0x000D(1)
            // = 11. (이 필드는 has_content가 true인 한 직렬화 시
            // serialize_para_text 결과 길이로 재계산되어 실제로는 무시되지만,
            // 값 자체가 4로 잘못되어 있으면 이 픽스처를 참고하는 다른 코드를
            // 오도할 수 있어 구조에 맞는 값으로 바로잡는다.)
            char_count: 11,
            text: "AB".to_string(),
            char_offsets: vec![0, 9], // 0~7 = secd 컨트롤, 8~8 gap? 아니, 0=A, 1~8=secd, 9=B
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            line_segs: vec![LineSeg {
                text_start: 0,
                ..Default::default()
            }],
            controls: vec![Control::SectionDef(Box::new(sd))],
            ..Default::default()
        };

        let section = Section {
            paragraphs: vec![para],
            raw_stream: None,
            ..Default::default()
        };

        let bytes = serialize_section(&section);
        let parsed = parse_body_text_section(&bytes).unwrap();

        assert_eq!(parsed.paragraphs[0].text, "AB");
        // SectionDef 컨트롤이 파싱되어 section_def에 반영
        assert_eq!(parsed.section_def.default_tab_spacing, 800);
    }

    /// 단 나누기 종류 라운드트립
    #[test]
    fn test_roundtrip_break_type() {
        let para = Paragraph {
            char_count: 2,
            text: "A".to_string(),
            char_offsets: vec![0],
            column_type: ColumnBreakType::Page,
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            line_segs: vec![LineSeg {
                text_start: 0,
                ..Default::default()
            }],
            ..Default::default()
        };

        let section = Section {
            paragraphs: vec![para],
            raw_stream: None,
            ..Default::default()
        };

        let bytes = serialize_section(&section);
        let parsed = parse_body_text_section(&bytes).unwrap();

        assert_eq!(parsed.paragraphs[0].column_type, ColumnBreakType::Page);
    }

    /// 묶음 빈칸(U+00A0, NO-BREAK SPACE) 라운드트립 -- 하이픈으로 오염되면 안 됨.
    /// HWP 5.0 표 7 기준 코드 24(0x0018)=하이픈, 코드 30(0x001E)=묶음 빈칸이며,
    /// 파서(parser/body_text.rs)는 이 매핑을 정확히 따른다.
    #[test]
    fn test_roundtrip_no_break_space() {
        let para = Paragraph {
            char_count: 4,
            text: "A\u{00A0}B".to_string(),
            char_offsets: vec![0, 1, 2],
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            line_segs: vec![LineSeg {
                text_start: 0,
                ..Default::default()
            }],
            ..Default::default()
        };

        let section = Section {
            paragraphs: vec![para],
            raw_stream: None,
            ..Default::default()
        };

        let bytes = serialize_section(&section);
        let parsed = parse_body_text_section(&bytes).unwrap();

        assert_eq!(
            parsed.paragraphs[0].text, "A\u{00A0}B",
            "no-break space must round-trip as itself, not decay into a hyphen"
        );
    }
