use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::SLIPWAY_FONT_COMPONENT_TAR_NAME;
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, LocalComponentPermission, Permission, Permissions, Rig,
    Rigging, SlipwayReference, StringPermission,
};

mod common;

#[test]
fn permissions_load_fonts_no_component_permissions() {
    let rig = create_rig(Permissions::empty());

    let output = get_rig_output(rig, "test", Permissions::allow_all()).unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert_eq!(output.bin_length, 0);
}

#[test]
fn permissions_load_fonts_no_rig_permissions() {
    let rig = create_rig(Permissions::allow_all());

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![Permission::LocalComponent(
            LocalComponentPermission::Any,
        )]),
    )
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert_eq!(output.bin_length, 0);
}

#[test]
fn permissions_load_fonts_single_font_permission() {
    let rig = create_rig(Permissions::allow_all());

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any),
            Permission::Font(StringPermission::Exact {
                exact: "sans-serif".to_string(),
            }),
        ]),
    )
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.bin_length > 0);
}

#[test]
fn permissions_load_fonts_any_font_permissions() {
    let rig = create_rig(Permissions::allow_all());

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any),
            Permission::Font(StringPermission::Any {}),
        ]),
    )
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.bin_length > 0);
}

fn create_rig(component_permissions: Permissions) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_permissions(
                SlipwayReference::Local {
                    path: SLIPWAY_FONT_COMPONENT_TAR_NAME.into(),
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
