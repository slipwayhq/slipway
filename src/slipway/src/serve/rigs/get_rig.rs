use std::borrow::Cow;
use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{HttpMessage, HttpRequest, get, web};
use anyhow::Context;
use image::imageops::{rotate90, rotate180, rotate270};
use serde::Deserialize;
use tracing::{Instrument, info_span};

use crate::primitives::{DeviceName, RigName};
use crate::serve::auth::compute_signature_parts;
use crate::serve::repository::{RigResultFormat, RigResultSpec};
use crate::serve::{
    API_GET_DEVICE_PATH, Device, RequestState, TRMNL_DISPLAY_PATH, truncate_hashed_api_key,
    try_get_api_key_from_state,
};

use crate::serve::responses::{FormatQuery, ImageResponse, RigResponse, ServeError, UrlResponse};

use super::super::ServeState;

#[derive(Deserialize)]
struct GetRigPath {
    rig_name: RigName,
}

#[derive(Deserialize)]
struct GetRigQuery {
    #[serde(flatten)]
    output: FormatQuery,

    device: Option<DeviceName>,
}

#[get("/rigs/{rig_name}")]
pub async fn get_rig(
    path: web::Path<GetRigPath>,
    query: web::Query<GetRigQuery>,
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<RigResponse, ServeError> {
    let path = path.into_inner();
    let query = query.into_inner();

    let state = data.into_inner();
    let rig_name = path.rig_name;
    let result_spec = query.output.into_spec();

    let device = match query.device {
        Some(device_name) => {
            let device = state.repository.get_device(&device_name).await?;
            Some(RequestingDevice::from(device_name, device))
        }
        None => None,
    };

    get_rig_response(&rig_name, device, result_spec, state, req)
        .instrument(info_span!("rig", ""=%rig_name))
        .await
}

pub struct RequestingDevice {
    pub name: DeviceName,
    pub context: Option<serde_json::Value>,
}

impl RequestingDevice {
    pub fn from(name: DeviceName, device: Device) -> Self {
        Self {
            name,
            context: device.context,
        }
    }
}

pub fn assert_api_key_is_valid_for_rig(
    device: &Option<RequestingDevice>,
    req: &HttpRequest,
) -> Result<(), ServeError> {
    // If there is no API key supplied we must be using a shared access signature to have got this far.
    let maybe_supplied_api_key = try_get_api_key_from_state(req);
    if let Some(supplied_api_key) = maybe_supplied_api_key {
        if let Some(resolved) = supplied_api_key.resolved {
            // If the API key is not associated with a device, we allow it to access any rig.
            if let Some(associated_device_name) = resolved.device {
                if let Some(current_device) = device {
                    if associated_device_name != current_device.name {
                        return Err(ServeError::UserFacing(
                            StatusCode::FORBIDDEN,
                            format!(
                                "The hashed API key {} can only be used with the device \"{associated_device_name}\" but was used with \"{}\".",
                                truncate_hashed_api_key(&resolved.hashed_key),
                                current_device.name
                            ),
                        ));
                    }
                } else {
                    return Err(ServeError::UserFacing(
                        StatusCode::FORBIDDEN,
                        format!(
                            "The hashed API key {} can only be used with the device \"{associated_device_name}\".",
                            truncate_hashed_api_key(&resolved.hashed_key)
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub async fn get_rig_response(
    rig_name: &RigName,
    device: Option<RequestingDevice>,
    result_spec: RigResultSpec,
    state: Arc<ServeState>,
    req: HttpRequest,
) -> Result<RigResponse, ServeError> {
    assert_api_key_is_valid_for_rig(&device, &req)?;

    let rig = state.repository.get_rig(rig_name).await?;

    let format = result_spec.format;
    let image_format = result_spec.image_format;
    let rotate = result_spec.rotate;

    match format {
        RigResultFormat::Image | RigResultFormat::DataUrl | RigResultFormat::Json => {
            let result =
                super::run_rig::run_rig(state, rig, rig_name, device.and_then(|d| d.context))
                    .await
                    .map_err(ServeError::Internal)?;

            if matches!(format, RigResultFormat::Json) {
                Ok(RigResponse::Json(web::Json(result.output)))
            } else {
                let maybe_image = crate::canvas::get_canvas_image(&result.handle, &result.output);

                if let Ok(image) = maybe_image {
                    let image = match rotate {
                        0 => image,
                        90 => rotate90(&image),
                        180 => rotate180(&image),
                        270 => rotate270(&image),
                        _ => {
                            return Err(ServeError::UserFacing(
                                StatusCode::BAD_REQUEST,
                                format!("Invalid rotation angle specified: {}", rotate),
                            ));
                        }
                    };
                    Ok(RigResponse::Image(ImageResponse {
                        image,
                        format: image_format,
                        wrap_in_html: matches!(format, RigResultFormat::DataUrl),
                    }))
                } else {
                    Err(ServeError::UserFacing(
                        StatusCode::BAD_REQUEST,
                        "Could not render rig output as an image.".to_string(),
                    ))
                }
            }
        }
        RigResultFormat::Url => {
            let connection_info = req.connection_info();
            let scheme = connection_info.scheme();
            let host = connection_info.host();
            let uri = req.uri();
            let path = {
                let path = uri.path();
                if path.ends_with(TRMNL_DISPLAY_PATH) {
                    let path_without_trmnl = &path[0..path.len() - TRMNL_DISPLAY_PATH.len()];
                    let device_name = device
                        .as_ref()
                        .map(|d| &d.name)
                        .expect("Trmnl display calls should be associated with a device");
                    Cow::Owned(format!(
                        "{path_without_trmnl}{API_GET_DEVICE_PATH}/{device_name}"
                    ))
                } else {
                    Cow::Borrowed(path)
                }
            };

            let full_url_without_qs = format!("{}://{}{}", scheme, host, path);

            let mut qs = url::form_urlencoded::Serializer::new(String::new());

            let new_format = RigResultFormat::Image;
            qs.append_pair(
                "format",
                serde_json::to_value(&new_format)
                    .expect("Format should serialize")
                    .as_str()
                    .expect("Format should be a string"),
            );

            qs.append_pair(
                "image_format",
                serde_json::to_value(&image_format)
                    .expect("Image format should serialize")
                    .as_str()
                    .expect("Image format should be a string"),
            );

            qs.append_pair("rotate", &rotate.to_string());

            if let Some(device) = device {
                qs.append_pair("device", &device.name.0);
            }

            let maybe_secret = state.secret.as_deref();
            if let Some(secret) = maybe_secret {
                // If we have a SLIPWAY_SECRET, we generate a SAS token.
                let sas_token_parts =
                    compute_signature_parts(secret, chrono::Duration::seconds(60));

                for (sas_key, sas_value) in sas_token_parts {
                    qs.append_pair(&sas_key, &sas_value);
                }
            } else if let Some(authorization) = req
                .extensions()
                .get::<RequestState>()
                .and_then(|state| state.supplied_api_key.as_ref())
            {
                // If we don't have a SLIPWAY_SECRET, we use the API key as a bearer token.
                qs.append_pair("authorization", &authorization.api_key);
            }

            // Used as a nonce to force Trmnl to reload the image.
            let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
            qs.append_pair("t", &timestamp);

            let full_url = format!("{}?{}", full_url_without_qs, qs.finish());

            let url = url::Url::parse(&full_url)
                .context("Failed to parse generated rig url.")
                .map_err(ServeError::Internal)?;

            Ok(RigResponse::Url(UrlResponse { url }))
        }
    }
}
