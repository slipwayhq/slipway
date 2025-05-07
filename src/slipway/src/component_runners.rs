use slipway_engine::ComponentRunner;

/// Returns the list of component runners in the order they will be tried.
/// The order is important because if multiple runners can run a component
/// then they will be executed in order with the output of one being
/// fed into the next one.
/// Changing the order of the runners will potentially break components which
/// rely on the order of execution, so should not be done lightly.
pub fn get_component_runners() -> Vec<Box<dyn ComponentRunner>> {
    vec![
        Box::new(slipway_engine::SpecialComponentRunner {}),
        Box::new(slipway_js_boa_runner::BoaComponentRunner {}),
        Box::new(slipway_wasmtime_runner::WasmComponentRunner::new()),
        Box::new(slipway_fragment_runner::FragmentComponentRunner {}),
    ]
}
