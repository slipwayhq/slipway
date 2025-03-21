use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{SLIPWAY_ENV_COMPONENT_TAR_NAME, SLIPWAY_ENV_JS_COMPONENT_TAR_NAME};
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, LocalComponentPermission, Permission, Permissions, Rig,
    Rigging, SlipwayReference, StringPermission,
};

mod common;

#[common_macros::slipway_test_async]
async fn permissions_load_env_no_component_permissions_wasm() {
    permissions_load_env_no_component_permissions(SLIPWAY_ENV_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_env_no_component_permissions_js() {
    permissions_load_env_no_component_permissions(SLIPWAY_ENV_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_env_no_component_permissions(component: &str) {
    let rig = create_rig(Permissions::empty(), component);

    let output = get_rig_output(rig, "test", Permissions::allow_all())
        .await
        .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.value.is_none());
}

#[test_log::test(tokio::test(flavor = "current_thread"))]
async fn permissions_load_env_no_rig_permissions_wasm() {
    permissions_load_env_no_rig_permissions(SLIPWAY_ENV_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_env_no_rig_permissions_js() {
    permissions_load_env_no_rig_permissions(SLIPWAY_ENV_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_env_no_rig_permissions(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![Permission::LocalComponent(
            LocalComponentPermission::Any {},
        )]),
    )
    .await
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.value.is_none());
}

#[common_macros::slipway_test_async]
async fn permissions_load_env_other_env_permission_wasm() {
    permissions_load_env_other_env_permission(SLIPWAY_ENV_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_env_other_env_permission_js() {
    permissions_load_env_other_env_permission(SLIPWAY_ENV_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_env_other_env_permission(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any {}),
            Permission::Env(StringPermission::Exact {
                exact: "ROAD".to_string(),
            }),
        ]),
    )
    .await
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.value.is_none());
}

#[common_macros::slipway_test_async]
async fn permissions_load_env_single_env_permission_wasm() {
    permissions_load_env_single_env_permission(SLIPWAY_ENV_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_env_single_env_permission_js() {
    permissions_load_env_single_env_permission(SLIPWAY_ENV_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_env_single_env_permission(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any {}),
            Permission::Env(StringPermission::Exact {
                exact: "PATH".to_string(),
            }),
        ]),
    )
    .await
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    let Some(value) = output.value else {
        panic!("Expected value to be returned");
    };

    assert!(!value.is_empty());
}

#[common_macros::slipway_test_async]
async fn permissions_load_env_env_prefix_permission_wasm() {
    permissions_load_env_env_prefix_permission(SLIPWAY_ENV_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_env_env_prefix_permission_js() {
    permissions_load_env_env_prefix_permission(SLIPWAY_ENV_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_env_env_prefix_permission(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any {}),
            Permission::Env(StringPermission::Prefix {
                prefix: "PA".to_string(),
            }),
        ]),
    )
    .await
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    let Some(value) = output.value else {
        panic!("Expected value to be returned");
    };

    assert!(!value.is_empty());
}

#[common_macros::slipway_test_async]
async fn permissions_load_env_any_env_permissions_wasm() {
    permissions_load_env_any_env_permissions(SLIPWAY_ENV_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_env_any_env_permissions_js() {
    permissions_load_env_any_env_permissions(SLIPWAY_ENV_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_env_any_env_permissions(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any {}),
            Permission::Env(StringPermission::Any {}),
        ]),
    )
    .await
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    let Some(value) = output.value else {
        panic!("Expected value to be returned");
    };

    assert!(!value.is_empty());
}

fn create_rig(component_permissions: Permissions, component: &str) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_permissions(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "key": "PATH"
                })),
                component_permissions,
            ),
        )]
        .into_iter()
        .collect(),
    })
}

#[derive(Deserialize)]
struct Output {
    value: Option<String>,
}
