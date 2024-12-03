use slipway_engine::ComponentRunner;

pub fn get_component_runners() -> Vec<Box<dyn ComponentRunner>> {
    vec![Box::new(slipway_wasmtime_runner::WasmComponentRunner {})]
}
