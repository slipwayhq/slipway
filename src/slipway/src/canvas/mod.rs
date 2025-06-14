mod errors;

use std::path::Path;

use base64::{Engine, prelude::BASE64_STANDARD};
use image::{DynamicImage, ImageBuffer, RgbaImage};
use slipway_engine::ComponentHandle;

pub use errors::CanvasError;

pub(super) fn get_canvas_image<'rig>(
    handle: &'rig ComponentHandle,
    output: &'rig serde_json::Value,
) -> Result<RgbaImage, CanvasError> {
    let canvas = read_canvas_data(handle, output)?;
    let rgba_bytes = BASE64_STANDARD
        .decode(canvas.data)
        .map_err(|e| CanvasError::InvalidData {
            handle: handle.clone(),
            message: format!("Data could not be decoded from base64\n{}", e),
        })?;

    let image: RgbaImage = ImageBuffer::from_raw(canvas.width, canvas.height, rgba_bytes)
        .ok_or_else(|| CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Data could not be converted into an image".to_string(),
        })?;

    let image = to_straight_alpha(image);

    Ok(image)
}

pub(super) fn render_canvas<'rig>(
    handle: &'rig ComponentHandle,
    output: &'rig serde_json::Value,
    save_path: Option<&Path>,
) -> Result<(), CanvasError> {
    let image = get_canvas_image(handle, output)?;

    if let Some(save_path) = save_path {
        save_image(handle, image, save_path)
    } else {
        print_image(handle, image)
    }
}

pub(super) fn render_canvas_if_exists<'rig>(
    handle: &'rig ComponentHandle,
    output: &'rig serde_json::Value,
    save_path: Option<&Path>,
) -> Result<bool, CanvasError> {
    let image = get_canvas_image(handle, output);

    let Ok(image) = image else {
        return Ok(false);
    };

    if let Some(save_path) = save_path {
        save_image(handle, image, save_path)?;
    } else {
        print_image(handle, image)?;
    }

    Ok(true)
}

fn save_image(
    handle: &ComponentHandle,
    image: RgbaImage,
    save_path: &Path,
) -> Result<(), CanvasError> {
    image
        .save(save_path)
        .map_err(|error| CanvasError::SaveFailed {
            handle: handle.clone(),
            path: save_path.to_path_buf(),
            error,
        })?;

    Ok(())
}

fn print_image(handle: &ComponentHandle, image: RgbaImage) -> Result<(), CanvasError> {
    let conf = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };

    viuer::print(&DynamicImage::ImageRgba8(image), &conf).map_err(|e| {
        CanvasError::PrintFailed {
            handle: handle.clone(),
            error: e.to_string(),
        }
    })?;

    Ok(())
}

fn read_canvas_data(
    handle: &ComponentHandle,
    output: &serde_json::Value,
) -> Result<CanvasResult, CanvasError> {
    // If the output has a `canvas` field, try and read the `width`, `height`, and `data` properties.
    let Some(canvas) = output.get("canvas") else {
        return Err(CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Output has no canvas field".to_string(),
        });
    };

    let canvas = canvas.as_object().ok_or_else(|| CanvasError::InvalidData {
        handle: handle.clone(),
        message: "Canvas field is not an object".to_string(),
    })?;

    let width = canvas
        .get("width")
        .ok_or_else(|| CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Canvas is missing a width field".to_string(),
        })?
        .as_u64()
        .ok_or_else(|| CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Canvas width field is not a number".to_string(),
        })?;

    let height = canvas
        .get("height")
        .ok_or_else(|| CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Canvas is missing a height field".to_string(),
        })?
        .as_u64()
        .ok_or_else(|| CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Canvas height field is not a number".to_string(),
        })?;

    let data = canvas
        .get("data")
        .ok_or_else(|| CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Canvas is missing a data field".to_string(),
        })?
        .as_str()
        .ok_or_else(|| CanvasError::InvalidData {
            handle: handle.clone(),
            message: "Canvas data field is not a string".to_string(),
        })?;

    Ok(CanvasResult {
        width: width as u32,
        height: height as u32,
        data: data.to_string(),
    })
}

pub(crate) fn to_straight_alpha(mut img: RgbaImage) -> RgbaImage {
    for px in img.pixels_mut() {
        let [r, g, b, a] = px.0;
        if a > 0 {
            let af = a as f32 / 255.0;
            let inv_af = 1.0 / af;
            px.0 = [
                (r as f32 * inv_af).min(255.0).round() as u8,
                (g as f32 * inv_af).min(255.0).round() as u8,
                (b as f32 * inv_af).min(255.0).round() as u8,
                a,
            ];
        } else {
            px.0 = [0, 0, 0, 0]; // avoid NaNs or garbage
        }
    }
    img
}

struct CanvasResult {
    width: u32,
    height: u32,
    data: String,
}
