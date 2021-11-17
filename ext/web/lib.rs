extern crate deno_core;

use deno_core::{Extension, ZeroCopyBuf, op_sync};
use deno_core::include_js_files;
use deno_core::OpState;
use deno_core::error::AnyError;
use std::fmt;

pub fn init() -> Extension {
    Extension::builder()
        .js(include_js_files!(
            prefix "deno:ext/web",
            "00_infra.js",
            "01_web_exception.js",
            "02_base64.js",
        ))
        .ops(vec![
            ("op_base64_decode", op_sync(op_base64_decode)),
            ("op_base64_encode", op_sync(op_base64_encode)),
        ])
        .build()
}

fn op_base64_decode(
    _state: &mut OpState,
    input: String,
    _: (),
) -> Result<ZeroCopyBuf, AnyError> {
    let mut input: &str = &input.replace(|c| char::is_ascii_whitespace(&c), "");
    // "If the length of input divides by 4 leaving no remainder, then:
    //  if input ends with one or two U+003D EQUALS SIGN (=) characters,
    //  remove them from input."
    if input.len() % 4 == 0 {
        if input.ends_with("==") {
            input = &input[..input.len() - 2]
        } else if input.ends_with('=') {
            input = &input[..input.len() - 1]
        }
    }

    // "If the length of input divides by 4 leaving a remainder of 1,
    //  throw an InvalidCharacterError exception and abort these steps."
    if input.len() % 4 == 1 {
        return Err(
            DomExceptionInvalidCharacterError::new("Failed to decode base64.").into(),
        );
    }

    if input
        .chars()
        .any(|c| c != '+' && c != '/' && !c.is_alphanumeric())
    {
        return Err(
            DomExceptionInvalidCharacterError::new(
                "Failed to decode base64: invalid character",
            )
                .into(),
        );
    }

    let cfg = base64::Config::new(base64::CharacterSet::Standard, true)
        .decode_allow_trailing_bits(true);
    let out = base64::decode_config(&input, cfg).map_err(|err| {
        DomExceptionInvalidCharacterError::new(&format!(
            "Failed to decode base64: {:?}",
            err
        ))
    })?;
    Ok(ZeroCopyBuf::from(out))
}

fn op_base64_encode(
    _state: &mut OpState,
    s: ZeroCopyBuf,
    _: (),
) -> Result<String, AnyError> {
    let cfg = base64::Config::new(base64::CharacterSet::Standard, true)
        .decode_allow_trailing_bits(true);
    let out = base64::encode_config(&s, cfg);
    Ok(out)
}

#[derive(Debug)]
pub struct DomExceptionInvalidCharacterError {
  pub msg: String,
}

impl DomExceptionInvalidCharacterError {
  pub fn new(msg: &str) -> Self {
    DomExceptionInvalidCharacterError {
      msg: msg.to_string(),
    }
  }
}

impl fmt::Display for DomExceptionInvalidCharacterError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.pad(&self.msg)
  }
}

impl std::error::Error for DomExceptionInvalidCharacterError {}
