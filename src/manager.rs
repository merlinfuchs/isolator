use std::sync::Arc;
use crate::GlobalState;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use crate::runtime::WrappedRuntime;
use crate::service::isolator::{IsolateRequest, IsolateResponse, IsolateInitializedMessage};

pub struct RuntimeContext {
    pub receiver: mpsc::Receiver<IsolateRequest>,
    pub sender: mpsc::Sender<IsolateResponse>,
}

pub fn runtime_manager(state: Arc<GlobalState>, mut context: RuntimeContext) {
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut runtime = WrappedRuntime::new();
    runtime.create_runtime();
    runtime.prepare_runtime();

    tokio_runtime.block_on(async move {
        runtime.execute_script(String::from("1 + 1")).await;

        loop {
            tokio::select! {
                req = context.receiver.recv() => {
                    if let Some(req) = req {
                        println!("{:?}", req);
                        // TODO: fulfill requests on runtime
                    } else {
                        break;
                    }
                }
                // TODO: pipe responses from runtime and forward them to the sender
            }
        }
    });
}

pub fn thread_pool_manager(state: Arc<GlobalState>, mut receiver: mpsc::Receiver<RuntimeContext>) {
    let pool = threadpool::Builder::new()
        .num_threads(100)
        .thread_stack_size(100_000)
        .build();

    while let Some(context) = receiver.blocking_recv() {
        let handle_state = state.clone();
        pool.execute(move || runtime_manager(handle_state, context))
    }
}

// enforces the cpu time across cpu intensive wakeups
pub fn cpu_time_manager(state: Arc<GlobalState>) {
    loop {
        thread::sleep(Duration::from_millis(1));

        let mut runtimes_guard = state.runtimes.lock().unwrap();
        let runtimes = &mut *runtimes_guard;

        for (_, runtime) in runtimes {
            let mut time_table_guard = runtime.time_table.lock().unwrap();
            let time_table = &mut *time_table_guard;

            if let Some(current_wakeup) = time_table.current_wakeup {
                if let Some(cpu_time_limit) = time_table.cpu_time_limit {
                    let cpu_elapsed = time_table.cpu_time + current_wakeup.elapsed();

                    let mut isolate_guard = runtime.isolate_handle.lock().unwrap();
                    let isolate_handle = &mut *isolate_guard;

                    if let Some(isolate_handle) = isolate_handle {
                        if cpu_elapsed > cpu_time_limit {
                            isolate_handle.terminate_execution();
                        }
                    }
                }
            }
        }
    }
}