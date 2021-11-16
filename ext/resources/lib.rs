extern crate deno_core;

use deno_core::{Extension, OpState, ZeroCopyBuf, op_async, ByteString, include_js_files};
use deno_core::error::{AnyError, generic_error};
use serde::Deserialize;
use serde::Serialize;
use std::rc::Rc;
use std::cell::RefCell;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub struct ResourceRequest {
    pub response_sender: Option<oneshot::Sender<ResourceResponse>>,
    pub kind: String,
    pub payload: Option<Vec<u8>>,
}

pub struct ResourceResponse {
    pub payload: Option<Vec<u8>>,
}

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
    _data: Option<ZeroCopyBuf>) -> Result<OpResourceRequestResponse, AnyError> {
    let (resp_sender, resp_receiver) = oneshot::channel::<ResourceResponse>();

    {
        let mut borrowed_state = state.borrow_mut();

        let req_sender = borrowed_state.borrow_mut::<ResourceRequestSender>();
        if let Some(req_sender) = req_sender {
            let res = req_sender.send(ResourceRequest {
                response_sender: Some(resp_sender),
                kind: args.kind,
                payload: match args.payload {
                    Some(p) => Some(p.to_vec()),
                    None => None
                },
            }).await;
            if let Err(_) = res {
                return Err(generic_error("Unable to communicate with the request manager"))
            }
        }
    }

    let resp = resp_receiver.await?;
    Ok(OpResourceRequestResponse {
        payload: match resp.payload {
            Some(p) => Some(ByteString(p)),
            None => None
        }
    })
}

pub async fn op_resource_request(
    state: Rc<RefCell<OpState>>,
    args: OpResourceRequestArgs,
    _data: Option<ZeroCopyBuf>) -> Result<(), AnyError> {
    let mut borrowed_state = state.borrow_mut();
    let req_sender = borrowed_state.borrow_mut::<ResourceRequestSender>();
    if let Some(req_sender) = req_sender {
        let res = req_sender.send(ResourceRequest {
            response_sender: None,
            kind: args.kind,
            payload: match args.payload {
                Some(p) => Some(p.to_vec()),
                None => None
            },
        }).await;
        if let Err(_) = res {
            return Err(generic_error("Unable to communicate with the request manager"))
        }
    }

    Ok(())
}