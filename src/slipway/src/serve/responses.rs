use actix_web::body::{BoxBody, EitherBody};
use actix_web::http::StatusCode;
use actix_web::http::header::{ContentType, HeaderName, HeaderValue};
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};

use base64::prelude::*;
use image::{DynamicImage, ImageFormat, RgbaImage};
use slipway_host::hash_bytes;
use std::io::Cursor;
use thiserror::Error;
use tracing::{debug, info};
use url::Url;

#[derive(Debug, Error)]
pub(super) enum ServeError {
    #[error("internal error: {0:?}")]
    Internal(anyhow::Error),

    #[error("{0}: {1}")]
    UserFacing(StatusCode, String),

    #[error("{0}: {1}")]
    UserFacingJson(StatusCode, serde_json::Value),
}

impl actix_web::error::ResponseError for ServeError {
    fn error_response(&self) -> HttpResponse {
        debug!("Error response: {:?}", self);

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            ServeError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServeError::UserFacing(status_code, _) => status_code,
            ServeError::UserFacingJson(status_code, _) => status_code,
        }
    }
}

pub(super) enum RigResponse {
    Image(ImageResponse),
    Json(web::Json<serde_json::Value>),
    Url(UrlResponse),
}

impl Responder for RigResponse {
    type Body = EitherBody<std::string::String>;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            RigResponse::Image(image) => image.respond_to(req).map_into_right_body(),
            RigResponse::Json(json) => json.respond_to(req),
            RigResponse::Url(url) => url.respond_to(req).map_into_right_body(),
        }
    }
}

pub(super) struct PlaylistResponse {
    pub refresh_rate_seconds: u32,
    pub rig_response: RigResponse,
}

impl Responder for PlaylistResponse {
    type Body = EitherBody<std::string::String>;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let mut response = match self.rig_response {
            RigResponse::Image(image) => image
                .respond_with_refresh(req, self.refresh_rate_seconds)
                .map_into_right_body(),
            RigResponse::Json(json) => json.respond_to(req),
            RigResponse::Url(url) => url
                .respond_with_refresh(req, self.refresh_rate_seconds)
                .map_into_right_body(),
        };

        response.headers_mut().append(
            HeaderName::from_static(super::REFRESH_RATE_HEADER),
            HeaderValue::from_str(&self.refresh_rate_seconds.to_string())
                .expect("Refresh rate header value should be valid."),
        );

        response
    }
}

pub(super) struct UrlResponse {
    pub url: Url,
}
impl UrlResponse {
    fn respond_with_refresh(
        self,
        _req: &HttpRequest,
        refresh_rate_seconds: u32,
    ) -> HttpResponse<BoxBody> {
        let url = self.url;

        let meta_refresh = if refresh_rate_seconds > 0 {
            format!(r#"<meta http-equiv="refresh" content="{refresh_rate_seconds}">"#)
        } else {
            String::new()
        };

        let html = format!(
            r#"<html><head>{meta_refresh}</head><body style="margin:0px"><img src="{}"/></body></html>"#,
            url
        );

        HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(html)
    }
}
impl Responder for UrlResponse {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        self.respond_with_refresh(req, 0)
    }
}

pub(super) struct ImageResponse {
    pub image: RgbaImage,
    pub format: RigResultImageFormat,
    pub wrap_in_html: bool,
}

impl ImageResponse {
    fn respond_with_refresh(
        self,
        _req: &HttpRequest,
        refresh_rate_seconds: u32,
    ) -> HttpResponse<BoxBody> {
        let width = self.image.width();

        let maybe_image_bytes = match self.format {
            RigResultImageFormat::Jpeg => get_image_bytes(self.image, ImageFormat::Jpeg),
            RigResultImageFormat::Png => get_image_bytes(self.image, ImageFormat::Png),
            RigResultImageFormat::Bmp1Bit => super::bmp::encode_1bit_bmp(self.image),
        };

        let image_bytes = match maybe_image_bytes {
            Err(e) => {
                return HttpResponse::InternalServerError().body(format!("{:?}", e));
            }
            Ok(image_bytes) => image_bytes,
        };

        info!("Responding with image of size {} bytes.", image_bytes.len());

        if self.wrap_in_html {
            let meta_refresh = if refresh_rate_seconds > 0 {
                format!(r#"<meta http-equiv="refresh" content="{refresh_rate_seconds}">"#)
            } else {
                String::new()
            };
            let html = format!(
                r#"<html><head>{meta_refresh}<meta name="viewport" content="width={width}"></head><body style="margin:0px; width={width}px"><img src="data:image/png;base64,{}"/></body></html>"#,
                BASE64_STANDARD.encode(&image_bytes)
            );

            return HttpResponse::Ok()
                .content_type(ContentType::html())
                .body(html);
        }

        let maybe_etag = _req
            .headers()
            .get("if-none-match")
            .and_then(|v| v.to_str().ok());

        if let Some(etag) = maybe_etag {
            debug!("Device supplied ETag: {}", etag);
            let new_etag = hash_bytes(&image_bytes);
            debug!("Calculated ETag     : {}", new_etag);

            if etag == new_etag {
                info!("Returning 304 Not Modified.");
                return HttpResponse::NotModified().finish();
            }
        }

        let etag = hash_bytes(&image_bytes);

        let body = image_bytes;

        let mut response = HttpResponse::Ok();

        response.insert_header((
            HeaderName::from_static("etag"),
            HeaderValue::from_str(&etag).expect("ETag value should lowercase hex"),
        ));

        match self.format {
            RigResultImageFormat::Jpeg => {
                response.content_type(ContentType::jpeg());
            }
            RigResultImageFormat::Png => {
                response.content_type(ContentType::png());
            }
            RigResultImageFormat::Bmp1Bit => {
                response.content_type("image/bmp");
            }
        };

        response.body(body)
    }
}

impl Responder for ImageResponse {
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        self.respond_with_refresh(req, 0)
    }
}

fn get_image_bytes(image: RgbaImage, format: ImageFormat) -> Result<Vec<u8>, image::ImageError> {
    let dynamic = DynamicImage::ImageRgba8(image);

    let mut buf = Cursor::new(Vec::new());

    dynamic.write_to(&mut buf, format)?;

    Ok(buf.into_inner())
}

#[derive(Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(super) enum RigResultImageFormat {
    Jpeg,

    #[default]
    Png,

    // Specify serde string as "bmp_1bit", to avoid default of `bmp1_bit`.
    #[serde(rename = "bmp_1bit")]
    Bmp1Bit,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(super) enum RigResultFormat {
    /// Return an image.
    #[default]
    Image,

    /// Return the JSON output of the rig.
    Json,

    /// Return the image encoded as a data URL.
    /// We expose this as `html_embed` to the user because while internally this is a data URL,
    /// the user sees it as an HTML page containing a data URL.
    #[serde(rename = "html_embed")]
    DataUrl,

    /// Return a URL which will generate the image.
    /// We expose this as `html` to the user because while internally this is a URL,
    /// the user sees it as an HTML page containing a URL.
    #[serde(rename = "html")]
    Url,
}

#[derive(Deserialize)]
pub(super) struct FormatQuery {
    #[serde(default)]
    pub image_format: Option<RigResultImageFormat>,

    #[serde(default)]
    pub format: Option<RigResultFormat>,
}
