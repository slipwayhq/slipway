use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{test_server::TestServer, SLIPWAY_FETCH_COMPONENT_TAR_NAME};
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{ComponentHandle, ComponentRigging, Rig, Rigging, SlipwayReference};

mod common;

#[test]
fn http_text() {
    run("text", 200);
}

#[test]
fn http_binary() {
    run("binary", 200);
}

#[test]
fn http_text_error_status_code() {
    run("text", 500);
}

const BODY: &str = "test_body💖";

fn run(file_type: &str, status_code: u16) {
    let test_server = TestServer::start_for_call(
        "/foo/bar".to_string(),
        "PUT".to_string(),
        vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Content-Length".to_string(), BODY.len().to_string()),
        ],
        BODY.to_string(),
        status_code,
    );

    let localhost_url = &test_server.localhost_url;

    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: SLIPWAY_FETCH_COMPONENT_TAR_NAME.into(),
                },
                Some(json!({
                    "url": format!("{}foo/bar", localhost_url),
                    "method": "PUT",
                    "headers": {
                        "Content-Type": "application/json",
                        "Content-Length": BODY.len().to_string()
                    },
                    "body": BODY,
                    "response_type": file_type
                })),
            ),
        )]
        .into_iter()
        .collect(),
    });

    let component_output = get_rig_output(rig, "test");
    test_server.stop();

    let output = serde_json::from_value::<Output>(component_output.value.clone()).unwrap();

    assert_eq!(output.status_code, status_code);

    // Errors contain a binary body in the response, so for errors we'd get the binary size back.
    if file_type == "text" && status_code < 400 {
        assert_eq!(output.body_text, Some(BODY.to_string()));
        assert!(output.body_bin.is_none());
    } else {
        assert!(output.body_text.is_none());
        assert_eq!(output.body_bin, Some(BODY.as_bytes().to_vec()));
    };
}

#[derive(Deserialize)]
struct Output {
    status_code: u16,
    body_text: Option<String>,
    body_bin: Option<Vec<u8>>,
}
