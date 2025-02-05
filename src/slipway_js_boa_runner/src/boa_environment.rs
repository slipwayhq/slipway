use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use boa_runtime::{ConsoleState, Logger, RegisterOptions};

use boa_engine::{
    context::ContextBuilder,
    job::{Job, JobExecutor, NativeAsyncJob, PromiseJob},
    module::SimpleModuleLoader,
    optimizer::OptimizerOptions,
    Context, JsResult, JsString, Source,
};
use slipway_engine::RunComponentError;
use tracing::{debug, error, info, trace, warn};

pub(super) fn prepare_environment() -> Result<Context, RunComponentError> {
    let executor = Rc::new(Executor::default());
    let loader = Rc::new(SimpleModuleLoader::new(".").map_err(|e| anyhow::anyhow!(e.to_string()))?);

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

    context
        .eval(Source::from_bytes(indoc::indoc! {r#"
            global= {};
            setTimeout = () => {};
            clearTimeout = () => {};
            "#}))
        .map_err(|e| {
            RunComponentError::Other(format!("Failed to prepare javascript environment.\n{}", e))
        })?;

    Ok(context)
}

#[derive(Debug, boa_macros::Trace, boa_macros::Finalize)]
struct BoaConsoleLogger;

impl Logger for BoaConsoleLogger {
    fn trace(&self, msg: String, state: &ConsoleState, context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        trace!("{msg:>indent$}");

        let stack_trace_dump = context
            .stack_trace()
            .map(|frame| frame.code_block().name())
            .filter(|name| !name.is_empty()) // The last frame has an empty name.
            .collect::<Vec<_>>()
            .into_iter()
            .map(JsString::to_std_string_escaped)
            .collect::<Vec<_>>();

        for frame in stack_trace_dump {
            trace!(" {frame:>indent$}");
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

// #[derive(Default)]
// pub struct SimpleJobQueue(RefCell<VecDeque<NativeJob>>);

// impl std::fmt::Debug for SimpleJobQueue {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_tuple("SimpleQueue").field(&"..").finish()
//     }
// }

// impl SimpleJobQueue {
//     /// Creates an empty `SimpleJobQueue`.
//     #[must_use]
//     pub fn new() -> Self {
//         Self::default()
//     }
// }

// impl JobQueue for SimpleJobQueue {
//     fn enqueue_promise_job(&self, job: NativeJob, _: &mut Context) {
//         self.0.borrow_mut().push_back(job);
//     }

//     fn run_jobs(&self, context: &mut Context) {
//         let mut next_job = self.0.borrow_mut().pop_front();
//         while let Some(job) = next_job {
//             if job.call(context).is_err() {
//                 self.0.borrow_mut().clear();
//                 return;
//             };
//             next_job = self.0.borrow_mut().pop_front();
//         }
//     }

//     fn enqueue_future_job(&self, future: FutureJob, context: &mut Context) {
//         let job = pollster::block_on(future);
//         self.enqueue_promise_job(job, context);
//     }
// }

#[derive(Default)]
struct Executor {
    promise_jobs: RefCell<VecDeque<PromiseJob>>,
    async_jobs: RefCell<VecDeque<NativeAsyncJob>>,
}

impl JobExecutor for Executor {
    fn enqueue_job(&self, job: Job, _: &mut Context) {
        match job {
            Job::PromiseJob(job) => self.promise_jobs.borrow_mut().push_back(job),
            Job::AsyncJob(job) => self.async_jobs.borrow_mut().push_back(job),
            job => eprintln!("unsupported job type {job:?}"),
        }
    }

    fn run_jobs(&self, context: &mut Context) -> JsResult<()> {
        loop {
            if self.promise_jobs.borrow().is_empty() && self.async_jobs.borrow().is_empty() {
                return Ok(());
            }

            let jobs = std::mem::take(&mut *self.promise_jobs.borrow_mut());
            for job in jobs {
                if let Err(e) = job.call(context) {
                    eprintln!("Uncaught {e}");
                }
            }

            let async_jobs = std::mem::take(&mut *self.async_jobs.borrow_mut());
            for async_job in async_jobs {
                if let Err(err) = pollster::block_on(async_job.call(&RefCell::new(context))) {
                    eprintln!("Uncaught {err}");
                }
                let jobs = std::mem::take(&mut *self.promise_jobs.borrow_mut());
                for job in jobs {
                    if let Err(e) = job.call(context) {
                        eprintln!("Uncaught {e}");
                    }
                }
            }
        }
    }
}
