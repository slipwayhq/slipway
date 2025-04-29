use image::{GrayImage, Luma, RgbaImage};

pub(super) fn encode_1bit_bmp(rgba_image: RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    // Convert to grayscale
    let gray_image: GrayImage = image::DynamicImage::ImageRgba8(rgba_image).into_luma8();

    // Dither to 1-bit
    let dithered = dither_image(&gray_image, DitherAlgorithm::Atkinson);

    // // Pack the dithered data into a 1-bit buffer
    // let packed = pack_to_1bit_buffer(&dithered);

    // // Create a new 1-bit BMP image
    // let mut buffer = Cursor::new(Vec::new());
    // let mut bmp = image::codecs::bmp::BmpEncoder::new(&mut buffer);
    // bmp.encode(
    //     &packed,
    //     dithered.width(),
    //     dithered.height(),
    //     image::ExtendedColorType::L1,
    // )?;

    // Ok(buffer.into_inner())

    let buffer = encode_1bit_bmp_custom(dithered);
    Ok(buffer)
}

fn encode_1bit_bmp_custom(dithered: GrayImage) -> Vec<u8> {
    // Pack each row as bits. We’ll still write from top-to-bottom here,
    // but we’ll specify a negative height in the BMP header to flip it.
    let width = dithered.width();
    let height = dithered.height();
    let row_bytes = width.div_ceil(32) * 4; // each row is 4-byte aligned
    let mut pixel_data = vec![0u8; (row_bytes * height) as usize];

    for y in 0..height {
        for x in 0..width {
            let bit = if dithered.get_pixel(x, height - y - 1).0[0] < 128 {
                0
            } else {
                1
            };
            let byte_index = (y * row_bytes) + (x / 8);
            let bit_index = 7 - (x % 8);
            pixel_data[byte_index as usize] |= bit << bit_index;
        }
    }

    // Construct a minimal BMP header
    let file_header_size = 14;
    let info_header_size = 40;
    let palette_size = 2 * 4;
    let data_offset = file_header_size + info_header_size + palette_size;
    let file_size = data_offset as u32 + pixel_data.len() as u32;

    let mut out = Vec::with_capacity(file_size as usize);

    // BMP file header (14 bytes)
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // Reserved1
    out.extend_from_slice(&0u16.to_le_bytes()); // Reserved2
    out.extend_from_slice(&(data_offset as u32).to_le_bytes());

    // DIB header (40 bytes)
    // We use a negative height to indicate a top-down BMP
    out.extend_from_slice(&(info_header_size as u32).to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // Planes = 1
    out.extend_from_slice(&1u16.to_le_bytes()); // Bits per pixel = 1
    out.extend_from_slice(&0u32.to_le_bytes()); // Compression = 0
    out.extend_from_slice(&(pixel_data.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // X pixels per meter
    out.extend_from_slice(&0u32.to_le_bytes()); // Y pixels per meter
    out.extend_from_slice(&2u32.to_le_bytes()); // colors in palette
    out.extend_from_slice(&0u32.to_le_bytes()); // important colors

    // Palette: 2 entries (black and white), 4 bytes each: B, G, R, 0
    out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // black
    out.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x00]); // white

    // Actual pixel data
    out.extend_from_slice(&pixel_data);

    out
}

#[derive(Debug, Clone, Copy)]
pub enum DitherAlgorithm {
    FloydSteinberg,
    Atkinson,
}

/// Apply dithering to a `GrayImage` using the specified algorithm.
fn dither_image(input: &GrayImage, algorithm: DitherAlgorithm) -> GrayImage {
    match algorithm {
        DitherAlgorithm::FloydSteinberg => floyd_steinberg_dither(input),
        DitherAlgorithm::Atkinson => atkinson_dither(input),
    }
}

/// Dither using Floyd-Steinberg.
fn floyd_steinberg_dither(input: &GrayImage) -> GrayImage {
    let mut output = input.clone();
    let (width, height) = output.dimensions();

    for y in 0..height {
        for x in 0..width {
            let old_pixel = output.get_pixel(x, y).0[0] as f32;
            let new_pixel = if old_pixel < 128.0 { 0.0 } else { 255.0 };
            let error = old_pixel - new_pixel;

            output.put_pixel(x, y, Luma([new_pixel as u8]));

            // Distribute the error to neighboring pixels (Floyd-Steinberg pattern):
            //   p[x+1, y  ] += 7/16 of error
            //   p[x-1, y+1] += 3/16
            //   p[x  , y+1] += 5/16
            //   p[x+1, y+1] += 1/16
            if x + 1 < width {
                propagate_error(&mut output, x + 1, y, error * 7.0 / 16.0);
            }
            if x > 0 && y + 1 < height {
                propagate_error(&mut output, x - 1, y + 1, error * 3.0 / 16.0);
            }
            if y + 1 < height {
                propagate_error(&mut output, x, y + 1, error * 5.0 / 16.0);
            }
            if x + 1 < width && y + 1 < height {
                propagate_error(&mut output, x + 1, y + 1, error * 1.0 / 16.0);
            }
        }
    }
    output
}

/// Dither using Atkinson.
fn atkinson_dither(input: &GrayImage) -> GrayImage {
    let mut output = input.clone();
    let (width, height) = output.dimensions();

    for y in 0..height {
        for x in 0..width {
            let old_pixel = output.get_pixel(x, y).0[0] as f32;
            let new_pixel = if old_pixel < 128.0 { 0.0 } else { 255.0 };
            let error = old_pixel - new_pixel;

            output.put_pixel(x, y, Luma([new_pixel as u8]));

            // Distribute the error (Atkinson pattern):
            //   p[x+1, y  ] += error/8
            //   p[x+2, y  ] += error/8
            //   p[x-1, y+1] += error/8
            //   p[x  , y+1] += error/8
            //   p[x+1, y+1] += error/8
            //   p[x  , y+2] += error/8
            if x + 1 < width {
                propagate_error(&mut output, x + 1, y, error / 8.0);
            }
            if x + 2 < width {
                propagate_error(&mut output, x + 2, y, error / 8.0);
            }
            if y + 1 < height {
                if x > 0 {
                    propagate_error(&mut output, x - 1, y + 1, error / 8.0);
                }
                propagate_error(&mut output, x, y + 1, error / 8.0);
                if x + 1 < width {
                    propagate_error(&mut output, x + 1, y + 1, error / 8.0);
                }
            }
            if y + 2 < height {
                propagate_error(&mut output, x, y + 2, error / 8.0);
            }
        }
    }
    output
}

/// Helper: safely add `amount` to a pixel's intensity.
fn propagate_error(img: &mut GrayImage, x: u32, y: u32, amount: f32) {
    let p = img.get_pixel(x, y).0[0] as f32;
    let new_val = (p + amount).clamp(0.0, 255.0);
    img.put_pixel(x, y, Luma([new_val as u8]));
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    // Helper function to check if all pixels in a GrayImage are 0 or 255.
    fn is_binary_image(img: &GrayImage) -> bool {
        for pixel in img.pixels() {
            let val = pixel.0[0];
            if val != 0 && val != 255 {
                return false;
            }
        }
        true
    }

    /// Make a simple 3x3 grayscale image with some pattern
    fn create_sample_image() -> image::ImageBuffer<Luma<u8>, Vec<u8>> {
        let mut input = GrayImage::new(3, 3);

        input.put_pixel(0, 0, Luma([50]));
        input.put_pixel(1, 0, Luma([100]));
        input.put_pixel(2, 0, Luma([150]));

        input.put_pixel(0, 1, Luma([200]));
        input.put_pixel(1, 1, Luma([128]));
        input.put_pixel(2, 1, Luma([64]));

        input.put_pixel(0, 2, Luma([0]));
        input.put_pixel(1, 2, Luma([255]));
        input.put_pixel(2, 2, Luma([128]));
        input
    }

    #[test]
    fn test_floyd_steinberg_dither_small_image() {
        let input = create_sample_image();

        let dithered = dither_image(&input, DitherAlgorithm::FloydSteinberg);

        // Verify we don't crash, and the output is strictly binary
        assert!(is_binary_image(&dithered));
    }

    #[test]
    fn test_atkinson_dither_small_image() {
        let input = create_sample_image();

        let dithered = dither_image(&input, DitherAlgorithm::Atkinson);

        // Verify we don't crash, and the output is strictly binary
        assert!(is_binary_image(&dithered));
    }

    #[test]
    fn encode_1bit_bmp_small_image() {
        // Create a 100x100 RGBA image which has linear gradient from black to white.
        let image = RgbaImage::from_fn(100, 100, |x, y| {
            let intensity = (x + y) as u8;
            image::Rgba([intensity, intensity, intensity, 255])
        });

        let bmp = encode_1bit_bmp(image).unwrap();

        // Verify we don't crash, and the output is a valid BMP
        let _ = image::load_from_memory(&bmp).unwrap();
    }
}
