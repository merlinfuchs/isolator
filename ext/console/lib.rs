// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_core::include_js_files;
use deno_core::Extension;
use std::path::PathBuf;

pub fn init() -> Extension {
    Extension::builder()
        .js(include_js_files!(
      prefix "deno:ext/console",
      "00_colors.js",
      "01_console.js",
    ))
        .middleware(|name, opfn| match name {
            "op_print" => deno_core::void_op_sync(),
            _ => opfn
        })
        .build()
}

pub fn get_declaration() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib.deno_console.d.ts")
}
