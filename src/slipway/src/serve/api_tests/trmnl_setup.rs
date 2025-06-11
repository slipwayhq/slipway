use std::{collections::HashMap, path::PathBuf};

use actix_web::{http::StatusCode, test};
use slipway_host::hash_string;

use crate::serve::api_tests::create_device_auth_for_key;
use crate::serve::{ID_HEADER, ShowApiKeys, SlipwayServeEnvironment};
use crate::serve::{RepositoryConfig, SlipwayServeConfig, create_app};

use super::{device, get_body_json, playlist, rig};

const MAC: &str = "aa:bb:cc:00:00:01";
const HASHED_API_KEY: &str = "bar";
const API_KEY: &str = "abcdefg";

#[test_log::test(actix_web::test)]
async fn when_device_already_configured_for_trmnl_it_should_return_new_credentials() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_device_auth_for_key(API_KEY, "d_1"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/setup")
        .append_header((ID_HEADER, MAC))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body_json(response).await;

    assert_response(status, body);
}

#[test_log::test(actix_web::test)]
async fn when_device_not_configured_for_trmnl_it_should_return_new_credentials() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: Vec::new(),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/setup")
        .append_header((ID_HEADER, MAC))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body_json(response).await;

    assert_response(status, body);
}

fn assert_response(status: StatusCode, body: serde_json::Value) {
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"].as_u64(), Some(200));

    let api_key = body["api_key"].as_str().unwrap();
    assert!(!api_key.is_empty());
    assert_ne!(api_key, MAC);

    let hashed_api_key = hash_string(api_key);
    assert_ne!(hashed_api_key, MAC);

    let friendly_id = body["friendly_id"].as_str().unwrap();
    assert!(!friendly_id.is_empty());
    assert_eq!(&hashed_api_key[..6], friendly_id);
}
