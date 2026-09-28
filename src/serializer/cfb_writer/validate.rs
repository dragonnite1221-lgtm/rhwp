use crate::model::control::Control;
use crate::model::paragraph::Paragraph;
use crate::model::shape::{DrawingObjAttr, ShapeObject};

use super::SerializeError;

pub(super) fn validate_paragraphs(paragraphs: &[Paragraph]) -> Result<(), SerializeError> {
    for para in paragraphs {
        for (name, count) in [
            ("char_shapes", para.char_shapes.len()),
            ("line_segs", para.line_segs.len()),
            ("range_tags", para.range_tags.len()),
        ] {
            if count > u16::MAX as usize {
                return Err(SerializeError::UnsupportedInput(format!(
                    "paragraph {name} count {count} exceeds HWP u16 limit {}",
                    u16::MAX
                )));
            }
        }
        for control in &para.controls {
            match control {
                Control::Table(table) => {
                    for cell in &table.cells {
                        validate_paragraphs(&cell.paragraphs)?;
                    }
                    if let Some(caption) = &table.caption {
                        validate_paragraphs(&caption.paragraphs)?;
                    }
                }
                Control::Shape(shape) => validate_shape(shape)?,
                Control::Picture(picture) => {
                    if let Some(caption) = &picture.caption {
                        validate_paragraphs(&caption.paragraphs)?;
                    }
                }
                Control::Header(header) => validate_paragraphs(&header.paragraphs)?,
                Control::Footer(footer) => validate_paragraphs(&footer.paragraphs)?,
                Control::Footnote(note) => validate_paragraphs(&note.paragraphs)?,
                Control::Endnote(note) => validate_paragraphs(&note.paragraphs)?,
                Control::HiddenComment(comment) => validate_paragraphs(&comment.paragraphs)?,
                _ => {}
            }
        }
    }
    Ok(())
}

fn validate_drawing(drawing: &DrawingObjAttr) -> Result<(), SerializeError> {
    if let Some(text_box) = &drawing.text_box {
        validate_paragraphs(&text_box.paragraphs)?;
    }
    if let Some(caption) = &drawing.caption {
        validate_paragraphs(&caption.paragraphs)?;
    }
    Ok(())
}

fn validate_shape(shape: &ShapeObject) -> Result<(), SerializeError> {
    match shape {
        ShapeObject::Line(s) => validate_drawing(&s.drawing),
        ShapeObject::Rectangle(s) => validate_drawing(&s.drawing),
        ShapeObject::Ellipse(s) => validate_drawing(&s.drawing),
        ShapeObject::Arc(s) => validate_drawing(&s.drawing),
        ShapeObject::Polygon(s) => validate_drawing(&s.drawing),
        ShapeObject::Curve(s) => validate_drawing(&s.drawing),
        ShapeObject::Group(group) => {
            for child in &group.children {
                validate_shape(child)?;
            }
            if let Some(caption) = &group.caption {
                validate_paragraphs(&caption.paragraphs)?;
            }
            Ok(())
        }
        ShapeObject::Picture(picture) => {
            if let Some(caption) = &picture.caption {
                validate_paragraphs(&caption.paragraphs)?;
            }
            Ok(())
        }
        ShapeObject::Chart(chart) => {
            validate_drawing(&chart.drawing)?;
            if let Some(caption) = &chart.caption {
                validate_paragraphs(&caption.paragraphs)?;
            }
            Ok(())
        }
        ShapeObject::Ole(ole) => {
            validate_drawing(&ole.drawing)?;
            if let Some(caption) = &ole.caption {
                validate_paragraphs(&caption.paragraphs)?;
            }
            Ok(())
        }
    }
}
