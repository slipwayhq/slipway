use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    future::Future,
    ops::DerefMut,
    pin::Pin,
};

use boa_engine::{
    Context, JsResult,
    context::time::{JsDuration, JsInstant},
    job::{Job, JobExecutor, NativeAsyncJob, PromiseJob, TimeoutJob},
};
use futures_concurrency::future::FutureGroup;
use futures_lite::StreamExt;

// https://github.com/boa-dev/boa/blob/main/examples/src/bin/tokio_event_loop.rs

/// An event queue using tokio to drive futures to completion.
pub(super) struct Queue {
    async_jobs: RefCell<VecDeque<NativeAsyncJob>>,
    promise_jobs: RefCell<VecDeque<PromiseJob>>,
    timeout_jobs: RefCell<BTreeMap<JsInstant, TimeoutJob>>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            async_jobs: RefCell::default(),
            promise_jobs: RefCell::default(),
            timeout_jobs: RefCell::default(),
        }
    }

    fn drain_timeout_jobs(&self, context: &mut Context) {
        let now = context.clock().now();

        let mut timeouts_borrow = self.timeout_jobs.borrow_mut();
        // `split_off` returns the jobs after (or equal to) the key. So we need to add 1ms to
        // the current time to get the jobs that are due, then swap with the inner timeout
        // tree so that we get the jobs to actually run.
        let jobs_to_keep = timeouts_borrow.split_off(&(now + JsDuration::from_millis(1)));
        let jobs_to_run = std::mem::replace(timeouts_borrow.deref_mut(), jobs_to_keep);
        drop(timeouts_borrow);

        for job in jobs_to_run.into_values() {
            if let Err(e) = job.call(context) {
                eprintln!("Uncaught {e}");
            }
        }
    }

    fn drain_jobs(&self, context: &mut Context) {
        // Run the timeout jobs first.
        self.drain_timeout_jobs(context);

        let jobs = std::mem::take(&mut *self.promise_jobs.borrow_mut());
        for job in jobs {
            if let Err(e) = job.call(context) {
                eprintln!("Uncaught {e}");
            }
        }
    }
}

impl JobExecutor for Queue {
    fn enqueue_job(&self, job: Job, context: &mut Context) {
        match job {
            Job::PromiseJob(job) => self.promise_jobs.borrow_mut().push_back(job),
            Job::AsyncJob(job) => self.async_jobs.borrow_mut().push_back(job),
            Job::TimeoutJob(t) => {
                let now = context.clock().now();
                self.timeout_jobs.borrow_mut().insert(now + t.timeout(), t);
            }
            _ => panic!("unsupported job type"),
        }
    }

    // While the sync flavor of `run_jobs` will block the current thread until all the jobs have finished...
    fn run_jobs(&self, context: &mut Context) -> JsResult<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        tokio::task::LocalSet::default()
            .block_on(&runtime, self.run_jobs_async(&RefCell::new(context)))
    }

    // ...the async flavor won't, which allows concurrent execution with external async tasks.
    fn run_jobs_async<'a, 'b, 'fut>(
        &'a self,
        context: &'b RefCell<&mut Context>,
    ) -> Pin<Box<dyn Future<Output = JsResult<()>> + 'fut>>
    where
        'a: 'fut,
        'b: 'fut,
    {
        Box::pin(async move {
            // Early return in case there were no jobs scheduled.
            if self.promise_jobs.borrow().is_empty() && self.async_jobs.borrow().is_empty() {
                return Ok(());
            }
            let mut group = FutureGroup::new();
            loop {
                for job in std::mem::take(&mut *self.async_jobs.borrow_mut()) {
                    group.insert(job.call(context));
                }

                if self.promise_jobs.borrow().is_empty() {
                    let Some(result) = group.next().await else {
                        // Both queues are empty. We can exit.
                        return Ok(());
                    };

                    if let Err(err) = result {
                        eprintln!("Uncaught {err}");
                    }

                    continue;
                }

                // We have some jobs pending on the microtask queue. Try to poll the pending
                // tasks once to see if any of them finished, and run the pending microtasks
                // otherwise.
                let Some(result) = futures_lite::future::poll_once(group.next())
                    .await
                    .flatten()
                else {
                    // No completed jobs. Run the microtask queue once.
                    self.drain_jobs(&mut context.borrow_mut());

                    tokio::task::yield_now().await;
                    continue;
                };

                if let Err(err) = result {
                    eprintln!("Uncaught {err}");
                }

                // Only one macrotask can be executed before the next drain of the microtask queue.
                self.drain_jobs(&mut context.borrow_mut());
            }
        })
    }
}
