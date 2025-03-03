use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpMessage, HttpRequest};
use serde::Deserialize;

use crate::primitives::RigName;

use super::{ImageResponse, RequestState, RigResponse, ServeError, ServeState};
use image::ImageFormat;

#[derive(Deserialize)]
struct GetRigPath {
    rig_name: RigName,
}

#[derive(Deserialize)]
struct GetRigQuery {
    #[serde(default)]
    format: Option<RigResultFormat>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RigResultFormat {
    Jpeg,
    Png,
    Json,
    PngHtml,
    JpegHtml,
    PngHtmlNoEmbed,
    JpegHtmlNoEmbed,
}

#[get("/rig/{rig_name}")]
async fn get_rig(
    path: web::Path<GetRigPath>,
    query: web::Query<GetRigQuery>,
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<RigResponse, ServeError> {
    let path = path.into_inner();
    let query = query.into_inner();
    let state = data.into_inner();

    match query.format {
        Some(RigResultFormat::PngHtmlNoEmbed) | Some(RigResultFormat::JpegHtmlNoEmbed) => {
            let connection_info = req.connection_info();
            let scheme = connection_info.scheme();
            let host = connection_info.host();
            let uri = req.uri();
            let path = uri.path();

            let full_url = format!("{}://{}{}", scheme, host, path);

            let mut qs = url::form_urlencoded::Serializer::new(String::new());

            qs.append_pair(
                "format",
                match query.format {
                    Some(RigResultFormat::JpegHtmlNoEmbed) => "jpeg",
                    _ => "png",
                },
            );

            if let Some(authorization) = req
                .extensions()
                .get::<RequestState>()
                .and_then(|state| state.authorized_header.as_ref())
            {
                qs.append_pair("authorization", authorization);
            }

            // Used as a nonce to force Trmnl to reload the image.
            let timestamp = chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string();
            qs.append_pair("timestamp", &timestamp);

            Ok(RigResponse::Html(format!(
                r#"<html><body style="margin:0px"><img src="{}?{}"/></body></html>"#,
                full_url,
                qs.finish()
            )))
        }
        _ => get_rig_inner(path.rig_name, query.format, state).await,
    }
}

async fn get_rig_inner(
    rig_name: RigName,
    result_format: Option<RigResultFormat>,
    state: Arc<ServeState>,
) -> Result<RigResponse, ServeError> {
    let rig = state.repository.get_rig(&rig_name).await?;

    let result = super::run_rig::run_rig(state, rig)
        .await
        .map_err(ServeError::Internal)?;

    match result_format {
        None
        | Some(RigResultFormat::Png)
        | Some(RigResultFormat::Jpeg)
        | Some(RigResultFormat::PngHtml)
        | Some(RigResultFormat::JpegHtml) => {
            let maybe_image = crate::canvas::get_canvas_image(&result.handle, &result.output);

            if let Ok(image) = maybe_image {
                Ok(RigResponse::Image(ImageResponse {
                    image,
                    format: match result_format {
                        Some(RigResultFormat::Jpeg) => ImageFormat::Jpeg,
                        _ => ImageFormat::Png,
                    },
                    wrap_in_html: matches!(
                        result_format,
                        Some(RigResultFormat::PngHtml) | Some(RigResultFormat::JpegHtml)
                    ),
                }))
            } else {
                match result_format {
                    None => Ok(RigResponse::Json(web::Json(result.output))),
                    _ => Err(ServeError::UserFacing(
                        StatusCode::BAD_REQUEST,
                        "Could not render first rig output as an image.".to_string(),
                    )),
                }
            }
        }
        Some(RigResultFormat::Json) => Ok(RigResponse::Json(web::Json(result.output))),
        Some(RigResultFormat::JpegHtmlNoEmbed) | Some(RigResultFormat::PngHtmlNoEmbed) => {
            unreachable!();
        }
    }
}
