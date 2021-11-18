use deno_core::{ModuleLoader, ModuleSpecifier, ModuleSourceFuture, OpState, error::AnyError, ModuleResolutionError, ModuleSource};
use std::pin::Pin;
use std::rc::Rc;
use std::cell::RefCell;
use deno_core::error::{anyhow};
use ext_resources::{ResourceRequest, ResourceRequestSender, ResourceResponse};
use futures::future::FutureExt;
use tokio::sync::oneshot;


pub struct InternalModuleLoader {
    // ugly workaround because I don't understand how prepare_load is supposed to work
    pub op_state: Rc<RefCell<Option<Rc<RefCell<OpState>>>>>,
}

pub fn make_module_specifier(specifier: &str, referrer: &str) -> Result<ModuleSpecifier, ModuleResolutionError> {
    match deno_core::resolve_import(specifier, referrer) {
        Ok(url) => Ok(url),
        Err(err) => match err {
            ModuleResolutionError::ImportPrefixMissing(_, _) => {
                // modules specifiers in Deno must always be a fully qualified URL
                // this is usually not what you would want here so we are just appending a default prefix if it's missing
                let mut new_specifier = String::from("https://isolator/");
                new_specifier.push_str(specifier);
                deno_core::resolve_import(&new_specifier, referrer)
            }
            _ => Err(err)
        }
    }
}

impl ModuleLoader for InternalModuleLoader {
    fn resolve(&self, specifier: &str, referrer: &str, _is_main: bool) -> Result<ModuleSpecifier, AnyError> {
        match make_module_specifier(specifier, referrer) {
            Ok(s) => Ok(s),
            Err(e) => Err(e.into())
        }
    }

    fn load(&self, specifier: &ModuleSpecifier, _maybe_referrer: Option<ModuleSpecifier>, _is_dyn_import: bool) -> Pin<Box<ModuleSourceFuture>> {
        let specifier = specifier.clone();

        let op_state = self.op_state.borrow().clone();

        async move {
            if let Some(op_state) = op_state {
                let (resp_sender, resp_receiver) = oneshot::channel::<ResourceResponse>();

                {
                    let mut borrowed_state = op_state.borrow_mut();
                    let req_sender = borrowed_state.borrow_mut::<ResourceRequestSender>();
                    if let Some(req_sender) = req_sender {
                        let payload = specifier.to_string().as_bytes().to_vec();
                        let res = req_sender.send(ResourceRequest {
                            response_sender: Some(resp_sender),
                            kind: "module".to_string(),
                            payload: Some(payload),
                        }).await;
                        if let Err(_) = res {
                            return Err(anyhow!("Request Manager unavailable to load module: {}", specifier.to_string()));
                        }
                    }
                }

                let resp = resp_receiver.await?;
                if let Some(payload) = resp.payload {
                    if let Ok(payload) = String::from_utf8(payload) {
                        return Ok(ModuleSource {
                            code: payload,
                            module_url_found: specifier.to_string(),
                            module_url_specified: specifier.to_string(),
                        });
                    }
                }

                Err(anyhow!("Failed to load module: {}", specifier.to_string()))
            } else {
                Err(anyhow!("OpState unavailable to load module: {}", specifier.to_string()))
            }
        }.boxed_local()
    }
}
