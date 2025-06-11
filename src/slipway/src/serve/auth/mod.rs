use std::borrow::Cow;
use std::collections::HashMap;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::http::header::HeaderMap;
use actix_web::middleware::Next;
use actix_web::web::Data;
use actix_web::{HttpMessage, web};

use slipway_host::hash_string;
use tracing::debug;

use crate::serve::{
    ACCESS_TOKEN_HEADER, AUTHORIZATION_HEADER, ShowApiKeys, SuppliedApiKey,
    truncate_hashed_api_key, write_api_key_message,
};

use super::SLIPWAY_SECRET_KEY;
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

    let query_string = req.query_string();
    let query_map: std::collections::HashMap<_, _> =
        url::form_urlencoded::parse(query_string.as_bytes()).collect();

    let headers = req.headers();

    let serve_state = req
        .app_data::<web::Data<ServeState>>()
        .expect("ServeState should exist.");

    let maybe_api_key = try_get_trmnl_api_key_from_request(headers, &query_map)
        .and_then(|raw_api_key| try_lookup_api_key(&raw_api_key, serve_state));

    req.extensions_mut().insert(RequestState {
        supplied_api_key: maybe_api_key,
    });

    next.call(req).await
}

fn try_get_trmnl_api_key_from_request(
    headers: &HeaderMap,
    query_map: &HashMap<Cow<'_, str>, Cow<'_, str>>,
) -> Option<String> {
    headers
        .get(ACCESS_TOKEN_HEADER) // Try the TRMNL standard header first.
        .and_then(|v| v.to_str().ok().map(|v| v.to_string()))
        .or_else(|| {
            headers
                .get(AUTHORIZATION_HEADER) // Fallback to the Authorization header.
                .and_then(|v| v.to_str().ok().map(|v| v.to_string()))
        })
        .or_else(|| {
            // Fallback to the Authorization query parameter, which must be used with the TRMNL Redirect plugin.
            let query_auth = query_map.get(AUTHORIZATION_HEADER);
            query_auth.map(|v| v.to_string())
        })
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
        auth_middleware_sas(&req, serve_state, query_map)?;
    } else {
        auth_middleware_api_key(&req, serve_state, query_map).await?;
    }

    next.call(req).await
}

fn try_lookup_api_key(raw_api_key: &str, serve_state: &Data<ServeState>) -> Option<SuppliedApiKey> {
    let registered_api_keys = &serve_state.config.api_keys;

    let api_key = if let Some(authorization_header) = raw_api_key.strip_prefix(BEARER_PREFIX) {
        authorization_header
    } else {
        raw_api_key
    };

    let hashed_api_key = hash_string(api_key);

    let resolved_api_key = registered_api_keys
        .iter()
        .find(|v| v.hashed_key == hashed_api_key);

    let Some(resolved_api_key) = resolved_api_key else {
        if matches!(serve_state.config.show_api_keys, ShowApiKeys::New) {
            write_api_key_message(api_key);
        }

        return Some(SuppliedApiKey {
            api_key: api_key.to_string(),
            resolved: None,
        });
    };

    if matches!(serve_state.config.show_api_keys, ShowApiKeys::Always) {
        write_api_key_message(api_key);
    }

    debug!(
        "Authenticated with hashed API key starting \"{}\"",
        truncate_hashed_api_key(&hashed_api_key)
    );

    Some(SuppliedApiKey {
        api_key: api_key.to_string(),
        resolved: Some(resolved_api_key.clone()),
    })
}

async fn auth_middleware_api_key(
    req: &ServiceRequest,
    serve_state: &Data<ServeState>,
    query_map: HashMap<Cow<'_, str>, Cow<'_, str>>,
) -> Result<(), actix_web::Error> {
    let full_authorization_header = req
        .headers()
        .get(AUTHORIZATION_HEADER)
        .and_then(|v| v.to_str().map(Cow::Borrowed).ok())
        .unwrap_or_else(|| {
            let query_auth = query_map.get(AUTHORIZATION_HEADER);
            query_auth.cloned().unwrap_or(Cow::Borrowed(""))
        });

    let maybe_api_key = try_lookup_api_key(&full_authorization_header, serve_state);

    let Some(used_api_key) = maybe_api_key else {
        return Err(
            ServeError::UserFacing(StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into(),
        );
    };

    if used_api_key.resolved.is_none() {
        return Err(
            ServeError::UserFacing(StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into(),
        );
    };

    req.extensions_mut().insert(RequestState {
        supplied_api_key: Some(used_api_key.clone()),
    });

    Ok(())
}

fn auth_middleware_sas(
    req: &ServiceRequest,
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

    let secret = serve_state.secret.as_ref().ok_or_else(|| {
        ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            format!(
                "{} environment variable has not been set.",
                SLIPWAY_SECRET_KEY
            ),
        )
    })?;

    if matches!(serve_state.config.show_api_keys, ShowApiKeys::Always) {
        debug!("The device authenticated using a shared access signature.");
    }

    req.extensions_mut().insert(RequestState {
        supplied_api_key: None,
    });

    sas::verify_sas_token(secret, expiry, signature)
}
