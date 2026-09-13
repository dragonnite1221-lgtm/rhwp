//! `validate_linesegs` 회귀 테스트 (2/2): needs_reflow_broadly + textrun reflow 패턴.
//!
//! `document.rs` 소스 파일이 커지는 것을 막기 위해 인라인 `mod
//! validate_linesegs_tests { ... }` 블록을 두 파일로 나눠 분리했다 (내용은
//! 변경 없음, 이 PR의 수정 대상도 아니다).

use super::*;
use crate::model::document::{Document, Section};
use crate::model::paragraph::{LineSeg, Paragraph};

    #[test]
    fn needs_reflow_broadly_covers_empty_linesegs() {
        let mut para = Paragraph::default();
        para.text = "hello".to_string();
        // line_segs 비움
        assert!(DocumentCore::needs_reflow_broadly(&para));
    }

    /// needs_reflow_broadly: 기존 조건 (line_segs=1, line_height=0) → true
    #[test]
    fn needs_reflow_broadly_covers_uncomputed_lineseg() {
        let mut para = Paragraph::default();
        para.text = "hello".to_string();
        para.line_segs.push(LineSeg::default());
        assert!(DocumentCore::needs_reflow_broadly(&para));
    }

    /// needs_reflow_broadly: 정상 line_segs → false
    #[test]
    fn needs_reflow_broadly_skips_healthy_paragraph() {
        let mut para = Paragraph::default();
        para.text = "hello".to_string();
        let mut seg = LineSeg::default();
        seg.line_height = 1000;
        para.line_segs.push(seg);
        assert!(!DocumentCore::needs_reflow_broadly(&para));
    }

    /// needs_reflow_broadly: 빈 문단 (text 없음) → false
    #[test]
    fn needs_reflow_broadly_skips_empty_paragraph() {
        let para = Paragraph::default();
        assert!(!DocumentCore::needs_reflow_broadly(&para));
    }

    // ---------- R3: LinesegTextRunReflow ----------

    #[test]
    fn validate_detects_textrun_reflow_pattern() {
        // 긴 텍스트(40자 초과) + lineseg 1개 + '\n' 없음 → R3 경고
        let mut doc = Document::default();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        para.text = "이것은 충분히 길어서 한 줄로 표시하기 어려운 한국어 문장입니다. 한컴은 textRun으로 reflow하지만 rhwp는 그대로 그립니다.".to_string();
        let mut seg = LineSeg::default();
        seg.line_height = 1000; // line_height 는 0 아님 → R2 는 해당 안 됨
        para.line_segs.push(seg);
        section.paragraphs.push(para);
        doc.sections.push(section);

        let report = DocumentCore::validate_linesegs(&doc, true);
        assert_eq!(report.len(), 1);
        assert_eq!(report.warnings[0].kind, WarningKind::LinesegTextRunReflow);
    }

    #[test]
    fn validate_skips_textrun_reflow_for_short_text() {
        // 짧은 텍스트(40자 이하) → R3 해당 안 됨
        let mut doc = Document::default();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        para.text = "짧은 문장입니다.".to_string();
        let mut seg = LineSeg::default();
        seg.line_height = 1000;
        para.line_segs.push(seg);
        section.paragraphs.push(para);
        doc.sections.push(section);

        let report = DocumentCore::validate_linesegs(&doc, true);
        assert!(report.is_empty(), "짧은 문장은 경고 대상이 아님");
    }

    #[test]
    fn validate_skips_textrun_reflow_when_has_newline() {
        // 긴 텍스트라도 '\n' 이 있으면 이미 분할된 것으로 간주 → R3 해당 안 됨
        let mut doc = Document::default();
        let mut section = Section::default();
        let mut para = Paragraph::default();
        para.text = "충분히 긴 텍스트이지만 줄바꿈이 있습니다.\n그래서 R3은 해당하지 않아야 합니다.".to_string();
        let mut seg = LineSeg::default();
        seg.line_height = 1000;
        para.line_segs.push(seg);
        section.paragraphs.push(para);
        doc.sections.push(section);

        let report = DocumentCore::validate_linesegs(&doc, true);
        assert!(report.is_empty(), "\\n 있는 문단은 R3 해당 안 됨");
    }

    #[test]
    fn needs_reflow_broadly_covers_textrun_reflow() {
        let mut para = Paragraph::default();
        para.text = "이것은 충분히 길어서 한 줄로 표시하기 어려운 한국어 문장입니다. 한컴은 textRun으로 reflow하지만 rhwp는 그대로 그립니다.".to_string();
        let mut seg = LineSeg::default();
        seg.line_height = 1000;
        para.line_segs.push(seg);
        assert!(DocumentCore::needs_reflow_broadly(&para));
    }
