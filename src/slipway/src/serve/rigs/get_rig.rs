use std::borrow::Cow;
use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{HttpRequest, get, web};
use anyhow::Context;
use serde::Deserialize;
use tracing::{Instrument, info_span};

use crate::primitives::RigName;
use crate::serve::auth::compute_signature_parts;
use crate::serve::{API_GET_RIG_PATH, SLIPWAY_ENCRYPTION_KEY_ENV_KEY, TRMNL_DISPLAY_PATH};

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

    get_rig_response(&rig_name, format, image_format, state, req)
        .instrument(info_span!("rig", %rig_name))
        .await
}

pub async fn get_rig_response(
    rig_name: &RigName,
    format: RigResultFormat,
    image_format: RigResultImageFormat,
    state: Arc<ServeState>,
    req: HttpRequest,
) -> Result<RigResponse, ServeError> {
    let rig = state.repository.get_rig(rig_name).await?;

    match format {
        RigResultFormat::Image | RigResultFormat::DataUrl | RigResultFormat::Json => {
            let result = super::run_rig::run_rig(state, rig, rig_name)
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
                    Cow::Owned(format!("{path_without_trmnl}{API_GET_RIG_PATH}/{rig_name}"))
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

            let encryption_key = state.encryption_key.as_deref().ok_or_else(|| {
                ServeError::UserFacing(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "{} environment variable has not been set.",
                        SLIPWAY_ENCRYPTION_KEY_ENV_KEY
                    ),
                )
            })?;

            let sas_token_parts =
                compute_signature_parts(encryption_key, chrono::Duration::seconds(60));

            for (sas_key, sas_value) in sas_token_parts {
                qs.append_pair(&sas_key, &sas_value);
            }

            // Used as a nonce to force Trmnl to reload the image.
            let timestamp = chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string();
            qs.append_pair("timestamp", &timestamp);

            let full_url = format!("{}?{}", full_url_without_qs, qs.finish());

            let url = url::Url::parse(&full_url)
                .context("Failed to parse generated rig url.")
                .map_err(ServeError::Internal)?;

            Ok(RigResponse::Url(UrlResponse { url }))
        }
    }
}
