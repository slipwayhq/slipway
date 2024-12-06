use std::str::FromStr;

use slipway_engine::{
    BasicComponentCache, ComponentHandle, ComponentRigging, PermissionChain, Rig, RigSession,
    Rigging, SlipwayReference,
};
use slipway_host::run::{no_event_handler, run_rig};

use common::{create_components_loader, get_component_runners};
use serde_json::json;

mod common;

#[test]
fn test_callout_panic() {
    let rig = create_rig(0, "panic");

    let component_cache = BasicComponentCache::primed(&rig, &create_components_loader()).unwrap();
    let component_runners = get_component_runners();
    let permission_chain = PermissionChain::full_trust_arc();
    let session = RigSession::new(rig, &component_cache);

    let result = run_rig(
        &session,
        &mut no_event_handler(),
        &component_runners,
        permission_chain,
    );

    let Err(error) = result else {
        panic!("Expected error");
    };

    print!("Result: {}", error);
    println!();
    panic!("poop");
}

fn create_rig(ttl: u32, result_type: &str) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: "slipway.test.0.0.1.tar".into(),
                },
                Some(json!({
                    "type": "callout_increment",
                    "value": 0,
                    "ttl": ttl,
                    "result_type": result_type
                })),
            ),
        )]
        .into_iter()
        .collect(),
    })
}
