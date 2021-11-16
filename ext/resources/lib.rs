extern crate deno_core;

use deno_core::{Extension, OpState, ZeroCopyBuf, op_async, ByteString, include_js_files};
use deno_core::error::AnyError;
use serde::Deserialize;
use serde::Serialize;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use uuid::Uuid;

pub struct ResourceRequest {
    pub resource_id: String,
    pub kind: String,
    pub payload: Option<Vec<u8>>,
}

pub struct ResourceResponse {
    pub payload: Option<Vec<u8>>,
}

pub type PendingResourceRequestsTable = HashMap<String, oneshot::Sender<ResourceResponse>>;
pub type ResourceRequestSender = Option<mpsc::Sender<ResourceRequest>>;

pub fn init() -> Extension {
    Extension::builder()
        .js(include_js_files!(
            prefix "isolator:ext/resources",
            "00_init.js",
        ))
        .ops(vec![
            ("op_resource_request_response", op_async(op_resource_request_response)),
            ("op_resource_request", op_async(op_resource_request)),
        ])
        .state(|state| {
            state.put(PendingResourceRequestsTable::default());
            state.put(ResourceRequestSender::None);
            Ok(())
        })
        .build()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResourceRequestArgs {
    kind: String,
    payload: Option<ByteString>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResourceRequestResponse {
    pub payload: Option<ByteString>,
}

pub async fn op_resource_request_response(
    state: Rc<RefCell<OpState>>,
    args: OpResourceRequestArgs,
    data: Option<ZeroCopyBuf>) -> Result<OpResourceRequestResponse, AnyError> {
    let resource_id = Uuid::new_v4().to_simple().to_string();
    let (resp_sender, resp_receiver) = oneshot::channel::<ResourceResponse>();

    {
        let mut borrowed_state = state.borrow_mut();

        let pending_requests = borrowed_state.borrow_mut::<PendingResourceRequestsTable>();
        pending_requests.insert(resource_id.clone(), resp_sender);

        let req_sender = borrowed_state.borrow_mut::<ResourceRequestSender>();
        if let Some(req_sender) = req_sender {
            req_sender.send(ResourceRequest {
                resource_id,
                kind: args.kind,
                payload: match args.payload {
                    Some(p) => Some(p.to_vec()),
                    None => None
                },
            }).await;
        }
    }

    let resp = resp_receiver.await?;
    Ok(OpResourceRequestResponse {payload: None})
}

pub async fn op_resource_request(
    state: Rc<RefCell<OpState>>,
    args: OpResourceRequestArgs,
    data: Option<ZeroCopyBuf>) -> Result<(), AnyError> {
    let resource_id = Uuid::new_v4().to_simple().to_string();
    let mut borrowed_state = state.borrow_mut();
    let req_sender = borrowed_state.borrow_mut::<ResourceRequestSender>();
    if let Some(req_sender) = req_sender {
        req_sender.send(ResourceRequest {
            resource_id,
            kind: args.kind,
            payload: match args.payload {
                Some(p) => Some(p.to_vec()),
                None => None
            },
        }).await;
    }

    Ok(())
}