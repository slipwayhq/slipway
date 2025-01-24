use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::SLIPWAY_ENV_COMPONENT_TAR_NAME;
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, LocalComponentPermission, Permission, Permissions, Rig,
    Rigging, SlipwayReference, StringPermission,
};

mod common;

#[test]
fn permissions_load_env_no_component_permissions() {
    let rig = create_rig(Permissions::empty());

    let output = get_rig_output(rig, "test", Permissions::allow_all()).unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.value.is_none());
}

#[test]
fn permissions_load_env_no_rig_permissions() {
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

    assert!(output.value.is_none());
}

#[test]
fn permissions_load_env_other_env_permission() {
    let rig = create_rig(Permissions::allow_all());

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any),
            Permission::Env(StringPermission::Exact {
                exact: "ROAD".to_string(),
            }),
        ]),
    )
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    assert!(output.value.is_none());
}

#[test]
fn permissions_load_env_single_env_permission() {
    let rig = create_rig(Permissions::allow_all());

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any),
            Permission::Env(StringPermission::Exact {
                exact: "PATH".to_string(),
            }),
        ]),
    )
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    let Some(value) = output.value else {
        panic!("Expected value to be returned");
    };

    assert!(value.len() > 0);
}

#[test]
fn permissions_load_env_env_prefix_permission() {
    let rig = create_rig(Permissions::allow_all());

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any),
            Permission::Env(StringPermission::Prefix {
                prefix: "PA".to_string(),
            }),
        ]),
    )
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    let Some(value) = output.value else {
        panic!("Expected value to be returned");
    };

    assert!(value.len() > 0);
}

#[test]
fn permissions_load_env_any_env_permissions() {
    let rig = create_rig(Permissions::allow_all());

    let output = get_rig_output(
        rig,
        "test",
        Permissions::allow(&vec![
            Permission::LocalComponent(LocalComponentPermission::Any),
            Permission::Env(StringPermission::Any {}),
        ]),
    )
    .unwrap();

    let output: Output = serde_json::from_value(output.value.clone()).unwrap();

    let Some(value) = output.value else {
        panic!("Expected value to be returned");
    };

    assert!(value.len() > 0);
}

fn create_rig(component_permissions: Permissions) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_permissions(
                SlipwayReference::Local {
                    path: SLIPWAY_ENV_COMPONENT_TAR_NAME.into(),
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
