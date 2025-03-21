use std::{rc::Rc, sync::Arc};

use boa_runtime::{ConsoleState, Logger, RegisterOptions};

use boa_engine::{
    Context, JsResult, JsString, Source, context::ContextBuilder, optimizer::OptimizerOptions,
};
use slipway_engine::{ComponentFiles, RunComponentError};
use tracing::{debug, error, info, trace, warn};

use crate::component_module_loader::ComponentModuleLoader;

const POLYFILLS: &str = include_str!("polyfills.js");

pub(super) fn prepare_environment(
    files: Arc<ComponentFiles>,
) -> Result<Context, RunComponentError> {
    let executor = Rc::new(super::async_environment::Queue::new());
    // let loader = Rc::new(SimpleModuleLoader::new(".").map_err(|e| anyhow::anyhow!(e.to_string()))?);
    let loader =
        Rc::new(ComponentModuleLoader::new(files).map_err(|e| anyhow::anyhow!(e.to_string()))?);

    let mut context = ContextBuilder::new()
        .job_executor(executor)
        .module_loader(loader.clone())
        .build()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    context.strict(false);

    let options = RegisterOptions::new().with_console_logger(BoaConsoleLogger {});
    boa_runtime::register(&mut context, options)
        .expect("should not fail while registering the runtime");

    let mut optimizer_options = OptimizerOptions::empty();
    optimizer_options.set(OptimizerOptions::STATISTICS, false);
    optimizer_options.set(OptimizerOptions::OPTIMIZE_ALL, true);
    context.set_optimizer_options(optimizer_options);

    context.eval(Source::from_bytes(POLYFILLS)).map_err(|e| {
        RunComponentError::Other(format!(
            "Failed to prepare javascript environment polyfills.\n{}",
            e
        ))
    })?;

    Ok(context)
}

#[derive(Debug, boa_macros::Trace, boa_macros::Finalize)]
struct BoaConsoleLogger;

impl Logger for BoaConsoleLogger {
    fn trace(&self, msg: String, state: &ConsoleState, context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        trace!("{msg:>indent$}");

        let stack_trace = context
            .stack_trace()
            .map(|frame| frame.code_block().name())
            .map(JsString::to_std_string_escaped)
            .collect::<Vec<_>>();

        let mut is_first = true;
        for frame in stack_trace {
            if is_first {
                is_first = false;
                trace!(" at {frame:>indent$}");
            } else {
                trace!("    {frame:>indent$}");
            }
        }

        Ok(())
    }

    fn debug(
        &self,
        msg: String,
        state: &ConsoleState,
        _context: &mut Context,
    ) -> boa_engine::JsResult<()> {
        let indent = state.indent();
        debug!("{msg:>indent$}");
        Ok(())
    }

    #[inline]
    fn log(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        info!("{msg:>indent$}");
        Ok(())
    }

    #[inline]
    fn info(&self, msg: String, state: &ConsoleState, context: &mut Context) -> JsResult<()> {
        self.log(msg, state, context)
    }

    #[inline]
    fn warn(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        warn!("{msg:>indent$}");
        Ok(())
    }

    #[inline]
    fn error(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        error!("{msg:>indent$}");
        Ok(())
    }
}
