use std::borrow::Cow;
use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{HttpMessage, HttpRequest, get, web};
use anyhow::Context;
use serde::Deserialize;
use tracing::{Instrument, info_span};

use crate::primitives::RigName;
use crate::serve::auth::compute_signature_parts;
use crate::serve::{API_GET_DEVICE_PATH, RequestState, TRMNL_DISPLAY_PATH};

use crate::serve::responses::{
    FormatQuery, ImageResponse, RigResponse, RigResultFormat, RigResultImageFormat, ServeError,
    UrlResponse,
};

use super::super::ServeState;

#[derive(Deserialize)]
struct GetRigPath {
    rig_name: RigName,
}

#[derive(Deserialize)]
struct GetRigQuery {
    #[serde(flatten)]
    output: FormatQuery,
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
    let image_format = query.output.image_format.unwrap_or_default();
    let format = query.output.format.unwrap_or_default();

    get_rig_response(&rig_name, None, format, image_format, state, req)
        .instrument(info_span!("rig", %rig_name))
        .await
}

pub struct RequestingDevice {
    pub name: String,
    pub context: Option<serde_json::Value>,
}

pub async fn get_rig_response(
    rig_name: &RigName,
    device: Option<RequestingDevice>,
    format: RigResultFormat,
    image_format: RigResultImageFormat,
    state: Arc<ServeState>,
    req: HttpRequest,
) -> Result<RigResponse, ServeError> {
    let rig = state.repository.get_rig(rig_name).await?;

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
                    let path = match device {
                        Some(device) => {
                            let device_name = device.name;
                            format!("{path_without_trmnl}{API_GET_DEVICE_PATH}/{device_name}")
                        }
                        None => panic!("TRMNL display requests should provide a device"),
                    };
                    Cow::Owned(path)
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
                qs.append_pair("authorization", authorization);
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
