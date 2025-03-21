use std::str::FromStr;

use common::{assert_messages_contains, get_rig_output};
use common_test_utils::{
    SLIPWAY_FETCH_COMPONENT_TAR_NAME, SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME, test_server::TestServer,
};
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, LocalComponentPermission, Permission, Permissions, Rig,
    Rigging, RunComponentError, RunError, SlipwayReference, UrlPermission, utils::ch,
};

mod common;

#[common_macros::slipway_test_async]
async fn permissions_http_no_allow_wasm() {
    permissions_http_no_allow(SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_http_no_allow_js() {
    permissions_http_no_allow(SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_http_no_allow(component: &str) {
    run(
        Permissions::allow(&vec![Permission::LocalComponent(
            LocalComponentPermission::Any {},
        )]),
        component,
    )
    .await;
}

#[common_macros::slipway_test_async]
async fn permissions_http_deny_wasm() {
    permissions_http_deny(SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_http_deny_js() {
    permissions_http_deny(SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_http_deny(component: &str) {
    run(
        Permissions::new(
            &vec![Permission::All],
            &vec![Permission::Http(UrlPermission::Any {})],
        ),
        component,
    )
    .await;
}

async fn run(permissions: Permissions<'_>, component: &str) {
    let test_server = TestServer::start_for_call(
        "/foo/bar".to_string(),
        "GET".to_string(),
        vec![],
        "".to_string(),
        200,
    );

    let localhost_url = &test_server.localhost_url;

    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "url": format!("{}foo/bar", localhost_url),
                    "method": "GET",
                    "headers": {},
                    "body": "",
                    "response_type": "text"
                })),
            ),
        )]
        .into_iter()
        .collect(),
    });

    let maybe_output = get_rig_output(rig, "test", permissions).await;

    let Err(error) = maybe_output else {
        panic!("Expected error, got {:?}", maybe_output);
    };

    match error {
        RunError::RunComponentFailed {
            component_handle,
            component_runner: _,
            error: RunComponentError::RunCallReturnedError { message, inner },
        } => {
            assert_eq!(component_handle, ch("test"));
            assert_messages_contains(
                "Component \"test\" does not have permission to fetch url",
                &message,
                &inner,
            );
        }
        _ => panic!("Expected permission error, got {:?}", error),
    }

    test_server.stop();
}
