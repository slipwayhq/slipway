use std::io::Cursor;

use dither::prelude::Dither;
use image::{GrayImage, RgbaImage};

pub(super) fn encode_1bit_bmp(rgba_image: RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    // Convert to grayscale
    let gray_image: GrayImage = image::DynamicImage::ImageRgba8(rgba_image).into_luma8();

    // Dither to 1-bit
    let ditherer = &dither::ditherer::ATKINSON;
    let quantize = dither::create_quantize_n_bits_func(1).expect("1 bit dithering should be valid");
    let binary_image = ditherer.dither(gray_image, quantize);

    // Create a new 1-bit BMP image
    let mut buffer = Cursor::new(Vec::new());
    let mut bmp = image::codecs::bmp::BmpEncoder::new(&mut buffer);
    bmp.encode(
        binary_image.into_vec(),
        binary_image.width(),
        binary_image.height(),
        image::ExtendedColorType::L1,
    )?;

    Ok(buffer.into_inner())
}
