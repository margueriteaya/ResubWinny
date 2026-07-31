use super::*;

pub(super) fn fill_rect(
    canvas: &mut [u8],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: [u8; 4],
) {
    for row in y.max(0)..y.saturating_add(height).min(TTML_PLANE_HEIGHT as i32) {
        for column in x.max(0)..x.saturating_add(width).min(TTML_PLANE_WIDTH as i32) {
            blend_pixel(canvas, column, row, color, 255);
        }
    }
}
pub(super) fn blend_glyph(
    canvas: &mut [u8],
    x: i32,
    y: i32,
    width: usize,
    height: usize,
    bitmap: &[u8],
    color: [u8; 4],
) {
    for row in 0..height {
        for column in 0..width {
            if let Some(alpha) = bitmap.get(row * width + column) {
                blend_pixel(canvas, x + column as i32, y + row as i32, color, *alpha);
            }
        }
    }
}
fn blend_pixel(canvas: &mut [u8], x: i32, y: i32, color: [u8; 4], coverage: u8) {
    if x < 0 || y < 0 || x >= TTML_PLANE_WIDTH as i32 || y >= TTML_PLANE_HEIGHT as i32 {
        return;
    }
    let index = ((y as u32 * TTML_PLANE_WIDTH + x as u32) * 4) as usize;
    let alpha = u32::from(color[3]) * u32::from(coverage) / 255;
    let destination_alpha = u32::from(canvas[index + 3]);
    let output_alpha = alpha + destination_alpha * (255 - alpha) / 255;
    if output_alpha == 0 {
        return;
    }
    for channel in 0..3 {
        canvas[index + channel] = ((u32::from(color[channel]) * alpha
            + u32::from(canvas[index + channel]) * destination_alpha * (255 - alpha) / 255)
            / output_alpha)
            .min(255) as u8;
    }
    canvas[index + 3] = output_alpha.min(255) as u8;
}

pub(super) fn decode_png(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().ok()?;
    let output_size = reader.output_buffer_size();
    if output_size == 0 || output_size > MAX_IMAGE_BYTES {
        return None;
    }
    let mut buffer = vec![0_u8; output_size];
    let info = reader.next_frame(&mut buffer).ok()?;
    let pixels = &buffer[..info.buffer_size()];
    let mut rgba = Vec::with_capacity(info.width as usize * info.height as usize * 4);
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(pixels),
        png::ColorType::Rgb => {
            for pixel in pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for value in pixels {
                rgba.extend_from_slice(&[*value, *value, *value, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for pixel in pixels.chunks_exact(2) {
                rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
        }
        png::ColorType::Indexed => return None,
    }
    Some((rgba, info.width, info.height))
}

#[allow(
    clippy::too_many_arguments,
    reason = "the pixel compositor keeps source and destination geometry explicit"
)]
pub(super) fn blend_layer(
    canvas: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    let max_width = canvas_width.saturating_sub(x);
    let max_height = canvas_height.saturating_sub(y);
    for row in 0..height.min(max_height) {
        for column in 0..width.min(max_width) {
            let source_index = ((row * width + column) * 4) as usize;
            let destination_index = (((y + row) * canvas_width + x + column) * 4) as usize;
            let Some(source) = pixels.get(source_index..source_index + 4) else {
                continue;
            };
            let Some(destination) = canvas.get_mut(destination_index..destination_index + 4) else {
                continue;
            };
            let source_alpha = u32::from(source[3]);
            if source_alpha == 255 {
                destination.copy_from_slice(source);
                continue;
            }
            if source_alpha == 0 {
                continue;
            }
            let destination_alpha = u32::from(destination[3]);
            let output_alpha =
                source_alpha + (destination_alpha * (255 - source_alpha) + 127) / 255;
            for channel in 0..3 {
                let source_value = u32::from(source[channel]);
                let destination_value = u32::from(destination[channel]);
                let value = (source_value * source_alpha * 255
                    + destination_value * destination_alpha * (255 - source_alpha)
                    + output_alpha * 127)
                    / (output_alpha.max(1) * 255);
                destination[channel] = value.min(255) as u8;
            }
            destination[3] = output_alpha.min(255) as u8;
        }
    }
}

pub(super) fn encode_png(width: u32, height: u32, pixels: &[u8]) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(pixels).ok()?;
    drop(writer);
    Some(bytes)
}
