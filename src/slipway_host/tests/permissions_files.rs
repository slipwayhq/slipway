use std::str::FromStr;

use common::{assert_messages_contains, get_rig_output};
use common_test_utils::{SLIPWAY_FETCH_COMPONENT_TAR_NAME, SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME};
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, LocalComponentPermission, Permission, Permissions, Rig,
    Rigging, RunComponentError, RunError, SlipwayReference, utils::ch,
};

mod common;

use slipway_engine::PathPermission;

#[common_macros::slipway_test_async]
async fn permissions_file_no_allow_wasm() {
    permissions_file_no_allow(SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_file_no_allow_js() {
    permissions_file_no_allow(SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_file_no_allow(component: &str) {
    run(
        Permissions::allow(&vec![Permission::LocalComponent(
            LocalComponentPermission::Any {},
        )]),
        component,
    )
    .await;
}

#[common_macros::slipway_test_async]
async fn permissions_file_deny_wasm() {
    permissions_file_deny(SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_file_deny_js() {
    permissions_file_deny(SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_file_deny(component: &str) {
    run(
        Permissions::new(
            &vec![Permission::All],
            &vec![Permission::File(PathPermission::Any {})],
        ),
        component,
    )
    .await;
}

async fn run(permissions: Permissions<'_>, component: &str) {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "url": "file:foo/bar.txt",
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
                "Component \"test\" does not have permission to fetch file",
                &message,
                &inner,
            );
        }
        _ => panic!("Expected permission error, got {:?}", error),
    }
}
