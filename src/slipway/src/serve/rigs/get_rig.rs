use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpMessage, HttpRequest};
use anyhow::Context;
use serde::Deserialize;
use tracing::{info_span, Instrument};

use crate::primitives::RigName;
use crate::serve::{RigResultImageFormat, RigResultPresentation, UrlResponse};

use super::super::{ImageResponse, RequestState, RigResponse, ServeError, ServeState};

#[derive(Deserialize)]
struct GetRigPath {
    rig_name: RigName,
}

#[derive(Deserialize)]
struct GetRigQuery {
    #[serde(default)]
    format: Option<RigResultImageFormat>,

    #[serde(default)]
    presentation: Option<RigResultPresentation>,
}

#[get("/rig/{rig_name}")]
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
    let format = query.format.unwrap_or_default();
    let presentation = query.presentation.unwrap_or_default();

    get_rig_response(&rig_name, format, presentation, state, req)
        .instrument(info_span!("rig", %rig_name))
        .await
}

pub async fn get_rig_response(
    rig_name: &RigName,
    format: RigResultImageFormat,
    presentation: RigResultPresentation,
    state: Arc<ServeState>,
    req: HttpRequest,
) -> Result<RigResponse, ServeError> {
    let rig = state.repository.get_rig(rig_name).await?;

    match presentation {
        RigResultPresentation::Image
        | RigResultPresentation::DataUrl
        | RigResultPresentation::Json => {
            let result = super::run_rig::run_rig(state, rig, rig_name)
                .await
                .map_err(ServeError::Internal)?;

            if matches!(presentation, RigResultPresentation::Json) {
                Ok(RigResponse::Json(web::Json(result.output)))
            } else {
                let maybe_image = crate::canvas::get_canvas_image(&result.handle, &result.output);

                if let Ok(image) = maybe_image {
                    Ok(RigResponse::Image(ImageResponse {
                        image,
                        format,
                        wrap_in_html: matches!(presentation, RigResultPresentation::DataUrl),
                    }))
                } else {
                    Err(ServeError::UserFacing(
                        StatusCode::BAD_REQUEST,
                        "Could not render rig output as an image.".to_string(),
                    ))
                }
            }
        }
        RigResultPresentation::Url => {
            let connection_info = req.connection_info();
            let scheme = connection_info.scheme();
            let host = connection_info.host();
            let uri = req.uri();
            let path = uri.path();

            let full_url_without_qs = format!("{}://{}{}", scheme, host, path);

            let mut qs = url::form_urlencoded::Serializer::new(String::new());

            qs.append_pair(
                "format",
                serde_json::to_value(&format)
                    .expect("Format should serialize")
                    .as_str()
                    .expect("Format should be a string"),
            );

            if let Some(authorization) = req
                .extensions()
                .get::<RequestState>()
                .and_then(|state| state.required_authorization_header.as_ref())
            {
                qs.append_pair("authorization", authorization);
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
