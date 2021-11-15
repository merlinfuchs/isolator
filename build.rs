use std::env;
use std::path::{Path, PathBuf};
use deno_core::{JsRuntime, RuntimeOptions, Extension};

fn build_protobuf() {
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile(&["./protobuf/service.proto"], &["./protobuf"])
        .unwrap();
}

fn create_runtime() -> JsRuntime {
    let extensions = vec![
        ext_resources::init()
    ];

    let mut runtime = JsRuntime::new(RuntimeOptions {
        will_snapshot: true,
        extensions,
        ..Default::default()
    });

    runtime
}

fn build_runtime_snapshot() {
    let mut runtime = create_runtime();

    let snapshot = runtime.snapshot();
    let snapshot_slice: &[u8] = &*snapshot;

    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = o.join("RUNTIME_SNAPSHOT.bin");

    std::fs::write(&snapshot_path, snapshot_slice).unwrap();
}

fn main() {
    build_protobuf();
    build_runtime_snapshot()
}