mod host;
mod run_component_wasm;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use run_component_wasm::WasmData;
pub use run_component_wasm::run_component_wasm;
use slipway_engine::{
    ComponentExecutionContext, ComponentFiles, ComponentRunner, RunComponentError,
    SlipwayReference, TryAotCompileComponentResult, TryRunComponentResult,
};
use slipway_host::{SLIPWAY_COMPONENT_WASM_FILE_NAME, hash_bytes};
use tracing::{debug, info, warn};
use wasmtime::{Config, Engine};

pub const WASMTIME_COMPONENT_RUNNER_IDENTIFIER: &str = "wasmtime";

pub struct WasmComponentRunner {
    engine: Engine,
}

fn create_engine(target: Option<&str>) -> anyhow::Result<Engine> {
    let mut config = Config::new();

    config.async_support(true);

    if let Some(target) = target {
        config.target(target)?;
    }

    Engine::new(&config)
}

impl WasmComponentRunner {
    pub fn new() -> Self {
        let engine =
            create_engine(None).expect("Should be able to create Wasmtime engine with no target");
        Self { engine }
    }
}

impl Default for WasmComponentRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl ComponentRunner for WasmComponentRunner {
    fn identifier(&self) -> String {
        WASMTIME_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    async fn aot_compile(
        &self,
        component_reference: &SlipwayReference,
        aot_path: &Path,
        target: Option<&str>,
        files: Arc<ComponentFiles>,
    ) -> Result<TryAotCompileComponentResult, RunComponentError> {
        let maybe_wasm_bytes = files.try_get_bin(SLIPWAY_COMPONENT_WASM_FILE_NAME).await?;

        let Some(wasm_bytes) = maybe_wasm_bytes else {
            return Ok(TryAotCompileComponentResult::CannotCompile);
        };

        let aot_bytes_path = get_aot_bytes_path(aot_path, &wasm_bytes);

        let target = target.map(|t| t.to_string());
        let aot_compiled_bytes = tokio::task::spawn_blocking(move || {
            let current_host = format!("{}", target_lexicon::Triple::host());
            let target = if let Some(target) = target {
                if target == current_host {
                    info!("Specified AOT target matches host: {target}");
                } else {
                    warn!(
                        "Specified AOT target \"{target}\" does not match host \"{current_host}\"."
                    );
                }
                target
            } else {
                info!("No AOT target specified, using host: {current_host}");

                // We explicitly set the current host, as passing in None does not seem to always produce
                // the expected result. In particular, on fly.io we had issues with the produced
                // AOT file not being runnable on the final host.
                current_host
            };

            let engine = create_engine(Some(&target))?;
            engine.precompile_component(&wasm_bytes)
        })
        .await
        .with_context(|| {
            format!(
                "Failed to AOT compile component \"{}\".",
                component_reference
            )
        })??;

        tokio::fs::write(aot_bytes_path, aot_compiled_bytes)
            .await
            .with_context(|| {
                format!(
                    "Failed to write AOT compiled file for component \"{}\".",
                    component_reference
                )
            })?;

        Ok(TryAotCompileComponentResult::Compiled)
    }

    async fn run<'call>(
        &self,
        input: &serde_json::Value,
        context: &'call ComponentExecutionContext<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let maybe_wasm_bytes = context
            .files
            .try_get_bin(SLIPWAY_COMPONENT_WASM_FILE_NAME)
            .await?;

        let Some(wasm_bytes) = maybe_wasm_bytes else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let wasm_data = if let Some(aot_path) = &context.rig_session_options.aot_path {
            let aot_bytes_path = get_aot_bytes_path(aot_path, &wasm_bytes);
            if tokio::fs::try_exists(aot_bytes_path.clone())
                .await
                .with_context(|| {
                    format!(
                        "Failed to check if AOT compiled file exists for WASM component: {}",
                        context.component_reference
                    )
                })?
            {
                let aot_bytes = tokio::fs::read(&aot_bytes_path).await.with_context(|| {
                    format!(
                        "Failed to read AOT compiled file for WASM component: {}",
                        context.component_reference
                    )
                })?;

                debug!(
                    "Using AOT compiled WASM component: {}",
                    context.component_reference
                );

                WasmData::Aot(aot_bytes)
            } else {
                warn!(
                    "AOT compiled file not found for WASM component: {}",
                    context.component_reference
                );
                WasmData::Wasm(Arc::clone(&wasm_bytes))
            }
        } else {
            debug!(
                "JIT compiling WASM component: {}",
                context.component_reference
            );
            WasmData::Wasm(Arc::clone(&wasm_bytes))
        };

        let run_result = run_component_wasm(input, wasm_data, &self.engine, context).await?;

        Ok(TryRunComponentResult::Ran { result: run_result })
    }
}

fn get_aot_bytes_path(aot_path: &Path, wasm_bytes: &[u8]) -> PathBuf {
    let wasm_bytes_hash = hash_bytes(wasm_bytes);
    aot_path.join(format!("{wasm_bytes_hash}.wasm_aot"))
}
