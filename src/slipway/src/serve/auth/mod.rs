use std::borrow::Cow;
use std::collections::HashMap;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::Next;
use actix_web::web::Data;
use actix_web::{HttpMessage, web};

use slipway_host::hash_string;
use tracing::debug;

use super::SLIPWAY_ENCRYPTION_KEY_ENV_KEY;
use super::{RequestState, ServeState, responses::ServeError};

mod sas;

const SHARED_ACCESS_SIGNATURE_KEY: &str = "sig";
const EXPIRY_KEY: &str = "exp";

const BEARER_PREFIX: &str = "Bearer ";

pub(super) use sas::compute_signature_parts;

/// Trmnl endpoints use their own authentication system.
pub(super) async fn trmnl_auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    debug!("Running trmnl_auth_middleware for {}", req.request().path());
    req.extensions_mut().insert(RequestState {});
    next.call(req).await
}

/// Non-Trmnl endpoints use either an API key or a shared access signature for authentication.
pub(super) async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    debug!("Running auth_middleware for {}", req.request().path());

    let query_string = req.query_string();
    let query_map: std::collections::HashMap<_, _> =
        url::form_urlencoded::parse(query_string.as_bytes()).collect();

    let serve_state = req
        .app_data::<web::Data<ServeState>>()
        .expect("ServeState should exist.");

    if query_map.contains_key(SHARED_ACCESS_SIGNATURE_KEY) {
        debug!("Shared access signature found in query string.");
        auth_middleware_sas(serve_state, query_map)?;
    } else {
        auth_middleware_api_key(&req, serve_state, query_map)?;
    }

    req.extensions_mut().insert(RequestState {});
    next.call(req).await
}

fn auth_middleware_api_key(
    req: &ServiceRequest,
    serve_state: &Data<ServeState>,
    query_map: HashMap<Cow<str>, Cow<str>>,
) -> Result<(), actix_web::Error> {
    let hashed_api_keys = &serve_state.config.hashed_api_keys;

    let full_authorization_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().map(Cow::Borrowed).ok())
        .unwrap_or_else(|| {
            let query_auth = query_map.get("authorization");
            query_auth.cloned().unwrap_or(Cow::Borrowed(""))
        });

    let hashed_api_key = if let Some(full_authorization_header) =
        full_authorization_header.strip_prefix(BEARER_PREFIX)
    {
        hash_string(full_authorization_header)
    } else {
        hash_string(full_authorization_header.as_ref())
    };

    let used_api_key = hashed_api_keys.iter().find(|(_, v)| hashed_api_key == **v);

    let Some((used_api_key_name, _)) = used_api_key else {
        return Err(
            ServeError::UserFacing(StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into(),
        );
    };

    debug!("Authenticated with API key \"{}\"", used_api_key_name);

    Ok(())
}

fn auth_middleware_sas(
    serve_state: &Data<ServeState>,
    query_map: HashMap<Cow<str>, Cow<str>>,
) -> Result<(), actix_web::Error> {
    let signature = query_map.get(SHARED_ACCESS_SIGNATURE_KEY).ok_or_else(|| {
        ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            "Missing signature in SAS token.".to_string(),
        )
    })?;

    let expiry = query_map.get(EXPIRY_KEY).ok_or_else(|| {
        ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            "Missing expiry in SAS token.".to_string(),
        )
    })?;

    let encryption_key = serve_state.encryption_key.as_ref().ok_or_else(|| {
        ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            format!(
                "{} environment variable has not been set.",
                SLIPWAY_ENCRYPTION_KEY_ENV_KEY
            ),
        )
    })?;

    sas::verify_sas_token(encryption_key, expiry, signature)
}
