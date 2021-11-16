use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::mem;
use std::os::unix::raw::time_t;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use deno_core::{JsRuntime, OpState, RuntimeOptions, Snapshot};
use deno_core::v8::{CreateParams, IsolateHandle};
use deno_core::error::{AnyError, generic_error};
use futures::task::{Waker};
use futures_util::task::{ArcWake, waker, waker_ref};
use tokio::sync::oneshot;

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/RUNTIME_SNAPSHOT.bin"));
const DEFAULT_SOFT_HEAP_LIMIT: usize = 1 << 20;

#[derive(Default)]
pub struct ExecutionResourceTable {
    pub execution_time_limit: Option<Duration>,
    pub cpu_time_limit: Option<Duration>,
    pub resource_requests_limit: Option<u32>,

    // when the execution has started
    pub started_at: Option<Instant>,
    // when the current poll of the loop started
    // only set when the loop is currently being polled aka the CPU is doing work
    pub current_wakeup: Option<Instant>,
    // The time the cpu has spent executing previous wakeups
    // does not include the current wakeup if it's still running (current_wakeup is set)
    pub cpu_time: Duration,

    // the count of resource requests that have been made
    pub resource_requests_count: u32,
}

struct ExecutionPollState {
    complete: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl ExecutionPollState {
    pub fn new() -> Self {
        Self {
            complete: AtomicBool::new(false),
            waker: Mutex::new(None),
        }
    }
}

struct ExecutionPollFuture {
    state: Arc<ExecutionPollState>,
}

struct ExecutionPollWaker {
    state: Arc<ExecutionPollState>,
}

impl ArcWake for ExecutionPollWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.state.complete.store(true, Ordering::SeqCst);
        let mut waker_guard = arc_self.state.waker.lock().unwrap();
        let waker_option = mem::take(&mut *waker_guard);
        if let Some(waker) = waker_option {
            *waker_guard = Some(waker.clone());
            waker.wake();
        } else {
            *waker_guard = None
        }
    }
}

impl Future for ExecutionPollFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.state.complete.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            let mut waker_guard = self.state.waker.lock().unwrap();
            *waker_guard = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// information about the runtime that are SEND
pub struct SharedRuntimeState {
    pub resource_table: Mutex<ExecutionResourceTable>,
    pub isolate_handle: Mutex<Option<IsolateHandle>>,
}

pub struct WrappedRuntime {
    pub state: Arc<SharedRuntimeState>,
    soft_heap_limit: usize,
    hard_heap_limit: Option<usize>,

    runtime: Option<JsRuntime>,
}

impl WrappedRuntime {
    pub fn new() -> Self {
        return Self {
            state: Arc::new(SharedRuntimeState {
                resource_table: Mutex::new(ExecutionResourceTable::default()),
                isolate_handle: Mutex::new(None),
            }),
            soft_heap_limit: DEFAULT_SOFT_HEAP_LIMIT,
            hard_heap_limit: None,
            runtime: None,
        };
    }

    pub fn resource_table(&self) -> MutexGuard<ExecutionResourceTable> {
        self.state.resource_table.lock().unwrap()
    }

    pub fn op_state(&mut self) -> Rc<RefCell<OpState>> {
        self.runtime.as_mut().unwrap().op_state()
    }

    pub fn create_runtime(&mut self) {
        let snapshot = Snapshot::Static(RUNTIME_SNAPSHOT);

        let create_params = CreateParams::default()
            .heap_limits(0, self.soft_heap_limit);

        let extensions = vec![
            ext_resources::init()
        ];

        let mut runtime = JsRuntime::new(RuntimeOptions {
            startup_snapshot: Some(snapshot),
            create_params: Some(create_params),
            extensions,
            ..Default::default()
        });

        let isolate_handle = runtime.v8_isolate().thread_safe_handle();
        let hard_heap_limit = self.hard_heap_limit;
        runtime.add_near_heap_limit_callback(move |current: usize, initial: usize| -> usize {
            // soft heap limit reached -> terminate
            isolate_handle.terminate_execution();

            if let Some(hard_limit) = hard_heap_limit {
                if current >= hard_limit {
                    // hard heap limit reached -> don't increase -> V8 will terminate the process
                    return current;
                }
            }

            current + initial
        });

        self.runtime = Some(runtime)
    }

