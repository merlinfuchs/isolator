use isolator::isolator_server::Isolator;
use isolator::{
    IsolateRequest,
    IsolateResponse,
    GetStatusRequest,
    GetStatusResponse,
    KillIsolatesRequest,
    KillIsolatesResponse,
    DrainRequest,
    DrainResponse,
    KillRequest,
    KillResponse,
};
use tonic::{Status, Response, Request, Streaming};
use std::pin::Pin;
use futures_core::Stream;
use futures_util::StreamExt;
use std::sync::{Arc};
use crate::GlobalState;
use crate::manager::ServiceChannelPair;
use tokio::sync::mpsc;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod isolator {
    tonic::include_proto!("isolator");
}

pub struct IsolatorService {
    pub state: Arc<GlobalState>,
    pub scheduler: mpsc::Sender<ServiceChannelPair>,
    pub accept_requests: AtomicBool
}

#[tonic::async_trait]
impl Isolator for IsolatorService {
    type AcquireIsolateStream = Pin<Box<dyn Stream<Item=Result<IsolateResponse, Status>> + Send + 'static>>;

    async fn acquire_isolate(&self, request: Request<Streaming<IsolateRequest>>) -> Result<Response<Self::AcquireIsolateStream>, Status> {
        if !self.accept_requests.load(Ordering::Relaxed) {
            return Err(Status::unavailable("instance has been drained"))
        }

        let (to_sender, to_receiver) = mpsc::channel(10);
        let (from_sender, mut from_receiver) = mpsc::channel(10);

        let service_c = ServiceChannelPair {
            receiver: to_receiver,
            sender: from_sender,
        };

        if let Err(_) = self.scheduler.send(service_c).await {
            panic!("Thread scheduler unavailable");
        }
        let mut stream = request.into_inner();

        let output = async_stream::try_stream! {
            loop {
                tokio::select! {
                    resp = from_receiver.recv() => {
                        if let Some(resp) = resp {
                            yield IsolateResponse {message: Some(resp)}
                        } else {
                            break;
                        }
                    }
                    req = stream.next() => {
                        if let Some(req) = req {
                            if let Ok(req) = req {
                                if let Some(msg) = req.message {
                                    let res = to_sender.send(msg).await;
                                    if let Err(_) = res { break; }
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(output) as Self::AcquireIsolateStream))
    }

    async fn kill_isolates(&self, _request: Request<KillIsolatesRequest>) -> Result<Response<KillIsolatesResponse>, Status> {
        Ok(Response::new(KillIsolatesResponse::default()))
    }

    async fn get_status(&self, _request: Request<GetStatusRequest>) -> Result<Response<GetStatusResponse>, Status> {
        Ok(Response::new(GetStatusResponse::default()))
    }

    async fn drain(&self, _request: Request<DrainRequest>) -> Result<Response<DrainResponse>, Status> {
        self.accept_requests.store(false, Ordering::Relaxed);
        Ok(Response::new(DrainResponse::default()))
    }

    async fn kill(&self, _request: Request<KillRequest>) -> Result<Response<KillResponse>, Status> {
        tokio::spawn(async { process::exit(0); });
        Ok(Response::new(KillResponse::default()))
    }
}
