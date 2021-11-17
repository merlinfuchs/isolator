use deno_core::include_js_files;
use deno_core::Extension;


pub fn init() -> Extension {
  Extension::builder()
    .js(include_js_files!(
      prefix "deno:ext/webidl",
      "00_webidl.js",
    ))
    .build()
}