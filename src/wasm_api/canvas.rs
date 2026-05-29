#[cfg(any(target_arch = "wasm32", test))]
const MAX_CANVAS_DIMENSION: f64 = 16_384.0;

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn normalize_canvas_scale(
    page_width: f64,
    page_height: f64,
    requested_scale: f64,
) -> Result<f64, &'static str> {
    if !page_width.is_finite()
        || !page_height.is_finite()
        || page_width <= 0.0
        || page_height <= 0.0
    {
        return Err("invalid page dimensions");
    }

    let scale = if requested_scale <= 0.0 || !requested_scale.is_finite() {
        1.0
    } else {
        requested_scale.clamp(0.25, 12.0)
    };

    let scaled_width = page_width * scale;
    let scaled_height = page_height * scale;
    if !scaled_width.is_finite() || !scaled_height.is_finite() {
        return Ok((MAX_CANVAS_DIMENSION / page_width)
            .min(MAX_CANVAS_DIMENSION / page_height)
            .min(scale));
    }

    if scaled_width > MAX_CANVAS_DIMENSION || scaled_height > MAX_CANVAS_DIMENSION {
        Ok((MAX_CANVAS_DIMENSION / page_width)
            .min(MAX_CANVAS_DIMENSION / page_height)
            .min(scale))
    } else {
        Ok(scale)
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn scaled_canvas_extent(page_extent: f64, scale: f64) -> u32 {
    (page_extent * scale).max(1.0).min(MAX_CANVAS_DIMENSION) as u32
}
