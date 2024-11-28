use slipway_engine::RunMetadata;

pub mod fonts;

pub mod run;

pub const SLIPWAY_COMPONENT_WASM_FILE_NAME: &str = "slipway_component.wasm";

pub struct RunComponentResult {
    pub output: serde_json::Value,
    pub metadata: RunMetadata,
}
