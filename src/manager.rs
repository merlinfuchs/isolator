use std::cell::Ref;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use crate::GlobalState;
use std::thread;
use std::time::Duration;
use deno_core::OpState;
use ext_resources::{ResourceRequest, ResourceResponse};
use tokio::sync::{mpsc, oneshot};
use crate::runtime::WrappedRuntime;
use crate::service::isolator::{
    IsolateRequest,
    IsolateResponse,
    IsolateScriptResourceRequestMessage,
    isolate_request,
    isolate_response,
    isolate_request::Message::{InitializeMessage, ScheduleMessage, ScriptResourceResponse},
    isolate_response::Message::{ScriptResourceRequest},
};
use uuid::Uuid;

pub struct ServiceChannelPair {
    pub sender: mpsc::Sender<isolate_response::Message>,
    pub receiver: mpsc::Receiver<isolate_request::Message>,
}

pub struct RuntimeChannelPair {
    pub sender: mpsc::Sender<isolate_request::Message>,
    pub receiver: mpsc::Receiver<isolate_response::Message>,
}

async fn runtime_messaging_task(mut service_c: ServiceChannelPair, mut runtime_c: RuntimeChannelPair, mut resource_request_c: mpsc::Receiver<ResourceRequest>) {
    let mut pending_resource_requests: HashMap<String, oneshot::Sender<ResourceResponse>> = HashMap::new();

    loop {
        tokio::select! {
            service_req = service_c.receiver.recv() => {
                if let Some(req) = service_req {
                    println!("{:?}", req);
                    match req {
                        InitializeMessage(msg) => {
                            runtime_c.sender.send(InitializeMessage(msg)).await;
                        },
                        ScheduleMessage(msg) => {
                            runtime_c.sender.send(ScheduleMessage(msg)).await;
                        },
                        ScriptResourceResponse(msg) => {
                            if let Some(response_sender) = pending_resource_requests.remove(&msg.resource_id) {
                                response_sender.send(ResourceResponse {payload: Some(msg.payload)});
                            }
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }
            runtime_req = runtime_c.receiver.recv() => {}
            resource_req = resource_request_c.recv() => {
                if let Some(resource_req) = resource_req {
                    let resource_id = Uuid::new_v4().to_simple().to_string();
                    if let Some(response_sender) = resource_req.response_sender {
                        pending_resource_requests.insert(resource_id.clone(), response_sender);
                    }

                    service_c.sender.send(ScriptResourceRequest(IsolateScriptResourceRequestMessage {
                        resource_id: resource_id,
                        kind: resource_req.kind,
                        payload: resource_req.payload.unwrap_or_default()
                    })).await;
                }
            }
        }
    }
}

pub fn runtime_manager(state: Arc<GlobalState>, mut service_c: ServiceChannelPair) {
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

    let (to_sender, to_receiver) = mpsc::channel(10);
    let (from_sender, mut from_receiver) = mpsc::channel(10);
    let mut runtime_c = RuntimeChannelPair {
        sender: from_sender,
        receiver: to_receiver
    };

    local_set.block_on(&mut tokio_runtime, async move {
        tokio::task::spawn_local(runtime_messaging_task(service_c, runtime_c, resource_request_receiver));

        loop {
            while let Some(req) = from_receiver.recv().await {
                match req {
                    InitializeMessage(msg) => {
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
                    }
                    ScheduleMessage(msg) => {
                        let res = runtime.execute_script(msg.content).await;
                        println!("script done {:?}", res);
                    }
                    _ => {}
                }
            }
        }
    });
}

pub fn thread_pool_manager(state: Arc<GlobalState>, mut receiver: mpsc::Receiver<ServiceChannelPair>) {
    let pool = threadpool::Builder::new()
        .num_threads(100)
        .thread_stack_size(100_000)
        .build();

    while let Some(service_c) = receiver.blocking_recv() {
        let handle_state = state.clone();
        pool.execute(move || runtime_manager(handle_state, service_c))
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