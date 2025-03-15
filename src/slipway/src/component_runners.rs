use slipway_engine::ComponentRunner;

pub fn get_component_runners() -> Vec<Box<dyn ComponentRunner>> {
    vec![
        Box::new(slipway_engine::SpecialComponentRunner {}),
        Box::new(slipway_fragment_runner::FragmentComponentRunner {}),
        Box::new(slipway_wasmtime_runner::WasmComponentRunner::new()),
        Box::new(slipway_js_boa_runner::BoaComponentRunner {}),
    ]
}
