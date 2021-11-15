use std::collections::HashMap;
use tonic::transport::Server;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use runtime::SharedRuntimeState;
use std::thread;
use crate::manager::{cpu_time_manager, thread_pool_manager};
use crate::service::IsolatorService;

use service::isolator::isolator_server::IsolatorServer;

mod service;
mod runtime;
mod manager;

pub struct GlobalState {
    pub runtimes: Mutex<HashMap<String, Arc<SharedRuntimeState>>>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (scheduler_sender, scheduler_receiver) = mpsc::channel(1);
    let state = Arc::new(GlobalState {
        runtimes: Mutex::new(HashMap::new())
    });

    let thread_state = state.clone();
    thread::spawn(move || thread_pool_manager(thread_state, scheduler_receiver));

    let thread_state = state.clone();
    thread::spawn(move || cpu_time_manager(thread_state));

    let service = IsolatorService {
        state,
        scheduler: scheduler_sender,
    };

    let addr = "127.0.0.1:50051".parse().unwrap();
    Server::builder()
        .add_service(IsolatorServer::new(service))
        .serve(addr)
        .await
        .unwrap();

    println!("Hello, world!");
    Ok(())
}
