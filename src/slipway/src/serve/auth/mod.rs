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

use crate::serve::repository::DEVICE_FOLDER_NAME;
use crate::serve::{ShowApiKeys, write_api_key_message};

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

    let maybe_api_key =
        super::trmnl::try_get_api_key_from_headers(req.headers())?.map(|v| v.to_string());
    req.extensions_mut().insert(RequestState {
        supplied_api_key: maybe_api_key,
    });

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
        auth_middleware_sas(&req, serve_state, query_map)?;
    } else {
        auth_middleware_api_key(&req, serve_state, query_map).await?;
    }

    next.call(req).await
}

async fn auth_middleware_api_key(
    req: &ServiceRequest,
    serve_state: &Data<ServeState>,
    query_map: HashMap<Cow<'_, str>, Cow<'_, str>>,
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

    let api_key =
        if let Some(authorization_header) = full_authorization_header.strip_prefix(BEARER_PREFIX) {
            authorization_header
        } else {
            full_authorization_header.as_ref()
        };

    req.extensions_mut().insert(RequestState {
        supplied_api_key: Some(api_key.to_string()),
    });

    let hashed_api_key = hash_string(api_key);

    let used_api_key_name = {
        // First search the hashed API keys in the config.
        let used_api_key_name = hashed_api_keys.iter().find_map(|(n, v)| {
            if hashed_api_key == **v {
                Some(Cow::Borrowed(&n.0))
            } else {
                None
            }
        });

        match used_api_key_name {
            Some(used_api_key_name) => Some(used_api_key_name),
            None => serve_state
                .repository
                .try_get_device_by_api_key(api_key)
                .await?
                .map(|(device_name, _)| Cow::Owned(format!("{DEVICE_FOLDER_NAME}/{device_name}",))),
        }
    };

    if matches!(serve_state.config.show_api_keys, ShowApiKeys::Always) {
        write_api_key_message(api_key);
    }

    let Some(used_api_key_name) = used_api_key_name else {
        if matches!(serve_state.config.show_api_keys, ShowApiKeys::New) {
            write_api_key_message(api_key);
        }

        return Err(
            ServeError::UserFacing(StatusCode::UNAUTHORIZED, "Unauthorized".to_string()).into(),
        );
    };

    debug!("Authenticated with API key \"{}\"", used_api_key_name);

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
