use std::{collections::HashMap, path::PathBuf, str::FromStr};

use actix_web::{
    body::MessageBody,
    dev::ServiceResponse,
    http::{StatusCode, header::HeaderName},
    test,
};
use slipway_engine::{Rigging, SpecialComponentReference};
use slipway_host::hash_string;

use crate::{
    primitives::{DeviceName, PlaylistName, RigName},
    serve::{
        REFRESH_RATE_HEADER, RegisteredApiKey, RepositoryConfig, SlipwayServeConfig,
        SlipwayServeEnvironment, create_app,
        repository::{RigResultFormat, RigResultImageFormat, RigResultPartialSpec},
    },
};

use super::{
    Device, Playlist, ShowApiKeys,
    repository::{PlaylistItem, Refresh},
};

mod trmnl_display;
mod trmnl_setup;

fn dn(s: &str) -> DeviceName {
    DeviceName::from_str(s).unwrap()
}

fn pn(s: &str) -> PlaylistName {
    PlaylistName::from_str(s).unwrap()
}

fn rn(s: &str) -> RigName {
    RigName::from_str(s).unwrap()
}

fn device(name: &str, playlist_name: &str) -> (DeviceName, Device) {
    (
        dn(name),
        Device {
            playlist: Some(pn(playlist_name)),
            context: None,
            result_spec: Default::default(),
        },
    )
}

fn device_with_spec(
    name: &str,
    playlist_name: &str,
    result_spec: RigResultPartialSpec,
) -> (DeviceName, Device) {
    (
        dn(name),
        Device {
            playlist: Some(pn(playlist_name)),
            context: None,
            result_spec,
        },
    )
}

fn playlist(name: &str, rig_name: &str) -> (PlaylistName, Playlist) {
    (
        pn(name),
        Playlist {
            schedule: vec![PlaylistItem {
                time: None,
                days: None,
                refresh: Refresh::Hours { hours: 1 },
                rig: rn(rig_name),
            }],
        },
    )
}

fn rig(name: &str) -> (RigName, slipway_engine::Rig) {
    (
        rn(name),
        slipway_engine::Rig::for_test(Rigging {
            components: [(
                "output".parse().unwrap(),
                slipway_engine::ComponentRigging::for_test_with_reference(
                    slipway_engine::SlipwayReference::Special(
                        SpecialComponentReference::Passthrough,
                    ),
                    Some(serde_json::json!({
                        "foo": "bar"
                    })),
                ),
            )]
            .into_iter()
            .collect(),
        }),
    )
}

fn get_refresh_rate(response: &ServiceResponse<impl MessageBody>) -> Option<u32> {
    let refresh_rate = response
        .headers()
        .get(HeaderName::from_static(REFRESH_RATE_HEADER))
        .map(|v| u32::from_str(v.to_str().unwrap()).unwrap());
    println!("Refresh rate: {:?}", refresh_rate);
    refresh_rate
}

fn create_auth_for_key(key: &str) -> Vec<RegisteredApiKey> {
    vec![RegisteredApiKey {
        hashed_key: hash_string(key),
        device: None,
        description: Some("Test API Key".to_string()),
    }]
}

fn create_device_auth_for_key(key: &str, device: &str) -> Vec<RegisteredApiKey> {
    vec![RegisteredApiKey {
        hashed_key: hash_string(key),
        device: Some(DeviceName::from_str(device).unwrap()),
        description: Some("Test API Key".to_string()),
    }]
}

async fn get_body(response: ServiceResponse<impl MessageBody>) -> String {
    let body = test::read_body(response).await;
    let result = String::from_utf8(body.to_vec()).unwrap();
    println!("{}", result);
    result
}

async fn get_body_json(response: ServiceResponse<impl MessageBody>) -> serde_json::Value {
    let body = test::read_body(response).await;
    let result = serde_json::from_slice(&body).unwrap();
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    result
}

