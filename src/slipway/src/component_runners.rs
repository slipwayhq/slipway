use slipway_host::run::ComponentRunner;

pub fn get_component_runners<'rig>() -> Vec<Box<dyn ComponentRunner<'rig>>> {
    vec![Box::new(slipway_wasmtime_runner::WasmComponentRunner {})]
}
