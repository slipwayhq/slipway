use std::{collections::HashMap, path::PathBuf};

use actix_web::body::MessageBody;
use actix_web::dev::ServiceResponse;
use actix_web::{http::StatusCode, test};

use crate::serve::api_tests::{
    create_auth_for_key, create_device_auth_for_key, device_with_spec, get_body,
};
use crate::serve::repository::{RigResultImageFormat, RigResultPartialSpec};
use crate::serve::{
    ACCESS_TOKEN_HEADER, AUTHORIZATION_HEADER, ShowApiKeys, SlipwayServeEnvironment,
};
use crate::serve::{RepositoryConfig, SlipwayServeConfig, create_app};

use super::{device, get_body_json, playlist, rig};

const MAC: &str = "aa:bb:cc:00:00:01";
const MAC2: &str = "aa:bb:cc:00:00:02";
const API_KEY: &str = "abcdefg";
const API_KEY2: &str = "abcdefg2";

fn secret() -> Option<String> {
    Some("secret_123".to_string())
}

#[test_log::test(actix_web::test)]
async fn when_no_device_associated_with_api_key_it_should_return_forbidden() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key(API_KEY),
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

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.contains("not associated with any device"));
}

#[test_log::test(actix_web::test)]
async fn when_api_key_incorrect_it_should_return_unauthorized() {
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

    let app = test::init_service(create_app(PathBuf::from("."), None, config, secret())).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ACCESS_TOKEN_HEADER, API_KEY2))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body(response).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("API key was not recognized"));
}

#[test_log::test(actix_web::test)]
async fn when_valid_request_and_secret_it_should_return_rig_result_with_sas() {
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

    let app = test::init_service(create_app(PathBuf::from("."), None, config, secret())).await;

    let request = test::TestRequest::get()
        .uri("/trmnl/api/display")
        .append_header((ACCESS_TOKEN_HEADER, API_KEY))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"].as_u64(), Some(0));

    let image_url = body["image_url"].as_str().unwrap();
    let url = body["url"].as_str().unwrap();
    assert_eq!(image_url, url);
    assert!(image_url.contains("/devices/d_1?format=image&image_format=bmp_1bit&rotate=0&"));
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

    async fn assert_response(response: ServiceResponse<impl MessageBody>) {
        let status = response.status();
        let body = get_body_json(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"].as_u64(), Some(0));

        let image_url = body["image_url"].as_str().unwrap();
        let url = body["url"].as_str().unwrap();
        assert_eq!(image_url, url);
        assert!(image_url.contains("/devices/d_1?format=image&image_format=bmp_1bit&rotate=0"));
        assert!(image_url.contains("&authorization="));
        assert!(image_url.contains("&device=d_1"));
        assert!(!image_url.contains("&sig="));
        assert!(!image_url.contains("&exp="));
        assert!(image_url.contains("&t="));
    }

    {
        let request = test::TestRequest::get()
            .uri("/trmnl/api/display")
            .append_header((ACCESS_TOKEN_HEADER, API_KEY))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/trmnl/api/display")
            .append_header((AUTHORIZATION_HEADER, API_KEY))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        // Required for TRMNL Redirect plugin.
        let request = test::TestRequest::get()
            .uri(&format!("/trmnl/api/display?authorization={API_KEY}"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response).await;
    }
}

#[test_log::test(actix_web::test)]
async fn when_valid_request_and_image_format_overridden_it_should_return_specified_format() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_device_auth_for_key(API_KEY, "d_1"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device_with_spec(
                "d_1",
                "p_1",
                RigResultPartialSpec {
                    image_format: Some(RigResultImageFormat::Png),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    async fn assert_response(response: ServiceResponse<impl MessageBody>) {
        let status = response.status();
        let body = get_body_json(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"].as_u64(), Some(0));

        let image_url = body["image_url"].as_str().unwrap();
        let url = body["url"].as_str().unwrap();
        assert_eq!(image_url, url);
        assert!(image_url.contains("/devices/d_1?format=image&image_format=png&rotate=0&"));
        assert!(image_url.contains("&authorization="));
        assert!(image_url.contains("&device=d_1"));
        assert!(!image_url.contains("&sig="));
        assert!(!image_url.contains("&exp="));
        assert!(image_url.contains("&t="));
    }

    {
        let request = test::TestRequest::get()
            .uri("/trmnl/api/display")
            .append_header((ACCESS_TOKEN_HEADER, API_KEY))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/trmnl/api/display")
            .append_header((AUTHORIZATION_HEADER, API_KEY))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        // Required for TRMNL Redirect plugin.
        let request = test::TestRequest::get()
            .uri(&format!("/trmnl/api/display?authorization={API_KEY}"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response).await;
    }
}
