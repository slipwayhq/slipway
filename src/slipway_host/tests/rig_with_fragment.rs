use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::SLIPWAY_FRAGMENT_COMPONENT_TAR_NAME;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference,
};

mod common;

#[common_macros::slipway_test_async]
async fn run() {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("frag").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: SLIPWAY_FRAGMENT_COMPONENT_TAR_NAME.into(),
                },
                Some(json!({
                    "type": "increment",
                    "value": 0
                })),
            ),
        )]
        .into_iter()
        .collect(),
    });

    let output = get_rig_output(rig, "frag", Permissions::allow_all())
        .await
        .unwrap();

    assert_eq!(
        output.value,
        json!({
            "value": 2
        })
    );
}
