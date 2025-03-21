use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{SLIPWAY_FONT_COMPONENT_TAR_NAME, SLIPWAY_FONT_JS_COMPONENT_TAR_NAME};
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, LocalComponentPermission, Permission, Permissions, Rig,
    Rigging, SlipwayReference, StringPermission,
};

mod common;

#[common_macros::slipway_test_async]
async fn permissions_load_fonts_no_component_permissions_wasm() {
    permissions_load_fonts_no_component_permissions(SLIPWAY_FONT_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_fonts_no_component_permissions_js() {
    permissions_load_fonts_no_component_permissions(SLIPWAY_FONT_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_fonts_no_component_permissions(component: &str) {
    let rig = create_rig(Permissions::empty(), component);

    let output = get_rig_output(rig, "test", Permissions::allow_all())
        .await
        .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert_eq!(output.bin_length, 0);
}

#[common_macros::slipway_test_async]
async fn permissions_load_fonts_no_rig_permissions_wasm() {
    permissions_load_fonts_no_rig_permissions(SLIPWAY_FONT_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_fonts_no_rig_permissions_js() {
    permissions_load_fonts_no_rig_permissions(SLIPWAY_FONT_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_fonts_no_rig_permissions(component: &str) {
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

    assert_eq!(output.bin_length, 0);
}

#[common_macros::slipway_test_async]
async fn permissions_load_fonts_single_font_permission_wasm() {
    permissions_load_fonts_single_font_permission(SLIPWAY_FONT_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_fonts_single_font_permission_js() {
    permissions_load_fonts_single_font_permission(SLIPWAY_FONT_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_fonts_single_font_permission(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any {}),
            Permission::Font(StringPermission::Exact {
                exact: "sans-serif".to_string(),
            }),
        ]),
    )
    .await
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.bin_length > 0);
}

#[common_macros::slipway_test_async]
async fn permissions_load_fonts_any_font_permissions_wasm() {
    permissions_load_fonts_any_font_permissions(SLIPWAY_FONT_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_fonts_any_font_permissions_js() {
    permissions_load_fonts_any_font_permissions(SLIPWAY_FONT_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_fonts_any_font_permissions(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any {}),
            Permission::Font(StringPermission::Any {}),
        ]),
    )
    .await
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.bin_length > 0);
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
                    "font_stack": "Hack, sans-serif"
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
    bin_length: u32,
}
