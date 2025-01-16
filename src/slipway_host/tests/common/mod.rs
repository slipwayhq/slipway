use std::{rc::Rc, str::FromStr};

use common_test_utils::get_slipway_test_components_path;
use slipway_engine::{
    BasicComponentCache, BasicComponentsLoader, BasicComponentsLoaderBuilder, CallChain,
    ComponentHandle, ComponentOutput, ComponentRunner, Rig, RigSession,
};
use slipway_host::run::{no_event_handler, run_rig};

pub fn get_component_runners() -> Vec<Box<dyn ComponentRunner>> {
    vec![
        Box::new(slipway_engine::SpecialComponentRunner {}),
        Box::new(slipway_fragment_runner::FragmentComponentRunner {}),
        Box::new(slipway_wasmtime_runner::WasmComponentRunner {}),
    ]
}

pub fn create_components_loader() -> BasicComponentsLoader {
    BasicComponentsLoaderBuilder::new()
        .local_base_directory(&get_slipway_test_components_path())
        .build()
}

#[allow(dead_code)]
pub fn get_rig_output(rig: Rig, output_handle_str: &str) -> Rc<ComponentOutput> {
    let component_cache = BasicComponentCache::primed(&rig, &create_components_loader()).unwrap();
    let component_runners = get_component_runners();
    let call_chain = CallChain::full_trust_arc();
    let session = RigSession::new(rig, &component_cache);

    let result = run_rig(
        &session,
        &mut no_event_handler(),
        &component_runners,
        call_chain,
    )
    .unwrap();

    let output = result
        .component_outputs
        .get(&ComponentHandle::from_str(output_handle_str).unwrap())
        .expect("Output handle should exist")
        .as_ref()
        .expect("Output should be populated");

    Rc::clone(output)
}
