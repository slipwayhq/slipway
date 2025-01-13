use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::test_server::TestServer;
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
            ComponentRigging::for_test_with_reference_callout_override(
                SlipwayReference::Local {
                    path: "slipway.test.0.0.1.tar".into(),
                },
                Some(json!({
                    "type": "http",
                    "url": format!("{}foo/bar", localhost_url),
                    "method": "PUT",
                    "headers": {
                        "Content-Type": "application/json",
                        "Content-Length": BODY.len().to_string()
                    },
                    "expected_status_code": status_code,
                    "body": BODY,
                    "response_type": file_type
                })),
                "other",
                SlipwayReference::Local {
                    path: "slipway.test_json_schema.0.0.1.tar".into(),
                },
            ),
        )]
        .into_iter()
        .collect(),
    });

    let output = get_rig_output(rig, "test");
    test_server.stop();

    // Errors contain a binary body in the response, so for errors we'd get the binary size back.
    let expected_length = if file_type == "text" && status_code < 400 {
        BODY.len()
    } else {
        BODY.as_bytes().len()
    };

    assert_eq!(
        output.value,
        json!({
            "value": expected_length
        })
    );
}
