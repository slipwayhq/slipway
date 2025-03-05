mod trmnl;

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use actix_web::{
    body::MessageBody,
    dev::ServiceResponse,
    http::{header::HeaderName, StatusCode},
    test,
};
use chrono_tz::Tz;
use slipway_engine::{Rigging, SpecialComponentReference};

use crate::{
    primitives::{DeviceName, PlaylistName, RigName},
    serve::{create_app, RepositoryConfig, SlipwayServeConfig, REFRESH_RATE_HEADER},
};

use super::{
    repository::{PlaylistItem, Refresh},
    Device, Playlist,
};

fn dn(s: &str) -> DeviceName {
    DeviceName::from_str(s).unwrap()
}

fn pn(s: &str) -> PlaylistName {
    PlaylistName::from_str(s).unwrap()
}

fn rn(s: &str) -> RigName {
    RigName::from_str(s).unwrap()
}

fn get_refresh_rate(response: &ServiceResponse<impl MessageBody>) -> u32 {
    let refresh_rate = response
        .headers()
        .get(HeaderName::from_static(REFRESH_RATE_HEADER))
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    println!("Refresh rate: {}", refresh_rate);
    refresh_rate
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
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        repository: RepositoryConfig::Memory {
            devices: HashMap::new(),
            playlists: HashMap::new(),
            rigs: HashMap::new(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), config, None)).await;

    {
        let req = test::TestRequest::get().uri("/device/foo").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    {
        let req = test::TestRequest::get().uri("/playlist/foo").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    {
        let req = test::TestRequest::get().uri("/rig/foo").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}

#[test_log::test(actix_web::test)]
async fn when_devices_playlists_and_rigs_exist_it_should_execute_rigs() {
    let config = SlipwayServeConfig {
        log_level: Some("debug".to_string()),
        registry_urls: vec![],
        timezone: Some(Tz::Canada__Eastern),
        rig_permissions: HashMap::new(),
        repository: RepositoryConfig::Memory {
            devices: vec![(
                dn("d_1"),
                Device {
                    id: "mac:123".to_string(),
                    friendly_id: "abc".to_string(),
                    hashed_api_key: "xyz".to_string(),
                    name: dn("d_1"),
                    playlist: Some(pn("p_1")),
                    context: serde_json::json!({}),
                    reset_firmware: false,
                },
            )]
            .into_iter()
            .collect(),
            playlists: vec![(
                pn("p_1"),
                Playlist {
                    items: vec![PlaylistItem {
                        span: None,
                        days: None,
                        refresh: Refresh::Hours { hours: 1 },
                        rig: rn("r_1"),
                    }],
                },
            )]
            .into_iter()
            .collect(),
            rigs: vec![(
                rn("r_1"),
                slipway_engine::Rig::for_test(Rigging {
                    components: [(
                        "output".parse().unwrap(),
                        slipway_engine::ComponentRigging::for_test_with_reference(
                            slipway_engine::SlipwayReference::Special(
                                SpecialComponentReference::Pass,
                            ),
                            Some(serde_json::json!({
                                "foo": "bar"
                            })),
                        ),
                    )]
                    .into_iter()
                    .collect(),
                }),
            )]
            .into_iter()
            .collect(),
        },
    };

    let app = test::init_service(create_app(PathBuf::from("."), config, None)).await;

    async fn assert_response(response: ServiceResponse<impl MessageBody>) {
        let status = response.status();
        let refresh_rate: u32 = get_refresh_rate(&response);
        let body = get_body_json(response).await;

        assert_eq!(status, StatusCode::OK);
        assert!(refresh_rate > 3598 && refresh_rate < 3602);
        assert_eq!(body, serde_json::json!({ "foo": "bar"}));
    }

    {
        let request = test::TestRequest::get()
            .uri("/device/d_1?format=json")
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/playlist/p_1?format=json")
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_response(response).await;
    }

    {
        let request = test::TestRequest::get()
            .uri("/rig/r_1?format=json")
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_response(response).await;
    }
}
