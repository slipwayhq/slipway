use std::path::PathBuf;

use base64::{prelude::BASE64_STANDARD, Engine};
use image::{DynamicImage, ImageBuffer, RgbaImage};
use slipway_lib::{ComponentHandle, RigExecutionState};

use super::errors::SlipwayDebugError;

pub(super) fn handle_render_command<'rig>(
    handle: &'rig ComponentHandle,
    state: &RigExecutionState<'rig>,
    save_path: Option<PathBuf>,
) -> Result<(), SlipwayDebugError> {
    let component_state = state
        .component_states
        .get(&handle)
        .expect("Component should exist");

    let output = component_state.output().ok_or_else(|| {
        SlipwayDebugError::UserError(format!("Component {} has no output", handle))
    })?;

    let canvas = read_canvas_data(handle, output)?;
    let rgba_bytes = BASE64_STANDARD.decode(canvas.data).map_err(|e| {
        SlipwayDebugError::ComponentError(format!(
            "Component {} canvas output data could not be decoded from base64\n{}",
            handle, e
        ))
    })?;

    let image: RgbaImage = ImageBuffer::from_raw(canvas.width, canvas.height, rgba_bytes)
        .ok_or_else(|| {
            SlipwayDebugError::ComponentError(format!(
                "Component {} canvas output data could not be converted into an image",
                handle
            ))
        })?;

    if let Some(save_path) = save_path {
        save_image(handle, image, save_path)?;
    } else {
        print_image(handle, image)?;
    }

    Ok(())
}

fn save_image(
    handle: &ComponentHandle,
    image: RgbaImage,
    save_path: PathBuf,
) -> Result<(), SlipwayDebugError> {
    image.save(save_path.clone()).map_err(|e| {
        SlipwayDebugError::ComponentError(format!(
            "Component {} canvas output image could not be saved to {}\n{}",
            handle,
            save_path.display(),
            e
        ))
    })?;

    Ok(())
}

fn print_image(handle: &ComponentHandle, image: RgbaImage) -> Result<(), SlipwayDebugError> {
    let conf = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };

    viuer::print(&DynamicImage::ImageRgba8(image), &conf).map_err(|e| {
        SlipwayDebugError::ComponentError(format!(
            "Component {} canvas output image could not be printed\n{}",
            handle, e
        ))
    })?;

    Ok(())
}

fn read_canvas_data(
    handle: &ComponentHandle,
    output: &serde_json::Value,
) -> Result<CanvasResult, SlipwayDebugError> {
    // If the output has a `canvas` property, try and read the `width`, `height`, and `data` properties.
    let Some(canvas) = output.get("canvas") else {
        return Err(SlipwayDebugError::UserError(format!(
            "Component {} output has no canvas property",
            handle
        )));
    };

    let canvas = canvas.as_object().ok_or_else(|| {
        SlipwayDebugError::UserError(format!(
            "Component {} has a canvas property, but it is not an object",
            handle
        ))
    })?;

    let width = canvas
        .get("width")
        .ok_or_else(|| {
            SlipwayDebugError::UserError(format!(
                "Component {} output has a canvas property, but it is missing a width property",
                handle
            ))
        })?
        .as_u64()
        .ok_or_else(|| {
            SlipwayDebugError::UserError(format!(
                "Component {} output has a canvas property, but the width property is not a number",
                handle
            ))
        })?;

    let height = canvas
            .get("height")
            .ok_or_else(|| {
                SlipwayDebugError::UserError(format!(
                    "Component {} output has a canvas property, but it is missing a height property",
                    handle
                ))
            })?
            .as_u64()
            .ok_or_else(|| {
                SlipwayDebugError::UserError(format!(
                    "Component {} output has a canvas property, but the height property is not a number",
                    handle
                ))
            })?;

    let data = canvas
        .get("data")
        .ok_or_else(|| {
            SlipwayDebugError::UserError(format!(
                "Component {} output has a canvas property, but it is missing a data property",
                handle
            ))
        })?
        .as_str()
        .ok_or_else(|| {
            SlipwayDebugError::UserError(format!(
                "Component {} output has a canvas property, but the data property is not a string",
                handle
            ))
        })?;

    Ok(CanvasResult {
        width: width as u32,
        height: height as u32,
        data: data.to_string(),
    })
}

struct CanvasResult {
    width: u32,
    height: u32,
    data: String,
}