#[test_log::test(actix_web::test)]
async fn when_devices_playlists_and_rigs_do_not_exist_should_return_not_found() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key(""),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: HashMap::new(),
            playlists: HashMap::new(),
            rigs: HashMap::new(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    {
        let request = test::TestRequest::get().uri("/devices/foo").to_request();
        let response = test::call_service(&app, request).await;
        let status = response.status();
        let body = get_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Device not found"));
    }

    {
        let request = test::TestRequest::get().uri("/playlists/foo").to_request();
        let response = test::call_service(&app, request).await;
        let status = response.status();
        let body = get_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Playlist not found"));
    }

    {
        let request = test::TestRequest::get().uri("/rigs/foo").to_request();
        let response = test::call_service(&app, request).await;
        let status = response.status();
        let body = get_body(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Rig not found"));
    }
}

#[test_log::test(actix_web::test)]
async fn when_devices_playlists_and_rigs_exist_it_should_execute_rigs() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key(""),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    async fn assert_response(response: ServiceResponse<impl MessageBody>, has_refresh_rate: bool) {
        let status = response.status();
        let refresh_rate = get_refresh_rate(&response);
        let body = get_body_json(response).await;

        assert_eq!(status, StatusCode::OK);
        if has_refresh_rate {
            let Some(refresh_rate) = refresh_rate else {
                panic!("Expected refresh rate.");
            };
            assert!(refresh_rate > 3598 && refresh_rate < 3602);
        } else {
            assert!(refresh_rate.is_none());
        }
        assert_eq!(body, serde_json::json!({ "foo": "bar"}));
    }

    {
        let request = test::TestRequest::get()
            .uri("/devices/d_1?format=json")
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response, true).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/playlists/p_1?format=json")
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response, true).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/rigs/r_1?format=json")
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response, false).await;
    }
}

#[test_log::test(actix_web::test)]
async fn when_auth_not_supplied_it_should_return_unauthorized() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key("auth123"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    async fn assert_response(
        response: Result<ServiceResponse<impl MessageBody>, actix_web::Error>,
    ) {
        match response {
            Ok(_) => panic!("Expected error."),
            Err(e) => assert_eq!(e.error_response().status(), StatusCode::UNAUTHORIZED),
        }
    }

    {
        let request = test::TestRequest::get()
            .uri("/devices/d_1?format=json")
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/playlists/p_1?format=json")
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/rigs/r_1?format=json")
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }
}

#[test_log::test(actix_web::test)]
async fn when_auth_not_supplied_it_should_allow_favicon_requests() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key("auth123"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    let request = test::TestRequest::get().uri("/favicon.ico").to_request();
    let response = test::try_call_service(&app, request).await.unwrap();
    let status = response.status();
    assert_eq!(status, StatusCode::OK);
}

#[test_log::test(actix_web::test)]
async fn when_auth_incorrect_it_should_return_unauthorized() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_device_auth_for_key("auth456", "d_1"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    async fn assert_response(
        response: Result<ServiceResponse<impl MessageBody>, actix_web::Error>,
    ) {
        match response {
            Ok(_) => panic!("Expected error."),
            Err(e) => assert_eq!(e.error_response().status(), StatusCode::UNAUTHORIZED),
        }
    }

    {
        let request = test::TestRequest::get()
            .uri("/devices/d_1?format=json")
            .append_header(("Authorization", "auth1234"))
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        // Auth in the query string.
        let request = test::TestRequest::get()
            .uri("/playlists/p_1?format=json&authorization=auth1234")
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/rigs/r_1?format=json")
            .append_header(("Authorization", "auth1234"))
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }
}

#[test_log::test(actix_web::test)]
async fn when_auth_for_incorrect_device_it_should_return_unauthorized() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_device_auth_for_key("auth1234", "d_2"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    async fn assert_response(
        response: Result<ServiceResponse<impl MessageBody>, actix_web::Error>,
    ) {
        // This isn't an error response because the forbidden is returned deeper
        // in the stack.
        let status_code = match response {
            Ok(r) => r.status(),
            Err(e) => e.error_response().status(),
        };

        assert_eq!(status_code, StatusCode::FORBIDDEN);
    }

    {
        let request = test::TestRequest::get()
            .uri("/devices/d_1?format=json")
            .append_header(("Authorization", "auth1234"))
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        // Auth in the query string.
        let request = test::TestRequest::get()
            .uri("/playlists/p_1?format=json&authorization=auth1234")
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/rigs/r_1?format=json")
            .append_header(("Authorization", "auth1234"))
            .to_request();
        let response = test::try_call_service(&app, request).await;
        assert_response(response).await;
    }
}

