use std::io::Cursor;

use image::{GrayImage, Luma, RgbaImage};

pub(super) fn encode_1bit_bmp(rgba_image: RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    // Convert to grayscale
    let gray_image: GrayImage = image::DynamicImage::ImageRgba8(rgba_image).into_luma8();

    // Dither to 1-bit
    let binary_image = dither_image(&gray_image, DitherAlgorithm::Atkinson);

    // Create a new 1-bit BMP image
    let mut buffer = Cursor::new(Vec::new());
    let mut bmp = image::codecs::bmp::BmpEncoder::new(&mut buffer);
    bmp.encode(
        &binary_image,
        binary_image.width(),
        binary_image.height(),
        image::ExtendedColorType::L1,
    )?;

    Ok(buffer.into_inner())
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
}
