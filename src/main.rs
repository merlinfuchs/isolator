use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use runtime::SharedRuntimeState;

mod service;
mod runtime;
mod threads;

pub struct GlobalState {
    pub max_runtime_count: Mutex<Option<u32>>,
    pub runtimes: Mutex<HashMap<String, Arc<SharedRuntimeState>>>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    Ok(())
}
