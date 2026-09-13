//! body_text 직렬화 라운드트립 테스트 (1/3): 기본 텍스트/탭/줄바꿈/빈 문단.
//!
//! `serializer/body_text.rs` 소스 파일이 커지는 것을 막기 위해 인라인
//! `mod tests { ... }` 블록을 세 파일로 나눠 분리했다 (내용은 변경 없음).


    use super::*;
    use crate::model::control::AutoNumber;
    use crate::model::document::{Section, SectionDef};
    use crate::model::paragraph::{CharShapeRef, LineSeg, Paragraph, RangeTag};
    use crate::parser::body_text::parse_body_text_section;

    /// 간단한 텍스트 문단 라운드트립
    #[test]
    fn test_roundtrip_simple_text() {
        let para = Paragraph {
            char_count: 6,
            text: "Hello".to_string(),
            char_offsets: vec![0, 1, 2, 3, 4],
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            line_segs: vec![LineSeg {
                text_start: 0,
                line_height: 400,
                text_height: 400,
                baseline_distance: 320,
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

        assert_eq!(parsed.paragraphs.len(), 1);
        assert_eq!(parsed.paragraphs[0].text, "Hello");
        assert_eq!(parsed.paragraphs[0].char_offsets, vec![0, 1, 2, 3, 4]);
    }

    /// 한글 텍스트 라운드트립
    #[test]
    fn test_roundtrip_korean_text() {
        let para = Paragraph {
            char_count: 10,
            text: "한글 테스트입니다.".to_string(),
            char_offsets: vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 1,
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

        assert_eq!(parsed.paragraphs[0].text, "한글 테스트입니다.");
    }

    /// 탭 문자 포함 라운드트립
    #[test]
    fn test_roundtrip_with_tab() {
        let para = Paragraph {
            char_count: 4,
            text: "A\tB".to_string(),
            char_offsets: vec![0, 1, 9],
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

        assert_eq!(parsed.paragraphs[0].text, "A\tB");
        assert_eq!(parsed.paragraphs[0].char_offsets, vec![0, 1, 9]);
    }

    /// 줄바꿈 포함 라운드트립
    #[test]
    fn test_roundtrip_with_linebreak() {
        let para = Paragraph {
            char_count: 4,
            text: "A\nB".to_string(),
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

        assert_eq!(parsed.paragraphs[0].text, "A\nB");
    }

    /// 빈 문단 직렬화
    #[test]
    fn test_serialize_empty_paragraph() {
        let para = Paragraph {
            char_count: 0,
            ..Default::default()
        };

        let section = Section {
            paragraphs: vec![para],
            raw_stream: None,
            ..Default::default()
        };

        let bytes = serialize_section(&section);
        let parsed = parse_body_text_section(&bytes).unwrap();

        assert_eq!(parsed.paragraphs.len(), 1);
        assert!(parsed.paragraphs[0].text.is_empty());
    }

