use core::panic;
use std::{str::FromStr, sync::Arc};

use common_test_utils::{
    get_slipway_test_components_path, get_slipway_test_components_registry_url,
};
use slipway_engine::{
    BasicComponentCache, BasicComponentsLoader, BasicComponentsLoaderBuilder, CallChain,
    ComponentHandle, ComponentOutput, ComponentRunner, Permissions, Rig, RigSession, RunError,
};
use slipway_host::run::{no_event_handler, run_rig};

pub fn get_component_runners() -> Vec<Box<dyn ComponentRunner>> {
    vec![
        Box::new(slipway_engine::SpecialComponentRunner {}),
        Box::new(slipway_js_boa_runner::BoaComponentRunner {}),
        Box::new(slipway_wasmtime_runner::WasmComponentRunner::new()),
        Box::new(slipway_fragment_runner::FragmentComponentRunner {}),
    ]
}

pub fn create_components_loader() -> BasicComponentsLoader {
    BasicComponentsLoaderBuilder::new()
        .registry_lookup_url(&get_slipway_test_components_registry_url())
        .local_base_directory(&get_slipway_test_components_path())
        .build()
}

#[allow(dead_code)]
pub async fn get_rig_output(
    rig: Rig,
    output_handle_str: &str,
    permissions: Permissions<'_>,
) -> Result<Arc<ComponentOutput>, RunError<()>> {
    let component_cache = BasicComponentCache::primed(&rig, &create_components_loader())
        .await
        .unwrap();
    let component_runners = get_component_runners();
    let call_chain = Arc::new(CallChain::new(permissions));
    let session = RigSession::new_for_test(rig, &component_cache);

    let result = run_rig(
        &session,
        &mut no_event_handler(),
        &component_runners,
        call_chain,
    )
    .await?;

    let output = result
        .component_states
        .get(&ComponentHandle::from_str(output_handle_str).unwrap())
        .expect("Output handle should exist")
        .execution_output
        .as_ref()
        .expect("Output should be populated");

    Ok(Arc::clone(output))
}

#[allow(dead_code)]
pub fn assert_messages_contains(expected: &str, message: &str, inner: &[String]) {
    let mut found = false;
    println!("Message: {}", message);
    if message.contains(expected) {
        found = true;
    } else {
        for i in inner {
            println!("Inner: {}", i);
            if i.contains(expected) {
                found = true;
                break;
            }
        }
    }
    if !found {
        panic!("Expected message to contain \"{}\"", expected);
    }
}
