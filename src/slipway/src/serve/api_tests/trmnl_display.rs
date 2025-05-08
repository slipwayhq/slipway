use std::{collections::HashMap, path::PathBuf};

use actix_web::{http::StatusCode, test};
use chrono_tz::Tz;
use slipway_host::hash_string;

use crate::serve::api_tests::get_body;
use crate::serve::{ACCESS_TOKEN_HEADER, ID_HEADER, ShowApiKeys};
use crate::serve::{RepositoryConfig, SlipwayServeConfig, create_app, repository::TrmnlDevice};

use super::super::Device;
use super::{device, dn, get_body_json, playlist, pn, rig};

const MAC: &str = "aa:bb:cc:00:00:01";
const MAC2: &str = "aa:bb:cc:00:00:02";
const API_KEY: &str = "abcdefg";
const API_KEY2: &str = "abcdefg2";

fn secret() -> Option<String> {
    Some("secret_123".to_string())
}

#[test_log::test(actix_web::test)]
async fn when_no_device_id_header_it_should_return_bad_request() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        hashed_api_keys: HashMap::new(),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, secret())).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ACCESS_TOKEN_HEADER, API_KEY))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body(response).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Missing ID"));
}

#[test_log::test(actix_web::test)]
async fn when_no_device_with_matching_id_it_should_return_not_found() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        hashed_api_keys: HashMap::new(),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![(
                dn("d_1"),
                Device {
                    trmnl: Some(TrmnlDevice {
                        hashed_id: hash_string(MAC2),
                        hashed_api_key: hash_string(API_KEY),
                        reset_firmware: false,
                    }),
                    playlist: Some(pn("p_1")),
                    context: None,
                },
            )]
            .into_iter()
            .collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, secret())).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ID_HEADER, MAC))
        .append_header((ACCESS_TOKEN_HEADER, API_KEY))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body(response).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("No device"));
}

#[test_log::test(actix_web::test)]
async fn when_api_key_incorrect_it_should_return_unauthorized() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        hashed_api_keys: HashMap::new(),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![(
                dn("d_1"),
                Device {
                    trmnl: Some(TrmnlDevice {
                        hashed_id: hash_string(MAC),
                        hashed_api_key: hash_string(API_KEY),
                        reset_firmware: false,
                    }),
                    playlist: Some(pn("p_1")),
                    context: None,
                },
            )]
            .into_iter()
            .collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, secret())).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ID_HEADER, MAC))
        .append_header((ACCESS_TOKEN_HEADER, API_KEY2))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body(response).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("Invalid credentials"));
}

#[test_log::test(actix_web::test)]
async fn when_reset_firmware_set_it_should_return_reset_firmware_flag() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        hashed_api_keys: HashMap::new(),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![(
                dn("d_1"),
                Device {
                    trmnl: Some(TrmnlDevice {
                        hashed_id: hash_string(MAC),
                        hashed_api_key: hash_string(API_KEY),
                        reset_firmware: true,
                    }),
                    playlist: Some(pn("p_1")),
                    context: None,
                },
            )]
            .into_iter()
            .collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, secret())).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ID_HEADER, MAC))
        .append_header((ACCESS_TOKEN_HEADER, API_KEY))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["reset_firmware"].as_bool(), Some(true));
}

#[test_log::test(actix_web::test)]
async fn when_valid_request_and_secret_it_should_return_rig_result_with_sas() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        hashed_api_keys: HashMap::new(),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![(
                dn("d_1"),
                Device {
                    trmnl: Some(TrmnlDevice {
                        hashed_id: hash_string(MAC),
                        hashed_api_key: hash_string(API_KEY),
                        reset_firmware: false,
                    }),
                    playlist: Some(pn("p_1")),
                    context: None,
                },
            )]
            .into_iter()
            .collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, secret())).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ID_HEADER, MAC))
        .append_header((ACCESS_TOKEN_HEADER, API_KEY))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["reset_firmware"].as_bool(), Some(false));
    assert_eq!(body["status"].as_u64(), Some(0));

    let image_url = body["image_url"].as_str().unwrap();
    assert!(image_url.contains("/rigs/r_1?format=image&image_format=bmp_1bit&"));
    assert!(!image_url.contains("&authorization="));
    assert!(image_url.contains("&device=d_1"));
    assert!(image_url.contains("&sig="));
    assert!(image_url.contains("&exp="));
    assert!(image_url.contains("&t="));
}

#[test_log::test(actix_web::test)]
async fn when_valid_request_and_no_secret_it_should_return_rig_result_with_api_key() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        hashed_api_keys: HashMap::new(),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![(
                dn("d_1"),
                Device {
                    trmnl: Some(TrmnlDevice {
                        hashed_id: hash_string(MAC),
                        hashed_api_key: hash_string(API_KEY),
                        reset_firmware: false,
                    }),
                    playlist: Some(pn("p_1")),
                    context: None,
                },
            )]
            .into_iter()
            .collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ID_HEADER, MAC))
        .append_header((ACCESS_TOKEN_HEADER, API_KEY))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["reset_firmware"].as_bool(), Some(false));
    assert_eq!(body["status"].as_u64(), Some(0));

    let image_url = body["image_url"].as_str().unwrap();
    assert!(image_url.contains("/rigs/r_1?format=image&image_format=bmp_1bit&"));
    assert!(image_url.contains("&authorization="));
    assert!(image_url.contains("&device=d_1"));
    assert!(!image_url.contains("&sig="));
    assert!(!image_url.contains("&exp="));
    assert!(image_url.contains("&t="));
}
