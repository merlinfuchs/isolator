use std::cell::Ref;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use crate::GlobalState;
use std::thread;
use std::time::Duration;
use deno_core::OpState;
use ext_resources::ResourceResponse;
use tokio::sync::mpsc;
use crate::runtime::WrappedRuntime;
use crate::service::isolator::{
    IsolateRequest,
    IsolateResponse,
    IsolateScriptResourceRequestMessage,
    isolate_request::Message::{InitializeMessage, ScheduleMessage, ScriptResourceResponse},
    isolate_response::Message::{ScriptResourceRequest}
};

pub struct RuntimeContext {
    pub receiver: mpsc::Receiver<IsolateRequest>,
    pub sender: mpsc::Sender<IsolateResponse>,
}

pub fn runtime_manager(state: Arc<GlobalState>, mut context: RuntimeContext) {
    let mut tokio_runtime = tokio::runtime::Builder::new_current_thread()
        // IO isn't enabled because communication only happens through channels
        .enable_time()
        .build()
        .unwrap();
    let local_set = tokio::task::LocalSet::new();

    let mut runtime = WrappedRuntime::new();
    runtime.create_runtime(); 
    runtime.prepare_runtime();

    let (resource_request_sender, mut resource_request_receiver) = mpsc::channel::<ext_resources::ResourceRequest>(10);

    runtime.op_state().borrow_mut().put(Some(resource_request_sender));

    local_set.block_on(&mut tokio_runtime, async move {
        loop {
            tokio::select! {
                req = context.receiver.recv() => {
                    if let Some(req) = req {
                        println!("{:?}", req);
                        match req.message {
                            Some(InitializeMessage(msg)) => {
                                let resource_table = &mut *runtime.resource_table();

                                if msg.cpu_time_limit == 0 {
                                    resource_table.cpu_time_limit = None
                                } else {
                                    resource_table.cpu_time_limit = Some(Duration::from_millis(msg.cpu_time_limit))
                                }

                                if msg.execution_time_limit == 0 {
                                    resource_table.execution_time_limit = None
                                } else {
                                    resource_table.execution_time_limit = Some(Duration::from_millis(msg.execution_time_limit))
                                }

                                if msg.resource_requests_limit == 0 {
                                    resource_table.resource_requests_limit = None
                                } else {
                                    resource_table.resource_requests_limit = Some(msg.resource_requests_limit)
                                }
                            },
                            Some(ScheduleMessage(msg)) => {
                                tokio::task::spawn_local(runtime.execute_script(msg.content));
                            },
                            Some(ScriptResourceResponse(msg)) => {
                                let op_state = runtime.op_state();
                                let mut borrowed_op_state = op_state.borrow_mut();
                                let pending_requests = borrowed_op_state.borrow_mut::<ext_resources::PendingResourceRequestsTable>();
                                if let Some(response_sender) = pending_requests.remove(&msg.resource_id) {
                                    response_sender.send(ResourceResponse {payload: None});
                                }
                            }
                            _ => {}
                        }
                    } else {
                        break;
                    }
                },
                resource_req = resource_request_receiver.recv() => {
                    println!("resource request");
                    if let Some(resource_req) = resource_req {
                        context.sender.send(IsolateResponse {message: Some(ScriptResourceRequest(IsolateScriptResourceRequestMessage {
                            resource_id: resource_req.resource_id,
                            kind: resource_req.kind,
                            payload: resource_req.payload.unwrap_or_default()
                        }))}).await;
                    }
                }
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
            let mut resource_table_guard = runtime.resource_table.lock().unwrap();
            let resource_table = &mut *resource_table_guard;

            if let Some(current_wakeup) = resource_table.current_wakeup {
                if let Some(cpu_time_limit) = resource_table.cpu_time_limit {
                    let cpu_elapsed = resource_table.cpu_time + current_wakeup.elapsed();

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