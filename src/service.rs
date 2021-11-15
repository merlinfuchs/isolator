use isolator::isolator_server::Isolator;
use isolator::{IsolateRequest, IsolateResponse, GetStatusRequest, GetStatusResponse, KillIsolatesRequest, KillIsolatesResponse};
use tonic::{Status, Response, Request, Streaming};
use std::pin::Pin;
use futures_core::Stream;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::GlobalState;

use crate::runtime::{WrappedRuntime, SharedRuntimeState};

pub mod isolator {
    tonic::include_proto!("isolator");
}

pub struct IsolatorService {
    state: Arc<GlobalState>
}

#[tonic::async_trait]
impl Isolator for IsolatorService {
    type AcquireIsolateStream = Pin<Box<dyn Stream<Item=Result<IsolateResponse, Status>> + Send + 'static>>;

    async fn acquire_isolate(&self, request: Request<Streaming<IsolateRequest>>) -> Result<Response<Self::AcquireIsolateStream>, Status> {
        // this function must be able to communicate with the associate runtime in both directions
        // most likely through a sender -> receiver in each direction
        unimplemented!()
    }

    async fn get_status(&self, request: Request<GetStatusRequest>) -> Result<Response<GetStatusResponse>, Status> {
        Ok(Response::new(GetStatusResponse::default()))
    }

    async fn kill_isolates(&self, request: Request<KillIsolatesRequest>) -> Result<Response<KillIsolatesResponse>, Status> {
        Ok(Response::new(KillIsolatesResponse::default()))
    }
}
