use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{SLIPWAY_FETCH_COMPONENT_TAR_NAME, SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME};
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference,
};

mod common;

mod serial_tests {
    use std::io::Write;

    use super::*;

    #[common_macros::slipway_test_async]
    async fn file_text_wasm() {
        run("text", 200, SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
    }

    #[common_macros::slipway_test_async]
    async fn file_binary_wasm() {
        run("binary", 200, SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
    }

    #[common_macros::slipway_test_async]
    async fn file_text_error_status_code_wasm() {
        run("text", 404, SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
    }

    #[common_macros::slipway_test_async]
    async fn file_text_js() {
        run("text", 200, SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
    }

    #[common_macros::slipway_test_async]
    async fn file_binary_js() {
        run("binary", 200, SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
    }

    #[common_macros::slipway_test_async]
    async fn file_text_error_status_code_js() {
        run("text", 404, SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
    }

    const BODY: &str = "test_body💖";

    async fn run(file_type: &str, status_code: u16, component: &str) {
        let temp_dir = tempfile::tempdir().unwrap();

        let tmp_file_path = temp_dir.path().join("temp.file");
        let mut temp_file = std::fs::File::create(&tmp_file_path).unwrap();
        temp_file.write_all(BODY.as_bytes()).unwrap();

        let file_url = if status_code == 404 {
            format!("file:{}.other", tmp_file_path.to_string_lossy())
        } else {
            format!("file:{}", tmp_file_path.to_string_lossy())
        };

        let rig: Rig = Rig::for_test(Rigging {
            components: [(
                ComponentHandle::from_str("test").unwrap(),
                ComponentRigging::for_test_with_reference(
                    SlipwayReference::Local {
                        path: component.into(),
                    },
                    Some(json!({
                        "url": file_url,
                        "method": "GET",
                        "headers": {},
                        "body": "",
                        "response_type": file_type
                    })),
                ),
            )]
            .into_iter()
            .collect(),
        });

        let component_output = get_rig_output(rig, "test", Permissions::allow_all())
            .await
            .unwrap();

        let output = serde_json::from_value::<Output>(component_output.value.clone()).unwrap();

        assert_eq!(output.status_code, status_code);

        if status_code < 400 {
            if file_type == "text" {
                assert_eq!(output.body_text, Some(BODY.to_string()));
                assert!(output.body_bin.is_none());
            } else {
                assert!(output.body_text.is_none());
                assert_eq!(output.body_bin, Some(BODY.as_bytes().to_vec()));
            }
        }
    }

    #[derive(Deserialize)]
    struct Output {
        status_code: u16,
        body_text: Option<String>,
        body_bin: Option<Vec<u8>>,
    }
}