    pub fn prepare_runtime(&mut self) {
        /* let runtime = self.runtime.as_mut().unwrap();
        runtime.execute_script(
            "<cleanup>",
            r#"
            __bootstrapRuntime();
            delete __boostrapRuntime;
        "#,
        ).unwrap(); */
    }

    fn prepare_wakeup(&mut self) -> Result<(), AnyError> {
        let resource_table = &mut *self.resource_table();

        if let Some(started_at) = resource_table.started_at {
            if let Some(execution_time_limit) = resource_table.execution_time_limit {
                // this avoids starting another wakeup when it has already ran out of execution time
                if started_at.elapsed() > execution_time_limit {
                    return Err(generic_error("Isolate has run out of execution time"));
                }
            }
        }

        if let Some(cpu_time_limit) = resource_table.cpu_time_limit {
            if resource_table.cpu_time > cpu_time_limit {
                // this avoids starting another wakeup when it has already ran out of cpu time
                return Err(generic_error("Isolate has run out of CPU time"));
            }
        }

        let new_wakeup = Instant::now();
        resource_table.current_wakeup = Some(new_wakeup);

        Ok(())
    }

    fn cleanup_wakeup(&mut self) {
        let resource_table = &mut *self.resource_table();

        if let Some(current_wakeup) = resource_table.current_wakeup {
            resource_table.cpu_time = resource_table.cpu_time.saturating_add(current_wakeup.elapsed());
            resource_table.current_wakeup = None
        }
    }

    async fn poll_and_wait(&mut self) -> Option<Result<(), AnyError>> {
        let poll_state = Arc::new(ExecutionPollState::new());

        let mut future = ExecutionPollFuture { state: poll_state.clone() };

        let waker = ExecutionPollWaker { state: poll_state };
        let waker_arc = Arc::new(waker);
        let waker_r = waker_ref(&waker_arc);
        let mut context = Context::from_waker(&*waker_r);

        if let Err(e) = self.prepare_wakeup() {
            return Some(Err(e));
        }

        let runtime = self.runtime.as_mut().unwrap();
        let poll = runtime.poll_event_loop(&mut context, false);

        self.cleanup_wakeup();

        match poll {
            Poll::Pending => {
                (&mut future).await;
            }
            Poll::Ready(r) => return Some(r)
        };

        None
    }

    async fn drive_execution(&mut self, script: String) -> Result<(), Box<dyn std::error::Error>> {
        self.prepare_wakeup();

        let runtime = self.runtime.as_mut().unwrap();
        let res = runtime.execute_script("", script.as_str());
        println!("{:?}", res);

        self.cleanup_wakeup();

        loop {
            println!("Poll");
            if let Some(result) = self.poll_and_wait().await {
                break result;
            }
        }?;

        Ok(())
    }

    pub async fn execute_script(&mut self, script: String) -> Result<(), Box<dyn std::error::Error>> {
        let execution_time_limit = {
            let resource_table = &mut *self.resource_table();
            resource_table.execution_time_limit
        };

        if let Some(execution_time_limit) = execution_time_limit {
            tokio::select! {
                res = self.drive_execution(script) => res,
                // this stops the execution loop from outside
                // can only kick in between two wakeups and is therefore not able to interrupt CPU intensive work
                _ = tokio::time::sleep(execution_time_limit) =>
                    Err(generic_error("Isolate has run out of execution time").into())
            }
        } else {
            self.drive_execution(script).await
        }
    }

    pub fn teardown_runtime() {}
}