#[test_log::test(actix_web::test)]
async fn when_auth_supplied_it_should_execute_rigs() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key("auth123"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    async fn assert_response(response: ServiceResponse<impl MessageBody>, has_refresh_rate: bool) {
        let status = response.status();
        let refresh_rate = get_refresh_rate(&response);
        let body = get_body_json(response).await;

        assert_eq!(status, StatusCode::OK);
        if has_refresh_rate {
            let Some(refresh_rate) = refresh_rate else {
                panic!("Expected refresh rate.");
            };
            assert!(refresh_rate > 3598 && refresh_rate < 3602);
        } else {
            assert!(refresh_rate.is_none());
        }
        assert_eq!(body, serde_json::json!({ "foo": "bar"}));
    }

    {
        let request = test::TestRequest::get()
            .uri("/devices/d_1?format=json")
            .append_header(("Authorization", "auth123"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response, true).await;
    }

    {
        // Auth in the query string.
        let request = test::TestRequest::get()
            .uri("/playlists/p_1?format=json&authorization=auth123")
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response, true).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/rigs/r_1?format=json")
            .append_header(("Authorization", "auth123"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response, false).await;
    }
}

#[test_log::test(actix_web::test)]
async fn when_device_auth_supplied_it_should_execute_rigs() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_device_auth_for_key("auth456", "d_1"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device("d_1", "p_1")].into_iter().collect(),
            playlists: vec![playlist("p_1", "r_1")].into_iter().collect(),
            rigs: vec![rig("r_1")].into_iter().collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), None, config, None)).await;

    async fn assert_response(response: ServiceResponse<impl MessageBody>, has_refresh_rate: bool) {
        let status = response.status();
        let refresh_rate = get_refresh_rate(&response);
        let body = get_body_json(response).await;

        assert_eq!(status, StatusCode::OK);
        if has_refresh_rate {
            let Some(refresh_rate) = refresh_rate else {
                panic!("Expected refresh rate.");
            };
            assert!(refresh_rate > 3598 && refresh_rate < 3602);
        } else {
            assert!(refresh_rate.is_none());
        }
        assert_eq!(body, serde_json::json!({ "foo": "bar"}));
    }

    {
        let request = test::TestRequest::get()
            .uri("/devices/d_1?format=json")
            .append_header(("Authorization", "auth456"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response, true).await;
    }
}

#[test_log::test(actix_web::test)]
async fn when_device_result_spec_it_should_pass_through_to_url() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key("auth123"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device_with_spec(
                "d_1",
                "p_1",
                RigResultPartialSpec {
                    format: Some(RigResultFormat::Url),
                    image_format: Some(RigResultImageFormat::Jpeg),
                    rotate: Some(90),
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
        .uri("/devices/d_1")
        .append_header(("Authorization", "auth123"))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body(response).await;

    assert_eq!(status, StatusCode::OK);

    assert!(body.contains("/devices/d_1?format=image&image_format=jpeg&rotate=90"));
}

#[test_log::test(actix_web::test)]
async fn when_device_result_spec_query_string_should_override() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        environment: SlipwayServeEnvironment::for_test(),
        rig_permissions: HashMap::new(),
        api_keys: create_auth_for_key("auth123"),
        show_api_keys: ShowApiKeys::Never,
        port: None,
        repository: RepositoryConfig::Memory {
            devices: vec![device_with_spec(
                "d_1",
                "p_1",
                RigResultPartialSpec {
                    format: Some(RigResultFormat::Image),
                    image_format: Some(RigResultImageFormat::Jpeg),
                    rotate: Some(90),
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
        .uri("/devices/d_1?format=html&image_format=bmp_1bit&rotate=180")
        .append_header(("Authorization", "auth123"))
        .to_request();
    let response = test::call_service(&app, request).await;
    let status = response.status();
    let body = get_body(response).await;

    assert_eq!(status, StatusCode::OK);

    assert!(body.contains("/devices/d_1?format=image&image_format=bmp_1bit&rotate=180"));
}
